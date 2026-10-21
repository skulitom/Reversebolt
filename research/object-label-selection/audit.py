"""Audit retained model indexes against source objects, not semantic truth."""
import ctypes as C, hashlib, json, re, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
compact=lambda v:json.dumps(v,separators=(',',':'),ensure_ascii=False).encode()
baseline=read(OUT/'baseline.json');old=Path(baseline['root']);old_frozen=read(Path(baseline['frozen']))
assert sha(old/old_frozen['binary'])==baseline['binary_sha256']==old_frozen['binary_sha256']
frozen=read(OUT/'revised-frozen.json');assert sha(OUT/frozen['binary'])==frozen['binary_sha256']
for name,digest in frozen['sources'].items():assert sha(OUT/'revised-source'/name)==digest,name
for name,digest in old_frozen['sources'].items():assert sha(old/'revised-source'/name)==digest,name
cases=[];pids=[];identities=set()
for case in frozen['cases']:
    name=case['name'];assert sha(Path(case['source']))==case['sha256']
    process=read(OUT/'revised'/(name+'-process.json'));assert process['exit_code']==0;pids.append(process['pid'])
    before=read(old/'revised'/name/'receipt.json');receipt=read(OUT/'revised'/name/'receipt.json')
    inventory=read(OUT/'revised'/name/'input.json')['inventory']
    assert inventory==read(old/'revised'/name/'input.json')['inventory']
    assert receipt['report_sha256']==inventory['report_sha256']==before['report_sha256']
    identities.add((receipt['model'],receipt['model_digest'],receipt['server_version']))
    assert (receipt['model'],receipt['model_digest'],receipt['server_version'])==(before['model'],before['model_digest'],before['server_version'])
    stage=receipt['stages'][0];index=json.loads(stage['prompt'].split('\nObserved types and path-scoped text examples: ',1)[1])
    assert len(index['types'])==len(inventory['types']) and stage['options']['num_ctx']==49152
    available=0;included=0
    for source,t in zip(inventory['types'],index['types']):
        assert t['id']==source['id'] and t['name']==source['key']['type_name'][:120]
        assert t['name_shortened']==(len(source['key']['type_name'])>120)
        assert t['field_labels']==[f['name'] for f in source['fields']]
        paths={}
        for field in source['fields']:
            for sample in field['samples'][:4]:
                texts=list(dict.fromkeys(h['text'] for h in sample['context'] if not h['truncated'] and len(h['text'].encode())<=256 and h['text'].strip()))
                paths[(field['name'],sample['path_id'])]=texts
                available+=bool(texts)
        examples=t.get('object_samples',[]);assert len(examples)<=4
        assert len({(s['field_name'],s['path_id']) for s in examples})==len(examples)
        for example in examples:
            assert example['object_text']==paths[(example['field_name'],example['path_id'])]
            assert example['object_text'];included+=1
    assert index['available_object_samples']==available and index['omitted_object_samples']==available-included
    base=json.loads(json.dumps(index))
    for t in base['types']:t.pop('object_samples',None)
    base['omitted_object_samples']=available
    assert len(compact(index))-len(compact(base))<=16384
    prompt_tokens=[]
    for s in receipt['stages']:
        assert len(s['prompt'].encode())<=110000
        assert hashlib.sha256(s['prompt'].encode()).hexdigest()==s['prompt_sha256']
        assert case['source'] not in s['prompt']
        tokens=s['response'].get('prompt_eval_count');prompt_tokens.append(tokens)
        if receipt['error'] is None:
            assert isinstance(tokens,int) and tokens<=s['options']['num_ctx']-s['options']['num_predict']-512
    batch=receipt.get('batch');prior=before.get('batch')
    accepted=(batch or {}).get('accepted',[])
    cases.append({'case':name,'baseline_accepted':len((prior or {}).get('accepted',[])),
        'baseline_rejected':len((prior or {}).get('rejected',[])),
        'accepted':len(accepted),'rejected':len((batch or {}).get('rejected',[])),
        'error':receipt['error'],'cancelled':receipt['cancelled'],'types_selected':(batch or {}).get('selected_types'),
        'type_count':len(index['types']),'context_samples_available':available,'context_samples_included':included,
        'context_samples_omitted':available-included,'selection_prompt_bytes':len(stage['prompt'].encode()),'prompt_tokens':prompt_tokens,
        'names':[h['proposal']['name'] for h in accepted],
        'fields':[h['candidate']['field_name'] for h in accepted],
        'distinct_fields':len({(h['candidate']['type_id'],h['candidate']['field_id']) for h in accepted}),
        'object_text_citations':[{'name':h['proposal']['name'],'path_id':h['candidate']['path_id'],'quotes':[c['quote'] for c in h['proposal']['citations'] if c['source']=='object_text']} for h in accepted if any(c['source']=='object_text' for c in h['proposal']['citations'])],
        'rejections':[{'name':h['proposal']['name'],'reason':h['reason']} for h in (batch or {}).get('rejected',[])],
        'receipt_sha256':sha(OUT/'revised'/name/'receipt.json'),'baseline_receipt_sha256':sha(old/'revised'/name/'receipt.json'),'seconds':process['seconds']})
assert len(identities)==1
kernel=C.WinDLL('kernel32');kernel.OpenProcess.restype=C.c_void_p
kernel.GetExitCodeProcess.argtypes=[C.c_void_p,C.POINTER(C.c_ulong)];kernel.CloseHandle.argtypes=[C.c_void_p]
for pid in pids:
    h=kernel.OpenProcess(0x1000,False,pid)
    if h:
        code=C.c_ulong();assert kernel.GetExitCodeProcess(h,C.byref(code)) and code.value!=259,pid;kernel.CloseHandle(h)
tests=(ROOT/'test-results/object-label-selection-tests.log').read_text(encoding='utf-8-sig')
assert 'FAILED' not in tests and sum(map(int,re.findall(r'test result: ok\. (\d+) passed',tests)))==179
clippy=(ROOT/'test-results/object-label-selection-clippy.log').read_text(encoding='utf-8-sig')
assert 'Finished' in clippy and 'error:' not in clippy
summary={'artifact_audit_passed':True,'artifact_root':str(OUT.relative_to(ROOT)),'model':list(next(iter(identities))),
    'baseline_reused':True,'cases':cases,'core_observer_tests':179,'strict_clippy':True,'runner_processes_exited':len(pids),
    'desktop_updated':False,'progress_percent':35,
    'limits':['Previously explored saved reports and an owned labelled fixture; not held-out game evaluation.',
        'Accepted questions establish reference/lexical grounding only; they do not establish semantic precision, controlling state or safe bounds.',
        'The type index exposes at most four complete-text examples per type within its shared byte budget; omitted examples remain unknown.',
        'Input token accounting is checked for headroom; the byte cap is not a universal tokenizer bound.',
        'The selection prompt, path context and context size changed together; this comparison does not isolate their individual effects.',
        'No native gameplay, input, write or manually reviewed image was needed. The blocked desktop executable was not rebuilt or restored.']}
Path(__file__).with_name('results.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'audit_passed':True,'cases':[{k:c[k] for k in ('case','baseline_accepted','accepted','rejected','error','context_samples_included','context_samples_omitted','selection_prompt_bytes','prompt_tokens')} for c in cases]},indent=2))
