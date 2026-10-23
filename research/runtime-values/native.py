"""Owned-process ABI and restoration checks. No GUI, model calls or game writes."""
import copy, hashlib, json, os, subprocess, sys, time
from pathlib import Path
OUT=Path(sys.argv[1]).resolve();BIN=OUT/'bin'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
save=lambda p,v:p.write_text(json.dumps(v,indent=2))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json')
assert sha(Path(__file__))==frozen['sources']['research/runtime-values/native.py']
for name,digest in frozen['binaries'].items():assert sha(BIN/name)==digest

def run_case(legacy=False):
    case=OUT/('legacy' if legacy else 'values');case.mkdir()
    control=case/'mode.txt';control.write_text('0');ack=control.with_suffix('.ack')
    env=dict(os.environ,**{('REVERSEBOLT_PATH_FIXTURE_CONTROL' if legacy else 'REVERSEBOLT_VALUE_FIXTURE_CONTROL'):str(control)})
    log=(case/'host.jsonl').open('w');calls=[];pauses=0;max_pause=0
    host=subprocess.Popen([str(BIN/'object-host.exe'),str(BIN/('reversebolt_path_fixture.dll' if legacy else 'reversebolt_value_fixture.dll')),
        str(case/'identity.json'),str(case/'stop')],env=env,stdout=log,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW)
    def command(args,label):
        p=subprocess.run(list(map(str,args)),capture_output=True,text=True,timeout=130,creationflags=subprocess.CREATE_NO_WINDOW)
        (case/(label+'.stdout')).write_text(p.stdout);(case/(label+'.stderr')).write_text(p.stderr)
        assert p.returncode==0,(label,p.stderr,p.stdout)
        return p
    try:
        end=time.monotonic()+10
        while not (case/'identity.json').exists():
            assert host.poll() is None and time.monotonic()<end
            time.sleep(.03)
        identity=read(case/'identity.json')
        base=[BIN/'reversebolt-runtime-loader.exe',identity['pid'],identity['created'],identity['session'],BIN/'reversebolt_runtime.dll',frozen['binaries']['reversebolt_runtime.dll']]
        def call(label,paths=None,patch=None):
            nonlocal pauses,max_pause
            extra=[]
            if paths is not None:
                save(case/(label+'.paths.json'),paths)
                extra=['--paths-file',case/(label+'.paths.json')]
            if patch:extra=['--patch-file',patch]
            p=command(base+[case/(label+'.json')]+extra,label)
            receipt=json.loads(p.stdout.splitlines()[-1]);assert receipt['helper_absent'] and receipt['inspection_exit']==0
            report=read(case/(label+'.json'))
            samples=report.get('pauses',report.get('snapshots',[])+[w['discarded_snapshot'] for w in report.get('warmups',[])])
            assert samples and all(s['helper_allocations_while_paused']==s['helper_deallocations_while_paused']==0 for s in samples)
            pauses+=len(samples);max_pause=max(max_pause,max(s['stop_to_resume_us'] for s in samples))
            calls.append({'label':label,'helper_absent':True,'pauses':len(samples)})
            return report
        report=call('discovery');assert report['schema']==5 and report['value_type_metadata']==(not legacy)
        command([BIN/'reversebolt-runtime-candidates.exe',case/'discovery.json',case/'candidates.json',4096],'candidates')
        candidates=read(case/'candidates.json')
        if legacy:
            checks=call('refresh',[c['locator'] for c in candidates[:4]])['checks']
            assert len(checks)==4 and all(c['status']=='read' for c in checks)
        else:
            def field_ids(c):return [s['member'].get('field',{}).get('field_id') for s in c['locator']['steps']]
            def pick(ids,root=0,element=None):
                result=[c for c in candidates if c['locator']['root']['field_id']==root and field_ids(c)==ids and
                    (element is None or any(s['member']=={'kind':'element','index':element} for s in c['locator']['steps']))]
                assert len(result)==1,(ids,root,element,len(result));return result[0]
            pose=pick([0,0]);ro=pick([1,0]);wrapped=pick([5,0,0]);array=pick([2,None,0],element=1)
            static=pick([0],1);static_ro=pick([0],2);phase=pick([3]);wide=pick([4]);enum_element=pick([6,None],element=2)
            assert pose['value']=='3.25' and wrapped['value']=='3.25' and array['value']=='12.25'
            assert phase['value']==7 and phase['type_code']==5
            assert wide['value']==str(2**64-3) and wide['type_code']==11 and len(wide['bytes'])==8
            assert enum_element['value']==255 and enum_element['type_code']==5
            assert not any(field_ids(c)[0]==7 for c in candidates if c['locator']['root']['field_id']==0)
            assert all(not c['edit_authority'] for c in candidates)
            selected=[pose,ro,wrapped,array,static,static_ro,phase,wide,enum_element]
            checks=call('refresh',[c['locator'] for c in selected])['checks']
            assert all(r['status']=='read' and r['bytes']==c['bytes'] for r,c in zip(checks,selected))
            # Distinct nested structures can start at the same address. They still have distinct nodes/types.
            shot=report['snapshots'][0];inline=[n for n in shot['nodes'] if n['inline']]
            assert any(a['id']!=b['id'] and a['address']==b['address'] and a['class']!=b['class'] for a in inline for b in inline)
            def trial(c,label,desired,readonly=False):
                locator=case/(label+'.locator.json');save(locator,c['locator'])
                before=call(label+'-before',[c['locator']])
                patch=case/(label+'.patch.json')
                command([BIN/'reversebolt-runtime-patch.exe','prepare',locator,case/(label+'-before.json'),patch,desired],label+'-prepare')
                changed=call(label+'-write',patch=patch)
                if readonly:
                    assert changed['patch']['reason']=='readonly_field' and not changed['patch']['attempted'],changed
                else:
                    assert changed['patch']['state']=='written',changed
                    restore=case/(label+'.restore.json')
                    command([BIN/'reversebolt-runtime-patch.exe','restore',patch,case/(label+'-write.json'),restore],label+'-restore-prepare')
                    restored=call(label+'-restore',patch=restore)
                    assert restored['patch']['state']=='written',restored
                after=call(label+'-after',[c['locator']])
                assert after['checks'][0]['bytes']==before['checks'][0]['bytes']
            for c,label,desired,readonly in [(pose,'pose','4.25',False),(wrapped,'nested','4.25',False),
                (array,'array','13.25',False),(static,'static','4.25',False),(phase,'enum','8',False),
                (wide,'wide-enum',str(2**64-4),False),(ro,'readonly','4.25',True),(static_ro,'static-readonly','4.25',True)]:trial(c,label,desired,readonly)
            # Readonly reference slots do not imply a readonly referenced object.
            target=copy.deepcopy(pick([0,2,0]));target['locator']['steps'][0]['member']['field']['field_id']=1
            trial(target,'readonly-reference','20')
            def mode(n):
                control.write_text(str(n));end=time.monotonic()+5
                while not (ack.exists() and ack.read_text()==str(n)):
                    assert host.poll() is None and time.monotonic()<end;time.sleep(.03)
            mode(1);assert call('null',[pose['locator']])['checks'][0]['status']=='null_reference'
            mode(2);assert call('bounds',[array['locator']])['checks'][0]['status']=='array_bounds'
            mode(3);assert call('layout',[pose['locator'],wrapped['locator']])['checks'][0]['status']=='type_changed'
            mode(0);assert call('recovered',[pose['locator']])['checks'][0]['status']=='read'
            final=call('all-restored',[c['locator'] for c in selected])['checks']
            assert all(r['status']=='read' and r['bytes']==c['bytes'] for r,c in zip(final,selected))
        save(case/'result.json',{'passed':True,'candidates':len(candidates),'calls':calls,'pauses':pauses,'max_stop_to_resume_us':max_pause})
    finally:
        (case/'stop').touch()
        try:host.wait(timeout=5)
        except subprocess.TimeoutExpired:host.terminate();host.wait(timeout=5)
        log.close();save(case/'execution.json',{'host_pid':host.pid,'host_exit':host.returncode,'calls':calls,'pauses':pauses})
    assert host.returncode==0 and read(case/'host.jsonl')=={'starts':pauses,'stops':pauses}
    print(json.dumps({'case':case.name,'passed':True,'calls':len(calls),'pauses':pauses,'max_pause_us':max_pause}),flush=True)
run_case()
run_case(True)
