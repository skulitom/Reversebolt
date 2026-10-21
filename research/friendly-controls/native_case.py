"""Freeze and direct controls evaluated through the shared MCP engine and native UI."""
import hashlib, json, os, queue, struct, subprocess, sys, threading, time, uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(sys.argv[1]).resolve(); OUT.mkdir(parents=True, exist_ok=False)
BIN = OUT.parent / 'bin'; DATA = OUT / 'data'; NO_WINDOW = 0x08000000
engine = mcp = host = None
children = []; calls = []; cases = []; lines = queue.Queue(); message_id = host_id = 0

def save(name, value):
    (OUT / (name + '.json')).write_text(json.dumps(value, indent=2), encoding='utf-8')

def rpc(method, params):
    global message_id
    message_id += 1
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','id':message_id,'method':method,'params':params})+'\n'); mcp.stdin.flush()
    deadline = time.monotonic()+160
    while time.monotonic() < deadline:
        result = json.loads(lines.get(timeout=max(.1,deadline-time.monotonic())))
        if result.get('id') == message_id:
            assert 'error' not in result, result
            return result['result']
    raise AssertionError('MCP response missing')

def call(name, args=None, label=None, ok=True):
    result = rpc('tools/call', {'name':'reversebolt_'+name, 'arguments':args or {}})
    value = json.loads(next(c['text'] for c in result['content'] if c['type']=='text'))
    calls.append({'tool':name, 'ok':value.get('ok'), 'error':value.get('error')})
    if label: save(label, value)
    assert value['ok'] == ok, (name, value.get('error'))
    return value

def status(): return call('status')
def operations(): return len(list((DATA/'runtime').glob('*-*/**/stdout.jsonl')))
def card(state): return next(c for c in state['live']['runtime']['cards'] if c['candidate']['id']==candidate_id)
def envelope(state):
    h=state['jobs']['history']
    return dict(boot=h['boot'],binding=h['current'],request_id=h['clients'].get('mcp-agent',{}).get('high_water',0)+1)

def done(label):
    deadline=time.monotonic()+160
    while time.monotonic()<deadline:
        current=status(); phases=[j['phase'].lower() for j in current['jobs']['history']['jobs'].values()]
        if phases and all(p in ('completed','cancelled','failed','interrupted') for p in phases):
            assert phases[-1]=='completed', current.get('error')
            save(label,current); return current['state']
        time.sleep(.1)
    raise AssertionError('Runtime job did not finish')

def launch():
    app=subprocess.Popen([str(BIN/'Reversebolt.exe'),'--data-dir',str(DATA)],creationflags=NO_WINDOW,
        stdout=(OUT/f'app-{len(children)}.stdout').open('w'),stderr=(OUT/f'app-{len(children)}.stderr').open('w'))
    children.append(app); deadline=time.monotonic()+20
    while time.monotonic()<deadline:
        assert app.poll() is None
        try:
            if json.loads((DATA/'endpoint.json').read_bytes())['pid']==app.pid: return app
        except (FileNotFoundError,json.JSONDecodeError): pass
        time.sleep(.05)
    raise AssertionError('App did not start')

def host_command(operation, **args):
    global host_id
    host_id+=1
    (OUT/'request.json').write_text(json.dumps(dict(id=host_id,operation=operation,**args)),encoding='utf-8')
    deadline=time.monotonic()+5
    while time.monotonic()<deadline:
        assert host.poll() is None
        try:
            result=json.loads((OUT/'response.json').read_bytes())
            if result['id']==host_id: return result
        except (FileNotFoundError,json.JSONDecodeError): pass
        time.sleep(.01)
    raise AssertionError('Host response missing')

def oracle(label, trial, expected):
    result=host_command('read',address=trial['patch']['field_address'],width=4)
    assert result['bytes']==list(struct.pack('<f',expected)), (label,result)
    save(label+'-oracle',dict(result,expected=expected,address=trial['patch']['field_address']))

def prepare(label, value='4.5'):
    current=status()['state']['live']['runtime']
    args=dict(catalog_id=current['catalog_id'],after_revision=current['revision'],candidate_id=candidate_id,value=value)
    result=call('prepare_runtime_change',args,label)
    trial=result['data']; assert trial['phase']=='prepared' and trial['write'] is None
    assert trial['context']['revision']==current['revision']+1
    cases.append(label); return trial

def apply(label, trial, restore=False, outcome='written'):
    result=call('restore_runtime_change' if restore else 'apply_runtime_change',dict(change_id=trial['id']),label)
    actual=result['data']; side='restoration' if restore else 'write'
    assert actual[side]['outcome']==outcome, actual
    assert actual['phase']==('restored' if restore and outcome=='written' else 'written' if restore or outcome=='written' else 'refused')
    cases.append(label); return actual

