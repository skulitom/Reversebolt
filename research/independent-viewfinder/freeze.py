"""Freeze the unchanged published product and existing generic native evaluator."""
import hashlib
import argparse
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser()
parser.add_argument('--output', default='test-results/independent-viewfinder-v1')
parser.add_argument('--bin', default='dist/0.1.0-20261022-233909')
parser.add_argument('--protocol', default='research/independent-viewfinder/PROTOCOL.md')
args = parser.parse_args()
OUT = ROOT / args.output
BIN = ROOT / args.bin
STORAGE = Path.home() / 'AppData/LocalLow/Sad Owl Studios/Viewfinder'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
OUT.mkdir()
shutil.copytree(STORAGE, OUT / 'storage-before')
sources = list((ROOT / 'crates').rglob('*.rs'))
sources += list((ROOT / 'crates').rglob('Cargo.toml'))
sources += [ROOT / p for p in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
    'research/independent-runtime/run.py', args.protocol,
    'research/independent-viewfinder/freeze.py']]
digests = {}
for path in sources:
    name = path.relative_to(ROOT).as_posix()
    dest = OUT / 'source' / name
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(path, dest)
    digests[name] = sha(dest)
manifest = Path('D:/STEAM/steamapps/appmanifest_1382070.acf')
shutil.copy2(manifest, OUT / 'appmanifest-before.acf')
frozen = {'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
    'bin': str(BIN), 'data': str(OUT / 'data'), 'appid': '1382070', 'sources': digests,
    'binaries': {p.name: sha(p) for p in BIN.iterdir() if p.suffix in ['.exe', '.dll']},
    'storage': str(STORAGE),
    'storage_before': {p.relative_to(STORAGE).as_posix(): sha(p) for p in STORAGE.rglob('*') if p.is_file()},
    'appmanifest_sha256': sha(manifest)}
(OUT / 'frozen.json').write_text(json.dumps(frozen, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'frozen': str(OUT), 'sources': len(digests), 'storage_files': len(frozen['storage_before'])}))
