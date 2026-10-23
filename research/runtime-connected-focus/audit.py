"""Independently check rooted connections and retained inference provenance."""
import ast
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-connected-focus-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
encoded = lambda v: json.dumps(v, ensure_ascii=False, separators=(',', ':')).encode()
key = lambda v: json.dumps(v, ensure_ascii=False, separators=(',', ':'), sort_keys=True)
frozen = read(OUT / 'frozen.json')
prior = Path(frozen['prior'])
assert frozen['connected_focus'] and read(OUT / 'terminal.json')['exit_code'] == 0
for group, folder in [('sources', 'source'), ('inputs', 'input'), ('binaries', 'bin')]:
    for name, digest in frozen[group].items():
        assert sha(OUT / folder / name) == digest
assert sha(prior / 'audit.json') == frozen['prior_audit_sha256']
for name in frozen['inputs']:
    if name != 'manifest.json':
        assert sha(OUT / 'input' / name) == sha(prior / name)
focus = read(OUT / 'review/focus.json')
receipt = read(OUT / 'review/receipt.json')
input_data = read(OUT / 'review/input.json')
assert hashlib.sha256(encoded(focus)).hexdigest() == input_data['focus_sha256']
assert focus['inventory'] == input_data['inventory'] and not focus['edit_authority']
assert focus['policy'] == 'response_connected_parameter_focus_v1'
assert focus['observed_batches'] == list(range(8)) and focus['unobserved_batches'] == [8, 9]
for i, response in enumerate(focus['responses']):
    assert response == read(prior / f'analysis-{i}.json')
positive = [f['path_id'] for r in focus['responses'] for f in r['fields'] if f['repeated_association']]
assert focus['responding_paths'] == positive
original = read(OUT / 'input/plan/plan.json')
seeds = {c['id']: c for c in original['candidates'] if c['id'] in positive}
report = read(OUT / 'input/discovery.json')
classes = {key(c['key']): c for c in report['classes']}
assert len({s['snapshot_id'] for s in report['snapshots']}) == len(report['snapshots'])

def role_chain(candidate):
    path = candidate['locator']
    names, embedded = [], []
    for step in path['steps']:
        member = step['member']
        if member['kind'] == 'field':
            field = member['field']
            descriptor = classes[key(field['declaring'])]['fields'][field['field_id']]
            names.append(descriptor['name'])
            embedded.append(descriptor.get('inline', False))
        else:
            names.append(f"[{member['index']}]")
            embedded.append(False)
    chain, parent = [], 0
    for anchor in range(len(path['steps'])):
        if anchor == 0 or not embedded[anchor - 1]:
            identity = {'scope': path['scope'], 'root': path['root'],
                        'references': path['steps'][:anchor], 'owner': path['steps'][anchor]['owner']}
            chain.append({'id': hashlib.sha256(key(identity).encode()).hexdigest(),
                          'via': names[parent:anchor]})
            parent = anchor
    return chain

connections = focus['connections']
assert connections['responding_paths'] == positive
assert connections['report_sha256'] == focus['inventory']['report_sha256']
assert connections['policy'] == 'immediate_rooted_connections_v1'
roles = {r['id']: r for r in connections['roles']}
responding_roles = {role_chain(c)[-1]['id'] for c in seeds.values()}
assert not (set(roles) & responding_roles)
plan = focus['neighborhood']
paths = {c['id']: c for c in plan['candidates']}
assert connections['available_fields'] == sum(r['available_fields'] for r in roles.values())
assert connections['selected_fields'] == len(paths)
assert connections['omitted_fields'] == connections['available_fields'] - len(paths)
assert connections['omitted_roles'] == sum(not r['selected'] for r in roles.values())
assert {r['id'] for r in plan['roles']} == {r['id'] for r in roles.values() if r['selected']}
for candidate, location in zip(plan['candidates'], plan['locations']):
    chain = role_chain(candidate)
    assert chain[-1]['id'] == location['role_id'] and location['path_id'] == candidate['id']
    assert not candidate['edit_authority']
    role = roles[location['role_id']]
    assert role['links']
    for link in role['links']:
        source = seeds[link['responding_path']]
        source_chain = role_chain(source)
        assert link['snapshot_id'] == source['snapshot_id'] == candidate['snapshot_id']
        assert link['responding_role'] == source_chain[-1]['id']
        if link['direction'] == 'parent':
            assert source_chain[-2]['id'] == chain[-1]['id']
            assert link['members_from_parent'] == source_chain[-1]['via']
        else:
            assert link['direction'] == 'child' and chain[-2]['id'] == source_chain[-1]['id']
            assert link['members_from_parent'] == chain[-1]['via']
original_types = {t['id']: t for t in read(prior / 'selection/input.json')['inventory']['types']}
samples = []
for typ in focus['inventory']['types']:
    assert typ['key'] == original_types[typ['id']]['key']
    originals = {f['field_id']: f for f in original_types[typ['id']]['fields']}
    for field in typ['fields']:
        assert field['name'] == originals[field['field_id']]['name']
        for sample in field['samples']:
            candidate = paths[sample['path_id']]
            assert sample['value'] == candidate['value'] and sample['context'] == candidate['context']
            samples.append(sample['path_id'])
assert len(samples) == focus['review_sampled_paths'] == len(set(samples))
assert receipt['selection_purpose'] == 'adjustable_parameters'
assert not receipt['error'] and not receipt['cancelled'] and receipt['activity'] is None
assert receipt['model_digest'] == frozen['model_digest'] and receipt['server_version'] == frozen['server_version']
assert receipt['report_sha256'] == focus['inventory']['report_sha256']
assert set(receipt['selection']['selected_types']) <= {t['id'] for t in focus['inventory']['types']}
assert all(row['path_id'] in samples for row in receipt['selection']['rows'])
assert len(receipt['stages']) <= frozen['max_selection_requests'] + frozen['max_proposal_requests']
for stage in receipt['stages']:
    assert hashlib.sha256(stage['prompt'].encode()).hexdigest() == stage['prompt_sha256']
    assert len(stage['prompt'].encode()) <= 110000
    assert stage['response']['prompt_eval_count'] <= stage['options']['num_ctx'] - stage['options']['num_predict'] - 512
for proposal in receipt['batch']['accepted']:
    assert proposal['candidate']['path_id'] in samples
    assert all(a['path_id'] in samples for a in proposal['alternatives'])
for folder in ['runtime-activity-focus', 'runtime-connected-focus']:
    for path in (ROOT / 'research' / folder).glob('*.py'):
        ast.parse(path.read_text(encoding='utf-8-sig'))
summary = {
    'passed': True, 'responding_paths': len(positive), 'connected_roles': len(roles),
    'available_fields': connections['available_fields'], 'selected_fields': len(paths),
    'omitted_fields': connections['omitted_fields'], 'omitted_roles': connections['omitted_roles'],
    'review_sampled_paths': len(samples), 'unobserved_batches': focus['unobserved_batches'],
    'selected_types': receipt['selection']['selected_types'], 'model_calls': len(receipt['stages']),
    'maximum_prompt_bytes': max(len(s['prompt'].encode()) for s in receipt['stages']),
    'model_elapsed_ms': sum(s['elapsed_ms'] for s in receipt['stages']),
    'accepted': len(receipt['batch']['accepted']), 'rejected': len(receipt['batch']['rejected']),
    'new_native_calls': 0, 'gameplay_writes': 0, 'new_images': 0, 'semantics_verified': False,
}
(OUT / 'audit.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
print(json.dumps(summary))
