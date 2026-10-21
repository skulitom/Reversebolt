"""Freeze product binaries and sources after checks and before the native run."""
import hashlib,json,shutil,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve()
for name in ['Reversebolt.exe','reversebolt-mcp.exe','reversebolt-runtime-loader.exe','reversebolt_runtime.dll']:
    shutil.copy2(ROOT/'target/release'/name,OUT/'bin'/name)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
paths=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
paths += [ROOT/'Cargo.toml',ROOT/'Cargo.lock',ROOT/'research/planned-input/host.py',ROOT/'research/runtime-paths/mock/src/lib.rs']
paths += [p for p in Path(__file__).parent.glob('*.py') if p.name != 'audit.py']
paths += [Path(__file__).with_name('PROTOCOL.md')]
frozen={'parent':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    'binaries':{p.name:sha(p) for p in (OUT/'bin').iterdir()},
    'sources':{p.relative_to(ROOT).as_posix():sha(p) for p in paths}}
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2))
print(json.dumps({'frozen':str(OUT),'binaries':len(frozen['binaries'])}))
