"""Frozen paired replay. Only original baseline files are made available."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
OLD = ROOT / "test-results/trial-appearance-suggestions-v1"
OUT = ROOT / "test-results/stable-baseline-regions-v1"
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
old_results = read(OLD / "results.json")
assert sha(OLD / "manifest.json") == old_results["manifest_sha256"]
old_manifest = read(OLD / "manifest.json")
inputs = set(RECORDS) | {OLD / "manifest.json", OLD / "results.json"}
for index, record in enumerate(RECORDS):
    assert record.stem == old_results["cases"][index]["test_id"]
    test = read(record)
    assert test["candidate"]["locator"]["root"]["declaring"]["image"] == "OwnedPathFixture"
    inputs.add(OLD / f"case-{index}-0.json")
    for frame in test["conditions"][0]["frames"]:
        for key, digest in [("path", "sha256"), ("input_path", "input_sha256")]:
            path = record.parent / frame[key]
            assert path.resolve().is_relative_to(record.parent.resolve())
            assert sha(path) == frame[digest]
            inputs.add(path)
for relative, expected in old_manifest["inputs"].items():
    assert ROOT / relative in inputs and sha(ROOT / relative) == expected
    assert sha(OLD / "inputs" / relative) == expected

OUT.mkdir()
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
                       cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True,
                       creationflags=subprocess.CREATE_NO_WINDOW)
    binary = ROOT / "target/debug/examples/trial-appearance-suggestions-replay.exe"
    results["binary_sha256"] = sha(binary)
    for index, original in enumerate(RECORDS):
        record = OUT / "inputs" / original.relative_to(ROOT)
        frame_count = len(read(record)["conditions"][0]["frames"])
        case = {"test_id": record.stem, "baseline_frames": frame_count, "methods": {}}
        results["cases"].append(case)
        for method in ["change_only", "stable_texture"]:
            row = {"runs": []}
            case["methods"][method] = row
            outputs = []
            for repeat in range(2):
                output = OUT / f"case-{index}-{method}-{repeat}.json"
                start = time.monotonic()
                process = subprocess.run([str(binary), str(record), str(output), method],
                                         cwd=ROOT, capture_output=True, text=True, timeout=120,
                                         creationflags=subprocess.CREATE_NO_WINDOW)
                receipt = {"exit_code": process.returncode, "seconds": time.monotonic() - start,
                           "stdout": process.stdout, "stderr": process.stderr}
                save(output.with_name(output.stem + "-process.json"), receipt)
                row["runs"].append({"exit_code": process.returncode, "seconds": receipt["seconds"], "output": output.name})
                if process.returncode:
                    raise RuntimeError(f"Replay failed for {index}/{method}; original outcome retained")
                data = read(output)
                assert data["error"] is None, data["error"]
                assert all(c == 0 for c, _ in data["images_read"] + data["inputs_read"])
                if frame_count != 6:
                    assert data["images_read"] == data["inputs_read"] == []
                else:
                    assert data["images_read"] == [[0, i] for i in range(6)]
                    assert data["inputs_read"] == [[0, i] for i in range(1, 6)]
                report = data["report"]
                assert report["attempted_frames"] <= 24 and len(report["suggestions"]) <= 4
                assert sum(len(s["frames"]) for s in report["suggestions"]) == report["attempted_frames"]
                if method == "change_only":
                    assert data == read(OLD / f"case-{index}-0.json"), "Legacy output changed"
                elif frame_count == 6:
                    s = report["selection"]
                    assert s["input_regions"] == sum(s[k] for k in ["eligible_regions", "rejected_low_texture", "rejected_weak_corner", "rejected_quiet_change"])
                outputs.append(data)
            assert outputs[0] == outputs[1]
            data = outputs[0]
            report = data["report"]
            row.update({"status": report["status"], "attempted_frames": report["attempted_frames"],
                        "ranked_regions": report["ranked_regions"], "selection": report.get("selection"),
                        "recommended": report["recommended"], "repeat_equal": True,
                        "prior_output_equal": True if method == "change_only" else None,
                        "images_read": data["images_read"], "inputs_read": data["inputs_read"],
                        "suggestions": [{"selection": s["selection"], "change": s["action_minus_idle_change"],
                                         "quality": s.get("selection_quality"), "status": s["status"],
                                         "displacement": s["displacement"], "checks": [m["status"] for m in s["frames"]]}
                                        for s in report["suggestions"]]})
            assert sha(binary) == results["binary_sha256"]
except Exception as error:
    results["error"] = str(error)
finally:
    changed = []
    for group, paths in manifest.items():
        for relative, expected in paths.items():
            if sha(ROOT / relative) != expected or sha(OUT / group / relative) != expected:
                changed.append(f"{group}/{relative}")
    results["audit"] = {"source_count": len(sources), "input_count": len(inputs), "changed": changed}
    save(OUT / "results.json", results)
    print(json.dumps(results, indent=2))
sys.exit(1 if results["error"] or changed else 0)
