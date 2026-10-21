"""Freeze the existing product and protocol, and back up game saves before launch."""
import hashlib,json,shutil,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve();OUT.mkdir(exist_ok=False);(OUT/'bin').mkdir()
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
version=(ROOT/'dist/current.txt').read_text(encoding='utf-8-sig').strip()
for name in ['Reversebolt.exe','reversebolt-mcp.exe','reversebolt-runtime-loader.exe','reversebolt_runtime.dll']:
    shutil.copy2(ROOT/'dist'/version/name,OUT/'bin'/name)
storage=Path('C:/Users/artem/AppData/LocalLow/William Chyr Studio/Manifold Garden')
shutil.copytree(storage,OUT/'storage-before')
sources=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
sources += [ROOT/'Cargo.lock',ROOT/'Cargo.toml']+[p for p in Path(__file__).parent.iterdir() if p.suffix in ('.py','.md')]
frozen={'revision':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'preview':version,
    'product_binaries':{p.name:sha(p) for p in (OUT/'bin').iterdir()},
    'sources':{p.relative_to(ROOT).as_posix():sha(p) for p in sources},
    'storage':str(storage),'storage_before':{str(p.relative_to(storage)):sha(p) for p in storage.rglob('*') if p.is_file()},
    'manifest_sha256':sha(Path('D:/Steam/steamapps/appmanifest_473950.acf'))}
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2))
print(json.dumps({'frozen':str(OUT),'preview':version,'backed_up_files':len(frozen['storage_before'])}))
