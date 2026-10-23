"""Read-only current-state activity with bounded input and selected-window images."""
import ast,ctypes as C
from ctypes import wintypes as W
import hashlib,json,os,queue,subprocess,sys,threading,time
from pathlib import Path
OUT=Path(sys.argv[1]).resolve();PID=int(sys.argv[2]);BIN=OUT/'bin';DATA=OUT/'data'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json')
assert sha(Path(__file__))==frozen['sources']['research/runtime-state-activity/live.py']
for name,digest in frozen['binaries'].items():assert sha(BIN/name)==digest
for name,names in [('research/runtime-embedded-activity/live.py',{'save','ticks','poll','command','native'}),('research/independent-runtime/run.py',{'rpc','call','status'})]:
 source=OUT/'source'/name;assert sha(source)==frozen['sources'][name]
 definitions=[n for n in ast.parse(source.read_text(encoding='utf-8-sig')).body if isinstance(n,ast.FunctionDef) and n.name in names]
 assert len(definitions)==len(names)
 exec(compile(ast.Module(body=definitions,type_ignores=[]),str(source),'exec'),globals())
assert not (OUT/'started.json').exists(),'Never replay a started native run'
user,kernel=C.WinDLL('user32'),C.WinDLL('kernel32')
user.GetForegroundWindow.restype=W.HWND;user.GetWindowThreadProcessId.argtypes=[W.HWND,C.POINTER(W.DWORD)]
user.GetAsyncKeyState.restype=C.c_short;user.MapVirtualKeyW.restype=W.UINT
class KEYBDINPUT(C.Structure):_fields_=[('wVk',W.WORD),('wScan',W.WORD),('dwFlags',W.DWORD),('time',W.DWORD),('dwExtraInfo',C.c_size_t)]
class MOUSEINPUT(C.Structure):_fields_=[('dx',W.LONG),('dy',W.LONG),('mouseData',W.DWORD),('dwFlags',W.DWORD),('time',W.DWORD),('dwExtraInfo',C.c_size_t)]
class UNION(C.Union):_fields_=[('ki',KEYBDINPUT),('mi',MOUSEINPUT)]
class INPUT(C.Structure):_fields_=[('type',W.DWORD),('u',UNION)]
assert C.sizeof(INPUT)==40
user.SendInput.argtypes=[W.UINT,C.POINTER(INPUT),C.c_int];user.SendInput.restype=W.UINT
frequency=C.c_int64();assert kernel.QueryPerformanceFrequency(C.byref(frequency))
session=W.DWORD();assert kernel.ProcessIdToSessionId(os.getpid(),C.byref(session)) and session.value==3
mcp=None;lines=queue.Queue();seq=0;calls=[];pauses=[];holds=[];captures=[]
def guard():
 p=poll();assert p['selected_foreground'] and not p['down'],'Focus/input conflict';return p
def send(key,up):
 scan=user.MapVirtualKeyW(key,4);assert 0<scan<256
 packet=INPUT(type=1,u=UNION(ki=KEYBDINPUT(0,scan,8|(2 if up else 0),0,0)))
 return user.SendInput(1,C.byref(packet),C.sizeof(INPUT))
def capture(batch,index):
 guard();begin=ticks();response=call('capture',{'mode':'play'});guard()
 frames=response['state']['live']['frames'];assert len(frames)==len(captures)+1
 record={'batch':batch,'sample':index,'frame':frames[-1],'begin_ticks':begin,'end_ticks':ticks()}
 captures.append(record);assert len(captures)<=frozen['max_images'];save(f'capture-{batch}-{index}',record)
def measure(batch,index,paths):
 guard();key={2:68,5:65,8:68,11:65}.get(index);polls=[];errors=[];stop=threading.Event();hold=None;input_thread=None
 def polling():
  try:
   while not stop.is_set():polls.append(poll());stop.wait(.005)
  except BaseException as error:errors.append(repr(error))
 def holding():
  hold['begin_ticks']=ticks()
  try:
   hold['down_accepted']=send(key,False);assert hold['down_accepted']==1
   end=time.perf_counter()+frozen['hold_ms']/1000
   while time.perf_counter()<end:
    current=poll()
    if not current['selected_foreground'] or any(k!=key for k in current['down']):hold['interrupted']=True;break
    time.sleep(min(.005,max(0,end-time.perf_counter())))
  except BaseException as error:errors.append(repr(error))
  finally:hold['up_accepted']=send(key,True);hold['end_ticks']=ticks()
 poll_thread=threading.Thread(target=polling,daemon=True);poll_thread.start()
 try:
  if key:
   hold={'key':key,'planned_ms':frozen['hold_ms'],'batch':batch,'sample':index};input_thread=threading.Thread(target=holding);input_thread.start();time.sleep(.1)
  else:time.sleep(.25)
  report=native(f'sample-{batch}-{index}',paths);time.sleep(.075)
 finally:
  if input_thread:input_thread.join(timeout=6)
  stop.set();poll_thread.join(timeout=2)
  if hold:holds.append(hold)
  evidence={'scope':scope,'frequency_hz':frequency.value,'expected_keys':[key] if key else [],'polls':polls}
  save(f'input-{batch}-{index}',{'input':evidence,'hold':hold,'errors':errors})
 assert not errors and all(p['selected_foreground'] for p in polls)
 assert not any(any(k!=key for k in p['down']) for p in polls)
 assert hold is None or (not hold.get('interrupted') and hold['down_accepted']==hold['up_accepted']==1 and not input_thread.is_alive())
 if index in frozen['capture_samples']:capture(batch,index)
 return {'report':report,'input':evidence}
