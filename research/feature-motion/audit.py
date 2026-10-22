"""Audit frozen correspondence results without new image, game or model execution."""
import hashlib
import json
from pathlib import Path
from collections import Counter
import re

ROOT=Path(__file__).resolve().parents[2]
RUN=ROOT/'test-results/feature-motion-v1'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f: return hashlib.file_digest(f,'sha256').hexdigest()

frozen=read(RUN/'frozen.json')
for name,digest in frozen['sources'].items(): assert sha(RUN/'source'/name)==digest,name
for name,digest in frozen['inputs'].items(): assert sha(Path(name))==digest,name
for binary in frozen['binaries'].values(): assert sha(Path(binary['file']))==binary['sha256']
for name in ['result','transform-result']: assert read(RUN/(name+'-process.json'))['exit_code']==0
result=read(RUN/'result.json')
transformed=read(RUN/'transform-result.json')
assert result['manifest_sha256']==sha(RUN/'cases.json')
assert transformed['manifest_sha256']==sha(RUN/'transforms.json')
old=read(ROOT/'test-results/patch-motion-v3/result.json')
manifest=read(RUN/'cases.json')

def validate(report):
    assert report['metric'].endswith('-v2')
    patches=report['patches']
    assert len(patches)==report['selection']['selected_patches']<=48
    cells=Counter((p['anchor'][0]*6//256,p['anchor'][1]*4//144) for p in patches)
    assert len(cells)==report['selection']['occupied_cells']
    assert all(n<=2 for n in cells.values())
    for i,a in enumerate(patches):
        assert a['corner_score']>=16
        for b in patches[i+1:]: assert sum((x-y)**2 for x,y in zip(a['anchor'],b['anchor']))>=144
        assert (a['displacement'] is not None)==(a['status']=='matched')
        if a['displacement'] is None: assert a['error_rms'] is a['brightness_change'] is None
    matched=[p for p in patches if p['displacement'] is not None]
    assert len(matched)==report['matched_patches']
    assert sum(p['displacement']==[0,0] for p in matched)==report['zero_displacement_patches']
    assert len(matched)-report['zero_displacement_patches']==report['nonzero_displacement_patches']
    assert len({(p['anchor'][0]*6//256,p['anchor'][1]*4//144) for p in matched})==report['matched_cells']
    if report['consensus']:
        c=report['consensus']
        support=[p for p in matched if all(abs(x-y)<=1 for x,y in zip(p['displacement'],c['displacement']))]
        cells={(p['anchor'][0]*6//256,p['anchor'][1]*4//144) for p in support}
        assert len(support)==c['supporting_patches']>=10 and len(support)*4>=len(matched)*3
        assert len(cells)==c['supporting_cells']>=6
        assert len({x for x,y in cells})>=2 and len({y for x,y in cells})>=2

summaries=[]
for case,prior,request in zip(result['cases'],old['cases'],manifest,strict=True):
    assert case['name']==prior['name']==request['name']
    assert case['evidence']==prior['evidence'] and case['frame_metadata_sha256']==prior['frame_metadata_sha256']
    assert all(case['old_difference_exact_matches'])
    reports=[]
    keys=[]
    for i,(interval,context) in enumerate(zip(case['patch_motion'],case['evidence']['intervals'],strict=True)):
        assert interval['error'] is None
        assert interval['before_sha256']==request['frames'][i]['reference']['sha256']
        assert interval['after_sha256']==request['frames'][i+1]['reference']['sha256']
        report=interval['measurement']
        validate(report)
        reports.append(report)
        if context['exposure']['kind']=='key':
            keys.append({'key':context['exposure']['value'],'selected':len(report['patches']),
                         'matched':report['matched_patches'],'matched_cells':report['matched_cells'],
                         'nonzero':report['nonzero_displacement_patches'],'consensus':report['consensus']})
    statuses=Counter(p['status'] for r in reports for p in r['patches'])
    summaries.append({'name':case['name'],'intervals':len(reports),
        'old_grid_matched':sum(i['measurement']['matched_patches'] for i in prior['patch_motion']),
        'old_grid_selected':sum(len(i['measurement']['patches']) for i in prior['patch_motion']),
        'new_selected':sum(len(r['patches']) for r in reports),'new_matched':statuses['matched'],
        'new_nonzero':sum(r['nonzero_displacement_patches'] for r in reports),'statuses':dict(statuses),
        'consensus_intervals':sum(r['consensus'] is not None for r in reports),
        'consensus_displacements':[r['consensus']['displacement'] for r in reports if r['consensus']],
        'key_intervals':keys})
checks=[]
assert len(transformed['cases'])==8
for i,case in enumerate(transformed['cases']):
    expected=[[0,0],[3,-2],[18,8],[-24,-10]][i%4]
    assert case['expected_displacement']==expected and case['brightness_added']==20
    report=case['measurement']
    validate(report)
    matched=[p for p in report['patches'] if p['displacement'] is not None]
    errors=[[x-y for x,y in zip(p['displacement'],expected)] for p in matched]
    assert errors==[p['error'] for p in case['accepted_errors']]
    wrong=sum(any(abs(x)>1 for x in error) for error in errors)
    assert wrong==case['wrong_matches_over_one_pixel']==0
    assert len(matched)>0
    checks.append({'source':case['source'],'expected':expected,'selected':len(report['patches']),
                   'matched':len(matched),'exact':sum(error==[0,0] for error in errors),'wrong_over_one_pixel':wrong})
log=ROOT/'test-results/feature-motion-tests.log'
text=log.read_text(encoding='utf-8-sig')
count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',text)))
assert count==314 and 'test result: FAILED' not in text
summary={'frozen_sha256':sha(RUN/'frozen.json'),'result_sha256':sha(RUN/'result.json'),
         'transform_result_sha256':sha(RUN/'transform-result.json'),'cases':summaries,'transforms':checks,
         'legacy_difference_values_exactly_preserved':43,'workspace_test_executions_passed':count,
         'test_log_sha256':sha(log),'new_model_calls':0,'target_access':False,'input_sent':False,'field_writes':0,
         'limitation':'Known-image regression and controlled transforms, not real-game flow truth or semantic verification. Selection denominators differ; most consensus is near zero in quiet intervals. One epoch test also runs in an example target.'}
Path(__file__).with_name('results.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'cases':len(summaries),'transforms':len(checks),'accepted_transform_matches':sum(c['matched'] for c in checks),
                  'wrong_over_one_pixel':sum(c['wrong_over_one_pixel'] for c in checks),'test_executions':count}))
