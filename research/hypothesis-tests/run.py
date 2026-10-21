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
def prepare_choice(reference, expected_path):
    live=status()['state']['live'];rt=live['runtime']
    args=dict(test_id=str(uuid.uuid4()),change_id=str(uuid.uuid4()),investigation_id=live['investigation_id'],catalog_id=rt['catalog_id'],
        after_revision=rt['revision'],hypothesis_ref=reference,value='0',
        actions=[{'keys':[k],'hold_ms':200} for k in [68,65,68,65]])
    before=ops()
    for bad in [dict(reference,review_id=str(uuid.uuid4())),dict(reference,selected_path_id='f'*64)]:
        call('prepare_runtime_test',dict(args,hypothesis_ref=bad),ok=False)
    call('prepare_runtime_test',dict(args,candidate_id='e'*64),ok=False)
    call('prepare_runtime_test',dict(args,hypothesis='A conflicting question?'),ok=False)
    verify('invalid references and conflicts invoke no helper',ops()==before)
    t=call('prepare_runtime_test',args)['data'];count=ops()
    verify('prepare resolves selected path',t['candidate_id']==expected_path and t['hypothesis_ref']==reference)
    verify('prepare replay invokes no helper',call('prepare_runtime_test',args)['data']==t and ops()==count)
    full=call('read_runtime_test',{'test_id':t['id']})['data']['test']
    verify('exact origin retained',full['origin']['reference']==reference and full['hypothesis']==full['origin']['hypothesis']['proposal']['question'])
    host_call('configure',address=full['preparation']['report']['checks'][0]['field_address'])
    # Existing direct edit consumes the resolved path; no caller-created locator.
    edit=dict(args,candidate_id=t['candidate_id'])
    return t,args,edit

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
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'hypothesis-test-native','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    tools=rpc('tools/list',{})['tools'];verify('51 MCP tools',len(tools)==51)
    verify('MCP exposes hypothesis choice','hypothesis_ref' in next(t for t in tools if t['name']=='reversebolt_prepare_runtime_test')['inputSchema']['properties'])
    call('set_observer_model',{'model':None});attach()
    s=status();rt=s['state']['live']['runtime'];count=ops()
    index=call('read_runtime_inventory',{'catalog_id':rt['catalog_id']})['data']
    # Agent-style evaluator uses observed labels, never fixture offsets or source layouts.
    type_id=next(t['id'] for t in index['types'] if any(f['name']=='WalkSpeed' for f in t['fields']))
    evidence=call('read_runtime_inventory',{'catalog_id':rt['catalog_id'],'selected_types':[type_id]})['data']
    rows=[r for r in evidence['selection']['rows'] if r['field_name']=='WalkSpeed']
    primary=rows[0];alternatives=rows[1:3]
    proposal={'candidate':primary['id'],'name':'Walk speed','explanation':'The observed field label suggests walking speed; its effect is untested.',
        'question':'Does changing WalkSpeed suppress the response to the planned inputs?',
        'citations':[{'source':'field_name','index':None,'quote':primary['field_name']}],
        'alternatives':[r['id'] for r in alternatives]}
    submission={'review_id':str(uuid.uuid4()),'catalog_id':rt['catalog_id'],'after_revision':rt['revision'],
        'after_review':None,'report_sha256':evidence['report_sha256'],'selected_types':[type_id],
        'hypotheses':[proposal,dict(proposal,name='Health')]}
    call('submit_runtime_hypotheses',dict(submission,report_sha256='0'*64),ok=False)
    first=call('submit_runtime_hypotheses',submission)['data']['review']
    verify('agent proposals share the citation validator',len(first['batch']['accepted'])==1 and len(first['batch']['rejected'])==1)
    call('submit_runtime_hypotheses',dict(submission,hypotheses=[]),ok=False)
    later=dict(submission,review_id=str(uuid.uuid4()),after_review=first['evidence_id'])
    review=call('submit_runtime_hypotheses',later)['data']['review']
    replay=call('submit_runtime_hypotheses',submission)['data']
    s=status()
    verify('older submission replay does not replace newer review',replay['replayed'] and not replay['published'] and s['state']['live']['runtime']['naming']['evidence_id']==review['evidence_id'])
    verify('inventory and proposals access no target helper',ops()==count)
    verify('proposals do not rename or mutate cards',s['state']['live']['runtime']['cards']==rt['cards'])
    save('discovered',s);save('submission',later)
    h=review['batch']['accepted'][0]
    cards={c['candidate']['id'] for c in s['state']['live']['runtime']['cards']}
    rows=[h['candidate']]+h['alternatives']
    selected=next((r for r in rows if r['path_id'] not in cards),h['candidate'])
    offboard=selected['path_id'] not in cards
    reference={'review_id':review['evidence_id'],'hypothesis_path_id':h['candidate']['path_id'],'selected_path_id':selected['path_id']}
    save('selection',{'reference':reference,'hypothesis':h,'offboard':offboard})
    t,args,edit=prepare_choice(reference,selected['path_id']);first_id=t['id'];count=ops()
    full=call('read_runtime_test',{'test_id':first_id})['data']['test']
    verify('original proposal and alternatives preserved',full['origin']['hypothesis']==h)
    call('set_runtime_value',edit_args(t,edit),ok=False);verify('no edit before baseline',ops()==count)
    t=condition(t);verify('baseline ready',t['phase']=='ready_to_apply')
    trial=call('set_runtime_value',edit_args(t,edit),label='write')['data'];verify('linked edit written',trial['phase']=='written')
    t=next(t for t in status()['state']['runtime_tests'] if t['id']==first_id);t=condition(t)
    verify('ready for recovery',t['phase']=='ready_to_restore')
    restored=call('restore_runtime_change',{'change_id':trial['id']},label='restored')['data'];verify('original value restored',restored['phase']=='restored')
    t=next(t for t in status()['state']['runtime_tests'] if t['id']==first_id);t=condition(t)
    verify('full response suppression observed',t['phase']=='complete' and t['assessment']['status']=='suppression_observed')
    complete=call('read_runtime_test',{'test_id':first_id})['data'];save('complete',complete)
    verify('comparison preserves question without semantic promotion',complete['test']['origin']['hypothesis']==h and not complete['test']['candidate']['edit_authority'])
    oracle=host_call('read',address=trial['patch']['field_address'],width=len(trial['patch']['expected']));save('original-oracle',oracle)
    verify('independent byte restoration',oracle['bytes']==trial['patch']['expected'])
    second=next((r for r in rows if r['path_id']!=selected['path_id']),None)
    if second:
        other_ref=dict(reference,selected_path_id=second['path_id'])
        alternate,other_args,_=prepare_choice(other_ref,second['path_id'])
        call('stop_runtime_test',{'test_id':alternate['id']})
        verify('alternative retained as a separate test',alternate['id']!=first_id and alternate['hypothesis_ref']['hypothesis_path_id']==reference['hypothesis_path_id'])
    engine.terminate();engine.wait(timeout=10);engine=launch()
    recovered=call('read_runtime_test',{'test_id':first_id})['data']['test']
    verify('restart preserves exact hypothesis origin',recovered['origin']==complete['test']['origin'])
    count=ops();verify('replay after restart returns historical comparison',call('prepare_runtime_test',args)['data']['phase']=='complete' and ops()==count)
    verify('archived submission replay after restart',call('submit_runtime_hypotheses',later)['data']['replayed'])
    verify('restart has no live target',status()['state']['live'] is None)
    save('result',{'passed':True,'assertions':assertions,'selected_offboard':offboard,'alternative_tested_preparation':second is not None,
        'input_actions':actions,'manually_reviewed_images':0,'helpers':ops(),'limitation':'Owned explicitly named fixture; evaluator chooses the field and input plan. No semantic or game generalization credit.'})
    print(json.dumps({'passed':True,'checks':len(assertions),'selected_offboard':offboard,'alternative_prepared':second is not None}),flush=True)
except BaseException as error:
    save('failure',{'error':repr(error),'assertions':assertions});raise
finally:
    if host is not None:
        (OUT/'stop').write_text('stop')
        try:host.wait(timeout=5)
        except subprocess.TimeoutExpired:host.terminate();host.wait(timeout=5)
    for p in [mcp,engine]:
        if p is not None and p.poll() is None:p.terminate();p.wait(timeout=10)
    save('children',[{'pid':p.pid,'exit_code':p.poll()} for p in children])
