"""Freeze a staged current build for one native development retest."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/sable-live-context-v1'
BIN=Path(sys.argv[1]).resolve()
assert BIN.parent==ROOT/'dist'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
entries=json.loads((BIN/'manifest.json').read_text(encoding='utf-8-sig'))
binaries={entry['name']:entry['sha256'].lower() for entry in entries}
assert set(binaries)=={'Reversebolt.exe','reversebolt-mcp.exe','reversebolt-runtime-loader.exe','reversebolt_runtime.dll'}
for name,digest in binaries.items():assert sha(BIN/name)==digest
OUT.mkdir()
storage=Path.home()/'AppData/LocalLow/Shedworks/Sable'
shutil.copytree(storage,OUT/'storage-before')
files=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
files += [ROOT/p for p in ('Cargo.toml','Cargo.lock','rust-toolchain.toml',
    'research/independent-runtime/run.py','research/independent-sable/LIVE-RETEST-PROTOCOL.md',
    'research/independent-sable/freeze-live.py','research/independent-sable/gpu_trace.py',
    'scripts/publish-preview.ps1')]
sources={}
for p in files:
    name=p.relative_to(ROOT).as_posix();dest=OUT/'source'/name
    dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dest);sources[name]=sha(dest)
manifest=Path('D:/STEAM/steamapps/appmanifest_757310.acf')
shutil.copy2(manifest,OUT/'appmanifest-before.acf')
frozen=dict(source_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    bin=str(BIN),data=str(OUT/'data'),sources=sources,binaries=binaries,
    appid='757310',buildid='10132013',development_retest=True,
    storage=str(storage),storage_before={p.relative_to(storage).as_posix():sha(p)
        for p in storage.rglob('*') if p.is_file()},appmanifest_sha256=sha(manifest),
    max_comparisons=1,max_setup_images=4,manually_reviewed_measurement_images=0)
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2)+'\n')
print(json.dumps(dict(frozen=str(OUT),sources=len(sources),bin=str(BIN))))
