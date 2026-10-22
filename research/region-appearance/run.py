"""Frozen offline replay; no target, model, capture or gameplay input."""
import hashlib, json, shutil, subprocess, time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/region-appearance-v1'
OUT.mkdir()
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f: return hashlib.file_digest(f, 'sha256').hexdigest()
def save(p, value):
    with p.open('x', encoding='utf-8') as f: f.write(json.dumps(value, indent=2) + '\n')

binaries = {}
for name in ['appearance-fixture.exe', 'observed-region-replay.exe']:
    target = OUT / name
    shutil.copy2(ROOT / 'target/debug/examples' / name, target)
    binaries[str(target)] = sha(target)
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__),
            Path(__file__).with_name('PROTOCOL.md'), Path(__file__).with_name('audit.py')]
hashes = {}
for p in sources:
    relative = p.relative_to(ROOT).as_posix()
    target = OUT / 'source' / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, target)
    hashes[relative] = sha(target)
save(OUT / 'source-frozen.json', {'parent': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                                'sources': hashes, 'binaries': binaries})
def run(name, args, receipt):
    start = time.monotonic()
    r = subprocess.run([str(OUT / name), *map(str, args)], cwd=ROOT, capture_output=True,
                       text=True, timeout=60, creationflags=subprocess.CREATE_NO_WINDOW)
    result = {'exit_code': r.returncode, 'seconds': time.monotonic() - start, 'stdout': r.stdout, 'stderr': r.stderr}
    save(OUT / receipt, result)
    assert r.returncode == 0, result
    return result
run('appearance-fixture.exe', [OUT / 'controls'], 'fixture-process.json')
prior_manifest = ROOT / 'test-results/observed-regions-v1/cases.json'
prior_result = ROOT / 'test-results/observed-regions-v1/result.json'
cases = read(OUT / 'controls/cases.json') + read(prior_manifest)
assert len(cases) == 8
manifest = OUT / 'cases.json'
save(manifest, cases)
inputs = [manifest, prior_manifest, prior_result, OUT / 'controls/cases.json', OUT / 'controls/truth.json']
for case in cases:
    for frame in case['frames']:
        p = Path(frame['file'])
        assert sha(p) == frame['reference']['sha256']
        inputs.append(p)
save(OUT / 'inputs-frozen.json', {str(p): sha(p) for p in inputs})
print(json.dumps(run('observed-region-replay.exe', [manifest, OUT / 'result.json'], 'process.json')))
