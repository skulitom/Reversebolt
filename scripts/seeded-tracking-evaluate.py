"""Frozen one-second replays. Labels stay in this scorer, never enter the tracker.

Run from the repository root with --out NEW_DIRECTORY. Existing game clips and
model/crop-review receipts must be present locally; no inference or game control.
"""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import statistics
import subprocess

ROOT = Path(__file__).resolve().parents[1]
read = lambda p: json.loads(p.read_text(encoding="utf-8"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()


def save(path, value):
    with path.open("x", encoding="utf-8") as f:
        json.dump(value, f, indent=2)


def box(bounds, size):
    l, t, r, b = bounds
    w, h = size
    return [l*w/1000, t*h/1000, (r-l)*w/1000, (b-t)*h/1000]


def iou(a, b):
    x, y, w, h = a
    xx, yy, ww, hh = b
    area = max(0, min(x+w, xx+ww)-max(x, xx)) * max(0, min(y+h, yy+hh)-max(y, yy))
    return area/max(1e-9, w*h+ww*hh-area)


def score(predictions, truth, ignores, threshold):
    edges = sorted([(iou(p, t["box"]), pi, ti) for pi, p in enumerate(predictions)
                    for ti, t in enumerate(truth)], reverse=True)
    matched, used = {}, set()
    for overlap, pi, ti in edges:
        if overlap >= threshold and pi not in matched and ti not in used:
            matched[pi] = ti
            used.add(ti)
    ignored = {i for i, p in enumerate(predictions) if i not in matched
               and any(iou(p, ib) >= .3 for ib in ignores)}
    counts = dict(tp=len(matched), fp=len(predictions)-len(matched)-len(ignored),
                  fn=len(truth)-len(used), ignored=len(ignored))
    return counts, matched, ignored


def center(b):
    return [b[0]+b[2]/2, b[1]+b[3]/2]


def delta(a, b):
    return [y-x for x, y in zip(center(a), center(b))]


def seeds(record):
    return [{"id": i, "bounds": next(l["bounds"] for l in entity["localizations"] if l["frame"] == 1)}
            for i, entity in enumerate(record["variants"]["filtered"])]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    # An experiment cannot silently replace receipts or change its inputs.
    out.mkdir(parents=True, exist_ok=False)
    source = ROOT / "test-results/local-observer-review-v1/frozen-replay/results.json"
    proposals = ROOT / "test-results/local-observer-v1/qwen-categorized-all"
    binary = ROOT / "target/release/reversebolt-track.exe"
    algorithm = ROOT / "crates/reversebolt-observer/src/tracking.rs"
    records = {(r["clip"], r["frame"]): r for r in read(source)["records"]}
    jobs, labels, manifests, receipt_hashes = [], {}, {}, {}
    for (clip, frame), record in sorted(records.items()):
        if (clip, frame+10) not in records:
            continue
        label_path = ROOT / "experiments/visual-challenge/annotations" / (clip+".json")
        label = read(label_path)
        labels[clip] = {"sha256": sha(label_path), "data": label}
        by_frame = {a["frame"]: a for a in label["frames"]}
        assert frame in by_frame and frame+10 in by_frame
        clip_dir = ROOT / "experiments/visual-challenge/local" / clip
        frame_manifest = clip_dir / "manifest.jsonl"
        manifests[clip] = sha(frame_manifest)
        images = [json.loads(line) for line in frame_manifest.read_text().splitlines()]
        images = {int(Path(f["path"]).stem): f for f in images}
        frames = [{"path": str(clip_dir / images[n]["path"]), "timestamp_ns": images[n]["timestamp_ns"]}
                  for n in range(frame, frame+11)]
        evidence = [{"sha256": sha(Path(f["path"])), "timestamp_ns": f["timestamp_ns"]} for f in frames]
        original = proposals / f"{clip}-{frame:05}.receipt.json"
        original_receipt = read(original)
        assert original_receipt["frames"][1]["sha256"] == evidence[0]["sha256"]
        assert original_receipt["frames"][1]["timestamp_ns"] == evidence[0]["timestamp_ns"]
        receipt_hashes[original.name] = sha(original)
        request = {"frames": frames, "seed_image_sha256": evidence[0]["sha256"], "seeds": seeds(record)}
        stem = f"{clip}-{frame:05}"
        request_path = out / (stem+".input.json")
        save(request_path, request)
        jobs.append({"clip": clip, "start": frame, "end": frame+10, "stem": stem,
                     "input_sha256": sha(request_path), "frames": evidence})
    assert len(jobs) == 48, "This protocol fixes all 48 adjacent annotated one-second windows."
    frozen = {"created_utc": datetime.now(timezone.utc).isoformat(), "binary_sha256": sha(binary),
              "algorithm_sha256": sha(algorithm), "script_sha256": sha(Path(__file__)),
              "source_results_sha256": sha(source), "proposal_receipts_sha256": receipt_hashes,
              "labels_sha256": {k: v["sha256"] for k, v in labels.items()},
              "frame_manifests_sha256": manifests, "jobs": jobs,
              "protocol": "All 48 windows, including zero seeds. 10 intervening 100 ms frames. Seeds are category-filtered original model boxes, never labels. Hold, ungated, gated and independently refreshed model boxes are scored at endpoint IoU .3/.5, with greedy one-to-one matching and unmatched ignore IoU >=.3. Identity/motion conditional on initial correct match at .3 and same endpoint annotation id. No tuning after real replay begins."}
    save(out / "frozen-manifest.json", frozen)
    # This is the freeze boundary: all requests and source/binary/scorer hashes
    # are written before any tracker outcome is available.
    print(json.dumps({"frozen_windows": len(jobs), "algorithm_sha256": frozen["algorithm_sha256"]}), flush=True)
    results, timings = [], {"gated": [], "ungated": []}
    rejections = Counter()
    for job in jobs:
        stem, clip = job["stem"], job["clip"]
        request_path, receipt_path = out/(stem+".input.json"), out/(stem+".receipt.json")
        run = subprocess.run([str(binary), str(request_path), str(receipt_path)], capture_output=True,
                             text=True, encoding="utf-8", timeout=120)
        if run.returncode:
            save(out/(stem+".error.json"), {"returncode": run.returncode, "stderr": run.stderr})
            raise RuntimeError("Tracker failed; preserve the run for diagnosis: "+stem)
        receipt, request = read(receipt_path), read(request_path)
        assert receipt["input_sha256"] == job["input_sha256"]
        assert receipt["algorithm_source_sha256"] == frozen["algorithm_sha256"]
        observed_frames = [receipt["initial_frame"]]+[step["frame"] for step in receipt["steps"]]
        assert len(observed_frames) == 11
        for actual, expected in zip(observed_frames, job["frames"]):
            assert all(actual[k] == v for k, v in expected.items())
        label = labels[clip]["data"]
        size = label["display_size"]
        initial, endpoint = [next(f for f in label["frames"] if f["frame"] == n) for n in [job["start"],job["end"]]]
        initial_truth = initial["objects"] + initial.get("static_reference", [])
        endpoint_truth = endpoint["objects"] + endpoint.get("static_reference", [])
        initial_boxes = [box(s["bounds"], size) for s in request["seeds"]]
        initial_counts, initial_matches, initial_ignored = score(initial_boxes, initial_truth, initial["ignore"], .3)
        end_by_id = {t["id"]: t for t in endpoint_truth}
        variants = {"hold": request["seeds"], "refreshed_model": seeds(records[(clip,job["end"])])}
        variants.update({name: receipt["steps"][-1][name] for name in ["gated","ungated"]})
        scores, trajectories = {}, {}
        for name, samples in variants.items():
            visible = [s for s in samples if s["bounds"] is not None]
            predictions = [box(s["bounds"],size) for s in visible]
            scores[name] = {str(t): score(predictions, endpoint_truth, endpoint["ignore"], t)[0] for t in [.3,.5]}
            if name == "refreshed_model":
                continue  # It has new proposals, not persistent track identities.
            _, matches, ignored = score(predictions, endpoint_truth, endpoint["ignore"], .3)
            visible_by_seed = {s["id"]: (i,s) for i,s in enumerate(visible)}
            trajectories[name] = []
            for si, s in enumerate(request["seeds"]):
                target = initial_truth[initial_matches[si]] if si in initial_matches else None
                present = s["id"] in visible_by_seed
                pi = visible_by_seed[s["id"]][0] if present else None
                matched = endpoint_truth[matches[pi]] if pi in matches else None
                opportunity = bool(target and target["id"] in end_by_id)
                retained = bool(target and matched and target["id"] == matched["id"])
                row = {"seed": s["id"], "initial_correct": target is not None, "initial_ignored": si in initial_ignored,
                       "initial_truth_id": target["id"] if target else None,
                       "survived": present, "endpoint_truth_id": matched["id"] if matched else None,
                       "endpoint_ignored": pi in ignored, "opportunity": opportunity, "retained": retained,
                       "switched": bool(target and matched and target["id"] != matched["id"])}
                if opportunity:
                    expected = delta(target["box"],end_by_id[target["id"]]["box"])
                    row.update(expected_displacement_px=expected, stationary=math.hypot(*expected)<.01)
                    if retained:
                        predicted = delta(initial_boxes[si],predictions[pi])
                        row.update(predicted_displacement_px=predicted, error_px=math.dist(expected,predicted))
                trajectories[name].append(row)
        for step in receipt["steps"]:
            for name in timings:
                if request["seeds"]:
                    timings[name].append(step[name+"_ms"])
            for s in step["gated"]:
                if s["reason"] and s["reason"] != "track_lost":
                    rejections[s["reason"]] += 1
        results.append({**job, "receipt_sha256": sha(receipt_path), "seeds": len(request["seeds"]),
                        "initial_scores": initial_counts, "scores": scores, "trajectories": trajectories})
        print(json.dumps({"completed": len(results), "of": len(jobs), "clip": clip, "start": job["start"]}), flush=True)
    totals = {name: {str(t): dict(sum((Counter(r["scores"][name][str(t)]) for r in results),Counter()))
                    for t in [.3,.5]} for name in variants}
    tracking = {}
    for name in ["hold","gated","ungated"]:
        rows = [row for r in results for row in r["trajectories"][name]]
        summary = {key: sum(r[key] for r in rows) for key in ["initial_correct","initial_ignored","survived","opportunity","retained","switched"]}
        summary["initial_false_survived"] = sum(not r["initial_correct"] and not r["initial_ignored"] and r["survived"] for r in rows)
        summary["motion"] = {}
        for label, stationary in [("stationary",True),("moving",False)]:
            group = [r for r in rows if r["opportunity"] and r["stationary"] == stationary]
            accepted = [r for r in group if r["retained"]]
            summary["motion"][label] = {"opportunities": len(group),"retained":len(accepted),
                "mean_error_px":statistics.mean(r["error_px"] for r in accepted) if accepted else None,
                "median_error_px":statistics.median(r["error_px"] for r in accepted) if accepted else None}
        tracking[name] = summary
    summary = {"windows":len(jobs),"zero_seed_windows":sum(r["seeds"]==0 for r in results),
               "seeds":sum(r["seeds"] for r in results), "totals":totals,"tracking":tracking,
               "gated_first_rejections":dict(rejections),"seeded_step_timings_ms":{
                   name:{"count":len(values),"median":statistics.median(values),"max":max(values)} for name,values in timings.items()}}
    save(out/"results.json", {"scope":"Previously examined clips; 48 one-second windows, not unseen games. Annotation units are 320x180 with roughly +/-2px uncertainty. Endpoint retention cannot establish intervening identity. False seeds remain false regardless of smooth tracking. Timing excludes acquisition and model seed latency.",
                             "frozen_manifest_sha256":sha(out/"frozen-manifest.json"),"parameters":receipt["parameters"],
                             "summary":summary,"records":results})
    print(json.dumps(summary),flush=True)


if __name__ == "__main__":
    main()
