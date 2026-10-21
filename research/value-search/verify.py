"""Post-run integrity and outcome audit; frozen assertions/failures stay intact."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parent.parent
read=lambda p:json.loads(p.read_text(encoding='utf-8'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
config=read(HERE/'protocol-v1.json')
for name,h in config['method_sha256'].items():assert sha(HERE/name)==h,name
for name,h in config['product_source_sha256'].items():assert sha(HERE/'source'/name)==h,name
for name,h in config['binaries'].items():assert sha(Path(config['binary_directory'])/name)==h,name
evidence={'protocol_sha256':sha(HERE/'protocol-v1.json'),'audit':'Post-run; all strict runner failures retained.', 'cases':[]}
for slot in [0,1,2]:
    folder=ROOT/'test-results/value-search-v1'/f'case-{slot}'
    report=read(folder/'report.json');assert report['protocol_sha256']==evidence['protocol_sha256']
    item={'report':report,'strict_passed':not report['errors']}
    if (folder/'initial-completed.json').exists():
        initial=read(folder/'initial-completed.json')['state']['live']
        item['initial_tracks']=[{'id':t['id'],'label':t['label'],'samples':t['samples']} for t in initial['tracks']]
        blank=[]
        for state in read(folder/'initial-states.json'):
            latest=state['live']['latest'];screen=latest['screen']
            if screen and not screen['words']:
                blank.append({'frame':latest['number'],'sha256':sha(Path(screen['path'])),'warning':screen['warning'],'sampling':screen['sampling']})
        item['initial_empty_text_captures']=blank
    commands=read(folder/'commands.json')
    assert not any(c['name'] in ['capture','correlate','start_prediction_check','prepare_probe','prepare_change','apply_change'] for c in commands)
    if (folder/'selection.json').exists():
        selection=read(folder/'selection.json');summary=selection['summary'];windows=selection['windows']
        initial=read(folder/'initial-completed.json')['state']['live']
        assert summary['source_frame_id']==initial['frames'][-1]['id']
        assert summary['source_image_sha256']==initial['frames'][-1]['sha256']
        assert summary['retained_windows']==len(windows)<=4096
        assert summary['matched_windows']==summary['retained_windows']+summary['omitted_windows']
        assert all(w['length']==64 for w in windows)
        assert len({w['address'] for w in windows})==len(windows)
        assert all('bytes' not in w for w in windows)
        searched=read(folder/'searched.json')['state']['live']
        assert searched['coverage']['attempted_bytes']-initial['coverage']['attempted_bytes']==summary['attempted_bytes']
        assert searched['coverage']['read_bytes']-initial['coverage']['read_bytes']==summary['read_bytes']
        item.update({'selected_snapshot_bytes':sum(w['length'] for w in windows),'scan_mib':summary['read_bytes']/1048576,
            'scan_ms':summary['elapsed_ms'],'omitted_windows':summary['omitted_windows']})
    if (folder/'focused-completed.json').exists():
        fresh=read(folder/'focused-completed.json')['state']['live']
        old_ids={f['id'] for f in initial['frames']};new_ids={f['id'] for f in fresh['frames']}
        assert not old_ids.intersection(new_ids)
        for check in fresh['prediction_checks']:
            assert check['batch']==2 and check['after_frame']==12
            assert len(check['basis'])==12 and all(f['id'] in new_ids for f in check['basis'])
        if slot<2 and not report['errors']:
            selected=read(folder/'field-check.json');candidate=selected['candidate']
            assert int(candidate['address'],16)==report['initial_oracle']['field']
            assert candidate['mapping']['kind']==['F32','F64'][slot]
            assert candidate['invalidated'] is None and len(candidate['samples'])==4
            for sample in candidate['samples']:
                assert sample['outcome']=='agrees'
                before=sample['before_image']['read']['capture_interval'];after=sample['read']['capture_interval'];image=sample['image']['capture_interval']
                assert before['clock_id']==after['clock_id']==image['clock_id']
                assert before['end_us']<=image['begin_us']<=image['end_us']<=after['begin_us']
            item['fresh_samples']=[s['observed'] for s in candidate['samples']]
            item['field_mapping']=candidate['mapping']
    item['raw_sha256']={p.name:sha(p) for p in sorted(folder.glob('*.json'))}
    evidence['cases'].append(item)
(HERE/'evidence.json').write_text(json.dumps(evidence,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'strict_passes':sum(c['strict_passed'] for c in evidence['cases']),
 'cases':[{k:v for k,v in c.items() if k not in ['raw_sha256','report','initial_tracks','initial_empty_text_captures']} for c in evidence['cases']]}))
