"""Automatic fresh type selection and bounded read-only input association."""
import ctypes as C
from ctypes import wintypes as W
import hashlib,json,os,subprocess,threading,time,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve();BIN=OUT/'bin'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json');target=read(OUT/'target.json');PID=target['pid']
scope={k:target[k] for k in ['pid','created','session']}
assert target['window']>0,'Capture the target window inside its own desktop session'
assert sha(Path(__file__))==frozen['sources']['research/runtime-embedded-activity/live.py']
for name,digest in frozen['binaries'].items():assert sha(BIN/name)==digest
assert not (OUT/'started.json').exists(),'Do not replay an uncertain run'
user,kernel=C.WinDLL('user32'),C.WinDLL('kernel32')
user.GetForegroundWindow.restype=W.HWND;user.GetWindowThreadProcessId.argtypes=[W.HWND,C.POINTER(W.DWORD)]
user.GetAsyncKeyState.restype=C.c_short;user.MapVirtualKeyW.restype=W.UINT
frequency=C.c_int64();assert kernel.QueryPerformanceFrequency(C.byref(frequency))
session=W.DWORD();assert kernel.ProcessIdToSessionId(os.getpid(),C.byref(session)) and session.value==scope['session']==3
calls=[];pauses=[];holds=[]
def save(name,value):
    with (OUT/(name+'.json')).open('x',encoding='utf-8') as f:json.dump(value,f,indent=2)
def ticks():
    value=C.c_int64();assert kernel.QueryPerformanceCounter(C.byref(value));return value.value
def poll():
    begin=ticks();window=user.GetForegroundWindow();owner=W.DWORD();user.GetWindowThreadProcessId(window,C.byref(owner))
    down=[k for k in range(1,256) if user.GetAsyncKeyState(k)&0x8000]
    return {'begin_ticks':begin,'end_ticks':ticks(),'selected_foreground':owner.value==PID and window==target['window'],'down':down}
def command(label,args,timeout=130):
    with (OUT/(label+'.stdout')).open('x') as stdout,(OUT/(label+'.stderr')).open('x') as stderr:
        child=subprocess.Popen(list(map(str,args)),stdout=stdout,stderr=stderr,creationflags=subprocess.CREATE_NO_WINDOW)
        save(label+'-process',{'pid':child.pid,'args':list(map(str,args))})
        child.wait(timeout=timeout)
    return child.returncode
def native(label,paths=None):
    extra=[]
    if paths is not None:
        save(label+'-paths',paths);extra=['--paths-file',OUT/(label+'-paths.json')]
    args=[BIN/'reversebolt-runtime-loader.exe',PID,scope['created'],scope['session'],BIN/'reversebolt_runtime.dll',frozen['binaries']['reversebolt_runtime.dll'],OUT/(label+'.json')]+extra
    code=command(label,args)
    events=[json.loads(line) for line in (OUT/(label+'.stdout')).read_text().splitlines()]
    assert events[-1]['helper_absent'] and events[-1]['inspection_exit']==0 and code==0,(label,code)
    report=read(OUT/(label+'.json'))
    samples=report.get('pauses',report.get('snapshots',[])+[w['discarded_snapshot'] for w in report.get('warmups',[])])
    assert all(s['helper_allocations_while_paused']==s['helper_deallocations_while_paused']==0 for s in samples)
    pauses.extend(samples);calls.append({'label':label,'report_sha256':sha(OUT/(label+'.json')),'helper_absent':True})
    return report
