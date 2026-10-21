"""Post-run audit. No launch, probe retry or target interaction."""
from collections import Counter
import hashlib,json
from pathlib import Path
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];RAW=ROOT/'test-results/runtime-objects-v1'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
freeze=read(HERE/'frozen-v1.json')
for f in freeze['files']:assert sha(ROOT/f['path'])==f['sha256'],f['path']
execution=read(RAW/'mock/execution.json');assert execution['host_exit']==0
balance=read(RAW/'mock/host.jsonl');assert balance=={'starts':6,'stops':6}
mock_shots=[]
for p in sorted((RAW/'mock').glob('probe-*.json')):
    report=read(p);classes={c['id']:c for c in report['classes']}
    for s in report['snapshots']:
        identity=next(r['value'] for r in s['reads'] if r['node']==0 and classes[r['declaring_class']]['fields'][r['field']]['name']=='Identity')
        assert s['helper_allocations_while_paused']==s['helper_deallocations_while_paused']==0
        mock_shots.append({'root_address':s['nodes'][0]['address'],'identity':identity,'pause_held_us':s['pause_held_us'],'stop_to_resume_us':s['stop_to_resume_us']})
reuse=sum(a['root_address']==b['root_address'] and a['identity']!=b['identity'] for a,b in zip(mock_shots,mock_shots[1:]))
assert reuse>0
summary=read(RAW/'blue-prince/summary.json');report=read(RAW/'blue-prince/probe.json')
assert sha(RAW/'blue-prince/probe.json')==summary['raw_sha256']
classes={c['id']:c for c in report['classes']};coverage=[]
for s in report['snapshots']:
    assert s['helper_allocations_while_paused']==s['helper_deallocations_while_paused']==0
    assert len(s['nodes'])==1024 and len(s['reads'])==2910
    nodes={n['id']:n for n in s['nodes']}
    assert len(nodes)==len(s['nodes'])
    assert all(r['read_ok'] for r in s['roots']+s['reads'])
    for r in s['reads']:
        n=nodes[r['node']];f=classes[r['declaring_class']]['fields'][r['field']]
        assert f['offset']+len(r['bytes'])<=classes[n['class']]['instance_size']
        if f['reference'] and 'id' in r['reference']:assert r['reference']['id'] in nodes
    coverage.append({'depths':dict(Counter(n['depth'] for n in s['nodes'])),'null_roots':sum(r['reference']=='0x0' for r in s['roots']),'root_statuses':dict(Counter(r['node'].get('status','followed') for r in s['roots'])),'edge_statuses':dict(Counter(r['reference'].get('status','followed') for r in s['reads'] if isinstance(r['reference'],dict)))})
after=read(RAW/'game-after.json');assert after['responding'] and after['window'] and not after['helper_present']
assert after['created']==summary['created'] and after['pid']==summary['pid']
cleanup=read(RAW/'cleanup.json');assert cleanup['remaining_owned_processes']==[] and cleanup['lease_released']
desktop=read(RAW/'desktop.json');assert desktop['ok'] and not desktop['changed_this_turn']
build_log=(RAW/'unity-build.log').read_text(errors='replace')
assert 'No valid Unity Editor license found' in build_log
sdk=Path(r'C:\Program Files\Unity\Hub\Editor\2022.3.18f1\Editor\Data\il2cpp')
sdk_files=[sdk/p for p in ['libil2cpp/il2cpp-api-functions.h','libil2cpp/il2cpp-api.cpp','libil2cpp/vm/Object.cpp','libil2cpp/gc/BoehmGC.cpp','external/bdwgc/misc.c']]
artifacts=[p for p in RAW.rglob('*') if p.is_file() and 'unity-project' not in p.parts and 'unity-build' not in p.parts]
result={'audit_passed':True,'frozen_files_verified':len(freeze['files']),'mock_snapshots':mock_shots,'observed_address_reuse_with_changed_identity':reuse,'balanced_pauses':balance,'game':summary,'coverage':coverage,'catalog_omissions':dict(Counter(report['catalog_omissions'])),'game_after':after,'unity_fixture_build':'failed before compilation: no active Editor license; no activation or retry','sdk_contract_sources':[{'path':str(p),'sha256':sha(p)} for p in sdk_files],'cleanup':cleanup,'desktop':desktop,'screenshots':0,'gameplay_inputs':0,'game_field_write_calls':0,'edit_authority':False,'progress_percent':31,'raw_artifacts':[{'path':str(p.relative_to(ROOT)).replace('\\','/'),'sha256':sha(p),'bytes':p.stat().st_size} for p in sorted(artifacts)]}
(HERE/'results.json').write_text(json.dumps(result,indent=2))
print(json.dumps({'audit_passed':True,'frozen_files_verified':len(freeze['files']),'address_reuse_cases':reuse,'coverage':coverage}))
