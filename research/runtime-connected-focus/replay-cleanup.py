"""Freeze and run a separate deterministic replay; original review stays intact."""
import hashlib
import json
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'test-results/runtime-connected-focus-v1'
OUT = BASE / 'cleanup-replay'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
assert read(BASE / 'audit.json')['passed']
OUT.mkdir()
(OUT / 'source').mkdir()
sources = [
    'crates/reversebolt-core/src/runtime_hypotheses.rs',
    'crates/reversebolt-core/examples/hypothesis_cleanup.rs',
    'crates/reversebolt-observer/src/runtime_naming.rs',
    'research/runtime-connected-focus/CLEANUP-PROTOCOL.md',
    'research/runtime-connected-focus/replay-cleanup.py',
]
frozen = {'sources': {}, 'original_receipt_sha256': sha(BASE / 'review/receipt.json')}
for name in sources:
    destination = OUT / 'source' / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / name, destination)
    frozen['sources'][name] = sha(destination)
binary = OUT / 'hypothesis_cleanup.exe'
shutil.copy2(ROOT / 'target/release/examples/hypothesis_cleanup.exe', binary)
frozen['binary_sha256'] = sha(binary)
(OUT / 'frozen.json').write_text(json.dumps(frozen, indent=2), encoding='utf-8')
with (OUT / 'started.json').open('x', encoding='utf-8') as marker:
    json.dump({'original_receipt_sha256': frozen['original_receipt_sha256']}, marker)
run = subprocess.run([str(binary), str(BASE / 'review/receipt.json'), str(OUT / 'normalized.json')],
                     capture_output=True, text=True, creationflags=subprocess.CREATE_NO_WINDOW)
(OUT / 'stdout.txt').write_text(run.stdout, encoding='utf-8')
(OUT / 'stderr.txt').write_text(run.stderr, encoding='utf-8')
(OUT / 'terminal.json').write_text(json.dumps({'exit_code': run.returncode}), encoding='utf-8')
print(run.stdout)
print(run.stderr)
raise SystemExit(run.returncode)
