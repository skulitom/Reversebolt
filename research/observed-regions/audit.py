"""Check the frozen replay and report measurements without claiming motion truth."""
import copy, hashlib, json, struct
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/observed-regions-v1'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f: return hashlib.file_digest(f, 'sha256').hexdigest()
inputs = read(OUT / 'inputs-frozen.json')
assert all(sha(Path(p)) == h for p, h in inputs.items())
frozen = read(OUT / 'source-frozen.json')
assert all(sha(OUT / 'source' / p) == h and sha(ROOT / p) == h for p, h in frozen['sources'].items())
assert all(sha(Path(p)) == h for p, h in frozen['binaries'].items())
result = read(OUT / 'result.json')
assert result['manifest_sha256'] == sha(OUT / 'cases.json')
assert result['new_model_calls'] == 0 and not result['target_access'] and not result['input_sent'] and result['field_writes'] == 0
cases = result['cases']
assert len(cases) == 3
s = cases[0]['selection']
assert s['considered_observations'] == 7 and s['eligible_observations'] == 3
assert s['omitted_observations'] == {'no_following_image': 4}
assert len(s['regions']) == len(cases[0]['measurements']) == 3
manifest = read(OUT / 'cases.json')
before, after = [f['reference'] for f in manifest[0]['frames'][1:3]]
for plan in s['regions']:
    assert plan['before_id'] == before['id'] and plan['after_id'] == after['id']
    for source in plan['sources']:
        original = next(t for t in manifest[0]['visual_tracks'] if t['id'] == source['track_id'])['observations'][1]
        # ObservationInput stores bounds as f32. Compare exact f32 values across
        # the two serializers; do not loosen hashes or other source fields.
        def canonical_observation(value):
            value = copy.deepcopy(value)
            value['input']['bounds'] = [struct.pack('<f', n).hex() for n in value['input']['bounds']]
            return value
        assert canonical_observation(source['observation']) == canonical_observation(original)
assert cases[1]['selection']['considered_observations'] == 0
assert cases[1]['selection']['regions'] == cases[1]['measurements'] == []
assert cases[2]['selection']['omitted_observations'] == {'no_following_image': 4, 'omitted_capture': 3}
assert cases[2]['selection']['regions'] == cases[2]['measurements'] == []
summary = []
for plan in s['regions']:
    row = next(m['result'] for m in cases[0]['measurements'] if m['source_sha256'] == plan['source_sha256'])
    assert row['before_sha256'] == before['sha256'] and row['after_sha256'] == after['sha256']
    report = row['measurement']
    summary.append({'claims': [o['observation']['input']['label'] for o in plan['sources']],
                    'region': plan['region'], 'error': row['error'],
                    'measurement': report['measurement'] if report else None})
report = {'source_manifest_sha256': sha(OUT / 'source-frozen.json'), 'result_sha256': sha(OUT / 'result.json'),
          'unchanged_input_files': len(inputs), 'frozen_sources': len(frozen['sources']),
          'process': read(OUT / 'process.json'), 'selection_assertions_passed': True,
          'measurements': summary, 'motion_accuracy': 'unknown', 'new_gameplay': False}
with (OUT / 'audit.json').open('x', encoding='utf-8') as f: f.write(json.dumps(report, indent=2) + '\n')
print(json.dumps({k:v for k,v in report.items() if k != 'measurements'}))
