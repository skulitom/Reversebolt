"""One automatic discovery and at most one product-planned comparison in Anode."""
import ctypes as C
from ctypes import wintypes as W
import hashlib
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time
import uuid

OUT = Path(sys.argv[1]).resolve()
PID = int(sys.argv[2])
read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / "frozen.json")
DATA = Path(frozen.get("data", str(OUT / "data")))
BIN = Path(frozen["bin"])
for name, digest in frozen["binaries"].items():
    assert sha(BIN / name) == digest, "Published binary changed"
assert sha(Path(__file__)) == frozen["sources"]["research/independent-runtime/run.py"]
assert not (OUT / "started.json").exists(), "Never replay a started experiment"
user = C.WinDLL("user32")
user.GetForegroundWindow.restype = W.HWND
user.GetWindowThreadProcessId.argtypes = [W.HWND, C.POINTER(W.DWORD)]
user.GetAsyncKeyState.restype = C.c_short
user.MapVirtualKeyW.restype = W.UINT
session = W.DWORD()
assert C.WinDLL("kernel32").ProcessIdToSessionId(os.getpid(), C.byref(session)) and session.value == 3
mcp = None
lines = queue.Queue()
seq = 0
inputs = []
active_test = None

def save(name, value):
    with (OUT / (name + ".json")).open("x", encoding="utf-8") as f:
        json.dump(value, f, indent=2)

def rpc(method, params):
    global seq
    seq += 1
    mcp.stdin.write(json.dumps({"jsonrpc": "2.0", "id": seq, "method": method, "params": params}) + "\n")
    mcp.stdin.flush()
    end = time.monotonic() + 180
    while time.monotonic() < end:
        response = json.loads(lines.get(timeout=max(.1, end - time.monotonic())))
        if response.get("id") == seq:
            assert "error" not in response, response
            return response["result"]
    raise RuntimeError("Unknown RPC outcome; do not replay")

def call(name, args=None, require_ok=True):
    result = rpc("tools/call", {"name": "reversebolt_" + name, "arguments": args or {}})
    response = json.loads(next(c["text"] for c in result["content"] if c["type"] == "text"))
    if name != "status":
        save(f"rpc-{seq:05}-{name}", {"args": args or {}, "response": response})
    if require_ok:
        assert response["ok"], (name, response.get("error"))
    return response

def status():
    return call("status")

def key_hold(key):
    hwnd = user.GetForegroundWindow()
    owner = W.DWORD()
    user.GetWindowThreadProcessId(hwnd, C.byref(owner))
    assert owner.value == PID and hwnd == target["window"], "Game focus changed; no input sent"
    assert not any(user.GetAsyncKeyState(k) & 0x8000 for k in range(1, 256)), "Conflicting input"
    scan = user.MapVirtualKeyW(key, 0)
    assert scan
    record = {"key": key, "planned_ms": 200, "begin_ns": time.time_ns()}
    user.keybd_event(0, scan, 8, 0)
    try:
        time.sleep(.2)
    finally:
        user.keybd_event(0, scan, 10, 0)
        record["end_ns"] = time.time_ns()
        inputs.append(record)
        save("input-" + str(len(inputs)), record)

def observe_condition(test):
    for _ in range(6):
        test = call("observe_runtime_test", {"test_id": test["id"], "after_revision": test["revision"], "perform_action": True})["data"]
        if test["phase"] == "stopped":
            break
    return test

