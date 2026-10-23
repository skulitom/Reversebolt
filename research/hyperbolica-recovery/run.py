"""Recover one genuine product record in a new process; no game-specific field or inference."""
import ctypes as C
from ctypes import wintypes as W
import hashlib,json,os,queue,shutil,subprocess,sys,threading,time,uuid
from pathlib import Path

ROOT=Path(sys.argv[1]).resolve();PID=int(sys.argv[2]);PHASE='recover'
OUT=ROOT/'recover';OUT.mkdir();DATA=ROOT/'data'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(ROOT/'frozen.json');BIN=Path(frozen['bin']);PREVIOUS=Path(frozen['previous'])
assert sha(Path(__file__))==frozen['sources']['research/hyperbolica-recovery/run.py']
for name,h in frozen['binaries'].items():assert sha(BIN/name)==h,name
for name,h in frozen['historical'].items():assert sha(PREVIOUS/name)==h,name
session=W.DWORD();assert C.WinDLL('kernel32').ProcessIdToSessionId(os.getpid(),C.byref(session))
assert session.value==frozen['seat_session']
mcp=None;seq=0;lines=queue.Queue();active_test=change=None

def save(name,value):
    with (OUT/(name+'.json')).open('x',encoding='utf-8') as f:json.dump(value,f,indent=2)

def rpc(method,params):
    global seq
    seq+=1;mcp.stdin.write(json.dumps({'jsonrpc':'2.0','id':seq,'method':method,'params':params})+'\n');mcp.stdin.flush()
    deadline=time.monotonic()+180
    while time.monotonic()<deadline:
        value=json.loads(lines.get(timeout=max(.1,deadline-time.monotonic())))
        if value.get('id')==seq:
            assert 'error' not in value,value
            return value['result']
    raise RuntimeError('Unknown response; do not repeat the request')

def call(name,args=None):
    raw=rpc('tools/call',{'name':'reversebolt_'+name,'arguments':args or {}})
    response=json.loads(next(c['text'] for c in raw['content'] if c['type']=='text'))
    if name!='status':save(f'rpc-{seq:05}-{name}',{'args':args or {},'response':response})
    assert response['ok'],(name,response.get('error'))
    return response

def status():return call('status')

def observe(test):
    for _ in range(6):
        test=call('observe_runtime_test',{'test_id':test['id'],'after_revision':test['revision'],'perform_action':True})['data']
        if test['phase']=='stopped':break
    return test

