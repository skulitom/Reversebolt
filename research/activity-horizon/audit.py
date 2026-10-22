"""Audit frozen inputs and retain all outcomes, without treating claims as truth."""
import hashlib, json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/activity-horizon-v1'

def read(p):
    return json.loads(p.read_text(encoding='utf-8-sig'))

def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

frozen = read(OUT / 'source-frozen.json')
for p, digest in frozen['sources'].items():
    assert sha(ROOT / p) == sha(OUT / 'source' / p) == digest, p
for binary in frozen['binaries'].values():
    assert sha(Path(binary['file'])) == binary['sha256']
for name in ['inputs-frozen.json', 'manifests-frozen.json']:
    for p, digest in read(OUT / name).items():
        assert sha(Path(p)) == digest, p

cases = []
for i in range(2):
    result = read(OUT / f'case-{i}/result.json')
    sequence = result['evidence']['automatic_region_motion']['appearance_sequences']
    baseline = read(OUT / f'case-{i}-baseline-receipt.json')
    augmented = read(OUT / f'case-{i}-augmented-receipt.json')
    summary = sequence['activity']['summary']
    assert baseline['source'] == augmented['source']
    assert baseline['motion'] == augmented['motion']
    assert baseline.get('appearances') is None and augmented['appearances'] == summary
    assert sequence['report']['target']['last_frame_id'] == baseline['source']['pairs'][1]['after']['id']
    assert sequence['report']['target']['source_sha256'] == summary['activity_source_sha256']
    for receipt in [baseline, augmented]:
        if receipt['stage']:
            prompt = receipt['stage']['prompt']
            assert hashlib.sha256(prompt.encode()).hexdigest() == receipt['stage']['prompt_sha256']
    for label in ['baseline', 'augmented']:
        models = read(OUT / f'case-{i}-{label}-model.json')['models']
        assert any(m['name'] == 'qwen3.8:latest' and m['digest'] ==
                   '22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643' for m in models)
    cases.append({
        'case': i,
        'selected_frames': [[p['before']['number'], p['after']['number']] for p in baseline['source']['pairs']],
        'attempted_pairs': sequence['report']['attempted_pairs'],
        'series': [{'accepted_steps': s['accepted_steps'], 'stop': s['stop'],
                    'stopped_before': s['stopped_before']} for s in sequence['report']['series']],
        'outcomes': [a['pairs'] for a in summary['appearances']],
        'baseline': baseline['observation'], 'augmented': augmented['observation'],
        'baseline_error': baseline['error'], 'augmented_error': augmented['error'],
        'baseline_receipt_sha256': sha(OUT / f'case-{i}-baseline-receipt.json'),
        'augmented_receipt_sha256': sha(OUT / f'case-{i}-augmented-receipt.json'),
        'global_motion_unchanged': True, 'accuracy': 'unknown'})
audit = {'source_manifest_sha256': sha(OUT / 'source-frozen.json'),
         'source_files': len(frozen['sources']),
         'input_files': len(read(OUT / 'inputs-frozen.json')),
         'image_interpretation_calls': 4, 'new_object_observation_calls': 0,
         'native_target_access': False, 'cases': cases}
with (OUT / 'audit.json').open('x', encoding='utf-8') as f:
    f.write(json.dumps(audit, indent=2) + '\n')
print(json.dumps(audit, indent=2))
