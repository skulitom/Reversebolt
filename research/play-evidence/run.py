"""Owned native fixture evaluation through the actual stdio MCP, only in Anode."""
import ctypes as C,json,os,queue,subprocess,sys,threading,time
from ctypes import wintypes as W
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve();OUT.mkdir(exist_ok=False)
BIN=OUT.parent/'bin';DATA=OUT/'data';engine=mcp=host=None;children=[];lines=queue.Queue();seq=0;checks=[];actions=[]
user=C.WinDLL('user32');user.GetForegroundWindow.restype=W.HWND
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def save(name,value):(OUT/(name+'.json')).write_text(json.dumps(value,indent=2),encoding='utf-8')
def check(name,ok):assert ok,name;checks.append(name)
def rpc(method,params):
    global seq
    seq+=1;mcp.stdin.write(json.dumps(dict(jsonrpc='2.0',id=seq,method=method,params=params))+'\n');mcp.stdin.flush();end=time.monotonic()+120
    while time.monotonic()<end:
        r=json.loads(lines.get(timeout=max(.1,end-time.monotonic())))
        if r.get('id')==seq:assert 'error' not in r,r;return r['result']
    raise AssertionError('Unknown RPC outcome; no automatic replay')
def call(name,args=None,ok=True,label=None):
    r=rpc('tools/call',{'name':'reversebolt_'+name,'arguments':args or {}})
    v=json.loads(next(c['text'] for c in r['content'] if c['type']=='text'));assert v['ok']==ok,(name,v.get('error'))
    if label:save(label,{'args':args,'response':v})
    return v
def status():return call('status')
def envelope():
    h=status()['jobs']['history'];return dict(boot=h['boot'],binding=h['current'],request_id=h['clients'].get('mcp-agent',{}).get('high_water',0)+1)
def settle():
    end=time.monotonic()+100
    while time.monotonic()<end:
        s=status();jobs=s['jobs']['history']['jobs']
        if jobs and all(j['phase'] in ('Completed','Failed','Cancelled','Interrupted') for j in jobs.values()):
            assert all(j['phase']=='Completed' for j in jobs.values()),jobs;return s
        time.sleep(.05)
    raise AssertionError('Job pending; retain its identity')
def ops():return len(list((DATA/'runtime').glob('*-*/**/stdout.jsonl')))
def key(keys):
    assert user.GetForegroundWindow()==ready['window'],'Fixture lost focus; no input sent'
    before=time.time_ns()
    for k in keys:user.keybd_event(k,0,0,0)
    try:time.sleep(.2)
    finally:
        for k in reversed(keys):user.keybd_event(k,0,2,0)
    actions.append({'keys':keys,'begin_ns':before,'end_ns':time.time_ns()})
def attach(auto):
    p=next(p for p in call('list_processes')['state']['processes'] if p['pid']==host.pid and p['window']==ready['window'])
    call('attach_process',dict(pid=p['pid'],created=p['created'],window=p['window'],budget_mib=1,auto_observe=auto))
def play_metadata(live):
    found=list(Path(live['latest']['screen']['path']).parent.glob('frame-*.json'))
    records=[read(p) for p in found]
    records=[r for r in records if r['mode']=='play']
    check('play metadata exists',len(records)>=15)
    check('play captures do not read memory or recognize text',all(r['captured_bytes']==0 and not r['memory_reads'] and not r['prediction_reads'] and r['recognition_ms']==0 and not r['screen']['words'] and not r['screen']['text'] for r in records))
    check('play retains no memory',live['retained_bytes']==0 and live['coverage']['attempted_bytes']==0)
