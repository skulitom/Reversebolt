"""Audit retained versions/results without starting another target operation."""
import hashlib,json,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
def read(p):return json.loads(p.read_bytes())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
audit={'versions':{},'runs':{}}
for version in (1,2):
    base=ROOT/'test-results'/f'runtime-product-v{version}'
    manifest=read(base/'frozen.json')
    for name,digest in manifest['binaries'].items():assert sha(base/'bin'/name)==digest,name
    for path,identity in manifest['sources'].items():
        data=subprocess.check_output(['git','show',manifest['revision']+':'+path],cwd=ROOT)
        assert hashlib.sha256(data).hexdigest()==identity['git_sha256'],path
        assert identity['working_sha256'] in (hashlib.sha256(data).hexdigest(),hashlib.sha256(data.replace(b'\n',b'\r\n')).hexdigest(),hashlib.sha256(data.replace(b'\r\n',b'\n')).hexdigest()),path
    audit['versions'][str(version)]={'revision':manifest['revision'],'frozen_sha256':sha(base/'frozen.json'),'binaries':manifest['binaries'],'source_files':len(manifest['sources'])}
    cases=['fixture'] if version==1 else ['fixture','game']
    for case in cases:
        run=base/case
        discovery=read(run/'discovery.json')
        assert discovery['state']['live']['sample_count']==0
        assert not discovery['state']['live']['frames']
        receipts=list((run/'data'/'runtime').glob('*-*/**/stdout.jsonl'))
        for path in receipts:
            rows=[json.loads(line) for line in path.read_text().splitlines()]
            assert rows[0]['helper_sha256']==manifest['binaries']['reversebolt_runtime.dll']
            assert rows[-1]['stage']=='complete' and rows[-1]['helper_absent']
            report=read(path.with_name('report.json'))
            assert all(report[k]==rows[0][k] for k in ('pid','created','session'))
        if version==1:
            failed=read(run/'refresh-receipt.json')
            assert not failed['ok'] and failed['error']=='invalid plan size'
            assert len(receipts)==1
            assert read(run/'host-balance.json')=={'starts':5,'stops':5}
            audit['runs']['v1-fixture']={'passed':False,'failure':'invalid plan size on first four-value refresh','discovery_cards':len(discovery['state']['live']['runtime']['cards']),'helper_unloaded':True,'execution':read(run/'execution.json')}
            continue
        summary=read(run/'summary.json');assert summary['passed']
        assert len(receipts)==summary['operations']
        for item in summary['evidence']:
            path=Path(item['path'])
            assert sha(path/'stdout.jsonl')==item['receipt_sha256']
            assert sha(path/'report.json')==item['report_sha256']
        assert not read(run/'stale.json')['ok']
        assert read(run/'replay.json')['data']['duplicate']
        ui=read(run/'ui.json');assert not ui['includeScreenshot']
        assert any(e['name']=='Refresh value' and e['enabled'] for e in ui['visible'])
        assert summary['screenshots']==summary['gameplay_input']==summary['gameplay_writes']==summary['verified_controls']==0
        execution=read(run/'execution.json')
        if case=='fixture':
            assert execution['host_exit']==0
            assert read(run/'host-balance.json')=={'starts':summary['pauses'],'stops':summary['pauses']}
        audit['runs']['v2-'+case]={**summary,'execution':execution}
after=read(ROOT/'test-results/runtime-product-v2/game-after.json')
assert after['pid']==audit['runs']['v2-game']['target_pid']
assert after['responding'] and after['helper_absent'] and after['exited']
audit['game_after']=after
desktop=read(ROOT/'test-results/runtime-product-v2/desktop.json')
assert desktop['ok'] and desktop['session']==1 and desktop['window']
assert desktop['history_before']==desktop['history_after']==29
for name,digest in audit['versions']['2']['binaries'].items():
    if name!='reversebolt_path_fixture.dll':
        assert sha(ROOT/'dist'/desktop['version']/name)==digest
audit['desktop']=desktop
smoke=ROOT/'test-results/smoke-f59105e1/report.json'
audit['smoke']={'sha256':sha(smoke),'result':read(smoke)}
audit['scope']='Product discovery/refresh integration; no verified gameplay effects or cross-game generalization.'
destination=Path(__file__).with_name('results.json')
destination.write_text(json.dumps(audit,indent=2))
print(json.dumps({'audit':'passed','versions':list(audit['versions']),'runs':{k:v['passed'] for k,v in audit['runs'].items()}}))