def direct(name, value, label):
    current=status()['state']; runtime=current['live']['runtime']
    chosen=next(c for c in runtime['cards'] if c['candidate']['name']==name)
    args=dict(change_id=str(uuid.uuid4()),investigation_id=current['live']['investigation_id'],catalog_id=runtime['catalog_id'],
        after_revision=runtime['revision'],candidate_id=chosen['candidate']['id'],value=value)
    result=call('set_runtime_value',args,label)['data']
    assert result['phase']=='written' and result['direct'] and result['write']['outcome']=='written',result
    cases.append(label)
    return args,result

def exact_oracle(label, trial, expected):
    result=host_command('read',address=trial['patch']['field_address'],width=len(expected))
    assert result['bytes']==list(expected),result
    save(label+'-oracle',dict(result,expected=list(expected),address=trial['patch']['field_address']))

def wait_freeze(identity, predicate, label):
    deadline=time.monotonic()+30
    while time.monotonic()<deadline:
        state=status()['state'];f=next(f for f in state['freezes'] if f['id']==identity)
        if predicate(f):save(label,f);return f
        time.sleep(.05)
    raise AssertionError(('Freeze did not settle',f))

def enable(name,value,label):
    state=status()['state'];live=state['live'];rt=live['runtime']
    chosen=next(c for c in rt['cards'] if c['candidate']['name']==name)
    args=dict(freeze_id=str(uuid.uuid4()),enabled=True,investigation_id=live['investigation_id'],catalog_id=rt['catalog_id'],after_revision=rt['revision'],candidate_id=chosen['candidate']['id'],value=value)
    result=call('set_runtime_freeze',args,label)['data'];assert result['phase']=='active'
    return args,wait_freeze(result['id'],lambda f:f['checks']>=1 and f['phase']=='active',label+'-checked')

def stop(f,label):
    result=call('set_runtime_freeze',dict(freeze_id=f['id'],enabled=False),label)['data']
    assert result['phase']=='stopped';return result

def read_f(f,expected,label):
    address=int(f['identity']['field'],16);result=host_command('read',address=address,width=len(expected))
    assert result['bytes']==list(expected),(label,result);save(label,dict(result,address=address))

