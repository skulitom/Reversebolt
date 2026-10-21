"""Automated native ABI harness. Run inside the leased Anode session."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
BIN = ROOT / "target/runtime-probe/debug"
OUT = Path(sys.argv[1]).resolve()
OUT.mkdir(parents=True, exist_ok=False)
helper = BIN / "reversebolt_runtime_probe.dll"
digest = hashlib.sha256(helper.read_bytes()).hexdigest()
host_log = (OUT / "host.log").open("w")
host = subprocess.Popen([str(BIN / "mock-host.exe"), str(BIN / "reversebolt_mock_runtime.dll"), str(OUT / "identity.json"), str(OUT / "stop")], stdout=host_log, stderr=subprocess.STDOUT)
results = []
try:
    until = time.monotonic() + 10
    while not (OUT / "identity.json").exists():
        if host.poll() is not None or time.monotonic() > until:
            raise RuntimeError("owned host failed to start")
        time.sleep(0.05)
    identity = json.loads((OUT / "identity.json").read_text())
    base = [str(BIN / "probe-loader.exe"), str(identity["pid"]), identity["created"], str(identity["session"]), str(helper), digest]
    def call(label, args, success):
        result = subprocess.run(args, capture_output=True, text=True, timeout=130)
        (OUT / f"{label}.stdout.jsonl").write_text(result.stdout)
        (OUT / f"{label}.stderr.jsonl").write_text(result.stderr)
        receipt = {"label": label, "exit_code": result.returncode, "expected_success": success}
        results.append(receipt)
        assert (result.returncode == 0) == success, receipt
        if success:
            complete = json.loads(result.stdout.splitlines()[-1])
            assert complete["helper_absent"] and complete["inspection_exit"] == 0
        else:
            assert not result.stdout, "guard failure must precede instrumentation"
    args = base + [str(OUT / "wrong-identity.json")]
    args[2] = "0000000000000000"
    call("wrong-identity", args, False)
    args = base + [str(OUT / "wrong-session.json")]
    args[3] = str(identity["session"] + 1000)
    call("wrong-session", args, False)
    args = base + [str(OUT / "wrong-hash.json")]
    args[5] = "0" * 64
    call("wrong-hash", args, False)
    reports = []
    for i in range(2):
        file = OUT / f"probe-{i}.json"
        call(f"probe-{i}", base + [str(file)], True)
        report = json.loads(file.read_text())
        assert report["omissions"] == []
        fields = {f["name"]["text"]: f for f in report["fields"]}
        assert len(fields) == 8
        for name, expected in [("charge",37),("hiddenBias","1.75"),("enabled",{"boolean":True,"raw":1})]:
            assert all(s["value"] == expected for s in fields[name]["samples"])
        for name, reason in [("threadLocal","outside_static_storage_or_thread_static"),("constant","literal"),("instanceOnly","instance_object_required"),("reference","unsupported_type")]:
            assert fields[name]["reason"] == reason and fields[name]["status"] == "metadata_only"
        reports.append(fields)
        time.sleep(0.25)
    assert reports[1]["ticks"]["samples"][0]["value"] > reports[0]["ticks"]["samples"][0]["value"]
    call("existing-output", base + [str(OUT / "probe-0.json")], False)
    print(json.dumps({"passed":True,"cases":results,"identity":identity}))
finally:
    (OUT / "stop").touch()
    try:
        host.wait(timeout=5)
    except subprocess.TimeoutExpired:
        host.terminate()
        host.wait(timeout=5)
    host_log.close()
    (OUT / "cases.json").write_text(json.dumps({"cases":results,"host_pid":host.pid,"host_exit_code":host.returncode},indent=2))
