"""Freeze a two-process product integration run before any gameplay."""
import hashlib, json, shutil, subprocess, sys
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
OUT=Path(sys.argv[1]).resolve()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
prior=ROOT/'test-results/runtime-recovery-declared-continuation-v1'
old=read(prior/'source-test.json')
assert old['phase']=='complete' and old['assessment']['status']=='suppression_observed'
assert old.get('recovery_recipe') is None
version=(ROOT/'dist/current.txt').read_text(encoding='utf-8-sig').strip()
OUT.mkdir();(OUT/'bin').mkdir();(OUT/'source').mkdir()
frozen={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        'preview':version,'appid':1382070,'seat_session':3,'main_editor_pid':98332,
        'sources':{},'binaries':{},'historical':{},'maximum_comparisons_per_process':1,
        'maximum_setup_images_per_process':3,'maximum_menu_actions_per_process':2,'model_calls':0}
for name in ['Reversebolt.exe','reversebolt-mcp.exe','reversebolt-runtime-loader.exe','reversebolt_runtime.dll']:
    shutil.copy2(ROOT/'dist'/version/name,OUT/'bin'/name)
    frozen['binaries'][name]=sha(OUT/'bin'/name)
shutil.copy2(ROOT/'target/release/examples/runtime_recovery.exe',OUT/'bin/runtime_recovery.exe')
frozen['binaries']['runtime_recovery.exe']=sha(OUT/'bin/runtime_recovery.exe')
for p in list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'research/runtime-recovery').glob('*.*')):
    name=p.relative_to(ROOT).as_posix();dest=OUT/'source'/name;dest.parent.mkdir(parents=True,exist_ok=True)
    shutil.copy2(p,dest);frozen['sources'][name]=sha(dest)
for name in ['source-test.json','source-report.json']:
    shutil.copy2(prior/name,OUT/name);frozen['historical'][name]=sha(OUT/name)
storage=Path(read(ROOT/'test-results/viewfinder-live-paging-v1/frozen.json')['storage'])
frozen['storage']=str(storage)
frozen['storage_before']={p.relative_to(storage).as_posix():sha(p) for p in storage.rglob('*') if p.is_file()}
shutil.copytree(storage,OUT/'storage-before')
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2),encoding='utf-8')
print(OUT)
