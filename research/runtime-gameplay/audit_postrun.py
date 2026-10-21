"""Audit setup cleanup and saved metadata facts without another launch or input."""
import hashlib
import json
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-gameplay-v1'


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


records = [read(OUT / f'metadata-{i}.json') for i in range(8)]
assert len({r['source_sha256'] for r in records}) == 1
source = Path(records[0]['source_path'])
assert sha(source) == records[0]['source_sha256']
report = read(source)
klass = next(c for c in report['classes'] if c['name'] == 'FirstPersonController')
selected_names = ['m_WalkSpeed', 'm_RunSpeed', 'm_JumpSpeed', 'm_GravityMultiplier']
selected = [f for f in klass['fields'] if f['name'] in selected_names]
assert len(selected) == len(selected_names)
assert all(f['type_code'] == 12 and f['width'] == 4 and not f['reference'] for f in selected)
active = next(f for f in klass['fields'] if f['name'] == 'Active')
roots = [r for s in report['snapshots'] for r in s['roots']
         if r['class'] == klass['id'] and r['field'] == active['id']]
assert len(roots) == 1 and roots[0]['read_ok'] and int(roots[0]['reference'], 16) == 0
nodes = [n for s in report['snapshots'] for n in s['nodes'] if n['class'] == klass['id']]
assert not nodes
for field in selected:
    assert not any(r['declaring_class'] == klass['id'] and r['field'] == field['id'] and
                   r['read_ok'] and r['value'] is not None
                   for s in report['snapshots'] for r in s['reads'])

cases = {}
for version in (1, 2):
    base = ROOT / f'test-results/runtime-gameplay-v{version}'
    before = read(base / 'process-before.json')
    after = read(base / 'process-after.json')
    assert before['session'] == 2 and before['pid'] == after['pid'] and before['created'] == after['created']
    assert after['close_requested'] and after['exited'] and after['responding'] and after['helper_absent']
    cases[str(version)] = {'before': before, 'after': after}
focus = read(ROOT / 'test-results/runtime-gameplay-v2/focus.json')
assert not focus['ok'] and 'GameInput' in focus['error']
storage = Path(os.environ['USERPROFILE']) / 'AppData/LocalLow/Dogubomb/BLUE PRINCE/storage'
baseline = read(OUT / 'storage-before.json')
comparison = [{'name': f['name'], 'before': f['sha256'], 'after': sha(storage / f['name'])} for f in baseline]
for item in comparison:
    if item['name'] != 'RewiredSaveData.dat':
        assert item['before'] == item['after'], item['name']
result = {
    'classification': 'exploratory metadata evidence; gameplay setup blocked',
    'source_path': str(source), 'source_sha256': sha(source),
    'source_scope': {k: report[k] for k in ('pid', 'created', 'session')},
    'query_counts': {r['query']: r['matches'] for r in records},
    'candidate_type': klass['key'],
    'metadata_fields': [{k: f[k] for k in ('id', 'name', 'type_code', 'width')} for f in selected],
    'active_reference_read_successfully': True, 'active_reference_null': True,
    'observed_exact_type_nodes': 0, 'selected_scalar_read_occurrences': 0,
    'cases': cases, 'focus_refusal': focus, 'storage_comparison': comparison,
    'manual_screenshots': {'v1': 3, 'v2': 0},
    'controller_pulses': {'v1': 1, 'v2': 0},
    'input_delivered_to_game': 'unverified; menu unchanged after the single v1 A pulse',
    'native_runtime_operations_this_study': 0, 'gameplay_writes': 0,
    'verified_controls': 0, 'progress_percent': 31,
    'limitations': [
        'The metadata source is a previously retained menu observation, not a fresh gameplay discovery.',
        'Names are unverified hypotheses. Zero bounded-traversal observations do not prove process-wide absence.',
        'The null Active field is declared on this type; the retained report does not specify its declared target type.',
        'Setup images and controller actions are recorded in the tool transcript; no image-hash audit is claimed.',
        'Controller configuration changed during launches/controller connection; gameplay saves and other baseline files matched.',
    ],
}
Path(__file__).with_name('results.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
print(json.dumps({'audit': 'passed', 'gameplay_reached': False,
                  'metadata_candidates': selected_names, 'verified_controls': 0}))
