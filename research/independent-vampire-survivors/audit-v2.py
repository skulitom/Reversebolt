"""Audit the retained root-first native rerun without launching a game or model."""
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/independent-vampire-survivors-v2'
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
assert result['workflow_finished'] and result['jobs'] == [{'phase': 'Completed', 'error': None}]
assert result['samples'] == len(live['frames']) == 15
assert runtime['phase'] == 'ready' and runtime['catalog_source'] and runtime['catalog_id']
review = runtime['naming']
assert review == result['review'] and review['status'] == 'unavailable'
assert review['batch'] is None and review['error'] == 'Type selection exhausted its review-round budget.'
assert len(runtime['cards']) == 64
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
report = read(report_path)
assert sha(report_path) == runtime['catalog_source']['document_sha256'] == review['report_sha256']
assert report['pid'] == live['process']['pid'] and report['session'] == 3 and not report['edit_authority']
metadata = report['metadata']
assert metadata['policy'] == 'roots_then_encountered_layouts_v1' and metadata['root_inventory']['complete']
assert metadata['loaded_fields'] == report['catalog_fields'] == 44050
assert metadata['loaded_classes'] == report['catalog_classes'] == 6073
assert metadata['static_roots'] == report['catalog_roots'] == 9935
assert report['next_root'] == 2304 and report['root_start'] == 0
assert not report['root_scan_complete'] and not report['graph_complete'] and report['stop_reason'] == 'output_budget'
shots = report['snapshots']
assert [s['root_start'] for s in shots] == list(range(0, 2304, 64))
assert all(s['root_end'] == s['root_start'] + 64 and len(s['roots']) == 64 for s in shots)
pauses = shots + [w['discarded_snapshot'] for w in report['warmups']]
assert len(pauses) == 113
assert all(p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0 for p in pauses)
receipt_path = data / 'runtime/naming' / review['evidence_id'] / 'receipt.json'
receipt = read(receipt_path)
inventory = read(receipt_path.with_name('inventory.json'))
assert receipt['report_sha256'] == inventory['report_sha256'] == review['report_sha256']
assert inventory['scope']['pid'] == report['pid'] and inventory['scope']['created'] == report['created']
assert receipt['model_digest'] == frozen['model']['digest']
assert receipt['error'] == review['error'] and not receipt['cancelled']
assert receipt['selection'] is receipt['batch'] is receipt['activity'] is None
assert read(receipt_path.with_name('card-publication.json')) is None
play = read(receipt_path.with_name('play-source.json'))
assert play['frames'] == live['frames']
observed = {t['id']: t for t in inventory['types']}
expected_ids = sorted(observed)
rounds = []
used_stages = []
for round_ in receipt['paged_selection']['rounds']:
    ids = [i for page in round_['pages'] for i in page['type_ids']]
    assert ids == expected_ids
    nominated = []
    for page in round_['pages']:
        stage = receipt['stages'][page['stage_index']]
        used_stages.append(page['stage_index'])
        assert hashlib.sha256(stage['prompt'].encode()).hexdigest() == stage['prompt_sha256']
        assert len(stage['prompt'].encode()) <= 12000
        assert stage['response']['done'] and stage['response']['model'] == frozen['model']['name']
        selected = json.loads(stage['response']['message']['content'])['types']
        assert selected == page['selected'] and len(selected) <= 8 and len(selected) == len(set(selected))
        assert set(selected) <= set(page['type_ids'])
        payload = json.loads(stage['prompt'].split('Observed types and path-scoped text examples: ', 1)[1])
        assert [t['id'] for t in payload['types']] == page['type_ids']
        for t in payload['types']:
            source = observed[t['id']]
            assert t['field_labels'] == [f['name'] for f in source['fields']]
            assert t['name'] == source['key']['type_name'][:120]
            for sample in t.get('object_samples', []):
                origins = [p for f in source['fields'] if f['name'] == sample['field_name']
                           for p in f['samples'] if p['path_id'] == sample['path_id']]
                assert origins
                assert any(all(text in [h['text'] for h in origin['context'] if not h['truncated']]
                               for text in sample['object_text']) for origin in origins)
        nominated.extend(selected)
    expected_ids = sorted(set(nominated))
    rounds.append({'input_types': len(ids), 'pages': len(round_['pages']), 'selected_types': len(expected_ids)})
