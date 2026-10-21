import hashlib,json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];BIN=ROOT/'target/debug'
pid,created,session,folder=sys.argv[1:];out=Path(folder).resolve();out.mkdir(parents=True,exist_ok=False)
helper=BIN/'reversebolt_runtime.dll';digest=hashlib.sha256(helper.read_bytes()).hexdigest()
base=[str(BIN/'reversebolt-runtime-loader.exe'),pid,created,session,str(helper),digest]
summaries=[];extra=[]
for index in range(2):
    path=out/f'probe-{index}.json'
    result=subprocess.run(base+[str(path)]+extra,capture_output=True,text=True,timeout=130)
    (out/f'probe-{index}.stdout.jsonl').write_text(result.stdout);(out/f'probe-{index}.stderr.jsonl').write_text(result.stderr)
    assert result.returncode==0,result.stderr
    receipt=json.loads(result.stdout.splitlines()[-1]);assert receipt['helper_absent']
    report=json.loads(path.read_text(encoding='utf-8'));classes={c['id']:c for c in report['classes']}
    depths=[];scalars=0;deeper_game=0;strings=0;arrays=0;pauses=[];limits=[]
    for shot in report['snapshots']:
        assert shot['helper_allocations_while_paused']==shot['helper_deallocations_while_paused']==0
        nodes={n['id']:n for n in shot['nodes']};depths.extend(n['depth'] for n in nodes.values())
        scalar=[r for r in shot['reads'] if r['read_ok'] and r['value'] is not None]
        scalars+=len(scalar);deeper_game+=sum(nodes[r['node']]['depth']>0 and classes[r['declaring_class']]['image']=='Assembly-CSharp.dll' for r in scalar)
        strings+=sum(t['read_ok'] for t in shot['strings']);arrays+=sum(a['elements_read']>0 for a in shot['arrays'])
        pauses.append({'held_us':shot['pause_held_us'],'total_us':shot['stop_to_resume_us']})
        limits.append({'nodes':shot['node_capacity_reached'],'time_or_reads':shot['time_or_read_budget_reached'],'unknown_nodes':sum(n['class'] is None for n in nodes.values()),'truncated_arrays':sum(a['truncated'] for a in shot['arrays']),'truncated_strings':sum(t['truncated'] for t in shot['strings'])})
    for w in report['warmups']:
        s=w['discarded_snapshot'];assert s['helper_allocations_while_paused']==s['helper_deallocations_while_paused']==0
        pauses.append({'held_us':s['pause_held_us'],'total_us':s['stop_to_resume_us']})
    summary={'index':index,'raw_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'root_start':report['root_start'],'next_root':report['next_root'],'catalog_roots':report['catalog_roots'],'root_scan_complete':report['root_scan_complete'],'root_catalog_sha256':report['root_catalog_sha256'],'stop_reason':report['stop_reason'],'snapshots':len(report['snapshots']),'warmups':len(report['warmups']),'classes_added':sum(w['added_classes'] for w in report['warmups']),'nodes':len(depths),'nonroot_nodes':sum(d>0 for d in depths),'max_depth':max(depths,default=0),'scalar_reads':scalars,'deeper_main_game_scalar_reads':deeper_game,'string_reads':strings,'arrays_read':arrays,'pauses':pauses,'limits':limits,'loader':receipt}
    summaries.append(summary)
    (out/'summary.json').write_text(json.dumps({'pid':int(pid),'created':created,'helper_sha256':digest,'cases':summaries},indent=2))
    if report['root_scan_complete']:break
    assert report['next_root']>report['root_start'],'no root progress'
    extra=[str(report['next_root']),report['root_catalog_sha256']]
assert sum(s['nonroot_nodes'] for s in summaries)>0
assert sum(s['deeper_main_game_scalar_reads'] for s in summaries)>0
assert sum(s['classes_added'] for s in summaries)>0
assert sum(s['string_reads'] for s in summaries)>0 and sum(s['arrays_read'] for s in summaries)>0
print(json.dumps({'passed':True,'cases':[{k:v for k,v in s.items() if k not in ['pauses','limits']} for s in summaries]}))
