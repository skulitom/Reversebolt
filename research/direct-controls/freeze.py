"""Retain exact product binaries and source identities before native evaluation."""
import hashlib, json, shutil, subprocess, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
out=Path(sys.argv[1]).resolve()
assert not out.exists()
subprocess.run(['git','diff','--quiet','HEAD'],cwd=ROOT,check=True)
revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
binary=out/'bin';binary.mkdir(parents=True)
names=['Reversebolt.exe','reversebolt-mcp.exe','reversebolt-runtime-loader.exe','reversebolt_runtime.dll','reversebolt-runtime-candidates.exe']
for name in names:
    shutil.copy2(ROOT/'target'/'release'/name,binary/name)
shutil.copy2(ROOT/'test-results'/'direct-controls-fixture-build'/'release'/'reversebolt_path_fixture.dll',binary/'reversebolt_path_fixture.dll')
paths=subprocess.check_output(['git','ls-files'],cwd=ROOT,text=True).splitlines()
sources={}
for path in paths:
    if path in ('Cargo.toml','Cargo.lock','rust-toolchain.toml','research/runtime-product/fixture_host.py') or path.startswith(('crates/','research/runtime-trials/','research/direct-controls/','research/runtime-paths/mock/','scripts/')) and path.endswith(('.rs','.toml','.py','.ps1','.md')):
        data=(ROOT/path).read_bytes()
        committed=subprocess.check_output(['git','show',revision+':'+path],cwd=ROOT)
        assert data.replace(b'\r\n',b'\n')==committed.replace(b'\r\n',b'\n'),path
        sources[path]={'working_sha256':hashlib.sha256(data).hexdigest(),'git_sha256':hashlib.sha256(committed).hexdigest()}
manifest={'revision':revision,'binaries':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in binary.iterdir()},'sources':sources}
(out/'frozen.json').write_text(json.dumps(manifest,indent=2))
print(json.dumps({'output':str(out),'revision':revision,'binaries':len(manifest['binaries']),'sources':len(sources)}))
