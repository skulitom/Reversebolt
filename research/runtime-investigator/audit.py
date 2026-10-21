"""Post-run artifact audit. Never launches, controls, captures or edits a game.

Written after the protocol was frozen; checks evidence, not a new effect threshold.
"""
import hashlib,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve()

def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json')
for name,digest in frozen['sources'].items():assert sha(ROOT/name)==digest,name
for name,digest in frozen['product_binaries'].items():assert sha(OUT/'bin'/name)==digest,name
target=read(OUT/'target.json');final=read(OUT/'final-state.json')['state']
assert not any(t['active'] for t in final['runtime_tests'])
assert not any(f['active'] for f in final['freezes'])
children=read(OUT/'children.json');assert all(c['exit_code'] is not None for c in children)
cleanup=read(OUT/'cleanup.json');assert cleanup['game_pid']==target['pid'] and cleanup['game_exited']
tests=[read(p) for p in (OUT/'data/runtime-tests').glob('*.json')]
trials=[read(p) for p in (OUT/'data/runtime-trials').glob('*.json')]
assert all(t['phase'] in ('complete','stopped') for t in tests)
assert all(t['phase'] in ('restored','refused') for t in trials),'Unresolved edit'
failure=read(OUT/'failure.json') if (OUT/'failure.json').exists() else None
selection=read(OUT/'selection.json') if (OUT/'selection.json').exists() else None
discovery=read(OUT/'discovery.json')['state']['live']['runtime'] if (OUT/'discovery.json').exists() else None
review=discovery['naming'] if discovery else None
if review:
    receipt=read(OUT/'data/runtime/naming'/review['evidence_id']/'receipt.json')
    assert receipt['batch']==review['batch'] and receipt['report_sha256']==review['report_sha256']
    assert receipt['model']==review['model'] and receipt['error'] is None and not receipt['cancelled']
if selection:
    assert selection['hypothesis'] in review['batch']['accepted']
    assert selection['report_sha256']==review['report_sha256']
images=0;inputs=[];outcomes=[]
for t in tests:
    origin=t['origin'];assert origin['hypothesis']==selection['hypothesis']
    assert origin['report_sha256']==selection['report_sha256']
    assert t['candidate']['id']==origin['reference']['selected_path_id']
    assert t['hypothesis']==origin['hypothesis']['proposal']['question']
    assert t['scope']['pid']==target['pid'] and t['scope']['session']==3
    assert t['candidate']['edit_authority'] is False
    for condition_index,condition in enumerate(t['conditions']):
        for index,frame in enumerate(condition['frames']):
            for prefix in ('','input_'):
                path=OUT/'data/runtime-tests'/frame[prefix+'path'];assert sha(path)==frame[prefix+'sha256']
            assert frame['path']==f"{t['id']}/{condition_index}-{index}.png"
            if index:
                window=read(OUT/'data/runtime-tests'/frame['input_path'])
                assert window['clock_id']==t['investigation']
                assert window['generation']==frame['input']['generation']
                inputs.append({'test_id':t['id'],'condition':condition_index,'index':index,**frame['input']})
            images+=1
    outcomes.append({'test_id':t['id'],'selected_path':t['candidate']['id'],'field':t['candidate']['name'],
        'phase':t['phase'],'assessment':t['assessment'],'frames':[len(c['frames']) for c in t['conditions']]})
actions=[read(p) for p in OUT.glob('input-*.json')]
assert images<=36 and len(actions)<=24
assert all(a['key'] in (65,68) and a['planned_ms']==200 and a['finished_ns']>a['started_ns'] for a in actions)
helpers=[];pauses=[];writes=0
for path in (OUT/'data/runtime').glob('*-*/**/stdout.jsonl'):
    entries=[json.loads(s) for s in path.read_text().splitlines() if s.startswith('{')]
    assert entries[-1]['stage']=='complete' and entries[-1]['helper_absent'] and entries[-1]['inspection_exit']==0
    report=read(path.with_name('report.json'));scope=report.get('scope',report)
    assert scope['pid']==target['pid'] and scope['session']==3
    pauses.extend(report.get('pauses',report.get('snapshots',[])))
    pauses.extend(w['discarded_snapshot'] for w in report.get('warmups',[]))
    writes+=int(report.get('patch',{}).get('state')=='written')
    helpers.append({'path':str(path.relative_to(OUT)),'sha256':sha(path),'report_sha256':sha(path.with_name('report.json'))})
assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in pauses)
storage=Path(frozen['storage']);after={str(p.relative_to(storage)):sha(p) for p in storage.rglob('*') if p.is_file()}
for name in ['Saves\\SaveSlot1.data','Saves\\SaveSlot2.data','Saves\\Screenshot1.jpg','Saves\\Screenshot2.jpg']:
    assert after[name]==frozen['storage_before'][name],name
changes={name:('added' if name not in frozen['storage_before'] else 'changed') for name,digest in after.items() if frozen['storage_before'].get(name)!=digest}
changes.update({name:'removed' for name in frozen['storage_before'] if name not in after})
summary={'audit_passed':True,'artifact_root':str(OUT.relative_to(ROOT)),'frozen_sha256':sha(OUT/'frozen.json'),
    'product_revision':frozen['revision'],'preview':frozen['preview'],'failure':failure,
    'accepted_hypotheses':len(review['batch']['accepted']) if review else None,
    'rejected_hypotheses':len(review['batch']['rejected']) if review else None,
    'outcomes':outcomes,'images':images,'input_holds':len(actions),'input_witnesses_match':all(i['matches'] for i in inputs) if inputs else None,
    'manual_measurement_image_review':0,'setup_screenshots_reviewed':cleanup['setup_screenshots_reviewed'],
    'helpers':len(helpers),'helper_receipts':helpers,'pauses':len(pauses),
    'maximum_stop_to_resume_us':max((p['stop_to_resume_us'] for p in pauses),default=0),
    'helper_heap_operations_while_paused':0,'confirmed_writes_including_restore':writes,
    'trials':[{'id':t['id'],'phase':t['phase'],'restoration':t.get('restoration')} for t in trials],
    'existing_saves_1_2_unchanged':True,'storage_changes':changes,'owned_children_exited':children,
    'limitations':['Exploratory test on an already inspected game and scene; not held-out generalization.',
        'Fresh model hypothesis selection; movement inputs come from the earlier evaluator, not learned action meaning.',
        'Image response is not physical speed or verified gameplay semantics; byte restoration is not world rollback.',
        'Audit written after protocol freeze; original source and binary hashes checked without changing effect criteria.']}
(OUT/'audit.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k not in ('helper_receipts','trials','failure')},indent=2))
