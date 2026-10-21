"""Owned visual/input adapter for the unchanged ABI fixture; not a game evaluator."""
import ctypes as C,json,os,sys,time,tkinter as tk
from pathlib import Path
from ctypes import wintypes as W

out=Path(sys.argv[1]);os.environ['REVERSEBOLT_PATH_FIXTURE_CONTROL']=str(out/'mode.txt')
(out/'mode.txt').write_text('0')
lib=C.CDLL(sys.argv[2]);lib.MockTick.argtypes=[];lib.MockTick.restype=None;lib.MockBalance.restype=C.c_uint64;lib.MockTick()
user=C.WinDLL('user32');user.GetForegroundWindow.restype=W.HWND;user.GetAncestor.argtypes=[W.HWND,W.UINT];user.GetAncestor.restype=W.HWND
user.SetForegroundWindow.argtypes=[W.HWND];user.GetAsyncKeyState.argtypes=[C.c_int];user.GetAsyncKeyState.restype=C.c_short
kernel=C.WinDLL('kernel32');session=W.DWORD();assert kernel.ProcessIdToSessionId(os.getpid(),C.byref(session)) and session.value!=1
window=tk.Tk();window.title('Reversebolt response test fixture');window.geometry('640x360+100+100');window.attributes('-topmost',True)
canvas=tk.Canvas(window,width=640,height=360,background='#20252b',highlightthickness=0);canvas.pack(fill='both',expand=True)
window.update();hwnd=user.GetAncestor(window.winfo_id(),2);window.focus_force();user.SetForegroundWindow(hwnd)
items=[canvas.create_rectangle(i*32,0,i*32+32,360,fill=('#e0e0e0' if i%2 else '#294968'),outline='') for i in range(-3,24)]
(out/'ready.json').write_text(json.dumps({'pid':os.getpid(),'window':hwnd,'session':session.value}))
seen=0;address=None;position=0.;previous=time.monotonic();ticks=0;held_ticks=0
try:
    while not (out/'stop').exists():
        window.update();now=time.monotonic();delta=min(.05,now-previous);previous=now;ticks+=1
        if address and user.GetForegroundWindow()==hwnd:
            direction=int(user.GetAsyncKeyState(68)<0)-int(user.GetAsyncKeyState(65)<0)
            if direction:
                held_ticks+=1;position+=direction*C.c_float.from_address(address).value*40*delta
                offset=position%64
                for item,i in zip(items,range(-3,24)):canvas.coords(item,i*32+offset,0,i*32+32+offset,360)
        try:request=json.loads((out/'request.json').read_bytes())
        except (FileNotFoundError,json.JSONDecodeError):time.sleep(.008);continue
        if request['id']<=seen:time.sleep(.008);continue
        seen=request['id'];op=request['operation']
        if op=='configure':address=int(request['address'],16);response={'address':request['address'],'value':C.c_float.from_address(address).value}
        elif op=='read':
            location=request['address'];location=int(location,16) if isinstance(location,str) else int(location)
            response={'bytes':list(C.string_at(location,request['width']))}
        else:raise AssertionError(op)
        (out/'response.json').write_text(json.dumps(dict(id=seen,**response)))
finally:
    balance=lib.MockBalance();(out/'balance.json').write_text(json.dumps({'stops':balance>>32,'starts':balance&0xffffffff,'ticks':ticks,'held_ticks':held_ticks}))
    window.destroy()
