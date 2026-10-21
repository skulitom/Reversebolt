"""Check frozen identities, native patch receipts, evaluator reads and cleanup."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'test-results/runtime-patch-v1'
RUN = BASE / 'fixture'


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


manifest = read(BASE / 'frozen.json')
for name, digest in manifest['binaries'].items():
    assert sha(BASE / 'bin' / name) == digest
assert sha(BASE / 'bin/reversebolt_path_fixture.dll') == sha(ROOT / 'test-results/runtime-paths-v2/bin/reversebolt_path_fixture.dll')
for path, hashes in manifest['sources'].items():
    data = subprocess.check_output(['git', 'show', manifest['revision'] + ':' + path], cwd=ROOT)
    assert hashlib.sha256(data).hexdigest() == hashes['git_sha256']
    assert hashes['working_sha256'] in {
        hashlib.sha256(d).hexdigest() for d in (data, data.replace(b'\n', b'\r\n'), data.replace(b'\r\n', b'\n'))}

summary = read(RUN / 'summary.json')
assert summary['passed']
identity = summary['target']
assert identity == read(RUN / 'ready.json')
assert summary['screenshots'] == summary['gameplay_input'] == summary['verified_controls'] == 0
expected = {
    'write': ('written', 'ready'), 'restore': ('written', 'ready'),
    'write-replacement': ('written', 'ready'), 'explicit-cleanup': ('written', 'ready'),
    'duplicate-bytes': ('refused', 'value_changed'), 'inverse-drift': ('refused', 'value_changed'),
    'wrong-type': ('refused', 'representation_changed'),
    'wrong-chain': ('refused', 'path_moved'), 'wrong-address': ('refused', 'path_moved'),
    'replaced-root': ('refused', 'path_moved'), 'restore-replaced-root': ('refused', 'path_moved'),
    'null-root': ('refused', None), 'changed-runtime-type': ('refused', None),
    'readonly-array': ('refused', 'ready'),
}
assert len(summary['cases']) == 21
pauses = operations = 0
max_pause = 0
receipts = []
states = {}
for case in summary['cases']:
    directory = RUN / case['name']
    rows = [json.loads(line) for line in (directory / 'stdout.jsonl').read_text(encoding='utf-8').splitlines()]
    execution = read(directory / 'execution.json')
    if case.get('preflight_refused'):
        assert execution['exit'] != 0 and not rows and not (directory / 'report.json').exists()
        continue
    assert execution['exit'] == 0
    assert rows[0]['stage'] == 'validated' and rows[0]['helper_sha256'] == manifest['binaries']['reversebolt_runtime.dll']
    assert rows[-1]['stage'] == 'complete' and rows[-1]['helper_absent']
    report = read(directory / 'report.json')
    assert all(report[k] == rows[0][k] == identity[k] for k in ('pid', 'created', 'session'))
    shots = report.get('pauses', report.get('snapshots', []) + [w['discarded_snapshot'] for w in report.get('warmups', [])])
    assert all(p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0 for p in shots)
    pauses += len(shots)
    max_pause = max(max_pause, max(p['stop_to_resume_us'] for p in shots))
    operations += 1
    if case['name'] in expected:
        patch = read(directory / 'request.json')
        # Requests originate from the typed Rust serializer; preserve that field
        # order when hashing its compact, UTF-8 representation.
        digest = hashlib.sha256(json.dumps(patch, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
        assert report['patch_id'] == digest
        receipt = report['patch']
        assert receipt == case['patch']
        assert (receipt['state'], receipt['guard']) == expected[case['name']]
        assert receipt['attempted'] == (receipt['state'] == 'written')
        if receipt['attempted']:
            assert receipt['transferred'] == len(patch['replacement'])
            assert receipt['observed_after'] == patch['replacement']
            check = report['checks'][0]
            assert check['bytes'] == patch['expected'] and check['type_code'] == patch['type_code']
            assert int(check['field_address'], 16) == patch['field_address']
            assert [int(v, 16) for v in check['snapshot_object_addresses']] == patch['object_addresses']
        else:
            assert receipt['transferred'] == 0 and receipt['observed_after'] is None
        states[case['name']] = receipt
    receipts.append({'case': case['name'], 'stdout_sha256': sha(directory / 'stdout.jsonl'),
                     'report_sha256': sha(directory / 'report.json')})
assert operations == 19 and pauses == 42
assert read(RUN / 'balance.json') == {'starts': pauses, 'stops': pauses}
assert states['readonly-array']['reason'] == 'page_not_writable'
assert read(RUN / 'null-root/report.json')['checks'][0]['status'] == 'null_reference'
assert read(RUN / 'changed-runtime-type/report.json')['checks'][0]['status'] == 'type_changed'

for source, inverse in [('prepared', 'inverse'), ('prepared-replacement', 'inverse-replacement')]:
    a = read(RUN / source / 'patch.json')
    b = read(RUN / inverse / 'patch.json')
    assert a['expected'] == b['replacement'] and a['replacement'] == b['expected']
    assert all(a[k] == b[k] for k in ('path', 'type_code', 'field_address', 'object_addresses'))
oracles = {}
for path in RUN.glob('*-oracle.json'):
    oracle = read(path)
    assert oracle['expected'] == oracle['result']['bytes']
    oracles[path.stem] = {'sha256': sha(path), **oracle}
assert len(oracles) == 9
assert oracles['before-oracle']['result']['bytes'] == oracles['restored-oracle']['result']['bytes'] == oracles['final-oracle']['result']['bytes']
assert oracles['written-oracle']['result']['bytes'] == oracles['restore-refused-oracle']['result']['bytes']
assert oracles['written-oracle']['result']['bytes'] != oracles['final-oracle']['result']['bytes']
assert not (RUN / 'nonfinite.json').exists() and read(RUN / 'nonfinite/execution.json')['exit'] != 0
execution = read(RUN / 'execution.json')
assert execution['host_exit'] == 0 and execution['host_pid'] == identity['pid']
recorded_children = {r['pid']: r['exit'] for r in execution['children']}
observed_children = {r['pid']: r['exit'] for r in [read(p) for p in RUN.glob('*/execution.json')]}
assert recorded_children == observed_children and len(recorded_children) == 29
assert all(v in (0, 1) for v in recorded_children.values())
assert sum(v == 1 for v in recorded_children.values()) == 3
smoke_path = ROOT / 'test-results/smoke-aa378b1e/report.json'
smoke = read(smoke_path)
assert smoke['mcp_tools'] == 39 and smoke['shared_session'] == smoke['auth_and_origin'] == 'passed'
assert all(p['edit_restore'] == 'passed' for p in smoke['practice'])
assert smoke['stale_restore'] == 'rejected'
audit = {'audit': 'passed', 'revision': manifest['revision'], 'frozen_sha256': sha(BASE / 'frozen.json'),
         'binaries': manifest['binaries'], 'source_files': len(manifest['sources']), 'target': identity,
         'cases': len(summary['cases']), 'states': states, 'helper_operations': operations,
         'balanced_pauses': pauses, 'max_stop_to_resume_us': max_pause, 'confirmed_numeric_writes': 4,
         'receipts': receipts, 'oracles': oracles, 'execution': execution,
         'smoke': {'sha256': sha(smoke_path), 'result': smoke},
         'screenshots': 0, 'gameplay_input': 0, 'verified_controls': 0,
         'scope': 'Owned ABI fixture. No real-game effects, persistent object identity, crash recovery or app/MCP edit integration proven.'}
Path(__file__).with_name('results.json').write_text(json.dumps(audit, indent=2), encoding='utf-8')
print(json.dumps({'audit': 'passed', 'cases': len(summary['cases']), 'helpers': operations, 'pauses': pauses,
                  'max_stop_to_resume_us': max_pause, 'confirmed_writes': 4, 'sources': len(manifest['sources'])}))
