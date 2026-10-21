"""Run only inside Anode: owned fixtures test foreground input context through MCP.

This runner never controls a game or the user's main desktop. Existing outputs
are preserved. All injected keys are released and owned processes are closed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import queue
import subprocess
import threading
import time

ROOT=Path(__file__).resolve().parents[1]
CREATE_NO_WINDOW=0x08000000


def read(path):
    try: return json.loads(path.read_text(encoding='utf-8-sig'))
    except (FileNotFoundError,json.JSONDecodeError,PermissionError): return None


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--out',type=Path,required=True)
    parser.add_argument('--bin',type=Path,required=True)
    parser.add_argument('--visible',action='store_true')
    parser.add_argument('--keep-ui',action='store_true')
    args=parser.parse_args()
    out=args.out.resolve();out.mkdir(parents=True,exist_ok=False)
    # Anode's seat-exec supplies this explicit expected child session.
    import ctypes
    session=ctypes.c_ulong()
    assert ctypes.windll.kernel32.ProcessIdToSessionId(os.getpid(),ctypes.byref(session))
    assert session.value==int(os.environ['REVERSEBOLT_TEST_SESSION'])
    assert session.value!=int(os.environ['REVERSEBOLT_PARENT_SESSION'])
    children=[];held=[];fixtures={};engine=mcp=None;counter=0;trace=[]
    def wait(check,timeout=60):
        deadline=time.monotonic()+timeout
        while time.monotonic()<deadline:
            value=check()
            if value: return value
            time.sleep(.05)
        raise AssertionError('Owned input test timed out; preserve artifacts before retrying.')
    def control(name,action,key=None):
        f=fixtures[name];f['seq']+=1
        if action=='focus':
            # Use Anode's foreground handling instead of bypassing Windows focus rules.
            anode=Path(os.environ['LOCALAPPDATA'])/'Programs/Anode/anode.exe'
            listed=subprocess.run([str(anode),'windows','--pid',str(f['process'].pid),'--json'],capture_output=True,text=True,check=True,timeout=20)
            windows=json.loads(listed.stdout)['windows']
            target=next(w for w in windows if w['title']=='Reversebolt input '+name)
            focused=subprocess.run([str(anode),'window',target['windowId'],'focus'],capture_output=True,text=True,check=True,timeout=20)
        payload={'seq':f['seq'],'action':action,'key':key}
        temporary=f['control'].with_suffix('.new')
        temporary.write_text(json.dumps(payload),encoding='utf-8');temporary.replace(f['control'])
        state=wait(lambda:(r if (r:=read(f['state'])) and (r['ack']==f['seq'] or r.get('error')) else None),10)
        assert not state['error'],state
        if action=='focus': assert state['foreground'],state
        if action=='down':held.append((name,key))
        if action=='up':held.remove((name,key))
        return state
    def rpc(method,params):
        nonlocal counter
        counter+=1
        mcp.stdin.write(json.dumps({'jsonrpc':'2.0','id':counter,'method':method,'params':params})+'\n');mcp.stdin.flush()
        deadline=time.monotonic()+90
        while time.monotonic()<deadline:
            value=json.loads(lines.get(timeout=max(.1,deadline-time.monotonic())))
            if value.get('id')==counter:
                assert 'error' not in value,value
                return value['result']
        raise AssertionError('MCP timed out; do not replay an uncertain command.')
    def call(action,arguments=None):
        result=rpc('tools/call',{'name':'reversebolt_'+action,'arguments':arguments or {}})
        assert not result.get('isError'),result
        value=json.loads(result['content'][-1]['text']);assert value['ok'],value.get('error')
        return value
    def job_call(action,arguments):
        status=call('status');history=status['jobs']['history']
        client=history['clients'].get('mcp-agent',{})
        return call(action,{'request_id':client.get('high_water',0)+1,'boot':history['boot'],
                            'binding':history['current'],**arguments})
    def phase(status,job):return status['jobs']['history']['jobs'][str(job)]['phase']
    def wait_phase(job,wanted):
        def check():
            status=call('status');p=phase(status,job)
            assert p not in ['Failed','Interrupted'],status.get('error') or status
            return status if p in wanted else None
        return wait(check,120)
    def polls(status):
        return [p for f in status['state']['live']['frames'] for p in (f.get('input_context') or {}).get('polls',[])]
    def wait_status(predicate,timeout=90):
        def check():
            status=call('status')
            return status if predicate(status) else None
        return wait(check,timeout)
    try:
        for name in ['source','other']:
            f={'control':out/(name+'.control.json'),'state':out/(name+'.state.json'),'seq':0}
            process=subprocess.Popen(['powershell','-NoProfile','-ExecutionPolicy','Bypass','-File',str(ROOT/'scripts/input-fixture.ps1'),
                '-ControlFile',str(f['control']),'-StateFile',str(f['state']),'-Title','Reversebolt input '+name],creationflags=CREATE_NO_WINDOW)
            children.append(process);f['process']=process;fixtures[name]=f
            wait(lambda:read(f['state']),30)
        data=out/'engine'
        engine=subprocess.Popen([str(args.bin/'Reversebolt.exe'),'--data-dir',str(data)]+([] if args.visible else ['--headless']),creationflags=CREATE_NO_WINDOW)
        children.append(engine)
        wait(lambda:(e if (e:=read(data/'endpoint.json')) and e['pid']==engine.pid else None),30)
        mcp=subprocess.Popen([str(args.bin/'reversebolt-mcp.exe'),'--data-dir',str(data)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8',creationflags=CREATE_NO_WINDOW)
        children.append(mcp);lines=queue.Queue()
        threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
        rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'input-context-check','version':'1'}})
        mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
        assert len(rpc('tools/list',{})['tools'])==29
        target=next(p for p in call('list_processes')['state']['processes'] if p['pid']==fixtures['source']['process'].pid)
        control('source','focus');control('source','down',68)
        status=call('attach_process',{**{k:target[k] for k in ['pid','window','created']},'budget_mib':1})
        automatic=status['data']['automatic_observation']['ack']['job']
        status=wait_status(lambda s:any(68 in p['down'] for p in polls(s)))
        assert read(fixtures['source']['state'])['down'].get('68',0)>0
        job_call('job_control',{'job':automatic,'operation':'pause'})
        paused=wait_phase(automatic,['Paused']);trace.append({'phase':'automatic_paused','status':paused})
        control('source','up',68)
        # Resume into a different window's key presses. Nothing from it may enter down[].
        control('other','focus');control('other','down',69)
        resumed=job_call('job_control',{'job':automatic,'operation':'resume'})
        before=paused['state']['live']['sample_count']
        status=wait_status(lambda s:s['state']['live']['sample_count']>=before+2)
        job_call('job_control',{'job':automatic,'operation':'cancel'})
        stopped=wait_phase(automatic,['Cancelled']);trace.append({'phase':'resumed_then_cancelled','status':stopped})
        control('other','up',69)
        assert read(fixtures['other']['state'])['down'].get('69',0)>0
        assert any(p['focus']=='outside_selected' for p in polls(stopped))
        assert not any(69 in p['down'] for p in polls(stopped))
        generations={f['input_context']['generation'] for f in stopped['state']['live']['frames'] if f.get('input_context')}
        assert len(generations)==2,generations
        # Cancelled sampler cannot collect a new key; two captures avoid pre-stop tail ambiguity.
        control('source','focus');control('source','down',70)
        call('capture',{'mode':'visual'});time.sleep(.2)
        after_stop=call('capture',{'mode':'visual'})
        assert after_stop['state']['live']['frames'][-1]['input_context']['polls']==[]
        control('source','up',70)
        # Fresh bounded job samples a different key, then completion revokes its owner.
        control('source','down',71)
        started=job_call('start_observation_job',{'count':3,'interval_ms':300,'mode':'visual','compare':False})
        second=started['data']['ack']['job']
        complete=wait_phase(second,['Completed']);trace.append({'phase':'completed','status':complete})
        assert any(71 in p['down'] for p in polls(complete))
        control('source','up',71)
        control('source','down',72)
        call('capture',{'mode':'visual'});time.sleep(.2)
        after_complete=call('capture',{'mode':'visual'})
        assert after_complete['state']['live']['frames'][-1]['input_context']['polls']==[]
        control('source','up',72)
        frames=complete['state']['live']['frames']
        chosen=[f for f in frames if f.get('input_context') and f['input_context']['polls']][-2:]
        assert len(chosen)==2
        for frame in frames:
            context=frame.get('input_context')
            if context:
                assert context['clock_id']==frame['capture_interval']['clock_id']==complete['state']['live']['investigation_id']
                assert context['requested_end_us']==frame['capture_interval']['end_us']
                assert all(p['begin_us']>=context['requested_begin_us'] and p['end_us']<=context['requested_end_us'] for p in context['polls'])
                assert all(p['focus']=='selected_foreground' or p['down']==[] for p in context['polls'])
        archive=next(data.glob('captures/*/evidence.json'))
        archived=read(archive)
        assert [f.get('input_context') for f in archived['frames']]==[f.get('input_context') for f in after_complete['state']['live']['frames']]
        retrieved=call('read_sequence',{'investigation_id':complete['state']['live']['investigation_id'],'frame_ids':[f['id'] for f in chosen]})
        assert [f['input_context'] for f in retrieved['data']['sequence']['frames']]==[f['input_context'] for f in chosen]
        saved=read(data/'session.json')
        assert [f.get('input_context') for f in saved['live']['frames']]==[f.get('input_context') for f in after_complete['state']['live']['frames']]
        delivered={name:read(f['state']) for name,f in fixtures.items()}
        report={'passed':True,'session':session.value,'engine_pid':engine.pid,'fixture_pids':{n:f['process'].pid for n,f in fixtures.items()},
                'automatic_job':automatic,'completed_job':second,'frames':len(after_complete['state']['live']['frames']),
                'input_generations':len(generations),'selected_keys_observed':[68,71],'foreign_key_excluded':69,
                'cancelled_new_key_excluded':70,'completed_new_key_excluded':72,'read_sequence_preserves_context':True,
                'persistence_preserves_context':True,'delivered':delivered,'trace':trace,
                'binary_sha256':{n:hashlib.sha256((args.bin/n).read_bytes()).hexdigest() for n in ['Reversebolt.exe','reversebolt-mcp.exe']}}
        (out/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
        print(json.dumps({k:v for k,v in report.items() if k not in ['trace','delivered']}),flush=True)
        if args.keep_ui:
            (out/'ui-ready.json').write_text(json.dumps({'engine_pid':engine.pid,'data':str(data)}),encoding='utf-8')
            wait(lambda:(out/'close-ui').exists(),300)
        archived_sha=hashlib.sha256(archive.read_bytes()).hexdigest()
        engine.terminate();engine.wait(timeout=10)
        engine=subprocess.Popen([str(args.bin/'Reversebolt.exe'),'--headless','--data-dir',str(data)],creationflags=CREATE_NO_WINDOW)
        children.append(engine)
        wait(lambda:(e if (e:=read(data/'endpoint.json')) and e['pid']==engine.pid else None),30)
        restarted=call('status')
        assert restarted['state']['live'] is None
        assert all(j['phase'] in ['Completed','Cancelled','Interrupted'] for j in restarted['jobs']['history']['jobs'].values())
        assert hashlib.sha256(archive.read_bytes()).hexdigest()==archived_sha
        report.update(restart_revokes_active_target=True,archived_context_unchanged_after_restart=True,
                      archive_sha256=archived_sha,restarted_engine_pid=engine.pid)
        (out/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
        print(json.dumps({'restart_check':True,'archive_sha256':archived_sha}),flush=True)
    finally:
        for name,key in list(held):
            try: control(name,'up',key)
            except Exception: pass
        for name,f in fixtures.items():
            if f['process'].poll() is None:
                try:control(name,'close')
                except Exception:pass
        for process in reversed(children):
            if process.poll() is None:process.terminate()
            try:process.wait(timeout=5)
            except subprocess.TimeoutExpired:process.kill();process.wait()


if __name__=='__main__':main()
