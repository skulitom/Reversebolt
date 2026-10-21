"""Audit paired replay, native integration and desktop delivery; no inference."""
import collections
import hashlib
import json
from pathlib import Path
import re

ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
RAW=ROOT/'test-results/observation-resilience-v1'
NATIVE=ROOT/'test-results/observation-resilience-native-v1'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
manifest=read(RAW/'manifest.json'); results=read(RAW/'results.json')
delivery=read(ROOT/'test-results/observation-resilience-desktop-delivery.json')
assert len(manifest['cases'])==7 and len(results)==14
assert manifest['script_sha256']==sha(HERE/'replay.py')
assert manifest['labels_sha256']==sha(ROOT/'research/ocr-v6/labels.json')
assert manifest['legacy_scorer_sha256']==sha(ROOT/'research/ocr-v6/score_native.py')
assert manifest['models_manifest_sha256']==sha(ROOT/'models/manifest.json')
for name,binary in manifest['binaries'].items():
    # target/release is a build output and can change in a later turn. Audit the
    # identical versioned executable delivered after this replay instead.
    path=Path(delivery['path'] if name=='candidate' else binary['path'])
    assert sha(path)==binary['sha256']
images=0
for case in manifest['cases']:
    for frame in case['images']:
        assert sha(Path(frame['source']))==sha(Path(frame['copy']))==frame['sha256']
        images+=1
    for reader in manifest['binaries']:
        result=next(r for r in results if r['case']==case['name'] and r['reader']==reader)
        output=Path(result['output']);scored=output.with_name(reader+'-score.json')
        assert sha(output)==result['output_sha256'] and sha(scored)==result['score_sha256']
        assert len(read(output))==result['images']==len(case['images'])
        score=read(scored)
        if 'oracle' in case:
            assert sha(Path(case['oracle']))==case['oracle_sha256']
            counts={key:dict(collections.Counter(r['status'] for r in score['rows'] if r['counter']==key))
                    for key in result['outcomes']}
            assert counts==score['outcomes']==result['outcomes']
            if reader=='baseline':
                original=read(Path(case['oracle']))
                assert counts=={k:v['outcomes'] for k,v in original['counters'].items()}
        else:
            assert all(score[k]==v for k,v in result['outcomes'].items())
            assert score['wrong']==score['uncertain']==score['missing']==0
assert images==100

native=read(NATIVE/'ready.json');state=read(NATIVE/'state.json');trace=read(NATIVE/'trace.json')
assert native['binary_sha256']==manifest['binaries']['candidate']['sha256']
assert native['phase']=='Completed' and native['captures']==4 and native['warning_count']==2
assert any(t['warnings']==1 and t['captures']<4 for t in trace)
assert len(state['state']['live']['local_observation_warnings'])==2
assert not state['state']['live']['visual_tracks'] and not state['state']['patches']
assert any(t['samples']==[72.0]*4 for t in state['state']['live']['tracks'])
for name,digest in native['receipts'].items():
    p=NATIVE/'captures/model-observations'/name
    assert sha(p)==digest and read(p)['proposed_observations']==[]
ui=read(NATIVE/'uia.json')
names=lambda stage:[e['name'] for e in ui[stage]['structuredContent']['elements']]
assert 'Quantity' in names('initial') and '72' in names('initial')
assert 'Visual review warnings' not in names('initial')
assert 'The requested model is not installed locally.' in names('expanded')
assert 'These reviews contributed no findings. Other observation continued.' in names('expanded')
failed=ROOT/'test-results/independent-games-v1/tetris-1-attempt-01/engine/captures/model-observations/6070e86b-5fbb-486e-b1d0-a700dec284fc.json'
fixture=ROOT/'crates/reversebolt-app/tests/fixtures/overlong-observation.json'
assert fixture.read_text()==read(failed)['pass']['proposal']['response']['message']['content']
tests=(ROOT/'test-results/observation-resilience-tests.log').read_text(encoding='utf-8-sig')
test_count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed;',tests)))
assert test_count==132 and 'test result: FAILED' not in tests
smoke=read(ROOT/'test-results/smoke-4246633c/report.json')
assert smoke['mcp_tools']==29
assert sha(Path(delivery['path']))==delivery['sha256']==native['binary_sha256']
assert delivery['history_preserved'] and delivery['packaged_binary_and_model_hashes_verified']

evidence={'scope':manifest['scope'],'binaries':manifest['binaries'],'source_images_audited':images,
    'paired_replays':results,'native':native,'desktop':delivery,'workspace_tests':test_count,
    'raw_files':{str(p.relative_to(ROOT)):sha(p) for p in [RAW/'manifest.json',RAW/'results.json',
        NATIVE/'state.json',NATIVE/'trace.json',NATIVE/'uia.json',fixture,
        ROOT/'test-results/observation-resilience-tests.log',ROOT/'test-results/observation-resilience-clippy.log',
        ROOT/'test-results/observation-resilience-build.log',ROOT/'test-results/smoke-4246633c/report.json']},
    'progress_percent':25,'limits':['Exposed saved-image selected-field regression, not full-scene precision or new-game acceptance.',
        'Wide regions still missing; partial neural results can introduce explicit conflicts.',
        'Native missing-model fixture tests continuation, not semantic accuracy.',
        'No new controlling memory, gameplay effect or editable game control verified.']}
folder=HERE/'evidence';folder.mkdir(exist_ok=True)
(folder/'v1.json').write_text(json.dumps(evidence,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'source_images':images,'paired_replays':len(results),'workspace_tests':test_count,'desktop':delivery['version']}))
