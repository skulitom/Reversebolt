"""Audit a native product run using retained data, without viewing its images."""
import hashlib
import json
import statistics
import struct
import sys
from pathlib import Path
from jsonschema import Draft202012Validator as Validator
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/bounded-exploration-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
DATA = Path(frozen['data'])
for name, digest in frozen['sources'].items():
    assert sha(OUT / 'source' / name) == digest
    if '--verify-current' in sys.argv:
        assert sha(ROOT / name) == digest
for name, digest in frozen['binaries'].items():
    assert sha(Path(frozen['bin']) / name) == digest
for name, digest in frozen['game_files'].items():
    assert sha(Path(frozen['game']) / name) == digest
assert sha(OUT / 'appmanifest-before.acf') == frozen['appmanifest_sha256']
for storage in frozen['storage']:
    for name, digest in storage['hashes'].items():
        assert sha(OUT / storage['backup'] / name) == digest
assert not (OUT / 'failure.json').exists() and not (OUT / 'needs-recovery.json').exists()
terminal = read(OUT / 'anode-terminal.json')
assert terminal['finished'] and terminal['exitCode'] == 0
result = read(OUT / 'result.json')
live = read(OUT / 'discovery.json')['state']['live']
runtime = live['runtime']
assert result['workflow_finished'] and result['samples'] == len(live['frames']) == 15
assert result['manually_reviewed_measurement_images'] == 0
assert [i['key'] for i in result['evaluator_inputs']] == [68, 65, 68, 65]
for i, hold in enumerate(result['evaluator_inputs'], 1):
    assert read(OUT / f'input-{i}.json') == hold
    assert hold['planned_ms'] == 200 and hold['end_ns'] > hold['begin_ns']
captures = [p for p in (DATA / 'captures').iterdir() if (p / 'evidence.json').is_file()]
assert len(captures) == 1
assert read(captures[0] / 'evidence.json')['frames'] == live['frames']
polls = {}
for frame in live['frames']:
    assert sha(captures[0] / f"frame-{frame['number']:03}.png") == frame['sha256']
    record = read(captures[0] / f"frame-{frame['number']:03}.json")
    assert record['number'] == frame['number'] and record['screen_error'] is None
    assert record['screen']['sampling'] == frame['sampling']
    context = frame['input_context']
    assert context['omitted_polls'] == 0 and context['clock_id'] == live['investigation_id']
    for poll in context['polls']:
        assert poll['focus'] == 'selected_foreground'
        assert polls.get(poll['begin_us'], poll) == poll
        polls[poll['begin_us']] = poll
runs, previous = [], []
for _, poll in sorted(polls.items()):
    if poll['down'] != previous:
        if poll['down']:
            runs.append(poll['down'])
        previous = poll['down']
assert runs == [[68], [65], [68], [65]] and not previous

