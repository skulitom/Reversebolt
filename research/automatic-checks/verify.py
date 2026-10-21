"""Post-run artifact audit. Does not control games or inspect screenshots."""
import collections
import hashlib
import importlib.util
import json
import struct
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
RAW = ROOT / "test-results/automatic-checks-v1"
read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
config = read(HERE / "protocol-v1.json")
for name, digest in config["method_sha256"].items():
    assert sha(HERE / name) == digest, name
for name, digest in config["product_source_sha256"].items():
    assert sha(HERE / "source" / name) == digest, name
for name, digest in config["binaries"].items():
    assert sha(Path(config["binary_directory"]) / name) == digest, name
spec = importlib.util.spec_from_file_location("alignment_audit", HERE.parent / "capture-alignment/verify.py")
alignment = importlib.util.module_from_spec(spec)
spec.loader.exec_module(alignment)


def audit(name):
    directory = RAW / name
    fixture = name == "fixture"
    run = read(directory / ("report.json" if fixture else "run.json"))
    assert not run["errors"], run["errors"]
    assert run["protocol_sha256" if fixture else "started_protocol_sha256"] == sha(HERE / "protocol-v1.json")
    states = read(directory / "states.json")
    if not fixture:
        states = [s["state"] for s in states]
    final = read(directory / "final-status.json")["state"]
    live = final["live"]
    assert [s["live"]["sample_count"] for s in states] == list(range(1, 17))
    assert not final["changes"]
    commands = read(directory / "commands.json")
    forbidden = {"capture", "correlate", "start_prediction_check", "start_observation_job", "prepare_change", "prepare_probe", "apply_change", "restore_change", "set_value"}
    assert not any(c["name"] in forbidden for c in commands)
    for frame in live["frames"]:
        images = list((directory / "engine/captures").rglob(f"frame-{frame['number']:03d}.png"))
        assert len(images) == 1 and sha(images[0]) == frame["sha256"]
    predictions = []
    check = live["prediction_check"]
    if check:
        initial = states[11]["live"]["prediction_check"]
        assert initial and initial["id"] == check["id"] and check["after_frame"] == 12
        assert initial["automatic_selection"] == check["automatic_selection"]
        assert check["automatic_selection"]["scheduled_fresh_captures"] == 4
        assert check["automatic_selection"]["ranked"][0]["track"] == check["track"]
        assert all(not c["samples"] for c in initial["candidates"])
        for before, candidate in zip(initial["candidates"], check["candidates"], strict=True):
            assert before["mapping"] == candidate["mapping"] and before["address"] == candidate["address"]
            assert [s["frame"] for s in candidate["samples"]] == [13, 14, 15, 16]
            for sample in candidate["samples"]:
                if sample["bytes"] is not None and sample["predicted"] is not None:
                    mapping = candidate["mapping"]
                    number = struct.unpack({"I32":"<i", "F32":"<f", "F64":"<d"}[mapping["kind"]], bytes(sample["bytes"]))[0]
                    assert abs(number*mapping["slope"]+mapping["intercept"]-sample["predicted"]) < 1e-8
                expected = ("missing" if sample["predicted"] is None else
                    "timing_uncertain" if not alignment.bracketed(sample) else
                    "pending" if sample["observed"] is None else
                    "agrees" if abs(sample["predicted"]-sample["observed"]) <= check["tolerance"] else "contradicts")
                assert sample["outcome"] == expected
            good = [s for s in candidate["samples"] if s["outcome"] == "agrees"]
            deltas = [abs(b["observed"]-a["observed"]) for a,b in zip(good,good[1:]) if b["frame"] == a["frame"]+1]
            assessment = ("invalidated" if check["invalidated"] or candidate["invalidated"] else
                "contradicted" if any(s["outcome"] == "contradicts" for s in candidate["samples"]) else
                "consistent_in_observed_scope" if len(good) >= 4 and any(d <= .55 for d in deltas) and any(d > 1.1 for d in deltas) else "needs_contrasting_evidence")
            predictions.append({"address":candidate["address"],"mapping":candidate["mapping"],"assessment_replayed":assessment,"samples":candidate["samples"]})
    if fixture:
        assert check and len(predictions) == 2
        assert all(p["assessment_replayed"] == "consistent_in_observed_scope" for p in predictions)
        scores, pulses = None, None
    else:
        source = run["source"]
        for filename, digest in source["files"].items():
            assert sha(ROOT / "test-results/independent-games-v1/upstream" / run["game"] / filename) == digest
        scope = read(directory / "scope-selection.json")
        chosen = sorted(scope["candidates"],key=lambda c:(-c["delta"],c["id"]))[0]
        assert chosen["id"] == live["memory_process"]["pid"]
        injected = [json.loads(line) for line in (directory / "input.jsonl").read_text(encoding="utf-8").splitlines()]
        injected = [p for p in injected if p["kind"] == "pulse"]
        delivered = [p for p in read(directory / "delivered-keys.json") if p["kind"] == "down"]
        keys = {32:" ",37:"ArrowLeft",38:"ArrowUp",39:"ArrowRight",40:"ArrowDown"}
        assert [keys[p["vk"]] for p in injected] == [p["key"] for p in delivered]
        assert all(p["trusted"] and not p["repeat"] for p in delivered)
        pulses = len(injected)
        score = read(directory / "score.json")
        scores = {k:v["outcomes"] for k,v in score["counters"].items()}
    return {"case":name,"captures":16,"check_started":check is not None,
        "selection":check["automatic_selection"] if check else None,
        "selected_track":check["track"] if check else None,
        "selected_label":check["label"] if check else None,
        "predictions":predictions,"assessments":dict(collections.Counter(p["assessment_replayed"] for p in predictions)),
        "observations":dict(collections.Counter(s["outcome"] for p in predictions for s in p["samples"])),
        "scores":scores,"input_pulses_audited":pulses,
        "artifact_sha256":{p.name:sha(p) for p in directory.glob("*.json") if p.name != "endpoint.json"}}


receipts = [audit(name) for name in ["fixture", "2048-1", "tetris-1"]]
(HERE / "evidence").mkdir(exist_ok=True)
(HERE / "evidence/v1.json").write_text(json.dumps(receipts,indent=2)+"\n",encoding="utf-8")
print(json.dumps([{k:v for k,v in r.items() if k not in ["predictions","artifact_sha256"]} for r in receipts],indent=2))
