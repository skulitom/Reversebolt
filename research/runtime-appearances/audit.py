"""Integration assertions; deliberately no accuracy or gameplay success score."""
import hashlib, json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-appearances-v1'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f: return hashlib.file_digest(f, 'sha256').hexdigest()
frozen = read(OUT / 'source-frozen.json')
for name, digest in frozen['sources'].items():
    assert sha(ROOT/name) == sha(OUT/'source'/name) == digest, name
for name, digest in read(OUT / 'inputs-frozen.json').items(): assert sha(Path(name)) == digest, name
summary = []
for i in range(2):
    result_path = OUT/f'case-{i}'/'result.json'; result = read(result_path)
    case = read(OUT/f'case-{i}.json'); source = case['frames'][1]['reference']
    assert not result['native_target_access'] and result['new_captures'] == result['inputs_sent'] == result['field_writes'] == 0
    assert result['evidence']['evidence'] == result['without_observations']['evidence']
    baseline = result['without_observations']['automatic_region_motion']
    assert baseline['selection']['regions'] == []
    assert baseline['appearance_sequences']['report']['attempted_pairs'] == 0
    tracks = result['visual_tracks']; assert len(tracks) == result['publication']['recorded']
    for track in tracks:
        assert len(track['observations']) == 15
        assert all(o is None for n,o in enumerate(track['observations']) if n != 1)
        observation = track['observations'][1]['input']
        assert observation['frame_id'] == source['id'] and observation['image_sha256'] == source['sha256']
        assert observation['quality'] == 'provisional'
    files = list((OUT/f'case-{i}'/'model-observations').glob('*.json')); assert len(files) == 1
    receipt = read(files[0]); assert receipt['sequence']['frames'] == [f['reference'] for f in case['frames'][:2]]
    p = receipt['pass']; receipts = ([p['proposal']] if p['proposal'] else []) + [r['receipt'] for r in p['reviews']]
    assert len(receipts) <= 7
    assert all(r['model_digest'] == '22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643' for r in receipts)
    automatic = result['evidence']['automatic_region_motion']; sequence = automatic['appearance_sequences']['report']
    assert sequence['attempted_pairs'] <= 12
    for region in automatic['selection']['regions']:
        assert region['before_id'] == source['id'] and region['after_id'] == case['frames'][2]['reference']['id']
    summary.append({'name':case['name'],'result_sha256':sha(result_path),'publication':result['publication'],
        'completed_model_receipts':len(receipts),'labels':[t['label'] for t in tracks],
        'regions':len(automatic['selection']['regions']),'omitted':automatic['selection']['omitted_observations'],
        'attempted_pairs':sequence['attempted_pairs'],
        'series':[{'accepted_steps':s['accepted_steps'],'stop':s['stop'],
                   'statuses':[x['measurement']['status'] if x['measurement'] else x['error'] for x in s['steps']]} for s in sequence['series']],
        'base_evidence_unchanged':True,'accuracy':'unknown'})
output = {'source_manifest_sha256':sha(OUT/'source-frozen.json'),'inputs_manifest_sha256':sha(OUT/'inputs-frozen.json'),
          'source_files':len(frozen['sources']),'input_files':len(read(OUT/'inputs-frozen.json')),'cases':summary,
          'limitation':'Previously exposed saved images; no new native game evidence or verified control.'}
with (OUT/'audit.json').open('x',encoding='utf-8') as f: f.write(json.dumps(output,indent=2)+'\n')
print(json.dumps(output,indent=2))
