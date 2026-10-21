"""Post-run audit; validates saved endpoint evidence without starting applications.

Preserves protocol failures rather than converting unavailable readings to passes.
The raw cases are local under test-results; compact receipts retain their hashes.
"""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()


def audit(case):
    directory = ROOT / f"test-results/intervention-persistence-v1/case-{case}"
    report = read(directory / "report.json")
    assert report["protocol_sha256"] == sha(HERE / "protocol-v1.json")
    prepared = read(directory / "prepared.json")["data"]
    assert prepared["probe"]["selected"] == 0
    assert prepared["observed"] == 50 and prepared["desired"] == 53
    commands = read(directory / "commands.json")
    assert sum(c["name"] == "apply_change" for c in commands) == 1
    assert sum(c["name"] == "restore_change" for c in commands) == 1
    assert sum(c["name"] == "prepare_probe" for c in commands) == 1
    records = [(p.name, read(p)) for p in sorted(directory.glob("*.json"))
        if p.name in ["applied.json", "checked.json", "restored.json", "checked-restore.json"]]
    summaries = []
    for name, change in records:
        assert change["models"] == prepared["models"]
        assert change["write"]["attempted"] and change["write"]["error"] is None
        assert change["write"]["observed_after"] == prepared["patch"]["after"]
        observations = []
        for observation in change["observations"]:
            image = Path(observation["image"])
            assert sha(image) == observation["image_sha256"]
            samples = observation["intervention_memory"]
            assert len(samples) == len(prepared["models"]) <= 200
            expected_samples = []
            for sample, model in zip(samples, prepared["models"]):
                assert sample["model"] == model["index"] and sample["address"] == model["address"]
                expected = model["before"].copy()
                if observation["stage"] == "after_edit":
                    patch = prepared["patch"]
                    for index in range(len(expected)):
                        offset = model["address"] + index - patch["address"]
                        if 0 <= offset < len(patch["after"]): expected[index] = patch["after"][offset]
                assert sample["expected"] == expected and 0 < len(expected) <= 8
                first, last = sample["before"], sample["after"]
                assert first["error"] is None and last["error"] is None
                a, b, image_time = first["interval"], last["interval"], observation["capture_interval"]
                assert a["clock_id"] == b["clock_id"] == image_time["clock_id"]
                assert a["begin_us"] <= a["end_us"] <= image_time["begin_us"] <= image_time["end_us"] <= b["begin_us"] <= b["end_us"]
                expected_samples.append(first["bytes"] == last["bytes"] == expected)
            if not all(expected_samples):
                outcome = "memory_changed"
            elif observation["error"] or observation["value"] is None:
                outcome = "visual_unavailable"
            else:
                assert observation["agrees"] == (abs(observation["value"] - observation["expected"]) <= .55)
                outcome = "matched" if observation["agrees"] else "did_not_follow"
            assert outcome == observation["intervention_outcome"]
            if outcome not in ["matched", "did_not_follow"]:
                assert all(c["fit"] == "unavailable" for c in observation["probe_comparisons"])
            observations.append({k: observation[k] for k in ["stage", "value", "expected", "agrees", "error",
                "image_sha256", "capture_interval", "intervention_memory", "intervention_outcome", "probe_comparisons"]})
        summaries.append({"file": name, "phase": change["phase"], "reason": change["reason"],
            "write": change["write"], "restoration": change["restoration"], "observations": observations})
    checked = read(directory / "checked.json")
    expected = ["matched", "did_not_follow", "memory_changed"][case]
    assert checked["observations"][-1]["intervention_outcome"] == expected
    if case == 2:
        restoration = read(directory / "restored.json")
        assert restoration["phase"] == "written" and not restoration["restoration"]["attempted"]
        assert report["checked_oracle"]["values"] == [47, 47]
    else:
        assert read(directory / "restored.json")["phase"] == "restored"
        assert report["restored_oracle"]["values"] == [50, 50]
    return {"case": case, "report": report, "expected_check": expected,
        "full_case_passed": not report["errors"], "stages": summaries,
        "files": {p.name: sha(p) for p in sorted(directory.glob("*.json"))}}


def main():
    config = read(HERE / "protocol-v1.json")
    for name, digest in config["method_sha256"].items(): assert sha(HERE / name) == digest
    for name, digest in config["product_source_sha256"].items(): assert sha(HERE / "source" / name) == digest
    for name, digest in config["binaries"].items(): assert sha(Path(config["binary_directory"]) / name) == digest
    result = {"audit": "post-run; failures retained", "protocol_sha256": sha(HERE / "protocol-v1.json"),
        "binaries": config["binaries"], "cases": [audit(case) for case in [0, 1, 2]]}
    (HERE / "evidence.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"cases": [{"case": c["case"], "check": c["expected_check"], "full_case_passed": c["full_case_passed"],
        "errors": c["report"]["errors"]} for c in result["cases"]]}))


if __name__ == "__main__":
    main()
