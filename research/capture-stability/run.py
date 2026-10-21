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
control.write_text("set:40", encoding="ascii")
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
    processes.append(fixture);report["fixture_pid"]=fixture.pid
    deadline=time.time()+25
    while not oracle_file.exists() and time.time()<deadline:
        assert fixture.poll() is None;time.sleep(.1)
    report["initial_oracle"]=oracle()
    engine=subprocess.Popen([str(binary/"Reversebolt.exe"),"--data-dir",str(data)],creationflags=0x08000000)
    processes.append(engine);report["engine_pid"]=engine.pid
    deadline=time.time()+20
    while not (data/"endpoint.json").exists() and time.time()<deadline:
        assert engine.poll() is None;time.sleep(.1)
    assert (data/"endpoint.json").exists()
    mcp=subprocess.Popen([str(binary/"reversebolt-mcp.exe"),"--data-dir",str(data)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,
        stderr=(out/"mcp-stderr.log").open("w",encoding="utf-8"),text=True,encoding="utf-8",creationflags=0x08000000)
    processes.append(mcp);report["mcp_pid"]=mcp.pid
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc("initialize",{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"learned-reference-evaluation","version":"1"}})
    mcp.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n');mcp.stdin.flush()
    assert len(rpc("tools/list",{})["tools"])==32
    target=next(p for p in call("list_processes")["state"]["processes"] if p["pid"]==fixture.pid)
    call("set_observer_model",{"model":None})
    attached=call("attach_process",{**{k:target[k] for k in ["pid","created","window"]},"budget_mib":32})
    save("attached.json",attached)

    def complete_sequence(response,values,name):
        job=response["data"]["automatic_observation"]["ack"]["job"]
        collected=[]
        for number in range(1,len(values)+1):
            deadline=time.time()+90
            while time.time()<deadline:
                status=call("status")
                if status["state"]["live"]["sample_count"]>=number:break
                assert status["jobs"]["history"]["jobs"][str(job)]["phase"] not in ["Failed","Cancelled","Interrupted"]
                time.sleep(.05)
            assert status["state"]["live"]["sample_count"]==number,"Missed capture; no replacement run"
            collected.append(status["state"]);save(name+"-states.json",collected)
            if number<len(values):control.write_text("set:"+str(values[number]),encoding="ascii")
            print(json.dumps({"stage":name,"captured":number}),flush=True)
        deadline=time.time()+20
        while time.time()<deadline:
            status=call("status")
            if status["jobs"]["history"]["jobs"][str(job)]["phase"]=="Completed":break
            time.sleep(.05)
        assert status["jobs"]["history"]["jobs"][str(job)]["phase"]=="Completed"
        save(name+"-completed.json",status)
        return status

    discovered=complete_sequence(attached,config["values"],"discovery")
    live=discovered["state"]["live"]
    assert live["prediction_checks"],"No automatic finding"
    check=next(c for c in live["prediction_checks"] if c["source"]=="ocr")
    assert any(c["references"]["routes"] for c in check["candidates"]),"No learned references"
    assert next(t for t in live["tracks"] if t["id"]==check["track"])["samples"]==config["values"]
    report["before_move_oracle"]=oracle()
    control.write_text("move:free",encoding="ascii")
    deadline=time.time()+5
    while oracle()["generation"]!=1 and time.time()<deadline:time.sleep(.05)
    report["moved_oracle"]=oracle();assert report["moved_oracle"]["generation"]==1
    assert report["moved_oracle"]["field"]!=report["before_move_oracle"]["field"]
    control.write_text("set:54",encoding="ascii");time.sleep(.2)
    recheck=call("recheck_findings",{"investigation_id":live["investigation_id"],"batch":1},False)
    save("recheck-start.json",recheck);assert recheck["ok"],recheck.get("error")
    completed=complete_sequence(recheck,config["fresh_values"],"recheck")
    fresh=completed["state"]["live"]
    current=next(c for c in fresh["prediction_checks"] if c["track"]==check["track"] and c["source"]=="ocr")
    assert current["rechecked_from"]["check"]==check["id"]
    index=next(i for i,c in enumerate(current["candidates"]) if c["via"] is not None)
    candidate=current["candidates"][index]
    report["chosen_candidate"]={"index":index,"address":candidate["address"],"via":candidate["via"],"mapping":candidate["mapping"]}
    assert candidate["invalidated"] is None
    assert all(s["outcome"]=="agrees" and s["reference_before"] and s["reference_after"] for s in candidate["samples"])
    prepared=call("prepare_probe",{"investigation_id":fresh["investigation_id"],"batch":2,"track":current["track"],
        "value":53,"prediction_id":current["id"],"candidate":index},False)
    save("prepared.json",prepared)
    assert prepared["ok"] and prepared["data"]["phase"]=="prepared",prepared.get("error")
    change=prepared["data"];change_id=change["id"];truth=oracle();patch=change["patch"]
    assert truth["field"]<=patch["address"] and patch["address"]+len(patch["after"])<=truth["field"]+4
    assert any(candidate["via"] in c["references"]["routes"] and candidate["mapping"]==c["mapping"] for c in check["candidates"]),"The reference and mapping must have been learned before the move"
    if slot==0:
        applied=call("apply_change",{"change_id":change_id})["data"];save("applied.json",applied)
        assert applied["phase"]=="written";written=True
        time.sleep(.2);report["applied_oracle"]=oracle();assert report["applied_oracle"]["value"]==53
        effect=call("observe_change",{"change_id":change_id})["data"];save("effect.json",effect)
        assert effect["observations"][-1]["intervention_outcome"]=="matched"
        restore_attempted=True
        recovery=call("restore_change",{"change_id":change_id})["data"];save("restored.json",recovery)
        assert recovery["phase"]=="restored";restored=True
        time.sleep(.2);report["final_oracle"]=oracle();assert report["final_oracle"]["value"]==50
        report["outcome"]="moved value found, edited, observed and restored"
    else:
        old_address=truth["field"]
        control.write_text("move:keep",encoding="ascii")
        deadline=time.time()+5
        while oracle()["generation"]!=2 and time.time()<deadline:time.sleep(.05)
        assert oracle()["generation"]==2
        refused=call("apply_change",{"change_id":change_id},False);save("refused.json",refused)
        assert not refused["ok"] and "reference moved" in refused["error"].lower()
        report["final_oracle"]=oracle();assert report["final_oracle"]["value"]==50
        k=ctypes.WinDLL("kernel32",use_last_error=True)
        k.OpenProcess.argtypes=[ctypes.c_ulong,ctypes.c_bool,ctypes.c_ulong];k.OpenProcess.restype=ctypes.c_void_p
        k.ReadProcessMemory.argtypes=[ctypes.c_void_p,ctypes.c_void_p,ctypes.c_void_p,ctypes.c_size_t,ctypes.POINTER(ctypes.c_size_t)]
        k.CloseHandle.argtypes=[ctypes.c_void_p]
        handle=k.OpenProcess(0x10,False,fixture.pid);assert handle
        try:
            value=ctypes.c_float();received=ctypes.c_size_t()
            assert k.ReadProcessMemory(handle,old_address,ctypes.byref(value),4,ctypes.byref(received)) and received.value==4
            report["retained_old_value"]=value.value;assert value.value==50
        finally:k.CloseHandle(handle)
        report["outcome"]="second move refused old prepared write; old and current values unchanged"
    save("final-status.json",call("status"))
    assert not any(c["name"] in ["capture","correlate","start_prediction_check"] for c in commands)

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
