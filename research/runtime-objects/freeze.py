import hashlib,json
from pathlib import Path
from datetime import datetime,timezone
here=Path(__file__).resolve().parent;root=here.parents[1]
files=sorted(p for p in here.rglob('*') if p.is_file() and p.suffix in {'.rs','.py','.toml','.lock','.md','.cs','.txt','.json'} and p.name!='frozen-v1.json')
files += [root/'research/runtime-probe/probe/src'/s for s in ['win.rs','bin/probe-loader.rs']]
files += [root/'target/runtime-objects/debug'/s for s in ['reversebolt_runtime_probe.dll','reversebolt_mock_runtime.dll','probe-loader.exe','object-host.exe']]
record={'frozen_utc':datetime.now(timezone.utc).isoformat(),'files':[{'path':str(p.relative_to(root)).replace('\\','/'),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size} for p in files]}
with (here/'frozen-v1.json').open('x') as f:json.dump(record,f,indent=2)
print(json.dumps({'frozen_files':len(files)}))
