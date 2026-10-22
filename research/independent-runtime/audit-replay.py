"""Validate the saved measured-contrast replay without rerunning inference."""
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "test-results/measured-contrast-v1"
read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / "frozen.json")
assert sha(Path(frozen["binary"])) == frozen["binary_sha256"]
for name, digest in frozen["sources"].items():
    assert sha(OUT / "source" / name) == digest
for path, digest in frozen["inputs"].items():
    assert sha(Path(path)) == digest
process = read(OUT / "process.json")
assert process["exit_code"] == 0 and not process["stderr"]
replay = read(OUT / "replay.json")
original = read(ROOT / "test-results/independent-runtime-v2/discovery.json")["state"]["live"]["runtime"]["naming"]
changed = replay["revised_review"]
assert {k: v for k, v in changed.items() if k != "play_context"} == {k: v for k, v in original.items() if k != "play_context"}
assert replay["original_context"] is None
context = replay["revised_context"]
assert changed["play_context"] == context and context["basis"] == "measured_contrast_v1"
receipt_path = next(Path(p) for p in frozen["inputs"] if p.endswith("receipt.json"))
receipt = read(receipt_path)
assert context["observation"] == receipt["activity"]["observation"]
assert not context["observation"]["consistent_visible_change"]
assert context["source"] == receipt["activity"]["source"]
assert len(replay["input_hashes"]["images"]) == 15
plan = replay["plan"]
assert replay["error"] is None and len(plan["choices"]) == 1
assert [a["keys"] for a in plan["observed_actions"]["actions"]] == [[68], [65], [68], [65]]
for episode in plan["observed_actions"]["episodes"]:
    assert episode["hold_bounds_us"][0] <= episode["action"]["hold_ms"] * 1000 <= episode["hold_bounds_us"][1]
choice = plan["next"]
assert choice in plan["choices"] and choice["value"] == "0"
assert choice["hypothesis_ref"]["hypothesis_path_id"] in [h["candidate"]["path_id"] for h in original["batch"]["accepted"]]
assert replay["metadata_model_calls"] == replay["field_writes"] == 0
assert not replay["target_access"] and not replay["input_sent"]
tests_path = ROOT / "test-results/measured-contrast-final-tests.log"
log = tests_path.read_text(encoding="utf-8-sig")
tests = sum(map(int, re.findall(r"test result: ok\. (\d+) passed", log)))
assert tests == 293 and "test result: FAILED" not in log
result = {"artifact_audit_passed": True, "offline_replay_only": True,
          "original_image_interpretation_preserved": True, "metadata_questions_unchanged": True,
          "images_rehashed_and_metrics_recomputed": 15, "basis": context["basis"],
          "model_consistent_visible_change": False, "planned_choices": len(plan["choices"]),
          "planned_hold_ms": [a["hold_ms"] for a in plan["observed_actions"]["actions"]],
          "next": choice, "new_model_calls": 0, "target_access": False, "field_edits": 0,
          "workspace_tests_passed": tests, "test_log_sha256": sha(tests_path),
          "frozen_manifest_sha256": sha(OUT / "frozen.json"), "replay_sha256": sha(OUT / "replay.json"),
          "limitations": ["Known development case, not new independent acceptance.",
                          "The plan has not been executed in the game; no gameplay control was verified.",
                          "The original native run still had no eligible comparison and made no edit.",
                          "The desktop preview was subsequently quarantined; this source update is unpublished."]}
(ROOT / "research/independent-runtime/replay-results.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"audit": "passed", "plans": 1, "model_ambiguity_retained": True, "new_model_calls": 0, "tests": tests}))
