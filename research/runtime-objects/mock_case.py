import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time
ROOT=Path(__file__).resolve().parents[2]
BIN=ROOT/'target/runtime-objects/debug'
OUT=Path(sys.argv[1]).resolve();OUT.mkdir(parents=True,exist_ok=False)
helper=BIN/'reversebolt_runtime_probe.dll';digest=hashlib.sha256(helper.read_bytes()).hexdigest()
log=(OUT/'host.jsonl').open('w')
host=subprocess.Popen([str(BIN/'object-host.exe'),str(BIN/'reversebolt_mock_runtime.dll'),str(OUT/'identity.json'),str(OUT/'stop')],stdout=log,stderr=subprocess.STDOUT)
receipts=[];identities=[]
try:
    deadline=time.monotonic()+10
    while not (OUT/'identity.json').exists():
        assert host.poll() is None and time.monotonic()<deadline
        time.sleep(.05)
    identity=json.loads((OUT/'identity.json').read_text())
    base=[str(BIN/'probe-loader.exe'),str(identity['pid']),identity['created'],str(identity['session']),str(helper),digest]
    bad=base+[str(OUT/'wrong-identity.json')];bad[2]='0'*16
    result=subprocess.run(bad,capture_output=True,text=True,timeout=10)
    (OUT/'wrong-identity.jsonl').write_text(result.stderr)
    assert result.returncode==1 and not result.stdout
    for index in range(3):
        time.sleep(.15)
        path=OUT/f'probe-{index}.json'
        result=subprocess.run(base+[str(path)],capture_output=True,text=True,timeout=130)
        (OUT/f'probe-{index}.stdout.jsonl').write_text(result.stdout)
        (OUT/f'probe-{index}.stderr.jsonl').write_text(result.stderr)
        assert result.returncode==0,result.stderr
        receipt=json.loads(result.stdout.splitlines()[-1]);assert receipt['helper_absent']
        receipts.append(receipt)
        report=json.loads(path.read_text());classes={k['id']:k for k in report['classes']}
        for snapshot in report['snapshots']:
            assert not snapshot['time_or_read_budget_reached']
            assert snapshot['helper_allocations_while_paused']==snapshot['helper_deallocations_while_paused']==0
            assert len(snapshot['nodes'])==2 and snapshot['processed_nodes']==2
            values={};edges={}
            for r in snapshot['reads']:
                assert r['read_ok']
                f=classes[r['declaring_class']]['fields'][r['field']]
                if f['reference']:edges[r['node']]=r['reference']['id']
                else:values.setdefault(r['node'],{})[f['name']]=r['value']
            assert edges=={0:1,1:0}
            first,second=values[0],values[1]
            assert first['Identity']+1==second['Identity']
            assert first['Counter']==(first['Identity']//2)*17
            assert second['Counter']==(first['Identity']//2)*19
            for v in values.values():
                assert float(v['HiddenScale'])==v['Identity']+.375
                assert float(v['Reserve'])==4.25
            identities.append(first['Identity'])
    assert len(set(identities))==6
finally:
    (OUT/'stop').touch()
    try:host.wait(timeout=5)
    except subprocess.TimeoutExpired:host.terminate();host.wait(timeout=5)
    log.close()
    (OUT/'execution.json').write_text(json.dumps({'host_pid':host.pid,'host_exit':host.returncode,'receipts':receipts,'snapshot_identities':identities},indent=2))
balance=json.loads((OUT/'host.jsonl').read_text())
assert host.returncode==0 and balance=={'stops':6,'starts':6},balance
print(json.dumps({'passed':True,'snapshots':6,'identities':identities,'balanced_pauses':balance}))
