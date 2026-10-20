"""Replay fixed annotated pairs through the Rust local observer, then score boxes.

Annotations never enter the observer process. Source footage remains local.
Default uses all existing annotated pairs; --first limits each clip for feasibility.
Existing receipts are reused only when input/binary/source hashes match the run manifest.
"""
import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument("--model", required=True)
parser.add_argument("--out", type=Path, required=True)
parser.add_argument("--first", type=int, default=0)
parser.add_argument("--rescore", action="store_true", help="Score retained receipts without starting inference; write results.rescored.json")
parser.add_argument("--clips", nargs="+", default=["dev_q_demo", "dev_c_idle", "dev_c_move", "test_q_demo", "test_c_transition"])
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
baseline = root / "experiments/visual-challenge"
binary = root / "target/release/reversebolt-observer.exe"
out = args.out.resolve()
out.mkdir(parents=True, exist_ok=True)
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
sources = sorted((root / "crates/reversebolt-observer").rglob("*.rs"))
manifest = {"model": args.model, "binary_sha256": sha(binary), "source_sha256": {str(p.relative_to(root)): sha(p) for p in sources}, "clips": args.clips, "first": args.first, "script_sha256": sha(Path(__file__))}
manifest_path = out / "run-manifest.json"
if manifest_path.exists():
    prior = json.loads(manifest_path.read_text(encoding="utf-8"))
    checked = ["model", "clips", "first"] if args.rescore else ["model", "clips", "first", "binary_sha256", "source_sha256"]
    assert all(prior[k] == manifest[k] for k in checked), "Run configuration changed; use a new output directory"
else:
    assert not args.rescore, "No retained run to score"
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")


def iou(a, b):
    ax, ay, aw, ah = a
    bx, by, bw, bh = b
    intersection = max(0, min(ax + aw, bx + bw) - max(ax, bx)) * max(0, min(ay + ah, by + bh) - max(ay, by))
    return intersection / max(1e-9, aw * ah + bw * bh - intersection)


def score(predictions, truth, ignores, threshold):
    candidates = sorted([(iou(p, t["box"]), pi, ti) for pi, p in enumerate(predictions) for ti, t in enumerate(truth)], reverse=True)
    matched_p, matched_t = set(), set()
    for overlap, pi, ti in candidates:
        if overlap >= threshold and pi not in matched_p and ti not in matched_t:
            matched_p.add(pi)
            matched_t.add(ti)
    ignored = {i for i, p in enumerate(predictions) if i not in matched_p and any(iou(p, box) >= .3 for box in ignores)}
    return {"tp": len(matched_p), "fp": len(predictions) - len(matched_p) - len(ignored), "fn": len(truth) - len(matched_t), "ignored": len(ignored), "actor_tp": sum(truth[i]["kind"] == "actor" for i in matched_t), "actor_total": sum(t["kind"] == "actor" for t in truth), "component_tp": sum(truth[i]["kind"] == "viewmodel_component" for i in matched_t), "component_total": sum(t["kind"] == "viewmodel_component" for t in truth), "static_reference_tp": sum(truth[i]["kind"] == "static_reference" for i in matched_t), "static_reference_total": sum(t["kind"] == "static_reference" for t in truth)}


