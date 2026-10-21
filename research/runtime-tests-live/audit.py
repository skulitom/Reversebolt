"""Read-only audit of a completed frozen native response-test workflow."""
import hashlib
import json
import math
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
RUN = Path(sys.argv[1]).resolve()
CASE = RUN / 'case'


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def address(value):
    return int(value, 16) if isinstance(value, str) else value


def same_record(left, right, path=''):
    # serde_json's parse/Value round-trip can move an image metric by one ULP.
    # Identity, paths, bytes, times, decisions and all other fields remain exact.
    assert type(left) is type(right), path
    if isinstance(left, dict):
        assert left.keys() == right.keys(), path
        for key in left:
            same_record(left[key], right[key], path + '/' + key)
    elif isinstance(left, list):
        assert len(left) == len(right), path
        for index, (a, b) in enumerate(zip(left, right)):
            same_record(a, b, path + '/' + str(index))
    elif isinstance(left, float) and (path.endswith('/difference') or path.startswith('/assessment/medians/')):
        assert math.isfinite(left) and math.isfinite(right) and abs(left - right) <= max(math.ulp(left), math.ulp(right)), path
    else:
        assert left == right, path


def check_evidence(evidence):
    report = evidence['report']
    encoded = json.dumps(report, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()
    assert hashlib.sha256(encoded).hexdigest() == evidence['report_sha256']
    assert read(Path(evidence['path'])) == report
    return report


frozen = read(RUN / 'frozen.json')
for name, expected in frozen['sources'].items():
    assert digest(ROOT / name) == expected, name
for name, expected in frozen['binaries'].items():
    assert digest(RUN / 'bin' / name) == expected, name

result = read(CASE / 'result.json')
assert result['passed'] and len(result['assertions']) == 52
assert result['manually_reviewed_images'] == 0 and len(result['input_actions']) == 12
records = [read(p) for p in (CASE / 'data/runtime-tests').glob('*.json')]
assert len(records) == 7
assert all(t['phase'] in ('complete', 'stopped') for t in records)
frames = 0
for test in records:
    for c, condition in enumerate(test['conditions']):
        for i, frame in enumerate(condition['frames']):
            assert frame['path'] == f"{test['id']}/{c}-{i}.png"
            assert frame['input_path'] == f"{test['id']}/{c}-{i}.input.json"
            for field in ('', 'input_'):
                assert digest(CASE / 'data/runtime-tests' / frame[field + 'path']) == frame[field + 'sha256']
            assert frame['interval']['clock_id'] == test['investigation']
            assert frame['interval']['begin_us'] <= frame['interval']['end_us']
            assert (frame['width'], frame['height']) == (640, 360)
            frames += 1
assert frames == result['images'] == 24
test = next(t for t in records if t['id'] == result['first_test'])
same_record(test, read(CASE / 'complete.json')['test'])
assert test['phase'] == 'complete' and not test['pending_capture']
assert test['assessment']['status'] == 'suppression_observed'
generations = set()
for condition in test['conditions']:
    assert len(condition['frames']) == 6
    for frame in condition['frames'][1:]:
        assert frame['input']['matches']
        window = read(CASE / 'data/runtime-tests' / frame['input_path'])
        assert window['clock_id'] == test['investigation']
        assert window['generation'] == frame['input']['generation']
        assert all(p['focus'] == 'selected_foreground' for p in window['polls'])
        generations.add(window['generation'])
assert len(generations) == 1
trial = read(CASE / 'restored.json')['data']
assert trial['phase'] == 'restored' and trial['id'] == test['change_id']
patch = trial['patch']
assert read(CASE / 'original-oracle.json')['bytes'] == patch['expected']
prepared = check_evidence(test['preparation'])
assert prepared['scope'] == test['scope']
assert prepared['checks'][0]['bytes'] == patch['expected']
for index, expected in ((1, patch['replacement']), (2, patch['expected'])):
    condition = test['conditions'][index]
    report = check_evidence(condition['runtime_evidence'])
    assert report['scope'] == test['scope']
    check, = report['checks']
    assert check['status'] == 'read' and check['bytes'] == expected
    assert check['path_id'] == test['candidate']['id']
    assert check['type_code'] == patch['type_code']
    assert address(check['field_address']) == patch['field_address']
    assert list(map(address, check['snapshot_object_addresses'])) == patch['object_addresses']
    assert condition['runtime_checked_unix_ms'] >= condition['frames'][-1]['completed_unix_ms']

helpers = list((CASE / 'data/runtime').glob('*-*/**/stdout.jsonl'))
assert len(helpers) == result['helpers'] == 18
pauses = []
writes = 0
for path in helpers:
    logs = [json.loads(line) for line in path.read_text().splitlines() if line.startswith('{')]
    last = logs[-1]
    assert last['stage'] == 'complete' and last['helper_absent'] and last['inspection_exit'] == 0
    report = read(path.parent / 'report.json')
    assert report['pid'] == test['scope']['pid'] and report['session'] == 3
    pauses.extend(report.get('pauses', report.get('snapshots', [])))
    pauses.extend(w['discarded_snapshot'] for w in report.get('warmups', []))
    if report.get('operation') == 'patch_runtime_path':
        writes += 1
assert writes == 4
assert all(p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0 for p in pauses)
balance = read(CASE / 'balance.json')
assert balance['starts'] == balance['stops'] == len(pauses)
children = read(CASE / 'children.json')
assert all(c['exit_code'] is not None for c in children)
takeover = read(CASE / 'ui-takeover.json')
assert takeover['action']['structuredContent']['performed'] == 'invoke'
ui = takeover['observation']['structuredContent']
assert ui['window']['pid'] == read(CASE / 'human-stop-ready.json')['pid']
assert any(e['name'] == 'Take control' and 'invoke' in e['actions'] for e in ui['elements'])
test_log = RUN / 'workspace-tests.log'
tests = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', test_log.read_text(encoding='utf-8-sig'))))
assert tests == 257
summary = {
    'passed': True, 'artifact_root': str(RUN.relative_to(ROOT)),
    'parent_revision': frozen['parent_revision'], 'frozen': frozen,
    'workflow_checks': len(result['assertions']), 'workspace_tests': tests,
    'images': frames, 'input_actions': len(result['input_actions']),
    'manually_reviewed_images': 0, 'medians': test['assessment']['medians'],
    'runtime_helpers': len(helpers), 'writes_including_restores': writes,
    'balanced_pauses': len(pauses), 'max_stop_to_resume_us': max(p['stop_to_resume_us'] for p in pauses),
    'helper_heap_operations_while_paused': 0, 'children': children,
    'image_metric_serialization_tolerance': 'One ULP in image differences/medians only; other fields and raw artifact digests exact.',
    'failures_retained': [
        'v1/case1: full comparison completed; evaluator byte-oracle parser rejected a numeric address.',
        'v1/case2: stopped before any input because the fixture was not foreground.',
        'v1/case3: passed after evaluator fixes; v2 also passed after scoped-Stop fix.',
        'v3: final workflow after preparation-cancellation review fix; focused controller regression passed.'
    ],
    'limits': [
        'Owned ABI/visual fixture; candidate and input plan selected by evaluator. No game generalization credit.',
        'Input samples measure foreground key states, not guaranteed game delivery.',
        'Image response does not identify semantic meaning or grant edit authority.',
        'Native takeover tested between observations, not during an in-flight helper.',
        'Preparation takeover and stale Stop have controller regression coverage; no delayed native helper test.',
        'Expiry is implemented, not a timed 15-minute native test.'
    ]
}
destination = Path(__file__).with_name('results.json')
destination.write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: v for k, v in summary.items() if k not in ('frozen', 'children')}, indent=2))
