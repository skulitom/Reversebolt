"""Post-hoc evaluation only. Never imported by the image-only observer.

Run from the repository root with the evaluation environment. Reads local PNGs
only to verify hashes; all region annotations here are scoring inputs, not OCR
inputs. Keeps first twelve frames, since the development sequence also contains
an unrelated thirteenth paused-menu capture.
"""
import hashlib
import json
import math
from pathlib import Path
import re
import statistics
import sys

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = ROOT / "test-results/ocr-v6"


def number(text):
    text = text.strip().rstrip("%")
    if not re.fullmatch(r"[0-9.+-]+", text):
        return None
    try:
        value = float(text)
        return value if math.isfinite(value) else None
    except ValueError:
        return None


def in_region(word, rect):
    points = word["polygon"]
    x0, y0, x1, y1 = (min(p[0] for p in points), min(p[1] for p in points),
                      max(p[0] for p in points), max(p[1] for p in points))
    rx, ry, rw, rh = rect
    area = max(0, min(x1, rx+rw)-max(x0, rx)) * max(0, min(y1, ry+rh)-max(y0, ry))
    return area > 0.5 * min((x1-x0)*(y1-y0), rw*rh)


def main():
    output = Path(sys.argv[1])
    assert not output.exists(), "Preserve earlier scores."
    labels = json.loads((Path(__file__).with_name("labels.json")).read_text())
    frozen = json.loads((Path(__file__).with_name("evidence") / "protocol-freeze.json").read_text())
    actual = hashlib.sha256(Path(__file__).with_name("evaluate.py").read_bytes()).hexdigest()
    assert actual == frozen["python_sha256"].lower(), "Frozen evaluator changed."
    report = {"scope": "Selected fields, not exhaustive OCR accuracy or game discovery",
              "evaluator_sha256": actual, "protocol": frozen, "results": []}
    for run in labels["runs"]:
        prediction_path = ARTIFACTS / run["prediction"]
        prediction = json.loads(prediction_path.read_text(encoding="utf-8"))
        sequence = labels["sequences"][run["sequence"]]
        assert len(prediction["frames"]) >= 12
        outcomes = {k: 0 for k in ("correct", "wrong", "conflicting", "missing")}
        frames = []
        for i, frame in enumerate(prediction["frames"][:12]):
            image = ROOT / frame["path"]
            assert hashlib.sha256(image.read_bytes()).hexdigest() == frame["sha256"]
            rows = []
            for field in sequence["fields"]:
                expected = field["values"][i]
                words = [w for w in frame["words"] if in_region(w, field["bounds_xywh"])]
                values = sorted({number(w["text"]) for w in words if number(w["text"]) is not None})
                if values == [expected]:
                    outcome = "correct"
                elif len(values) > 1:
                    outcome = "conflicting"
                elif values:
                    outcome = "wrong"
                else:
                    outcome = "missing"
                outcomes[outcome] += 1
                rows.append({"field": field["name"], "expected": expected, "values": values,
                             "outcome": outcome, "raw_texts": [w["text"] for w in words]})
            extras = [w for w in frame["words"] if number(w["text"]) is not None
                      and not any(in_region(w, f["bounds_xywh"]) for f in sequence["fields"])]
            rows_entry = {"image": str(image.relative_to(ROOT)).replace("\\", "/"),
                          "sha256": frame["sha256"], "fields": rows, "unscored_numeric": extras}
            if "line_template" in sequence:
                expected_text = sequence["line_template"].format(sequence["fields"][0]["values"][i])
                rows_entry["full_line_exact"] = any(w["text"] == expected_text for w in frame["words"])
            frames.append(rows_entry)
        times = [f["elapsed_ms"] for f in prediction["frames"][:12]]
        result = {**run, **outcomes, "prediction_sha256": hashlib.sha256(prediction_path.read_bytes()).hexdigest(),
                  "first_ms": times[0], "warm_median_ms": statistics.median(times[1:]),
                  "warm_min_ms": min(times[1:]), "warm_max_ms": max(times[1:]), "frames": frames}
        if "line_template" in sequence:
            result["full_line_exact"] = sum(f["full_line_exact"] for f in frames)
        report["results"].append(result)
        print(json.dumps({k: v for k, v in result.items() if k != "frames"}))
    output.write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")


if __name__ == "__main__":
    main()
