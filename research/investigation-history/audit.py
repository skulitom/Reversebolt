"""Audit the disposable history replay and preservation of native source evidence."""
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/investigation-history-v1'

def read(p):
    return json.loads(p.read_text(encoding='utf-8-sig'))

def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

manifest = read(OUT / 'frozen.json')
for name, digest in manifest['sources'].items():
    assert sha(OUT / 'source' / name) == digest, name
for name, digest in manifest['inputs'].items():
    assert sha(Path(name)) == digest, name
assert sha(Path(manifest['binary'])) == manifest['binary_sha256']
assert read(OUT / 'process.json')['exit_code'] == 0
result = read(OUT / 'replay/result.json')
source = Path(manifest['original_record'])
original = read(source)
store = OUT / 'replay/runtime-tests'
assert sha(store / source.name) == sha(source) == result['original_record_sha256']
assert result['original_record_unchanged'] is True
assert result['original_id'] == original['id']
assert original['phase'] == 'complete' and result['original_assessment'] == original['assessment']
assert result['retained_records'] == 9
assert result['images_and_input_receipts_verified'] == 18
assert result['execution_receipts_verified'] == 18
assert result['full_investigation']['investigation_records'] == 8
assert result['full_investigation']['remaining_record_slots'] == 0
assert result['new_investigation']['investigation_records'] == 1
assert result['new_investigation']['remaining_record_slots'] == 7
assert 'eight-comparison' in result['same_investigation_refusal']
assert len(result['synthetic_record_ids']) == 8
assert len(list(store.glob('*.json'))) == 9
other_investigations = []
for identifier in result['synthetic_record_ids']:
    record = read(store / (identifier + '.json'))
    assert record['phase'] == 'stopped'
    assert record['actor'] == 'offline-history-fixture'
    assert record['origin'] is None and record['assessment'] is None and record['preparation'] is None
    assert all(not condition['frames'] for condition in record['conditions'])
    assert not (store / identifier).exists()
    if record['investigation'] != original['investigation']:
        other_investigations.append(record['investigation'])
assert len(other_investigations) == 1
for item in result['copied_files']:
    assert sha(store / original['id'] / item['file']) == item['sha256']
    assert sha(source.with_suffix('') / item['file']) == item['sha256']
assert result['new_model_calls'] == result['field_writes'] == 0
assert result['target_access'] is False and result['input_sent'] is False
tests = ROOT / 'test-results/investigation-history-tests.log'
log = tests.read_text(encoding='utf-8-sig')
count = sum(int(n) for n in re.findall(r'test result: ok\. (\d+) passed', log))
assert count == 306 and 'test result: FAILED' not in log
summary = {'frozen_manifest_sha256': sha(OUT / 'frozen.json'), 'result_sha256': sha(OUT / 'replay/result.json'),
           'original_record_sha256': sha(source), 'original_evidence_files_unchanged': len(result['copied_files']),
           'retained_records': 9, 'synthetic_records': 8, 'images_and_inputs_verified': 18,
           'execution_receipts_verified': 18, 'original_native_assessment_preserved': True,
           'new_model_calls': 0, 'target_access': False, 'input_sent': False, 'field_writes': 0,
           'workspace_tests_passed': count, 'test_log_sha256': sha(tests),
           'limitation': 'Disposable-copy storage regression. Synthetic added records are not experiments; no new gameplay effect, native integration or security clearance was tested.'}
Path(__file__).with_name('results.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps(summary))
