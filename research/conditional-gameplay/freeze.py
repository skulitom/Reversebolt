"""Freeze one read-only gameplay baseline and retain pre-launch storage."""
import hashlib
import json
import shutil
import subprocess
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/conditional-gameplay-v1'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
OUT.mkdir()
(OUT / 'bin').mkdir()
(OUT / 'source').mkdir()
version = (ROOT / 'dist/current.txt').read_text(encoding='utf-8-sig').strip()
published = ROOT / 'dist' / version
frozen = {'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
          'preview': version, 'sources': {}, 'binaries': {}, 'model': 'qwen3.8:latest',
          'max_images': 15, 'interval_ms': 1500, 'hold_ms': 1000, 'max_setup_images': 4,
          'measurement_images_viewed': 0, 'gameplay_writes': 0}
for name in ['Reversebolt.exe', 'reversebolt-mcp.exe', 'reversebolt-runtime-loader.exe', 'reversebolt_runtime.dll']:
    shutil.copy2(published / name, OUT / 'bin' / name)
    frozen['binaries'][name] = sha(OUT / 'bin' / name)
for name in ['activity-replay.exe', 'activity-question-plan.exe', 'test-design-replay.exe']:
    shutil.copy2(ROOT / 'target/release/examples' / name, OUT / 'bin' / name)
    frozen['binaries'][name] = sha(OUT / 'bin' / name)
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'research/conditional-gameplay').glob('*.*'))
sources.append(ROOT / 'research/independent-runtime/run.py')
for source in sources:
    name = source.relative_to(ROOT).as_posix()
    destination = OUT / 'source' / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    frozen['sources'][name] = sha(destination)
questions = ROOT / 'test-results/runtime-connected-focus-v1/cleanup-replay/normalized.json'
assert read(questions.parent / 'audit.json')['passed']
shutil.copy2(questions, OUT / 'historical-questions.json')
frozen['historical_questions_sha256'] = sha(OUT / 'historical-questions.json')
storage = Path('C:/Users/artem/AppData/LocalLow/Shedworks/Sable')
frozen['storage'] = str(storage)
frozen['storage_before'] = {p.relative_to(storage).as_posix(): sha(p) for p in storage.rglob('*') if p.is_file()}
shutil.copytree(storage, OUT / 'storage-before')
tags = json.load(urllib.request.urlopen('http://127.0.0.1:11434/api/tags'))
frozen['model_digest'] = next(m['digest'] for m in tags['models'] if m['name'] == frozen['model'])
frozen['server_version'] = json.load(urllib.request.urlopen('http://127.0.0.1:11434/api/version'))['version']
(OUT / 'frozen.json').write_text(json.dumps(frozen, indent=2), encoding='utf-8')
print(OUT)
