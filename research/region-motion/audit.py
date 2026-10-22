"""Check generated displacement truth and preserve unknown genuine-game regions."""
import hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/region-motion-v1'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
frozen=read(OUT/'source-frozen.json')
help_changes=[]
for name,value in frozen['sources'].items():
    assert sha(OUT/'source'/name)==value
    if sha(ROOT/name)!=value:
        # Only the public help wording was clarified after the replay. No
        # measurement, API schema or executable control path changed.
        assert name=='crates/reversebolt-mcp/src/main.rs'
        prior=(OUT/'source'/name).read_text(encoding='utf-8')
        before='Invalid geometry/epochs/hashes or an out-of-image region produce explicit per-pair errors, never invented zero motion.'
        after='Region measurement failures produce explicit per-pair errors, never invented zero motion. The initial saved-frame verification still rejects the whole request if a source capture is unavailable or corrupted.'
        assert prior.count(before)==1
        assert prior.replace(before,after)==(ROOT/name).read_text(encoding='utf-8')
        help_changes.append(name)
for name,value in frozen['binaries'].items(): assert sha(Path(name))==value
inputs=read(OUT/'inputs-frozen.json')
for name,value in inputs.items():assert sha(Path(name))==value
result=read(OUT/'result.json')
assert result['manifest_sha256']==sha(OUT/'cases.json')
assert result['target_access'] is False and result['new_model_calls']==0
truth=read(OUT/'controls/truth.json')
old=read(ROOT/'test-results/feature-motion-v1/result.json')
rows=[]
for case in result['cases']:
    assert all(case['old_difference_exact_matches'])
    region=case['region_motion']; pairs=region['intervals']
    known=next((t for t in truth if t['name']==case['name']),None)
    if known:
        assert len(pairs)==1
        pair=pairs[0]
        assert (pair['error'] is not None)==known['measurement_error']
        displacement=None
        if pair['measurement'] is not None:
            consensus=pair['measurement']['measurement']['consensus']
            if consensus is not None:displacement=consensus['displacement']
        assert displacement==known['region_normalized_consensus'],(case['name'],displacement)
        if known['region_normalized_consensus'] is not None:
            assert case['patch_motion'][0]['measurement']['consensus']['displacement']==[0,0]
        accuracy='constructed image-shift regression only'
    else:
        prior=next(c for c in old['cases'] if c['name']==case['name'])
        assert prior['evidence']==case['evidence']
        assert prior['patch_motion']==case['patch_motion']
        accuracy='unknown; no genuine region ground truth'
    rows.append({'name':case['name'],'region':region['region'],'source_sha256':region['source_sha256'],
        'intervals':len(pairs),'errors':sum(p['error'] is not None for p in pairs),
        'consensus_displacements':[p['measurement']['measurement']['consensus']['displacement'] if p['measurement'] and p['measurement']['measurement']['consensus'] else None for p in pairs],
        'matched_patches':[p['measurement']['measurement']['matched_patches'] if p['measurement'] else None for p in pairs],
        'accuracy_scope':accuracy})
summary={'artifact_root':str(OUT.relative_to(ROOT)),'source_frozen_sha256':sha(OUT/'source-frozen.json'),
    'result_sha256':sha(OUT/'result.json'),'source_files_verified':len(frozen['sources']),'input_files_unchanged':len(inputs),
    'process':read(OUT/'process.json'),'cases':rows,'full_image_comparisons_unchanged':28,
    'new_captures':0,'model_calls':0,'target_access':False,'field_writes':0,
    'current_help_only_changes':help_changes,
    'limitation':'No object tracking, gameplay semantics, physical-speed accuracy or independent-game acceptance.'}
Path(__file__).with_name('results.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps(summary,indent=2))
