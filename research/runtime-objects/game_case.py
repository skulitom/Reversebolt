import hashlib
import json
from pathlib import Path
import subprocess
import sys
ROOT=Path(__file__).resolve().parents[2]
BIN=ROOT/'target/runtime-objects/debug'
pid,created,session,folder=sys.argv[1:]
out=Path(folder).resolve();out.mkdir(parents=True,exist_ok=False)
helper=BIN/'reversebolt_runtime_probe.dll';digest=hashlib.sha256(helper.read_bytes()).hexdigest()
result=subprocess.run([str(BIN/'probe-loader.exe'),pid,created,session,str(helper),digest,str(out/'probe.json')],capture_output=True,text=True,timeout=130)
(out/'stdout.jsonl').write_text(result.stdout);(out/'stderr.jsonl').write_text(result.stderr)
assert result.returncode==0,result.stderr
receipt=json.loads(result.stdout.splitlines()[-1]);assert receipt['helper_absent']
report=json.loads((out/'probe.json').read_text());classes={k['id']:k for k in report['classes']}
shots=[]
for shot in report['snapshots']:
    assert shot['helper_allocations_while_paused']==shot['helper_deallocations_while_paused']==0
    scalar=[r for r in shot['reads'] if r['read_ok'] and r['value'] is not None]
    game=[r for r in scalar if classes[r['declaring_class']]['image']=='Assembly-CSharp.dll']
    assert len(game)>0,'no main-game instance scalar values'
    shots.append({k:shot[k] for k in ['pause_held_us','stop_to_resume_us','helper_allocations_while_paused','helper_deallocations_while_paused','processed_nodes','time_or_read_budget_reached','node_capacity_reached']})
    shots[-1].update({'nodes':len(shot['nodes']),'unknown_type_nodes':sum(n['class'] is None for n in shot['nodes']),'roots':len(shot['roots']),'reads':len(shot['reads']),'scalar_reads':len(scalar),'main_game_scalar_reads':len(game)})
summary={'pid':int(pid),'created':created,'helper_sha256':digest,'raw_sha256':hashlib.sha256((out/'probe.json').read_bytes()).hexdigest(),'catalog_ms':report['catalog_ms'],'catalog_classes':report['catalog_classes'],'catalog_fields':report['catalog_fields'],'catalog_roots':report['catalog_roots'],'snapshots':shots,'loader':receipt,'edit_authority':False}
(out/'summary.json').write_text(json.dumps(summary,indent=2));print(json.dumps(summary))
