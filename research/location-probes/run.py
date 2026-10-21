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
assert slot in [0, 1]
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


def wait_phase(phase):
    deadline = time.time() + 240
    while time.time() < deadline:
        status = call("status")
        change = next(c for c in status["state"]["changes"] if c["id"] == change_id)
        if change["phase"] == phase:
            return change
        assert change["phase"] not in ["uncertain", "refused", "expired"], change["phase"]
        time.sleep(.3)
    raise RuntimeError("UI phase deadline expired; no operation was replayed")


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
    call("attach_process", {**{k: target[k] for k in ["pid", "created", "window"]}, "auto_observe": False, "budget_mib": 32})
    for number, value in enumerate(config["values"], 1):
        control.write_text(str(value), encoding="ascii")
        deadline = time.time() + 4
        while oracle()["values"] != [value, value] and time.time() < deadline:
            time.sleep(.05)
        assert oracle()["values"] == [value, value]
        time.sleep(.15)
        state = call("capture")["state"]
        assert state["live"]["sample_count"] == number
        states.append(state); save("states.json", states)
        if number == 12:
            live = call("correlate")["state"]["live"]
            save("training.json", live)
            eligible = sorted((t for t in live["tracks"] if t["active"] and t["matches"]), key=lambda t: t["id"])
            assert eligible, "No matching OCR series; no replacement selection"
            track = eligible[0]["id"]
            call("start_prediction_check", {"investigation_id": live["investigation_id"], "batch": live["coverage"]["batch"],
                "after_frame": 12, "source": "ocr", "track": track})
        print(json.dumps({"slot": slot, "captured": number}), flush=True)
    live = state["live"]
    save("fresh.json", live)
    args = {"investigation_id": live["investigation_id"], "batch": live["coverage"]["batch"], "track": track, "value": 53.}
    ordinary = call("prepare_change", args, False); save("ordinary.json", ordinary)
    prepared = call("prepare_probe", {**args, "prediction_id": live["prediction_check"]["id"], "candidate": 0}, False)
    save("prepared.json", prepared)
    if not prepared["ok"] or prepared["data"]["phase"] != "prepared":
        report["outcome"] = "probe unavailable; no replacement or write"
    else:
        change = prepared["data"]; change_id = change["id"]; patch = change["patch"]
        truth = oracle()
        fields = truth["fields"]
        selected_field = next((i for i, address in enumerate(fields) if address <= patch["address"] and
            patch["address"] + len(patch["after"]) <= address + 4), None)
        assert selected_field is not None, "Selected patch outside owned fixture fields; no alternative selected"
        report["selected_field"] = selected_field
        report["prepared_oracle"] = truth
        assert all(x == 50 for x in truth["values"]), "Preparation changed the fixture"
        if slot == 1:
            save("ui-ready.json", {"engine_pid": engine.pid, "change_id": change_id, "requested": 53, "candidate": 0})
            print("UI_READY: set Try value to 53, then invoke Try test for the prepared plan.", flush=True)
            applied = wait_phase("written")
        else:
            applied = call("apply_change", {"change_id": change_id})["data"]
        assert applied["phase"] == "written"
        written = True; save("applied.json", applied)
        time.sleep(.2)
        checked = call("observe_change", {"change_id": change_id})["data"]; save("checked.json", checked)
        truth = oracle(); report["applied_oracle"] = truth
        assert truth["values"][selected_field] == 53 and truth["values"][1-selected_field] == 50
        expected = 53 if selected_field == slot else 50
        assert checked["observations"][-1]["value"] == expected
        assert checked["observations"][-1]["probe_comparisons"]
        replay = call("apply_change", {"change_id": change_id})["data"]
        assert replay["write"] == applied["write"]
        if slot == 1:
            print("UI_RESTORE_READY: invoke Restore bytes.", flush=True)
            restored_change = wait_phase("restored")
        else:
            restored_change = call("restore_change", {"change_id": change_id})["data"]
        assert restored_change["phase"] == "restored"
        restored = True; save("restored.json", restored_change)
        time.sleep(.2)
        checked_restore = call("observe_change", {"change_id": change_id})["data"]; save("checked-restore.json", checked_restore)
        report["restored_oracle"] = oracle()
        assert report["restored_oracle"]["values"] == [50, 50]
        assert checked_restore["observations"][-1]["value"] == 50
        report["outcome"] = "tested and restored; source affected" if selected_field == slot else "tested and restored; copy had no visible effect"
    save("final-status.json", call("status"))
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
