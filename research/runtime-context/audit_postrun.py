"""Verify frozen context/search artifacts and retained native results; no target calls."""
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'test-results/runtime-context-v1'


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

audit = {'revision': manifest['revision'], 'frozen_sha256': sha(BASE / 'frozen.json'),
         'binaries': manifest['binaries'], 'source_files': len(manifest['sources']), 'runs': {}}
for case, expected_operations in [('fixture', 4), ('game', 2)]:
    run = BASE / case
    summary = read(run / 'summary.json')
    assert summary['passed']
    discovery = read(run / 'discovery.json')['state']['live']
    assert discovery['sample_count'] == 0 and not discovery['frames']
    original = discovery['runtime']
    assert len(original['cards']) == summary['cards']
    assert all(not c['candidate']['edit_authority'] for c in original['cards'])
    receipts = list((run / 'data/runtime').glob('*-*/**/stdout.jsonl'))
    assert len(receipts) == summary['operations'] == expected_operations
    pauses = 0
    for path in receipts:
        rows = [json.loads(line) for line in path.read_text().splitlines()]
        assert rows[0]['stage'] == 'validated'
        assert rows[0]['helper_sha256'] == manifest['binaries']['reversebolt_runtime.dll']
        assert rows[-1]['stage'] == 'complete' and rows[-1]['helper_absent']
        report = read(path.with_name('report.json'))
        assert all(report[k] == rows[0][k] for k in ('pid', 'created', 'session'))
        snapshots = report.get('pauses', report.get('snapshots', []) +
                               [w['discarded_snapshot'] for w in report.get('warmups', [])])
        assert all(p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0
                   for p in snapshots)
        pauses += len(snapshots)
    assert pauses == summary['pauses']
    for item in summary['evidence']:
        path = Path(item['path'])
        assert sha(path / 'stdout.jsonl') == item['receipt_sha256']
        assert sha(path / 'report.json') == item['report_sha256']
    assert not read(run / 'stale-selection.json')['ok']
    assert not read(run / 'stale.json')['ok']
    assert read(run / 'replay.json')['data']['duplicate']
    shown = read(run / 'shown.json')['state']['live']['runtime']
    refreshed = read(run / 'refresh.json')['state']['live']['runtime']
    assert shown['revision'] == original['revision'] + 1
    assert refreshed['revision'] == shown['revision'] + 1
    assert shown['catalog_source'] == refreshed['catalog_source'] == original['catalog_source']
    selected = {c['id'] for c in summary['selected']}
    old = {c['candidate']['id'] for c in original['cards']}
    assert len(selected - old) == summary['selected_beyond_initial_display']
    for card in shown['cards']:
        if card['candidate']['id'] in selected:
            assert card['observed_unix_ms'] == original['catalog_source']['observed_unix_ms']
    assert all(c['status'] == 'read' for c in refreshed['cards'] if c['candidate']['id'] in selected)
    query = read(run / 'text-search.json')['data']['search']
    assert query['query'] == summary['text_query'] and query['matches'] == summary['text_matches']
    ui = read(run / 'ui.json')
    assert not ui['includeScreenshot']
    names = {e['name'] for e in ui['visible'] if e['enabled']}
    assert {'Find more values', 'Search', 'Show card'} <= names, names
    assert summary['screenshots'] == summary['gameplay_input'] == summary['gameplay_writes'] == summary['verified_controls'] == 0
    execution = read(run / 'execution.json')
    job = read(run / 'job.json')
    assert job['finished'] and job['exitCode'] == 0
    if case == 'fixture':
        assert execution['host_exit'] == 0
        assert read(run / 'host-balance.json') == {'starts': pauses, 'stops': pauses}
    else:
        assert len(selected) == 4 and len(selected - old) == 4
    audit['runs'][case] = {**summary, 'execution': execution, 'job': job}

replay = BASE / 'replay'
source = read(replay / 'input.json')
assert sha(Path(source['input_path'])) == source['input_sha256'] and source['exploratory']
query = read(replay / 'query.json')
display = read(replay / 'display.json')
old_display = read(ROOT / 'test-results/runtime-product-v2/game/discovery.json')['state']['live']['runtime']['cards']
ids = {c['id'] for c in query['candidates']}
recomputed = {'query_matches': query['matches'], 'query_fields': [c['name'] for c in query['candidates']],
              'in_old_display': len(ids & {c['candidate']['id'] for c in old_display}),
              'in_new_display': len(ids & {c['id'] for c in display}),
              'new_cards_with_context': sum(bool(c['context']) for c in display), 'meaning_verified': False}
assert recomputed == read(replay / 'summary.json')
assert query['query'] == source['query']
audit['exploratory_replay'] = {**source, **recomputed}

after = read(BASE / 'game-after.json')
assert after['pid'] == audit['runs']['game']['target_pid']
assert after['responding'] and after['helper_absent'] and after['exited']
audit['game_after'] = after
desktop = read(BASE / 'desktop.json')
assert desktop['ok'] and desktop['session'] == 1 and desktop['window']
assert desktop['history_before'] == desktop['history_after']
for name in ('Reversebolt.exe', 'reversebolt-mcp.exe', 'reversebolt-runtime-loader.exe', 'reversebolt_runtime.dll'):
    assert sha(ROOT / 'dist' / desktop['version'] / name) == manifest['binaries'][name]
audit['desktop'] = desktop
smoke = ROOT / 'test-results/smoke-7be4d64a/report.json'
result = read(smoke)
assert result['mcp_tools'] == 39 and result['shared_session'] == result['auth_and_origin'] == 'passed'
assert all(p['edit_restore'] == 'passed' for p in result['practice'])
audit['smoke'] = {'sha256': sha(smoke), 'result': result}
audit['scope'] = 'Snapshot context and searchable catalogue; no verified gameplay effects or cross-game generalization.'
Path(__file__).with_name('results.json').write_text(json.dumps(audit, indent=2), encoding='utf-8')
print(json.dumps({'audit': 'passed', 'source_files': audit['source_files'],
                  'runs': {k: v['passed'] for k, v in audit['runs'].items()}}))
