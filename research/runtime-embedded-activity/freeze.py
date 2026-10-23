import hashlib,json,shutil,subprocess,urllib.request,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
assert not OUT.exists();OUT.mkdir();(OUT/'bin').mkdir()
previous=ROOT/'test-results/runtime-values-v1'
assert read(previous/'audit.json')['passed']
sources=[p for d in ['crates/reversebolt-core/src','crates/reversebolt-observer/src','crates/reversebolt-runtime/src'] for p in (ROOT/d).rglob('*.rs')]
sources += list((ROOT/'research/runtime-embedded-activity').glob('*.*'))
sources += [ROOT/p for p in ['Cargo.toml','Cargo.lock','rust-toolchain.toml','crates/reversebolt-core/tests/runtime_paths.rs']]
frozen={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'sources':{},'binaries':{},
 'reader_source':str(previous),'reader_manifest_sha256':sha(previous/'frozen.json'),'max_batches':8,'max_fields':128,'hold_ms':5000,
 'model':'qwen3.8:latest','discovery_pages':1,'known_development_game':True,'gameplay_writes':0}
for p in sources:
 name=p.relative_to(ROOT).as_posix();dest=OUT/'source'/name;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dest);frozen['sources'][name]=sha(dest)
for name in ['reversebolt_runtime.dll','reversebolt-runtime-loader.exe']:
 source=previous/'bin'/name;assert sha(source)==read(previous/'frozen.json')['binaries'][name];shutil.copy2(source,OUT/'bin'/name);frozen['binaries'][name]=sha(source)
for name in ['reversebolt-runtime-neighbors.exe','reversebolt-runtime-response.exe','reversebolt-runtime-hypotheses.exe']:
 source=ROOT/'target/release'/name;shutil.copy2(source,OUT/'bin'/name);frozen['binaries'][name]=sha(source)
tags=json.load(urllib.request.urlopen('http://127.0.0.1:11434/api/tags'))
frozen['model_digest']=next(m['digest'] for m in tags['models'] if m['name']==frozen['model'])
frozen['server_version']=json.load(urllib.request.urlopen('http://127.0.0.1:11434/api/version'))['version']
storage=Path('C:/Users/artem/AppData/LocalLow/Shedworks/Sable')
preflight=ROOT/'test-results/runtime-embedded-activity-v1'
if (preflight/'preflight-failure.json').exists():
 frozen['storage']=str(storage);frozen['storage_before']=read(preflight/'frozen.json')['storage_before']
 frozen['storage_before_source']=str(preflight/'storage-before');shutil.copytree(preflight/'storage-before',OUT/'storage-before')
else:
 frozen['storage']=str(storage);frozen['storage_before']={str(p.relative_to(storage)):sha(p) for p in storage.rglob('*') if p.is_file()}
 shutil.copytree(storage,OUT/'storage-before')
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2))
print(json.dumps({'output':str(OUT),'sources':len(sources),'model_digest':frozen['model_digest']}))
