"""Audit fixed saved-report model runs; this does not verify gameplay meanings."""
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/bounded-type-selection-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'revised-frozen.json')
assert sha(OUT / frozen['binary']) == frozen['binary_sha256']
for name, digest in frozen['sources'].items():
    assert sha(OUT / 'revised-source' / name) == digest, name
    if '--verify-current' in sys.argv:
        assert sha(ROOT / name) == digest, name

def answer(stage):
    assert hashlib.sha256(stage['prompt'].encode()).hexdigest() == stage['prompt_sha256']
    response = stage['response']
    assert response['done_reason'] == 'stop'
    assert response['model'] == frozen['model']['name']
    assert response['prompt_eval_count'] > 0
    assert response['prompt_eval_count'] + stage['options']['num_predict'] < stage['options']['num_ctx']
    assert response['message']['role'] == 'assistant' and not response['message'].get('tool_calls')
    return json.loads(response['message']['content'])

def complete_page(stage, observed, prior_rows):
    page = json.loads(stage['prompt'].split('\nObserved types and path-scoped text examples: ', 1)[1])
    for row in page['types']:
        source = observed[row['id']]
        assert row['name'] == source['key']['type_name'][:120]
        assert row['name_shortened'] == (len(source['key']['type_name']) > 120)
        assert row['field_labels'] == [f['name'] for f in source['fields']]
        for sample in row.get('object_samples', []):
            matching = [s for f in source['fields'] if f['name'] == sample['field_name']
                        for s in f['samples'] if s['path_id'] == sample['path_id']]
            assert len(matching) == 1
            text = list(dict.fromkeys(h['text'] for h in matching[0]['context']
                if not h['truncated'] and len(h['text'].encode()) <= 256 and h['text'].strip()))
            assert sample['object_text'] == text
        if row['id'] in prior_rows:
            assert prior_rows[row['id']] == row
        prior_rows[row['id']] = row
    return page

cases = []
for case in frozen['cases']:
    assert sha(Path(case['source'])) == case['sha256']
    assert sha(Path(case['baseline_receipt'])) == case['baseline_sha256']
    baseline = read(Path(case['baseline_receipt']))
    process = read(OUT / 'revised' / (case['name'] + '-process.json'))
    assert process['exit_code'] == 0
    directory = OUT / 'revised' / case['name']
    receipt = read(directory / 'receipt.json')
    inputs = read(directory / 'input.json')
    inventory = inputs['inventory']
    assert inputs['source'] == case['source'] and inputs['activity_manifest'] is None
    assert inputs['opaque_research_only'] == ('--opaque' in case.get('flags', []))
    if case['name'] == 'vampire':
        original = Path(case['original_inventory'])
        assert sha(original) == frozen['original_inventory_sha256']
        assert inventory == read(original)
    assert receipt['report_sha256'] == inventory['report_sha256']
    assert receipt['model_digest'] == frozen['model']['digest'] == baseline['model_digest']
    assert not receipt['cancelled'] and receipt['activity'] is None
    assert not receipt.get('incomplete_stage')
    observed = {t['id']: t for t in inventory['types']}
    expected = list(observed)
    rows, rounds, used = {}, [], []
    paged = receipt.get('paged_selection')
    if paged:
        assert paged['policy'] == 'complete_type_pages_with_quotas_v2'
        assert paged['prompt_bytes'] == 12000 and len(paged['rounds']) <= 4
        for round_ in paged['rounds']:
            ids = [i for page in round_['pages'] for i in page['type_ids']]
            assert ids == expected
            union = []
            for item in round_['pages']:
                used.append(item['stage_index'])
                stage = receipt['stages'][item['stage_index']]
                page = complete_page(stage, observed, rows)
                assert [t['id'] for t in page['types']] == item['type_ids']
                assert len(stage['prompt'].encode()) <= 12000
                quota = item['max_types']
                assert 1 <= quota <= 8
                assert stage['prompt'].startswith(f'This page permits at most {quota} type IDs.')
                assert stage['schema']['properties']['types']['maxItems'] == quota
                selected = answer(stage)['types']
                assert selected == item['selected']
                assert len(selected) == len(set(selected)) <= quota
                assert set(selected) <= set(item['type_ids'])
                union.extend(selected)
            assert len(union) == len(set(union))
            quotas = [p['max_types'] for p in round_['pages']]
            if len(quotas) <= 8:
                assert sum(quotas) == min(len(ids), 8)
            rounds.append({'types': len(ids), 'pages': len(quotas), 'quotas': quotas, 'nominated': len(union)})
            expected = sorted(union)
        assert used == list(range(len(used))) and len(used) <= 24
    else:
        stage = receipt['stages'][0]
        page = complete_page(stage, observed, rows)
        assert [t['id'] for t in page['types']] == expected
        expected = answer(stage)['types']
        assert len(expected) == len(set(expected)) <= 8 and set(expected) <= set(observed)
        used = [0]
    selection = receipt['selection']
    if selection is not None:
        assert selection['selected_types'] == expected
        assert selection['report_sha256'] == receipt['report_sha256']
    for stage in receipt['stages'][len(used):]:
        proposals = answer(stage)['hypotheses']
        assert len(proposals) <= stage['schema']['properties']['hypotheses']['maxItems']
        assert stage['schema']['properties']['hypotheses']['maxItems'] in (1, 2)
    batch = receipt['batch']
    if receipt['error']:
        assert batch is None
    else:
        assert batch is not None and selection is not None
        assert batch['report_sha256'] == receipt['report_sha256']
        assert len(batch['accepted']) + len(batch['rejected']) <= 12
        for item in batch['accepted']:
            assert item['candidate'] in selection['rows']
            assert all(a in selection['rows'] for a in item['alternatives'])
    cases.append({'case': case['name'], 'inventory_types': len(inventory['types']),
        'selection_rounds': rounds, 'selection_model_calls': len(used),
        'total_model_calls': len(receipt['stages']), 'error': receipt['error'],
        'selected_types': None if selection is None else selection['selected_types'],
        'accepted': None if batch is None else len(batch['accepted']),
        'rejected': None if batch is None else len(batch['rejected']),
        'questions': [] if batch is None else [{'name': h['proposal']['name'],
            'question': h['proposal']['question'], 'field': h['candidate']['field_name'],
            'type': h['candidate']['type_name']} for h in batch['accepted']],
        'rejections': [] if batch is None else batch['rejected'],
        'baseline_error': baseline['error'], 'baseline_model_calls': len(baseline['stages']),
        'baseline_accepted': None if baseline['batch'] is None else len(baseline['batch']['accepted']),
        'baseline_rejected': None if baseline['batch'] is None else len(baseline['batch']['rejected']),
        'seconds': process['seconds'], 'receipt_sha256': sha(directory / 'receipt.json')})

