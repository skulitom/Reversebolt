"""Freeze the known-case image evaluation; no game or model execution."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/patch-motion-v3'
OUT.mkdir()

def read(p):
    return json.loads(p.read_text(encoding='utf-8-sig'))

def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

status = ROOT / 'test-results/independent-runtime-v2/discovery.json'
prior = ROOT / 'test-results/measured-contrast-v1/replay.json'
test_path = ROOT / 'test-results/planned-input-v1/case/data/runtime-tests/080b5643-5718-494f-9ed6-751bf8059402.json'
live = read(status)['state']['live']
native = read(ROOT / 'test-results/independent-runtime-v2/frozen.json')
captures = list((Path(native['data']) / 'captures').iterdir())
assert len(captures) == 1
inputs = [status, prior, test_path]
cases = [{'name': 'Hyperbolica retained discovery', 'frames': [],
          'expected_differences': [i['measurement']['difference'] for i in read(prior)['evidence']['intervals']]}]
for frame in live['frames']:
    p = captures[0] / f"frame-{frame['number']:03}.png"
    cases[0]['frames'].append({'file': str(p), 'reference': frame})
    inputs.append(p)
test = read(test_path)
assert test['candidate']['locator']['root']['declaring']['image'] == 'OwnedPathFixture'
for c, label in enumerate(['baseline', 'changed', 'recovered']):
    case = {'name': 'Owned striped fixture retained ' + label, 'frames': [],
            'expected_differences': [f['difference'] for f in test['conditions'][c]['frames'][1:]]}
    for i, frame in enumerate(test['conditions'][c]['frames']):
        p = test_path.parent / frame['path']
        input_path = test_path.parent / frame['input_path']
        inputs.extend([p,input_path])
        reference = {'id': f"{test['id']}:{c}:{i}", 'number': i+1, 'sha256': frame['sha256'],
                     'width': frame['width'], 'height': frame['height'], 'epoch': 0,
                     'started_unix_ms': frame['started_unix_ms'], 'capture_interval': frame['interval'],
                     'input_context': read(input_path)}
        case['frames'].append({'file': str(p), 'reference': reference})
    cases.append(case)
garden = ROOT / 'test-results/comparison-planner-v1'
garden_status = garden / 'discovery.json'
garden_evidence = garden / 'play-evidence.json'
garden_live = read(garden_status)['state']['live']
assert garden_live['process']['name'] == 'ManifoldGarden.exe'
garden_captures = list((garden / 'data/captures').iterdir())
assert len(garden_captures) == 1
inputs.extend([garden_status,garden_evidence,garden/'target.json',garden/'frozen.json'])
case = {'name':'Manifold Garden retained discovery','frames':[],
        'expected_differences':[i['measurement']['difference'] for i in read(garden_evidence)['data']['evidence']['intervals']]}
for frame in garden_live['frames']:
    p = garden_captures[0] / f"frame-{frame['number']:03}.png"
    case['frames'].append({'file':str(p),'reference':frame})
    inputs.append(p)
cases.append(case)
request = OUT / 'cases.json'
request.write_text(json.dumps(cases, indent=2) + '\n', encoding='utf-8')
inputs.append(request)
binary = ROOT / 'target/debug/examples/motion-replay.exe'
archived = OUT / binary.name
shutil.copy2(binary, archived)
assert sha(binary) == sha(archived)
assert sha(archived) == read(ROOT/'test-results/patch-motion-v1/frozen.json')['binary_sha256']
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__), Path(__file__).with_name('PROTOCOL.md'),Path(__file__).with_name('PROTOCOL-v2.md')]
digests = {}
for p in sources:
    name = p.relative_to(ROOT).as_posix()
    copy = OUT / 'source' / name
    copy.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, copy)
    digests[name] = sha(copy)
manifest = {'parent': subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
            'sources':digests,'inputs':{str(p):sha(p) for p in inputs},
            'binary':str(archived),'binary_sha256':sha(archived)}
(OUT / 'frozen.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
start = time.monotonic()
process = subprocess.run([str(archived),str(request),str(OUT/'result.json')],cwd=ROOT,capture_output=True,
                         text=True,timeout=60,creationflags=subprocess.CREATE_NO_WINDOW)
result = {'exit_code':process.returncode,'seconds':time.monotonic()-start,'stdout':process.stdout,'stderr':process.stderr}
(OUT/'process.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print(json.dumps(result))
assert process.returncode == 0
