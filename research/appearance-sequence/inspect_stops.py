"""Post-hoc diagnostic only: inspect saved state at sequence stopping points.

The oracle never enters product matching. This does not score persistent identity
or convert a matcher failure into an accepted correspondence.
"""
import bisect, hashlib, json
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).resolve().parents[2]
RUN=ROOT/'test-results/appearance-sequence-v1'
OUT=RUN/'stop-diagnostic'
OUT.mkdir()
GAME=ROOT/'test-results/visual-objects-v1/tetris-1-attempt-01'
IMAGES=GAME/'engine/captures/726934a1-2e14-4fa9-948b-e0d7ba7be155'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(p,v):
    with p.open('x',encoding='utf-8') as f:f.write(json.dumps(v,indent=2)+'\n')
paths=[GAME/n for n in ['states.json','oracle.json','final-status.json']]
paths += [IMAGES/f'frame-{n:03}.png' for n in [2,3,4,5]]
paths += [RUN/'result.json',RUN/'source-frozen.json',Path(__file__)]
hashes={str(p):sha(p) for p in paths};save(OUT/'inputs-frozen.json',hashes)
(OUT/'diagnostic.py').write_bytes(Path(__file__).read_bytes())
frozen=read(RUN/'source-frozen.json');assert all(sha(ROOT/p)==h for p,h in frozen['sources'].items())
states=read(GAME/'states.json');oracle=read(GAME/'oracle.json');times=[o['time_ms'] for o in oracle]
assert all(a<=b for a,b in zip(times,times[1:]))
live=read(GAME/'final-status.json')['state']['live'];refs={f['id']:f for f in live['frames']}
rows=[]
for n in [2,3,4,5]:
    s=next(v['state']['live']['latest'] for v in states if v['state']['live']['sample_count']==n)
    start=s['started_unix_ms'];end=start+s['screen_capture_ms']
    lo,hi=bisect.bisect_right(times,start)-1,bisect.bisect_left(times,end)
    bracket=oracle[lo:hi+1] if 0<=lo<=hi<len(oracle) else []
    valid=bool(bracket) and all(b['time_ms']-a['time_ms']<=200 for a,b in zip(bracket,bracket[1:])) and all(o['response_ms']-o['request_ms']<=200 for o in bracket)
    stable=valid and all(o['objects']==bracket[0]['objects'] and o['viewport']==bracket[0]['viewport'] and not any(o['objects']['pending_redraw'].values()) for o in bracket)
    row={'frame':n,'capture_ms':[start,end],'oracle_bracket':[lo,hi],'stable_recorded_geometry':stable}
    if stable:row.update(current=bracket[0]['current'],next=bracket[0]['next'],landed_cells=len(bracket[0]['objects']['landed']))
    rows.append(row)
case=read(RUN/'result.json')['cases'][5];comparisons=[]
for series in case['appearance_sequences']['report']['series']:
    origin=refs[series['source']['before_id']];r=series['source']['region']
    for step in series['steps']:
        if step['displacement_from_origin']!=[0,0]:continue
        target=refs[step['after_id']]
        def pixels(frame):
            path=IMAGES/f"frame-{frame['number']:03}.png";assert sha(path)==frame['sha256']
            with Image.open(path) as image:return image.convert('RGB').crop((r['x'],r['y'],r['x']+r['width'],r['y']+r['height'])).tobytes()
        a,b=pixels(origin),pixels(target)
        comparisons.append({'source_sha256':series['source']['source_sha256'],'origin':origin['number'],'target':target['number'],
            'region':r,'identical_rgb_bytes':a==b,'maximum_channel_difference':max(abs(x-y) for x,y in zip(a,b,strict=True))})
assert all(sha(Path(p))==h for p,h in hashes.items())
output={'scope':'Post-hoc known-case stop diagnostic; no identity or causal validation, no failed match reclassified.',
    'oracle_sent_to_product':False,'algorithm_changed':False,'recorded_states':rows,'stationary_pixel_checks':comparisons,
    'inputs_sha256':sha(OUT/'inputs-frozen.json')}
save(OUT/'result.json',output);print(json.dumps(output))
