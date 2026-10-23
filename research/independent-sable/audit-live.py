"""Read-only artifact validation of the separately staged Sable development run."""
import csv
import hashlib
import json
from pathlib import Path
import statistics
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT/'test-results/sable-live-context-v1'
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
assert frozen['development_retest'] and frozen['max_comparisons'] == 1
assert not (OUT/'failure.json').exists() and not (OUT/'needs-recovery.json').exists()
job = read(OUT/'anode-job.json')
assert job['finished'] and job['state'] == 'completed' and job['exitCode'] == 0
result = read(OUT/'result.json')
assert result['workflow_finished'] and result['manually_reviewed_measurement_images'] == 0
assert read(OUT/'child.json')['exit_code'] == 0
setup = read(OUT/'setup.json')
assert setup['gameplay_observed'] and setup['setup_images_reviewed'] <= 4
assert setup['manually_reviewed_measurement_images'] == 0
discovery = read(OUT/'discovery.json')
live = discovery['state']['live']
assert result['samples'] == len(live['frames']) == 15
assert [i['key'] for i in result['evaluator_inputs']] == [68,65,68,65]
assert all(i['planned_ms'] == 200 and i['end_ns'] > i['begin_ns'] for i in result['evaluator_inputs'])
captures = [p for p in (data/'captures').iterdir() if (p/'frame-001.png').is_file()]
assert len(captures) == 1
for frame in live['frames']:
    assert sha(captures[0]/f"frame-{frame['number']:03}.png") == frame['sha256']
review = live['runtime']['naming']
assert review == result['review']
directory = data/'runtime/naming'/review['evidence_id']
receipt, inventory = read(directory/'receipt.json'), read(directory/'inventory.json')
assert receipt['report_sha256'] == inventory['report_sha256'] == review['report_sha256']
original = {t['id']:t for t in inventory['types']}
expected, saved_rows, selected_stages = set(original), {}, []
for number, round_ in enumerate(receipt['paged_selection']['rounds']):
    seen, selected = set(), set()
    for page in round_['pages']:
        stage = receipt['stages'][page['stage_index']]
        assert len(stage['prompt'].encode()) <= 12000
        rows = json.loads(stage['prompt'].split('\nObserved types and path-scoped text examples: ',1)[1])['types']
        assert [r['id'] for r in rows] == page['type_ids']
        for row in rows:
            i = row['id']; assert i in expected and i not in seen; seen.add(i)
            assert row['field_labels'] == [f['name'] for f in original[i]['fields']]
            if number: assert row == saved_rows[i]
            else: saved_rows[i] = row
        ids = json.loads(stage['response']['message']['content'])['types']
        assert ids == page['selected'] and len(ids) == len(set(ids)) <= 8
        assert set(ids) <= set(page['type_ids']); selected.update(ids)
        selected_stages.append(page['stage_index'])
    assert seen == expected
    expected = selected
if receipt['selection']:
    assert expected == set(receipt['selection']['selected_types'])
assert selected_stages == list(range(len(selected_stages)))
stages = receipt['stages']
incomplete = receipt.get('incomplete_stage')
for stage in stages + ([incomplete['stage']] if incomplete else []):
    assert hashlib.sha256(stage['prompt'].encode()).hexdigest() == stage['prompt_sha256']
for stage in stages:
    response, options = stage['response'], stage['options']
    assert response['done'] and response['done_reason'] != 'length'
    assert response['prompt_eval_count'] <= options['num_ctx']-options['num_predict']-512
activity = receipt.get('activity')
if activity and activity.get('stage'):
    activity_stage = activity['stage']
    assert hashlib.sha256(activity_stage['prompt'].encode()).hexdigest() == activity_stage['prompt_sha256']
    assert activity_stage['response']['done'] and activity_stage['response']['done_reason'] != 'length'
if incomplete:
    assert incomplete['error'] == receipt['error'] and incomplete['phase'] in ('preflight','request','identity_recheck')
batch = receipt.get('batch') or {}
if batch:
    rows = {r['id']:r for r in receipt['selection']['rows']}
    for hypothesis in batch['accepted']:
        candidate = hypothesis['candidate']
        assert candidate == rows[hypothesis['proposal']['candidate']]
        assert hypothesis['alternatives'] == [rows[i] for i in hypothesis['proposal']['alternatives']]
        for citation in hypothesis['proposal']['citations']:
            text = candidate[citation['source']]
            if citation['source'] == 'object_text': text = text[citation['index']]
            assert text == citation['quote']
helpers, pauses = [], []
for log in (data/'runtime').glob('*-*/**/stdout.jsonl'):
    events = [json.loads(line) for line in log.read_text().splitlines() if line.startswith('{')]
    assert events[-1]['stage'] == 'complete' and events[-1]['helper_absent'] and events[-1]['inspection_exit'] == 0
    report = read(log.with_name('report.json'))
    assert report['pid'] == live['process']['pid'] and report['session'] == 3
    if 'snapshots' in report: assert sha(log.with_name('report.json')) == inventory['report_sha256']
    pauses.extend(report.get('snapshots',report.get('pauses',[])))
    pauses.extend(w['discarded_snapshot'] for w in report.get('warmups',[]))
    helpers.append(sha(log))
