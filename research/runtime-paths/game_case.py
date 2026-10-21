"""One unchanged discovery and one path refresh; no screenshots or gameplay input."""
import hashlib,json,subprocess,sys
from pathlib import Path
pid,created,session,folder=sys.argv[1:]
OUT=Path(folder).resolve();OUT.mkdir(parents=True,exist_ok=False);BIN=OUT.parent/'bin'
helper=BIN/'reversebolt_runtime.dll';digest=hashlib.sha256(helper.read_bytes()).hexdigest()
base=[str(BIN/'reversebolt-runtime-loader.exe'),pid,created,session,str(helper),digest]
def call(label,extra=()):
    path=OUT/(label+'.json')
    result=subprocess.run(base+[str(path)]+list(extra),capture_output=True,text=True,timeout=130)
    (OUT/(label+'.stdout.jsonl')).write_text(result.stdout);(OUT/(label+'.stderr.jsonl')).write_text(result.stderr)
    assert result.returncode==0,result.stderr
    receipt=json.loads(result.stdout.splitlines()[-1]);assert receipt['helper_absent']
    return json.loads(path.read_bytes()),receipt
report,discovery_receipt=call('discovery')
converted=subprocess.run([str(BIN/'reversebolt-runtime-candidates.exe'),str(OUT/'discovery.json'),str(OUT/'candidates.json'),'4096'],capture_output=True,text=True,timeout=30)
(OUT/'candidates.stdout.jsonl').write_text(converted.stdout);(OUT/'candidates.stderr.jsonl').write_text(converted.stderr)
assert converted.returncode==0,converted.stderr
candidates=json.loads((OUT/'candidates.json').read_bytes())
# Fixed generic selection: first four candidates at least one reference hop deep,
# with distinct terminal field/element descriptors. No game names are used.
chosen=[];seen=set()
for candidate in candidates:
    path=candidate['locator'];terminal=json.dumps(path['steps'][-1],sort_keys=True)
    if len(path['steps'])>=2 and terminal not in seen:
        chosen.append(candidate);seen.add(terminal)
    if len(chosen)==4:break
assert len(chosen)==4
(OUT/'selected.json').write_text(json.dumps(chosen,indent=2))
paths=OUT/'paths.json';paths.write_text(json.dumps([c['locator'] for c in chosen]))
fresh,refresh_receipt=call('refresh',['--paths-file',str(paths)])
assert not fresh['edit_authority']
assert [c['path_id'] for c in fresh['checks']]==[c['id'] for c in chosen]
assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in fresh['pauses'])
assert all(c['status']=='read' for c in fresh['checks'])
summary={'passed':True,'pid':int(pid),'created':created,'helper_sha256':digest,
         'discovery_root_scan_complete':report['root_scan_complete'],'discovery_next_root':report['next_root'],'discovery_catalog_roots':report['catalog_roots'],
         'candidate_count':len(candidates),'selected_path_steps':[len(c['locator']['steps']) for c in chosen],
         'refresh_statuses':[c['status'] for c in fresh['checks']],'refresh_pauses':fresh['pauses'],'refresh_classes_added':fresh['classes_added'],
         'discovery_loader':discovery_receipt,'refresh_loader':refresh_receipt,'edit_authority':False}
(OUT/'summary.json').write_text(json.dumps(summary,indent=2));print(json.dumps(summary))
