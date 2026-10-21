"""Post-run integrity audit, added after launch; does not select product outcomes."""
import collections
import hashlib
import json
from pathlib import Path

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
read=lambda p:json.loads(p.read_text(encoding='utf-8'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
protocol=read(HERE/'protocol-v1.json')
for name,digest in protocol['method_sha256'].items():
    assert sha(HERE/name)==digest,('Changed frozen method',name)
for name,digest in protocol['binaries'].items():
    assert sha(Path(protocol['binary_directory'])/name)==digest,('Changed binary',name)
assert sha(HERE/protocol['product_evidence'])==protocol['product_evidence_sha256']
manifest=ROOT/'test-results/independent-games-v1/upstream/manifest.json'
assert sha(manifest)==protocol['upstream_manifest_sha256']
for source in read(manifest):
    for name,digest in source['files'].items():
        assert sha(manifest.parent/source['name']/name)==digest

def compact_check(check):
    return {'id':check['id'],'source':check['source'],'track':check['track'],
        'after_frame':check['after_frame'],'basis_ids':[f['id'] for f in check['basis']],
        'candidate_count':len(check['candidates']),
        'candidates':[{'address':c['address'],'mapping':c['mapping'],
            'samples':[{'frame':s['frame'],'frame_id':s['image']['id'],
                'outcome':s['outcome'],'observed':s.get('observed'),'predicted':s.get('predicted')}
                for s in c['samples']]} for c in check['candidates']]}

results=[]
for game,replicate in protocol['order']:
    directory=ROOT/f'test-results/game-value-search-v1/{game}-{replicate}'
    run=read(directory/'run.json')
    assert run['started_protocol_sha256']==sha(HERE/'protocol-v1.json')
    commands=read(directory/'commands.json') if (directory/'commands.json').exists() else []
    assert not {c['name'] for c in commands}&{'capture_window','submit_observation','prepare_change','prepare_probe','apply_change','restore_change','correlate','freeze_predictions'}
    states=read(directory/'states.json') if (directory/'states.json').exists() else []
    result={'game':game,'errors':run['errors'],'captures':len(states),'search_outcome':run['searchOutcome'],
        'commands':dict(collections.Counter(c['name'] for c in commands)),
        'owned_processes':run['ownedProcesses'],'browser_processes':run.get('browserProcesses',[]),
        'stages':{}}
    for phase in dict.fromkeys(s['evaluation_phase'] for s in states):
        stage=[s['state']['live'] for s in states if s['evaluation_phase']==phase]
        assert [s['sample_count'] for s in stage]==list(range(1,len(stage)+1))
        final=stage[-1]
        ids=[f['id'] for f in final['frames']]
        assert len(ids)==len(set(ids))==len(stage)
        assert len(final['prediction_checks'])<=8
        assert sum(len(c['candidates']) for c in final['prediction_checks'])<=200
        checks=[compact_check(c) for c in final['prediction_checks']]
        for check in checks:
            assert check['after_frame']==12
            assert check['basis_ids']==ids[:12]
            for candidate in check['candidates']:
                assert [s['frame_id'] for s in candidate['samples']]==ids[12:]
        result['stages'][phase]={'count':len(stage),'ids':ids,'checks':checks,
            'numeric_readings':{str(t['id']):{'label':t['label'],'samples':t['samples']} for t in final['tracks']},
            'budget':{k:v for k,v in final['coverage'].items() if k!='batches'}}
    if (directory/'searched.json').exists():
        initial=read(directory/'initial-completed.json')['state']['live']
        searched=read(directory/'searched.json')
        summary=searched['state']['live']['value_searches'][-1]
        selection=read(directory/'selection.json')
        assert summary['source_frame']==16 and summary['source_batch']==1
        assert summary['source_frame_id']==initial['frames'][-1]['id']
        assert summary['source_image_sha256']==initial['frames'][-1]['sha256']
        assert summary['value']==next(t['samples'][-1] for t in initial['tracks'] if t['id']==summary['source_track'])
        assert summary['attempted_bytes']<=512*1024*1024
        assert summary['read_bytes']<=summary['attempted_bytes']
        windows=selection['windows']
        assert len(windows)==summary['retained_windows']<=4096
        assert len({w['address'] for w in windows})==len(windows)
        assert all(w['length']==64 for w in windows)
        result['search']=summary
        result['selected_bytes_per_capture']=sum(w['length'] for w in windows)
        if searched['ok']:
            fresh=searched['state']['live']
            assert fresh['sample_count']==0 and not fresh['prediction_checks'] and not fresh['tracks']
            if 'focused' in result['stages']:
                assert not set(result['stages']['initial']['ids'])&set(result['stages']['focused']['ids'])
            old=read(Path(fresh['coverage']['batches'][-1]['archive']))
            assert all(not (w['address']<p['address']+p['length'] and p['address']<w['address']+w['length']) for w in windows for p in old['plan'])
    if (directory/'input.jsonl').exists():
        input_records=[json.loads(line) for line in (directory/'input.jsonl').read_text(encoding='utf-8').splitlines()]
        pulses=[r for r in input_records if r['kind']=='pulse']
        delivered=read(directory/'delivered-keys.json')
        downs=[e for e in delivered if e['kind']=='down'];ups=[e for e in delivered if e['kind']=='up']
        key_names={32:' ',37:'ArrowLeft',38:'ArrowUp',39:'ArrowRight',40:'ArrowDown'}
        assert len(pulses)==len(downs)==len(ups)
        assert all(e['trusted'] for e in delivered)
        assert [key_names[p['vk']] for p in pulses]==[e['key'] for e in downs]==[e['key'] for e in ups]
        assert len(pulses)==1+56*len(result['stages'])
        result['input']={'pulses':len(pulses),'trusted_downs':len(downs),'trusted_ups':len(ups)}
    result['files']={p.name:sha(p) for p in directory.glob('*.json') if p.name not in ['score.json']}
    results.append(result)

receipt={'audit_kind':'post-run integrity, not acceptance or gameplay control','protocol_sha256':sha(HERE/'protocol-v1.json'),
    'product_binaries':protocol['binaries'],'cases':results}
(HERE/'audit.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'verified':True,'cases':[{'game':r['game'],'captures':r['captures'],'errors':len(r['errors']),'stages':list(r['stages'])} for r in results]}))
