"""Freeze unchanged previously evaluated product, new protocol and save backup."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
PARENT = ROOT/'test-results/viewfinder-live-paging-v1'
OUT = ROOT/'test-results/independent-sable-v1'
STORAGE = Path.home()/'AppData/LocalLow/Shedworks/Sable'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
prior = read(PARENT/'frozen.json')
BIN = Path(prior['bin'])
assert BIN.name == '0.1.0-20261022-235934'
for name, digest in prior['binaries'].items():
    assert sha(BIN/name) == digest
OUT.mkdir()
shutil.copytree(STORAGE, OUT/'storage-before')
shutil.copy2(PARENT/'frozen.json', OUT/'product-parent-frozen.json')
hashes = {}
for name, digest in prior['sources'].items():
    source = PARENT/'source'/name
    assert sha(source) == digest
    destination = OUT/'source'/name
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    hashes[name] = sha(destination)
for name in ('research/independent-runtime/run.py', 'research/independent-viewfinder/continue.py',
             'research/independent-viewfinder/PROTOCOL-FOLLOWUP.md',
             'research/independent-sable/PROTOCOL.md', 'research/independent-sable/freeze.py'):
    source = ROOT/name
    destination = OUT/'source'/name
    if destination.exists():
        assert sha(source) == sha(destination), 'Existing evaluator changed'
    else:
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)
    hashes[name] = sha(destination)
manifest = Path('D:/STEAM/steamapps/appmanifest_757310.acf')
shutil.copy2(manifest, OUT/'appmanifest-before.acf')
frozen = dict(source_commit=prior['source_commit'],
    product_source_provenance='Exact archived source from the earlier unchanged-binary native evaluation',
    harness_repository_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
    product_parent_frozen_sha256=sha(PARENT/'frozen.json'), bin=str(BIN), data=str(OUT/'data'),
    appid='757310', buildid='10132013', binaries=prior['binaries'], sources=hashes,
    storage=str(STORAGE), storage_before={p.relative_to(STORAGE).as_posix():sha(p)
        for p in STORAGE.rglob('*') if p.is_file()}, appmanifest_sha256=sha(manifest),
    max_comparisons=5, max_manually_reviewed_setup_images=4,
    manually_reviewed_measurement_images=0)
(OUT/'frozen.json').write_text(json.dumps(frozen, indent=2)+'\n')
print(json.dumps(dict(frozen=str(OUT), sources=len(hashes), storage_files=len(frozen['storage_before']))))
