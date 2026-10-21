"""Post-run evidence audit. Does not run the target or define acceptance criteria."""
from pathlib import Path
import hashlib,json

HERE=Path(__file__).resolve().parent
ROOT=HERE.parent.parent
digest=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
read=lambda path:json.loads(path.read_text(encoding='utf-8'))
protocol=read(HERE/'protocol-v1.json')
for name,expected in protocol['method_sha256'].items():
    assert digest(HERE/name)==expected,name
for name,expected in protocol['product_source_sha256'].items():
    assert digest(HERE/'source'/name)==expected,name
for name,expected in protocol['binaries'].items():
    assert digest(Path(protocol['binary_directory'])/name)==expected,name
evidence={'protocol_sha256':digest(HERE/'protocol-v1.json'),'audit':'Written after both frozen runs; no reruns or replacement observations.','cases':[]}
for slot in [0,1]:
    case=ROOT/'test-results/recheck-findings-v1'/f'case-{slot}'
    report=read(case/'report.json')
    assert report['protocol_sha256']==evidence['protocol_sha256']
    assert report['errors']==([] if slot==0 else ['AssertionError()']),report
    start=read(case/'recheck-start.json')
    completed=read(case/'recheck-completed.json')
    assert start['state']['live']['sample_count']==0
    assert completed['state']['live']['sample_count']==4
    planned=completed['state']['live']['planned_bytes']
    requested=completed['state']['live']['coverage']['attempted_bytes']-start['state']['live']['coverage']['attempted_bytes']
    assert planned==8 and requested==64
    job=str(start['data']['automatic_observation']['ack']['job'])
    assert completed['jobs']['history']['jobs'][job]['phase']=='Completed'
    assert len(completed['jobs']['history']['jobs'][job]['steps'])==4
    assert all(json.loads(s['payload']).get('local_model') is None for s in completed['jobs']['history']['jobs'][job]['steps'])
    assert not read(case/'unrestored-recheck.json')['ok']
    assert not read(case/'stale-recheck.json')['ok']
    before=read(case/'discovery-status.json')['state']['live']
    checks=[]
    for check in completed['state']['live']['prediction_checks']:
        origin=check['rechecked_from']
        original=next(c for c in before['prediction_checks'] if c['id']==origin['check'])
        assert check['id']!=original['id'] and check['after_frame']==0
        assert [(c['address'],c['mapping']) for c in check['candidates']]==[(c['address'],c['mapping']) for c in original['candidates']]
        archived=read(Path(origin['archive']))
        assert any(c['id']==original['id'] and c['invalidated'] for c in archived['state']['prediction_checks'])
        candidates=[]
        for candidate in check['candidates']:
            assert [s['frame'] for s in candidate['samples']]==[1,2,3,4]
            assert all(s['outcome']=='agrees' for s in candidate['samples'])
            candidates.append({'address':candidate['address'],'mapping':candidate['mapping'],
                'observed':[s['observed'] for s in candidate['samples']],
                'outcomes':[s['outcome'] for s in candidate['samples']]})
        checks.append({'id':check['id'],'from':origin,'track':check['track'],'candidates':candidates})
    prepared=read(case/'fresh-prepared.json')
    if slot==0:
        assert prepared['ok'] and prepared['data']['phase']=='prepared'
        assert read(case/'fresh-applied.json')['phase']=='written'
        assert read(case/'fresh-effect.json')['observations'][-1]['intervention_outcome']=='matched'
        assert read(case/'fresh-restored.json')['phase']=='restored'
    else:
        # Preserve the frozen runner's failure. Its exact-wording expectation
        # was wrong; this is a post-run classification, not a replacement pass.
        assert not prepared['ok'] and prepared['error']=='This location has overlapping interpretations that still need fresh evidence.'
        assert all(c['scope']['batch']==1 for c in prepared['state']['changes'])
    commands=read(case/'commands.json')
    assert not any(c['name'] in ['capture','correlate','start_prediction_check'] for c in commands)
    evidence['cases'].append({'report':report,'checks':checks,'preparation_ok':prepared['ok'],
        'strict_run_passed':slot==0,'requested_bytes':requested,'planned_bytes':planned,
        'post_run_classification':'fresh edit applied, observed and restored' if slot==0 else 'edit refused; frozen runner failed its error-wording assertion',
        'preparation_error':prepared.get('error'),
        'raw_sha256':{p.name:digest(p) for p in sorted(case.glob('*.json'))}})
(HERE/'evidence.json').write_text(json.dumps(evidence,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'audit':'passed','strict_passes':1,'strict_failures':1,'cases':[c['post_run_classification'] for c in evidence['cases']]}))
