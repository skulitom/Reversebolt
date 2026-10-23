"""Freeze a distinct continuation using an already settled partial review."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT=Path(__file__).resolve().parents[2]
PRIOR=ROOT/'test-results/runtime-siblings-sable-v1'
OUT=ROOT/'test-results/runtime-siblings-sable-selected-v1'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
prior=read(PRIOR/'frozen.json');discovery=read(PRIOR/'discovery.json')
assert read(PRIOR/'anode-job.json')['finished'] and read(PRIOR/'anode-job.json')['exitCode']==1
assert discovery['jobs']['in_flight'] is None and discovery['jobs']['fence'] is None
runtime=discovery['state']['live']['runtime']
receipt=Path(prior['data'])/'runtime/naming'/runtime['naming']['evidence_id']/'receipt.json'
report=Path(runtime['catalog_source']['path'])
assert runtime['naming']['status']=='unavailable' and read(receipt)['selection']['rows']
OUT.mkdir();(OUT/'analysis-bin').mkdir()
frozen={k:prior[k] for k in ['bin','data','binaries','storage','storage_before','appmanifest_sha256','max_batches','action_max_ms']}
frozen.update(prior=str(PRIOR),prior_frozen_sha256=sha(PRIOR/'frozen.json'),
    source_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    sources={},analysis={},inputs={},partial_review=True)
files=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
files += [ROOT/p for p in ['Cargo.toml','Cargo.lock','rust-toolchain.toml',
    'research/runtime-siblings/SELECTED-PROTOCOL.md','research/runtime-siblings/freeze-selected.py',
    'research/runtime-siblings/selected.py']]
for p in files:
    name=p.relative_to(ROOT).as_posix();destination=OUT/'source'/name
    destination.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,destination)
    frozen['sources'][name]=sha(destination)
for name in ['reversebolt-runtime-neighbors.exe','reversebolt-runtime-response.exe']:
    destination=OUT/'analysis-bin'/name;shutil.copy2(ROOT/'target/release'/name,destination)
    frozen['analysis'][name]=sha(destination)
for name,p in [('report',report),('receipt',receipt)]:
    destination=OUT/f'{name}.json';shutil.copy2(p,destination)
    frozen['inputs'][name]={'source':str(p),'sha256':sha(p)}
frozen['measurement_source']=str(PRIOR/'source/research/runtime-siblings/live.py')
frozen['measurement_source_sha256']=sha(Path(frozen['measurement_source']))
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2)+'\n')
run=subprocess.run([str(OUT/'analysis-bin/reversebolt-runtime-neighbors.exe'),str(OUT/'report.json'),
    str(OUT/'receipt.json'),str(OUT/'plan'),'--selected-types'],capture_output=True,text=True,timeout=120)
(OUT/'selection.stdout').write_text(run.stdout);(OUT/'selection.stderr').write_text(run.stderr)
assert run.returncode==0,run.stderr
print(run.stdout)
