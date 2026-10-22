"""Freeze and run the existing file-only replay; no native/editor execution."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/path-planning-v1'
OUT.mkdir()  # Do not overwrite an earlier run or failed attempt.

def read(p):
    return json.loads(p.read_text(encoding='utf-8-sig'))

def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

status = ROOT / 'test-results/independent-runtime-v2/discovery.json'
prior = ROOT / 'test-results/measured-contrast-v1/replay.json'
baseline = ROOT / 'test-results/activity-handoff-v3/replay.json'
live = read(status)['state']['live']
native = read(ROOT / 'test-results/independent-runtime-v2/frozen.json')
captures = list((Path(native['data']) / 'captures').iterdir())
assert len(captures) == 1
inputs = [status, prior, baseline, Path(live['runtime']['catalog_source']['path'])]
inputs += [captures[0] / f"frame-{f['number']:03}.png" for f in live['frames']]
binary = ROOT / 'target/debug/examples/agent-handoff-replay.exe'
archived = OUT / binary.name
shutil.copy2(binary, archived)
assert sha(binary) == sha(archived)
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__), Path(__file__).with_name('PROTOCOL.md')]
digests = {}
for p in sources:
    name = p.relative_to(ROOT).as_posix()
    copy = OUT / 'source' / name
    copy.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, copy)
    digests[name] = sha(copy)
manifest = {'parent': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
            'sources': digests, 'inputs': {str(p): sha(p) for p in inputs},
            'binary': str(archived), 'binary_sha256': sha(archived)}
(OUT / 'frozen.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
start = time.monotonic()
process = subprocess.run([str(archived), str(status), str(prior), str(captures[0]), str(OUT / 'replay.json')],
                         cwd=ROOT, capture_output=True, text=True, timeout=30,
                         creationflags=subprocess.CREATE_NO_WINDOW)
result = {'exit_code': process.returncode, 'seconds': time.monotonic()-start,
          'stdout': process.stdout, 'stderr': process.stderr}
(OUT / 'process.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
print(json.dumps(result))
assert process.returncode == 0
