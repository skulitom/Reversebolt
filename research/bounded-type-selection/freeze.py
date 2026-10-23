"""Freeze existing reports and one revised evaluator before local inference."""
import hashlib
import json
import shutil
import subprocess
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/bounded-type-selection-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
old = ROOT / 'test-results/object-label-selection-v1'
native = ROOT / 'test-results/independent-vampire-survivors-v2'
discovery = read(native / 'discovery.json')['state']['live']['runtime']
data = native / 'data'
report = next((data / 'runtime').glob('*-*/**/report.json'))
receipt = data / 'runtime/naming' / discovery['naming']['evidence_id'] / 'receipt.json'
assert sha(report) == discovery['catalog_source']['document_sha256']
assert read(receipt)['error'] == 'Type selection exhausted its review-round budget.'
cases = [{'name': 'vampire', 'source': str(report), 'sha256': sha(report),
          'baseline_receipt': str(receipt), 'original_inventory': str(receipt.with_name('inventory.json'))}]
for name in ['latest', 'menu', 'fixture', 'opaque']:
    case = next(c for c in read(old / 'revised-frozen.json')['cases'] if c['name'] == name).copy()
    assert sha(Path(case['source'])) == case['sha256']
    case['baseline_receipt'] = str(old / 'revised' / name / 'receipt.json')
    cases.append(case)
with urllib.request.urlopen('http://127.0.0.1:11434/api/tags', timeout=10) as response:
    model = next(m for m in json.load(response)['models'] if m['name'] == 'qwen3.8:latest')
assert model['digest'] == '22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643'
for case in cases:
    case['baseline_sha256'] = sha(Path(case['baseline_receipt']))
    assert read(Path(case['baseline_receipt']))['model_digest'] == model['digest']
OUT.mkdir()
(OUT / 'bin').mkdir()
binary = OUT / 'bin/revised.exe'
shutil.copy2(ROOT / 'target/release/reversebolt-runtime-hypotheses.exe', binary)
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / p for p in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
    'research/bounded-type-selection/PROTOCOL.md', 'research/bounded-type-selection/freeze.py',
    'research/metadata-independence/run.py']]
digests = {}
for path in sources:
    name = path.relative_to(ROOT).as_posix()
    target = OUT / 'revised-source' / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(path, target)
    digests[name] = sha(target)
frozen = {'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
          'binary': 'bin/revised.exe', 'binary_sha256': sha(binary), 'sources': digests,
          'model': {k: model[k] for k in ['name', 'digest', 'size']}, 'cases': cases,
          'original_inventory_sha256': sha(Path(cases[0]['original_inventory']))}
(OUT / 'revised-frozen.json').write_text(json.dumps(frozen, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'output': str(OUT), 'cases': [c['name'] for c in cases], 'sources': len(digests)}))
