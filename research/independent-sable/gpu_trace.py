"""Read-only global GPU and local-model sampling, stopped by a scoped flag file."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import urllib.request

OUT=Path(sys.argv[1]).resolve()
assert OUT.is_dir() and (OUT/'frozen.json').is_file()
stop=OUT/'stop-gpu-telemetry'
assert not stop.exists()
exe=shutil.which('nvidia-smi')
assert exe,'nvidia-smi unavailable'
args=[exe,'--query-gpu=timestamp,index,memory.total,memory.used,memory.free,utilization.gpu,utilization.memory',
      '--format=csv','-lms','1000','-f',str(OUT/'gpu.csv')]
gpu=subprocess.Popen(args,stdout=subprocess.DEVNULL,stderr=(OUT/'gpu.stderr').open('x'),
    creationflags=subprocess.CREATE_NO_WINDOW)
(OUT/'telemetry-start.json').write_text(json.dumps(dict(pid=os.getpid(),gpu_pid=gpu.pid,
    started_ns=time.time_ns(),args=args),indent=2)+'\n')
deadline=time.monotonic()+1800
reason='deadline'
try:
    with (OUT/'model-state.jsonl').open('x') as sink:
        while time.monotonic()<deadline:
            if stop.exists():
                reason='stop_requested';break
            record=dict(time_ns=time.time_ns(),gpu_process_exit=gpu.poll())
            try:
                with urllib.request.urlopen('http://127.0.0.1:11434/api/ps',timeout=2) as response:
                    payload=json.loads(response.read(128*1024+1))
                record['models']=[{k:m.get(k) for k in ('name','digest','size','size_vram','context_length','expires_at')}
                    for m in payload['models'] if m['name']=='qwen3.8:latest']
            except Exception as failure:
                record['error']=repr(failure)
            sink.write(json.dumps(record)+'\n');sink.flush()
            time.sleep(2)
finally:
    if gpu.poll() is None:
        gpu.terminate()
    gpu.wait(timeout=10)
    (OUT/'telemetry-end.json').write_text(json.dumps(dict(pid=os.getpid(),gpu_pid=gpu.pid,
        finished_ns=time.time_ns(),reason=reason,gpu_exit_code=gpu.returncode),indent=2)+'\n')
