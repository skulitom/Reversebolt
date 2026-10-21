"""Malformed offline evidence must fail before inference or output creation."""
import copy, hashlib, json, subprocess, sys
from pathlib import Path
OUT=Path(sys.argv[1]).resolve();read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
frozen=read(OUT/'revised-frozen.json');exe=OUT/frozen['binary']
assert hashlib.sha256(exe.read_bytes()).hexdigest()==frozen['binary_sha256']
directory=OUT/'invalid';directory.mkdir(exist_ok=False)
original=read(OUT/'activity.json');variants=[]
v=copy.deepcopy(original);v['frames'].append('must-not-be-opened.png');variants.append(('count',json.dumps(v),'exactly four'))
v=copy.deepcopy(original);v['frames'][0],v['frames'][1]=v['frames'][1],v['frames'][0];variants.append(('order',json.dumps(v),'differ from the retained source'))
v=copy.deepcopy(original);v['source']['pairs'][0]['before']['sha256']='f'*64;variants.append(('hash',json.dumps(v),'differ from the retained source'))
variants.append(('size',' '*65537,'exceeds 64 KiB'))
results=[]
for name,content,error in variants:
    manifest=directory/(name+'.json');manifest.write_text(content,encoding='utf-8');output=directory/(name+'-output')
    p=subprocess.run([str(exe),frozen['cases'][0]['source'],str(output),'qwen3.8:latest','--activity',str(manifest)],capture_output=True,text=True,encoding='utf-8',creationflags=0x08000000,timeout=15)
    assert p.returncode!=0 and error in p.stderr and not output.exists(),(name,p.stdout,p.stderr)
    results.append({'case':name,'exit_code':p.returncode,'stderr':p.stderr,'output_created':False})
(OUT/'invalid-results.json').write_text(json.dumps(results,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'rejected_before_inference':len(results)}))
