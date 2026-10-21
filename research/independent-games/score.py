"""Post-run scoring; this file never calls Reversebolt or changes a game."""
import bisect
import collections
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
config = read(Path(__file__).with_name('protocol-v1.json'))

def score(directory):
    run = read(directory/'run.json')
    states = read(directory/'states.json') if (directory/'states.json').exists() else []
    oracle = read(directory/'oracle.json')
    final = read(directory/'final-status.json') if (directory/'final-status.json').exists() else None
    times = [o['time_ms'] for o in oracle]
    assert all(b >= a for a,b in zip(times,times[1:])), 'Oracle clock went backwards'
    rows = []
    keys = ['score','best'] if run['game']=='2048' else ['score','rows']
    for state in states:
        live=state['state']['live']; frame=live['latest']; screen=frame.get('screen')
        start,end=frame['started_unix_ms'],frame['started_unix_ms']+frame['screen_capture_ms']
        lo,hi=bisect.bisect_right(times,start)-1,bisect.bisect_left(times,end)
        bracket=oracle[lo:hi+1] if 0<=lo<=hi<len(oracle) else []
        timing_ok=bool(bracket) and all(b['time_ms']-a['time_ms']<=config['oracle_max_gap_ms'] for a,b in zip(bracket,bracket[1:]))
        timing_ok=timing_ok and all(o['response_ms']-o['request_ms']<=config['oracle_max_gap_ms'] for o in bracket)
        for key in keys:
            row={'frame':live['sample_count'],'counter':key,'interval_ms':[start,end],'status':'unscorable'}
            if not screen or not timing_ok:
                row['reason']='Missing image or oracle coverage';rows.append(row);continue
            truth=bracket[0]['values'][key];viewport=bracket[0]['viewport'];geometry=run.get('geometry')
            if any(o['values'][key]!=truth or o['viewport']!=viewport for o in bracket):
                row['reason']='Value or geometry changed during image interval';rows.append(row);continue
            if not geometry or [geometry['width'],geometry['height']]!=[screen['width'],screen['height']]:
                row['reason']='Client/image geometry mismatch';rows.append(row);continue
            scale=viewport['dpr'];ox=(screen['width']-viewport['width']*scale)/2;oy=screen['height']-viewport['height']*scale
            if abs(ox)>1 or oy<0 or oy>200:
                row['reason']='Unsupported browser/client geometry';rows.append(row);continue
            x,y,w,h=truth['bounds'];box=[x*scale+ox,y*scale+oy,w*scale,h*scale]
            matching=[]
            for track in live['tracks']:
                tx,ty,tw,th=track['bounds'];cx,cy=tx+tw/2,ty+th/2
                if track['active'] and box[0]<=cx<=box[0]+box[2] and box[1]<=cy<=box[1]+box[3]:
                    matching.append({'track':track['id'],'label':track['label'],'value':track['samples'][-1],
                        'bounds':track['bounds'],'matches':len(track['matches'])})
            values={m['value'] for m in matching if m['value'] is not None}
            row.update(expected=truth['value'],bounds=box,observed=matching,
                status='abstention' if not values else 'conflicting' if len(values)>1 else 'correct' if next(iter(values))==truth['value'] else 'wrong')
            rows.append(row)
    counters={key:{'outcomes':dict(collections.Counter(r['status'] for r in rows if r['counter']==key)),
        'distinct_oracle_values':sorted({o['values'][key]['value'] for o in oracle})} for key in keys}
    live=(final or {}).get('state',{}).get('live') or {}
    contexts=[s['state']['live']['frames'][-1].get('input_context') for s in states if s['state']['live'].get('frames')]
    input_polls=[p for c in contexts if c for p in c['polls']]
    events=read(directory/'delivered-keys.json')
    downs=[e for e in events if e['kind']=='down']
    receipt={'game':run['game'],'replicate':run['replicate'],'directory':str(directory),
        'run_errors':run['errors'],'captured':live.get('sample_count',0),'scored_capture_states':len(states),
        'counters':counters,'rows':rows,'selected_process':run.get('selected'),
        'browser_processes':run.get('browserProcesses'),
        'planned_bytes':live.get('planned_bytes'),'eligible_bytes':live.get('eligible_bytes'),
        'numeric_tracks':len(live.get('tracks',[])),
        'numeric_matches':sum(len(t['matches']) for t in live.get('tracks',[])),
        'visual_tracks':live.get('visual_tracks',[]),'temporal_tracks':live.get('temporal_tracks',[]),
        'action_patterns':live.get('action_patterns',[]),'rule_checks':live.get('rule_checks',[]),
        'input':{'injected_pulses':run['injected_pulses'],'browser_keydowns':len(downs),
            'trusted_keydowns':sum(bool(e['trusted']) for e in downs),'by_key':dict(collections.Counter(e['key'] for e in downs)),
            'sampled_polls':len(input_polls),'poll_fields':list(input_polls[0]) if input_polls else [],
            'capture_contexts':sum(c is not None for c in contexts)},
        'oracle_nonnumeric':{'unique_tiles':len({json.dumps(o.get('tiles'),sort_keys=True) for o in oracle}) if run['game']=='2048' else None,
            'unique_current_pieces':len({json.dumps(o.get('current'),sort_keys=True) for o in oracle}) if run['game']=='tetris' else None,
            'actual_display_score_disagreements':sum(o.get('actual_score')!=o.get('display_score') for o in oracle) if run['game']=='tetris' else None},
        'files':{name:hashlib.sha256((directory/name).read_bytes()).hexdigest() for name in ['run.json','oracle.json','delivered-keys.json','final-status.json'] if (directory/name).exists()}}
    (directory/'score.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:receipt[k] for k in ['game','replicate','captured','counters','numeric_matches','input','oracle_nonnumeric','run_errors']}))
    return receipt

if __name__=='__main__':
    for value in sys.argv[1:]:score(Path(value).resolve())
