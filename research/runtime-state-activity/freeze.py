import hashlib,json,shutil,subprocess,sys,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert not OUT.exists();OUT.mkdir();(OUT/'source').mkdir();(OUT/'bin').mkdir()
native=ROOT/'test-results/runtime-embedded-activity-v3';selection=ROOT/'test-results/input-delivery-v1/state-selection'
assert read(native/'audit.json')['passed'] and read(selection/'audit.json')['passed']
version=(ROOT/'dist/current.txt').read_text(encoding='utf-8-sig').strip();published=ROOT/'dist'/version
frozen={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'sources':{},'binaries':{},'origins':{},'capture_version':version,'max_batches':8,'max_fields':128,'hold_ms':5000,'capture_samples':[0,1,2,4,5,6],'max_images':48,'model':'qwen3.8:latest','selection_purpose':'current_state','discovery_pages':1}
sources=[p for d in ['crates/reversebolt-core/src','crates/reversebolt-observer/src','crates/reversebolt-runtime/src'] for p in (ROOT/d).rglob('*.rs')]
sources += [ROOT/'research/runtime-embedded-activity/live.py',ROOT/'research/independent-runtime/run.py']+list((ROOT/'research/runtime-state-activity').glob('*.*'))
for p in sources:
 name=p.relative_to(ROOT).as_posix();dest=OUT/'source'/name;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dest);frozen['sources'][name]=sha(dest)
for name in ['reversebolt_runtime.dll','reversebolt-runtime-loader.exe','reversebolt-runtime-neighbors.exe','reversebolt-runtime-response.exe','reversebolt-runtime-hypotheses.exe','Reversebolt.exe','reversebolt-mcp.exe']:
 origin=selection/'bin' if name=='reversebolt-runtime-hypotheses.exe' else published if name in ['Reversebolt.exe','reversebolt-mcp.exe'] else native/'bin'
 p=origin/name;shutil.copy2(p,OUT/'bin'/name);frozen['binaries'][name]=sha(p);frozen['origins'][name]=str(p)
 if origin==native/'bin':assert sha(p)==read(native/'frozen.json')['binaries'][name]
 if origin==selection/'bin':assert sha(p)==read(selection/'frozen.json')['binaries'][name]
tags=json.load(urllib.request.urlopen('http://127.0.0.1:11434/api/tags'))
frozen['model_digest']=next(m['digest'] for m in tags['models'] if m['name']==frozen['model'])
frozen['server_version']=json.load(urllib.request.urlopen('http://127.0.0.1:11434/api/version'))['version']
storage=Path('C:/Users/artem/AppData/LocalLow/Shedworks/Sable');frozen['storage']=str(storage)
frozen['storage_before']={str(p.relative_to(storage)):sha(p) for p in storage.rglob('*') if p.is_file()}
shutil.copytree(storage,OUT/'storage-before')
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2));print(OUT)
