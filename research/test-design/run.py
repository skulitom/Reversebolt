"""Freeze 24 saved-plan development requests; never run against a game."""
import copy
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/test-design-v1'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
OUT.mkdir()
exe=ROOT/'target/release/examples/test-design-replay.exe'
shutil.copy2(exe,OUT/'evaluator.exe');exe=OUT/'evaluator.exe'
plans={}
for name,folder in [('viewfinder','viewfinder-live-paging-v1'),('sable','sable-live-context-v1')]:
    source=ROOT/'test-results'/folder/'chosen-plan.json'
    shutil.copy2(source,OUT/f'{name}-source.json')
    plans[name]=json.loads(source.read_text())
    assert len(plans[name]['plan']['choices'])==10
cases={name:copy.deepcopy(p) for name,p in plans.items()}
for name,source,index in [('opaque','viewfinder',2),('brightness','viewfinder',2),
                          ('viewfinder_metadata_event','viewfinder',1),('sable_metadata_event','sable',0)]:
    case=copy.deepcopy(plans[source]);plan=case['plan'];choice=copy.deepcopy(plan['choices'][index])
    # Only choices, activity and report/actions feed the Rust input. Strip the
    # old chosen args and next entry to avoid misleading transformed artifacts.
    case={'plan':{k:v for k,v in plan.items() if k not in ('next','choices')}}
    case['plan']['choices']=[choice]
    if name=='opaque':
        choice['selected_candidate'].update(type_name='Container',field_name='F',object_text=[])
        choice.update(name='Unknown',question='Does this field affect the observed pattern?',explanation='No semantic label is available.')
    elif name=='brightness':
        activity=case['plan']['activity_context']
        activity.update(motion=None,appearances=None,regions=None)
        activity['observation']={'consistent_visible_change':True,
            'description':'In both pairs the visible surfaces become brighter. Their shapes and positions appear unchanged.',
            'uncertainty':'The source of brightness change is unknown; no movement is established.'}
    else:
        choice['selected_candidate']['object_text'].append('Footsteps were heard in this clip.')
    cases[name]=case
for name,p in cases.items():(OUT/f'{name}-plan.json').write_text(json.dumps(p,indent=2)+'\n')
sources={}
files=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
files += [ROOT/p for p in ('Cargo.toml','Cargo.lock','rust-toolchain.toml','research/test-design/PROTOCOL.md','research/test-design/run.py')]
for p in files:
    name=p.relative_to(ROOT).as_posix();target=OUT/'source'/name
    target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,target);sources[name]=sha(target)
frozen={'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    'working_changes_included':True,'sources':sources,'evaluator_sha256':sha(exe),'model':'qwen3.8:latest',
    'plans':{name:sha(OUT/f'{name}-plan.json') for name in cases},
    'source_plans':{name:sha(OUT/f'{name}-source.json') for name in plans},'requests':24}
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2)+'\n')
results=[]
for name in cases:
    assert sha(exe)==frozen['evaluator_sha256']
    done=subprocess.run([str(exe),str(OUT/f'{name}-plan.json'),str(OUT/name),frozen['model']],cwd=ROOT,text=True,capture_output=True)
    (OUT/f'{name}.stdout').write_text(done.stdout);(OUT/f'{name}.stderr').write_text(done.stderr)
    results.append({'case':name,'exit_code':done.returncode});print(json.dumps(results[-1]),flush=True)
    if done.returncode:break
(OUT/'result.json').write_text(json.dumps(results,indent=2)+'\n')
sys.exit(1 if any(r['exit_code'] for r in results) else 0)
