import hashlib,json,shutil,subprocess,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'test-results/input-delivery-v1/state-selection'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
OUT.mkdir();(OUT/'source').mkdir();(OUT/'bin').mkdir()
prior=ROOT/'test-results/runtime-embedded-activity-v3';report=prior/'discovery.json'
assert sha(report)==read(prior/'selection/receipt.json')['report_sha256'] or sha(report)==read(prior/'result.json')['calls'][0]['report_sha256']
frozen={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'report':str(report),'report_sha256':sha(report),'prior_receipt_sha256':sha(prior/'selection/receipt.json'),'sources':{},'binaries':{},'model':'qwen3.8:latest','selection_purpose':'current_state','max_requests':24,'max_prompt_bytes':12000}
sources=[ROOT/'crates/reversebolt-observer/src/runtime_naming.rs',ROOT/'crates/reversebolt-observer/src/runtime_naming/selection.rs',ROOT/'crates/reversebolt-observer/src/runtime_naming/request.rs',ROOT/'crates/reversebolt-observer/src/bin/reversebolt-runtime-hypotheses.rs']+list((ROOT/'research/input-delivery').glob('*.*'))
for p in sources:
 name=p.relative_to(ROOT).as_posix();dest=OUT/'source'/name;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dest);frozen['sources'][name]=sha(dest)
for name in ['reversebolt-runtime-hypotheses.exe','reversebolt-runtime-neighbors.exe']:
 p=ROOT/'target/release'/name;shutil.copy2(p,OUT/'bin'/name);frozen['binaries'][name]=sha(p)
tags=json.load(urllib.request.urlopen('http://127.0.0.1:11434/api/tags'))
frozen['model_digest']=next(m['digest'] for m in tags['models'] if m['name']==frozen['model'])
frozen['server_version']=json.load(urllib.request.urlopen('http://127.0.0.1:11434/api/version'))['version']
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2));print(OUT)
