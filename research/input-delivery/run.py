"""Paired input-API diagnostic using unchanged product capture/evidence APIs."""
import ast,ctypes as C
from ctypes import wintypes as W
import hashlib,json,os,queue,subprocess,sys,threading,time
from pathlib import Path
OUT=Path(sys.argv[1]).resolve();PID=int(sys.argv[2]);DATA=OUT/'data'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json');BIN=Path(frozen['bin'])
assert sha(Path(__file__))==frozen['sources']['research/input-delivery/run.py']
for name,digest in frozen['binaries'].items():assert sha(BIN/name)==digest
source=OUT/'source/research/independent-runtime/run.py'
assert sha(source)==frozen['sources']['research/independent-runtime/run.py']
names={'save','rpc','call','status'}
definitions=[n for n in ast.parse(source.read_text(encoding='utf-8-sig')).body if isinstance(n,ast.FunctionDef) and n.name in names]
assert len(definitions)==len(names)
exec(compile(ast.Module(body=definitions,type_ignores=[]),str(source),'exec'),globals())
assert not (OUT/'started.json').exists(),'Never replay a started run'
user=C.WinDLL('user32');kernel=C.WinDLL('kernel32')
user.GetForegroundWindow.restype=W.HWND;user.GetWindowThreadProcessId.argtypes=[W.HWND,C.POINTER(W.DWORD)]
user.GetAsyncKeyState.restype=C.c_short;user.MapVirtualKeyW.restype=W.UINT
class KEYBDINPUT(C.Structure):
 _fields_=[('wVk',W.WORD),('wScan',W.WORD),('dwFlags',W.DWORD),('time',W.DWORD),('dwExtraInfo',C.c_size_t)]
class MOUSEINPUT(C.Structure):
 _fields_=[('dx',W.LONG),('dy',W.LONG),('mouseData',W.DWORD),('dwFlags',W.DWORD),('time',W.DWORD),('dwExtraInfo',C.c_size_t)]
class UNION(C.Union):_fields_=[('ki',KEYBDINPUT),('mi',MOUSEINPUT)]
class INPUT(C.Structure):_fields_=[('type',W.DWORD),('u',UNION)]
assert C.sizeof(INPUT)==40
user.SendInput.argtypes=[W.UINT,C.POINTER(INPUT),C.c_int];user.SendInput.restype=W.UINT
session=W.DWORD();assert kernel.ProcessIdToSessionId(os.getpid(),C.byref(session)) and session.value==3
frequency=C.c_int64();assert kernel.QueryPerformanceFrequency(C.byref(frequency))
mcp=None;lines=queue.Queue();seq=0;holds=[];captures=[]
def ticks():
 t=C.c_int64();assert kernel.QueryPerformanceCounter(C.byref(t));return t.value
def poll():
 begin=ticks();hwnd=user.GetForegroundWindow();owner=W.DWORD();user.GetWindowThreadProcessId(hwnd,C.byref(owner))
 return {'begin_ticks':begin,'end_ticks':ticks(),'foreground':owner.value==PID and hwnd==target['window'],'down':[k for k in range(1,256) if user.GetAsyncKeyState(k)&0x8000]}
def guard(allowed):
 p=poll();assert p['foreground'] and all(k in allowed for k in p['down']),'Focus or foreign input conflict';return p
def wait_quiet():
 end=time.perf_counter()+frozen['hold_ms']/1000
 while time.perf_counter()<end:guard([]);time.sleep(.005)
def capture():
 guard([]);begin=ticks();r=call('capture',{'mode':'play'});guard([])
 frames=r['state']['live']['frames'];assert len(frames)==len(captures)+1
 frame=frames[-1];captures.append({'frame':frame,'begin_ticks':begin,'end_ticks':ticks()})
 assert len(captures)<=frozen['max_images'];return frame
def hold(method,key):
 guard([]);scan=user.MapVirtualKeyW(key,4 if method=='send_input' else 0);assert scan and scan<256
 record={'method':method,'key':key,'scan':scan,'planned_ms':frozen['hold_ms'],'frequency_hz':frequency.value,'polls':[],'begin_ticks':ticks()}
 def send(up):
  if method=='legacy':user.keybd_event(0,scan,10 if up else 8,0);return None
  packet=INPUT(type=1,u=UNION(ki=KEYBDINPUT(0,scan,8|(2 if up else 0),0,0)))
  return user.SendInput(1,C.byref(packet),C.sizeof(INPUT))
 try:
  record['down_accepted']=send(False)
  assert method=='legacy' or record['down_accepted']==1
  end=time.perf_counter()+frozen['hold_ms']/1000
  while time.perf_counter()<end:record['polls'].append(guard([key]));time.sleep(.005)
 finally:
  record['up_accepted']=send(True);record['end_ticks']=ticks();holds.append(record);save(f'hold-{len(holds)}',record)
 assert method=='legacy' or record['up_accepted']==1
 guard([])
save('started',{'runner_pid':os.getpid(),'engine_pid':read(DATA/'endpoint.json')['pid'],'game_pid':PID,'unix_ms':time.time_ns()//1000000})
try:
 mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(OUT/'mcp.stderr').open('x'),text=True,encoding='utf-8',creationflags=subprocess.CREATE_NO_WINDOW)
 threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
 rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'input-delivery-diagnostic','version':'1'}})
 mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
 assert status()['state']['live'] is None
 target=next(p for p in call('list_processes')['state']['processes'] if p['pid']==PID)
 assert target['window']>0;save('target',target);guard([])
 call('attach_process',{'pid':PID,'created':target['created'],'window':target['window'],'budget_mib':1,'auto_observe':False})
 capture()
 for method in frozen['methods']:
  for key in frozen['keys']:
   wait_quiet();capture();hold(method,key);capture()
   print(json.dumps({'trial':len(holds),'method':method,'key':key,'images':len(captures)}),flush=True)
 wait_quiet();capture()
 current=status();live=current['state']['live'];save('captures',captures)
 evidence=call('read_play_evidence',{'investigation_id':live['investigation_id'],'batch':live['coverage']['batch'],'frame_ids':[c['frame']['id'] for c in captures],'auto_regions':False})['data']
 save('evidence',evidence);save('result',{'finished':True,'holds':holds,'captures':captures,'gameplay_writes':0,'model_calls':0,'manually_viewed_measurement_images':0})
 print(json.dumps({'finished':True,'images':len(captures),'holds':len(holds)}),flush=True)
except BaseException as error:save('failure',{'error':repr(error),'holds':holds,'captures':captures});raise
finally:
 if mcp is not None and mcp.poll() is None:
  mcp.stdin.close()
  try:mcp.wait(timeout=15)
  except subprocess.TimeoutExpired:mcp.terminate();mcp.wait(timeout=15)
 save('child',{'pid':mcp.pid if mcp else None,'exit_code':mcp.poll() if mcp else None})
