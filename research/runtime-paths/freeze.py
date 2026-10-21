import hashlib,json
from pathlib import Path
from datetime import datetime,timezone
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
files=sorted(p for p in HERE.rglob('*') if p.is_file() and p.name!='frozen-v1.json')
files+=sorted(p for p in (ROOT/'crates/reversebolt-runtime').rglob('*') if p.is_file())
files += [ROOT/p for p in ['Cargo.toml','Cargo.lock','crates/reversebolt-core/src/lib.rs','crates/reversebolt-core/src/runtime.rs','crates/reversebolt-core/tests/runtime_paths.rs','research/runtime-objects/probe/src/bin/object-host.rs']]
files+=sorted((ROOT/'test-results/runtime-paths-v1/bin').glob('*'))
record={'frozen_utc':datetime.now(timezone.utc).isoformat(),'files':[{'path':str(p.relative_to(ROOT)).replace('\\','/'),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size} for p in files]}
with (HERE/'frozen-v1.json').open('x') as f:json.dump(record,f,indent=2)
print(json.dumps({'frozen_files':len(files)}))