save('started',{'runner_pid':os.getpid(),'engine_pid':read(DATA/'endpoint.json')['pid'],'game_pid':PID,'unix_ms':time.time_ns()//1000000})
try:
 mcp=subprocess.Popen([str(BIN/'reversebolt-mcp.exe'),'--data-dir',str(DATA)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(OUT/'mcp.stderr').open('x'),text=True,encoding='utf-8',creationflags=subprocess.CREATE_NO_WINDOW)
 threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
 rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'current-state-diagnostic','version':'1'}})
 mcp.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');mcp.stdin.flush()
 assert status()['state']['live'] is None
 target=next(p for p in call('list_processes')['state']['processes'] if p['pid']==PID)
 assert target['window']>0;save('target',target);guard()
 scope={'pid':PID,'created':target['created'].lower(),'session':session.value}
 call('attach_process',{'pid':PID,'created':target['created'],'window':target['window'],'budget_mib':1,'auto_observe':False})
 report=native('discovery');print(json.dumps({'stage':'discovery','next_root':report['next_root'],'catalog_roots':report['catalog_roots'],'complete':report['root_scan_complete']}),flush=True)
 code=command('selection',[BIN/'reversebolt-runtime-hypotheses.exe',OUT/'discovery.json',OUT/'selection',frozen['model'],'--select-only','--current-state'],timeout=1000)
 assert code==0,'State selection process failed'
 receipt=read(OUT/'selection/receipt.json')
 assert receipt['model_digest']==frozen['model_digest'] and receipt['server_version']==frozen['server_version']
 assert receipt['selection_purpose']=='current_state' and not receipt['error'] and not receipt['cancelled'] and receipt['batch'] is None and receipt['selection']['rows']
 print(json.dumps({'stage':'selected','calls':len(receipt['stages']),'types':receipt['selection']['selected_types']}),flush=True)
 code=command('planning',[BIN/'reversebolt-runtime-neighbors.exe',OUT/'discovery.json',OUT/'selection/receipt.json',OUT/'plan','--selected-types']);assert code==0
 plan=read(OUT/'plan/plan.json');assert plan['scope']==scope
 save('frozen-plan',{'sha256':sha(OUT/'plan/plan.json'),'batches':len(plan['batches']),'selected':len(plan['candidates']),'omitted':plan['omitted_fields']})
 results=[]
 for batch,paths in enumerate(plan['batches'][:frozen['max_batches']]):
  observations=[]
  for index in range(14):
   observations.append(measure(batch,index,paths))
   print(json.dumps({'stage':'measured','batch':batch,'sample':index+1,'fields':len(paths),'images':len(captures)}),flush=True)
  save(f'observations-{batch}',observations)
  code=command(f'analysis-{batch}',[BIN/'reversebolt-runtime-response.exe',OUT/'plan/plan.json',batch,OUT/f'observations-{batch}.json',OUT/f'analysis-{batch}.json'])
  results.append({'batch':batch,'exit':code,'analysis':read(OUT/f'analysis-{batch}.json') if code==0 else None})
 current=status();live=current['state']['live']
 image_evidence=call('read_play_evidence',{'investigation_id':live['investigation_id'],'batch':live['coverage']['batch'],'frame_ids':[c['frame']['id'] for c in captures],'auto_regions':False})['data']
 save('image-evidence',image_evidence)
 save('result',{'finished':True,'batches':results,'omitted_batches':max(0,len(plan['batches'])-frozen['max_batches']),'calls':calls,'pauses':len(pauses),'max_stop_to_resume_us':max(s['stop_to_resume_us'] for s in pauses),'holds':holds,'captures':captures,'model_calls':len(receipt['stages']),'gameplay_writes':0,'measurement_images':len(captures),'viewed_measurement_images':0})
 print(json.dumps({'finished':True,'valid_batches':sum(b['exit']==0 for b in results),'associations':sum(sum(f['repeated_association'] for f in b['analysis']['fields']) for b in results if b['exit']==0)}),flush=True)
except BaseException as error:save('failure',{'error':repr(error),'calls':calls,'holds':holds,'captures':captures});raise
finally:
 if mcp is not None and mcp.poll() is None:
  mcp.stdin.close()
  try:mcp.wait(timeout=15)
  except subprocess.TimeoutExpired:mcp.terminate();mcp.wait(timeout=15)
 save('child',{'pid':mcp.pid if mcp else None,'exit_code':mcp.poll() if mcp else None})
