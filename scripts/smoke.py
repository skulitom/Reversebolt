"""Exercise actual process memory, the shared HTTP engine, and stdio MCP."""
import json
import os
from pathlib import Path
import queue
import subprocess
import threading
import time
import urllib.error
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[1]
BIN = Path(os.environ.get("REVERSEBOLT_BIN_DIR", ROOT / "target" / "debug"))
DATA = ROOT / "test-results" / ("smoke-" + uuid.uuid4().hex[:8])
DATA.mkdir(parents=True)
CREATE_NO_WINDOW = 0x08000000
engine = None
mcp = None

def start_engine():
    process = subprocess.Popen([str(BIN / "Reversebolt.exe"), "--headless", "--data-dir", str(DATA)],
                               creationflags=CREATE_NO_WINDOW)
    for _ in range(200):
        if process.poll() is not None:
            raise AssertionError("Engine failed to start")
        if (DATA / "endpoint.json").exists():
            endpoint = json.loads((DATA / "endpoint.json").read_text())
            if endpoint["pid"] == process.pid:
                return process, endpoint
        time.sleep(0.05)
    raise AssertionError("Engine startup timed out")

def call(action, args=None, token=True, origin=None):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = "Bearer " + endpoint["token"]
    if origin:
        headers["Origin"] = origin
    req = urllib.request.Request(endpoint["url"] + "/command",
        data=json.dumps({"action": action, "args": args or {}}).encode(), headers=headers)
    with urllib.request.urlopen(req, timeout=90) as response:
        return json.load(response)

def require(action, args=None):
    result = call(action,args)
    assert result["ok"], result
    return result["state"]

def send(message):
    mcp.stdin.write(json.dumps(message) + "\n")
    mcp.stdin.flush()

def response(request_id):
    deadline = time.monotonic()+120
    while time.monotonic()<deadline:
        line = lines.get(timeout=max(0.1,deadline-time.monotonic()))
        value = json.loads(line)
        if value.get("id")==request_id:
            assert "error" not in value, value
            return value["result"]
    raise AssertionError("MCP response timed out")

try:
    engine, endpoint = start_engine()
    for token, origin, status in [(False,None,401),(True,"https://example.org",403)]:
        try:
            call("status",token=token,origin=origin)
            raise AssertionError("Unauthenticated/browser request accepted")
        except urllib.error.HTTPError as error:
            assert error.code==status

    rejected = call("set_value",{"track":0,"value":100})
    assert not rejected["ok"]
    reports = []
    for seed in [8675309,1001,20261020]:
        require("open_practice",{"seed":seed})
        assert not call("set_value",{"track":0,"value":100})["ok"]
        require("observe")
        hypothesis = require("discover")
        assert all(len(t["matches"])>=3 for t in hypothesis["tracks"])
        assert not call("set_value",{"track":0,"value":100})["ok"]
        state = require("test_candidates")
        assert all(sum(m["passed"] for m in t["matches"])==1 for t in state["tracks"]), state
        assert all(any(m["tested"] and not m["passed"] for m in t["matches"]) for t in state["tracks"])
        before = [t["value"] for t in state["tracks"]]
        for track,value in [(0,100),(1,50)]:
            changed = require("set_value",{"track":track,"value":value})
            assert abs(changed["tracks"][track]["value"]-value)<0.6
            restored = require("undo")
            assert [t["value"] for t in restored["tracks"]]==before
        reports.append({"seed":seed,"matches":[len(t["matches"]) for t in state["tracks"]],
                        "unique_controls":2,"edit_restore":"passed"})
    # A real MCP client lifecycle joins the already-running HTTP/desktop session.
    mcp = subprocess.Popen([str(BIN/"reversebolt-mcp.exe"),"--data-dir",str(DATA)],
        stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,
        encoding="utf-8",creationflags=CREATE_NO_WINDOW)
    lines = queue.Queue()
    threading.Thread(target=lambda: [lines.put(line) for line in mcp.stdout],daemon=True).start()
    send({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"reversebolt-smoke","version":"1"}}})
    initialize = response(1)
    assert initialize["serverInfo"]["name"]=="reversebolt"
    send({"jsonrpc":"2.0","method":"notifications/initialized"})
    send({"jsonrpc":"2.0","id":2,"method":"tools/list"})
    listed = response(2)
    assert len(listed["tools"])==33
    assert {"reversebolt_list_processes", "reversebolt_attach_process",
            "reversebolt_capture", "reversebolt_correlate",
            "reversebolt_detach_process", "reversebolt_read_capture",
            "reversebolt_submit_observation", "reversebolt_advance_memory", "reversebolt_start_observation_job",
            "reversebolt_job_control", "reversebolt_job_events", "reversebolt_start_prediction_check", "reversebolt_read_sequence", "reversebolt_submit_temporal_observation", "reversebolt_set_observer_model"} <= {tool["name"] for tool in listed["tools"]}
    send({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
        "name":"reversebolt_set_value","arguments":{"track":0,"value":111}}})
    edited = response(3)
    assert not edited.get("isError")
    state = require("status")
    assert abs(state["tracks"][0]["value"]-111)<0.6
    assert state["history"][-1]["source"]=="agent"
    require("undo")
    # The value changes after this patch, so undo must refuse to overwrite it.
    require("set_value",{"track":0,"value":123})
    require("observe")
    assert not call("undo")["ok"]
    # Restart invalidates active locations while preserving the audit history.
    old_count = len(require("status")["history"])
    require("close_target")
    engine.terminate()
    engine.wait(timeout=10)
    engine, endpoint = start_engine()
    state = require("status")
    assert not state["connected"] and not state["tracks"]
    assert len(state["history"])>=old_count
    assert not call("undo")["ok"]
    report = {"practice":reports,"mcp_tools":len(listed["tools"]),"shared_session":"passed",
              "auth_and_origin":"passed","stale_restore":"rejected","restart":"history retained; controls invalidated"}
    (DATA/"report.json").write_text(json.dumps(report,indent=2))
    print(json.dumps(report,indent=2))
    print("Evidence:",DATA/"report.json")
finally:
    if mcp is not None:
        mcp.terminate()
        mcp.wait(timeout=10)
    if engine is not None:
        try:
            call("close_target")
        except Exception:
            pass
        engine.terminate()
        engine.wait(timeout=10)
