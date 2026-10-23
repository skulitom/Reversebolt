"""Freeze one prospective native recovery test from an audited historical effect."""
import hashlib, json, shutil, subprocess, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
assert sys.argv[1:] in ([], ['--declared-route'], ['--declared-route', '--buffered-helper'])
declared = '--declared-route' in sys.argv[1:]
buffered = '--buffered-helper' in sys.argv[1:]
folder = 'runtime-recovery-declared-buffered-v1' if buffered else ('runtime-recovery-declared-v1' if declared else 'runtime-recovery-live-v1')
OUT=ROOT/'test-results'/folder
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert read(ROOT/'research/independent-viewfinder/live-results.json')['artifact_audit']=='passed'
prior=ROOT/'test-results/viewfinder-followup-v1'
tests=[read(p)['test'] for p in sorted(prior.glob('comparison-*.json')) if read(p)['test']['assessment']['status']=='suppression_observed']
assert len(tests)==1 and tests[0]['phase']=='complete'
OUT.mkdir();(OUT/'bin').mkdir();(OUT/'source').mkdir()
(OUT/'source-test.json').write_text(json.dumps(tests[0],indent=2),encoding='utf-8')
source_path=Path(read(prior/'final-state.json')['state']['live']['runtime']['catalog_source']['path'])
shutil.copy2(source_path,OUT/'source-report.json')
version=(ROOT/'dist/current.txt').read_text(encoding='utf-8-sig').strip()
frozen={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    'preview':version,'sources':{},'binaries':{},'historical':{},'maximum_comparisons':1,
    'maximum_setup_images':3,'maximum_menu_actions':2,'model_calls':0,
    'recovery_policy':'declared_route_proposal' if declared else 'strict',
    'buffered_helper':buffered}
for name in ['Reversebolt.exe','reversebolt-mcp.exe','reversebolt-runtime-loader.exe','reversebolt_runtime.dll']:
    binary = ROOT/'target/release'/name if buffered and name=='reversebolt_runtime.dll' else ROOT/'dist'/version/name
    shutil.copy2(binary,OUT/'bin'/name);frozen['binaries'][name]=sha(OUT/'bin'/name)
shutil.copy2(ROOT/'target/release/examples/runtime_recovery.exe',OUT/'bin/runtime_recovery.exe')
frozen['binaries']['runtime_recovery.exe']=sha(OUT/'bin/runtime_recovery.exe')
for p in list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'research/runtime-recovery').glob('*.*')):
    name=p.relative_to(ROOT).as_posix();dest=OUT/'source'/name;dest.parent.mkdir(parents=True,exist_ok=True)
    shutil.copy2(p,dest);frozen['sources'][name]=sha(dest)
for name in ['source-test.json','source-report.json']:frozen['historical'][name]=sha(OUT/name)
storage=Path(read(ROOT/'test-results/viewfinder-live-paging-v1/frozen.json')['storage'])
frozen['storage']=str(storage);frozen['storage_before']={p.relative_to(storage).as_posix():sha(p) for p in storage.rglob('*') if p.is_file()}
shutil.copytree(storage,OUT/'storage-before')
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2),encoding='utf-8')
print(OUT)
