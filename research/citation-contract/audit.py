"""Check revised schemas, fresh outputs, retained sources and packaged interfaces."""
import copy
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from jsonschema import Draft202012Validator as Validator

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/citation-contract-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
assert all(sha(OUT / name) == digest for name, digest in frozen['binaries'].items())
assert sha(OUT / 'schemas.json') == frozen['schemas_sha256']
schemas = read(OUT / 'schemas.json')
for name, digest in frozen['sources'].items():
    assert sha(OUT / 'source' / name) == digest
    if '--verify-current' in sys.argv:
        assert sha(ROOT / name) == digest
schema_check = read(OUT / 'schema-check.json')
assert schema_check['passed'] and schema_check['schemas_sha256'] == frozen['schemas_sha256']
for source in schema_check['historical_receipts']:
    assert sha(Path(source['path'])) == source['sha256']
cases = []
for case in frozen['cases']:
    assert all(sha(Path(name)) == digest for name, digest in case['hashes'].items())
    directory = OUT / case['name']
    process = read(OUT / (case['name'] + '-process.json'))
    assert process['exit_code'] == 0
    receipt = read(directory / 'receipt.json')
    baseline = read(Path(case['baseline']))
    inputs = read(directory / 'input.json')
    assert inputs['source'] == case['source']
    assert receipt['model'] == frozen['model']['name'] and receipt['model_digest'] == frozen['model']['digest']
    assert not receipt['cancelled']
    stages = receipt['stages'] if case['kind'] == 'naming' else [receipt['stage']]
    citation_count = 0
    for stage in stages:
        assert stage is not None
        assert hashlib.sha256(stage['prompt'].encode()).hexdigest() == stage['prompt_sha256']
        response = stage['response']
        assert response['done'] and response['done_reason'] == 'stop'
        assert 0 < response['prompt_eval_count'] <= stage['options']['num_ctx'] - stage['options']['num_predict'] - 512
        parsed = json.loads(response['message']['content'])
        Validator(stage['schema']).validate(parsed)
        if 'hypotheses' in parsed:
            expected = copy.deepcopy(schemas['hypotheses'])
            expected['properties']['hypotheses']['maxItems'] = stage['schema']['properties']['hypotheses']['maxItems']
            assert stage['schema'] == expected
            citation_count += sum(len(h['citations']) for h in parsed['hypotheses'])
        elif 'judgments' in parsed:
            assert stage['schema']['properties']['judgments']['items']['properties']['citations']['items'] == schemas['relevance_citation']
            citation_count += sum(len(j['citations']) for j in parsed['judgments'])
    summary = {'case': case['name'], 'error': receipt['error'], 'model_calls': len(stages),
               'citation_count': citation_count, 'baseline_error': baseline['error'],
               'seconds': process['seconds'], 'receipt_sha256': sha(directory / 'receipt.json')}
    if case['kind'] == 'review':
        if case['variation'] == 'synthetic':
            assert inputs['synthetic'] and inputs['input'] == read(Path(case['source']))['input']
        else:
            original = read(Path(case['source']) / 'receipt.json')
            assert inputs['input']['report_sha256'] == original['report_sha256']
            assert inputs['input']['candidates'] == [h['candidate'] for h in original['batch']['accepted']]
        if receipt['assessment'] is not None:
            check = subprocess.run([str(OUT / 'review.exe'), 'check', str(directory)], capture_output=True,
                text=True, encoding='utf-8', timeout=15, creationflags=subprocess.CREATE_NO_WINDOW)
            assert check.returncode == 0, check.stderr
            checked = json.loads(check.stdout)
            assert checked['checked'] and checked['model_calls'] == 0 and not checked['target_access']
            assert checked['assessment'] == receipt['assessment']
        summary['assessment'] = receipt['assessment']
        summary['binding_errors'] = []
        if receipt['assessment'] is None:
            for judgment in json.loads(receipt['stage']['response']['message']['content'])['judgments']:
                row = inputs['input']['candidates'][judgment['candidate']]
                for citation in judgment['citations']:
                    observed = row.get(citation['source']) if citation['index'] is None else (
                        row['object_text'][citation['index']] if citation['source'] == 'object_text'
                        and citation['index'] < len(row['object_text']) else None)
                    if observed != citation['quote']:
                        summary['binding_errors'].append({'candidate': judgment['candidate'], 'field': row['field_name'],
                            'citation': citation, 'observed': observed})
    else:
        assert inputs['inventory'] == read(Path(case['prior_input']))['inventory']
        batch = receipt['batch']
        assert receipt['report_sha256'] == inputs['inventory']['report_sha256']
        summary['accepted'] = None if batch is None else len(batch['accepted'])
        summary['rejected'] = None if batch is None else len(batch['rejected'])
        summary['baseline_accepted'] = None if baseline['batch'] is None else len(baseline['batch']['accepted'])
        summary['baseline_rejected'] = None if baseline['batch'] is None else len(baseline['batch']['rejected'])
        summary['names'] = [] if batch is None else [h['proposal']['name'] for h in batch['accepted']]
        summary['rejections'] = [] if batch is None else batch['rejected']
        if batch is not None:
            assert batch['report_sha256'] == receipt['selection']['report_sha256'] == receipt['report_sha256']
            for hypothesis in batch['accepted']:
                assert hypothesis['candidate'] in receipt['selection']['rows']
                assert all(a in receipt['selection']['rows'] for a in hypothesis['alternatives'])
    cases.append(summary)
