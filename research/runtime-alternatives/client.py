"""Phased experiment client of the shared MCP engine; no native input or writes outside it."""
import hashlib, json, math, queue, subprocess, sys, threading, time, uuid
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
OUT=Path(sys.argv[1]).resolve(); PHASE=sys.argv[2]; BIN=OUT/'bin'; DATA=OUT/'data'
lines=queue.Queue(); seq=0

def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def save(name,value):
    with (OUT/(name+'.json')).open('x',encoding='utf-8') as f: json.dump(value,f,indent=2)

mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,
    stderr=(OUT/('mcp-'+PHASE+'-'+str(time.time_ns())+'.stderr')).open('w'),text=True,encoding='utf-8',creationflags=0x08000000)
threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()

def rpc(method,params):
    global seq
    seq+=1
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','id':seq,'method':method,'params':params})+'\n');mcp.stdin.flush()
    end=time.monotonic()+180
    while time.monotonic()<end:
        result=json.loads(lines.get(timeout=max(.1,end-time.monotonic())))
        if result.get('id')==seq:
            assert 'error' not in result,result
            return result['result']
    raise AssertionError('MCP result missing; do not retry an uncertain action')

def call(name,args=None):
    stamp=str(time.time_ns())+'-'+name
    save(stamp+'-request',args or {})
    result=rpc('tools/call',{'name':'reversebolt_'+name,'arguments':args or {}})
    value=json.loads(next(c['text'] for c in result['content'] if c['type']=='text'))
    save(stamp+'-response',value)
    assert value.get('ok'),(name,value.get('error'))
    return value

def status():return call('status')
def envelope(s):
    h=s['jobs']['history']
    return dict(boot=h['boot'],binding=h['current'],request_id=h['clients'].get('mcp-agent',{}).get('high_water',0)+1)

def finish_job():
    end=time.monotonic()+180
    while time.monotonic()<end:
        s=status();jobs=s['jobs']['history']['jobs']
        if jobs and all(j['phase'].lower() in ('completed','failed','cancelled','interrupted') for j in jobs.values()):
            assert all(j['phase'].lower()=='completed' for j in jobs.values()),jobs
            return s
        time.sleep(.2)
    raise AssertionError('Discovery still pending; preserve the job handle')

def refresh(ids):
    s=status();rt=s['state']['live']['runtime']
    call('start_runtime_job',dict(**envelope(s),task=dict(operation='refresh',catalog_id=rt['catalog_id'],after_revision=rt['revision'],ids=ids),count=1,interval_ms=250))
    s=finish_job();rt=s['state']['live']['runtime']
    paths={}
    for c in rt['cards']:
        if c['candidate']['id'] in ids:
            r=read(Path(c['readings'][-1]['evidence_path']))
            for check in r['checks']:
                if check['path_id'] in ids: paths[check['path_id']]=check
    assert set(paths)==set(ids)
    assert all(c['status']=='read' for c in paths.values())
    return paths

