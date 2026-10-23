"""Bounded coverage and fresh read check on an owned game launch; no input or writes."""
import hashlib,json,subprocess,sys,time,os
from pathlib import Path
OUT=Path(sys.argv[1]).resolve();CASE=OUT/'live';BIN=OUT/'bin'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
save=lambda name,v:(CASE/name).write_text(json.dumps(v,indent=2))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json');scope=read(CASE/'scope.json')
assert sha(Path(__file__))==read(CASE/'protocol.json')['script_sha256']
for name,digest in frozen['binaries'].items():assert sha(BIN/name)==digest
assert not (CASE/'started.json').exists(),'Do not replay a started run'
save('started.json',{'runner_pid':os.getpid(),'target':scope,'unix_ms':time.time_ns()//1000000})
base=[BIN/'reversebolt-runtime-loader.exe',str(scope['pid']),scope['created'],str(scope['session']),BIN/'reversebolt_runtime.dll',frozen['binaries']['reversebolt_runtime.dll']]
calls=[];samples=[]
def command(args,label):
    p=subprocess.run(list(map(str,args)),capture_output=True,text=True,timeout=130,creationflags=subprocess.CREATE_NO_WINDOW)
    (CASE/(label+'.stdout')).write_text(p.stdout);(CASE/(label+'.stderr')).write_text(p.stderr)
    assert p.returncode==0,(label,p.stderr,p.stdout)
    return p
def call(label,extra=()):
    p=command(base+[CASE/(label+'.json')]+list(extra),label)
    completion=json.loads(p.stdout.splitlines()[-1]);assert completion['helper_absent'] and completion['inspection_exit']==0
    r=read(CASE/(label+'.json'))
    pauses=r.get('pauses',r.get('snapshots',[])+[w['discarded_snapshot'] for w in r.get('warmups',[])])
    assert pauses and all(s['helper_allocations_while_paused']==s['helper_deallocations_while_paused']==0 for s in pauses)
    calls.append({'label':label,'helper_absent':True,'report_sha256':sha(CASE/(label+'.json'))})
    samples.extend(pauses)
    return r
try:
    next_root=0;fingerprint=None;pages=[];selected=[];seen=set()
    for page in range(8):
        label=f'discovery-{page}'
        r=call(label,() if page==0 else [next_root,fingerprint])
        assert r['schema']==5 and r['value_type_metadata'] and not r['edit_authority']
        command([BIN/'reversebolt-runtime-candidates.exe',CASE/(label+'.json'),CASE/(label+'.candidates.json'),4096],label+'-candidates')
        candidates=read(CASE/(label+'.candidates.json'))
        key=lambda k:json.dumps(k,sort_keys=True)
        classes={key(c['key']):c for c in r['classes']}
        def new_storage(c):
            root=c['locator']['root'];f=classes[key(root['declaring'])]['fields'][root['field_id']]
            if f['inline']:return 'embedded'
            for s in c['locator']['steps']:
                m=s['member'];owner=classes[key(s['owner'])]
                if m['kind']=='element':
                    if owner.get('element_inline_type'):return 'embedded'
                else:
                    f=classes[key(m['field']['declaring'])]['fields'][m['field']['field_id']]
                    if f['inline']:return 'embedded'
                    if f.get('declared_type') and not f['reference']:return 'enum'
            return None
        eligible=[(c,new_storage(c)) for c in candidates if c['locator']['root']['declaring']['image']!=r.get('core_image')]
        for c,kind in eligible:
            if kind and c['id'] not in seen and len(selected)<32:
                selected.append({'candidate':c,'storage':kind});seen.add(c['id'])
        fields=[f for c in r['classes'] for f in c['fields']]
        pages.append({'page':page,'root_start':r['root_start'],'next_root':r['next_root'],'root_scan_complete':r['root_scan_complete'],
            'catalog_roots':r['catalog_roots'],'catalog_ms':r['catalog_ms'],'classes':len(r['classes']),
            'inline_fields':sum(f['inline'] for f in fields),'enum_fields':sum(bool(f.get('declared_type')) and not f['inline'] and f['width']>0 for f in fields),
            'inline_nodes':sum(n['inline'] for s in r['snapshots'] for n in s['nodes']),
            'scalar_reads':sum(x['read_ok'] and x['reference'] is None and x.get('embedded') is None for s in r['snapshots'] for x in s['reads']),
            'candidate_sample':len(candidates),'new_storage_candidates_in_sample':sum(kind is not None for c,kind in eligible)})
        print(json.dumps(pages[-1]),flush=True)
        if r['root_scan_complete']:break
        assert r['next_root']>next_root
        next_root=r['next_root'];fingerprint=r['root_catalog_sha256']
    save('selected.json',selected);assert selected,'No new-storage paths reached'
    batches=[];batch=[]
    for item in selected:
        proposal=batch+[item['candidate']['locator']]
        # Bound the actual loader request, including its absolute output path.
        encoded=json.dumps({'output':str(CASE/'refresh-99-99.json'),'root_start':0,'expected_root_catalog_sha256':None,'paths':proposal,'patch':None},ensure_ascii=False)
        if batch and (len(proposal)>16 or len(encoded.encode('utf-16-le'))//2>29000):batches.append(batch);batch=[]
        batch.append(item['candidate']['locator'])
    if batch:batches.append(batch)
    checks=[]
    for i,paths in enumerate(batches):
        file=CASE/f'batch-{i}.json';save(file.name,paths)
        for sample in range(3):
            r=call(f'refresh-{i}-{sample}',['--paths-file',file]);checks.extend(r['checks']);time.sleep(.2)
    save('result.json',{'passed':True,'pages':pages,'selected':len(selected),'batches':len(batches),'checks':len(checks),
        'successful_reads':sum(c['status']=='read' for c in checks),'statuses':sorted(set(c['status'] for c in checks)),
        'calls':calls,'pauses':len(samples),'max_stop_to_resume_us':max(s['stop_to_resume_us'] for s in samples),
        'model_calls':0,'measurement_screenshots':0,'gameplay_inputs':0,'gameplay_writes':0,'meaning_verified':False})
    print(json.dumps({'passed':True,'selected':len(selected),'reads':sum(c['status']=='read' for c in checks),'total':len(checks),'pauses':len(samples)}),flush=True)
except BaseException as e:
    save('failure.json',{'error':repr(e),'calls':calls});raise
