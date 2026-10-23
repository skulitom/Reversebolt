"""Audit retained evidence IDs, canonical expansion and packaged interfaces."""
import copy
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from jsonschema import Draft202012Validator as Validator

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/evidence-references-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
assert all(sha(OUT / n) == d for n, d in frozen['binaries'].items())
assert sha(OUT / 'schemas.json') == frozen['schemas_sha256']
schemas = read(OUT / 'schemas.json')
for name, digest in frozen['sources'].items():
    assert sha(OUT / 'source' / name) == digest
    if '--verify-current' in sys.argv:
        assert sha(ROOT / name) == digest, name
cases = []
for case in frozen['cases']:
    assert all(sha(Path(n)) == d for n, d in case['hashes'].items())
    directory = OUT / case['name']
    process = read(OUT / (case['name'] + '-process.json'))
    assert process['exit_code'] == 0
    receipt = read(directory / 'receipt.json')
    inputs = read(directory / 'input.json')
    baseline = read(Path(case['baseline']))
    assert inputs['source'] == case['source']
    assert receipt['model'] == frozen['model']['name'] and receipt['model_digest'] == frozen['model']['digest']
    assert not receipt['cancelled']
    naming = case['kind'] == 'naming'
    if naming:
        assert inputs['inventory'] == read(Path(case['prior_input']))['inventory']
        rows = receipt['selection']['rows']
        report = inputs['inventory']['report_sha256']
    else:
        observed = inputs['input']
        if case['variation'] == 'synthetic':
            assert inputs['synthetic'] and observed == read(Path(case['source']))['input']
        else:
            source = read(Path(case['source']) / 'receipt.json')
            expected = [copy.deepcopy(h['candidate']) for h in source['batch']['accepted']]
            if case['variation'] == 'opaque':
                for row in expected:
                    row.update(type_name='Container', field_name=f"F{row['id']:010x}", object_text=[])
            assert observed == {'report_sha256': source['report_sha256'], 'candidates': expected}
        report = observed['report_sha256']
        rows = [dict(r, id=i) for i, r in enumerate(observed['candidates'])]
    table = receipt['citation_evidence']
    assert table['policy'] == 'candidate_evidence_references_v1' and table['report_sha256'] == report
    expected = []
    for row in rows:
        for source, index, quote in [('field_name', None, row['field_name']), ('type_name', None, row['type_name'])] + [
                ('object_text', i, text) for i, text in enumerate(row['object_text'])]:
            expected.append({'id': len(expected), 'candidate': row['id'],
                'citation': {'source': source, 'index': index, 'quote': quote}})
    assert table['entries'] == expected
    stages = receipt['stages'] if naming else [receipt['stage']]
    count, binding_errors = 0, []
    for stage in stages:
        assert hashlib.sha256(stage['prompt'].encode()).hexdigest() == stage['prompt_sha256']
        response = stage['response']
        assert response['done'] and response['done_reason'] == 'stop'
        assert 0 < response['prompt_eval_count'] <= stage['options']['num_ctx'] - stage['options']['num_predict'] - 512
        parsed = json.loads(response['message']['content'])
        Validator(stage['schema']).validate(parsed)
        key = 'hypotheses' if 'hypotheses' in parsed else 'judgments' if 'judgments' in parsed else None
        if key is None:
            continue
        if naming:
            expected_schema = copy.deepcopy(schemas['referenced_hypotheses'])
            expected_schema['properties']['hypotheses']['maxItems'] = stage['schema']['properties']['hypotheses']['maxItems']
            assert stage['schema'] == expected_schema
        else:
            expected_schema = copy.deepcopy(schemas['relevance'])
            judgments = expected_schema['properties']['judgments']
            judgments['minItems'] = judgments['maxItems'] = len(rows)
            judgments['items']['properties']['candidate']['maximum'] = max(0, len(rows) - 1)
            assert stage['schema'] == expected_schema
        marker = 'Observed paths: ' if naming else 'Observed candidates: '
        printed = json.loads(stage['prompt'].rsplit(marker, 1)[1])
        for row in printed:
            source_row = next(r for r in rows if r['id'] == row['candidate'])
            own = [e for e in table['entries'] if e['candidate'] == row['candidate']]
            assert row['field_name'] == {'id': own[0]['id'], 'text': source_row['field_name']}
            assert row['type_name'] == {'id': own[1]['id'], 'text': source_row['type_name']}
            assert row['object_text'] == [{'id': e['id'], 'text': e['citation']['quote']} for e in own[2:]]
            if naming:
                assert row['observed_value'] == source_row['value']
            else:
                assert set(row) == {'candidate', 'field_name', 'type_name', 'object_text'}
        for proposal in parsed[key]:
            count += len(proposal['evidence_ids'])
            for identifier in proposal['evidence_ids']:
                if identifier >= len(expected) or expected[identifier]['candidate'] != proposal['candidate']:
                    binding_errors.append({'candidate': proposal['candidate'], 'evidence_id': identifier})
    checked = subprocess.run([str(OUT / 'review.exe'), 'check-naming' if naming else 'check', str(directory)],
        capture_output=True, text=True, encoding='utf-8', timeout=20, creationflags=subprocess.CREATE_NO_WINDOW)
    if receipt['error'] is None:
        assert checked.returncode == 0, checked.stderr
        assert not binding_errors
        confirmation = json.loads(checked.stdout)
        assert confirmation['checked'] and confirmation['model_calls'] == 0 and not confirmation['target_access']
    else:
        assert checked.returncode != 0
    summary = {'case': case['name'], 'model_calls': len(stages), 'evidence_references': count,
        'error': receipt['error'], 'binding_errors': binding_errors, 'baseline_error': baseline['error'],
        'receipt_sha256': sha(directory / 'receipt.json'), 'seconds': process['seconds']}
    if naming:
        batch = receipt['batch']
        summary.update(accepted=None if batch is None else len(batch['accepted']),
            rejected=None if batch is None else len(batch['rejected']),
            names=[] if batch is None else [h['proposal']['name'] for h in batch['accepted']],
            rejections=[] if batch is None else batch['rejected'])
    else:
        summary['assessment'] = receipt['assessment']
    cases.append(summary)

