"""Frozen, file-only region measurement through the product's saved-pair path."""
import hashlib, json, shutil, subprocess, time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/region-motion-v1'
OUT.mkdir()
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f: return hashlib.file_digest(f,'sha256').hexdigest()
def save(p,v): p.write_text(json.dumps(v,indent=2)+'\n',encoding='utf-8')

binaries={}
for relative in ['target/debug/examples/motion-replay.exe','target/debug/examples/region-motion-fixture.exe']:
    binary=OUT/Path(relative).name
    shutil.copy2(ROOT/relative,binary)
    binaries[str(binary)]=sha(binary)
sources=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
sources += [ROOT/'Cargo.toml',ROOT/'Cargo.lock',Path(__file__),Path(__file__).with_name('PROTOCOL.md')]
digests={}
for p in sources:
    name=p.relative_to(ROOT).as_posix(); target=OUT/'source'/name
    target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,target);digests[name]=sha(target)
save(OUT/'source-frozen.json',{'parent':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    'sources':digests,'binaries':binaries})

def run(binary,args,name,timeout):
    start=time.monotonic()
    process=subprocess.run([str(OUT/binary),*map(str,args)],cwd=ROOT,capture_output=True,text=True,
                           timeout=timeout,creationflags=subprocess.CREATE_NO_WINDOW)
    receipt={'exit_code':process.returncode,'seconds':time.monotonic()-start,'stdout':process.stdout,'stderr':process.stderr}
    save(OUT/name,receipt);assert process.returncode==0,receipt
    return receipt
run('region-motion-fixture.exe',[OUT/'controls'],'fixture-process.json',30)
cases=read(OUT/'controls/cases.json')
prior_manifest=ROOT/'test-results/feature-motion-v1/cases.json'
prior_result=ROOT/'test-results/feature-motion-v1/result.json'
old=read(prior_manifest)
for name in ['Hyperbolica retained discovery','Manifold Garden retained discovery']:
    case=next(c for c in old if c['name']==name)
    frame=case['frames'][0]['reference'];w=frame['width'];h=frame['height']
    case['region']={'x':w//4,'y':h//4,'width':w//2,'height':h//2}
    cases.append(case)
assert len(cases)==8
manifest=OUT/'cases.json';save(manifest,cases)
inputs=[prior_manifest,prior_result,OUT/'controls/truth.json',OUT/'controls/cases.json',manifest]
for case in cases:
    for frame in case['frames']:
        file=Path(frame['file']);assert sha(file)==frame['reference']['sha256'];inputs.append(file)
save(OUT/'inputs-frozen.json',{str(p):sha(p) for p in inputs})
print(json.dumps(run('motion-replay.exe',[manifest,OUT/'result.json'],'process.json',60)))
