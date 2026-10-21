"""Audit both retained attempts. Never launch a process or retry native work."""
from collections import Counter
import hashlib,json,re,subprocess
from pathlib import Path
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def read(p):return json.loads(p.read_bytes())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def data_sha(b):return hashlib.sha256(b).hexdigest()
versions=[]
for version in [1,2]:
    raw=ROOT/f'test-results/runtime-paths-v{version}'
    context=read(raw/'execution-context.json');frozen=read(HERE/f'frozen-v{version}.json')
    assert sha(HERE/f'frozen-v{version}.json')==context['frozen_manifest_sha256']
    source_count=0;binary_count=0;line_endings=[]
    for item in frozen['files']:
        if item['path'].startswith('test-results/'):
            assert sha(ROOT/item['path'])==item['sha256'],item['path'];binary_count+=1
        else:
            blob=subprocess.check_output(['git','show',context['source_revision']+':'+item['path']],cwd=ROOT)
            if data_sha(blob)!=item['sha256']:
                crlf=blob.replace(b'\r\n',b'\n').replace(b'\n',b'\r\n')
                assert data_sha(crlf)==item['sha256'],item['path']
                line_endings.append(item['path'])
            source_count+=1
    versions.append({'version':version,'execution_context':context,'frozen_source_files_verified':source_count,
                     'frozen_binary_files_verified':binary_count,'git_line_ending_reconstruction':line_endings})

def pauses(report):
    if report.get('operation')=='refresh_runtime_paths':return report['pauses']
    assert all(w['added_classes']>0 for w in report['warmups'])
    return report['snapshots']+[w['discarded_snapshot'] for w in report['warmups']]

def audit_case(raw,label,version):
    report=read(raw/(label+'.json'))
    assert not report['edit_authority']
    receipts=[json.loads(s) for s in (raw/(label+'.stdout.jsonl')).read_text().splitlines()]
    assert receipts[0]['stage']=='validated' and receipts[-1]['helper_absent']
    assert receipts[0]['helper_sha256']==sha(raw.parent/'bin/reversebolt_runtime.dll')
    for p in pauses(report):assert p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0
    if version==2:
        assert all(report[k]==receipts[0][k] for k in ['pid','created','session'])
    if report.get('operation')=='refresh_runtime_paths':
        locators=read(raw/(label+'.paths.json')) if (raw/(label+'.paths.json')).exists() else read(raw/'paths.json')
        assert len(locators)==len(report['checks'])
        for locator,check in zip(locators,report['checks']):
            assert locator['scope']==report['scope']
            encoded=json.dumps(locator,separators=(',',':'),ensure_ascii=False).encode()
            assert data_sha(encoded)==check['path_id']
            if check['status']!='read':
                assert check['value'] is None and check['field_address'] is None and check['bytes']==[]
    return report,receipts[-1]

v1=ROOT/'test-results/runtime-paths-v1/mock'
failed=read(v1/'execution.json');assert failed['host_exit']==0
assert failed['calls']==[{'label':'discovery','exit':0},{'label':'refresh-0','exit':1}]
assert 'report identity mismatch' in (v1/'refresh-0.stderr.jsonl').read_text()
first,_=audit_case(v1,'discovery',1);rejected,_=audit_case(v1,'refresh-0',1)
assert 'pid' not in rejected and all(c['status']=='read' for c in rejected['checks'])
assert len(pauses(first))+len(pauses(rejected))==8
assert read(v1/'host.jsonl')=={'starts':8,'stops':8}

raw=ROOT/'test-results/runtime-paths-v2';mock=raw/'mock'
summary=read(mock/'summary.json');assert summary['passed']
assert read(mock/'execution.json')['host_exit']==0
mock_cases=[];total=0
expected={
 'null-root':['null_reference']*4,'generic-type-changed':['type_changed']*4,
 'array-shortened':['array_bounds']*3+['read'],'layout-changed':['type_changed']*4,
 'wrong-root-layout':['root_unavailable'],'missing-field':['field_unavailable']}
