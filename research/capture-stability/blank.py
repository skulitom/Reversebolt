"""Single persistent-blank case; explicit captures test sampling, not discovery."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time
from PIL import Image

HERE = Path(__file__).resolve().parent
config = json.loads((HERE / 'protocol-v1.json').read_text(encoding='utf-8'))
out = Path(sys.argv[1]).resolve()
out.mkdir(parents=True, exist_ok=False)
binary = Path(config['binary_directory'])
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
for name, digest in config['binaries'].items(): assert sha(binary / name) == digest
for name, digest in config['method_sha256'].items(): assert sha(HERE / name) == digest
session = ctypes.c_ulong()
assert ctypes.windll.kernel32.ProcessIdToSessionId(os.getpid(), ctypes.byref(session))
assert session.value == int(os.environ['REVERSEBOLT_TEST_SESSION']) != int(os.environ['REVERSEBOLT_PARENT_SESSION'])
control, ready, data = out/'control.txt', out/'ready.json', out/'engine'
processes, commands, lines = [], [], queue.Queue()
report = {'slot': 2, 'errors': [], 'protocol_sha256': sha(HERE/'protocol-v1.json')}
serial, mcp = 0, None
def save(name, value): (out/name).write_text(json.dumps(value, indent=2)+'\n', encoding='utf-8')
def rpc(method, params):
    global serial
    serial += 1
    mcp.stdin.write(json.dumps({'jsonrpc':'2.0','id':serial,'method':method,'params':params})+'\n'); mcp.stdin.flush()
    while True:
        response = json.loads(lines.get(timeout=120))
        if response.get('id') == serial:
            assert 'error' not in response, response
            return response['result']
def call(name, args=None):
    result = rpc('tools/call', {'name':'reversebolt_'+name,'arguments':args or {}})
    value = json.loads(next(c['text'] for c in result['content'] if c['type']=='text'))
    commands.append({'name':name,'args':args or {},'ok':value['ok'],'error':value.get('error')})
    save('commands.json', commands)
    assert not result.get('isError') and value['ok'], value.get('error')
    return value
try:
    fixture = subprocess.Popen(['powershell.exe','-NoProfile','-ExecutionPolicy','Bypass','-File',str(HERE/'blank-fixture.ps1'),'-ControlFile',str(control),'-OracleFile',str(ready)], creationflags=0x08000000)
    processes.append(fixture); report['fixture_pid']=fixture.pid
    deadline=time.time()+25
    while not ready.exists() and time.time()<deadline:
        assert fixture.poll() is None; time.sleep(.1)
    assert ready.exists()
    engine=subprocess.Popen([str(binary/'Reversebolt.exe'),'--data-dir',str(data)],creationflags=0x08000000)
    processes.append(engine); report['engine_pid']=engine.pid
    deadline=time.time()+20
    while not (data/'endpoint.json').exists() and time.time()<deadline:
        assert engine.poll() is None; time.sleep(.1)
    assert (data/'endpoint.json').exists()
    mcp=subprocess.Popen([str(binary/'reversebolt-mcp.exe'),'--data-dir',str(data)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(out/'mcp-stderr.log').open('w',encoding='utf-8'),text=True,encoding='utf-8',creationflags=0x08000000)
    processes.append(mcp); report['mcp_pid']=mcp.pid
    threading.Thread(target=lambda:[lines.put(line) for line in mcp.stdout],daemon=True).start()
    rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'capture-stability-blank','version':'1'}})
    mcp.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n'); mcp.stdin.flush()
    target=next(p for p in call('list_processes')['state']['processes'] if p['pid']==fixture.pid)
    call('set_observer_model',{'model':None})
    call('attach_process',{**{k:target[k] for k in ['pid','created','window']},'budget_mib':1,'auto_observe':False})
    receipts=[]
    for number in range(1,5):
        captured=call('capture',{'mode':'full'}); save(f'capture-{number}.json',captured)
        live=captured['state']['live']; screen=live['latest']['screen']; sampling=screen['sampling']
        assert sampling and sampling['policy']=='last_completed_in_fixed_window_v1'
        frames=sampling['frames']; assert 1<=len(frames)<=8 and sampling['selected']==len(frames)-1
        assert sampling['settle_ms']==80 and sampling['max_frames']==8
        times=[f['completed_us'] for f in frames]; assert times==sorted(times) and times[-1]-times[0]<80000
        assert frames[-1]['detail_fraction']<.001 and screen['warning']
        assert not screen['words'] and not screen['text'].strip() and not live['tracks']
        with Image.open(screen['path']) as image:
            assert hashlib.sha256(image.convert('RGBA').tobytes()).hexdigest()==frames[-1]['rgba_sha256']
        receipts.append(sampling); print(json.dumps({'captured':number,'frames':len(frames),'detail':frames[-1]['detail_fraction']}),flush=True)
    report['sampling']=receipts; report['outcome']='four genuine blank captures retained with no invented values'
    save('final-status.json',call('status'))
    assert not any(c['name'] in ['prepare_change','prepare_probe','apply_change'] for c in commands)
except Exception as error:
    report['errors'].append(repr(error)); raise
finally:
    control.write_text('close',encoding='ascii')
    for process in reversed(processes):
        if process.poll() is None:
            if process is processes[0]:
                try: process.wait(timeout=4)
                except subprocess.TimeoutExpired: process.terminate()
            else: process.terminate()
        try: process.wait(timeout=5)
        except subprocess.TimeoutExpired: report['errors'].append('Owned process did not stop: '+str(process.pid))
    save('report.json',report); print(json.dumps(report),flush=True)
