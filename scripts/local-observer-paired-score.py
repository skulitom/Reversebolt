"""Score within-request image correspondences against retained adjacent-pair labels.
No inference, new annotations, interpolated truth or physical-motion claims.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics

parser = argparse.ArgumentParser()
parser.add_argument("run", type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
run = args.run.resolve()
labels_root = root / "experiments/visual-challenge/annotations"
results = json.loads((run / "results.json").read_text(encoding="utf-8"))


def box(bounds, size):
    l, t, r, b = bounds
    w, h = size
    return [l*w/1000, t*h/1000, (r-l)*w/1000, (b-t)*h/1000]


def iou(a, b):
    x, y, w, h = a
    xx, yy, ww, hh = b
    area = max(0, min(x+w, xx+ww)-max(x, xx)) * max(0, min(y+h, yy+hh)-max(y, yy))
    return area / max(1e-9, w*h+ww*hh-area)


def delta(a, b):
    return [b[0]+b[2]/2-a[0]-a[2]/2, b[1]+b[3]/2-a[1]-a[3]/2]


rows = []
for record in results["records"]:
    label_file = labels_root / (record["clip"] + ".json")
    assert hashlib.sha256(label_file.read_bytes()).hexdigest() == record["labels_sha256"]
    annotation = json.loads(label_file.read_text(encoding="utf-8"))
    truth = next(f for f in annotation["frames"] if f["frame"] == record["frame"])
    receipt_path = run / f"{record['clip']}-{record['frame']:05}.receipt.json"
    if not receipt_path.exists():
        continue
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    assert [f["sha256"] for f in receipt["frames"]] == record["image_sha256"]
    proposed = []
    for e in (receipt["interpretation"] or {"entities": []})["entities"]:
        if e.get("kind", "actor") not in ("actor", "held_component"):
            continue
        locations = {l["frame"]: box(l["bounds"], annotation["display_size"]) for l in e["localizations"]}
        if 1 in locations:
            proposed.append(locations)
    # Static references were supplied only for the later frame. Do not invent
    # earlier annotations by copying them backward, even for an idle clip.
    current_truth = truth["objects"]
    previous_truth = {t["id"]: t for t in truth["previous_objects"]}
    edges = sorted([(iou(p[1], t["box"]), pi, ti) for pi, p in enumerate(proposed) for ti, t in enumerate(current_truth)], reverse=True)
    used_p, used_t = set(), set()
    for overlap, pi, ti in edges:
        if overlap < .3 or pi in used_p or ti in used_t:
            continue
        used_p.add(pi)
        used_t.add(ti)
        p, t = proposed[pi], current_truth[ti]
        row = {"clip": record["clip"], "frame": record["frame"], "truth_id": t["id"], "kind": t["kind"], "current_iou": overlap, "previous_truth": t["id"] in previous_truth, "previous_prediction": 0 in p, "paired_match": False}
        if row["previous_truth"] and row["previous_prediction"]:
            prev = previous_truth[t["id"]]
            row["previous_iou"] = iou(p[0], prev["box"])
            if row["previous_iou"] >= .3:
                row["paired_match"] = True
                expected, predicted = delta(prev["box"], t["box"]), delta(p[0], p[1])
                row["truth_delta_px"] = expected
                row["predicted_delta_px"] = predicted
                row["endpoint_error_px"] = math.dist(expected, predicted)
                row["stationary_truth"] = math.hypot(*expected) < .01
                row["predicted_displacement_px"] = math.hypot(*predicted)
                row["interval_ns"] = receipt["frames"][1]["timestamp_ns"] - receipt["frames"][0]["timestamp_ns"]
        rows.append(row)

paired = [r for r in rows if r["paired_match"]]
stationary = [r for r in paired if r["stationary_truth"]]
summary = {"scope": "Conditional on BOTH image boxes matching the same annotated visible entity at IoU >=0.3. Units are 320x180 annotation pixels, with about +/-2 pixel label uncertainty. No continuous identity, world speed or animation rule is established. Static references lacking earlier labels are excluded, not copied backward.", "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), "results_sha256": hashlib.sha256((run / "results.json").read_bytes()).hexdigest(), "current_matches": len(rows), "with_previous_truth": sum(r["previous_truth"] for r in rows), "with_previous_prediction_and_truth": sum(r["previous_truth"] and r["previous_prediction"] for r in rows), "paired_matches": len(paired), "mean_endpoint_error_px": statistics.mean(r["endpoint_error_px"] for r in paired) if paired else None, "median_endpoint_error_px": statistics.median(r["endpoint_error_px"] for r in paired) if paired else None, "stationary_pairs": len(stationary), "stationary_mean_predicted_displacement_px": statistics.mean(r["predicted_displacement_px"] for r in stationary) if stationary else None, "rows": rows}
with (run / "paired-results.json").open("x", encoding="utf-8") as f:
    json.dump(summary, f, indent=2)
print(json.dumps({k: v for k, v in summary.items() if k != "rows"}))