for call in summary['calls']:
    label=call['label']
    if label=='wrong-process':
        assert call['exit']==1 and not (mock/(label+'.json')).exists()
        assert (mock/(label+'.stdout.jsonl')).read_text()==''
        continue
    assert call['exit']==0
    report,receipt=audit_case(mock,label,2);total+=len(pauses(report))
    statuses=[c['status'] for c in report.get('checks',[])]
    if label in expected:assert statuses==expected[label]
    elif label!='discovery':assert statuses==['read']*4
    mock_cases.append({'label':label,'statuses':statuses,'pauses':len(pauses(report)),
                       'max_stop_to_resume_us':max(p['stop_to_resume_us'] for p in pauses(report)),
                       'loader_elapsed_ms':receipt['elapsed_ms']})
assert total==summary['pauses']==31
assert read(mock/'host.jsonl')=={'starts':total,'stops':total}
observations=summary['observations'];assert len({r['identity'] for r in observations})==3
reuse=sum(a['address']==b['address'] and a['identity']!=b['identity'] for i,a in enumerate(observations) for b in observations[i+1:])
assert reuse>0

game=raw/'blue-prince';game_summary=read(game/'summary.json');assert game_summary['passed']
discovery,_=audit_case(game,'discovery',2);fresh,_=audit_case(game,'refresh',2)
candidates=read(game/'candidates.json');chosen=read(game/'selected.json')
assert len(candidates)==4096 and len(chosen)==4
assert [c['id'] for c in chosen]==[c['path_id'] for c in fresh['checks']]
selected=[]
for before,after in zip(chosen,fresh['checks']):
    assert not before['edit_authority'] and after['status']=='read'
    step=before['locator']['steps'][-1]
    key=step['member']['field']['declaring'] if step['member']['kind']=='field' else step['owner']
    selected.append({'name':before['name'],'declaring_image':key['image'],'steps':len(before['locator']['steps']),
                     'value_changed':before['value']!=after['value']})
assert all(s['declaring_image']=='mscorlib.dll' for s in selected)
after=read(raw/'game-after.json');assert after['responding'] and after['window'] and not after['helper_present']
assert after['pid']==game_summary['pid'] and after['created']==game_summary['created']
cleanup=read(raw/'cleanup.json');assert cleanup['remaining_owned_processes']==[] and cleanup['lease_released']
desktop=read(raw/'desktop.json');assert desktop['ok'] and not desktop['changed_this_turn']
tests=ROOT/'test-results/runtime-paths-workspace-tests.log';test_text=tests.read_text(encoding='utf-8-sig')
assert 'test result: FAILED' not in test_text
test_count=sum(map(int,re.findall(r'test result: ok. (\d+) passed',test_text)));assert test_count==194
files=[tests]
for version in [1,2]:files+=sorted(p for p in (ROOT/f'test-results/runtime-paths-v{version}').rglob('*') if p.is_file())
result={'audit_passed':True,'versions':versions,'retained_v1_failure':'Refresh response lacked the top-level identity envelope; loader rejected after successful unload. Host exit 0 and balanced 8/8 pauses.',
 'mock_cases':mock_cases,'mock_balanced_pauses':total,'observed_same_address_changed_identity_pairs':reuse,
 'game':game_summary,'selected_game_case_fields':selected,
 'game_discovery':{'catalog_ms':discovery['catalog_ms'],'elapsed_before_json_write_ms':discovery['elapsed_ms'],
    'snapshots':len(discovery['snapshots']),'warmups':len(discovery['warmups']),
    'max_stop_to_resume_us':max(p['stop_to_resume_us'] for p in pauses(discovery)),
    'root_scan_complete':discovery['root_scan_complete'],'graph_complete':discovery['graph_complete']},
 'refresh_catalog_ms':fresh['catalog_ms'],'workspace_tests_passed':test_count,
 'game_after':after,'cleanup':cleanup,'desktop':desktop,'screenshots':0,'gameplay_inputs':0,
 'game_field_write_calls':0,'edit_authority':False,'progress_percent':31,
 'raw_artifacts':[{'path':str(p.relative_to(ROOT)).replace('\\','/'),'sha256':sha(p),'bytes':p.stat().st_size} for p in files]}
(HERE/'results.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
print(json.dumps({'audit_passed':True,'versions':versions,'mock_cases':len(summary['calls']),
                  'mock_balanced_pauses':total,'address_reuse_pairs':reuse,'workspace_tests':test_count,'selected':selected}))
