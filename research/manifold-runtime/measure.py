"""Evaluate a frozen screenshot contrast; never controls the game or labels a field."""
import hashlib, json, sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageFilter

root = Path(sys.argv[1]).resolve()
protocol = json.loads(Path(__file__).with_name('movement-protocol.json').read_bytes())
crop = tuple(protocol['measurement']['crop_xyxy'])
def pixels(name):
    with Image.open(root / 'frames' / (name+'.png')) as image:
        assert image.size == (640, 360), image.size
        return np.asarray(image.convert('L').crop(crop).filter(ImageFilter.GaussianBlur(1)), dtype=np.float64)
def difference(a,b):
    diff=np.abs(pixels(a)-pixels(b))
    return {'before':a, 'after':b, 'mean_absolute_difference':float(diff.mean()),
            'fraction_difference_gt_12':float((diff>12).mean())}
conditions={}
for stage in protocol['conditions']:
    if not (root/'frames'/f'{stage}-4.png').exists(): continue
    stationary=difference(stage+'-idle',stage+'-0')
    pulses=[difference(f'{stage}-{i}',f'{stage}-{i+1}') for i in range(4)]
    conditions[stage]={'stationary':stationary,'pulses':pulses,
                       'median_difference':float(np.median([p['mean_absolute_difference'] for p in pulses]))}
passed=None
if len(conditions)==3:
    b,c,r=(conditions[s] for s in protocol['conditions'])
    passed=bool(all(s['median_difference']>=5 and s['median_difference']>=5*s['stationary']['mean_absolute_difference'] for s in (b,r))
                and c['median_difference']<=max(2,.2*b['median_difference'])
                and r['median_difference']>=.5*b['median_difference'])
result={'criteria_passed':passed,'conditions':conditions,
        'protocol_sha256':hashlib.sha256(Path(__file__).with_name('movement-protocol.json').read_bytes()).hexdigest(),
        'frame_sha256':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((root/'frames').glob('*.png'))},
        'limitation':protocol['limits']}
(root/'movement-measurement.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
print(json.dumps({'criteria_passed':passed,'conditions':{k:{'stationary':v['stationary']['mean_absolute_difference'],'median':v['median_difference'],'pulses':[p['mean_absolute_difference'] for p in v['pulses']]} for k,v in conditions.items()}}))
