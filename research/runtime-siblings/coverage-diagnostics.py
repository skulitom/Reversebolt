"""Describe retained coverage gaps, without choosing new live fields."""
import hashlib
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/runtime-siblings-sable-selected-v1'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json');report=read(OUT/'report.json');receipt=read(OUT/'receipt.json')
assert sha(OUT/'report.json')==frozen['inputs']['report']['sha256']
prior=Path(frozen['prior']);discovery=read(prior/'discovery.json')
inventory=read(Path(frozen['data'])/'runtime/naming'/discovery['state']['live']['runtime']['naming']['evidence_id']/'inventory.json')
keys=[t['key'] for t in inventory['types'] if t['id'] in receipt['selection']['selected_types']]
unsupported=[(c,f) for c in report['classes'] for f in c['fields'] if f['type_code']==17 and f['width']==0 and not f['reference']]
plan=read(OUT/'plan/plan.json');names={c['id']:c['name'] for c in plan['candidates']}
changing=[];constant=0
for b in range(len(plan['batches'])):
    observations=read(OUT/f'observations-{b}.json')
    for j,c in enumerate(observations[0]['report']['checks']):
        checks=[o['report']['checks'][j] for o in observations]
        assert all(v['status']=='read' for v in checks)
        variants=len({json.dumps(v['bytes']) for v in checks})
        if variants==1:constant+=1
        else:changing.append({'path_id':c['path_id'],'field_name':names[c['path_id']],
            'field_address':c['field_address'],'distinct_values':variants,'batch':b})
result={'report_sha256':sha(OUT/'report.json'),'sampled_constant_paths':constant,
    'sampled_changing_paths':changing,'distinct_snapshot_addresses_among_changing_paths':len({c['field_address'] for c in changing}),
    'unsupported_value_type_field_descriptors':len(unsupported),
    'unsupported_value_type_fields_in_selected_types':sum(c['key'] in keys for c,f in unsupported),
    'limitation':'Field descriptors and addresses are retained runtime evidence, not known gameplay meaning or persistent identity. Unsupported/omitted fields remain untested.'}
(OUT/'coverage-diagnostics.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