tests = (ROOT / 'test-results/citation-contract-tests.log').read_text(encoding='utf-8-sig')
assert 'FAILED' not in tests
count = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', tests)))
assert count == 504
for name in ['clippy', 'build', 'review-build']:
    log = (ROOT / f'test-results/citation-contract-{name}.log').read_text(encoding='utf-8-sig')
    assert 'Finished' in log and 'error:' not in log
package = read(OUT / 'package.json')
preview = read(Path(package['headless']) / 'result.json')
catalog = read(Path(package['catalog']) / 'result.json')
assert preview['passed'] and preview['process_exited'] and not preview['target_access']
assert catalog['passed'] and catalog['exit_code'] == 0 and catalog['tool_count'] == 65
binary_directory = Path(preview['bin'])
assert sha(binary_directory / 'Reversebolt.exe') == preview['binary_sha256']
assert sha(binary_directory / 'reversebolt-mcp.exe') == catalog['binary_sha256']
tools = read(Path(package['catalog']) / 'tools.json')['tools']
assert sha(Path(package['catalog']) / 'tools.json') == catalog['tools_sha256']
submission = next(t for t in tools if t['name'] == 'reversebolt_submit_runtime_hypotheses')
assert submission['inputSchema']['properties']['hypotheses'] == schemas['hypotheses']['properties']['hypotheses']
helper_hash = sha(binary_directory / 'reversebolt_runtime.dll')
prior_helper_hash = sha(ROOT / 'dist/0.1.0-20261023-081315/reversebolt_runtime.dll')
native_check = subprocess.run([sys.executable, '-X', 'utf8', str(ROOT / 'research/runtime-metadata-scaling/audit.py'), package['native']],
    capture_output=True, text=True, encoding='utf-8', timeout=30, creationflags=subprocess.CREATE_NO_WINDOW)
assert native_check.returncode == 0, native_check.stderr
native = read(Path(package['native']) / 'audit.json')
assert json.loads(native_check.stdout) == native
assert native['artifact_audit'] == 'passed' and native['helper_sha256'] == helper_hash
assert native['native_calls'] == 51 and native['writes_restored'] == native['readonly_writes_refused'] == 4
summary = {'artifact_audit_passed': True, 'workspace_tests': count, 'schema_check': schema_check,
    'cases': cases, 'preview': binary_directory.name, 'mcp_tools': catalog['tool_count'],
    'mcp_uses_shared_schema': True, 'headless_startup_passed': True,
    'native_helper_sha256': helper_hash, 'native_helper_unchanged': helper_hash == prior_helper_hash,
    'native_fixture_audit': native,
    'progress_percent': 38, 'frozen_manifest_sha256': sha(OUT / 'frozen.json'),
    'limitations': ['Structure does not verify semantic meaning, scope, liveness, safe edits or effects.',
        'Known saved reports and an authored failed batch; no independent-game evidence.',
        'Relevance remains research-only and never hides production candidates.',
        'No gameplay process, input, screenshot or game-memory operation was used; native fixture edits were restored.']}
Path(__file__).with_name('results.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'audit_passed': True, 'tests': count, 'preview': summary['preview'], 'helper_unchanged': summary['native_helper_unchanged'],
    'cases': [{k: c.get(k) for k in ['case', 'error', 'model_calls', 'citation_count', 'accepted', 'rejected']} for c in cases]}))
