"""Check a staged headless app and stdio MCP without attaching any target."""
import json
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time

binary, output = map(lambda p: Path(p).resolve(), sys.argv[1:])
output.mkdir()
data = output / 'data'
processes = []
lines = queue.Queue()
seq = 0
app = mcp = None

def save(name, value):
    (output / (name + '.json')).write_text(json.dumps(value, indent=2), encoding='utf-8')

def rpc(method, params):
    global seq
    seq += 1
    mcp.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': seq, 'method': method, 'params': params}) + '\n')
    mcp.stdin.flush()
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        value = json.loads(lines.get(timeout=max(.1, deadline - time.monotonic())))
        if value.get('id') == seq:
            assert 'error' not in value, value
            return value['result']
    raise TimeoutError('Recover this request; do not restart it')

try:
    app = subprocess.Popen([str(binary / 'Reversebolt.exe'), '--headless', '--data-dir', str(data)],
        stdout=(output / 'app.stdout').open('x'), stderr=(output / 'app.stderr').open('x'),
        creationflags=subprocess.CREATE_NO_WINDOW)
    processes.append(app)
    save('app-process', {'pid': app.pid, 'path': str(binary / 'Reversebolt.exe')})
    deadline = time.monotonic() + 20
    while not (data / 'endpoint.json').exists():
        assert app.poll() is None, 'Headless app exited before becoming ready'
        assert time.monotonic() < deadline, 'Headless readiness deadline expired'
        time.sleep(.05)
    mcp = subprocess.Popen([str(binary / 'reversebolt-mcp.exe'), '--data-dir', str(data)],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=(output / 'mcp.stderr').open('x'),
        text=True, encoding='utf-8', creationflags=subprocess.CREATE_NO_WINDOW)
    processes.append(mcp)
    save('mcp-process', {'pid': mcp.pid})
    threading.Thread(target=lambda: [lines.put(line) for line in mcp.stdout], daemon=True).start()
    rpc('initialize', {'protocolVersion': '2025-11-25', 'capabilities': {},
        'clientInfo': {'name': 'investigation-inspection-smoke', 'version': '1'}})
    mcp.stdin.write(json.dumps({'jsonrpc': '2.0', 'method': 'notifications/initialized'}) + '\n')
    mcp.stdin.flush()
    status = rpc('tools/call', {'name': 'reversebolt_status', 'arguments': {}})
    assert not status.get('isError', False), status
    save('status', status)
    response = rpc('tools/call', {'name': 'reversebolt_inspect_runtime_investigation',
        'arguments': {'catalog_id': 'no-catalogue', 'review_id': 'no-review'}})
    save('inspection', response)
    assert response.get('isError') is True
    assert 'Connect an application first.' in json.dumps(response)
    # Native transport reaches the new read handler; no game is connected.
    save('result', {'passed': True, 'target_attached': False, 'model_calls': 0,
        'writes': 0, 'expected_missing_application_error': True})
    mcp.stdin.close()
    assert mcp.wait(timeout=15) == 0
    print('Staged app and MCP route passed; no target was attached.')
finally:
    for child in reversed(processes):
        if child.poll() is None:
            child.terminate()
            child.wait(timeout=15)
    save('cleanup', [{'pid': child.pid, 'exit_code': child.poll()} for child in processes])
