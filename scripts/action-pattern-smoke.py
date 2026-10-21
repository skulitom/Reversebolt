"""Owned Anode fixture matrix for automatic input/value contrasts; no game control.

OCR numbers are read by the production pipeline. Boolean observations are explicitly
fixture-supplied annotations, testing the shared value contract, not model understanding.
"""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import queue
import subprocess
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
HIDDEN = 0x08000000


def read(path):
    try:
        return json.loads(path.read_text(encoding='utf-8-sig'))
    except (FileNotFoundError, PermissionError, json.JSONDecodeError):
        return None


def wait(check, timeout=120):
    deadline = time.monotonic()+timeout
    while time.monotonic() < deadline:
        value = check()
        if value:
            return value
        time.sleep(.025)
    raise AssertionError('Fixture wait expired; preserve evidence, do not replay input.')


def run(args, mode):
    out = args.out/mode
    out.mkdir(parents=True, exist_ok=False)
    children = []
    seq = request = 0
    held = False
    fixture = engine = mcp = None
    trace = []
    control_file, state_file = out/'control.json', out/'fixture.json'

    def control(action):
        nonlocal seq, held
        seq += 1
        temporary = control_file.with_suffix('.new')
        temporary.write_text(json.dumps({'seq': seq, 'action': action, 'key': 68}))
        temporary.replace(control_file)
        state = wait(lambda: (s if (s := read(state_file)) and (s['ack'] == seq or s['error']) else None), 10)
        assert not state['error'], state
        if action == 'down': held = True
        if action == 'up': held = False
        return state

    def rpc(method, params):
        nonlocal request
        request += 1
        mcp.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': request, 'method': method, 'params': params})+'\n')
        mcp.stdin.flush()
        while True:
            value = json.loads(lines.get(timeout=120))
            if value.get('id') == request:
                assert 'error' not in value, value
                return value['result']

    def call(action, arguments=None):
        result = rpc('tools/call', {'name': 'reversebolt_'+action, 'arguments': arguments or {}})
        assert not result.get('isError'), result
        value = json.loads(result['content'][-1]['text'])
        assert value['ok'], value.get('error')
        return value

    try:
        fixture = subprocess.Popen(['powershell', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
            str(ROOT/'scripts/action-pattern-fixture.ps1'), '-ControlFile', str(control_file),
            '-StateFile', str(state_file), '-Title', 'Reversebolt action '+mode, '-Mode', mode], creationflags=HIDDEN)
        children.append(fixture)
        wait(lambda: read(state_file), 30)
        data = out/'engine'
        engine = subprocess.Popen([str(args.bin/'Reversebolt.exe'), '--data-dir', str(data)]+([] if args.visible else ['--headless']), creationflags=HIDDEN)
        children.append(engine)
        wait(lambda: (e if (e := read(data/'endpoint.json')) and e['pid'] == engine.pid else None), 30)
        mcp = subprocess.Popen([str(args.bin/'reversebolt-mcp.exe'), '--data-dir', str(data)],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding='utf-8', creationflags=HIDDEN)
        children.append(mcp)
        lines = queue.Queue()
        threading.Thread(target=lambda: [lines.put(line) for line in mcp.stdout], daemon=True).start()
        rpc('initialize', {'protocolVersion': '2025-11-25', 'capabilities': {},
            'clientInfo': {'name': 'action-pattern-fixture', 'version': '1'}})
        mcp.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n'); mcp.stdin.flush()
        assert len(rpc('tools/list', {})['tools']) == 29
        anode = Path(os.environ['LOCALAPPDATA'])/'Programs/Anode/anode.exe'
        windows = json.loads(subprocess.run([str(anode), 'windows', '--pid', str(fixture.pid), '--json'], capture_output=True, text=True, check=True, timeout=20).stdout)['windows']
        window = next(w for w in windows if w['title'] == 'Reversebolt action '+mode)
        subprocess.run([str(anode), 'window', window['windowId'], 'focus'], capture_output=True, check=True, timeout=20)
        target = next(p for p in call('list_processes')['state']['processes'] if p['pid'] == fixture.pid)
        call('attach_process', {**{k: target[k] for k in ['pid', 'window', 'created']}, 'budget_mib': 1, 'auto_observe': False})
        history = call('status')['jobs']['history']
        started = call('start_observation_job', {'request_id': history['clients'].get('mcp-agent', {}).get('high_water', 0)+1,
            'boot': history['boot'], 'binding': history['current'], 'count': 12, 'interval_ms': 600,
            'mode': 'full', 'compare': False})
        job = started['data']['ack']['job']
        for n in range(1, 13):
            def captured():
                state = call('status')
                assert state['jobs']['history']['jobs'][str(job)]['phase'] not in ['Failed', 'Interrupted', 'Cancelled'], state
                return state if state['state']['live']['sample_count'] >= n else None
            status = wait(captured)
            live = status['state']['live']
            assert live['sample_count'] == n, 'Missed a capture; do not alter the test schedule.'
            frame = live['frames'][-1]
            fixture_state = read(state_file)
            trace.append({'capture': n, 'fixture': fixture_state, 'live': live})
            # The fixture's bit is intentionally declared, not inferred from its image.
            call('submit_observation', {'investigation_id': live['investigation_id'], 'frame_id': frame['id'],
                'image_sha256': frame['sha256'], 'observer': 'owned-fixture-oracle', 'key': 'fixture-bit',
                'label': 'Fixture bit', 'unit': '', 'value': {'kind': 'boolean', 'value': fixture_state['flag']},
                'quality': 'provisional', 'bounds': None, 'explanation': 'Test-only fixture annotation; not a visual model result.'})
            if n in [2, 5, 8]: control('down')
            if n in [3, 6, 9]:
                time.sleep(.12)  # Ensure the held tail is sampled; never invent a release boundary.
                control('up')
        status = call('status')
        live = status['state']['live']
        final_fixture = read(state_file)
        assert final_fixture['down'].get('68') == 3, final_fixture
        expected = {'coupled': 'tentative', 'none': 'no_pattern', 'conditional': 'contradicted', 'drift': 'no_contrast'}[mode]
        number_track = next(t for t in live['tracks'] if t['active'] and any(v is not None and 300 <= v <= 500 for v in t['samples']))
        numeric = next(e for e in live['action_patterns'] if e['source'] == 'ocr' and e['track'] == number_track['id'])
        pattern = next(p for p in numeric['patterns'] if p['key'] == 68)
        assert pattern['state'] == expected, {'expected': expected, 'evidence': numeric, 'track': number_track}
        assert len(pattern['first_episodes']) == 2
        if mode in ['coupled', 'drift']: assert len(pattern['later_agree']) == 1
        if mode == 'conditional': assert len(pattern['later_disagree']) == 1
        external = next(e for e in live['action_patterns'] if e['source'] == 'external')
        bit_pattern = next(p for p in external['patterns'] if p['key'] == 68)
        assert bit_pattern['state'] == (expected if mode != 'drift' else 'no_pattern'), bit_pattern
        archive = next(data.glob('captures/*/evidence.json'))
        assert read(archive)['action_patterns'] == live['action_patterns']
        assert read(data/'session.json')['live']['action_patterns'] == live['action_patterns']
        report = {'passed': True, 'mode': mode, 'expected': expected, 'engine_pid': engine.pid, 'fixture_pid': fixture.pid,
            'numeric_track': number_track, 'numeric_evidence': numeric, 'declared_boolean_evidence': external,
            'archive_sha256': hashlib.sha256(archive.read_bytes()).hexdigest(), 'trace': trace,
            'binary_sha256': {n: hashlib.sha256((args.bin/n).read_bytes()).hexdigest() for n in ['Reversebolt.exe', 'reversebolt-mcp.exe']}}
        (out/'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps({'mode': mode, 'passed': True, 'numeric': pattern, 'samples': number_track['samples']}), flush=True)
        if args.keep_ui:
            (out/'ui-ready.json').write_text(json.dumps({'engine_pid': engine.pid, 'data': str(data)}))
            wait(lambda: (out/'close-ui').exists(), 300)
        engine.terminate(); engine.wait(timeout=10)
        engine = subprocess.Popen([str(args.bin/'Reversebolt.exe'), '--headless', '--data-dir', str(data)], creationflags=HIDDEN)
        children.append(engine)
        wait(lambda: (e if (e := read(data/'endpoint.json')) and e['pid'] == engine.pid else None), 30)
        assert call('status')['state']['live'] is None
        assert hashlib.sha256(archive.read_bytes()).hexdigest() == report['archive_sha256']
        report['restart_invalidates_live_patterns_preserves_archive'] = True
        (out/'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    except Exception:
        (out/'failed-trace.json').write_text(json.dumps(trace, indent=2), encoding='utf-8')
        raise
    finally:
        if held:
            try: control('up')
            except Exception: pass
        if fixture and fixture.poll() is None:
            try: control('close')
            except Exception: pass
        for child in reversed(children):
            if child.poll() is None: child.terminate()
            try: child.wait(timeout=5)
            except subprocess.TimeoutExpired: child.kill(); child.wait()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', required=True, type=Path)
    parser.add_argument('--bin', required=True, type=Path)
    parser.add_argument('--modes', nargs='+', default=['coupled', 'none', 'conditional', 'drift'], choices=['coupled', 'none', 'conditional', 'drift'])
    parser.add_argument('--visible', action='store_true')
    parser.add_argument('--keep-ui', action='store_true')
    args = parser.parse_args()
    args.out = args.out.resolve(); args.bin = args.bin.resolve()
    session = ctypes.c_ulong()
    assert ctypes.windll.kernel32.ProcessIdToSessionId(os.getpid(), ctypes.byref(session))
    assert session.value == int(os.environ['REVERSEBOLT_TEST_SESSION'])
    assert session.value != int(os.environ['REVERSEBOLT_PARENT_SESSION'])
    for mode in args.modes: run(args, mode)
