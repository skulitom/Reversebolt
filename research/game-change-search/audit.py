"""Post-launch integrity audit; does not drive games or select outcomes."""
import collections
import hashlib
import json
from pathlib import Path

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
config=read(HERE/'protocol-v1.json')
for name,digest in config['method_sha256'].items():assert sha(HERE/name)==digest, name
for name,digest in config['binaries'].items():assert sha(Path(config['binary_directory'])/name)==digest, name
assert sha(HERE/config['product_evidence'])==config['product_evidence_sha256']
manifest=ROOT/'test-results/independent-games-v1/upstream/manifest.json'
assert sha(manifest)==config['upstream_manifest_sha256']
for source in read(manifest):
    for name,digest in source['files'].items():assert sha(manifest.parent/source['name']/name)==digest, name

def compact_check(c):
    return {'id':c['id'],'source':c['source'],'track':c['track'],'after_frame':c['after_frame'],
        'basis_ids':[f['id'] for f in c['basis']],
        'candidates':[{'address':m['address'],'mapping':m['mapping'],
            'samples':[{'frame':s['frame'],'frame_id':s['image']['id'],'outcome':s['outcome'],
                'observed':s.get('observed'),'predicted':s.get('predicted')} for s in m['samples']]} for m in c['candidates']]}

cases=[]
for game,replicate in config['order']:
    directory=ROOT/f'test-results/game-change-search-v1/{game}-{replicate}'
    run=read(directory/'run.json');assert run['started_protocol_sha256']==sha(HERE/'protocol-v1.json')
    commands=read(directory/'commands.json') if (directory/'commands.json').exists() else []
    counts=collections.Counter(c['name'] for c in commands)
    assert not set(counts)&{'capture_window','submit_observation','prepare_change','correlate','freeze_predictions','finish_change_search','start_observation_job','search_number'}
    for name in ['start_change_search','prepare_probe','apply_change','restore_change']:
        assert counts[name]<=1,(game,name,counts[name])
    assert counts['observe_change']<=2
    states=read(directory/'states.json') if (directory/'states.json').exists() else []
    result={'game':game,'errors':run['errors'],'captures':len(states),'search_outcome':run['searchOutcome'],
        'trial_outcome':run.get('trialOutcome'),'commands':dict(counts),'owned_processes':run['ownedProcesses'],
        'browser_processes':run.get('browserProcesses',[]),'stages':{},'png_hashes_verified':0}
    all_ids=[]
    for phase in dict.fromkeys(s['evaluation_phase'] for s in states):
        stage=[s['state']['live'] for s in states if s['evaluation_phase']==phase]
        assert [s['sample_count'] for s in stage]==list(range(1,len(stage)+1))
        final=stage[-1];ids=[f['id'] for f in final['frames']]
        assert len(ids)==len(set(ids))==len(stage)
        assert not set(ids)&set(all_ids);all_ids+=ids
        assert len(final['prediction_checks'])<=8
        assert sum(len(c['candidates']) for c in final['prediction_checks'])<=200
        checks=[compact_check(c) for c in final['prediction_checks']]
        for check in checks:
            assert check['after_frame']==12 and check['basis_ids']==ids[:12]
            for candidate in check['candidates']:assert [s['frame_id'] for s in candidate['samples']]==ids[12:]
        for live in stage:
            screen=live['latest']['screen'];frame=live['frames'][-1]
            assert screen and sha(Path(screen['path']))==frame['sha256']
            result['png_hashes_verified']+=1
        result['stages'][phase]={'count':len(stage),'ids':ids,'checks':checks,
            'numeric_readings':{str(t['id']):{'label':t['label'],'samples':t['samples']} for t in final['tracks']},
            'coverage':{k:v for k,v in final['coverage'].items() if k!='batches'}}
    if (directory/'selection.json').exists():
        selection=read(directory/'selection.json');summary=selection['summary']
        initial=read(directory/'initial-completed.json')['state']['live']
        assert summary['source_frame']['id']==initial['frames'][-1]['id']
        assert [f['id'] for f in summary['basis']]==result['stages']['selection']['ids']
        assert summary['planned_bytes']<=128*1024**2
        windows=selection['windows'];witnesses=selection['witnesses']
        assert len(windows)==len(witnesses)==summary['retained_windows']<=4096
        assert summary['omitted_windows']==summary['matched_windows']-len(windows)
        assert len({w['address'] for w in windows})==len(windows)
        assert all(w['length']==64 and len(samples)==4 and all(len(b)==64 for b in samples) for w,samples in zip(windows,witnesses))
        selected=read(directory/'selection-completed.json')['state']['live']
        old=read(Path(next(b['archive'] for b in selected['coverage']['batches'] if b['number']==summary['source_batch'])))
        assert all(not (w['address']<p['address']+p['length'] and p['address']<w['address']+w['length']) for w in windows for p in old['plan'])
        result['selection']={k:v for k,v in summary.items() if k not in ['basis','source_frame']}
        result['selected_bytes_per_capture']=sum(w['length'] for w in windows)
    if (directory/'fresh-start.json').exists():
        fresh=read(directory/'fresh-start.json')
        jobs=fresh['jobs']['history']['jobs']
        result['jobs']={k:{'phase':j['phase'],'steps':len(j['steps']),'binding':j['binding']} for k,j in jobs.items()}
        assert len(jobs)==3
    if (directory/'trial-selection.json').exists():
        trial=read(directory/'trial-selection.json');fresh=read(directory/'focused-completed.json')['state']['live']
        checks=sorted((c for c in fresh['prediction_checks'] if c['source']=='ocr'),key=lambda c:c['track'])
        choices=[(c,i,m) for c in checks for i,m in enumerate(c['candidates']) if len(m['samples'])==4 and all(s['outcome']=='agrees' for s in m['samples'])]
        c,i,m=choices[0]
        assert (trial['prediction_id'],trial['candidate'],trial['track'])==(c['id'],i,c['track'])
        assert trial['requested']==next(t['samples'][-1] for t in fresh['tracks'] if t['id']==c['track'])+10
        result['trial_selection']=trial
        prepared=read(directory/'prepared.json')
        if prepared['ok'] and prepared.get('data',{}).get('phase')=='prepared':
            change=prepared['data']
            assert change['scope']['process']==fresh['memory_process']
            assert change['probe']['selected']==trial['candidate']
            assert 1<=len(change['patch']['after'])==len(change['patch']['before'])<=8
            result['trial_patch_bytes']=len(change['patch']['after'])
        for name in ['prepared','applied','checked','restored','checked-restore','cleanup-restore']:
            file=directory/(name+'.json')
            if file.exists():
                response=read(file);change=response.get('data') or {}
                result[name]={'ok':response['ok'],'error':response.get('error'),'phase':change.get('phase'),
                    'id':change.get('id'),'reason':change.get('reason')}
        latest=directory/'checked-restore.json'
        if latest.exists():
            change=read(latest)['data'];images=[change['baseline']]+change['observations']
            for observation in images:
                assert sha(Path(observation['image']))==observation['image_sha256']
            result['trial_png_hashes_verified']=len({o['image'] for o in images})
            result['trial_observations']=[{k:o.get(k) for k in ['stage','value','expected','agrees','error','image_sha256']} for o in images]
            result['write_receipt']=change['write'];result['restoration_receipt']=change['restoration']
            assert change['phase']=='restored'
            assert change['write']['attempted'] and change['restoration']['attempted']
            assert change['write']['observed_after']==change['patch']['after']
            assert change['restoration']['observed_after']==change['patch']['before']
    if (directory/'input.jsonl').exists():
        records=[json.loads(line) for line in (directory/'input.jsonl').read_text(encoding='utf-8').splitlines()]
        pulses=[r for r in records if r['kind']=='pulse'];events=read(directory/'delivered-keys.json')
        downs=[e for e in events if e['kind']=='down'];ups=[e for e in events if e['kind']=='up']
        keys={32:' ',37:'ArrowLeft',38:'ArrowUp',39:'ArrowRight',40:'ArrowDown'}
        assert len(pulses)==len(downs)==len(ups)
        assert all(e['trusted'] for e in events)
        assert [keys[p['vk']] for p in pulses]==[e['key'] for e in downs]==[e['key'] for e in ups]
        if not run['errors']:
            expected=1+sum(12 if phase=='selection' else 56 for phase in result['stages'])
            assert len(pulses)==expected,(game,len(pulses),expected)
        result['input']={'pulses':len(pulses),'trusted_downs':len(downs),'trusted_ups':len(ups)}
    result['raw_json_sha256']={p.name:sha(p) for p in directory.glob('*.json') if p.name!='score.json'}
    cases.append(result)
(HERE/'audit.json').write_text(json.dumps({'kind':'post-launch integrity audit, not acceptance','protocol_sha256':sha(HERE/'protocol-v1.json'),'cases':cases},indent=2)+'\n',encoding='utf-8')
print(json.dumps({'verified':True,'cases':[{'game':c['game'],'captures':c['captures'],'errors':len(c['errors']),'pngs':c['png_hashes_verified']} for c in cases]}))
