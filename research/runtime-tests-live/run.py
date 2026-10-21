"""Automated native MCP workflow, executed only in Anode's child session."""
import base64,ctypes as C,hashlib,json,os,queue,subprocess,sys,threading,time,uuid
from pathlib import Path
from ctypes import wintypes as W
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve();OUT.mkdir(exist_ok=False)
BIN=OUT.parent/'bin';DATA=OUT/'data';NO_WINDOW=0x08000000
engine=mcp=host=None;children=[];lines=queue.Queue();seq=host_seq=0;calls=[];actions=[];assertions=[]
user=C.WinDLL('user32');user.GetForegroundWindow.restype=W.HWND;user.GetWindowThreadProcessId.argtypes=[W.HWND,C.POINTER(W.DWORD)]
user.GetAsyncKeyState.argtypes=[C.c_int];user.GetAsyncKeyState.restype=C.c_short
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def save(name,value):(OUT/(name+'.json')).write_text(json.dumps(value,indent=2),encoding='utf-8')
def verify(name,condition):
    assert condition,name
    assertions.append(name)
def rpc(method,params):
    global seq
    seq+=1;mcp.stdin.write(json.dumps({'jsonrpc':'2.0','id':seq,'method':method,'params':params})+'\n');mcp.stdin.flush()
    end=time.monotonic()+180
    while time.monotonic()<end:
        v=json.loads(lines.get(timeout=max(.1,end-time.monotonic())))
        if v.get('id')==seq:assert 'error' not in v,v;return v['result']
    raise AssertionError('RPC timeout')
def call(name,args=None,ok=True,label=None):
    response=rpc('tools/call',{'name':'reversebolt_'+name,'arguments':args or {}})
    value=json.loads(next(x['text'] for x in response['content'] if x['type']=='text'))
    calls.append({'name':name,'ok':value['ok'],'error':value.get('error')})
    if label:save(label,value)
    assert value['ok']==ok,(name,value.get('error'))
    if name=='read_runtime_test' and args.get('frame') is not None and ok:
        images=[c for c in response['content'] if c['type']=='image'];assert len(images)==1
        expected=value['data']['test']['conditions'][args['condition']]['frames'][args['frame']]['sha256']
        assert hashlib.sha256(base64.b64decode(images[0]['data'])).hexdigest()==expected
    return value
def status():return call('status')
def ops():return len(list((DATA/'runtime').glob('*-*/**/stdout.jsonl')))
def launch(gui=False):
    p=subprocess.Popen([str(BIN/'Reversebolt.exe')]+([] if gui else ['--headless'])+['--data-dir',str(DATA)],creationflags=NO_WINDOW,
        stdout=(OUT/('engine-'+str(len(children))+'.stdout')).open('w'),stderr=(OUT/('engine-'+str(len(children))+'.stderr')).open('w'));children.append(p)
    end=time.monotonic()+20
    while time.monotonic()<end:
        assert p.poll() is None
        try:
            if read(DATA/'endpoint.json')['pid']==p.pid:return p
        except (FileNotFoundError,json.JSONDecodeError):pass
        time.sleep(.05)
    raise AssertionError('Engine startup timed out')
def host_call(operation,**args):
    global host_seq
    host_seq+=1;(OUT/'request.json').write_text(json.dumps(dict(id=host_seq,operation=operation,**args)));end=time.monotonic()+5
    while time.monotonic()<end:
        assert host.poll() is None
        try:
            r=read(OUT/'response.json')
            if r['id']==host_seq:return r
        except (FileNotFoundError,json.JSONDecodeError):pass
        time.sleep(.01)
    raise AssertionError('Fixture timeout')
def attach():
    p=next(p for p in call('list_processes')['state']['processes'] if p['pid']==host.pid and p['window']==host_info['window'])
    call('attach_process',dict(pid=p['pid'],created=p['created'],window=p['window'],budget_mib=1,auto_observe=True))
    end=time.monotonic()+160
    while time.monotonic()<end:
        s=status();jobs=s['jobs']['history']['jobs']
        if jobs and all(j['phase'].lower() in ('completed','failed','cancelled','interrupted') for j in jobs.values()):
            assert s['state']['live']['runtime']['cards'];return
        time.sleep(.1)
    raise AssertionError('Discovery timeout')
