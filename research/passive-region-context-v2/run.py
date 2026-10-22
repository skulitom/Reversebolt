"""Source-bound numeric context replay, followed by four fixed local model calls."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
OLD = ROOT / "test-results/stable-region-selection-v1"
HORIZON = ROOT / "test-results/activity-horizon-v1"
PRIOR = ROOT / "test-results/passive-region-context-v1"
OUT = ROOT / "test-results/passive-region-context-v2"
MODEL = "qwen3.8:latest"
DIGEST = "22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643"

def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def save(path, value):
    with path.open("x", encoding="utf-8") as file:
        json.dump(value, file, indent=2, ensure_ascii=False)
        file.write("\n")

def model():
    with urllib.request.urlopen("http://127.0.0.1:11434/api/tags", timeout=5) as response:
        tags = json.load(response)
    assert any(m["name"] == MODEL and m["digest"] == DIGEST for m in tags["models"])
    return {"name": MODEL, "digest": DIGEST}

model_before = model()
OUT.mkdir()
save(OUT / "model-before.json", model_before)
inputs = {OLD / "manifest.json", OLD / "results.json"}
generated = {"model-before.json": sha(OUT / "model-before.json")}
old_manifest = read(OLD / "manifest.json")
for i in range(2):
    folder = OLD / f"case-{i}"
    inputs.update([folder / "case.json", folder / "stable_texture-0.json", HORIZON / f"case-{i}-augmented.json",
                   PRIOR / f"case-{i}/summary-0.json", PRIOR / f"case-{i}/baseline-receipt.json", PRIOR / f"case-{i}/augmented-receipt.json"])
    case = read(folder / "case.json")
    expected = read(folder / "stable_texture-0.json")
    ids = set(expected["images_read"])
    assert len(ids) == 5 and len(case["frames"]) == 15
    dest = OUT / f"case-{i}"
    (dest / "images").mkdir(parents=True)
    shutil.copy2(folder / "case.json", dest / "case.json")
    generated[f"case-{i}/case.json"] = sha(dest / "case.json")
    assert generated[f"case-{i}/case.json"] == old_manifest["generated"][f"case-{i}/case.json"]
    for f in case["frames"]:
        if f["reference"]["id"] not in ids:
            continue
        path = folder / f["file"]
        assert path.resolve().is_relative_to(folder.resolve())
        assert sha(path) == f["reference"]["sha256"]
        inputs.add(path)
        shutil.copy2(path, dest / f["file"])
        generated[f"case-{i}/{f['file']}"] = sha(dest / f["file"])

sources = {p for p in (ROOT / "crates").rglob("*") if p.is_file() and p.suffix in {".rs", ".toml"}}
sources.update([ROOT / "Cargo.toml", ROOT / "Cargo.lock", Path(__file__), Path(__file__).with_name("PROTOCOL.md")])
manifest = {"sources": {}, "inputs": {}, "generated": generated.copy()}
for group, paths in [("sources", sources), ("inputs", inputs)]:
    for path in sorted(paths):
        relative = path.relative_to(ROOT).as_posix()
        dest = OUT / group / relative
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, dest)
        manifest[group][relative] = sha(path)
save(OUT / "manifest.json", manifest)
results = {"manifest_sha256": sha(OUT / "manifest.json"), "cases": [], "error": None, "binaries": {}}

def run(binary, args, label):
    start = time.monotonic()
    process = subprocess.run([str(binary), *map(str, args)], cwd=ROOT, capture_output=True, text=True,
                             timeout=180, creationflags=subprocess.CREATE_NO_WINDOW)
    receipt = {"exit_code": process.returncode, "seconds": time.monotonic() - start,
               "stdout": process.stdout, "stderr": process.stderr}
    save(OUT / f"{label}-process.json", receipt)
    if process.returncode:
        raise RuntimeError(f"{label} failed; outcome retained")
    return receipt

try:
    for package, name in [("reversebolt-vision", "activity-region-suggestions-replay"), ("reversebolt-observer", "activity-replay")]:
        with (OUT / f"{name}-build.log").open("x", encoding="utf-8") as log:
            subprocess.run(["cargo", "build", "--offline", "-p", package, "--example", name], cwd=ROOT,
                           stdout=log, stderr=subprocess.STDOUT, check=True, creationflags=subprocess.CREATE_NO_WINDOW)
        results["binaries"][name] = sha(ROOT / f"target/debug/examples/{name}.exe")
    interpretation_manifests = []
    for i in range(2):
        folder = OUT / f"case-{i}"
        case = read(folder / "case.json")
        outputs = []
        for repeat in range(2):
            output = folder / f"summary-{repeat}.json"
            run(ROOT / "target/debug/examples/activity-region-suggestions-replay.exe",
                [folder / "case.json", output, "stable_texture", "--summary"], f"summary-{i}-{repeat}")
            data = read(output)
            assert data["summary_error"] is None and data["summary"] is not None
            prior_shape = {k: v for k, v in data.items() if k not in ["summary", "summary_error"]}
            assert prior_shape == read(OLD / f"case-{i}/stable_texture-0.json")
            outputs.append(data)
            generated[output.relative_to(OUT).as_posix()] = sha(output)
        assert outputs[0] == outputs[1]
        assert outputs[0] == read(PRIOR / f"case-{i}/summary-0.json")
        summary = outputs[0]["summary"]
        base = read(HORIZON / f"case-{i}-augmented.json")
        assert base["source"] == case["source"]
        lookup = {f["reference"]["id"]: str(folder / f["file"]) for f in case["frames"]}
        base["frames"] = [lookup[p[k]["id"]] for p in case["source"]["pairs"] for k in ["before", "after"]]
        row = {"name": read(OLD / "results.json")["cases"][i]["name"], "proposal_equal": True,
               "summary_repeat_equal": True, "regions": summary["regions"], "arms": {}}
        results["cases"].append(row)
        for arm in ["baseline", "augmented"]:
            path = folder / f"{arm}.json"
            payload = dict(base)
            if arm == "augmented":
                payload["regions"] = summary
            save(path, payload)
            generated[path.relative_to(OUT).as_posix()] = sha(path)
            interpretation_manifests.append((i, arm, path))
    save(OUT / "interpretation-inputs-frozen.json", generated)
    results["interpretation_inputs_sha256"] = sha(OUT / "interpretation-inputs-frozen.json")
    for i, arm, path in interpretation_manifests:
        save(OUT / f"{i}-{arm}-model.json", model())
        output = path.with_name(f"{arm}-receipt.json")
        process = run(ROOT / "target/debug/examples/activity-replay.exe", [path, output, MODEL], f"{i}-{arm}")
        receipt = read(output)
        supplied = read(path)
        assert receipt.get("regions") == supplied.get("regions")
        assert receipt.get("appearances") == supplied.get("appearances")
        old_receipt = read(PRIOR / f"case-{i}/{arm}-receipt.json")
        if arm == "baseline":
            assert receipt["stage"]["prompt"] == old_receipt["stage"]["prompt"]
        else:
            assert len(receipt["stage"]["prompt"].encode()) < len(old_receipt["stage"]["prompt"].encode())
        summary = json.loads(process["stdout"])
        if receipt["error"] is None:
            assert summary["context_retained"]
            assert summary["regions_retained"] == supplied.get("regions")
        results["cases"][i]["arms"][arm] = {"error": receipt["error"], "observation": receipt["observation"],
            "seconds": process["seconds"], "context_retained": summary["context_retained"],
            "exact_regions_retained": summary["regions_retained"] == supplied.get("regions"),
            "done_reason": receipt["stage"]["response"].get("done_reason") if receipt["stage"] else None,
            "prompt_tokens": receipt["stage"]["response"].get("prompt_eval_count") if receipt["stage"] else None,
            "output_tokens": receipt["stage"]["response"].get("eval_count") if receipt["stage"] else None,
            "prompt_bytes": len(receipt["stage"]["prompt"].encode()) if receipt["stage"] else None,
            "summary_bytes": len(json.dumps(supplied["regions"], separators=(',', ':')).encode()) if "regions" in supplied else 0}
        print(json.dumps({"case": i, "arm": arm, **results["cases"][i]["arms"][arm]}), flush=True)
    for i in range(2):
        a = read(OUT / f"case-{i}/baseline-receipt.json")
        b = read(OUT / f"case-{i}/augmented-receipt.json")
        assert a["motion"] == b["motion"] and a["appearances"] == b["appearances"] and a["source"] == b["source"]
    save(OUT / "model-after.json", model())
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
    for name, expected in results["binaries"].items():
        if sha(ROOT / f"target/debug/examples/{name}.exe") != expected:
            changed.append(name)
    results["audit"] = {"source_count": len(sources), "input_count": len(inputs), "generated_count": len(generated), "changed": changed}
    save(OUT / "results.json", results)
    print(json.dumps(results, indent=2))
sys.exit(1 if results["error"] or changed else 0)
