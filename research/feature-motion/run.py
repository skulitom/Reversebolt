"""Frozen file-only correspondence evaluation; refuses to replace a prior run."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/feature-motion-v1'
OUT.mkdir()

def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f: return hashlib.file_digest(f,'sha256').hexdigest()

old=ROOT/'test-results/patch-motion-v3'
cases=read(old/'cases.json')
assert len(cases)==5
assert cases[0]['name'].startswith('Hyperbolica')
assert cases[4]['name'].startswith('Manifold Garden')
assert all(c['name'].startswith('Owned striped fixture') for c in cases[1:4])
inputs=[old/'cases.json',old/'result.json',old/'frozen.json']
for case in cases:
    for frame in case['frames']:
        p=Path(frame['file'])
        assert sha(p)==frame['reference']['sha256']
        inputs.append(p)
request=OUT/'cases.json'
shutil.copy2(old/'cases.json',request)
inputs.append(request)
transforms=[{'name':c['name'],'file':c['frames'][0]['file'],'sha256':c['frames'][0]['reference']['sha256']} for c in [cases[0],cases[4]]]
transform_request=OUT/'transforms.json'
transform_request.write_text(json.dumps(transforms,indent=2)+'\n',encoding='utf-8')
inputs.append(transform_request)
binaries={}
for name in ['motion-replay','motion-transform']:
    source=ROOT/f'target/debug/examples/{name}.exe'
    archived=OUT/source.name
    shutil.copy2(source,archived)
    assert sha(source)==sha(archived)
    binaries[name]={'file':str(archived),'sha256':sha(archived)}
sources=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
sources += [ROOT/'Cargo.toml',ROOT/'Cargo.lock',Path(__file__),Path(__file__).with_name('PROTOCOL.md')]
digests={}
for p in sources:
    name=p.relative_to(ROOT).as_posix()
    copy=OUT/'source'/name
    copy.parent.mkdir(parents=True,exist_ok=True)
    shutil.copy2(p,copy)
    digests[name]=sha(copy)
frozen={'parent':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        'sources':digests,'inputs':{str(p):sha(p) for p in inputs},'binaries':binaries}
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2)+'\n',encoding='utf-8')
for name,manifest,output in [('motion-replay',request,'result'),('motion-transform',transform_request,'transform-result')]:
    start=time.monotonic()
    process=subprocess.run([binaries[name]['file'],str(manifest),str(OUT/(output+'.json'))],cwd=ROOT,
                           capture_output=True,text=True,timeout=60,creationflags=subprocess.CREATE_NO_WINDOW)
    record={'exit_code':process.returncode,'seconds':time.monotonic()-start,'stdout':process.stdout,'stderr':process.stderr}
    (OUT/(output+'-process.json')).write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(dict(name=name,**record)))
    assert process.returncode==0
