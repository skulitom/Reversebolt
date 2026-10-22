"""One frozen file-only check; no target, editor, loader or model is invoked."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/file-export-detection-v1'
OUT.mkdir()
STEAM = Path('D:/STEAM/steamapps/common')
cases = [
    ('hyperbolica-runtime', STEAM / 'Hyperbolica/GameAssembly.dll', True, False),
    ('freight-hopper-runtime', STEAM / 'FreightHopper/GameAssembly.dll', True, False),
    ('manifold-runtime', STEAM / 'Manifold Garden/GameAssembly.dll', True, False),
    ('hyperbolica-player', STEAM / 'Hyperbolica/UnityPlayer.dll', False, False),
    ('freight-hopper-player', STEAM / 'FreightHopper/UnityPlayer.dll', False, False),
    ('hyperbolica-executable', STEAM / 'Hyperbolica/Hyperbolica.exe', False, False),
    ('windows-kernel', Path('C:/Windows/System32/kernel32.dll'), False, False),
    ('reversebolt-helper', ROOT / 'dist/0.1.0-20261022-010125/reversebolt_runtime.dll', False, True),
]
def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

binary = ROOT / 'target/debug/examples/file-exports.exe'
sources = list((ROOT / 'crates/reversebolt-runtime').rglob('*.rs'))
sources += [ROOT / name for name in ['Cargo.toml', 'Cargo.lock', 'crates/reversebolt-runtime/Cargo.toml',
                                   'crates/reversebolt-app/src/runtime_bridge.rs']]
sources += [Path(__file__), Path(__file__).with_name('PROTOCOL.md')]
digests = {}
for path in sources:
    name = path.relative_to(ROOT).as_posix()
    copy = OUT / 'source' / name
    copy.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(path, copy)
    digests[name] = sha(copy)
frozen = {'parent': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
          'sources': digests, 'binary': str(binary), 'binary_sha256': sha(binary),
          'cases': [{'name': n, 'path': str(p), 'sha256': sha(p), 'bytes': p.stat().st_size,
                     'expected_runtime': r, 'expected_helper': h} for n, p, r, h in cases]}
(OUT / 'frozen.json').write_text(json.dumps(frozen, indent=2) + '\n', encoding='utf-8')
start = time.monotonic()
process = subprocess.run([str(binary)] + [str(p) for _, p, _, _ in cases], cwd=ROOT,
                         capture_output=True, text=True, timeout=30,
                         creationflags=subprocess.CREATE_NO_WINDOW)
(OUT / 'process.json').write_text(json.dumps({'exit_code': process.returncode, 'seconds': time.monotonic() - start,
    'stdout': process.stdout, 'stderr': process.stderr}, indent=2) + '\n', encoding='utf-8')
rows = [json.loads(line) for line in process.stdout.splitlines()]
assert process.returncode == 0 and len(rows) == len(cases), 'Retained failed inspection'
summary = []
for case, row in zip(frozen['cases'], rows):
    value = row['inspection']
    assert value['path'] == case['path']
    assert sha(Path(case['path'])) == case['sha256']
    assert value['runtime_provider'] == case['expected_runtime']
    assert ('ReverseboltInspect' in value['exports']['functions']) == case['expected_helper']
    assert not value['exports']['forwarded']
    assert 0 < value['exports']['read_bytes'] <= 8 * 1024 * 1024
    assert row['target_access'] is False and row['dll_mapping'] is False
    summary.append({'name': case['name'], 'runtime_provider': value['runtime_provider'],
                    'matched_functions': len(value['exports']['functions']),
                    'file_bytes': case['bytes'], 'read_bytes': value['exports']['read_bytes'],
                    'elapsed_ms': row['elapsed_ms']})
result = {'file_only_evaluation_passed': True, 'cases': summary, 'target_access': False, 'dll_mapping': False,
          'native_loader_integration_tested': False, 'windows_detection_resolved': False,
          'frozen_manifest_sha256': sha(OUT / 'frozen.json'), 'process_record_sha256': sha(OUT / 'process.json')}
(ROOT / 'research/file-export-detection/results.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
print(json.dumps(result))
