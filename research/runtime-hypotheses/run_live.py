"""Owned native metadata-review integration through the actual MCP server, in Anode."""
import json,os,queue,subprocess,sys,threading,time,uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve();OUT.mkdir(exist_ok=False)
BIN=OUT.parent/'bin';DATA=OUT/'data';NO_WINDOW=0x08000000
children=[];host=engine=mcp=None;lines=queue.Queue();sequence=0;checks=[]
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def save(name,value):(OUT/(name+'.json')).write_text(json.dumps(value,indent=2),encoding='utf-8')
def check(name,value):
    assert value,name
    checks.append(name)
def rpc(method,params):
    global sequence
    sequence+=1;mcp.stdin.write(json.dumps({'jsonrpc':'2.0','id':sequence,'method':method,'params':params})+'\n');mcp.stdin.flush()
    end=time.monotonic()+120
    while time.monotonic()<end:
        r=json.loads(lines.get(timeout=max(.1,end-time.monotonic())))
        if r.get('id')==sequence:assert 'error' not in r,r;return r['result']
    raise AssertionError('RPC timeout')
def call(name,args=None):
    r=rpc('tools/call',{'name':'reversebolt_'+name,'arguments':args or {}})
    v=json.loads(next(c['text'] for c in r['content'] if c['type']=='text'));assert v['ok'],v.get('error');return v
def status():return call('status')
def envelope(args):
    h=status()['jobs']['history'];c=h['clients'].get('mcp-agent')
    return dict(args,request_id=c['high_water']+1 if c else 1,boot=h['boot'],binding=h['current'])
def launch():
    p=subprocess.Popen([str(BIN/'Reversebolt.exe'),'--headless','--data-dir',str(DATA)],creationflags=NO_WINDOW,
        stdout=(OUT/f'engine-{len(children)}.stdout').open('w'),stderr=(OUT/f'engine-{len(children)}.stderr').open('w'));children.append(p)
    end=time.monotonic()+15
    while time.monotonic()<end:
        assert p.poll() is None
        try:
            if read(DATA/'endpoint.json')['pid']==p.pid:return p
        except (FileNotFoundError,json.JSONDecodeError):pass
        time.sleep(.03)
    raise AssertionError('Engine startup failed')
def terminal(job):
    end=time.monotonic()+180
    while time.monotonic()<end:
        s=status();j=s['jobs']['history']['jobs'][str(job)]
        if j['phase'].lower() in ('completed','failed','cancelled','interrupted'):return s,j
        time.sleep(.05)
    raise AssertionError('Job did not settle')
def pending(job,previous):
    end=time.monotonic()+30
    while time.monotonic()<end:
        s=status();n=s['state']['live']['runtime'].get('naming')
        if n and n['evidence_id']!=previous and n['status']=='pending':return s,n
        assert s['jobs']['history']['jobs'][str(job)]['phase'].lower() not in ('completed','failed','cancelled','interrupted'),'Review settled before pending-state test'
        time.sleep(.01)
    raise AssertionError('No pending review')
def new_job(model):
    args=envelope({'task':{'operation':'discover'},'local_model':model})
    r=call('start_runtime_job',args)['data'];assert r['ack']['rejection'] is None
    return r['ack']['job'],args
