"""Audit the frozen run artifacts and write a compact, portable evidence record."""
import collections
import hashlib
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
BASE=ROOT/'test-results/independent-games-v1'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
config=read(HERE/'protocol-v1.json')
assert sha(BASE/'upstream/manifest.json')==config['upstream_manifest_sha256']
assert sha(HERE/'upstream.json')==config['upstream_manifest_sha256']
for entry in read(BASE/'upstream/manifest.json'):
    assert sha(BASE/'upstream'/(entry['name']+'.zip'))==entry['archive_sha256']
    for name,digest in entry['files'].items():assert sha(BASE/'upstream'/entry['name']/name)==digest
for name,digest in config['binaries'].items():assert sha(Path(config['binary_directory'])/name)==digest
for name,digest in config['pre_capture_method_sha256'].items():assert sha(HERE/name)==digest

records=[]
for game,replicate in config['order']:
    out=BASE/f'{game}-{replicate}-attempt-01'
    run,scored,final=read(out/'run.json'),read(out/'score.json'),read(out/'final-status.json')
    assert run['started_protocol_sha256']==sha(HERE/'protocol-v1.json')
    assert run['binaries']==config['binaries']
    live=final['state']['live']
    assert len(live['frames'])==live['sample_count']==16
    assert run['selected']['pid']==live['process']['pid']
    assert next(p for p in run['browserProcesses'] if p['type']=='browser')['id']==live['process']['pid']
    for frame in live['frames']:
        images=list((out/'engine/captures').glob(f'*/frame-{frame["number"]:03}.png'))
        assert len(images)==1 and sha(images[0])==frame['sha256']
    assert scored['input']['injected_pulses']==scored['input']['browser_keydowns']==scored['input']['trusted_keydowns']
    pulses=[json.loads(line) for line in (out/'input.jsonl').read_text().splitlines()]
    assert not any(p['kind']=='error' for p in pulses)
    pulses=[p for p in pulses if p['kind']=='pulse']
    events=read(out/'delivered-keys.json')
    keys={32:' ',37:'ArrowLeft',38:'ArrowUp',39:'ArrowRight',40:'ArrowDown'}
    assert len(events)==2*len(pulses)
    for index,pulse in enumerate(pulses):
        down,up=events[index*2:index*2+2]
        assert down['kind']=='down' and up['kind']=='up'
        assert down['key']==up['key']==keys[pulse['vk']]
        assert down['trusted'] and up['trusted'] and not down['repeat']
    states=read(out/'states.json')
    assert [s['state']['live']['sample_count'] for s in states]==list(range(1,17))
    model=[]
    for file in sorted((out/'engine/captures/model-observations').glob('*.json')):
        evidence=read(file);proposal=evidence['pass']['proposal']
        assert proposal['model_digest']==config['model_digest']
        response=json.loads(proposal['response']['message']['content'])
        model.append({'id':evidence['id'],'sha256':sha(file),'error':evidence['pass']['error'],
            'summary_characters':len(response['summary']),'entity_kinds':[e['kind'] for e in response['entities']],
            'proposed_observations':len(evidence['proposed_observations'])})
    assert len(model)==2
    counters={key:{'outcomes':value['outcomes'],'distinct_value_count':len(value['distinct_oracle_values']),
        'range':[min(value['distinct_oracle_values']),max(value['distinct_oracle_values'])]} for key,value in scored['counters'].items()}
    records.append({'game':game,'replicate':replicate,'raw_directory':out.relative_to(ROOT).as_posix(),
        'run_errors':run['errors'],'job_phases':[j['phase'] for j in final['jobs']['history']['jobs'].values()],
        'captures':16,'counters':counters,'input':scored['input'],
        'window_process':run['selected'],'browser_processes':run['browserProcesses'],
        'planned_bytes':live['planned_bytes'],'eligible_bytes':live['eligible_bytes'],
        'numeric_matches_at_end':scored['numeric_matches'],
        'transient_matches':[{'frame':s['state']['live']['sample_count'],'matches':sum(len(t['matches']) for t in s['state']['live']['tracks'])}
            for s in states if any(t['matches'] for t in s['state']['live']['tracks'])],
        'vision_warnings':dict(collections.Counter(s['state']['live']['latest']['screen']['warning'] for s in states if s['state']['live']['latest']['screen']['warning'])),
        'visual_tracks':len(live['visual_tracks']),'temporal_tracks':len(live['temporal_tracks']),
        'action_pattern_states':dict(collections.Counter(p['state'] for e in live['action_patterns'] for p in e['patterns'])),
        'rule_checks':len(live['rule_checks']),'model_calls':model,'oracle_nonnumeric':scored['oracle_nonnumeric'],
        'artifact_hashes':{name:sha(out/name) for name in ['run.json','score.json','oracle.json','states.json','final-status.json','delivered-keys.json','input.jsonl']}})

result={'protocol_sha256':sha(HERE/'protocol-v1.json'),'product_binaries':config['binaries'],
    'scope':'Independent browser-game baseline, not withheld final acceptance. Product unchanged, no native writes, no oracle observations submitted.',
    'runs':records,'image_hashes_checked':64,'delivered_keydowns':sum(r['input']['browser_keydowns'] for r in records),
    'input_sequence_verified':True,
    'progress_percent':25,'limitations':['DOM scoring rectangles include transient overlays; a wrong score-region reading need not be a misrecognized glyph.',
        'Animation/timing ambiguity is unscorable; constant zero row counters are separate.',
        'Read-only evaluator instrumentation can allocate copies and add timing overhead.',
        'No controlling memory, hidden behavior or causal edit was verified.','No manual screenshot review was used.']}
folder=HERE/'evidence';folder.mkdir(exist_ok=True)
(folder/'browser-v1.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'runs':len(records),'images_verified':64,'delivered_keys':result['delivered_keydowns'],'job_phases':[r['job_phases'] for r in records]}))
