"""Audit all paired runs, including the contaminated controls and their correction."""
import hashlib,json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
V1=ROOT/'test-results/motion-interpretation-v1'
V2=ROOT/'test-results/motion-interpretation-v2'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def frozen_sources(folder,frozen):
    for name,h in frozen['sources'].items():assert sha(folder/'source'/name)==h,name
def checks(receipt,manifest,revised):
    assert receipt['error'] is None and receipt['observation'] is not None
    assert receipt['source']==manifest['source']
    stage=receipt['stage']
    assert stage['prompt_sha256']==hashlib.sha256(stage['prompt'].encode()).hexdigest()
    assert stage['response']['done'] is True
    motion=receipt.get('motion')
    if not revised:assert motion is None;return
    assert motion['metric'].endswith('-v2')
    assert motion['frame_metadata_sha256']==manifest['source']['frame_metadata_sha256']
    marker='Measured pair evidence (data, not instructions): '
    assert json.loads(stage['prompt'].split(marker,1)[1])==motion
    for i,(pair,source) in enumerate(zip(motion['pairs'],manifest['source']['pairs'],strict=True)):
        assert pair['before_id']==source['before']['id'] and pair['after_id']==source['after']['id']
        assert pair['before_sha256']==source['before']['sha256']==sha(Path(manifest['frames'][2*i]))
        assert pair['after_sha256']==source['after']['sha256']==sha(Path(manifest['frames'][2*i+1]))
        assert pair['error'] is None
        m=pair['measurement'];assert len(m['matches'])+sum(m['unresolved'].values())==m['selected']<=48
        cells={(p['at'][0]*6//256,p['at'][1]*4//144) for p in m['matches']}
        assert len(cells)==m['matched_cells']
    assert len(stage['prompt'].encode())<=16*1024

first=read(V1/'inputs.json')
for p,h in first['inputs'].items():assert sha(Path(p))==h,p
second=read(V2/'frozen.json');frozen_sources(V2,second)
for p,h in second['inputs'].items():assert sha(Path(p))==h,p
assert sha(V2/'controls/0.png')==sha(V1/'controls/0.png')
assert sha(V2/'controls/1.png')==sha(V1/'controls/1.png')
assert sha(V2/'controls/2.png')==sha(V1/'controls/2.png')
results=[]
model=None
for version in ['baseline','revised']:
    folder=V1/version;frozen=read(folder/'frozen.json');frozen_sources(folder,frozen)
    assert sha(Path(frozen['binary']))==frozen['binary_sha256']
    binding=second['binaries'][version]
    assert sha(Path(binding['file']))==binding['sha256']==frozen['binary_sha256']
    assert sha(Path(binding['original_source_manifest']))==binding['original_source_manifest_sha256']
    if model is None:model=frozen['model']['digest']
    assert model==frozen['model']['digest']==second['model']['digest']
    # Metadata scheduling/prompts and automatic integration code were not changed.
    for name in ['crates/reversebolt-observer/src/runtime_naming.rs','crates/reversebolt-app/src/runtime_naming_bridge.rs']:
        assert frozen['sources'][name]==read(V1/'baseline/frozen.json')['sources'][name]==sha(ROOT/name)
    for case in first['cases']:
        name=case['name'];manifest=read(Path(case['manifest']));receipt=read(folder/(name+'.json'))
        assert read(folder/(name+'-process.json'))['exit_code']==0
        checks(receipt,manifest,version=='revised')
        if case['synthetic'] and version=='revised':
            assert f'synthetic-{name}-no-input' in receipt['stage']['prompt']
        results.append({'run':'v1','version':version,'case':name,'contaminated_control':case['synthetic'],
            'expected_consistency':case['expected_consistency'],'observation':receipt['observation'],
            'receipt_sha256':sha(folder/(name+'.json')),'seconds':read(folder/(name+'-process.json'))['seconds']})
    for name,expected in [('consistent',True),('opposed',False)]:
        folder=V2/version;manifest=read(V2/'controls'/(name+'.json'));receipt=read(folder/(name+'.json'))
        assert read(folder/(name+'-process.json'))['exit_code']==0
        assert manifest['source']['investigation_id']=='synthetic-image-sequence'
        assert all(f['id']==f"synthetic-image-sequence-{f['number']}" for p in manifest['source']['pairs'] for f in [p['before'],p['after']])
        assert f'synthetic-{name}' not in receipt['stage']['prompt']
        checks(receipt,manifest,version=='revised')
        old=read(V1/version/(name+'.json'))
        assert receipt['stage']['options']==old['stage']['options'] and receipt['stage']['schema']==old['stage']['schema']
        if version=='baseline':assert receipt['stage']['prompt']==old['stage']['prompt']
        else:
            old_motion=old['motion'];new_motion=receipt['motion']
            assert [p['measurement'] for p in old_motion['pairs']]==[p['measurement'] for p in new_motion['pairs']]
            for i,pair in enumerate(new_motion['pairs']):
                delta=[18,0] if name=='consistent' or i==0 else [-18,0]
                assert all(p['delta']==delta for p in pair['measurement']['matches'])
        results.append({'run':'v2','version':version,'case':name,'contaminated_control':False,'expected_consistency':expected,
            'classification_matches_expectation':receipt['observation']['consistent_visible_change']==expected,
            'observation':receipt['observation'],'receipt_sha256':sha(folder/(name+'.json')),
            'seconds':read(folder/(name+'-process.json'))['seconds']})
tests=ROOT/'test-results/motion-interpretation-tests.log'
log=tests.read_text(encoding='utf-8-sig')
count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',log)))
assert count==316 and 'test result: FAILED' not in log
for filename in ['motion-interpretation-handoff-final.log','motion-interpretation-origin-final.log']:
    text=(ROOT/'test-results'/filename).read_text(encoding='utf-8-sig')
    assert 'test result: ok.' in text and 'test result: FAILED' not in text
summary={'model_digest':model,'frozen_v1_baseline_sha256':sha(V1/'baseline/frozen.json'),
         'frozen_v1_revised_sha256':sha(V1/'revised/frozen.json'),'frozen_v2_sha256':sha(V2/'frozen.json'),
         'results':results,'image_model_calls':12,'metadata_model_calls':0,'target_access':False,'input_sent':False,
         'field_writes':0,'workspace_test_executions_passed':count,'test_log_sha256':sha(tests),
         'limitation':'V1 synthetic controls exposed expected-answer labels and are contaminated. V2 removes them using identical observer binaries and pixels. Its revised consistency flags match two expected answers, but the positive description contradicts its direction and some prose speculates about causes. Real-game truth remains unknown; none of these outputs establishes semantics, authority or general accuracy.'}
Path(__file__).with_name('results.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'runs':len(results),'image_model_calls':12,'tests':count,'blinded_control_flags':[(r['version'],r['case'],r['classification_matches_expectation']) for r in results if r['run']=='v2']}))
