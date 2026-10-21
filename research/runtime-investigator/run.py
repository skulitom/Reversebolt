"""Actual MCP/engine comparison client. Runs in Anode; selection has no game names."""
import ctypes as C,json,math,queue,subprocess,sys,threading,time,uuid
from ctypes import wintypes as W
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve();PID=int(sys.argv[2]);BIN=OUT/'bin';DATA=OUT/'data'
NO_WINDOW=0x08000000;engine=mcp=None;lines=queue.Queue();seq=0;actions=[];results=[];active_test=None;active_trial=None;children=[]
user=C.WinDLL('user32');user.GetForegroundWindow.restype=W.HWND;user.GetWindowThreadProcessId.argtypes=[W.HWND,C.POINTER(W.DWORD)]
user.MapVirtualKeyW.argtypes=[W.UINT,W.UINT];user.MapVirtualKeyW.restype=W.UINT
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def save(name,value):
    with (OUT/(name+'.json')).open('x',encoding='utf-8') as f:json.dump(value,f,indent=2)
def rpc(method,params):
    global seq
    seq+=1;mcp.stdin.write(json.dumps({'jsonrpc':'2.0','id':seq,'method':method,'params':params})+'\n');mcp.stdin.flush();end=time.monotonic()+180
    while time.monotonic()<end:
        r=json.loads(lines.get(timeout=max(.1,end-time.monotonic())))
        if r.get('id')==seq:assert 'error' not in r,r;return r['result']
    raise AssertionError('RPC outcome unknown; do not replay with a new identity')
def call(name,args=None):
    stamp=str(time.time_ns())+'-'+name
    if name!='status':save(stamp+'-request',args or {})
    r=rpc('tools/call',{'name':'reversebolt_'+name,'arguments':args or {}})
    v=json.loads(next(c['text'] for c in r['content'] if c['type']=='text'))
    if name!='status':save(stamp+'-response',v)
    assert v.get('ok'),(name,v.get('error'))
    return v
def status():return call('status')
def envelope(s):
    h=s['jobs']['history'];return {'boot':h['boot'],'binding':h['current'],'request_id':h['clients'].get('mcp-agent',{}).get('high_water',0)+1}
def settle():
    end=time.monotonic()+240
    while time.monotonic()<end:
        s=status();jobs=s['jobs']['history']['jobs']
        if jobs and all(j['phase'] in ('Completed','Failed','Cancelled','Interrupted') for j in jobs.values()):
            assert all(j['phase']=='Completed' for j in jobs.values()),[(j['id'],j['phase']) for j in jobs.values()]
            return s
        time.sleep(.4)
    raise AssertionError('Job still pending; retain its state/handle')
def foreground():
    hwnd=user.GetForegroundWindow();pid=W.DWORD();user.GetWindowThreadProcessId(hwnd,C.byref(pid))
    assert pid.value==PID and hwnd==target['window'],'Game focus changed; no input sent'
def key(vk):
    foreground();scan=user.MapVirtualKeyW(vk,0);assert scan
    receipt={'key':vk,'scan':scan,'planned_ms':200,'started_ns':time.time_ns()}
    user.keybd_event(0,scan,8,0)
    try:time.sleep(.2)
    finally:user.keybd_event(0,scan,8|2,0)
    receipt['finished_ns']=time.time_ns();actions.append(receipt)
    save('input-'+str(len(actions)),receipt);time.sleep(.35)
def condition(t):
    for i in range(6):
        foreground()
        if i>=2:key(t['actions'][i-2]['keys'][0])
        elif i:time.sleep(.35)
        t=call('observe_runtime_test',{'test_id':t['id'],'after_revision':t['revision']})['data']
        if t['phase']=='stopped':break
    return t
def refresh(ids):
    s=status();rt=s['state']['live']['runtime']
    call('start_runtime_job',dict(envelope(s),task={'operation':'refresh','catalog_id':rt['catalog_id'],'after_revision':rt['revision'],'ids':ids},count=1,interval_ms=250))
    rt=settle()['state']['live']['runtime'];checks={}
    for card in rt['cards']:
        if card['candidate']['id'] in ids:
            report=read(Path(card['readings'][-1]['evidence_path']))
            for check in report['checks']:
                if check['path_id'] in ids:checks[check['path_id']]=check
    assert set(checks)==set(ids) and all(c['status']=='read' for c in checks.values())
    return checks
