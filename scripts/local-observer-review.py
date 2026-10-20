"""Compare unchanged proposals, category-filtered proposals and refined crop boxes.
Only images/geometry enter Rust inference. Existing labels are used by this scorer.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics
import subprocess
import time
import urllib.request

parser = argparse.ArgumentParser()
parser.add_argument("--source", type=Path, required=True)
parser.add_argument("--out", type=Path, required=True)
parser.add_argument("--model", required=True)
parser.add_argument("--rescore", action="store_true")
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
source, out = args.source.resolve(), args.out.resolve()
out.mkdir(parents=True, exist_ok=True)
binary = root / "target/release/reversebolt-review.exe"
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
read = lambda p: json.loads(p.read_text(encoding="utf-8"))
base = read(source / "results.json")
manifest_path = out / "run-manifest.json"
if args.rescore:
    manifest = read(manifest_path)
    assert manifest["source_results_sha256"] == sha(source / "results.json") and manifest["model"] == args.model
else:
    with urllib.request.urlopen("http://127.0.0.1:11434/api/tags", timeout=5) as response:
        model = next(m for m in json.load(response)["models"] if m["name"] == args.model)
    manifest = {"source_results_sha256": sha(source / "results.json"), "model": args.model, "model_digest": model["digest"], "binary_sha256": sha(binary), "script_sha256": sha(Path(__file__)), "source_sha256": {str(p.relative_to(root)): sha(p) for p in sorted((root / "crates/reversebolt-observer").rglob("*.rs"))}}
    if manifest_path.exists():
        assert read(manifest_path) == manifest, "Run changed; use a fresh directory"
    else:
        manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")


def iou(a, b):
    x, y, w, h = a
    xx, yy, ww, hh = b
    intersection = max(0, min(x+w, xx+ww)-max(x, xx)) * max(0, min(y+h, yy+hh)-max(y, yy))
    return intersection / max(1e-9, w*h+ww*hh-intersection)


def as_box(bounds, size):
    l, t, r, b = bounds
    w, h = size
    return [l*w/1000, t*h/1000, (r-l)*w/1000, (b-t)*h/1000]


def score(boxes, truth, ignores, threshold):
    edges = sorted([(iou(b, t["box"]), pi, ti) for pi, b in enumerate(boxes) for ti, t in enumerate(truth)], reverse=True)
    used_p, used_t, matches = set(), set(), []
    for overlap, pi, ti in edges:
        if overlap >= threshold and pi not in used_p and ti not in used_t:
            used_p.add(pi)
            used_t.add(ti)
            matches.append((pi, ti))
    ignored = {i for i, b in enumerate(boxes) if i not in used_p and any(iou(b, ib) >= .3 for ib in ignores)}
    metrics = {"tp": len(used_p), "fp": len(boxes)-len(used_p)-len(ignored), "fn": len(truth)-len(used_t), "ignored": len(ignored)}
    for label, kind in [("actor", "actor"), ("component", "viewmodel_component"), ("static_reference", "static_reference")]:
        metrics[label+"_tp"] = sum(truth[i]["kind"] == kind for i in used_t)
        metrics[label+"_total"] = sum(t["kind"] == kind for t in truth)
    return metrics, matches


def displacement(tracks, truth, size):
    tracks = [t for t in tracks if any(l["frame"] == 1 for l in t["localizations"])]
    locations = [{l["frame"]: as_box(l["bounds"], size) for l in t["localizations"]} for t in tracks]
    _, matches = score([l[1] for l in locations], truth["objects"], [], .3)
    previous = {t["id"]: t for t in truth["previous_objects"]}
    rows = []
    for pi, ti in matches:
        target, loc = truth["objects"][ti], locations[pi]
        row = {"paired": False, "has_previous_truth": target["id"] in previous, "has_previous_prediction": 0 in loc}
        if row["has_previous_truth"] and row["has_previous_prediction"] and iou(loc[0], previous[target["id"]]["box"]) >= .3:
            def delta(a, b):
                return [b[0]+b[2]/2-a[0]-a[2]/2, b[1]+b[3]/2-a[1]-a[3]/2]
            expected = delta(previous[target["id"]]["box"], target["box"])
            predicted = delta(loc[0], loc[1])
            row.update(paired=True, error_px=math.dist(expected, predicted), stationary=math.hypot(*expected) < .01, predicted_displacement_px=math.hypot(*predicted))
        rows.append(row)
    return rows


records, elapsed = [], []
for record in base["records"]:
    clip, frame = record["clip"], record["frame"]
    stem = f"{clip}-{frame:05}"
    proposal_file = source / (stem+".receipt.json")
    proposal = read(proposal_file)
    original_input = read(source / (stem+".input.json"))
    annotation_file = root / "experiments/visual-challenge/annotations" / (clip+".json")
    assert sha(annotation_file) == record["labels_sha256"]
    annotation = read(annotation_file)
    truth = next(f for f in annotation["frames"] if f["frame"] == frame)
    size = annotation["display_size"]
    full_truth = truth["objects"] + truth.get("static_reference", [])
    variants = {"original": [], "filtered": [], "refined": []}
    reviews = []
    entities = (proposal["interpretation"] or {"entities": []})["entities"]
    for index, entity in enumerate(entities):
        if entity["kind"] not in ("actor", "held_component") or not any(l["frame"] == 1 for l in entity["localizations"]):
            continue
        variants["original"].append(entity)
        request = {"model": args.model, "frames": original_input["frames"], "proposal_receipt": str(proposal_file), "candidate": index}
        request_file, receipt_file, error_file = [out / f"{stem}-{index}{suffix}" for suffix in [".input.json", ".receipt.json", ".error.json"]]
        if request_file.exists():
            assert read(request_file) == request
        else:
            assert not args.rescore, "Rescore needs existing inputs"
            request_file.write_text(json.dumps(request, indent=2), encoding="utf-8")
        if not receipt_file.exists() and not error_file.exists():
            assert not args.rescore, "Missing review; rescore never starts inference"
            start = time.monotonic()
            try:
                result = subprocess.run([str(binary), str(request_file), str(receipt_file)], capture_output=True, text=True, encoding="utf-8", timeout=125)
                if result.returncode:
                    error_file.write_text(json.dumps({"returncode": result.returncode, "stderr": result.stderr, "elapsed_ms": (time.monotonic()-start)*1000}), encoding="utf-8")
            except subprocess.TimeoutExpired:
                error_file.write_text(json.dumps({"timeout": True, "elapsed_ms": (time.monotonic()-start)*1000}), encoding="utf-8")
        review = None
        if receipt_file.exists():
            receipt = read(receipt_file)
            assert receipt["candidate"] == index and receipt["proposal_receipt_sha256"] == sha(proposal_file)
            review = receipt["review"]
            assert review["model_digest"] == manifest["model_digest"]
            for crop in review["crops"]:
                assert crop["source"] == proposal["frames"][crop["frame"]]
            elapsed.append(review["elapsed_ms"])
        judgment = review and review["judgment"]
        supported = bool(judgment and judgment["kind"] in ("actor", "held_component") and any(l["frame"] == 1 for l in review["refined"]))
        reviews.append({"candidate": index, "supported": supported, "kind": judgment["kind"] if judgment else None, "rejection": review["rejection"] if review else "transport/process error", "receipt_sha256": sha(receipt_file) if receipt_file.exists() else None})
        if supported:
            # Do not manufacture a previous observation when the review omits it.
            seen = {l["frame"] for l in review["refined"]}
            variants["filtered"].append({**entity, "localizations": [l for l in entity["localizations"] if l["frame"] in seen]})
            variants["refined"].append({"kind": judgment["kind"], "name": judgment["name"], "localizations": review["refined"]})
    scores, motion = {}, {}
    for key, tracks in variants.items():
        boxes = [as_box(l["bounds"], size) for t in tracks for l in t["localizations"] if l["frame"] == 1]
        scores[key] = {str(t): score(boxes, full_truth, truth["ignore"], t)[0] for t in [.3, .5]}
        motion[key] = displacement(tracks, truth, size)
    # An evaluator change must not silently alter the established baseline.
    for threshold in ["0.3", "0.5"]:
        assert all(scores["original"][threshold][k] == v for k, v in record["scores"][threshold].items())
    rows = {"clip": clip, "frame": frame, "reviews": reviews, "scores": scores, "motion": motion, "variants": variants, "negative": not full_truth, "proposal_receipt_sha256": sha(proposal_file), "labels_sha256": sha(annotation_file)}
    records.append(rows)
    if not args.rescore:
        (out / "results.partial.json").write_text(json.dumps(records, indent=2), encoding="utf-8")
    print(json.dumps({"clip": clip, "frame": frame, "reviews": len(reviews), "kept": sum(r["supported"] for r in reviews), "strict": {k: [s["0.5"][n] for n in ["tp", "fp", "fn"]] for k, s in scores.items()}}), flush=True)

totals, motion = {}, {}
for variant in ["original", "filtered", "refined"]:
    totals[variant] = {}
    for threshold in ["0.3", "0.5"]:
        m = {k: sum(r["scores"][variant][threshold][k] for r in records) for k in records[0]["scores"][variant][threshold]}
        m["precision"] = m["tp"]/(m["tp"]+m["fp"]) if m["tp"]+m["fp"] else None
        m["recall"] = m["tp"]/(m["tp"]+m["fn"]) if m["tp"]+m["fn"] else None
        totals[variant][threshold] = m
    samples = [m for r in records for m in r["motion"][variant]]
    pairs = [s for s in samples if s["paired"]]
    moving = [s for s in pairs if not s["stationary"]]
    still = [s for s in pairs if s["stationary"]]
    motion[variant] = {"current_matches": len(samples), "paired_matches": len(pairs), "moving_pairs": len(moving), "moving_mean_error_px": statistics.mean(s["error_px"] for s in moving) if moving else None, "stationary_pairs": len(still), "stationary_mean_predicted_displacement_px": statistics.mean(s["predicted_displacement_px"] for s in still) if still else None, "all_mean_error_px": statistics.mean(s["error_px"] for s in pairs) if pairs else None}
reviews = [rev for r in records for rev in r["reviews"]]
summary = {"scope": "Same-model crop review on previously seen archived pairs. Category filtering and coordinate refinement are separate variants. These are provisional image judgments, not independent semantic/causal validation. Both endpoint boxes must match at IoU >=0.3 for displacement; units are 320x180 annotation pixels, with roughly +/-2px label uncertainty.", "script_sha256": sha(Path(__file__)), "pairs": len(records), "reviews": len(reviews), "kept": sum(r["supported"] for r in reviews), "invalid_reviews": sum(r["rejection"] is not None for r in reviews), "median_review_ms": statistics.median(elapsed) if elapsed else None, "totals": totals, "motion": motion, "records": records}
with (out / ("results.rescored.json" if args.rescore else "results.json")).open("x", encoding="utf-8") as file:
    json.dump(summary, file, indent=2)
print(json.dumps({k: v for k, v in summary.items() if k != "records"}), flush=True)
