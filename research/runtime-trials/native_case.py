"""Shared runtime edit/recovery checks with an unchanged owned ABI fixture."""
import hashlib, json, os, queue, struct, subprocess, sys, threading, time
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

manifest=json.loads((OUT.parent/'frozen.json').read_bytes())
for name,digest in manifest['binaries'].items(): assert hashlib.sha256((BIN/name).read_bytes()).hexdigest()==digest
try:
    host=subprocess.Popen([sys.executable,str(ROOT/'research/runtime-trials/host.py'),str(OUT),str(BIN/'reversebolt_path_fixture.dll')],
        creationflags=NO_WINDOW,stdout=(OUT/'host.stdout').open('w'),stderr=(OUT/'host.stderr').open('w'))
    children.append(host)
    deadline=time.monotonic()+10
    while not (OUT/'ready.json').exists():
        assert host.poll() is None and time.monotonic()<deadline; time.sleep(.02)
    identity=json.loads((OUT/'ready.json').read_bytes())
    engine=launch()
    mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,
        stderr=(OUT/'mcp.stderr').open('w'),text=True,encoding='utf-8',creationflags=NO_WINDOW)
    children.append(mcp)
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'runtime-trials-evaluation','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    listed=rpc('tools/list',{})['tools']; assert len(listed)==42
    for name in ('apply_runtime_change','restore_runtime_change'):
        tool=next(t for t in listed if t['name']=='reversebolt_'+name)
        assert tool['annotations']['destructiveHint'] and tool['annotations']['openWorldHint']
    save('tools',listed)
    inventory=call('list_processes',label='inventory')
    target=next(p for p in inventory['state']['processes'] if p['pid']==host.pid)
    attach=dict(pid=target['pid'],created=target['created'],window=target['window'],auto_observe=True,budget_mib=1)
    call('attach_process',attach,'attached'); state=done('discovery')
    candidate_id=next(c['candidate']['id'] for c in state['live']['runtime']['cards'] if c['candidate']['name']=='Coefficient')
    runtime=state['live']['runtime']
    call('search_runtime_values',dict(catalog_id=runtime['catalog_id'],query='Coefficient'),'search')
    call('show_runtime_values',dict(catalog_id=runtime['catalog_id'],after_revision=runtime['revision'],ids=[candidate_id]),'shown')
    trial=prepare('review'); oracle('review-unchanged',trial,3.5)
    written=apply('write',trial); oracle('written',trial,4.5)
    assert card(status()['state'])['value']=='4.5'
    count=operations(); assert apply('write-replay',trial)==written and operations()==count
    restored=apply('restore',trial,True); oracle('restored',trial,3.5)
    count=operations(); assert apply('restore-replay',trial,True)==restored and operations()==count
    stale=prepare('review-stale'); current=status(); runtime=current['state']['live']['runtime']
    call('start_runtime_job',dict(envelope(current),task=dict(operation='refresh',catalog_id=runtime['catalog_id'],after_revision=runtime['revision'],ids=[candidate_id])))
    done('refreshed'); count=operations()
    assert call('apply_runtime_change',dict(change_id=stale['id']),'stale')['data']['phase']=='expired'
    assert operations()==count; cases.append('stale-revision')
    scoped=prepare('review-scope'); call('detach_process'); count=operations()
    call('apply_runtime_change',dict(change_id=scoped['id']),'detached-refusal',False); assert operations()==count
    call('attach_process',attach); state=done('rediscovery'); count=operations()
    call('apply_runtime_change',dict(change_id=scoped['id']),'scope-refusal',False); assert operations()==count
    candidate_id=next(c['candidate']['id'] for c in state['live']['runtime']['cards'] if c['candidate']['name']=='Coefficient')
    runtime=state['live']['runtime']
    call('search_runtime_values',dict(catalog_id=runtime['catalog_id'],query='Coefficient'))
    call('show_runtime_values',dict(catalog_id=runtime['catalog_id'],after_revision=runtime['revision'],ids=[candidate_id]))
    moved=prepare('review-moved'); host_command('tick',mode=0)
    apply('moved-refusal',moved,outcome='refused'); oracle('moved-unchanged',moved,3.5)
    fresh=prepare('review-fresh'); apply('fresh-write',fresh); oracle('fresh-written',fresh,4.5)
    host_command('tick',mode=0); apply('restore-moved-refusal',fresh,True,'refused'); oracle('restore-refused',fresh,4.5)
    cleanup=prepare('review-cleanup','3.5'); apply('explicit-cleanup',cleanup); oracle('cleanup',cleanup,3.5)
    last=prepare('review-ui'); apply('ui-written',last); oracle('ui-written',last,4.5)
    save('processes',{'app_pid':engine.pid,'target_pid':host.pid,'session':identity['session']})
    (OUT/'ui-ready').touch(); print(json.dumps({'ui_ready':True,'app_pid':engine.pid}),flush=True)
    deadline=time.monotonic()+480
    while not (OUT/'finish').exists() and time.monotonic()<deadline:
        assert engine.poll() is None; time.sleep(.2)
    assert (OUT/'finish').exists(), 'UI inspection did not finish'
    apply('ui-restored',last,True); oracle('ui-restored',last,3.5)
    pending=prepare('review-before-restart'); before=status(); count=operations()
    assert not before['state']['live']['frames'] and before['state']['live']['sample_count']==0
    engine.terminate(); engine.wait(timeout=10); engine=launch()
    after=call('status',label='restarted')
    assert after['state']['live'] is None and operations()==count
    old={t['id']:t for t in before['state']['runtime_trials']}; new={t['id']:t for t in after['state']['runtime_trials']}
    assert old.keys()==new.keys()
    for key,value in old.items():
        if value['phase']=='prepared': assert new[key]['phase']=='expired' and new[key]['write'] is None
        else: assert value==new[key]
    call('apply_runtime_change',dict(change_id=pending['id']),'restart-refusal',False)
    assert operations()==count; cases.append('restart-history-no-replay')
    evidence=[]; pauses=0
    for receipt_file in (DATA/'runtime').glob('*-*/**/stdout.jsonl'):
        receipts=[json.loads(line) for line in receipt_file.read_text(encoding='utf-8').splitlines()]
        assert receipts[0]['stage']=='validated' and receipts[-1]['stage']=='complete' and receipts[-1]['helper_absent']
        report=json.loads(receipt_file.with_name('report.json').read_bytes())
        shots=report.get('pauses',report.get('snapshots',[])+[w['discarded_snapshot'] for w in report.get('warmups',[])])
        assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in shots)
        pauses+=len(shots)
        evidence.append({'path':str(receipt_file.parent),'receipt_sha256':hashlib.sha256(receipt_file.read_bytes()).hexdigest(),
            'report_sha256':hashlib.sha256(receipt_file.with_name('report.json').read_bytes()).hexdigest()})
    summary={'passed':True,'cases':cases,'operations':len(evidence),'pauses':pauses,'evidence':evidence,
        'screenshots':0,'gameplay_input':0,'verified_controls':0,'trials':len(new),'target':identity}
    save('summary',summary)
finally:
    save('calls',calls)
    if mcp is not None and mcp.poll() is None: mcp.terminate();mcp.wait(timeout=5)
    if engine is not None and engine.poll() is None: engine.terminate();engine.wait(timeout=10)
    if host is not None:
        (OUT/'stop').touch()
        try: host.wait(timeout=5)
        except subprocess.TimeoutExpired: host.terminate();host.wait(timeout=5)
    save('execution',[{'pid':c.pid,'exit':c.poll()} for c in children])
balance=json.loads((OUT/'balance.json').read_bytes())
assert host.returncode==0 and balance=={'starts':pauses,'stops':pauses},balance
print(json.dumps({'passed':True,'cases':len(cases),'operations':len(evidence),'balanced_pauses':pauses,'trials':len(new),'cleanup':True}),flush=True)
