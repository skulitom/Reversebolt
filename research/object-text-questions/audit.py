"""Audit saved replay evidence; passing provenance is not gameplay validation."""
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
versions = []
for version in ["v1", "v2"]:
    out = ROOT / ("test-results/object-text-questions-" + version)
    frozen = read(out / "frozen.json")
    base = Path(frozen["baseline"])
    assert sha(out / frozen["binary"]) == frozen["binary_sha256"]
    for name, digest in frozen["sources"].items():
        assert sha(out / "source" / name) == digest, (version, name)
    cases = []
    for case in frozen["cases"]:
        name = case["name"]
        assert sha(Path(case["source"])) == case["sha256"]
        prior_path = base / "revised" / name / "receipt.json"
        assert sha(prior_path) == frozen["receipts"][name]
        prior = read(prior_path)
        path = out / "replayed" / name / "receipt.json"
        receipt = read(path)
        process = read(out / "replayed" / (name + "-process.json"))
        summary = json.loads(process["stdout"])
        assert process["exit_code"] == 0 and not process["stderr"]
        assert summary["metadata_model_calls"] == 0
        if version == "v2":
            assert summary["elapsed_ms_is_retained"] is True
        inventory = read(out / "replayed" / name / "input.json")["inventory"]
        assert inventory == read(base / "revised" / name / "input.json")["inventory"]
        assert receipt["report_sha256"] == inventory["report_sha256"] == prior["report_sha256"]
        # Everything outside the new fallback and resulting batch is retained exactly,
        # including selection, raw prompts/responses, timing and model identity.
        assert {k: v for k, v in receipt.items() if k not in ["object_text_fallback", "batch"]} == {
            k: v for k, v in prior.items() if k not in ["object_text_fallback", "batch"]}
        assert receipt["error"] is None and not receipt["cancelled"]
        batch = receipt["batch"]
        fallback = receipt.get("object_text_fallback")
        if prior["batch"]["accepted"] or prior["batch"]["rejected"]:
            assert batch == prior["batch"] and fallback is None
        else:
            assert fallback["original_batch"] == prior["batch"]
            assert fallback["coverage"]["report_sha256"] == receipt["report_sha256"]
        if name == "opaque":
            assert not batch["accepted"] and not batch["rejected"]
        if name == "fixture":
            assert fallback["coverage"]["incomplete_context_rows"] == 3
            if version == "v1":
                assert not batch["accepted"] and not batch["rejected"]
                assert fallback["coverage"]["policy"] == "one-object-label-v1"
            else:
                assert len(batch["accepted"]) == 1 and not batch["rejected"]
                assert summary["question_source"] == "object_text"
                assert summary["question_policy"] == "complete-object-clue-v2"
                hypothesis = batch["accepted"][0]
                assert len(hypothesis["alternatives"]) == 2
                paths = [hypothesis["candidate"], *hypothesis["alternatives"]]
                assert {r["path_id"] for r in paths} == {r["path_id"] for r in receipt["selection"]["rows"]}
                assert all(r["object_text"] == ["Speed"] for r in paths)
                assert hypothesis["proposal"]["citations"] == [{"source": "object_text", "index": 0, "quote": "Speed"}]
                assert "untested" in hypothesis["proposal"]["explanation"]
                assert "Does changing this value" in hypothesis["proposal"]["question"]
                assert fallback["coverage"]["accepted_paths"] == [hypothesis["candidate"]["path_id"]]
                for row in paths:
                    field = next(f for t in inventory["types"] if t["id"] == row["type_id"]
                                 for f in t["fields"] if f["field_id"] == row["field_id"])
                    sample = next(s for s in field["samples"] if s["path_id"] == row["path_id"])
                    assert any(h["text"] == "Speed" and not h["truncated"] for h in sample["context"])
                    assert any(h["truncated"] for h in sample["context"])
        cases.append({"case": name, "accepted": len(batch["accepted"]), "rejected": len(batch["rejected"]),
                      "question_source": summary["question_source"], "question_policy": summary.get("question_policy"),
                      "fallback": fallback["coverage"] if fallback else None,
                      "receipt_sha256": sha(path), "process_exit": process["exit_code"],
                      "new_model_calls": 0, "replay_seconds": process["seconds"]})
    versions.append({"version": version, "artifact_audit_passed": True,
                     "protocol_expectations_passed": version == "v2",
                     "failure": "Expected positive fixture question was absent: incomplete surrounding text excluded all three paths." if version == "v1" else None,
                     "frozen_manifest_sha256": sha(out / "frozen.json"),
                     "desktop_sample_sha256_at_freeze": frozen["desktop_sample_sha256"], "cases": cases})

test_path = ROOT / "test-results/object-text-questions-v2-workspace-tests.log"
test_log = test_path.read_text(encoding="utf-8-sig")
assert "test result: FAILED" not in test_log
count = sum(map(int, re.findall(r"test result: ok\. (\d+) passed", test_log)))
assert count == 292
clippy_path = ROOT / "test-results/object-text-questions-v2-clippy.log"
assert "Finished `dev`" in clippy_path.read_text(encoding="utf-8-sig")
result = {"versions": versions, "workspace_tests_passed": count,
          "test_log_sha256": sha(test_path), "clippy_log_sha256": sha(clippy_path),
          "limitations": ["Known development fixtures and saved game reports, not held-out evaluation.",
                          "No model, target access, gameplay edit, input or manual image review during replay.",
                          "One literal question does not prove meaning, gameplay relevance, domain bounds or authority to edit.",
                          "The existing unwanted menu proposals are unchanged.",
                          "No desktop deployment or live causal verification is established by this audit."]}
(ROOT / "research/object-text-questions/results.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"artifact_audit": "passed", "v1_protocol": "failed", "v2_protocol": "passed", "tests": count, "new_model_calls": 0}))
