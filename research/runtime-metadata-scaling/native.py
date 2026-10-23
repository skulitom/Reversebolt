"""Owned ABI regression matrix; no game, desktop input or model calls."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()

def save(path, value):
    with path.open('x', encoding='utf-8') as stream:
        json.dump(value, stream, indent=2)

def worker(out, helper, fixture, count, root):
    out.mkdir()
    control = out / 'mode.txt'
    control.write_text('0')
    os.environ['REVERSEBOLT_VALUE_FIXTURE_CONTROL'] = str(control)
    mock = ctypes.CDLL(str(fixture))
    mock.MockMetadata.argtypes = [ctypes.c_uint32, ctypes.c_bool]
    mock.MockBalance.restype = ctypes.c_uint64
    mock.MockMetadata(count, root)
    mock.MockTick()
    library = ctypes.WinDLL(str(helper))
    library.ReverseboltInspect.argtypes = [ctypes.c_void_p]
    library.ReverseboltInspect.restype = ctypes.c_uint32
    calls, pauses = [], []

    def inspect(label, **args):
        output = out / (label + '.json')
        request = ctypes.create_unicode_buffer(json.dumps({'output': str(output), **args}))
        assert library.ReverseboltInspect(ctypes.cast(request, ctypes.c_void_p)) == 0
        report = read(output)
        shots = report.get('pauses', report.get('snapshots', []))
        shots += [w['discarded_snapshot'] for w in report.get('warmups', [])]
        assert all(s['helper_allocations_while_paused'] == s['helper_deallocations_while_paused'] == 0 for s in shots)
        pauses.extend(shots)
        calls.append({'label': label, 'sha256': sha(output), 'pauses': len(shots)})
        return report

    def cli(name, *args):
        result = subprocess.run([str(out.parent / 'bin' / name), *map(str, args)],
                                capture_output=True, text=True, timeout=30,
                                creationflags=subprocess.CREATE_NO_WINDOW)
        assert result.returncode == 0, (name, result.stderr)

    report = inspect('discovery')
    candidates = []
    if 'error' not in report:
        cli('reversebolt-runtime-candidates.exe', out / 'discovery.json', out / 'candidates.json', 4096)
        candidates = read(out / 'candidates.json')
        assert candidates and all(not c['edit_authority'] for c in candidates)
        def pick(ids, root_id=0, element=None):
            values = [c for c in candidates if c['locator']['root']['field_id'] == root_id
                      and [s['member'].get('field', {}).get('field_id') for s in c['locator']['steps']] == ids
                      and (element is None or any(s['member'] == {'kind': 'element', 'index': element} for s in c['locator']['steps']))]
            assert len(values) == 1
            return values[0]
        pose, readonly = pick([0, 0]), pick([1, 0])
        array, phase, wide = pick([2, None, 0], element=1), pick([3]), pick([4])
        selected = [pose, readonly, array, phase, wide, pick([5, 0, 0]), pick([0], 1)]
        refresh = inspect('refresh', paths=[c['locator'] for c in selected])
        assert all(r['status'] == 'read' and r['bytes'] == c['bytes'] for r, c in zip(refresh['checks'], selected, strict=True))
        assert pose['value'] == '3.25' and array['value'] == '12.25' and phase['value'] == 7
        assert wide['value'] == str(2**64-3)
        for candidate, label, allowed in [(pose, 'edit', True), (readonly, 'readonly', False)]:
            locator = out / (label + '.locator.json')
            save(locator, candidate['locator'])
            before = inspect(label + '-before', paths=[candidate['locator']])
            patch = out / (label + '.patch.json')
            cli('reversebolt-runtime-patch.exe', 'prepare', locator, out / (label + '-before.json'), patch, '4.25')
            changed = inspect(label + '-write', patch=read(patch))
            if allowed:
                assert changed['patch']['state'] == 'written'
                inverse = out / (label + '.restore.json')
                cli('reversebolt-runtime-patch.exe', 'restore', patch, out / (label + '-write.json'), inverse)
                assert inspect(label + '-restore', patch=read(inverse))['patch']['state'] == 'written'
            else:
                assert changed['patch']['reason'] == 'readonly_field' and not changed['patch']['attempted']
            after = inspect(label + '-after', paths=[candidate['locator']])
            assert after['checks'][0]['bytes'] == before['checks'][0]['bytes']
        control.write_text('1'); mock.MockTick()
        assert inspect('null', paths=[pose['locator']])['checks'][0]['status'] == 'null_reference'
        control.write_text('3'); mock.MockTick()
        assert inspect('layout', paths=[pose['locator']])['checks'][0]['status'] == 'type_changed'
        control.write_text('0'); mock.MockTick()
        final = inspect('restored', paths=[c['locator'] for c in selected])
        assert all(r['bytes'] == c['bytes'] for r, c in zip(final['checks'], selected, strict=True))
    balance = mock.MockBalance()
    assert balance >> 32 == balance & 0xffffffff == len(pauses)
    save(out / 'result.json', {'error': report.get('error'), 'metadata': report.get('metadata'),
                             'calls': calls, 'pauses': len(pauses), 'candidates': len(candidates),
                             'mock_stops': balance >> 32, 'mock_resumes': balance & 0xffffffff,
                             'root_fingerprint': report.get('root_catalog_sha256'),
                             'max_stop_to_resume_us': max((p['stop_to_resume_us'] for p in pauses), default=0),
                             'restored': bool(candidates), 'readonly_refused': bool(candidates)})

def matrix(out, binary_directory):
    out.mkdir()
    (out / 'bin').mkdir()
    binaries = {'reversebolt_runtime.dll': binary_directory / 'reversebolt_runtime.dll'}
    binaries.update({name: ROOT / 'target/debug' / name for name in
                     ['reversebolt-runtime-candidates.exe', 'reversebolt-runtime-patch.exe']})
    binaries['old-runtime.dll'] = ROOT / 'dist/0.1.0-20261023-070135/reversebolt_runtime.dll'
    binaries['fixture.dll'] = ROOT / 'research/runtime-values/mock/target/release/reversebolt_value_fixture.dll'
    hashes = {}
    for name, source in binaries.items():
        shutil.copy2(source, out / 'bin' / name); hashes[name] = sha(out / 'bin' / name)
    files = list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml'))
    files += [ROOT / p for p in ['Cargo.toml', 'Cargo.lock', 'research/runtime-values/mock/Cargo.toml',
              'research/runtime-values/mock/src/lib.rs', 'research/runtime-metadata-scaling/native.py',
              'research/runtime-metadata-scaling/PROTOCOL.md']]
    sources = {}
    for path in files:
        name = path.relative_to(ROOT).as_posix(); dest = out / 'source' / name
        dest.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(path, dest); sources[name] = sha(dest)
    save(out / 'frozen.json', {'binaries': hashes, 'sources': sources,
         'base_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()})
    results = {}
    def case(name, count, root=False, old=False):
        helper = out / 'bin' / ('old-runtime.dll' if old else 'reversebolt_runtime.dll')
        run = subprocess.run([sys.executable, '-X', 'utf8', str(Path(__file__).resolve()), '--worker',
                              str(out / name), str(helper), str(out / 'bin/fixture.dll'), str(count), str(int(root))],
                             capture_output=True, text=True, timeout=120, creationflags=subprocess.CREATE_NO_WINDOW)
        save(out / (name + '-execution.json'), {'exit_code': run.returncode, 'stdout': run.stdout, 'stderr': run.stderr})
        assert run.returncode == 0, (name, run.stderr)
        result = read(out / name / 'result.json'); results[name] = result
        print(json.dumps({'case': name, 'error': result['error'], 'candidates': result['candidates']}), flush=True)
        return result
    old = case('old-small', 0, old=True)
    assert case('old-large', 200001, old=True)['error'] == 'field metadata budget'
    small = case('new-small', 0)
    large = case('new-large', 200001)
    assert old['error'] is small['error'] is large['error'] is None
    overhead = small['metadata']['root_inventory']['field_headers_examined']
    exact = case('new-exact-header-limit', 2000000 - overhead)
    assert exact['error'] is None and exact['metadata']['root_inventory']['field_headers_examined'] == 2000000
    over = case('new-over-header-limit', 2000001 - overhead)
    assert over['error'] == 'root field header budget' and not over['metadata']['root_inventory']['complete']
    required = case('new-required-layout-limit', 200001, root=True)
    assert required['error'].startswith('field metadata budget:') and required['metadata']['loaded_fields'] == 200000
    def routes(name):
        return sorted(json.dumps({'root': c['locator']['root'], 'steps': c['locator']['steps'],
                                  'value': c['value'], 'bytes': c['bytes']}, sort_keys=True)
                      for c in read(out / name / 'candidates.json'))
    assert routes('old-small') == routes('new-small') == routes('new-large') == routes('new-exact-header-limit')
    assert old['root_fingerprint'] == small['root_fingerprint'] == large['root_fingerprint'] == exact['root_fingerprint']
    assert all(sha(out / 'bin' / n) == h for n, h in hashes.items())
    save(out / 'matrix.json', {'passed': True, 'cases': results, 'equal_routes_values_and_root_fingerprints': True,
                             'limitation': 'Owned mock ABI via direct helper calls; not real-game coverage or remote loader unload evidence.'})

if __name__ == '__main__':
    if sys.argv[1] == '--worker':
        worker(Path(sys.argv[2]), Path(sys.argv[3]), Path(sys.argv[4]), int(sys.argv[5]), bool(int(sys.argv[6])))
    else:
        matrix(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else ROOT / 'target/debug')
