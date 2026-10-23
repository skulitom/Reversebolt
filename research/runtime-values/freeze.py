import hashlib, json, shutil, subprocess, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=Path(sys.argv[1]).resolve()
OUT.mkdir(parents=True,exist_ok=False)
(OUT/'bin').mkdir()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
sources=['crates/reversebolt-runtime/src/lib.rs','crates/reversebolt-runtime/src/paths.rs',
    'crates/reversebolt-runtime/src/values.rs','crates/reversebolt-core/src/runtime.rs',
    'crates/reversebolt-core/tests/runtime_paths.rs','research/runtime-values/PROTOCOL.md',
    'research/runtime-values/mock/src/lib.rs','research/runtime-values/mock/Cargo.toml',
    'research/runtime-values/mock/Cargo.lock','research/runtime-values/native.py',
    'research/runtime-values/freeze.py']
for relative in sources:
    destination=OUT/'source'/relative
    destination.parent.mkdir(parents=True,exist_ok=True)
    shutil.copy2(ROOT/relative,destination)
binaries=[ROOT/'target/release'/name for name in ['reversebolt_runtime.dll',
    'reversebolt-runtime-loader.exe','reversebolt-runtime-candidates.exe','reversebolt-runtime-patch.exe']]
binaries += [ROOT/'research/runtime-values/mock/target/release/reversebolt_value_fixture.dll',
    ROOT/'test-results/runtime-paths-v2/bin/object-host.exe',
    ROOT/'test-results/runtime-paths-v2/bin/reversebolt_path_fixture.dll']
for source in binaries:shutil.copy2(source,OUT/'bin'/source.name)
manifest={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    'sources':{p:sha(ROOT/p) for p in sources},'binaries':{p.name:sha(p) for p in binaries},
    'legacy_host_and_fixture':'test-results/runtime-paths-v2/bin', 'local_only':True}
(OUT/'frozen.json').write_text(json.dumps(manifest,indent=2))
print(json.dumps({'output':str(OUT),'binaries':manifest['binaries']}))
