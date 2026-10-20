"""Read-only native coverage check against an explicitly selected application.

Usage: python scripts/coverage-smoke.py <isolated-data-dir> <pid>
Leaves the selected application running and the second batch available for UI
inspection. Never supplies gameplay labels, addresses or writes to the engine.
"""
import json
from pathlib import Path
import sys
import urllib.request

root = Path(sys.argv[1])
pid = int(sys.argv[2])
endpoint = json.loads((root / "endpoint.json").read_text(encoding="utf-8-sig"))

def call(action, args=None):
    request = urllib.request.Request(endpoint["url"] + "/command",
        data=json.dumps({"action": action, "args": args or {}}).encode(),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + endpoint["token"]})
    with urllib.request.urlopen(request, timeout=120) as response:
        return json.load(response)

def good(action, args=None):
    result = call(action, args)
    assert result["ok"], result.get("error")
    return result["state"]["live"]

programs = call("list_processes")["state"]["processes"]
program = next(p for p in programs if p["pid"] == pid)
live = good("attach_process", {"pid": pid, "created": program["created"],
    "window": program["window"], "budget_mib": 16})
investigation = live["investigation_id"]
report = {"process": program, "investigation_id": investigation, "batches": [],
    "scope": "Read-only coverage on a static scene; no semantic or control claim."}
old_addresses = set()
for batch in (1, 2):
    captures = []
    batch_addresses = set()
    for number in range(1, 13):
        live = good("capture")
        assert live["sample_count"] == number
        assert live["coverage"]["batch"] == batch
        latest = live["latest"]
        assert latest["screen"] is not None, latest["screen_error"]
        batch_addresses.update(r["address"] for r in latest["memory_reads"])
        captures.append({k: latest[k] for k in ("number", "captured_bytes", "unreadable_bytes",
            "changed_mapping_bytes", "elapsed_ms", "memory_read_ms")})
        (root / f"batch-{batch:02}-capture-{number:02}.json").write_text(json.dumps(live), encoding="utf-8")
        print(json.dumps({"batch": batch, "capture": number, "bytes": latest["captured_bytes"],
            "changed": latest["changed_mapping_bytes"], "ms": latest["elapsed_ms"]}), flush=True)
    assert not old_addresses.intersection(batch_addresses), "Repeated earlier batch addresses"
    old_addresses.update(batch_addresses)
    correlation = call("correlate")
    report["batches"].append({"number": batch, "captures": captures,
        "coverage": live["coverage"], "retained_bytes": live["retained_bytes"],
        "correlation_ok": correlation["ok"], "correlation_error": correlation.get("error"),
        "phase": correlation["state"]["live"]["phase"]})
    if batch == 1:
        live = good("advance_memory", {"investigation_id": investigation, "batch": 1})
        assert live["retained_bytes"] == 0 and live["sample_count"] == 0
        assert len(live["coverage"]["batches"]) == 1
        stale = call("advance_memory", {"investigation_id": investigation, "batch": 1})
        assert not stale["ok"] and stale["state"]["live"]["coverage"]["batch"] == 2
        report["stale_advance_rejected"] = stale["error"]
report["distinct_chunk_addresses"] = len(old_addresses)
report["engine_pid"] = endpoint["pid"]
(root / "coverage-report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
print(json.dumps({"report": str(root / "coverage-report.json"),
    "read_unique_mib": live["coverage"]["read_unique_bytes"] / 1048576,
    "complete_mib": live["coverage"]["complete_trajectory_bytes"] / 1048576}), flush=True)
