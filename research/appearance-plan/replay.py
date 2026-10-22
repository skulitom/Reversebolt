"""Freeze a compatibility replay and audit exact old/new results."""
import hashlib, json, shutil, subprocess, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OLD = ROOT / 'test-results/trial-appearance-v1'
OUT = ROOT / 'test-results/appearance-plan-v1'

def read(p):
    return json.loads(p.read_text(encoding='utf-8-sig'))

def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

def save(p, value):
    with p.open('x', encoding='utf-8') as f:
        f.write(json.dumps(value, indent=2) + '\n')

OUT.mkdir()
inputs = read(OLD / 'frozen.json')['inputs']
inputs[str(OLD / 'frozen.json')] = sha(OLD / 'frozen.json')
processes = [read(p) for p in sorted(OLD.glob('*-process.json'))]
assert len(processes) == 3
for previous in processes:
    output = Path(previous['output'])
    inputs[str(output)] = sha(output)
for name, digest in inputs.items():
    assert sha(Path(name)) == digest, name
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__), Path(__file__).with_name('PROTOCOL.md')]
hashes = {}
for p in sources:
    name = p.relative_to(ROOT).as_posix()
    target = OUT / 'source' / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, target)
    hashes[name] = sha(target)
binary = OUT / 'trial-motion-replay.exe'
shutil.copy2(ROOT / 'target/debug/examples/trial-motion-replay.exe', binary)
save(OUT / 'frozen.json', {
    'parent': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
    'sources': hashes, 'inputs': inputs, 'binary': str(binary), 'binary_sha256': sha(binary)})
results = []
for previous in processes:
    record = Path(previous['record'])
    selection = Path(previous['selection'])
    test = read(record)
    assert test.get('appearance') is None
    assert test['candidate']['locator']['root']['declaring']['image'] == 'OwnedPathFixture'
    output = OUT / record.name
    start = time.monotonic()
    process = subprocess.run([str(binary), str(record), str(output), str(selection)],
                             cwd=ROOT, capture_output=True, text=True, timeout=60,
                             creationflags=subprocess.CREATE_NO_WINDOW)
    receipt = {'exit_code': process.returncode, 'seconds': time.monotonic()-start,
               'stdout': process.stdout, 'stderr': process.stderr}
    save(OUT / f'{record.stem}-process.json', receipt)
    assert process.returncode == 0, receipt
    actual = read(output)
    assert actual == read(Path(previous['output'])), record.name
    regional = actual['trial_appearance_comparison']
    results.append({'test_id': test['id'], 'exact_previous_result': True,
                    'appearance_status': regional['report']['status'] if regional['report'] else None,
                    'appearance_error': regional['error'], 'output_sha256': sha(output)})
frozen = read(OUT / 'frozen.json')
for name, digest in frozen['sources'].items():
    assert sha(ROOT / name) == sha(OUT / 'source' / name) == digest, name
for name, digest in frozen['inputs'].items():
    assert sha(Path(name)) == digest, name
assert sha(binary) == frozen['binary_sha256']
audit = {'frozen_manifest_sha256': sha(OUT / 'frozen.json'), 'source_files': len(hashes),
         'bound_input_files': len(inputs), 'provenance': 'OwnedPathFixture',
         'new_plans_added_to_old_records': 0, 'native_target_access': False,
         'model_calls': 0, 'results': results}
save(OUT / 'audit.json', audit)
print(json.dumps(audit, indent=2))
