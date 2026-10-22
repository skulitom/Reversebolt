"""One frozen, clock-only run; never launches Reversebolt or any target."""
import hashlib
import json
import pathlib
import shutil
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT = ROOT / "test-results/capture-clock-v1"
OUT.mkdir()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, data):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(data, stream, indent=2)


binary = OUT / "capture-clock-probe.exe"
shutil.copyfile(ROOT / "target/debug/examples/capture-clock-probe.exe", binary)
tracked = subprocess.check_output(
    ["git", "ls-files", "crates", "Cargo.toml", "Cargo.lock"], cwd=ROOT, text=True
).splitlines()
paths = [ROOT / p for p in tracked if p.endswith((".rs", ".toml")) or p == "Cargo.lock"]
paths += [ROOT / p for p in [
    "crates/reversebolt-core/src/image_clock.rs",
    "crates/reversebolt-app/src/capture_clock.rs",
    "crates/reversebolt-app/examples/capture-clock-probe.rs",
    "research/capture-clock/PROTOCOL.md", "research/capture-clock/run.py",
]]
paths.append(binary)
frozen = {str(p): sha(p) for p in sorted(set(paths))}
write(OUT / "frozen.json", frozen)
process = subprocess.run([str(binary), str(OUT / "results.json")], cwd=ROOT,
                         text=True, capture_output=True, timeout=60)
write(OUT / "process.json", {"exit_code": process.returncode,
                             "stdout": process.stdout, "stderr": process.stderr})
assert process.returncode == 0, process.stderr
result = json.loads((OUT / "results.json").read_text(encoding="utf-8"))
rows = [r["measurement"] for r in result["results"]]
assert result["samples"] == len(rows) == 100
assert [r["index"] for r in rows] == list(range(100))
assert all(r["other_thread"] == (r["index"] % 2 == 1) for r in rows)
assert not any(result[k] for k in ["image_captured", "compositor_api_called",
                                  "new_model_calls", "native_target_access"])
groups = []
for label, selected in [("all", rows), ("calling_thread", rows[::2]), ("worker_thread", rows[1::2])]:
    widths = sorted(r["width_us"] for r in selected if "width_us" in r)
    groups.append({"group": label, "samples": len(selected),
                   "consistent": sum(r["overlaps_measured_clock_sample"] for r in selected),
                   "errors": [r for r in selected if r["error"] is not None],
                   "width_us_min": min(widths) if widths else None,
                   "width_us_upper_median": widths[len(widths)//2] if widths else None,
                   "width_us_max": max(widths) if widths else None})
changed = [p for p, digest in frozen.items() if sha(pathlib.Path(p)) != digest]
summary = {"method": result["method"], "groups": groups,
           "frozen_files": len(frozen), "changed": changed,
           "results_sha256": sha(OUT / "results.json"),
           "limitation": "Clock APIs and arithmetic only; no captured image or game validation."}
write(OUT / "summary.json", summary)
print(json.dumps(summary))
assert not changed
assert result["consistent"] == groups[0]["consistent"] == 100
