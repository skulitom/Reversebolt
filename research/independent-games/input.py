"""Evaluation-only input driver. Run inside Anode; never drive the parent desktop."""
import ctypes as c
import json
import os
from pathlib import Path
import sys
import time

config = json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'))
u = c.windll.user32
u.GetForegroundWindow.restype = c.c_void_p
u.GetWindowThreadProcessId.argtypes = [c.c_void_p, c.POINTER(c.c_ulong)]
u.GetClientRect.argtypes = [c.c_void_p, c.c_void_p]
session = c.c_ulong()
assert c.windll.kernel32.ProcessIdToSessionId(os.getpid(), c.byref(session))
assert session.value == config['session'] and session.value != config['parent_session']

class Keyboard(c.Structure):
    _fields_ = [('vk', c.c_ushort), ('scan', c.c_ushort), ('flags', c.c_ulong),
                ('time', c.c_ulong), ('extra', c.c_void_p)]
class Mouse(c.Structure):
    _fields_ = [('x', c.c_long), ('y', c.c_long), ('data', c.c_ulong),
                ('flags', c.c_ulong), ('time', c.c_ulong), ('extra', c.c_void_p)]
class Union(c.Union):
    _fields_ = [('keyboard', Keyboard), ('mouse', Mouse)]
class Input(c.Structure):
    _fields_ = [('type', c.c_ulong), ('data', Union)]
class Rect(c.Structure):
    _fields_ = [('left', c.c_long), ('top', c.c_long), ('right', c.c_long), ('bottom', c.c_long)]

def guard():
    hwnd = u.GetForegroundWindow()
    pid = c.c_ulong()
    u.GetWindowThreadProcessId(hwnd, c.byref(pid))
    assert hwnd == config['window'] and pid.value == config['pid'], 'Owned game window lost foreground; no further input.'

def key(vk, up):
    event = Input(type=1, data=Union(keyboard=Keyboard(vk=vk, flags=(2 if up else 0) | (1 if vk in [37,38,39,40] else 0))))
    assert u.SendInput(1, c.byref(event), c.sizeof(event)) == 1, 'Input not delivered to Windows.'

held = None
out = Path(config['output'])
try:
    guard()
    rect = Rect()
    assert u.GetClientRect(c.c_void_p(config['window']), c.byref(rect))
    print(json.dumps({'kind':'geometry','time_ms':time.time_ns()/1e6,
        'width':rect.right,'height':rect.bottom,'session':session.value}), flush=True)
    deadline = time.monotonic() + config['max_seconds']
    index = 0
    prefix = list(config['prefix'])
    while time.monotonic() < deadline and not Path(config['stop_file']).exists():
        vk = prefix.pop(0) if prefix else config['keys'][index % len(config['keys'])]
        guard()
        pressed = time.time_ns()/1e6
        key(vk, False); held = vk
        time.sleep(config['hold_ms']/1000)
        key(vk, True); held = None
        print(json.dumps({'kind':'pulse','index':index,'vk':vk,'down_ms':pressed,'up_ms':time.time_ns()/1e6}), flush=True)
        index += 1
        time.sleep((config['period_ms']-config['hold_ms'])/1000)
except Exception as error:
    print(json.dumps({'kind':'error','error':str(error)}), flush=True)
    raise
finally:
    if held is not None: key(held, True)