tests = (ROOT / 'test-results/evidence-references-tests.log').read_text(encoding='utf-8-sig')
assert 'FAILED' not in tests
count = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', tests)))
assert count >= 509
for name in ['clippy', 'build', 'review-build']:
    log = (ROOT / f'test-results/evidence-references-{name}.log').read_text(encoding='utf-8-sig')
    assert 'Finished' in log and 'error:' not in log
summary = {'artifact_audit_passed': True, 'workspace_tests': count, 'cases': cases,
    'progress_percent': 38, 'frozen_manifest_sha256': sha(OUT / 'frozen.json'),
    'limitations': ['Valid references do not prove meaning, scope, liveness, safe values or effects.',
        'Known saved reports and authored cases; no independent-game evidence.',
        'Relevance remains research-only and never hides production candidates.']}
package_file = OUT / 'package.json'
if package_file.exists():
    package = read(package_file)
    preview = read(Path(package['headless']) / 'result.json')
    catalog = read(Path(package['catalog']) / 'result.json')
    assert preview['passed'] and preview['process_exited'] and not preview['target_access']
    assert catalog['passed'] and catalog['exit_code'] == 0 and catalog['tool_count'] == 65
    binary_directory = Path(preview['bin'])
    assert sha(binary_directory / 'Reversebolt.exe') == preview['binary_sha256']
    assert sha(binary_directory / 'reversebolt-mcp.exe') == catalog['binary_sha256']
    assert sha(Path(package['catalog']) / 'tools.json') == catalog['tools_sha256']
    tools = read(Path(package['catalog']) / 'tools.json')['tools']
    submission = next(t for t in tools if t['name'] == 'reversebolt_submit_runtime_hypotheses')
    assert submission['inputSchema']['properties']['hypotheses'] == schemas['hypotheses']['properties']['hypotheses']
    native = subprocess.run([sys.executable, '-X', 'utf8', str(ROOT / 'research/runtime-metadata-scaling/audit.py'), package['native']],
        capture_output=True, text=True, encoding='utf-8', timeout=30, creationflags=subprocess.CREATE_NO_WINDOW)
    assert native.returncode == 0, native.stderr
    native = json.loads(native.stdout)
    assert native['artifact_audit'] == 'passed' and native['helper_sha256'] == sha(binary_directory / 'reversebolt_runtime.dll')
    assert native['native_calls'] == 51 and native['writes_restored'] == native['readonly_writes_refused'] == 4
    summary.update(preview=binary_directory.name, mcp_tools=65, manual_submission_contract_preserved=True,
        headless_startup_passed=True, native_fixture_audit=native)
Path(__file__).with_name('results.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'audit_passed': True, 'tests': count, 'preview': summary.get('preview'),
    'cases': [{k: c.get(k) for k in ['case', 'error', 'model_calls', 'evidence_references', 'accepted', 'rejected', 'names']} for c in cases]}))
