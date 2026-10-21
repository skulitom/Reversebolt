"""Track internal values through GUI/shared engine/MCP using the unchanged owned fixture."""
import collections, ctypes, hashlib, json, os, queue, subprocess, sys, threading, time
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
OUT=Path(sys.argv[1]).resolve(); OUT.mkdir(parents=True,exist_ok=False)
BIN=OUT.parent/'bin'
DATA=OUT/'data'
owned=sys.argv[2]=='fixture'
assert owned, 'This native runner requires the unchanged owned fixture.'
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
        host=subprocess.Popen([sys.executable,str(ROOT/'research/runtime-product/fixture_host.py'),str(OUT),str(BIN/'reversebolt_path_fixture.dll')],
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
    tools=rpc('tools/list',{})['tools'];assert len(tools)==39
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
    selected=[next(c for c in cards if c['candidate']['name']==name) for name in ('Identity','Coefficient')]
    ids=[c['candidate']['id'] for c in selected]
    def start_watch(count,interval,label):
        current=status(); runtime=current['state']['live']['runtime']
        request=dict(envelope(current),task={'operation':'refresh','catalog_id':runtime['catalog_id'],'after_revision':runtime['revision'],'ids':ids},count=count,interval_ms=interval)
        receipt=call('start_runtime_job',request,label=label+'-receipt');assert receipt['ok'],receipt.get('error')
        return request,receipt['data']['ack']['job']
    def controls(operation,job):
        response=call('job_control',dict(envelope(status()),operation=operation,job=job))
        assert response['ok'],response.get('error')
    def pause_after(number,job,label):
        deadline=time.monotonic()+90
        while time.monotonic()<deadline:
            current=status(); runtime=current['state']['live']['runtime']
            card=next(c for c in runtime['cards'] if c['candidate']['id']==ids[0])
            if len(card['readings'])>=number:break
            assert current['jobs']['history']['jobs'][str(job)]['phase'] not in ('Failed','Cancelled','Interrupted')
            time.sleep(.04)
        assert len(card['readings'])==number
        controls('pause',job)
        deadline=time.monotonic()+90
        while time.monotonic()<deadline:
            current=status()
            if current['jobs']['history']['jobs'][str(job)]['phase']=='Paused':break
            time.sleep(.04)
        assert current['jobs']['history']['jobs'][str(job)]['phase']=='Paused'
        assert len(next(c for c in current['state']['live']['runtime']['cards'] if c['candidate']['id']==ids[0])['readings'])==number
        save(label,current)
    request,job=start_watch(4,1000,'watch')
    pause_after(1,job,'first-paused');fixture_mode(1);controls('resume',job)
    pause_after(2,job,'null-paused');fixture_mode(0);controls('resume',job)
    state=done('watch');runtime=state['live']['runtime']
    selected=[next(c for c in runtime['cards'] if c['candidate']['id']==id) for id in ids]
    for card in selected:
        readings=card['readings']
        assert [r['status'] for r in readings]==['read','null_reference','read','read']
        assert readings[1]['value'] is None and readings[1]['bytes']==[]
        assert [r['revision'] for r in readings]==[1,2,3,4]
        assert all(r['completed_unix_ms']>=r['started_unix_ms'] for r in readings)
        assert all(b['started_unix_ms']>=a['completed_unix_ms']+1000 for a,b in zip(readings,readings[1:]))
    assert selected[0]['readings'][2]['bytes']!=selected[0]['readings'][3]['bytes']
    assert selected[1]['readings'][2]['bytes']==selected[1]['readings'][3]['bytes']
    before=operation_count()
    replay=call('start_runtime_job',request,label='replay');assert replay['ok'] and replay['data']['duplicate']
    assert replay['data']['ack']['job']==job
    stale=call('start_runtime_job',dict(envelope(status()),task=request['task'],count=4,interval_ms=1000),label='stale')
    assert not stale['ok'] and operation_count()==before
    _,rollover=start_watch(16,250,'rollover')
    state=done('rollover');runtime=state['live']['runtime']
    selected=[next(c for c in runtime['cards'] if c['candidate']['id']==id) for id in ids]
    for card in selected:
        assert len(card['readings'])==16 and card['readings_dropped']==4
        assert [r['revision'] for r in card['readings']]==list(range(5,21))
        assert all(r['status']=='read' for r in card['readings'])
        assert all(b['started_unix_ms']>=a['completed_unix_ms']+250 for a,b in zip(card['readings'],card['readings'][1:]))
    _,inflight=start_watch(2,1000,'inflight-pause')
    deadline=time.monotonic()+90
    while time.monotonic()<deadline:
        current=status();runtime=current['state']['live']['runtime']
        if runtime['phase']=='inspecting' and runtime['revision']==20:break
        assert runtime['revision']==20,'First check finished before in-flight pause could be tested'
        assert current['jobs']['history']['jobs'][str(inflight)]['phase'] not in ('Failed','Cancelled','Interrupted')
        time.sleep(.04)
    assert runtime['phase']=='inspecting' and runtime['revision']==20
    controls('pause',inflight)
    deadline=time.monotonic()+90
    while time.monotonic()<deadline:
        current=status()
        if current['jobs']['history']['jobs'][str(inflight)]['phase']=='Paused':break
        time.sleep(.04)
    assert current['jobs']['history']['jobs'][str(inflight)]['phase']=='Paused'
    assert current['state']['live']['runtime']['revision']==21
    save('inflight-paused',current)
    controls('resume',inflight)
    state=done('inflight-resumed');runtime=state['live']['runtime']
    assert runtime['revision']==22
    selected=[next(c for c in runtime['cards'] if c['candidate']['id']==id) for id in ids]
    for card in selected:
        assert len(card['readings'])==16 and card['readings_dropped']==6
        assert [r['revision'] for r in card['readings']]==list(range(7,23))
        assert all(r['status']=='read' for r in card['readings'])
    before=operation_count()
    _,cancelled=start_watch(3,1000,'cancelled')
    controls('pause',cancelled)
    deadline=time.monotonic()+90
    while time.monotonic()<deadline:
        current=status()
        if current['jobs']['history']['jobs'][str(cancelled)]['phase']=='Paused':break
        time.sleep(.04)
    assert current['jobs']['history']['jobs'][str(cancelled)]['phase']=='Paused'
    paused_operations=operation_count()
    controls('cancel',cancelled)
    deadline=time.monotonic()+90
    while time.monotonic()<deadline:
        current=status()
        if current['jobs']['history']['jobs'][str(cancelled)]['phase']=='Cancelled':break
        time.sleep(.04)
    assert current['jobs']['history']['jobs'][str(cancelled)]['phase']=='Cancelled'
    assert operation_count()==paused_operations
    # Pause may drain one already-started helper; it must not dispatch later steps.
    assert before<=paused_operations<=before+1
    save('cancelled',current)
    assert current['state']['live']['sample_count']==0 and not current['state']['live']['frames']
    evidence=[];pauses=0
    for receipt_file in (DATA/'runtime').glob('*-*/**/stdout.jsonl'):
        receipts=[json.loads(line) for line in receipt_file.read_text().splitlines()]
        assert receipts[0]['stage']=='validated' and receipts[-1]['stage']=='complete' and receipts[-1]['helper_absent']
        r=json.loads(receipt_file.with_name('report.json').read_bytes())
        shots=r.get('pauses',r.get('snapshots',[])+[w['discarded_snapshot'] for w in r.get('warmups',[])])
        assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in shots)
        pauses+=len(shots)
        evidence.append({'path':str(receipt_file.parent),'receipt_sha256':hashlib.sha256(receipt_file.read_bytes()).hexdigest(),'report_sha256':hashlib.sha256(receipt_file.with_name('report.json').read_bytes()).hexdigest()})
    for card in selected:
        for reading in card['readings']:
            r=json.loads(Path(reading['evidence_path']).read_bytes())
            canonical=json.dumps(r,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()
            assert hashlib.sha256(canonical).hexdigest()==reading['report_sha256']
            check=next(c for c in r['checks'] if c['path_id']==card['candidate']['id'])
            assert check['bytes']==reading['bytes'] and check['value']==reading['value']
    summary={'passed':True,'app_pid':engine.pid,'target_pid':target_pid,'cards':len(cards),'selected':[{'id':c['candidate']['id'],'name':c['candidate']['name'],'readings':len(c['readings']),'dropped':c['readings_dropped'],'differences':sum(a['bytes']!=b['bytes'] for a,b in zip(c['readings'],c['readings'][1:]))} for c in selected],
        'watch_job':job,'rollover_job':rollover,'inflight_pause_job':inflight,'cancelled_job':cancelled,'operations':len(evidence),'pauses':pauses,'evidence':evidence,'screenshots':0,'gameplay_input':0,'gameplay_writes':0,'verified_controls':0}
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
