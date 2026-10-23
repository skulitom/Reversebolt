"""Resume observation from a settled partial review; never replay discovery."""
import ast
import ctypes as C
from ctypes import wintypes as W
import hashlib
import json
import msvcrt
import os
from pathlib import Path
import queue
import subprocess
import threading
import time
import uuid

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/runtime-siblings-sable-selected-v1'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json');PRIOR=Path(frozen['prior'])
assert sha(PRIOR/'frozen.json')==frozen['prior_frozen_sha256']
assert sha(Path(__file__))==frozen['sources']['research/runtime-siblings/selected.py']
DATA,BIN=Path(frozen['data']),Path(frozen['bin'])
for name,digest in frozen['binaries'].items():assert sha(BIN/name)==digest
for name,digest in frozen['analysis'].items():assert sha(OUT/'analysis-bin'/name)==digest
source=Path(frozen['measurement_source'])
assert sha(source)==frozen['measurement_source_sha256']
# Only these already frozen function definitions are reused; no module startup,
# discovery loop, model invocation or comparison/edit routine is executed.
names={'ticks','save','write','rpc','call','current_input','idle','measure'}
tree=ast.parse(source.read_text(encoding='utf-8-sig'))
definitions=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names]
assert {n.name for n in definitions}==names
exec(compile(ast.Module(body=definitions,type_ignores=[]),str(source),'exec'),globals())
target=read(PRIOR/'target.json');PID=target['pid']
user,kernel=C.WinDLL('user32'),C.WinDLL('kernel32')
user.GetForegroundWindow.restype=W.HWND
user.GetWindowThreadProcessId.argtypes=[W.HWND,C.POINTER(W.DWORD)]
user.GetAsyncKeyState.restype=C.c_short
user.MapVirtualKeyW.restype=W.UINT
session=W.DWORD();assert kernel.ProcessIdToSessionId(os.getpid(),C.byref(session)) and session.value==3
frequency=C.c_int64();assert kernel.QueryPerformanceFrequency(C.byref(frequency))
mcp=None;lines=queue.Queue();seq=0
save('started',{'runner_pid':os.getpid(),'engine_pid':read(DATA/'endpoint.json')['pid'],
    'game_pid':PID,'time_ns':time.time_ns(),'partial_review':True})
try:
    mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,stderr=(OUT/'mcp.stderr').open('w'),text=True,encoding='utf-8',creationflags=subprocess.CREATE_NO_WINDOW)
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'runtime-sibling-partial-evaluator','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    current=idle();save('initial-state',current)
    now=next(p for p in call('list_processes')['state']['processes'] if p['pid']==PID)
    assert now['created'].lower()==target['created'].lower() and now['window']==target['window']
    assert current['state']['live']['runtime']['catalog_source']['document_sha256']==read(OUT/'plan/plan.json')['report_sha256']
    plan=read(OUT/'plan/plan.json');scope=plan['scope'];results=[]
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
        'omitted_batches':max(0,len(plan['batches'])-frozen['max_batches']),
        'omitted_roles':read(OUT/'plan/seed-selection.json')['omitted_roles'],
        'discovery_inputs':read(PRIOR/'failure.json')['discovery_inputs'],'batches':results,
        'manually_reviewed_measurement_images':0,'gameplay_field_writes':0,'additional_model_requests':0})
    print(json.dumps({'finished':True,'batches':len(results),'valid_batches':sum(r['exit_code']==0 for r in results)}),flush=True)
except BaseException as error:
    save('failure',{'error':repr(error)});raise
finally:
    if mcp is not None and mcp.poll() is None:
        mcp.stdin.close()
        try:mcp.wait(timeout=15)
        except subprocess.TimeoutExpired:mcp.terminate();mcp.wait(timeout=15)
    save('child',{'pid':mcp.pid if mcp else None,'exit_code':mcp.poll() if mcp else None})
