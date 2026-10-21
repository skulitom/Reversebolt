"""Frozen actual-game planner evaluation; bounded comparisons with recovery."""
import ctypes as C,json,math,os,queue,subprocess,sys,threading,time,uuid
from ctypes import wintypes as W
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve();PID=int(sys.argv[2]);BIN=OUT/'bin';DATA=OUT/'data'
engine=mcp=None;children=[];lines=queue.Queue();seq=0;actions=[];checks=[];active_test=None;comparisons=[]
user=C.WinDLL('user32');user.GetForegroundWindow.restype=W.HWND;user.GetWindowThreadProcessId.argtypes=[W.HWND,C.POINTER(W.DWORD)];user.MapVirtualKeyW.restype=W.UINT
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def save(name,value):
    with (OUT/(name+'.json')).open('x',encoding='utf-8') as f:json.dump(value,f,indent=2)
def check(name,ok):assert ok,name;checks.append(name)
def rpc(method,params):
    global seq
    seq+=1;mcp.stdin.write(json.dumps(dict(jsonrpc='2.0',id=seq,method=method,params=params))+'\n');mcp.stdin.flush();end=time.monotonic()+180
    while time.monotonic()<end:
        r=json.loads(lines.get(timeout=max(.1,end-time.monotonic())))
        if r.get('id')==seq:assert 'error' not in r,r;return r['result']
    raise AssertionError('Unknown RPC outcome; no replay')
def call(name,args=None):
    r=rpc('tools/call',{'name':'reversebolt_'+name,'arguments':args or {}})
    v=json.loads(next(c['text'] for c in r['content'] if c['type']=='text'))
    if name!='status':save(str(time.time_ns())+'-'+name,{'args':args,'response':v})
    assert v['ok'],(name,v.get('error'));return v
def status():return call('status')
def foreground():
    hwnd=user.GetForegroundWindow();pid=W.DWORD();user.GetWindowThreadProcessId(hwnd,C.byref(pid));assert pid.value==PID and hwnd==target['window'],'Game focus changed; no input sent'
def key(k):
    foreground();scan=user.MapVirtualKeyW(k,0);assert scan;begin=time.time_ns();user.keybd_event(0,scan,8,0)
    try:time.sleep(.2)
    finally:user.keybd_event(0,scan,10,0)
    record={'key':k,'begin_ns':begin,'end_ns':time.time_ns(),'planned_ms':200};actions.append(record);save('input-'+str(len(actions)),record)
def launch():
    p=subprocess.Popen([str(BIN/'Reversebolt.exe'),'--headless','--data-dir',str(DATA)],creationflags=0x08000000,stdout=(OUT/('engine-'+str(len(children))+'.stdout')).open('w'),stderr=(OUT/('engine-'+str(len(children))+'.stderr')).open('w'));children.append(p);end=time.monotonic()+15
    while time.monotonic()<end:
        assert p.poll() is None
        try:
            if read(DATA/'endpoint.json')['pid']==p.pid:return p
        except (FileNotFoundError,json.JSONDecodeError):pass
        time.sleep(.05)
    raise AssertionError('Engine startup did not settle')
