"""Exercise GUI/shared engine via real stdio MCP. Runs inside Anode, no pixels/input."""
import collections, ctypes, hashlib, json, os, queue, subprocess, sys, threading, time
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
OUT=Path(sys.argv[1]).resolve(); OUT.mkdir(parents=True,exist_ok=False)
BIN=OUT.parent/'bin'
DATA=OUT/'data'
owned=sys.argv[2]=='fixture'
target_pid=None if owned else int(sys.argv[2])
engine=mcp=host=None
lines=queue.Queue(); message_id=0; calls=[]
CREATE_NO_WINDOW=0x08000000
def save(name,value):
    (OUT/(name+'.json')).write_text(json.dumps(value,indent=2),encoding='utf-8')
def rpc(method,params):
    global message_id
    message_id+=1
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','id':message_id,'method':method,'params':params})+'\n');mcp.stdin.flush()
    deadline=time.monotonic()+160
    while time.monotonic()<deadline:
        result=json.loads(lines.get(timeout=max(.1,deadline-time.monotonic())))
        if result.get('id')==message_id:
            assert 'error' not in result,result
            return result['result']
    raise AssertionError('MCP response deadline')
def call(name,args=None,label=None):
    result=rpc('tools/call',{'name':'reversebolt_'+name,'arguments':args or {}})
    texts=[c['text'] for c in result.get('content',[]) if c['type']=='text']
    value=json.loads(texts[0])
    calls.append({'tool':name,'ok':value.get('ok'),'error':value.get('error')})
    if label:save(label,value)
    return value
def status():
    result=call('status');assert result['ok'],result.get('error')
    return result
def envelope(state):
    h=state['jobs']['history']
    return {'boot':h['boot'],'binding':h['current'],
            'request_id':h['clients'].get('mcp-agent',{}).get('high_water',0)+1}
def done(label):
    deadline=time.monotonic()+160
    while time.monotonic()<deadline:
        result=status(); jobs=result['jobs']['history']['jobs']
        phases=[j['phase'].lower() for j in jobs.values()]
        if phases and all(p in ('completed','failed','cancelled','interrupted') for p in phases):
            save(label,result)
            assert phases[-1]=='completed',(phases,result.get('error'),result['state']['live']['runtime'])
            return result['state']
        time.sleep(.35)
    raise AssertionError('Runtime job deadline')
def operation_count():
    return len(list((DATA/'runtime').glob('*-*/**/stdout.jsonl')))
def refresh(task,label):
    request=dict(envelope(status()),task=task)
    receipt=call('start_runtime_job',request,label+'-receipt')
    assert receipt['ok'],receipt.get('error')
    state=done(label)
    return request,receipt,state
def fixture_mode(value):
    (OUT/'mode.txt').write_text(str(value))
    deadline=time.monotonic()+5
    while time.monotonic()<deadline:
        try:
            if (OUT/'mode.ack').read_text()==str(value):return
        except FileNotFoundError:pass
        assert host.poll() is None
        time.sleep(.04)
    raise AssertionError('Fixture mode acknowledgement missing')
