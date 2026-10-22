"""Frozen paired replay of old/new region selection, with unchanged passive evidence."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
OLD = ROOT / "test-results/activity-region-suggestions-v1"
OUT = ROOT / "test-results/stable-region-selection-v1"

def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def save(path, value):
    with path.open("x", encoding="utf-8") as file:
        json.dump(value, file, indent=2, ensure_ascii=False)
        file.write("\n")

OUT.mkdir()
inputs = {OLD / "manifest.json", OLD / "results.json"}
generated = {}
selected = []
old_results = read(OLD / "results.json")
assert sha(OLD / "manifest.json") == old_results["manifest_sha256"]
old_manifest = read(OLD / "manifest.json")
for index in range(2):
    folder = OLD / f"case-{index}"
    inputs.update([folder / "case.json", folder / "result-0.json"])
    assert sha(folder / "case.json") == old_manifest["generated"][f"case-{index}/case.json"]
    case = read(folder / "case.json")
    first = next(i for i, f in enumerate(case["frames"]) if f["reference"]["id"] == case["source"]["pairs"][0]["before"]["id"])
    ids = {p[k]["id"] for p in case["source"]["pairs"] for k in ["before", "after"]}
    ids.add(case["frames"][first - 1]["reference"]["id"])
    assert len(ids) == 5 and len(case["frames"]) == 15
    selected.append(ids)
    dest = OUT / f"case-{index}"
    (dest / "images").mkdir(parents=True)
    shutil.copy2(folder / "case.json", dest / "case.json")
    generated[f"case-{index}/case.json"] = sha(dest / "case.json")
    for f in case["frames"]:
        if f["reference"]["id"] not in ids:
            continue
        relative = f["file"]
        path = folder / relative
        assert path.resolve().is_relative_to(folder.resolve())
        assert sha(path) == f["reference"]["sha256"]
        assert sha(path) == old_manifest["generated"][f"case-{index}/{relative}"]
        inputs.add(path)
        shutil.copy2(path, dest / relative)
        generated[f"case-{index}/{relative}"] = sha(dest / relative)

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
    for index in range(2):
        case = {"name": old_results["cases"][index]["name"], "methods": {}}
        results["cases"].append(case)
        for method in ["change_only", "stable_texture"]:
            row = {"runs": []}
            case["methods"][method] = row
            outputs = []
            for repeat in range(2):
                output = OUT / f"case-{index}" / f"{method}-{repeat}.json"
                start = time.monotonic()
                process = subprocess.run([str(binary), str(output.parent / "case.json"), str(output), method],
                                         cwd=ROOT, capture_output=True, text=True, timeout=120,
                                         creationflags=subprocess.CREATE_NO_WINDOW)
                receipt = {"exit_code": process.returncode, "seconds": time.monotonic() - start,
                           "stdout": process.stdout, "stderr": process.stderr}
                save(output.with_name(f"{method}-{repeat}-process.json"), receipt)
                row["runs"].append({"exit_code": process.returncode, "seconds": receipt["seconds"], "output": output.relative_to(OUT).as_posix()})
                if process.returncode:
                    raise RuntimeError(f"Replay failed for {index}/{method}; outcome retained")
                data = read(output)
                assert set(data["images_read"]) == selected[index] and len(data["images_read"]) == 5
                assert data["error"] is None, data["error"]
                report = data["report"]
                assert report["attempted_frames"] <= 20 and len(report["suggestions"]) <= 4
                assert sum(len(s["frames"]) for s in report["suggestions"]) == report["attempted_frames"]
                if method == "change_only":
                    assert data == read(OLD / f"case-{index}/result-0.json"), "Legacy output changed"
                else:
                    s = report["selection"]
                    assert s["input_regions"] == sum(s[k] for k in ["eligible_regions", "rejected_low_texture", "rejected_weak_corner", "rejected_quiet_change"])
                outputs.append(data)
            assert outputs[0] == outputs[1]
            report = outputs[0]["report"]
            row.update({"status": report["status"], "attempted_frames": report["attempted_frames"],
                        "ranked_regions": report["ranked_regions"], "selection": report.get("selection"),
                        "recommended": report["recommended"], "repeat_equal": True,
                        "prior_output_equal": True if method == "change_only" else None,
                        "suggestions": [{"region": s["region"], "change": s["action_minus_idle_change"],
                                         "quality": s.get("selection_quality"), "status": s["status"],
                                         "displacement": s["displacement"], "checks": [m["status"] for m in s["frames"]]}
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
