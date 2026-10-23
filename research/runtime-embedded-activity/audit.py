"""Recompute the frozen analysis from retained reports; no target access."""
import hashlib,json,subprocess,sys,collections
from pathlib import Path
OUT=Path(sys.argv[1]).resolve();read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json');result=read(OUT/'result.json');plan=read(OUT/'plan/plan.json')
for name,digest in frozen['sources'].items():assert sha(OUT/'source'/name)==digest
for name,digest in frozen['binaries'].items():assert sha(OUT/'bin'/name)==digest
assert sha(OUT/'plan/plan.json')==read(OUT/'frozen-plan.json')['sha256']
assert plan['policy']=='rooted_role_sibling_sampling_v3'
receipt=read(OUT/'selection/receipt.json')
assert receipt['model_digest']==frozen['model_digest'] and not receipt['error'] and receipt['batch'] is None
assert len(receipt['stages'])<=24
assert all(len(s['prompt'].encode())<=12000 and sha_prompt==s['prompt_sha256'] for s in receipt['stages'] for sha_prompt in [hashlib.sha256(s['prompt'].encode()).hexdigest()])
pauses=[]
for call in result['calls']:
    file=OUT/(call['label']+'.json');assert sha(file)==call['report_sha256']
    r=read(file);events=[json.loads(line) for line in (OUT/(call['label']+'.stdout')).read_text().splitlines()]
    assert events[-1]['helper_absent'] and events[-1]['inspection_exit']==0
    assert events[0]['created']==plan['scope']['created'] and events[0]['pid']==plan['scope']['pid']
    samples=r.get('pauses',r.get('snapshots',[])+[w['discarded_snapshot'] for w in r.get('warmups',[])])
    assert all(s['helper_allocations_while_paused']==s['helper_deallocations_while_paused']==0 for s in samples)
    pauses.extend(samples)
output=OUT/'audit-reanalysis';output.mkdir(exist_ok=True)
positive=[];unresolved=[];measured=[]
for batch in result['batches']:
    i=batch['batch'];path=output/f'analysis-{i}.json'
    # New output names only; a second audit reuses the verified stored replay.
    if not path.exists():
        p=subprocess.run([str(OUT/'bin/reversebolt-runtime-response.exe'),str(OUT/'plan/plan.json'),str(i),str(OUT/f'observations-{i}.json'),str(path)],capture_output=True,text=True,creationflags=subprocess.CREATE_NO_WINDOW)
        (output/f'analysis-{i}.stdout').write_text(p.stdout);(output/f'analysis-{i}.stderr').write_text(p.stderr)
        assert p.returncode==batch['exit']
    if batch['exit']:
        unresolved.append({'batch':i,'error':(OUT/f'analysis-{i}.stderr').read_text()});continue
    analysis=read(OUT/f'analysis-{i}.json');assert read(path)==analysis==batch['analysis']
    observations=read(OUT/f'observations-{i}.json')
    for j,field in enumerate(analysis['fields']):
        record=dict(field,batch=i,field_address=observations[0]['report']['checks'][j]['field_address'])
        measured.append(record)
        if field['repeated_association']:positive.append(record)
assert len(pauses)==result['pauses']
summary={'passed':True,'native_calls':len(result['calls']),'pauses':len(pauses),'max_stop_to_resume_us':max(s['stop_to_resume_us'] for s in pauses),
 'model_calls':len(receipt['stages']),'selected_types':receipt['selection']['selected_types'],'roles':len(plan['roles']),
 'selected_paths':len(plan['candidates']),'selected_embedded_paths':sum(len(l['members'])>1 for l in plan['locations']),
 'omitted_paths':plan['omitted_fields'],'omitted_batches':result['omitted_batches'],'valid_batches':len(result['batches'])-len(unresolved),
 'unresolved_batches':unresolved,'measured_fields':len(measured),'all_read_fields':sum(f['all_read'] for f in measured),
 'positive_paths':len(positive),'distinct_positive_addresses':len({f['field_address'] for f in positive}),
 'positive':positive,'quiet_stable_counts':dict(collections.Counter(f['quiet_stable_pulses'] for f in measured)),
 'meaning_verified':False,'gameplay_writes':0,'measurement_images':0}
(OUT/'audit.json').write_text(json.dumps(summary,indent=2))
print(json.dumps({k:v for k,v in summary.items() if k!='positive'}))
