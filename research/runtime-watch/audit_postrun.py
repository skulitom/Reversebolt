"""Audit retained watcher evidence without contacting the target process."""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'test-results/runtime-watch-v1'
RUN = BASE / 'fixture'


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


manifest = read(BASE / 'frozen.json')
for name, digest in manifest['binaries'].items():
    assert sha(BASE / 'bin' / name) == digest, name
for path, identity in manifest['sources'].items():
    data = subprocess.check_output(['git', 'show', manifest['revision'] + ':' + path], cwd=ROOT)
    assert hashlib.sha256(data).hexdigest() == identity['git_sha256'], path
    assert identity['working_sha256'] in {
        hashlib.sha256(d).hexdigest() for d in
        (data, data.replace(b'\n', b'\r\n'), data.replace(b'\r\n', b'\n'))
    }, path

summary = read(RUN / 'summary.json')
assert summary['passed']
selected_ids = {c['id'] for c in summary['selected']}
assert len(selected_ids) == 2
original = read(RUN / 'discovery.json')['state']['live']['runtime']
assert len(original['cards']) == summary['cards'] == 48
assert original['revision'] == 0
assert all(not c['candidate']['edit_authority'] and not c['readings'] for c in original['cards'])
readings = {}
states = {}
for label in ('discovery', 'first-paused', 'null-paused', 'watch', 'rollover',
              'inflight-paused', 'inflight-resumed', 'cancelled'):
    state = read(RUN / (label + '.json'))
    live = state['state']['live']
    runtime = live['runtime']
    assert live['sample_count'] == 0 and not live['frames']
    assert runtime['catalog_source'] == original['catalog_source']
    assert runtime['catalog_id'] == original['catalog_id']
    states[label] = state
    for card in runtime['cards']:
        if card['candidate']['id'] not in selected_ids:
            continue
        assert not card['candidate']['edit_authority']
        for item in card['readings']:
            key = (card['candidate']['id'], item['revision'])
            if key in readings:
                assert readings[key] == item
            readings[key] = item
            path = Path(item['evidence_path']).resolve()
            assert path.is_relative_to(RUN.resolve())
            report = read(path)
            canonical = json.dumps(report, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()
            assert hashlib.sha256(canonical).hexdigest() == item['report_sha256']
            scope = card['candidate']['locator']['scope']
            assert report['scope'] == scope
            check = next(c for c in report['checks'] if c['path_id'] == key[0])
            assert check['status'] == item['status']
            if item['status'] == 'read':
                assert check['value'] == item['value'] and check['bytes'] == item['bytes']
            else:
                assert item['value'] is None and item['bytes'] == []
            assert item['completed_unix_ms'] >= item['started_unix_ms']

def runtime(label):
    return states[label]['state']['live']['runtime']


def phase(label, job):
    return states[label]['jobs']['history']['jobs'][str(job)]['phase']


for label, revision in [('first-paused', 1), ('null-paused', 2), ('watch', 4),
                        ('rollover', 20), ('inflight-paused', 21), ('inflight-resumed', 22)]:
    assert runtime(label)['revision'] == revision
assert phase('first-paused', summary['watch_job']) == 'Paused'
assert phase('null-paused', summary['watch_job']) == 'Paused'
assert phase('inflight-paused', summary['inflight_pause_job']) == 'Paused'
assert phase('inflight-resumed', summary['inflight_pause_job']) == 'Completed'
assert phase('cancelled', summary['cancelled_job']) == 'Cancelled'
assert runtime('cancelled')['revision'] == 22
assert len(readings) == 44
for candidate_id in selected_ids:
    ordered = [readings[(candidate_id, revision)] for revision in range(1, 23)]
    assert [r['status'] for r in ordered[:4]] == ['read', 'null_reference', 'read', 'read']
    assert all(r['status'] == 'read' for r in ordered[4:])
    for a, b in zip(ordered, ordered[1:]):
        delay = 1000 if b['revision'] in (2, 3, 4, 22) else 250 if 6 <= b['revision'] <= 20 else 0
        assert b['started_unix_ms'] >= a['completed_unix_ms'] + delay
for card in runtime('inflight-resumed')['cards']:
    if card['candidate']['id'] in selected_ids:
        assert len(card['readings']) == 16 and card['readings_dropped'] == 6
        assert [r['revision'] for r in card['readings']] == list(range(7, 23))
        item = next(c for c in summary['selected'] if c['id'] == card['candidate']['id'])
        assert item['differences'] == sum(a['bytes'] != b['bytes'] for a, b in zip(card['readings'], card['readings'][1:]))
assert read(RUN / 'replay.json')['data']['duplicate']
assert read(RUN / 'replay.json')['data']['ack']['job'] == summary['watch_job']
assert not read(RUN / 'stale.json')['ok']

receipts = list((RUN / 'data/runtime').glob('*-*/**/stdout.jsonl'))
assert len(receipts) == summary['operations'] == 23
pauses = 0
for path in receipts:
    rows = [json.loads(line) for line in path.read_text(encoding='utf-8').splitlines()]
    assert rows[0]['stage'] == 'validated'
    assert rows[0]['helper_sha256'] == manifest['binaries']['reversebolt_runtime.dll']
    assert rows[-1]['stage'] == 'complete' and rows[-1]['helper_absent']
    report = read(path.with_name('report.json'))
    assert all(report[k] == rows[0][k] for k in ('pid', 'created', 'session'))
    shots = report.get('pauses', report.get('snapshots', []) + [w['discarded_snapshot'] for w in report.get('warmups', [])])
    assert all(p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0 for p in shots)
    pauses += len(shots)
for item in summary['evidence']:
    assert sha(Path(item['path']) / 'stdout.jsonl') == item['receipt_sha256']
    assert sha(Path(item['path']) / 'report.json') == item['report_sha256']
assert pauses == summary['pauses'] == 48
assert read(RUN / 'host-balance.json') == {'starts': pauses, 'stops': pauses}
job = read(RUN / 'job.json')
execution = read(RUN / 'execution.json')
assert job['finished'] and job['exitCode'] == 0 and execution['host_exit'] == 0
ui = read(RUN / 'ui.json')
assert not ui['includeScreenshot']
assert {'Watch changes', '16 checks · 0 differences'} <= {e['name'] for e in ui['visible'] if e['enabled']}
assert '16 checks · 15 differences' in ui['treeLabels']
assert {c['tool'] for c in read(RUN / 'calls.json')} <= {'status', 'list_processes', 'attach_process', 'start_runtime_job', 'job_control'}
assert summary['screenshots'] == summary['gameplay_input'] == summary['gameplay_writes'] == summary['verified_controls'] == 0
smoke_path = ROOT / 'test-results/smoke-3faf3db3/report.json'
smoke = read(smoke_path)
assert smoke['mcp_tools'] == 39 and smoke['shared_session'] == smoke['auth_and_origin'] == 'passed'
assert all(p['edit_restore'] == 'passed' for p in smoke['practice'])
assert smoke['stale_restore'] == 'rejected'
durations = [r['completed_unix_ms'] - r['started_unix_ms'] for (candidate, _), r in readings.items() if candidate == next(iter(selected_ids))]
audit = {'audit': 'passed', 'revision': manifest['revision'], 'frozen_sha256': sha(BASE / 'frozen.json'),
         'binaries': manifest['binaries'], 'source_files': len(manifest['sources']),
         'cards': summary['cards'], 'selected': summary['selected'], 'refreshes': 22,
         'helper_operations': len(receipts), 'balanced_pauses': pauses,
         'refresh_duration_ms': {'min': min(durations), 'max': max(durations)},
         'execution': execution, 'job': job, 'smoke': {'sha256': sha(smoke_path), 'result': smoke},
         'screenshots': 0, 'gameplay_input': 0, 'gameplay_writes': 0, 'verified_controls': 0,
         'scope': 'Owned fixture workflow only; gameplay effects and independent-game generalization unverified.'}
if '--with-desktop' in sys.argv:
    desktop = read(BASE / 'desktop.json')
    assert desktop['ok'] and desktop['session'] == 1 and desktop['window']
    assert desktop['history_exactly_preserved'] and desktop['history_before'] == desktop['history_after']
    for name in ('Reversebolt.exe', 'reversebolt-mcp.exe', 'reversebolt-runtime-loader.exe', 'reversebolt_runtime.dll'):
        assert sha(ROOT / 'dist' / desktop['version'] / name) == manifest['binaries'][name]
    audit['desktop'] = desktop
    Path(__file__).with_name('results.json').write_text(json.dumps(audit, indent=2), encoding='utf-8')
(BASE / 'audit.json').write_text(json.dumps(audit, indent=2), encoding='utf-8')
print(json.dumps({'audit': 'passed', 'sources': audit['source_files'], 'operations': len(receipts), 'pauses': pauses,
                  'duration_ms': audit['refresh_duration_ms'], 'desktop_checked': 'desktop' in audit}))
