"""Assess analytic truth and retain unknown accuracy on exposed gameplay images."""
import copy, hashlib, json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/region-appearance-v1'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f: return hashlib.file_digest(f, 'sha256').hexdigest()
frozen = read(OUT / 'source-frozen.json')
assert all(sha(OUT / 'source' / p) == h and sha(ROOT / p) == h for p, h in frozen['sources'].items())
assert all(sha(Path(p)) == h for p, h in frozen['binaries'].items())
inputs = read(OUT / 'inputs-frozen.json')
assert all(sha(Path(p)) == h for p, h in inputs.items())
result = read(OUT / 'result.json')
assert result['manifest_sha256'] == sha(OUT / 'cases.json')
assert not result['target_access'] and not result['input_sent'] and result['new_model_calls'] == result['field_writes'] == 0
cases = result['cases']
assert len(cases) == 8
baseline = read(ROOT / 'test-results/observed-regions-v1/result.json')['cases']
for current, prior in zip(cases[5:], baseline, strict=True):
    stripped = copy.deepcopy(current)
    for measurement in stripped['measurements']: measurement.pop('appearance_match')
    assert stripped == prior

def compact(measurement):
    outer = measurement['appearance_match']
    report = outer['measurement']
    if report is None: return {'error': outer['error']}
    return {k: report[k] for k in ['policy', 'status', 'source_region', 'measurement_region',
                                  'source_pixel_scale', 'requirements', 'matched_region', 'displacement', 'forward', 'reverse']}
analytic = []
for case, truth in zip(cases[:5], read(OUT / 'controls/truth.json'), strict=True):
    assert case['name'] == truth['name'] and len(case['measurements']) == 1
    measured = compact(case['measurements'][0])
    expected = truth['expected_displacement']
    passed = (measured.get('status') == 'matched' and measured.get('displacement') == expected) if expected is not None else (
        'error' not in measured and measured['status'] != 'matched' and measured['displacement'] is None)
    analytic.append({'truth': truth, 'passed': passed, 'result': measured})
game = []
for case in cases[5:]:
    rows = []
    for measurement in case['measurements']:
        plan = next(p for p in case['selection']['regions'] if p['source_sha256'] == measurement['source_sha256'])
        rows.append({'observer_claims': [s['observation']['input']['label'] for s in plan['sources']],
                     'source_sha256': measurement['source_sha256'], 'result': compact(measurement)})
    game.append({'name': case['name'], 'selection': case['selection']['omitted_observations'], 'measurements': rows})
audit = {'source_manifest_sha256': sha(OUT / 'source-frozen.json'), 'result_sha256': sha(OUT / 'result.json'),
         'frozen_sources': len(frozen['sources']), 'unchanged_inputs': len(inputs),
         'original_three_cases_unchanged': True, 'analytic_passed': sum(c['passed'] for c in analytic),
         'analytic_total': len(analytic), 'analytic': analytic, 'game': game,
         'game_motion_accuracy': 'unknown', 'process': read(OUT / 'process.json')}
with (OUT / 'audit.json').open('x', encoding='utf-8') as f: f.write(json.dumps(audit, indent=2) + '\n')
print(json.dumps({k: v for k, v in audit.items() if k not in ['analytic', 'game']}))