review = runtime.get('naming')
assert review == result['review']
naming = None
if review:
    path = DATA / 'runtime/naming' / review['evidence_id'] / 'receipt.json'
    receipt = read(path)
    inventory = read(path.with_name('inventory.json'))
    assert receipt['report_sha256'] == inventory['report_sha256'] == review['report_sha256']
    assert receipt['model_digest'] == frozen['model']['digest'] and not receipt['cancelled']
    assert receipt['batch'] == review['batch'] and receipt['error'] == review['error']
    assert sha(Path(runtime['catalog_source']['path'])) == runtime['catalog_source']['document_sha256'] == receipt['report_sha256']
    table = receipt['citation_evidence']
    assert table['report_sha256'] == receipt['report_sha256']
    entries = []
    for row in receipt['selection']['rows']:
        for source, index, quote in [('field_name', None, row['field_name']), ('type_name', None, row['type_name'])] + [
                ('object_text', i, text) for i, text in enumerate(row['object_text'])]:
            entries.append({'id': len(entries), 'candidate': row['id'], 'citation': {'source': source, 'index': index, 'quote': quote}})
    assert table['entries'] == entries
    stages = list(receipt['stages'])
    if receipt['activity'] and receipt['activity']['stage']:
        stages.append(receipt['activity']['stage'])
    for stage in stages:
        assert hashlib.sha256(stage['prompt'].encode()).hexdigest() == stage['prompt_sha256']
        response = stage['response']
        assert response['done'] and response['done_reason'] == 'stop'
        assert response['model'] == frozen['model']['name']
        assert response['prompt_eval_count'] <= stage['options']['num_ctx'] - stage['options']['num_predict'] - 512
        parsed = json.loads(response['message']['content'])
        Validator(stage['schema']).validate(parsed)
        for proposal in parsed.get('hypotheses', []):
            assert all(entries[i]['candidate'] == proposal['candidate'] for i in proposal['evidence_ids'])
    if receipt['batch']:
        for hypothesis in receipt['batch']['accepted']:
            candidate = hypothesis['candidate']
            assert candidate in receipt['selection']['rows']
            for citation in hypothesis['proposal']['citations']:
                observed = candidate[citation['source']]
                if citation['source'] == 'object_text':
                    observed = observed[citation['index']]
                else:
                    assert citation['index'] is None
                assert observed == citation['quote']
    activity = receipt['activity']
    naming = {'status': review['status'], 'error': review['error'], 'receipt_sha256': sha(path),
        'metadata_model_calls': len(receipt['stages']),
        'metadata_call_elapsed_ms': [stage['elapsed_ms'] for stage in receipt['stages']],
        'activity': None if activity is None else {'error': activity['error'], 'observation': activity['observation'],
            'model_calls': int(activity['stage'] is not None)},
        'types': len(inventory['types']), 'fields': sum(len(t['fields']) for t in inventory['types']),
        'reachable_paths': inventory['reachable_paths'],
        'questions': [] if receipt['batch'] is None else [{'name': h['proposal']['name'],
            'question': h['proposal']['question'], 'field': h['candidate']['field_name'],
            'observed_value': h['candidate']['value'], 'path_id': h['candidate']['path_id'],
            'alternatives': [r['path_id'] for r in h['alternatives']]} for h in receipt['batch']['accepted']],
        'rejected': [] if receipt['batch'] is None else receipt['batch']['rejected']}

image_models = []
for path in (DATA / 'captures/model-observations').glob('*.json'):
    model = read(path)
    assert model['investigation_id'] == live['investigation_id']
    assert model['sequence']['frames'] == live['frames'][:2]
    assert not model['pass']['cancelled']
    receipts = [model['pass']['proposal'], *[r['receipt'] for r in model['pass']['reviews']]]
    for image_receipt in receipts:
        assert image_receipt['model'] == frozen['model']['name']
        assert image_receipt['model_digest'] == frozen['model']['digest']
        assert hashlib.sha256(image_receipt['prompt'].encode()).hexdigest() == image_receipt['prompt_sha256']
        assert image_receipt['response']['done'] and image_receipt['response']['model'] == image_receipt['model']
    assert [f['sha256'] for f in receipts[0]['frames']] == [f['sha256'] for f in live['frames'][:2]]
    image_models.append({'receipt_sha256': sha(path), 'calls': len(receipts), 'error': model['pass']['error'], 'summary': receipts[0]['interpretation']['summary']})

helpers, pauses = [], []
for log in (DATA / 'runtime').glob('*-*/**/stdout.jsonl'):
    events = [json.loads(line) for line in log.read_text().splitlines()]
    assert events[0]['helper_sha256'] == frozen['binaries']['reversebolt_runtime.dll']
    assert events[0]['pid'] == live['process']['pid'] and events[0]['session'] == 3
    assert events[-1]['stage'] == 'complete' and events[-1]['helper_absent']
    assert events[-1]['inspection_exit'] == 0 and events[-1]['unload_exit'] == 1
    report = read(log.with_name('report.json'))
    assert report['pid'] == live['process']['pid']
    shots = report.get('snapshots', report.get('pauses', []))
    shots += [w['discarded_snapshot'] for w in report.get('warmups', [])]
    pauses.extend(shots)
    helpers.append({'path': str(log), 'events_sha256': sha(log), 'report_sha256': sha(log.with_name('report.json')),
        'elapsed_ms': events[-1]['elapsed_ms'], 'error': report.get('error'),
        'metadata': report.get('metadata'), 'graph_complete': report.get('graph_complete'),
        'roots_complete': report.get('root_scan_complete')})