records = []
for clip in args.clips:
    labels_path = baseline / "annotations" / (clip + ".json")
    labels = json.loads(labels_path.read_text(encoding="utf-8"))
    labels_frames = labels["frames"][:args.first or None]
    timeline = [json.loads(line) for line in (baseline / "local" / clip / "manifest.jsonl").read_text(encoding="utf-8").splitlines()]
    for label in labels_frames:
        numbers = [label["previous_frame"], label["frame"]]
        images = [baseline / "local" / clip / timeline[n]["path"] for n in numbers]
        input_value = {"model": args.model, "frames": [{"path": str(p), "timestamp_ns": timeline[n]["timestamp_ns"]} for n, p in zip(numbers, images)]}
        stem = f"{clip}-{label['frame']:05}"
        input_file, receipt_file, error_file = [out / (stem + suffix) for suffix in [".input.json", ".receipt.json", ".error.json"]]
        if input_file.exists():
            assert json.loads(input_file.read_text(encoding="utf-8")) == input_value
        else:
            input_file.write_text(json.dumps(input_value), encoding="utf-8")
        if not receipt_file.exists() and not error_file.exists():
            assert not args.rescore, "Missing receipt; rescore never runs inference"
            start = time.monotonic()
            try:
                run = subprocess.run([str(binary), str(input_file), str(receipt_file)], capture_output=True, text=True, encoding="utf-8", timeout=125)
                if run.returncode:
                    error_file.write_text(json.dumps({"returncode": run.returncode, "stderr": run.stderr, "elapsed_ms": (time.monotonic() - start) * 1000}), encoding="utf-8")
            except subprocess.TimeoutExpired:
                error_file.write_text(json.dumps({"timeout": True, "elapsed_ms": (time.monotonic() - start) * 1000}), encoding="utf-8")
        receipt = json.loads(receipt_file.read_text(encoding="utf-8")) if receipt_file.exists() else None
        if receipt:
            assert [f["sha256"] for f in receipt["frames"]] == [sha(p) for p in images]
        parsed = receipt and receipt["interpretation"]
        boxes = []
        if parsed:
            width, height = labels["display_size"]
            for entity in parsed["entities"]:
                if entity.get("kind", "actor") not in ("actor", "held_component"):
                    continue
                for loc in entity["localizations"]:
                    if loc["frame"] == 1:
                        l, t, r, b = loc["bounds"]
                        boxes.append([l * width / 1000, t * height / 1000, (r - l) * width / 1000, (b - t) * height / 1000])
        truth = label["objects"] + label.get("static_reference", [])
        record = {"clip": clip, "frame": label["frame"], "accepted": bool(parsed), "image_sha256": [sha(p) for p in images], "labels_sha256": sha(labels_path), "rejection": receipt["rejection"] if receipt else "transport/process failure", "elapsed_ms": receipt["elapsed_ms"] if receipt else None, "predictions": boxes, "unscored_kinds": [e["kind"] for e in parsed["entities"] if e.get("kind", "actor") not in ("actor", "held_component")] if parsed else [], "negative": not truth, "exhaustive": label["object_exhaustive"], "scores": {str(t): score(boxes, truth, label["ignore"], t) for t in [.3, .5]}}
        records.append(record)
        if not args.rescore:
            (out / "results.partial.json").write_text(json.dumps(records, indent=2), encoding="utf-8")
        print(json.dumps({"clip": clip, "frame": label["frame"], "accepted": bool(parsed), "elapsed_ms": record["elapsed_ms"], "scores": record["scores"]}), flush=True)

totals = {}
for threshold in ["0.3", "0.5"]:
    totals[threshold] = {key: sum(r["scores"][threshold][key] for r in records if r["exhaustive"]) for key in records[0]["scores"][threshold]}
    s = totals[threshold]
    s["precision"] = s["tp"] / (s["tp"] + s["fp"]) if s["tp"] + s["fp"] else None
    s["recall"] = s["tp"] / (s["tp"] + s["fn"]) if s["tp"] + s["fn"] else None
durations = [r["elapsed_ms"] for r in records if r["elapsed_ms"] is not None]
summary = {"scope": "Replay of existing annotated clips; no fresh or unseen-game claim. Score original dynamic actor/component boxes PLUS supplied static_reference boxes. This differs from the prior activity benchmark; static actors are valid here. Single-rater +/-2 processing pixels. Rejected outputs count as no detections. Other unlabeled object types cannot establish precision for all useful gameplay objects.", "evaluator_sha256":sha(Path(__file__)), "pairs": len(records), "accepted": sum(r["accepted"] for r in records), "exhaustive_pairs": sum(r["exhaustive"] for r in records), "negative_pairs": sum(r["negative"] for r in records), "negative_pairs_with_proposals": sum(r["negative"] and bool(r["predictions"]) for r in records), "median_ms": statistics.median(durations) if durations else None, "scores": totals, "records": records}
result_path = out / ("results.rescored.json" if args.rescore else "results.json")
with result_path.open("x", encoding="utf-8") as f:
    json.dump(summary, f, indent=2)
print(json.dumps({k: v for k, v in summary.items() if k != "records"}), flush=True)
