"""Audit saved evidence only. --record writes the compact receipt after checks."""
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
RAW = ROOT / 'test-results/visual-objects-v1'
RUN = RAW / 'tetris-1-attempt-01'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()

config = read(HERE / 'protocol-v1.json')
run = read(RUN / 'run.json')
assert sha(HERE / 'protocol-v1.json') == run['started_protocol_sha256']
for name, digest in config['pre_capture_method_sha256'].items():
    assert sha(HERE / name) == digest, name
for name, digest in config['binaries'].items():
    assert sha(RAW / 'binaries' / name) == digest == run['binaries'][name], name
upstream = ROOT / 'test-results/independent-games-v1/upstream'
assert sha(upstream / 'manifest.json') == config['upstream_manifest_sha256']
for name, digest in run['source']['files'].items():
    assert sha(upstream / 'tetris' / name) == digest, name
assert not run['errors'] and not run['blocked']
assert run['captures'] == 16 and run['oracle_samples'] == 1169

state = read(RUN / 'final-status.json')
live = state['state']['live']
frames = {f['id']: f for f in live['frames']}
assert live['sample_count'] == len(frames) == 16
assert not state['state']['changes'] and not state['state']['patches']
assert not live['prediction_check'] and not live['local_observation_warnings']
assert [j['phase'] for j in state['jobs']['history']['jobs'].values()] == ['Completed']
for frame in frames.values():
    paths = list((RUN / 'engine/captures').glob(f"*/frame-{frame['number']:03}.png"))
    assert len(paths) == 1 and sha(paths[0]) == frame['sha256']

input_rows = [json.loads(line) for line in (RUN / 'input.jsonl').read_text().splitlines()]
assert not any(r['kind'] == 'error' for r in input_rows)
pulses = [r for r in input_rows if r['kind'] == 'pulse']
delivered = read(RUN / 'delivered-keys.json')
plan = read(RUN / 'input-config.json')
assert plan['session'] != plan['parent_session']
assert plan['session'] == run['selected']['session'] == run['geometry']['session']
assert plan['pid'] == run['selected']['pid'] and plan['window'] == run['selected']['window']
assert len(pulses) == run['injected_pulses'] == 48
assert len(delivered) == run['delivered_keys'] == 2 * len(pulses)
keys = {32: ' ', 37: 'ArrowLeft', 38: 'ArrowUp', 39: 'ArrowRight', 40: 'ArrowDown'}
for i, pulse in enumerate(pulses):
    expected = plan['prefix'][i] if i < len(plan['prefix']) else plan['keys'][i % len(plan['keys'])]
    assert pulse['index'] == i and pulse['vk'] == expected
    down, up = delivered[2 * i:2 * i + 2]
    assert down['kind'] == 'down' and up['kind'] == 'up'
    assert down['key'] == up['key'] == keys[expected]
    assert down['trusted'] and up['trusted'] and not down['repeat']
    # Date.now() has integer milliseconds; the native driver records fractions.
    assert pulse['down_ms'] - 1 <= down['time_ms'] <= up['time_ms'] <= pulse['up_ms'] + 200

published = []
for track in live['visual_tracks']:
    assert not track['matches']
    for slot, observation in enumerate(track['observations']):
        if observation:
            item = observation['input']
            assert item['quality'] == 'provisional'
            assert item['value'] == {'kind': 'text', 'value': 'Object'}
            assert frames[item['frame_id']]['number'] == slot + 1
            assert frames[item['frame_id']]['sha256'] == item['image_sha256']
            published.append(item)
assert len(published) == 7
passes = []
for path in sorted((RUN / 'engine/captures/model-observations').glob('*.json')):
    receipt = read(path)
    p = receipt['pass']
    assert p['error'] is None and not p['cancelled']
    proposal = p['proposal']
    assert proposal['model_digest'] == config['model_digest'] and not proposal['rejection']
    for image, frame in zip(proposal['frames'], receipt['sequence']['frames'], strict=True):
        assert image['sha256'] == frames[frame['id']]['sha256']
    for review in p['reviews']:
        assert review['receipt']['model_digest'] == config['model_digest']
        assert not review['receipt']['rejection']
    for item in receipt['proposed_observations']:
        actual = next(o for o in published if o['key'] == item['key'])
        assert {k: v for k, v in item.items() if k != 'bounds'} == {k: v for k, v in actual.items() if k != 'bounds'}
        # JSON float round trips can differ in the last double-precision digit.
        assert all(abs(a - b) < 1e-9 for a, b in zip(item['bounds'], actual['bounds'], strict=True))
    passes.append({'file': path.name, 'sha256': sha(path),
                   'source_frame': receipt['sequence']['frames'][-1]['number'],
                   'published': len(receipt['proposed_observations'])})
