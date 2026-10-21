"""Check replay provenance and table routing; semantic promotion is a separate review."""
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "test-results/shared-question-budget-v1"
read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / "frozen.json")
base = Path(frozen["baseline"])
assert sha(OUT / frozen["binary"]) == frozen["binary_sha256"]
assert sha(base / "revised-frozen.json") == frozen["baseline_manifest_sha256"]
for name, digest in frozen["sources"].items():
    assert sha(OUT / "source" / name) == digest, name
cases = []
identities = set()
for case in frozen["cases"]:
    name = case["name"]
    assert sha(Path(case["source"])) == case["sha256"]
    prior_path = base / "revised" / name / "receipt.json"
    assert sha(prior_path) == frozen["baseline_receipts"][name]
    prior = read(prior_path)
    process = read(OUT / "revised" / (name + "-process.json"))
    assert process["exit_code"] == 0, process
    path = OUT / "revised" / name / "receipt.json"
    receipt = read(path)
    inventory = read(OUT / "revised" / name / "input.json")["inventory"]
    assert inventory == read(base / "revised" / name / "input.json")["inventory"]
    assert receipt["report_sha256"] == inventory["report_sha256"] == prior["report_sha256"]
    identity = tuple(receipt[k] for k in ["model", "model_digest", "server_version"])
    assert identity == tuple(prior[k] for k in ["model", "model_digest", "server_version"])
    identities.add(identity)
    assert receipt["activity"] is None and not receipt["cancelled"]
    stages = receipt["stages"]
    assert 1 <= len(stages) <= 2
    assert stages[0]["prompt"] == prior["stages"][0]["prompt"]
    assert stages[0]["schema"] == prior["stages"][0]["schema"]
    assert stages[0]["options"] == prior["stages"][0]["options"]
    selection_same = receipt["selection"] == prior["selection"]
    tokens = []
    for s in stages:
        assert hashlib.sha256(s["prompt"].encode()).hexdigest() == s["prompt_sha256"]
        assert len(s["prompt"].encode()) <= 110000
        count = s["response"].get("prompt_eval_count")
        tokens.append(count)
        if receipt["error"] is None:
            assert isinstance(count, int) and count <= s["options"]["num_ctx"] - s["options"]["num_predict"] - 512
    if len(stages) == 2:
        s = stages[1]
        type_text, row_text = s["prompt"].split("\nObserved types: ", 1)[1].split("\nObserved paths: ", 1)
        types = json.loads(type_text)
        rows = json.loads(row_text)
        selected = receipt["selection"]
        assert types == [{"type_id": tid, "type_name": next(t["key"]["type_name"] for t in inventory["types"] if t["id"] == tid)} for tid in selected["selected_types"]]
        assert rows == [{"candidate": r["id"], "type_id": r["type_id"], "field_name": r["field_name"], "object_text": r["object_text"], "observed_value": r["value"]} for r in selected["rows"]]
        assert s["options"]["num_ctx"] == 49152 and s["options"]["num_predict"] == 10200
    batch = receipt.get("batch") or {}
    accepted = batch.get("accepted", [])
    rejected = batch.get("rejected", [])
    cases.append({
        "case": name, "baseline_accepted": len((prior.get("batch") or {}).get("accepted", [])),
        "baseline_rejected": len((prior.get("batch") or {}).get("rejected", [])),
        "accepted": len(accepted), "rejected": len(rejected),
        "selection_unchanged": selection_same, "metadata_calls": len(stages),
        "baseline_metadata_calls": len(prior["stages"]), "prompt_tokens": tokens,
        "error": receipt["error"], "seconds": process["seconds"],
        "baseline_seconds": read(base / "revised" / (name + "-process.json"))["seconds"],
        "proposals": [{"name": h["proposal"]["name"], "field": h["candidate"]["field_name"],
                       "type": h["candidate"]["type_name"], "question": h["proposal"]["question"],
                       "citations": h["proposal"]["citations"],
                       "alternative_fields": [a["field_name"] for a in h["alternatives"]]}
                      for h in accepted],
        "rejections": [{"name": h["proposal"]["name"], "reason": h["reason"]} for h in rejected],
        "receipt_sha256": sha(path), "runner_pid": process["pid"], "runner_exit_code": process["exit_code"],
    })
assert len(identities) == 1
test_log = (ROOT / "test-results/shared-question-budget-tests.log").read_text(encoding="utf-8-sig")
assert "FAILED" not in test_log
tests = sum(map(int, re.findall(r"test result: ok\. (\d+) passed", test_log)))
clippy = (ROOT / "test-results/shared-question-budget-clippy.log").read_text(encoding="utf-8-sig")
assert "Finished" in clippy and "error:" not in clippy
result = {"mechanical_audit_passed": True, "model": list(next(iter(identities))),
          "core_observer_tests": tests, "strict_clippy": True, "cases": cases,
          "promotion_review": "See promotion.json for the separate decision against the frozen protocol.",
          "limits": ["Known development reports, not held-out evaluation.",
                     "Accepted means reference/lexical checks, not semantic truth or causal effect.",
                     "Question-budget allocation, table compression and wording changed together.",
                     "No native target access, gameplay input, writes or image review in the replay."]}
Path(__file__).with_name("results.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"audit": True, "tests": tests, "cases": [{k: c[k] for k in ["case", "baseline_accepted", "accepted", "rejected", "selection_unchanged", "metadata_calls", "error", "seconds"]} for c in cases]}, indent=2))
