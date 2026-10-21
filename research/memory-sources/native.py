"""Owned Anode process-family integration; oracle addresses never reach the engine.
Usage: native.py <binary-directory> <new-output> <seat-session> <parent-session>
"""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
binary, out = [Path(p).resolve() for p in sys.argv[1:3]]
session = ctypes.c_ulong()
assert ctypes.windll.kernel32.ProcessIdToSessionId(os.getpid(), ctypes.byref(session))
assert session.value == int(sys.argv[3]) and session.value != int(sys.argv[4])
out.mkdir(exist_ok=False)
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
def save(name, value):
    (out/name).write_text(json.dumps(value, indent=2)+'\n', encoding='utf-8')
save('manifest.json', {'binary_sha256': sha(binary/'Reversebolt.exe'),
    'mcp_sha256': sha(binary/'reversebolt-mcp.exe'),
    'method_sha256': {p.name: sha(p) for p in [HERE/'native.py', HERE/'fixture.ps1']},
    'scope': 'Owned known-arena fixture; API isolation and native access, not gameplay generalization.'})
engine = fixture = mcp = None
serial = control_id = 0
lines = queue.Queue()
trace = []
def rpc(method, params):
    global serial
    serial += 1
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','id':serial,'method':method,'params':params})+'\n');mcp.stdin.flush()
    while True:
        value = json.loads(lines.get(timeout=120))
        if value.get('id') == serial:
            assert 'error' not in value, value
            return value['result']
def call(name, args=None, ok=True):
    result = rpc('tools/call', {'name':'reversebolt_'+name,'arguments':args or {}})
    value = json.loads(result['content'][-1]['text'])
    trace.append({'name':name,'args':args,'ok':value['ok'],'error':value.get('error')})
    save('commands.json', trace)
    assert value['ok'] == ok, value.get('error')
    return value
def fixture_command(op, worker=0):
    global control_id
    control_id += 1
    save('control.json', {'op':op,'worker':worker,'id':control_id})
    if op == 'close': return
    deadline = time.monotonic()+8
    while time.monotonic() < deadline:
        if (out/'response.json').exists():
            try:
                reply = read(out/'response.json')
                if reply['id'] == control_id: return reply
            except (ValueError, OSError): pass
        time.sleep(.05)
    raise TimeoutError('Fixture command did not finish; do not replay it.')
try:
    fixture = subprocess.Popen(['powershell','-NoProfile','-ExecutionPolicy','Bypass','-File',
        str(HERE/'fixture.ps1'),'-Binary',str(binary/'Reversebolt.exe'),'-Directory',str(out)],
        stdout=(out/'fixture-stdout.log').open('w'),stderr=(out/'fixture-stderr.log').open('w'),creationflags=0x08000000)
    engine = subprocess.Popen([str(binary/'Reversebolt.exe'),'--data-dir',str(out/'engine')])
    for _ in range(200):
        assert fixture.poll() is None and engine.poll() is None
        if (out/'workers.json').exists() and (out/'engine/endpoint.json').exists(): break
        time.sleep(.05)
    workers = read(out/'workers.json')
    fixture_command('step')
    mcp = subprocess.Popen([str(binary/'reversebolt-mcp.exe'),'--data-dir',str(out/'engine')],
        stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(out/'mcp-stderr.log').open('w'),
        text=True,encoding='utf-8',creationflags=0x08000000)
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc('initialize', {'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'memory-source-test','version':'1'}})
    mcp.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n');mcp.stdin.flush()
    assert len(rpc('tools/list', {})['tools']) == 30
    window = None
    for _ in range(100):
        window = next((p for p in call('list_processes')['state']['processes'] if p['pid']==fixture.pid), None)
        if window: break
        time.sleep(.1)
    assert window and window['session'] == session.value
    args = {k:window[k] for k in ['pid','created','window']}
    args.update(budget_mib=16, auto_observe=False)
    parent = call('attach_process', args)
    parent_id = parent['state']['live']['investigation_id']
    assert parent['state']['live']['memory_process']['pid'] == fixture.pid
    sources = call('list_memory_processes')['data']
    save('sources.json', sources)
    assert not sources['error']
    children = [next(p for p in sources['processes'] if p['pid']==w['pid']) for w in workers]
    assert all(p['window'] == 0 for p in children)
    for bad in [{'memory_pid':engine.pid,'memory_created':children[0]['created']},
                {'memory_pid':children[0]['pid'],'memory_created':'0000000000000000'},
                {'memory_pid':children[0]['pid']}]:
        failed = call('attach_process', {**args,**bad}, ok=False)
        assert failed['state']['live']['investigation_id'] == parent_id
    child = children[0]
    attached = call('attach_process', {**args,'memory_pid':child['pid'],'memory_created':child['created']})
    investigation = attached['state']['live']['investigation_id']
    assert investigation != parent_id and attached['state']['live']['sample_count'] == 0
    readings = []
    for _ in range(12):
        expected = fixture_command('step')
        state = call('capture')
        live = state['state']['live']
        assert live['process']['pid'] == fixture.pid and live['memory_process']['pid'] == child['pid']
        readings.append({'expected':expected,'latest':live['latest']})
    state = call('correlate')
    save('child-state.json', state);save('readings.json', readings)
    live = state['state']['live']
    base = workers[0]['base']; end = base+workers[0]['length']
    # Arena knowledge is used only after the product has independently selected/read memory.
    covered = all(any(int(r['address'],16)<=base and int(r['address'],16)+r['retained']>base
                      for r in row['latest']['memory_reads']) for row in readings)
    matches = [m for t in live['tracks'] for m in t['matches'] if base<=int(m['address'],16)<end]
    assert covered, 'The bounded batch did not include the owned child arena; retain as a failure.'
    assert matches, 'No numeric correspondence in the owned arena; retain failure.'
    other = children[1]
    switched = call('attach_process', {**args,'memory_pid':other['pid'],'memory_created':other['created']})
    fresh = switched['state']['live']
    assert fresh['investigation_id'] != investigation
    assert fresh['memory_process']['pid'] == other['pid'] and not fresh['tracks'] and fresh['sample_count']==0
    fixture_command('kill', 1)
    failed = call('capture', ok=False)
    assert not failed['state']['live']['connected']
    save('exited-state.json', failed)
    # Return to the surviving child for accessibility inspection, with fresh evidence.
    final = call('attach_process', {**args,'memory_pid':child['pid'],'memory_created':child['created']})
    final = call('capture')
    save('final-state.json', final)
    assert not final['state']['changes'] and not final['state']['patches']
    ready = {'engine_pid':engine.pid,'fixture_pid':fixture.pid,'child_pid':child['pid'],
        'session':session.value,'captures':12,'owned_arena_matches':len(matches),
        'stale_identity_rejected':True,'unrelated_process_rejected':True,'partial_identity_rejected':True,
        'source_switch_starts_fresh':True,'child_exit_invalidates':True,'writes':0}
    save('ready.json', ready);print(json.dumps(ready),flush=True)
    deadline = time.monotonic()+300
    while not (out/'finish-ui').exists() and time.monotonic()<deadline: time.sleep(.25)
finally:
    fixture_command('close')
    if fixture and fixture.poll() is None:
        try: fixture.wait(timeout=8)
        except subprocess.TimeoutExpired: fixture.terminate();fixture.wait(timeout=5)
    for process in [mcp, engine]:
        if process and process.poll() is None:
            process.terminate();process.wait(timeout=5)
    print(json.dumps({'owned_processes_closed':True}),flush=True)
