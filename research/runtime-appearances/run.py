"""One frozen local-model pass per saved runtime case; no live target access."""
import hashlib, json, shutil, subprocess, time, urllib.request
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-appearances-v1'
MODEL = 'qwen3.8:latest'
DIGEST = '22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f: return hashlib.file_digest(f, 'sha256').hexdigest()
def save(p, v):
    with p.open('x', encoding='utf-8') as f: f.write(json.dumps(v, indent=2) + '\n')
with urllib.request.urlopen('http://127.0.0.1:11434/api/tags', timeout=5) as r:
    tags = json.load(r)
assert any(m['name'] == MODEL and m['digest'] == DIGEST for m in tags['models'])
OUT.mkdir()
save(OUT / 'model-before.json', tags)
prior = ROOT / 'test-results/patch-motion-v3/cases.json'
cases = read(prior)
statuses = [ROOT / 'test-results/independent-runtime-v2/discovery.json',
            ROOT / 'test-results/comparison-planner-v1/discovery.json']
inputs = [prior, *statuses, OUT / 'model-before.json']
manifests = []
for index, (old, status) in enumerate(zip([cases[0], cases[4]], statuses, strict=True)):
    live = read(status)['state']['live']
    assert live['frames'] == [f['reference'] for f in old['frames']]
    assert not live['visual_tracks'] and len(live['frames']) == 15
    case = dict(name=old['name'], investigation_id=live['investigation_id'],
                batch=live['coverage']['batch'], frames=old['frames'])
    path = OUT / f'case-{index}.json'; save(path, case); manifests.append(path)
    inputs.append(path)
    for frame in case['frames']:
        image = Path(frame['file']); assert sha(image) == frame['reference']['sha256']; inputs.append(image)
save(OUT / 'inputs-frozen.json', {str(p): sha(p) for p in inputs})
binary = OUT / 'runtime-appearance-replay.exe'
shutil.copy2(ROOT / 'target/debug/examples/runtime-appearance-replay.exe', binary)
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__),
            Path(__file__).with_name('PROTOCOL.md'), Path(__file__).with_name('audit.py')]
digests = {}
for p in sources:
    relative = p.relative_to(ROOT).as_posix(); target = OUT / 'source' / relative
    target.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(p, target)
    digests[relative] = sha(target)
save(OUT / 'source-frozen.json', {'parent': subprocess.check_output(['git','rev-parse','HEAD'], cwd=ROOT, text=True).strip(),
    'sources': digests, 'binary': {'file': str(binary), 'sha256': sha(binary)}})
for index, manifest in enumerate(manifests):
    start = time.monotonic()
    process = subprocess.run([str(binary), str(manifest), MODEL, str(OUT / f'case-{index}')],
        cwd=ROOT, capture_output=True, text=True, timeout=300, creationflags=subprocess.CREATE_NO_WINDOW)
    receipt = {'exit_code': process.returncode, 'seconds': time.monotonic()-start,
               'stdout': process.stdout, 'stderr': process.stderr}
    save(OUT / f'case-{index}-process.json', receipt)
    print(json.dumps(dict(case=index, **receipt)), flush=True)
    assert process.returncode == 0
