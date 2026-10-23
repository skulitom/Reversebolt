"""Check restart recovery and fresh intervention using retained local evidence."""
import hashlib
import json
import statistics
import sys
from pathlib import Path
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/hyperbolica-recovery-v1'
PHASE = OUT / 'recover'
DATA = OUT / 'data'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
PREVIOUS = Path(frozen['previous'])
for name, digest in frozen['sources'].items():
    assert sha(OUT / 'source' / name) == digest
    if '--verify-current' in sys.argv:
        assert sha(ROOT / name) == digest
for name, digest in frozen['binaries'].items():
    assert sha(Path(frozen['bin']) / name) == digest
for name, digest in frozen['game_files'].items():
    assert sha(Path(frozen['game']) / name) == digest
for name, digest in frozen['historical'].items():
    assert sha(PREVIOUS / name) == sha(OUT / 'historical' / name) == digest
for name, digest in frozen['copied_data'].items():
    assert sha(Path(frozen['previous_data']) / name) == digest, 'Original data changed'
    if name.startswith(('runtime-tests/', 'runtime-trials/', 'runtime/', 'captures/')):
        assert sha(DATA / name) == digest, 'Copied historical evidence changed'
assert 'endpoint.json' not in frozen['copied_data']
assert sha(OUT / 'appmanifest-before.acf') == frozen['appmanifest_sha256']
for storage in frozen['storage']:
    for name, digest in storage['hashes'].items():
        assert sha(OUT / storage['backup'] / name) == digest
