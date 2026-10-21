"""Native MCP verification of already-original recovery on an owned ABI fixture."""
import hashlib,json,os,queue,subprocess,sys,threading,time,uuid
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve();OUT.mkdir(parents=True,exist_ok=False)
BIN=OUT.parent/'bin';DATA=OUT/'data';NO_WINDOW=0x08000000
engine=mcp=host=None;children=[];lines=queue.Queue();seq=host_seq=0;calls=[]
def save(name,v):(OUT/(name+'.json')).write_text(json.dumps(v,indent=2),encoding='utf-8')
def read(p):return json.loads(p.read_bytes())
def rpc(method,params):
    global seq
    seq+=1;mcp.stdin.write(json.dumps({'jsonrpc':'2.0','id':seq,'method':method,'params':params})+'\n');mcp.stdin.flush()
    end=time.monotonic()+180
    while time.monotonic()<end:
        r=json.loads(lines.get(timeout=max(.1,end-time.monotonic())))
        if r.get('id')==seq:
            assert 'error' not in r,r
            return r['result']
    raise AssertionError('No RPC result')
def call(name,args=None,label=None):
    r=rpc('tools/call',{'name':'reversebolt_'+name,'arguments':args or {}})
    v=json.loads(next(c['text'] for c in r['content'] if c['type']=='text'));calls.append({'action':name,'ok':v.get('ok'),'error':v.get('error')})
    if label:save(label,v)
    assert v['ok'],v.get('error');return v
def status():return call('status')
def ops():return len(list((DATA/'runtime').glob('*-*/**/stdout.jsonl')))
def launch():
    p=subprocess.Popen([str(BIN/'Reversebolt.exe'),'--headless','--data-dir',str(DATA)],creationflags=NO_WINDOW,stdout=(OUT/('engine-'+str(len(children))+'.stdout')).open('w'),stderr=(OUT/('engine-'+str(len(children))+'.stderr')).open('w'))
    children.append(p);end=time.monotonic()+20
    while time.monotonic()<end:
        assert p.poll() is None
        try:
            if read(DATA/'endpoint.json')['pid']==p.pid:return p
        except (FileNotFoundError,json.JSONDecodeError):pass
        time.sleep(.05)
    raise AssertionError('Engine startup timed out')
def host_call(operation,**args):
    global host_seq
    host_seq+=1;(OUT/'request.json').write_text(json.dumps(dict(id=host_seq,operation=operation,**args)),encoding='utf-8');end=time.monotonic()+5
    while time.monotonic()<end:
        assert host.poll() is None
        try:
            r=read(OUT/'response.json')
            if r['id']==host_seq:return r
        except (FileNotFoundError,json.JSONDecodeError):pass
        time.sleep(.01)
    raise AssertionError('Fixture response timed out')
def direct(name,value,label):
    live=status()['state']['live'];rt=live['runtime'];c=next(c for c in rt['cards'] if c['candidate']['name']==name)
    t=call('set_runtime_value',dict(change_id=str(uuid.uuid4()),investigation_id=live['investigation_id'],catalog_id=rt['catalog_id'],after_revision=rt['revision'],candidate_id=c['candidate']['id'],value=value),label)['data']
    assert t['phase']=='written';return t
def oracle(t,expected,label):
    r=host_call('read',address=t['patch']['field_address'],width=len(expected));assert r['bytes']==expected,(label,r)
    save(label,dict(r,expected=expected))
def restore(t,label,outcome):
    r=call('restore_runtime_change',{'change_id':t['id']},label)['data'];a=r['restoration']
    assert a['outcome']==outcome
    assert r['phase']==('restored' if outcome=='already_original' else 'written')
    assert a['evidence']['report']['patch']['attempted'] is False
    assert a['evidence']['report']['patch']['transferred']==0
    count=ops();again=call('restore_runtime_change',{'change_id':t['id']},label+'-replay')['data'];assert again==r and ops()==count
    return r

