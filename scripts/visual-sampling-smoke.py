"""Native capture cadence and cancellation check. No game or manual image input.
Pass isolated DATA_DIR and observation-fixture.ps1 PID on the same desktop.
The running engine and REVERSEBOLT_BIN_DIR must use the same published build.
"""
import base64
import hashlib
import json
import os
from pathlib import Path
import queue
import statistics
import subprocess
import sys
import threading
import time
import urllib.request

data, pid = Path(sys.argv[1]), int(sys.argv[2])
binary = Path(os.environ["REVERSEBOLT_BIN_DIR"])
endpoint = json.loads((data / "endpoint.json").read_text(encoding="utf-8-sig"))
process = subprocess.Popen([str(binary / "reversebolt-mcp.exe"), "--data-dir", str(data)],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    text=True, encoding="utf-8", creationflags=0x08000000)
lines = queue.Queue()
threading.Thread(target=lambda: [lines.put(line) for line in process.stdout], daemon=True).start()
counter = 0

def rpc(method, params):
    global counter
    counter += 1
    process.stdin.write(json.dumps({"jsonrpc":"2.0","id":counter,"method":method,"params":params}) + "\n")
    process.stdin.flush()
    while True:
        value = json.loads(lines.get(timeout=120))
        if value.get("id") == counter:
            assert "error" not in value, value
            return value["result"]

def call(action, args=None, error=False):
    result = rpc("tools/call", {"name":"reversebolt_"+action, "arguments":args or {}})
    assert bool(result.get("isError")) == error, result
    payload = json.loads(result["content"][-1]["text"])
    assert payload["ok"] != error, payload.get("error")
    return payload, result

def status():
    request = urllib.request.Request(endpoint["url"] + "/command", data=b'{"action":"status","args":{}}',
        headers={"Content-Type":"application/json", "Authorization":"Bearer "+endpoint["token"]})
    with urllib.request.urlopen(request, timeout=10) as response:
        return json.load(response)

def envelope(extra):
    history = status()["jobs"]["history"]
    return {"request_id":history["clients"].get("mcp-agent",{}).get("high_water",0)+1,
        "boot":history["boot"],"binding":history["current"],**extra}

def wait(predicate, timeout=120):
    deadline = time.monotonic()+timeout
    while time.monotonic()<deadline:
        state=status()
        if predicate(state): return state
        time.sleep(.025)
    raise AssertionError("Timed out waiting for capture job")

def phase(state, job): return state["jobs"]["history"]["jobs"][str(job)]["phase"]

report={"scope":"One owned 560x260 text window on this host; no game cadence or semantic claim",
    "manual_screenshots":0,"game_inputs":0,"binary":str(binary),"modes":{}}
