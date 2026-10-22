"""Create-only compatibility replay; no game, model, native capture or input."""
import hashlib
import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT = ROOT / "test-results/regional-activity-entry-v1"
EXE = ROOT / "target/debug/examples/runtime-appearance-replay.exe"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_new(path, value):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2)


def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


paths = subprocess.check_output(
    ["git", "ls-files", "crates", "Cargo.toml", "Cargo.lock"], cwd=ROOT, text=True
).splitlines()
paths = [ROOT / p for p in paths if p.endswith((".rs", ".toml")) or p == "Cargo.lock"]
paths += [pathlib.Path(__file__), pathlib.Path(__file__).with_name("PROTOCOL.md"), EXE]
cases = []
for index in range(2):
    manifest = ROOT / f"test-results/runtime-appearances-v1/case-{index}.json"
    prior = ROOT / f"test-results/runtime-appearances-v1/case-{index}/result.json"
    expected = ROOT / f"test-results/passive-region-context-v2/case-{index}/summary-0.json"
    expected_source = ROOT / f"test-results/passive-region-context-v2/case-{index}/baseline.json"
    paths += [manifest, prior, expected, expected_source]
    paths += [pathlib.Path(frame["file"]) for frame in read(manifest)["frames"]]
    cases.append((manifest, prior, expected, expected_source))
frozen = {str(p): digest(p) for p in sorted(set(paths))}
write_new(OUT / "frozen.json", frozen)
results = []
for index, (manifest, prior_path, expected_path, source_path) in enumerate(cases):
    destination = OUT / f"case-{index}"
    completed = subprocess.run(
        [str(EXE), str(manifest), "cached-only", str(destination), "--observations", str(prior_path)],
        cwd=ROOT, capture_output=True, text=True, timeout=120,
    )
    write_new(OUT / f"case-{index}-process.json", {
        "exit_code": completed.returncode, "stdout": completed.stdout, "stderr": completed.stderr,
    })
    completed.check_returncode()
    result, prior, expected, source = map(read, [destination / "result.json", prior_path, expected_path, source_path])
    current = result["evidence"]
    suggestions = current["automatic_region_motion"]["activity_region_suggestions"]
    checks = {
        "source_equal": current["activity_source"] == source["source"],
        "regional_report_equal": suggestions["report"] == expected["report"],
        "regional_summary_equal": suggestions["summary"] == expected["summary"],
        "base_evidence_equal": current["evidence"] == prior["evidence"]["evidence"],
        "observations_equal": result["visual_tracks"] == prior["visual_tracks"],
        "new_model_calls_zero": result["publication"]["new_model_calls"] == 0,
        "no_regional_fallback": "regional_activity_search" not in current["automatic_region_motion"],
    }
    record = {"name": result["name"], "checks": checks, "result_sha256": digest(destination / "result.json")}
    results.append(record)
    print(json.dumps(record), flush=True)
changed = [path for path, sha in frozen.items() if digest(pathlib.Path(path)) != sha]
write_new(OUT / "replay-results.json", {"cases": results, "frozen_inputs": len(frozen), "changed": changed})
if changed or not all(all(case["checks"].values()) for case in results):
    raise SystemExit("Compatibility replay or frozen-source audit failed; inspect retained results.")
