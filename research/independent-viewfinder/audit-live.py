"""Offline audit of paged live discovery and its separately frozen continuation."""
import hashlib
import json
from pathlib import Path
import statistics
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT/'test-results/viewfinder-live-paging-v1'
FOLLOW = ROOT/'test-results/viewfinder-followup-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT/'frozen.json')
data = Path(frozen['data'])
for name, digest in frozen['sources'].items():
    assert sha(OUT/'source'/name) == digest, name
for name, digest in frozen['binaries'].items():
    assert sha(Path(frozen['bin'])/name) == digest, name
for name, digest in frozen['storage_before'].items():
    assert sha(OUT/'storage-before'/name) == digest, name
assert sha(OUT/'appmanifest-before.acf') == frozen['appmanifest_sha256']
follow = read(FOLLOW/'frozen.json')
assert follow['binaries'] == frozen['binaries'] and follow['data'] == frozen['data']
assert follow['max_comparisons'] == 4
for name, digest in {**follow['sources'], **follow['parent_artifacts']}.items():
    assert sha(FOLLOW/name) == digest, name
for name in ('frozen', 'result', 'final-state'):
    assert sha(OUT/f'{name}.json') == sha(FOLLOW/f'parent-{name}.json')
for folder in (OUT, FOLLOW):
    assert not (folder/'failure.json').exists() and not (folder/'needs-recovery.json').exists()
    assert read(folder/'result.json')['workflow_finished']
    assert read(folder/'result.json')['manually_reviewed_measurement_images'] == 0
    assert read(folder/'child.json')['exit_code'] == 0

discovery = read(OUT/'discovery.json')['state']['live']
result = read(OUT/'result.json')
assert result['samples'] == len(discovery['frames']) == 15
assert [i['key'] for i in result['evaluator_inputs']] == [68,65,68,65]
assert all(i['planned_ms']==200 and i['end_ns']>i['begin_ns'] for i in result['evaluator_inputs'])
assert result['jobs'] == [{'phase':'Failed','error':None}]
job = read(OUT/'discovery.json')['jobs']['history']['jobs']['1']
assert job['next_step']==16 and len(job['checkpoints'])==16
assert job['checkpoints'][-1]['outcome']['note']=='executor exceeded result budget; actual effect knowledge preserved'
review = discovery['runtime']['naming']
assert review['status'] == 'complete' and review == result['review']
naming = data/'runtime/naming'/review['evidence_id']
receipt, inventory = read(naming/'receipt.json'), read(naming/'inventory.json')
assert receipt['report_sha256'] == inventory['report_sha256'] == review['report_sha256']
assert receipt['error'] is None and not receipt['cancelled']
assert receipt['activity']['error'] is None
assert len(receipt['batch']['accepted']) == 6 and not receipt['batch']['rejected']
original = {t['id']:t for t in inventory['types']}
expected, saved_rows, stage_ids = set(original), {}, []
for number, round in enumerate(receipt['paged_selection']['rounds']):
    seen, selected = set(), set()
    for page in round['pages']:
        stage = receipt['stages'][page['stage_index']]
        assert len(stage['prompt'].encode()) <= 12000
        rows = json.loads(stage['prompt'].split('\nObserved types and path-scoped text examples: ',1)[1])['types']
        assert [r['id'] for r in rows] == page['type_ids']
        for row in rows:
            i = row['id']; assert i in expected and i not in seen; seen.add(i)
            assert row['field_labels'] == [f['name'] for f in original[i]['fields']]
            assert row['name'] == original[i]['key']['type_name'][:120]
            if number:
                assert row == saved_rows[i]
            else:
                saved_rows[i] = row
                for sample in row.get('object_samples', []):
                    candidates = [s for f in original[i]['fields'] if f['name']==sample['field_name']
                        for s in f['samples'] if s['path_id']==sample['path_id']]
                    assert any(all(text in [h['text'] for h in s['context'] if not h['truncated']]
                        for text in sample['object_text']) for s in candidates)
        ids = json.loads(stage['response']['message']['content'])['types']
        assert ids == page['selected'] and len(ids)==len(set(ids))<=8 and set(ids)<=set(page['type_ids'])
        selected.update(ids); stage_ids.append(page['stage_index'])
    assert seen == expected
    expected = selected
assert stage_ids == list(range(len(stage_ids))) and len(stage_ids)<=24
assert expected == set(receipt['selection']['selected_types']) and len(expected)<=8
for stage in receipt['stages']:
    assert hashlib.sha256(stage['prompt'].encode()).hexdigest() == stage['prompt_sha256']
    assert stage['response']['done'] and stage['response']['done_reason'] != 'length'
    assert stage['response']['prompt_eval_count'] <= stage['options']['num_ctx']-stage['options']['num_predict']-512
