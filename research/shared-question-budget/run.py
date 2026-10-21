"""Freeze and run one offline replay per development case; no native target access."""
import hashlib
import json
import shutil
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "test-results/shared-question-budget-v1"
BASE = ROOT / "test-results/object-label-selection-v1"
read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()

OUT.mkdir()  # A replay never overwrites previous evidence.
(OUT / "bin").mkdir()
(OUT / "revised").mkdir()
exe = OUT / "bin/reversebolt-runtime-hypotheses.exe"
shutil.copy2(ROOT / "target/release/reversebolt-runtime-hypotheses.exe", exe)
baseline = read(BASE / "revised-frozen.json")
source_files = ["Cargo.lock", "Cargo.toml", "research/shared-question-budget/PROTOCOL.md",
                "research/shared-question-budget/run.py"]
for crate in ["reversebolt-core", "reversebolt-observer"]:
    source_files.append(f"crates/{crate}/Cargo.toml")
    source_files.extend(str(p.relative_to(ROOT)).replace("\\", "/")
                        for p in (ROOT / f"crates/{crate}/src").rglob("*.rs"))
sources = {}
for name in source_files:
    destination = OUT / "source" / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / name, destination)
    sources[name] = sha(destination)

model = read(BASE / "revised/latest/receipt.json")["model"]
manifest = {
    "created_utc": datetime.now(timezone.utc).isoformat(),
    "parent": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
    "binary": str(exe.relative_to(OUT)), "binary_sha256": sha(exe),
    "sources": sources, "cases": baseline["cases"], "model": model,
    "baseline": str(BASE), "baseline_manifest_sha256": sha(BASE / "revised-frozen.json"),
    "baseline_receipts": {c["name"]: sha(BASE / "revised" / c["name"] / "receipt.json")
                          for c in baseline["cases"]},
}
(OUT / "frozen.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
for case in manifest["cases"]:
    assert sha(Path(case["source"])) == case["sha256"]
    assert sha(exe) == manifest["binary_sha256"]
    command = [str(exe), case["source"], str(OUT / "revised" / case["name"]), model,
               *case.get("flags", [])]
    started = time.monotonic()
    process = subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               text=True, creationflags=subprocess.CREATE_NO_WINDOW)
    print(json.dumps({"case": case["name"], "pid": process.pid, "state": "running"}), flush=True)
    stdout, stderr = process.communicate()
    result = {"pid": process.pid, "exit_code": process.returncode,
              "seconds": time.monotonic() - started, "stdout": stdout, "stderr": stderr}
    (OUT / "revised" / (case["name"] + "-process.json")).write_text(
        json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"case": case["name"], **result}), flush=True)
