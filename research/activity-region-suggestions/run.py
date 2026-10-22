"""Frozen, file-only two-game replay; five PNGs each, no model or target access."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "test-results/activity-region-suggestions-v1"
CASES = [ROOT / f"test-results/runtime-appearances-v1/case-{i}.json" for i in range(2)]
SOURCES = [ROOT / f"test-results/appearance-context-v1/case-{i}-baseline.json" for i in range(2)]

def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def save(path, value):
    with path.open("x", encoding="utf-8") as file:
        json.dump(value, file, indent=2, ensure_ascii=False)
        file.write("\n")

OUT.mkdir()
inputs = set(CASES + SOURCES)
selected = []
generated = {}
for index, (case_file, source_file) in enumerate(zip(CASES, SOURCES)):
    case = read(case_file)
    source = read(source_file)["source"]
    assert len(case["frames"]) == 15
    assert case["investigation_id"] == source["investigation_id"]
    assert case["batch"] == source["batch"]
    first = next(i for i, f in enumerate(case["frames"]) if f["reference"]["id"] == source["pairs"][0]["before"]["id"])
    assert first > 0
    ids = {pair[edge]["id"] for pair in source["pairs"] for edge in ["before", "after"]}
    ids.add(case["frames"][first - 1]["reference"]["id"])
    assert len(ids) == 5
    selected.append(ids)
    folder = OUT / f"case-{index}"
    (folder / "images").mkdir(parents=True)
    replay = {"source": source, "frames": []}
    for f in case["frames"]:
        reference = f["reference"]
        relative = f"images/{reference['sha256']}.png"
        replay["frames"].append({"reference": reference, "file": relative})
        if reference["id"] in ids:
            path = Path(f["file"])
            assert path.resolve().is_relative_to(ROOT)
            assert sha(path) == reference["sha256"]
            inputs.add(path)
            dest = folder / relative
            if not dest.exists():
                shutil.copy2(path, dest)
            assert sha(dest) == reference["sha256"]
            generated[dest.relative_to(OUT).as_posix()] = sha(dest)
    path = folder / "case.json"
    save(path, replay)
    generated[path.relative_to(OUT).as_posix()] = sha(path)

sources = {p for p in (ROOT / "crates").rglob("*") if p.is_file() and p.suffix in {".rs", ".toml"}}
sources.update([ROOT / "Cargo.toml", ROOT / "Cargo.lock", Path(__file__), Path(__file__).with_name("PROTOCOL.md")])
manifest = {"sources": {}, "inputs": {}, "generated": generated}
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
        subprocess.run(["cargo", "build", "--offline", "-p", "reversebolt-vision", "--example", "activity-region-suggestions-replay"],
                       cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True,
                       creationflags=subprocess.CREATE_NO_WINDOW)
    binary = ROOT / "target/debug/examples/activity-region-suggestions-replay.exe"
    results["binary_sha256"] = sha(binary)
    for index, original in enumerate(CASES):
        case = {"name": read(original)["name"], "runs": []}
        results["cases"].append(case)
        outputs = []
        for repeat in range(2):
            output = OUT / f"case-{index}" / f"result-{repeat}.json"
            start = time.monotonic()
            process = subprocess.run([str(binary), str(output.parent / "case.json"), str(output)],
                                     cwd=ROOT, capture_output=True, text=True, timeout=120,
                                     creationflags=subprocess.CREATE_NO_WINDOW)
            receipt = {"exit_code": process.returncode, "seconds": time.monotonic() - start,
                       "stdout": process.stdout, "stderr": process.stderr}
            save(output.with_name(f"process-{repeat}.json"), receipt)
            case["runs"].append({"exit_code": process.returncode, "seconds": receipt["seconds"], "output": output.relative_to(OUT).as_posix()})
            if process.returncode:
                raise RuntimeError(f"Replay failed for case {index}; outcome retained")
            data = read(output)
            assert set(data["images_read"]) == selected[index] and len(data["images_read"]) == 5
            assert data["error"] is None, data["error"]
            assert data["report"]["attempted_frames"] <= 20
            assert len(data["report"]["suggestions"]) <= 4
            outputs.append(data)
        assert outputs[0] == outputs[1]
        data = outputs[0]
        report = data["report"]
        case.update({"status": report["status"], "attempted_frames": report["attempted_frames"],
                     "ranked_regions": report["ranked_regions"], "recommended": report["recommended"],
                     "images_read": data["images_read"], "repeat_equal": True,
                     "suggestions": [{key: s[key] for key in ["region", "action_minus_idle_change", "status", "displacement"]}
                                     for s in report["suggestions"]]})
        assert sha(binary) == results["binary_sha256"]
except Exception as error:
    results["error"] = str(error)
finally:
    changed = []
    for group in ["sources", "inputs"]:
        for relative, expected in manifest[group].items():
            if sha(ROOT / relative) != expected or sha(OUT / group / relative) != expected:
                changed.append(f"{group}/{relative}")
    for relative, expected in generated.items():
        if sha(OUT / relative) != expected:
            changed.append(relative)
    results["audit"] = {"source_count": len(sources), "input_count": len(inputs), "generated_count": len(generated), "changed": changed}
    save(OUT / "results.json", results)
    print(json.dumps(results, indent=2))
sys.exit(1 if results["error"] or changed else 0)
