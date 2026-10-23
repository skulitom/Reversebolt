"""Audit the frozen two-process native product recovery, without viewing images."""
import hashlib,json,statistics,subprocess,sys,tempfile
from pathlib import Path
import numpy as np
from PIL import Image

ROOT=Path(sys.argv[1]).resolve();DATA=ROOT/'data'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(ROOT/'frozen.json')
for folder, records in [('source',frozen['sources']),('bin',frozen['binaries']),('',frozen['historical']),('storage-before',frozen['storage_before'])]:
    for name,h in records.items():assert sha(ROOT/folder/name)==h,name
assert sha(Path(__file__))==frozen['sources']['research/runtime-recovery/audit-native-product.py']
summaries=[];tests=[];scopes=[];reports=[]
for phase in ['learn','recover']:
    out=ROOT/phase
    assert not (out/'failure.json').exists(),read(out/'failure.json') if (out/'failure.json').exists() else None
    assert not (out/'needs-recovery.json').exists() and not (out/'cleanup-error.json').exists()
    assert read(out/'anode-terminal.json')['exitCode']==0
    assert read(out/'cleanup.json')['engine_exited'] and read(out/'cleanup.json')['mcp_exit_code']==0
    setup=read(out/'setup.json');assert setup['setup_images_viewed']<=3 and setup['menu_actions']<=2
    assert setup['measurement_images_viewed']==0
    final=read(out/'final-state.json');assert final['jobs']['in_flight'] is None and final['jobs']['fence'] is None
    assert not final['jobs']['history']['unresolved'] and not final['state']['freezes']
    live=final['state']['live'];assert live['sample_count']==0 and not live['frames'] and live['runtime']['naming'] is None
    source=live['runtime']['catalog_source'];report=read(out/'current-report.json');reports.append(report)
    assert sha(out/'current-report.json')==sha(Path(source['path']))==source['document_sha256']
    started=read(out/'started.json');assert report['pid']==started['game_pid']
    scopes.append((report['pid'],report['created']))
    comparison=read(out/'comparison.json');test=comparison['test'];tests.append(test)
    plan=read(out/'new-plan.json');assert test==read(DATA/'runtime-tests'/f"{test['id']}.json")
    assert test['id']==plan['test_id'] and test['change_id']==plan['change_id']
    assert test['candidate']['id']==plan['candidate_id'] and test['scope']['pid']==report['pid']
    assert test['phase']=='complete' and test['assessment']['status']=='suppression_observed'
    recipe=test['recovery_recipe'];assert recipe['report_sha256']==source['document_sha256']
    assert recipe['source']==test['candidate']['locator'] and recipe['root_catalog_sha256']==report['root_catalog_sha256']
    trial=next(t for t in final['state']['runtime_trials'] if t['id']==test['change_id'])
    assert trial==read(DATA/'runtime-trials'/f"{trial['id']}.json") and trial['phase']=='restored'
    assert trial['candidate_id']==test['candidate']['id'] and trial['patch']['path']==test['candidate']['locator']
    def evidence(e):
        assert sha(Path(e['path']))==e['report_sha256'] and read(Path(e['path']))==e['report']
    evidence(test['preparation']);evidence(trial['baseline'])
    for key in ['write','restoration']:
        a=trial[key];evidence(a['evidence']);assert a['error'] is None and a['outcome']=='written'
        assert a['evidence']['report']['patch']['guard']=='ready'
        assert a['evidence']['report']['patch']['observed_after']==a['patch']['replacement']
    assert trial['restoration']['patch']['replacement']==trial['patch']['expected']
    assert trial['restoration']['patch']['expected']==trial['patch']['replacement']
    assert test['conditions'][0]['frames'][-1]['completed_unix_ms']<=trial['write']['started_ms']
    assert test['conditions'][1]['runtime_checked_unix_ms']<=trial['restoration']['started_ms']
    raw=[];holds=0
    for condition,c in enumerate(test['conditions']):
        assert len(c['frames'])==6
        if condition:
            evidence(c['runtime_evidence'])
            assert c['runtime_evidence']['report']['checks'][0]['bytes']==trial['patch']['replacement' if condition==1 else 'expected']
            assert c['frames'][0]['started_unix_ms']>=trial['write' if condition==1 else 'restoration']['completed_ms']
            assert c['runtime_checked_unix_ms']>=c['frames'][-1]['completed_unix_ms']
        previous=None;differences=[]
        for i,frame in enumerate(c['frames']):
            image=DATA/'runtime-tests'/frame['path'];input_path=DATA/'runtime-tests'/frame['input_path']
            assert sha(image)==frame['sha256'] and sha(input_path)==frame['input_sha256']
            witness=read(input_path);assert witness['omitted_polls']==0
            assert all(p['focus']=='selected_foreground' for p in witness['polls'])
            if i:assert frame['input']['matches']
            executed=comparison['executions'][condition*6+i]
            prefix=DATA/'runtime-tests'/test['id']/f'{condition}-{i}'
            assert sha(Path(str(prefix)+'.action.json'))==executed['plan_sha256']
            assert read(Path(str(prefix)+'.action.json'))==executed['plan']
            assert sha(Path(str(prefix)+'.action.outcome.json'))==executed['outcome']['sha256']
            assert read(Path(str(prefix)+'.action.outcome.json'))==executed['outcome']['receipt']
            action=None if i<2 else test['actions'][i-2];assert executed['plan']['action']==action
            receipt=executed['outcome']['receipt'];assert receipt['error'] is None
            assert receipt['plan_sha256']==executed['plan_sha256']
            assert receipt['keydown_accepted']==receipt['keyup_accepted']==(len(action['keys']) if action else 0)
            if action:
                assert any(p['down']==action['keys'] for p in witness['polls'])
                assert all(not p['down'] or p['down']==action['keys'] for p in witness['polls']);holds+=1
            pixels=np.asarray(Image.open(image).convert('RGB'),dtype=np.int32)
            gray=((pixels[:,:,0]*299+pixels[:,:,1]*587+pixels[:,:,2]*114+500)//1000).astype(np.float64)
            if i>=2:differences.append(float(np.mean(np.abs(gray-previous))))
            previous=gray
        raw.append(statistics.median(differences))
    assert holds==12 and len(comparison['executions'])==18
    medians=[statistics.median(f['difference'] for f in c['frames'][2:]) for c in test['conditions']]
    assert medians==test['assessment']['medians']
    for i in range(2,6):
        durations=[c['frames'][i]['input']['held_us'][0] for c in test['conditions']]
        assert max(durations)-min(durations)<=max(50000,max(durations)/4)
    summaries.append({'phase':phase,'test_id':test['id'],'change_id':trial['id'],'candidate_id':test['candidate']['id'],
                      'recipe_report_sha256':recipe['report_sha256'],'product_medians':medians,'independent_full_frame_gray_medians':raw,
                      'input_holds':holds,'automatic_measurement_images':18,'restored':True,'setup':setup})
assert scopes[0][0]!=scopes[1][0] and scopes[0][1]!=scopes[1][1]
assert tests[0]['candidate']['id']!=tests[1]['candidate']['id']
assert tests[0]['actions']==tests[1]['actions'] and tests[0]['value']==tests[1]['value']
assert tests[0]['hypothesis']==tests[1]['hypothesis']
initial=read(ROOT/'recover/initial.json');prior=read(ROOT/'learn/final-state.json')
assert initial['jobs']['history']['boot']>prior['jobs']['history']['boot']
discovered=read(ROOT/'recover/discovery.json')['state']['live']['runtime']
hint=discovered['recovered_controls'][0];assert len(discovered['recovered_controls'])==1
assert hint['source_test_id']==tests[0]['id'] and hint['candidate_id']==tests[1]['candidate']['id'] and hint['needs_revalidation']
recovery=read(ROOT/'recover/recovery.json')
assert recovery['shown'] and not recovery['edit_authority'] and recovery['recovery']['matching_routes']==1
assert recovery['source_test_id']==tests[0]['id'] and recovery['recovery']['candidate']['id']==hint['candidate_id']
assert hint['source_test_sha256']==recovery['source_test_sha256'] and hint['recipe_sha256']==recovery['recovery']['recipe_sha256']
with tempfile.TemporaryDirectory(prefix='reversebolt-native-recovery-audit-') as temp:
    output=Path(temp)/'proposal.json'
    subprocess.run([str(ROOT/'bin/runtime_recovery.exe'),'--propose-declared-route',str(ROOT/'learn/current-report.json'),
                    tests[0]['candidate']['id'],str(ROOT/'recover/current-report.json'),str(output)],check=True,capture_output=True,timeout=90)
    proposal=read(output);assert proposal['status']=='candidate_for_revalidation'
    assert proposal['candidate']==recovery['recovery']['candidate']
helpers=[];pauses=[]
for log in (DATA/'runtime').glob('*-*/**/stdout.jsonl'):
    events=[json.loads(s) for s in log.read_text().splitlines()]
    assert events[0]['helper_sha256']==frozen['binaries']['reversebolt_runtime.dll']
    assert events[-1]['stage']=='complete' and events[-1]['helper_absent'] and events[-1]['inspection_exit']==0 and events[-1]['unload_exit']==1
    report=read(log.with_name('report.json'));assert (report['pid'],report['created']) in scopes
    pauses.extend(report.get('snapshots',report.get('pauses',[])));pauses.extend(w['discarded_snapshot'] for w in report.get('warmups',[]))
    helpers.append({'path':str(log),'sha256':sha(log),'elapsed_ms':events[-1]['elapsed_ms']})
assert len(helpers)==14
assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in pauses)
cleanup=read(ROOT/'cleanup.json');assert cleanup['owned_processes_exited'] and cleanup['preserved_editor']['pid']==frozen['main_editor_pid']
after=read(ROOT/'storage-after.json')['hashes'];before=frozen['storage_before']
result={'artifact_audit':'passed','preview':frozen['preview'],'frozen_sha256':sha(ROOT/'frozen.json'),
        'phases':summaries,'helpers':helpers,'pauses':len(pauses),'product_recipe_retained':True,'automatic_recovery':True,
        'mcp_recovery':True,'new_native_comparisons':2,'research_backfill':False,'model_calls':0,'manually_viewed_measurement_images':0,
        'save_bytes_unchanged':{n:n in before and before[n]==after.get(n) for n in ['viewfinder_0.sav','viewfinder_0.bak']},
        'storage_changes':[n for n in sorted(set(before)|set(after)) if before.get(n)!=after.get(n)],'cleanup':cleanup,
        'limitation':'Known development game and action-specific rendered response; no exact semantics, persistent actor identity, complete rollback or independent-game generalization.'}
(ROOT/'audit.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print(json.dumps({k:result[k] for k in ['artifact_audit','new_native_comparisons','automatic_recovery','save_bytes_unchanged']}))
