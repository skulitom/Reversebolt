"""Exercise durable observation jobs on an explicitly selected, read-only target.

Run against an isolated Anode desktop engine: <data-dir> <pid> <phase>.
Phases: exercise, takeover-check, prepare-restart, restart-check.
Between exercise and takeover-check, invoke the desktop's Take control button.
Between prepare-restart and restart-check, terminate/restart only the test engine.
Never sends gameplay input, supplies semantic labels, or writes target memory.
"""
import json
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
DATA, PID, PHASE = Path(sys.argv[1]).resolve(), int(sys.argv[2]), sys.argv[3]
endpoint = json.loads((DATA / "endpoint.json").read_text(encoding="utf-8-sig"))
report_path = DATA / "jobs-report.json"
report = json.loads(report_path.read_text()) if report_path.exists() else {
    "scope": "Read-only job lifecycle on a paused independent game; no discovery or editing claim.",
    "checks": {}, "engine_pids": []}
if endpoint["pid"] not in report["engine_pids"]:
    report["engine_pids"].append(endpoint["pid"])


def record(name, value=True):
    report["checks"][name] = value
    report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(name, flush=True)


def http(action, args=None, source="agent"):
    request = urllib.request.Request(endpoint["url"] + "/command",
        data=json.dumps({"action": action, "args": args or {}, "source": source}).encode(),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + endpoint["token"]})
    with urllib.request.urlopen(request, timeout=20) as response:
        return json.load(response)


mcp = subprocess.Popen([str(ROOT / "target/release/reversebolt-mcp.exe"), "--data-dir", str(DATA)],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
    creationflags=0x08000000)
lines = queue.Queue()
threading.Thread(target=lambda: [lines.put(line) for line in mcp.stdout], daemon=True).start()
rpc_id = 0


def rpc(method, params):
    global rpc_id
    rpc_id += 1
    mcp.stdin.write(json.dumps({"jsonrpc": "2.0", "id": rpc_id, "method": method, "params": params}) + "\n")
    mcp.stdin.flush()
    while True:
        value = json.loads(lines.get(timeout=30))
        if value.get("id") == rpc_id:
            assert "error" not in value, value
            return value["result"]


def call(action, args=None, expected=True):
    result = rpc("tools/call", {"name": "reversebolt_" + action, "arguments": args or {}})
    value = json.loads(next(c["text"] for c in result["content"] if c["type"] == "text"))
    assert value["ok"] == expected, value.get("error")
    return value


def wait_for(predicate, timeout=15):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        state = http("status")
        if predicate(state):
            return state
        time.sleep(0.025)
    raise AssertionError("State did not arrive: " + json.dumps(state["jobs"]))


def envelope(extra):
    history = call("status")["jobs"]["history"]
    return {"request_id": history["clients"].get("mcp-agent", {}).get("high_water", 0) + 1,
        "boot": history["boot"], "binding": history["current"], **extra}


def start(count, interval):
    args = envelope({"count": count, "interval_ms": interval})
    result = call("start_observation_job", args)
    return result["data"]["ack"]["job"], args, result


def control(job, operation):
    return call("job_control", envelope({"job": job, "operation": operation}))


def job(state, ident):
    return state["jobs"]["history"]["jobs"][str(ident)]


