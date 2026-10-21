"""Post-run evidence audit; never captures, controls, infers or writes game state."""
import hashlib,json,re,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve()
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def ordered(value,keys):return {k:value[k] for k in keys.split()}
def input_digest(window):
    # Exact serde struct order in inputs::{Window,Poll}, not JSON map order.
    value=ordered(window,'clock_id generation requested_begin_us requested_end_us monitor_started_us retained_from_us omitted_polls largest_unsampled_us polls limitation')
    value['polls']=[ordered(p,'begin_us end_us focus down') for p in value['polls']]
    return hashlib.sha256(json.dumps(value,ensure_ascii=False,separators=(',',':')).encode()).hexdigest()
frozen=read(OUT/'frozen.json')
for name,digest in frozen['sources'].items():assert sha(ROOT/name)==digest,name
for name,digest in frozen['binaries'].items():assert sha(OUT/'bin'/name)==digest,name
result=read(OUT/'result.json');assert result['completed']
target=read(OUT/'target.json');live=read(OUT/'discovery.json')['state']['live'];review=live['runtime']['naming']
directory=OUT/'data/runtime/naming'/review['evidence_id'];receipt=read(directory/'receipt.json');source=read(directory/'play-source.json')
assert receipt['batch']==review['batch'] and receipt['report_sha256']==review['report_sha256']
assert receipt['model']==review['model'] and len(receipt['model_digest'])==64
assert not receipt['cancelled'] and receipt['error'] is None
frames=live['frames'];assert len(frames)==live['sample_count']==15 and frames==source['frames']
folder=Path(live['latest']['screen']['path']).parent
for frame in frames:
    assert sha(folder/f"frame-{frame['number']:03}.png")==frame['sha256']
    capture=read(folder/f"frame-{frame['number']:03}.json")
    assert capture['mode']=='play' and capture['captured_bytes']==0
    assert not capture['memory_reads'] and not capture['prediction_reads']
    assert capture['recognition_ms']==0 and not capture['screen']['words'] and not capture['screen']['text']
    assert frame['capture_interval']['clock_id']==live['investigation_id']
assert live['retained_bytes']==live['coverage']['attempted_bytes']==0
recomputed=read(OUT/'play-evidence.json')['data']
assert recomputed==source['evidence']
selected=source.get('selected');context=review.get('play_context');activity=receipt.get('activity')
if selected:
    assert selected==activity['source'] and selected['frame_metadata_sha256']==recomputed['frame_metadata_sha256']
    assert selected['investigation_id']==live['investigation_id'] and selected['batch']==live['coverage']['batch']
    for index,pair in enumerate(selected['pairs']):
        before=next(f for f in frames if f['id']==pair['before']['id']);after=next(f for f in frames if f['id']==pair['after']['id'])
        assert after['number']==before['number']+1
        assert pair['input_sha256']==input_digest(after['input_context'])
        for offset,frame in enumerate([before,after]):
            assert sha(directory/f'play-{2*index+offset}.png')==frame['sha256']
            citation=pair['before' if offset==0 else 'after']
            assert all(citation[k]==frame[k] for k in ('id','sha256','number','width','height'))
            assert citation['interval']==frame['capture_interval']
if context:
    assert context=={'source':selected,'observation':activity['observation']}
    assert context['observation']['consistent_visible_change'] and activity['error'] is None
    assert len(receipt['stages'])==2
    assert all(context['observation']['description'] in stage['prompt'] for stage in receipt['stages'])
stages=receipt['stages']+([activity['stage']] if activity and activity.get('stage') else [])
for stage in stages:assert hashlib.sha256(stage['prompt'].encode()).hexdigest()==stage['prompt_sha256']
actions=[read(p) for p in sorted(OUT.glob('input-*.json'))]
assert [a['key'] for a in actions]==[68,65,68,65]
assert all(a['planned_ms']==200 and a['end_ns']>a['begin_ns'] for a in actions)
final=read(OUT/'final-state.json')['state']
assert not final['changes'] and not final['runtime_trials'] and not final['freezes']
tests=[read(p) for p in (OUT/'data/runtime-tests').glob('*.json')]
assert len(tests)==int(result['prepared_test'] is not None)
for test in tests:
    assert test['phase']=='stopped' and test['origin']==result['prepared_test']['origin']
    assert test['origin']['play_context']==context and test['origin']['hypothesis'] in review['batch']['accepted']
    assert all(not condition['frames'] for condition in test['conditions'])
    assert final['live'] is None
