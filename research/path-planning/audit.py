"""Verify frozen inputs, archived executable and exact comparison provenance."""
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/path-planning-v1'

def read(p):
    return json.loads(p.read_text(encoding='utf-8-sig'))

def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

manifest = read(OUT / 'frozen.json')
for name, expected in manifest['sources'].items():
    assert sha(OUT / 'source' / name) == expected, name
for name, expected in manifest['inputs'].items():
    assert sha(Path(name)) == expected, name
assert sha(Path(manifest['binary'])) == manifest['binary_sha256']
assert read(OUT / 'process.json')['exit_code'] == 0
actual = read(OUT / 'replay.json')
baseline = read(ROOT / 'test-results/activity-handoff-v3/replay.json')
prior = read(ROOT / 'test-results/measured-contrast-v1/replay.json')
assert actual['agent_review']['batch'] == baseline['agent_review']['batch']
context = actual['agent_review']['play_context']
assert context == baseline['agent_review']['play_context']
assert context['observation']['consistent_visible_change'] is False
assert context['interpretation_source']['model'] == 'qwen3.8:latest'
assert actual['observed_actions'] == baseline['observed_actions'] == prior['plan']['observed_actions']
assert [a['hold_ms'] for a in actual['observed_actions']['actions']] == [198] * 4
assert len(actual['input_hashes']['images']) == 15
assert actual['input_hashes'] == baseline['input_hashes']
assert actual['roundtrip_actions_equal'] and not actual['changed_image_transfer']['retained']
assert actual['new_model_calls'] == actual['field_writes'] == 0
assert actual['target_access'] is False and actual['input_sent'] is False
questions = actual['agent_review']['batch']['accepted']
assert len(questions) == 2
assert len(baseline['choices']) == 1 and len(actual['choices']) == 2
expected_rows = [questions[0]['candidate'], questions[0]['alternatives'][0]]
assert [r['field_name'] for r in expected_rows] == ['walkingSpeed', 'jumpSpeed']
for i, (choice, row, origin) in enumerate(zip(actual['choices'], expected_rows, actual['test_origins'], strict=True)):
    ref = choice['hypothesis_ref']
    assert ref['review_id'] == actual['agent_review']['evidence_id']
    assert ref['hypothesis_path_id'] == questions[0]['candidate']['path_id']
    assert ref['selected_path_id'] == row['path_id']
    assert choice['selection_role'] == ('primary' if i == 0 else 'alternative')
    assert choice['selected_candidate'] == row
    assert choice['question_refs'] == [ref]
    assert choice['value'] == '0' and choice['status'] == 'untried'
    assert choice['actions'] == actual['observed_actions']['actions']
    for key in ['name', 'explanation', 'question']:
        assert choice[key] == questions[0]['proposal'][key]
    assert origin['reference'] == ref and origin['hypothesis'] == questions[0]
    assert origin['play_context'] == context
first = {key: actual['choices'][0][key] for key in baseline['choices'][0]}
first['hypothesis_ref'] = dict(first['hypothesis_ref'], review_id=baseline['agent_review']['evidence_id'])
assert first == baseline['choices'][0]
tests = ROOT / 'test-results/path-planning-tests.log'
log = tests.read_text(encoding='utf-8-sig')
counts = [int(n) for n in re.findall(r'test result: ok\. (\d+) passed', log)]
assert sum(counts) == 302 and 'test result: FAILED' not in log
summary = {'frozen_manifest_sha256': sha(OUT / 'frozen.json'), 'replay_sha256': sha(OUT / 'replay.json'),
           'archived_binary_verified': True, 'retained_questions': len(questions),
           'prior_choices': len(baseline['choices']), 'current_choices': len(actual['choices']),
           'selected_fields': [r['field_name'] for r in expected_rows],
           'images_rehashed_and_measurements_recomputed': 15, 'original_ambiguous_interpretation_preserved': True,
           'new_model_calls': 0, 'target_access': False, 'input_sent': False, 'field_writes': 0,
           'workspace_tests_passed': sum(counts), 'test_log_sha256': sha(tests),
           'limitation': 'Known-case offline regression. Neither proposal was executed; no gameplay effect or new generalization is established.'}
Path(__file__).with_name('results.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps(summary))
