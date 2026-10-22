"""Frozen cached-observation replay plus a four-call activity-context ablation."""
import hashlib, json, shutil, subprocess, time, urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/appearance-context-v1'
OLD=ROOT/'test-results/runtime-appearances-v1'
MODEL='qwen3.8:latest'
DIGEST='22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(p,v):
    with p.open('x',encoding='utf-8') as f:f.write(json.dumps(v,indent=2)+'\n')
with urllib.request.urlopen('http://127.0.0.1:11434/api/tags',timeout=5) as r: tags=json.load(r)
assert any(m['name']==MODEL and m['digest']==DIGEST for m in tags['models'])
OUT.mkdir();save(OUT/'model-before.json',tags)
sources=[ROOT/'test-results/measured-contrast-v1/replay.json',
    ROOT/'test-results/comparison-planner-v1/data/runtime/naming/d5e5aad6-b727-4233-ab81-24641f7dd9a8/play-source.json']
inputs=[*sources,OUT/'model-before.json']
for i in range(2):
    case=OLD/f'case-{i}.json'; inputs += [case,OLD/f'case-{i}'/'result.json']
    inputs += [Path(f['file']) for f in read(case)['frames']]
save(OUT/'inputs-frozen.json',{str(p):sha(p) for p in inputs})
binaries={}
for name in ['runtime-appearance-replay','activity-replay']:
    source=ROOT/f'target/debug/examples/{name}.exe';target=OUT/source.name;shutil.copy2(source,target)
    binaries[name]={'file':str(target),'sha256':sha(target)}
files=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
files += [ROOT/'Cargo.toml',ROOT/'Cargo.lock',Path(__file__),Path(__file__).with_name('PROTOCOL.md'),Path(__file__).with_name('audit.py')]
digests={}
for p in files:
    name=p.relative_to(ROOT).as_posix();target=OUT/'source'/name;target.parent.mkdir(parents=True,exist_ok=True)
    shutil.copy2(p,target);digests[name]=sha(target)
save(OUT/'source-frozen.json',{'parent':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'sources':digests,'binaries':binaries})
def run(name,args,label):
    if name=='activity-replay':
        with urllib.request.urlopen('http://127.0.0.1:11434/api/tags',timeout=5) as r: current=json.load(r)
        assert any(m['name']==MODEL and m['digest']==DIGEST for m in current['models'])
        save(OUT/f'{label}-model.json',current)
    start=time.monotonic()
    p=subprocess.run([binaries[name]['file'],*map(str,args)],cwd=ROOT,capture_output=True,text=True,timeout=180,creationflags=subprocess.CREATE_NO_WINDOW)
    receipt={'exit_code':p.returncode,'seconds':time.monotonic()-start,'stdout':p.stdout,'stderr':p.stderr}
    save(OUT/f'{label}-process.json',receipt);print(json.dumps(dict(label=label,**receipt)),flush=True)
    assert p.returncode==0
manifests=[]
for i in range(2):
    run('runtime-appearance-replay',[OLD/f'case-{i}.json','cached',OUT/f'case-{i}','--observations',OLD/f'case-{i}'/'result.json'],f'replay-{i}')
    replay=read(OUT/f'case-{i}'/'result.json');previous=read(OLD/f'case-{i}'/'result.json')
    assert replay['publication']['new_model_calls']==0
    assert replay['visual_tracks']==previous['visual_tracks']
    assert replay['evidence']['evidence']==previous['evidence']['evidence']
    seq=replay['evidence']['automatic_region_motion']['appearance_sequences']
    assert seq['report']==previous['evidence']['automatic_region_motion']['appearance_sequences']['report']
    assert seq['activity']['error'] is None
    source=read(sources[i]);source=source['revised_context']['source'] if i==0 else source['selected']
    frame_files={f['reference']['id']:f['file'] for f in read(OLD/f'case-{i}.json')['frames']}
    frames=[frame_files[p[side]['id']] for p in source['pairs'] for side in ['before','after']]
    for augmented in [False,True]:
        manifest={'source':source,'frames':frames}
        if augmented:manifest['appearances']=seq['activity']['summary']
        path=OUT/f'case-{i}-{"augmented" if augmented else "baseline"}.json';save(path,manifest);manifests.append(path)
save(OUT/'manifests-frozen.json',{str(p):sha(p) for p in manifests})
for manifest in manifests:
    run('activity-replay',[manifest,OUT/f'{manifest.stem}-receipt.json',MODEL],manifest.stem)