save("started", {"runner_pid": os.getpid(), "engine_pid": read(DATA / "endpoint.json")["pid"], "game_pid": PID, "time_ns": time.time_ns()})
try:
    mcp = subprocess.Popen([str(BIN / "reversebolt-mcp.exe"), "--data-dir", str(DATA)],
                           stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=(OUT / "mcp.stderr").open("w"),
                           text=True, encoding="utf-8", creationflags=subprocess.CREATE_NO_WINDOW)
    threading.Thread(target=lambda: [lines.put(line) for line in mcp.stdout], daemon=True).start()
    rpc("initialize", {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "independent-runtime-evaluator", "version": "1"}})
    mcp.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
    mcp.stdin.flush()
    initial = status()
    assert initial["state"]["live"] is None and not initial["state"]["runtime_trials"], "Test engine is not fresh"
    target = next(p for p in call("list_processes")["state"]["processes"] if p["pid"] == PID)
    save("target", target)
    call("set_observer_model", {"model": "qwen3.8:latest"})
    call("attach_process", {"pid": PID, "created": target["created"], "window": target["window"], "budget_mib": 1, "auto_observe": True})
    handled = set()
    end = time.monotonic() + 1000
    next_progress = 0
    while time.monotonic() < end:
        current = status()
        live = current["state"]["live"]
        number = live["sample_count"]
        if number not in handled:
            handled.add(number)
            if number in (2, 4, 8, 10):
                key_hold(68 if number in (2, 8) else 65)
        jobs = current["jobs"]["history"]["jobs"]
        if time.monotonic() >= next_progress:
            print(json.dumps({"samples": number, "jobs": [j["phase"] for j in jobs.values()], "naming": live["runtime"].get("naming") is not None}), flush=True)
            next_progress = time.monotonic() + 20
        if jobs and all(j["phase"] in ("Completed", "Failed", "Cancelled", "Interrupted") for j in jobs.values()):
            break
        time.sleep(.05 if number < 15 else .5)
    else:
        raise RuntimeError("Job still pending; retain existing engine and job, do not restart")
    save("discovery", current)
    runtime = live["runtime"]
    review = runtime.get("naming")
    plan_result = None
    comparison = None
    if review and review["status"] == "complete":
        plan_result = call("plan_runtime_comparisons", {"catalog_id": runtime["catalog_id"], "review_id": review["evidence_id"]}, False)
        if plan_result["ok"]:
            plan = plan_result["data"]
            choice = plan["next"]
            if choice:
                active_test = str(uuid.uuid4())
                change = str(uuid.uuid4())
                args = {"test_id": active_test, "change_id": change, "investigation_id": plan["investigation_id"],
                        "catalog_id": plan["catalog_id"], "after_revision": plan["after_revision"],
                        "hypothesis_ref": choice["hypothesis_ref"], "value": choice["value"], "actions": choice["actions"]}
                save("chosen-plan", {"plan": plan, "args": args})
                test = observe_condition(call("prepare_runtime_test", args)["data"])
                if test["phase"] == "ready_to_apply":
                    fresh = status()["state"]["live"]
                    trial = call("set_runtime_value", {"change_id": change, "investigation_id": fresh["investigation_id"],
                                 "catalog_id": fresh["runtime"]["catalog_id"], "after_revision": fresh["runtime"]["revision"],
                                 "candidate_id": test["candidate_id"], "value": choice["value"]})["data"]
                    assert trial["phase"] == "written", trial["phase"]
                    test = next(t for t in status()["state"]["runtime_tests"] if t["id"] == active_test)
                    test = observe_condition(test)
                    restored = call("restore_runtime_change", {"change_id": change})["data"]
                    assert restored["phase"] == "restored", restored["phase"]
                    test = next(t for t in status()["state"]["runtime_tests"] if t["id"] == active_test)
                    if test["phase"] == "restored":
                        test = observe_condition(test)
                if test["active"]:
                    call("stop_runtime_test", {"test_id": active_test})
                comparison = call("read_runtime_test", {"test_id": active_test})["data"]
                save("comparison", comparison)
                call("plan_runtime_comparisons", {"catalog_id": runtime["catalog_id"], "review_id": review["evidence_id"]}, False)
    result = {"workflow_finished": True, "jobs": [{"phase": j["phase"], "error": j.get("error")} for j in jobs.values()],
              "samples": number, "evaluator_inputs": inputs, "review": review,
              "plan_available": bool(plan_result and plan_result["ok"]),
              "comparison": comparison["test"] if comparison else None,
              "manually_reviewed_measurement_images": 0}
    save("result", result)
    print(json.dumps({"finished": True, "samples": number, "questions": len((review or {}).get("batch", {}).get("accepted", [])) if (review or {}).get("batch") else 0,
                      "comparison": bool(comparison), "plan_available": result["plan_available"]}), flush=True)
except BaseException as error:
    save("failure", {"error": repr(error), "evaluator_inputs": inputs})
    raise
finally:
    if mcp is not None and mcp.poll() is None:
        try:
            current = status()
            if active_test and any(t["id"] == active_test and t["active"] for t in current["state"]["runtime_tests"]):
                call("stop_runtime_test", {"test_id": active_test})
            for trial in status()["state"]["runtime_trials"]:
                if trial["phase"] == "written":
                    call("restore_runtime_change", {"change_id": trial["id"]})
            settled = status()
            save("final-state", settled)
            if any(t["phase"] in ("written", "uncertain") for t in settled["state"]["runtime_trials"]):
                save("needs-recovery", {"reason": "Unresolved trial; keep engine alive", "data": str(DATA)})
        except BaseException as error:
            save("needs-recovery", {"reason": repr(error), "data": str(DATA)})
        mcp.stdin.close()
        try:
            mcp.wait(timeout=15)
        except subprocess.TimeoutExpired:
            mcp.terminate()
            mcp.wait(timeout=15)
    save("child", {"pid": mcp.pid if mcp else None, "exit_code": mcp.poll() if mcp else None,
                   "engine_closed_by_runner": False})
