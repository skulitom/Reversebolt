import hashlib,json,subprocess,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=Path(sys.argv[1]).resolve();OUT.mkdir(parents=True,exist_ok=False)
BIN=ROOT/'target/debug';helper=BIN/'reversebolt_runtime.dll';digest=hashlib.sha256(helper.read_bytes()).hexdigest()
log=(OUT/'host.jsonl').open('w')
host=subprocess.Popen([str(ROOT/'target/runtime-objects/debug/object-host.exe'),str(ROOT/'target/runtime-traversal/debug/reversebolt_traversal_fixture.dll'),str(OUT/'identity.json'),str(OUT/'stop')],stdout=log,stderr=subprocess.STDOUT)
receipts=[];expected_pauses=0
try:
    deadline=time.monotonic()+10
    while not (OUT/'identity.json').exists():
        assert host.poll() is None and time.monotonic()<deadline
        time.sleep(.05)
    identity=json.loads((OUT/'identity.json').read_text())
    base=[str(BIN/'reversebolt-runtime-loader.exe'),str(identity['pid']),identity['created'],str(identity['session']),str(helper),digest]
    def call(label,extra=(),success=True):
        path=OUT/f'{label}.json';result=subprocess.run(base+[str(path)]+list(extra),capture_output=True,text=True,timeout=130)
        (OUT/f'{label}.stdout.jsonl').write_text(result.stdout);(OUT/f'{label}.stderr.jsonl').write_text(result.stderr)
        receipts.append({'label':label,'exit':result.returncode})
        assert (result.returncode==0)==success,result.stderr
        if result.stdout:
            receipt=json.loads(result.stdout.splitlines()[-1]);assert receipt['helper_absent']
        return json.loads(path.read_text()) if path.exists() else None
    assert call('malformed-root',['oops','0'*64],False) is None
    first=call('initial');fingerprint=first['root_catalog_sha256']
    second=call('continuation',['64',fingerprint])
    rejected=call('wrong-fingerprint',['64','0'*64],False)
    assert 'root catalog changed' in rejected['error']
    for report,start in [(first,0),(second,64)]:
        assert report['root_start']==start and report['next_root']==130 and report['root_scan_complete']
        assert not report['graph_complete'] and report['catalog_classes']==7
        assert len(report['warmups'])==2 and all(w['added_classes']>0 for w in report['warmups'])
        expected_pauses+=len(report['snapshots'])+len(report['warmups'])
        classes={c['id']:c for c in report['classes']}
        assert [s['root_start'] for s in report['snapshots']]==list(range(start,130,64))
        for w in report['warmups']:
            s=w['discarded_snapshot'];assert s['helper_allocations_while_paused']==s['helper_deallocations_while_paused']==0
        for s in report['snapshots']:
            assert len(s['nodes'])==8 and max(n['depth'] for n in s['nodes'])==3
            assert not s['time_or_read_budget_reached'] and not s['node_capacity_reached']
            assert s['helper_allocations_while_paused']==s['helper_deallocations_while_paused']==0
            assert all(n['class'] is not None for n in s['nodes'])
            values={};edges={};elements={}
            for r in s['reads']:
                assert r['read_ok']
                if r['element_index'] is not None:elements.setdefault(r['node'],[]).append(r);continue
                f=classes[r['declaring_class']]['fields'][r['field']]
                if f['reference']:edges[(r['node'],f['name'])]=r['reference']['id']
                else:values.setdefault(r['node'],{})[f['name']]=r['value']
            actors=[(n,v) for n,v in values.items() if 'Identity' in v];assert len(actors)==3
            actors.sort(key=lambda pair:pair[1]['Identity']);head,head_value=actors[0]
            for offset,(node,v) in enumerate(actors):
                assert v['Identity']==head_value['Identity']+offset
                assert v['Counter']==(head_value['Identity']//10)*17+offset
                assert float(v['HiddenScale'])==v['Identity']+.375 and float(v['Reserve'])==4.25
                if offset:assert edges[(node,'Peer')]==head
            boxed=edges[(head,'Peer')];assert float(values[boxed]['Coefficient'])==3.5
            ref_array=edges[(boxed,'Items')];primitive=edges[(boxed,'Numbers')]
            assert len(elements[ref_array])==64
            assert [r['reference']['id'] for r in elements[ref_array]]==[actors[1+i%2][0] for i in range(64)]
            assert [float(r['value']) for r in elements[primitive]]==[1.25,-2.5,3.75]
            strings={t['node']:t for t in s['strings']}
            assert strings[edges[(boxed,'Label')]]['text']=='Speed'
            long=strings[edges[(boxed,'LongLabel')]];assert long['text']=='X'*128 and long['truncated'] and long['length']==200
            arr=next(a for a in s['arrays'] if a['node']==ref_array);assert arr['length']==70 and arr['elements_read']==64 and arr['truncated']
finally:
    (OUT/'stop').touch()
    try:host.wait(timeout=5)
    except subprocess.TimeoutExpired:host.terminate();host.wait(timeout=5)
    log.close()
    (OUT/'execution.json').write_text(json.dumps({'host_pid':host.pid,'host_exit':host.returncode,'receipts':receipts,'expected_pauses':expected_pauses},indent=2))
balance=json.loads((OUT/'host.jsonl').read_text());assert host.returncode==0 and balance=={'stops':expected_pauses,'starts':expected_pauses}
print(json.dumps({'passed':True,'balanced_pauses':balance,'receipts':receipts}))
