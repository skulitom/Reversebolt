"""Offline artifact checks; passing is not a gameplay/generalization verdict."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT/'test-results/independent-sable-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT/'frozen.json')
data = Path(frozen['data'])
assert sha(OUT/'product-parent-frozen.json') == frozen['product_parent_frozen_sha256']
parent = read(OUT/'product-parent-frozen.json')
assert frozen['binaries'] == parent['binaries']
assert frozen['source_commit'] == parent['source_commit']
for name, digest in frozen['sources'].items():
    assert sha(OUT/'source'/name) == digest, name
for name, digest in frozen['binaries'].items():
    assert sha(Path(frozen['bin'])/name) == digest, name
for name, digest in frozen['storage_before'].items():
    assert sha(OUT/'storage-before'/name) == digest, name
assert sha(OUT/'appmanifest-before.acf') == frozen['appmanifest_sha256']
setup = read(OUT/'setup.json')
assert setup['gameplay_observed'] and setup['setup_images_reviewed'] <= 4
assert setup['manually_reviewed_measurement_images'] == 0
assert not (OUT/'needs-recovery.json').exists()
result = read(OUT/'result.json')
assert result['workflow_finished']
assert result['manually_reviewed_measurement_images'] == 0
discovery = read(OUT/'discovery.json')
live = discovery['state']['live']
assert result['samples'] == len(live['frames']) == 15
assert [i['key'] for i in result['evaluator_inputs']] == [68,65,68,65]
assert all(i['planned_ms']==200 and i['end_ns']>i['begin_ns'] for i in result['evaluator_inputs'])
captures = [p for p in (data/'captures').iterdir() if (p/'frame-001.png').is_file()]
assert len(captures) == 1
for frame in live['frames']:
    assert sha(captures[0]/f"frame-{frame['number']:03}.png") == frame['sha256']

review = live['runtime'].get('naming')
receipt = inventory = None
page_counts = []
if review:
    assert review == result['review']
    directory = data/'runtime/naming'/review['evidence_id']
    receipt, inventory = read(directory/'receipt.json'), read(directory/'inventory.json')
    assert receipt['report_sha256'] == inventory['report_sha256'] == review['report_sha256']
    for stage in receipt.get('stages', []):
        assert hashlib.sha256(stage['prompt'].encode()).hexdigest() == stage['prompt_sha256']
    paged = receipt.get('paged_selection')
    if paged:
        original = {t['id']:t for t in inventory['types']}
        expected, saved_rows = set(original), {}
        for number, round_ in enumerate(paged['rounds']):
            seen, selected = set(), set()
            page_counts.append(len(round_['pages']))
            for page in round_['pages']:
                stage = receipt['stages'][page['stage_index']]
                assert len(stage['prompt'].encode()) <= 12000
                rows = json.loads(stage['prompt'].split('\nObserved types and path-scoped text examples: ',1)[1])['types']
                assert [r['id'] for r in rows] == page['type_ids']
                for row in rows:
                    i = row['id']
                    assert i in expected and i not in seen
                    seen.add(i)
                    assert row['field_labels'] == [f['name'] for f in original[i]['fields']]
                    if number:
                        assert row == saved_rows[i]
                    else:
                        saved_rows[i] = row
                ids = json.loads(stage['response']['message']['content'])['types']
                assert ids == page['selected'] and len(ids)==len(set(ids))<=8
                assert set(ids) <= set(page['type_ids'])
                selected.update(ids)
            assert seen == expected
            expected = selected
        if receipt.get('selection'):
            assert expected == set(receipt['selection']['selected_types'])
    if receipt.get('batch'):
        rows = {r['id']:r for r in receipt['selection']['rows']}
        for hypothesis in receipt['batch']['accepted']:
            candidate = hypothesis['candidate']
            assert candidate == rows[hypothesis['proposal']['candidate']]
            assert hypothesis['alternatives'] == [rows[i] for i in hypothesis['proposal']['alternatives']]
            for citation in hypothesis['proposal']['citations']:
                text = candidate[citation['source']]
                if citation['source']=='object_text':
                    text = text[citation['index']]
                assert text == citation['quote']

helpers, pauses = [], []
runtime_dir = data/'runtime'
for log in runtime_dir.glob('*-*/**/stdout.jsonl'):
    events = [json.loads(line) for line in log.read_text().splitlines() if line.startswith('{')]
    assert events[-1]['stage']=='complete' and events[-1]['helper_absent']
    assert events[-1]['inspection_exit']==0
    report = read(log.with_name('report.json'))
    assert report['pid']==live['process']['pid'] and report['session']==3
    if inventory and 'snapshots' in report:
        assert sha(log.with_name('report.json')) == inventory['report_sha256']
    pauses.extend(report.get('snapshots', report.get('pauses', [])))
    pauses.extend(w['discarded_snapshot'] for w in report.get('warmups', []))
    helpers.append(dict(events_sha256=sha(log), report_sha256=sha(log.with_name('report.json'))))
assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in pauses)
final = read(OUT/'final-state.json')
assert final['jobs']['fence'] is None and final['jobs']['in_flight'] is None
assert not any(t['active'] for t in final['state']['runtime_tests'])
assert not any(f['active'] for f in final['state']['freezes'])
assert read(OUT/'child.json')['exit_code'] == 0
cleanup = read(OUT/'cleanup.json')
assert cleanup['owned_processes_exited'] and cleanup['desktop_responsive']
assert all(cleanup['protection'].values()) and cleanup['new_reversebolt_detections']==0
after = read(OUT/'storage-after.json')['hashes']
storage_differences = {
    'changed':[name for name, digest in frozen['storage_before'].items() if after.get(name)!=digest],
    'new':[name for name in after if name not in frozen['storage_before']],
}
summary = dict(artifact_audit='passed', app_id=frozen['appid'], installed_build=frozen['buildid'],
    release=Path(frozen['bin']).name, unchanged_from_viewfinder=True,
    source_commit=frozen['source_commit'], jobs=result['jobs'],
    named_hypotheses=len((receipt or {}).get('batch',{}).get('accepted',[])) if (receipt or {}).get('batch') else 0,
    metadata_error=(receipt or {}).get('error'),
    runtime_summary={k:live['runtime'].get(k) for k in ('available','phase','notice','catalog_id','revision')},
    model_identity={k:(receipt or {}).get(k) for k in ('model','model_digest','server_version')},
    completed_metadata_model_calls=len((receipt or {}).get('stages', [])),
    metadata_stage_ms=sum(s['elapsed_ms'] for s in (receipt or {}).get('stages', [])),
    activity_error=((receipt or {}).get('activity') or {}).get('error'),
    activity_interpretation_performed=bool((receipt or {}).get('activity')),
    types=len(inventory['types']) if inventory else None,
    fields=sum(len(t['fields']) for t in inventory['types']) if inventory else None,
    reachable_paths=inventory['reachable_paths'] if inventory else None,
    pages_per_round=page_counts, comparisons=bool(result.get('comparison')),
    automatic_discovery_images=15, discovery_input_holds=4,
    manually_reviewed_setup_images=setup['setup_images_reviewed'], manually_reviewed_measurement_images=0,
    helper_runs=len(helpers), pause_count=len(pauses),
    max_stop_to_resume_us=max((p['stop_to_resume_us'] for p in pauses), default=None),
    storage_differences=storage_differences, frozen_sha256=sha(OUT/'frozen.json'),
    result_sha256=sha(OUT/'result.json'), cleanup=cleanup,
    limitation='Artifact checks do not establish meaningful controls, safe ranges, world rollback or generalization. Comparison effects require a separate raw-evidence audit.')
(Path(__file__).parent/'results.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k not in ('runtime_summary','cleanup')},indent=2))
