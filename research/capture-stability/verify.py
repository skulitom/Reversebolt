"""Post-run audit; never alters a frozen assertion or replaces a failed case."""
from pathlib import Path
import hashlib,json
from PIL import Image
HERE=Path(__file__).resolve().parent;ROOT=HERE.parent.parent
read=lambda p:json.loads(p.read_text(encoding='utf-8'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
protocol=read(HERE/'protocol-v1.json')
for name,h in protocol['method_sha256'].items():assert sha(HERE/name)==h,name
for name,h in protocol['product_source_sha256'].items():assert sha(HERE/'source'/name)==h,name
for name,h in protocol['binaries'].items():assert sha(Path(protocol['binary_directory'])/name)==h,name
def walk(value):
    if isinstance(value,dict):
        yield value
        for child in value.values():yield from walk(child)
    elif isinstance(value,list):
        for child in value:yield from walk(child)

evidence={'protocol_sha256':sha(HERE/'protocol-v1.json'),
 'audit':'Post-run integrity, sampling and outcome audit; original runner failures remain failures.', 'cases':[]}
for slot in [0,1,2]:
    folder=ROOT/'test-results/capture-stability-v1'/f'case-{slot}'
    report=read(folder/'report.json');assert report['protocol_sha256']==evidence['protocol_sha256']
    issues=[];images={}
    for path in sorted(folder.glob('*.json')):
        for item in walk(read(path)):
            image=item.get('path') if 'method' in item else item.get('image')
            if not isinstance(image,str) or not image or 'sampling' not in item:continue
            if image in images:continue
            sampling=item['sampling'];file=Path(image)
            row={'path':image,'sha256':sha(file),'sampling':sampling,'warning':item.get('warning')}
            images[image]=row
            if sampling is None:
                issues.append('Missing WGC receipt: '+image);continue
            frames=sampling['frames'];times=[f['completed_us'] for f in frames]
            valid=(sampling['policy']=='last_completed_in_fixed_window_v1' and sampling['settle_ms']==80
                   and sampling['max_frames']==8 and 1<=len(frames)<=8 and sampling['selected']==len(frames)-1
                   and times==sorted(times) and times[-1]-times[0]<80000)
            if not valid:issues.append('Sampling policy mismatch: '+image)
            selected=frames[sampling['selected']]
            with Image.open(file) as im:
                actual=hashlib.sha256(im.convert('RGBA').tobytes()).hexdigest()
                if actual!=selected['rgba_sha256'] or im.size!=(selected['width'],selected['height']):
                    issues.append('Selected image mismatch: '+image)
            if any(f['compositor_100ns'] is None for f in frames):issues.append('Missing source timestamp: '+image)
            row.update({'sampled_frames':len(frames),'span_us':times[-1]-times[0],
                'first_low_detail':frames[0]['detail_fraction']<.001,'selected_low_detail':selected['detail_fraction']<.001})
    commands=read(folder/'commands.json')
    if slot<2:
        assert not any(c['name'] in ['capture','correlate','start_prediction_check'] for c in commands)
        if not report['errors']:
            discovered=read(folder/'discovery-completed.json')['state']['live']
            original=next(c for c in discovered['prediction_checks'] if c['source']=='ocr')
            assert next(t for t in discovered['tracks'] if t['id']==original['track'])['samples']==protocol['values']
            fresh=read(folder/'recheck-completed.json')['state']['live']
            check=next(c for c in fresh['prediction_checks'] if c['source']=='ocr' and c['track']==original['track'])
            selected=check['candidates'][report['chosen_candidate']['index']]
            assert int(selected['address'],16)==report['moved_oracle']['field']
            assert selected['via']['root']['address']==report['initial_oracle']['root']
            assert selected['via']['offset']==report['initial_oracle']['offset']
            assert [s['observed'] for s in selected['samples']]==protocol['fresh_values']
            for sample in selected['samples']:
                assert sample['outcome']=='agrees'
                before,after=sample['reference_before'],sample['reference_after']
                assert before['bytes']==after['bytes']
                intervals=[before['read']['capture_interval'],sample['before_image']['read']['capture_interval'],
                    sample['image']['capture_interval'],sample['read']['capture_interval'],after['read']['capture_interval']]
                assert len({i['clock_id'] for i in intervals})==1
                assert all(i['begin_us']<=i['end_us'] for i in intervals)
                assert all(a['end_us']<=b['begin_us'] for a,b in zip(intervals,intervals[1:]))
            prepared=read(folder/'prepared.json')['data'];assert all(m['reference'] for m in prepared['models'])
            if slot==0:
                assert read(folder/'applied.json')['phase']=='written'
                assert report['applied_oracle']['value']==53
                effect=read(folder/'effect.json');assert effect['observations'][-1]['intervention_outcome']=='matched'
                assert read(folder/'restored.json')['phase']=='restored' and report['final_oracle']['value']==50
            else:
                refused=read(folder/'refused.json');assert not refused['ok'] and 'reference moved' in refused['error'].lower()
                assert report['retained_old_value']==report['final_oracle']['value']==50
    elif not report['errors']:
        assert len(report['sampling'])==4 and len(images)==4
        assert all(i['selected_low_detail'] and i['warning'] for i in images.values())
        assert not any(c['name'] in ['prepare_change','prepare_probe','apply_change'] for c in commands)
    rows=list(images.values());with_sampling=[i for i in rows if i.get('sampling')]
    case={'report':report,'runner_passed':not report['errors'],'sampling_issues':issues,
      'captures':len(rows),'first_low_detail':sum(i['first_low_detail'] for i in with_sampling),
      'selected_low_detail':sum(i['selected_low_detail'] for i in with_sampling),
      'frame_counts':[i['sampled_frames'] for i in with_sampling],
      'maximum_completion_span_us':max((i['span_us'] for i in with_sampling),default=0),
      'images':rows,'raw_sha256':{p.name:sha(p) for p in sorted(folder.glob('*.json'))}}
    evidence['cases'].append(case)
(HERE/'evidence.json').write_text(json.dumps(evidence,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'runner_passes':sum(c['runner_passed'] for c in evidence['cases']),
 'cases':[{k:c[k] for k in ['captures','first_low_detail','selected_low_detail','frame_counts','maximum_completion_span_us','sampling_issues']} for c in evidence['cases']]}))
assert not any(c['sampling_issues'] for c in evidence['cases'])
