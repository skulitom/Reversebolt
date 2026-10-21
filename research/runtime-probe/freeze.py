"""Run once after checks and before native evaluation. Never overwrite a receipt."""
import hashlib
import json
from pathlib import Path
from datetime import datetime, timezone
root = Path(__file__).resolve().parent
repo = root.parents[1]
files = sorted(p for p in root.rglob("*") if p.is_file() and p.suffix in {".rs",".py",".toml",".lock",".md"})
files += [repo / "target/runtime-probe/debug" / n for n in ["reversebolt_runtime_probe.dll","probe-loader.exe","reversebolt_mock_runtime.dll","mock-host.exe"]]
record = {"frozen_utc":datetime.now(timezone.utc).isoformat(),"files":[{"path":str(p.relative_to(repo)).replace("\\","/"),"bytes":p.stat().st_size,"sha256":hashlib.sha256(p.read_bytes()).hexdigest()} for p in files]}
with (root / "frozen-v1.json").open("x") as f:
    json.dump(record,f,indent=2)
print(json.dumps({"frozen_files":len(files),"helper_sha256":next(x["sha256"] for x in record["files"] if x["path"].endswith("reversebolt_runtime_probe.dll"))}))
