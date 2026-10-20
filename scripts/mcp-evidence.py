"""Exercise a real stdio MCP request against an already-running isolated engine.

Usage: python scripts/mcp-evidence.py <data-dir> <request.json> <output-dir>
Request: {"tool":"reversebolt_read_capture", "arguments":{...}}
Image content is decoded locally and its bytes checked against the engine's
frame hash. No model or expected gameplay values are supplied by this helper.
"""
import base64
import hashlib
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading

data, request_file, output = map(Path, sys.argv[1:])
request = json.loads(request_file.read_text(encoding="utf-8-sig"))
root = Path(__file__).resolve().parents[1]
binary = Path(os.environ.get("REVERSEBOLT_BIN_DIR", root / "target" / "release"))
output.mkdir(parents=True, exist_ok=True)
process = subprocess.Popen([str(binary / "reversebolt-mcp.exe"), "--data-dir", str(data)],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    text=True, encoding="utf-8", creationflags=0x08000000)
lines = queue.Queue()
threading.Thread(target=lambda: [lines.put(line) for line in process.stdout], daemon=True).start()

def send(message):
    process.stdin.write(json.dumps(message) + "\n")
    process.stdin.flush()

def receive(request_id):
    while True:
        value = json.loads(lines.get(timeout=120))
        if value.get("id") == request_id:
            assert "error" not in value, value
            return value["result"]

try:
    send({"jsonrpc":"2.0", "id":1, "method":"initialize", "params":{
        "protocolVersion":"2025-11-25", "capabilities":{},
        "clientInfo":{"name":"reversebolt-evidence-check", "version":"1"}}})
    receive(1)
    send({"jsonrpc":"2.0", "method":"notifications/initialized"})
    send({"jsonrpc":"2.0", "id":2, "method":"tools/call", "params":{
        "name":request["tool"], "arguments":request.get("arguments",{})}})
    result = receive(2)
    text_blocks = [json.loads(c["text"]) for c in result["content"] if c["type"] == "text"]
    state = text_blocks[0]
    images = []
    for i, content in enumerate(result["content"]):
        if content["type"] == "image":
            assert content["mimeType"] == "image/png"
            pixels = base64.b64decode(content["data"], validate=True)
            assert hashlib.sha256(pixels).hexdigest() == state["data"]["frame"]["sha256"]
            image = output / f"image-{i}.png"
            image.write_bytes(pixels)
            images.append(str(image.resolve()))
    (output / "response.json").write_text(json.dumps(state, indent=2), encoding="utf-8")
    print(json.dumps({"ok":state["ok"], "error":state.get("error"), "images":images,
        "capture":state.get("data",{}), "series":len((state.get("state",{}).get("live") or {}).get("visual_tracks",[]))}, indent=2))
    if result.get("isError"):
        sys.exit(1)
finally:
    process.stdin.close()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.terminate()
        process.wait(timeout=5)
