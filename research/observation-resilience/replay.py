"""Paired saved-image regression. No game is launched or controlled.

Usage: python replay.py <candidate-exe> <new-output-directory>
Product inputs are only full images in order and pinned text model files.
Oracle rectangles/values are used after each replay, never passed to the app.
"""
import collections
import hashlib
import json
from pathlib import Path
import shutil
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
candidate, out = map(lambda s: Path(s).resolve(), sys.argv[1:])
out.mkdir(exist_ok=False)
frozen = read(ROOT/'research/independent-games/protocol-v1.json')
baseline = Path(frozen['binary_directory'])/'Reversebolt.exe'
assert sha(baseline) == frozen['binaries']['Reversebolt.exe']
executables = {'baseline': baseline, 'candidate': candidate}
cases = []
for game, replicate in frozen['order']:
    name = f'{game}-{replicate}-attempt-01'
    raw = ROOT/'test-results/independent-games-v1'/name
    frames = read(raw/'final-status.json')['state']['live']['frames']
    paths = []
    for frame in frames:
        matches = list((raw/'engine/captures').glob(f'*/frame-{frame["number"]:03}.png'))
        assert len(matches) == 1 and sha(matches[0]) == frame['sha256']
        paths.append(matches[0])
    cases.append({'name':name,'paths':paths,'oracle':raw/'score.json'})

# Existing selected-field regressions, with their original separate annotations.
for key, previous in [('blue-native-fresh','blue-live-native.json'),
                      ('hl2-development','hl2-fused-v2.json'),
                      ('ordinary-original','ordinary-fused-v2.json')]:
    old = read(ROOT/'test-results/ocr-native-v1'/previous)
    cases.append({'name':key,'paths':[Path(f['screen']['path']) for f in old[:12]],
                  'labels_key':key})

for case in cases:
    folder = out/case['name']/'frames'
    folder.mkdir(parents=True)
    case['images'] = []
    for i, source in enumerate(case.pop('paths')):
        dest = folder/f'frame-{i+1:03}.png'
        shutil.copyfile(source, dest)
        assert sha(source) == sha(dest)
        case['images'].append({'source':str(source),'copy':str(dest),'sha256':sha(source)})
    if 'oracle' in case:
        case['oracle'] = str(case['oracle'])
        case['oracle_sha256'] = sha(Path(case['oracle']))

manifest = {'scope':'Paired regression on exposed saved images; no new-game or control-validation claim.',
    'binaries':{name:{'path':str(p),'sha256':sha(p)} for name,p in executables.items()},
    'cases':cases,'script_sha256':sha(Path(__file__)),
    'labels_sha256':sha(ROOT/'research/ocr-v6/labels.json'),
    'legacy_scorer_sha256':sha(ROOT/'research/ocr-v6/score_native.py'),
    'models_manifest_sha256':sha(ROOT/'models/manifest.json')}
(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')

def browser_score(case, report):
    oracle_path = Path(case['oracle'])
    assert sha(oracle_path) == case['oracle_sha256']
    rows = []
    for original in read(oracle_path)['rows']:
        row = {k:original[k] for k in ['frame','counter','status']}
        if row['status'] != 'unscorable':
            frame = report[row['frame']-1]
            # Frozen image-coordinate oracle boxes, not expected-value search.
            x,y,w,h = original['bounds']
            tracks = [t for t in frame['tracks'] if t['active'] and
                x <= t['bounds'][0]+t['bounds'][2]/2 <= x+w and
                y <= t['bounds'][1]+t['bounds'][3]/2 <= y+h]
            values = {t['samples'][-1] for t in tracks if t['samples'][-1] is not None}
            row.update(expected=original['expected'], values=sorted(values),
                status='abstention' if not values else 'conflicting' if len(values)>1
                else 'correct' if next(iter(values))==original['expected'] else 'wrong')
        rows.append(row)
    return {key:dict(collections.Counter(r['status'] for r in rows if r['counter']==key))
            for key in sorted({r['counter'] for r in rows})}, rows

results = []
for case in cases:
    folder = out/case['name']
    for name, exe in executables.items():
        assert sha(exe) == manifest['binaries'][name]['sha256']
        output = folder/f'{name}.json'
        assert not output.exists()
        started = time.monotonic()
        with (folder/f'{name}.log').open('wb') as log:
            subprocess.run([str(exe),'--ocr-sequence',str(ROOT/'models'),str(folder/'frames'),str(output)],
                check=True,stdout=log,stderr=subprocess.STDOUT,
                creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
        report = read(output)
        assert len(report) == len(case['images'])
        for image, frame in zip(case['images'], report):
            assert sha(Path(frame['screen']['path'])) == image['sha256']
        if 'oracle' in case:
            outcomes, rows = browser_score(case, report)
            score = folder/f'{name}-score.json'
            score.write_text(json.dumps({'outcomes':outcomes,'rows':rows},indent=2)+'\n',encoding='utf-8')
        else:
            score = folder/f'{name}-score.json'
            subprocess.run([sys.executable,str(ROOT/'research/ocr-v6/score_native.py'),case['labels_key'],str(output),str(score)],
                check=True,cwd=ROOT,stdout=subprocess.PIPE)
            scored = read(score)
            outcomes = {key:scored[key] for key in ['correct','wrong','uncertain','missing']}
        result = {'case':case['name'],'reader':name,'images':len(report),'outcomes':outcomes,
            'elapsed_seconds':time.monotonic()-started,
            'median_recognition_ms':statistics.median(f['elapsed_ms'] for f in report),
            'warnings':dict(collections.Counter(f['screen']['warning'] for f in report if f['screen']['warning'])),
            'frames_with_neural_words':sum(any(w['source'] in ['PpOcrV6','PpOcrV6Region','pp_ocr_v6','pp_ocr_v6_region']
                for w in f['screen']['words']) for f in report),
            'output':str(output),'output_sha256':sha(output),'score_sha256':sha(score)}
        results.append(result)
        (out/'results.json').write_text(json.dumps(results,indent=2)+'\n',encoding='utf-8')
        print(json.dumps(result),flush=True)

print(json.dumps({'complete':True,'replays':len(results),'source_images':sum(len(c['images']) for c in cases)}),flush=True)
