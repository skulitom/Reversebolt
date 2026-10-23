"""Audit the stopped native run without accessing a process or replaying input."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-recovery-live-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
for directory, records in [('source', frozen['sources']), ('bin', frozen['binaries']),
                           ('storage-before', frozen['storage_before']), ('', frozen['historical'])]:
    for name, digest in records.items():
        assert sha(OUT / directory / name) == digest, name
for name in ['failure', 'needs-recovery', 'cleanup-error', 'new-plan', 'comparison']:
    assert not (OUT / f'{name}.json').exists(), name
assert read(OUT / 'anode-terminal.json')['exitCode'] == 0
assert read(OUT / 'anode-terminal.json')['finished']
assert read(OUT / 'child.json')['exit_code'] == 0
assert read(OUT / 'recovery-terminal.json')['exit_code'] == 0
result = read(OUT / 'result.json')
assert result == dict(finished=True, status='catalog_changed', gameplay_writes=0, model_calls=0)
final = read(OUT / 'final-state.json')
assert final == read(OUT / 'discovery.json')
assert final['jobs']['in_flight'] is None and final['jobs']['fence'] is None
assert not final['jobs']['history']['unresolved']
jobs = list(final['jobs']['history']['jobs'].values())
assert len(jobs) == 1 and jobs[0]['phase'] == 'Completed'
assert len(jobs[0]['steps']) == 1 and jobs[0]['steps'][0]['kind'] == 'Observe'
assert not final['state']['runtime_trials'] and not final['state']['runtime_tests']
assert not final['state']['freezes']
live = final['state']['live']
assert live['sample_count'] == 0 and not live['frames']
assert live['runtime']['naming'] is None
source = read(OUT / 'source-report.json')
current = read(OUT / 'current-report.json')
catalog = live['runtime']['catalog_source']
assert sha(Path(catalog['path'])) == sha(OUT / 'current-report.json') == catalog['document_sha256']
assert source['schema'] == 4 and current['schema'] == 5
assert source['pid'] != current['pid'] and source['created'] != current['created']
assert current['pid'] == read(OUT / 'started.json')['game_pid']
assert current['root_scan_complete'] and current['stop_reason'] == 'root_scan_complete'
assert source['root_catalog_sha256'] != current['root_catalog_sha256']
recovery = read(OUT / 'recovery.json')
assert recovery['status'] == 'catalog_changed' and recovery['candidate'] is None
assert not recovery['same_process'] and not recovery['edit_authority']
old_test = read(OUT / 'source-test.json')
assert old_test['phase'] == 'complete' and old_test['assessment']['status'] == 'suppression_observed'
for key in ['id', 'locator', 'type_code', 'value', 'bytes']:
    assert recovery['source'][key] == old_test['candidate'][key], key
with tempfile.TemporaryDirectory(prefix='reversebolt-recovery-audit-') as temp:
    output = Path(temp) / 'recovery.json'
    subprocess.run([str(OUT / 'bin/runtime_recovery.exe'), str(OUT / 'source-report.json'),
                    old_test['candidate']['id'], str(OUT / 'current-report.json'), str(output)],
                   check=True, capture_output=True, text=True, timeout=90)
    assert read(output) == recovery

# Compare declared metadata for diagnosis only. This is not a migration policy.
route = recovery['source']['locator']
references = [route['root']] + [s['member']['field'] for s in route['steps'] if 'field' in s['member']]
descriptors = []
for ref in references:
    key = ref['declaring']
    before = next(c for c in source['classes'] if c['key'] == key)
    after = [c for c in current['classes'] if all(c['key'][k] == key[k] for k in ['image', 'type_name'])]
    assert len(after) == 1
    field = next(f for f in before['fields'] if f['id'] == ref['field_id'])
    fields = [f for f in after[0]['fields'] if f['name'] == field['name']]
    assert len(fields) == 1
    descriptors.append(dict(type=key['type_name'], old_key=key, new_key=after[0]['key'],
                            old_field=field, new_field=fields[0],
                            shared_descriptors_equal=all(fields[0][k] == v for k, v in field.items())))

logs = list((OUT / 'data/runtime').glob('*-*/**/stdout.jsonl'))
assert len(logs) == 1
events = [json.loads(line) for line in logs[0].read_text().splitlines()]
assert events[0]['helper_sha256'] == frozen['binaries']['reversebolt_runtime.dll']
assert events[-1]['stage'] == 'complete' and events[-1]['helper_absent']
assert events[-1]['inspection_exit'] == 0 and events[-1]['unload_exit'] == 1
pauses = current['snapshots'] + [w['discarded_snapshot'] for w in current.get('warmups', [])]
assert all(p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0 for p in pauses)
setup = read(OUT / 'setup.json')
assert setup['setup_images_viewed'] <= frozen['maximum_setup_images']
assert setup['menu_actions'] <= frozen['maximum_menu_actions']
assert setup['measurement_images_viewed'] == 0
cleanup = read(OUT / 'cleanup.json')
assert cleanup['owned_processes_exited'] and not cleanup['remaining_owned']
assert cleanup['preserved_editor']['ProcessId'] == setup['main_editor_preserved']

after_path = OUT / 'storage-after.json'
if not after_path.exists():
    storage = Path(frozen['storage'])
    with after_path.open('x', encoding='utf-8') as output:
        json.dump({'hashes': {p.relative_to(storage).as_posix(): sha(p)
                             for p in storage.rglob('*') if p.is_file()}}, output, indent=2)
after = read(after_path)['hashes']
before = frozen['storage_before']
changes = [n for n in sorted(set(before) | set(after)) if before.get(n) != after.get(n)]
save_unchanged = {n: before.get(n) == after.get(n) and n in before
                  for n in ['viewfinder_0.sav', 'viewfinder_0.bak']}
summary = dict(artifact_audit='passed', outcome=result, frozen_sha256=sha(OUT / 'frozen.json'),
               recovery_sha256=sha(OUT / 'recovery.json'), schemas=[source['schema'], current['schema']],
               source_roots=source['catalog_roots'], current_roots=current['catalog_roots'],
               descriptor_diagnosis=descriptors, helper_runs=len(logs), pauses=len(pauses),
               setup_images=setup['setup_images_viewed'], menu_actions=setup['menu_actions'],
               measurement_images=0, save_bytes_unchanged=save_unchanged, storage_changes=changes,
               cleanup=cleanup,
               limitation='Strict recovery stopped before any effect comparison. Matching shared field descriptors does not establish compatible layouts, persistent objects, verified semantics or a successful migration.')
(ROOT / 'research/runtime-recovery/results.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: summary[k] for k in ['artifact_audit', 'outcome', 'helper_runs', 'pauses',
                                       'save_bytes_unchanged', 'storage_changes']}))
