"""Exercise the shared recovery bridge on archived native evidence, without a game.

An older completed test receives route metadata only in an isolated research copy.
This is a retrospective replay, not a fresh native test or production migration.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(sys.argv[1]).resolve()
OUT.mkdir()
read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
archive = ROOT / "test-results/runtime-recovery-declared-continuation-v1"
prior = ROOT / "test-results/viewfinder-followup-v1"
manifest = {
    "test": str(archive / "source-test.json"),
    "source_report": str(archive / "source-report.json"),
    "current_report": str(archive / "current-report.json"),
    "source_data": read(prior / "frozen.json")["data"],
    "output": str(OUT / "replay"),
}
files = [Path(manifest[k]) for k in ("test", "source_report", "current_report")]
files += [ROOT / name for name in (
    "crates/reversebolt-core/src/runtime_recovery.rs",
    "crates/reversebolt-core/src/runtime_tests.rs",
    "crates/reversebolt-core/src/runtime_test_store.rs",
    "crates/reversebolt-vision/src/response.rs",
    "crates/reversebolt-app/src/runtime_recovery_bridge.rs",
    "crates/reversebolt-app/src/runtime_recovery_tests.rs",
    "crates/reversebolt-app/src/runtime_test_history_tests.rs",
    "research/runtime-recovery/replay-product.py",
)]
manifest["inputs"] = {str(p): sha(p) for p in files}
path = OUT / "manifest.json"
path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
env = dict(os.environ, REVERSEBOLT_RECOVERY_REPLAY=str(path))
with (OUT / "test.log").open("w", encoding="utf-8") as log:
    result = subprocess.run([
        "cargo", "test", "--locked", "-p", "reversebolt-app", "--all-features",
        "replay_remembered_archive", "--", "--ignored", "--nocapture",
    ], cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT,
        creationflags=subprocess.CREATE_NO_WINDOW)
for name, expected in manifest["inputs"].items():
    assert sha(Path(name)) == expected, name
assert result.returncode == 0, (OUT / "test.log").read_text()
replayed = read(OUT / "replay/result.json")
assert replayed["passed"] and replayed["research_backfill"]
assert replayed["target_access"] is False and replayed["model_calls"] == 0
assert len(replayed["evidence"]) == 36
for entry in replayed["evidence"]:
    assert sha(Path(entry["path"])) == entry["sha256"]
summary = {"passed": True, "type": "retrospective_shared_engine_replay",
           "research_backfill": True, "target_access": False, "model_calls": 0,
           "elapsed_ms": replayed["elapsed_ms"],
           "source_test_id": replayed["manual"]["source_test_id"],
           "source_recipe_sha256": replayed["manual"]["recovery"]["recipe_sha256"],
           "current_candidate_id": replayed["manual"]["recovery"]["candidate"]["id"],
           "strict_status": replayed["manual"]["recovery"]["strict_status"],
           "manual_shown": replayed["manual"]["shown"],
           "automatic": replayed["automatic"], "evidence_files_checked": 36,
           "manifest_sha256": sha(path), "result_sha256": sha(OUT / "replay/result.json")}
(OUT / "result.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
print(json.dumps(summary))
