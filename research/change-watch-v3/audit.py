"""Post-launch integrity audit; never drives the fixture or selects a candidate."""
from pathlib import Path
import json, hashlib, collections, sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
config=read(HERE/'protocol-v1.json')
for name,digest in config['method_sha256'].items():assert sha(HERE/name)==digest,name
for name,digest in config['product_source_sha256'].items():assert sha(ROOT/name)==digest,name
for name,digest in config['binaries'].items():assert sha(Path(config['binary_directory'])/name)==digest,name
assert sha(HERE/config['retained_first_revision'])==config['retained_first_revision_sha256']
slot=int(sys.argv[1]);assert slot in config['order']
folder=ROOT/f'test-results/change-watch-v3/case-{slot}'
report=read(folder/'report.json');assert report['protocol_sha256']==sha(HERE/'protocol-v1.json')
if report['errors']:
 receipt={'kind':'retained failed watch case; no success claim','slot':slot,'errors':report['errors'],
  'outcome':report['outcome'],'discovery_readings':report.get('discovery_readings'),
  'raw_json_sha256':{p.name:sha(p) for p in folder.glob('*.json')}}
 (HERE/f'audit-{slot}.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
 print(json.dumps({'slot':slot,'failed_case_retained':True,'errors':report['errors']}));sys.exit(0)
final=read(folder/'final-status.json');change=final['state']['changes'][-1]
commands=read(folder/'commands.json');counts=collections.Counter(c['name'] for c in commands)
assert not report['errors'],report['errors']
assert counts['apply_change']==counts['restore_change']==counts['prepare_probe']==1
assert counts['start_change_watch']==2 and counts['observe_change']==0
assert counts['capture']==counts['correlate']==counts['finish_change_search']==0
assert change['phase']=='restored'
assert change['write']['observed_after']==change['patch']['after']
assert change['restoration']['observed_after']==change['patch']['before']
assert change['scope']['process']['pid']==report['fixture_pid']
assert change['models'][0]['address']==report['initial_oracle']['field']
assert change['models'][0]['mapping']['kind']=='F32'
assert change['models'][0]['mapping']['slope']==2
stages={};pngs=[];ids=[]
for name in ['initial','selection','focused']:
 states=read(folder/(name+'-states.json'))
 stage=[]
 for number,state in enumerate(states,1):
  live=state['live'];frame=live['frames'][-1]
  assert live['sample_count']==frame['number']==number
  assert sha(Path(live['latest']['screen']['path']))==frame['sha256']
  stage.append(frame['id']);pngs.append(frame['sha256'])
 assert not set(stage)&set(ids);ids+=stage;stages[name]=stage
assert [len(v) for v in stages.values()]==[1,4,16]
selection=read(folder/'selection.json');summary=selection['summary']
assert [f['id'] for f in summary['basis']]==stages['selection']
assert summary['planned_bytes']<=128*1024**2 and len(selection['windows'])<=4096
fresh=read(folder/'focused-states.json')[-1]['live']
checks=fresh['prediction_checks']
for check in checks:
 assert [f['id'] for f in check['basis']]==stages['focused'][:12]
 for candidate in check['candidates']:
  assert [s['image']['id'] for s in candidate['samples']]==stages['focused'][12:]
choices=[(c,i,m) for c in sorted(checks,key=lambda c:c['track']) if c['source']=='ocr'
 for i,m in enumerate(c['candidates']) if len(m['samples'])==4 and all(s['outcome']=='agrees' for s in m['samples'])]
c,index,m=choices[0]
assert (report['trial_selection']['track'],report['trial_selection']['candidate'])==(c['track'],index)
observations=change['observations'];assert len(observations)==7
assert [o['value'] for o in observations]==[50,50,60,60,60,60,50]
assert [o['intervention_outcome'] for o in observations]==['did_not_follow','did_not_follow','matched','matched','did_not_follow','did_not_follow','matched']
for o in [change['baseline']]+observations:
 assert sha(Path(o['image']))==o['image_sha256'];pngs.append(o['image_sha256'])
for o in observations:
 capture=o['capture_interval']
 assert len(o['intervention_memory'])==len(change['models'])
 for m in o['intervention_memory']:
  assert m['before']['bytes']==m['after']['bytes']==m['expected']
  assert m['before']['error'] is None and m['after']['error'] is None
  a=m['before']['interval'];b=m['after']['interval']
  assert a['clock_id']==b['clock_id']==capture['clock_id']
  assert a['begin_us']<=a['end_us']<=capture['begin_us']<=capture['end_us']<=b['begin_us']<=b['end_us']
assert len(report['input'])==report['final_oracle']['delivered']==2
assert report['final_oracle']['value']==float(report['final_oracle']['display'])==50
for index in [2,6]:
 context=observations[index]['input_context']
 assert context['clock_id']==observations[index]['capture_interval']['clock_id']
 assert any(32 in p['down'] and p['focus']=='selected_foreground' for p in context['polls'])
jobs=final['jobs']['history']['jobs'];assert len(jobs)==5 and all(j['phase']=='Completed' for j in jobs.values())
for name,start,count,phase in [('effect',1,3,'written'),('restoration',5,2,'restored')]:
 started=read(folder/(name+'-started.json'));job=jobs[str(started['data']['ack']['job'])]
 assert len(job['steps'])==count
 for i,step in enumerate(job['steps']):
  assert step['kind']=='observe' or step['kind']=='Observe'
  assert len(step['payload'].encode('utf-8'))<=256
  plan=json.loads(step['payload'])
  assert plan['change']=={'id':change['id'],'phase':phase,'after':start+i}
receipt={'kind':'post-launch integrity and owned-fixture response audit, not independent-game acceptance',
 'slot':slot,'discovery_readings':report.get('discovery_readings'),'protocol_sha256':sha(HERE/'protocol-v1.json'),'outcome':report['outcome'],'png_hashes_verified':len(pngs),
 'stage_frame_ids':stages,'selection':{k:v for k,v in summary.items() if k not in ['basis','source_frame']},
 'commands':dict(counts),'selected_candidate':report['trial_selection'],'patch':change['patch'],
 'write':change['write'],'restoration':change['restoration'],
 'observations':[{'value':o['value'],'expected':o['expected'],'stage':o['stage'],'outcome':o['intervention_outcome'],
     'input_polls':len((o.get('input_context') or {}).get('polls',[])),
     'space_polls':sum(32 in p['down'] for p in (o.get('input_context') or {}).get('polls',[])),
     'image_sha256':o['image_sha256']} for o in observations],
 'jobs':{k:{'phase':j['phase'],'steps':len(j['steps'])} for k,j in jobs.items()},
 'input':report['input'],'final_oracle':report['final_oracle'],
 'raw_json_sha256':{p.name:sha(p) for p in folder.glob('*.json')}}
(HERE/f'audit-{slot}.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'slot':slot,'verified':True,'images':len(pngs),'outcomes':[o['outcome'] for o in receipt['observations']]}))
