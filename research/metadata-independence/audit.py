"""Audit frozen offline replays; counts do not measure semantic correctness."""
import ctypes as C, hashlib, json, re, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
results={};pids=[];models=set()
for version in ('baseline','revised'):
    frozen=read(OUT/f'{version}-frozen.json');assert sha(OUT/frozen['binary'])==frozen['binary_sha256']
    for name,digest in frozen['sources'].items():assert sha(OUT/f'{version}-source'/name)==digest
    cases=[]
    for case in frozen['cases']:
        assert sha(Path(case['source']))==case['sha256']
        process=read(OUT/version/(case['name']+'-process.json'));assert process['exit_code']==0
        pids.append(process['pid'])
        r=read(OUT/version/case['name']/'receipt.json');inp=read(OUT/version/case['name']/'input.json')
        assert r['error'] is None and not r['cancelled'] and r['batch'] is not None
        assert r['report_sha256']==inp['inventory']['report_sha256']==r['batch']['report_sha256']
        models.add((r['model'],r['model_digest'],r['server_version']))
        for index,stage in enumerate(r['stages']):
            assert hashlib.sha256(stage['prompt'].encode()).hexdigest()==stage['prompt_sha256']
            assert case['source'] not in stage['prompt']
            assert stage['response']['done_reason']=='stop'
            if index:assert stage['schema']['properties']['hypotheses']['maxItems'] in (1,2)
        b=r['batch'];accepted=b['accepted'];rejected=b['rejected']
        assert len(accepted)+len(rejected)<=12
        fields={(h['candidate']['type_id'],h['candidate']['field_id']) for h in accepted}
        cases.append({'case':case['name'],'accepted':len(accepted),'rejected':len(rejected),'distinct_fields':len(fields),
            'distinct_types':len({t for t,_ in fields}),'names':[h['proposal']['name'] for h in accepted],
            'fields':[h['candidate']['field_name'] for h in accepted],
            'rejections':[{'name':h['proposal']['name'],'reason':h['reason']} for h in rejected],
            'selected_types':b['selected_types'],'omitted_samples':b['omitted_samples'],
            'receipt_sha256':sha(OUT/version/case['name']/'receipt.json'),'seconds':process['seconds']})
    results[version]=cases
assert len(models)==1
plain=read(OUT/'revised/latest/receipt.json');paired=read(OUT/'revised/latest-with-activity/receipt.json')
assert plain['activity'] is None and paired['activity'] is not None
assert [s['prompt_sha256'] for s in plain['stages']]==[s['prompt_sha256'] for s in paired['stages']]
assert plain['selection']==paired['selection']
assert [s['schema'] for s in plain['stages']]==[s['schema'] for s in paired['stages']]
assert [s['options'] for s in plain['stages']]==[s['options'] for s in paired['stages']]
plain_refs=[(h['candidate'],h['alternatives']) for h in plain['batch']['accepted']]
paired_refs=[(h['candidate'],h['alternatives']) for h in paired['batch']['accepted']]
assert plain_refs==paired_refs
wording_changes=sum(a['proposal']!=b['proposal'] for a,b in zip(plain['batch']['accepted'],paired['batch']['accepted']))
manifest=read(OUT/'activity.json');frozen=read(OUT/'revised-frozen.json')
assert sha(OUT/'activity.json')==frozen['activity_sha256']
assert paired['activity']['source']==manifest['source']
for path,digest in frozen['images'].items():assert sha(Path(path))==digest
assert all('Current observed activity' not in s['prompt'] for s in paired['stages'])
assert paired['activity']['stage'] is not None
assert hashlib.sha256(paired['activity']['stage']['prompt'].encode()).hexdigest()==paired['activity']['stage']['prompt_sha256']
invalid=read(OUT/'invalid-results.json');assert len(invalid)==4
assert all(r['exit_code']!=0 and not r['output_created'] for r in invalid)
kernel=C.WinDLL('kernel32');kernel.OpenProcess.restype=C.c_void_p
kernel.GetExitCodeProcess.argtypes=[C.c_void_p,C.POINTER(C.c_ulong)];kernel.CloseHandle.argtypes=[C.c_void_p]
for pid in pids:
    h=kernel.OpenProcess(0x1000,False,pid)
    if h:
        code=C.c_ulong();assert kernel.GetExitCodeProcess(h,C.byref(code)) and code.value!=259,pid;kernel.CloseHandle(h)
tests=(ROOT/'test-results/metadata-independence-tests.log').read_text(encoding='utf-8-sig')
assert 'FAILED' not in tests and sum(map(int,re.findall(r'test result: ok\. (\d+) passed',tests)))==17
clippy=(ROOT/'test-results/metadata-independence-clippy.log').read_text(encoding='utf-8-sig')
assert 'Finished' in clippy and 'error:' not in clippy
summary={'audit_passed':True,'artifact_root':str(OUT.relative_to(ROOT)),'model':list(next(iter(models))),
    'cases':results,'same_metadata_prompts_selection_and_references_with_activity':True,
    'identical_full_batch':plain['batch']==paired['batch'],'accepted_wording_changes':wording_changes,
    'initial_audit_failure':read(OUT/'initial-audit-failure.json'),
    'activity_observation':paired['activity']['observation'],'activity_error':paired['activity']['error'],
    'invalid_manifests_rejected':4,'observer_tests':17,'strict_clippy':True,'runner_processes_exited':len(pids),
    'desktop_updated':False,'progress_percent':35,
    'limits':['Development replay on already explored reports; no fresh independent-game evidence.',
        'More accepted questions are broader candidate coverage, not semantic precision, useful controls or verified effects.',
        'Metadata-only and image-only evidence stay separate; their causal relationship is not learned here.',
        'Fixed seed and temperature did not produce byte-identical wording. The initial exact-batch audit failed; original outputs are retained without retry.',
        'The known fixture may still abstain before same-object text reaches the second stage.',
        'No gameplay process was opened, no image was manually reviewed, and no edit or input was attempted.',
        'The separate desktop executable remains blocked by Windows security; it was not restored or repackaged.']}
Path(__file__).with_name('results.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'audit_passed':True,'cases':{v:[{k:c[k] for k in ('case','accepted','rejected','distinct_fields')} for c in cases] for v,cases in results.items()},'activity_error':summary['activity_error'],'processes_exited':len(pids)},indent=2))
