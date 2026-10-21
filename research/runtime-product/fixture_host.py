"""Windowed host of the previously frozen owned runtime ABI fixture."""
import ctypes, json, os, sys, tkinter as tk
from pathlib import Path

out = Path(sys.argv[1])
library = ctypes.CDLL(sys.argv[2])
library.MockTick.argtypes = []
library.MockTick.restype = None
library.MockBalance.argtypes = []
library.MockBalance.restype = ctypes.c_uint64
root = tk.Tk()
root.title('Reversebolt runtime product fixture')
root.geometry('400x150+700+500')
tk.Label(root, text='Owned runtime fixture\nInternal values have no on-screen counters.').pack(pady=35)
def tick():
    library.MockTick()
    if (out / 'host-stop').exists():
        root.destroy()
    else:
        root.after(100, tick)
root.after(0, tick)
try:
    root.mainloop()
finally:
    value = library.MockBalance()
    (out / 'host-balance.json').write_text(json.dumps({'stops':value >> 32, 'starts':value & 0xffffffff}))
