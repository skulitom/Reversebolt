"""Audit retained process-scope evidence; no desktop interaction or inference."""
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
RAW = ROOT/'test-results/memory-sources-native-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
manifest = read(RAW/'manifest.json')
for name, digest in manifest['method_sha256'].items():
    assert sha(HERE/name) == digest
assert sha(RAW/'binaries/Reversebolt.exe') == manifest['binary_sha256']
assert sha(RAW/'binaries/reversebolt-mcp.exe') == manifest['mcp_sha256']
ready = read(RAW/'ready.json')
assert ready['captures'] == 12 and ready['owned_arena_matches'] == 6 and ready['writes'] == 0
for flag in ['stale_identity_rejected', 'unrelated_process_rejected', 'partial_identity_rejected',
             'source_switch_starts_fresh', 'child_exit_invalidates']:
    assert ready[flag]
state = read(RAW/'child-state.json'); live = state['state']['live']
assert live['process']['pid'] == ready['fixture_pid'] != live['memory_process']['pid'] == ready['child_pid']
assert live['sample_count'] == len(live['frames']) == 12
assert not state['state']['changes'] and not state['state']['patches']
workers = read(RAW/'workers.json')
base, end = workers[0]['base'], workers[0]['base']+workers[0]['length']
matches = [m for t in live['tracks'] for m in t['matches'] if base <= int(m['address'],16) < end]
assert len(matches) == 6
readings = read(RAW/'readings.json')
assert len(readings) == 12
for row, frame in zip(readings, live['frames'], strict=True):
    assert row['expected']['pid'] == ready['child_pid']
    assert sha(Path(row['latest']['screen']['path'])) == frame['sha256']
    assert any(int(r['address'],16) <= base < int(r['address'],16)+r['retained'] for r in row['latest']['memory_reads'])
commands = read(RAW/'commands.json')
assert [c['name'] for c in commands if not c['ok']] == ['attach_process']*3 + ['capture']
assert not any(c['name'] in ['apply_change', 'restore_change', 'set_value'] for c in commands)
for c in commands:
    if c['name'] == 'attach_process':
        assert set(c['args']) <= {'pid','created','window','budget_mib','auto_observe','memory_pid','memory_created'}
    else:
        assert c['args'] is None
assert not read(RAW/'exited-state.json')['state']['live']['connected']
main = read(RAW/'uia-main.json')['elements']
assert not any(e['name'] == 'Memory source' for e in main)
assert any(e['name'] == 'Inspect application' for e in main)
sources = read(RAW/'uia-sources.json')['elements']
assert any(e['name'] == f"Reading Reversebolt.exe (PID {ready['child_pid']})" for e in sources)
assert any(e['name'] == f"Observe Reversebolt.exe (PID {ready['child_pid']})" and not e['enabled'] for e in sources)
ui_state = read(RAW/'ui-automatic-state.json')
assert ui_state['state']['live']['process']['pid'] == ui_state['state']['live']['memory_process']['pid'] == ready['fixture_pid']
assert ui_state['state']['live']['investigation_id'] != read(RAW/'final-state.json')['state']['live']['investigation_id']
assert ui_state['state']['live']['sample_count'] == 16
assert [j['phase'] for j in ui_state['jobs']['history']['jobs'].values()] == ['Completed']
write_path = ROOT/'test-results/memory-source-write-final.json'
write = read(write_path)
assert write['window_process']['pid'] != write['memory_process']['pid'] == write['pid']
assert write['restored']
for name in ['write', 'restore']:
    assert write[name]['attempted'] and write[name]['transferred'] == 4 and write[name]['error'] is None
for name in ['identity_guard', 'mapping_guard', 'stale_guard']:
    assert not write[name]['attempted'] and write[name]['transferred'] == 0
delivery_path = ROOT/'test-results/memory-source-desktop-delivery.json'
delivery = read(delivery_path)
assert sha(Path(delivery['path'])) == delivery['sha256']
assert sha(Path(delivery['path']).parent/'reversebolt-mcp.exe') == delivery['mcp_sha256']
assert delivery['history_preserved'] and delivery['history_count'] == 8 and delivery['packaged_binary_and_model_hashes_verified']
assert delivery['observer_model'] == 'qwen3.8:latest'
smoke_path = ROOT/delivery['smoke_report']
smoke = read(smoke_path)
assert smoke['mcp_tools'] == 30 and smoke['shared_session'] == 'passed'
assert all(p['edit_restore'] == 'passed' for p in smoke['practice'])
tests = ROOT/'test-results/memory-source-tests-final.log'
counts = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed', tests.read_text(encoding='utf-8-sig'))
assert sum(int(p) for p, f in counts) == 136 and all(int(f) == 0 for p, f in counts)
files = [RAW/name for name in ['manifest.json', 'ready.json', 'sources.json', 'workers.json', 'child-state.json',
         'readings.json', 'commands.json', 'exited-state.json', 'final-state.json', 'ui-automatic-state.json',
         'uia-main.json', 'uia-inspect.json', 'uia-sources.json']]
files += [write_path, delivery_path, smoke_path, tests, ROOT/'test-results/memory-source-clippy-final.log',
          ROOT/'test-results/memory-source-build-final.log']
result = {'scope': manifest['scope'], 'native_candidate_sha256': manifest['binary_sha256'],
          'delivered_sha256': delivery['sha256'], 'native_captures': 12, 'owned_arena_hypotheses': 6,
          'ui_automatic_captures': 16, 'integration_memory_writes': 0, 'owned_native_trial_bytes': 4,
          'workspace_tests': 136, 'mcp_tools': 30, 'progress_percent': 26,
          'artifacts': {str(p.relative_to(ROOT)).replace('\\','/'):sha(p) for p in files}}
receipt = HERE/'evidence/v1.json'
if '--record' in sys.argv:
    assert not receipt.exists(), 'Do not replace frozen evidence.'
    receipt.parent.mkdir(exist_ok=True)
    receipt.write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
else:
    assert read(receipt) == result
print(json.dumps({'verified':True, 'workspace_tests':136, 'mcp_tools':30, 'progress_percent':26}))
