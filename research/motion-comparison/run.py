"""Freeze and replay retained receipts without a model, images or native target."""
import hashlib,json,shutil,subprocess,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/motion-comparison-v1';OUT.mkdir()
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(p,v):p.write_text(json.dumps(v,indent=2)+'\n',encoding='utf-8')
previous=ROOT/'research/motion-interpretation/results.json'
audit=read(previous)
inputs=[previous]
cases=[]
for version in ['baseline','revised']:
    for name in ['hyperbolica','manifold','consistent','opposed']:
        run='v1' if name in ['hyperbolica','manifold'] else 'v2'
        folder=ROOT/('test-results/motion-interpretation-'+run)
        receipt=folder/version/(name+'.json')
        recorded=next(r for r in audit['results'] if (r['run'],r['version'],r['case'])==(run,version,name))
        assert sha(receipt)==recorded['receipt_sha256']
        cases.append({'name':version+'-'+name,'file':str(receipt),'sha256':sha(receipt)})
        inputs.append(receipt)
        manifest=folder/'cases'/(name+'.json') if run=='v1' else folder/'controls'/(name+'.json')
        inputs.append(manifest)
        source=read(manifest)
        refs=[f for pair in source['source']['pairs'] for f in [pair['before'],pair['after']]]
        for path,reference in zip(source['frames'],refs,strict=True):
            path=Path(path);assert sha(path)==reference['sha256'];inputs.append(path)
request=OUT/'cases.json';save(request,cases);inputs.append(request)
binary=OUT/'motion-comparison-replay.exe';shutil.copy2(ROOT/'target/debug/examples/motion-comparison-replay.exe',binary)
sources=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
sources += [ROOT/'Cargo.toml',ROOT/'Cargo.lock',Path(__file__),Path(__file__).with_name('PROTOCOL.md')]
digests={}
for p in sources:
    name=p.relative_to(ROOT).as_posix();copy=OUT/'source'/name;copy.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,copy);digests[name]=sha(copy)
save(OUT/'frozen.json',{'parent':subprocess.check_output(['git','rev-parse','HEAD'],text=True,cwd=ROOT).strip(),
    'sources':digests,'inputs':{str(p):sha(p) for p in inputs},'binary':str(binary),'binary_sha256':sha(binary)})
start=time.monotonic()
process=subprocess.run([str(binary),str(request),str(OUT/'result.json')],cwd=ROOT,capture_output=True,text=True,timeout=30,creationflags=subprocess.CREATE_NO_WINDOW)
record={'exit_code':process.returncode,'seconds':time.monotonic()-start,'stdout':process.stdout,'stderr':process.stderr}
save(OUT/'process.json',record);print(json.dumps(record));assert process.returncode==0
