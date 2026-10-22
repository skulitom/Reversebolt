"""Freeze and replay retained observations without a live process or model."""
import copy, hashlib, json, shutil, subprocess, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/observed-regions-v1'
OUT.mkdir()

def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f: return hashlib.file_digest(f, 'sha256').hexdigest()
def save(p, v):
    with p.open('x', encoding='utf-8') as f: f.write(json.dumps(v, indent=2) + '\n')

source_case = ROOT / 'test-results/visual-objects-v1/tetris-1-attempt-01'
status = source_case / 'final-status.json'
live = read(status)['state']['live']
images = source_case / 'engine/captures/726934a1-2e14-4fa9-948b-e0d7ba7be155'
frames = [{'file': str(images / f"frame-{f['number']:03}.png"), 'reference': f}
          for f in live['frames']]
assert len(frames) == 16 and len(live['visual_tracks']) == 7
original = {'name': 'Retained Tetris observations', 'investigation_id': live['investigation_id'],
            'frames': frames, 'visual_tracks': live['visual_tracks']}
legacy = copy.deepcopy(original)
legacy.update(name='No retained observations (ablation)', visual_tracks=[])
gap = copy.deepcopy(original)
gap.update(name='Capture 3 omitted (ablation)', frames=[f for f in frames if f['reference']['number'] != 3])
manifest = OUT / 'cases.json'
save(manifest, [original, legacy, gap])
inputs = [status, manifest] + [Path(f['file']) for f in frames]
for frame in frames: assert sha(Path(frame['file'])) == frame['reference']['sha256']
save(OUT / 'inputs-frozen.json', {str(p): sha(p) for p in inputs})

binary = OUT / 'observed-region-replay.exe'
shutil.copy2(ROOT / 'target/debug/examples/observed-region-replay.exe', binary)
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__),
            Path(__file__).with_name('PROTOCOL.md'), Path(__file__).with_name('audit.py')]
digests = {}
for p in sources:
    name = p.relative_to(ROOT).as_posix()
    target = OUT / 'source' / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, target)
    digests[name] = sha(target)
save(OUT / 'source-frozen.json', {
    'parent': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
    'sources': digests, 'binaries': {str(binary): sha(binary)}})
start = time.monotonic()
result = subprocess.run([str(binary), str(manifest), str(OUT / 'result.json')], cwd=ROOT,
                        capture_output=True, text=True, timeout=60,
                        creationflags=subprocess.CREATE_NO_WINDOW)
receipt = {'exit_code': result.returncode, 'seconds': time.monotonic() - start,
           'stdout': result.stdout, 'stderr': result.stderr}
save(OUT / 'process.json', receipt)
print(json.dumps(receipt))
assert result.returncode == 0
