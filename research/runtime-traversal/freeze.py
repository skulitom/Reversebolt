import hashlib,json
from pathlib import Path
from datetime import datetime,timezone
here=Path(__file__).resolve().parent;root=here.parents[1]
files=sorted(p for p in (root/'crates/reversebolt-runtime').rglob('*') if p.is_file())
files+=sorted(p for p in here.rglob('*') if p.is_file() and p.name!='frozen-v1.json')
files += [root/p for p in ['Cargo.toml','Cargo.lock','research/runtime-objects/probe/src/bin/object-host.rs','target/runtime-objects/debug/object-host.exe','target/debug/reversebolt_runtime.dll','target/debug/reversebolt-runtime-loader.exe','target/runtime-traversal/debug/reversebolt_traversal_fixture.dll']]
record={'frozen_utc':datetime.now(timezone.utc).isoformat(),'files':[{'path':str(p.relative_to(root)).replace('\\','/'),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size} for p in files]}
with (here/'frozen-v1.json').open('x') as f:json.dump(record,f,indent=2)
print(json.dumps({'frozen_files':len(files)}))
