"""Freeze and run four local saved-data comparisons, once each."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/('test-results/'+sys.argv[1]) if len(sys.argv)>1 else ROOT/'test-results/comparison-priority-v1'
EXE=ROOT/'target/release/examples/comparison-priority-replay.exe'
SOURCE=ROOT/'test-results/viewfinder-live-paging-v1/chosen-plan.json'
sha=lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
OUT.mkdir()
shutil.copy2(SOURCE,OUT/'chosen-plan.json')
shutil.copy2(EXE,OUT/'evaluator.exe')
EXE=OUT/'evaluator.exe'
files=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
files += [ROOT/p for p in ('Cargo.toml','Cargo.lock','research/comparison-priority/PROTOCOL.md','research/comparison-priority/run.py')]
hashes={}
for p in files:
    name=p.relative_to(ROOT).as_posix(); dest=OUT/'source'/name
    dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dest);hashes[name]=sha(dest)
frozen={'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    'working_changes_included':True,'sources':hashes,'source_plan_sha256':sha(SOURCE),
    'evaluator':str(EXE),'evaluator_sha256':sha(EXE),'model':'qwen3.8:latest',
    'cases':['original','reversed','opaque','other_activity']}
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2)+'\n')
results=[]
for case in frozen['cases']:
    assert sha(EXE)==frozen['evaluator_sha256']
    done=subprocess.run([str(EXE),str(OUT/'chosen-plan.json'),str(OUT/case),frozen['model'],case],cwd=ROOT,text=True,capture_output=True)
    (OUT/f'{case}.stdout').write_text(done.stdout)
    (OUT/f'{case}.stderr').write_text(done.stderr)
    results.append({'case':case,'exit_code':done.returncode})
    print(done.stdout or done.stderr,flush=True)
    if done.returncode: break
(OUT/'result.json').write_text(json.dumps(results,indent=2)+'\n')
sys.exit(1 if any(r['exit_code'] for r in results) else 0)
