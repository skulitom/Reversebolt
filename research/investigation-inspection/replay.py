"""Freeze and inspect the retained native timing failure, with no game/model I/O."""
import hashlib
import json
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'test-results/conditional-gameplay-v1'
OUT = ROOT / 'test-results/investigation-inspection-v1'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
OUT.mkdir()
(OUT / 'source').mkdir()
frozen = {'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
          'sources': {}, 'inputs': {}, 'model_calls': 0, 'target_access': False}
for p in list((ROOT / 'crates').rglob('*.rs')) + [Path(__file__)]:
    name = p.relative_to(ROOT).as_posix()
    dest = OUT / 'source' / name
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, dest)
    frozen['sources'][name] = sha(dest)
inputs = ['analysis/activity.json', 'image-evidence.json', 'final-state.json']
for name in inputs:
    dest = OUT / 'inputs' / name
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(SOURCE / name, dest)
    frozen['inputs'][name] = sha(dest)
binary = OUT / 'activity-assessment.exe'
shutil.copy2(ROOT / 'target/release/examples/activity-assessment.exe', binary)
frozen['binary_sha256'] = sha(binary)
(OUT / 'frozen.json').write_text(json.dumps(frozen, indent=2), encoding='utf-8')
with (OUT / 'stdout.txt').open('x') as stdout, (OUT / 'stderr.txt').open('x') as stderr:
    child = subprocess.Popen([str(binary), *[str(OUT / 'inputs' / name) for name in inputs], str(OUT / 'assessment.json')],
                             stdout=stdout, stderr=stderr, creationflags=subprocess.CREATE_NO_WINDOW)
    (OUT / 'process.json').write_text(json.dumps({'pid': child.pid}))
    code = child.wait()
(OUT / 'terminal.json').write_text(json.dumps({'pid': child.pid, 'exit_code': code}))
assert code == 0, (OUT / 'stderr.txt').read_text()
result = read(OUT / 'assessment.json')
original = read(SOURCE / 'audit.json')
assert result['replay']['actions'] is None and result['execution_authority'] is False
assert result['context']['observation'] == read(SOURCE / 'analysis/activity.json')['observation']
issues = result['replay']['timing_issues']
expected = [t for t in original['sampled_timings'] if not t['within_planner_100_to_1000_ms']]
assert len(issues) == len(expected) == 3
for actual, prior in zip(issues, expected):
    assert actual['key'] == prior['key']
    assert actual['hold_bounds_us'] == prior['bounds_us']
    assert actual['estimated_hold_ms'] == prior['estimated_hold_ms']
assert all(sha(OUT / 'source' / n) == h for n, h in frozen['sources'].items())
assert all(sha(OUT / 'inputs' / n) == h == sha(SOURCE / n) for n, h in frozen['inputs'].items())
summary = {'passed': True, 'timing_issues': issues, 'model_interpretation_unchanged': True,
           'replay_available': False, 'model_calls': 0, 'target_access': False,
           'original_evidence_unchanged': True, 'assessment_sha256': sha(OUT / 'assessment.json')}
(OUT / 'audit.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
print(json.dumps(summary))
