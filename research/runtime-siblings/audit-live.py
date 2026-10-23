"""Post-run native evidence audit; no target, input, screenshots or model calls."""
import hashlib
import json
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parents[2]
OUT=Path(sys.argv[1]).resolve() if len(sys.argv)>1 else ROOT/'test-results/runtime-siblings-sable-v1'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json')
PRIOR=Path(frozen.get('prior',str(OUT)))
if frozen.get('partial_review'):
    assert sha(PRIOR/'frozen.json')==frozen['prior_frozen_sha256']
    assert sha(Path(frozen['measurement_source']))==frozen['measurement_source_sha256']
    for name,entry in frozen['inputs'].items():assert sha(OUT/f'{name}.json')==entry['sha256']
for name,digest in frozen['sources'].items():assert sha(OUT/'source'/name)==digest,name
for name,digest in frozen['binaries'].items():assert sha(Path(frozen['bin'])/name)==digest,name
for name,digest in frozen['analysis'].items():assert sha(OUT/'analysis-bin'/name)==digest,name
for name,digest in frozen['storage_before'].items():assert sha(PRIOR/'storage-before'/name)==digest,name
assert sha(PRIOR/'appmanifest-before.acf')==frozen['appmanifest_sha256']
setup=read(PRIOR/'setup.json');job=read(OUT/'anode-job.json')
assert setup['gameplay_observed'] and setup['setup_images_reviewed']<=4
assert job['finished'] and job['exitCode']==0 and job['state']=='completed'
assert read(OUT/'child.json')['exit_code']==0
assert not (OUT/'failure.json').exists() and not (OUT/'needs-recovery.json').exists()
discovery=read(PRIOR/'discovery.json');live=discovery['state']['live']
plan=read(OUT/'plan/plan.json');result=read(OUT/'result.json')
assert result['workflow_finished'] and result['gameplay_field_writes']==0
assert result['manually_reviewed_measurement_images']==0
assert live['sample_count']==15 and [i['key'] for i in result['discovery_inputs']]==[68,65,68,65]
assert all(i['planned_ms']==200 for i in result['discovery_inputs'])
assert plan['policy']=='rooted_role_sibling_sampling_v2'
assert plan['scope']['pid']==setup['game_pid'] and plan['scope']['session']==3
assert len(plan['candidates'])<=128 and len(result['batches'])==min(len(plan['batches']),8)
assert sum(r['selected_fields'] for r in plan['roles'])==len(plan['candidates'])
assert sum(r['available_fields'] for r in plan['roles'])-len(plan['candidates'])==plan['omitted_fields']
assert [p for b in plan['batches'] for p in b]==[c['locator'] for c in plan['candidates']]
for i,paths in enumerate(plan['batches']):
    file=OUT/'plan'/f'paths-{i}.json'
    assert read(file)==paths and len(paths)<=32 and file.stat().st_size<=60*1024
    assert len(file.read_text().encode('utf-16-le'))//2<=30*1024
analysis_root=OUT
batches=result['batches']
if (OUT/'repair').exists():
    analysis_root=OUT/'repair';repair=read(analysis_root/'frozen.json')
    assert sha(analysis_root/'reversebolt-runtime-response.exe')==repair['binary_sha256']
    for name,digest in repair['sources'].items():assert sha(analysis_root/'source'/name)==digest
    for name,digest in repair['inputs'].items():assert sha(OUT/name)==digest
    assert repair['original_analysis_rejections']==[b['error'] for b in batches]
    assert all(b['exit_code']!=0 for b in batches)
    batches=read(analysis_root/'result.json')['batches']
