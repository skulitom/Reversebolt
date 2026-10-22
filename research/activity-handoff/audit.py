"""Audit the failed attempts and final replay without inference or native access."""
import hashlib
import json
from pathlib import Path
import re
import struct

ROOT = Path(__file__).resolve().parents[2]
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

attempts = []
for version in ['v1', 'v2', 'v3']:
    out = ROOT / ('test-results/activity-handoff-' + version)
    frozen = read(out / 'frozen.json')
    for name, digest in frozen['sources'].items():
        assert sha(out / 'source' / name) == digest
    for path, digest in frozen['inputs'].items():
        assert sha(Path(path)) == digest
    process = read(out / 'process.json')
    assert process['exit_code'] == (0 if version == 'v3' else 1)
    if version != 'v3':
        assert 'Recomputed play evidence changed' in process['stderr']
    if version == 'v2':
        archived = read(out / 'binary-preservation.json')
        assert sha(Path(archived['path'])) == archived['sha256'] == frozen['binary_sha256']
    if version == 'v3':
        assert sha(Path(frozen['binary'])) == frozen['binary_sha256']
    attempts.append({'version':version,'exit_code':process['exit_code'],'frozen_manifest_sha256':sha(out / 'frozen.json'),
                     'binary_bytes_reverified':version != 'v1'})

prior = read(ROOT / 'test-results/measured-contrast-v1/replay.json')
diagnostic = read(ROOT / 'test-results/activity-handoff-v2/replay.json')
assert diagnostic['actual'] == prior['evidence']
actual = diagnostic['actual']['actions'][1]['median_difference']
parsed = diagnostic['expected']['actions'][1]['median_difference']
assert struct.unpack('>Q', struct.pack('>d', actual))[0] - struct.unpack('>Q', struct.pack('>d', parsed))[0] == 1
expected = diagnostic['expected']
expected['actions'][1]['median_difference'] = actual
assert expected == diagnostic['actual']
r = read(ROOT / 'test-results/activity-handoff-v3/replay.json')
old = prior['revised_review']
context = r['agent_review']['play_context']
assert context['source'] == old['play_context']['source']
assert context['observation'] == old['play_context']['observation']
assert not context['observation']['consistent_visible_change']
assert context['interpretation_source'] == {'review_id':old['evidence_id'],'model':old['model']}
assert r['activity_transfer']['retained'] and r['activity_transfer']['from_review_id'] == old['evidence_id']
assert r['agent_review']['model'] == 'agent:offline-replay' and r['agent_review']['evidence_id'] != old['evidence_id']
assert r['agent_review']['batch'] == old['batch']
assert r['prior_submission_context'] is None
assert r['observed_actions'] == prior['plan']['observed_actions']
assert len(r['choices']) == len(prior['plan']['choices']) == len(r['test_origins']) == 1
for choice, origin, previous in zip(r['choices'],r['test_origins'],prior['plan']['choices']):
    assert choice['hypothesis_ref']['review_id'] == r['agent_review']['evidence_id']
    assert origin['reference'] == choice['hypothesis_ref'] and origin['play_context'] == context
    choice['hypothesis_ref']['review_id'] = previous['hypothesis_ref']['review_id']
    assert choice == previous
assert r['roundtrip_actions_equal'] and not r['changed_image_transfer']['retained']
assert r['new_model_calls'] == r['field_writes'] == 0 and not r['target_access'] and not r['input_sent']
logpath = ROOT / 'test-results/activity-handoff-final-tests.log'
log = logpath.read_text(encoding='utf-8-sig')
tests = sum(map(int,re.findall(r'test result: ok\. (\d+) passed',log)))
assert tests == 299 and 'test result: FAILED' not in log
result = {'artifact_audit_passed':True,'attempts':attempts,'image_inputs_rehashed':len(r['input_hashes']['images']),
          'hypotheses_reused':len(r['agent_review']['batch']['accepted']),'comparison_choices':len(r['choices']),
          'observed_hold_ms':[a['hold_ms'] for a in r['observed_actions']['actions']],
          'original_interpretation_preserved':True,'original_source_preserved':True,'stale_context_discarded':True,
          'numeric_roundtrip_fixed_without_tolerance':True,'workspace_tests_passed':tests,'test_log_sha256':sha(logpath),
          'new_model_calls':0,'target_access':False,'field_edits':0,
          'limitations':['Saved development evidence, not a new native run or independent-game acceptance.',
                         'The agent proposals are replayed existing questions, not new inference.',
                         'The proposed comparison is unexecuted and its gameplay meaning unverified.',
                         'V1 binary bytes are unavailable; frozen source, hash and failure remain.',
                         'Other historical JSON identities were not exhaustively audited.',
                         'The desktop remains quarantined; no replacement build was published.']}
(ROOT / 'research/activity-handoff/results.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'audit':'passed','attempts':3,'choices':1,'tests':tests,'target_access':False}))