helpers=[];pauses=[]
for p in (OUT/'data/runtime').glob('*-*/**/stdout.jsonl'):
    entries=[json.loads(s) for s in p.read_text(encoding='utf-8').splitlines() if s.startswith('{')]
    assert entries[-1]['stage']=='complete' and entries[-1]['helper_absent'] and entries[-1]['inspection_exit']==0
    report=read(p.with_name('report.json'));scope=report.get('scope',report)
    assert scope['pid']==target['pid'] and scope['session']==3 and 'patch' not in report
    pauses.extend(report.get('pauses',report.get('snapshots',[])))
    pauses.extend(w['discarded_snapshot'] for w in report.get('warmups',[]))
    helpers.append({'path':str(p.relative_to(OUT)),'sha256':sha(p),'report_sha256':sha(p.with_name('report.json'))})
assert helpers and all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in pauses)
children=read(OUT/'children.json');assert all(c['exit_code'] is not None for c in children)
cleanup=read(OUT/'cleanup.json');assert cleanup['game_pid']==target['pid'] and cleanup['game_exited']
storage=Path(frozen['storage']);after={str(p.relative_to(storage)):sha(p) for p in storage.rglob('*') if p.is_file()}
for name in ['Saves\\SaveSlot1.data','Saves\\SaveSlot2.data','Saves\\Screenshot1.jpg','Saves\\Screenshot2.jpg']:
    assert after[name]==frozen['storage_before'][name],name
changes={name:('added' if name not in frozen['storage_before'] else 'changed') for name,digest in after.items() if frozen['storage_before'].get(name)!=digest}
changes.update({name:'removed' for name in frozen['storage_before'] if name not in after})
before=read(OUT/'desktop-before.json');desktop=read(OUT/'desktop-after.json')
for key in ('history','observer_model','runtime_trials','runtime_tests','changes','freezes'):assert before['state'].get(key)==desktop['state'].get(key)
assert before['pid']!=desktop['pid']
version=(ROOT/'dist/current.txt').read_text(encoding='utf-8-sig').strip()
for name,digest in frozen['binaries'].items():assert sha(ROOT/'dist'/version/name)==digest
test_log=(ROOT/'test-results/activity-context-tests-final.log').read_text(encoding='utf-8-sig')
assert 'FAILED' not in test_log and sum(map(int,re.findall(r'test result: ok\. (\d+) passed',test_log)))==271
smoke=read(ROOT/'test-results/smoke-0ee9c505/report.json');assert smoke['mcp_tools']==52 and smoke['shared_session']=='passed'
summary={'audit_passed':True,'artifact_root':str(OUT.relative_to(ROOT)),'frozen_sha256':sha(OUT/'frozen.json'),
    'parent_revision':frozen['parent'],'native_checks':len(result['checks']),'workspace_tests':271,'mcp_tools':52,
    'frames':15,'evaluator_holds':len(actions),'manual_measurement_images':0,'setup_screenshots':cleanup['setup_screenshots_reviewed'],
    'activity_accepted':bool(context),'observation':activity.get('observation') if activity else None,
    'activity_error':activity.get('error') if activity else None,
    'selected_key':selected['key'] if selected else None,'model':receipt['model'],
    'model_digest':receipt['model_digest'],'server_version':receipt['server_version'],
    'stage_elapsed_ms':{'activity':activity['stage']['elapsed_ms'] if activity and activity.get('stage') else None,'metadata':[s['elapsed_ms'] for s in receipt['stages']]},
    'accepted_hypotheses':len(review['batch']['accepted']),'rejected_hypotheses':len(review['batch']['rejected']),
    'hypothesis_names':[h['proposal']['name'] for h in review['batch']['accepted']],
    'prepared_field':result['prepared_test']['candidate']['name'] if result['prepared_test'] else None,
    'prepared_test':result['prepared_test']['id'] if result['prepared_test'] else None,
    'helpers':len(helpers),'helper_receipts':helpers,'reported_pauses':len(pauses),
    'maximum_stop_to_resume_us':max((p['stop_to_resume_us'] for p in pauses),default=0),'helper_heap_operations_while_paused':0,
    'writes':0,'children':children,'storage_changes':changes,'user_saves_1_2_preserved':True,
    'preview':version,'desktop_pid':desktop['pid'],'history_preserved':len(desktop['state']['history']),
    'limitations':['Exploratory previously inspected game and save; no independent or held-out generalization.',
        'Known evaluator keys and duration, not learned input semantics or an autonomous action policy.',
        'Model descriptions and metadata matches are provisional; no physical, semantic or causal field verification.',
        'Preparing and stopping a comparison verifies provenance only; no comparison images or gameplay edits.',
        'Pauses and helper heap counts are helper-reported, without an independent game pause oracle.',
        'Audit authored after the frozen run; product, evaluator and selection thresholds unchanged.']}
Path(__file__).with_name('results.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps({k:v for k,v in summary.items() if k not in ('helper_receipts','children')},indent=2))
