"""Owned Anode fixture: unavailable optional model must not stop a capture job.
Usage: native.py <binary-directory> <new-output-directory> <child-session> <parent-session>
Leaves owned windows available for accessibility inspection until finish-ui exists.
"""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import queue
import shutil
import subprocess
import sys
import threading
import time

ROOT=Path(__file__).resolve().parents[2]
binary,out=map(lambda p:Path(p).resolve(),sys.argv[1:3])
session=ctypes.c_ulong()
assert ctypes.windll.kernel32.ProcessIdToSessionId(os.getpid(),ctypes.byref(session))
assert session.value==int(sys.argv[3]) and session.value!=int(sys.argv[4])
out.mkdir(exist_ok=False)
for entry in json.loads((ROOT/'models/manifest.json').read_text())['models']:
    source=ROOT/'models'/entry['name']; dest=out/'models'/entry['name']
    assert hashlib.sha256(source.read_bytes()).hexdigest()==entry['sha256'].lower()
    dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,dest)
control=out/'fixture.txt';control.write_text('72')
fixture=engine=mcp=None
lines=queue.Queue();serial=0
def rpc(method,params):
    global serial
    serial+=1
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','id':serial,'method':method,'params':params})+'\n');mcp.stdin.flush()
    while True:
        value=json.loads(lines.get(timeout=120))
        if value.get('id')==serial:
            assert 'error' not in value,value
            return value['result']
def call(name,args=None):
    response=rpc('tools/call',{'name':'reversebolt_'+name,'arguments':args or {}})
    assert not response.get('isError'),response
    value=json.loads(response['content'][-1]['text']);assert value['ok'],value.get('error')
    return value
try:
    fixture=subprocess.Popen(['powershell','-NoProfile','-ExecutionPolicy','Bypass','-File',
        str(ROOT/'scripts/observation-fixture.ps1'),'-ControlFile',str(control)],creationflags=0x08000000)
    # This is an owned visible test application, on the guarded Anode child session.
    engine=subprocess.Popen([str(binary/'Reversebolt.exe'),'--data-dir',str(out)])
    for _ in range(200):
        assert engine.poll() is None and fixture.poll() is None
        if (out/'endpoint.json').exists():break
        time.sleep(.05)
    assert (out/'endpoint.json').exists()
    mcp=subprocess.Popen([str(binary/'reversebolt-mcp.exe'),'--data-dir',str(out)],
        stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8',creationflags=0x08000000)
    threading.Thread(target=lambda:[lines.put(s) for s in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'resilience-check','version':'1'}})
    mcp.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n');mcp.stdin.flush()
    assert len(rpc('tools/list',{})['tools'])==29
    target=None
    for _ in range(100):
        target=next((p for p in call('list_processes')['state']['processes'] if p['pid']==fixture.pid),None)
        if target:break
        time.sleep(.1)
    assert target and target['session']==session.value
    state=call('attach_process',{**{k:target[k] for k in ['pid','created','window']},'budget_mib':1,'auto_observe':False})
    history=state['jobs']['history']
    state=call('start_observation_job',{'request_id':history['clients'].get('mcp-agent',{}).get('high_water',0)+1,
        'boot':history['boot'],'binding':history['current'],'count':4,'interval_ms':250,
        'local_model':'reversebolt-deliberately-absent-resilience-test'})
    job=state['data']['ack']['job']
    deadline=time.monotonic()+180;trace=[]
    while True:
        state=call('status'); live=state['state']['live'];phase=state['jobs']['history']['jobs'][str(job)]['phase']
        trace.append({'captures':live['sample_count'],'warnings':len(live['local_observation_warnings']),'phase':phase})
        if phase in ['Completed','Failed','Cancelled','Interrupted'] and not state['state']['analysis']:break
        assert time.monotonic()<deadline,'Observation deadline; preserve the exact owned process for diagnosis'
        time.sleep(.1)
    (out/'state.json').write_text(json.dumps(state,indent=2),encoding='utf-8')
    (out/'trace.json').write_text(json.dumps(trace,indent=2),encoding='utf-8')
    assert phase=='Completed' and live['sample_count']==4
    assert len(live['local_observation_warnings'])==2
    assert all('not installed' in w['message'] for w in live['local_observation_warnings'])
    assert not live['visual_tracks'] and not state['state']['patches'] and state['state']['error'] is None
    assert any(t['samples']==[72.0]*4 for t in live['tracks'])
    assert any(t['warnings']==1 and t['captures']<4 for t in trace)
    receipts=list((out/'captures/model-observations').glob('*.json'));assert len(receipts)==2
    for p in receipts:
        value=json.loads(p.read_text());assert value['pass']['error'] and not value['proposed_observations']
    ready={'engine_pid':engine.pid,'fixture_pid':fixture.pid,'session':session.value,'captures':4,'phase':phase,
        'warning_count':2,'current_numeric_value':72,'binary_sha256':hashlib.sha256((binary/'Reversebolt.exe').read_bytes()).hexdigest(),
        'receipts':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in receipts}}
    (out/'ready.json').write_text(json.dumps(ready,indent=2),encoding='utf-8');print(json.dumps(ready),flush=True)
    deadline=time.monotonic()+300
    while not (out/'finish-ui').exists() and time.monotonic()<deadline:time.sleep(.25)
finally:
    control.write_text('close')
    for process in [mcp,engine,fixture]:
        if process and process.poll() is None:
            process.terminate()
            try:process.wait(timeout=5)
            except subprocess.TimeoutExpired:process.kill();process.wait(timeout=5)
    print(json.dumps({'owned_processes_closed':True}),flush=True)
