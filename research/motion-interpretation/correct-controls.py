"""Preserve V1 and rerun only blinded controls with the exact frozen observers."""
import hashlib,json,shutil,subprocess,time,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OLD=ROOT/'test-results/motion-interpretation-v1'
OUT=ROOT/'test-results/motion-interpretation-v2'
OUT.mkdir()
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def save(p,v):p.write_text(json.dumps(v,indent=2)+'\n',encoding='utf-8')
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
original=read(OLD/'revised/frozen.json')
changed=[name for name,h in original['sources'].items() if sha(ROOT/name)!=h]
assert changed==['crates/reversebolt-vision/examples/activity-controls.rs'],changed
generator=OUT/'activity-controls.exe';shutil.copy2(ROOT/'target/debug/examples/activity-controls.exe',generator)
first=read(ROOT/'test-results/patch-motion-v3/cases.json')[0]['frames'][0]
controls=OUT/'controls'
process=subprocess.run([str(generator),first['file'],first['reference']['sha256'],str(controls)],cwd=ROOT,
                       capture_output=True,text=True,timeout=30,creationflags=subprocess.CREATE_NO_WINDOW)
save(OUT/'controls-process.json',{'exit_code':process.returncode,'stdout':process.stdout,'stderr':process.stderr})
assert process.returncode==0,process.stderr
for i in range(3):assert sha(controls/f'{i}.png')==sha(OLD/'controls'/f'{i}.png')
for name in ['consistent','opposed']:
    source=read(controls/(name+'.json'))['source']
    assert source['investigation_id']=='synthetic-image-sequence'
    assert all(f['id']==f"synthetic-image-sequence-{f['number']}" for p in source['pairs'] for f in [p['before'],p['after']])
binaries={}
for version in ['baseline','revised']:
    folder=OUT/version;folder.mkdir();old=read(OLD/version/'frozen.json')
    binary=folder/'activity-replay.exe';shutil.copy2(Path(old['binary']),binary)
    assert sha(binary)==old['binary_sha256']
    binaries[version]={'file':str(binary),'sha256':sha(binary),'original_source_manifest':str(OLD/version/'frozen.json'),
                       'original_source_manifest_sha256':sha(OLD/version/'frozen.json')}
sources=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
sources += [ROOT/'Cargo.toml',ROOT/'Cargo.lock',Path(__file__),Path(__file__).with_name('PROTOCOL.md'),Path(__file__).with_name('PROTOCOL-v2.md')]
digests={}
for p in sources:
    name=p.relative_to(ROOT).as_posix();copy=OUT/'source'/name;copy.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,copy);digests[name]=sha(copy)
inputs=[Path(first['file']),generator]+list(controls.iterdir())
save(OUT/'frozen.json',{'sources':digests,'inputs':{str(p):sha(p) for p in inputs},'binaries':binaries,
                      'model':original['model'],'server':original['server'],'reason':'Remove expected-answer labels from synthetic image IDs; observer code and all image bytes unchanged.'})
for version in ['baseline','revised']:
    for name in ['consistent','opposed']:
        with urllib.request.urlopen('http://127.0.0.1:11434/api/tags',timeout=3) as r:tags=json.load(r)
        assert any(m['name']=='qwen3.8:latest' and m['digest']==original['model']['digest'] for m in tags['models'])
        output=OUT/version/(name+'.json');assert not output.exists()
        start=time.monotonic()
        process=subprocess.Popen([binaries[version]['file'],str(controls/(name+'.json')),str(output),'qwen3.8:latest'],cwd=ROOT,
                                 stdout=subprocess.PIPE,stderr=subprocess.PIPE,creationflags=subprocess.CREATE_NO_WINDOW)
        print(json.dumps({'version':version,'case':name,'pid':process.pid,'status':'running'}),flush=True)
        stdout,stderr=process.communicate(timeout=200)
        record={'pid':process.pid,'exit_code':process.returncode,'seconds':time.monotonic()-start,
                'stdout':stdout.decode('utf-8','replace'),'stderr':stderr.decode('utf-8','replace')}
        save(OUT/version/(name+'-process.json'),record)
        print(json.dumps({'version':version,'case':name,**record}),flush=True)
        assert process.returncode==0
        assert read(output)['error'] is None,'Stopped on model failure; verify server state before another request'
