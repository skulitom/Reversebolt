"""Freeze the committed native trial method and exact built binaries."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
out = Path(sys.argv[1]).resolve()
assert not out.exists()
subprocess.run(['git', 'diff', '--quiet', 'HEAD'], cwd=root, check=True)
revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
binary = out / 'bin'
binary.mkdir(parents=True)
for name in ['Reversebolt.exe', 'reversebolt-mcp.exe', 'reversebolt-runtime-loader.exe',
             'reversebolt_runtime.dll', 'reversebolt-runtime-candidates.exe', 'reversebolt-runtime-patch.exe']:
    shutil.copy2(root / 'target/release' / name, binary / name)
shutil.copy2(root / 'test-results/runtime-paths-v2/bin/reversebolt_path_fixture.dll', binary / 'reversebolt_path_fixture.dll')
sha = lambda data: hashlib.sha256(data).hexdigest()
sources = {}
for path in subprocess.check_output(['git', 'ls-files'], cwd=root, text=True).splitlines():
    if path in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml') or path.startswith(('crates/', 'research/runtime-patch/', 'scripts/')) and path.endswith(('.rs', '.toml', '.py', '.ps1', '.md')):
        data = (root / path).read_bytes()
        committed = subprocess.check_output(['git', 'show', revision + ':' + path], cwd=root)
        assert data.replace(b'\r\n', b'\n') == committed.replace(b'\r\n', b'\n')
        sources[path] = {'working_sha256': sha(data), 'git_sha256': sha(committed)}
manifest = {'revision': revision, 'binaries': {p.name: sha(p.read_bytes()) for p in binary.iterdir()}, 'sources': sources}
(out / 'frozen.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
print(json.dumps({'revision': revision, 'binaries': len(manifest['binaries']), 'sources': len(sources)}))
