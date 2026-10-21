"""Audit a completed hypothesis-to-response fixture run without another model call."""
import hashlib,json,re,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];RUN=Path(sys.argv[1]).resolve();CASE=RUN/'case'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(RUN/'frozen.json')
for name,expected in frozen['sources'].items():assert sha(ROOT/name)==expected,name
for name,expected in frozen['binaries'].items():assert sha(RUN/'bin'/name)==expected,name
result=read(CASE/'result.json');assert result['passed']
selection=read(CASE/'selection.json');test=read(CASE/'complete.json')['test']
assert test['phase']=='complete' and test['assessment']['status']=='suppression_observed'
origin=test['origin'];assert origin['reference']==selection['reference'] and origin['hypothesis']==selection['hypothesis']
assert test['candidate']['id']==origin['reference']['selected_path_id']
assert test['hypothesis']==origin['hypothesis']['proposal']['question']
assert test['candidate']['edit_authority'] is False
discovered=read(CASE/'discovered.json')['state']['live']['runtime']
review=discovered['naming'];assert origin['report_sha256']==review['report_sha256']
assert origin['hypothesis'] in review['batch']['accepted']
receipt=read(CASE/'data/runtime/naming'/review['evidence_id']/'receipt.json')
assert receipt['review']==review
assert read(CASE/'submission.json')==read(CASE/'data/runtime/naming'/review['evidence_id']/'agent-request.json')['args']
records=[read(p) for p in (CASE/'data/runtime-tests').glob('*.json')]
assert all(t['phase'] in ('complete','stopped') for t in records)
assert next(t for t in records if t['id']==test['id'])['origin']==origin
images=0
for condition_index,condition in enumerate(test['conditions']):
    assert len(condition['frames'])==6
    for index,frame in enumerate(condition['frames']):
        for prefix in ('','input_'):
            path=CASE/'data/runtime-tests'/frame[prefix+'path']
            assert sha(path)==frame[prefix+'sha256']
        # The first frame establishes the time/image anchor, not a completed hold.
        if index:
            assert frame['input']['matches'] and frame['input']['largest_gap_us']<=50000
            window=read(CASE/'data/runtime-tests'/frame['input_path'])
            assert window['clock_id']==test['investigation'] and window['generation']==frame['input']['generation']
            assert all(p['focus']=='selected_foreground' for p in window['polls'])
        assert frame['path']==f"{test['id']}/{condition_index}-{index}.png"
        images+=1
assert images==18 and len(result['input_actions'])==12 and result['manually_reviewed_images']==0
trial=read(CASE/'data/runtime-trials'/f"{test['change_id']}.json")
assert trial['phase']=='restored'
assert read(CASE/'original-oracle.json')['bytes']==trial['patch']['expected']
assert trial['candidate_id']==test['candidate']['id'] and trial['context']['catalog_id']==test['catalog_id']
helpers=list((CASE/'data/runtime').glob('*-*/**/stdout.jsonl'));pauses=[];writes=0
for path in helpers:
    entries=[json.loads(s) for s in path.read_text().splitlines() if s.startswith('{')]
    assert entries[-1]['stage']=='complete' and entries[-1]['helper_absent'] and entries[-1]['inspection_exit']==0
    report=read(path.parent/'report.json');scope=report.get('scope',report)
    assert scope['pid']==test['scope']['pid'] and scope['session']==3
    pauses.extend(report.get('pauses',report.get('snapshots',[])))
    pauses.extend(w['discarded_snapshot'] for w in report.get('warmups',[]))
    writes+=int(report.get('operation')=='patch_runtime_path')
assert writes==2 and len(helpers)==result['helpers']
assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in pauses)
balance=read(CASE/'balance.json');assert balance['starts']==balance['stops']==len(pauses)
children=read(CASE/'children.json');assert all(c['exit_code'] is not None for c in children)
tests=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',(RUN/'workspace-tests.log').read_text(encoding='utf-8-sig'))))
assert tests==265,tests
summary={'passed':True,'artifact_root':str(RUN.relative_to(ROOT)),'frozen':frozen,
    'fixture_build':read(RUN/'fixture-build.json'),'native_checks':len(result['assertions']),'workspace_tests':tests,
    'selected_offboard':selection['offboard'],'alternative_prepared':result['alternative_tested_preparation'],
    'proposal_source':review['model'],
    'accepted_hypotheses':len(review['batch']['accepted']),'rejected_hypotheses':len(review['batch']['rejected']),
    'assessment':test['assessment'],'images':images,'input_actions':len(result['input_actions']),
    'manually_reviewed_images':0,'helpers':len(helpers),'writes_including_restore':writes,
    'balanced_pauses':len(pauses),'maximum_stop_to_resume_us':max(p['stop_to_resume_us'] for p in pauses),'children':children,
    'limitations':['Owned fixture explicitly labels its field WalkSpeed; evaluator selects that label and supplies the input plan.',
        'No game-specific product rule, but this is not independent game evaluation or proof of semantic meaning.',
        'Alternative preparation only is checked when available; competing effects are not compared in this workflow.',
        'Off-board resolution also has a synthetic 80-field report regression beyond the 64-card limit.',
        'Names, input choice and effect scope remain unverified; the result concerns rendered response only.']}
Path(__file__).with_name('results.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k not in ('frozen','children','fixture_build')},indent=2))
