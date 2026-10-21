"""Post-run artifact audit, including the retained v1 blocked restart. No game control."""
import ctypes as C,hashlib,json,re,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve();DATA=OUT/'data'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json')
for name,digest in frozen['sources'].items():assert sha(OUT/'source-at-run'/name)==digest,name
missing=[]
for name,digest in frozen['binaries'].items():
    p=OUT/'bin'/name
    if p.exists():assert sha(p)==digest,name
    else:missing.append(name)
failed=(OUT/'failure.json').exists()
if failed:
    failure=read(OUT/'failure.json');security=read(OUT/'security-detection.json')
    assert '225' in failure['error'] and missing==['Reversebolt.exe']
    assert security['ThreatID']==2147738096 and security['ActionSuccess']
    assert any(str(OUT/'bin'/'Reversebolt.exe').lower() in r.lower() for r in security['Resources'])
    assert not (OUT/'result.json').exists()
    # Audit summary only: do not fabricate a completed result.json.
    result={'completed':False,'checks':failure['checks'],'observation_inputs':failure['inputs'],'comparisons':[],'activity_accepted':True}
else:
    result=read(OUT/'result.json');assert result['completed'] and not missing
target=read(OUT/'target.json');live=read(OUT/'discovery.json')['state']['live'];review=live['runtime']['naming']
directory=DATA/'runtime/naming'/review['evidence_id'];receipt=read(directory/'receipt.json');source=read(directory/'play-source.json')
assert receipt['batch']==review['batch'] and receipt['report_sha256']==review['report_sha256']
frames=live['frames'];assert len(frames)==15 and frames==source['frames']
folder=Path(live['latest']['screen']['path']).parent
for f in frames:
    assert sha(folder/f"frame-{f['number']:03}.png")==f['sha256']
    capture=read(folder/f"frame-{f['number']:03}.json")
    assert capture['mode']=='play' and capture['captured_bytes']==0 and not capture['memory_reads'] and not capture['prediction_reads']
