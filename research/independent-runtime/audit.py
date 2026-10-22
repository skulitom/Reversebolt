"""Audit the frozen native baseline without interpreting new measurement images."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "test-results/independent-runtime-v2"
V1 = ROOT / "test-results/independent-runtime-v1"
read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / "frozen.json")
prior = read(V1 / "frozen.json")
unavailable_binaries = []
for folder, manifest in [(V1, prior), (OUT, frozen)]:
    for name, digest in manifest["sources"].items():
        assert sha(folder / "source" / name) == digest, name
    for name, digest in manifest["binaries"].items():
        path = Path(manifest["bin"]) / name
        if path.is_file():
            assert sha(path) == digest
        elif name == "Reversebolt.exe":
            unavailable_binaries.append(str(path))
        else:
            raise AssertionError(f"Unexpected missing binary: {path}")
    for name, digest in manifest["storage_before"].items():
        assert sha(folder / "storage-before" / name) == digest
    assert sha(folder / "appmanifest-before.acf") == manifest["appmanifest_sha256"]
assert frozen["binaries"] == prior["binaries"]
assert all(frozen["sources"][p] == h for p, h in prior["sources"].items() if p.startswith("crates/"))
setup_stop = read(V1 / "setup-stop.json")
assert setup_stop["stopped_before_attach"] and setup_stop["field_edits"] == 0
data = Path(frozen["data"])
result = read(OUT / "result.json")
state = read(OUT / "discovery.json")["state"]
live = state["live"]
runtime = live["runtime"]
assert result["workflow_finished"] and result["jobs"] == [{"phase": "Completed", "error": None}]
assert result["samples"] == len(live["frames"]) == 15
assert [i["key"] for i in result["evaluator_inputs"]] == [68, 65, 68, 65]
assert all(i["planned_ms"] == 200 and i["end_ns"] > i["begin_ns"] for i in result["evaluator_inputs"])
assert result["manually_reviewed_measurement_images"] == 0
review = runtime["naming"]
assert review == result["review"] and review["status"] == "complete"
receipt_path = data / "runtime/naming" / review["evidence_id"] / "receipt.json"
receipt = read(receipt_path)
play_path = receipt_path.with_name("play-source.json")
play = read(play_path)
inventory = read(receipt_path.with_name("inventory.json"))
assert receipt["batch"] == review["batch"] and receipt["error"] is None and not receipt["cancelled"]
assert receipt["report_sha256"] == review["report_sha256"] == inventory["report_sha256"]
assert receipt["model_digest"] == "22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643"
assert play["frames"] == live["frames"] and play["selected"] == receipt["activity"]["source"]
assert receipt["activity"]["error"] is None
assert not receipt["activity"]["observation"]["consistent_visible_change"]
assert review["play_context"] is None and result["comparison"] is None
planning = next(OUT.glob("rpc-*-plan_runtime_comparisons.json"))
refusal = read(planning)["response"]
assert not refusal["ok"] and "No consistent observed activity" in refusal["error"]
selected = {r["id"]: r for r in receipt["selection"]["rows"]}
questions = []
for hypothesis in review["batch"]["accepted"]:
    candidate = hypothesis["candidate"]
    assert candidate == selected[hypothesis["proposal"]["candidate"]]
    assert hypothesis["alternatives"] == [selected[i] for i in hypothesis["proposal"]["alternatives"]]
    for citation in hypothesis["proposal"]["citations"]:
        source = candidate[citation["source"]]
        if citation["source"] == "object_text":
            source = source[citation["index"]]
        assert citation["quote"] in source
    questions.append({"name": hypothesis["proposal"]["name"], "field": candidate["field_name"],
                      "value": candidate["value"], "alternatives": len(hypothesis["alternatives"]),
                      "primary_path": candidate["path_id"]})
for stage in [*receipt["stages"], receipt["activity"]["stage"]]:
    assert hashlib.sha256(stage["prompt"].encode()).hexdigest() == stage["prompt_sha256"]
    assert stage["response"]["done"] and stage["response"]["model"] == receipt["model"]
capture_dirs = list((data / "captures").iterdir())
assert len(capture_dirs) == 1
for frame in live["frames"]:
    assert sha(capture_dirs[0] / f"frame-{frame['number']:03}.png") == frame["sha256"]
helpers = []
pauses = []
for path in (data / "runtime").glob("*-*/**/stdout.jsonl"):
    events = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.startswith("{")]
    assert events[-1]["stage"] == "complete" and events[-1]["helper_absent"] and events[-1]["inspection_exit"] == 0
    report_path = path.with_name("report.json")
    report = read(report_path)
    assert report["pid"] == live["process"]["pid"] and report["session"] == 3
    assert not report["edit_authority"]
    pauses.extend(report["snapshots"])
    pauses.extend(w["discarded_snapshot"] for w in report["warmups"])
    helpers.append({"report_sha256": sha(report_path), "events_sha256": sha(path),
                    "graph_complete": report["graph_complete"], "roots_complete": report["root_scan_complete"]})
assert len(helpers) == 1
assert all(p["helper_allocations_while_paused"] == p["helper_deallocations_while_paused"] == 0 for p in pauses)
final = read(OUT / "final-state.json")["state"]
assert not final["runtime_trials"] and not final["runtime_tests"] and not final["freezes"]
cleanup = read(OUT / "cleanup.json")
later = read(OUT / "later-detection.json")
assert cleanup["ownedProcessesExited"] and cleanup["mcpExitCode"] == 0 and cleanup["desktopResponsive"]
assert cleanup["newRelatedDetectionEvents"] == 0 and all(cleanup["protection"].values())
assert cleanup["originalReviewSampleSha256"] == frozen["preserved_review_sample_sha256"]
if unavailable_binaries:
    assert not later["publishedSampleAvailable"] and later["desktopProcessExited"]
    assert any(e["id"] == 1117 and e["action"] == "Quarantine" for e in later["events"])
    assert later["preDetectionSha256"] == frozen["binaries"]["Reversebolt.exe"]
assert read(OUT / "storage-after.json")["hashes"]["game.save"] == frozen["storage_before"]["game.save"]
summary = {"artifact_audit_passed": True, "native_workflow_finished": True,
           "postrun_binary_rehash_unavailable": sorted(set(unavailable_binaries)),
           "later_security_detection": later,
           "original_product_commit": frozen["source_commit"], "product_unchanged_between_targets": True,
           "freight_hopper_setup": {"stopped_before_attach": True, "reason": setup_stop["reason"],
                                    "setup_images_reviewed": 4, "normal_menu_storage_changed": True},
           "hyperbolica": {"app_id": "1256230", "installed_build": "8675290", "setup_images_reviewed": 3,
                           "automatic_images": 15, "manual_measurement_image_reviews": 0, "input_holds": 4,
                           "metadata_model_calls": len(receipt["stages"]), "image_model_calls": 1,
                           "observed_types": len(inventory["types"]), "observed_fields": sum(len(t["fields"]) for t in inventory["types"]),
                           "reachable_paths": inventory["reachable_paths"], "questions": questions,
                           "rejected_questions": len(review["batch"]["rejected"]),
                           "image_interpretation": receipt["activity"]["observation"],
                           "planner_refusal": refusal["error"], "comparisons": 0, "field_edits": 0,
                           "original_game_save_unchanged": True, "logs_and_analytics_added": True},
           "helpers": helpers, "pause_count": len(pauses),
           "max_stop_to_resume_us": max(p["stop_to_resume_us"] for p in pauses),
           "cleanup": cleanup, "frozen_manifest_sha256": sha(OUT / "frozen.json"),
           "receipt_sha256": sha(receipt_path), "measurement_source_sha256": sha(play_path),
           "limitation": "No gameplay control was tested or validated. Paths may alias. Root scan completion is not full graph coverage. Neither the two accepted names nor an artifact audit establish generalization."}
(ROOT / "research/independent-runtime/results.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"audit": "passed", "questions": len(questions), "comparisons": 0, "field_edits": 0, "saved_game_unchanged": True}))
