"""Recheck frozen evidence without rerunning a program or reading game state."""
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/file-export-detection-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

frozen = read(OUT / 'frozen.json')
for name, digest in frozen['sources'].items():
    assert sha(OUT / 'source' / name) == digest
assert sha(Path(frozen['binary'])) == frozen['binary_sha256']
process = read(OUT / 'process.json')
assert process['exit_code'] == 0 and not process['stderr']
rows = [json.loads(line) for line in process['stdout'].splitlines()]
assert len(rows) == len(frozen['cases']) == 8
for case, row in zip(frozen['cases'], rows):
    assert sha(Path(case['path'])) == case['sha256']
    value = row['inspection']
    assert value['path'] == case['path'] and value['runtime_provider'] == case['expected_runtime']
    exports = value['exports']
    assert ('ReverseboltInspect' in exports['functions']) == case['expected_helper']
    if case['expected_runtime']:
        assert len(exports['functions']) == 39
    assert not exports['forwarded'] and 0 < exports['read_bytes'] <= 8 * 1024 * 1024
    assert not row['target_access'] and not row['dll_mapping']
result = read(ROOT / 'research/file-export-detection/results.json')
assert result['frozen_manifest_sha256'] == sha(OUT / 'frozen.json')
assert result['process_record_sha256'] == sha(OUT / 'process.json')
log = (ROOT / 'test-results/pe-exports-workspace-tests.log').read_text(encoding='utf-8-sig')
passed = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', log)))
assert passed == 296 and 'test result: FAILED' not in log
print(json.dumps({'audit': 'passed', 'file_cases': len(rows), 'workspace_tests': passed,
                  'native_integration_verified': False, 'windows_detection_resolved': False}))
