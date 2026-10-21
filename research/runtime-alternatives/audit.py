"""Audit the completed comparative experiment; never controls or edits a game."""
import hashlib,json,subprocess,sys
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve()
def read(p):return json.loads(p.read_bytes())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json')
for name,digest in frozen['product_binaries'].items():assert sha(OUT/'bin'/name)==digest
source_notes=[]
for name,digest in frozen['evaluator_sources'].items():
    if sha(ROOT/name)==digest:continue
    # The prelaunch correction used '/' while the original Windows key used '\\'.
    # Verify the exact two-line correction rather than silently replacing history.
    assert name=='research\\runtime-alternatives\\client.py' and frozen['prelaunch_corrections']
    current=(ROOT/name).read_text()
    prior=current.replace("call('start_runtime_job',dict(**envelope(s),task=dict(operation='refresh',catalog_id=rt['catalog_id'],after_revision=rt['revision'],ids=ids),count=1,interval_ms=250))", "call('start_runtime_job',dict(**envelope(s),catalog_id=rt['catalog_id'],after_revision=rt['revision'],ids=ids,count=1,interval_ms=50))")
    prior=prior.replace("checks={}\n        for item in chosen: checks.update(refresh(item['paths']))", "checks=refresh(ids)")
    assert hashlib.sha256(prior.encode()).hexdigest()==digest
    assert sha(ROOT/name)==frozen['evaluator_sources']['research/runtime-alternatives/client.py']
    source_notes.append('Both original and corrected prelaunch client hashes verified; duplicate path separators retained in frozen manifest.')
assert sha(Path(frozen['proposal_path']))==frozen['proposal_sha256']
assert sha(Path(frozen['source_report']))==frozen['source_sha256']
assert sha(ROOT/'research/runtime-alternatives/capture.js')==read(OUT/'capture-implementation.json')['sha256']
selected=read(OUT/'selection.json');inventory=read(OUT/'inventory.json')
old=read(Path(frozen['source_report']));assert inventory['scope']['created']!=old['created']
outcomes=[];frame_count=pulses=0
for index,candidate in enumerate(selected['candidates']):
    case=OUT/('candidate-'+str(index))
    # Recompute all image metrics using the frozen preceding method.
    subprocess.run([sys.executable,str(ROOT/'research/manifold-runtime/measure.py'),str(case)],check=True,capture_output=True)
    measurement=read(case/'movement-measurement.json')
    for stage in ('baseline','changed','restored'):
        actions=read(case/(stage+'-actions.json'));assert actions['status']=='completed'
        assert actions['pid']==read(OUT/'target.json')['pid']
        keys=[a for a in actions['actions'] if a['action']=='key'];assert [a['key'] for a in keys]==['d','a','d','a']
        assert all(a['holdMs']==200 and a['result'][0]['text']=='ok' for a in keys)
        images=[a for a in actions['actions'] if a['action']=='screenshot'];assert len(images)==6
        assert all((case/'frames'/(a['label']+'.png')).is_file() for a in images)
        pulses+=len(keys);frame_count+=len(images)
    write=read(OUT/f'candidate-{index}-write.json')['data'];restore=read(OUT/f'candidate-{index}-restore.json')['data'];recovery=read(OUT/f'candidate-{index}-recovery.json')
    after=read(OUT/f'candidate-{index}-after-change-checks.json');final=read(OUT/f'candidate-{index}-final-checks.json')
    assert write['write']['outcome']=='written' and write['patch']['path']['scope']==inventory['scope']
    assert all(c['bytes']==write['patch']['expected'] for c in final.values())
    persisted=all(c['bytes']==write['patch']['replacement'] for c in after.values())
    returned=all(c['bytes']==write['patch']['expected'] for c in after.values())
    assert restore['phase']=='restored' or recovery['already_returned']
    outcomes.append({'candidate':index,'field_name':candidate['name'],'trial_id':write['id'],
        'effect_criteria_passed':measurement['criteria_passed'],'replacement_observed_after_input':persisted,
        'original_observed_after_input':returned,'recovery':recovery,
        'medians':{k:v['median_difference'] for k,v in measurement['conditions'].items()},
        'stationary':{k:v['stationary']['mean_absolute_difference'] for k,v in measurement['conditions'].items()},
        'measurement_sha256':sha(case/'movement-measurement.json')})
evidence=[];pauses=maximum=writes=refusals=0
for p in (OUT/'data/runtime').glob('*-*/**/stdout.jsonl'):
    receipts=[json.loads(line) for line in p.read_text().splitlines()];assert receipts[-1]['stage']=='complete' and receipts[-1]['helper_absent'] and receipts[-1]['inspection_exit']==0
    report=read(p.with_name('report.json'));shots=report.get('pauses',report.get('snapshots',[])+[w['discarded_snapshot'] for w in report.get('warmups',[])])
    assert all(s['helper_allocations_while_paused']==s['helper_deallocations_while_paused']==0 for s in shots)
    pauses+=len(shots);maximum=max([maximum]+[s['stop_to_resume_us'] for s in shots])
    writes+=int(report.get('patch',{}).get('state')=='written');refusals+=int(report.get('patch',{}).get('state')=='refused')
    evidence.append({'receipt':str(p),'sha256':sha(p),'report_sha256':sha(p.with_name('report.json'))})
storage=Path('C:/Users/artem/AppData/LocalLow/William Chyr Studio/Manifold Garden')
after={str(p.relative_to(storage)):sha(p) for p in storage.rglob('*') if p.is_file()}
changes={name:('added' if name not in frozen['storage_before'] else 'changed') for name,digest in after.items() if frozen['storage_before'].get(name)!=digest}
for name in ['Saves\\SaveSlot1.data','Saves\\SaveSlot2.data','Saves\\Screenshot1.jpg','Saves\\Screenshot2.jpg']:
    assert after[name]==frozen['storage_before'][name]
assert frame_count==36 and pulses==24
summary={'product_revision':frozen['product_revision'],'source_notes':source_notes,'outcomes':outcomes,'measurement_frames':frame_count,
    'movement_pulses':pulses,'requested_input_ms':pulses*200,'setup_screenshots_reviewed':7,
    'helper_operations':len(evidence),'pauses':pauses,'maximum_stop_to_resume_us':maximum,
    'helper_heap_operations_while_paused':0,'confirmed_writes':writes,'refusals':refusals,
    'storage_changes':changes,'existing_saves_1_2_unchanged':True,'evidence':evidence,
    'limitation':'Exploratory same-game, same-scene comparison using a previously interpreted input and adapted model proposal. Rebinding does not prove persistent actor identity. Image differences measure rendered response, not velocity. Failure to persist does not prove display-only state. No held-out generalization claim.'}
(OUT/'summary.json').write_text(json.dumps(summary,indent=2),encoding='utf-8')
print(json.dumps({k:v for k,v in summary.items() if k!='evidence'}))