def measure(batch,index,paths):
    current=poll();assert current['selected_foreground'] and not current['down'],'Focus/input conflict before sample'
    key={2:68,5:65,8:68,11:65}.get(index);polls=[];errors=[];stop=threading.Event();hold=None;input_thread=None
    def polling():
        try:
            while not stop.is_set():polls.append(poll());stop.wait(.005)
        except BaseException as e:errors.append(repr(e))
    def holding():
        scan=user.MapVirtualKeyW(key,0);hold['begin_ticks']=ticks();end=time.monotonic()+frozen['hold_ms']/1000
        user.keybd_event(0,scan,8,0)
        try:
            while time.monotonic()<end:
                current=poll()
                if not current['selected_foreground'] or any(k!=key for k in current['down']):hold['interrupted']=True;break
                time.sleep(min(.005,max(0,end-time.monotonic())))
        finally:user.keybd_event(0,scan,10,0);hold['end_ticks']=ticks()
    poll_thread=threading.Thread(target=polling,daemon=True);poll_thread.start()
    try:
        if key:
            hold={'key':key,'planned_ms':frozen['hold_ms']};input_thread=threading.Thread(target=holding);input_thread.start();time.sleep(.1)
        else:time.sleep(.25)
        report=native(f'sample-{batch}-{index}',paths);time.sleep(.075)
    finally:
        if input_thread:input_thread.join(timeout=6)
        stop.set();poll_thread.join(timeout=2)
        if hold:holds.append(hold)
        evidence={'scope':scope,'frequency_hz':frequency.value,'expected_keys':[key] if key else [],'polls':polls}
        save(f'input-{batch}-{index}',{'input':evidence,'hold':hold,'errors':errors})
    assert not errors and all(p['selected_foreground'] for p in polls)
    assert not any(any(k!=key for k in p['down']) for p in polls)
    assert hold is None or not hold.get('interrupted')
    return {'report':report,'input':evidence}
save('started',{'runner_pid':os.getpid(),'target':target,'unix_ms':time.time_ns()//1000000})
try:
    report=native('discovery');print(json.dumps({'stage':'discovery','next_root':report['next_root'],'catalog_roots':report['catalog_roots'],'complete':report['root_scan_complete']}),flush=True)
    code=command('selection',[BIN/'reversebolt-runtime-hypotheses.exe',OUT/'discovery.json',OUT/'selection',frozen['model'],'--select-only'],timeout=1000)
    assert code==0,'Type selection process failed'
    receipt=read(OUT/'selection/receipt.json')
    assert receipt['model_digest']==frozen['model_digest'] and receipt['server_version']==frozen['server_version']
    assert not receipt['error'] and not receipt['cancelled'] and receipt['batch'] is None and receipt['selection']['rows'],receipt.get('error')
    print(json.dumps({'stage':'selected','model_calls':len(receipt['stages']),'types':receipt['selection']['selected_types']}),flush=True)
    code=command('planning',[BIN/'reversebolt-runtime-neighbors.exe',OUT/'discovery.json',OUT/'selection/receipt.json',OUT/'plan','--selected-types'])
    assert code==0,'Sibling planning failed'
    plan=read(OUT/'plan/plan.json');assert plan['scope']==scope
    save('frozen-plan',{'sha256':sha(OUT/'plan/plan.json'),'batches':len(plan['batches']),'selected':len(plan['candidates']),'omitted':plan['omitted_fields']})
    print(json.dumps({'stage':'planned','fields':len(plan['candidates']),'batches':len(plan['batches']),'roles':len(plan['roles'])}),flush=True)
    results=[]
    for batch,paths in enumerate(plan['batches'][:frozen['max_batches']]):
        observations=[]
        for index in range(14):
            observations.append(measure(batch,index,paths))
            print(json.dumps({'stage':'measured','batch':batch,'sample':index+1,'fields':len(paths)}),flush=True)
        save(f'observations-{batch}',observations)
        code=command(f'analysis-{batch}',[BIN/'reversebolt-runtime-response.exe',OUT/'plan/plan.json',batch,OUT/f'observations-{batch}.json',OUT/f'analysis-{batch}.json'])
        results.append({'batch':batch,'exit':code,'analysis':read(OUT/f'analysis-{batch}.json') if code==0 else None})
    save('result',{'finished':True,'batches':results,'omitted_batches':max(0,len(plan['batches'])-frozen['max_batches']),
        'calls':calls,'pauses':len(pauses),'max_stop_to_resume_us':max(s['stop_to_resume_us'] for s in pauses),'holds':holds,
        'model_calls':len(receipt['stages']),'gameplay_writes':0,'measurement_images':0})
    print(json.dumps({'finished':True,'valid_batches':sum(b['exit']==0 for b in results),'associations':sum(sum(f['repeated_association'] for f in b['analysis']['fields']) for b in results if b['exit']==0)}),flush=True)
except BaseException as error:
    save('failure',{'error':repr(error),'calls':calls,'holds':holds});raise
