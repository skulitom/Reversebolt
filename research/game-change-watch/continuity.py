"""Post-run analysis of retained bytes, not a new product outcome or acceptance test.

Reuses each trial's pre-existing mapping without fitting or feeding anything back
to Reversebolt. Correlation after input remains insufficient to establish cause.
"""
import json
import math
from pathlib import Path
import struct

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))

def compare(model, sample, observation):
    row={'model':model['index'],'mapping':model['mapping'],'status':'unavailable'}
    before=sample['before'];after=sample.get('after');capture=observation.get('capture_interval')
    if not after or before.get('error') or after.get('error') or not capture:return row
    first,last=before['interval'],after['interval']
    ordered=(first['clock_id']==capture['clock_id']==last['clock_id'] and
        first['begin_us']<=first['end_us']<=capture['begin_us']<=capture['end_us']<=last['begin_us']<=last['end_us'])
    if not ordered:return {**row,'status':'timing_uncertain'}
    mapping=model['mapping'];fmt={'I32':'<i','F32':'<f','F64':'<d'}[mapping['kind']]
    if before.get('bytes') is None or after.get('bytes') is None:return row
    if len(before['bytes'])!=struct.calcsize(fmt) or len(after['bytes'])!=struct.calcsize(fmt):return row
    raw=[struct.unpack(fmt,bytes(v['bytes']))[0] for v in [before,after]]
    if any(not math.isfinite(v) or abs(v)>1e12 or (v!=0 and abs(v)<1e-12) for v in raw):return row
    predicted=[v*mapping['slope']+mapping['intercept'] for v in raw]
    if not all(map(math.isfinite,predicted)):return row
    row.update(raw_endpoints=raw,predicted_endpoints=predicted,ordered_capture_bracket=True,
        original_expected_bytes=sample['expected'],endpoint_bytes_equal=before['bytes']==after['bytes'])
    if before['bytes']!=after['bytes']:return {**row,'status':'changed_during_capture'}
    value=observation['value']
    if observation.get('error') or value is None or not math.isfinite(value):return {**row,'status':'visual_unavailable'}
    # Exact equality is descriptive here, not a newly fitted tolerance or gate.
    return {**row,'status':'exact_current_mapping_agreement' if predicted[0]==value else 'current_mapping_differs',
        'observed':value,'difference':value-predicted[0]}

cases=[]
for game in ['2048','tetris']:
    directory=ROOT/f'test-results/game-change-watch-v1/{game}-1'
    state=read(directory/'final-status.json');run=read(directory/'run.json')
    case={'game':game,'change_id':run.get('changeId'),'observations':[]}
    for change in state['state']['changes']:
        if change['id']!=run.get('changeId'):continue
        for index,o in enumerate(change['observations']):
            models=[compare(m,s,o) for s in o['intervention_memory'] for m in change['models']
                if m['index']==s['model'] and m['address']==s['address']]
            case['observations'].append({'index':index,'product_outcome':o['intervention_outcome'],
                'value':o['value'],'models':models})
    cases.append(case)
result={'kind':'post-run descriptive analysis; no refit, causal claim or product reclassification',
    'limitation':'Equal endpoints do not exclude ABA, display delay, natural changes or observer artifacts. No paired counterfactual was run.',
    'cases':cases}
(HERE/'continuity.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print(json.dumps([{'game':c['game'],'statuses':[[m['status'] for m in o['models']] for o in c['observations']]} for c in cases]))
