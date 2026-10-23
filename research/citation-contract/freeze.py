"""Freeze the shared schema repair and fixed fresh-output cases."""
import hashlib
import json
import shutil
import subprocess
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/citation-contract-v1'
NAMING = ROOT / 'test-results/bounded-type-selection-v1/revised'
CHALLENGE = ROOT / 'test-results/runtime-relevance-challenge-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
save = lambda p, v: p.write_text(json.dumps(v, indent=2) + '\n', encoding='utf-8')
with urllib.request.urlopen('http://127.0.0.1:11434/api/tags', timeout=10) as response:
    model = next(m for m in json.load(response)['models'] if m['name'] == 'qwen3.8:latest')
assert model['digest'] == '22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643'
cases = [
    {'name': 'failed-relevance-batch', 'kind': 'review', 'source': str(CHALLENGE / 'inputs/balanced-1.json'),
     'variation': 'synthetic', 'baseline': str(CHALLENGE / 'balanced-1/receipt.json')},
    {'name': 'vampire-relevance', 'kind': 'review', 'source': str(NAMING / 'vampire'),
     'variation': 'original', 'baseline': str(ROOT / 'test-results/runtime-relevance-v1/vampire-original/receipt.json')},
]
for name in ['fixture', 'vampire']:
    cases.append({'name': name + '-naming', 'kind': 'naming', 'source': read(NAMING / name / 'input.json')['source'],
                  'variation': None, 'baseline': str(NAMING / name / 'receipt.json'),
                  'prior_input': str(NAMING / name / 'input.json')})
for case in cases:
    source = Path(case['source'])
    case['hashes'] = {str(p): sha(p) for p in
        ([source / 'receipt.json', source / 'input.json'] if source.is_dir() else [source]) + [Path(case['baseline'])]}
    if 'prior_input' in case:
        case['hashes'][case['prior_input']] = sha(Path(case['prior_input']))
    assert read(Path(case['baseline']))['model_digest'] == model['digest']
OUT.mkdir()
shutil.copy2(ROOT / 'target/release/examples/runtime-relevance-replay.exe', OUT / 'review.exe')
shutil.copy2(ROOT / 'target/release/reversebolt-runtime-hypotheses.exe', OUT / 'naming.exe')
schemas = json.loads(subprocess.check_output([str(OUT / 'review.exe'), 'schema'], text=True, encoding='utf-8'))
save(OUT / 'schemas.json', schemas)
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / p for p in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
    'research/citation-contract/PROTOCOL.md', 'research/citation-contract/freeze.py',
    'research/citation-contract/run.py', 'research/citation-contract/schema-check.py']]
digests = {}
for source in sources:
    name = source.relative_to(ROOT).as_posix()
    target = OUT / 'source' / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)
    digests[name] = sha(target)
save(OUT / 'frozen.json', {'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
    'binaries': {p: sha(OUT / p) for p in ['review.exe', 'naming.exe']}, 'sources': digests,
    'schemas_sha256': sha(OUT / 'schemas.json'), 'cases': cases,
    'model': {k: model[k] for k in ['name', 'digest', 'size']}})
print(json.dumps({'output': str(OUT), 'sources': len(digests), 'cases': len(cases)}))
