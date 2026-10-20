"""Automated shared-engine/MCP image-sequence contract; no semantic model claim.
Start appearance-fixture.ps1 and an isolated Reversebolt engine on the same
desktop, then pass DATA_DIR CONTROL_FILE FIXTURE_PID. No game input is used.
"""
import base64
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time
from PIL import Image, ImageChops

data, control, pid = Path(sys.argv[1]), Path(sys.argv[2]), int(sys.argv[3])
root = Path(__file__).resolve().parents[1]
binary = Path(os.environ.get("REVERSEBOLT_BIN_DIR", root / "target/release"))
process = subprocess.Popen([str(binary / "reversebolt-mcp.exe"), "--data-dir", str(data)],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    text=True, encoding="utf-8", creationflags=0x08000000)
lines = queue.Queue()
threading.Thread(target=lambda: [lines.put(line) for line in process.stdout], daemon=True).start()
counter = 0

def rpc(method, params):
    global counter
    counter += 1
    process.stdin.write(json.dumps({"jsonrpc":"2.0", "id":counter, "method":method, "params":params}) + "\n")
    process.stdin.flush()
    while True:
        value = json.loads(lines.get(timeout=120))
        if value.get("id") == counter:
            assert "error" not in value, value
            return value["result"]

def call(action, args=None, error=False):
    result = rpc("tools/call", {"name":"reversebolt_" + action, "arguments":args or {}})
    assert bool(result.get("isError")) == error, result
    # Sequence returns frame labels + images, followed by the shared state JSON.
    payload = json.loads(result["content"][-1]["text"])
    assert payload["ok"] != error, payload.get("error")
    return payload, result

try:
    rpc("initialize", {"protocolVersion":"2025-11-25", "capabilities":{},
        "clientInfo":{"name":"temporal-native-check", "version":"1"}})
    process.stdin.write(json.dumps({"jsonrpc":"2.0", "method":"notifications/initialized"}) + "\n")
    process.stdin.flush()
    targets, _ = call("list_processes")
    target = next(p for p in targets["state"]["processes"] if p["pid"] == pid)
    identity = {k:target[k] for k in ["pid", "created", "window"]}
    call("attach_process", {**identity, "budget_mib":1, "auto_observe":False})
    for command in ["round", "wide", "round"]:
        control.write_text(command)
        time.sleep(.2)
        captured, _ = call("capture")
    live = captured["state"]["live"]
    assert len(live["frames"]) == 3
    image_time = live["frames"][-1]["capture_interval"]
    for read in live["latest"]["memory_reads"]:
        read_time = read["capture_interval"]
        assert read_time["clock_id"] == image_time["clock_id"]
        assert image_time["end_us"] <= read_time["begin_us"] <= read_time["end_us"]
    ids = [f["id"] for f in live["frames"]]
    request = {"investigation_id":live["investigation_id"], "frame_ids":ids}
    payload, result = call("read_sequence", request)
    sequence = payload["data"]["sequence"]
    images = [base64.b64decode(c["data"], validate=True) for c in result["content"] if c["type"] == "image"]
    assert len(images) == 3
    for image, frame in zip(images, sequence["frames"]):
        assert hashlib.sha256(image).hexdigest() == frame["sha256"]
    decoded = [Image.open(io.BytesIO(image)).convert("RGB") for image in images]
    # Image-only difference; neither fixture command nor source layout determines value.
    changed = ImageChops.difference(decoded[0], decoded[1]).getbbox() is not None
    restored = ImageChops.difference(decoded[0], decoded[2]).getbbox() is None
    assert changed and restored
    assert sequence["clock_id"] == live["investigation_id"]
    assert sequence["elapsed_us"][0] <= sequence["elapsed_us"][1]
    assert sequence["omitted_moments"] == 0
    declaration = {"investigation_id":live["investigation_id"], "batch":live["coverage"]["batch"],
        "observer":"automated image difference; no semantic model", "key":"appearance-change",
        "label":"Appearance changed", "unit":"", "basis":"appearance",
        "frames":[{"frame_id":f["id"], "image_sha256":f["sha256"], "bounds":None} for f in sequence["frames"]],
        "value":{"kind":"boolean", "value":changed}, "quality":"provisional",
        "explanation":"Decoded pixels differ between the first two sampled images and return to the first image in the third. This does not identify an object, cause, or continuous animation."}
    recorded, _ = call("submit_temporal_observation", declaration)
    assert recorded["data"]["recorded"]
    repeated, _ = call("submit_temporal_observation", declaration)
    assert repeated["data"]["duplicate"]
    conflict = copy.deepcopy(declaration)
    conflict["value"]["value"] = False
    call("submit_temporal_observation", conflict, error=True)
    bad = copy.deepcopy(declaration)
    bad["frames"][0]["image_sha256"] = "0" * 64
    call("submit_temporal_observation", bad, error=True)
    call("read_sequence", {**request, "frame_ids":ids[::-1]}, error=True)
    call("read_sequence", {**request, "frame_ids":[ids[0], ids[0]]}, error=True)
    call("submit_temporal_observation", declaration)  # Valid retry clears the displayed test error.
    final, _ = call("status")
    assert len(final["state"]["live"]["temporal_tracks"]) == 1
    assert not final["state"]["patches"]
    (data / "temporal-state.json").write_text(json.dumps(final, indent=2), encoding="utf-8")
    report = {"scope":"Owned native appearance fixture, no gameplay or semantic-recognition evidence",
        "images":len(images), "hashes_verified":True, "image_only_difference":changed,
        "image_only_return":restored, "timing":sequence, "round_trip":"recorded, replay idempotent",
        "rejections":["conflicting replacement", "wrong hash", "reversed frames", "duplicate frames"],
        "game_inputs":0, "manual_screenshots":0, "edits":0}
    (data / "temporal-report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({"images":len(images), "checks":"passed", "report":str(data / "temporal-report.json")}))
    # Leave the owned connection available for a bounded UI check; caller detaches.
finally:
    process.stdin.close()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.terminate()
        process.wait(timeout=5)
