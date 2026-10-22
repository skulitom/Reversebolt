"""Exploratory scoring only, after the matcher and primary replay were frozen.

Uses the original visual-objects timing/pixel checks on captures 2 and 3.
No oracle field, image annotation or score is sent back to product inference.
No persistent object identity or predeclared real-game acceptance is asserted.
"""
import bisect, hashlib, json
from pathlib import Path
from PIL import Image, ImageColor

ROOT = Path(__file__).resolve().parents[2]
RUN = ROOT / 'test-results/region-appearance-v1'
OUT = RUN / 'saved-geometry-check'
OUT.mkdir()
GAME = ROOT / 'test-results/visual-objects-v1/tetris-1-attempt-01'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f: return hashlib.file_digest(f, 'sha256').hexdigest()
def save(p, value):
    with p.open('x', encoding='utf-8') as f: f.write(json.dumps(value, indent=2) + '\n')
paths = [GAME / n for n in ['run.json', 'states.json', 'oracle.json', 'final-status.json']]
paths += [RUN / 'result.json', RUN / 'source-frozen.json', Path(__file__), ROOT / 'research/visual-objects/score.py']
images = GAME / 'engine/captures/726934a1-2e14-4fa9-948b-e0d7ba7be155'
paths += [images / f'frame-{n:03}.png' for n in [2, 3]]
hashes = {str(p): sha(p) for p in paths}
save(OUT / 'inputs-frozen.json', hashes)
(OUT / 'scorer.py').write_bytes(Path(__file__).read_bytes())
source = read(RUN / 'source-frozen.json')
assert all(sha(ROOT / p) == h for p, h in source['sources'].items())
run = read(GAME / 'run.json')
states = read(GAME / 'states.json')
oracle = read(GAME / 'oracle.json')
live = read(GAME / 'final-status.json')['state']['live']
times = [o['time_ms'] for o in oracle]
assert all(a <= b for a, b in zip(times, times[1:]))
frames = {}
for number in [2, 3]:
    ref = next(f for f in live['frames'] if f['number'] == number)
    summary = next(s['state']['live']['latest'] for s in states if s['state']['live']['sample_count'] == number)
    start = summary['started_unix_ms']
    end = start + summary['screen_capture_ms']
    lo, hi = bisect.bisect_right(times, start) - 1, bisect.bisect_left(times, end)
    bracket = oracle[lo:hi + 1] if 0 <= lo <= hi < len(oracle) else []
    timing = bool(bracket) and all(b['time_ms'] - a['time_ms'] <= 200 for a, b in zip(bracket, bracket[1:])) and all(
        o['response_ms'] - o['request_ms'] <= 200 for o in bracket)
    path = images / f'frame-{number:03}.png'
    assert sha(path) == ref['sha256']
    image = Image.open(path).convert('RGB')
    targets = {}
    for key, canvas_key, redraw in [('current', 'court', 'court'), ('next', 'preview', 'next')]:
        t = {'scorable': False}
        targets[key] = t
        if not timing: t['reason'] = 'oracle timing'; continue
        first = bracket[0]
        objects = first['objects']
        piece, canvas = objects[key], objects[canvas_key]
        if not piece or any(o['objects'][key] != piece or o['objects'][canvas_key] != canvas or o['viewport'] != first['viewport'] or o['objects']['pending_redraw'][redraw] for o in bracket):
            t['reason'] = 'changing or undrawn object'; continue
        viewport = first['viewport']; scale = viewport['dpr']
        ox = (image.width - viewport['width'] * scale) / 2
        oy = image.height - viewport['height'] * scale
        if abs(ox) > 1 or not 0 <= oy <= 200 or [image.width, image.height] != [run['geometry']['width'], run['geometry']['height']]:
            t['reason'] = 'image geometry'; continue
        sx, sy = canvas['width'] / canvas['logical_width'] * scale, canvas['height'] / canvas['logical_height'] * scale
        cells = [[canvas['left'] * scale + ox + (x * objects['dx'] + .5) * sx,
                  canvas['top'] * scale + oy + (y * objects['dy'] + .5) * sy,
                  objects['dx'] * sx, objects['dy'] * sy] for x, y in piece['cells']]
        color = ImageColor.getrgb(piece['color'])
        pixels = [image.getpixel((round(x + w / 2), round(y + h / 2))) if
                  0 <= round(x + w / 2) < image.width and 0 <= round(y + h / 2) < image.height else None for x, y, w, h in cells]
        t.update(cells=cells, color=color, center_pixels=pixels, canvas=canvas, viewport=viewport)
        if len(cells) != 4 or any(p is None or any(abs(a-b) > 2 for a, b in zip(p, color)) for p in pixels):
            t['reason'] = 'pixels do not support recorded cell colors'; continue
        left, top = min(c[0] for c in cells), min(c[1] for c in cells)
        t.update(scorable=True, bounds=[left, top, max(c[0]+c[2] for c in cells)-left, max(c[1]+c[3] for c in cells)-top],
                 relative_cells=sorted([[x-left, y-top, w, h] for x, y, w, h in cells]))
    frames[number] = {'frame_id': ref['id'], 'sha256': ref['sha256'], 'capture_ms': [start, end],
                      'oracle_bracket_indices': [lo, hi], 'targets': targets}

def iou(a, b):
    x,y,w,h = a; u,v,s,t = b
    area = max(0,min(x+w,u+s)-max(x,u))*max(0,min(y+h,v+t)-max(y,v))
    return area / max(1e-9,w*h+s*t-area)
case = read(RUN / 'result.json')['cases'][5]
rows = []
for measurement in case['measurements']:
    plan = next(p for p in case['selection']['regions'] if p['source_sha256'] == measurement['source_sha256'])
    assert plan['before_id'] == frames[2]['frame_id'] and plan['after_id'] == frames[3]['frame_id']
    region = plan['region']; box = [region[k] for k in ['x','y','width','height']]
    row = {'source_sha256': plan['source_sha256'], 'scorable': False}
    rows.append(row)
    candidates = [(iou(box,t['bounds']), key) for key,t in frames[2]['targets'].items() if t['scorable']]
    if not candidates or max(candidates)[0] < .5: row['reason'] = 'no source overlap at IoU 0.5'; continue
    overlap, key = max(candidates)
    a, b = frames[2]['targets'][key], frames[3]['targets'][key]
    if not b['scorable'] or any(a[k] != b[k] for k in ['relative_cells','color','canvas','viewport']):
        row['reason'] = 'unsupported corresponding geometry'; continue
    expected = [b['bounds'][i]-a['bounds'][i] for i in [0,1]]
    measured = measurement['appearance_match']['measurement']
    row.update(scorable=True, oracle_slot=key, source_iou=overlap, expected_displacement=expected,
               status=measured['status'], measured_displacement=measured['displacement'],
               absolute_error=None if measured['displacement'] is None else [abs(measured['displacement'][i]-expected[i]) for i in [0,1]],
               destination_iou=None if measured['matched_region'] is None else iou(measured['matched_region'],b['bounds']))
assert all(sha(Path(p)) == h for p,h in hashes.items())
save(OUT / 'result.json', {'scope':'Post-hoc known-case geometry check, not predeclared acceptance, persistent identity or unseen-game accuracy.',
                         'algorithm_changed':False, 'new_gameplay':False, 'oracle_sent_to_matcher':False,
                         'frames':frames,'comparisons':rows,'inputs_sha256':sha(OUT/'inputs-frozen.json')})
print(json.dumps(rows))