try:
    session=W.DWORD();assert C.WinDLL('kernel32').ProcessIdToSessionId(os.getpid(),C.byref(session)) and session.value==3
    assert not DATA.exists(),'Never replay a started experiment'
    engine=launch();mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],creationflags=0x08000000,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(OUT/'mcp.stderr').open('w'),text=True,encoding='utf-8');children.append(mcp)
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'comparison-planner-evaluator','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    target=next(p for p in call('list_processes')['state']['processes'] if p['pid']==PID);save('target',target);foreground()
    call('set_observer_model',{'model':'qwen3.8:latest'});call('attach_process',dict(pid=target['pid'],created=target['created'],window=target['window'],budget_mib=1,auto_observe=True))
    handled=set();end=time.monotonic()+1000
    while time.monotonic()<end:
        s=status();number=s['state']['live']['sample_count']
        if number not in handled:
            handled.add(number)
            if number in (2,4,8,10):key(68 if number in (2,8) else 65)
        jobs=s['jobs']['history']['jobs']
        if jobs and all(j['phase'] in ('Completed','Failed','Cancelled','Interrupted') for j in jobs.values()):
            assert all(j['phase']=='Completed' for j in jobs.values()),[(j['phase'],j.get('error')) for j in jobs.values()];break
        time.sleep(.02 if number<15 else .4)
    else:raise AssertionError('Job still pending; retain original handle')
    save('discovery',s);check('four planned input holds',len(actions)==4);live=s['state']['live'];rt=live['runtime'];review=rt['naming'];check('fifteen automatic captures',live['sample_count']==15)
    receipt=read(DATA/'runtime/naming'/review['evidence_id']/'receipt.json');source=read(DATA/'runtime/naming'/review['evidence_id']/'play-source.json')
    save('play-evidence',call('read_play_evidence',{'investigation_id':live['investigation_id'],'batch':live['coverage']['batch'],'frame_ids':[f['id'] for f in live['frames']]}))
    context=review.get('play_context');plans=[]
    if context:
        check('context matches archived interpretation',context=={'source':receipt['activity']['source'],'observation':receipt['activity']['observation']})
        check('context conditions every metadata stage',all(context['observation']['description'] in stage['prompt'] for stage in receipt['stages']))
        check('per-type question budgets',all(stage['schema']['properties']['hypotheses']['maxItems']<=2 for stage in receipt['stages'][1:]))
        def planning():return call('plan_runtime_comparisons',{'catalog_id':rt['catalog_id'],'review_id':review['evidence_id']})['data']
        def observe(t):
            for _ in range(6):
                t=call('observe_runtime_test',{'test_id':t['id'],'after_revision':t['revision'],'perform_action':True})['data']
                if t['phase']=='stopped':break
            return t
        for number in range(2):
            proposal=planning();plans.append(proposal);choice=proposal['next']
            if choice is None:break
            check('planned keys match observed sequence',[a['keys'][0] for a in proposal['observed_actions']['actions']]==[a['key'] for a in actions])
            check('planned durations remain within sampled bounds',all(e['hold_bounds_us'][0]<=e['action']['hold_ms']*1000<=e['hold_bounds_us'][1] for e in proposal['observed_actions']['episodes']))
            active_test=str(uuid.uuid4());change=str(uuid.uuid4())
            args={'test_id':active_test,'change_id':change,'investigation_id':proposal['investigation_id'],'catalog_id':proposal['catalog_id'],'after_revision':proposal['after_revision'],
                'hypothesis_ref':choice['hypothesis_ref'],'value':choice['value'],'actions':choice['actions']}
            save('plan-'+str(number),{'proposal':proposal,'prepared_args':args})
            t=call('prepare_runtime_test',args)['data'];t=observe(t)
            if t['phase']=='ready_to_apply':
                live_now=status()['state']['live'];runtime=live_now['runtime']
                edit={'change_id':change,'investigation_id':live_now['investigation_id'],'catalog_id':runtime['catalog_id'],'after_revision':runtime['revision'],'candidate_id':t['candidate_id'],'value':choice['value']}
                trial=call('set_runtime_value',edit)['data'];check('planned edit settled',trial['phase']=='written')
                t=next(t for t in status()['state']['runtime_tests'] if t['id']==active_test);t=observe(t)
                restored=call('restore_runtime_change',{'change_id':change})['data'];check('original value recovered',restored['phase']=='restored')
                t=next(t for t in status()['state']['runtime_tests'] if t['id']==active_test)
                if t['phase']=='restored':t=observe(t)
            if t['active']:call('stop_runtime_test',{'test_id':active_test})
            saved=call('read_runtime_test',{'test_id':active_test})['data'];comparisons.append(saved)
            save('comparison-'+str(number),saved)
            next_plan=planning();save('after-comparison-'+str(number),next_plan)
            retained=next(c for c in next_plan['choices'] if c['hypothesis_ref']==choice['hypothesis_ref'])
            expected=saved['test']['assessment']['status'] if saved['test']['assessment'] else 'stopped'
            check('outcome retained without semantic promotion',retained['status']==expected and not saved['test']['candidate']['edit_authority'])
            check('completed choice is not repeated',next_plan['next'] is None or next_plan['next']['hypothesis_ref']!=choice['hypothesis_ref'])
        live_final=status();save('before-restart',live_final)
        check('no unresolved gameplay edits',all(t['phase'] in ('restored','refused','expired') for t in live_final['state']['runtime_trials']))
        engine.terminate();engine.wait(timeout=10);engine=launch()
        for old in comparisons:
            after=call('read_runtime_test',{'test_id':old['test']['id']})['data']
            check('restart retains comparison evidence',after==old)
        check('restart has no live target',status()['state']['live'] is None)
    final=status();save('final-state',final)
    summary={'completed':True,'checks':checks,'observation_inputs':actions,'activity_accepted':bool(context),'caption':receipt.get('activity'),'hypotheses':review.get('batch'),
        'plans':plans,'comparisons':[{'id':c['test']['id'],'field':c['test']['candidate']['name'],'phase':c['test']['phase'],'assessment':c['test']['assessment']} for c in comparisons],
        'manually_reviewed_measurement_images':0}
    save('result',summary);print(json.dumps({'completed':True,'checks':len(checks),'comparisons':summary['comparisons'],'activity_accepted':bool(context)}),flush=True)
except BaseException as error:save('failure',{'error':repr(error),'checks':checks,'inputs':actions});raise
finally:
    cleanup_ready=True
    if mcp is not None and mcp.poll() is None:
        try:
            s=status()
            if active_test and any(t['id']==active_test and t['active'] for t in s['state']['runtime_tests']):call('stop_runtime_test',{'test_id':active_test})
            for trial in status()['state']['runtime_trials']:
                if trial['phase']=='written':call('restore_runtime_change',{'change_id':trial['id']})
            settled=status()
            cleanup_ready=all(t['phase'] not in ('written','uncertain') for t in settled['state']['runtime_trials'])
            if not (OUT/'final-state.json').exists():save('final-state',settled)
        except BaseException as error:
            cleanup_ready=False;save('cleanup-error',{'error':repr(error)})
    if not cleanup_ready:save('needs-recovery', {'engine_pid':engine.pid if engine else None,'data':str(DATA)})
    for p in [mcp,engine if cleanup_ready else None]:
        if p is not None and p.poll() is None:p.terminate();p.wait(timeout=10)
    save('children',[{'pid':p.pid,'exit_code':p.poll()} for p in children])
