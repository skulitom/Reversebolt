"""Retain a status snapshot, printing only the cards under this fixture test."""
import json,sys,urllib.request
from pathlib import Path
root=Path(sys.argv[1]).resolve()
endpoint=json.loads((root/'data/endpoint.json').read_bytes())
req=urllib.request.Request(endpoint['url']+'/command',data=b'{"action":"status"}',headers={'Content-Type':'application/json','Authorization':'Bearer '+endpoint['token']})
result=json.loads(urllib.request.urlopen(req,timeout=20).read());state=result['state']
(root/(sys.argv[2]+'.json')).write_text(json.dumps(result,indent=2),encoding='utf-8')
live=state.get('live') or {};runtime=live.get('runtime') or {}
print(json.dumps({'error':result.get('error'),'cards':[{k:c[k] for k in ('value','status')}|{'name':c['candidate']['name'],'id':c['candidate']['id']} for c in runtime.get('cards',[]) if c['candidate']['name'] in ('Coefficient','Enabled')],
    'freezes':[{k:f[k] for k in ('id','phase','value','checks','writes','reason','identity')} for f in state.get('freezes',[]) if f['investigation']==live.get('investigation_id')],
    'trials':[{k:t[k] for k in ('id','phase','name')} for t in state.get('runtime_trials',[])]}))
