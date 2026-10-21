"""Post-run integrity and independent fixture-field checks; never drives product."""
from pathlib import Path
import collections
import hashlib
import json
import struct

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
config=read(HERE/'protocol-v1.json')
for name,digest in config['method_sha256'].items():assert sha(HERE/name)==digest
for name,digest in config['product_source_sha256'].items():assert sha(ROOT/name)==digest
for name,digest in config['binaries'].items():assert sha(Path(config['binary_directory'])/name)==digest
results=[]
for slot in config['order']:
    directory=ROOT/f'test-results/change-search-v1/case-{slot}'
    report=read(directory/'report.json')
    assert report['protocol_sha256']==sha(HERE/'protocol-v1.json')
    assert not report['errors'],report['errors']
    commands=read(directory/'commands.json')
    assert collections.Counter(c['name'] for c in commands)=={
        'list_processes':1,'set_observer_model':1,'attach_process':1,'start_observation_job':1,'start_change_search':1}
    final=read(directory/'final-status.json');live=final['state']['live']
    initial=read(directory/'initial-states.json')[-1]['live']
    stages={'initial':read(directory/'initial-states.json'),'selection':read(directory/'selection-states.json')}
    if slot==0:stages['focused']=read(directory/'focused-states.json')
    ids={};pngs=0
    for name,states in stages.items():
        ids[name]=[s['live']['frames'][-1]['id'] for s in states]
        assert len(ids[name])==len(set(ids[name]))
        for number,state in enumerate(states,1):
            frame=state['live']['frames'][-1];screen=state['live']['latest']['screen']
            assert frame['number']==state['live']['sample_count']==number
            assert sha(Path(screen['path']))==frame['sha256'];pngs+=1
    assert not set(ids['initial'])&set(ids['selection'])
    selection=read(directory/'selection.json');summary=selection['summary']
    assert summary['source_frame']['id']==initial['frames'][-1]['id']
    assert [f['id'] for f in summary['basis']]==ids['selection']
    assert summary['observations']==config['selection_values'][str(slot)]
    assert summary['planned_bytes']<=128*1024*1024
    assert live['coverage']['attempted_bytes']<=4*1024**3
    assert live['retained_bytes']<=512*1024**2
    result={'slot':slot,'outcome':report['outcome'],'png_hashes_verified':pngs,'stage_ids':ids,
        'commands':dict(collections.Counter(c['name'] for c in commands)),
        'selection':{k:v for k,v in summary.items() if k not in ['basis','source_frame']},
        'coverage':{k:v for k,v in live['coverage'].items() if k!='batches'},
        'owned_pids':[report[k] for k in ['fixture_pid','engine_pid','mcp_pid']],
        'jobs':{k:{'phase':v['phase'],'steps':len(v['steps']),'binding':v['binding']} for k,v in final['jobs']['history']['jobs'].items()}}
    assert all(j['phase']=='Completed' for j in result['jobs'].values())
    if slot==0:
        assert not set(ids['focused'])&(set(ids['initial'])|set(ids['selection']))
        windows=selection['windows'];witnesses=selection['witnesses']
        assert len(windows)==len(witnesses)==summary['retained_windows']<=4096
        assert summary['omitted_windows']==summary['matched_windows']-len(windows)
        assert all(w['length']==64 and len(samples)==4 and all(len(b)==64 for b in samples) for w,samples in zip(windows,witnesses))
        field=report['initial_oracle']['field']
        found=[(w,s) for w,s in zip(windows,witnesses) if w['address']<=field and field+4<=w['address']+64]
        assert len(found)==1
        window,raw=found[0];offset=field-window['address']
        observed=[struct.unpack_from('<f',bytes(b),offset)[0]*2 for b in raw]
        assert observed==config['selection_values']['0']
        result['independent_field_witness_values']=observed
        result['field_offset_mib']=(field-report['initial_oracle']['arena'])/1024**2
        result['selected_bytes_per_fresh_capture']=sum(w['length'] for w in windows)
        checks=live['prediction_checks'];assert checks
        for check in checks:
            assert check['after_frame']==12
            assert [f['id'] for f in check['basis']]==ids['focused'][:12]
            assert all([v['image']['id'] for v in c['samples']]==ids['focused'][12:] for c in check['candidates'])
        result['checks']=[{'source':c['source'],'track':c['track'],'candidates':[{
            'address':m['address'],'mapping':m['mapping'],
            'samples':[{'frame':v['frame'],'observed':v['observed'],'predicted':v['predicted'],'outcome':v['outcome']} for v in m['samples']]}
            for m in c['candidates']]} for c in checks]
        actual=[m for c in checks for m in c['candidates'] if int(m['address'],16)==field and m['mapping']['kind']=='F32']
        assert len(actual)==1 and actual[0]['mapping']['slope']==2 and actual[0]['mapping']['intercept']==0
        assert all(s['outcome']=='agrees' for s in actual[0]['samples'])
        assert len(result['jobs'])==3
    else:
        assert summary['status']=='no_selection' and 'repeated changes' in summary['reason']
        assert not summary['retained_windows'] and not live['prediction_checks'] and live['coverage']['batch']==2
        assert len(result['jobs'])==2
    result['raw_json_sha256']={p.name:sha(p) for p in directory.glob('*.json')}
    results.append(result)
(HERE/'audit.json').write_text(json.dumps({'kind':'post-run audit, not predeclared acceptance','protocol_sha256':sha(HERE/'protocol-v1.json'),'cases':results},indent=2)+'\n',encoding='utf-8')
print(json.dumps({'verified':True,'cases':[{'slot':r['slot'],'outcome':r['outcome'],'pngs':r['png_hashes_verified']} for r in results]}))
