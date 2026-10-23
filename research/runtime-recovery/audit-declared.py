"""Audit the declared-route attempts, preserving the timed-out first run."""
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import tempfile
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()


def audit(out):
    frozen = read(out / 'frozen.json')
    data = Path(frozen.get('data', out / 'data'))
    parent = Path(frozen['continuation_of']) if 'continuation_of' in frozen else None
    storage_frozen = read(parent / 'frozen.json') if parent else frozen
    for directory, records in [('source', frozen['sources']), ('bin', frozen['binaries']),
                               ('storage-before', frozen.get('storage_before', {})), ('', frozen['historical'])]:
        for name, digest in records.items():
            assert sha(out / directory / name) == digest, name
    for path, digest in frozen.get('parent_artifacts', {}).items():
        assert sha(Path(path)) == digest
    final = read(out / 'final-state.json')
    old = read(out / 'source-test.json')
    assert old['phase'] == 'complete' and old['assessment']['status'] == 'suppression_observed'
    assert final['jobs']['in_flight'] is None and final['jobs']['fence'] is None
    assert not final['jobs']['history']['unresolved'] and not final['state']['freezes']
    assert read(out / 'child.json')['exit_code'] == 0
    assert not (out / 'needs-recovery.json').exists() and not (out / 'cleanup-error.json').exists()
    assert final['state']['live']['sample_count'] == 0 and not final['state']['live']['frames']
    assert final['state']['live']['runtime']['naming'] is None
    jobs = list(final['jobs']['history']['jobs'].values())
    assert len(jobs) == 1
    logs = list((data / 'runtime').glob('*-*/**/stdout.jsonl'))
    if (out / 'failure.json').exists():
        logs = [Path(final['state']['live']['runtime']['evidence']) / 'stdout.jsonl']
    helpers, pauses = [], []
    for log in logs:
        events = [json.loads(line) for line in log.read_text().splitlines()]
        assert events[0]['helper_sha256'] == frozen['binaries']['reversebolt_runtime.dll']
        helpers.append(dict(path=str(log), sha256=sha(log), completed=events[-1]['stage'] == 'complete'))
        report = read(log.with_name('report.json'))
        assert report['pid'] == read(out / 'started.json')['game_pid']
        assert report['created'] == events[0]['created'] and report['session'] == 3
        pauses.extend(report.get('snapshots', report.get('pauses', [])))
        pauses.extend(w['discarded_snapshot'] for w in report.get('warmups', []))
        if events[-1]['stage'] == 'complete':
            assert events[-1]['helper_absent'] and events[-1]['inspection_exit'] == 0 and events[-1]['unload_exit'] == 1
    assert all(p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0 for p in pauses)
    setup = read(out / 'setup.json')
    assert setup['setup_images_viewed'] <= frozen['maximum_setup_images']
    assert setup['menu_actions'] <= frozen['maximum_menu_actions'] and setup['measurement_images_viewed'] == 0
    cleanup = read(out / 'cleanup.json')
    assert cleanup['owned_processes_exited'] and not cleanup['remaining_owned']
    assert cleanup['preserved_editor']['ProcessId'] == setup['main_editor_preserved']
    after = read(out / 'storage-after.json')['hashes']
    before = storage_frozen['storage_before']
    result = dict(artifact_audit='passed', frozen_sha256=sha(out / 'frozen.json'),
                  model_calls=0, helpers=helpers, pauses=len(pauses), setup_images=setup['setup_images_viewed'],
                  menu_actions=setup['menu_actions'], manually_viewed_measurement_images=0,
                  save_bytes_unchanged={n: n in before and before[n] == after.get(n)
                                       for n in ['viewfinder_0.sav', 'viewfinder_0.bak']},
                  storage_changes=[n for n in sorted(set(before) | set(after)) if before.get(n) != after.get(n)],
                  cleanup=cleanup)
    if (out / 'failure.json').exists():
        assert not final['state']['runtime_tests'] and not final['state']['runtime_trials']
        assert not (out / 'new-plan.json').exists() and len(logs) == 1
        assert read(out / 'anode-terminal.json')['exitCode'] == 1
        if jobs[0]['phase'] == 'Failed':
            assert final['state']['live']['runtime']['phase'] == 'uncertain'
            assert final['state']['live']['runtime']['catalog_source'] is None
            assert not (out / 'recovery.json').exists()
            assert '40s' in logs[0].with_name('stderr.jsonl').read_text()
            assert not helpers[0]['completed']
            result.update(outcome='inspection_timeout_before_recovery', gameplay_writes=0,
                          automatic_measurement_images=0, helper_unload_confirmed=False,
                          late_report_sha256=sha(logs[0].with_name('report.json')))
        else:
            assert jobs[0]['phase'] == 'Completed' and helpers[0]['completed']
            assert 'Search internal values first.' in read(out / 'failure.json')['error']
            assert read(out / 'recovery.json')['status'] == 'candidate_for_revalidation'
            result.update(outcome='selection_step_rejected_before_test', gameplay_writes=0,
                          automatic_measurement_images=0, helper_unload_confirmed=True,
                          discovery_total_ms=events[-1]['elapsed_ms'])
    else:
        assert jobs[0]['phase'] == 'Completed' and all(h['completed'] for h in helpers)
        assert read(out / 'anode-terminal.json')['exitCode'] == 0
        proposal = read(out / 'recovery.json')
        assert proposal['status'] == 'candidate_for_revalidation' and proposal['matching_routes'] == 1
        assert not proposal['edit_authority'] and not proposal['strict']['same_process']
        assert proposal['strict']['status'] == 'catalog_changed'
        assert read((parent or out) / 'recovery-terminal.json')['exit_code'] == 0
        if parent:
            assert read(out / 'continuation-state.json')['state']['live'] == read(parent / 'final-state.json')['state']['live']
            assert read(out / 'continuation-state.json')['jobs'] == read(parent / 'final-state.json')['jobs']
        with tempfile.TemporaryDirectory(prefix='reversebolt-declared-audit-') as temp:
            output = Path(temp) / 'proposal.json'
            subprocess.run([str(out / 'bin/runtime_recovery.exe'), '--propose-declared-route',
                            str(out / 'source-report.json'), old['candidate']['id'],
                            str(out / 'current-report.json'), str(output)],
                           check=True, capture_output=True, timeout=90)
            assert read(output) == proposal
        source = final['state']['live']['runtime']['catalog_source']
        assert sha(out / 'current-report.json') == sha(Path(source['path'])) == source['document_sha256']
        plan = read(out / 'new-plan.json')
        assert plan['candidate_id'] == proposal['candidate']['id'] != old['candidate']['id']
        assert plan['actions'] == old['actions'] and plan['value'] == old['value'] and plan['hypothesis'] == old['hypothesis']
        comparison = read(out / 'comparison.json')
        test = comparison['test']
        assert len(final['state']['runtime_trials']) == len(final['state']['runtime_tests']) == 1
        trial = final['state']['runtime_trials'][0]
        assert test == read(data / 'runtime-tests' / f"{test['id']}.json")
        summary = final['state']['runtime_tests'][0]
        for key in set(summary) & set(test):
            assert summary[key] == test[key], key
        assert summary['frames'] == [len(c['frames']) for c in test['conditions']]
        assert summary['candidate_id'] == test['candidate']['id']
        assert trial == read(data / 'runtime-trials' / f"{trial['id']}.json")
        assert test['phase'] == 'complete' and not summary['active'] and trial['phase'] == 'restored'
        assert test['id'] == plan['test_id'] and trial['id'] == plan['change_id'] == test['change_id']
        assert test['candidate']['id'] == trial['candidate_id'] == plan['candidate_id']
        assert test['candidate']['locator'] == proposal['candidate']['locator']

        def evidence(record):
            assert sha(Path(record['path'])) == record['report_sha256']
            assert read(Path(record['path'])) == record['report']

        evidence(test['preparation']); evidence(trial['baseline'])
        for key in ['write', 'restoration']:
            action = trial[key]; evidence(action['evidence'])
            assert action['error'] is None and action['outcome'] == 'written'
            assert action['evidence']['report']['patch']['observed_after'] == action['patch']['replacement']
            assert action['evidence']['report']['patch']['guard'] == 'ready'
        assert trial['restoration']['patch']['replacement'] == trial['patch']['expected']
        assert trial['restoration']['patch']['expected'] == trial['patch']['replacement']
        raw_medians, holds = [], 0
        for condition, c in enumerate(test['conditions']):
            assert len(c['frames']) == 6
            if condition:
                evidence(c['runtime_evidence'])
                assert c['runtime_evidence']['report']['checks'][0]['bytes'] == trial['patch']['replacement' if condition == 1 else 'expected']
                assert c['frames'][0]['started_unix_ms'] >= trial['write' if condition == 1 else 'restoration']['completed_ms']
            differences, previous = [], None
            for i, frame in enumerate(c['frames']):
                image = data / 'runtime-tests' / frame['path']
                assert sha(image) == frame['sha256']
                assert sha(data / 'runtime-tests' / frame['input_path']) == frame['input_sha256']
                witness = read(data / 'runtime-tests' / frame['input_path'])
                assert witness['omitted_polls'] == 0 and all(p['focus'] == 'selected_foreground' for p in witness['polls'])
                if i: assert frame['input']['matches']
                executed = comparison['executions'][condition * 6 + i]
                prefix = data / 'runtime-tests' / test['id'] / f'{condition}-{i}'
                assert sha(Path(str(prefix) + '.action.json')) == executed['plan_sha256']
                assert read(Path(str(prefix) + '.action.json')) == executed['plan']
                assert sha(Path(str(prefix) + '.action.outcome.json')) == executed['outcome']['sha256']
                assert read(Path(str(prefix) + '.action.outcome.json')) == executed['outcome']['receipt']
                action = None if i < 2 else test['actions'][i - 2]
                assert executed['plan']['action'] == action
                outcome = executed['outcome']['receipt']
                assert outcome['error'] is None and outcome['plan_sha256'] == executed['plan_sha256']
                assert outcome['keydown_accepted'] == outcome['keyup_accepted'] == (len(action['keys']) if action else 0)
                if action:
                    assert any(p['down'] == action['keys'] for p in witness['polls'])
                    assert all(not p['down'] or p['down'] == action['keys'] for p in witness['polls'])
                    holds += 1
                pixels = np.asarray(Image.open(image).convert('RGB'), dtype=np.int32)
                gray = ((pixels[:, :, 0]*299 + pixels[:, :, 1]*587 + pixels[:, :, 2]*114 + 500)//1000).astype(np.float64)
                if i >= 2: differences.append(float(np.mean(np.abs(gray - previous))))
                previous = gray
            raw_medians.append(statistics.median(differences))
        medians = [statistics.median(f['difference'] for f in c['frames'][2:]) for c in test['conditions']]
        assert medians == test['assessment']['medians']
        assert len(comparison['executions']) == 18 and holds == 12
        for i in range(2, 6):
            held = [c['frames'][i]['input']['held_us'][0] for c in test['conditions']]
            assert max(held) - min(held) <= max(50000, max(held) / 4)
        assert test['conditions'][1]['runtime_checked_unix_ms'] <= trial['restoration']['started_ms']
        result.update(outcome=test['assessment']['status'], strict_status=proposal['strict']['status'],
                      matching_routes=proposal['matching_routes'], candidate_id=plan['candidate_id'],
                      gameplay_writes=1, restored=True, automatic_measurement_images=18, input_holds=holds,
                      product_medians=medians, independent_full_frame_gray_medians=raw_medians,
                      proposal_sha256=sha(out / 'recovery.json'), comparison_sha256=sha(out / 'comparison.json'))
    result['limitation'] = 'Known development game; no persistent actor identity, exact semantics, physical speed units, complete world rollback or unfamiliar-game generalization established.'
    (ROOT / 'research/runtime-recovery' / f'{out.name}-results.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: result[k] for k in ['outcome', 'artifact_audit', 'gameplay_writes', 'save_bytes_unchanged']}))


for name in ['runtime-recovery-declared-v1', 'runtime-recovery-declared-buffered-v1', 'runtime-recovery-declared-continuation-v1']:
    if (ROOT / 'test-results' / name / 'cleanup.json').exists():
        audit(ROOT / 'test-results' / name)