assert all(p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0 for p in pauses)
final = read(OUT/'final-state.json')
assert final['jobs']['fence'] is None and final['jobs']['in_flight'] is None
assert not any(t['active'] for t in final['state']['runtime_tests'])
assert not any(f['active'] for f in final['state']['freezes'])
assert not any(t['phase'] in ('written','uncertain') for t in final['state']['runtime_trials'])
comparison_summary = None
automatic_images, input_holds = 15, 4
if result['comparison']:
    chosen, comparison = read(OUT/'chosen-plan.json'), read(OUT/'comparison.json')
    test = comparison['test']
    assert len(final['state']['runtime_tests']) == len(final['state']['runtime_trials']) == 1
    trial = final['state']['runtime_trials'][0]
    assert test == read(data/'runtime-tests'/f"{test['id']}.json")
    test_summary = final['state']['runtime_tests'][0]
    for key in ('id','phase','change_id','catalog_id','actions','assessment','revision'):
        assert test[key] == test_summary[key]
    assert test_summary['frames'] == [len(c['frames']) for c in test['conditions']]
    assert test_summary['candidate_id'] == test['candidate']['id']
    assert trial == read(data/'runtime-trials'/f"{trial['id']}.json")
    assert test['phase'] == 'complete' and trial['phase'] == 'restored'
    choice, args = chosen['plan']['next'], chosen['args']
    assert choice['status'] == 'untried'
    assert choice['hypothesis_ref'] == args['hypothesis_ref'] == test['origin']['reference']
    assert args['test_id'] == test['id'] and args['change_id'] == trial['id'] == test['change_id']
    assert args['actions'] == choice['actions'] == test['actions']
    assert args['value'] == choice['value'] == test['value']
    assert test['candidate']['id'] == trial['candidate_id'] == choice['hypothesis_ref']['selected_path_id']
    def evidence(record):
        assert sha(Path(record['path'])) == record['report_sha256']
        assert read(Path(record['path'])) == record['report']
    evidence(test['preparation']); evidence(trial['baseline'])
    for key in ('write','restoration'):
        action = trial[key]; evidence(action['evidence'])
        assert action['error'] is None and action['outcome'] == 'written'
        patch = action['evidence']['report']['patch']
        assert patch['guard'] == 'ready' and patch['state'] == 'written'
        assert patch['observed_after'] == action['patch']['replacement']
    assert trial['restoration']['patch']['replacement'] == trial['patch']['expected']
    assert trial['restoration']['patch']['expected'] == trial['patch']['replacement']
    raw_medians = []
    assert len(test['conditions']) == 3 and len(comparison['executions']) == 18
    for condition, c in enumerate(test['conditions']):
        assert len(c['frames']) == 6
        if condition:
            evidence(c['runtime_evidence'])
            expected_bytes = trial['patch']['replacement' if condition == 1 else 'expected']
            assert c['runtime_evidence']['report']['checks'][0]['bytes'] == expected_bytes
            boundary = trial['write' if condition == 1 else 'restoration']['completed_ms']
            assert c['frames'][0]['started_unix_ms'] >= boundary
        previous, differences = None, []
        for i, frame in enumerate(c['frames']):
            assert sha(data/'runtime-tests'/frame['path']) == frame['sha256']
            assert sha(data/'runtime-tests'/frame['input_path']) == frame['input_sha256']
            window = read(data/'runtime-tests'/frame['input_path'])
            assert window['omitted_polls'] == 0 and all(p['focus'] == 'selected_foreground' for p in window['polls'])
            if i: assert frame['input']['matches']
            executed = comparison['executions'][condition*6+i]
            prefix = data/'runtime-tests'/test['id']/f'{condition}-{i}'
            assert sha(Path(str(prefix)+'.action.json')) == executed['plan_sha256']
            assert read(Path(str(prefix)+'.action.json')) == executed['plan']
            assert sha(Path(str(prefix)+'.action.outcome.json')) == executed['outcome']['sha256']
            assert read(Path(str(prefix)+'.action.outcome.json')) == executed['outcome']['receipt']
            action = None if i < 2 else test['actions'][i-2]
            assert executed['plan']['action'] == action
            outcome = executed['outcome']['receipt']
            assert outcome['error'] is None and outcome['plan_sha256'] == executed['plan_sha256']
            assert outcome['keydown_accepted'] == outcome['keyup_accepted'] == (len(action['keys']) if action else 0)
            if action:
                assert any(p['down'] == action['keys'] for p in window['polls'])
                assert all(not p['down'] or p['down'] == action['keys'] for p in window['polls'])
                input_holds += 1
            # Full-resolution metric independent of the product's resize and blur.
            pixels = np.asarray(Image.open(data/'runtime-tests'/frame['path']).convert('RGB'),dtype=np.int32)
            gray = ((pixels[:,:,0]*299+pixels[:,:,1]*587+pixels[:,:,2]*114+500)//1000).astype(np.float64)
            if i >= 2: differences.append(float(np.mean(np.abs(gray-previous))))
            previous = gray; automatic_images += 1
        raw_medians.append(statistics.median(differences))
    assert test['conditions'][1]['runtime_checked_unix_ms'] <= trial['restoration']['started_ms']
    medians = [statistics.median(f['difference'] for f in c['frames'][2:]) for c in test['conditions']]
    assert medians == test['assessment']['medians']
    for i in range(2,6):
        holds = [c['frames'][i]['input']['held_us'][0] for c in test['conditions']]
        assert max(holds)-min(holds) <= max(50000,max(holds)/4)
    comparison_summary = dict(name=choice['name'], observed_value=choice['observed_value'],
        changed_value=test['value'], assessment=test['assessment']['status'],
        product_medians=medians, independent_full_frame_medians=raw_medians,
        trial_restored=True, comparison_sha256=sha(OUT/'comparison.json'))
else:
    assert not final['state']['runtime_tests'] and not final['state']['runtime_trials']
cleanup = read(OUT/'cleanup.json')
assert cleanup['owned_processes_exited'] and cleanup['desktop_responsive']
assert all(cleanup['protection'].values()) and cleanup['new_reversebolt_detections'] == 0
assert cleanup['launcher_unchanged'] and cleanup['shortcut_unchanged']
end = read(OUT/'telemetry-end.json')
assert end['reason'] == 'stop_requested'
model_samples = [json.loads(line) for line in (OUT/'model-state.jsonl').read_text().splitlines()]
assert all(s['gpu_process_exit'] is None for s in model_samples)
# The terminated nvidia-smi child may leave a partial final buffered line.
gpu_samples, incomplete_gpu_rows = [], 0
with (OUT/'gpu.csv').open(newline='') as f:
    for row in csv.DictReader(f, skipinitialspace=True):
        if any(v is None for v in row.values()) or len(row) != 7:
            incomplete_gpu_rows += 1; continue
        gpu_samples.append({k: int(v.split()[0]) for k,v in row.items() if k != 'timestamp'})
assert gpu_samples and incomplete_gpu_rows <= 1
after = read(OUT/'storage-after.json')['hashes']
for name, digest in after.items():
    assert sha(OUT/'storage-after'/name) == digest
storage_differences = dict(changed=[n for n,d in frozen['storage_before'].items() if after.get(n) != d],
                           new=[n for n in after if n not in frozen['storage_before']])
activation = read(OUT/'activation.json')
assert activation['version'] == Path(frozen['bin']).name and activation['tested_binary_path_preserved']
assert sha(Path(frozen['bin'])/'manifest.json') == activation['manifest_sha256'].lower()
assert sha(OUT/'activation-script.ps1') == activation['activation_script_sha256'].lower()
assert not activation['security_settings_changed'] and activation['main_editor']['responsive']
summary = dict(artifact_audit='passed', development_retest=True, source_commit=frozen['source_commit'],
    release=Path(frozen['bin']).name, jobs=result['jobs'], evaluator_elapsed_ms=job['elapsedMs'],
    types=len(original), fields=sum(len(t['fields']) for t in original.values()),
    named_hypotheses=[h['proposal']['name'] for h in batch.get('accepted',[])],
    metadata_error=receipt['error'], incomplete_request_phase=incomplete['phase'] if incomplete else None,
    completed_metadata_requests=len(stages), metadata_stage_ms=sum(s['elapsed_ms'] for s in stages),
    incomplete_request_ms=incomplete['stage']['elapsed_ms'] if incomplete else None,
    pages_per_round=[len(r['pages']) for r in receipt['paged_selection']['rounds']],
    activity_error=(activity or {}).get('error'), activity_stage_ms=((activity or {}).get('stage') or {}).get('elapsed_ms'),
    comparison=comparison_summary,
    automatic_images=automatic_images, input_holds=input_holds, setup_images_reviewed=setup['setup_images_reviewed'],
    manually_reviewed_measurement_images=0,
    helper_runs=len(helpers), pause_count=len(pauses),
    max_stop_to_resume_us=max((p['stop_to_resume_us'] for p in pauses),default=None),
    model_contexts=sorted({m['context_length'] for s in model_samples for m in s.get('models',[])}),
    telemetry_errors=sum('error' in s for s in model_samples), gpu_complete_samples=len(gpu_samples),
    gpu_incomplete_rows=incomplete_gpu_rows, minimum_reported_free_mib=min(s['memory.free [MiB]'] for s in gpu_samples),
    maximum_reported_used_mib=max(s['memory.used [MiB]'] for s in gpu_samples),
    gpu_csv_sha256=sha(OUT/'gpu.csv'), model_telemetry_sha256=sha(OUT/'model-state.jsonl'),
    activation_sha256=sha(OUT/'activation.json'),
    storage_differences=storage_differences, frozen_sha256=sha(OUT/'frozen.json'),
    receipt_sha256=sha(directory/'receipt.json'), result_sha256=sha(OUT/'result.json'),
    cleanup=cleanup, limitation='Development retest. Names are hypotheses; telemetry is global and observational. A rendered-response comparison does not establish gameplay semantics, generalization or full world rollback.')
(Path(__file__).parent/'live-results.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k != 'cleanup'},indent=2))
