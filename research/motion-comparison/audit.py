"""Audit frozen receipt replay and unchanged observations/images without execution."""
import hashlib,json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/motion-comparison-v1'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
frozen=read(OUT/'frozen.json')
for p,h in frozen['sources'].items():assert sha(OUT/'source'/p)==h,p
for p,h in frozen['inputs'].items():assert sha(Path(p))==h,p
assert sha(Path(frozen['binary']))==frozen['binary_sha256']
assert read(OUT/'process.json')['exit_code']==0
result=read(OUT/'result.json');requests=read(OUT/'cases.json')
assert result['manifest_sha256']==sha(OUT/'cases.json')
assert result['model_calls']==result['field_writes']==0
assert not result['target_access'] and not result['input_sent']
summaries=[]
for request,case in zip(requests,result['results'],strict=True):
    original=read(Path(request['file']))
    assert case['name']==request['name'] and case['receipt_sha256']==request['sha256']==sha(Path(request['file']))
    assert case['source']==original['source']
    assert case['motion']==original.get('motion')
    assert case['original_observation']==original['observation']
    c=case['comparison']
    if request['name'].startswith('baseline-'):
        assert c is None
    else:
        assert c['policy']=='selected_patch_displacement_relation_v1' and c['scope']=='discovery_activity_pairs'
        assert c['measurement_dimensions']==[256,144]
        assert len(c['source_sha256'])==len(c['motion_summary_sha256'])==64
        for p,m in zip(c['pairs'],case['motion']['pairs'],strict=True):
            for key in ['before_id','before_sha256','after_id','after_sha256']:assert p[key]==m[key]
            measured=m['measurement'];consensus=measured['consensus']
            assert p['selected']==measured['selected'] and p['matched']==len(measured['matches'])
            assert p['matched_cells']==measured['matched_cells']
            assert p['displacement']==(consensus['delta'] if consensus else None)
        if request['name'].endswith(('hyperbolica','manifold')):
            assert c['relation']=='insufficient_support'
            assert all(p['displacement'] is None for p in c['pairs'])
        elif request['name'].endswith('-consistent'):
            assert c['relation']=='matching_displacement'
            assert [p['displacement'] for p in c['pairs']]==[[18,0],[18,0]]
            assert '18 pixels right, then 18 pixels right' in c['description']
            assert 'leftward' in case['original_observation']['description'] # Retained model contradiction.
        else:
            assert c['relation']=='opposed_direction'
            assert [p['displacement'] for p in c['pairs']]==[[18,0],[-18,0]]
            assert '18 pixels right, then 18 pixels left' in c['description']
    summaries.append({'case':case['name'],'relation':c['relation'] if c else None,
                      'description':c['description'] if c else None,'original_observation':case['original_observation'],
                      'source_sha256':c['source_sha256'] if c else None,'motion_summary_sha256':c['motion_summary_sha256'] if c else None})
tests=ROOT/'test-results/motion-comparison-tests.log';text=tests.read_text(encoding='utf-8-sig')
count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',text)))
assert count==320 and 'test result: FAILED' not in text
summary={'frozen_sha256':sha(OUT/'frozen.json'),'result_sha256':sha(OUT/'result.json'),'cases':summaries,
         'unchanged_input_files':len(frozen['inputs']),'new_model_calls':0,'target_access':False,'input_sent':False,'field_writes':0,
         'workspace_test_executions_passed':count,'test_log_sha256':sha(tests),
         'limitation':'Deterministic descriptions of retained selected-patch displacement. Both genuine game cases lack support. Original model prose and flags remain unchanged; this establishes no gameplay effect, physical velocity or trial outcome.'}
Path(__file__).with_name('results.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'cases':len(summaries),'tests':count,'unchanged_input_files':len(frozen['inputs'])}))