try:
    host=subprocess.Popen([sys.executable,str(ROOT/'research/friendly-controls/host.py'),str(OUT),str(BIN/'reversebolt_path_fixture.dll')],creationflags=NO_WINDOW,stdout=(OUT/'host.stdout').open('w'),stderr=(OUT/'host.stderr').open('w'));children.append(host)
    end=time.monotonic()+15
    while not (OUT/'ready.json').exists():
        assert host.poll() is None and time.monotonic()<end;time.sleep(.05)
    engine=launch()
    mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(OUT/'mcp.stderr').open('w'),text=True,encoding='utf-8',creationflags=NO_WINDOW);children.append(mcp)
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'already-original-native-test','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    tools=rpc('tools/list',{})['tools'];assert len(tools)==45
    assert 'already_original' in next(t for t in tools if t['name']=='reversebolt_restore_runtime_change')['description']
    target=next(p for p in call('list_processes')['state']['processes'] if p['pid']==host.pid)
    call('attach_process',dict(pid=target['pid'],created=target['created'],window=target['window'],auto_observe=True,budget_mib=1),'attach')
    end=time.monotonic()+160
    while time.monotonic()<end:
        s=status();jobs=s['jobs']['history']['jobs']
        if jobs and all(j['phase'].lower() in ('completed','failed','cancelled','interrupted') for j in jobs.values()):
            assert all(j['phase'].lower()=='completed' for j in jobs.values());break
        time.sleep(.1)
    else:raise AssertionError('Discovery timed out')
    recovered=[]
    for name,value in [('Coefficient','4.5'),('Enabled','true')]:
        t=direct(name,value,name+'-write');oracle(t,t['patch']['replacement'],name+'-written-oracle')
        host_call('write',address=t['patch']['field_address'],bytes=t['patch']['expected'])
        r=restore(t,name+'-restore','already_original');oracle(t,t['patch']['expected'],name+'-original-oracle')
        recovered.append(r)
    # A different current number cannot be labelled recovered.
    import struct
    t=direct('Coefficient','4.5','drift-write');drift=list(struct.pack('<f',5.5))
    host_call('write',address=t['patch']['field_address'],bytes=drift)
    restore(t,'drift-refusal','refused');oracle(t,drift,'drift-unchanged-oracle')
    host_call('write',address=t['patch']['field_address'],bytes=t['patch']['expected'])
    # Equal original bytes on a changed reference chain cannot prove recovery.
    t=direct('Coefficient','4.5','replacement-write')
    host_call('write',address=t['patch']['field_address'],bytes=t['patch']['expected']);host_call('tick',mode=1)
    restore(t,'replacement-refusal','refused');oracle(t,t['patch']['expected'],'replacement-original-oracle')
    before=status()['state']['runtime_trials'];count=ops();engine.terminate();engine.wait(timeout=10);engine=launch()
    after=call('status',label='restart')['state']['runtime_trials'];assert after==before and ops()==count
    for t in recovered:
        assert call('restore_runtime_change',{'change_id':t['id']})['data']==t
    assert ops()==count
    evidence=[];pauses=0;maximum=0
    for path in (DATA/'runtime').glob('*-*/**/stdout.jsonl'):
        receipts=[json.loads(line) for line in path.read_text().splitlines()];assert receipts[-1]['stage']=='complete' and receipts[-1]['helper_absent']
        report=read(path.with_name('report.json'));shots=report.get('pauses',report.get('snapshots',[])+[w['discarded_snapshot'] for w in report.get('warmups',[])])
        assert all(s['helper_allocations_while_paused']==s['helper_deallocations_while_paused']==0 for s in shots)
        pauses+=len(shots);maximum=max([maximum]+[s['stop_to_resume_us'] for s in shots])
        evidence.append({'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'report_sha256':hashlib.sha256(path.with_name('report.json').read_bytes()).hexdigest()})
    save('summary',dict(passed=True,tools=len(tools),trials=len(after),operations=ops(),pauses=pauses,maximum_stop_to_resume_us=maximum,evidence=evidence))
finally:
    save('calls',calls)
    if mcp is not None and mcp.poll() is None:mcp.terminate();mcp.wait(timeout=5)
    if engine is not None and engine.poll() is None:engine.terminate();engine.wait(timeout=10)
    if host is not None:
        (OUT/'stop').touch()
        try:host.wait(timeout=5)
        except subprocess.TimeoutExpired:host.terminate();host.wait(timeout=5)
    save('execution',[{'pid':p.pid,'exit':p.poll()} for p in children])
print(json.dumps({'passed':True,'cleanup':True}))
