"""One engine-owned image/input baseline; no runtime discovery or gameplay edit."""
import ast
import ctypes as C
from ctypes import wintypes as W
import hashlib
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time

OUT = Path(sys.argv[1]).resolve()
PID = int(sys.argv[2])
BIN, DATA = OUT / 'bin', OUT / 'data'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
assert sha(Path(__file__)) == frozen['sources']['research/conditional-gameplay/live.py']
for name, digest in frozen['binaries'].items():
    assert sha(BIN / name) == digest
source = OUT / 'source/research/independent-runtime/run.py'
assert sha(source) == frozen['sources']['research/independent-runtime/run.py']
names = {'save', 'rpc', 'call', 'status'}
definitions = [n for n in ast.parse(source.read_text(encoding='utf-8-sig')).body if isinstance(n, ast.FunctionDef) and n.name in names]
assert len(definitions) == len(names)
exec(compile(ast.Module(body=definitions, type_ignores=[]), str(source), 'exec'), globals())
user, kernel = C.WinDLL('user32'), C.WinDLL('kernel32')
user.GetForegroundWindow.restype = W.HWND
user.GetWindowThreadProcessId.argtypes = [W.HWND, C.POINTER(W.DWORD)]
user.GetAsyncKeyState.restype = C.c_short
user.MapVirtualKeyW.restype = W.UINT
class KEYBDINPUT(C.Structure):
    _fields_ = [('wVk', W.WORD), ('wScan', W.WORD), ('dwFlags', W.DWORD), ('time', W.DWORD), ('dwExtraInfo', C.c_size_t)]
class MOUSEINPUT(C.Structure):
    _fields_ = [('dx', W.LONG), ('dy', W.LONG), ('mouseData', W.DWORD), ('dwFlags', W.DWORD), ('time', W.DWORD), ('dwExtraInfo', C.c_size_t)]
class UNION(C.Union):
    _fields_ = [('ki', KEYBDINPUT), ('mi', MOUSEINPUT)]
class INPUT(C.Structure):
    _fields_ = [('type', W.DWORD), ('u', UNION)]
assert C.sizeof(INPUT) == 40
user.SendInput.argtypes = [W.UINT, C.POINTER(INPUT), C.c_int]
user.SendInput.restype = W.UINT
frequency = C.c_int64()
assert kernel.QueryPerformanceFrequency(C.byref(frequency))
session = W.DWORD()
assert kernel.ProcessIdToSessionId(os.getpid(), C.byref(session)) and session.value == 3
mcp, seq, lines = None, 0, queue.Queue()
holds = []
def ticks():
    value = C.c_int64()
    assert kernel.QueryPerformanceCounter(C.byref(value))
    return value.value
def guard(allowed=None):
    hwnd = user.GetForegroundWindow()
    owner = W.DWORD()
    user.GetWindowThreadProcessId(hwnd, C.byref(owner))
    down = [k for k in range(1, 256) if user.GetAsyncKeyState(k) & 0x8000]
    assert hwnd == target['window'] and owner.value == PID, 'Selected game lost focus'
    assert all(k == allowed for k in down), 'Foreign input'
def send(key, up):
    scan = user.MapVirtualKeyW(key, 4)
    assert 0 < scan < 256
    packet = INPUT(type=1, u=UNION(ki=KEYBDINPUT(0, scan, 8 | (2 if up else 0), 0, 0)))
    return user.SendInput(1, C.byref(packet), C.sizeof(INPUT))
def hold(key, sample):
    guard()
    record = {'key': key, 'after_sample': sample, 'planned_ms': frozen['hold_ms'], 'frequency_hz': frequency.value, 'begin_ticks': ticks()}
    try:
        record['down_accepted'] = send(key, False)
        assert record['down_accepted'] == 1
        end = time.perf_counter() + frozen['hold_ms'] / 1000
        while time.perf_counter() < end:
            guard(key)
            time.sleep(min(.005, max(0, end - time.perf_counter())))
    finally:
        record['up_accepted'] = send(key, True)
        record['end_ticks'] = ticks()
        holds.append(record)
        save('hold-' + str(len(holds)), record)
    assert record['up_accepted'] == 1
    guard()

save('started', {'runner_pid': os.getpid(), 'engine_pid': read(DATA / 'endpoint.json')['pid'], 'game_pid': PID})
try:
    mcp = subprocess.Popen([str(BIN / 'reversebolt-mcp.exe'), '--data-dir', str(DATA)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                           stderr=(OUT / 'mcp.stderr').open('x'), text=True, encoding='utf-8', creationflags=subprocess.CREATE_NO_WINDOW)
    threading.Thread(target=lambda: [lines.put(line) for line in mcp.stdout], daemon=True).start()
    rpc('initialize', {'protocolVersion': '2025-11-25', 'capabilities': {}, 'clientInfo': {'name': 'conditional-gameplay-baseline', 'version': '1'}})
    mcp.stdin.write(json.dumps({'jsonrpc': '2.0', 'method': 'notifications/initialized'}) + '\n')
    mcp.stdin.flush()
    assert status()['state']['live'] is None
    target = next(p for p in call('list_processes')['state']['processes'] if p['pid'] == PID)
    save('target', target)
    guard()
    attached = call('attach_process', {'pid': PID, 'created': target['created'], 'window': target['window'], 'budget_mib': 1, 'auto_observe': False})
    history = attached['jobs']['history']
    args = {'request_id': 1, 'boot': history['boot'], 'binding': history['current'], 'count': frozen['max_images'],
            'interval_ms': frozen['interval_ms'], 'mode': 'play'}
    call('start_observation_job', args)
    completed_holds = set()
    end = time.perf_counter() + 180
    while time.perf_counter() < end:
        current = status()
        count = current['state']['live']['sample_count']
        if count in [2, 4, 8, 10] and count not in completed_holds:
            completed_holds.add(count)
            hold(68 if count in [2, 8] else 65, count)
            print(json.dumps({'sample': count, 'holds': len(holds)}), flush=True)
        jobs = current['jobs']['history']['jobs']
        if jobs and all(j['phase'] in ['Completed', 'Failed', 'Cancelled', 'Interrupted'] for j in jobs.values()):
            break
        time.sleep(.025)
    else:
        raise RuntimeError('Existing observation job still live; inspect it, do not restart')
    save('final-state', current)
    assert all(j['phase'] == 'Completed' for j in jobs.values()), 'Observation failed'
    assert count == frozen['max_images'] and len(holds) == 4, 'Incomplete fixed sequence'
    live = current['state']['live']
    evidence = call('read_play_evidence', {'investigation_id': live['investigation_id'], 'batch': live['coverage']['batch'],
                                          'frame_ids': [f['id'] for f in live['frames']], 'auto_regions': False})['data']
    save('image-evidence', evidence)
    save('result', {'finished': True, 'images': count, 'holds': holds, 'activity_source_available': evidence['activity_source'] is not None,
                    'native_runtime_calls': 0, 'gameplay_writes': 0, 'viewed_measurement_images': 0})
    print(json.dumps({'finished': True, 'images': count, 'activity_source_available': evidence['activity_source'] is not None}), flush=True)
except BaseException as error:
    save('failure', {'error': repr(error), 'holds': holds})
    raise
finally:
    if mcp is not None and mcp.poll() is None:
        mcp.stdin.close()
        try:
            mcp.wait(timeout=15)
        except subprocess.TimeoutExpired:
            mcp.terminate()
            mcp.wait(timeout=15)
    save('child', {'pid': mcp.pid if mcp else None, 'exit_code': mcp.poll() if mcp else None})
