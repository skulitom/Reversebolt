"""Check frozen integrity, declared controls, and preservation of earlier evidence."""
import copy, hashlib, json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/appearance-sequence-v1'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
frozen=read(OUT/'source-frozen.json')
assert all(sha(ROOT/p)==h and sha(OUT/'source'/p)==h for p,h in frozen['sources'].items())
assert all(sha(Path(p))==h for p,h in frozen['binaries'].items())
inputs=read(OUT/'inputs-frozen.json');assert all(sha(Path(p))==h for p,h in inputs.items())
result=read(OUT/'result.json');assert result['manifest_sha256']==sha(OUT/'cases.json')
assert not result['target_access'] and not result['input_sent'] and result['new_model_calls']==result['field_writes']==0
cases=result['cases'];assert len(cases)==8
previous=read(ROOT/'test-results/region-appearance-v1/result.json')['cases'][5:]
for current,old in zip(cases[5:],previous,strict=True):
    stripped=copy.deepcopy(current);stripped.pop('appearance_sequences');assert stripped==old
def compact(case):
    outer=case['appearance_sequences'];assert outer['error'] is None
    report=outer['report']
    return {'name':case['name'],'source_sha256':report['source_sha256'],'attempted_pairs':report['attempted_pairs'],
        'series':[{'origin':s['source']['before_id'],'source_sha256':s['source']['source_sha256'],
            'claims':[o['observation']['input']['label'] for o in s['source']['sources']],
            'accepted':s['accepted_steps'],'stop':s['stop'],'stopped_before':s['stopped_before'],'reason':s['reason'],
            'steps':[{k:step[k] for k in ['before_id','after_id','source_sha256','elapsed_us','timing_error','exposure',
                'input_sha256','displacement_from_origin','displacement_from_previous','error']} |
                {'measurement_status':step['measurement']['status'] if step['measurement'] else None}
                for step in s['steps']]} for s in report['series']]}
controls=[]
for case,truth in zip(cases[:5],read(OUT/'truth.json'),strict=True):
    c=compact(case);assert c['name']==truth['name']
    passed=[s['accepted'] for s in c['series']]==truth['accepted'] and c['attempted_pairs']==truth['attempts'] and all(s['stop']==truth['stop'] for s in c['series'])
    if 'deltas' in truth:passed=passed and [s['displacement_from_previous'] for s in c['series'][0]['steps']]==truth['deltas']
    controls.append({'expected':truth,'passed':passed,'result':c})
audit={'source_manifest_sha256':sha(OUT/'source-frozen.json'),'result_sha256':sha(OUT/'result.json'),
    'frozen_sources':len(frozen['sources']),'unchanged_inputs':len(inputs),'previous_three_cases_unchanged':True,
    'analytic_passed':sum(c['passed'] for c in controls),'analytic_total':len(controls),'analytic':controls,
    'game':[compact(c) for c in cases[5:]],'game_accuracy':'unknown','process':read(OUT/'process.json')}
with (OUT/'audit.json').open('x',encoding='utf-8') as f:f.write(json.dumps(audit,indent=2)+'\n')
print(json.dumps({k:v for k,v in audit.items() if k not in ['analytic','game']}))
