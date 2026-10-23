"""Fresh automatic discovery, then sequential read-only sibling measurements."""
import ctypes as C
from ctypes import wintypes as W
import hashlib
import json
import msvcrt
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-siblings-sable-v1'
PID = int(sys.argv[1])
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT/'frozen.json')
DATA, BIN = Path(frozen['data']), Path(frozen['bin'])
for name, digest in frozen['binaries'].items(): assert sha(BIN/name) == digest
for name, digest in frozen['analysis'].items(): assert sha(OUT/'analysis-bin'/name) == digest
assert sha(Path(__file__)) == frozen['sources']['research/runtime-siblings/live.py']
assert not (OUT/'started.json').exists(), 'Never replay a started native run'
user, kernel = C.WinDLL('user32'), C.WinDLL('kernel32')
user.GetForegroundWindow.restype = W.HWND
user.GetWindowThreadProcessId.argtypes = [W.HWND, C.POINTER(W.DWORD)]
user.GetAsyncKeyState.restype = C.c_short
user.MapVirtualKeyW.restype = W.UINT
session = W.DWORD()
assert kernel.ProcessIdToSessionId(os.getpid(),C.byref(session)) and session.value == 3
frequency = C.c_int64()
assert kernel.QueryPerformanceFrequency(C.byref(frequency))
def ticks():
    value = C.c_int64()
    assert kernel.QueryPerformanceCounter(C.byref(value))
    return value.value
def save(name,value):
    with (OUT/(name+'.json')).open('x',encoding='utf-8') as f: json.dump(value,f,indent=2)
def write(path,value):
    path.write_text(json.dumps(value,indent=2,ensure_ascii=False),encoding='utf-8')
mcp = None
lines, seq, inputs = queue.Queue(), 0, []
def rpc(method,params):
    global seq
    seq += 1
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','id':seq,'method':method,'params':params})+'\n')
    mcp.stdin.flush()
    deadline = time.monotonic()+180
    while time.monotonic() < deadline:
        response = json.loads(lines.get(timeout=max(.1,deadline-time.monotonic())))
        if response.get('id') == seq:
            assert 'error' not in response,response
            return response['result']
    raise RuntimeError('Uncertain RPC result; do not replay')
def call(name,args=None):
    result = rpc('tools/call',{'name':'reversebolt_'+name,'arguments':args or {}})
    response = json.loads(next(c['text'] for c in result['content'] if c['type']=='text'))
    if name != 'status': save(f'rpc-{seq:05}-{name}',{'args':args or {},'response':response})
    assert response['ok'],(name,response.get('error'))
    return response
def current_input():
    begin = ticks()
    window = user.GetForegroundWindow()
    owner = W.DWORD()
    user.GetWindowThreadProcessId(window,C.byref(owner))
    down = [k for k in range(1,256) if user.GetAsyncKeyState(k)&0x8000]
    return dict(begin_ticks=begin,end_ticks=ticks(),
        selected_foreground=owner.value==PID and window==target['window'],down=down)
def idle():
    state = call('status')
    assert state['jobs']['in_flight'] is None and state['jobs']['fence'] is None
    assert not state['state']['runtime_trials'] and not any(f['active'] for f in state['state']['freezes'])
    return state
def discovery_hold(key):
    p=current_input()
    assert p['selected_foreground'] and not p['down'],'Focus or input conflict'
    scan=user.MapVirtualKeyW(key,0)
    record={'key':key,'planned_ms':200,'begin_ticks':ticks()}
    user.keybd_event(0,scan,8,0)
    try:time.sleep(.2)
    finally:
        user.keybd_event(0,scan,10,0)
        record['end_ticks']=ticks();inputs.append(record)
        save('discovery-input-'+str(len(inputs)),record)

