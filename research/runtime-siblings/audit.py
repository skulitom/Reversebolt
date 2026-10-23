"""Post-run integrity/coverage audit, without target or model access."""
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-siblings-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
for name, digest in frozen['sources'].items():
    assert sha(ROOT / name) == sha(OUT / 'source' / name) == digest, name
for name, digest in frozen['binaries'].items():
    assert sha(OUT / 'bin' / name) == digest, name
for name, digest in frozen['inputs'].items():
    assert sha(ROOT / name) == digest, name
coverage = []
for result in read(OUT / 'coverage.json'):
    plan_file = OUT / result['case'] / 'plan.json'
    assert sha(plan_file) == result['plan_sha256']
    plan = read(plan_file)
    ids = [c['id'] for c in plan['candidates']]
    assert len(ids) == len(set(ids)) <= 128
    assert sum(r['selected_fields'] for r in plan['roles']) == len(ids)
    assert sum(r['available_fields'] for r in plan['roles']) - len(ids) == plan['omitted_fields']
    paths = []
    for i, batch in enumerate(plan['batches']):
        path = OUT / result['case'] / f'paths-{i}.json'
        assert read(path) == batch and len(batch) <= 32 and path.stat().st_size <= 60 * 1024
        paths.extend(batch)
    assert paths == [c['locator'] for c in plan['candidates']]
    assert all(c['edit_authority'] is False for c in plan['candidates'])
    coverage.append({'case': result['case'], 'roles': len(plan['roles']),
        'selected': len(ids), 'omitted': plan['omitted_fields'], 'batches': result['batches'],
        'plan_sha256': result['plan_sha256']})
native = read(OUT / 'native/result.json')
execution = read(OUT / 'native/execution.json')
balance = read(OUT / 'native/host.jsonl')
assert execution['host_exit'] == 0 and balance == {'starts': native['pauses'], 'stops': native['pauses']}
maximum = 0
for i, call in enumerate(native['calls']):
    output = OUT / 'native' / f'{call["label"]}.json'
    report = read(output)
    final = json.loads((OUT / 'native' / f'{call["label"]}.stdout.jsonl').read_text().splitlines()[-1])
    assert call['exit'] == 0 and final['helper_absent'] and final['inspection_exit'] == 0
    if i:
        for p in report['pauses']:
            q = p['sampling_qpc']
            assert q['frequency_hz'] == native['frequency_hz']
            assert call['begin_ticks'] <= q['begin_ticks'] <= q['end_ticks'] <= call['end_ticks']
            assert p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0
            maximum = max(maximum, p['stop_to_resume_us'])
tests = (ROOT / 'test-results/runtime-siblings-workspace.log').read_text(encoding='utf-8-sig')
counts = re.findall(r'test result: ok\. (\d+) passed; 0 failed', tests)
assert counts and not re.search(r'test result: FAILED|error:', tests)
result = {'passed': True, 'coverage': coverage, 'source_files': len(frozen['sources']),
    'native_calls': len(native['calls']), 'balanced_pauses': native['pauses'],
    'maximum_refresh_stop_to_resume_us': maximum, 'workspace_tests': sum(map(int, counts)),
    'native_input_association_evaluated': False, 'gameplay_controls_verified': 0,
    'progress_percent': 36}
(OUT / 'audit.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
