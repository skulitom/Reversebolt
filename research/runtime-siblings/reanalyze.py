"""Freeze and analyze existing native measurements after the batch-boundary fix."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/runtime-siblings-sable-selected-v1'
DEST=OUT/'repair'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
result=read(OUT/'result.json')
assert read(OUT/'anode-job.json')['finished'] and read(OUT/'anode-job.json')['exitCode']==0
assert all(b['exit_code']!=0 and 'one to four distinct runtime values' in b['error'] for b in result['batches'])
assert not read(OUT/'cleanup.json')['owned_remaining']
DEST.mkdir()
frozen={'sources':{},'inputs':{},'original_analysis_rejections':[b['error'] for b in result['batches']]}
for name in ['crates/reversebolt-core/src/runtime.rs','crates/reversebolt-core/src/runtime_response.rs',
    'crates/reversebolt-core/src/runtime_neighbors.rs','crates/reversebolt-core/tests/runtime_paths.rs',
    'research/runtime-siblings/REANALYSIS-PROTOCOL.md','research/runtime-siblings/reanalyze.py']:
    source=ROOT/name;destination=DEST/'source'/name
    destination.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source,destination)
    frozen['sources'][name]=sha(destination)
binary=DEST/'reversebolt-runtime-response.exe'
shutil.copy2(ROOT/'target/release/reversebolt-runtime-response.exe',binary)
frozen['binary_sha256']=sha(binary)
plan=OUT/'plan/plan.json';frozen['inputs']['plan/plan.json']=sha(plan)
for b in result['batches']:
    name=f"observations-{b['batch']}.json";frozen['inputs'][name]=sha(OUT/name)
(DEST/'frozen.json').write_text(json.dumps(frozen,indent=2)+'\n')
replayed=[]
for b in result['batches']:
    index=b['batch'];observations=OUT/f'observations-{index}.json';output=DEST/f'analysis-{index}.json'
    run=subprocess.run([str(binary),str(plan),str(index),str(observations),str(output)],
        capture_output=True,text=True,timeout=30)
    (DEST/f'analysis-{index}.stdout').write_text(run.stdout);(DEST/f'analysis-{index}.stderr').write_text(run.stderr)
    replayed.append({'batch':index,'exit_code':run.returncode,'error':run.stderr,
        'analysis':read(output) if run.returncode==0 else None})
(DEST/'result.json').write_text(json.dumps({'batches':replayed},indent=2)+'\n')
print(json.dumps([{'batch':b['batch'],'exit':b['exit_code'],
    'associations':sum(f['repeated_association'] for f in b['analysis']['fields']) if b['analysis'] else None,
    'error':b['error']} for b in replayed]))