try:
    rpc("initialize",{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"visual-sampling-check","version":"1"}})
    process.stdin.write(json.dumps({"jsonrpc":"2.0","method":"notifications/initialized"})+"\n")
    process.stdin.flush()
    target=next(p for p in call("list_processes")[0]["state"]["processes"] if p["pid"]==pid)
    identity={k:target[k] for k in ["pid","created","window"]}
    for mode, interval in [("full",250),("visual",50)]:
        call("attach_process",{**identity,"budget_mib":1,"auto_observe":False})
        if mode=="visual":
            call("start_observation_job",envelope({"mode":"visual","count":6,"interval_ms":50,"compare":True}),error=True)
            call("capture",{"mode":"invalid"},error=True)
            assert status()["state"]["live"]["sample_count"]==0
        warm,_=call("capture",{"mode":mode})
        request=envelope({"mode":mode,"count":6,"interval_ms":interval})
        started,_=call("start_observation_job",request)
        job=started["data"]["ack"]["job"]
        replay,_=call("start_observation_job",request)
        assert replay["data"]["duplicate"]
        finished=wait(lambda s:phase(s,job)=="Completed")
        live=finished["state"]["live"]
        assert live["sample_count"]==7
        folder=Path(live["latest"]["screen"]["path"]).parent.resolve()
        assert folder.is_relative_to(data.resolve())
        captures=[json.loads((folder/f"frame-{n:03}.json").read_text(encoding="utf-8")) for n in range(2,8)]
        assert all(c["mode"]==mode and c["captured_bytes"]>0 for c in captures)
        assert all(c["screen"]["method"]=="windows_graphics_capture" for c in captures)
        if mode=="visual":
            assert all(c["recognition_ms"]==0 and not c["screen"]["words"] and not c["screen"]["text"] for c in captures)
            assert not live["tracks"]
        else:
            sources={word["source"] for c in captures for word in c["screen"]["words"]}
            assert any("pp_ocr" in source for source in sources), sources
            assert any(t["samples"]==[40]*7 for t in live["tracks"])
        selected=live["frames"][1:]
        sequence,result=call("read_sequence",{"investigation_id":live["investigation_id"],"frame_ids":[f["id"] for f in selected]})
        images=[base64.b64decode(c["data"],validate=True) for c in result["content"] if c["type"]=="image"]
        assert [hashlib.sha256(b).hexdigest() for b in images]==[f["sha256"] for f in selected]
        starts=[f["capture_interval"]["begin_us"] for f in selected]
        intervals=[(b-a)/1000 for a,b in zip(starts,starts[1:])]
        report["modes"][mode]={"requested_delay_ms":interval,"captures":captures,
            "warmup_elapsed_ms":warm["state"]["live"]["latest"]["elapsed_ms"],
            "median_capture_ms":statistics.median(c["elapsed_ms"] for c in captures),
            "median_recognition_ms":statistics.median(c["recognition_ms"] for c in captures),
            "median_start_interval_ms":statistics.median(intervals),"start_intervals_ms":intervals,
            "image_hashes_verified":len(images)}
        (data/f"{mode}-complete.json").write_text(json.dumps(finished,indent=2),encoding="utf-8")
        if mode=="full": call("detach_process")
    started,_=call("start_observation_job",envelope({"mode":"visual","count":8,"interval_ms":50}))
    job=started["data"]["ack"]["job"]
    wait(lambda s:s["state"]["live"]["sample_count"]>7)
    call("job_control",envelope({"operation":"pause","job":job}))
    paused=wait(lambda s:phase(s,job)=="Paused")
    held=paused["state"]["live"]["sample_count"]
    time.sleep(.5)
    assert status()["state"]["live"]["sample_count"]==held
    call("job_control",envelope({"operation":"resume","job":job}))
    wait(lambda s:s["state"]["live"]["sample_count"]>held)
    call("job_control",envelope({"operation":"cancel","job":job}))
    stopped=wait(lambda s:phase(s,job)=="Cancelled")
    end=stopped["state"]["live"]["sample_count"]
    time.sleep(.5)
    assert status()["state"]["live"]["sample_count"]==end<15
    restored,_=call("capture",{"mode":"full"})
    assert any(t["samples"][-1]==40 and all(v is None for v in t["samples"][:-1]) for t in restored["state"]["live"]["tracks"])
    report["control"]={"paused_at":held,"cancelled_at":end,"resumed":True,"no_captures_after_stop":True,"text_resumed_without_filling_gaps":True}
    report["capture_time_speedup"]=report["modes"]["full"]["median_capture_ms"]/max(1,report["modes"]["visual"]["median_capture_ms"])
    (data/"visual-sampling-report.json").write_text(json.dumps(report,indent=2),encoding="utf-8")
    print(json.dumps({"modes":{k:{n:v[n] for n in ["median_capture_ms","median_recognition_ms","median_start_interval_ms"]} for k,v in report["modes"].items()},"controls":report["control"],"report":str(data/"visual-sampling-report.json")}))
    call("detach_process")
finally:
    process.stdin.close()
    try: process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.terminate(); process.wait(timeout=5)
