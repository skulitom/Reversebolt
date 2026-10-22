"""Frozen file-only replay. No later-condition evidence is copied or opened."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "test-results/trial-appearance-suggestions-v1"
RECORDS = sorted((ROOT / "test-results/planned-input-v1/case/data/runtime-tests").glob("*.json"))

def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def save(path, value):
    with path.open("x", encoding="utf-8") as file:
        json.dump(value, file, indent=2, ensure_ascii=False)
        file.write("\n")

assert len(RECORDS) == 3
OUT.mkdir()
inputs = set(RECORDS)
for record in RECORDS:
    test = read(record)
    assert test["candidate"]["locator"]["root"]["declaring"]["image"] == "OwnedPathFixture"
    for frame in test["conditions"][0]["frames"]:
        for key, digest in [("path", "sha256"), ("input_path", "input_sha256")]:
            path = record.parent / frame[key]
            assert path.resolve().is_relative_to(record.parent.resolve())
            assert sha(path) == frame[digest]
            inputs.add(path)
sources = {p for p in (ROOT / "crates").rglob("*") if p.is_file() and p.suffix in {".rs", ".toml"}}
sources.update([ROOT / "Cargo.toml", ROOT / "Cargo.lock", Path(__file__), Path(__file__).with_name("PROTOCOL.md")])
manifest = {"sources": {}, "inputs": {}}
for group, paths in [("sources", sources), ("inputs", inputs)]:
    for path in sorted(paths):
        relative = path.relative_to(ROOT).as_posix()
        dest = OUT / group / relative
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, dest)
        manifest[group][relative] = sha(path)
        assert sha(dest) == sha(path)
save(OUT / "manifest.json", manifest)
results = {"manifest_sha256": sha(OUT / "manifest.json"), "cases": [], "error": None}
try:
    with (OUT / "build.log").open("x", encoding="utf-8") as log:
        subprocess.run(["cargo", "build", "--offline", "-p", "reversebolt-vision", "--example", "trial-appearance-suggestions-replay"],
                       cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
    binary = ROOT / "target/debug/examples/trial-appearance-suggestions-replay.exe"
    results["binary_sha256"] = sha(binary)
    for index, record in enumerate(RECORDS):
        record = OUT / "inputs" / record.relative_to(ROOT)
        case = {"test_id": record.stem, "runs": []}
        results["cases"].append(case)
        outputs = []
        for repeat in range(2):
            output = OUT / f"case-{index}-{repeat}.json"
            process = subprocess.run([str(binary), str(record), str(output)], cwd=ROOT, capture_output=True, text=True)
            save(OUT / f"case-{index}-{repeat}-process.json", {"exit_code": process.returncode, "stdout": process.stdout, "stderr": process.stderr})
            case["runs"].append({"exit_code": process.returncode, "output": output.name})
            if process.returncode:
                raise RuntimeError(f"Replay failed for {record.stem}; original outcome retained")
            data = read(output)
            assert all(c == 0 for c, _ in data["images_read"] + data["inputs_read"])
            assert data["error"] is None
            assert data["report"]["attempted_frames"] <= 24
            assert len(data["report"]["suggestions"]) <= 4
            outputs.append(data)
        assert outputs[0] == outputs[1]
        data = outputs[0]
        case.update({"status": data["report"]["status"], "attempted_frames": data["report"]["attempted_frames"],
                     "ranked_regions": data["report"]["ranked_regions"], "suggestions": len(data["report"]["suggestions"]),
                     "recommended": data["report"]["recommended"], "images_read": data["images_read"],
                     "inputs_read": data["inputs_read"], "repeat_equal": True})
        assert sha(binary) == results["binary_sha256"]
except Exception as error:
    results["error"] = str(error)
finally:
    changed = []
    for group, paths in manifest.items():
        for relative, expected in paths.items():
            if sha(ROOT / relative) != expected or sha(OUT / group / relative) != expected:
                changed.append(relative)
    results["audit"] = {"source_count": len(sources), "input_count": len(inputs), "changed": changed}
    save(OUT / "results.json", results)
    print(json.dumps(results, indent=2))
sys.exit(1 if results["error"] or changed else 0)
