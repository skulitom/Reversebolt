"""Freeze one native sibling measurement before game launch."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-siblings-sable-v1'
BIN = Path(sys.argv[1]).resolve()
assert BIN.parent == ROOT / 'dist'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
entries = json.loads((BIN / 'manifest.json').read_text(encoding='utf-8-sig'))
binaries = {e['name']: e['sha256'].lower() for e in entries}
assert set(binaries) == {'Reversebolt.exe','reversebolt-mcp.exe','reversebolt-runtime-loader.exe','reversebolt_runtime.dll'}
for name, digest in binaries.items():
    assert sha(BIN / name) == digest
OUT.mkdir()
(OUT / 'analysis-bin').mkdir()
analysis = {}
for name in ['reversebolt-runtime-neighbors.exe', 'reversebolt-runtime-response.exe']:
    destination = OUT / 'analysis-bin' / name
    shutil.copy2(ROOT / 'target/release' / name, destination)
    analysis[name] = sha(destination)
storage = Path.home() / 'AppData/LocalLow/Shedworks/Sable'
shutil.copytree(storage, OUT / 'storage-before')
files = list((ROOT/'crates').rglob('*.rs')) + list((ROOT/'crates').rglob('Cargo.toml'))
files += [ROOT/p for p in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
    'research/runtime-siblings/LIVE-PROTOCOL.md','research/runtime-siblings/freeze-live.py',
    'research/runtime-siblings/live.py', 'research/runtime-siblings/PROTOCOL.md']]
sources = {}
for p in files:
    name = p.relative_to(ROOT).as_posix()
    destination = OUT / 'source' / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, destination)
    sources[name] = sha(destination)
manifest = Path('D:/STEAM/steamapps/appmanifest_757310.acf')
shutil.copy2(manifest, OUT / 'appmanifest-before.acf')
frozen = dict(source_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    bin=str(BIN), data=str(OUT/'data'), sources=sources, binaries=binaries, analysis=analysis,
    appid='757310', buildid='10132013', development_test=True, storage=str(storage),
    storage_before={p.relative_to(storage).as_posix():sha(p) for p in storage.rglob('*') if p.is_file()},
    appmanifest_sha256=sha(manifest), max_batches=8, action_max_ms=3000, max_setup_images=4,
    manually_reviewed_measurement_images=0, gameplay_field_writes=0)
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2)+'\n')
print(json.dumps({'frozen':str(OUT),'bin':str(BIN),'source_files':len(sources)}))