try:
    save('started',{'runner_pid':os.getpid(),'engine_pid':read(DATA/'endpoint.json')['pid'],'game_pid':PID,'phase':PHASE})
    mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,
        stderr=(OUT/'mcp.stderr').open('x'),text=True,encoding='utf-8',creationflags=subprocess.CREATE_NO_WINDOW)
    threading.Thread(target=lambda:[lines.put(s) for s in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'product-native-recovery','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    initial=status();save('initial',initial);assert initial['state']['live'] is None
    prior=read(PREVIOUS/'result.json')
    assert prior['comparison']['phase']=='complete' and prior['comparison']['assessment']['status']=='suppression_observed'
    assert read(PREVIOUS/'cleanup.json')['owned_processes_exited']
    old=read(PREVIOUS/'comparison.json')['test']
    assert old['recovery_recipe'] and old==read(DATA/'runtime-tests'/f"{old['id']}.json")
    assert len(initial['state']['runtime_tests'])==len(initial['state']['runtime_trials'])==1
    assert initial['jobs']['history']['boot']>read(PREVIOUS/'final-state.json')['jobs']['history']['boot']
    target=next(p for p in call('list_processes')['state']['processes'] if p['pid']==PID);save('target',target)
    assert target['session']==frozen['seat_session']
    if PHASE=='recover':assert target['created'].lower()!=old['scope']['created'].lower() and PID!=old['scope']['pid']
    attached=call('attach_process',{k:target[k] for k in ['pid','created','window']}|{'budget_mib':1,'auto_observe':False})
    history=attached['jobs']['history'];job_id=str(history['next_job'])
    call('start_runtime_job',{'request_id':history['clients'].get('mcp-agent',{}).get('high_water',0)+1,
        'boot':history['boot'],'binding':history['current'],'task':{'operation':'discover'},'local_model':None,'observe_play':False})
    deadline=time.monotonic()+1000;progress=0
    while time.monotonic()<deadline:
        current=status();job=current['jobs']['history']['jobs'].get(job_id)
        if time.monotonic()>=progress:
            print(json.dumps({'phase':PHASE,'operation':'discovery','job':job['phase'] if job else None}),flush=True);progress=time.monotonic()+20
        if job and job['phase'] in ['Completed','Failed','Cancelled','Interrupted']:break
        time.sleep(.5)
    else:raise RuntimeError('Discovery remains pending; recover its existing job')
    save('discovery',current);assert job['phase']=='Completed','Discovery failed'
    live=current['state']['live'];runtime=live['runtime'];assert live['sample_count']==0 and not live['frames']
    shutil.copy2(Path(runtime['catalog_source']['path']),OUT/'current-report.json')
    hints=runtime['recovered_controls'];assert len(hints)==1 and hints[0]['source_test_id']==old['id'] and hints[0]['needs_revalidation']
    candidate=next(c['candidate'] for c in runtime['cards'] if c['candidate']['id']==hints[0]['candidate_id'])
    assert candidate['locator']['scope']['pid']==PID and candidate['id']!=old['candidate']['id']
    recovered=call('recover_runtime_control',{'test_id':old['id'],'catalog_id':runtime['catalog_id'],'after_revision':runtime['revision']})['data']
    save('recovery',recovered);assert recovered['shown'] and not recovered['edit_authority']
    assert recovered['recovery']['candidate']['id']==candidate['id']
    plan=recovered['preparation']
    active_test=str(uuid.uuid4());change=str(uuid.uuid4());plan|={'test_id':active_test,'change_id':change}
    save('new-plan',plan)
    print(json.dumps({'phase':PHASE,'operation':'new_baseline','candidate_id':candidate['id']}),flush=True)
    prepared=call('prepare_runtime_test',plan)['data'];assert prepared['remembered_route']['recipe_sha256']
    assert read(DATA/'runtime-tests'/f'{active_test}.json')['recovery_recipe']
    test=observe(prepared)
    if test['phase']!='ready_to_apply':
        save('result',{'status':'baseline_unavailable','test':test,'gameplay_writes':0,'model_calls':0})
    else:
        live=status()['state']['live']
        trial=call('set_runtime_value',{'change_id':change,'investigation_id':live['investigation_id'],'catalog_id':live['runtime']['catalog_id'],
            'after_revision':live['runtime']['revision'],'candidate_id':candidate['id'],'value':plan['value']})['data'];assert trial['phase']=='written'
        print(json.dumps({'phase':PHASE,'operation':'changed'}),flush=True)
        test=observe(next(t for t in status()['state']['runtime_tests'] if t['id']==active_test))
        restored=call('restore_runtime_change',{'change_id':change})['data'];assert restored['phase']=='restored'
        test=next(t for t in status()['state']['runtime_tests'] if t['id']==active_test)
        if test['phase']=='restored':test=observe(test)
        comparison=call('read_runtime_test',{'test_id':active_test})['data'];save('comparison',comparison)
        save('result',{'status':test['phase'],'assessment':test.get('assessment'),'gameplay_write_attempts':1,'restored':True,'model_calls':0})
        print(json.dumps({'phase':PHASE,'operation':test['phase'],'assessment':test.get('assessment')}),flush=True)
    if not test['active']:active_test=None
except BaseException as error:
    save('failure',{'error':repr(error)});raise
finally:
    if mcp is not None and mcp.poll() is None:
        try:
            current=status()
            if active_test and any(t['id']==active_test and t['active'] for t in current['state']['runtime_tests']):call('stop_runtime_test',{'test_id':active_test})
            for t in status()['state']['runtime_trials']:
                if t['id']==change and t['phase']=='written':call('restore_runtime_change',{'change_id':change})
            final=status();save('final-state',final)
            if any(t['phase'] not in ['restored','prepared','cancelled'] for t in final['state']['runtime_trials']):save('needs-recovery',final['state']['runtime_trials'])
        except BaseException as error:save('cleanup-error',{'error':repr(error)})
        mcp.stdin.close()
        try:mcp.wait(timeout=15)
        except subprocess.TimeoutExpired:mcp.terminate();mcp.wait(timeout=15)
    # The independent engine job remains alive until external cleanup verifies settlement.
    save('cleanup',{'mcp_pid':mcp.pid if mcp else None,'mcp_exit_code':mcp.poll() if mcp else None,
        'engine_closed_by_runner':False})
