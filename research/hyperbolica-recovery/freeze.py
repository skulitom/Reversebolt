"""Retain the real completed history, then freeze a separate restart evaluation."""
import hashlib
import json
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/hyperbolica-recovery-v1'
PREVIOUS = ROOT / 'test-results/hyperbolica-effects-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
old = read(PREVIOUS / 'frozen.json')
test = read(PREVIOUS / 'comparison.json')['test']
assert test['phase'] == 'complete' and test['assessment']['status'] == 'suppression_observed'
assert test['recovery_recipe'] and read(PREVIOUS / 'cleanup.json')['owned_processes_exited']
BIN = ROOT / 'dist/0.1.0-20261023-093127'
assert all(sha(BIN / n) == h for n, h in old['binaries'].items())
assert all(sha(Path(old['game']) / n) == h for n, h in old['game_files'].items())
OUT.mkdir()
prior_data = Path(old['data'])
shutil.copytree(prior_data, OUT / 'data', ignore=lambda directory, names:
    ['endpoint.json'] if Path(directory) == prior_data and 'endpoint.json' in names else [])
copied = {p.relative_to(OUT / 'data').as_posix(): sha(p) for p in (OUT / 'data').rglob('*') if p.is_file()}
assert all(sha(prior_data / n) == h for n, h in copied.items())
historical = ['frozen.json', 'comparison.json', 'result.json', 'final-state.json', 'cleanup.json']
for name in historical:
    (OUT / 'historical').mkdir(exist_ok=True)
    shutil.copy2(PREVIOUS / name, OUT / 'historical' / name)
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / p for p in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
    'research/hyperbolica-recovery/PROTOCOL.md', 'research/hyperbolica-recovery/freeze.py',
    'research/hyperbolica-recovery/run.py']]
digests = {}
for source in sources:
    name = source.relative_to(ROOT).as_posix()
    destination = OUT / 'source' / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    digests[name] = sha(destination)
storage = []
for i, previous in enumerate(old['storage']):
    path = Path(previous['path'])
    shutil.copytree(path, OUT / 'storage-before' / str(i))
    storage.append({'path': str(path), 'backup': f'storage-before/{i}',
        'hashes': {p.relative_to(path).as_posix(): sha(p) for p in path.rglob('*') if p.is_file()}})
manifest = Path('D:/Steam/steamapps/appmanifest_1256230.acf')
shutil.copy2(manifest, OUT / 'appmanifest-before.acf')
frozen = {'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
    'bin': str(BIN), 'data': str(OUT / 'data'), 'previous': str(PREVIOUS), 'previous_data': str(prior_data),
    'copied_data': copied, 'historical': {n: sha(PREVIOUS / n) for n in historical},
    'sources': digests, 'binaries': old['binaries'], 'game': old['game'], 'game_files': old['game_files'],
    'appmanifest_sha256': sha(manifest), 'appid': 1256230, 'seat_session': 3, 'main_editor_pid': 98332,
    'storage': storage, 'source_test_id': test['id'], 'model_calls': 0, 'maximum_comparisons': 1,
    'maximum_setup_images': 4, 'maximum_setup_actions': 6}
(OUT / 'frozen.json').write_text(json.dumps(frozen, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'output': str(OUT), 'copied_files': len(copied), 'sources': len(digests),
    'source_test_id': test['id'], 'product_history_rewritten': False}))