assert sorted((p['source_frame'], p['published']) for p in passes) == [(2, 3), (16, 4)]
first_visible = next(s['state']['live']['sample_count'] for s in read(RUN / 'states.json')
                     if s['state']['live']['visual_tracks'])
assert first_visible == 11
score = read(RUN / 'object-score.json')
for name, digest in score['artifacts'].items():
    assert sha(RUN / name) == digest
assert score['kinds'] == {'Object': 7} and not score['passes_without_published_findings']
assert {p['sha256'] for p in score['passes']} == {p['sha256'] for p in passes}
for threshold in ['0.3', '0.5']:
    assert sum(len(r['scores'][threshold]['matched_targets']) for r in score['rows']) == 3
    assert sum(len(r['scores'][threshold]['missed_targets']) for r in score['rows']) == 1
    assert all(not r['scores'][threshold]['unscorable_targets'] for r in score['rows'])

regression = read(RAW / 'actor-regression/results.json')
reviews = read(RAW / 'actor-reviews/results.json')
assert len(regression['records']) == len(reviews['records']) == 3
for folder, script, binary in [('actor-regression', 'local-observer-evaluate.py', 'reversebolt-observer.exe'),
                               ('actor-reviews', 'local-observer-review.py', 'reversebolt-review.exe')]:
    manifest = read(RAW / folder / 'run-manifest.json')
    assert manifest['binary_sha256'] == config['binaries'][binary]
    assert sha(ROOT / 'scripts' / script) == manifest['script_sha256']
    for name, digest in config['product_source_sha256'].items():
        if name in manifest['source_sha256']:
            assert manifest['source_sha256'][name] == digest
for record in regression['records']:
    assert sha(ROOT / 'experiments/visual-challenge/annotations' / (record['clip'] + '.json')) == record['labels_sha256']
for threshold in ['0.3', '0.5']:
    for variant, expected in [('original', [4, 1, 0]), ('filtered', [4, 0, 0]), ('refined', [3, 1, 1])]:
        assert [reviews['totals'][variant][threshold][k] for k in ['tp', 'fp', 'fn']] == expected

test_log = ROOT / 'test-results/visual-objects-tests.log'
counts = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed', test_log.read_text(encoding='utf-8-sig'))
assert sum(int(passed) for passed, _ in counts) == 133
assert all(int(failed) == 0 for _, failed in counts)
delivery_path = ROOT / 'test-results/visual-objects-desktop-delivery.json'
delivery = read(delivery_path)
assert sha(Path(delivery['path'])) == delivery['sha256'] == config['binaries']['Reversebolt.exe']
assert delivery['history_preserved'] and delivery['history_count'] == 8
assert delivery['observer_model'] == config['model']
assert delivery['packaged_binary_and_model_hashes_verified']
smoke_path = ROOT / delivery['smoke_report']
smoke = read(smoke_path)
assert smoke['mcp_tools'] == 29
assert smoke['shared_session'] == smoke['auth_and_origin'] == 'passed'
assert all(case['edit_restore'] == 'passed' for case in smoke['practice'])

result = {
    'scope': 'Selected exposed-object locations; not semantics, identity, control authority or release acceptance.',
    'protocol_sha256': sha(HERE / 'protocol-v1.json'),
    'binary_sha256': config['binaries']['Reversebolt.exe'],
    'model_digest': config['model_digest'],
    'native': {'captures': 16, 'oracle_samples': 1169, 'ordered_delivered_pulses': 48,
               'published_objects': 7, 'first_visible_by_capture': first_visible,
               'selected_target_matches': 3, 'selected_target_misses': 1,
               'unscorable_targets': 0, 'passes': passes},
    'actor_regression': {k: {t: {n: v[n] for n in ['tp', 'fp', 'fn']} for t, v in values.items()}
                         for k, values in reviews['totals'].items()},
    'workspace_tests': 133,
    'packaged_mcp_tools': 29,
    'progress_percent': 25,
    'artifacts': {str(p.relative_to(ROOT)).replace('\\', '/'): sha(p) for p in [
        RUN / 'run.json', RUN / 'object-score.json', RUN / 'oracle.json', RUN / 'final-status.json',
        RUN / 'states.json', RUN / 'input.jsonl', RUN / 'delivered-keys.json',
        RAW / 'actor-regression/results.json', RAW / 'actor-reviews/results.json',
        test_log, ROOT / 'test-results/visual-objects-clippy.log',
        ROOT / 'test-results/visual-objects-build-final.log', delivery_path, smoke_path]}
}
receipt = HERE / 'evidence/v1.json'
if '--record' in sys.argv:
    assert not receipt.exists(), 'Never replace a frozen evidence receipt.'
    receipt.parent.mkdir(exist_ok=True)
    receipt.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
else:
    assert read(receipt) == result
print(json.dumps({'verified': True, 'objects': 7, 'selected_locations': '3/4',
                  'workspace_tests': 133, 'progress_percent': 25}))
