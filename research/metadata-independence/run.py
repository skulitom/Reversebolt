"""Frozen offline replay. No target process or game control."""
import hashlib, json, subprocess, sys, time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=Path(sys.argv[1]).resolve(); VERSION=sys.argv[2]
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
save=lambda p,v:p.write_text(json.dumps(v,indent=2)+'\n',encoding='utf-8')
frozen=read(OUT/f'{VERSION}-frozen.json');exe=OUT/frozen['binary']
assert hashlib.sha256(exe.read_bytes()).hexdigest()==frozen['binary_sha256']
for case in frozen['cases']:
    source=Path(case['source']);assert hashlib.sha256(source.read_bytes()).hexdigest()==case['sha256']
    destination=OUT/VERSION/case['name'];destination.parent.mkdir(exist_ok=True)
    assert not destination.exists(), 'Never replay a started case'
    command=[str(exe),str(source),str(destination),'qwen3.8:latest']+case.get('flags',[])
    begin=time.time();p=subprocess.Popen(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE,creationflags=0x08000000)
    stdout,stderr=p.communicate(timeout=1200)
    save(OUT/VERSION/(case['name']+'-process.json'),{'pid':p.pid,'exit_code':p.returncode,'seconds':time.time()-begin,'stdout':stdout.decode('utf-8','replace'),'stderr':stderr.decode('utf-8','replace')})
    print(json.dumps({'case':case['name'],'exit_code':p.returncode,'stdout':stdout.decode('utf-8','replace')}),flush=True)
    if p.returncode:raise SystemExit(p.returncode)