def prepare():
    live=status()['state']['live'];rt=live['runtime'];candidate=next(c['candidate'] for c in rt['cards'] if c['candidate']['name']=='Coefficient')
    args=dict(test_id=str(uuid.uuid4()),change_id=str(uuid.uuid4()),investigation_id=live['investigation_id'],catalog_id=rt['catalog_id'],
        after_revision=rt['revision'],candidate_id=candidate['id'],value='0',hypothesis='The selected field may suppress the fixture response',
        actions=[{'keys':[k],'hold_ms':200} for k in [68,65,68,65]])
    t=call('prepare_runtime_test',args)['data'];count=ops()
    verify('prepare replay has no helper',call('prepare_runtime_test',args)['data']==t and ops()==count)
    full=call('read_runtime_test',{'test_id':t['id']})['data']['test']
    host_call('configure',address=full['preparation']['report']['checks'][0]['field_address'])
    return t,args
def edit_args(t,args):
    live=status()['state']['live']
    return dict(change_id=t['change_id'],investigation_id=live['investigation_id'],catalog_id=live['runtime']['catalog_id'],
        after_revision=live['runtime']['revision'],candidate_id=args['candidate_id'],value=args['value'])
def key(vk):
    hwnd=user.GetForegroundWindow();pid=W.DWORD();user.GetWindowThreadProcessId(hwnd,C.byref(pid))
    assert hwnd==host_info['window'] and pid.value==host.pid,'Owned fixture is not foreground'
    start=time.time_ns();user.keybd_event(vk,0,0,0)
    try:time.sleep(.2)
    finally:user.keybd_event(vk,0,2,0)
    actions.append({'key':vk,'hold_ms':200,'started_ns':start,'finished_ns':time.time_ns()})
    time.sleep(.3)
def condition(t,with_input=True):
    for index in range(6):
        if index>=2 and with_input:key(t['actions'][index-2]['keys'][0])
        elif index>0:time.sleep(.35)
        revision=t['revision']
        t=call('observe_runtime_test',{'test_id':t['id'],'after_revision':revision},label=f"{t['id']}-{revision}")['data']
        count=len(list((DATA/'runtime-tests').glob('*/*.png')))
        call('observe_runtime_test',{'test_id':t['id'],'after_revision':revision},ok=False)
        verify('stale observation does not capture',len(list((DATA/'runtime-tests').glob('*/*.png')))==count)
    return t
