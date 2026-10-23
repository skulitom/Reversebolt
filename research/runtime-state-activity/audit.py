"""Replay native association and rendered differences without target access."""
import collections,hashlib,json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json');result=read(OUT/'result.json');plan=read(OUT/'plan/plan.json');job=read(OUT/'anode-job.json')
assert result['finished'] and job['finished'] and job['exitCode']==0
for name,digest in frozen['sources'].items():assert sha(OUT/'source'/name)==digest
for name,digest in frozen['binaries'].items():assert sha(OUT/'bin'/name)==digest
assert sha(OUT/'plan/plan.json')==read(OUT/'frozen-plan.json')['sha256']
receipt=read(OUT/'selection/receipt.json')
assert receipt['selection_purpose']=='current_state' and receipt['model_digest']==frozen['model_digest']
assert receipt['server_version']==frozen['server_version'] and not receipt['error'] and not receipt['cancelled'] and receipt['batch'] is None
assert len(receipt['stages'])<=24
for stage in receipt['stages']:
 assert len(stage['prompt'].encode())<=12000 and hashlib.sha256(stage['prompt'].encode()).hexdigest()==stage['prompt_sha256']
pauses=[]
for call in result['calls']:
 p=OUT/(call['label']+'.json');assert sha(p)==call['report_sha256']
 report=read(p);events=[json.loads(line) for line in (OUT/(call['label']+'.stdout')).read_text().splitlines()]
 assert events[-1]['helper_absent'] and events[-1]['inspection_exit']==0
 assert events[0]['pid']==plan['scope']['pid'] and events[0]['created']==plan['scope']['created']
 samples=report.get('pauses',report.get('snapshots',[])+[w['discarded_snapshot'] for w in report.get('warmups',[])])
 assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in samples);pauses.extend(samples)
assert len(pauses)==result['pauses']
replay=OUT/'audit-reanalysis';replay.mkdir(exist_ok=True);fields=[];unresolved=[]
for batch in result['batches']:
 i=batch['batch'];p=replay/f'analysis-{i}.json'
 if not p.exists():
  run=subprocess.run([str(OUT/'bin/reversebolt-runtime-response.exe'),str(OUT/'plan/plan.json'),str(i),str(OUT/f'observations-{i}.json'),str(p)],capture_output=True,text=True,creationflags=subprocess.CREATE_NO_WINDOW)
  (replay/f'analysis-{i}.stdout').write_text(run.stdout);(replay/f'analysis-{i}.stderr').write_text(run.stderr);assert run.returncode==batch['exit']
 if batch['exit']:unresolved.append({'batch':i,'error':(OUT/f'analysis-{i}.stderr').read_text()});continue
 analysis=read(p);assert analysis==batch['analysis']==read(OUT/f'analysis-{i}.json')
 observations=read(OUT/f'observations-{i}.json')
 for j,field in enumerate(analysis['fields']):fields.append(dict(field,batch=i,field_address=observations[0]['report']['checks'][j]['field_address']))
captures=result['captures'];frames=[c['frame'] for c in captures];images=read(OUT/'image-evidence.json')
assert len(captures)==6*len(result['batches'])<=48
assert len({(f['width'],f['height'],f['epoch']) for f in frames})==1
assert all(c['error'] is None for c in images['capture_clocks']['frames'])
files={sha(p):p for p in (OUT/'data/captures').rglob('*.png')}
for frame in frames:assert frame['sha256'] in files and frame['input_context'] is None
assert images['activity_source'] is None
pairs=[[str(files[a['sha256']]),str(files[b['sha256']])] for a,b in zip(frames,frames[1:])]
(replay/'image-pairs.json').write_text(json.dumps(pairs));metric=ROOT/'target/release/examples/response_audit.exe'
run=subprocess.run([str(metric),str(replay/'image-pairs.json')],capture_output=True,text=True,creationflags=subprocess.CREATE_NO_WINDOW)
assert run.returncode==0,run.stderr
replayed=json.loads(run.stdout);(replay/'images.json').write_text(json.dumps(replayed,indent=2))
intervals=images['evidence']['intervals'];assert len(intervals)==len(captures)-1
for i,(pair,interval) in enumerate(zip(replayed['pairs'],intervals)):
 assert pair['hashes']==[frames[i]['sha256'],frames[i+1]['sha256']]==[interval['before_sha256'],interval['after_sha256']]
 assert interval['measurement']['error'] is None and pair['difference']==interval['measurement']['difference']
 assert interval['exposure']['kind']=='unknown'
image_batches=[]
for b in range(len(result['batches'])):
 group=captures[6*b:6*b+6];assert [(c['batch'],c['sample']) for c in group]==[(b,i) for i in frozen['capture_samples']]
 for c in group:
  check=read(OUT/f"sample-{b}-{c['sample']}.json")['pauses'][-1]['sampling_qpc'];assert check['end_ticks']<c['begin_ticks']
  if c['sample'] in [2,5]:
   h=next(h for h in result['holds'] if h['batch']==b and h['sample']==c['sample']);assert h['end_ticks']<c['begin_ticks']
 image_batches.append({'batch':b,'intervals':[{'kind':kind,'difference':intervals[6*b+j]['measurement']['difference'],'elapsed_us':intervals[6*b+j]['elapsed_us'],'patch_consensus':images['patch_motion']['intervals'][6*b+j]['measurement']['consensus'] if images['patch_motion']['intervals'][6*b+j]['measurement'] else None} for j,kind in enumerate(['quiet_before','D','post_D','A','post_A'])]})
assert all(h['down_accepted']==h['up_accepted']==1 and not h.get('interrupted') for h in result['holds'])
locations={l['path_id']:l for l in plan['locations']};changed=[f for f in fields if len({json.dumps(v,sort_keys=True) for v in f['values']})>1];positive=[f for f in fields if f['repeated_association']]
summary={'passed':True,'native_calls':len(result['calls']),'pauses':len(pauses),'maximum_stop_to_resume_us':max(p['stop_to_resume_us'] for p in pauses),'model_calls':len(receipt['stages']),
 'selected_paths':len(plan['candidates']),'selected_roles':len(plan['roles']),'selected_embedded_paths':sum(len(l['members'])>1 for l in plan['locations']),'omitted_fields':plan['omitted_fields'],'omitted_batches':result['omitted_batches'],
 'valid_batches':len(result['batches'])-len(unresolved),'unresolved_batches':unresolved,'measured_fields':len(fields),'read_fields':sum(f['all_read'] for f in fields),'measured_embedded':sum(len(locations[f['path_id']]['members'])>1 for f in fields),'distinct_initial_addresses':len({f['field_address'] for f in fields}),
 'changed_fields':len(changed),'positive_paths':len(positive),'distinct_positive_addresses':len({f['field_address'] for f in positive}),'positive':positive,'quiet_stable_counts':dict(collections.Counter(f['quiet_stable_pulses'] for f in fields)),
 'image_count':len(frames),'image_batches':image_batches,'image_metric_binary_sha256':sha(metric),'gameplay_writes':0,'viewed_measurement_images':0,'meaning_verified':False}
(OUT/'audit.json').write_text(json.dumps(summary,indent=2));print(json.dumps({k:v for k,v in summary.items() if k not in ['positive','image_batches']}))
