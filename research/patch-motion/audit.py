"""Check all frozen runs, provenance correction and unchanged legacy measurements."""
import hashlib
import json
from pathlib import Path
import re
from collections import Counter

ROOT=Path(__file__).resolve().parents[2]
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f: return hashlib.file_digest(f,'sha256').hexdigest()

runs=[]
for version in ['v1','v2','v3']:
    folder=ROOT/('test-results/patch-motion-'+version)
    frozen=read(folder/'frozen.json')
    for p,digest in frozen['sources'].items(): assert sha(folder/'source'/p)==digest,p
    for p,digest in frozen['inputs'].items(): assert sha(Path(p))==digest,p
    assert sha(Path(frozen['binary']))==frozen['binary_sha256']
    assert read(folder/'process.json')['exit_code']==0
    result=read(folder/'result.json')
    assert sha(folder/'cases.json')==result['manifest_sha256']
    assert result['new_model_calls']==result['field_writes']==0
    assert not result['target_access'] and not result['input_sent']
    cases=read(folder/'cases.json')
    for request,case in zip(cases,result['cases'],strict=True):
        assert all(case['old_difference_exact_matches'])
        for i,interval in enumerate(case['patch_motion']):
            report=interval['measurement']
            assert interval['error'] is None
            a,b=request['frames'][i:i+2]
            assert interval['before_sha256']==a['reference']['sha256']==sha(Path(a['file']))
            assert interval['after_sha256']==b['reference']['sha256']==sha(Path(b['file']))
            assert len(report['patches'])==21
            matched=[p for p in report['patches'] if p['status']=='matched']
            assert len(matched)==report['matched_patches']
            assert report['matched_patches']==report['zero_displacement_patches']+report['nonzero_displacement_patches']
            for p in report['patches']:
                assert (p['displacement'] is not None)==(p['status']=='matched')
            assert report['consensus'] is None # Known-case result, not an acceptance target.
    runs.append((frozen,result,folder))
assert len({f['binary_sha256'] for f,_,_ in runs})==1
for name in ['crates/reversebolt-vision/src/patch_motion.rs','crates/reversebolt-vision/src/response.rs',
             'crates/reversebolt-app/src/play_evidence.rs','crates/reversebolt-app/src/runtime_test_bridge.rs']:
    assert len({f['sources'][name] for f,_,_ in runs})==1
assert runs[1][1]==runs[2][1] # Final source freeze includes only extra tests/descriptions.
fixture=read(ROOT/'test-results/planned-input-v1/case/data/runtime-tests/080b5643-5718-494f-9ed6-751bf8059402.json')
assert fixture['candidate']['locator']['root']['declaring']['image']=='OwnedPathFixture'
assert read(ROOT/'test-results/comparison-planner-v1/discovery.json')['state']['live']['process']['name']=='ManifoldGarden.exe'
old=runs[0][1]['cases']
new=runs[2][1]['cases']
assert old[0]==new[0]
for i in range(1,4):
    corrected=dict(old[i],name=new[i]['name'])
    assert corrected==new[i]
    assert old[i]['name'].startswith('Manifold Garden retained')
    assert new[i]['name'].startswith('Owned striped fixture retained')
summaries=[]
for case in new:
    reports=[i['measurement'] for i in case['patch_motion']]
    statuses=Counter(p['status'] for r in reports for p in r['patches'])
    summaries.append({'name':case['name'],'intervals':len(reports),'patches':21*len(reports),
                      'matched':statuses['matched'],'nonzero_matches':sum(r['nonzero_displacement_patches'] for r in reports),
                      'statuses':dict(statuses),'consensus_intervals':sum(r['consensus'] is not None for r in reports)})
tests=ROOT/'test-results/patch-motion-tests.log'
log=tests.read_text(encoding='utf-8-sig')
count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',log)))
assert count==313 and 'test result: FAILED' not in log
summary={'runs':[{'version':p.name,'frozen_sha256':sha(p/'frozen.json'),'result_sha256':sha(p/'result.json')} for _,_,p in runs],
         'same_archived_binary_all_runs':True,'final_source_replay_equals_v2':True,
         'source_correction':'V1 mislabeled three owned striped-fixture cases as Manifold Garden. V2/V3 retain those images correctly labeled and add genuine Manifold Garden observations.',
         'images':48,'intervals':43,'legacy_difference_values_exactly_preserved':43,'cases':summaries,
         'new_model_calls':0,'target_access':False,'input_sent':False,'field_writes':0,
         'workspace_test_executions_passed':count,'test_log_sha256':sha(tests),
         'limitation':'Low known-game match coverage, no broad motion consensus, no ground-truth flow map or verified gameplay meaning. One epoch-guard test also runs in the example target.'}
Path(__file__).with_name('results.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps(summary))
