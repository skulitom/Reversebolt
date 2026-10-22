"""Run each frozen observer/case once. Stops on a failed request; never retries it."""
import hashlib,json,subprocess,time,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/motion-interpretation-v1'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
inputs=read(OUT/'inputs.json')
for p,h in inputs['inputs'].items():assert sha(Path(p))==h,p
for version in ['baseline','revised']:
    folder=OUT/version;frozen=read(folder/'frozen.json');binary=Path(frozen['binary'])
    assert sha(binary)==frozen['binary_sha256']
    for name,h in frozen['sources'].items():assert sha(folder/'source'/name)==h,name
    for case in inputs['cases']:
        output=folder/(case['name']+'.json');assert not output.exists(),'Never restart a started case'
        with urllib.request.urlopen('http://127.0.0.1:11434/api/tags',timeout=3) as r:tags=json.load(r)
        assert any(m['name']=='qwen3.8:latest' and m['digest']==frozen['model']['digest'] for m in tags['models'])
        start=time.monotonic()
        process=subprocess.Popen([str(binary),case['manifest'],str(output),'qwen3.8:latest'],cwd=ROOT,
                                 stdout=subprocess.PIPE,stderr=subprocess.PIPE,creationflags=subprocess.CREATE_NO_WINDOW)
        print(json.dumps({'version':version,'case':case['name'],'pid':process.pid,'status':'running'}),flush=True)
        stdout,stderr=process.communicate(timeout=200)
        record={'pid':process.pid,'exit_code':process.returncode,'seconds':time.monotonic()-start,
                'stdout':stdout.decode('utf-8','replace'),'stderr':stderr.decode('utf-8','replace')}
        (folder/(case['name']+'-process.json')).write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8')
        print(json.dumps({'version':version,'case':case['name'],**record}),flush=True)
        assert process.returncode==0,'Stopped on runner failure'
        receipt=read(output)
        assert receipt['error'] is None,'Stopped on model failure; verify server state before another request'
