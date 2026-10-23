"""Audit a retained failed independent run without launching a game or model."""
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/independent-vampire-survivors-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
for name, digest in frozen['sources'].items():
    assert sha(OUT / 'source' / name) == digest, name
    if '--verify-current' in sys.argv:
        assert sha(ROOT / name) == digest, f'Working source changed: {name}'
for name, digest in frozen['binaries'].items():
    assert sha(Path(frozen['bin']) / name) == digest, name
for name, digest in frozen['game_files'].items():
    assert sha(Path(frozen['game']) / name) == digest, name
assert sha(OUT / 'appmanifest-before.acf') == frozen['appmanifest_sha256']
for storage in frozen['storage']:
    for name, digest in storage['hashes'].items():
        assert sha(OUT / storage['backup'] / name) == digest

assert not (OUT / 'failure.json').exists() and not (OUT / 'needs-recovery.json').exists()
terminal = read(OUT / 'anode-terminal.json')
assert terminal['finished'] and terminal['exitCode'] == 0
result = read(OUT / 'result.json')
state = read(OUT / 'discovery.json')['state']
live = state['live']
runtime = live['runtime']
assert result['workflow_finished'] and result['jobs'] == [{'phase': 'Failed', 'error': None}]
assert result['samples'] == len(live['frames']) == 15
assert runtime['phase'] == 'failed' and runtime['notice'] == 'field metadata budget'
assert runtime['catalog_source'] is None and not runtime['catalog_id'] and not runtime['cards']
assert runtime['naming'] is result['review'] is None
assert result['comparison'] is None and not result['plan_available']
assert result['manually_reviewed_measurement_images'] == 0
assert [i['key'] for i in result['evaluator_inputs']] == [68, 65, 68, 65]
for index, hold in enumerate(result['evaluator_inputs'], 1):
    assert read(OUT / f'input-{index}.json') == hold
    assert hold['planned_ms'] == 200 and hold['end_ns'] > hold['begin_ns']

data = Path(frozen['data'])
captures = [p for p in (data / 'captures').iterdir() if (p / 'evidence.json').is_file()]
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
        if poll['begin_us'] in polls:
            assert polls[poll['begin_us']] == poll
        polls[poll['begin_us']] = poll
runs, previous = [], []
for _, poll in sorted(polls.items()):
    if poll['down'] != previous:
        if poll['down']:
            runs.append(poll['down'])
        previous = poll['down']
assert runs == [[68], [65], [68], [65]] and not previous

logs = list((data / 'runtime').glob('*-*/**/stdout.jsonl'))
assert len(logs) == 1
events = [json.loads(line) for line in logs[0].read_text().splitlines()]
assert events[0]['helper_sha256'] == frozen['binaries']['reversebolt_runtime.dll']
assert events[0]['pid'] == live['process']['pid'] and events[0]['session'] == 3
assert events[-1]['stage'] == 'complete' and events[-1]['helper_absent']
assert events[-1]['inspection_exit'] == 0 and events[-1]['unload_exit'] == 1
report_path = logs[0].with_name('report.json')
assert read(report_path) == {'error': 'field metadata budget'}
assert events[-1]['elapsed_ms'] < 20000

models = []
for path in (data / 'captures/model-observations').glob('*.json'):
    model = read(path)
    assert model['investigation_id'] == live['investigation_id']
    assert model['sequence']['frames'] == live['frames'][:2]
    assert not model['pass']['cancelled'] and model['pass']['error'] is None
    receipts = [model['pass']['proposal'], *[r['receipt'] for r in model['pass']['reviews']]]
    for receipt in receipts:
        assert receipt['model'] == frozen['model']['name']
        assert receipt['model_digest'] == frozen['model']['digest']
        assert hashlib.sha256(receipt['prompt'].encode()).hexdigest() == receipt['prompt_sha256']
        assert receipt['response']['done'] and receipt['response']['model'] == receipt['model']
    assert [f['sha256'] for f in receipts[0]['frames']] == [f['sha256'] for f in live['frames'][:2]]
    models.append({'receipt_sha256': sha(path), 'calls': len(receipts),
                   'summary': receipts[0]['interpretation']['summary']})
assert len(models) == 1 and not (data / 'runtime/naming').exists()

final = read(OUT / 'final-state.json')
assert not final['state']['runtime_trials'] and not final['state']['runtime_tests']
assert not final['state']['freezes']
assert final['jobs']['fence'] is None and final['jobs']['in_flight'] is None
assert not final['jobs']['history']['unresolved']
assert read(OUT / 'child.json')['exit_code'] == 0
setup = read(OUT / 'setup.json')
assert setup['setup_images_viewed'] <= frozen['maximum_setup_images']
assert len(setup['actions']) <= frozen['maximum_setup_actions'] and setup['gameplay_confirmed']
assert setup['manually_reviewed_measurement_images'] == 0
cleanup = read(OUT / 'cleanup.json')
assert cleanup['owned_processes_exited'] and cleanup['anode_lease_released']
assert cleanup['preserved_editor']['pid'] == frozen['main_editor_pid']
assert cleanup['preserved_editor']['session'] == 1 and cleanup['preserved_editor']['responsive']
after = read(OUT / 'storage-after.json')
storage_changes = []
for index, (before, saved) in enumerate(zip(frozen['storage'], after['existing'], strict=True)):
    assert before['path'] == saved['path']
    for name, digest in saved['hashes'].items():
        assert sha(OUT / 'storage-after' / str(index) / name) == digest
    changes = [n for n in sorted(set(before['hashes']) | set(saved['hashes']))
               if before['hashes'].get(n) != saved['hashes'].get(n)]
    assert changes == saved['changes']
    storage_changes.append({'path': saved['path'], 'changes': changes})
for name, digest in after['new_storage']['hashes'].items():
    assert sha(OUT / 'new-storage-after' / name) == digest

summary = {'artifact_audit': 'passed', 'native_discovery': 'failed_metadata_budget',
           'source_commit': frozen['source_commit'], 'release': Path(frozen['bin']).name,
           'game_app_id': frozen['appid'], 'product_and_evaluator_unchanged': True,
           'setup': setup, 'automatic_images': 15, 'input_holds': 4,
           'sampled_input_episodes': runs, 'metadata_naming_calls': 0,
           'image_model_evidence': models, 'named_controls': 0, 'comparisons': 0, 'gameplay_edits': 0,
           'helper_elapsed_ms': events[-1]['elapsed_ms'], 'helper_unloaded': True,
           'runtime_report_sha256': sha(report_path), 'runtime_events_sha256': sha(logs[0]),
           'metadata_limit_diagnosis': 'The source rejects either 200000 fields or 20 seconds; the whole helper finished in 8367 ms, implicating the field-count cap. No field count or pause telemetry was retained in its error-only report.',
           'storage_changes': storage_changes, 'new_storage_files': sorted(after['new_storage']['hashes']),
           'cleanup': cleanup, 'frozen_manifest_sha256': sha(OUT / 'frozen.json'),
           'limitation': 'No runtime catalogue or tested control. The model reports a level-up menu in the first two observation frames; its interpretation is unverified and movement effects cannot be assumed. Artifact integrity and orderly cleanup do not establish generalization.'}
(ROOT / 'research/independent-vampire-survivors/results.json').write_text(
    json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: summary[k] for k in ['artifact_audit', 'native_discovery', 'automatic_images',
                                       'named_controls', 'comparisons', 'gameplay_edits', 'helper_unloaded']}))
