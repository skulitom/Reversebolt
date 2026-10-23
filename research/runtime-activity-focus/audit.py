"""Check retained scope, provenance and omissions without model or game access."""
import ast
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-activity-focus-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
prior = Path(frozen['prior'])
assert read(OUT / 'terminal.json')['exit_code'] == 0
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
assert hashlib.sha256(json.dumps(focus, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest() == input_data['focus_sha256']
assert focus['inventory'] == input_data['inventory']
assert not focus['edit_authority']
assert focus['observed_batches'] == list(range(8)) and focus['unobserved_batches'] == [8, 9]
for i, response in enumerate(focus['responses']):
    assert response == read(prior / f'analysis-{i}.json')
expected = [f['path_id'] for response in focus['responses'] for f in response['fields'] if f['repeated_association']]
assert focus['responding_paths'] == expected
roles = {f['role_id'] for response in focus['responses'] for f in response['fields'] if f['repeated_association']}
plan = focus['neighborhood']
assert {r['id'] for r in plan['roles']} == roles
assert all(l['role_id'] in roles for l in plan['locations'])
assert all(not c['edit_authority'] for c in plan['candidates'])
paths = {c['id']: c for c in plan['candidates']}
original_types = {t['id']: t for t in read(prior / 'selection/input.json')['inventory']['types']}
samples = []
for typ in focus['inventory']['types']:
    assert typ['key'] == original_types[typ['id']]['key']
    originals = {f['field_id']: f for f in original_types[typ['id']]['fields']}
    for field in typ['fields']:
        original = originals[field['field_id']]
        assert (field['name'], field['type_code']) == (original['name'], original['type_code'])
        for sample in field['samples']:
            candidate = paths[sample['path_id']]
            assert sample['value'] == candidate['value'] and sample['context'] == candidate['context']
            samples.append(sample['path_id'])
assert len(samples) == focus['review_sampled_paths'] == len(set(samples))
assert not receipt['error'] and not receipt['cancelled'] and receipt['activity'] is None
assert receipt['selection_purpose'] == 'adjustable_parameters'
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
for path in (ROOT / 'research/runtime-activity-focus').glob('*.py'):
    ast.parse(path.read_text(encoding='utf-8-sig'))
summary = {
    'passed': True, 'responding_paths': len(expected), 'roles': len(roles),
    'focused_paths': len(paths), 'review_sampled_paths': len(samples),
    'omitted_neighbors': plan['omitted_fields'], 'unobserved_batches': focus['unobserved_batches'],
    'selected_types': receipt['selection']['selected_types'],
    'model_calls': len(receipt['stages']),
    'maximum_prompt_bytes': max(len(s['prompt'].encode()) for s in receipt['stages']),
    'model_elapsed_ms': sum(s['elapsed_ms'] for s in receipt['stages']),
    'accepted': len(receipt['batch']['accepted']), 'rejected': len(receipt['batch']['rejected']),
    'new_native_calls': 0, 'gameplay_writes': 0, 'new_images': 0, 'semantics_verified': False,
}
(OUT / 'audit.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
print(json.dumps(summary))