try:
    session=W.DWORD();assert C.WinDLL('kernel32').ProcessIdToSessionId(os.getpid(),C.byref(session)) and session.value==3
    host=subprocess.Popen([sys.executable,str(ROOT/'research/runtime-tests-live/host.py'),str(OUT),str(BIN/'reversebolt_path_fixture.dll')],creationflags=0x08000000);children.append(host)
    end=time.monotonic()+15
    while not (OUT/'ready.json').exists():assert host.poll() is None and time.monotonic()<end;time.sleep(.05)
    ready=read(OUT/'ready.json');print(json.dumps({'focus_fixture':ready}),flush=True)
    end=time.monotonic()+120
    while not (OUT/'fixture-focused').exists():assert time.monotonic()<end;time.sleep(.05)
    assert user.GetForegroundWindow()==ready['window']
    engine=subprocess.Popen([str(BIN/'Reversebolt.exe'),'--headless','--data-dir',str(DATA)],creationflags=0x08000000,stdout=(OUT/'engine.stdout').open('w'),stderr=(OUT/'engine.stderr').open('w'));children.append(engine)
    end=time.monotonic()+15
    while not (DATA/'endpoint.json').exists():assert engine.poll() is None and time.monotonic()<end;time.sleep(.05)
    mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],creationflags=0x08000000,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(OUT/'mcp.stderr').open('w'),text=True,encoding='utf-8');children.append(mcp)
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'play-evidence-native','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    check('52 MCP tools',len(rpc('tools/list',{})['tools'])==52)
    call('set_observer_model',{'model':None});attach(True);s=settle();save('automatic',s)
    live=s['state']['live'];check('automatic attach captures fifteen play moments',live['sample_count']==15);play_metadata(live)
    check('one automatic runtime discovery',ops()==1 and live['runtime']['cards'])
    # New connection leaves prior captures archived; the next sequence has its own scope.
    attach(False);call('start_runtime_job',dict(envelope(),task={'operation':'discover'}));s=settle();rt=s['state']['live']['runtime']
    card=next(c for c in rt['cards'] if c['candidate']['name']=='WalkSpeed');identifier=card['candidate']['id']
    call('start_runtime_job',dict(envelope(),task={'operation':'refresh','catalog_id':rt['catalog_id'],'after_revision':rt['revision'],'ids':[identifier]}));s=settle()
    card=next(c for c in s['state']['live']['runtime']['cards'] if c['candidate']['id']==identifier)
    address=read(Path(card['readings'][-1]['evidence_path']))['checks'][0]['field_address']
    (OUT/'request.json').write_text(json.dumps({'id':1,'operation':'configure','address':address}));end=time.monotonic()+5
    while not (OUT/'response.json').exists():assert time.monotonic()<end;time.sleep(.02)
    assert read(OUT/'response.json')['id']==1
    count=ops();call('start_observation_job',dict(envelope(),count=15,interval_ms=500,mode='play'))
    handled=set();end=time.monotonic()+30
    while time.monotonic()<end:
        s=status();number=s['state']['live']['sample_count']
        if number not in handled:
            handled.add(number)
            if number in (2,4,8,9,12):key([68])
            elif number==6:key([65,68])
        if number==15:break
        time.sleep(.015)
    s=settle();live=s['state']['live'];save('observed',s);play_metadata(live)
    check('six bounded evaluator inputs',len(actions)==6)
    check('play and evidence reads launch no runtime helper',ops()==count)
    args={'investigation_id':live['investigation_id'],'batch':live['coverage']['batch'],'frame_ids':[f['id'] for f in live['frames']]}
    result=call('read_play_evidence',args,label='evidence')['data'];e=result['evidence']
    action=next(a for a in e['actions'] if a['key']==68)
    check('repeated key episodes observed',len(action['bounded_episodes'])>=2)
    check('rendered response observed',action['median_difference']>3 and action['quiet_median_difference']<1)
    check('chord remains unknown',any(i['exposure']['kind']=='unknown' and 'Several' in i['exposure']['value'] for i in e['intervals']))
    for index,bad in enumerate([dict(args,investigation_id='wrong'),dict(args,batch=args['batch']+1),dict(args,frame_ids=list(reversed(args['frame_ids']))),dict(args,frame_ids=[args['frame_ids'][0]]*2)]):call('read_play_evidence',bad,ok=False,label='refusal-'+str(index))
    check('stale and malformed requests rejected',True)
    first_image=Path(live['latest']['screen']['path']).parent/'frame-001.png';original=first_image.read_bytes()
    try:
        first_image.write_bytes(original[:-1]+bytes([original[-1]^1]))
        call('read_play_evidence',args,ok=False,label='changed-image-refusal')
    finally:first_image.write_bytes(original)
    check('altered retained image rejected',call('read_play_evidence',args)['data']==result)
    check('all evidence reads use retained data only',ops()==count)
    final=status();check('no changes or runtime trials',not final['state']['changes'] and not final['state']['runtime_trials'])
    save('final',final);save('result',{'passed':True,'checks':checks,'actions':actions,'helpers':ops(),'evidence':result,'manually_reviewed_images':0})
    print(json.dumps({'passed':True,'checks':len(checks),'actions':len(actions),'response':action}),flush=True)
except BaseException as error:save('failure',{'error':repr(error),'checks':checks,'actions':actions});raise
finally:
    if host:
        (OUT/'stop').write_text('stop')
        try:host.wait(timeout=5)
        except subprocess.TimeoutExpired:host.terminate();host.wait(timeout=5)
    for p in [mcp,engine]:
        if p is not None and p.poll() is None:p.terminate();p.wait(timeout=10)
    save('children',[{'pid':p.pid,'exit_code':p.poll()} for p in children])
