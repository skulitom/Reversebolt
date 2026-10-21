"""Read-only evidence audit; accepted proposals are NOT verified gameplay meanings."""
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUN = Path(sys.argv[1]).resolve()
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def digest(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def sha(data): return hashlib.sha256(data).hexdigest()

frozen = read(RUN / 'frozen.json')
for name, expected in frozen['sources'].items():
    assert digest(ROOT / name) == expected, name
for name, expected in frozen['binaries'].items():
    assert digest(RUN / 'bin' / name) == expected, name
v3 = ROOT / 'test-results/runtime-hypotheses-v3'
assert digest(v3 / 'bin/reversebolt-runtime-hypotheses.exe') == frozen['binaries']['reversebolt-runtime-hypotheses.exe']

def audit_receipt(receipt, inventory):
    assert receipt['error'] is None and not receipt['cancelled']
    assert receipt['model_digest'] and receipt['report_sha256'] == inventory['report_sha256']
    assert 1 <= len(receipt['stages']) <= 2
    responses = []
    for stage in receipt['stages']:
        assert sha(stage['prompt'].encode()) == stage['prompt_sha256']
        assert len(stage['prompt'].encode()) <= 110000
        response = stage['response']
        assert response['done'] and response['done_reason'] == 'stop'
        assert not response.get('message', {}).get('tool_calls')
        content = response.get('response', response.get('message', {}).get('content'))
        responses.append(json.loads(content))
    selection = receipt['selection']
    assert selection['selected_types'] == responses[0]['types']
    assert len(selection['rows']) <= 256 and len(selection['selected_types']) <= 8
    for row in selection['rows']:
        t = next(t for t in inventory['types'] if t['id'] == row['type_id'])
        f = next(f for f in t['fields'] if f['field_id'] == row['field_id'])
        s = next(s for s in f['samples'] if s['path_id'] == row['path_id'])
        assert row['field_name'] == f['name'] and row['type_name'] == t['key']['type_name']
        assert row['value'] == s['value']
        assert row['object_text'] == [c['text'] for c in s['context'] if not c['truncated'] and len(c['text'].encode()) <= 256]
    batch = receipt['batch']
    assert batch['report_sha256'] == selection['report_sha256'] == inventory['report_sha256']
    proposed = responses[1]['hypotheses'] if len(responses) == 2 else []
    assert len(batch['accepted']) + len(batch['rejected']) == len(proposed) <= 12
    seen = set()
    for h in batch['accepted']:
        p = h['proposal']; row = selection['rows'][p['candidate']]
        assert p in proposed and h['candidate'] == row and row['path_id'] not in seen
        seen.add(row['path_id'])
        assert h['alternatives'] == [selection['rows'][i] for i in p['alternatives']]
        assert all(a['path_id'] != row['path_id'] for a in h['alternatives'])
        for c in p['citations']:
            source = row[c['source']][c['index']] if c['source'] == 'object_text' else row[c['source']]
            assert c['quote'] == source
    for r in batch['rejected']:
        assert r['proposal'] in proposed and r['reason']
    return {'accepted': len(batch['accepted']), 'rejected': len(batch['rejected']),
            'elapsed_ms': sum(s['elapsed_ms'] for s in receipt['stages']),
            'selected_types': selection['selected_types'], 'samples': len(selection['rows']),
            'omitted_samples': selection['omitted_samples'], 'model_digest': receipt['model_digest'],
            'receipt_sha256': sha(json.dumps(receipt, sort_keys=True).encode()),
            'names': [h['proposal']['name'] for h in batch['accepted']]}

replays = {}
for version in (1, 2, 3):
    base = ROOT / f'test-results/runtime-hypotheses-v{version}'
    replays[str(version)] = {}
    for case in ('manifold', 'menu', 'fixture', 'opaque'):
        data = read(base / case / 'input.json')
        assert data['opaque_research_only'] == (case == 'opaque')
        receipt = read(base / case / 'receipt.json')
        replays[str(version)][case] = audit_receipt(receipt, data['inventory'])

native = RUN / 'native'
result = read(native / 'result.json'); assert result['passed']
automatic = read(native / 'automatic.json')['state']
naming = automatic['live']['runtime']['naming']
review_dir = native / 'data/runtime/naming' / naming['evidence_id']
receipt = read(review_dir / 'receipt.json')
assert receipt == result['model_receipt'] and receipt['batch'] == naming['batch']
live = audit_receipt(receipt, read(review_dir / 'inventory.json'))
intent = read(review_dir / 'intent.json')
assert intent['report_sha256'] == naming['report_sha256']
assert intent['catalog_id'] == automatic['live']['runtime']['catalog_id']
assert intent['scope'] == read(review_dir / 'inventory.json')['scope']
for filename, status in [('unavailable', 'unavailable'), ('cancelled', 'cancelled'), ('storage-failure', 'failed')]:
    n = read(native / (filename + '.json'))['state']['live']['runtime']['naming']
    assert n['status'] == status and n['batch'] is None
assert not list((native / 'data').rglob('*.png'))
assert result['images'] == result['input_actions'] == result['gameplay_writes'] == 0
helpers = list((native / 'data/runtime').glob('*-*/**/stdout.jsonl'))
pauses = []
for path in helpers:
    logs = [json.loads(s) for s in path.read_text().splitlines() if s.startswith('{')]
    assert logs[-1]['stage'] == 'complete' and logs[-1]['helper_absent'] and logs[-1]['inspection_exit'] == 0
    report = read(path.parent / 'report.json')
    assert report.get('operation') != 'patch_runtime_path' and report['session'] == intent['scope']['session'] == 3
    assert report['pid'] == intent['scope']['pid']
    pauses.extend(report.get('pauses', report.get('snapshots', [])))
    pauses.extend(w['discarded_snapshot'] for w in report.get('warmups', []))
assert helpers and pauses
assert all(p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0 for p in pauses)
balance = read(native / 'balance.json')
assert balance['starts'] == balance['stops'] == len(pauses) and balance['held_ticks'] == 0
children = read(native / 'children.json')
assert all(c['exit_code'] is not None for c in children)
tests = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', (RUN / 'workspace-final.log').read_text(encoding='utf-8-sig'))))
assert tests == 264, tests
summary = {'passed': True, 'artifact_root': str(RUN.relative_to(ROOT)), 'frozen': frozen,
           'workspace_tests': tests, 'native_checks': len(result['checks']), 'native_review': live,
           'replays': replays, 'helpers': len(helpers), 'balanced_pauses': len(pauses),
           'maximum_stop_to_resume_us': max(p['stop_to_resume_us'] for p in pauses),
           'helper_heap_operations_while_paused': 0, 'images': 0, 'input_actions': 0,
           'gameplay_writes': 0, 'children': children,
           'limits': ['Iterative retrospective replays on previously inspected reports, not held-out evaluation.',
                      'Correct citations and names do not establish gameplay meaning; irrelevant settings still pass.',
                      'No automatic semantic promotion, experiment planning, or game generalization credit.',
                      'Fixture stage-one selection abstains before its same-object text reaches stage two.',
                      'Cancellation verified during pending review; not claimed to interrupt a running HTTP call.',
                      'v3 native attempt failed: real job payload was 263 bytes, above its 256-byte limit. v4 binds the model out of line.']}
Path(__file__).with_name('results.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: v for k, v in summary.items() if k not in ('frozen', 'replays', 'children')}, indent=2))