assert helpers and all(p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0 for p in pauses)
final = read(OUT / 'final-state.json')
assert final['jobs']['fence'] is None and final['jobs']['in_flight'] is None
assert not final['jobs']['history']['unresolved'] and not final['state']['freezes']
assert not any(t['active'] for t in final['state']['runtime_tests'])
assert not any(t['phase'] in ('written', 'uncertain') for t in final['state']['runtime_trials'])
assert read(OUT / 'child.json')['exit_code'] == 0
comparison = None
comparison_summaries = []
trial_summaries = []
assert result['comparison_limit'] == frozen['maximum_comparisons'] == 3
assert len(result['comparisons']) <= 3
for comparison_index, recorded_test in enumerate(result['comparisons'], 1):
    comparison = read(OUT / f'comparison-{comparison_index}.json')
    test = comparison['test']
    assert recorded_test == test == read(DATA / 'runtime-tests' / f"{test['id']}.json")
    plan = read(OUT / f'chosen-plan-{comparison_index}.json')
    assert plan['args']['test_id'] == test['id'] and plan['args']['change_id'] == test['change_id']
    assert plan['plan']['next']['value'] == test['value'] and plan['plan']['next']['actions'] == test['actions']
    assert plan['args']['hypothesis_ref'] == plan['plan']['next']['hypothesis_ref']
    assert plan['args']['hypothesis_ref']['selected_path_id'] == test['candidate']['id']
    raw_medians, holds, execution_index = [], 0, 0
    complete = test['phase'] == 'complete'
    assert test['phase'] in ('complete', 'stopped')
    for condition_index, condition in enumerate(test['conditions']):
        previous = None
        differences = []
        for i, frame in enumerate(condition['frames']):
            assert sha(DATA / 'runtime-tests' / frame['path']) == frame['sha256']
            assert sha(DATA / 'runtime-tests' / frame['input_path']) == frame['input_sha256']
            witness = read(DATA / 'runtime-tests' / frame['input_path'])
            assert witness['omitted_polls'] == 0
            assert all(p['focus'] == 'selected_foreground' for p in witness['polls'])
            if i and complete:
                assert frame['input']['matches']
            executed = comparison['executions'][execution_index]
            execution_index += 1
            prefix = DATA / 'runtime-tests' / test['id'] / f'{condition_index}-{i}'
            assert sha(Path(str(prefix) + '.action.json')) == executed['plan_sha256']
            assert read(Path(str(prefix) + '.action.json')) == executed['plan']
            assert sha(Path(str(prefix) + '.action.outcome.json')) == executed['outcome']['sha256']
            assert read(Path(str(prefix) + '.action.outcome.json')) == executed['outcome']['receipt']
            action = None if i < 2 else test['actions'][i - 2]
            assert executed['plan']['action'] == action
            outcome = executed['outcome']['receipt']
            assert outcome['plan_sha256'] == executed['plan_sha256']
            if complete:
                assert outcome['error'] is None
            assert outcome['keydown_accepted'] == outcome['keyup_accepted'] == (len(action['keys']) if action else 0)
            if action:
                assert any(p['down'] == action['keys'] for p in witness['polls'])
                assert all(not p['down'] or p['down'] == action['keys'] for p in witness['polls'])
                holds += 1
            pixels = np.asarray(Image.open(DATA / 'runtime-tests' / frame['path']).convert('RGB'), dtype=np.int32)
            gray = ((pixels[:, :, 0]*299 + pixels[:, :, 1]*587 + pixels[:, :, 2]*114 + 500)//1000).astype(np.float64)
            if i >= 2:
                differences.append(float(np.mean(np.abs(gray - previous))))
            previous = gray
        raw_medians.append(statistics.median(differences) if differences else None)
    assert len(comparison['executions']) >= execution_index
    if complete:
        assert len(comparison['executions']) == 18 and holds == 12
        assert [len(c['frames']) for c in test['conditions']] == [6, 6, 6]
        medians = [statistics.median(f['difference'] for f in c['frames'][2:]) for c in test['conditions']]
        assert medians == test['assessment']['medians']
    for i in range(2, 6) if complete else []:
        durations = [c['frames'][i]['input']['held_us'][0] for c in test['conditions']]
        assert max(durations) - min(durations) <= max(50000, max(durations)/4)
    for trial in final['state']['runtime_trials']:
        if trial['id'] != test['change_id']:continue
        assert trial == read(DATA / 'runtime-trials' / f"{trial['id']}.json")
        if trial.get('write') and trial['write']['outcome'] == 'written':
            assert trial['phase'] == 'restored'
            for key in ['write', 'restoration']:
                action = trial[key]
                assert action['error'] is None and action['outcome'] == 'written'
                evidence = action['evidence']
                assert sha(Path(evidence['path'])) == evidence['report_sha256']
                assert read(Path(evidence['path'])) == evidence['report']
                assert evidence['report']['patch']['observed_after'] == action['patch']['replacement']
            assert trial['restoration']['patch']['replacement'] == trial['patch']['expected']
            assert trial['restoration']['patch']['expected'] == trial['patch']['replacement']
            assert test['conditions'][0]['frames'][-1]['completed_unix_ms'] <= trial['write']['started_ms']
            for index, phase in [(1, 'write'), (2, 'restoration')]:
                condition = test['conditions'][index]
                evidence = condition['runtime_evidence']
                assert sha(Path(evidence['path'])) == evidence['report_sha256']
                assert read(Path(evidence['path'])) == evidence['report']
                assert evidence['report']['checks'][0]['bytes'] == trial['patch']['replacement' if index == 1 else 'expected']
                assert condition['frames'][0]['started_unix_ms'] >= trial[phase]['completed_ms']
                assert condition['runtime_checked_unix_ms'] >= condition['frames'][-1]['completed_unix_ms']
            assert test['conditions'][1]['runtime_checked_unix_ms'] <= trial['restoration']['started_ms']
            trial_summaries.append({'id': trial['id'], 'phase': trial['phase'], 'candidate_id': trial['candidate_id'],
                'original_bytes': trial['patch']['expected'], 'replacement_bytes': trial['patch']['replacement'],
                'restored_bytes': trial['restoration']['patch']['replacement'], 'type_code': trial['patch']['type_code'],
                'record_sha256': sha(DATA / 'runtime-trials' / f"{trial['id']}.json")})
            if trial['patch']['type_code'] == 12:
                trial_summaries[-1]['original_f32'] = struct.unpack('<f', bytes(trial['patch']['expected']))[0]
                trial_summaries[-1]['replacement_f32'] = struct.unpack('<f', bytes(trial['patch']['replacement']))[0]
    comparison_summary = {'test_id': test['id'], 'candidate_id': test['candidate']['id'],
        'phase': test['phase'], 'hypothesis': test['hypothesis'], 'value': test['value'],
        'assessment': test['assessment'], 'independent_full_frame_gray_medians': raw_medians,
        'actions': test['actions'], 'measurement_images': execution_index, 'input_holds': holds,
        'comparison_sha256': sha(OUT / f'comparison-{comparison_index}.json'), 'remembered_recipe_retained': test.get('recovery_recipe') is not None}

    comparison_summaries.append(comparison_summary)
    before = plan['plan']['exploration']
    after_plan = read(OUT / f'after-comparison-{comparison_index}.json')
    after = after_plan['exploration']
    assert before['policy'] == after['policy'] == 'bounded_response_exploration_v1'
    assert before['comparison_limit'] == after['comparison_limit'] == 3
    assert before['status'] == 'continue'
    assert set(before['test_ids']) == {t['id'] for t in result['comparisons'][:comparison_index-1]}
    assert set(after['test_ids']) == {t['id'] for t in result['comparisons'][:comparison_index]}
    if comparison_index > 1:
        previous = result['comparisons'][comparison_index-2]
        assert previous['assessment']['status'] == 'suppression_not_observed'
        assert test['created_unix_ms'] > previous['conditions'][2]['frames'][-1]['completed_unix_ms']
        assert test['candidate']['id'] not in [t['candidate']['id'] for t in result['comparisons'][:comparison_index-1]]
    expected = ('needs_review' if test['phase'] != 'complete' or test['assessment']['status'] == 'inconclusive' else
        'response_found' if test['assessment']['status'] == 'suppression_observed' else
        'budget_exhausted' if comparison_index == 3 else 'continue')
    assert after['status'] == expected
    if after['status'] != 'continue':assert after_plan['next'] is None
    if comparison_index == len(result['comparisons']):assert result['stop_reason'] == after['status']

assert len({t['id'] for t in result['comparisons']}) == len(result['comparisons'])
assert {t['id'] for t in trial_summaries} == {t['id'] for t in final['state']['runtime_trials'] if t['write'] and t['write']['outcome'] == 'written'}

setup = read(OUT / 'setup.json')
assert setup['setup_images_viewed'] <= frozen['maximum_setup_images'] and len(setup['actions']) <= frozen['maximum_setup_actions']
assert setup['measurement_images_manually_viewed'] == 0
cleanup = read(OUT / 'cleanup.json')
assert cleanup['owned_processes_exited'] and cleanup['anode_lease_released']
assert cleanup['preserved_editor']['pid'] == frozen['main_editor_pid']
assert cleanup['preserved_editor']['session'] == 1 and cleanup['preserved_editor']['responsive']
after = read(OUT / 'storage-after.json')
changes = []
for i, (before, saved) in enumerate(zip(frozen['storage'], after['existing'], strict=True)):
    assert before['path'] == saved['path']
    for name, digest in saved['hashes'].items():
        assert sha(OUT / 'storage-after' / str(i) / name) == digest
    changed = [n for n in sorted(set(before['hashes']) | set(saved['hashes'])) if before['hashes'].get(n) != saved['hashes'].get(n)]
    assert changed == saved['changes']
    changes.append({'path': saved['path'], 'changes': changed})
planner_receipts = [read(p) for p in sorted(OUT.glob('rpc-*-plan_runtime_comparisons.json'))]
planner_errors = [r['response']['error'] for r in planner_receipts if not r['response']['ok']]
assert all(r['args']['comparison_limit'] == 3 for r in planner_receipts)
if result['stop_reason'] == 'plan_unavailable':
    assert planner_errors and not result['comparisons'] and not final['state']['runtime_trials']
play_sources = list((DATA / 'runtime/naming').glob('*/play-source.json'))
input_coverage = None
if play_sources:
    assert len(play_sources) == 1
    packet = read(play_sources[0])
    ev = packet['evidence']['evidence']
    input_coverage = {'source_sha256': sha(play_sources[0]), 'actions': ev['actions'],
        'episodes': ev['episodes']}
summary = {'artifact_audit': 'passed', 'preview': Path(frozen['bin']).name, 'setup': setup,
    'audit_source_sha256': sha(Path(__file__)), 'progress_percent': 40,
    'frozen_sha256': sha(OUT / 'frozen.json'), 'automatic_discovery_images': 15, 'input_holds': 4,
    'manually_viewed_measurement_images': 0, 'jobs': result['jobs'], 'naming': naming, 'image_models': image_models,
    'plan_available': result['plan_available'], 'comparisons': comparison_summaries, 'stop_reason': result['stop_reason'], 'planner_errors': planner_errors, 'input_coverage': input_coverage,
    'helpers': helpers, 'recorded_pauses': len(pauses),
    'max_stop_to_resume_us': max((p['stop_to_resume_us'] for p in pauses), default=None),
    'runtime_trials': trial_summaries, 'storage_changes': changes, 'cleanup': cleanup,
    'limitation': 'Known development target following the retained negative first trial; not a withheld evaluation. Labels and image differences alone do not verify gameplay meaning; a completed audit may retain a negative or unavailable comparison.'}
Path(__file__).with_name('results.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'artifact_audit': 'passed', 'naming': naming and naming['status'],
    'plan_available': result['plan_available'], 'comparisons': len(result['comparisons']), 'stop_reason': result['stop_reason'],
    'trials': len(final['state']['runtime_trials']), 'helpers': len(helpers), 'storage_changes': changes}))
