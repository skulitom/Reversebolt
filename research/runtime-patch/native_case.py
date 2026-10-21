"""Evaluate the generic patch primitive on an unchanged owned native ABI fixture."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(sys.argv[1]).resolve()
OUT.mkdir(parents=True, exist_ok=False)
BIN = OUT.parent / 'bin'
NO_WINDOW = 0x08000000
host = None
children = []
cases = []
command_id = 0
identity = None


def save(path, data):
    path.write_text(json.dumps(data, indent=2), encoding='utf-8')


def execute(label, argv, expect_success=True):
    directory = OUT / label
    directory.mkdir(exist_ok=True)
    with (directory / 'stdout.jsonl').open('w', encoding='utf-8') as stdout, (directory / 'stderr.txt').open('w', encoding='utf-8') as stderr:
        child = subprocess.Popen([str(a) for a in argv], stdout=stdout, stderr=stderr, creationflags=NO_WINDOW, cwd=ROOT)
        children.append(child)
        code = child.wait(timeout=150)
    save(directory / 'execution.json', {'pid': child.pid, 'exit': code, 'args': [str(a) for a in argv]})
    assert (code == 0) == expect_success, (label, code, (directory / 'stderr.txt').read_text(encoding='utf-8'))
    return directory


def host_command(operation, **kwargs):
    global command_id
    command_id += 1
    request = {'id': command_id, 'operation': operation, **kwargs}
    save(OUT / 'request.json', request)
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        assert host.poll() is None
        try:
            result = json.loads((OUT / 'response.json').read_bytes())
            if result['id'] == command_id:
                return result
        except (FileNotFoundError, json.JSONDecodeError):
            pass
        time.sleep(.01)
    raise AssertionError('Host evaluator response unavailable')


def oracle(label, address, expected):
    result = host_command('read', address=address, width=len(expected))
    save(OUT / (label + '-oracle.json'), {'address': address, 'expected': list(expected), 'result': result})
    assert result['bytes'] == list(expected), (label, result)


def inspect(label, flag=None, value=None, expect_success=True):
    directory = OUT / label
    directory.mkdir()
    args = [BIN / 'reversebolt-runtime-loader.exe', str(identity['pid']), identity['created'], str(identity['session']),
            BIN / 'reversebolt_runtime.dll', manifest['binaries']['reversebolt_runtime.dll'], directory / 'report.json']
    if flag:
        save(directory / 'request.json', value)
        args += [flag, directory / 'request.json']
    execute(label, args, expect_success)
    receipts = [json.loads(line) for line in (directory / 'stdout.jsonl').read_text(encoding='utf-8').splitlines()]
    if not expect_success:
        assert not receipts and not (directory / 'report.json').exists()
        cases.append({'name': label, 'preflight_refused': True})
        return None
    assert receipts[0]['stage'] == 'validated'
    assert receipts[-1]['stage'] == 'complete' and receipts[-1]['helper_absent']
    report = json.loads((directory / 'report.json').read_bytes())
    assert all(report[k] == identity[k] for k in ('pid', 'created', 'session'))
    cases.append({'name': label, 'operation': report['operation'] if 'operation' in report else 'discovery',
                  'patch': report.get('patch')})
    return report


def prepare(label, path, reading_label, desired):
    directory = OUT / label
    directory.mkdir()
    save(directory / 'path.json', path)
    execute(label, [BIN / 'reversebolt-runtime-patch.exe', 'prepare', directory / 'path.json',
                   OUT / reading_label / 'report.json', directory / 'patch.json', desired])
    return json.loads((directory / 'patch.json').read_bytes())


def inverse(label, original_label, write_label):
    directory = OUT / label
    directory.mkdir()
    execute(label, [BIN / 'reversebolt-runtime-patch.exe', 'restore', OUT / original_label / 'patch.json',
                   OUT / write_label / 'report.json', directory / 'patch.json'])
    return json.loads((directory / 'patch.json').read_bytes())


def apply(label, patch, state, reason, guard=None):
    report = inspect(label, '--patch-file', patch)
    result = report['patch']
    assert result['state'] == state and result['reason'] == reason, (label, result)
    assert result['attempted'] == (state != 'refused')
    if guard:
        assert result['guard'] == guard, (label, result)
    return report


manifest = json.loads((OUT.parent / 'frozen.json').read_bytes())
for name, digest in manifest['binaries'].items():
    assert hashlib.sha256((BIN / name).read_bytes()).hexdigest() == digest
try:
    host = subprocess.Popen([sys.executable, str(ROOT / 'research/runtime-patch/host.py'), str(OUT), str(BIN / 'reversebolt_path_fixture.dll')],
        stdout=(OUT / 'host.stdout').open('w'), stderr=(OUT / 'host.stderr').open('w'), creationflags=NO_WINDOW)
    deadline = time.monotonic() + 10
    while not (OUT / 'ready.json').exists():
        assert host.poll() is None and time.monotonic() < deadline
        time.sleep(.02)
    identity = json.loads((OUT / 'ready.json').read_bytes())
    assert identity['pid'] == host.pid
    inspect('discovery')
    execute('candidates', [BIN / 'reversebolt-runtime-candidates.exe', OUT / 'discovery/report.json', OUT / 'candidates.json', '64'])
    candidates = json.loads((OUT / 'candidates.json').read_bytes())
    chosen = next(c for c in candidates if c['name'] == 'Coefficient')
    path = chosen['locator']
    inspect('read-initial', '--paths-file', [path])
    patch = prepare('prepared', path, 'read-initial', '4.5')
    address = patch['field_address']
    before, changed = struct.pack('<f', 3.5), struct.pack('<f', 4.5)
    oracle('before', address, before)
    apply('write', patch, 'written', 'bytes_confirmed', 'ready')
    oracle('written', address, changed)
    apply('duplicate-bytes', patch, 'refused', 'guard_refused', 'value_changed')
    oracle('duplicate-refused', address, changed)
    restoration = inverse('inverse', 'prepared', 'write')
    apply('restore', restoration, 'written', 'bytes_confirmed', 'ready')
    oracle('restored', address, before)
    apply('inverse-drift', restoration, 'refused', 'guard_refused', 'value_changed')
    bad = copy.deepcopy(patch)
    bad['type_code'] = 8
    bad['expected'], bad['replacement'] = list(struct.pack('<i', 1)), list(struct.pack('<i', 2))
    apply('wrong-type', bad, 'refused', 'guard_refused', 'representation_changed')
    bad = copy.deepcopy(patch)
    bad['object_addresses'][0] += 8
    apply('wrong-chain', bad, 'refused', 'guard_refused', 'path_moved')
    bad = copy.deepcopy(patch)
    bad['field_address'] += 4
    apply('wrong-address', bad, 'refused', 'guard_refused', 'path_moved')
    host_command('tick', mode=0)
    apply('replaced-root', patch, 'refused', 'guard_refused', 'path_moved')
    oracle('replacement-refused', address, before)
    inspect('read-replacement', '--paths-file', [path])
    patch2 = prepare('prepared-replacement', path, 'read-replacement', '4.5')
    apply('write-replacement', patch2, 'written', 'bytes_confirmed', 'ready')
    inverse2 = inverse('inverse-replacement', 'prepared-replacement', 'write-replacement')
    host_command('tick', mode=2)
    apply('restore-replaced-root', inverse2, 'refused', 'guard_refused', 'path_moved')
    oracle('restore-refused', address, changed)
    inspect('read-cleanup', '--paths-file', [path])
    cleanup = prepare('prepared-cleanup', path, 'read-cleanup', '3.5')
    apply('explicit-cleanup', cleanup, 'written', 'bytes_confirmed', 'ready')
    oracle('cleanup', address, before)
    host_command('tick', mode=1)
    apply('null-root', patch2, 'refused', 'path_unavailable')
    host_command('tick', mode=3)
    apply('changed-runtime-type', patch2, 'refused', 'path_unavailable')
    host_command('tick', mode=0)
    array = next(c for c in candidates if c['type_code'] == 12 and c['locator']['steps'][-1]['member']['kind'] == 'element')
    inspect('read-array', '--paths-file', [array['locator']])
    array_patch = prepare('prepared-array', array['locator'], 'read-array', '2.25')
    apply('readonly-array', array_patch, 'refused', 'page_not_writable', 'ready')
    oracle('readonly-refused', array_patch['field_address'], bytes(array_patch['expected']))
    expired = copy.deepcopy(patch)
    expired['prepared_unix_ms'] = 0
    inspect('expired', '--patch-file', expired, expect_success=False)
    wrong_scope = copy.deepcopy(patch)
    wrong_scope['path']['scope']['created'] = '0000000000000000'
    inspect('wrong-scope', '--patch-file', wrong_scope, expect_success=False)
    execute('nonfinite', [BIN / 'reversebolt-runtime-patch.exe', 'prepare', OUT / 'prepared/path.json', OUT / 'read-initial/report.json', OUT / 'nonfinite.json', 'NaN'], False)
    assert not (OUT / 'nonfinite.json').exists()
    oracle('final', address, before)
    save(OUT / 'summary.json', {'passed': True, 'target': identity, 'selected_id': chosen['id'], 'cases': cases,
        'screenshots': 0, 'gameplay_input': 0, 'verified_controls': 0,
        'scope': 'Owned ABI fixture; numeric writes only, not verified gameplay effects or persistent object identity.'})
finally:
    if host is not None:
        (OUT / 'stop').touch()
        try:
            host.wait(timeout=5)
        except subprocess.TimeoutExpired:
            host.terminate()
            host.wait(timeout=5)
    for child in children:
        if child.poll() is None:
            child.terminate()
            child.wait(timeout=5)
    save(OUT / 'execution.json', {'host_pid': None if host is None else host.pid,
        'host_exit': None if host is None else host.returncode,
        'children': [{'pid': c.pid, 'exit': c.returncode} for c in children]})
assert host.returncode == 0
pauses = 0
operations = 0
for case in cases:
    if case.get('preflight_refused'):
        continue
    report = json.loads((OUT / case['name'] / 'report.json').read_bytes())
    shots = report.get('pauses', report.get('snapshots', []) + [w['discarded_snapshot'] for w in report.get('warmups', [])])
    assert all(p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0 for p in shots)
    pauses += len(shots)
    operations += 1
assert json.loads((OUT / 'balance.json').read_bytes()) == {'starts': pauses, 'stops': pauses}
print(json.dumps({'passed': True, 'cases': len(cases), 'helper_operations': operations, 'balanced_pauses': pauses,
    'host_exit': host.returncode, 'child_processes': len(children), 'verified_controls': 0}))
