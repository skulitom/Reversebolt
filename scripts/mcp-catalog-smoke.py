"""Check stdio tool metadata without an engine, target process, or credentials."""
import hashlib
import json
import queue
import subprocess
import sys
import threading
from pathlib import Path

binary, output = [Path(s).resolve() for s in sys.argv[1:]]
output.mkdir()  # Preserve previous runs.
process = subprocess.Popen(
    [str(binary), "--data-dir", str(output / "no-engine")],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    text=True, encoding="utf-8", creationflags=subprocess.CREATE_NO_WINDOW,
)
lines = queue.Queue()
threading.Thread(target=lambda: [lines.put(line) for line in process.stdout], daemon=True).start()

def send(message):
    process.stdin.write(json.dumps(message) + "\n")
    process.stdin.flush()

def response(request_id):
    while True:
        value = json.loads(lines.get(timeout=15))
        if value.get("id") == request_id:
            assert "error" not in value, value
            return value["result"]

try:
    send({"jsonrpc":"2.0", "id":1, "method":"initialize", "params":{
        "protocolVersion":"2025-11-25", "capabilities":{},
        "clientInfo":{"name":"reversebolt-catalog-check", "version":"1"}}})
    initialization = response(1)
    send({"jsonrpc":"2.0", "method":"notifications/initialized"})
    send({"jsonrpc":"2.0", "id":2, "method":"tools/list", "params":{}})
    result = response(2)
    (output / "tools.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    definitions = result["tools"]
    names = [t["name"] for t in definitions]
    assert len(names) == len(set(names))
    tool = next(t for t in definitions if t["name"] == "reversebolt_read_runtime_inventory")
    assert tool["annotations"]["readOnlyHint"] is True
    assert tool["inputSchema"]["required"] == ["catalog_id"]
    assert "object_samples" in tool["description"] and "context_coverage" in tool["description"]
    process.stdin.close()
    code = process.wait(timeout=15)
    assert code == 0
    summary = {"passed":True, "pid":process.pid, "exit_code":code,
               "tool_count":len(definitions), "protocol":initialization["protocolVersion"],
               "binary_sha256":hashlib.sha256(binary.read_bytes()).hexdigest(),
               "tools_sha256":hashlib.sha256((output / "tools.json").read_bytes()).hexdigest(),
               "engine_connected":False, "target_access":False}
    (output / "result.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary))
finally:
    if process.poll() is None:
        process.terminate()
        process.wait(timeout=15)
