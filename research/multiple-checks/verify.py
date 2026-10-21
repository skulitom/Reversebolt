"""Post-run artifact audit; never starts or controls the product or fixture."""
import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
spec = importlib.util.spec_from_file_location("alignment", HERE.parent / "capture-alignment/verify.py")
alignment = importlib.util.module_from_spec(spec)
spec.loader.exec_module(alignment)


def audit(case, config):
    directory = ROOT / f"test-results/multiple-checks-v1/case-{case}"
    load = lambda n: read(directory / n)
    report = load("report.json")
    assert not report["errors"]
    assert report["protocol_sha256"] == sha(HERE / "protocol-v1.json")
    states = load("states.json")
    assert [s["live"]["sample_count"] for s in states] == list(range(1, 17))
    frozen = states[11]["live"]["prediction_checks"]
    discovery = load("discovery-status.json")["state"]["live"]
    checks = discovery["prediction_checks"]
    tracks = {t["id"]: t for t in discovery["tracks"]}
    ordered = sorted(checks, key=lambda c: tracks[c["track"]]["bounds"][1])
    assert len(checks) == len(frozen) == len({c["id"] for c in checks}) == 2
    assert sum(len(c["candidates"]) for c in checks) <= 200
    compact = []
    for field, check in enumerate(ordered):
        initial = next(c for c in frozen if c["id"] == check["id"])
        assert check["after_frame"] == 12 and check["invalidated"] is None
        assert len(check["automatic_selection"]["planned_tracks"]) == 2
        assert tracks[check["track"]]["samples"] == [v[field] for v in config["values"]]
        candidates = []
        for before, candidate in zip(initial["candidates"], check["candidates"]):
            assert before["samples"] == [] and before["mapping"] == candidate["mapping"]
            assert [s["frame"] for s in candidate["samples"]] == [13, 14, 15, 16]
            samples = []
            for sample in candidate["samples"]:
                assert alignment.bracketed(sample) and sample["outcome"] == "agrees"
                assert sample["observed"] == config["values"][sample["frame"] - 1][field]
                samples.append({**sample, "image": {k: v for k, v in sample["image"].items() if k != "input_context"}})
            candidates.append({"mapping": candidate["mapping"], "address": candidate["address"], "samples": samples})
        compact.append({"id": check["id"], "track": check["track"], "label": check["label"],
            "automatic_selection": check["automatic_selection"], "candidates": candidates})
    for state in states[12:]:
        reads = state["live"]["latest"]["prediction_reads"]
        pairs = {(r["address"], r["requested"]) for r in reads}
        assert len(pairs) == len(reads) <= 200
        assert all(0 < r["requested"] <= 8 for r in reads)
    plans = [load(f"prepared-{field}.json")["data"] for field in [0, 1]]
    for field, plan in enumerate(plans):
        assert plan["scope"]["prediction"] == ordered[field]["id"]
        assert plan["scope"]["track"] == ordered[field]["track"]
        assert plan["desired"] == config["values"][-1][field] + 3
        assert plan["probe"]["selected"] == 0
        address = report["prepared_oracle"]["fields"][field]
        assert address <= plan["patch"]["address"]
        assert plan["patch"]["address"] + len(plan["patch"]["after"]) <= address + 4
    applied, checked, expired, restored = [load(n) for n in ["applied.json", "checked.json", "other-expired.json", "restored.json"]]
    assert applied["id"] == plans[case]["id"] and applied["phase"] == "written"
    assert applied["write"]["attempted"] and applied["write"]["error"] is None
    assert applied["write"]["observed_after"] == plans[case]["patch"]["after"]
    assert checked["observations"][-1]["intervention_outcome"] == "matched"
    assert expired["id"] == plans[1 - case]["id"] and expired["phase"] == "expired" and expired["write"] is None
    assert restored["id"] == applied["id"] and restored["phase"] == "restored"
    assert restored["restoration"]["attempted"] and restored["restoration"]["error"] is None
    expected = config["values"][-1].copy(); expected[case] += 3
    assert report["applied_oracle"]["values"] == expected
    assert report["restored_oracle"]["values"] == config["values"][-1]
    final = load("final-status.json")
    assert final["state"]["live"]["sample_count"] == 16
    assert all(c["invalidated"] for c in final["state"]["live"]["prediction_checks"])
    commands = load("commands.json")
    assert not any(c["name"] in ["capture", "correlate", "start_prediction_check"] for c in commands)
    assert sum(c["name"] == "prepare_probe" for c in commands) == 2
    assert sum(c["name"] == "apply_change" for c in commands) == 2
    assert sum(c["name"] == "restore_change" for c in commands) == 1
    return {"case": case, "report": report, "checks": compact,
        "trial": {"selected_check": plans[case]["scope"]["prediction"], "selected_track": plans[case]["scope"]["track"],
            "write": applied["write"], "intervention_outcome": checked["observations"][-1]["intervention_outcome"],
            "other_plan": {"phase": expired["phase"], "reason": expired["reason"], "write": expired["write"]},
            "restoration": restored["restoration"]},
        "files": {p.name: sha(p) for p in sorted(directory.glob("*.json"))}}


def main():
    config = read(HERE / "protocol-v1.json")
    for name, digest in config["method_sha256"].items(): assert sha(HERE / name) == digest
    for name, digest in config["product_source_sha256"].items(): assert sha(HERE / "source" / name) == digest
    for name, digest in config["binaries"].items(): assert sha(Path(config["binary_directory"]) / name) == digest
    result = {"audit": "post-run; not preregistered", "protocol_sha256": sha(HERE / "protocol-v1.json"),
        "binaries": config["binaries"], "cases": [audit(case, config) for case in [0, 1]]}
    (HERE / "evidence.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"cases": [{"case": c["case"], "checks": len(c["checks"]), "trial": c["trial"]} for c in result["cases"]]}))


if __name__ == "__main__":
    main()
