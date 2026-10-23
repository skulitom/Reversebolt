"""Freeze and replay sibling selection; no live process access or generation."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-siblings-v1'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
OUT.mkdir()
(OUT / 'bin').mkdir()
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / p for p in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
    'research/runtime-siblings/PROTOCOL.md', 'research/runtime-siblings/offline.py',
    'research/runtime-siblings/native.py']]
frozen = {'sources': {}, 'binaries': {}, 'inputs': {},
          'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()}
for p in sources:
    name = p.relative_to(ROOT).as_posix()
    dest = OUT / 'source' / name
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, dest)
    frozen['sources'][name] = sha(dest)
for name in ['reversebolt_runtime.dll', 'reversebolt-runtime-loader.exe',
             'reversebolt-runtime-neighbors.exe', 'reversebolt-runtime-response.exe',
             'reversebolt-runtime-candidates.exe']:
    dest = OUT / 'bin' / name
    shutil.copy2(ROOT / 'target/release' / name, dest)
    frozen['binaries'][name] = sha(dest)
# Unchanged owned ABI fixture from the earlier path experiment, with provenance.
for name in ['object-host.exe', 'reversebolt_path_fixture.dll']:
    source = ROOT / 'test-results/runtime-paths-v2/bin' / name
    dest = OUT / 'bin' / name
    shutil.copy2(source, dest)
    frozen['binaries'][name] = sha(dest)
    frozen['inputs'][source.relative_to(ROOT).as_posix()] = sha(source)
cases = []
for label, prior in [('sable', 'sable-live-context-v1'), ('viewfinder', 'viewfinder-live-paging-v1')]:
    runtime = ROOT / 'test-results' / prior / 'data/runtime'
    receipts = list((runtime / 'naming').glob('*/receipt.json'))
    reports = [p for p in runtime.glob('*/*/report.json') if read(p).get('schema') == 4]
    assert len(receipts) == len(reports) == 1
    for kind, source in [('report', reports[0]), ('receipt', receipts[0])]:
        dest = OUT / f'{label}-{kind}.json'
        shutil.copy2(source, dest)
        frozen['inputs'][source.relative_to(ROOT).as_posix()] = sha(source)
    cases.append(label)
(OUT / 'frozen.json').write_text(json.dumps(frozen, indent=2) + '\n')
results = []
for label in cases:
    result = subprocess.run([str(OUT / 'bin/reversebolt-runtime-neighbors.exe'),
        str(OUT / f'{label}-report.json'), str(OUT / f'{label}-receipt.json'), str(OUT / label)],
        capture_output=True, text=True, timeout=120)
    (OUT / f'{label}.stdout').write_text(result.stdout)
    (OUT / f'{label}.stderr').write_text(result.stderr)
    assert result.returncode == 0, result.stderr
    plan = read(OUT / label / 'plan.json')
    results.append({'case': label, 'plan_sha256': sha(OUT / label / 'plan.json'),
        'roles': plan['roles'], 'selected': len(plan['candidates']), 'omitted': plan['omitted_fields'],
        'batches': [len(b) for b in plan['batches']]})
(OUT / 'coverage.json').write_text(json.dumps(results, indent=2) + '\n')
print(json.dumps([{'case': r['case'], 'roles': len(r['roles']), 'selected': r['selected'],
                  'omitted': r['omitted'], 'batches': r['batches']} for r in results]))
