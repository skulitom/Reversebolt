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
assert slot in [0,1,2]
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
        "-ControlFile", str(control), "-OracleFile", str(oracle_file), "-Variant", str(slot)], creationflags=0x08000000)
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
    rpc("initialize",{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"value-search-evaluation","version":"1"}})
    mcp.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n');mcp.stdin.flush()
    assert len(rpc("tools/list",{})["tools"])==33
    target=next(p for p in call("list_processes")["state"]["processes"] if p["pid"]==fixture.pid)
    call("set_observer_model",{"model":None})
    attached=call("attach_process",{**{k:target[k] for k in ["pid","created","window"]},"budget_mib":1})
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

    discovered=complete_sequence(attached,config["values"],"initial")
    live=discovered["state"]["live"]
    track=next(t for t in live["tracks"] if t["active"] and t["samples"][-1] is not None)
    assert track["samples"]==config["values"]
    report["initial_track"]=track["id"]
    searched=call("search_number",{"investigation_id":live["investigation_id"],"batch":1,
        "after_frame":16,"track":track["id"]},False)
    save("searched.json",searched)
    summary=searched["state"]["live"]["value_searches"][-1]
    report["search_summary"]=summary
    selection=json.loads(Path(summary["evidence"]).read_text(encoding="utf-8"));save("selection.json",selection)
    field=oracle()["field"]
    retained=any(w["address"]<=field and field+(8 if slot==1 else 4)<=w["address"]+w["length"] for w in selection["windows"])
    report["actual_field_retained"]=retained
    assert retained==(slot<2),"Direct seed should retain the field; encoded seed should not"
    assert summary["source_frame"]==16 and summary["source_batch"]==1 and summary["value"]==50
    assert summary["source_image_sha256"]==live["frames"][-1]["sha256"]
    assert summary["attempted_bytes"]<=512*1024*1024 and summary["retained_windows"]<=4096
    assert summary["read_bytes"]>=128*1024*1024
    if searched["ok"]:
        fresh=searched["state"]["live"]
        assert fresh["sample_count"]==0 and not fresh["prediction_checks"] and not fresh["tracks"]
        archived=json.loads(Path(fresh["coverage"]["batches"][-1]["archive"]).read_text(encoding="utf-8"))
        assert not any(c["address"]<=field< c["address"]+c["length"] for c in archived["plan"])
        completed=complete_sequence(searched,config["fresh_values"],"focused")
        fresh=completed["state"]["live"]
        candidates=[(c,m) for c in fresh["prediction_checks"] if c["source"]=="ocr" for m in c["candidates"] if int(m["address"],16)==field]
        report["actual_field_checks"]=len(candidates)
        if slot<2:
            assert candidates,"No fresh check at the distant field"
            check,candidate=candidates[0]
            assert check["after_frame"]==12
            assert len(candidate["samples"])==4 and all(s["outcome"]=="agrees" for s in candidate["samples"])
            save("field-check.json",{"check":check,"candidate":candidate})
            report["outcome"]="distant direct field selected, fitted on twelve new moments and agreed on four fresh moments"
        else:
            assert not candidates
            report["outcome"]="literal matches were sampled; unsupported encoded field remained undiscovered"
    else:
        assert slot==2 and summary["retained_windows"]==0
        report["outcome"]="unsupported encoded field absent; no retained locations and no new batch"
    save("final-status.json",call("status"))
    assert not any(c["name"] in ["capture","correlate","start_prediction_check","prepare_probe","prepare_change","apply_change"] for c in commands)
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
