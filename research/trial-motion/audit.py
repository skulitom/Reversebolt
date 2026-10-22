"""Audit retained outputs and unchanged originals without rerunning the analyzer."""
import hashlib, json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/trial-motion-v1'

def read(p):
    return json.loads(p.read_text(encoding='utf-8-sig'))

def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

frozen = read(OUT / 'frozen.json')
for name, expected in frozen['sources'].items():
    assert sha(OUT / 'source' / name) == expected
    assert sha(ROOT / name) == expected
for name, expected in frozen['inputs'].items():
    assert sha(Path(name)) == expected
assert sha(Path(frozen['binary'])) == frozen['binary_sha256']
results = []
for run in read(OUT / 'processes.json'):
    assert run['exit_code'] == 0
    record = read(Path(run['record']))
    output = read(Path(run['output']))
    assert output['record_file_sha256'] == sha(Path(run['record']))
    assert output['original_assessment'] == record['assessment']
    assert output['target_access'] is False and output['model_calls'] == 0
    motion = output['trial_motion_comparison']
    assert motion['scope'] == 'retained_trial_conditions'
    counts = [len(c['frames']) for c in record['conditions']]
    assert [len(c['pairs']) for c in motion['conditions']] == [max(0, n-1) for n in counts]
    pairs = [p for c in motion['conditions'] for p in c['pairs']]
    results.append({'test_id': record['id'], 'provenance': 'OwnedPathFixture; not an independent game',
        'frames': counts, 'original_assessment': record['assessment'], 'motion_status': motion['status'],
        'measurement_errors': sum(p['measurement_error'] is not None for p in pairs),
        'input_errors': sum(p['input_error'] is not None for p in pairs),
        'selected': [p['motion']['selected'] if p['motion'] else None for p in pairs],
        'consensus_pairs': sum(p['motion'] is not None and p['motion']['displacement'] is not None for p in pairs),
        'output_sha256': sha(Path(run['output'])), 'seconds': run['seconds']})
assert len(results) == 3
summary = {'artifact_root': str(OUT.relative_to(ROOT)), 'frozen_sha256': sha(OUT/'frozen.json'),
    'source_files_verified': len(frozen['sources']), 'original_files_unchanged': len(frozen['inputs']), 'results': results,
    'new_captures': 0, 'model_calls': 0, 'target_access': False, 'field_writes': 0,
    'limitation': 'Known owned-fixture regression. No new verified gameplay control or independent-game acceptance.'}
(Path(__file__).with_name('results.json')).write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps(summary,indent=2))
