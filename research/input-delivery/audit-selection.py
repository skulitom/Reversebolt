"""Saved current-state selection audit; no inference, images or target access."""
import hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'test-results/input-delivery-v1/state-selection';PRIOR=ROOT/'test-results/runtime-embedded-activity-v3'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json');receipt=read(OUT/'selection/receipt.json');previous=read(PRIOR/'selection/receipt.json')
for name,digest in frozen['sources'].items():assert sha(OUT/'source'/name)==digest
for name,digest in frozen['binaries'].items():assert sha(OUT/'bin'/name)==digest
assert sha(Path(frozen['report']))==frozen['report_sha256'] and sha(PRIOR/'selection/receipt.json')==frozen['prior_receipt_sha256']
assert receipt['report_sha256']==previous['report_sha256']
assert receipt['model_digest']==frozen['model_digest'] and receipt['server_version']==frozen['server_version']
assert receipt['selection_purpose']=='current_state' and not receipt['cancelled'] and not receipt['error'] and receipt['batch'] is None
assert not receipt.get('incomplete_stage') and len(receipt['stages'])<=24
for stage in receipt['stages']:
 assert hashlib.sha256(stage['prompt'].encode()).hexdigest()==stage['prompt_sha256'] and len(stage['prompt'].encode())<=12000
 assert stage['response']['done'] and stage['response']['done_reason']=='stop'
assert frozen['binaries']['reversebolt-runtime-neighbors.exe']==read(PRIOR/'frozen.json')['binaries']['reversebolt-runtime-neighbors.exe']
inventory=read(OUT/'selection/input.json')['inventory'];types={t['id']:t for t in inventory['types']}
selected=receipt['selection']['selected_types'];assert len(set(selected))==len(selected)<=8 and all(i in types for i in selected)
plan=read(OUT/'plan/plan.json');oldplan=read(PRIOR/'plan/plan.json')
assert plan['policy']==oldplan['policy'] and plan['scope']==oldplan['scope'] and len(plan['candidates'])<=128
assert all(not c['edit_authority'] for c in plan['candidates'])
summary={'passed':True,'source_report_sha256':frozen['report_sha256'],'selection_purpose':'current_state','model_calls':len(receipt['stages']),
 'maximum_prompt_bytes':max(len(s['prompt'].encode()) for s in receipt['stages']),'selected_types':[{'id':i,'observed_type_label':types[i]['key']['type_name']} for i in selected],
 'type_overlap':len(set(selected)&set(previous['selection']['selected_types'])),
 'path_overlap':len({c['id'] for c in plan['candidates']}&{c['id'] for c in oldplan['candidates']}),
 'paths':len(plan['candidates']),'embedded_paths':sum(len(l['members'])>1 for l in plan['locations']),'roles':len(plan['roles']),'batches':len(plan['batches']),'omitted_fields':plan['omitted_fields'],
 'native_activity_tested':False,'better_discovery_verified':False,'gameplay_writes':0}
(OUT/'audit.json').write_text(json.dumps(summary,indent=2));print(json.dumps(summary))
