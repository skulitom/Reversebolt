"""Freeze and replay two unchanged discovery snapshots through the shared core."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "test-results" / "automatic-cards-v1"
CASES = [
    ("Hyperbolica", "test-results/independent-runtime-v2/discovery.json",
     "test-results/independent-runtime-v1/data/runtime/84124-01dd61b9d9c6e438/55053c39-9513-44e7-a433-35f2d3c6fe69/report.json"),
    ("Manifold Garden", "test-results/comparison-planner-v1/discovery.json",
     "test-results/comparison-planner-v1/data/runtime/80420-01dd61a9213c67af/2f37c7e3-8dae-414b-ad6c-a7a704234c95/report.json"),
]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def save(path, value):
    with path.open("x", encoding="utf-8") as file:
        json.dump(value, file, indent=2, ensure_ascii=False)
        file.write("\n")

OUT.mkdir()
sources = sorted({p.relative_to(ROOT).as_posix() for p in (ROOT / "crates").rglob("*")
                  if p.is_file() and p.suffix in {".rs", ".toml"}}
                 | {"Cargo.toml", "Cargo.lock", "research/automatic-cards/run.py",
                    "research/automatic-cards/PROTOCOL.md"})
manifest = {"sources": {}, "inputs": {}}
for group, paths in [("sources", sources), ("inputs", sorted({p for _, s, r in CASES for p in (s, r)}))]:
    for relative in paths:
        source = ROOT / relative
        dest = OUT / group / relative
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, dest)
        manifest[group][relative] = sha(source)
        assert sha(dest) == sha(source)
save(OUT / "manifest.json", manifest)
results = {"manifest_sha256": sha(OUT / "manifest.json"), "cases": [], "error": None}
try:
    with (OUT / "build.log").open("x", encoding="utf-8") as log:
        subprocess.run(["cargo", "build", "--offline", "-p", "reversebolt-core", "--example", "runtime-cards-replay"],
                       cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
    binary = ROOT / "target/debug/examples/runtime-cards-replay.exe"
    results["binary_sha256"] = sha(binary)
    for index, (name, snapshot, report) in enumerate(CASES):
        output = OUT / f"case-{index}.json"
        run = subprocess.run([str(binary), str(OUT / "inputs" / snapshot),
                              str(OUT / "inputs" / report), str(output)],
                             cwd=ROOT, capture_output=True, text=True)
        (OUT / f"case-{index}.log").write_text(run.stdout + run.stderr, encoding="utf-8")
        case = {"name": name, "exit_code": run.returncode, "log": run.stdout.strip()}
        results["cases"].append(case)
        if run.returncode:
            raise RuntimeError(f"{name} replay failed; original failure retained")
        data = json.loads(output.read_text(encoding="utf-8"))
        case.update({"output_sha256": sha(output), "initially_present": len(data["initially_present"]),
                     "publication": data["publication"], "repeat_unchanged": data["repeat_unchanged"]})
        assert sha(binary) == results["binary_sha256"]
except Exception as error:
    results["error"] = str(error)
finally:
    differences = []
    for group, paths in manifest.items():
        for relative, expected in paths.items():
            if sha(ROOT / relative) != expected or sha(OUT / group / relative) != expected:
                differences.append(relative)
    results["audit"] = {"source_count": len(sources), "input_count": len(manifest["inputs"]),
                        "changed": differences}
    save(OUT / "results.json", results)
    print(json.dumps({"cases": results["cases"], "error": results["error"], "audit": results["audit"]}, indent=2))
sys.exit(1 if results["error"] or differences else 0)
