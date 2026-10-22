"""Audit provenance and expose changed model claims without scoring accuracy."""
import hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'test-results/appearance-context-v1'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
frozen=read(OUT/'source-frozen.json')
for p,s in frozen['sources'].items():assert sha(ROOT/p)==sha(OUT/'source'/p)==s,p
for b in frozen['binaries'].values():assert sha(Path(b['file']))==b['sha256']
for manifest in ['inputs-frozen.json','manifests-frozen.json']:
    for p,s in read(OUT/manifest).items():assert sha(Path(p))==s,p
cases=[]
for i in range(2):
    baseline=read(OUT/f'case-{i}-baseline-receipt.json');augmented=read(OUT/f'case-{i}-augmented-receipt.json')
    request=read(OUT/f'case-{i}-augmented.json');summary=request['appearances']
    assert baseline['source']==augmented['source']==request['source']
    assert baseline['motion']==augmented['motion']
    assert baseline.get('appearances') is None and augmented['appearances']==summary
    assert all(a['pairs'][1]['status']=='not_measured' and a['pairs'][1]['displacement'] is None for a in summary['appearances'])
    assert len([a for a in summary['appearances'] if a['pairs'][0]['status']=='matched'])==1
    for r in [baseline,augmented]:
        if r['stage']:
            assert r['stage']['prompt_sha256']==hashlib.sha256(r['stage']['prompt'].encode()).hexdigest()
        tag='baseline' if r is baseline else 'augmented'
        models=read(OUT/f'case-{i}-{tag}-model.json')['models']
        assert any(m['name']=='qwen3.8:latest' and m['digest']=='22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643' for m in models)
    cases.append({'case':i,'selected_frame_numbers':[[p['before']['number'],p['after']['number']] for p in request['source']['pairs']],
        'outcomes':[a['pairs'] for a in summary['appearances']],
        'baseline':baseline['observation'],'augmented':augmented['observation'],
        'baseline_error':baseline['error'],'augmented_error':augmented['error'],
        'baseline_receipt_sha256':sha(OUT/f'case-{i}-baseline-receipt.json'),
        'augmented_receipt_sha256':sha(OUT/f'case-{i}-augmented-receipt.json'),
        'global_motion_unchanged':True,'accuracy':'unknown'})
result={'source_manifest_sha256':sha(OUT/'source-frozen.json'),'source_files':len(frozen['sources']),
    'input_files':len(read(OUT/'inputs-frozen.json')),'image_interpretation_calls':4,
    'new_object_observation_calls':0,'native_target_access':False,'cases':cases}
with (OUT/'audit.json').open('x',encoding='utf-8') as f:f.write(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
