"""Owned-window integration through MCP. No supplied field addresses or layouts.
Usage: trial-edit-smoke.py DATA_DIR CONTROL_FILE FIXTURE_PID [BIN_DIR]
Saves honest abstention if live hypotheses cannot agree on an action.
"""
import json
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time

data, control, pid = Path(sys.argv[1]), Path(sys.argv[2]), int(sys.argv[3])
binary = Path(sys.argv[4]) if len(sys.argv)>4 else Path(__file__).resolve().parents[1]/"target/release"
mcp = subprocess.Popen([str(binary/"reversebolt-mcp.exe"),"--data-dir",str(data)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding="utf-8",creationflags=0x08000000)
lines=queue.Queue()
threading.Thread(target=lambda:[lines.put(x) for x in mcp.stdout],daemon=True).start()
serial=0
def rpc(method,params):
    global serial
    serial+=1
    mcp.stdin.write(json.dumps({"jsonrpc":"2.0","id":serial,"method":method,"params":params})+"\n");mcp.stdin.flush()
    while True:
        r=json.loads(lines.get(timeout=120))
        if r.get("id")==serial:
            assert "error" not in r,r
            return r["result"]
def call(action,args=None,required=True):
    r=rpc("tools/call",{"name":"reversebolt_"+action,"arguments":args or {}})
    value=json.loads(next(c["text"] for c in r["content"] if c["type"]=="text"))
    if required: assert not r.get("isError") and value["ok"],value.get("error",r)
    return value
report={"scope":"Owned WinForms fixture. No game compatibility or gameplay-authority claim."}
try:
    rpc("initialize",{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"trial-edit-test","version":"1"}})
    mcp.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n');mcp.stdin.flush()
    tools=rpc("tools/list",{})["tools"]
    for name in ("apply_change","restore_change"):
        assert next(t for t in tools if t["name"]=="reversebolt_"+name)["annotations"]["destructiveHint"]
    target=next(p for p in call("list_processes")["state"]["processes"] if p["pid"]==pid and p["title"]=="Reversebolt trial fixture")
    call("attach_process",{**{k:target[k] for k in ("pid","created","window")},"auto_observe":False,"budget_mib":32})
    values=[40,56,31,85,64,27,75,92,44,67,38,50,54,54,50,50]
    states=[]
    for i,value in enumerate(values):
        control.write_text(str(value),encoding="ascii");time.sleep(.18)
        state=call("capture")["state"]
        states.append({"frame":i+1,"expected":value,"readings":[t["samples"][-1] for t in state["live"]["tracks"] if t["active"]]})
        if i==11:
            live=call("correlate")["state"]["live"]
            track=next(t for t in live["tracks"] if t["active"] and t["samples"]==values[:12])
            report["matches"]=len(track["matches"])
            assert track["matches"],"No passive matches; record this discovery failure."
            call("start_prediction_check",{"investigation_id":live["investigation_id"],"batch":live["coverage"]["batch"],"after_frame":12,"source":"ocr","track":track["id"]})
        print("captured",i+1,flush=True)
    live=state["live"]
    report["captures"]=states
    report["prediction"]=live["prediction_check"]
    prepared=call("prepare_change",{"investigation_id":live["investigation_id"],"batch":live["coverage"]["batch"],"track":track["id"],"value":53.},required=False)
    report["prepared"]=prepared
    if "--prepare-only" in sys.argv:
        report["outcome"]="prepared for desktop interaction; no write requested"
        print(report["outcome"],flush=True)
        raise SystemExit(0)
    if prepared["ok"] and prepared["data"]["phase"]=="prepared":
        change_id=prepared["data"]["id"]
        applied=call("apply_change",{"change_id":change_id})
        report["applied"]=applied
        assert applied["data"]["phase"]=="written",applied["data"]
        time.sleep(.2)
        checked=call("observe_change",{"change_id":change_id})
        report["checked"]=checked
        assert checked["data"]["observations"][-1]["agrees"],checked["data"]
        replay=call("apply_change",{"change_id":change_id})
        assert replay["data"]["write"]==applied["data"]["write"]
        restored=call("restore_change",{"change_id":change_id})
        report["restored"]=restored
        assert restored["data"]["phase"]=="restored",restored["data"]
        time.sleep(.2)
        check_restore=call("observe_change",{"change_id":change_id})
        report["checked_restore"]=check_restore
        assert check_restore["data"]["observations"][-1]["agrees"]
        report["outcome"]="write and restore observed through MCP"
    else:
        report["outcome"]="abstained: no unanimous live action"
    print(report["outcome"],flush=True)
except Exception as error:
    report["error"]=str(error)
    raise
finally:
    (data/"trial-result.json").write_text(json.dumps(report,indent=2),encoding="utf-8")
    mcp.terminate();mcp.wait(timeout=10)
