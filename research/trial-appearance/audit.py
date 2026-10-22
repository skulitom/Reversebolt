"""Audit unchanged sources/evidence and retain unresolved regional outcomes."""
import hashlib, json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/trial-appearance-v1'

def read(p):
    return json.loads(p.read_text(encoding='utf-8-sig'))

def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

frozen = read(OUT / 'frozen.json')
for p, digest in frozen['sources'].items():
    assert sha(ROOT / p) == sha(OUT / 'source' / p) == digest, p
for p, digest in frozen['inputs'].items():
    assert sha(Path(p)) == digest, p
assert sha(Path(frozen['binary'])) == frozen['binary_sha256']
results = []
for p in sorted(OUT.glob('*-process.json')):
    process = read(p)
    assert process['exit_code'] == 0
    result = read(Path(process['output']))
    original = read(Path(process['record']))
    assert result['original_assessment'] == original['assessment']
    assert not result['target_access'] and not result['input_sent']
    assert result['new_captures'] == result['field_writes'] == result['model_calls'] == 0
    wrapper = result['trial_appearance_comparison']
    report = wrapper['report']
    if report:
        assert report['attempted_frames'] <= report['maximum_attempted_frames'] == 18
        assert report['record_sha256'] == result['trial_motion_comparison']['record_sha256']
        assert report['selection'] == read(Path(process['selection']))
    results.append({'test_id': original['id'], 'frame_counts': [len(c['frames']) for c in original['conditions']],
        'appearance_status': report['status'] if report else None, 'appearance_error': wrapper['error'],
        'attempted_frames': report['attempted_frames'] if report else 0,
        'match_statuses': [[f['measurement']['status'] if f['measurement'] else
                           'stopped' if f['stopped'] else 'source_error' for f in c['frames']]
                          for c in report['conditions']] if report else [],
        'global_status': result['trial_motion_comparison']['status'],
        'original_assessment': original['assessment'], 'seconds': process['seconds'],
        'output_sha256': sha(Path(process['output']))})
audit = {'frozen_manifest_sha256': sha(OUT / 'frozen.json'), 'source_files': len(frozen['sources']),
         'bound_input_files': len(frozen['inputs']), 'provenance': 'OwnedPathFixture',
         'model_calls': 0, 'native_target_access': False, 'results': results}
with (OUT / 'audit.json').open('x', encoding='utf-8') as f:
    f.write(json.dumps(audit, indent=2) + '\n')
print(json.dumps(audit, indent=2))
