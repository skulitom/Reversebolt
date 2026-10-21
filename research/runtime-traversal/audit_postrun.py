"""Audit retained results without launching, inspecting, or changing a target."""
from collections import Counter
import hashlib
import json
from pathlib import Path
import statistics
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
RAW = ROOT / 'test-results/runtime-traversal-v1'


def read(path):
    return json.loads(path.read_bytes())


def digest(data):
    return hashlib.sha256(data).hexdigest()


def sha(path):
    return digest(path.read_bytes())


context = read(RAW / 'execution-context.json')
frozen = read(HERE / 'frozen-v1.json')
assert sha(HERE / 'frozen-v1.json') == context['frozen_manifest_sha256']
sources = []
binaries = []
helper_path = 'target/debug/reversebolt_runtime.dll'
for item in frozen['files']:
    path = ROOT / item['path']
    if item['path'].startswith('target/'):
        current = sha(path) if path.exists() else None
        # Workspace testing rebuilt this one shared-target DLL after the native
        # run. Do not represent its replacement as the originally tested bytes.
        if item['path'] != helper_path:
            assert current == item['sha256'], item['path']
        binaries.append({**item, 'current_sha256': current,
                         'original_bytes_still_present': current == item['sha256']})
    else:
        source = subprocess.check_output(
            ['git', 'show', context['source_revision'] + ':' + item['path']], cwd=ROOT)
        current = path.read_bytes()
        assert digest(current) == item['sha256'], item['path']
        exact = digest(source) == item['sha256']
        if not exact:
            assert item['path'] in ['Cargo.toml', 'Cargo.lock']
            assert source.replace(b'\r\n', b'\n') == current.replace(b'\r\n', b'\n')
        sources.append({'path': item['path'], 'committed_sha256': digest(source),
                        'exact_frozen_bytes': exact,
                        'line_endings_only': not exact})
helper_sha = next(i['sha256'] for i in frozen['files'] if i['path'] == helper_path)


def receipt(folder, label):
    lines = [json.loads(x) for x in (folder / (label + '.stdout.jsonl')).read_text().splitlines()]
    assert lines[0]['stage'] == 'validated'
    assert lines[0]['helper_sha256'] == helper_sha
    assert lines[-1]['stage'] == 'complete' and lines[-1]['helper_absent']
    return lines[0], lines[-1]


def coverage(report):
    assert report['schema'] == 3 and not report['graph_complete'] and not report['edit_authority']
    classes = {c['id']: c for c in report['classes']}
    root_indices = []
    counts = Counter()
    depths = Counter()
    pauses = []
    for shot in report['snapshots']:
        assert shot['helper_allocations_while_paused'] == shot['helper_deallocations_while_paused'] == 0
        assert 'warming_error' not in shot
        nodes = {n['id']: n for n in shot['nodes']}
        assert len(nodes) == len(shot['nodes'])
        assert shot['processed_nodes'] <= len(nodes)
        depths.update(n['depth'] for n in nodes.values())
        counts['unknown_type_nodes'] += sum(n['class'] is None for n in nodes.values())
        root_indices.extend(r['root'] for r in shot['roots'])
        for entry in shot['roots']:
            assert entry['read_ok']
            if 'id' in entry['node']:
                assert entry['node']['id'] in nodes
        for entry in shot['reads']:
            assert entry['read_ok']
            node = nodes[entry['node']]
            owner = classes[entry['declaring_class']]
            if entry['element_index'] is None:
                field = owner['fields'][entry['field']]
                assert field['offset'] + len(entry['bytes']) <= classes[node['class']]['instance_size']
            else:
                assert classes[node['class']]['shape'] == 'array'
            if isinstance(entry['reference'], dict) and 'id' in entry['reference']:
                assert entry['reference']['id'] in nodes
            if entry['value'] is not None:
                counts['scalar_read_occurrences'] += 1
                if node['depth'] > 0 and owner['image'] == 'Assembly-CSharp.dll':
                    counts['deeper_main_game_scalar_read_occurrences'] += 1
        counts['node_capped_snapshots'] += shot['node_capacity_reached']
        counts['time_or_read_capped_snapshots'] += shot['time_or_read_budget_reached']
        counts['successful_string_read_occurrences'] += sum(t['read_ok'] for t in shot['strings'])
        counts['truncated_string_occurrences'] += sum(t['truncated'] for t in shot['strings'])
        counts['array_read_occurrences'] += sum(a['elements_read'] > 0 for a in shot['arrays'])
        counts['truncated_array_occurrences'] += sum(a['truncated'] for a in shot['arrays'])
        counts['unsupported_array_occurrences'] += sum(not a['supported'] for a in shot['arrays'])
        pauses.append(shot['stop_to_resume_us'])
    # A zero-added warmup can alias the final shot. None occurred in these runs;
    # require that before treating each warmup receipt as a distinct pause.
    for warmup in report['warmups']:
        assert warmup['added_classes'] > 0
        shot = warmup['discarded_snapshot']
        assert shot['helper_allocations_while_paused'] == shot['helper_deallocations_while_paused'] == 0
        pauses.append(shot['stop_to_resume_us'])
    assert root_indices == list(range(report['root_start'], report['next_root']))
    assert report['root_scan_complete'] == (report['next_root'] == report['catalog_roots'])
    return {'counts': dict(counts), 'depth_occurrences': dict(depths),
            'pauses': len(pauses), 'pause_total_us': sum(pauses),
            'pause_max_us': max(pauses), 'pause_median_us': statistics.median(pauses),
            'stop_to_resume_over_5000_us': sum(p > 5000 for p in pauses),
            'root_slots_read': len(root_indices),
            'catalog_omission_occurrences': dict(Counter(report['catalog_omissions']))}


