"""Collect raw evidence from an isolated engine; no recognition labels are inputs."""
import json
from pathlib import Path
import sys
import urllib.request

data = Path(sys.argv[1])
endpoint = json.loads((data / "endpoint.json").read_text(encoding="utf-8-sig"))

def call(action, args=None):
    request = urllib.request.Request(endpoint["url"] + "/command",
        data=json.dumps({"action": action, "args": args or {}}).encode(),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + endpoint["token"]})
    with urllib.request.urlopen(request, timeout=30) as response:
        result = json.load(response)
    assert result["ok"], result.get("error")
    return result

if len(sys.argv) > 2:
    program = next(p for p in call("list_processes")["state"]["processes"] if p["pid"] == int(sys.argv[2]))
    call("attach_process", {**{k: program[k] for k in ("pid", "created", "window")}, "budget_mib": 16})
else:
    result = call("capture")
    live = result["state"]["live"]
    number = live["sample_count"]
    output = data / f"capture-{number:02}.json"
    assert not output.exists()
    output.write_text(json.dumps(result), encoding="utf-8")
    print(json.dumps({"capture": number, "path": live["latest"]["screen"]["path"]}))
