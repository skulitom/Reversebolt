"""Run both unchanged and corrected classifiers on frozen local input metadata."""
import hashlib
import json
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
PHASE = sys.argv[1]
assert PHASE in ("before", "after")
OUT = ROOT / "test-results/input-cadence-v1" / PHASE
OUT.mkdir(parents=True)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, data):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(data, stream, indent=2)


binary = OUT / "input-cadence.exe"
shutil.copyfile(ROOT / "target/debug/examples/input-cadence.exe", binary)
tracked = subprocess.check_output(["git", "ls-files", "crates", "Cargo.toml", "Cargo.lock"], cwd=ROOT, text=True).splitlines()
paths = [ROOT / p for p in tracked if p.endswith((".rs", ".toml")) or p == "Cargo.lock"]
inputs = [ROOT / f"test-results/independent-games-v1/{name}-attempt-01/final-status.json"
          for name in ["2048-1", "tetris-1", "2048-2", "tetris-2"]]
method = [pathlib.Path(__file__), pathlib.Path(__file__).with_name("PROTOCOL.md"),
          ROOT / "crates/reversebolt-core/examples/input-cadence.rs"]
paths += inputs + method + [binary]
frozen = {str(p): sha(p) for p in sorted(set(paths))}
if PHASE == "after":
    before = json.loads((OUT.parent / "before/frozen.json").read_text(encoding="utf-8"))
    assert all(before[str(p)] == frozen[str(p)] for p in inputs + method)
write(OUT / "frozen.json", frozen)
process = subprocess.run([str(binary), str(ROOT), str(OUT / "results.json")], cwd=ROOT,
                         text=True, capture_output=True, timeout=180)
write(OUT / "process.json", {"exit_code": process.returncode, "stdout": process.stdout, "stderr": process.stderr})
assert process.returncode == 0, process.stderr
result = json.loads((OUT / "results.json").read_text(encoding="utf-8"))
assert len(result["cases"]) == 4 and all(len(c["scenarios"]) == 168 for c in result["cases"])
grid = []
for cost in [100, 200, 400]:
    for delay in [50, 100, 250, 500]:
        selected = [s["classification"] for c in result["cases"] for s in c["scenarios"]
                    if s["cost_ms"] == cost and s["delay_ms"] == delay]
        grid.append({"cost_ms": cost, "delay_ms": delay, "scenarios": len(selected),
                     "timing_only_repeated_key_scenarios": sum(bool(s["timing_only_repeated_keys"]) for s in selected),
                     "accepted_capture_overlap_intervals": sum(len(s["accepted_capture_overlap_intervals"]) for s in selected),
                     "unknown_intervals": sum(s["counts"].get("unknown", 0) for s in selected)})
changed = [p for p, digest in frozen.items() if sha(pathlib.Path(p)) != digest]
report = {"phase": PHASE, "original": [{k: c[k] for k in
          ["name", "retained_unique_polls", "original_omitted_polls", "original_capture_us", "original"]}
          for c in result["cases"]], "grid": grid, "frozen_file_count": len(frozen), "changed": changed,
          "results_sha256": sha(OUT / "results.json")}
write(OUT / "summary.json", report)
print(json.dumps({"phase": PHASE, "original_accepted_overlaps": [len(c["original"]["accepted_capture_overlap_intervals"]) for c in report["original"]], "grid": grid, "changed": changed}))
assert not changed
