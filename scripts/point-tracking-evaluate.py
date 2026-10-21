"""Freeze and score all 48 existing image-only windows using learned point tracking.

Annotations are read here, never passed to the separate inference process or Rust
aggregator. All clips were previously examined; this is not unseen-game evidence.
"""
import argparse
from collections import Counter
from datetime import datetime, timezone
import importlib.util
import json
import math
from pathlib import Path
import statistics
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "test-results/seeded-tracking-v1/frozen-replay"
spec = importlib.util.spec_from_file_location("old_scorer", ROOT/"scripts/seeded-tracking-evaluate.py")
old = importlib.util.module_from_spec(spec)
spec.loader.exec_module(old)
read, save, sha, score, box, delta = old.read, old.save, old.sha, old.score, old.box, old.delta


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    assert sha(BASE/"results.json") == "788e2119d74a0c6c59d715e913973231f3817c2871c6437a29edc547d0fed64c"
    assert sha(BASE/"frozen-manifest.json") == "08b921b92a9cfdcb38db5cfb99407b0b9aa353e70902da3479404756f0d6c6d1"
    baseline, protocol = read(BASE/"results.json"), read(BASE/"frozen-manifest.json")
    binary = ROOT / "target/release/reversebolt-point-track.exe"
    jobs = protocol["jobs"]
    assert len(jobs) == 48
    labels, batch = {}, []
    for job in jobs:
        path = BASE / (job["stem"]+".input.json")
        assert sha(path) == job["input_sha256"]
        request = read(path)
        for source, expected in zip(request["frames"], job["frames"], strict=True):
            assert sha(Path(source["path"])) == expected["sha256"]
            assert source["timestamp_ns"] == expected["timestamp_ns"]
        clip = job["clip"]
        label_path = ROOT/"experiments/visual-challenge/annotations"/(clip+".json")
        assert sha(label_path) == protocol["labels_sha256"][clip]
        labels[clip] = read(label_path)
        batch.append({"key": job["stem"], "path": str(path)})
    save(out/"batch.json", batch)
    source_paths = [Path(__file__), ROOT/"scripts/point-tracking-infer.py", ROOT/"scripts/seeded-tracking-evaluate.py",
                    ROOT/"crates/reversebolt-observer/src/point_tracks.rs",
                    ROOT/"crates/reversebolt-observer/src/bin/reversebolt-point-track.rs",
                    ROOT/"research/learned-tracking/upstream.json"]
    frozen = {"created_utc": datetime.now(timezone.utc).isoformat(), "baseline_manifest_sha256": sha(BASE/"frozen-manifest.json"),
              "baseline_results_sha256": sha(BASE/"results.json"), "binary_sha256": sha(binary),
              "source_sha256": {p.relative_to(ROOT).as_posix(): sha(p) for p in source_paths},
              "batch_sha256": sha(out/"batch.json"), "jobs": jobs, "labels_sha256": protocol["labels_sha256"],
              "protocol": "All original 48 windows and 57 model-box seeds, including empty and false seeds. Nine fixed inner grid points per seed. Causal BootsTAPIR, 256-square input, one frame per inference call, initial-image query features retained, no future frames, no reseeding. Rust robust translation/scale aggregation with and without visibility/coherence gates; permanent loss. Original endpoint IoU .3/.5 scorer and conditional same-ID motion metrics. Parameters frozen before first game inference; no tuning on outcomes."}
    save(out/"frozen-manifest.json", frozen)
    print(json.dumps({"frozen": len(jobs), "manifest_sha256": sha(out/"frozen-manifest.json")}), flush=True)
    subprocess.run([sys.executable, str(ROOT/"scripts/point-tracking-infer.py"), "--batch", str(out/"batch.json"),
                    "--out", str(out/"inference")], check=True, timeout=900)
    results, timings, rejections = [], [], Counter()
    for job in jobs:
        stem, clip = job["stem"], job["clip"]
        point_path = out/"inference"/(stem+".points.json")
        meta_path = out/"inference"/(stem+".model.json")
        aggregation_path = out/(stem+".receipt.json")
        subprocess.run([str(binary), str(point_path), str(aggregation_path)], check=True, timeout=15)
        receipt, meta = read(aggregation_path), read(meta_path)
        assert receipt["input_sha256"] == sha(point_path) == meta["points_sha256"]
        assert meta["input_sha256"] == job["input_sha256"]
        assert meta["environment_sha256"] == sha(out/"inference/environment.json")
        assert receipt["algorithm_source_sha256"] == frozen["source_sha256"]["crates/reversebolt-observer/src/point_tracks.rs"]
        actual_frames = [receipt["initial_frame"]]+[s["frame"] for s in receipt["steps"]]
        for actual, expected in zip(actual_frames, job["frames"], strict=True):
            assert all(actual[k] == v for k, v in expected.items())
        if receipt["seeds"]:
            timings.extend(meta["frame_times_ms"][1:])
        for step in receipt["steps"]:
            for sample in step["gated"]:
                if sample["reason"] and sample["reason"] != "track_lost":
                    rejections[sample["reason"]] += 1
        label = labels[clip]
        size = label["display_size"]
        initial, endpoint = [next(f for f in label["frames"] if f["frame"] == n) for n in [job["start"], job["end"]]]
        initial_truth = initial["objects"]+initial.get("static_reference", [])
        endpoint_truth = endpoint["objects"]+endpoint.get("static_reference", [])
        initial_boxes = [box(s["bounds"], size) for s in receipt["seeds"]]
        initial_counts, initial_matches, initial_ignored = score(initial_boxes, initial_truth, initial["ignore"], .3)
        end_by_id = {t["id"]: t for t in endpoint_truth}
        scores, trajectories = {}, {}
        for name in ("gated", "ungated"):
            visible = [s for s in receipt["steps"][-1][name] if s["bounds"] is not None]
            predictions = [box(s["bounds"], size) for s in visible]
            scores[name] = {str(t): score(predictions, endpoint_truth, endpoint["ignore"], t)[0] for t in (.3, .5)}
            _, matches, ignored = score(predictions, endpoint_truth, endpoint["ignore"], .3)
            visible_by_seed = {s["id"]: i for i, s in enumerate(visible)}
            trajectories[name] = []
            for si, seed in enumerate(receipt["seeds"]):
                target = initial_truth[initial_matches[si]] if si in initial_matches else None
                pi = visible_by_seed.get(seed["id"])
                matched = endpoint_truth[matches[pi]] if pi in matches else None
                opportunity = bool(target and target["id"] in end_by_id)
                retained = bool(target and matched and target["id"] == matched["id"])
                row = {"seed": seed["id"], "initial_correct": target is not None, "initial_ignored": si in initial_ignored,
                       "initial_truth_id": target["id"] if target else None, "survived": pi is not None,
                       "endpoint_truth_id": matched["id"] if matched else None, "endpoint_ignored": pi in ignored,
                       "opportunity": opportunity, "retained": retained,
                       "switched": bool(target and matched and target["id"] != matched["id"])}
                if opportunity:
                    expected = delta(target["box"], end_by_id[target["id"]]["box"])
                    row.update(expected_displacement_px=expected, stationary=math.hypot(*expected)<.01)
                    if retained:
                        predicted = delta(initial_boxes[si], predictions[pi])
                        row.update(predicted_displacement_px=predicted, error_px=math.dist(expected, predicted))
                trajectories[name].append(row)
        results.append({**job, "points_sha256": sha(point_path), "model_sha256": sha(meta_path),
                        "receipt_sha256": sha(aggregation_path), "seeds": len(receipt["seeds"]),
                        "initial_scores": initial_counts, "scores": scores, "trajectories": trajectories})
    totals = {name: {str(t): dict(sum((Counter(r["scores"][name][str(t)]) for r in results), Counter()))
                    for t in (.3, .5)} for name in ("gated", "ungated")}
    tracking = {}
    for name in ("gated", "ungated"):
        rows = [row for r in results for row in r["trajectories"][name]]
        summary = {key: sum(r[key] for r in rows) for key in ("initial_correct", "initial_ignored", "survived", "opportunity", "retained", "switched")}
        summary["initial_false_survived"] = sum(not r["initial_correct"] and not r["initial_ignored"] and r["survived"] for r in rows)
        summary["motion"] = {}
        for category, predicate in (("stationary", lambda r: r["stationary"]),
                                    ("moving", lambda r: not r["stationary"]),
                                    ("displacement_at_least_4px", lambda r: math.hypot(*r["expected_displacement_px"])>=4)):
            group = [r for r in rows if r["opportunity"] and predicate(r)]
            retained = [r for r in group if r["retained"]]
            summary["motion"][category] = {"opportunities": len(group), "retained": len(retained),
                                          "mean_error_px": statistics.mean(r["error_px"] for r in retained) if retained else None,
                                          "median_error_px": statistics.median(r["error_px"] for r in retained) if retained else None}
        tracking[name] = summary
    summary = {"windows": len(jobs), "seeds": sum(r["seeds"] for r in results),
               "zero_seed_windows": sum(r["seeds"] == 0 for r in results), "totals": totals, "tracking": tracking,
               "gated_first_rejections": dict(rejections), "seeded_step_times_ms": {
                   "count": len(timings), "median": statistics.median(timings), "max": max(timings)}}
    for path, digest in frozen["source_sha256"].items():
        assert sha(ROOT/path) == digest, "Source changed after freeze."
    assert sha(binary) == frozen["binary_sha256"]
    save(out/"results.json", {"scope": "Previously examined clips, 10 Hz saved frames and one-second endpoints. Endpoint retention is not continuous identity. Annotation units 320x180, roughly +/-2 px. False initial detections remain false. No semantic, hidden-rule, world-motion or gameplay authority claims. Timing includes decode/preprocess/inference/device synchronization but excludes capture, seed discovery and Rust aggregation.",
                             "frozen_manifest_sha256": sha(out/"frozen-manifest.json"),
                             "parameters": receipt["parameters"], "summary": summary,
                             "prior_summary": baseline["summary"], "records": results})
    print(json.dumps(summary), flush=True)


if __name__ == "__main__":
    main()
