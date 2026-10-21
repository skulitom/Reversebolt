"""Re-measure retained images only. No process access, input or image review."""
import hashlib,json,statistics
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/runtime-test-protocol-v1'
def read(path):return json.loads(path.read_text(encoding='utf-8-sig'))
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
measurements=read(OUT/'measurements.json')
assert measurements['metric']=='rgb601-triangle640x360-crop24,40,592,296-gaussian1-mad-v1'
assert len(measurements['pairs'])==30
results=[]
for candidate in range(2):
    case=ROOT/f'test-results/runtime-alternatives-v1/candidate-{candidate}'
    previous=read(case/'movement-measurement.json')
    conditions={}
    for i,name in enumerate(['baseline','changed','restored']):
        pairs=measurements['pairs'][candidate*15+i*5:candidate*15+i*5+5]
        names=[f'{name}-idle.png',f'{name}-0.png']+[f'{name}-{n}.png' for n in range(1,5)]
        for j,pair in enumerate(pairs):
            assert [Path(p).name for p in pair['paths']]==names[j:j+2]
            for path,digest in zip(pair['paths'],pair['hashes']):
                p=Path(path)
                assert p.resolve().parent==(case/'frames').resolve()
                assert sha(p)==digest==previous['frame_sha256'][p.name]
        conditions[name]={'stationary':pairs[0]['difference'],
            'response_median':statistics.median(p['difference'] for p in pairs[1:])}
    b,c,r=[conditions[n] for n in ['baseline','changed','restored']]
    visual_criterion=(all(s['response_median']>=5 and s['response_median']>=5*s['stationary'] for s in (b,r))
        and c['response_median']<=max(2,.2*b['response_median']) and r['response_median']>=.5*b['response_median'])
    assert visual_criterion==previous['criteria_passed']
    results.append({'candidate':candidate,'conditions':conditions,'visual_suppression_criterion':visual_criterion,
        'previous_measurement_sha256':sha(case/'movement-measurement.json')})
sources=['crates/reversebolt-core/src/runtime_tests.rs','crates/reversebolt-core/tests/runtime_tests.rs',
         'crates/reversebolt-vision/src/response.rs','crates/reversebolt-vision/examples/response_audit.rs',
         'research/runtime-test-protocol/audit.py']
result={'metric':measurements['metric'],'retrospective_only':True,'new_captures':0,'game_input_actions':0,
        'retained_images':36,'image_pairs':30,'results':results,'source_sha256':{p:sha(ROOT/p) for p in sources},
        'measurement_sha256':sha(OUT/'measurements.json'),
        'limitation':'This is a retrospective metric-port check on exploratory images. Rust and Pillow blur differ; identical numerical output is not claimed. These captures have no engine-owned input windows and cannot pass the new full protocol. The synthetic protocol tests do not establish real-game delivery, semantics or generalization. No new candidate is promoted.'}
(OUT/'audit.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
Path(__file__).with_name('results.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
print(json.dumps({'retained_images':36,'pairs':30,'same_visual_decisions':True,'full_protocol_on_game':'not tested'}))
