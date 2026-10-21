"""Historical smoke for the blocking first-pass build (f2b5dd1).
For current builds use background-observer-smoke.py; pause now has a separate
analysis-drain state, so this older check's receipt-count assumptions do not apply.
MCP/native-window/local-model integration, using saved imagery without gameplay.
Run a desktop engine and image-fixture.ps1 in Anode, then provide DATA_DIR PID MODEL.
Leaves the engine showing its provisional findings for a separate UIA check.
"""
import hashlib
import json
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time

data, target_pid, model = Path(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
root = Path(__file__).resolve().parents[1]
process = subprocess.Popen([str(root/'target/release/reversebolt-mcp.exe'), '--data-dir', str(data)],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    text=True, encoding='utf-8', creationflags=0x08000000)
lines = queue.Queue()
threading.Thread(target=lambda: [lines.put(line) for line in process.stdout], daemon=True).start()
serial = 0

def rpc(method, params):
    global serial
    serial += 1
    process.stdin.write(json.dumps({'jsonrpc':'2.0','id':serial,'method':method,'params':params})+'\n')
    process.stdin.flush()
    while True:
        value = json.loads(lines.get(timeout=120))
        if value.get('id') == serial:
            assert 'error' not in value, value
            return value['result']

def call(action, args=None, error=False):
    value = rpc('tools/call', {'name':'reversebolt_'+action, 'arguments':args or {}})
    assert bool(value.get('isError')) == error, value
    payload = json.loads(value['content'][-1]['text'])
    assert payload['ok'] != error, payload.get('error')
    return payload

def control(action, extra):
    history = call('status')['jobs']['history']
    args = {'request_id':history['clients'].get('mcp-agent',{}).get('high_water',0)+1,
        'boot':history['boot'], 'binding':history['current'], **extra}
    return call(action,args)

def wait(predicate, seconds=180):
    deadline=time.monotonic()+seconds
    while time.monotonic()<deadline:
        began=time.monotonic()
        state=call('status')
        latencies.append(time.monotonic()-began)
        if predicate(state): return state
        time.sleep(.15)
    raise AssertionError('The owned job did not reach the requested state before the observation deadline; do not restart it automatically.')

def job(state, id): return state['jobs']['history']['jobs'][str(id)]

latencies=[]
try:
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'local-observer-native-check','version':'1'}})
    process.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');process.stdin.flush()
    assert len(rpc('tools/list',{})['tools'])==24
    configured=call('set_observer_model',{'model':model})
    assert configured['state']['observer_model']==model
    rejected=call('set_observer_model',{'model':'reversebolt-definitely-not-installed'},error=True)
    assert rejected['state']['observer_model']==model
    state=call('list_processes')
    target=next(p for p in state['state']['processes'] if p['pid']==target_pid)
    attached=call('attach_process',{**{k:target[k] for k in ['pid','created','window']},'budget_mib':1})
    first=attached['data']['automatic_observation']['ack']['job']
    plans=[json.loads(s['payload']) for s in job(attached,first)['steps']]
    assert [i for i,p in enumerate(plans) if p.get('local_model')]==[1,15]
    state=wait(lambda s: bool(s['state']['live']['visual_tracks']) or job(s,first)['phase']=='Failed')
    assert job(state,first)['phase']!='Failed', state['state'].get('error')
    control('job_control',{'job':first,'operation':'pause'})
    state=wait(lambda s:job(s,first)['phase']=='Paused')
    tracks=[t for t in state['state']['live']['visual_tracks'] if t['submitted_by']=='local-model']
    assert tracks, 'No local appearance findings; retain the failure evidence rather than adding fixture labels.'
    assert all(not t['matches'] for t in tracks)
    assert all(o['input']['quality']=='provisional' and o['input']['value']['kind']=='text' for t in tracks for o in t['observations'] if o)
    assert not state['state']['patches']
    receipts=list((data/'captures/model-observations').glob('*.json'))
    assert len(receipts)==1
    proof=json.loads(receipts[0].read_text())
    frame_map={f['id']:f for f in state['state']['live']['frames']}
    for frame in proof['sequence']['frames']:
        assert frame['sha256']==frame_map[frame['id']]['sha256']
    assert len(proof['proposed_observations'])==len(tracks)
    print(json.dumps({'automatic_cards':len(tracks),'names':[t['label'] for t in tracks],'paused_after':state['state']['live']['sample_count']}),flush=True)
    (data/'cards-state.json').write_text(json.dumps(state,indent=2),encoding='utf-8')
    control('job_control',{'job':first,'operation':'cancel'})
    state=wait(lambda s:job(s,first)['phase']=='Cancelled')
    started=control('start_observation_job',{'count':2,'interval_ms':50,'mode':'visual','local_model':model})
    second=started['data']['ack']['job']
    def model_in_flight(s):
        flight=s['jobs']['in_flight']
        return flight and flight['operation']['job']==second and json.loads(flight['operation']['instruction']['payload']).get('local_model')==model
    wait(model_in_flight)
    time.sleep(.6)  # Let native capture settle and inference begin; no gameplay input.
    began=time.monotonic()
    ack=control('job_control',{'job':second,'operation':'cancel'})
    cancellation_ms=(time.monotonic()-began)*1000
    state=wait(lambda s:job(s,second)['phase']=='Cancelled')
    assert len(state['state']['live']['visual_tracks'])==len(tracks)
    receipts=list((data/'captures/model-observations').glob('*.json'))
    assert len(receipts)==2
    cancelled=[json.loads(p.read_text()) for p in receipts if json.loads(p.read_text())['pass']['cancelled']]
    assert len(cancelled)==1 and not cancelled[0]['proposed_observations']
    assert len(cancelled[0]['pass']['reviews'])<6
    persisted=json.loads((data/'session.json').read_text())
    assert persisted['observer_model']==model
    result={'scope':'Owned native window displaying previously seen saved imagery; integration evidence, not a new gameplay/generalization evaluation.',
        'cards':len(tracks),'names':[t['label'] for t in tracks], 'model':model,
        'automatic_plan_models':[i+1 for i,p in enumerate(plans) if p.get('local_model')],
        'cancellation_ack_ms':cancellation_ms,'maximum_status_ms':max(latencies)*1000,
        'cancelled_pass_reviews':len(cancelled[0]['pass']['reviews']),
        'no_publication_after_cancel':True,'provisional_text_only':True,'no_memory_matches_or_writes':True,
        'model_setting_persisted':True,'invalid_model_preserved_setting':True,
        'receipts':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in receipts}}
    (data/'integration-result.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
    print(json.dumps(result),flush=True)
finally:
    process.terminate()
    try:process.wait(timeout=5)
    except subprocess.TimeoutExpired:process.kill();process.wait(timeout=5)
