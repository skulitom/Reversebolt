"""Pinned, image-only causal TAPIR research runner. No target/process access.

This intentionally isolated Python dependency is not part of the desktop runtime.
Each prediction receives one current image and past causal state, never future images.
"""
import argparse
import hashlib
import importlib.metadata
import json
from pathlib import Path
import platform
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "research/learned-tracking/vendor"))
import numpy as np
from PIL import Image
import torch
from tapnet.torch import tapir_model

RESOLUTION = 256
GRID = (.25, .5, .75)


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def save(path, value):
    with Path(path).open("x", encoding="utf-8") as file:
        json.dump(value, file, indent=2, allow_nan=False)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--batch", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    jobs = read(args.batch)
    assert 1 <= len(jobs) <= 100
    assert len({j["key"] for j in jobs}) == len(jobs)
    assert all(j["key"].replace("-", "").replace("_", "").isalnum() for j in jobs)
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    provenance = read(ROOT / "research/learned-tracking/upstream.json")
    for source in provenance["sources"]:
        assert sha(ROOT / source["path"]) == source["sha256"]
    checkpoint = ROOT / provenance["model"]["path"]
    assert sha(checkpoint) == provenance["model"]["sha256"]
    assert torch.cuda.is_available(), "This frozen protocol uses CUDA; no silent CPU fallback."
    torch.manual_seed(0)
    torch.set_num_threads(4)
    torch.backends.cudnn.benchmark = False
    torch.backends.cudnn.deterministic = True
    torch.backends.cuda.matmul.allow_tf32 = False
    torch.backends.cudnn.allow_tf32 = False
    environment = {"python": sys.version, "prefix": sys.prefix, "platform": platform.platform(),
                   "packages": {n: importlib.metadata.version(n) for n in
                                ("torch", "numpy", "Pillow", "dm-tree", "einshape")},
                   "cuda": torch.version.cuda, "gpu": torch.cuda.get_device_name(0),
                   "runner_sha256": sha(__file__), "upstream_sha256": sha(ROOT / "research/learned-tracking/upstream.json"),
                   "checkpoint_sha256": sha(checkpoint), "resolution": RESOLUTION, "grid": GRID,
                   "causal": True, "future_frames_per_call": 0, "query_chunk_size": 64,
                   "batch_sha256": sha(args.batch), "tf32": False}
    save(out / "environment.json", environment)
    started = time.perf_counter()
    model = tapir_model.TAPIR(pyramid_level=1, use_casual_conv=True)
    model.load_state_dict(torch.load(checkpoint, weights_only=True, map_location="cpu"), strict=True)
    model = model.to("cuda").eval()
    print(json.dumps({"model_loaded_ms": (time.perf_counter()-started)*1000}), flush=True)
    for job in jobs:
        request_path = Path(job["path"])
        request = read(request_path)
        assert set(request) == {"frames", "seed_image_sha256", "seeds"}
        assert 2 <= len(request["frames"]) <= 120 and len(request["seeds"]) <= 6
        assert len({s["id"] for s in request["seeds"]}) == len(request["seeds"])
        for seed in request["seeds"]:
            l, t, r, b = seed["bounds"]
            assert 0 <= l < r <= 1000 and 0 <= t < b <= 1000
        causal = features = initial = None
        steps, timings, initial_predictions = [], [], []
        previous = None
        with torch.inference_mode():
            for index, source in enumerate(request["frames"]):
                torch.cuda.synchronize()
                started = time.perf_counter()
                path = Path(source["path"])
                assert path.stat().st_size <= 32*1024*1024
                digest = sha(path)
                with Image.open(path) as image:
                    width, height = image.size
                    assert image.format == "PNG" and 0 < width <= 16384 and 0 < height <= 16384
                    assert width*height <= 40_000_000
                    rgb = np.array(image.convert("RGB").resize((RESOLUTION, RESOLUTION), Image.Resampling.BILINEAR))
                evidence = {"sha256": digest, "width": width, "height": height, "timestamp_ns": source["timestamp_ns"]}
                if initial is None:
                    assert digest == request["seed_image_sha256"]
                    initial = evidence
                else:
                    assert (width, height) == (initial["width"], initial["height"])
                    assert 0 < source["timestamp_ns"] - previous <= 2_000_000_000
                previous = source["timestamp_ns"]
                groups = []
                if request["seeds"]:
                    video = torch.from_numpy(rgb).to(device="cuda", dtype=torch.float32)[None, None] / 255*2-1
                    grids = model.get_feature_grids(video, is_training=False)
                    if features is None:
                        queries = []
                        for seed in request["seeds"]:
                            l, t, r, b = seed["bounds"]
                            queries.extend([[0, (t+y*(b-t))*RESOLUTION/1000, (l+x*(r-l))*RESOLUTION/1000]
                                            for y in GRID for x in GRID])
                        queries = torch.tensor([queries], device="cuda", dtype=torch.float32)
                        features = model.get_query_features(video, False, queries, grids)
                        causal = [{k: v.to("cuda") for k, v in state.items()} for state in
                                  model.construct_initial_causal_state(len(queries[0]), len(features.resolutions)-1)]
                    trajectories = model.estimate_trajectories(
                        video.shape[-3:-1], False, grids, features, None, query_chunk_size=64,
                        causal_context=causal, get_causal_context=True)
                    causal = trajectories["causal_context"]
                    tracks = trajectories["tracks"][-1][0, :, 0].cpu().numpy()
                    visibility = ((1-torch.sigmoid(trajectories["occlusion"][-1])) *
                                  (1-torch.sigmoid(trajectories["expected_dist"][-1])))[0, :, 0].cpu().numpy()
                    for i, seed in enumerate(request["seeds"]):
                        groups.append({"id": seed["id"], "points": [
                            {"xy": [float(tracks[n, 0])*width/RESOLUTION, float(tracks[n, 1])*height/RESOLUTION],
                             "visibility": float(visibility[n])} for n in range(i*9, (i+1)*9)]})
                torch.cuda.synchronize()
                timings.append((time.perf_counter()-started)*1000)
                if index:
                    steps.append({"frame": evidence, "points": groups})
                else:
                    initial_predictions = groups
        key = job["key"]
        save(out / (key+".points.json"), {"seed_frame": initial, "seeds": request["seeds"], "steps": steps})
        save(out / (key+".model.json"), {"input_sha256": sha(request_path), "environment_sha256": sha(out/"environment.json"),
                                      "points_sha256": sha(out/(key+".points.json")),
                                      "initial_predictions": initial_predictions, "frame_times_ms": timings})
        print(json.dumps({"completed": key, "seeds": len(request["seeds"]), "elapsed_ms": sum(timings)}), flush=True)


if __name__ == "__main__":
    main()
