"""Freeze the selection-only repair against the same still-live discovery."""
import ast, hashlib, json, shutil, subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
PARENT=ROOT/'test-results/runtime-recovery-declared-buffered-v1'
OUT=ROOT/'test-results/runtime-recovery-declared-continuation-v1'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert 'Search internal values first.' in read(PARENT/'failure.json')['error']
state=read(PARENT/'final-state.json')
assert not state['state']['runtime_tests'] and not state['state']['runtime_trials']
assert state['jobs']['in_flight'] is None and state['state']['live']['runtime']['catalog_source']
assert read(PARENT/'recovery.json')['status']=='candidate_for_revalidation'
ast.parse((ROOT/'research/runtime-recovery/live.py').read_text())
OUT.mkdir();(OUT/'bin').mkdir();(OUT/'source').mkdir()
frozen={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    'continuation_of':str(PARENT),'data':str(PARENT/'data'),'recovery_policy':'declared_route_proposal',
    'sources':{},'binaries':{},'historical':{},'parent_artifacts':{},'maximum_comparisons':1,
    'maximum_setup_images':0,'maximum_menu_actions':0,'model_calls':0}
for name,digest in read(PARENT/'frozen.json')['binaries'].items():
    assert sha(PARENT/'bin'/name)==digest
    shutil.copy2(PARENT/'bin'/name,OUT/'bin'/name);frozen['binaries'][name]=digest
for p in (ROOT/'research/runtime-recovery').glob('*.*'):
    name=p.relative_to(ROOT).as_posix();dest=OUT/'source'/name;dest.parent.mkdir(parents=True,exist_ok=True)
    shutil.copy2(p,dest);frozen['sources'][name]=sha(dest)
for name in ['source-test','source-report','current-report','recovery']:
    file=name+'.json';shutil.copy2(PARENT/file,OUT/file);frozen['historical'][file]=sha(OUT/file)
for name in ['frozen','failure','discovery','final-state','recovery-terminal','anode-terminal']:
    p=PARENT/(name+'.json');frozen['parent_artifacts'][str(p)]=sha(p)
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2),encoding='utf-8')
print(OUT)