try:
    host=subprocess.Popen([sys.executable,str(ROOT/'research/runtime-tests-live/host.py'),str(OUT),str(BIN/'reversebolt_path_fixture.dll')],creationflags=NO_WINDOW,
        stdout=(OUT/'host.stdout').open('w'),stderr=(OUT/'host.stderr').open('w'));children.append(host)
    end=time.monotonic()+15
    while not (OUT/'ready.json').exists():assert host.poll() is None and time.monotonic()<end;time.sleep(.03)
    info=read(OUT/'ready.json');check('owned fixture runs in Anode',info['session']!=1)
    engine=launch();mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,
        stderr=(OUT/'mcp.stderr').open('w'),text=True,encoding='utf-8',creationflags=NO_WINDOW);children.append(mcp)
    threading.Thread(target=lambda:[lines.put(s) for s in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'runtime-hypotheses-native','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    tools=rpc('tools/list',{})['tools'];tool=next(t for t in tools if t['name']=='reversebolt_start_runtime_job')
    check('model option exposed through MCP','local_model' in tool['inputSchema']['properties'])
    call('set_observer_model',{'model':'qwen3.8:latest'})
    p=next(p for p in call('list_processes')['state']['processes'] if p['pid']==host.pid and p['window']==info['window'])
    attached=call('attach_process',{'pid':p['pid'],'created':p['created'],'window':p['window'],'budget_mib':1})
    save('attached',attached)
    assert 'automatic_observation' in attached['data'],attached['data']
    job=attached['data']['automatic_observation']['ack']['job'];initial,n=pending(job,None);initial_cards=initial['state']['live']['runtime']['cards']
    check('cards appear before model finishes',len(initial_cards)>0)
    s,j=terminal(job);save('automatic',s);n=s['state']['live']['runtime']['naming']
    check('automatic review completed',j['phase']=='Completed' and n['status']=='complete' and n['batch'] is not None)
    check('model does not rename or edit cards',s['state']['live']['runtime']['cards']==initial_cards)
    receipt=read(DATA/'runtime/naming'/n['evidence_id']/'receipt.json')
    check('full receipt retained before publication',receipt['batch']==n['batch'] and receipt['model_digest'])
    check('hypotheses bound to discovery',n['report_sha256']==s['state']['live']['runtime']['catalog_source']['document_sha256'])
    job,args=new_job('reversebolt-nonexistent-local-model');s,j=terminal(job);save('unavailable',s)
    n=s['state']['live']['runtime']['naming'];check('missing model preserves usable discovery',j['phase']=='Completed' and n['status']=='unavailable' and n['batch'] is None and s['state']['live']['runtime']['cards'])
    count=len(list((DATA/'runtime/naming').iterdir()));replay=call('start_runtime_job',args)['data']
    check('job replay does not repeat inference',replay['duplicate'] and len(list((DATA/'runtime/naming').iterdir()))==count)
    job,args=new_job('qwen3.8:latest');_,pending_review=pending(job,n['evidence_id'])
    call('job_control',envelope({'operation':'cancel','job':job}));s,j=terminal(job);save('cancelled',s)
    n=s['state']['live']['runtime']['naming'];check('pending cancellation prevents publication',j['phase']=='Cancelled' and n['status']=='cancelled' and n['batch'] is None)
    job,args=new_job('qwen3.8:latest');_,pending_review=pending(job,n['evidence_id'])
    fault=(DATA/'runtime/naming'/str(uuid.UUID(pending_review['evidence_id']))/'receipt.json').resolve();assert fault.is_relative_to(OUT);fault.mkdir()
    s,j=terminal(job);save('storage-failure',s);n=s['state']['live']['runtime']['naming']
    check('archive failure fails job without publishing guesses',j['phase']=='Failed' and n['status']=='failed' and n['batch'] is None)
    fault.rmdir()
    check('no writes, response tests or captures',not s['state']['runtime_trials'] and not s['state']['runtime_tests'] and not list(DATA.rglob('*.png')))
    history=s['state']['history'];engine.terminate();engine.wait(timeout=10);engine=launch();s=status();save('restarted',s)
    check('restart keeps history without resuming review',s['state']['history']==history and s['state']['live'] is None)
    check('model setting survives restart',s['state']['observer_model']=='qwen3.8:latest')
    save('result',{'passed':True,'checks':checks,'model_receipt':receipt,'images':0,'input_actions':0,'gameplay_writes':0,'limitation':'Owned native fixture and local-model integration; no independent game semantics.'})
    print(json.dumps({'passed':True,'checks':len(checks),'hypotheses':len(receipt['batch']['accepted'])}),flush=True)
except BaseException as error:
    save('failure',{'error':repr(error),'checks':checks});raise
finally:
    if host is not None:
        (OUT/'stop').write_text('stop')
        try:host.wait(timeout=5)
        except subprocess.TimeoutExpired:host.terminate();host.wait(timeout=5)
    for p in [mcp,engine]:
        if p is not None and p.poll() is None:p.terminate();p.wait(timeout=10)
    save('children',[{'pid':p.pid,'exit_code':p.poll()} for p in children])