def selection(s):
    rt=s['state']['live']['runtime'];review=rt['naming'];assert review and review['status']=='complete'
    index=call('read_runtime_inventory',{'catalog_id':rt['catalog_id']})['data'];types={t['id']:t for t in index['types']}
    chosen=None
    for h in review['batch']['accepted']:
        fields={f['id']:f for f in types[h['candidate']['type_id']]['fields']};members=[]
        for row in [h['candidate']]+h['alternatives']:
            if row['type_id']!=h['candidate']['type_id'] or any(m['field_id']==row['field_id'] for m in members):continue
            f=fields[row['field_id']]
            try:value=float(row['value'])
            except (TypeError,ValueError):continue
            if f['type_code'] in (12,13) and math.isfinite(value) and 0<value<=20 and 1<=f['paths']<=4:members.append(row)
        if len(members)>=2:chosen=(h,members[:2],fields);break
    assert chosen,'No eligible competing hypothesis; policy abstained'
    h,members,fields=chosen
    observed=call('read_runtime_inventory',{'catalog_id':rt['catalog_id'],'selected_types':[h['candidate']['type_id']]})['data']['selection']
    selections=[]
    for member in members:
        samples=[r for r in observed['rows'] if r['field_id']==member['field_id']]
        assert len(samples)==fields[member['field_id']]['paths'],'Path samples truncated'
        ids=[r['path_id'] for r in samples]
        search=call('search_runtime_values',{'catalog_id':rt['catalog_id'],'query':member['field_name']})['data']['search']
        assert set(ids).issubset({c['id'] for c in search['candidates']}),'Search page does not contain every selected path'
        current=status()['state']['live']['runtime']
        call('show_runtime_values',{'catalog_id':rt['catalog_id'],'after_revision':current['revision'],'ids':ids})
        checks=refresh(ids);values=list(checks.values())
        assert len({c['field_address'] for c in values})==1 and all(c['bytes']==values[0]['bytes'] for c in values),'Multiple current objects remain ambiguous'
        value=float(values[0]['value']);assert math.isfinite(value) and 0<value<=20
        selections.append({'row':member,'paths':ids,'checks':checks,'reference':{'review_id':review['evidence_id'],'hypothesis_path_id':h['candidate']['path_id'],'selected_path_id':member['path_id']}})
    save('selection',{'hypothesis':h,'candidates':selections,'report_sha256':index['report_sha256']})
    return selections
try:
    kernel=C.WinDLL('kernel32');session=W.DWORD();assert kernel.ProcessIdToSessionId(__import__('os').getpid(),C.byref(session)) and session.value==3
    assert not (OUT/'result.json').exists() and not DATA.exists(),'Never replay a started experiment'
    engine=subprocess.Popen([str(BIN/'Reversebolt.exe'),'--headless','--data-dir',str(DATA)],creationflags=NO_WINDOW,stdout=(OUT/'engine.stdout').open('w'),stderr=(OUT/'engine.stderr').open('w'));children.append(engine)
    end=time.monotonic()+15
    while not (DATA/'endpoint.json').exists():assert engine.poll() is None and time.monotonic()<end;time.sleep(.05)
    mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],creationflags=NO_WINDOW,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(OUT/'mcp.stderr').open('w'),text=True,encoding='utf-8');children.append(mcp)
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'generic-runtime-investigation','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    target=next(p for p in call('list_processes')['state']['processes'] if p['pid']==PID);save('target',target);foreground()
    call('set_observer_model',{'model':'qwen3.8:latest'})
    call('attach_process',dict(pid=target['pid'],created=target['created'],window=target['window'],auto_observe=True,budget_mib=1))
    s=settle();save('discovery',s);selected=selection(s)
    print(json.dumps({'selected':[c['row']['field_name'] for c in selected]}),flush=True)
    for index,item in enumerate(selected):
        live=status()['state']['live'];rt=live['runtime'];active_test=str(uuid.uuid4());active_trial=str(uuid.uuid4())
        plan={'test_id':active_test,'change_id':active_trial,'investigation_id':live['investigation_id'],'catalog_id':rt['catalog_id'],'after_revision':rt['revision'],
            'hypothesis_ref':item['reference'],'value':'0','actions':[{'keys':[k],'hold_ms':200} for k in [68,65,68,65]]}
        save('candidate-'+str(index)+'-plan',plan)
        t=call('prepare_runtime_test',plan)['data'];t=condition(t)
        if t['phase']!='ready_to_apply':
            results.append({'candidate':index,'status':'no_baseline','test':call('read_runtime_test',{'test_id':active_test})['data']['test']});active_test=active_trial=None;continue
        live=status()['state']['live'];trial=call('set_runtime_value',{'change_id':active_trial,'investigation_id':live['investigation_id'],'catalog_id':live['runtime']['catalog_id'],
            'after_revision':live['runtime']['revision'],'candidate_id':t['candidate_id'],'value':'0'})['data'];assert trial['phase']=='written'
        t=next(t for t in status()['state']['runtime_tests'] if t['id']==active_test);t=condition(t)
        restored=call('restore_runtime_change',{'change_id':active_trial})['data'];assert restored['phase']=='restored','Recovery unresolved; stop'
        t=next(t for t in status()['state']['runtime_tests'] if t['id']==active_test)
        if t['phase']=='restored':t=condition(t)
        full=call('read_runtime_test',{'test_id':active_test})['data']['test']
        outcome={'candidate':index,'test':full,'trial':restored};results.append(outcome);save('candidate-'+str(index)+'-result',outcome)
        print(json.dumps({'candidate':index,'phase':t['phase'],'assessment':t['assessment']}),flush=True)
        active_test=active_trial=None
    save('result',{'completed':True,'outcomes':results,'input_actions':actions,'manual_measurement_review':0})
except BaseException as error:
    save('failure',{'error':repr(error),'outcomes':results,'input_actions':actions});raise
finally:
    if mcp is not None and mcp.poll() is None:
        try:
            s=status()
            if active_test and any(t['id']==active_test and t['active'] for t in s['state']['runtime_tests']):call('stop_runtime_test',{'test_id':active_test})
            trial=next((t for t in s['state']['runtime_trials'] if t['id']==active_trial),None)
            if trial and trial['phase']=='written' and trial.get('restoration') is None:save('failure-recovery',call('restore_runtime_change',{'change_id':active_trial}))
            save('final-state',status())
        except BaseException as error:save('cleanup-error',{'error':repr(error)})
    for p in [mcp,engine]:
        if p is not None and p.poll() is None:p.terminate();p.wait(timeout=10)
    save('children',[{'pid':p.pid,'exit_code':p.poll()} for p in children])
