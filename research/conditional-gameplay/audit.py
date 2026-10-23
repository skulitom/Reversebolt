"""Audit the retained baseline without another model call or game launch."""
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/conditional-gameplay-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
result = read(OUT / 'result.json')
evidence = read(OUT / 'image-evidence.json')
frames = read(OUT / 'final-state.json')['state']['live']['frames']
analysis = OUT / 'analysis'
activity = read(analysis / 'activity.json')
for name, digest in frozen['sources'].items():
    assert sha(OUT / 'source' / name) == digest
for name, digest in frozen['binaries'].items():
    assert sha(OUT / 'bin' / name) == digest
assert sha(OUT / 'historical-questions.json') == frozen['historical_questions_sha256']
started = read(analysis / 'started.json')
assert sha(analysis / 'driver.py') == started['driver_sha256']
assert sha(OUT / 'image-evidence.json') == started['image_evidence_sha256']
assert sha(OUT / 'result.json') == started['native_result_sha256']
assert result['finished'] and result['images'] == len(frames) == 15
assert result['gameplay_writes'] == result['native_runtime_calls'] == result['viewed_measurement_images'] == 0
assert read(OUT / 'anode-job.json')['exitCode'] == read(OUT / 'child.json')['exit_code'] == 0
assert read(analysis / 'activity-terminal.json')['exit_code'] == 0
assert read(analysis / 'planning-terminal.json')['exit_code'] == 1
assert not (analysis / 'plan.json').exists() and not (analysis / 'design').exists()
assert activity['error'] is None and activity['stage'] is not None
assert activity['source'] == evidence['activity_source']
assert hashlib.sha256(activity['stage']['prompt'].encode()).hexdigest() == activity['stage']['prompt_sha256']
assert not activity['observation']['consistent_visible_change']
assert all(c['error'] is None for c in evidence['capture_clocks']['frames'])
files = {sha(p): p for p in (OUT / 'data/captures').rglob('*.png')}
assert all(f['sha256'] in files and f['input_context'] is not None for f in frames)
by_id = {f['id']: f for f in frames}
manifest = read(analysis / 'activity-manifest.json')
assert manifest['source'] == activity['source']
references = [f for pair in activity['source']['pairs'] for f in [pair['before'], pair['after']]]
assert len(manifest['frames']) == len(references) == 4
for path, ref in zip(manifest['frames'], references):
    assert sha(Path(path)) == ref['sha256'] == by_id[ref['id']]['sha256']

# Recompute every adjacent image difference with the existing pure image auditor.
# This binary is recorded here; it was not part of the original pre-launch freeze.
replay = OUT / 'audit-replay'
replay.mkdir(exist_ok=True)
pairs = [[str(files[a['sha256']]), str(files[b['sha256']])] for a, b in zip(frames, frames[1:])]
(replay / 'pairs.json').write_text(json.dumps(pairs), encoding='utf-8')
metric = ROOT / 'target/release/examples/response_audit.exe'
run = subprocess.run([str(metric), str(replay / 'pairs.json')], capture_output=True, text=True, creationflags=subprocess.CREATE_NO_WINDOW)
assert run.returncode == 0, run.stderr
images = json.loads(run.stdout)
assert len(images['pairs']) == len(evidence['evidence']['intervals']) == 14
for i, (pair, interval) in enumerate(zip(images['pairs'], evidence['evidence']['intervals'])):
    assert pair['hashes'] == [frames[i]['sha256'], frames[i + 1]['sha256']]
    assert pair['difference'] == interval['measurement']['difference']
(replay / 'images.json').write_text(json.dumps(images, indent=2), encoding='utf-8')

holds = result['holds']
assert [h['key'] for h in holds] == [68, 65, 68, 65]
assert all(h['down_accepted'] == h['up_accepted'] == 1 for h in holds)
timings = []
for episode in evidence['evidence']['episodes']:
    if episode['exposure']['kind'] != 'key':
        continue
    key = episode['exposure']['value']
    assert episode['bounded_by_quiet'] and episode['first_interval'] == episode['last_interval']
    polls = frames[episode['first_interval'] + 1]['input_context']['polls']
    pressed = [i for i, p in enumerate(polls) if p['down'] == [key]]
    assert pressed and pressed[0] > 0 and pressed[-1] + 1 < len(polls)
    lower = polls[pressed[-1]]['begin_us'] - polls[pressed[0]]['end_us']
    upper = polls[pressed[-1] + 1]['end_us'] - polls[pressed[0] - 1]['begin_us']
    assert 0 <= upper - lower <= 100000
    midpoint = (lower + (upper - lower) // 2 + 500) // 1000
    timings.append({'key': key, 'bounds_us': [lower, upper], 'estimated_hold_ms': midpoint,
                    'within_planner_100_to_1000_ms': 100 <= midpoint <= 1000})
assert len(timings) == 4 and sum(t['within_planner_100_to_1000_ms'] for t in timings) == 1

storage = Path(frozen['storage'])
before = frozen['storage_before']
for name, digest in before.items():
    assert sha(OUT / 'storage-before' / name) == digest
after = {p.relative_to(storage).as_posix(): sha(p) for p in storage.rglob('*') if p.is_file()}
diff = {'changed': sorted(n for n in before.keys() & after.keys() if before[n] != after[n]),
        'added': sorted(after.keys() - before.keys()), 'removed': sorted(before.keys() - after.keys()),
        'before': before, 'after': after}
(OUT / 'storage-diff.json').write_text(json.dumps(diff, indent=2), encoding='utf-8')
summary = {'passed': True, 'images': len(frames), 'replayed_image_differences': len(images['pairs']),
           'metric_binary_sha256': sha(metric), 'image_model_calls': 1, 'test_design_calls': 0,
           'planning_error': (analysis / 'planning.stderr').read_text().strip(),
           'sampled_timings': timings,
           'actual_hold_envelopes_ms': [(h['end_ticks'] - h['begin_ticks']) * 1000 / h['frequency_hz'] for h in holds],
           'consistent_visible_change': False, 'live_paths_reacquired': False, 'gameplay_writes': 0,
           'storage_changed': diff['changed'], 'storage_added': diff['added'], 'storage_removed': diff['removed'],
           'limitation': 'A valid input/image association did not yield an eligible repeated-action plan or a verified semantic consequence.'}
(OUT / 'audit.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
print(json.dumps(summary))
