"""Audit saved prospective evidence; never collect, fit, or change predictions.
Usage: python research/predictions/summarize.py <run-dir> <ocr-track-id> <new-report>
The OCR track is selected by inspection, so its score is not whole-image accuracy.
"""
import hashlib
import json
from pathlib import Path
import struct
import sys

run = Path(sys.argv[1])
track_id = int(sys.argv[2])
output = Path(sys.argv[3])
assert not output.exists(), "Preserve prior reports."
read = lambda name: json.loads((run / name).read_text(encoding="utf-8-sig"))
start = read("start-response/response.json")["state"]["live"]["prediction_check"]
live = read("fresh-submission.json")["state"]["live"]
end = live["prediction_check"]
assert end["id"] == start["id"] and end["after_frame"] == start["after_frame"]
assert len(start["candidates"]) == len(end["candidates"])
assert all(not c["samples"] for c in start["candidates"])
annotations = read("discovery-annotations.json") + read("fresh-annotations.json")
labels = {a["frame"]: a["value"] for a in annotations}
frames = {f["number"]: f for f in live["frames"]}
for number, frame in frames.items():
    pixels = (run / "fresh-images" / f"frame-{number:03}.png").read_bytes()
    assert hashlib.sha256(pixels).hexdigest() == frame["sha256"]
summaries = []
for before, candidate in zip(start["candidates"], end["candidates"]):
    for field in ["address", "chunk_address", "offset_in_chunk", "mapping"]:
        assert before[field] == candidate[field], (field, before, candidate)
    mapping = candidate["mapping"]
    for sample in candidate["samples"]:
        assert sample["frame"] > start["after_frame"]
        assert sample["image"] == frames[sample["frame"]]
        assert sample["read"] is not None and sample["bytes"] is not None
        value = struct.unpack({"I32": "<i", "F32": "<f", "F64": "<d"}[mapping["kind"]], bytes(sample["bytes"]))[0]
        assert mapping["slope"] * value + mapping["intercept"] == sample["predicted"]
        assert labels[sample["frame"]] == sample["observed"]
        outcome = "agrees" if abs(sample["predicted"] - sample["observed"]) <= end["tolerance"] else "contradicts"
        assert sample["outcome"] == outcome
    summaries.append({
        "address": candidate["address"], "mapping": mapping,
        "invalidated": candidate["invalidated"],
        "samples": [{k: s[k] for k in ["frame", "bytes", "predicted", "observed", "outcome"]} for s in candidate["samples"]],
    })
track = next(t for t in live["tracks"] if t["id"] == track_id)
score = {"correct": 0, "incorrect": 0, "missing": 0}
for number, expected in labels.items():
    value = track["samples"][number - 1]
    score["missing" if value is None else "correct" if value == expected else "incorrect"] += 1
report = {
    "schema_version": 1, "freeze": read("freeze.json"),
    "scope": "One Blue Prince run. Agent image interpretations are provisional. No game writes or verified controls.",
    "check_id": end["id"], "after_frame": end["after_frame"],
    "source": end["source"], "tolerance": end["tolerance"],
    "observations": annotations, "frames": list(frames.values()),
    "ocr_selected_track": {"id": track_id, "values": track["samples"], "score": score},
    "phase": live["phase"], "candidates": summaries,
    "audit": "All image hashes, original mappings, fresh byte-derived predictions, submitted readings and per-frame outcomes checked.",
    "limitations": ["Initial full memory snapshots are transient; this audit checks the frozen mappings, not a refit of the discovery interval.", "Image and memory acquisition are sequential, not atomic.", "Matching values cannot establish causal authority, representation or allocation continuity."],
}
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(report, indent=2), encoding="utf-8")
print(json.dumps({"candidates": len(summaries), "phase": live["phase"], "ocr": score, "output": str(output)}))