rows={r['id']:r for r in receipt['selection']['rows']}
for hypothesis in receipt['batch']['accepted']:
    candidate=hypothesis['candidate']
    assert candidate==rows[hypothesis['proposal']['candidate']]
    assert hypothesis['alternatives']==[rows[i] for i in hypothesis['proposal']['alternatives']]
    for citation in hypothesis['proposal']['citations']:
        text=candidate[citation['source']]
        if citation['source']=='object_text': text=text[citation['index']]
        assert text==citation['quote']
captures = [p for p in (data/'captures').iterdir() if (p/'frame-001.png').is_file()]
assert len(captures)==1
for f in discovery['frames']:
    assert sha(captures[0]/f"frame-{f['number']:03}.png") == f['sha256']

final = read(FOLLOW/'final-state.json')
assert len(final['state']['runtime_trials']) == len(final['state']['runtime_tests']) == 5
assert final['jobs']['in_flight'] is None and final['jobs']['fence'] is None
assert not final['state']['freezes'] and all(not t['active'] for t in final['state']['runtime_tests'])
trials = {t['id']:t for t in final['state']['runtime_trials']}
comparisons = [(OUT/'chosen-plan.json', OUT/'comparison.json')]
comparisons += [(FOLLOW/f'chosen-{n}.json',FOLLOW/f'comparison-{n}.json') for n in range(1,5)]
summaries, total_images, total_holds = [], 15, 4

def evidence(record):
    assert sha(Path(record['path'])) == record['report_sha256']
    assert read(Path(record['path'])) == record['report']