assert not any((PHASE / (name + '.json')).exists() for name in ['failure', 'needs-recovery', 'cleanup-error'])
terminal = read(PHASE / 'anode-terminal.json')
assert terminal['finished'] and terminal['exitCode'] == 0
initial = read(PHASE / 'initial.json')
old = read(PREVIOUS / 'comparison.json')['test']
assert old['recovery_recipe'] and old['id'] == frozen['source_test_id']
assert old == read(DATA / 'runtime-tests' / f"{old['id']}.json")
assert initial['state']['live'] is None
assert initial['jobs']['history']['boot'] > read(PREVIOUS / 'final-state.json')['jobs']['history']['boot']
assert len(initial['state']['runtime_tests']) == len(initial['state']['runtime_trials']) == 1
discovery = read(PHASE / 'discovery.json')
live = discovery['state']['live']
runtime = live['runtime']
assert live['sample_count'] == 0 and not live['frames'] and runtime['naming'] is None
report = read(PHASE / 'current-report.json')
assert sha(PHASE / 'current-report.json') == sha(Path(runtime['catalog_source']['path'])) == runtime['catalog_source']['document_sha256']
assert report['pid'] != old['scope']['pid'] and report['created'].lower() != old['scope']['created'].lower()
assert report['pid'] == read(PHASE / 'started.json')['game_pid']
assert len(runtime['recovered_controls']) == 1
hint = runtime['recovered_controls'][0]
assert hint['source_test_id'] == old['id'] and hint['needs_revalidation']
recovered = read(PHASE / 'recovery.json')
assert recovered['shown'] and not recovered['edit_authority']
assert recovered['source_test_id'] == old['id']
assert recovered['recovery']['status'] == 'candidate_for_revalidation' and recovered['recovery']['matching_routes'] == 1
candidate = recovered['recovery']['candidate']
assert candidate['id'] == hint['candidate_id'] and candidate['id'] != old['candidate']['id']
assert candidate['locator']['scope']['pid'] == report['pid']
# The product fingerprints compact typed JSON, not the pretty-printed store bytes.
stored = read(DATA / 'runtime-tests' / f"{old['id']}.json")
canonical = lambda value: hashlib.sha256(json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()
assert hint['source_test_sha256'] == recovered['source_test_sha256'] == canonical(stored)
assert hint['recipe_sha256'] == recovered['recovery']['recipe_sha256'] == canonical(stored['recovery_recipe'])
assert any(card['candidate'] == candidate for card in runtime['cards'])
plan = read(PHASE / 'new-plan.json')
template = recovered['preparation']
assert {k: v for k, v in plan.items() if k not in ['test_id', 'change_id']} == template
assert plan['actions'] == old['actions'] and plan['value'] == old['value']
comparison = read(PHASE / 'comparison.json')
test = comparison['test']
assert test['id'] == plan['test_id'] and test['change_id'] == plan['change_id']
assert test == read(DATA / 'runtime-tests' / f"{test['id']}.json")
assert test['candidate'] == candidate and test['scope']['pid'] == report['pid']
assert test['hypothesis'] == old['hypothesis'] and test['actions'] == old['actions'] and test['value'] == old['value']
assert test['recovery_recipe']['report_sha256'] == runtime['catalog_source']['document_sha256']
assert test['recovery_recipe']['source'] == candidate['locator']
assert test['phase'] == 'complete'
result = read(PHASE / 'result.json')
assert result['status'] == test['phase'] and result['assessment'] == test['assessment']
assert result['model_calls'] == 0 and result['restored'] and result['gameplay_write_attempts'] == 1
final = read(PHASE / 'final-state.json')
assert final['jobs']['in_flight'] is None and final['jobs']['fence'] is None and not final['jobs']['history']['unresolved']
assert not final['state']['freezes'] and not any(t['active'] for t in final['state']['runtime_tests'])
assert len(final['state']['runtime_tests']) == len(final['state']['runtime_trials']) == 2
assert all(t['phase'] == 'restored' for t in final['state']['runtime_trials'])
trial = next(t for t in final['state']['runtime_trials'] if t['id'] == test['change_id'])
assert trial == read(DATA / 'runtime-trials' / f"{trial['id']}.json")
assert trial['candidate_id'] == candidate['id'] and trial['patch']['path'] == candidate['locator']

def evidence(item):
    assert sha(Path(item['path'])) == item['report_sha256'] and read(Path(item['path'])) == item['report']

evidence(test['preparation'])
evidence(trial['baseline'])
for phase in ['write', 'restoration']:
    action = trial[phase]
    evidence(action['evidence'])
    assert action['error'] is None and action['outcome'] == 'written'
    assert action['evidence']['report']['patch']['guard'] == 'ready'
    assert action['evidence']['report']['patch']['observed_after'] == action['patch']['replacement']
assert trial['restoration']['patch']['replacement'] == trial['patch']['expected']
assert trial['restoration']['patch']['expected'] == trial['patch']['replacement']
assert test['conditions'][0]['frames'][-1]['completed_unix_ms'] <= trial['write']['started_ms']
assert test['conditions'][1]['runtime_checked_unix_ms'] <= trial['restoration']['started_ms']
gray_medians, holds = [], 0
for condition_index, condition in enumerate(test['conditions']):
    assert len(condition['frames']) == 6
    if condition_index:
        evidence(condition['runtime_evidence'])
        assert condition['runtime_evidence']['report']['checks'][0]['bytes'] == trial['patch']['replacement' if condition_index == 1 else 'expected']
        assert condition['frames'][0]['started_unix_ms'] >= trial['write' if condition_index == 1 else 'restoration']['completed_ms']
        assert condition['runtime_checked_unix_ms'] >= condition['frames'][-1]['completed_unix_ms']
    previous, differences = None, []
    for i, frame in enumerate(condition['frames']):
        image = DATA / 'runtime-tests' / frame['path']
        input_path = DATA / 'runtime-tests' / frame['input_path']
        assert sha(image) == frame['sha256'] and sha(input_path) == frame['input_sha256']
        witness = read(input_path)
        assert witness['omitted_polls'] == 0 and all(p['focus'] == 'selected_foreground' for p in witness['polls'])
        if i:
            assert frame['input']['matches']
        executed = comparison['executions'][condition_index*6 + i]
        prefix = DATA / 'runtime-tests' / test['id'] / f'{condition_index}-{i}'
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
            assert any(p['down'] == action['keys'] for p in witness['polls'])
            assert all(not p['down'] or p['down'] == action['keys'] for p in witness['polls'])
            holds += 1
        rgb = np.asarray(Image.open(image).convert('RGB'), dtype=np.int32)
        gray = ((rgb[:,:,0]*299 + rgb[:,:,1]*587 + rgb[:,:,2]*114 + 500)//1000).astype(np.float64)
        if i >= 2:
            differences.append(float(np.mean(np.abs(gray - previous))))
        previous = gray
    gray_medians.append(statistics.median(differences))
assert holds == 12 and len(comparison['executions']) == 18
medians = [statistics.median(f['difference'] for f in c['frames'][2:]) for c in test['conditions']]
assert medians == test['assessment']['medians']
for i in range(2, 6):
    durations = [c['frames'][i]['input']['held_us'][0] for c in test['conditions']]
    assert max(durations) - min(durations) <= max(50000, max(durations)/4)
helpers, pauses = [], []
for log in (DATA / 'runtime').glob('*-*/**/stdout.jsonl'):
    events = [json.loads(line) for line in log.read_text().splitlines()]
    if events[0]['pid'] != report['pid']:
        continue
    assert events[0]['helper_sha256'] == frozen['binaries']['reversebolt_runtime.dll']
    assert events[-1]['stage'] == 'complete' and events[-1]['helper_absent']
    assert events[-1]['inspection_exit'] == 0 and events[-1]['unload_exit'] == 1
    native = read(log.with_name('report.json'))
    assert native['pid'] == report['pid'] and native['created'] == report['created']
    pauses.extend(native.get('snapshots', native.get('pauses', [])))
    pauses.extend(w['discarded_snapshot'] for w in native.get('warmups', []))
    helpers.append({'path': str(log), 'sha256': sha(log), 'report_sha256': sha(log.with_name('report.json')),
        'elapsed_ms': events[-1]['elapsed_ms']})
assert len(helpers) == 7
assert all(p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0 for p in pauses)
setup = read(OUT / 'setup.json')
assert setup['setup_images_viewed'] <= 4 and len(setup['actions']) <= 6 and setup['measurement_images_manually_viewed'] == 0
cleanup = read(OUT / 'cleanup.json')
assert cleanup['owned_processes_exited'] and cleanup['anode_lease_released'] and cleanup['mcp_exit_code'] == 0
assert cleanup['preserved_editor']['pid'] == frozen['main_editor_pid'] and cleanup['preserved_editor']['responsive']
after = read(OUT / 'storage-after.json')['existing']
changes = []
for i, (before, saved) in enumerate(zip(frozen['storage'], after, strict=True)):
    assert before['path'] == saved['path']
    for name, digest in saved['hashes'].items():
        assert sha(OUT / 'storage-after' / str(i) / name) == digest
    changed = [n for n in sorted(set(before['hashes']) | set(saved['hashes'])) if before['hashes'].get(n) != saved['hashes'].get(n)]
    assert changed == saved['changes']
    changes.append({'path': saved['path'], 'changes': changed})
summary = {'artifact_audit': 'passed', 'audit_source_sha256': sha(Path(__file__)),
    'frozen_sha256': sha(OUT / 'frozen.json'), 'preview': Path(frozen['bin']).name,
    'source_test_id': old['id'], 'source_candidate_id': old['candidate']['id'],
    'new_test_id': test['id'], 'new_candidate_id': candidate['id'],
    'source_process': old['scope'], 'new_process': test['scope'],
    'source_boot': read(PREVIOUS / 'final-state.json')['jobs']['history']['boot'],
    'new_boot': initial['jobs']['history']['boot'], 'automatic_recovery': True, 'mcp_recovery': True,
    'matching_routes': recovered['recovery']['matching_routes'], 'new_recipe_retained': True,
    'original_data_preserved': True, 'historical_files_copied': len(frozen['copied_data']),
    'model_calls': 0, 'new_discovery_images': 0, 'comparison_images': 18, 'comparison_key_holds': holds,
    'assessment': test['assessment'], 'independent_gray_medians': gray_medians,
    'original_bytes': trial['patch']['expected'], 'replacement_bytes': trial['patch']['replacement'],
    'restored_bytes': trial['restoration']['patch']['replacement'], 'helpers': helpers,
    'recorded_pauses': len(pauses), 'max_pause_us': max(p['stop_to_resume_us'] for p in pauses),
    'setup': setup, 'cleanup': cleanup, 'storage_changes': changes,
    'save_unchanged': frozen['storage'][0]['hashes']['game.save'] == after[0]['hashes']['game.save'],
    'progress_percent': 40,
    'limitation': 'Known development target and one action-specific visual effect after one restart. Exact semantics, persistent actor identity, scene/load recovery, downstream effects and withheld-game generalization remain unproven.'}
Path(__file__).with_name('results.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: summary[k] for k in ['artifact_audit', 'automatic_recovery', 'mcp_recovery', 'assessment', 'independent_gray_medians', 'save_unchanged', 'recorded_pauses', 'max_pause_us']}))
