"""Continue a settled plan after fixing only the session-local window identity."""
import ast,ctypes as C
from ctypes import wintypes as W
import hashlib,json,os,subprocess,threading,time,sys
from pathlib import Path
OUT=Path(sys.argv[1]).resolve();BIN=OUT/'bin'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json');PRIOR=Path(frozen['prior'])
assert sha(PRIOR/'frozen.json')==frozen['prior_frozen_sha256']
assert read(PRIOR/'failure.json')['holds']==[] and len(read(PRIOR/'failure.json')['calls'])==1
assert sha(Path(__file__))==frozen['sources']['research/runtime-embedded-activity/resume.py']
for name,digest in frozen['binaries'].items():assert sha(BIN/name)==digest
source=Path(frozen['measurement_source']);assert sha(source)==frozen['measurement_source_sha256']
names={'save','ticks','poll','command','native','measure'}
definitions=[n for n in ast.parse(source.read_text(encoding='utf-8-sig')).body if isinstance(n,ast.FunctionDef) and n.name in names]
assert {n.name for n in definitions}==names
exec(compile(ast.Module(body=definitions,type_ignores=[]),str(source),'exec'),globals())
target=read(OUT/'target.json');PID=target['pid'];scope={k:target[k] for k in ['pid','created','session']}
assert all(target[k]==read(PRIOR/'target.json')[k] for k in scope) and target['window']>0
user,kernel=C.WinDLL('user32'),C.WinDLL('kernel32')
user.GetForegroundWindow.restype=W.HWND;user.GetWindowThreadProcessId.argtypes=[W.HWND,C.POINTER(W.DWORD)]
user.GetAsyncKeyState.restype=C.c_short;user.MapVirtualKeyW.restype=W.UINT
frequency=C.c_int64();assert kernel.QueryPerformanceFrequency(C.byref(frequency))
session=W.DWORD();assert kernel.ProcessIdToSessionId(os.getpid(),C.byref(session)) and session.value==scope['session']==3
initial=poll();assert initial['selected_foreground'] and not initial['down']
assert not (OUT/'started.json').exists(),'Do not replay a started continuation'
calls=read(PRIOR/'failure.json')['calls'];holds=[]
discovery=read(OUT/'discovery.json');pauses=discovery['snapshots']+[w['discarded_snapshot'] for w in discovery['warmups']]
plan=read(OUT/'plan/plan.json');assert plan['scope']==scope and sha(OUT/'plan/plan.json')==read(OUT/'frozen-plan.json')['sha256']
receipt=read(OUT/'selection/receipt.json')
save('started',{'runner_pid':os.getpid(),'target':target,'unix_ms':time.time_ns()//1000000,'resumed_plan':True,'initial_input':initial})
try:
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
        'calls':calls,'reused_discovery_calls':1,'pauses':len(pauses),'max_stop_to_resume_us':max(s['stop_to_resume_us'] for s in pauses),'holds':holds,
        'model_calls':len(receipt['stages']),'additional_model_calls':0,'gameplay_writes':0,'measurement_images':0})
    print(json.dumps({'finished':True,'valid_batches':sum(b['exit']==0 for b in results),'associations':sum(sum(f['repeated_association'] for f in b['analysis']['fields']) for b in results if b['exit']==0)}),flush=True)
except BaseException as error:
    save('failure',{'error':repr(error),'calls':calls,'holds':holds});raise
