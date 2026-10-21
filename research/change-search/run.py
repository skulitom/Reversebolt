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
control.write_text("set:50", encoding="ascii")
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
    fixture=subprocess.Popen(["powershell.exe","-NoProfile","-ExecutionPolicy","Bypass","-File",str(HERE/"fixture.ps1"),
        "-ControlFile",str(control),"-OracleFile",str(oracle_file),"-Variant","2"],creationflags=0x08000000)
    processes.append(fixture);report["fixture_pid"]=fixture.pid
    deadline=time.time()+25
    while not oracle_file.exists() and time.time()<deadline:
        assert fixture.poll() is None;time.sleep(.1)
    report["initial_oracle"]=oracle()
    engine=subprocess.Popen([str(binary/"Reversebolt.exe"),"--headless","--data-dir",str(data)],creationflags=0x08000000)
    processes.append(engine);report["engine_pid"]=engine.pid
    deadline=time.time()+20
    while not (data/"endpoint.json").exists() and time.time()<deadline:
        assert engine.poll() is None;time.sleep(.1)
    assert (data/"endpoint.json").exists()
    mcp=subprocess.Popen([str(binary/"reversebolt-mcp.exe"),"--data-dir",str(data)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,
        stderr=(out/"mcp-stderr.log").open("w",encoding="utf-8"),text=True,encoding="utf-8",creationflags=0x08000000)
    processes.append(mcp);report["mcp_pid"]=mcp.pid
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc("initialize",{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"change-search-evaluation","version":"1"}})
    mcp.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n');mcp.stdin.flush()
    assert len(rpc("tools/list",{})["tools"])==35
    target=next(p for p in call("list_processes")["state"]["processes"] if p["pid"]==fixture.pid)
    call("set_observer_model",{"model":None})
    attached=call("attach_process",{**{k:target[k] for k in ["pid","created","window"]},"budget_mib":1,"auto_observe":False})
    save("attached.json",attached)

    def collect(job,batch,values,name):
        collected=[]
        for number in range(1,len(values)+1):
            deadline=time.time()+90
            while time.time()<deadline:
                status=call("status");live=status["state"]["live"]
                if live["coverage"]["batch"]==batch and live["sample_count"]>=number:
                    assert live["sample_count"]==number,"Skipped capture; preserve failure"
                    state=status["state"];break
                if name=="selection" and number==4 and live["coverage"]["batch"]==batch+1:
                    archive=next(b for b in live["coverage"]["batches"] if b["number"]==batch)
                    state={"live":json.loads(Path(archive["archive"]).read_text(encoding="utf-8"))["state"]}
                    report["last_selection_state_from_archive"]=True;break
                assert status["jobs"]["history"]["jobs"][str(job)]["phase"] not in ["Failed","Cancelled","Interrupted"]
                time.sleep(.05)
            else:raise AssertionError("Capture timeout; do not restart")
            collected.append(state);save(name+"-states.json",collected)
            if number<len(values):control.write_text("set:"+str(values[number]),encoding="ascii")
            print(json.dumps({"stage":name,"captured":number}),flush=True)
        assert collected[-1]["live"]["tracks"][0]["samples"]==values,"Strict fixture OCR sequence differed"
        save(name+"-completed-state.json",collected[-1])
        return collected

    ledger=attached["jobs"]["history"]
    first=call("start_observation_job",{"request_id":(ledger["clients"].get("mcp-agent") or {}).get("high_water",0)+1,
        "boot":ledger["boot"],"binding":ledger["current"],"count":1,"interval_ms":250})
    job=first["data"]["ack"]["job"]
    collect(job,1,[50],"initial")
    deadline=time.time()+20
    while time.time()<deadline:
        current=call("status")
        if current["jobs"]["history"]["jobs"][str(job)]["phase"]=="Completed":break
        time.sleep(.05)
    live=current["state"]["live"]
    track=min((t for t in live["tracks"] if t["active"] and t["samples"][-1] is not None),key=lambda t:t["id"])
    selection_values=config["selection_values"][str(slot)]
    control.write_text("set:"+str(selection_values[0]),encoding="ascii")
    started=call("start_change_search",{"investigation_id":live["investigation_id"],"batch":1,"after_frame":1,"track":track["id"]})
    save("started.json",started)
    selection_job=started["data"]["automatic_observation"]["ack"]["job"]
    selection_states=collect(selection_job,2,selection_values,"selection")
    deadline=time.time()+90
    while time.time()<deadline:
        current=call("status");live=current["state"]["live"];summary=live["change_searches"][-1]
        if summary["status"]!="collecting" and (slot==1 or live["coverage"]["batch"]==3):break
        time.sleep(.05)
    else:raise AssertionError("Selection handoff did not settle")
    report["search_summary"]=summary
    selection=json.loads(Path(summary["evidence"]).read_text(encoding="utf-8"));save("selection.json",selection)
    field=oracle()["field"];report["final_oracle"]=oracle()
    if slot==0:
        assert summary["status"]=="completed"
        assert any(w["address"]<=field and field+4<=w["address"]+w["length"] for w in selection["windows"]),"Actual encoded field was not retained"
        archived=json.loads(Path(live["coverage"]["batches"][0]["archive"]).read_text(encoding="utf-8"))
        assert not any(w["address"]<=field< w["address"]+w["length"] for w in archived["plan"])
        deadline=time.time()+15
        while time.time()<deadline:
            current=call("status")
            jobs=current["jobs"]["history"]["jobs"]
            children=[(int(k),v) for k,v in jobs.items() if v["binding"]["target"].startswith(live["investigation_id"]+"/3/")]
            if children:break
            time.sleep(.05)
        assert len(children)==1
        focused=collect(children[0][0],3,config["fresh_values"],"focused")
        fresh=focused[-1]["live"]
        candidates=[m for c in fresh["prediction_checks"] if c["source"]=="ocr" for m in c["candidates"] if int(m["address"],16)==field and m["mapping"]["kind"]=="F32"]
        assert candidates,"No frozen fresh F32 hypothesis at encoded field"
        assert len(candidates[0]["samples"])==4 and all(s["outcome"]=="agrees" for s in candidates[0]["samples"])
        report["actual_field_checks"]=len(candidates)
        old_ids={f["id"] for f in selection_states[-1]["live"]["frames"]}
        assert not old_ids & {f["id"] for f in fresh["frames"]}
        report["outcome"]="encoded distant field selected, fitted on twelve new moments, four fresh agreements"
        deadline=time.time()+20
        while time.time()<deadline:
            current=call("status")
            if current["jobs"]["history"]["jobs"][str(children[0][0])]["phase"]=="Completed":break
            time.sleep(.05)
        assert current["jobs"]["history"]["jobs"][str(children[0][0])]["phase"]=="Completed"

    else:
        assert summary["status"]=="no_selection" and "repeated changes" in summary["reason"]
        assert live["coverage"]["batch"]==2 and not live["prediction_checks"] and not summary["retained_windows"]
        assert len(current["jobs"]["history"]["jobs"])==2
        report["outcome"]="insufficient contrast abstained; no fresh batch or job"
    save("final-status.json",call("status"))
    assert not any(c["name"] in ["capture","correlate","finish_change_search","start_prediction_check","prepare_probe","prepare_change","apply_change"] for c in commands)
except Exception as error:
    report["errors"].append(repr(error));raise
finally:
    control.write_text("close",encoding="ascii")
    for process in reversed(processes):
        if process.poll() is None:
            if process is processes[0]:
                try:process.wait(timeout=4)
                except subprocess.TimeoutExpired:process.terminate()
            else:process.terminate()
        try:process.wait(timeout=5)
        except subprocess.TimeoutExpired:report["errors"].append("Owned process did not stop: "+str(process.pid))
    save("report.json",report)
    print(json.dumps({k:v for k,v in report.items() if k not in ["search_summary"]}),flush=True)
