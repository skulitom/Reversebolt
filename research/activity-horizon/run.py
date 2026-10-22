"""Frozen, file-only two-case replay and four-call activity ablation."""
import hashlib, json, shutil, subprocess, time, urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/activity-horizon-v1'
OLD = ROOT / 'test-results/runtime-appearances-v1'
PRIOR = ROOT / 'test-results/appearance-context-v1'
MODEL = 'qwen3.8:latest'
DIGEST = '22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643'

def read(p):
    return json.loads(p.read_text(encoding='utf-8-sig'))

def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

def save(p, value):
    with p.open('x', encoding='utf-8') as f:
        f.write(json.dumps(value, indent=2) + '\n')

def check_model():
    with urllib.request.urlopen('http://127.0.0.1:11434/api/tags', timeout=5) as r:
        tags = json.load(r)
    assert any(m['name'] == MODEL and m['digest'] == DIGEST for m in tags['models'])
    return tags

tags = check_model()
OUT.mkdir()
save(OUT / 'model-before.json', tags)
inputs = [OUT / 'model-before.json']
for i in range(2):
    inputs += [OLD / f'case-{i}.json', OLD / f'case-{i}/result.json',
               PRIOR / f'case-{i}-baseline.json']
    inputs += [Path(f['file']) for f in read(OLD / f'case-{i}.json')['frames']]
save(OUT / 'inputs-frozen.json', {str(p): sha(p) for p in inputs})
binaries = {}
for name in ['runtime-appearance-replay', 'activity-replay']:
    source = ROOT / f'target/debug/examples/{name}.exe'
    target = OUT / source.name
    shutil.copy2(source, target)
    binaries[name] = {'file': str(target), 'sha256': sha(target)}
files = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
files += [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__),
          Path(__file__).with_name('PROTOCOL.md'), Path(__file__).with_name('audit.py')]
digests = {}
for p in files:
    name = p.relative_to(ROOT).as_posix()
    target = OUT / 'source' / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, target)
    digests[name] = sha(target)
save(OUT / 'source-frozen.json', {
    'parent': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
    'sources': digests, 'binaries': binaries})

def run(name, args, label):
    if name == 'activity-replay':
        save(OUT / f'{label}-model.json', check_model())
    start = time.monotonic()
    p = subprocess.run([binaries[name]['file'], *map(str, args)], cwd=ROOT,
                       capture_output=True, text=True, timeout=180,
                       creationflags=subprocess.CREATE_NO_WINDOW)
    receipt = {'exit_code': p.returncode, 'seconds': time.monotonic() - start,
               'stdout': p.stdout, 'stderr': p.stderr}
    save(OUT / f'{label}-process.json', receipt)
    print(json.dumps({'label': label, **receipt}), flush=True)
    assert p.returncode == 0

manifests = []
for i in range(2):
    run('runtime-appearance-replay', [OLD / f'case-{i}.json', 'cached', OUT / f'case-{i}',
        '--observations', OLD / f'case-{i}/result.json'], f'replay-{i}')
    replay = read(OUT / f'case-{i}/result.json')
    previous = read(OLD / f'case-{i}/result.json')
    assert replay['publication']['new_model_calls'] == 0
    assert replay['visual_tracks'] == previous['visual_tracks']
    assert replay['evidence']['evidence'] == previous['evidence']['evidence']
    new = replay['evidence']['automatic_region_motion']['appearance_sequences']
    old = previous['evidence']['automatic_region_motion']['appearance_sequences']
    assert new['error'] is None and new['activity']['error'] is None
    assert new['report']['attempted_pairs'] <= 12
    assert new['report']['selection_sha256'] == old['report']['selection_sha256']
    assert len(new['report']['series']) == len(old['report']['series'])
    for a, b in zip(old['report']['series'], new['report']['series']):
        assert a['source'] == b['source']
        for x, y in zip(a['steps'], b['steps']):
            assert {k: v for k, v in x.items() if k != 'source_sha256'} == {
                k: v for k, v in y.items() if k != 'source_sha256'}
    for augmented in [False, True]:
        manifest = read(PRIOR / f'case-{i}-baseline.json')
        if augmented:
            manifest['appearances'] = new['activity']['summary']
        path = OUT / f'case-{i}-{"augmented" if augmented else "baseline"}.json'
        save(path, manifest)
        manifests.append(path)
save(OUT / 'manifests-frozen.json', {str(p): sha(p) for p in manifests})
for manifest in manifests:
    run('activity-replay', [manifest, OUT / f'{manifest.stem}-receipt.json', MODEL], manifest.stem)
