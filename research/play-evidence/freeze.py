"""Freeze the built product and existing fixture, without starting them."""
import hashlib,json,shutil,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve();OUT.mkdir(exist_ok=False);(OUT/'bin').mkdir()
for name in ['Reversebolt.exe','reversebolt-mcp.exe','reversebolt-runtime-loader.exe','reversebolt_runtime.dll']:
    shutil.copy2(ROOT/'target/release'/name,OUT/'bin'/name)
shutil.copy2(ROOT/'test-results/hypothesis-tests-v2/bin/reversebolt_path_fixture.dll',OUT/'bin/reversebolt_path_fixture.dll')
paths=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))+[ROOT/'Cargo.toml',ROOT/'Cargo.lock',ROOT/'research/runtime-tests-live/host.py']
paths += [p for p in Path(__file__).parent.iterdir() if p.suffix in ('.py','.md')]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
frozen={'parent':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    'binaries':{p.name:sha(p) for p in (OUT/'bin').iterdir()},'sources':{p.relative_to(ROOT).as_posix():sha(p) for p in paths}}
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2),encoding='utf-8')
print(json.dumps({'frozen':str(OUT),'binaries':len(frozen['binaries'])}))
