import hashlib,json,shutil,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert not OUT.exists();OUT.mkdir();(OUT/'source').mkdir()
version=(ROOT/'dist/current.txt').read_text(encoding='utf-8-sig').strip();BIN=ROOT/'dist'/version
sources=[ROOT/'research/independent-runtime/run.py']+list((ROOT/'research/input-delivery').glob('*.*'))
frozen={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'version':version,'bin':str(BIN),'sources':{},'binaries':{},'hold_ms':1000,'methods':['legacy','send_input','send_input','legacy'],'keys':[68,65],'max_images':18}
for p in sources:
 name=p.relative_to(ROOT).as_posix();dest=OUT/'source'/name;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dest);frozen['sources'][name]=sha(dest)
for name in ['Reversebolt.exe','reversebolt-mcp.exe']:
 frozen['binaries'][name]=sha(BIN/name)
storage=Path('C:/Users/artem/AppData/LocalLow/Shedworks/Sable');frozen['storage']=str(storage)
frozen['storage_before']={str(p.relative_to(storage)):sha(p) for p in storage.rglob('*') if p.is_file()}
shutil.copytree(storage,OUT/'storage-before')
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2));print(OUT)
