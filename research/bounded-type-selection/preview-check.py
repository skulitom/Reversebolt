"""Check a staged package without attaching to a target or replacing the editor."""
import hashlib
import json
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BIN = Path(sys.argv[1]).resolve()
OUT = Path(sys.argv[2]).resolve()
OUT.mkdir()
data = OUT / 'data'
process = subprocess.Popen([str(BIN / 'Reversebolt.exe'), '--headless', '--data-dir', str(data)],
    stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=subprocess.CREATE_NO_WINDOW)
result = {'passed': False, 'pid': process.pid, 'bin': str(BIN), 'target_access': False,
          'binary_sha256': hashlib.sha256((BIN / 'Reversebolt.exe').read_bytes()).hexdigest()}
try:
    for _ in range(200):
        assert process.poll() is None, 'Engine exited before readiness'
        endpoint_path = data / 'endpoint.json'
        if endpoint_path.exists():
            endpoint = json.loads(endpoint_path.read_text(encoding='utf-8-sig'))
            if endpoint['pid'] == process.pid:
                break
        time.sleep(0.05)
    else:
        raise RuntimeError('Engine startup timed out')
    errors = {}
    result['unconnected_command_errors'] = errors
    requests = [
        ('inspect_runtime_investigation', {'catalog_id': '', 'review_id': ''}),
        ('recover_runtime_control', {'catalog_id': '', 'test_id': '', 'after_revision': 0}),
    ]
    for action, args in requests:
        request = urllib.request.Request(endpoint['url'] + '/command',
            data=json.dumps({'action': action, 'args': args}).encode(), headers={
                'Content-Type': 'application/json', 'Authorization': 'Bearer ' + endpoint['token']})
        with urllib.request.urlopen(request, timeout=15) as response:
            value = json.load(response)
        errors[action] = value['error']
        assert value['ok'] is False and value['error'] == 'Connect an application first.'
    result.update(passed=True, unconnected_command_errors=errors)
finally:
    process.terminate()
    stdout, stderr = process.communicate(timeout=15)
    (OUT / 'stdout.txt').write_bytes(stdout)
    (OUT / 'stderr.txt').write_bytes(stderr)
    result['process_exited'] = process.poll() is not None
    (OUT / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
print(json.dumps(result))
