"""Freeze raw-label review before any relevance inference."""
import hashlib
import json
import shutil
import subprocess
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-relevance-v1'
SOURCE = ROOT / 'test-results/bounded-type-selection-v1/revised'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
with urllib.request.urlopen('http://127.0.0.1:11434/api/tags', timeout=10) as response:
    model = next(m for m in json.load(response)['models'] if m['name'] == 'qwen3.8:latest')
assert model['digest'] == '22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643'
cases = []
for case, variation in [(c, 'original') for c in ['vampire', 'latest', 'menu', 'fixture', 'opaque']] + [
        ('latest', 'reversed'), ('menu', 'reversed'), ('vampire', 'opaque')]:
    source = SOURCE / case
    receipt = read(source / 'receipt.json')
    assert receipt['error'] is None and not receipt['cancelled'] and receipt['model_digest'] == model['digest']
    cases.append({'name': case + '-' + variation, 'source': str(source), 'variation': variation,
                  'hashes': {p: sha(source / p) for p in ['receipt.json', 'input.json']}})
OUT.mkdir()
shutil.copy2(ROOT / 'target/release/examples/runtime-relevance-replay.exe', OUT / 'review.exe')
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / p for p in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
    'research/runtime-relevance/PROTOCOL.md', 'research/runtime-relevance/freeze.py', 'research/runtime-relevance/run.py']]
digests = {}
for source in sources:
    name = source.relative_to(ROOT).as_posix()
    destination = OUT / 'source' / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    digests[name] = sha(destination)
frozen = {'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
          'binary_sha256': sha(OUT / 'review.exe'), 'sources': digests, 'cases': cases,
          'model': {k: model[k] for k in ['name', 'digest', 'size']}}
(OUT / 'frozen.json').write_text(json.dumps(frozen, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'output': str(OUT), 'sources': len(digests), 'cases': len(cases)}))