assert used_stages == list(range(20))
assert [r['input_types'] for r in rounds] == [364, 30, 18, 12]
assert [r['selected_types'] for r in rounds] == [30, 18, 12, 10]
assert len(expected_ids) > 8

models = []
for path in (data / 'captures/model-observations').glob('*.json'):
    model = read(path)
    assert model['investigation_id'] == live['investigation_id']
    assert model['sequence']['frames'] == live['frames'][:2]
    assert not model['pass']['cancelled'] and model['pass']['error'] is None
    receipts = [model['pass']['proposal'], *[r['receipt'] for r in model['pass']['reviews']]]
    for image_receipt in receipts:
        assert image_receipt['model'] == frozen['model']['name']
        assert image_receipt['model_digest'] == frozen['model']['digest']
        assert hashlib.sha256(image_receipt['prompt'].encode()).hexdigest() == image_receipt['prompt_sha256']
        assert image_receipt['response']['done'] and image_receipt['response']['model'] == image_receipt['model']
    assert [f['sha256'] for f in receipts[0]['frames']] == [f['sha256'] for f in live['frames'][:2]]
    models.append({'receipt_sha256': sha(path), 'calls': len(receipts),
                   'summary': receipts[0]['interpretation']['summary']})
assert len(models) == 1

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
summary = {'artifact_audit': 'passed', 'native_discovery': 'partial_catalogue',
           'naming': 'review_round_budget_exhausted', 'source_commit': frozen['source_commit'],
           'release': Path(frozen['bin']).name, 'game_app_id': frozen['appid'],
           'setup': setup, 'automatic_images': 15, 'input_holds': 4, 'sampled_input_episodes': runs,
           'metadata': metadata, 'report_bytes': report_path.stat().st_size,
           'catalog_ms': report['catalog_ms'], 'helper_elapsed_ms': events[-1]['elapsed_ms'],
           'roots_visited': report['next_root'], 'root_inventory_size': report['catalog_roots'],
           'root_scan_complete': False, 'graph_complete': False, 'stop_reason': report['stop_reason'],
           'recorded_pauses': len(pauses), 'max_stop_to_resume_us': max(p['stop_to_resume_us'] for p in pauses),
           'helper_unloaded': True, 'inventory_types': len(inventory['types']),
           'inventory_fields': sum(len(t['fields']) for t in inventory['types']),
           'reachable_paths': inventory['reachable_paths'], 'metadata_model_calls': len(receipt['stages']),
           'selection_rounds': rounds, 'image_model_evidence': models,
           'raw_runtime_cards': len(runtime['cards']), 'accepted_gameplay_hypotheses': 0,
           'validated_controls': 0, 'comparisons': 0, 'gameplay_edits': 0,
           'runtime_report_sha256': sha(report_path), 'runtime_events_sha256': sha(logs[0]),
           'naming_receipt_sha256': sha(receipt_path), 'storage_changes': storage_changes,
           'cleanup': cleanup, 'frozen_manifest_sha256': sha(OUT / 'frozen.json'),
           'limitation': 'Known development target after a reader change. Partial catalogue only; the complete-type shortlist failed to reach eight candidates, so no hypotheses or gameplay controls were tested. The local image model again reports a level-up menu; no movement or causal effect is established.'}
(ROOT / 'research/independent-vampire-survivors/results-v2.json').write_text(
    json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: summary[k] for k in ['artifact_audit', 'native_discovery', 'naming', 'inventory_types',
                                       'inventory_fields', 'accepted_gameplay_hypotheses', 'comparisons', 'gameplay_edits']}))