def raw_gray(path):
    # Independent measurement, not a reproduction of the product's resize/blur.
    pixels = np.asarray(Image.open(path).convert('RGB'),dtype=np.int32)
    return ((pixels[:,:,0]*299+pixels[:,:,1]*587+pixels[:,:,2]*114+500)//1000).astype(np.float64)

for chosen_path, comparison_path in comparisons:
    chosen, comparison = read(chosen_path), read(comparison_path)
    test = comparison['test']; trial = trials[test['change_id']]
    assert test == read(data/'runtime-tests'/f"{test['id']}.json")
    assert trial == read(data/'runtime-trials'/f"{trial['id']}.json")
    choice, args = chosen['plan']['next'], chosen['args']
    assert choice['status']=='untried'
    assert choice['hypothesis_ref'] == args['hypothesis_ref'] == test['origin']['reference']
    assert args['test_id']==test['id'] and args['change_id']==trial['id']
    assert args['actions']==choice['actions']==test['actions'] and args['value']==choice['value']==test['value']
    assert test['phase']=='complete' and trial['phase']=='restored'
    assert test['candidate']['id']==trial['candidate_id']==choice['hypothesis_ref']['selected_path_id']
    evidence(test['preparation']); evidence(trial['baseline'])
    for key in ('write','restoration'):
        action = trial[key]; evidence(action['evidence'])
        assert action['error'] is None and action['outcome']=='written'
        patch = action['evidence']['report']['patch']
        assert patch['guard']=='ready' and patch['state']=='written'
        assert patch['observed_after']==action['patch']['replacement']
    assert trial['restoration']['patch']['replacement']==trial['patch']['expected']
    assert trial['restoration']['patch']['expected']==trial['patch']['replacement']
    raw_medians = []
    for condition, c in enumerate(test['conditions']):
        assert len(c['frames']) == 6
        if condition:
            evidence(c['runtime_evidence'])
            expected_bytes = trial['patch']['replacement' if condition==1 else 'expected']
            assert c['runtime_evidence']['report']['checks'][0]['bytes']==expected_bytes
            boundary = trial['write' if condition==1 else 'restoration']['completed_ms']
            assert c['frames'][0]['started_unix_ms']>=boundary
        previous_pixels, raw_differences = None, []
        for i, frame in enumerate(c['frames']):
            assert sha(data/'runtime-tests'/frame['path'])==frame['sha256']
            assert sha(data/'runtime-tests'/frame['input_path'])==frame['input_sha256']
            window = read(data/'runtime-tests'/frame['input_path'])
            assert window['omitted_polls']==0 and all(p['focus']=='selected_foreground' for p in window['polls'])
            if i:
                assert frame['input']['matches']
            executed = comparison['executions'][condition*6+i]
            prefix = data/'runtime-tests'/test['id']/f'{condition}-{i}'
            assert sha(Path(str(prefix)+'.action.json'))==executed['plan_sha256']
            assert read(Path(str(prefix)+'.action.json'))==executed['plan']
            assert sha(Path(str(prefix)+'.action.outcome.json'))==executed['outcome']['sha256']
            assert read(Path(str(prefix)+'.action.outcome.json'))==executed['outcome']['receipt']
            action = None if i<2 else test['actions'][i-2]
            assert executed['plan']['action']==action
            outcome = executed['outcome']['receipt']
            assert outcome['error'] is None and outcome['plan_sha256']==executed['plan_sha256']
            assert outcome['keydown_accepted']==outcome['keyup_accepted']==(len(action['keys']) if action else 0)
            if action:
                assert any(p['down']==action['keys'] for p in window['polls'])
                assert all(not p['down'] or p['down']==action['keys'] for p in window['polls'])
                total_holds+=1
            pixels=raw_gray(data/'runtime-tests'/frame['path'])
            if i>=2:
                raw_differences.append(float(np.mean(np.abs(pixels-previous_pixels))))
            previous_pixels=pixels
            total_images+=1
        raw_medians.append(statistics.median(raw_differences))
    assert test['conditions'][1]['runtime_checked_unix_ms']<=trial['restoration']['started_ms']
    medians = [statistics.median([f['difference'] for f in c['frames'][2:]]) for c in test['conditions']]
    assert medians==test['assessment']['medians']
    for i in range(2,6):
        holds=[c['frames'][i]['input']['held_us'][0] for c in test['conditions']]
        assert max(holds)-min(holds)<=max(50000,max(holds)/4)
    positive=test['assessment']['status']=='suppression_observed'
    assert positive == (medians[1]<=max(2,.2*medians[0]))
    assert positive == (raw_medians[1]<.05*raw_medians[0] and raw_medians[2]>.8*raw_medians[0])
    summaries.append({'name':choice['name'],'role':choice['selection_role'],'test_id':test['id'],
        'observed_value':choice['observed_value'],'changed_value':test['value'],
        'assessment':test['assessment']['status'],'product_medians':medians,'independent_full_frame_medians':raw_medians,
        'trial_restored':True,'comparison_sha256':sha(comparison_path)})

helpers, pauses = [], []
for log in (data/'runtime').glob('*-*/**/stdout.jsonl'):
    events=[json.loads(line) for line in log.read_text().splitlines() if line.startswith('{')]
    assert events[-1]['stage']=='complete' and events[-1]['helper_absent'] and events[-1]['inspection_exit']==0
    report=read(log.with_name('report.json'))
    assert report['pid']==discovery['process']['pid'] and report['session']==3
    if 'snapshots' in report:
        assert sha(log.with_name('report.json'))==inventory['report_sha256']
    pauses.extend(report.get('snapshots',report.get('pauses',[])))
    pauses.extend(w['discarded_snapshot'] for w in report.get('warmups',[]))
    helpers.append(sha(log))
assert len(helpers)==31
assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in pauses)
cleanup=read(OUT/'cleanup.json')
assert cleanup['owned_processes_exited'] and cleanup['desktop_responsive']
assert all(cleanup['protection'].values()) and cleanup['new_reversebolt_detections']==0
after=read(OUT/'storage-after.json')['hashes']
save_unchanged={name: after[name]==frozen['storage_before'][name] for name in ('viewfinder_0.sav','viewfinder_0.bak')}
setup=read(OUT/'setup.json'); assert setup['gameplay_observed'] and setup['setup_images_reviewed']==3
summary={'artifact_audit':'passed','development_test':True,'release':Path(frozen['bin']).name,
    'outer_discovery_job':'Failed: cumulative result budget exceeded after hypotheses were published',
    'source_commit':frozen['source_commit'],'types':len(original),'fields':sum(len(t['fields']) for t in original.values()),
    'named_hypotheses':6,'rejected_hypotheses':0,'runtime_naming_model_calls':len(receipt['stages'])+1,
    'metadata_stage_ms':sum(s['elapsed_ms'] for s in receipt['stages']),
    'pages_per_round':[len(r['pages']) for r in receipt['paged_selection']['rounds']],
    'comparisons':summaries,'positive_comparisons':sum(s['assessment']=='suppression_observed' for s in summaries),
    'automatic_images':total_images,'input_holds':total_holds,'setup_images_reviewed':3,
    'manually_reviewed_measurement_images':0,'helper_runs':len(helpers),'pause_count':len(pauses),
    'max_stop_to_resume_us':max(p['stop_to_resume_us'] for p in pauses),'save_bytes_unchanged':save_unchanged,
    'frozen_sha256':sha(OUT/'frozen.json'),'followup_frozen_sha256':sha(FOLLOW/'frozen.json'),
    'receipt_sha256':sha(naming/'receipt.json'),'cleanup':cleanup,
    'limitation':'Known development target. One action-specific rendered suppression and return; no proof of exact gameplay semantics, units, complete world rollback, restart recovery or multi-game generalization.'}
(ROOT/'research/independent-viewfinder/live-results.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:summary[k] for k in ('artifact_audit','named_hypotheses','positive_comparisons','automatic_images','input_holds','helper_runs','save_bytes_unchanged')}))
