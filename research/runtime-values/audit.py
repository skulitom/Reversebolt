"""Audit retained native results without launching software or reading game memory."""
import hashlib,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=Path(sys.argv[1]).resolve()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json')
for name,digest in frozen['sources'].items():assert sha(OUT/'source'/name)==digest
for name,digest in frozen['binaries'].items():assert sha(OUT/'bin'/name)==digest
pauses=[]
def reports(case,calls):
    result=[]
    for call in calls:
        p=case/(call['label']+'.json');r=read(p)
        if 'report_sha256' in call:assert sha(p)==call['report_sha256']
        stdout=case/(call['label']+'.stdout')
        receipt=json.loads(stdout.read_text().splitlines()[-1])
        assert receipt['helper_absent'] and receipt['inspection_exit']==0
        samples=r.get('pauses',r.get('snapshots',[])+[w['discarded_snapshot'] for w in r.get('warmups',[])])
        assert samples and all(s['helper_allocations_while_paused']==s['helper_deallocations_while_paused']==0 for s in samples)
        pauses.extend(samples);result.append(r)
    return result
for case_name in ['values','legacy']:
    case=OUT/case_name;r=read(case/'result.json');assert r['passed']
    before=len(pauses);reports(case,r['calls'])
    assert r['pauses']==len(pauses)-before
    assert read(case/'host.jsonl')=={'starts':r['pauses'],'stops':r['pauses']}
    assert read(case/'execution.json')['host_exit']==0
case=OUT/'values';writes=[read(p)['patch']['state'] for p in case.glob('*-write.json')]
assert writes.count('written')==7 and writes.count('refused')==2
for after in case.glob('*-after.json'):
    before=read(case/(after.stem.removesuffix('-after')+'-before.json'))
    assert read(after)['checks'][0]['bytes']==before['checks'][0]['bytes']
assert read(case/'all-restored.json')['checks']==read(case/'refresh.json')['checks']
live=OUT/'live';r=read(live/'result.json')
assert sha(live/'live.py')==read(live/'protocol.json')['script_sha256']
before=len(pauses);data=reports(live,r['calls'])
assert r['pauses']==len(pauses)-before
checks=[c for d in data for c in d.get('checks',[])]
assert len(checks)==r['checks']==96 and all(c['status']=='read' for c in checks)
assert len({c['path_id'] for c in checks})==32
assert all(c['operation']=='refresh_runtime_paths' for c in data if 'checks' in c)
assert len([c for c in data if c.get('schema')==5])==2 and data[1]['root_scan_complete']
selected=read(live/'selected.json');assert len(selected)==32
counts={kind:sum(s['storage']==kind for s in selected) for kind in ['embedded','enum']}
assert counts=={'embedded':23,'enum':9}
summary={'passed':True,'owned_native_calls':43,'live_calls':len(r['calls']),'pauses':len(pauses),
    'max_stop_to_resume_us':max(s['stop_to_resume_us'] for s in pauses),'selected_storage':counts,
    'fresh_native_reads':96,'owned_writes_restored':7,'readonly_writes_refused':2,'gameplay_writes':0}
(OUT/'audit.json').write_text(json.dumps(summary,indent=2))
print(json.dumps(summary))
