"""Freeze a saved-data review; never opens the game or changes its state."""
import hashlib
import json
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
assert sys.argv[1:] in [[], ['--connected']]
CONNECTED = bool(sys.argv[1:])
OUT = ROOT / ('test-results/runtime-connected-focus-v1' if CONNECTED else 'test-results/runtime-activity-focus-v1')
PRIOR = ROOT / 'test-results/runtime-state-activity-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
assert read(PRIOR / 'audit.json')['passed']
assert read(PRIOR / 'cleanup.json')['owned_still_running'] == []
OUT.mkdir()
(OUT / 'bin').mkdir()
(OUT / 'source').mkdir()
(OUT / 'input').mkdir()
frozen = {
    'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
    'sources': {}, 'inputs': {}, 'binaries': {}, 'model': 'qwen3.8:latest',
    'prior': str(PRIOR), 'prior_audit_sha256': sha(PRIOR / 'audit.json'),
    'max_selection_requests': 24, 'max_proposal_requests': 8,
    'manual_measurement_images': 0, 'gameplay_writes': 0,
    'connected_focus': CONNECTED,
}
sources = [p for folder in ['crates/reversebolt-core/src', 'crates/reversebolt-observer/src']
           for p in (ROOT / folder).rglob('*.rs')]
sources += list((ROOT / 'research/runtime-activity-focus').glob('*.*'))
if CONNECTED:
    sources += list((ROOT / 'research/runtime-connected-focus').glob('*.*'))
for source in sources:
    name = source.relative_to(ROOT).as_posix()
    destination = OUT / 'source' / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    frozen['sources'][name] = sha(destination)
inputs = ['discovery.json', 'plan/plan.json'] + [f'observations-{i}.json' for i in range(8)]
for name in inputs:
    destination = OUT / 'input' / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(PRIOR / name, destination)
    frozen['inputs'][name] = sha(destination)
manifest = {'plan': 'plan/plan.json', 'batches': [
    {'batch': i, 'observations': f'observations-{i}.json'} for i in range(8)]}
(OUT / 'input/manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
frozen['inputs']['manifest.json'] = sha(OUT / 'input/manifest.json')
name = 'reversebolt-runtime-hypotheses.exe'
shutil.copy2(ROOT / 'target/release' / name, OUT / 'bin' / name)
frozen['binaries'][name] = sha(OUT / 'bin' / name)
tags = json.load(urllib.request.urlopen('http://127.0.0.1:11434/api/tags'))
frozen['model_digest'] = next(m['digest'] for m in tags['models'] if m['name'] == frozen['model'])
frozen['server_version'] = json.load(urllib.request.urlopen('http://127.0.0.1:11434/api/version'))['version']
(OUT / 'frozen.json').write_text(json.dumps(frozen, indent=2), encoding='utf-8')
print(OUT)