assert read(OUT/'play-evidence.json')['data']==source['evidence']
actions=result['observation_inputs'];assert [a['key'] for a in actions]==[68,65,68,65]
if failed:
    assert not receipt['batch']['accepted'] and not receipt['batch']['rejected']
    assert receipt['error'] is None and not receipt['cancelled'] and len(receipt['stages'])==9
    assert review['play_context']=={'source':receipt['activity']['source'],'observation':receipt['activity']['observation']}
    assert source['selected']==review['play_context']['source']
    for i,stage in enumerate(receipt['stages']):
        assert hashlib.sha256(stage['prompt'].encode()).hexdigest()==stage['prompt_sha256']
        assert review['play_context']['observation']['description'] in stage['prompt']
        if i:
            assert stage['schema']['properties']['hypotheses']['maxItems'] in (1,2)
            assert json.loads(stage['response']['message']['content'])=={'hypotheses':[]}
    rpc=read(next(OUT.glob('*-plan_runtime_comparisons.json')));assert rpc['response']['ok']
    plan=rpc['response']['data'];assert not plan['busy'] and not plan['choices'] and plan['next'] is None
    assert plan['report_sha256']==review['report_sha256'] and plan['review_id']==review['evidence_id']
    observed=plan['observed_actions'];assert len(observed['episodes'])==4
    assert observed['frame_metadata_sha256']==review['play_context']['source']['frame_metadata_sha256']
    assert observed['actions']==[e['action'] for e in observed['episodes']]
    assert [a['keys'] for a in observed['actions']]==[[68],[65],[68],[65]]
    for episode in observed['episodes']:
        frame=next(f for f in frames if f['id']==episode['after']);window=frame['input_context']
        pressed=[(i,p) for i,p in enumerate(window['polls']) if p['down']==episode['action']['keys']]
        first,down=pressed[0];last,up=pressed[-1]
        bounds=[up['end_us']-down['begin_us'],window['polls'][last+1]['end_us']-window['polls'][first-1]['begin_us']]
        assert bounds==episode['hold_bounds_us']
        assert episode['action']['hold_ms']==(bounds[0]+(bounds[1]-bounds[0])//2+500)//1000
        assert bounds[0]<=episode['action']['hold_ms']*1000<=bounds[1]
comparisons=[];images=0;workers=[];holds=0;executions=0
for n,item in enumerate(result['comparisons']):
    before=read(OUT/f'plan-{n}.json');plan=before['proposal'];args=before['prepared_args'];chosen=plan['next'];record=read(OUT/f'comparison-{n}.json');t=record['test']
    assert t['id']==args['test_id'] and t['origin']['reference']==chosen['hypothesis_ref']
    assert t['origin']['play_context']==review['play_context'] and t['candidate']['edit_authority'] is False
    assert args['actions']==t['actions']==plan['observed_actions']['actions']
    assert plan['observed_actions']['frame_metadata_sha256']==review['play_context']['source']['frame_metadata_sha256']
    assert args['value']==t['value']==chosen['value']
    assert len(plan['observed_actions']['episodes'])==4
    for episode in plan['observed_actions']['episodes']:
        frame=next(f for f in frames if f['id']==episode['after']);window=frame['input_context']
        pressed=[(i,p) for i,p in enumerate(window['polls']) if p['down']==episode['action']['keys']]
        first,down=pressed[0];last,up=pressed[-1]
        bounds=[up['end_us']-down['begin_us'],window['polls'][last+1]['end_us']-window['polls'][first-1]['begin_us']]
        assert bounds==episode['hold_bounds_us']
        assert episode['action']['hold_ms']==(bounds[0]+(bounds[1]-bounds[0])//2+500)//1000
    for ci,condition in enumerate(t['conditions']):
        for fi,frame in enumerate(condition['frames']):
            assert sha(DATA/'runtime-tests'/frame['path'])==frame['sha256']
            assert sha(DATA/'runtime-tests'/frame['input_path'])==frame['input_sha256']
            assert frame['interval']['clock_id']==t['investigation']
            images+=1
    for e in record['executions']:
        p=DATA/'runtime-tests'/t['id']/f"{e['condition']}-{e['frame']}.action.json"
        outcome=read(p.with_suffix('.outcome.json'))
        assert e['plan_sha256']==sha(p)==outcome['plan_sha256']
        assert e['outcome']['sha256']==sha(p.with_suffix('.outcome.json')) and e['outcome']['receipt']==outcome
        assert e['plan']['action']==(None if e['frame']<2 else t['actions'][e['frame']-2])
        assert outcome['keydown_accepted']==outcome['keyup_accepted']
        holds+=outcome['keydown_accepted'];workers.append(outcome['worker_pid']);executions+=1
    after=read(OUT/f'after-comparison-{n}.json')
    seen=next(c for c in after['choices'] if c['hypothesis_ref']==chosen['hypothesis_ref'])
    expected=t['assessment']['status'] if t['assessment'] else 'stopped'
    assert seen['status']==expected and (after['next'] is None or after['next']['hypothesis_ref']!=chosen['hypothesis_ref'])
    comparisons.append({'id':t['id'],'field':t['candidate']['name'],'question':t['hypothesis'],'phase':t['phase'],'assessment':t['assessment'],'actions':t['actions'],'frames':[len(c['frames']) for c in t['conditions']]})
final=read(OUT/('before-restart.json' if failed else 'final-state.json'))['state']
if failed:
    for key in ('runtime_trials','runtime_tests','changes','freezes'):assert not final[key]
    assert not list((DATA/'runtime-tests').glob('*/**/*.action.json'))
else:assert final['live'] is None if result['activity_accepted'] else True
assert not any(f['active'] for f in final['freezes'])
assert all(t['phase'] in ('restored','refused','expired') for t in final['runtime_trials'])
helpers=[];pauses=[];writes=0
for p in (DATA/'runtime').glob('*-*/**/stdout.jsonl'):
    events=[json.loads(s) for s in p.read_text(encoding='utf-8').splitlines() if s.startswith('{')]
    assert events[-1]['stage']=='complete' and events[-1]['helper_absent'] and events[-1]['inspection_exit']==0
    report=read(p.with_name('report.json'));scope=report.get('scope',report)
    assert scope['pid']==target['pid'] and scope['session']==3
    pauses.extend(report.get('pauses',report.get('snapshots',[])));pauses.extend(w['discarded_snapshot'] for w in report.get('warmups',[]))
    writes+=int(report.get('patch',{}).get('state')=='written')
    helpers.append({'path':str(p.relative_to(OUT)),'sha256':sha(p),'report_sha256':sha(p.with_name('report.json'))})
assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in pauses)
children=read(OUT/'children.json');assert all(c['exit_code'] is not None for c in children)
cleanup=read(OUT/'cleanup.json');assert cleanup['game_pid']==target['pid'] and cleanup['game_exited']
kernel=C.WinDLL('kernel32');kernel.OpenProcess.restype=C.c_void_p;kernel.GetExitCodeProcess.argtypes=[C.c_void_p,C.POINTER(C.c_ulong)];kernel.CloseHandle.argtypes=[C.c_void_p]
for pid in workers+[c['pid'] for c in children]:
    handle=kernel.OpenProcess(0x1000,False,pid)
    if handle:
        code=C.c_ulong();assert kernel.GetExitCodeProcess(handle,C.byref(code)) and code.value!=259,pid;kernel.CloseHandle(handle)
storage=Path(frozen['storage']);after={str(p.relative_to(storage)):sha(p) for p in storage.rglob('*') if p.is_file()}
for name in ['Saves\\SaveSlot1.data','Saves\\SaveSlot2.data','Saves\\Screenshot1.jpg','Saves\\Screenshot2.jpg']:assert after[name]==frozen['storage_before'][name],name
changes={name:('added' if name not in frozen['storage_before'] else 'changed') for name,digest in after.items() if frozen['storage_before'].get(name)!=digest}
changes.update({name:'removed' for name in frozen['storage_before'] if name not in after})
version=(ROOT/'dist/current.txt').read_text(encoding='utf-8-sig').strip()
if failed:
    desktop=read(OUT/'desktop-retained.json')
    assert desktop['pid']==53660 and not desktop['updated'] and desktop['preview']==version=='0.1.0-20261021-225832'
else:
    before=read(OUT/'desktop-before.json');desktop=read(OUT/'desktop-after.json')
    for key in ('history','observer_model','runtime_trials','runtime_tests','changes','freezes'):assert before['state'].get(key)==desktop['state'].get(key)
    assert before['pid']!=desktop['pid']
    for name,digest in frozen['binaries'].items():assert sha(ROOT/'dist'/version/name)==digest
log=(ROOT/'test-results/comparison-planner-tests.log').read_text(encoding='utf-8-sig')
assert 'FAILED' not in log and sum(map(int,re.findall(r'test result: ok\. (\d+) passed',log)))==274
assert read(ROOT/'test-results/smoke-45778740/report.json')['mcp_tools']==53
post_log=ROOT/'test-results/comparison-planner-post-review-tests.log'
post_text=post_log.read_text(encoding='utf-8-sig')
assert 'FAILED' not in post_text and sum(map(int,re.findall(r'test result: ok\. (\d+) passed',post_text)))==158
post_clippy=ROOT/'test-results/comparison-planner-post-review-clippy.log'
assert 'Finished' in post_clippy.read_text(encoding='utf-8-sig') and 'error:' not in post_clippy.read_text(encoding='utf-8-sig')
changed={name:sha(ROOT/name) for name,digest in frozen['sources'].items() if sha(ROOT/name)!=digest}
assert set(changed)=={'crates/reversebolt-core/src/runtime_planning.rs','crates/reversebolt-mcp/src/main.rs'}
summary={'artifact_audit_passed':True,'native_run_completed':result['completed'],'missing_binaries':missing,
    'outcome':'Eight question groups abstained; restart blocked by Windows security (225).' if failed else 'completed',
    'desktop_updated':not failed,'planned_holds_ms':[a['hold_ms'] for a in observed['actions']] if failed else None,
    'artifact_root':str(OUT.relative_to(ROOT)),'frozen_sha256':sha(OUT/'frozen.json'),'native_checks':len(result['checks']),
    'workspace_tests':274,'mcp_tools':53,'observation_images':15,'observation_evaluator_holds':4,'comparison_images':images,'product_holds':holds,'execution_receipts':executions,
    'post_review_core_mcp_tests':158,'post_review_tests_sha256':sha(post_log),'post_review_clippy_sha256':sha(post_clippy),'post_review_changed_sources':changed,
    'manual_measurement_images':0,'setup_screenshots':cleanup['setup_screenshots_reviewed'],'activity_accepted':result['activity_accepted'],
    'model':receipt.get('model'),'model_digest':receipt.get('model_digest'),'server_version':receipt.get('server_version'),'model_error':receipt.get('error'),
    'metadata_stages':len(receipt.get('stages',[])),'accepted_hypotheses':len((review.get('batch') or {}).get('accepted',[])),
    'rejected_hypotheses':len((review.get('batch') or {}).get('rejected',[])),'comparisons':comparisons,
    'helpers':len(helpers),'helper_receipts':helpers,'reported_pauses':len(pauses),'maximum_stop_to_resume_us':max((p['stop_to_resume_us'] for p in pauses),default=0),
    'helper_heap_while_paused':0,'writes_including_restore':writes,'children':children,'input_workers_exited':len(workers),
    'storage_changes':changes,'user_saves_1_2_preserved':True,'preview':version,'desktop_pid':desktop['pid'],'history_entries':len(desktop['state']['history']),
    'limitations':['Previously explored game and scene, not held-out generalization.',
        'Four exploratory setup inputs are evaluator supplied; subsequent comparison actions/timing and field choices come from the product.',
        'The evaluator orchestrates API steps; this is not a self-running autonomous investigation.',
        'Rendered contrast is not verified gameplay semantics, physical speed, persistent actor identity or world rollback.',
        'No independent real-game pause or memory oracle; helper receipts and retained image/input evidence are audited.',
        'Audit authored after the frozen run; no prompt, field or input substitution.',
        'Source copies retained after execution only after matching the pre-launch hashes.',
        'The v1 quarantined executable cannot be rehashed; its pre-launch digest is retained. Protection was not changed.',
        'The v1 restart check did not run. Cleanup failed after engine exit; zero pre-restart edits and exited children resolve its retained recovery flag.',
        'The v1 desktop remains on the preceding build.']}
Path(__file__).with_name('results.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps({k:v for k,v in summary.items() if k not in ('helper_receipts','children')},indent=2))