try:
    rpc("initialize", {"protocolVersion": "2024-11-05", "capabilities": {},
        "clientInfo": {"name": "reversebolt-job-check", "version": "1"}})
    mcp.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
    mcp.stdin.flush()
    if PHASE == "exercise":
        names = {t["name"] for t in rpc("tools/list", {})["tools"]}
        assert len(names) == 23
        record("mcp_tools", len(names))
        programs = call("list_processes")["state"]["processes"]
        program = next(p for p in programs if p["pid"] == PID)
        args = {k: program[k] for k in ("pid", "created", "window")}
        args["budget_mib"] = 16
        args["auto_observe"] = False
        call("attach_process", args)
        ident, original, receipt = start(3, 250)
        call("detach_process", expected=False)
        done = wait_for(lambda s: job(s, ident)["phase"] == "Completed")
        assert done["state"]["live"]["sample_count"] == 3
        assert job(done, ident)["next_step"] == 3
        assert all(p["outcome"]["success"] for p in job(done, ident)["checkpoints"])
        record("three_native_captures", {"job": ident, "checkpoints": job(done, ident)["checkpoints"]})
        replay = call("start_observation_job", original)
        assert replay["data"]["duplicate"] and replay["data"]["ack"] == receipt["data"]["ack"]
        call("start_observation_job", {**original, "count": 2}, expected=False)
        assert call("status")["state"]["live"]["sample_count"] == 3
        record("exact_retry_and_collision")

        ident, _, _ = start(3, 1000)
        paused = control(ident, "pause")
        assert job(paused, ident)["phase"] == "Paused"
        time.sleep(1.2)
        assert call("status")["state"]["live"]["sample_count"] == 3
        record("pause_before_capture")
        control(ident, "resume")
        flight = wait_for(lambda s: s["jobs"]["in_flight"] is not None)
        then = time.perf_counter()
        responsive = call("status")
        status_ms = (time.perf_counter() - then) * 1000
        then = time.perf_counter()
        cancelled = control(ident, "cancel")
        cancel_ms = (time.perf_counter() - then) * 1000
        assert status_ms < 1000 and cancel_ms < 1000
        settled = wait_for(lambda s: job(s, ident)["phase"] == "Cancelled")
        count = settled["state"]["live"]["sample_count"]
        assert count == 4 and job(settled, ident)["next_step"] == 1
        time.sleep(1.2)
        assert call("status")["state"]["live"]["sample_count"] == count
        record("cancel_while_native_capture_owned", {"job": ident, "status_ms": status_ms,
            "cancel_ms": cancel_ms, "ack_phase": cancelled["data"]["ack"]["phase"],
            "final_phase": job(settled, ident)["phase"], "captures_after_cancel": count - 3,
            "observed_flight": flight["jobs"]["in_flight"],
            "status_during_flight": responsive["jobs"]["in_flight"] is not None})

        spoof = http("job_control", envelope({"operation": "take_control"}), source="desktop")
        assert not spoof["ok"] and "human authority" in spoof["error"]
        record("http_cannot_claim_desktop_authority", spoof["error"])
        ident, original, receipt = start(2, 60000)
        control(ident, "pause")
        detached = call("detach_process")
        assert job(detached, ident)["phase"] == "Interrupted"
        call("attach_process", args)
        replay = call("start_observation_job", original)
        assert replay["data"]["duplicate"] and replay["data"]["ack"] == receipt["data"]["ack"]
        assert call("status")["state"]["live"]["sample_count"] == 0
        record("rebind_invalidates_and_retry_stays_historical")
        page = call("job_events", {"cursor": 0})["data"]
        events = page["events"]
        while page["next"] < page["head"]:
            page = call("job_events", {"cursor": page["next"]})["data"]
            events += page["events"]
        assert all(a["cursor"] < b["cursor"] for a, b in zip(events, events[1:]))
        call("job_events", {"cursor": page["head"] + 1}, expected=False)
        record("ordered_event_pages", {"count": len(events), "head": page["head"]})
        ident, _, _ = start(2, 60000)
        report["ui_job"] = ident
        record("ready_for_desktop_takeover", ident)
    elif PHASE == "takeover-check":
        state = call("status")
        ident = report["ui_job"]
        assert state["jobs"]["history"]["human_hold"]["role"] == "Human"
        assert job(state, ident)["phase"] == "Paused" and job(state, ident)["next_step"] == 0
        assert job(state, ident)["controller"]["role"] == "Human"
        call("job_control", envelope({"job": ident, "operation": "resume"}), expected=False)
        call("capture", expected=False)
        call("start_observation_job", envelope({"count": 1, "interval_ms": 250}), expected=False)
        record("desktop_takeover_blocks_agent_mutations", {"job": ident, "history": state["jobs"]["history"]})
    elif PHASE == "prepare-restart":
        state = call("status")
        assert state["jobs"]["history"]["human_hold"] is None
        assert job(state, report["ui_job"])["phase"] == "Cancelled"
        ident, args, receipt = start(2, 60000)
        state = wait_for(lambda s: job(s, ident)["phase"] == "Running")
        report["restart"] = {"job": ident, "args": args, "ack": receipt["data"]["ack"],
            "boot": state["jobs"]["history"]["boot"], "head": state["jobs"]["history"]["event_head"]}
        record("ready_for_restart", {"job": ident, "phase": job(state, ident)["phase"]})
    elif PHASE == "restart-check":
        old = report["restart"]
        state = call("status")
        assert state["state"]["live"] is None
        assert state["jobs"]["history"]["boot"] == old["boot"] + 1
        assert state["jobs"]["history"]["current"] is None
        assert job(state, old["job"])["phase"] == "Interrupted"
        assert job(state, old["job"])["next_step"] == 0
        replay = call("start_observation_job", old["args"])
        assert replay["data"]["duplicate"] and replay["data"]["ack"] == old["ack"]
        request_id = state["jobs"]["history"]["clients"]["mcp-agent"]["high_water"] + 1
        call("start_observation_job", {**old["args"], "request_id": request_id}, expected=False)
        time.sleep(1.2)
        final = call("status")
        assert final["state"]["live"] is None and final["jobs"]["in_flight"] is None
        record("restart_interrupts_without_replay", final["jobs"])
    else:
        raise ValueError("Unknown phase")
finally:
    mcp.stdin.close()
    try:
        mcp.wait(timeout=5)
    except subprocess.TimeoutExpired:
        mcp.terminate()
        mcp.wait(timeout=5)
