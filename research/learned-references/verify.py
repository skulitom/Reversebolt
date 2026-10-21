"""Post-run audit of retained evidence, including the failed first case."""
from pathlib import Path
import hashlib,json
from PIL import Image,ImageStat

HERE=Path(__file__).resolve().parent;ROOT=HERE.parent.parent
read=lambda p:json.loads(p.read_text(encoding='utf-8'))
digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
protocol=read(HERE/'protocol-v1.json')
for name,h in protocol['method_sha256'].items():assert digest(HERE/name)==h,name
for name,h in protocol['product_source_sha256'].items():assert digest(HERE/'source'/name)==h,name
for name,h in protocol['binaries'].items():assert digest(Path(protocol['binary_directory'])/name)==h,name
evidence={'protocol_sha256':digest(HERE/'protocol-v1.json'),'audit':'Post-run integrity and outcome classification; the frozen failure was not rerun.','cases':[]}
for slot in [0,1]:
    folder=ROOT/'test-results/learned-references-v1'/f'case-{slot}'
    report=read(folder/'report.json');assert report['protocol_sha256']==evidence['protocol_sha256']
    discovered=read(folder/'discovery-completed.json')['state']['live']
    original=next(c for c in discovered['prediction_checks'] if c['source']=='ocr')
    track=next(t for t in discovered['tracks'] if t['id']==original['track'])
    initial=report['initial_oracle']
    assert any(int(c['address'],16)==initial['field'] and any(r['root']['address']==initial['root'] and r['offset']==initial['offset'] for r in c['references']['routes']) for c in original['candidates'])
    item={'report':report,'initial_samples':track['samples'],'initial_candidates':len(original['candidates']),
        'initial_routes':sum(len(c['references']['routes']) for c in original['candidates']),
        'strict_passed':not report['errors']}
    commands=read(folder/'commands.json');assert not any(c['name'] in ['capture','correlate','start_prediction_check'] for c in commands)
    if slot==0:
        assert report['errors']==['AssertionError()']
        assert track['samples'][:-1]==protocol['values'][:-1] and track['samples'][-1] is None
        assert not any(c['name'] in ['recheck_findings','prepare_probe','apply_change'] for c in commands)
        item['classification']='Failed at missing final initial OCR reading; relocation and edits not exercised.'
        states=read(folder/'discovery-states.json')
        images=[]
        for state in states[-2:]:
            latest=state['live']['latest'];image_path=Path(latest['screen']['path'])
            with Image.open(image_path) as im:
                im=im.convert('RGB');stats=ImageStat.Stat(im)
                images.append({'frame':latest['number'],'sha256':digest(image_path),'mean':stats.mean,'variance':stats.var,
                    'text':latest['screen']['text'],'words':len(latest['screen']['words']),'recognition_ms':latest['recognition_ms']})
        item['post_run_image_metrics']=images
    else:
        assert not report['errors']
        fresh=read(folder/'recheck-completed.json')['state']['live']
        current=next(c for c in fresh['prediction_checks'] if c['source']=='ocr' and c['track']==original['track'])
        selected=current['candidates'][report['chosen_candidate']['index']]
        assert current['rechecked_from']['check']==original['id']
        assert int(selected['address'],16)==report['moved_oracle']['field']
        assert selected['via']['root']['address']==initial['root'] and selected['via']['offset']==initial['offset']
        assert [s['observed'] for s in selected['samples']]==protocol['fresh_values']
        witnesses=[]
        for s in selected['samples']:
            assert s['outcome']=='agrees'
            before,after=s['reference_before'],s['reference_after']
            assert before['bytes']==after['bytes']
            first=before['read']['capture_interval'];last=after['read']['capture_interval']
            value_first=s['before_image']['read']['capture_interval'];value_last=s['read']['capture_interval'];image=s['image']['capture_interval']
            assert first['clock_id']==last['clock_id']==value_first['clock_id']==value_last['clock_id']==image['clock_id']
            assert first['end_us']<=value_first['begin_us']<=value_first['end_us']<=image['begin_us']<=image['end_us']<=value_last['begin_us']<=value_last['end_us']<=last['begin_us']
            assert int.from_bytes(bytes(before['bytes']),'little')+selected['via']['offset']==report['moved_oracle']['field']
            witnesses.append({'frame':s['frame'],'observed':s['observed'],'pointer_bytes':before['bytes'],
                'reference_before':first,'value_before':value_first,'image':image,'value_after':value_last,'reference_after':last})
        prepared=read(folder/'prepared.json');assert prepared['ok'] and prepared['data']['phase']=='prepared'
        assert all(m['reference'] for m in prepared['data']['models'])
        refused=read(folder/'refused.json');assert not refused['ok'] and 'reference moved' in refused['error'].lower()
        assert report['retained_old_value']==report['final_oracle']['value']==50
        assert report['final_oracle']['generation']==2
        item.update({'classification':'Learned relocation, fresh validation and preparation passed; second move blocked the stale write.',
            'fresh_candidate_count':len(current['candidates']),'selected':report['chosen_candidate'],'witnesses':witnesses,'refusal':refused['error']})
    item['raw_sha256']={p.name:digest(p) for p in sorted(folder.glob('*.json'))}
    evidence['cases'].append(item)
(HERE/'evidence.json').write_text(json.dumps(evidence,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'audit':'passed','strict_passes':sum(c['strict_passed'] for c in evidence['cases']),
    'cases':[c['classification'] for c in evidence['cases']], 'image_metrics':evidence['cases'][0]['post_run_image_metrics']}))