def measure(batch,index,paths,scope):
    idle()
    p=current_input()
    assert p['selected_foreground'] and not p['down'],'Focus or input conflict before read'
    key = {2:68,5:65,8:68,11:65}.get(index)
    process_dir=DATA/'runtime'/f"{scope['pid']}-{scope['created']}"
    lock=(process_dir/'lock').open('a+b')
    lock.seek(0);msvcrt.locking(lock.fileno(),msvcrt.LK_NBLCK,1)
    fence=process_dir/'active.json'
    assert not fence.exists() or read(fence).get('settled') is True,'Unsettled native operation'
    identity=str(uuid.uuid4());directory=process_dir/identity;directory.mkdir()
    output=directory/'report.json'
    pathfile=directory/'paths.json'
    pathfile.write_text(json.dumps(paths,ensure_ascii=False,separators=(',',':')),encoding='utf-8')
    request=json.dumps({'output':str(output),'root_start':0,'expected_root_catalog_sha256':None,
        'paths':paths,'patch':None},ensure_ascii=False,separators=(',',':'))
    assert pathfile.stat().st_size<=65536 and len(request.encode('utf-16-le'))//2<32768,'Native request exceeds budget'
    polls=[];poll_errors=[];stop=threading.Event();hold=None;loader=None
    poll_thread=None
    def poller():
        try:
            while not stop.is_set():
                polls.append(current_input());stop.wait(.005)
        except BaseException as error:poll_errors.append(repr(error))
    def hold_key():
        scan=user.MapVirtualKeyW(key,0)
        hold['begin_ticks']=ticks()
        deadline=time.monotonic()+3
        user.keybd_event(0,scan,8,0)
        try:
            while time.monotonic()<deadline:
                p=current_input()
                if not p['selected_foreground'] or any(k != key for k in p['down']):
                    hold['interrupted']='Focus changed or conflicting key';break
                time.sleep(min(.005,max(0,deadline-time.monotonic())))
        finally:
            user.keybd_event(0,scan,10,0);hold['end_ticks']=ticks()
    try:
        poll_thread=threading.Thread(target=poller,daemon=True);poll_thread.start()
        if key:
            hold={'key':key,'planned_ms':3000}
            input_thread=threading.Thread(target=hold_key);input_thread.start()
            time.sleep(.1)
        else:time.sleep(.25)
        write(fence,{'id':identity,'scope':scope,'helper_sha256':frozen['binaries']['reversebolt_runtime.dll'],'settled':False})
        args=[str(BIN/'reversebolt-runtime-loader.exe'),str(PID),scope['created'],str(scope['session']),
            str(BIN/'reversebolt_runtime.dll'),frozen['binaries']['reversebolt_runtime.dll'],str(output),
            '--paths-file',str(pathfile)]
        with (directory/'stdout.jsonl').open('w') as stdout,(directory/'stderr.jsonl').open('w') as stderr:
            try:
                loader=subprocess.Popen(args,stdout=stdout,stderr=stderr,creationflags=subprocess.CREATE_NO_WINDOW)
            except BaseException:
                write(fence,{'id':identity,'settled':True,'spawn_failed':True});raise
            save(f'loader-{batch}-{index}',{'pid':loader.pid,'directory':str(directory),'args':args})
            try:loader.wait(timeout=130)
            except subprocess.TimeoutExpired:
                save('needs-recovery',{'loader_pid':loader.pid,'directory':str(directory)});raise
        events=[json.loads(line) for line in (directory/'stdout.jsonl').read_text().splitlines()]
        first,last=events[0],events[-1]
        confirmed=first.get('stage')=='validated' and all(first.get(k)==v for k,v in scope.items())
        confirmed=confirmed and first.get('helper_sha256')==frozen['binaries']['reversebolt_runtime.dll']
        confirmed=confirmed and last.get('stage')=='complete' and last.get('helper_absent') is True
        if not confirmed:
            save('needs-recovery',{'loader_pid':loader.pid,'directory':str(directory)});
            raise RuntimeError('No confirmed native completion; retain target and engine')
        write(fence,{'id':identity,'settled':True})
        assert loader.returncode==0 and last['inspection_exit']==0
        time.sleep(.075)
    finally:
        if hold is not None:input_thread.join(timeout=5)
        stop.set()
        if poll_thread:poll_thread.join(timeout=2)
        lock.seek(0);msvcrt.locking(lock.fileno(),msvcrt.LK_UNLCK,1);lock.close()
        save(f'input-{batch}-{index}',{'hold':hold,'poll_errors':poll_errors,
            'scope':scope,'frequency_hz':frequency.value,'expected_keys':[key] if key else [],'polls':polls})
    assert not poll_errors
    assert all(p['selected_foreground'] for p in polls),'Focus changed during measurement'
    assert not any(any(k != key for k in p['down']) for p in polls),'Conflicting input during measurement'
    assert hold is None or 'interrupted' not in hold
    report=read(output)
    assert report['operation']=='refresh_runtime_paths' and report['scope']==scope and report['edit_authority'] is False
    return {'report':report,'input':{'scope':scope,'frequency_hz':frequency.value,
        'expected_keys':[key] if key else [],'polls':polls}}

