"""Freeze and replay existing fixture trials; no native or model operations."""
import hashlib, json, shutil, subprocess, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/trial-appearance-v1'

def read(p):
    return json.loads(p.read_text(encoding='utf-8-sig'))

def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

def save(p, value):
    with p.open('x', encoding='utf-8') as f:
        f.write(json.dumps(value, indent=2) + '\n')

OUT.mkdir()
records = sorted((ROOT / 'test-results/planned-input-v1/case/data/runtime-tests').glob('*.json'))
assert len(records) == 3
inputs, selections = {}, []
for p in records:
    test = read(p)
    assert test['candidate']['locator']['root']['declaring']['image'] == 'OwnedPathFixture'
    inputs[str(p)] = sha(p)
    for c in test['conditions']:
        for f in c['frames']:
            for key, digest in [('path', 'sha256'), ('input_path', 'input_sha256')]:
                path = p.parent / f[key]
                assert sha(path) == f[digest]
                inputs[str(path)] = sha(path)
    first = test['conditions'][0]['frames']
    if first:
        first = first[0]
        assert first['width'] >= 64 and first['height'] >= 64
        selection = {'baseline_sha256': first['sha256'], 'region': {
            'x': (first['width'] - 64) // 2, 'y': (first['height'] - 64) // 2,
            'width': 64, 'height': 64}}
    else:
        selection = {'baseline_sha256': '0' * 64,
                     'region': {'x': 0, 'y': 0, 'width': 64, 'height': 64}}
    selected = OUT / f'{p.stem}-selection.json'
    save(selected, selection)
    selections.append(selected)
    inputs[str(selected)] = sha(selected)
    old = ROOT / f'test-results/trial-motion-v1/{p.name}'
    inputs[str(old)] = sha(old)

binary = OUT / 'trial-motion-replay.exe'
shutil.copy2(ROOT / 'target/debug/examples/trial-motion-replay.exe', binary)
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
save(OUT / 'frozen.json', {'parent': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
    'sources': digests, 'inputs': inputs, 'binary': str(binary), 'binary_sha256': sha(binary)})
for record, selected in zip(records, selections):
    output = OUT / record.name
    start = time.monotonic()
    p = subprocess.run([str(binary), str(record), str(output), str(selected)], cwd=ROOT,
                       capture_output=True, text=True, timeout=60,
                       creationflags=subprocess.CREATE_NO_WINDOW)
    receipt = {'record': str(record), 'selection': str(selected), 'output': str(output),
               'seconds': time.monotonic() - start, 'exit_code': p.returncode,
               'stdout': p.stdout, 'stderr': p.stderr}
    save(OUT / f'{record.stem}-process.json', receipt)
    print(json.dumps(receipt), flush=True)
    assert p.returncode == 0
    result = read(output)
    old = read(ROOT / f'test-results/trial-motion-v1/{record.name}')
    assert result['trial_motion_comparison'] == old['trial_motion_comparison']
    assert result['original_assessment'] == read(record)['assessment'] == old['original_assessment']
