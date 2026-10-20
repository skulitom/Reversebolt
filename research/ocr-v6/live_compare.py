"""Exercise real MCP correlation against an already captured isolated session.
Usage: python live_compare.py <mcp-exe> <isolated-data-dir>
No expected values or regions are supplied to the engine.
"""
import json
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time

exe, data = map(Path, sys.argv[1:])
output = data / "mcp-correlation.json"
assert not output.exists(), "Preserve earlier results."
child = subprocess.Popen([str(exe.resolve()), "--data-dir", str(data.resolve())],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    text=True, encoding="utf-8", creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
lines = queue.Queue()
threading.Thread(target=lambda: [lines.put(line) for line in child.stdout], daemon=True).start()

def send(message):
    child.stdin.write(json.dumps(message)+"\n")
    child.stdin.flush()

def receive(identifier):
    deadline = time.monotonic()+120
    while time.monotonic()<deadline:
        item = json.loads(lines.get(timeout=max(0.01,deadline-time.monotonic())))
        if item.get("id")==identifier:
            assert "error" not in item, item
            return item["result"]
    raise TimeoutError("MCP response timeout; inspect the same engine before retrying")

try:
    send({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "protocolVersion":"2025-11-25","capabilities":{},
        "clientInfo":{"name":"reversebolt-native-observer-check","version":"1"}}})
    assert receive(1)["serverInfo"]["name"]=="reversebolt"
    send({"jsonrpc":"2.0","method":"notifications/initialized"})
    send({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
        "name":"reversebolt_status","arguments":{}}})
    before = receive(2)
    assert not before.get("isError"), before
    began = time.monotonic()
    send({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
        "name":"reversebolt_correlate","arguments":{}}})
    correlated = receive(3)
    result = {"before":before,"correlation":correlated,"elapsed_ms":(time.monotonic()-began)*1000}
    output.write_text(json.dumps(result,indent=2),encoding="utf-8")
    assert not correlated.get("isError"), correlated
    state = json.loads(next(c["text"] for c in correlated["content"] if c["type"]=="text"))
    live = state["state"]["live"]
    print(json.dumps({"phase":live["phase"],"elapsed_ms":result["elapsed_ms"],
        "hypotheses":sum(len(t["matches"]) for t in live["tracks"]),"output":str(output)}))
finally:
    child.stdin.close()
    if child.poll() is None:
        child.terminate()
    child.wait(timeout=10)
