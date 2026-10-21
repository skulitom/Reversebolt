"""Post-run scoring of published cards; oracle data never enters inference."""
import bisect
import collections
import hashlib
import json
from pathlib import Path
import sys
from PIL import Image, ImageColor

out=Path(sys.argv[1]).resolve()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
run=read(out/'run.json');state=read(out/'final-status.json');oracle=read(out/'oracle.json')
live=state['state']['live'];times=[o['time_ms'] for o in oracle]
assert all(b>=a for a,b in zip(times,times[1:]))
frames={f['id']:f for f in live['frames']}
passes=[]
evaluation_frames=set()
for p in (out/'engine/captures/model-observations').glob('*.json'):
    r=read(p);last=r['sequence']['frames'][-1];evaluation_frames.add(last['id'])
    passes.append({'id':r['id'],'frame':last['number'],'error':r['pass']['error'],
        'proposed_observations':len(r['proposed_observations']),'sha256':sha(p)})
rows=[]
for frame_id in sorted(evaluation_frames,key=lambda i:frames[i]['number']):
    frame=frames[frame_id];number=frame['number']
    summary=next(s['state']['live']['latest'] for s in read(out/'states.json') if s['state']['live']['sample_count']==number)
    start=summary['started_unix_ms'];end=start+summary['screen_capture_ms']
    lo,hi=bisect.bisect_right(times,start)-1,bisect.bisect_left(times,end)
    bracket=oracle[lo:hi+1] if 0<=lo<=hi<len(oracle) else []
    timing=bool(bracket) and all(b['time_ms']-a['time_ms']<=200 for a,b in zip(bracket,bracket[1:])) and all(o['response_ms']-o['request_ms']<=200 for o in bracket)
    paths=list((out/'engine/captures').glob(f'*/frame-{number:03}.png'));assert len(paths)==1 and sha(paths[0])==frame['sha256']
    image=Image.open(paths[0]).convert('RGB')
    published=[o['input'] for t in live['visual_tracks'] for o in t['observations'] if o and o['input']['frame_id']==frame_id]
    targets=[]
    for key,canvas_key,redraw in [('current','court','court'),('next','preview','next')]:
        target={'name':key,'scorable':False}
        if not timing:
            target['reason']='oracle timing';targets.append(target);continue
        first=bracket[0];objects=first['objects'];piece=objects[key];canvas=objects[canvas_key]
        if not piece or any(o['objects'][key]!=piece or o['objects'][canvas_key]!=canvas or o['viewport']!=first['viewport'] or o['objects']['pending_redraw'][redraw] for o in bracket):
            target['reason']='changing or undrawn object';targets.append(target);continue
        viewport=first['viewport'];scale=viewport['dpr'];ox=(image.width-viewport['width']*scale)/2;oy=image.height-viewport['height']*scale
        if abs(ox)>1 or not 0<=oy<=200 or [image.width,image.height]!=[run['geometry']['width'],run['geometry']['height']]:
            target['reason']='image geometry';targets.append(target);continue
        sx=canvas['width']/canvas['logical_width']*scale;sy=canvas['height']/canvas['logical_height']*scale
        cells=[[canvas['left']*scale+ox+(x*objects['dx']+.5)*sx,
                canvas['top']*scale+oy+(y*objects['dy']+.5)*sy,objects['dx']*sx,objects['dy']*sy] for x,y in piece['cells']]
        color=ImageColor.getrgb(piece['color']);pixels=[]
        for x,y,w,h in cells:
            px,py=round(x+w/2),round(y+h/2)
            pixels.append(image.getpixel((px,py)) if 0<=px<image.width and 0<=py<image.height else None)
        target.update(cells=cells,color=color,center_pixels=pixels)
        if len(cells)!=4 or any(p is None or any(abs(a-b)>2 for a,b in zip(p,color)) for p in pixels):
            target['reason']='pixels do not support recorded cell colors';targets.append(target);continue
        left=min(c[0] for c in cells);top=min(c[1] for c in cells)
        target.update(scorable=True,bounds=[left,top,max(c[0]+c[2] for c in cells)-left,max(c[1]+c[3] for c in cells)-top])
        targets.append(target)
    def iou(a,b):
        x,y,w,h=a;u,v,s,t=b
        area=max(0,min(x+w,u+s)-max(x,u))*max(0,min(y+h,v+t)-max(y,v))
        return area/max(1e-9,w*h+s*t-area)
    objects=[p for p in published if p['value']=={'kind':'text','value':'Object'} and p['bounds']]
    scores={}
    for threshold in [.3,.5]:
        matched_p=set();matched_t=set()
        for overlap,pi,ti in sorted([(iou(p['bounds'],t['bounds']),pi,ti) for pi,p in enumerate(objects) for ti,t in enumerate(targets) if t['scorable']],reverse=True):
            if overlap>=threshold and pi not in matched_p and ti not in matched_t:matched_p.add(pi);matched_t.add(ti)
        scores[str(threshold)]={'matched_targets':[targets[i]['name'] for i in sorted(matched_t)],
            'missed_targets':[t['name'] for i,t in enumerate(targets) if t['scorable'] and i not in matched_t],
            'unscorable_targets':[t['name'] for t in targets if not t['scorable']],
            'unmatched_objects':len(objects)-len(matched_p)}
    rows.append({'frame':number,'frame_id':frame_id,'sha256':frame['sha256'],'published':published,'targets':targets,'scores':scores})

# Empty/failed passes still contribute missed or unscorable target opportunities.
published_frames={r['frame'] for r in rows if r['published']}
result={'scope':'Selected published-object localization, not semantic/control verification or full-scene precision.',
    'passes':passes,'rows':rows,'passes_without_published_findings':[p['frame'] for p in passes if p['frame'] not in published_frames],
    'kinds':dict(collections.Counter(o['input']['value']['value'] for t in live['visual_tracks'] for o in t['observations'] if o)),
    'job_phases':[j['phase'] for j in state['jobs']['history']['jobs'].values()],
    'artifacts':{p.name:sha(p) for p in [out/'run.json',out/'oracle.json',out/'final-status.json']}}
(out/'object-score.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print(json.dumps({k:result[k] for k in ['kinds','job_phases','passes_without_published_findings']}))
for row in rows:print(json.dumps({'frame':row['frame'],'scores':row['scores']}))
