"""Automated native report test using the owned WinForms fixture, without game input.
Usage: python scripts/automatic-report-smoke.py <data-dir> <control-file> <fixture-pid>
Launch the engine and observation-fixture.ps1 in the same Windows desktop first.
"""
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time
import urllib.request

data, control, pid = Path(sys.argv[1]), Path(sys.argv[2]), int(sys.argv[3])
endpoint = json.loads((data / "endpoint.json").read_text(encoding="utf-8-sig"))
root = Path(__file__).resolve().parents[1]
binary = Path(os.environ.get("REVERSEBOLT_BIN_DIR", root / "target/release"))
process = subprocess.Popen([str(binary / "reversebolt-mcp.exe"), "--data-dir", str(data)],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    text=True, encoding="utf-8", creationflags=0x08000000)
lines = queue.Queue()
threading.Thread(target=lambda: [lines.put(line) for line in process.stdout], daemon=True).start()
sequence = 0

def rpc(method, params):
    global sequence
    sequence += 1
    process.stdin.write(json.dumps({"jsonrpc":"2.0", "id":sequence, "method":method, "params":params}) + "\n")
    process.stdin.flush()
    while True:
        result = json.loads(lines.get(timeout=120))
        if result.get("id") == sequence:
            assert "error" not in result, result
            return result["result"]

def call(action, args=None):
    result = rpc("tools/call", {"name":"reversebolt_"+action, "arguments":args or {}})
    assert not result.get("isError"), result
    value = json.loads(next(c["text"] for c in result["content"] if c["type"] == "text"))
    assert value["ok"], value.get("error")
    return value

def status():
    request = urllib.request.Request(endpoint["url"] + "/command",
        data=b'{"action":"status","args":{}}',
        headers={"Content-Type":"application/json", "Authorization":"Bearer "+endpoint["token"]})
    with urllib.request.urlopen(request, timeout=10) as response:
        return json.load(response)

def wait_until(predicate, seconds=120):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        state = status()
        if predicate(state):
            return state
        time.sleep(0.05)
    raise AssertionError("Timed out waiting for bounded native test.")

def current_job(state, job_id):
    return state["jobs"]["history"]["jobs"][str(job_id)]

def job_control(operation, job_id):
    history = status()["jobs"]["history"]
    return call("job_control", {"request_id":history["clients"]["mcp-agent"]["high_water"]+1,
        "boot":history["boot"], "binding":history["current"], "job":job_id, "operation":operation})

report = {"scope":"Owned WinForms fixture. Automatic connection/report and interruption; no game understanding claim."}
try:
    rpc("initialize", {"protocolVersion":"2025-11-25", "capabilities":{},
        "clientInfo":{"name":"automatic-report-check", "version":"1"}})
    process.stdin.write(json.dumps({"jsonrpc":"2.0", "method":"notifications/initialized"})+"\n")
    process.stdin.flush()
    target = next(p for p in call("list_processes")["state"]["processes"] if p["pid"] == pid)
    identity = {k:target[k] for k in ["pid", "created", "window"]}
    values = [40,56,31,85,64,27,75,92,44,67,38,81,23,59,72,46]
    control.write_text(str(values[0]))
    time.sleep(0.2)
    began = time.monotonic()
    attached = call("attach_process", {**identity, "budget_mib":4})
    job_id = attached["data"]["automatic_observation"]["ack"]["job"]
    assert len(current_job(attached,job_id)["steps"]) == 16
    assert attached["state"]["live"]["sample_count"] == 0
    report["attach_ms"] = round((time.monotonic()-began)*1000)
    captures=[]
    for index, expected in enumerate(values):
        state = wait_until(lambda s:s["state"]["live"]["sample_count"] >= index+1)
        live=state["state"]["live"]
        assert live["sample_count"] == index+1, "Test driver missed a capture; do not infer its value."
        captures.append({"frame":index+1,"expected":expected,"phase":live["phase"],
            "elapsed_ms":live["latest"]["elapsed_ms"]})
        if index+1 < len(values):
            control.write_text(str(values[index+1]))
    finished=wait_until(lambda s:current_job(s,job_id)["phase"] == "Completed")
    live=finished["state"]["live"]
    assert any(t["samples"] == values for t in live["tracks"]), live["tracks"]
    assert live["phase"].startswith("Compared "), live["phase"]
    assert live["coverage"]["analyzed_current_bytes"] > 0
    assert len(live["frames"]) == 16
    assert all(c["outcome"]["success"] for c in current_job(finished,job_id)["checkpoints"])
    time.sleep(1.3)
    assert status()["state"]["live"]["sample_count"] == 16
    (data/"automatic-completed.json").write_text(json.dumps(finished,indent=2))
    report["automatic_report"]={"captures":captures,"phase":live["phase"],"job":job_id,
        "correct_visible_values":16,"seconds":round(time.monotonic()-began,2),
        "capture_or_start_commands_from_driver":0,"stopped_at_budget":True}
    call("detach_process")
    second=call("attach_process", {**identity,"budget_mib":4})
    second_id=second["data"]["automatic_observation"]["ack"]["job"]
    wait_until(lambda s:s["state"]["live"]["sample_count"]>=1)
    job_control("pause",second_id)
    paused=wait_until(lambda s:current_job(s,second_id)["phase"]=="Paused")
    held=paused["state"]["live"]["sample_count"]
    time.sleep(1.3)
    assert status()["state"]["live"]["sample_count"]==held
    job_control("resume",second_id)
    wait_until(lambda s:s["state"]["live"]["sample_count"]>held)
    job_control("cancel",second_id)
    stopped=wait_until(lambda s:current_job(s,second_id)["phase"]=="Cancelled")
    stopped_count=stopped["state"]["live"]["sample_count"]
    time.sleep(1.3)
    assert status()["state"]["live"]["sample_count"]==stopped_count<16
    report["controls"]={"paused_at":held,"cancelled_at":stopped_count,"resume":"passed","no_restart_after_stop":True}
    call("detach_process")
    manual=call("attach_process",{**identity,"budget_mib":4,"auto_observe":False})
    time.sleep(1.3)
    assert status()["state"]["live"]["sample_count"]==0
    assert not any(j["phase"] in ["Running","Queued"] for j in manual["jobs"]["history"]["jobs"].values())
    report["manual_opt_out"]="no automatic work"
    call("detach_process")
    (data/"automatic-report.json").write_text(json.dumps(report,indent=2))
    print(json.dumps({"automatic_captures":16,"phase":live["phase"],"controls":report["controls"],"report":str(data/"automatic-report.json")}))
finally:
    process.stdin.close()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.terminate()
        process.wait(timeout=5)
