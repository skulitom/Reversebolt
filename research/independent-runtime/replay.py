"""Freeze and run one offline measured-contrast replay; never launch the editor."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "test-results/independent-runtime-v2"
OUT = ROOT / "test-results/measured-contrast-v1"
OUT.mkdir()
read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
native = read(BASE / "frozen.json")
data = Path(native["data"])
status = BASE / "discovery.json"
live = read(status)["state"]["live"]
receipt = data / "runtime/naming" / live["runtime"]["naming"]["evidence_id"] / "receipt.json"
captures = list((data / "captures").iterdir())
assert len(captures) == 1
binary = ROOT / "target/desktop-preview/release/examples/plan-replay.exe"
paths = list((ROOT / "crates").rglob("*.rs")) + list((ROOT / "crates").rglob("Cargo.toml"))
paths += [ROOT / "Cargo.toml", ROOT / "Cargo.lock", Path(__file__), Path(__file__).with_name("MEASURED-CONTRAST.md")]
sources = {}
for path in paths:
    name = path.relative_to(ROOT).as_posix()
    target = OUT / "source" / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(path, target)
    sources[name] = sha(target)
inputs = [status, receipt, Path(live["runtime"]["catalog_source"]["path"])]
inputs += [captures[0] / f"frame-{f['number']:03}.png" for f in live["frames"]]
frozen = {"parent": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
          "sources": sources, "binary": str(binary), "binary_sha256": sha(binary),
          "native_manifest_sha256": sha(BASE / "frozen.json"), "inputs": {str(p): sha(p) for p in inputs},
          "scope": "Offline recomputation and planning only; no native editor, model or target access."}
(OUT / "frozen.json").write_text(json.dumps(frozen, indent=2) + "\n", encoding="utf-8")
start = time.monotonic()
process = subprocess.Popen([str(binary), str(status), str(captures[0]), str(receipt), str(OUT / "replay.json")],
                           cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                           creationflags=subprocess.CREATE_NO_WINDOW)
try:
    stdout, stderr = process.communicate(timeout=30)
except subprocess.TimeoutExpired:
    process.terminate()
    stdout, stderr = process.communicate(timeout=15)
    stderr += "\nOffline replay deadline exceeded."
result = {"pid": process.pid, "exit_code": process.returncode, "seconds": time.monotonic() - start,
          "stdout": stdout, "stderr": stderr}
(OUT / "process.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
print(json.dumps(result))
