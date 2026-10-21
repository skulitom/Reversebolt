"""Headless evaluator host of the unchanged runtime-path ABI fixture."""
import ctypes as C
import json
import os
from pathlib import Path
import sys
import time
from ctypes import wintypes as W

out = Path(sys.argv[1])
os.environ['REVERSEBOLT_PATH_FIXTURE_CONTROL'] = str(out / 'mode.txt')
(out / 'mode.txt').write_text('0', encoding='utf-8')
k = C.WinDLL('kernel32', use_last_error=True)
k.GetCurrentProcess.restype = W.HANDLE
k.GetProcessTimes.argtypes = [W.HANDLE] + [C.POINTER(W.FILETIME)] * 4
k.ProcessIdToSessionId.argtypes = [W.DWORD, C.POINTER(W.DWORD)]
k.ReadProcessMemory.argtypes = [W.HANDLE, C.c_void_p, C.c_void_p, C.c_size_t, C.POINTER(C.c_size_t)]
times = [W.FILETIME() for _ in range(4)]
assert k.GetProcessTimes(k.GetCurrentProcess(), *[C.byref(t) for t in times])
session = W.DWORD()
assert k.ProcessIdToSessionId(os.getpid(), C.byref(session))
library = C.CDLL(sys.argv[2])
library.MockTick.argtypes = []
library.MockTick.restype = None
library.MockBalance.argtypes = []
library.MockBalance.restype = C.c_uint64
library.MockTick()
(out / 'ready.json').write_text(json.dumps({'pid': os.getpid(), 'session': session.value,
    'created': f'{(times[0].dwHighDateTime << 32) | times[0].dwLowDateTime:016x}'}), encoding='utf-8')
seen = 0
try:
    while not (out / 'stop').exists():
        try:
            command = json.loads((out / 'request.json').read_bytes())
        except (FileNotFoundError, json.JSONDecodeError):
            time.sleep(.01)
            continue
        if command['id'] <= seen:
            time.sleep(.01)
            continue
        seen = command['id']
        if command['operation'] == 'tick':
            (out / 'mode.txt').write_text(str(command['mode']), encoding='utf-8')
            library.MockTick()
            result = {'id': seen, 'mode': command['mode']}
        elif command['operation'] == 'read':
            assert 1 <= command['width'] <= 8
            buf = (C.c_ubyte * command['width'])()
            count = C.c_size_t()
            assert k.ReadProcessMemory(k.GetCurrentProcess(), command['address'], buf, len(buf), C.byref(count))
            assert count.value == len(buf)
            result = {'id': seen, 'bytes': list(buf)}
        else:
            raise AssertionError('Unknown evaluator operation')
        (out / 'response.json').write_text(json.dumps(result), encoding='utf-8')
finally:
    balance = library.MockBalance()
    (out / 'balance.json').write_text(json.dumps({'stops': balance >> 32, 'starts': balance & 0xffffffff}), encoding='utf-8')
