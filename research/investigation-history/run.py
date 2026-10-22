"""Freeze a file-only history replay; original native evidence remains read-only."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/investigation-history-v1'
OUT.mkdir()

def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

record = ROOT / 'test-results/planned-input-v1/case/data/runtime-tests/080b5643-5718-494f-9ed6-751bf8059402.json'
inputs = [record] + list(record.with_suffix('').iterdir())
assert all(p.is_file() for p in inputs)
binary = ROOT / 'target/debug/examples/history-replay.exe'
archived = OUT / binary.name
shutil.copy2(binary, archived)
assert sha(binary) == sha(archived)
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__), Path(__file__).with_name('PROTOCOL.md')]
digests = {}
for p in sources:
    relative = p.relative_to(ROOT).as_posix()
    copy = OUT / 'source' / relative
    copy.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, copy)
    digests[relative] = sha(copy)
manifest = {'parent': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
            'sources': digests, 'inputs': {str(p): sha(p) for p in inputs},
            'binary': str(archived), 'binary_sha256': sha(archived), 'original_record': str(record)}
(OUT / 'frozen.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
start = time.monotonic()
process = subprocess.run([str(archived), str(record), str(OUT / 'replay')], cwd=ROOT,
                         capture_output=True, text=True, timeout=30, creationflags=subprocess.CREATE_NO_WINDOW)
result = {'exit_code': process.returncode, 'seconds': time.monotonic()-start,
          'stdout': process.stdout, 'stderr': process.stderr}
(OUT / 'process.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
print(json.dumps(result))
assert process.returncode == 0
