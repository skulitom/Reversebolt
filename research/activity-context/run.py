"""Actual-game observation and provenance check; no gameplay writes."""
import ctypes as C,json,math,os,queue,subprocess,sys,threading,time,uuid
from ctypes import wintypes as W
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve();PID=int(sys.argv[2]);BIN=OUT/'bin';DATA=OUT/'data'
engine=mcp=None;children=[];lines=queue.Queue();seq=0;actions=[];checks=[];active_test=None
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
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'activity-context-evaluator','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    target=next(p for p in call('list_processes')['state']['processes'] if p['pid']==PID);save('target',target);foreground()
    call('set_observer_model',{'model':'qwen3.8:latest'});call('attach_process',dict(pid=target['pid'],created=target['created'],window=target['window'],budget_mib=1,auto_observe=True))
    handled=set();end=time.monotonic()+420
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
    context=review.get('play_context');prepared=None
    if context:
        check('context matches archived image interpretation',context=={'source':receipt['activity']['source'],'observation':receipt['activity']['observation']})
        check('selected control was observed',context['source']['key'] in (65,68))
        check('context conditions both metadata stages',all(context['observation']['description'] in stage['prompt'] for stage in receipt['stages']))
        index=call('read_runtime_inventory',{'catalog_id':rt['catalog_id']})['data'];types={t['id']:t for t in index['types']}
        for h in review['batch']['accepted']:
            row=h['candidate'];field=next(f for f in types[row['type_id']]['fields'] if f['id']==row['field_id'])
            try:value=float(row['value'])
            except (ValueError,TypeError):continue
            if field['type_code'] not in (12,13) or not math.isfinite(value) or not 0<value<=20:continue
            active_test=str(uuid.uuid4());plan={'test_id':active_test,'change_id':str(uuid.uuid4()),'investigation_id':live['investigation_id'],'catalog_id':rt['catalog_id'],'after_revision':rt['revision'],
                'hypothesis_ref':{'review_id':review['evidence_id'],'hypothesis_path_id':row['path_id'],'selected_path_id':row['path_id']},'value':'0','actions':[{'keys':[context['source']['key']],'hold_ms':200}]*4}
            save('proposed-plan',plan);call('prepare_runtime_test',plan);prepared=call('read_runtime_test',{'test_id':active_test})['data']['test']
            check('prepared test retains exact activity context',prepared['origin']['play_context']==context)
            call('stop_runtime_test',{'test_id':active_test});engine.terminate();engine.wait(timeout=10);engine=launch()
            restored=call('read_runtime_test',{'test_id':active_test})['data']['test'];check('restart retains context without resuming',restored['phase']=='stopped' and restored['origin']==prepared['origin'] and status()['state']['live'] is None)
            break
    final=status();check('no gameplay edits',not final['state']['runtime_trials'] and not final['state']['changes'] and not final['state']['freezes']);save('final-state',final)
    summary={'completed':True,'checks':checks,'inputs':actions,'activity_accepted':bool(context),'caption':receipt.get('activity'),'hypotheses':review.get('batch'),'prepared_test':prepared,'manually_reviewed_measurement_images':0}
    save('result',summary);print(json.dumps({'completed':True,'activity_accepted':bool(context),'prepared_test':prepared is not None,'checks':len(checks),'hypotheses':len((review.get('batch') or {}).get('accepted',[]))}),flush=True)
except BaseException as error:save('failure',{'error':repr(error),'checks':checks,'inputs':actions});raise
finally:
    if mcp is not None and mcp.poll() is None:
        try:
            s=status()
            if active_test and any(t['id']==active_test and t['active'] for t in s['state']['runtime_tests']):call('stop_runtime_test',{'test_id':active_test})
            if not (OUT/'final-state.json').exists():save('final-state',status())
        except BaseException as error:save('cleanup-error',{'error':repr(error)})
    for p in [mcp,engine]:
        if p is not None and p.poll() is None:p.terminate();p.wait(timeout=10)
    save('children',[{'pid':p.pid,'exit_code':p.poll()} for p in children])
