"""Check packaged engine/MCP recovery dispatch with no application attached."""
import hashlib
import json
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time
import uuid

binary, output = [Path(p).resolve() for p in sys.argv[1:]]
output.mkdir()
data = output / "data"
engine = mcp = None
cleanup = []
try:
    with (output / "engine.log").open("w") as log:
        engine = subprocess.Popen([str(binary / "Reversebolt.exe"), "--headless", "--data-dir", str(data)],
                                  stdout=log, stderr=subprocess.STDOUT, creationflags=subprocess.CREATE_NO_WINDOW)
    for _ in range(200):
        assert engine.poll() is None, "Packaged engine exited during startup"
        if (data / "endpoint.json").exists():
            endpoint = json.loads((data / "endpoint.json").read_text())
            if endpoint["pid"] == engine.pid:
                break
        time.sleep(.05)
    else:
        raise AssertionError("Engine startup timed out")
    mcp = subprocess.Popen([str(binary / "reversebolt-mcp.exe"), "--data-dir", str(data)],
                           stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                           text=True, encoding="utf-8", creationflags=subprocess.CREATE_NO_WINDOW)
    lines = queue.Queue()
    threading.Thread(target=lambda: [lines.put(line) for line in mcp.stdout], daemon=True).start()
    counter = 0
    def rpc(method, params):
        global counter
        counter += 1
        mcp.stdin.write(json.dumps({"jsonrpc":"2.0", "id":counter, "method":method, "params":params}) + "\n")
        mcp.stdin.flush()
        while True:
            value = json.loads(lines.get(timeout=30))
            if value.get("id") == counter:
                assert "error" not in value, value
                return value["result"]
    rpc("initialize", {"protocolVersion":"2025-11-25", "capabilities":{},
                       "clientInfo":{"name":"reversebolt-recovery-smoke", "version":"1"}})
    mcp.stdin.write(json.dumps({"jsonrpc":"2.0", "method":"notifications/initialized"}) + "\n")
    mcp.stdin.flush()
    def call(name, args):
        result = rpc("tools/call", {"name":"reversebolt_" + name, "arguments":args})
        return json.loads(next(c["text"] for c in result["content"] if c["type"] == "text"))
    before = call("status", {})
    assert before["ok"] and before["state"]["live"] is None
    rejected = call("recover_runtime_control", {"test_id":str(uuid.uuid4()), "catalog_id":str(uuid.uuid4()), "after_revision":0})
    assert not rejected["ok"] and "Connect an application first" in rejected["error"], rejected
    after = call("status", {})
    assert after["ok"] and after["state"]["live"] is None
    assert not after["state"]["runtime_tests"] and not after["state"]["runtime_trials"] and not after["state"]["freezes"]
    (output / "dispatch.json").write_text(json.dumps({"rejected":rejected, "after":after}, indent=2), encoding="utf-8")
    mcp.stdin.close()
    assert mcp.wait(timeout=15) == 0
    summary = {"passed":True, "target_attached":False, "model_calls":0, "writes":0,
               "missing_application_rejected":True,
               "binaries":{name:hashlib.sha256((binary / name).read_bytes()).hexdigest()
                           for name in ("Reversebolt.exe", "reversebolt-mcp.exe")}}
finally:
    for process in (mcp, engine):
        if process is not None:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=15)
            cleanup.append({"pid":process.pid, "exit_code":process.returncode})
    (output / "cleanup.json").write_text(json.dumps(cleanup, indent=2), encoding="utf-8")
summary["cleanup"] = cleanup
(output / "result.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
print(json.dumps(summary))
