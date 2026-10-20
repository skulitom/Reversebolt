"""Score the native application's actual current readings, including abstentions.
Annotations are evaluation-only and never passed to the native observer.
Usage: python score_native.py <labels-key> <app-replay.json> <new-output.json>
"""
import hashlib
import json
from pathlib import Path
import sys
from score import in_region

key, prediction_path, output_path = sys.argv[1:]
output = Path(output_path)
assert not output.exists()
sequence = json.loads(Path(__file__).with_name("labels.json").read_text(encoding="utf-8"))["sequences"][key]
predictions = json.loads(Path(prediction_path).read_text(encoding="utf-8"))
result = {"sequence": key, "prediction_sha256": hashlib.sha256(Path(prediction_path).read_bytes()).hexdigest(),
    "correct": 0, "wrong": 0, "uncertain": 0, "missing": 0, "frames": []}
for i, prediction in enumerate(predictions[:12]):
    image = Path(prediction["screen"]["path"])
    record = {"image_sha256": hashlib.sha256(image.read_bytes()).hexdigest(), "fields": []}
    for field in sequence["fields"]:
        found = []
        for track in prediction["tracks"]:
            reading = track["readings"][-1]
            if reading is None:
                continue
            x,y,w,h = reading["bounds"]
            if in_region({"polygon": [[x,y],[x+w,y],[x+w,y+h],[x,y+h]]}, field["bounds_xywh"]):
                found.append(reading)
        expected = field["values"][i]
        values = {r["value"] for r in found}
        if not found:
            outcome = "missing"
        elif None in values or len(values) > 1:
            outcome = "uncertain"
        elif values == {expected}:
            outcome = "correct"
        else:
            outcome = "wrong"
        result[outcome] += 1
        record["fields"].append({"field": field["name"], "expected": expected,
            "outcome": outcome, "readings": found})
    result["frames"].append(record)
assert len(result["frames"]) == 12
output.write_text(json.dumps(result, indent=2), encoding="utf-8")
print(json.dumps({k:v for k,v in result.items() if k != "frames"}))
