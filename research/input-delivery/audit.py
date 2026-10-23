"""Audit saved API/timing/images and replay the existing rendered metric offline."""
import collections,hashlib,json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json');result=read(OUT/'result.json');evidence=read(OUT/'evidence.json');job=read(OUT/'anode-job.json')
assert result['finished'] and job['finished'] and job['exitCode']==0
for name,digest in frozen['sources'].items():assert sha(OUT/'source'/name)==digest
for name,digest in frozen['binaries'].items():assert sha(Path(frozen['bin'])/name)==digest
captures=result['captures'];holds=result['holds'];assert len(captures)==18 and len(holds)==8
assert [(h['method'],h['key']) for h in holds]==[(m,k) for m in frozen['methods'] for k in frozen['keys']]
frames=[c['frame'] for c in captures];assert [f['number'] for f in frames]==list(range(1,19))
assert len({(f['width'],f['height'],f['epoch']) for f in frames})==1
assert all(c['error'] is None for c in evidence['capture_clocks']['frames'])
files={sha(p):p for p in (OUT/'data/captures').rglob('*.png')}
for f in frames:assert f['sha256'] in files
intervals=evidence['evidence']['intervals'];assert len(intervals)==17
for i,interval in enumerate(intervals):
 assert interval['before']==frames[i]['id'] and interval['after']==frames[i+1]['id']
 assert interval['before_sha256']==frames[i]['sha256'] and interval['after_sha256']==frames[i+1]['sha256']
 assert interval['measurement']['error'] is None
 assert interval['exposure']['kind']=='unknown' # Direct captures do not own an input monitor.
assert evidence['activity_source'] is None
assert all(f['input_context'] is None for f in frames)
rows=[]
for i,h in enumerate(holds):
 before,after=captures[2*i+1],captures[2*i+2]
 assert before['end_ticks']<h['begin_ticks']<h['end_ticks']<after['begin_ticks']
 polls=h['polls'];assert len(polls)>10 and all(p['foreground'] and p['down']==[h['key']] for p in polls)
 gaps=[b['begin_ticks']-a['end_ticks'] for a,b in zip(polls,polls[1:])]
 assert min(gaps)>0 and max(gaps)*1000/h['frequency_hz']<50
 # Retain QPC duration separately from the runner's coarse monotonic deadline.
 # Do not silently round it to the nominal one-second hold.
 duration=1000*(h['end_ticks']-h['begin_ticks'])/h['frequency_hz']
 if h['method']=='send_input':assert h['down_accepted']==h['up_accepted']==1
 else:assert h['down_accepted'] is None and h['up_accepted'] is None
 selected=after['frame']['sampling'];stamp=selected['frames'][selected['selected']]['compositor_100ns']
 assert stamp*h['frequency_hz']>h['end_ticks']*10_000_000
 quiet_before=intervals[2*i]['measurement']['difference'];action=intervals[2*i+1]['measurement']['difference'];quiet_after=intervals[2*i+2]['measurement']['difference']
 spatial=evidence['patch_motion']['intervals'][2*i+1]
 rows.append({'method':h['method'],'key':h['key'],'hold_ms':duration,'sampled_key_polls':len(polls),'quiet_before':quiet_before,'action':action,'quiet_after':quiet_after,
  'action_to_max_quiet':action/max(quiet_before,quiet_after) if max(quiet_before,quiet_after)>0 else None,
  'patch_error':spatial['error'],'matched_patches':spatial['measurement']['matched_patches'] if spatial['measurement'] else None,'consensus':spatial['measurement']['consensus'] if spatial['measurement'] else None})
pairs=[[str(files[a['sha256']].resolve()),str(files[b['sha256']].resolve())] for a,b in zip(frames,frames[1:])]
(OUT/'pairs.json').write_text(json.dumps(pairs))
binary=ROOT/'target/release/examples/response_audit.exe'
run=subprocess.run([str(binary),str(OUT/'pairs.json')],capture_output=True,text=True,creationflags=subprocess.CREATE_NO_WINDOW)
assert run.returncode==0,run.stderr
replayed=json.loads(run.stdout)
assert len(replayed['pairs'])==len(intervals)
for pair,interval in zip(replayed['pairs'],intervals):
 assert pair['hashes']==[interval['before_sha256'],interval['after_sha256']]
 assert pair['difference']==interval['measurement']['difference']
(OUT/'metric-replay.json').write_text(json.dumps(replayed,indent=2))
last=read(sorted(OUT.glob('rpc-*-capture.json'))[-1])['response']['state']['live']
assert last['coverage']['read_bytes']==last['coverage']['attempted_bytes']==0
assert last['sample_count']==18
summary={'passed':True,'measurement_images':18,'viewed_measurement_images':0,'memory_bytes_read':0,'model_calls':0,'field_writes':0,'input_holds':8,'rows':rows,
 'product_input_classification':'unknown: direct captures have no job-owned monitor','external_input_binding':'fixed diagnostic sequence and retained QPC-bracketed runner holds, not a rewritten product assessment',
 'replay_binary_sha256':sha(binary),'replay_sources':{name:sha(ROOT/name) for name in ['crates/reversebolt-vision/examples/response_audit.rs','crates/reversebolt-vision/src/response.rs']},
 'meaning_verified':False,'causal_api_comparison':False}
(OUT/'audit.json').write_text(json.dumps(summary,indent=2));print(json.dumps(summary))
