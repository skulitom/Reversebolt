"""Four product-chosen comparisons after a completed, restored native run."""
import ctypes as C
from ctypes import wintypes as W
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
import uuid

PARENT, OUT = (Path(p).resolve() for p in sys.argv[1:3])
ROOT = Path(__file__).resolve().parents[2]
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
prior = read(PARENT / 'frozen.json')
DATA, BIN = Path(prior['data']), Path(prior['bin'])
previous = read(PARENT / 'final-state.json')
assert read(PARENT / 'result.json')['workflow_finished']
assert not (PARENT / 'needs-recovery.json').exists()
assert not any(t['active'] for t in previous['state']['runtime_tests'])
assert not any(t['phase'] != 'restored' for t in previous['state']['runtime_trials'])
assert previous['jobs']['in_flight'] is None and previous['jobs']['fence'] is None
for name, digest in prior['binaries'].items():
    assert sha(BIN / name) == digest
session = W.DWORD()
assert C.WinDLL('kernel32').ProcessIdToSessionId(os.getpid(), C.byref(session)) and session.value == 3
OUT.mkdir()
sources = [Path(__file__), Path(__file__).with_name('PROTOCOL-FOLLOWUP.md')]
for p in sources:
    shutil.copy2(p, OUT / p.name)
shutil.copy2(PARENT / 'frozen.json', OUT / 'parent-frozen.json')
shutil.copy2(PARENT / 'result.json', OUT / 'parent-result.json')
shutil.copy2(PARENT / 'final-state.json', OUT / 'parent-final-state.json')
manifest = {'parent': str(PARENT), 'data': str(DATA), 'bin': str(BIN),
    'binaries': prior['binaries'], 'max_comparisons': 4,
    'parent_artifacts': {p.name: sha(p) for p in OUT.glob('parent-*.json')},
    'sources': {p.name: sha(p) for p in sources}, 'runner_pid': os.getpid(),
    'started_ns': time.time_ns()}
(OUT / 'frozen.json').write_text(json.dumps(manifest, indent=2)+'\n')
endpoint = read(DATA / 'endpoint.json')
assert endpoint['pid'] == read(PARENT / 'started.json')['engine_pid']
seq, active_test, mcp = 0, None, None
lines = queue.Queue()

def save(name, value):
    with (OUT / (name + '.json')).open('x', encoding='utf-8') as f:
        json.dump(value, f, indent=2)

def rpc(method, params):
    global seq
    seq += 1
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0', 'id':seq, 'method':method, 'params':params})+'\n')
    mcp.stdin.flush()
    end = time.monotonic() + 180
    while time.monotonic() < end:
        response = json.loads(lines.get(timeout=max(.1, end-time.monotonic())))
        if response.get('id') == seq:
            assert 'error' not in response, response
            return response['result']
    raise RuntimeError('Unknown RPC outcome; never replay')

def call(name, args=None):
    raw = rpc('tools/call', {'name':'reversebolt_'+name, 'arguments':args or {}})
    response = json.loads(next(c['text'] for c in raw['content'] if c['type']=='text'))
    save(f'rpc-{seq:05}-{name}', {'args':args or {}, 'response':response})
    assert response['ok'], (name, response.get('error'))
    return response

def observe(test):
    for _ in range(6):
        test = call('observe_runtime_test', {'test_id':test['id'], 'after_revision':test['revision'], 'perform_action':True})['data']
        assert test['phase'] != 'stopped', test
    return test

try:
    mcp = subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=(OUT/'mcp.stderr').open('w'),
        text=True, encoding='utf-8', creationflags=subprocess.CREATE_NO_WINDOW)
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc('initialize', {'protocolVersion':'2025-11-25','capabilities':{},
        'clientInfo':{'name':'bounded-runtime-continuation','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n')
    mcp.stdin.flush()
    current = call('status')
    assert current['state']['live']['process'] == previous['state']['live']['process']
    assert current['state']['runtime_tests'] == previous['state']['runtime_tests']
    assert current['state']['runtime_trials'] == previous['state']['runtime_trials']
    runtime = current['state']['live']['runtime']
    outcomes = []
    for number in range(manifest['max_comparisons']):
        plan = call('plan_runtime_comparisons', {'catalog_id':runtime['catalog_id'], 'review_id':runtime['naming']['evidence_id']})['data']
        choice = plan['next']
        if not choice:
            break
        active_test, change = str(uuid.uuid4()), str(uuid.uuid4())
        args = {'test_id':active_test, 'change_id':change, 'investigation_id':plan['investigation_id'],
            'catalog_id':plan['catalog_id'], 'after_revision':plan['after_revision'],
            'hypothesis_ref':choice['hypothesis_ref'], 'value':choice['value'], 'actions':choice['actions']}
        save(f'chosen-{number+1}', {'plan':plan,'args':args})
        print(json.dumps({'comparison':number+1, 'name':choice['name'], 'role':choice['selection_role'], 'phase':'baseline'}), flush=True)
        test = observe(call('prepare_runtime_test', args)['data'])
        assert test['phase'] == 'ready_to_apply', test
        fresh = call('status')['state']['live']
        trial = call('set_runtime_value', {'change_id':change, 'investigation_id':fresh['investigation_id'],
            'catalog_id':fresh['runtime']['catalog_id'], 'after_revision':fresh['runtime']['revision'],
            'candidate_id':test['candidate_id'], 'value':choice['value']})['data']
        assert trial['phase'] == 'written', trial['phase']
        test = next(t for t in call('status')['state']['runtime_tests'] if t['id']==active_test)
        test = observe(test)
        restored = call('restore_runtime_change', {'change_id':change})['data']
        assert restored['phase'] == 'restored', restored['phase']
        test = next(t for t in call('status')['state']['runtime_tests'] if t['id']==active_test)
        assert test['phase'] == 'restored', test
        test = observe(test)
        assert test['phase'] == 'complete' and not test['active'], test
        result = call('read_runtime_test', {'test_id':active_test})['data']
        save(f'comparison-{number+1}', result)
        outcomes.append({'number':number+1, 'name':choice['name'], 'selected_path':choice['hypothesis_ref']['selected_path_id'],
            'test_id':active_test, 'change_id':change, 'phase':test['phase'], 'assessment':test['assessment']})
        active_test = None
        print(json.dumps(outcomes[-1]), flush=True)
    save('result', {'workflow_finished':True, 'outcomes':outcomes,'manually_reviewed_measurement_images':0})
except BaseException as error:
    save('failure', {'error':repr(error)})
    raise
finally:
    if mcp is not None and mcp.poll() is None:
        try:
            current = call('status')
            if active_test and any(t['id']==active_test and t['active'] for t in current['state']['runtime_tests']):
                call('stop_runtime_test', {'test_id':active_test})
            for trial in call('status')['state']['runtime_trials']:
                if trial['phase']=='written':
                    call('restore_runtime_change', {'change_id':trial['id']})
            settled = call('status')
            save('final-state', settled)
            if any(t['phase'] in ('written','uncertain') for t in settled['state']['runtime_trials']):
                save('needs-recovery', {'reason':'Unresolved trial; keep engine alive'})
        except BaseException as error:
            save('needs-recovery', {'reason':repr(error)})
        mcp.stdin.close()
        try:
            mcp.wait(timeout=15)
        except subprocess.TimeoutExpired:
            mcp.terminate()
            mcp.wait(timeout=15)
    save('child', {'pid':mcp.pid if mcp else None,'exit_code':mcp.poll() if mcp else None})
