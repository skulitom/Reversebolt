"""Check overlapping capture/analysis, source freshness and stop/resume via MCP.
Use an owned image fixture in Anode. Arguments: DATA_DIR PID MODEL [BINARY_DIR].
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
binaries=Path(sys.argv[4]) if len(sys.argv)>4 else root/'target/release'
process = subprocess.Popen([str(binaries/'reversebolt-mcp.exe'), '--data-dir', str(data)],
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
        trace.append({'elapsed':time.monotonic()-started_at,'analysis':state['state'].get('analysis'),'sample_count':(state['state'].get('live') or {}).get('sample_count'),'jobs':{k:v['phase'] for k,v in state['jobs']['history']['jobs'].items()}})
        if predicate(state): return state
        time.sleep(.15)
    raise AssertionError('The owned job did not reach the requested state before the observation deadline; do not restart it automatically.')

def job(state, id): return state['jobs']['history']['jobs'][str(id)]

latencies=[]
trace=[]
started_at=time.monotonic()
try:
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'background-observer-check','version':'1'}})
    process.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');process.stdin.flush()
    assert len(rpc('tools/list',{})['tools'])==24
    call('set_observer_model',{'model':model})
    target=next(p for p in call('list_processes')['state']['processes'] if p['pid']==target_pid)
    binding={k:target[k] for k in ['pid','created','window']}
    state=call('attach_process',{**binding,'budget_mib':1})
    first=state['data']['automatic_observation']['ack']['job']
    plans=[json.loads(s['payload']) for s in job(state,first)['steps']]
    assert [i for i,p in enumerate(plans) if p.get('background_model')]==[1]
    state=wait(lambda s:bool(s['state'].get('analysis')) or job(s,first)['phase']=='Failed')
    assert job(state,first)['phase']!='Failed',state['state'].get('error')
    first_review=state['state']['analysis']['evidence_id']
    state=wait(lambda s:job(s,first)['phase'] in ['Completed','Failed'])
    assert job(state,first)['phase']=='Completed',state['state'].get('error')
    assert state['state']['analysis'] is None
    assert state['state']['live']['sample_count']==16
    overlap=[t['sample_count'] for t in trace if t['analysis'] and t['analysis']['evidence_id']==first_review]
    assert max(overlap)>2, 'No capture advanced while the first analysis was outstanding'
    tracks=state['state']['live']['visual_tracks']
    assert tracks and all(not t['matches'] for t in tracks)
    assert not state['state']['patches']
    first_receipt=json.loads((data/'captures/model-observations'/f'{first_review}.json').read_text(encoding='utf-8'))
    first_frame=first_receipt['sequence']['frames'][-1]['id']
    early=[t for t in tracks if any(o and o['input']['frame_id']==first_frame for o in t['observations'])]
    assert early and all(t['observations'][-1] is None for t in early)
    (data/'complete-state.json').write_text(json.dumps(state,indent=2),encoding='utf-8')
    print(json.dumps({'automatic_complete':True,'captures_while_first_review_pending':max(overlap)-2,'cards':len(tracks)}),flush=True)

    # A fresh investigation gives an isolated stop/resume check without another automatic job.
    call('attach_process',{**binding,'budget_mib':1,'auto_observe':False})
    state=control('start_observation_job',{'count':4,'interval_ms':1000,'mode':'visual','local_model':model})
    second=state['data']['ack']['job']
    state=wait(lambda s:bool(s['state'].get('analysis')) or job(s,second)['phase']=='Failed')
    assert state['state']['analysis'],state['state'].get('error')
    cancelled_review=state['state']['analysis']['evidence_id']
    began=time.monotonic()
    ack=control('job_control',{'job':second,'operation':'pause'})
    pause_ms=(time.monotonic()-began)*1000
    assert ack['ok']
    # Collection may be Paused while an already-issued image request drains.
    if ack['state'].get('analysis'):
        call('detach_process',error=True)
        history=call('status')['jobs']['history']
        call('job_control',{'request_id':history['clients']['mcp-agent']['high_water']+1,'boot':history['boot'],
             'binding':history['current'],'job':second,'operation':'resume'},error=True)
    state=wait(lambda s:job(s,second)['phase']=='Paused' and not s['state'].get('analysis'))
    paused_count=state['state']['live']['sample_count']
    assert paused_count<=ack['state']['live']['sample_count']+1
    assert not state['state']['live']['visual_tracks']
    cancelled=json.loads((data/'captures/model-observations'/f'{cancelled_review}.json').read_text(encoding='utf-8'))
    assert cancelled['pass']['cancelled'] and not cancelled['proposed_observations']
    control('job_control',{'job':second,'operation':'resume'})
    state=wait(lambda s:job(s,second)['phase'] in ['Completed','Failed'])
    assert job(state,second)['phase']=='Completed',state['state'].get('error')
    assert state['state']['live']['sample_count']==4 and not state['state'].get('analysis')
    assert state['state']['live']['visual_tracks']
    assert not state['state']['patches']
    (data/'cards-state.json').write_text(json.dumps(state,indent=2),encoding='utf-8')
    receipts=list((data/'captures/model-observations').glob('*.json'))
    result={'scope':'Owned saved-image fixture; concurrency/publication evidence, not new object accuracy or generalization.',
        'model':model,'automatic_capture_count':16,'maximum_capture_while_first_review_pending':max(overlap),
        'first_review_frame':first_frame,'earlier_cards_keep_original_frame':True,
        'pause_ack_ms':pause_ms,'paused_capture_count':paused_count,'resumed_capture_count':4,
        'no_publication_after_pause':True,'resume_waits_for_old_worker':True,'no_memory_matches_or_writes':True,
        'maximum_status_ms':max(latencies)*1000,'cancelled_reviews':len(cancelled['pass']['reviews']),
        'receipts':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in receipts},
        'binaries':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [binaries/'Reversebolt.exe',binaries/'reversebolt-mcp.exe']}}
    (data/'background-result.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
    print(json.dumps(result),flush=True)
finally:
    (data/'background-trace.json').write_text(json.dumps(trace,indent=2),encoding='utf-8')
    process.terminate()
    try:process.wait(timeout=5)
    except subprocess.TimeoutExpired:process.kill();process.wait(timeout=5)
