"""Summarize saved native evidence only; never calls or labels the observer.
Usage: python summarize_native.py <native-results-directory>
"""
import hashlib
import json
from pathlib import Path
import statistics
import sys

root = Path(sys.argv[1])
repository = Path(__file__).resolve().parents[2]
freeze = json.loads((root / "native-freeze.json").read_text(encoding="utf-8"))
for record in freeze["files"]:
    assert hashlib.sha256((repository / record["path"]).read_bytes()).hexdigest().upper() == record["sha256"]
captures = [json.loads((root / "live-blue" / f"capture-{i:02}.json").read_text(encoding="utf-8"))["state"]["live"] for i in range(1,13)]
replay = [{"screen": c["latest"]["screen"], "tracks": c["tracks"]} for c in captures]
with (root / "blue-live-native.json").open("x", encoding="utf-8") as output:
    json.dump(replay, output, indent=2)
correlation = json.loads((root / "live-blue" / "mcp-correlation.json").read_text(encoding="utf-8"))
result = json.loads(next(c["text"] for c in correlation["correlation"]["content"] if c["type"] == "text"))
assert result["ok"], result["error"]
live = result["state"]["live"]
summary = {"freeze_verified": True, "frozen_at": freeze["frozen_at"],
    "input": "Full images and bounded memory; no evaluation values or regions supplied",
    "capture_count": len(captures), "phase": live["phase"],
    "eligible_bytes": live["eligible_bytes"], "coverage": live["coverage"],
    "correlation_ms": correlation["elapsed_ms"],
    "tracks": [{"id": t["id"], "label": t["label"], "samples": t["samples"],
        "hypotheses": len(t["matches"]), "exact_matches": sum(m["mapping"]["exact"] for m in t["matches"])} for t in live["tracks"]],
    "timing_ms": {}, "frames": []}
for key in ["elapsed_ms", "recognition_ms", "screen_capture_ms", "memory_read_ms"]:
    timings = [c["latest"][key] for c in captures]
    summary["timing_ms"][key] = {"min": min(timings), "median": statistics.median(timings), "max": max(timings)}
for c in captures:
    latest = c["latest"]
    assert latest["screen_error"] is None and latest["screen"]["warning"] is None
    summary["frames"].append({"number": latest["number"],
        "image_sha256": hashlib.sha256(Path(latest["screen"]["path"]).read_bytes()).hexdigest(),
        "sources": sorted({w["source"] for w in latest["screen"]["words"]})})
with (root / "blue-live-summary.json").open("x", encoding="utf-8") as output:
    json.dump(summary, output, indent=2)
print(json.dumps({k:v for k,v in summary.items() if k not in ["frames", "tracks"]}, indent=2))
