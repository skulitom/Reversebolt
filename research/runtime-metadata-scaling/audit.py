"""Recheck retained owned-process evidence without executing native code."""
import hashlib
import json
from pathlib import Path
import sys

OUT = Path(sys.argv[1]).resolve()
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
for folder, records in [('source', frozen['sources']), ('bin', frozen['binaries'])]:
    for name, digest in records.items():
        assert sha(OUT / folder / name) == digest, name
matrix = read(OUT / 'matrix.json')
assert matrix['passed']
pauses, calls, restored, readonly, maxima = 0, 0, 0, 0, []
for name, result in matrix['cases'].items():
    case = OUT / name
    assert read(OUT / (name + '-execution.json'))['exit_code'] == 0
    assert read(case / 'result.json') == result
    recorded = 0
    for call in result['calls']:
        path = case / (call['label'] + '.json')
        assert sha(path) == call['sha256']
        report = read(path)
        shots = report.get('pauses', report.get('snapshots', []))
        shots += [w['discarded_snapshot'] for w in report.get('warmups', [])]
        assert len(shots) == call['pauses']
        assert all(s['helper_allocations_while_paused'] == s['helper_deallocations_while_paused'] == 0 for s in shots)
        assert report.get('edit_authority', False) is False
        recorded += len(shots); calls += 1
        maxima.extend(s['stop_to_resume_us'] for s in shots)
    assert recorded == result['pauses']
    if 'mock_stops' in result:
        assert recorded == result['mock_stops'] == result['mock_resumes']
    pauses += recorded
    report = read(case / 'discovery.json')
    assert report.get('error') == result['error']
    if result['error']:
        assert not result['candidates'] and not recorded
        continue
    candidates = read(case / 'candidates.json')
    assert len(candidates) == result['candidates'] == 23
    assert read(case / 'null.json')['checks'][0]['status'] == 'null_reference'
    assert read(case / 'layout.json')['checks'][0]['status'] == 'type_changed'
    for label in ['edit', 'readonly']:
        before = read(case / (label + '-before.json'))
        after = read(case / (label + '-after.json'))
        assert after['checks'][0]['bytes'] == before['checks'][0]['bytes']
        change = read(case / (label + '-write.json'))['patch']
        if label == 'edit':
            patch = read(case / 'edit.patch.json'); inverse = read(case / 'edit.restore.json')
            assert change['state'] == 'written' and change['observed_after'] == patch['replacement']
            assert inverse['replacement'] == patch['expected'] and inverse['expected'] == patch['replacement']
            restoration = read(case / 'edit-restore.json')['patch']
            assert restoration['state'] == 'written' and restoration['observed_after'] == patch['expected']
            restored += 1
        else:
            assert change['reason'] == 'readonly_field' and not change['attempted']; readonly += 1
    before, after = read(case / 'refresh.json'), read(case / 'restored.json')
    assert before['checks'] == after['checks']

cases = matrix['cases']
assert cases['old-large']['error'] == 'field metadata budget'
assert cases['new-large']['metadata']['loaded_fields'] == 17
assert cases['new-large']['metadata']['root_inventory']['field_headers_examined'] == 200018
assert cases['new-exact-header-limit']['error'] is None
assert cases['new-exact-header-limit']['metadata']['root_inventory']['field_headers_examined'] == 2000000
assert cases['new-over-header-limit']['error'] == 'root field header budget'
assert not cases['new-over-header-limit']['metadata']['root_inventory']['complete']
assert cases['new-required-layout-limit']['metadata']['loaded_fields'] == 200000
assert cases['new-required-layout-limit']['error'].startswith('field metadata budget:')
def routes(name):
    return sorted(json.dumps({'root': c['locator']['root'], 'steps': c['locator']['steps'],
                              'value': c['value'], 'bytes': c['bytes']}, sort_keys=True)
                  for c in read(OUT / name / 'candidates.json'))
for name in ['new-small', 'new-large', 'new-exact-header-limit']:
    assert routes(name) == routes('old-small')
    assert cases[name]['root_fingerprint'] == cases['old-small']['root_fingerprint']
assert calls == 51 and restored == readonly == 4
summary = {'artifact_audit': 'passed', 'cases': len(cases), 'native_calls': calls, 'managed_pauses': pauses,
           'max_stop_to_resume_us': max(maxima), 'writes_restored': restored, 'readonly_writes_refused': readonly,
           'helper_sha256': frozen['binaries']['reversebolt_runtime.dll'], 'frozen_sha256': sha(OUT / 'frozen.json'),
           'unchanged_routes_values_and_root_fingerprints': True, 'large_case_loaded_fields': 17,
           'large_case_headers': 200018, 'exact_header_limit_passed': True, 'oversized_required_layout_refused': True,
           'gameplay_edits': 0, 'limitations': matrix['limitation']}
(OUT / 'audit.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps(summary))