try:
    host=subprocess.Popen([sys.executable,str(ROOT/'research/runtime-tests-live/host.py'),str(OUT),str(BIN/'reversebolt_path_fixture.dll')],
        creationflags=NO_WINDOW,stdout=(OUT/'host.stdout').open('w'),stderr=(OUT/'host.stderr').open('w'));children.append(host)
    end=time.monotonic()+15
    while not (OUT/'ready.json').exists():assert host.poll() is None and time.monotonic()<end;time.sleep(.05)
    host_info=read(OUT/'ready.json');verify('fixture is in Anode',host_info['session']!=1)
    print(json.dumps({'fixture_focus_ready':host_info}),flush=True)
    end=time.monotonic()+90
    while not (OUT/'fixture-focused').exists():
        assert host.poll() is None and time.monotonic()<end,'Anode focus setup timed out'
        time.sleep(.05)
    engine=launch();mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,
        stderr=(OUT/'mcp.stderr').open('w'),text=True,encoding='utf-8',creationflags=NO_WINDOW);children.append(mcp)
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'response-test-native','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    verify('49 MCP tools',len(rpc('tools/list',{})['tools'])==49);attach()
    t,args=prepare();first_id=t['id'];count=ops()
    call('set_runtime_value',edit_args(t,args),ok=False);verify('premature edit does not invoke helper',ops()==count)
    t=condition(t);verify('strong input baseline ready',t['phase']=='ready_to_apply')
    trial=call('set_runtime_value',edit_args(t,args),label='write')['data'];verify('linked edit written',trial['phase']=='written')
    t=next(t for t in status()['state']['runtime_tests'] if t['id']==first_id)
    verify('changed phase synchronized',t['phase']=='changed');t=condition(t)
    verify('changed observations ready for Undo',t['phase']=='ready_to_restore')
    restored=call('restore_runtime_change',{'change_id':trial['id']},label='restored')['data'];verify('linked edit restored',restored['phase']=='restored')
    t=next(t for t in status()['state']['runtime_tests'] if t['id']==first_id);t=condition(t)
    verify('full response suppression observed',t['phase']=='complete' and t['assessment']['status']=='suppression_observed')
    save('complete',call('read_runtime_test',{'test_id':first_id})['data'])
    oracle=host_call('read',address=trial['patch']['field_address'],width=len(trial['patch']['expected']));save('original-oracle',oracle)
    verify('independent original bytes',oracle['bytes']==trial['patch']['expected'])
    call('read_runtime_test',{'test_id':first_id,'condition':2,'frame':5})
    t,args=prepare();t=condition(t,False);verify('missing action stops baseline',t['phase']=='stopped')
    count=ops();call('set_runtime_value',edit_args(t,args),ok=False);verify('stopped plan cannot write',count==ops())
    t,args=prepare();stopped=call('stop_runtime_test',{'test_id':t['id']})['data'];verify('stop replay',call('stop_runtime_test',{'test_id':t['id']})['data']==stopped)
    verify('prepare replay cannot resume stopped test',call('prepare_runtime_test',args)['data']['phase']=='stopped')
    t,args=prepare();manual=edit_args(t,args);manual['change_id']=str(uuid.uuid4());manual['value']='4.5'
    other=call('set_runtime_value',manual)['data'];verify('another edit interrupts comparison',call('read_runtime_test',{'test_id':t['id']})['data']['test']['phase']=='stopped')
    call('restore_runtime_change',{'change_id':other['id']})
    t,args=prepare();call('detach_process');verify('detach interrupts comparison',call('read_runtime_test',{'test_id':t['id']})['data']['test']['phase']=='stopped');attach()
    t,args=prepare();pending=DATA/'runtime-tests'/f"{t['id']}.pending";pending.mkdir()
    call('observe_runtime_test',{'test_id':t['id'],'after_revision':t['revision']},ok=False)
    verify('storage failure stops sampling state',call('read_runtime_test',{'test_id':t['id']})['data']['test']['phase']=='stopped')
    pending.rmdir();engine.terminate();engine.wait(timeout=10);engine=launch()
    verify('restart retains stopped state',call('read_runtime_test',{'test_id':t['id']})['data']['test']['phase']=='stopped')
    call('read_runtime_test',{'test_id':first_id,'condition':0,'frame':2})
    verify('restart has no live target',status()['state']['live'] is None)
    engine.terminate();engine.wait(timeout=10);engine=launch(gui=True);attach();t,args=prepare()
    save('human-stop-ready',{'pid':engine.pid,'test_id':t['id']});print(json.dumps({'human_takeover_ready':engine.pid,'test_id':t['id']}),flush=True)
    end=time.monotonic()+90
    while time.monotonic()<end:
        s=status();current=next(x for x in s['state']['runtime_tests'] if x['id']==t['id'])
        if current['phase']=='stopped' and s['jobs']['history']['human_hold'] is not None:break
        time.sleep(.25)
    else:raise AssertionError('Native desktop takeover did not stop the test')
    verify('native desktop takeover stops comparison',True)
    call('observe_runtime_test',{'test_id':t['id'],'after_revision':current['revision']},ok=False)
    call('read_runtime_test',{'test_id':t['id']})
    verify('human hold permits evidence but blocks agent observation',True)
    save('result',{'passed':True,'assertions':assertions,'helpers':ops(),'input_actions':actions,'calls':calls,'first_test':first_id,
        'images':len(list((DATA/'runtime-tests').glob('*/*.png'))),'manually_reviewed_images':0,'limitation':'Owned ABI/visual fixture only; no real-game semantics or generalization evidence.'})
    print(json.dumps({'passed':True,'checks':len(assertions),'helpers':ops(),'inputs':len(actions)}),flush=True)
except BaseException as error:
    save('failure',{'error':repr(error),'assertions':assertions,'calls':calls,'input_actions':actions});raise
finally:
    if host is not None:
        (OUT/'stop').write_text('stop')
        try:host.wait(timeout=5)
        except subprocess.TimeoutExpired:host.terminate();host.wait(timeout=5)
    for child in [mcp,engine]:
        if child is not None and child.poll() is None:child.terminate();child.wait(timeout=10)
    save('children', [{'pid':p.pid,'exit_code':p.poll()} for p in children])
