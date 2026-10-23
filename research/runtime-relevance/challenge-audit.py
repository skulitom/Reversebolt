"""Audit fixed synthetic challenges without turning an authored rubric into ground truth."""
import copy
import hashlib
import json
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-relevance-challenge-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
assert sha(OUT / 'review.exe') == frozen['binary_sha256']
prior_path = ROOT / 'test-results/runtime-relevance-v1/frozen.json'
assert sha(prior_path) == frozen['prior_manifest_sha256']
prior = read(prior_path)
for name, digest in frozen['sources'].items():
    assert sha(OUT / 'source' / name) == digest
    if '--verify-current' in sys.argv:
        assert sha(ROOT / name) == digest
for name in frozen['unchanged_from_prior']:
    assert frozen['sources'][name] == prior['sources'][name]
assert len(frozen['cases']) == 11 and len(frozen['rubric']) == 32
all_categories, cases, process_ids = {}, [], []
for case in frozen['cases']:
    source = Path(case['source'])
    assert sha(source) == case['source_sha256']
    saved = read(source)
    assert saved['synthetic'] is True
    expected = [copy.deepcopy(frozen['rubric'][key]['row']) for key in case['keys']]
    if case['attack']:
        expected[0]['object_text'].append(case['attack'])
    assert saved['input']['candidates'] == expected and len(expected) == 8
    directory = OUT / case['name']
    inputs = read(directory / 'input.json')
    assert inputs['synthetic'] is True and inputs['variation'] == 'synthetic'
    assert inputs['source'] == case['source'] and inputs['input'] == saved['input']
    process = read(OUT / (case['name'] + '-process.json'))
    assert process['exit_code'] == 0
    process_ids.append(process['pid'])
    receipt = read(directory / 'receipt.json')
    assert receipt['model'] == frozen['model']['name'] and receipt['model_digest'] == frozen['model']['digest']
    assert not receipt['cancelled'] and receipt['stage'] is not None
    stage = receipt['stage']
    assert hashlib.sha256(stage['prompt'].encode()).hexdigest() == stage['prompt_sha256']
    raw = json.loads(stage['prompt'].split('\nObserved candidates: ', 1)[1])
    assert raw == [{'candidate': i, 'type_name': row['type_name'], 'field_name': row['field_name'],
                   'object_text': row['object_text']} for i, row in enumerate(expected)]
    assert len(stage['prompt'].encode()) <= 12000
    assert stage['options'] == {'temperature': 0, 'seed': 42, 'num_ctx': 16384, 'num_predict': 2048}
    assert stage['schema']['properties']['judgments']['minItems'] == stage['schema']['properties']['judgments']['maxItems'] == 8
    judgments, citation_errors = [], []
    if receipt['assessment'] is not None:
        assert receipt['error'] is None
        check = subprocess.run([str(OUT / 'review.exe'), 'check', str(directory)], capture_output=True,
                               encoding='utf-8', timeout=15, creationflags=subprocess.CREATE_NO_WINDOW)
        assert check.returncode == 0, check.stderr
        checked = json.loads(check.stdout)
        assert checked['checked'] and checked['model_calls'] == 0 and not checked['target_access']
        assert checked['assessment'] == receipt['assessment']
        response = stage['response']
        assert response['done'] and response['done_reason'] == 'stop'
        assert 0 < response['prompt_eval_count'] <= 16384 - 2048 - 512
        raw = json.loads(response['message']['content'])['judgments']
        assert sorted(raw, key=lambda j: j['candidate']) == receipt['assessment']['judgments']
        all_categories[case['name']] = {}
        for judgment in receipt['assessment']['judgments']:
            key = case['keys'][judgment['candidate']]
            row = expected[judgment['candidate']]
            for citation in judgment['citations']:
                quoted = row[citation['source']] if citation['index'] is None else row['object_text'][citation['index']]
                assert citation['quote'] == quoted
            intended = frozen['rubric'][key]['category']
            actual = judgment['category']
            all_categories[case['name']][key] = actual
            judgments.append({'key': key, 'intended': intended, 'actual': actual, 'reason': judgment['reason'],
                'citations': judgment['citations'], 'rubric_disagreement': intended != actual,
                'would_exclude_a_retained_lead': intended in ['gameplay', 'unclear'] and
                    actual in ['application_configuration', 'implementation_detail']})
    else:
        assert receipt['error'] is not None
        response = stage['response']
        if response.get('done') and response.get('done_reason') == 'stop':
            for judgment in json.loads(response['message']['content'])['judgments']:
                row = expected[judgment['candidate']]
                for citation in judgment['citations']:
                    observed = None
                    if citation['source'] in ['field_name', 'type_name'] and citation['index'] is None:
                        observed = row[citation['source']]
                    elif citation['source'] == 'object_text' and isinstance(citation['index'], int):
                        if 0 <= citation['index'] < len(row['object_text']):
                            observed = row['object_text'][citation['index']]
                    if observed != citation['quote']:
                        citation_errors.append({'candidate': judgment['candidate'], 'key': case['keys'][judgment['candidate']], 'citation': citation})
    cases.append({'case': case['name'], 'error': receipt['error'], 'seconds': process['seconds'],
                  'categories': dict(Counter(j['actual'] for j in judgments)), 'judgments': judgments,
                  'response_complete': stage['response'].get('done_reason') == 'stop', 'citation_errors': citation_errors,
                  'receipt_sha256': sha(directory / 'receipt.json')})

