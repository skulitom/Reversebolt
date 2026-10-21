import copy, hashlib, json, os, subprocess, sys, time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=Path(sys.argv[1]).resolve();OUT.mkdir(parents=True,exist_ok=False)
BIN=OUT.parent/'bin';helper=BIN/'reversebolt_runtime.dll'
digest=hashlib.sha256(helper.read_bytes()).hexdigest()
control=OUT/'mode.txt';control.write_text('0');ack=control.with_suffix('.ack')
env=dict(os.environ,REVERSEBOLT_PATH_FIXTURE_CONTROL=str(control))
log=(OUT/'host.jsonl').open('w');calls=[];pauses=0
host=subprocess.Popen([str(BIN/'object-host.exe'),str(BIN/'reversebolt_path_fixture.dll'),str(OUT/'identity.json'),str(OUT/'stop')],env=env,stdout=log,stderr=subprocess.STDOUT)

def mode(value):
    control.write_text(str(value));deadline=time.monotonic()+5
    while True:
        try: ready=ack.read_text()==str(value)
        except FileNotFoundError: ready=False
        if ready:return
        assert host.poll() is None and time.monotonic()<deadline
        time.sleep(.03)

try:
    deadline=time.monotonic()+10
    while not (OUT/'identity.json').exists():
        assert host.poll() is None and time.monotonic()<deadline
        time.sleep(.05)
    identity=json.loads((OUT/'identity.json').read_bytes())
    base=[str(BIN/'reversebolt-runtime-loader.exe'),str(identity['pid']),identity['created'],str(identity['session']),str(helper),digest]
    def call(label,paths=None,success=True):
        global pauses
        output=OUT/(label+'.json');extra=[]
        if paths is not None:
            path_file=OUT/(label+'.paths.json');path_file.write_text(json.dumps(paths))
            extra=['--paths-file',str(path_file)]
        result=subprocess.run(base+[str(output)]+extra,capture_output=True,text=True,timeout=130)
        (OUT/(label+'.stdout.jsonl')).write_text(result.stdout)
        (OUT/(label+'.stderr.jsonl')).write_text(result.stderr)
        calls.append({'label':label,'exit':result.returncode})
        assert (result.returncode==0)==success,result.stderr
        if result.stdout:
            assert json.loads(result.stdout.splitlines()[-1])['helper_absent']
        if not output.exists():return None
        report=json.loads(output.read_bytes())
        if paths is not None:
            assert not report['edit_authority'] and report['operation']=='refresh_runtime_paths'
            assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in report['pauses'])
            pauses+=len(report['pauses'])
        else:
            assert report['schema']==4
            shots=report['snapshots']+[w['discarded_snapshot'] for w in report['warmups']]
            assert all(w['added_classes']>0 for w in report['warmups'])
            assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in shots)
            pauses+=len(shots)
        return report
    discovery=call('discovery')
    converted=subprocess.run([str(BIN/'reversebolt-runtime-candidates.exe'),str(OUT/'discovery.json'),str(OUT/'candidates.json'),'4096'],capture_output=True,text=True,timeout=30)
    (OUT/'candidates.stdout.jsonl').write_text(converted.stdout);(OUT/'candidates.stderr.jsonl').write_text(converted.stderr)
    assert converted.returncode==0,converted.stderr
    candidates=json.loads((OUT/'candidates.json').read_bytes())
    def child(c):
        return c['locator']['root']['field_id']==0 and any(s['member']=={'kind':'element','index':1} for s in c['locator']['steps'])
    chosen=[next(c for c in candidates if c['name']==name and child(c)) for name in ['Identity','HiddenScale','Reserve']]
    chosen.append(next(c for c in candidates if c['name']=='Element 2' and c['locator']['root']['field_id']==0))
    paths=[c['locator'] for c in chosen]
    (OUT/'selected.json').write_text(json.dumps(chosen,indent=2))
    assert all('address' not in json.dumps(p) for p in paths)
    def valid(report,reordered=False):
        checks=report['checks'];assert all(c['status']=='read' for c in checks)
        value=checks[0]['value'];assert value%10==(1 if reordered else 2)
        assert float(checks[1]['value'])==value+.375
        assert float(checks[2]['value'])==4.25
        assert float(checks[3]['value'])==3.75
        assert [c['path_id'] for c in checks]==[c['id'] for c in chosen]
        return {'identity':value,'address':checks[0]['field_address']}
    observations=[]
    for i in range(3):
        time.sleep(.13)
        observations.append(valid(call('refresh-'+str(i),paths)))
    assert len({r['identity'] for r in observations})==3
    assert any(a['address']==b['address'] and a['identity']!=b['identity'] for i,a in enumerate(observations) for b in observations[i+1:])
    mode(2);reordered=valid(call('reordered',paths),True)
    mode(1);assert all(c['status']=='null_reference' for c in call('null-root',paths)['checks'])
    mode(3);assert all(c['status']=='type_changed' for c in call('generic-type-changed',paths)['checks'])
    mode(4);bounds=call('array-shortened',paths)['checks'];assert [c['status'] for c in bounds]==['array_bounds']*3+['read']
    mode(5);assert all(c['status']=='type_changed' for c in call('layout-changed',paths)['checks'])
    mode(0);valid(call('restored-structure',paths))
    wrong=copy.deepcopy(paths);wrong[0]['scope']['created']='0'*16
    assert call('wrong-process',wrong,False) is None
    wrong=copy.deepcopy(paths[:1]);wrong[0]['root']['declaring']['layout_sha256']='0'*64
    assert call('wrong-root-layout',wrong)['checks'][0]['status']=='root_unavailable'
    wrong=copy.deepcopy(paths[:1]);wrong[0]['steps'][-1]['member']['field']['field_id']=99999
    assert call('missing-field',wrong)['checks'][0]['status']=='field_unavailable'
    (OUT/'summary.json').write_text(json.dumps({'passed':True,'observations':observations,'reordered':reordered,'pauses':pauses,'calls':calls,'helper_sha256':digest},indent=2))
finally:
    (OUT/'stop').touch()
    try:host.wait(timeout=5)
    except subprocess.TimeoutExpired:host.terminate();host.wait(timeout=5)
    log.close()
    (OUT/'execution.json').write_text(json.dumps({'host_pid':host.pid,'host_exit':host.returncode,'calls':calls,'expected_pauses':pauses},indent=2))
balance=json.loads((OUT/'host.jsonl').read_bytes());assert host.returncode==0 and balance=={'starts':pauses,'stops':pauses}
print(json.dumps({'passed':True,'balanced_pauses':balance,'calls':calls}))