try:
    if owned:
        (OUT/'mode.txt').write_text('0')
        host=subprocess.Popen([sys.executable,str(Path(__file__).with_name('fixture_host.py')),str(OUT),str(BIN/'reversebolt_path_fixture.dll')],
            env=dict(os.environ,REVERSEBOLT_PATH_FIXTURE_CONTROL=str(OUT/'mode.txt')),
            stdout=(OUT/'host.stdout').open('w'),stderr=(OUT/'host.stderr').open('w'),creationflags=CREATE_NO_WINDOW)
        target_pid=host.pid
    engine=subprocess.Popen([str(BIN/'Reversebolt.exe'),'--data-dir',str(DATA)],creationflags=CREATE_NO_WINDOW,
        stdout=(OUT/'app.stdout').open('w'),stderr=(OUT/'app.stderr').open('w'))
    deadline=time.monotonic()+20
    while not (DATA/'endpoint.json').exists():
        assert engine.poll() is None and time.monotonic()<deadline
        time.sleep(.05)
    save('processes',{'app_pid':engine.pid,'target_pid':target_pid,'owned_fixture':owned})
    mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,
        stderr=(OUT/'mcp.stderr').open('w'),text=True,encoding='utf-8',creationflags=CREATE_NO_WINDOW)
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'runtime-product-evaluation','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    tools=rpc('tools/list',{})['tools'];assert len(tools)==37
    inventory=call('list_processes',label='inventory')
    target=next(p for p in inventory['state']['processes'] if p['pid']==target_pid)
    attached=call('attach_process',dict(pid=target['pid'],created=target['created'],window=target['window'],auto_observe=True,budget_mib=1),label='attached')
    assert attached['ok'],attached.get('error')
    state=done('discovery'); live=state['live']; runtime=live['runtime']
    assert runtime['available'] and runtime['phase']=='ready',runtime
    assert live['sample_count']==0 and not live['frames']
    assert len(status()['jobs']['history']['jobs'])==1
    cards=runtime['cards'];assert 0<len(cards)<=64
    assert len({c['candidate']['id'] for c in cards})==len(cards)
    assert all(c['status']=='read' and not c['candidate']['edit_authority'] for c in cards)
    report=json.loads((Path(runtime['evidence'])/'report.json').read_bytes())
    core=report.get('core_image')
    def image(card):
        last=card['candidate']['locator']['steps'][-1]
        return last['member'].get('field',{}).get('declaring',last['owner'])['image']
    selected=([c for c in cards if core and image(c)!=core] or cards)[:4]
    ids=[c['candidate']['id'] for c in selected]
    task={'operation':'refresh','catalog_id':runtime['catalog_id'],'after_revision':runtime['revision'],'ids':ids}
    start=time.monotonic();request,receipt,state=refresh(task,'refresh');elapsed_ms=round((time.monotonic()-start)*1000)
    current=state['live']['runtime'];assert current['revision']==1
    refreshed=[c for c in current['cards'] if c['candidate']['id'] in ids]
    assert all(c['status']=='read' for c in refreshed),refreshed
    count=operation_count()
    replay=call('start_runtime_job',request,label='replay');assert replay['ok']
    assert replay['data']['ack']['job']==receipt['data']['ack']['job']
    stale=call('start_runtime_job',dict(envelope(status()),task=task),label='stale')
    assert not stale['ok']
    assert operation_count()==count
    if owned:
        fixture_mode(1)
        nulltask=dict(task,after_revision=1)
        _,_,state=refresh(nulltask,'null-root')
        invalid=[c for c in state['live']['runtime']['cards'] if c['candidate']['id'] in ids]
        assert all(c['status']=='null_reference' and c['value'] is None and c['candidate']['value'] is not None for c in invalid),invalid
        fixture_mode(0)
        _,_,state=refresh(dict(task,after_revision=2),'restored')
        assert all(c['status']=='read' for c in state['live']['runtime']['cards'] if c['candidate']['id'] in ids)
    assert state['live']['sample_count']==0 and not state['live']['frames']
    evidence=[];pauses=0
    for receipt_file in (DATA/'runtime').glob('*-*/**/stdout.jsonl'):
        receipts=[json.loads(line) for line in receipt_file.read_text().splitlines()]
        assert receipts[0]['stage']=='validated' and receipts[-1]['stage']=='complete' and receipts[-1]['helper_absent']
        r=json.loads(receipt_file.with_name('report.json').read_bytes())
        shots=r.get('pauses',r.get('snapshots',[])+[w['discarded_snapshot'] for w in r.get('warmups',[])])
        assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in shots)
        pauses+=len(shots)
        evidence.append({'path':str(receipt_file.parent),'receipt_sha256':hashlib.sha256(receipt_file.read_bytes()).hexdigest(),'report_sha256':hashlib.sha256(receipt_file.with_name('report.json').read_bytes()).hexdigest()})
    summary={'passed':True,'app_pid':engine.pid,'target_pid':target_pid,'owned_fixture':owned,'cards':len(cards),
        'images':dict(collections.Counter(image(c) for c in cards)),'selected':[{'id':c['candidate']['id'],'name':c['candidate']['name'],'image':image(c),'steps':len(c['candidate']['locator']['steps'])} for c in selected],
        'refresh_ms':elapsed_ms,'operations':len(evidence),'pauses':pauses,'evidence':evidence,'screenshots':0,'gameplay_input':0,'gameplay_writes':0,'verified_controls':0}
    save('summary',summary);(OUT/'ready').touch();print(json.dumps(summary),flush=True)
    deadline=time.monotonic()+480
    while not (OUT/'finish').exists() and time.monotonic()<deadline:
        assert engine.poll() is None
        time.sleep(.25)
finally:
    save('calls',calls)
    if mcp is not None:
        mcp.terminate();mcp.wait(timeout=5)
    if engine is not None:
        engine.terminate();engine.wait(timeout=10)
    if host is not None:
        (OUT/'host-stop').touch()
        try:host.wait(timeout=5)
        except subprocess.TimeoutExpired:host.terminate();host.wait(timeout=5)
    save('execution',{'app_exit':None if engine is None else engine.returncode,'host_exit':None if host is None else host.returncode,'mcp_exit':None if mcp is None else mcp.returncode})
if owned:
    balance=json.loads((OUT/'host-balance.json').read_bytes())
    assert host.returncode==0 and balance=={'starts':pauses,'stops':pauses},balance
print('Cleanup complete',flush=True)
