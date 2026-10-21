"""Replay retained model abstentions locally; this runner makes zero model calls."""
import hashlib
import json
import shutil
import subprocess
import time
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "test-results/object-label-selection-v1"
VERSION = sys.argv[1] if len(sys.argv) > 1 else "v2"
assert VERSION in ["v1", "v2"]
OUT = ROOT / ("test-results/object-text-questions-" + VERSION)
read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
OUT.mkdir()
(OUT / "bin").mkdir()
(OUT / "replayed").mkdir()
exe = OUT / "bin/reversebolt-runtime-hypotheses.exe"
shutil.copy2(ROOT / "target/release/reversebolt-runtime-hypotheses.exe", exe)
files = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml",
         "research/object-text-questions/PROTOCOL.md", "research/object-text-questions/PROTOCOL-v2.md", "research/object-text-questions/run.py"]
for crate in ["reversebolt-core", "reversebolt-observer", "reversebolt-app"]:
    files.append(f"crates/{crate}/Cargo.toml")
    files.extend(str(p.relative_to(ROOT)).replace("\\", "/")
                 for folder in ["src", "tests"] for p in (ROOT / f"crates/{crate}/{folder}").rglob("*.rs"))
sources = {}
for name in files:
    copy = OUT / "source" / name
    copy.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / name, copy)
    sources[name] = sha(copy)
cases = read(BASE / "revised-frozen.json")["cases"]
frozen = {"parent": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
          "sources": sources, "binary": str(exe.relative_to(OUT)), "binary_sha256": sha(exe),
          "cases": cases, "baseline": str(BASE),
          "receipts": {c["name"]: sha(BASE / "revised" / c["name"] / "receipt.json") for c in cases},
          "desktop_sample_sha256": sha(ROOT / "target/release/Reversebolt.exe")}
(OUT / "frozen.json").write_text(json.dumps(frozen, indent=2) + "\n", encoding="utf-8")
for case in cases:
    assert sha(Path(case["source"])) == case["sha256"]
    prior = BASE / "revised" / case["name"] / "receipt.json"
    assert sha(prior) == frozen["receipts"][case["name"]]
    command = [str(exe), case["source"], str(OUT / "replayed" / case["name"]), read(prior)["model"],
               *case.get("flags", []), "--replay", str(prior)]
    started = time.monotonic()
    process = subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               text=True, creationflags=subprocess.CREATE_NO_WINDOW)
    try:
        stdout, stderr = process.communicate(timeout=30)
    except subprocess.TimeoutExpired:
        process.terminate()  # Only this offline replay process is owned here.
        stdout, stderr = process.communicate(timeout=15)
        stderr += "\nReplay exceeded its 30-second deadline."
    result = {"pid":process.pid, "exit_code":process.returncode, "seconds":time.monotonic()-started,
              "stdout":stdout, "stderr":stderr}
    (OUT / "replayed" / (case["name"] + "-process.json")).write_text(
        json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"case":case["name"], **result}), flush=True)
