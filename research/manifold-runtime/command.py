"""Call the shared product engine and retain a response, without printing credentials."""
import json, sys, urllib.request
from pathlib import Path

root=Path(sys.argv[1]).resolve()
action=sys.argv[2]
args=json.loads(Path(sys.argv[3]).read_text(encoding='utf-8-sig')) if len(sys.argv)>3 else {}
endpoint=json.loads((root/'data/endpoint.json').read_bytes())
request=urllib.request.Request(endpoint['url']+'/command',data=json.dumps({'action':action,'args':args}).encode(),
    headers={'Content-Type':'application/json','Authorization':'Bearer '+endpoint['token']})
result=json.loads(urllib.request.urlopen(request,timeout=180).read())
(root/(action+'-latest.json')).write_text(json.dumps(result,indent=2),encoding='utf-8')
live=result.get('state',{}).get('live')
data=result.get('data')
data_summary=data if len(json.dumps(data)) < 2000 else {'saved':action+'-latest.json','keys':list(data) if isinstance(data,dict) else None}
print(json.dumps({'ok':result.get('ok'),'error':result.get('error'),'data':data_summary,
    'runtime':{k:v for k,v in (live or {}).get('runtime',{}).items() if k not in ('cards','search')},
    'jobs':[(k,j['phase']) for k,j in result.get('jobs',{}).get('history',{}).get('jobs',{}).items()]}))