tests = (ROOT / 'test-results/type-selection-tests.log').read_text(encoding='utf-8-sig')
assert 'FAILED' not in tests
test_count = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', tests)))
assert test_count >= 497
for name in ['clippy', 'build']:
    log = (ROOT / f'test-results/type-selection-{name}.log').read_text(encoding='utf-8-sig')
    assert 'Finished' in log and 'error:' not in log
preview = read(ROOT / 'test-results/bounded-type-selection-preview-v2/result.json')
catalog = read(ROOT / 'test-results/bounded-type-selection-catalog-v1/result.json')
assert preview['passed'] and preview['process_exited'] and not preview['target_access']
assert catalog['passed'] and catalog['exit_code'] == 0 and catalog['tool_count'] == 65
package = Path(preview['bin'])
assert sha(package / 'Reversebolt.exe') == preview['binary_sha256']
assert sha(package / 'reversebolt-mcp.exe') == catalog['binary_sha256']
assert sha(package / 'reversebolt_runtime.dll') == sha(ROOT / 'dist/0.1.0-20261023-074606/reversebolt_runtime.dll')
summary = {'audit_passed': True, 'progress_percent': 38, 'model': frozen['model'],
    'artifact_root': str(OUT.relative_to(ROOT)), 'frozen_sha256': sha(OUT / 'revised-frozen.json'),
    'workspace_tests': test_count, 'cases': cases, 'packaged_preview': package.name,
    'headless_check': preview, 'mcp_check': catalog, 'unchanged_native_helper': True,
    'limits': ['Saved development reports, not new independent-game evidence.',
        'Page quotas bound the shortlist, but page order and grouping can discard useful candidates.',
        'Accepted questions pass reference/lexical checks; gameplay meaning and effects are unverified.',
        'Older four baselines precede other changes and do not isolate this quota revision.',
        'No target process, input, image review or memory edit was used for this replay.']}
Path(__file__).with_name('results.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'audit_passed': True, 'tests': test_count, 'cases': [
    {k: c[k] for k in ['case', 'selection_model_calls', 'error', 'accepted', 'rejected']} for c in cases]}))
