"""Prepare fixed cases and freeze the revised observer; baseline is already frozen."""
import hashlib,json,shutil,subprocess,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/motion-interpretation-v1'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def save(p,v): p.write_text(json.dumps(v,indent=2)+'\n',encoding='utf-8')
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
assert (OUT/'baseline/frozen.json').exists()
case_dir=OUT/'cases';case_dir.mkdir()
prior=ROOT/'test-results/patch-motion-v3/cases.json'
cases=read(prior)
hyper=ROOT/'test-results/measured-contrast-v1/replay.json'
garden=ROOT/'test-results/comparison-planner-v1/data/runtime/naming/d5e5aad6-b727-4233-ab81-24641f7dd9a8/play-source.json'
inputs=[prior,hyper,garden]
requests=[]
for name,source,case in [('hyperbolica',read(hyper)['revised_context']['source'],cases[0]),('manifold',read(garden)['selected'],cases[4])]:
    frames=[]
    lookup={f['reference']['id']:f for f in case['frames']}
    for pair in source['pairs']:
        for key in ['before','after']:
            f=lookup[pair[key]['id']]
            p=Path(f['file'])
            assert sha(p)==pair[key]['sha256']==f['reference']['sha256']
            frames.append(str(p));inputs.append(p)
    manifest=case_dir/(name+'.json');save(manifest,{'source':source,'frames':frames});inputs.append(manifest)
    requests.append({'name':name,'manifest':str(manifest),'expected_consistency':None,'synthetic':False})
generator=ROOT/'target/debug/examples/activity-controls.exe'
archive=OUT/'activity-controls.exe';shutil.copy2(generator,archive)
inputs.append(archive)
source=cases[0]['frames'][0]
inputs.append(Path(source['file']))
controls=OUT/'controls'
process=subprocess.run([str(archive),source['file'],source['reference']['sha256'],str(controls)],cwd=ROOT,capture_output=True,text=True,timeout=30,creationflags=subprocess.CREATE_NO_WINDOW)
save(OUT/'controls-process.json',{'exit_code':process.returncode,'stdout':process.stdout,'stderr':process.stderr})
assert process.returncode==0,process.stderr
for name,expected in [('consistent',True),('opposed',False)]:
    requests.append({'name':name,'manifest':str(controls/(name+'.json')),'expected_consistency':expected,'synthetic':True})
inputs.extend(controls.iterdir())
request_file=OUT/'cases.json';save(request_file,requests);inputs.append(request_file)
revised=OUT/'revised';revised.mkdir()
binary=revised/'activity-replay.exe';shutil.copy2(ROOT/'target/debug/examples/activity-replay.exe',binary)
sources=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
sources += [ROOT/'Cargo.toml',ROOT/'Cargo.lock',Path(__file__),Path(__file__).with_name('run.py'),Path(__file__).with_name('PROTOCOL.md')]
digests={}
for p in sources:
    name=p.relative_to(ROOT).as_posix();copy=revised/'source'/name;copy.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,copy);digests[name]=sha(copy)
with urllib.request.urlopen('http://127.0.0.1:11434/api/tags',timeout=3) as r:tags=json.load(r)
model=next(m for m in tags['models'] if m['name']=='qwen3.8:latest')
assert model['digest']==read(OUT/'baseline/frozen.json')['model']['digest']
with urllib.request.urlopen('http://127.0.0.1:11434/api/version',timeout=3) as r:version=json.load(r)
save(revised/'frozen.json',{'sources':digests,'binary':str(binary),'binary_sha256':sha(binary),'model':model,'server':version})
save(OUT/'inputs.json',{'inputs':{str(p):sha(p) for p in inputs},'cases':requests,
                     'generator_source_sha256':digests['crates/reversebolt-vision/examples/activity-controls.rs']})
print(json.dumps({'cases':len(requests),'inputs':len(inputs),'model':model['digest']}))
