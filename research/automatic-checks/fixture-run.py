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
slot, out = 0, Path(sys.argv[1]).resolve()
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
control.write_text("40", encoding="ascii")
processes, commands, states = [], [], []
report = {"slot": slot, "errors": [], "protocol_sha256": hashlib.sha256((HERE / "protocol-v1.json").read_bytes()).hexdigest()}
serial, mcp, change_id, written, restored = 0, None, None, False, False
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
        "-ControlFile", str(control), "-OracleFile", str(oracle_file), "-SourceSlot", str(slot)], creationflags=0x08000000)
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
    assert len(tools) == 31
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
            control.write_text(str(config["values"][number]),encoding="ascii")
        print(json.dumps({"captured":number}),flush=True)
    deadline=time.time()+20
    while time.time()<deadline:
        status=call("status")
        if status["jobs"]["history"]["jobs"][str(job)]["phase"]=="Completed":break
        time.sleep(.05)
    assert status["jobs"]["history"]["jobs"][str(job)]["phase"]=="Completed"
    live=status["state"]["live"]
    assert any(t["samples"]==config["values"] for t in live["tracks"])
    check=live["prediction_check"]
    assert check and check["after_frame"]==12 and check["automatic_selection"]["scheduled_fresh_captures"]==4
    frozen=states[11]["live"]["prediction_check"]
    assert frozen and frozen["id"]==check["id"]
    assert all(not c["samples"] for c in frozen["candidates"])
    assert [c["mapping"] for c in frozen["candidates"]]==[c["mapping"] for c in check["candidates"]]
    assert all([s["frame"] for s in c["samples"]]==[13,14,15,16] for c in check["candidates"])
    assert all(s["outcome"]=="agrees" for c in check["candidates"] for s in c["samples"])
    assert not status["state"]["changes"]
    assert not any(c["name"] in ["capture","correlate","start_prediction_check","prepare_change","prepare_probe","apply_change"] for c in commands)
    time.sleep(1.3)
    assert call("status")["state"]["live"]["sample_count"]==16
    save("final-status.json",status)
    report.update({"outcome":"automatic prospective check completed without manual capture, check or edit commands","track":check["track"],"candidates":len(check["candidates"]),"fresh_agreements":sum(len(c["samples"]) for c in check["candidates"])})

except Exception as error:
    report["errors"].append(repr(error)); raise
finally:
    if written and not restored:
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
