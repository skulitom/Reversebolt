"""Post-run audit of frozen V1/V2 runs; never calls or controls the product/game.

Raw evidence stays in test-results. This checks retained predictions and native
receipts, not unavailable training-memory snapshots or unobserved game effects.
"""
import collections
import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


spec = importlib.util.spec_from_file_location("alignment", HERE.parent / "capture-alignment/verify.py")
alignment = importlib.util.module_from_spec(spec)
spec.loader.exec_module(alignment)


def audit(version, case, config):
    directory = ROOT / f"test-results/game-location-probes-v{version}/tetris-{case}"
    load = lambda name: read(directory / name)
    run = load("run.json")
    assert run["started_protocol_sha256"] == sha(HERE / f"protocol-v{version}.json")
    assert run["binaries"] == config["binaries"] and not run["errors"]
    for name, digest in run["source"]["files"].items():
        assert sha(ROOT / "test-results/independent-games-v1/upstream/tetris" / name) == digest
    selection = load("scope-selection.json")
    before = {p["id"]: p["cpuTime"] for p in selection["cpuBefore"]}
    identities = {p["pid"]: p for p in selection["sources"]["processes"]}
    choices = sorted(((p["cpuTime"] - before.get(p["id"], p["cpuTime"]), p["id"])
        for p in selection["cpuAfter"] if p["type"] == "renderer" and p["id"] in identities),
        key=lambda p: (-p[0], p[1]))
    assert choices[0][0] > 0 and choices[0][1] == selection["chosen"]["id"]
    states = load("states.json")
    assert [s["state"]["live"]["sample_count"] for s in states] == list(range(1, 17))
    frozen = load("frozen-status.json")["state"]["live"]["prediction_check"]
    live = load("before-trial-status.json")["state"]["live"]
    check = live["prediction_check"]
    assert live["memory_process"] == identities[choices[0][1]]
    assert check["id"] == frozen["id"] and check["after_frame"] == 12
    assert check["automatic_selection"] and check["invalidated"] is None
    assert len(check["candidates"]) == len(frozen["candidates"])
    candidates = []
    for index, (old, current) in enumerate(zip(frozen["candidates"], check["candidates"])):
        assert old["mapping"] == current["mapping"] and old["address"] == current["address"]
        assert [s["frame"] for s in current["samples"]] == [13, 14, 15, 16]
        samples = []
        for sample in current["samples"]:
            if sample["outcome"] in ("agrees", "contradicts"):
                assert alignment.bracketed(sample)
            image = {k: v for k, v in (sample.get("image") or {}).items() if k != "input_context"}
            samples.append({**sample, "image": image})
        candidates.append({"index": index, "address": current["address"],
            "mapping": current["mapping"], "samples": samples})
    commands = load("commands.json")
    names = collections.Counter(c["name"] for c in commands)
    assert set(names) <= {"list_processes", "set_observer_model", "attach_process", "list_memory_processes",
        "job_control", "prepare_change", "prepare_probe", "apply_change", "observe_change", "restore_change"}
    assert names["prepare_change"] == names["prepare_probe"] == 1
    assert names["apply_change"] <= 1 and names["restore_change"] <= 1
    probe = next(c for c in commands if c["name"] == "prepare_probe")
    selected = 0 if case == 1 else len(candidates) - 1
    assert probe["args"]["candidate"] == selected and probe["args"]["prediction_id"] == check["id"]
    latest = candidates[selected]["samples"][-1]["observed"]
    assert probe["args"]["value"] == latest + config["requested_delta"]
    events = [json.loads(line) for line in (directory / "input.jsonl").read_text(encoding="utf-8-sig").splitlines()]
    pulses = [e for e in events if e["kind"] == "pulse"]
    delivered = load("delivered-keys.json")
    downs = [e for e in delivered if e["kind"] == "down"]
    ups = [e for e in delivered if e["kind"] == "up"]
    keys = {32: " ", 37: "ArrowLeft", 38: "ArrowUp", 39: "ArrowRight", 40: "ArrowDown"}
    expected = [keys[p["vk"]] for p in pulses]
    assert len(pulses) == run["injected_pulses"]
    assert [e["key"] for e in downs] == [e["key"] for e in ups] == expected
    assert all(e["trusted"] and not e["repeat"] for e in downs)
    assert all(p["down_ms"] < p["up_ms"] for p in pulses)
    assert all(a["up_ms"] <= b["down_ms"] for a, b in zip(pulses, pulses[1:]))
    if version == 2:
        assert len(pulses) == 113
        complete = [e for e in events if e["kind"] == "burst_complete"]
        assert [e["generation"] for e in complete] == list(range(13))
        assert all(not any(p["down_ms"] < e["time_ms"] < p["up_ms"] for p in pulses) for e in complete)
        controls = [c for c in commands if c["name"] == "job_control"]
        assert [c["args"]["operation"] for c in controls] == ["pause", "resume"] * 15
        assert all(c["ok"] for c in controls)
    else:
        modes = [e for e in events if e["kind"] == "mode"]
        assert [e["paused"] for e in modes] == [False, True, False, True]
        assert not any(modes[1]["time_ms"] < p["down_ms"] < modes[2]["time_ms"] for p in pulses)
    prepared = load("prepared.json")
    stages = {}
    for name in ("prepared", "applied", "checked", "restored"):
        if not (directory / f"{name}.json").exists():
            continue
        response = load(f"{name}.json")
        change = response.get("data")
        stages[name] = {"ok": response["ok"], "error": response["error"]}
        if change:
            stages[name].update({k: change[k] for k in ("id", "phase", "reason", "observed", "desired", "patch", "write", "restoration")})
            stages[name]["observations"] = [{k: o.get(k) for k in ("stage", "value", "expected", "agrees", "error", "image_sha256")}
                for o in change["observations"]]
            if name == "prepared":
                assert change["probe"]["selected"] == selected
                assert change["scope"]["process"] == live["memory_process"]
                assert 1 <= len(change["patch"]["after"]) <= 8
    if version == 1:
        assert not prepared["ok"] and names["apply_change"] == names["restore_change"] == 0
        outcome = "abstained: insufficient fresh contrast"
    elif case == 1:
        assert stages["applied"]["phase"] == "refused" and stages["applied"]["write"] is None
        assert "bytes changed" in stages["applied"]["reason"] and names["restore_change"] == 0
        outcome = "application refused: bytes changed before native entry"
    else:
        receipt = stages["applied"]["write"]
        assert receipt["attempted"] and receipt["error"] is None and receipt["transferred"] == 1
        assert receipt["observed_after"] == stages["prepared"]["patch"]["after"]
        assert stages["restored"]["phase"] == "written" and stages["restored"]["restoration"] is None
        assert "bytes changed" in stages["restored"]["reason"]
        assert all(not o["agrees"] and o["value"] == 150 for o in stages["restored"]["observations"])
        outcome = "one confirmed byte write; no observed score response; restoration refused"
    oracle = load("oracle.json")
    timeline = load("trial-stages.json")
    windows = []
    for start, end in zip(timeline, timeline[1:]):
        rows = [o for o in oracle if start["time_ms"] <= o["time_ms"] <= end["time_ms"]]
        windows.append({"from": start["stage"], "to": end["stage"], "samples": len(rows),
            "actual": sorted({o.get("actual_score") for o in rows}, key=str),
            "display": sorted({o.get("display_score") for o in rows}, key=str),
            "dom": sorted({o["values"]["score"]["value"] for o in rows}, key=str),
            "playing": sorted({o.get("playing") for o in rows}, key=str)})
    if version == 2 and case == 2:
        assert all(w["actual"] == w["display"] == w["dom"] == [150] and w["playing"] == [False]
            for w in windows if w["samples"])
    score = load("score.json")
    return {"case": case, "outcome": outcome, "unrevised_runner_label": run["trialOutcome"],
        "selected_candidate": selected, "memory_process": live["memory_process"],
        "captures": len(states), "pulses_and_trusted_keydowns": len(pulses),
        "ocr": {k: v["outcomes"] for k, v in score["counters"].items()},
        "candidates": candidates, "stages": stages, "oracle_operation_windows": windows,
        "files": {p.name: sha(p) for p in sorted(directory.glob("*.json"))},
        "input_files": {p.name: sha(p) for p in sorted(directory.glob("*.jsonl"))}}


def main():
    (HERE / "evidence").mkdir(exist_ok=True)
    for version in [1, 2]:
        protocol = HERE / f"protocol-v{version}.json"
        config = read(protocol)
        for name, digest in config["method_sha256"].items():
            assert sha(HERE / name) == digest, name
        for name, digest in config["binaries"].items():
            assert sha(Path(config["binary_directory"]) / name) == digest
        assert sha(HERE / config["product_evidence"]) == config["product_evidence_sha256"]
        assert sha(ROOT / "test-results/independent-games-v1/upstream/manifest.json") == config["upstream_manifest_sha256"]
        if version == 2:
            assert sha(HERE / "protocol-v1.json") == config["parent_protocol_sha256"]
        result = {"audit": "post-run; not a preregistered evaluator", "protocol_sha256": sha(protocol),
            "binaries": config["binaries"], "cases": [audit(version, case, config) for case in [1, 2]]}
        (HERE / "evidence" / f"v{version}.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        print(json.dumps({"version": version, "outcomes": [c["outcome"] for c in result["cases"]]}))


if __name__ == "__main__":
    main()
