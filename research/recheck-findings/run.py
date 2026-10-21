"""Owned-window probe integration. Run inside a leased Anode session only."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time

HERE = Path(__file__).resolve().parent
config = json.loads((HERE / "protocol-v1.json").read_text(encoding="utf-8"))
slot, out = int(sys.argv[1]), Path(sys.argv[2]).resolve()
assert slot in [0,1]
out.mkdir(parents=True, exist_ok=False)
binary = Path(config["binary_directory"])
for name, digest in config["binaries"].items():
    assert hashlib.sha256((binary / name).read_bytes()).hexdigest() == digest
for name, digest in config["method_sha256"].items():
    assert hashlib.sha256((HERE / name).read_bytes()).hexdigest() == digest
session = ctypes.c_ulong()
assert ctypes.windll.kernel32.ProcessIdToSessionId(os.getpid(), ctypes.byref(session))
assert session.value == int(os.environ["REVERSEBOLT_TEST_SESSION"]) != int(os.environ["REVERSEBOLT_PARENT_SESSION"])
control, oracle_file, data = out / "control.txt", out / "oracle.json", out / "engine"
control.write_text("40,72", encoding="ascii")
processes, commands, states = [], [], []
report = {"slot": slot, "errors": [], "protocol_sha256": hashlib.sha256((HERE / "protocol-v1.json").read_bytes()).hexdigest()}
serial, mcp, change_id, written, restored, restore_attempted = 0, None, None, False, False, False
lines = queue.Queue()


def save(name, value):
    (out / name).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def oracle():
    for _ in range(50):
        try:
            return json.loads(oracle_file.read_text(encoding="utf-8-sig"))
        except (OSError, json.JSONDecodeError):
            time.sleep(.02)
    raise RuntimeError("Fixture oracle unavailable")


def rpc(method, params):
    global serial
    serial += 1
    mcp.stdin.write(json.dumps({"jsonrpc": "2.0", "id": serial, "method": method, "params": params}) + "\n")
    mcp.stdin.flush()
    while True:
        response = json.loads(lines.get(timeout=120))
        if response.get("id") == serial:
            assert "error" not in response, response
            return response["result"]


def call(name, args=None, required=True):
    result = rpc("tools/call", {"name": "reversebolt_" + name, "arguments": args or {}})
    value = json.loads(next(c["text"] for c in result["content"] if c["type"] == "text"))
    if name != "status":
        commands.append({"name": name, "args": args or {}, "ok": value["ok"], "error": value.get("error")})
        save("commands.json", commands)
    if required:
        assert not result.get("isError") and value["ok"], value.get("error")
    return value




try:
    fixture = subprocess.Popen(["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(HERE / "fixture.ps1"),
        "-ControlFile", str(control), "-OracleFile", str(oracle_file)], creationflags=0x08000000)
    processes.append(fixture)
    deadline = time.time() + 25
    while not oracle_file.exists() and time.time() < deadline:
        assert fixture.poll() is None, "Fixture exited during startup"
        time.sleep(.1)
    report["fixture_pid"] = fixture.pid
    report["initial_oracle"] = oracle()
    engine = subprocess.Popen([str(binary / "Reversebolt.exe"), "--data-dir", str(data)], creationflags=0x08000000)
    processes.append(engine)
    deadline = time.time() + 20
    while not (data / "endpoint.json").exists() and time.time() < deadline:
        assert engine.poll() is None, "Engine exited during startup"
        time.sleep(.1)
    assert (data / "endpoint.json").exists()
    report["engine_pid"] = engine.pid
    mcp = subprocess.Popen([str(binary / "reversebolt-mcp.exe"), "--data-dir", str(data)], stdin=subprocess.PIPE,
        stdout=subprocess.PIPE, stderr=(out / "mcp-stderr.log").open("w", encoding="utf-8"), text=True, encoding="utf-8", creationflags=0x08000000)
    processes.append(mcp)
    report["mcp_pid"] = mcp.pid
    threading.Thread(target=lambda: [lines.put(line) for line in mcp.stdout], daemon=True).start()
    rpc("initialize", {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "location-probe-evaluation", "version": "1"}})
    mcp.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n'); mcp.stdin.flush()
    tools = rpc("tools/list", {})["tools"]
    assert len(tools) == 32
    for name in ["apply_change", "restore_change"]:
        assert next(t for t in tools if t["name"] == "reversebolt_" + name)["annotations"]["destructiveHint"]
    target = next(p for p in call("list_processes")["state"]["processes"] if p["pid"] == fixture.pid)
    call("set_observer_model", {"model": None})
    attached=call("attach_process", {**{k:target[k] for k in ["pid","created","window"]}, "budget_mib":32})
    job=attached["data"]["automatic_observation"]["ack"]["job"]
    save("attached.json",attached)
    for number,value in enumerate(config["values"],1):
        deadline=time.time()+90
        while time.time()<deadline:
            status=call("status")
            state=status["state"]
            if state["live"]["sample_count"]>=number:break
            assert status["jobs"]["history"]["jobs"][str(job)]["phase"] not in ["Failed","Cancelled","Interrupted"]
            time.sleep(.05)
        assert state["live"]["sample_count"]==number,"Capture missed; no replacement run"
        states.append(state);save("states.json",states)
        if number<16:
            control.write_text(",".join(map(str,config["values"][number])),encoding="ascii")
        print(json.dumps({"captured":number}),flush=True)
    deadline=time.time()+20
    while time.time()<deadline:
        status=call("status")
        if status["jobs"]["history"]["jobs"][str(job)]["phase"]=="Completed":break
        time.sleep(.05)
    assert status["jobs"]["history"]["jobs"][str(job)]["phase"]=="Completed"
    live=status["state"]["live"]
    save("discovery-status.json",status)
    checks=live["prediction_checks"]
    assert len(checks)==2, "Expected two automatic checks; retain any missing discovery"
    tracks={t["id"]:t for t in live["tracks"]}
    checks=sorted(checks,key=lambda c:tracks[c["track"]]["bounds"][1])
    frozen=states[11]["live"]["prediction_checks"]
    for field,check in enumerate(checks):
        assert tracks[check["track"]]["samples"]==[v[field] for v in config["values"]]
        before=next(c for c in frozen if c["id"]==check["id"])
        assert check["after_frame"]==12 and len(check["automatic_selection"]["planned_tracks"])==2
        assert all(not c["samples"] for c in before["candidates"])
        assert [c["mapping"] for c in before["candidates"]]==[c["mapping"] for c in check["candidates"]]
        assert all([s["frame"] for s in c["samples"]]==[13,14,15,16] for c in check["candidates"])
        assert all(s["outcome"]=="agrees" for c in check["candidates"] for s in c["samples"])
    assert not status["state"]["changes"]
    assert not any(c["name"] in ["capture","correlate","start_prediction_check"] for c in commands)
    plans=[]
    for field,check in enumerate(checks):
        response=call("prepare_probe",{"investigation_id":live["investigation_id"],"batch":live["coverage"]["batch"],
            "track":check["track"],"value":config["values"][-1][field]+3,"prediction_id":check["id"],"candidate":0},False)
        save("prepared-"+str(field)+".json",response)
        assert response["ok"] and response["data"]["phase"]=="prepared", "No replacement for unavailable plan"
        change=response["data"]; truth=oracle();patch=change["patch"]
        assert truth["fields"][field] <= patch["address"] and patch["address"]+len(patch["after"]) <= truth["fields"][field]+4
        assert change["scope"]["prediction"]==check["id"]
        plans.append(change)
    report["prepared_oracle"]=oracle()
    change_id=plans[slot]["id"]
    applied=call("apply_change",{"change_id":change_id})["data"];save("applied.json",applied)
    assert applied["phase"]=="written";written=True
    time.sleep(.2)
    expected=config["values"][-1].copy();expected[slot]+=3
    report["applied_oracle"]=oracle()
    assert report["applied_oracle"]["values"]==expected
    checked=call("observe_change",{"change_id":change_id})["data"];save("checked.json",checked)
    assert checked["observations"][-1]["intervention_outcome"]=="matched"
    current=call("status")
    assert all(c["invalidated"] for c in current["state"]["live"]["prediction_checks"])
    expired=call("apply_change",{"change_id":plans[1-slot]["id"]})["data"];save("other-expired.json",expired)
    assert expired["phase"]=="expired" and expired["write"] is None
    assert oracle()["values"]==expected
    refused=call("recheck_findings",{"investigation_id":live["investigation_id"],"batch":1},False)
    save("unrestored-recheck.json",refused)
    assert not refused["ok"] and refused["state"]["live"]["coverage"]["batch"]==1
    restore_attempted=True
    restored_change=call("restore_change",{"change_id":change_id})["data"];save("restored.json",restored_change)
    assert restored_change["phase"]=="restored";restored=True
    time.sleep(.2)
    report["restored_oracle"]=oracle()
    assert report["restored_oracle"]["values"]==config["values"][-1]
    sequence=config["fresh_values"][slot]
    control.write_text(",".join(map(str,sequence[0])),encoding="ascii")
    time.sleep(.2)
    recheck=call("recheck_findings",{"investigation_id":live["investigation_id"],"batch":1})
    save("recheck-start.json",recheck)
    fresh=recheck["state"]["live"]
    assert fresh["coverage"]["batch"]==2 and fresh["sample_count"]==0
    assert fresh["planned_bytes"]<=sum(len(c["candidates"]) for c in checks)*8
    for c in fresh["prediction_checks"]:
        previous=next(x for x in checks if x["id"]==c["rechecked_from"]["check"])
        assert c["id"]!=previous["id"] and c["after_frame"]==0
        assert [x["mapping"] for x in c["candidates"]]==[x["mapping"] for x in previous["candidates"]]
        assert [x["address"] for x in c["candidates"]]==[x["address"] for x in previous["candidates"]]
        assert all(not x["samples"] for x in c["candidates"])
        archive=json.loads(Path(c["rechecked_from"]["archive"]).read_text(encoding="utf-8"))
        assert any(x["id"]==previous["id"] for x in archive["state"]["prediction_checks"])
    job=recheck["data"]["automatic_observation"]["ack"]["job"]
    plan=recheck["jobs"]["history"]["jobs"][str(job)]
    assert len(plan["steps"])==4
    fresh_states=[]
    for number in range(1,5):
        deadline=time.time()+90
        while time.time()<deadline:
            status=call("status")
            if status["state"]["live"]["sample_count"]>=number:break
            assert status["jobs"]["history"]["jobs"][str(job)]["phase"] not in ["Failed","Cancelled","Interrupted"]
            time.sleep(.05)
        assert status["state"]["live"]["sample_count"]==number,"Missed fresh capture; no replacement run"
        fresh_states.append(status["state"]);save("fresh-states.json",fresh_states)
        if number<4:control.write_text(",".join(map(str,sequence[number])),encoding="ascii")
        print(json.dumps({"recheck_captured":number}),flush=True)
    deadline=time.time()+20
    while time.time()<deadline:
        status=call("status")
        if status["jobs"]["history"]["jobs"][str(job)]["phase"]=="Completed":break
        time.sleep(.05)
    assert status["jobs"]["history"]["jobs"][str(job)]["phase"]=="Completed"
    save("recheck-completed.json",status)
    stale=call("recheck_findings",{"investigation_id":live["investigation_id"],"batch":1},False)
    save("stale-recheck.json",stale)
    assert not stale["ok"] and stale["state"]["live"]["coverage"]["batch"]==2
    fresh=status["state"]["live"]
    tracks={t["id"]:t for t in fresh["tracks"]}
    checks=sorted(fresh["prediction_checks"],key=lambda c:tracks[c["track"]]["bounds"][1])
    for field,c in enumerate(checks):
        assert tracks[c["track"]]["samples"]==[v[field] for v in sequence]
        assert all([s["frame"] for s in x["samples"]]==[1,2,3,4] for x in c["candidates"])
        assert all(s["outcome"]=="agrees" for x in c["candidates"] for s in x["samples"])
    field=1-slot; check=checks[field]
    prepared=call("prepare_probe",{"investigation_id":fresh["investigation_id"],"batch":2,
        "track":check["track"],"value":sequence[-1][field]+3,"prediction_id":check["id"],"candidate":0},False)
    save("fresh-prepared.json",prepared)
    if slot==0:
        assert prepared["ok"] and prepared["data"]["phase"]=="prepared"
        change=prepared["data"];truth=oracle();patch=change["patch"]
        assert truth["fields"][field]<=patch["address"] and patch["address"]+len(patch["after"])<=truth["fields"][field]+4
        change_id=change["id"];written=False;restored=False;restore_attempted=False
        applied=call("apply_change",{"change_id":change_id})["data"];save("fresh-applied.json",applied)
        assert applied["phase"]=="written";written=True
        time.sleep(.2)
        expected=sequence[-1].copy();expected[field]+=3
        report["fresh_applied_oracle"]=oracle()
        assert report["fresh_applied_oracle"]["values"]==expected
        effect=call("observe_change",{"change_id":change_id})["data"];save("fresh-effect.json",effect)
        assert effect["observations"][-1]["intervention_outcome"]=="matched"
        restore_attempted=True
        restored_change=call("restore_change",{"change_id":change_id})["data"];save("fresh-restored.json",restored_change)
        assert restored_change["phase"]=="restored";restored=True
        time.sleep(.2)
        assert oracle()["values"]==sequence[-1]
    else:
        assert not prepared["ok"] and "contrasting evidence" in prepared["error"].lower()
        assert oracle()["values"]==sequence[-1]
    save("final-status.json",call("status"))
    report.update({"outcome":"new edit after fresh contrast" if slot==0 else "constant fresh readings refused edit",
        "tracks":[c["track"] for c in checks],"candidates":[len(c["candidates"]) for c in checks],
        "fresh_planned_bytes":fresh["planned_bytes"],"fresh_reads":fresh["coverage"]["attempted_bytes"]-recheck["state"]["live"]["coverage"]["attempted_bytes"]})

except Exception as error:
    report["errors"].append(repr(error)); raise
finally:
    if written and not restored and not restore_attempted:
        try:
            save("cleanup-restore.json", call("restore_change", {"change_id": change_id}, False))
        except Exception as error:
            report["errors"].append("cleanup: " + repr(error))
    control.write_text("close", encoding="ascii")
    for process in reversed(processes):
        if process.poll() is None:
            if process is processes[0]:
                try: process.wait(timeout=4)
                except subprocess.TimeoutExpired: process.terminate()
            else: process.terminate()
        try: process.wait(timeout=5)
        except subprocess.TimeoutExpired: report["errors"].append("Owned process did not stop: " + str(process.pid))
    save("report.json", report)
    print(json.dumps(report), flush=True)
