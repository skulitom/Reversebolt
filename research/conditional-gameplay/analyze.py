"""Interpret one completed baseline, then assess historical questions once."""
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
job = read(OUT / 'anode-job.json')
assert result['finished'] and job['finished'] and job['exitCode'] == 0
assert sha(OUT / 'historical-questions.json') == frozen['historical_questions_sha256']
for name, digest in frozen['binaries'].items():
    assert sha(OUT / 'bin' / name) == digest
analysis = OUT / 'analysis'
analysis.mkdir()
(analysis / 'driver.py').write_bytes(Path(__file__).read_bytes())
(analysis / 'started.json').write_text(json.dumps({'driver_sha256': sha(analysis / 'driver.py'),
    'image_evidence_sha256': sha(OUT / 'image-evidence.json'), 'native_result_sha256': sha(OUT / 'result.json')}), encoding='utf-8')
def run(name, args):
    with (analysis / (name + '.stdout')).open('x', encoding='utf-8') as stdout, (analysis / (name + '.stderr')).open('x', encoding='utf-8') as stderr:
        child = subprocess.Popen(args, cwd=ROOT, stdout=stdout, stderr=stderr, creationflags=subprocess.CREATE_NO_WINDOW)
        (analysis / (name + '-process.json')).write_text(json.dumps({'pid': child.pid}), encoding='utf-8')
        code = child.wait()
    (analysis / (name + '-terminal.json')).write_text(json.dumps({'pid': child.pid, 'exit_code': code}), encoding='utf-8')
    print(json.dumps({'stage': name, 'exit_code': code}), flush=True)
    if code != 0:
        error = (analysis / (name + '.stderr')).read_text(encoding='utf-8')
        # Keep a terminal summary even when a normal eligibility check stops the
        # chain. Never rerun a stage to recover its outcome.
        (analysis / 'result.json').write_text(json.dumps({'finished': True,
            'stopped_at': name, 'exit_code': code, 'error': error,
            'live_paths_reacquired': False, 'gameplay_writes': 0}), encoding='utf-8')
        raise RuntimeError(error)

evidence = read(OUT / 'image-evidence.json')
source = evidence['activity_source']
if source is None:
    (analysis / 'result.json').write_text(json.dumps({'finished': True, 'activity_source_available': False, 'image_model_calls': 0, 'design_calls': 0}), encoding='utf-8')
    print('No eligible activity source; no inference or edit performed.')
    raise SystemExit(0)
files = {sha(p): p for p in (OUT / 'data/captures').rglob('*.png')}
references = [frame for pair in source['pairs'] for frame in [pair['before'], pair['after']]]
manifest = {'source': source, 'frames': [str(files[frame['sha256']]) for frame in references]}
(analysis / 'activity-manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
run('activity', [str(OUT / 'bin/activity-replay.exe'), str(analysis / 'activity-manifest.json'), str(analysis / 'activity.json'), frozen['model']])
activity = read(analysis / 'activity.json')
if activity['error'] is not None or activity['observation'] is None:
    (analysis / 'result.json').write_text(json.dumps({'finished': True, 'activity_source_available': True, 'image_model_calls': int(activity['stage'] is not None), 'design_calls': 0, 'activity_error': activity['error']}), encoding='utf-8')
    raise SystemExit(0)
run('planning', [str(OUT / 'bin/activity-question-plan.exe'), str(OUT / 'historical-questions.json'), str(analysis / 'activity.json'),
                 str(OUT / 'image-evidence.json'), str(OUT / 'final-state.json'), str(analysis / 'plan.json')])
run('design', [str(OUT / 'bin/test-design-replay.exe'), str(analysis / 'plan.json'), str(analysis / 'design'), frozen['model']])
designs = []
for path in sorted((analysis / 'design').iterdir()):
    receipt = read(path / 'receipt.json')
    assert receipt['model_digest'] == frozen['model_digest'] and receipt['server_version'] == frozen['server_version']
    designs.append({'case': path.name, 'error': receipt['error'], 'design': receipt['design']})
(analysis / 'result.json').write_text(json.dumps({'finished': True, 'activity_source_available': True, 'image_model_calls': 1,
    'design_calls': len(designs), 'designs': designs, 'live_paths_reacquired': False, 'gameplay_writes': 0}, indent=2), encoding='utf-8')
print(json.dumps({'finished': True, 'designs': designs}), flush=True)