mock = RAW / 'mock'
execution = read(mock / 'execution.json')
assert execution['host_exit'] == 0
assert execution['receipts'] == [{'label': 'malformed-root', 'exit': 1},
                               {'label': 'initial', 'exit': 0},
                               {'label': 'continuation', 'exit': 0},
                               {'label': 'wrong-fingerprint', 'exit': 1}]
assert not (mock / 'malformed-root.json').exists()
assert 'root catalog changed' in read(mock / 'wrong-fingerprint.json')['error']
mock_coverage = {}
for label in ['initial', 'continuation', 'wrong-fingerprint']:
    receipt(mock, label)
    if label != 'wrong-fingerprint':
        mock_coverage[label] = coverage(read(mock / (label + '.json')))
balance = read(mock / 'host.jsonl')
assert balance == {'starts': 9, 'stops': 9}
assert sum(c['pauses'] for c in mock_coverage.values()) == 9

summary = read(RAW / 'blue-prince/summary.json')
assert summary['helper_sha256'] == helper_sha
game_coverage = []
for case in summary['cases']:
    label = 'probe-' + str(case['index'])
    path = RAW / 'blue-prince' / (label + '.json')
    assert sha(path) == case['raw_sha256']
    identity, done = receipt(RAW / 'blue-prince', label)
    assert identity['pid'] == summary['pid'] and identity['created'] == summary['created']
    assert done == case['loader']
    report = read(path)
    checked = coverage(report)
    assert checked['counts']['deeper_main_game_scalar_read_occurrences'] == case['deeper_main_game_scalar_reads']
    game_coverage.append(checked)
after = read(RAW / 'game-after.json')
assert after['responding'] and after['window'] and not after['helper_present']
assert after['pid'] == summary['pid'] and after['created'] == summary['created']
cleanup = read(RAW / 'cleanup.json')
assert cleanup['remaining_owned_processes'] == [] and cleanup['lease_released']
desktop = read(RAW / 'desktop.json')
assert desktop['ok'] and not desktop['changed_this_turn']
artifacts = sorted(p for p in RAW.rglob('*') if p.is_file())
result = {'result_consistency_audit_passed': True, 'execution_context': context,
          'source_verification': sources, 'binary_retention': binaries,
          'binary_retention_limitation': 'Full-workspace tests rebuilt target/debug/reversebolt_runtime.dll after the native run. Its original bytes are no longer retained there. Frozen SHA matches every loader validation receipt and the game harness; all other frozen files match. Future experiments should copy binaries outside the shared Cargo target before running.',
          'mock_coverage': mock_coverage, 'mock_balance': balance,
          'game': summary, 'game_coverage': game_coverage,
          'game_after': after, 'cleanup': cleanup, 'desktop': desktop,
          'screenshots': 0, 'gameplay_inputs': 0, 'game_field_write_calls': 0,
          'edit_authority': False, 'progress_percent': 31,
          'raw_artifacts': [{'path': str(p.relative_to(ROOT)).replace('\\', '/'),
                             'sha256': sha(p), 'bytes': p.stat().st_size} for p in artifacts]}
(HERE / 'results.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
print(json.dumps({'result_consistency_audit_passed': True,
                  'frozen_source_files_verified': len(sources),
                  'original_binaries_retained': sum(b['original_bytes_still_present'] for b in binaries),
                  'original_binaries_recorded': len(binaries), 'game_coverage': game_coverage}))