def changes(before, after):
    a, b = all_categories.get(before), all_categories.get(after)
    if a is None or b is None:
        return None
    assert set(a) == set(b)
    return [{'key': key, 'before': value, 'after': b[key]} for key, value in a.items() if value != b[key]]
order = {str(i): changes(f'balanced-{i}', f'reversed-{i}') for i in range(4)}
attacks = {name: changes('attack-baseline', name) for name in ['attack-promote-all', 'attack-exclude-this']}
baseline = [j for c in cases if c['case'].startswith('balanced-') for j in c['judgments']]
retention_misses = [{'case': c['case'], 'key': j['key'], 'intended': j['intended'], 'actual': j['actual']}
                    for c in cases for j in c['judgments'] if j['would_exclude_a_retained_lead']]
overclaims = [{'case': c['case'], 'key': j['key'], 'actual': j['actual']}
              for c in cases for j in c['judgments'] if j['intended'] == 'unclear' and j['actual'] != 'unclear']
for kind in ['clippy', 'build']:
    log = (ROOT / f'test-results/runtime-relevance-challenge-{kind}.log').read_text(encoding='utf-8-sig')
    assert 'Finished' in log and 'error:' not in log
summary = {'artifact_audit_passed': True, 'authored_rubric_not_game_truth': True,
    'frozen_manifest_sha256': sha(OUT / 'frozen.json'), 'model': frozen['model'], 'cases': cases,
    'review_failures': [c['case'] for c in cases if c['error']],
    'incomplete_responses': [c['case'] for c in cases if not c['response_complete']],
    'baseline_judgments': len(baseline), 'baseline_exact_rubric_matches': sum(not j['rubric_disagreement'] for j in baseline),
    'baseline_retention_misses': sum(j['would_exclude_a_retained_lead'] for j in baseline),
    'all_retention_misses': retention_misses, 'all_ambiguous_overclaims': overclaims,
    'order_changes': order, 'attack_changes': attacks, 'inference_processes_exited': len(process_ids),
    'production_filter_enabled': False, 'progress_percent': 38,
    'limitations': ['Authored development labels and intended roles; not independent gameplay observations.',
        'Exact rubric matches do not measure accuracy on real games or verify meaning, ownership or effects.',
        'The same local model and fixed prompt are evaluated; finite attacks cannot prove prompt-injection robustness.',
        'No game, screenshot, input, memory access, candidate removal or edit occurred.']}
Path(__file__).with_name('challenge-results.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: summary[k] for k in ['artifact_audit_passed', 'review_failures', 'incomplete_responses', 'baseline_judgments',
    'baseline_exact_rubric_matches', 'baseline_retention_misses', 'all_retention_misses', 'order_changes', 'attack_changes']}))
