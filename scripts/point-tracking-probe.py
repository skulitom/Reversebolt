"""Generate an owned procedural point-translation check without game labels."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import hashlib
import math
import statistics
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    rng = np.random.default_rng(8182)
    patch = rng.integers(20, 255, (64, 64, 3), dtype=np.uint8)
    # Fixed coarse texture with sharply visible features, translated as one object.
    patch = np.array(Image.fromarray(patch).resize((16, 16)).resize((64, 64), Image.Resampling.NEAREST))
    frames = []
    for i in range(11):
        pixels = np.full((256, 256, 3), 25, dtype=np.uint8)
        pixels[64+i:128+i, 64+3*i:128+3*i] = patch
        path = out / f"frame-{i:02}.png"
        Image.fromarray(pixels).save(path)
        frames.append({"path": str(path), "timestamp_ns": 1+i*100_000_000})
    request = {"frames": frames, "seed_image_sha256": hashlib.sha256(Path(frames[0]["path"]).read_bytes()).hexdigest(),
               "seeds": [{"id": 0, "bounds": [250, 250, 500, 500]}]}
    (out/"input.json").write_text(json.dumps(request), encoding="utf-8")
    (out/"batch.json").write_text(json.dumps([{"key": "probe", "path": str(out/"input.json")}]), encoding="utf-8")
    subprocess.run([sys.executable, str(ROOT/"scripts/point-tracking-infer.py"), "--batch", str(out/"batch.json"),
                    "--out", str(out/"inference")], check=True)
    points = json.loads((out/"inference/probe.points.json").read_text())
    errors = []
    for i, p in enumerate(points["steps"][-1]["points"][0]["points"]):
        expected = [64+(i%3+1)*16+30, 64+(i//3+1)*16+10]
        errors.append(math.dist(expected, p["xy"]))
    subprocess.run([str(ROOT/"target/release/reversebolt-point-track.exe"), str(out/"inference/probe.points.json"),
                    str(out/"aggregation.json")], check=True)
    aggregation = json.loads((out/"aggregation.json").read_text())
    result = {"endpoint_point_errors_px": errors, "median_error_px": statistics.median(errors),
              "final_aggregation": aggregation["steps"][-1],
              "passed": statistics.median(errors) < 3 and aggregation["steps"][-1]["gated"][0]["bounds"] is not None}
    (out/"result.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result), flush=True)
    assert result["passed"]


if __name__ == "__main__":
    main()
