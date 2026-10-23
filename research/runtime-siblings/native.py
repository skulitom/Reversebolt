"""Read-only native ABI/clock smoke test, without foreground interaction."""
import ctypes as C
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-siblings-v1'
BIN = OUT / 'bin'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
assert sha(Path(__file__)) == frozen['sources']['research/runtime-siblings/native.py']
for name, digest in frozen['binaries'].items():
    assert sha(BIN / name) == digest
CASE = OUT / 'native'
CASE.mkdir()
control = CASE / 'mode.txt'
control.write_text('0')
env = dict(os.environ, REVERSEBOLT_PATH_FIXTURE_CONTROL=str(control))
kernel = C.WinDLL('kernel32', use_last_error=True)
frequency = C.c_int64()
assert kernel.QueryPerformanceFrequency(C.byref(frequency))
def ticks():
    value = C.c_int64()
    assert kernel.QueryPerformanceCounter(C.byref(value))
    return value.value
calls = []
pauses = 0
hostlog = (CASE / 'host.jsonl').open('w')
host = subprocess.Popen([str(BIN / 'object-host.exe'), str(BIN / 'reversebolt_path_fixture.dll'),
    str(CASE / 'identity.json'), str(CASE / 'stop')], env=env, stdout=hostlog, stderr=subprocess.STDOUT,
    creationflags=subprocess.CREATE_NO_WINDOW)
try:
    end = time.monotonic() + 10
    while not (CASE / 'identity.json').exists():
        assert host.poll() is None and time.monotonic() < end
        time.sleep(.05)
    identity = read(CASE / 'identity.json')
    base = [str(BIN / 'reversebolt-runtime-loader.exe'), str(identity['pid']),
        identity['created'], str(identity['session']), str(BIN / 'reversebolt_runtime.dll'),
        frozen['binaries']['reversebolt_runtime.dll']]
    def call(label, extra=()):
        global pauses
        before = ticks()
        result = subprocess.run(base + [str(CASE / f'{label}.json')] + list(extra),
            capture_output=True, text=True, timeout=130, creationflags=subprocess.CREATE_NO_WINDOW)
        after = ticks()
        (CASE / f'{label}.stdout.jsonl').write_text(result.stdout)
        (CASE / f'{label}.stderr.jsonl').write_text(result.stderr)
        calls.append({'label': label, 'exit': result.returncode, 'begin_ticks': before, 'end_ticks': after})
        assert result.returncode == 0, result.stderr
        completion = json.loads(result.stdout.splitlines()[-1])
        assert completion['stage'] == 'complete' and completion['helper_absent'] and completion['inspection_exit'] == 0
        report = read(CASE / f'{label}.json')
        if extra:
            samples = report['pauses']
            previous = before
            for sample in samples:
                qpc = sample['sampling_qpc']
                assert qpc['frequency_hz'] == frequency.value
                assert previous <= qpc['begin_ticks'] <= qpc['end_ticks'] <= after
                previous = qpc['end_ticks']
            assert all(c['status'] == 'read' for c in report['checks'])
        else:
            samples = report['snapshots'] + [w['discarded_snapshot'] for w in report['warmups']]
        assert all(s['helper_allocations_while_paused'] == s['helper_deallocations_while_paused'] == 0 for s in samples)
        pauses += len(samples)
        return report
    call('discovery')
    subprocess.run([str(BIN / 'reversebolt-runtime-candidates.exe'), str(CASE / 'discovery.json'),
        str(CASE / 'candidates.json'), '128'], check=True, capture_output=True,
        creationflags=subprocess.CREATE_NO_WINDOW)
    candidates = read(CASE / 'candidates.json')
    # The fixture's first supported paths; no field-name or value selection.
    paths = [c['locator'] for c in candidates[:4]]
    assert paths
    (CASE / 'paths.json').write_text(json.dumps(paths))
    observations = [call(f'refresh-{i}', ['--paths-file', str(CASE / 'paths.json')]) for i in range(3)]
    (CASE / 'result.json').write_text(json.dumps({'passed': True, 'calls': calls,
        'pauses': pauses, 'frequency_hz': frequency.value,
        'changes': [[c['value'] for c in r['checks']] for r in observations]}, indent=2))
finally:
    (CASE / 'stop').touch()
    try:
        host.wait(timeout=5)
    except subprocess.TimeoutExpired:
        host.terminate()
        host.wait(timeout=5)
    hostlog.close()
    (CASE / 'execution.json').write_text(json.dumps({'host_pid': host.pid, 'host_exit': host.returncode,
        'calls': calls, 'expected_pauses': pauses}, indent=2))
assert host.returncode == 0 and read(CASE / 'host.jsonl') == {'starts': pauses, 'stops': pauses}
print(json.dumps({'passed': True, 'calls': len(calls), 'balanced_pauses': pauses}))
