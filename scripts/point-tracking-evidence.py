"""Verify local point-tracking receipts and retain a compact, image-free record."""
from collections import Counter
import hashlib
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "test-results/point-tracking-v1/frozen-replay-validated"
FIRST = ROOT / "test-results/point-tracking-v1/frozen-replay"
read = lambda p: json.loads(p.read_text(encoding="utf-8"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()


def main():
    results, frozen = read(RAW/"results.json"), read(RAW/"frozen-manifest.json")
    assert results["frozen_manifest_sha256"] == sha(RAW/"frozen-manifest.json")
    for path, digest in frozen["source_sha256"].items():
        assert sha(ROOT/path) == digest
    assert sha(ROOT/"target/release/reversebolt-point-track.exe") == frozen["binary_sha256"]
    upstream = read(ROOT/"research/learned-tracking/upstream.json")
    for source in upstream["sources"]:
        assert sha(ROOT/source["path"]) == source["sha256"]
    assert sha(ROOT/upstream["model"]["path"]) == upstream["model"]["sha256"]
    assert sha(ROOT/"research/learned-tracking/vendor/MODEL_CARD.md") == upstream["model_card"]["sha256"]
    assert len(results["records"]) == len(frozen["jobs"]) == 48
    summaries, large_motion = [], []
    for record, job in zip(results["records"], frozen["jobs"], strict=True):
        assert record["stem"] == job["stem"]
        stem = record["stem"]
        receipt_path = RAW/(stem+".receipt.json")
        points_path = RAW/"inference"/(stem+".points.json")
        model_path = RAW/"inference"/(stem+".model.json")
        assert record["receipt_sha256"] == sha(receipt_path)
        assert record["points_sha256"] == sha(points_path)
        assert record["model_sha256"] == sha(model_path)
        receipt, model, points = read(receipt_path), read(model_path), read(points_path)
        assert receipt["input_sha256"] == model["points_sha256"] == sha(points_path)
        assert model["input_sha256"] == record["input_sha256"]
        assert model["environment_sha256"] == sha(RAW/"inference/environment.json")
        frames = [receipt["initial_frame"]]+[s["frame"] for s in receipt["steps"]]
        assert frames == [points["seed_frame"]]+[s["frame"] for s in points["steps"]]
        for actual, expected in zip(frames, job["frames"], strict=True):
            assert all(actual[k] == v for k, v in expected.items())
        for track in record["trajectories"]["gated"]:
            if track["opportunity"] and math.hypot(*track["expected_displacement_px"]) >= 4:
                first_loss = next(({"update": i+1, "sample": s} for i, step in enumerate(receipt["steps"])
                                   for s in step["gated"] if s["id"] == track["seed"] and s["reason"]), None)
                large_motion.append({"window": stem, "trajectory": track, "first_loss": first_loss})
        summaries.append({k: record[k] for k in ("stem", "input_sha256", "points_sha256", "model_sha256", "receipt_sha256")})
    totals = {name: {threshold: dict(sum((Counter(r["scores"][name][threshold]) for r in results["records"]), Counter()))
                    for threshold in ("0.3", "0.5")} for name in ("gated", "ungated")}
    assert totals == results["summary"]["totals"]
    first = read(FIRST/"results.json")
    for key in ("windows", "seeds", "zero_seed_windows", "totals", "tracking", "gated_first_rejections"):
        assert first["summary"][key] == results["summary"][key], key
    snapshot = FIRST/"source-snapshot"
    first_frozen = read(FIRST/"frozen-manifest.json")
    assert sha(snapshot/"point_tracks.rs") == first_frozen["source_sha256"]["crates/reversebolt-observer/src/point_tracks.rs"]
    assert sha(snapshot/"reversebolt-point-track.exe") == first_frozen["binary_sha256"]
    record = {"schema_version": 1, "scope": results["scope"],
              "decision": "Keep research-only: better small-motion retention, but all three >=4px opportunities lost; no generalization credit. Goal stays 22%.",
              "raw_directory": RAW.relative_to(ROOT).as_posix(), "raw_results_sha256": sha(RAW/"results.json"),
              "frozen_manifest_sha256": sha(RAW/"frozen-manifest.json"), "binary_sha256": frozen["binary_sha256"],
              "source_sha256": frozen["source_sha256"], "labels_sha256": frozen["labels_sha256"],
              "upstream": upstream, "environment": read(RAW/"inference/environment.json"),
              "parameters": results["parameters"], "summary": results["summary"], "prior_summary": results["prior_summary"],
              "larger_motion_cases": large_motion, "receipts": summaries,
              "procedural_probe": {"path": "test-results/point-tracking-v1/probe-01/result.json",
                                   "sha256": sha(ROOT/"test-results/point-tracking-v1/probe-01/result.json"),
                                   "result": read(ROOT/"test-results/point-tracking-v1/probe-01/result.json")},
              "prior_attempt": {"path": FIRST.relative_to(ROOT).as_posix(), "results_sha256": sha(FIRST/"results.json"),
                                "manifest_sha256": sha(FIRST/"frozen-manifest.json"),
                                "change": "After the first replay, input validation rejects subpixel seed boxes to prevent degenerate floating-point fits. Minimum seed side in this cohort was 25.6 source pixels. Tracking thresholds, inference and scoring were unchanged. Complete rerun has identical detection, retention, rejection and displacement metrics; timings differ. Original source and binary retained locally in source-snapshot/."}}
    destination = ROOT/"research/learned-tracking/evidence/replay-v1.json"
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        assert read(destination) == record
    else:
        destination.write_text(json.dumps(record, indent=2)+"\n", encoding="utf-8")
    print(json.dumps({"verified_windows": len(summaries), "larger_motion_cases": len(large_motion),
                      "evidence_sha256": sha(destination), "summary": results["summary"]}))


if __name__ == "__main__":
    main()
