"""Freeze the release binaries and source identities before native evaluation."""
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(sys.argv[1]).resolve()
OUT.mkdir(exist_ok=False)
(OUT / 'bin').mkdir()
names = ['Reversebolt.exe', 'reversebolt-mcp.exe', 'reversebolt-runtime-loader.exe',
         'reversebolt_runtime.dll', 'reversebolt-runtime-hypotheses.exe']
for name in names:
    shutil.copy2(ROOT / 'target/release' / name, OUT / 'bin' / name)
shutil.copy2(ROOT / 'test-results/runtime-hypotheses-v3/bin/reversebolt_path_fixture.dll', OUT / 'bin')
def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
sources = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__), Path(__file__).with_name('run_live.py')]
frozen = {'parent': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
          'sources': {p.relative_to(ROOT).as_posix(): sha(p) for p in sources},
          'binaries': {p.name: sha(p) for p in (OUT / 'bin').iterdir()}}
(OUT / 'frozen.json').write_text(json.dumps(frozen, indent=2), encoding='utf-8')
for name in ['workspace-final.log', 'clippy-final.log', 'build-final.log']:
    shutil.copy2(ROOT / 'test-results/runtime-hypotheses-v3' / name, OUT / name)
prior = json.loads((ROOT / 'test-results/runtime-hypotheses-v3/frozen.json').read_text(encoding='utf-8-sig'))
assert frozen['binaries']['reversebolt-runtime-hypotheses.exe'] == prior['binaries']['reversebolt-runtime-hypotheses.exe']
print(json.dumps({'root': str(OUT), 'binaries': len(frozen['binaries']), 'replay_cli_unchanged_from_v3': True}))
