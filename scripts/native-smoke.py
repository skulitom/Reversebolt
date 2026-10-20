"""Run after launching the app and observation-fixture.ps1 in Anode.

python scripts/native-smoke.py <isolated-data-directory> <control-file> <fixture-pid>
This evaluates Windows plumbing and OCR, not game understanding.
"""
import json
from pathlib import Path
import sys
import time
import urllib.request

data, control, pid = Path(sys.argv[1]), Path(sys.argv[2]), int(sys.argv[3])
endpoint = json.loads((data / "endpoint.json").read_text())

def call(action, args=None):
    request = urllib.request.Request(endpoint["url"] + "/command",
        data=json.dumps({"action": action, "args": args or {}}).encode(),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + endpoint["token"]})
    with urllib.request.urlopen(request, timeout=120) as response:
        return json.load(response)

def require(action, args=None):
    result = call(action, args)
    assert result["ok"], result.get("error")
    return result["state"]

windows = require("list_processes")["processes"]
target = next(p for p in windows if p["pid"] == pid and p["title"] == "Reversebolt observation fixture")
identity = {k: target[k] for k in ("pid", "created", "window")}
assert not call("attach_process", {**identity, "created": "stale"})["ok"]
live = require("attach_process", {**identity, "budget_mib": 4})["live"]
assert live["process"]["pid"] == pid and live["sample_count"] == 0
assert not call("attach_process", {**identity, "window": 1})["ok"]
assert require("status")["live"]["process"] == live["process"]
assert not call("correlate")["ok"]
captures = []
for value in [40, 56, 31, 85, 64, 27, 75, 92, 44, 67, 38, 81]:
    control.write_text(str(value))
    time.sleep(0.25)
    live = require("capture")["live"]
    latest = live["latest"]
    assert latest["captured_bytes"] > 0
    screen = latest["screen"]
    assert screen and not latest["screen_error"], latest
    assert str(value) in screen["text"].split(), screen["text"]
    assert Path(screen["path"]).exists()
    captures.append({"value": value, "text": screen["text"], "bytes": latest["captured_bytes"],
                     "ms": latest["elapsed_ms"], "image": screen["path"]})
correlated = require("correlate")["live"]
assert correlated["sample_count"] == 12
assert any(len([v for v in track["samples"] if v is not None]) == 12 for track in correlated["tracks"])
require("detach_process")
assert require("status")["live"] is None
assert any(p["pid"] == pid for p in require("list_processes")["processes"]), "Detach terminated target"
require("attach_process", identity)
control.write_text("close")
for _ in range(60):
    state = require("status")
    if not state["live"]["connected"]:
        break
    time.sleep(0.1)
assert not state["live"]["connected"], "Exited target still marked connected"
assert not call("capture")["ok"]
assert not call("attach_process", identity)["ok"]
require("detach_process")
report = {
    "scope": "Owned WinForms fixture; native transport and OCR only, not independent-game generalization",
    "stale_identity": "rejected", "wrong_window": "rejected; original connection preserved",
    "disconnect": "left target running", "process_exit": "connection invalidated automatically",
    "captures": captures, "tracks": correlated["tracks"], "phase": correlated["phase"],
}
(data / "native-report.json").write_text(json.dumps(report, indent=2))
print(json.dumps({k: v for k, v in report.items() if k not in ("captures", "tracks")}, indent=2))
print("Evidence:", data / "native-report.json")