try:
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'runtime-alternative-investigation','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    if PHASE=='attach':
        pid=int(sys.argv[3]);p=next(p for p in call('list_processes')['state']['processes'] if p['pid']==pid)
        save('target',p)
        call('attach_process',dict(pid=p['pid'],created=p['created'],window=p['window'],auto_observe=True,budget_mib=1))
        s=finish_job();save('discovery',s)
        print(json.dumps({'phase':PHASE,'catalog':s['state']['live']['runtime']['catalog_id']}))
    elif PHASE=='select':
        frozen=read(OUT/'frozen.json');proposal=read(Path(frozen['proposal_path']));old=read(Path(frozen['source_report']))
        assert hashlib.sha256(Path(frozen['proposal_path']).read_bytes()).hexdigest()==frozen['proposal_sha256']
        assert hashlib.sha256(Path(frozen['source_report']).read_bytes()).hexdigest()==frozen['source_sha256']
        classes={c['id']:c for c in old['classes']};groups=[]
        for group in proposal['accepted']:
            members=[]
            for ref in group['members']:
                cid,name=ref.split(':',1);owner=classes[int(cid)]
                fields=[f for f in owner['fields'] if f['name']==name]
                if len(fields)!=1:break
                members.append((owner,fields[0]))
            if 2<=len(members)<=3 and len(members)==len(group['members']) and len({o['id'] for o,f in members})==1 and len({f['type_code'] for o,f in members})==1 and members[0][1]['type_code'] in (12,13):
                groups.append((group,members))
        assert groups,'No eligible alternative group'
        group,members=groups[0];s=status();rt=s['state']['live']['runtime'];report=Path(rt['catalog_source']['path'])
        subprocess.run([str(ROOT/'target/debug/reversebolt-runtime-inventory.exe'),str(report),str(OUT/'inventory.json')],check=True,capture_output=True)
        inventory=read(OUT/'inventory.json');chosen=[];ids=[]
        for owner,field in members:
            fresh=next(t for t in inventory['types'] if t['key']==owner['key'])
            f=next(f for f in fresh['fields'] if f['field_id']==field['id'] and f['name']==field['name'])
            assert f['path_count']==len(f['samples']) and 1<=f['path_count']<=3,'Ambiguous/truncated path set'
            page=call('search_runtime_values',dict(catalog_id=rt['catalog_id'],query=field['name'],offset=0))['data']['search']
            assert page['matches']<=20,'Narrow search needed before testing'
            paths=[]
            for c in page['candidates']:
                leaf=c['locator']['steps'][-1]['member']
                if leaf.get('kind')=='field' and leaf['field']=={'declaring':owner['key'],'field_id':field['id']}:
                    paths.append(c['id'])
            assert set(paths)=={s['path_id'] for s in f['samples']}
            current=status()['state']['live']['runtime']
            call('show_runtime_values',dict(catalog_id=current['catalog_id'],after_revision=current['revision'],ids=paths))
            ids+=paths;chosen.append({'name':f['name'],'type_key':fresh['key'],'field_id':f['field_id'],'paths':paths,'primary':paths[0]})
        checks={}
        for item in chosen: checks.update(refresh(item['paths']))
        for item in chosen:
            samples=[checks[p] for p in item['paths']]
            assert len({c['field_address'] for c in samples})==1,'Multiple object instances remain ambiguous'
            assert all(c['bytes']==samples[0]['bytes'] for c in samples)
            number=float(samples[0]['value']);assert math.isfinite(number) and 0<number<=20
            item['initial']=samples[0]
        save('selection',{'proposal':group,'candidates':chosen,'fresh_report_sha256':inventory['report_sha256'],'checks':checks})
        print(json.dumps({'phase':PHASE,'candidates':[{'name':c['name'],'paths':len(c['paths']),'value':c['initial']['value']} for c in chosen]}))
    elif PHASE in ('refresh','apply','restore'):
        index=int(sys.argv[3]);selected=read(OUT/'selection.json')['candidates'][index];prefix='candidate-'+str(index)
        if PHASE=='refresh':
            checks=refresh(selected['paths']);save(prefix+'-'+sys.argv[4]+'-checks',checks)
            print(json.dumps({'phase':PHASE,'checks':[{k:c[k] for k in ('path_id','status','value','bytes')} for c in checks.values()]}))
        elif PHASE=='apply':
            s=status()['state']['live'];rt=s['runtime'];args=dict(change_id=str(uuid.uuid4()),investigation_id=s['investigation_id'],catalog_id=rt['catalog_id'],after_revision=rt['revision'],candidate_id=selected['primary'],value='0')
            save(prefix+'-intent',args)
            result=call('set_runtime_value',args);save(prefix+'-write',result)
            assert result['data']['phase']=='written' and result['data']['write']['outcome']=='written'
            print(json.dumps({'phase':PHASE,'candidate':index,'trial':result['data']['id'],'before':result['data']['patch']['expected']}))
        else:
            trial=read(OUT/(prefix+'-write.json'))['data'];result=call('restore_runtime_change',{'change_id':trial['id']});save(prefix+'-restore',result)
            t=result['data'];check=t['restoration'].get('evidence',{}).get('report',{}).get('checks',[{}])[0]
            p=trial['patch']
            returned=t['restoration']['outcome']=='refused' and check.get('status')=='read' and check.get('bytes')==p['expected'] and check.get('path_id')==selected['primary'] and check.get('type_code')==p['type_code'] and int(check.get('field_address','0'),16)==p['field_address'] and [int(a,16) for a in check.get('snapshot_object_addresses',[])]==p['object_addresses']
            assert t['phase']=='restored' or returned,'Restoration unresolved; stop further experiments'
            save(prefix+'-recovery',{'phase':t['phase'],'outcome':t['restoration']['outcome'],'already_returned':returned,'limitation':'Original bytes at the guarded path only; not full gameplay rollback.'})
            print(json.dumps({'phase':PHASE,'candidate':index,'trial_phase':t['phase'],'already_returned':returned}))
    else:raise AssertionError('Unknown phase')
finally:
    mcp.stdin.close()
    try:mcp.wait(timeout=5)
    except subprocess.TimeoutExpired:mcp.terminate();mcp.wait(timeout=5)
