"""Run the frozen local review once and retain terminal evidence."""
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-activity-focus-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
assert sha(Path(__file__)) == frozen['sources']['research/runtime-activity-focus/run.py']
for name, digest in frozen['inputs'].items():
    assert sha(OUT / 'input' / name) == digest
for name, digest in frozen['binaries'].items():
    assert sha(OUT / 'bin' / name) == digest
with (OUT / 'started.json').open('x', encoding='utf-8') as marker:
    json.dump({'state': 'starting'}, marker)
with (OUT / 'review.stdout').open('x', encoding='utf-8') as stdout, (OUT / 'review.stderr').open('x', encoding='utf-8') as stderr:
    child = subprocess.Popen([
        str(OUT / 'bin/reversebolt-runtime-hypotheses.exe'),
        str(OUT / 'input/discovery.json'), str(OUT / 'review'), frozen['model'],
        '--focus', str(OUT / 'input/manifest.json'),
    ], cwd=ROOT, stdout=stdout, stderr=stderr, creationflags=subprocess.CREATE_NO_WINDOW)
    (OUT / 'started.json').write_text(json.dumps({'pid': child.pid, 'state': 'running'}), encoding='utf-8')
    code = child.wait()
(OUT / 'terminal.json').write_text(json.dumps({'pid': child.pid, 'exit_code': code}), encoding='utf-8')
print((OUT / 'review.stdout').read_text(encoding='utf-8'))
print((OUT / 'review.stderr').read_text(encoding='utf-8'))
raise SystemExit(code)
