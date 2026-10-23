"""Fresh discovery and one durable revalidation; no field-specific selection."""
import ctypes as C
from ctypes import wintypes as W
import hashlib,json,os,queue,shutil,subprocess,sys,threading,time,uuid
from pathlib import Path
OUT=Path(sys.argv[1]).resolve();PID=int(sys.argv[2]);BIN=OUT/'bin';DATA=OUT/'data'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json');old=read(OUT/'source-test.json')
assert sha(Path(__file__))==frozen['sources']['research/runtime-recovery/live.py']
for n,h in frozen['binaries'].items():assert sha(BIN/n)==h
for n,h in frozen['historical'].items():assert sha(OUT/n)==h
session=W.DWORD();assert C.WinDLL('kernel32').ProcessIdToSessionId(os.getpid(),C.byref(session)) and session.value==3
mcp=None;seq=0;lines=queue.Queue();active_test=None;change=None
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
    raise RuntimeError('Unknown outcome; do not repeat this request')
def call(name,args=None,required=True):
    raw=rpc('tools/call',{'name':'reversebolt_'+name,'arguments':args or {}})
    response=json.loads(next(c['text'] for c in raw['content'] if c['type']=='text'))
    if name!='status':save(f'rpc-{seq:05}-{name}',{'args':args or {},'response':response})
    if required:assert response['ok'],(name,response.get('error'))
    return response
def status():return call('status')
def observe(test):
    for _ in range(6):
        test=call('observe_runtime_test',{'test_id':test['id'],'after_revision':test['revision'],'perform_action':True})['data']
        if test['phase']=='stopped':break
    return test
save('started',{'runner_pid':os.getpid(),'game_pid':PID,'engine_pid':read(DATA/'endpoint.json')['pid']})
try:
    mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,
        stderr=(OUT/'mcp.stderr').open('x'),text=True,encoding='utf-8',creationflags=subprocess.CREATE_NO_WINDOW)
    threading.Thread(target=lambda:[lines.put(s) for s in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'runtime-recovery-revalidation','version':'1'}})
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
    assert status()['state']['live'] is None
    target=next(p for p in call('list_processes')['state']['processes'] if p['pid']==PID);save('target',target)
    attached=call('attach_process',{k:target[k] for k in ['pid','created','window']}|{'budget_mib':1,'auto_observe':False})
    history=attached['jobs']['history']
    call('start_runtime_job',{'request_id':1,'boot':history['boot'],'binding':history['current'],
        'task':{'operation':'discover'},'local_model':None,'observe_play':False})
    deadline=time.monotonic()+1000;progress=0
    while time.monotonic()<deadline:
        current=status();jobs=current['jobs']['history']['jobs']
        if time.monotonic()>=progress:
            print(json.dumps({'phase':'discovery','jobs':[j['phase'] for j in jobs.values()]}),flush=True);progress=time.monotonic()+20
        if jobs and all(j['phase'] in ['Completed','Failed','Cancelled','Interrupted'] for j in jobs.values()):break
        time.sleep(.5)
    else:raise RuntimeError('Discovery remains pending; recover its existing job')
    save('discovery',current)
    assert all(j['phase']=='Completed' for j in jobs.values()),'Discovery failed'
    runtime=current['state']['live']['runtime'];assert current['state']['live']['sample_count']==0
    shutil.copy2(Path(runtime['catalog_source']['path']),OUT/'current-report.json')
    with (OUT/'recovery.stdout').open('x') as stdout,(OUT/'recovery.stderr').open('x') as stderr:
        child=subprocess.Popen([str(BIN/'runtime_recovery.exe'),str(OUT/'source-report.json'),old['candidate']['id'],
            str(OUT/'current-report.json'),str(OUT/'recovery.json')],stdout=stdout,stderr=stderr,creationflags=subprocess.CREATE_NO_WINDOW)
        save('recovery-process',{'pid':child.pid});code=child.wait();save('recovery-terminal',{'pid':child.pid,'exit_code':code})
    assert code==0
    recovered=read(OUT/'recovery.json');assert not recovered['same_process']
    if recovered['status']!='matched':
        save('result',{'finished':True,'status':recovered['status'],'gameplay_writes':0,'model_calls':0})
    else:
        candidate=recovered['candidate'];assert candidate['locator']['scope']['pid']==PID and candidate['id']!=old['candidate']['id']
        call('show_runtime_values',{'catalog_id':runtime['catalog_id'],'after_revision':runtime['revision'],'ids':[candidate['id']]})
        live=status()['state']['live'];active_test=str(uuid.uuid4());change=str(uuid.uuid4())
        args={'test_id':active_test,'change_id':change,'investigation_id':live['investigation_id'],'catalog_id':live['runtime']['catalog_id'],
            'after_revision':live['runtime']['revision'],'candidate_id':candidate['id'],'hypothesis':old['hypothesis'],'value':old['value'],'actions':old['actions']}
        save('new-plan',args)
        print(json.dumps({'phase':'new_baseline','path':candidate['id']}),flush=True)
        test=observe(call('prepare_runtime_test',args)['data'])
        if test['phase']!='ready_to_apply':
            save('result',{'finished':True,'status':'baseline_unavailable','test':test,'gameplay_writes':0,'model_calls':0})
        else:
            live=status()['state']['live']
            trial=call('set_runtime_value',{'change_id':change,'investigation_id':live['investigation_id'],'catalog_id':live['runtime']['catalog_id'],
                'after_revision':live['runtime']['revision'],'candidate_id':candidate['id'],'value':old['value']})['data']
            assert trial['phase']=='written'
            print(json.dumps({'phase':'changed'}),flush=True)
            test=observe(next(t for t in status()['state']['runtime_tests'] if t['id']==active_test))
            restored=call('restore_runtime_change',{'change_id':change})['data'];assert restored['phase']=='restored'
            test=next(t for t in status()['state']['runtime_tests'] if t['id']==active_test)
            if test['phase']=='restored':test=observe(test)
            comparison=call('read_runtime_test',{'test_id':active_test})['data'];save('comparison',comparison)
            save('result',{'finished':True,'status':test['phase'],'assessment':test.get('assessment'),'gameplay_write_attempts':1,
                'restored':True,'model_calls':0,'manually_reviewed_measurement_images':0})
            print(json.dumps({'phase':test['phase'],'assessment':test.get('assessment')}),flush=True)
        active_test=None
except BaseException as error:
    save('failure',{'error':repr(error)})
    raise
finally:
    if mcp is not None and mcp.poll() is None:
        try:
            current=status()
            if active_test and any(t['id']==active_test and t['active'] for t in current['state']['runtime_tests']):call('stop_runtime_test',{'test_id':active_test})
            # Only this evaluator's known-written trial; unknown writes are never retried.
            for t in status()['state']['runtime_trials']:
                if t['id']==change and t['phase']=='written':call('restore_runtime_change',{'change_id':change})
            final=status();save('final-state',final)
            if any(t['phase'] not in ['restored','prepared','cancelled'] for t in final['state']['runtime_trials']):save('needs-recovery',final['state']['runtime_trials'])
        except BaseException as error:save('cleanup-error',{'error':repr(error)})
        mcp.stdin.close()
        try:mcp.wait(timeout=15)
        except subprocess.TimeoutExpired:mcp.terminate();mcp.wait(timeout=15)
    save('child',{'pid':mcp.pid if mcp else None,'exit_code':mcp.poll() if mcp else None})
