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
assert slot == 0
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
    assert os.environ.get("ANODE_AGENT_ID") and os.environ.get("ANODE_LEASE_TOKEN")
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
    assert len(rpc("tools/list",{})["tools"])==36
    target=next(p for p in call("list_processes")["state"]["processes"] if p["pid"]==fixture.pid)
    anode=Path(os.environ['LOCALAPPDATA'])/'Programs/Anode/anode.exe'
    inventory=subprocess.run([str(anode),'windows','--pid',str(fixture.pid),'--json'],capture_output=True,text=True,check=True,timeout=20)
    windows=json.loads(inventory.stdout)['windows']
    owned=next(w for w in windows if w['title']=='Reversebolt delayed-result fixture')
    subprocess.run([str(anode),'window',owned['windowId'],'focus'],capture_output=True,text=True,check=True,timeout=20)
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

    choices=[(c,i,m) for c in sorted(fresh['prediction_checks'],key=lambda c:c['track']) if c['source']=='ocr'
        for i,m in enumerate(c['candidates']) if len(m['samples'])==4 and all(v['outcome']=='agrees' for v in m['samples'])]
    assert choices,'No eligible fresh candidate; no fallback'
    check,index,candidate=choices[0]
    observed=next(t['samples'][-1] for t in fresh['tracks'] if t['id']==check['track'])
    report['trial_selection']={'track':check['track'],'candidate':index,'address':candidate['address'],'requested':observed+10}
    prepared=call('prepare_probe',{'investigation_id':fresh['investigation_id'],'batch':3,'track':check['track'],
        'prediction_id':check['id'],'candidate':index,'value':observed+10},False)
    save('prepared.json',prepared)
    assert prepared['ok'] and prepared['data']['phase']=='prepared','Preparation refused; no fallback'
    change_id=prepared['data']['id']
    patch=prepared['data']['patch']
    assert field<=patch['address']<field+4,'Selected trial is not the independently known fixture field; no fallback'
    applied=call('apply_change',{'change_id':change_id},False);save('applied.json',applied)
    assert applied['ok'] and applied['data']['phase']=='written'
    written=True
    assert applied['data']['observations'][-1]['intervention_outcome']=='did_not_follow'
    report['after_apply_oracle']=oracle()
    assert report['after_apply_oracle']['value']==observed+10 and float(report['after_apply_oracle']['display'])==observed

    # Exact owned-window native input, in the already checked child session.
    class Keyboard(ctypes.Structure):
        _fields_=[('vk',ctypes.c_ushort),('scan',ctypes.c_ushort),('flags',ctypes.c_ulong),('time',ctypes.c_ulong),('extra',ctypes.c_void_p)]
    class Mouse(ctypes.Structure):
        _fields_=[('x',ctypes.c_long),('y',ctypes.c_long),('data',ctypes.c_ulong),('flags',ctypes.c_ulong),('time',ctypes.c_ulong),('extra',ctypes.c_void_p)]
    class Union(ctypes.Union):_fields_=[('keyboard',Keyboard),('mouse',Mouse)]
    class Input(ctypes.Structure):_fields_=[('type',ctypes.c_ulong),('data',Union)]
    user=ctypes.windll.user32;user.GetForegroundWindow.restype=ctypes.c_void_p
    user.GetWindowThreadProcessId.argtypes=[ctypes.c_void_p,ctypes.POINTER(ctypes.c_ulong)]
    def pulse():
        hwnd=user.GetForegroundWindow();actual=ctypes.c_ulong();user.GetWindowThreadProcessId(hwnd,ctypes.byref(actual))
        assert hwnd==target['window'] and actual.value==target['pid'],'Fixture lost foreground; do not send input'
        before=oracle()['delivered'];stamp=time.time_ns()//1000000
        event=Input(type=1,data=Union(keyboard=Keyboard(vk=32,flags=0)))
        assert user.SendInput(1,ctypes.byref(event),ctypes.sizeof(event))==1
        try:time.sleep(.12)
        finally:
            event.data.keyboard.flags=2
            assert user.SendInput(1,ctypes.byref(event),ctypes.sizeof(event))==1
        deadline=time.time()+2
        while time.time()<deadline:
            result=oracle()
            if result['delivered']==before+1:break
            time.sleep(.02)
        assert result['delivered']==before+1,'Input not confirmed; never replay'
        report.setdefault('input',[]).append({'pressed_ms':stamp,'released_ms':time.time_ns()//1000000,'window':hwnd,'pid':actual.value,'oracle':result})

    def watch(change,name,count):
        current=call('status');ledger=current['jobs']['history'];before=len(change['observations'])
        response=call('start_change_watch',{'request_id':ledger['clients']['mcp-agent']['high_water']+1,
            'boot':ledger['boot'],'binding':ledger['current'],'change_id':change_id,'phase':change['phase'],
            'after_observation':before,'count':count,'interval_ms':2000})
        save(name+'-started.json',response);job=response['data']['ack']['job'];collected=[]
        for offset in range(1,count+1):
            deadline=time.time()+45
            while time.time()<deadline:
                state=call('status');updated=next(c for c in state['state']['changes'] if c['id']==change_id)
                if len(updated['observations'])>=before+offset:break
                assert state['jobs']['history']['jobs'][str(job)]['phase'] not in ['Failed','Cancelled','Interrupted']
                time.sleep(.05)
            assert len(updated['observations'])==before+offset,'Missing/skipped watch observation; no replacement'
            collected.append({'change':updated,'oracle':oracle(),'job':state['jobs']['history']['jobs'][str(job)]})
            save(name+'-states.json',collected)
            if offset==1:pulse()
            print(json.dumps({'stage':name,'observation':before+offset}),flush=True)
        deadline=time.time()+20
        while time.time()<deadline:
            state=call('status')
            if state['jobs']['history']['jobs'][str(job)]['phase']=='Completed':break
            time.sleep(.05)
        assert state['jobs']['history']['jobs'][str(job)]['phase']=='Completed'
        save(name+'-completed.json',state)
        observations=updated['observations'][before:]
        assert [o['intervention_outcome'] for o in observations]==['did_not_follow']+['matched']*(count-1)
        assert any(o.get('input_context') and any(32 in poll['down'] for poll in o['input_context']['polls']) for o in observations[1:]),'No sampled input witness'
        return updated

    watched=watch(applied['data'],'effect',3)
    assert [o['value'] for o in watched['observations']]==[observed,observed,observed+10,observed+10]
    restore_attempted=True
    restored_response=call('restore_change',{'change_id':change_id},False);save('restored.json',restored_response)
    assert restored_response['ok'] and restored_response['data']['phase']=='restored'
    restored=True
    assert restored_response['data']['observations'][-1]['intervention_outcome']=='did_not_follow'
    final_change=watch(restored_response['data'],'restoration',2)
    assert final_change['observations'][-1]['value']==observed
    report['outcome']='delayed effect and delayed restoration observed after two native input events; no automatic writes'
    report['final_oracle']=oracle()
    save('final-status.json',call('status'))
except Exception as error:
    report["errors"].append(repr(error));raise
finally:
    if change_id and not restore_attempted:
        try:
            current=call('status');change=next(c for c in current['state']['changes'] if c['id']==change_id)
            if change['phase']=='written':
                restore_attempted=True;save('cleanup-restore.json',call('restore_change',{'change_id':change_id},False))
        except Exception as cleanup:report['errors'].append('Checked cleanup: '+repr(cleanup))
    if mcp:
        try:save('final-status.json',call('status'))
        except Exception as cleanup:report['errors'].append('Final status: '+repr(cleanup))
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
