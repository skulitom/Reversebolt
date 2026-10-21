"""Audit saved runs and emit compact, reproducible receipts (no screenshots opened).

Usage: python verify.py v1|v2
Raw captures stay local under test-results; compact receipts have no endpoint tokens.
This audits retained predictions, not the unavailable raw training-memory fit.
"""
import collections
import hashlib
import json
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def contrast(values):
    values = sorted(v for v in values if v is not None)
    return len(values) >= 4 and values[-2] - values[1] > 2.2


def audit(directory, protocol):
    run, training, final, score, selection, commands = [read(directory / name) for name in
        ["run.json", "training-status.json", "final-status.json", "score.json", "scope-selection.json", "commands.json"]]
    config = read(protocol)
    assert run["started_protocol_sha256"] == sha(protocol)
    assert run["binaries"] == config["binaries"]
    assert not run["errors"] and training["state"]["live"]["sample_count"] == 12
    assert final["state"]["live"]["sample_count"] == 16
    source = ROOT / "test-results/independent-games-v1/upstream" / run["game"]
    for name, digest in run["source"]["files"].items():
        assert sha(source / name) == digest
    for name, digest in config["pre_capture_method_sha256"].items():
        assert sha(HERE / name) == digest
    for name, digest in config.get("product_source_sha256", {}).items():
        assert name == "crates/reversebolt-core/src/discovery.rs"
        assert sha(HERE / "evidence/discovery-v2.rs") == digest
    before = {p["id"]: p["cpuTime"] for p in selection["cpuBefore"]}
    identities = {p["pid"]: p for p in selection["sources"]["processes"]}
    choices = sorted(((p["cpuTime"] - before.get(p["id"], p["cpuTime"]), p["id"])
        for p in selection["cpuAfter"] if p["type"] == "renderer" and p["id"] in identities),
        key=lambda p: (-p[0], p[1]))
    assert choices[0][0] > 0 and choices[0][1] == selection["chosen"]["id"]
    selected = identities[choices[0][1]]
    train, live = training["state"]["live"], final["state"]["live"]
    assert train["memory_process"] == live["memory_process"] == selected
    assert selected["pid"] != run["selected"]["pid"]
    assert all(c["ok"] for c in commands)
    expected = ["list_processes", "set_observer_model", "attach_process", "list_memory_processes",
                "attach_process", "start_observation_job"]
    eligible = sorted((t for t in train["tracks"] if t["active"] and t["matches"]), key=lambda t: t["id"])
    if eligible:
        expected.append("start_prediction_check")
    assert [c["name"] for c in commands] == expected + ["start_observation_job", "advance_memory"]
    assert commands[1]["args"] == {"model": None}
    jobs = [c["args"] for c in commands if c["name"] == "start_observation_job"]
    assert [(j["count"], j["compare"], j["mode"]) for j in jobs] == [(12, True, "full"), (4, False, "full")]
    pulses = [p for line in (directory / "input.jsonl").read_text(encoding="utf-8").splitlines()
              if (p := json.loads(line))["kind"] == "pulse"]
    delivered = read(directory / "delivered-keys.json")
    schedule = config["input"][run["game"]]
    names = {32: " ", 37: "ArrowLeft", 38: "ArrowUp", 39: "ArrowRight", 40: "ArrowDown"}
    assert len(delivered) == len(pulses) * 2 == run["injected_pulses"] * 2
    for i, pulse in enumerate(pulses):
        vk = schedule["prefix"][i] if i < len(schedule["prefix"]) else schedule["keys"][i % len(schedule["keys"])]
        assert pulse["index"] == i and pulse["vk"] == vk
        down, up = delivered[2*i:2*i+2]
        assert (down["kind"], up["kind"]) == ("down", "up")
        assert down["key"] == up["key"] == names[vk]
        assert down["trusted"] and up["trusted"] and not down["repeat"]
        assert down["time_ms"] <= up["time_ms"]
    predictions = []
    if eligible:
        frozen = read(directory / "frozen-status.json")["state"]["live"]["prediction_check"]
        check = live["prediction_check"]
        assert check["track"] == eligible[0]["id"] and check["after_frame"] == 12
        assert check["id"] == frozen["id"] and check["invalidated"] is None
        assert check["basis"] == frozen["basis"]
        assert [f["number"] for f in check["basis"]] == list(range(1, 13))
        assert len(check["candidates"]) == len(frozen["candidates"]) == len(eligible[0]["matches"])
        for old, candidate in zip(frozen["candidates"], check["candidates"]):
            assert old["samples"] == []
            assert all(old[k] == candidate[k] for k in ["address", "chunk_address", "mapping", "offset_in_chunk"])
            assert [s["frame"] for s in candidate["samples"]] == [13, 14, 15, 16]
            samples = []
            for sample in candidate["samples"]:
                m = candidate["mapping"]
                if sample["bytes"] is not None:
                    value = struct.unpack({"I32": "<i", "F32": "<f", "F64": "<d"}[m["kind"]], bytes(sample["bytes"]))[0]
                    predicted = value * m["slope"] + m["intercept"]
                    assert abs(predicted - sample["predicted"]) < 1e-8
                    if sample["observed"] is not None and sample["outcome"] == "agrees":
                        assert abs(predicted - sample["observed"]) <= check["tolerance"]
                    if sample["observed"] is not None and sample["outcome"] == "contradicts":
                        assert abs(predicted - sample["observed"]) > check["tolerance"]
                samples.append({k: sample[k] for k in ["frame", "bytes", "predicted", "observed", "outcome"]})
            agreements = [s for s in samples if s["outcome"] == "agrees"]
            deltas = [abs(a["observed"] - b["observed"]) for a, b in zip(samples, samples[1:])
                      if a["outcome"] == b["outcome"] == "agrees" and b["frame"] == a["frame"] + 1]
            assessment = ("invalidated" if candidate["invalidated"] else
                "contradicted" if any(s["outcome"] == "contradicts" for s in samples) else
                "consistent_in_observed_scope" if len(agreements) >= 4 and
                    any(d <= check["tolerance"] for d in deltas) and any(d > check["tolerance"]*2 for d in deltas)
                else "needs_contrasting_evidence")
            predictions.append({"address": candidate["address"], "mapping": candidate["mapping"],
                "invalidated": candidate["invalidated"], "assessment_replayed": assessment, "samples": samples})
    else:
        assert live["prediction_check"] is None and (directory / "no-hypothesis.json").exists()
    archives = list((directory / "engine/captures").glob("*/memory-batch.json"))
    assert len(archives) == 1
    archive = read(archives[0])
    for frame in live["frames"]:
        assert sha(archives[0].parent / f"frame-{frame['number']:03d}.png") == frame["sha256"]
    chunks = archive["plan"]
    assert sum(c["length"] for c in chunks) == train["planned_bytes"] == 16 * 1024 * 1024
    tracks = [{"id": t["id"], "label": t["label"], "samples": t["samples"], "hypotheses": len(t["matches"]),
               "repeated_contrast_in_both_halves": contrast(t["samples"][:6]) and contrast(t["samples"][6:])}
              for t in train["tracks"]]
    files = [directory / n for n in ["run.json", "training-status.json", "final-status.json", "score.json",
             "commands.json", "scope-selection.json", "oracle.json", "delivered-keys.json", "input.jsonl"]]
    files += sorted((directory / "engine/captures").glob("*/*"))
    return {"directory": str(directory.relative_to(ROOT)), "protocol_sha256": sha(protocol),
        "game": run["game"], "source_commit": run["source"].get("commit"), "window_process": run["selected"],
        "memory_process": selected, "renderer_cpu_delta": choices[0][0], "captures": 16,
        "input": {"pulses": len(pulses), "paired_trusted_delivery": True}, "counters": score["counters"],
        "fresh_counter_outcomes": [{k: v for k, v in row.items() if k != "observed"}
                                   for row in score["rows"] if row["frame"] >= 13],
        "tracks": tracks, "training_coverage": train["coverage"], "eligible_bytes": train["eligible_bytes"],
        "plan": {"chunks": len(chunks), "allocation_bases": len({c['region']['allocation_base'] for c in chunks}),
                 "bytes_by_kind": dict(collections.Counter({kind: sum(c["length"] for c in chunks if str(c['region']['kind']) == kind)
                    for kind in {str(c['region']['kind']) for c in chunks}})),
                 "largest_selected_private_region": max((c['region']['length'] for c in chunks if c['region']['kind'] == 131072), default=0)},
        "predictions": predictions, "retained_training_witnesses": len(archive["witnesses"]),
        "limitations": ["Exposed development games, one run each; no edit or authority validation.",
            "Renderer chosen by experimental browser CPU rule, not automatic product inference.",
            "Oracle instrumentation can allocate copies. No raw training-memory replay is possible.",
            "Eligibility replay tests observed variation only; candidate inliers are not replayed.",
            "Timing-excluded OCR values remain unscored, including any wrong values."],
        "files": {str(p.relative_to(directory)): sha(p) for p in files if p.is_file()}}


if __name__ == "__main__":
    version = sys.argv[1]
    assert version in ["v1", "v2"]
    receipts = [audit(ROOT / f"test-results/process-scope-study-{version}/{game}-1-attempt-01",
                      HERE / f"protocol-{version}.json") for game in ["2048", "tetris"]]
    output = HERE / "evidence"
    output.mkdir(exist_ok=True)
    (output / f"{version}.json").write_text(json.dumps(receipts, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps([{"game": r["game"], "hypotheses": sum(t["hypotheses"] for t in r["tracks"]),
        "frozen": len(r["predictions"]), "input_pulses_audited": r["input"]["pulses"]} for r in receipts]))
