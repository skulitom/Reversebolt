"""Freeze and replay three retained owned-fixture trials, without native access."""
import hashlib, json, shutil, subprocess, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/trial-motion-v1'
OUT.mkdir()

def read(p):
    return json.loads(p.read_text(encoding='utf-8-sig'))

def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

def save(p, v):
    p.write_text(json.dumps(v, indent=2) + '\n', encoding='utf-8')

records = sorted((ROOT / 'test-results/planned-input-v1/case/data/runtime-tests').glob('*.json'))
assert len(records) == 3
inputs = {}
for p in records:
    t = read(p)
    assert t['candidate']['locator']['root']['declaring']['image'] == 'OwnedPathFixture'
    inputs[str(p)] = sha(p)
    for c in t['conditions']:
        for f in c['frames']:
            for path_key, hash_key in [('path', 'sha256'), ('input_path', 'input_sha256')]:
                path = p.parent / f[path_key]
                assert sha(path) == f[hash_key]
                inputs[str(path)] = sha(path)
binary = OUT / 'trial-motion-replay.exe'
shutil.copy2(ROOT / 'target/debug/examples/trial-motion-replay.exe', binary)
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__), Path(__file__).with_name('PROTOCOL.md')]
digests = {}
for p in sources:
    name = p.relative_to(ROOT).as_posix()
    target = OUT / 'source' / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, target)
    digests[name] = sha(target)
save(OUT / 'frozen.json', {'parent': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
    'sources': digests, 'inputs': inputs, 'binary': str(binary), 'binary_sha256': sha(binary)})
runs = []
for p in records:
    start = time.monotonic()
    output = OUT / (p.stem + '.json')
    process = subprocess.run([str(binary), str(p), str(output)], cwd=ROOT, capture_output=True, text=True,
                             timeout=30, creationflags=subprocess.CREATE_NO_WINDOW)
    run = {'record': str(p), 'output': str(output), 'seconds': time.monotonic()-start,
           'exit_code': process.returncode, 'stdout': process.stdout, 'stderr': process.stderr}
    runs.append(run)
    save(OUT / 'processes.json', runs)
    assert process.returncode == 0, run
print(json.dumps({'runs': len(runs), 'seconds': sum(r['seconds'] for r in runs)}))
