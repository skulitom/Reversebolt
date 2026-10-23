"""Offline integrity/robustness audit; category correctness remains unverified."""
import hashlib
import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-relevance-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
binary = OUT / 'review.exe'
assert sha(binary) == frozen['binary_sha256']
for name, digest in frozen['sources'].items():
    assert sha(OUT / 'source' / name) == digest
    if '--verify-current' in sys.argv:
        assert sha(ROOT / name) == digest
cases = []
path_categories = {}
for case in frozen['cases']:
    source = Path(case['source'])
    assert all(sha(source / name) == digest for name, digest in case['hashes'].items())
    original = read(source / 'receipt.json')
    candidates = [h['candidate'] for h in original['batch']['accepted']]
    expected = json.loads(json.dumps(candidates))
    if case['variation'] == 'reversed':
        expected.reverse()
    elif case['variation'] == 'opaque':
        for row in expected:
            row.update(type_name='Container', field_name=f"F{row['id']:010x}", object_text=[])
    directory = OUT / case['name']
    inputs = read(directory / 'input.json')
    assert inputs['source'] == case['source'] and inputs['variation'] == case['variation']
    assert inputs['input']['candidates'] == expected
    assert inputs['input']['report_sha256'] == original['report_sha256']
    receipt = read(directory / 'receipt.json')
    assert receipt['model'] == frozen['model']['name'] and receipt['model_digest'] == frozen['model']['digest']
    assert not receipt['cancelled']
    process = read(OUT / (case['name'] + '-process.json'))
    assert process['exit_code'] == 0
    stage = receipt['stage']
    if stage:
        assert expected
        assert hashlib.sha256(stage['prompt'].encode()).hexdigest() == stage['prompt_sha256']
        raw = json.loads(stage['prompt'].split('\nObserved candidates: ', 1)[1])
        assert raw == [{'candidate': i, 'type_name': r['type_name'], 'field_name': r['field_name'],
                       'object_text': r['object_text']} for i, r in enumerate(expected)]
        assert len(stage['prompt'].encode()) <= 12000
        assert stage['schema']['properties']['judgments']['minItems'] == len(expected)
        assert stage['schema']['properties']['judgments']['maxItems'] == len(expected)
        assert stage['options'] == {'temperature': 0, 'seed': 42, 'num_ctx': 16384,
                                    'num_predict': 128 + len(expected) * 240}
    else:
        assert not expected
    checked = None
    assessment = receipt['assessment']
    if assessment is not None:
        assert receipt['error'] is None
        checked = subprocess.run([str(binary), 'check', str(directory)], capture_output=True,
                                 encoding='utf-8', timeout=15, creationflags=subprocess.CREATE_NO_WINDOW)
        assert checked.returncode == 0, checked.stderr
        verification = json.loads(checked.stdout)
        assert verification['checked'] and verification['model_calls'] == 0 and not verification['target_access']
        assert verification['assessment'] == assessment
        if stage:
            response = stage['response']
            assert response['done'] and response['done_reason'] == 'stop'
            assert 0 < response['prompt_eval_count'] <= 16384 - (128 + len(expected) * 240) - 512
            raw = json.loads(response['message']['content'])
            assert sorted(raw['judgments'], key=lambda j: j['candidate']) == assessment['judgments']
        judgments = []
        for judgment in assessment['judgments']:
            row = expected[judgment['candidate']]
            for citation in judgment['citations']:
                quoted = row[citation['source']] if citation['index'] is None else row['object_text'][citation['index']]
                assert citation['quote'] == quoted
            judgments.append({'path_id': row['path_id'], 'field': row['field_name'], 'type': row['type_name'], **judgment})
        path_categories[case['name']] = {j['path_id']: j['category'] for j in judgments}
    else:
        assert receipt['error'] is not None
        judgments = []
    cases.append({'case': case['name'], 'candidate_count': len(expected), 'model_calls': int(stage is not None),
                  'error': receipt['error'], 'categories': dict(Counter(j['category'] for j in judgments)),
                  'judgments': judgments, 'seconds': process['seconds'],
                  'receipt_sha256': sha(directory / 'receipt.json'), 'offline_checked': checked is not None})
orders = {}
for case in ['latest', 'menu']:
    original, reversed_ = [path_categories.get(case + '-' + v) for v in ['original', 'reversed']]
    orders[case] = None if original is None or reversed_ is None else {
        'same_paths': set(original) == set(reversed_),
        'changed_categories': [{'path_id': p, 'original': c, 'reversed': reversed_.get(p)}
                               for p, c in original.items() if reversed_.get(p) != c]}
tests = (ROOT / 'test-results/runtime-relevance-tests.log').read_text(encoding='utf-8-sig')
assert 'FAILED' not in tests
count = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', tests)))
assert count == 149
for name in ['clippy', 'build']:
    log = (ROOT / f'test-results/runtime-relevance-{name}.log').read_text(encoding='utf-8-sig')
    assert 'Finished' in log and 'error:' not in log
summary = {'artifact_audit_passed': True, 'model': frozen['model'], 'cases': cases,
           'order_comparisons': orders, 'core_observer_tests': count, 'progress_percent': 38,
           'desktop_changed': False, 'production_filter_enabled': False,
           'frozen_manifest_sha256': sha(OUT / 'frozen.json'),
           'limitations': ['Known development candidates, not independent or held-out semantic validation.',
               'The same model generates and classifies. Agreement does not establish truth.',
               'Category labels and exact citations do not establish liveness, owner, effect or edit authority.',
               'Unknown candidates are retained and no production candidates are removed.',
               'Reordered and opaque variants are synthetic robustness checks, not gameplay evidence.']}
Path(__file__).with_name('results.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'audit_passed': True, 'cases': [{k: c[k] for k in ['case', 'error', 'categories']} for c in cases],
                  'order_comparisons': orders}))