save('started',{'runner_pid':os.getpid(),'engine_pid':read(DATA/'endpoint.json')['pid'],
    'game_pid':PID,'time_ns':time.time_ns()})
try:
    mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,stderr=(OUT/'mcp.stderr').open('w'),text=True,encoding='utf-8',creationflags=subprocess.CREATE_NO_WINDOW)
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'runtime-sibling-evaluator','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    initial=call('status')
    assert initial['state']['live'] is None and not initial['state']['runtime_trials']
    target=next(p for p in call('list_processes')['state']['processes'] if p['pid']==PID)
    save('target',target)
    call('set_observer_model',{'model':'qwen3.8:latest'})
    call('attach_process',{'pid':PID,'created':target['created'],'window':target['window'],'budget_mib':1,'auto_observe':True})
    handled=set();deadline=time.monotonic()+1000;next_progress=0
    while time.monotonic()<deadline:
        current=call('status');live=current['state']['live'];number=live['sample_count']
        if number not in handled:
            handled.add(number)
            if number in (2,4,8,10):discovery_hold(68 if number in (2,8) else 65)
        jobs=current['jobs']['history']['jobs']
        if time.monotonic()>=next_progress:
            print(json.dumps({'stage':'discovery','samples':number,'jobs':[j['phase'] for j in jobs.values()]}),flush=True)
            next_progress=time.monotonic()+20
        if jobs and all(j['phase'] in ('Completed','Failed','Cancelled','Interrupted') for j in jobs.values()):break
        time.sleep(.05 if number<15 else .5)
    else:raise RuntimeError('Discovery still pending; retain existing engine/job')
    save('discovery',current);idle()
    runtime=live['runtime'];review=runtime.get('naming')
    assert review and review['status']=='complete','No completed automatic hypotheses'
    report_path=Path(runtime['catalog_source']['path'])
    receipt=DATA/'runtime/naming'/review['evidence_id']/'receipt.json'
    selector=subprocess.run([str(OUT/'analysis-bin/reversebolt-runtime-neighbors.exe'),str(report_path),str(receipt),str(OUT/'plan')],
        capture_output=True,text=True,timeout=120,creationflags=subprocess.CREATE_NO_WINDOW)
    (OUT/'selection.stdout').write_text(selector.stdout);(OUT/'selection.stderr').write_text(selector.stderr)
    assert selector.returncode==0,selector.stderr
    plan=read(OUT/'plan/plan.json');scope=plan['scope'];results=[]
    assert scope['pid']==PID and scope['session']==3
    for batch,paths in enumerate(plan['batches'][:frozen['max_batches']]):
        observations=[]
        for index in range(14):
            observations.append(measure(batch,index,paths,scope))
            save(f'observation-{batch}-{index}',observations[-1])
            print(json.dumps({'stage':'measure','batch':batch,'sample':index+1,'fields':len(paths)}),flush=True)
        file=OUT/f'observations-{batch}.json';write(file,observations)
        response=subprocess.run([str(OUT/'analysis-bin/reversebolt-runtime-response.exe'),str(OUT/'plan/plan.json'),str(batch),str(file),str(OUT/f'analysis-{batch}.json')],
            capture_output=True,text=True,timeout=30,creationflags=subprocess.CREATE_NO_WINDOW)
        (OUT/f'analysis-{batch}.stdout').write_text(response.stdout);(OUT/f'analysis-{batch}.stderr').write_text(response.stderr)
        results.append({'batch':batch,'exit_code':response.returncode,'error':response.stderr,
            'analysis':read(OUT/f'analysis-{batch}.json') if response.returncode==0 else None})
    save('final-state',idle())
    save('result',{'workflow_finished':True,'selected_fields':len(plan['candidates']),'omitted_fields':plan['omitted_fields'],
        'omitted_batches':max(0,len(plan['batches'])-frozen['max_batches']),'discovery_inputs':inputs,'batches':results,
        'manually_reviewed_measurement_images':0,'gameplay_field_writes':0})
    print(json.dumps({'finished':True,'batches':len(results),'valid_batches':sum(r['exit_code']==0 for r in results)}),flush=True)
except BaseException as error:
    save('failure',{'error':repr(error),'discovery_inputs':inputs});raise
finally:
    if mcp is not None and mcp.poll() is None:
        mcp.stdin.close()
        try:mcp.wait(timeout=15)
        except subprocess.TimeoutExpired:mcp.terminate();mcp.wait(timeout=15)
    save('child',{'pid':mcp.pid if mcp else None,'exit_code':mcp.poll() if mcp else None})
