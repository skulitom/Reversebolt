"""Audit retained experiments and the checked Rust inventory, without inference."""
import hashlib
import json
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(sys.argv[1]).resolve()
OUT.mkdir(parents=True, exist_ok=False)
EXE = ROOT / 'target/debug/reversebolt-runtime-inventory.exe'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


cases = [('v1', 'manifold'), ('v1', 'blue-prince-menu'), ('v2', 'manifold'),
         ('v3', 'manifold'), ('v3', 'blue-prince-menu'), ('v3', 'manifold-opaque')]
runs = []
for version, case in cases:
    directory = ROOT / 'test-results' / ('runtime-naming-' + version) / case
    result = read(directory / 'result.json')
    source = read(directory / 'source.json')
    assert sha(Path(source['path'])) == source['sha256']
    stages = []
    for stage in ['types', 'controls']:
        request = read(directory / (stage + '-request.json'))
        response = read(directory / (stage + '-response.json'))
        receipt = read(directory / (stage + '-receipt.json'))
        assert response['done'] and response['done_reason'] == 'stop'
        assert response['message']['role'] == 'assistant'
        prompt = request['messages'][0]['content'].encode()
        assert hashlib.sha256(prompt).hexdigest() == receipt['prompt_sha256']
        assert json.loads(response['message']['content']) == read(directory / (stage + '-parsed.json'))
        stages.append({'name': stage, **receipt})
    runs.append({'version': version, 'case': case, 'source': source,
                 'structurally_accepted': len(result['accepted']),
                 'structurally_rejected': len(result['rejected']),
                 'result_sha256': sha(directory / 'result.json'), 'stages': stages})

inputs = {
    'manifold': ROOT / 'test-results/manifold-runtime-v1/data/runtime/69972-01dd618378b76b89/e16ab151-1999-4b16-af71-963def104364/report.json',
    'menu': ROOT / 'test-results/runtime-context-v1/game/data/runtime/86048-01dd6170d2e43e52/1d678759-872f-442c-add2-a258b225a26e/report.json',
    'fixture': ROOT / 'test-results/runtime-context-v1/fixture/data/runtime/84844-01dd61701463aed6/dbfeabc6-e0aa-46c0-a7c6-87044d6f98a2/report.json',
}
inventories = []
for case, source in inputs.items():
    path = OUT / (case + '-inventory.json')
    start = time.monotonic()
    process = subprocess.run([str(EXE), str(source), str(path)], capture_output=True, text=True, timeout=120)
    assert process.returncode == 0, process.stderr
    inventory = read(path)
    assert inventory['schema'] == 1
    assert len(inventory['report_sha256']) == 64
    groups = inventory['types']
    assert [t['id'] for t in groups] == list(range(len(groups)))
    fields = [f for t in groups for f in t['fields']]
    assert sum(f['path_count'] for f in fields) + inventory['omitted_core_paths'] + inventory['omitted_array_elements'] + inventory['omitted_non_numeric_paths'] == inventory['reachable_paths']
    assert all(0 < len(f['values']) <= 3 and 0 < len(f['samples']) <= 4 for f in fields)
    assert all(len(f['samples']) <= f['path_count'] for f in fields)
    if case == 'manifold':
        controller = next(t for t in groups if t['key']['type_name'].split(',')[0] == 'RigidbodyController')
        members = {f['name']: f for f in controller['fields']}
        assert members['walkSpeed']['field_id'] != members['moveSpeed']['field_id']
        assert any(s['path_id'] == 'ef12ce1f687138cd5c7ddd4348d6ea79e490a857c67bd769bc05c0d46c00495a' for s in members['walkSpeed']['samples'])
    if case == 'fixture':
        coefficient = next(f for f in fields if f['name'] == 'Coefficient')
        assert any(h['text'] == 'Speed' and not h['truncated'] for s in coefficient['samples'] for h in s['context'])
        assert any(h['truncated'] for s in coefficient['samples'] for h in s['context'])
    inventories.append({'case': case, 'source_sha256': sha(source), 'output_sha256': sha(path),
                        'report_sha256': inventory['report_sha256'], 'types': len(groups),
                        'fields': len(fields), 'reachable_paths': inventory['reachable_paths'],
                        'omitted_core_paths': inventory['omitted_core_paths'],
                        'omitted_array_elements': inventory['omitted_array_elements'],
                        'omitted_non_numeric_paths': inventory['omitted_non_numeric_paths'],
                        'elapsed_seconds': time.monotonic() - start, 'exit_code': process.returncode})

summary = {'schema': 1, 'model_runs': runs, 'inventories': inventories,
           'inventory_executable_sha256': sha(EXE),
           'inventory_source_sha256': sha(ROOT / 'crates/reversebolt-core/src/runtime_inventory.rs'),
           'limitation': 'Structural acceptance is not semantic correctness. Model-only naming failed relevance and opaque-name abstention. No model proposals published; no live process access or writes.'}
(OUT / 'audit.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
print(json.dumps({'model_runs': len(runs), 'inventories': inventories, 'audit': str(OUT / 'audit.json')}))