summary=[];holds=[];read_count=0;report_hashes=[]
for batch in batches:
    b=batch['batch'];observations=read(OUT/f'observations-{b}.json');assert len(observations)==14
    for i,o in enumerate(observations):
        assert read(OUT/f'observation-{b}-{i}.json')==o
        captured=read(OUT/f'input-{b}-{i}.json')
        assert not captured['poll_errors']
        assert {k:captured[k] for k in ['scope','frequency_hz','expected_keys','polls']}==o['input']
        expected={2:68,5:65,8:68,11:65}.get(i)
        assert o['input']['expected_keys']==([expected] if expected else [])
        if expected:
            h=captured['hold'];assert h['key']==expected and h['planned_ms']==3000 and 'interrupted' not in h
            holds.append((h['end_ticks']-h['begin_ticks'])*1000/o['input']['frequency_hz'])
        else:assert captured['hold'] is None
        loader=read(OUT/f'loader-{b}-{i}.json');directory=Path(loader['directory'])
        assert read(directory/'report.json')==o['report']
        assert read(directory/'paths.json')==plan['batches'][b]
        assert o['report']['operation']=='refresh_runtime_paths' and o['report']['scope']==plan['scope']
        assert o['report']['edit_authority'] is False
        report_hashes.append(sha(directory/'report.json'));read_count+=1
    if batch['exit_code']==0:
        analysis=read(analysis_root/f'analysis-{b}.json');assert analysis==batch['analysis']
        fields=analysis['fields'];assert len(fields)==len(plan['batches'][b])
        assert [f['path_id'] for f in fields]==[c['path_id'] for c in observations[0]['report']['checks']]
        for j,f in enumerate(fields):
            checks=[o['report']['checks'][j] for o in observations]
            complete=all(c['status']=='read' for c in checks)
            assert f['all_read']==complete
            if complete and f['locations_stable']:
                quiet=state=step=0
                for action in [2,5,8,11]:
                    pre,during,post=[checks[k]['bytes'] for k in [action-1,action,action+1]]
                    if checks[action-2]['bytes']==pre and checks[action+2]['bytes']==post:
                        quiet+=1;state+=pre==post and during!=pre;step+=pre!=post and during==post
                assert [f['quiet_stable_pulses'],f['state_associated_pulses'],f['step_associated_pulses']]==[quiet,state,step]
                assert f['repeated_association']==(state==4 or step==4)
        summary.append({'batch':b,'valid':True,'fields':len(fields),
            'all_read':sum(f['all_read'] for f in fields),'locations_stable':sum(f['locations_stable'] for f in fields),
            'quiet_stable_all_four':sum(f['quiet_stable_pulses']==4 for f in fields),
            'state_pulses':sum(f['state_associated_pulses'] for f in fields),
            'step_pulses':sum(f['step_associated_pulses'] for f in fields),
            'associations':[f for f in fields if f['repeated_association']]})
    else:
        assert batch['analysis'] is None and batch['error']
        summary.append({'batch':b,'valid':False,'error':batch['error']})
helpers=[];pauses=[]
DATA=Path(frozen['data'])
for log in (DATA/'runtime').glob('*-*/**/stdout.jsonl'):
    events=[json.loads(line) for line in log.read_text().splitlines()]
    first,last=events[0],events[-1]
    assert first['pid']==plan['scope']['pid'] and first['created']==plan['scope']['created'] and first['session']==3
    assert first['helper_sha256']==frozen['binaries']['reversebolt_runtime.dll']
    assert last['stage']=='complete' and last['helper_absent'] and last['inspection_exit']==0
    report=read(log.with_name('report.json'))
    pauses.extend(report.get('snapshots',report.get('pauses',[])))
    pauses.extend(w['discarded_snapshot'] for w in report.get('warmups',[]))
    helpers.append(sha(log))
assert len(helpers)==read_count+1
assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in pauses)
final=read(OUT/'final-state.json')
assert final['jobs']['fence'] is None and final['jobs']['in_flight'] is None
assert not final['state']['runtime_trials'] and not final['state']['runtime_tests']
assert not any(f['active'] for f in final['state']['freezes'])
review=live['runtime']['naming']
receipt=read(DATA/'runtime/naming'/review['evidence_id']/'receipt.json')
stages=receipt['stages']
if frozen.get('partial_review'):
    assert receipt['incomplete_stage']['phase']=='request'
    selection=read(OUT/'plan/seed-selection.json')
    assert selection['requested_paths']==list(dict.fromkeys(r['path_id'] for r in receipt['selection']['rows']))
    assert selection['seeds']==plan['seed_ids']
    assert selection['observed_roles']-len(plan['roles'])==selection['omitted_roles']==result['omitted_roles']
else:assert not receipt.get('incomplete_stage')
storage=Path(frozen['storage'])
after={p.relative_to(storage).as_posix():sha(p) for p in storage.rglob('*') if p.is_file()}
changed=[name for name in sorted(set(frozen['storage_before'])|set(after)) if frozen['storage_before'].get(name)!=after.get(name)]
audit={'passed':True,'source_commit':frozen['source_commit'],'product':frozen['bin'],
    'partial_review':frozen.get('partial_review',False),'original_analysis_rejections':sum(b['exit_code']!=0 for b in result['batches']),
    'offline_reanalysis':analysis_root!=OUT,
    'roles':len(plan['roles']),'selected_fields':len(plan['candidates']),'omitted_fields':plan['omitted_fields'],
    'omitted_batches':result['omitted_batches'],'batches':summary,'native_helpers':len(helpers),
    'pauses':len(pauses),'max_stop_to_resume_us':max(p['stop_to_resume_us'] for p in pauses),
    'measurement_input_holds':len(holds),'hold_duration_ms_range':[min(holds),max(holds)] if holds else None,
    'discovery_inputs':len(result['discovery_inputs']),'automatic_images':15,'manually_reviewed_setup_images':setup['setup_images_reviewed'],
    'manually_reviewed_measurement_images':0,'gameplay_field_writes':0,'metadata_requests':len(stages),
    'metadata_generation_ms':sum(s['response']['total_duration'] for s in stages)/1e6,
    'changed_save_folder_files':changed,'report_hashes':report_hashes,'progress_percent':36}
(OUT/'audit.json').write_text(json.dumps(audit,indent=2)+'\n')
print(json.dumps({k:v for k,v in audit.items() if k not in ['batches','report_hashes']}))
print(json.dumps({'batches':summary}))
