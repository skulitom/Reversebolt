"""Audit the saved-catalog replay; no model or target process is invoked."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT/'test-results/type-selection-pages-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT/'frozen.json')
for name, digest in frozen['sources'].items():
    assert sha(OUT/'source'/name) == digest
assert sha(Path(frozen['native_report'])) == frozen['native_report_sha256']
assert sha(Path(frozen['evaluator'])) == frozen['evaluator_sha256']
receipt = read(OUT/'replay/receipt.json')
inventory = read(OUT/'replay/input.json')['inventory']
assert receipt['report_sha256'] == inventory['report_sha256'] == frozen['native_report_sha256']
assert receipt['error'] is None and not receipt['cancelled'] and receipt['activity'] is None
assert receipt['model_digest'] == '22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643'
policy = receipt['paged_selection']
assert policy['policy'] == frozen['policy'] and policy['prompt_bytes'] == 12000
assert len(policy['rounds']) <= 4
original = {t['id']:t for t in inventory['types']}
original_rows = {}
expected = set(original)
selection_stages = []
for number, round in enumerate(policy['rounds']):
    seen, nominated = set(), set()
    for page in round['pages']:
        stage = receipt['stages'][page['stage_index']]
        assert len(stage['prompt'].encode()) <= 12000
        supplied = json.loads(stage['prompt'].split('\nObserved types and path-scoped text examples: ',1)[1])
        assert [r['id'] for r in supplied['types']] == page['type_ids']
        for row in supplied['types']:
            i = row['id']; assert i in expected and i not in seen; seen.add(i)
            source = original[i]
            assert row['field_labels'] == [f['name'] for f in source['fields']]
            assert row['name'] == source['key']['type_name'][:120]
            if number == 0:
                original_rows[i] = row
                for sample in row.get('object_samples',[]):
                    candidates = [s for f in source['fields'] if f['name'] == sample['field_name'] for s in f['samples'] if s['path_id'] == sample['path_id']]
                    assert any(all(text in [h['text'] for h in s['context'] if not h['truncated']] for text in sample['object_text']) for s in candidates)
            else:
                assert row == original_rows[i]
        selected = json.loads(stage['response']['message']['content'])['types']
        assert selected == page['selected'] and len(selected) <= 8 and len(set(selected)) == len(selected)
        assert set(selected).issubset(page['type_ids'])
        nominated.update(selected)
        selection_stages.append(page['stage_index'])
    assert seen == expected
    expected = nominated
assert len(selection_stages) <= 24 and selection_stages == list(range(len(selection_stages)))
assert set(receipt['selection']['selected_types']) == expected and len(expected) <= 8
for stage in receipt['stages']:
    assert hashlib.sha256(stage['prompt'].encode()).hexdigest() == stage['prompt_sha256']
    assert stage['response']['done'] and stage['response']['model'] == receipt['model']
    assert stage['response']['done_reason'] != 'length'
    assert stage['response']['prompt_eval_count'] <= stage['options']['num_ctx'] - stage['options']['num_predict'] - 512
rows = {r['id']:r for r in receipt['selection']['rows']}
questions = []
for hypothesis in receipt['batch']['accepted']:
    candidate = hypothesis['candidate']
    assert candidate == rows[hypothesis['proposal']['candidate']]
    assert hypothesis['alternatives'] == [rows[i] for i in hypothesis['proposal']['alternatives']]
    for citation in hypothesis['proposal']['citations']:
        text = candidate[citation['source']]
        if citation['source'] == 'object_text': text = text[citation['index']]
        assert text == citation['quote']
    questions.append({'name':hypothesis['proposal']['name'],'field':candidate['field_name'],'alternatives':len(hypothesis['alternatives'])})
result = {'artifact_audit':'passed','native_trial':'unchanged_failed_baseline','development_replay':True,
    'types_covered_once_in_first_round':len(original),'complete_field_labels':sum(len(t['fields']) for t in inventory['types']),
    'pages_per_round':[len(r['pages']) for r in policy['rounds']], 'selection_model_calls':len(selection_stages),
    'total_model_calls':len(receipt['stages']),'summed_stage_ms':sum(s['elapsed_ms'] for s in receipt['stages']),
    'slowest_stage_ms':max(s['elapsed_ms'] for s in receipt['stages']),
    'max_selection_prompt_bytes':max(len(receipt['stages'][i]['prompt'].encode()) for i in selection_stages),
    'selected_types':receipt['selection']['selected_types'],'questions':questions,'rejected':len(receipt['batch']['rejected']),
    'frozen_manifest_sha256':sha(OUT/'frozen.json'),'receipt_sha256':sha(OUT/'replay/receipt.json'),
    'target_access':False,'new_images':0,'gameplay_edits':0,'verified_controls':0,
    'limitation':'Six retained label-grounded hypotheses are not semantic or causal verification; no new independent game acceptance.'}
(ROOT/'research/independent-viewfinder/paging-results.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:result[k] for k in ['artifact_audit','types_covered_once_in_first_round','pages_per_round','total_model_calls','summed_stage_ms','max_selection_prompt_bytes']}))