try:
    host=subprocess.Popen([sys.executable,str(ROOT/'research/friendly-controls/host.py'),str(OUT),str(BIN/'reversebolt_path_fixture.dll')],creationflags=NO_WINDOW,stdout=(OUT/'host.stdout').open('w'),stderr=(OUT/'host.stderr').open('w'))
    children.append(host);deadline=time.monotonic()+10
    while not (OUT/'ready.json').exists():
        assert host.poll() is None and time.monotonic()<deadline;time.sleep(.03)
    identity=json.loads((OUT/'ready.json').read_bytes());engine=launch()
    mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(OUT/'mcp.stderr').open('w'),text=True,encoding='utf-8',creationflags=NO_WINDOW)
    children.append(mcp);threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'friendly-controls-evaluation','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    listed=rpc('tools/list',{})['tools'];assert len(listed)==45
    assert next(t for t in listed if t['name']=='reversebolt_set_runtime_freeze')['annotations']['destructiveHint']
    save('tools',listed)
    inventory=call('list_processes');target=next(p for p in inventory['state']['processes'] if p['pid']==host.pid)
    call('attach_process',dict(pid=target['pid'],created=target['created'],window=target['window'],auto_observe=True,budget_mib=1),'attached');done('discovery')
    for name in ('Enabled','Coefficient'):
        rt=status()['state']['live']['runtime'];r=call('search_runtime_values',dict(catalog_id=rt['catalog_id'],query=name),'search-'+name)
        chosen=next(c for c in r['state']['live']['runtime']['search']['candidates'] if c['name']==name)
        call('show_runtime_values',dict(catalog_id=rt['catalog_id'],after_revision=rt['revision'],ids=[chosen['id']]),'show-'+name)
    args,f=enable('Coefficient','4.5','number-enable');assert f['writes']==1;read_f(f,struct.pack('<f',4.5),'number-initial-oracle')
    host_command('write',address=int(f['identity']['field'],16),bytes=list(struct.pack('<f',2.5)))
    f=wait_freeze(f['id'],lambda x:x['writes']==2,'number-reheld');read_f(f,struct.pack('<f',4.5),'number-reheld-oracle')
    checked=f['checks'];f=wait_freeze(f['id'],lambda x:x['checks']>=checked+2,'number-noop');assert f['writes']==2
    f=stop(f,'number-off');host_command('write',address=int(f['identity']['field'],16),bytes=list(struct.pack('<f',3.5)))
    time.sleep(1.2);read_f(f,struct.pack('<f',3.5),'number-off-oracle')
    assert call('set_runtime_freeze',args,'enable-replay')['data']['phase']=='stopped'
    call('set_runtime_freeze',dict(args,value='5.5'),'enable-conflict',False)
    args,b=enable('Enabled','true','boolean-enable');assert b['writes']==1;read_f(b,bytes([1]),'boolean-oracle')
    host_command('write',address=int(b['identity']['field'],16),bytes=[0]);b=wait_freeze(b['id'],lambda x:x['writes']==2,'boolean-reheld')
    b=stop(b,'boolean-off');host_command('write',address=int(b['identity']['field'],16),bytes=[0]);read_f(b,bytes([0]),'boolean-off-oracle')
    args,f=enable('Coefficient','4.5','replacement-enable');host_command('tick',mode=0)
    f=wait_freeze(f['id'],lambda x:x['phase']=='stopped','replacement-stop');assert 'object changed' in f['reason']
    args,f=enable('Coefficient','3.5','restart-enable');assert f['writes']==0
    engine.terminate();engine.wait(timeout=10);count=operations();engine=launch()
    restart=call('status',label='restart');assert restart['state']['live'] is None and operations()==count
    assert all(f['phase']=='stopped' for f in restart['state']['freezes'])
    assert call('set_runtime_freeze',args,'restart-enable-replay')['data']['phase']=='stopped'
    call('attach_process',dict(pid=target['pid'],created=target['created'],window=target['window'],auto_observe=True,budget_mib=1),'ui-attach');done('ui-discovery')
    for name in ('Enabled','Coefficient'):
        rt=status()['state']['live']['runtime'];r=call('search_runtime_values',dict(catalog_id=rt['catalog_id'],query=name),'ui-search-'+name)
        chosen=next(c for c in r['state']['live']['runtime']['search']['candidates'] if c['name']==name)
        call('show_runtime_values',dict(catalog_id=rt['catalog_id'],after_revision=rt['revision'],ids=[chosen['id']]),'ui-show-'+name)
    save('processes',dict(app_pid=engine.pid,target_pid=host.pid,session=identity['session']))
    (OUT/'ui-ready').touch();print(json.dumps({'ui_ready':True,'app_pid':engine.pid}),flush=True)
    deadline=time.monotonic()+900
    while not (OUT/'finish').exists() and time.monotonic()<deadline:
        assert engine.poll() is None;time.sleep(.2)
    assert (OUT/'finish').exists(),'UI verification did not finish'
    final=call('status',label='ui-complete');assert all(not f['phase'] in ('active','writing','uncertain') for f in final['state']['freezes'])
    evidence=[];pauses=0;maximum=0
    for receipt_file in (DATA/'runtime').glob('*-*/**/stdout.jsonl'):
        receipts=[json.loads(line) for line in receipt_file.read_text(encoding='utf-8').splitlines()]
        assert receipts[-1]['stage']=='complete' and receipts[-1]['helper_absent']
        report=json.loads(receipt_file.with_name('report.json').read_bytes())
        shots=report.get('pauses',report.get('snapshots',[])+[w['discarded_snapshot'] for w in report.get('warmups',[])])
        assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in shots)
        pauses+=len(shots);maximum=max([maximum]+[s['stop_to_resume_us'] for s in shots])
        evidence.append(dict(path=str(receipt_file.parent),receipt_sha256=hashlib.sha256(receipt_file.read_bytes()).hexdigest(),report_sha256=hashlib.sha256(receipt_file.with_name('report.json').read_bytes()).hexdigest()))
    save('summary',dict(passed=True,operations=len(evidence),pauses=pauses,maximum_stop_to_resume_us=maximum,evidence=evidence,freezes=len(final['state']['freezes']),tools=len(listed)))
finally:
    save('calls',calls)
    if mcp is not None and mcp.poll() is None:mcp.terminate();mcp.wait(timeout=5)
    if engine is not None and engine.poll() is None:engine.terminate();engine.wait(timeout=10)
    if host is not None:
        (OUT/'stop').touch()
        try:host.wait(timeout=5)
        except subprocess.TimeoutExpired:host.terminate();host.wait(timeout=5)
    save('execution',[{'pid':c.pid,'exit':c.poll()} for c in children])
print(json.dumps({'passed':True,'cleanup':True}),flush=True)

