"""Freeze one offline handoff replay without invoking native/editor code."""
import hashlib
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser()
parser.add_argument('--version', default='v1', choices=['v1', 'v2', 'v3'])
version = parser.parse_args().version
OUT = ROOT / ('test-results/activity-handoff-' + version)
OUT.mkdir()
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

status = ROOT / 'test-results/independent-runtime-v2/discovery.json'
prior = ROOT / 'test-results/measured-contrast-v1/replay.json'
live = read(status)['state']['live']
native = read(ROOT / 'test-results/independent-runtime-v2/frozen.json')
captures = list((Path(native['data']) / 'captures').iterdir())
assert len(captures) == 1
inputs = [status, prior, Path(live['runtime']['catalog_source']['path'])]
inputs += [captures[0] / f"frame-{f['number']:03}.png" for f in live['frames']]
binary = ROOT / 'target/debug/examples/agent-handoff-replay.exe'
archived_binary = OUT / binary.name
shutil.copy2(binary, archived_binary)
assert sha(binary) == sha(archived_binary)
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__)] + list(Path(__file__).parent.glob('*.md'))
digests = {}
for p in sources:
    name = p.relative_to(ROOT).as_posix()
    copy = OUT / 'source' / name
    copy.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, copy)
    digests[name] = sha(copy)
manifest = {'parent': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
            'sources': digests, 'inputs': {str(p): sha(p) for p in inputs},
            'binary': str(archived_binary), 'launched_binary': str(binary), 'binary_sha256': sha(binary)}
(OUT / 'frozen.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
start = time.monotonic()
process = subprocess.run([str(binary), str(status), str(prior), str(captures[0]), str(OUT / 'replay.json')],
    cwd=ROOT, capture_output=True, text=True, timeout=30, creationflags=subprocess.CREATE_NO_WINDOW)
result = {'exit_code': process.returncode, 'seconds': time.monotonic()-start,
          'stdout': process.stdout, 'stderr': process.stderr}
(OUT / 'process.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
print(json.dumps(result))
assert process.returncode == 0
