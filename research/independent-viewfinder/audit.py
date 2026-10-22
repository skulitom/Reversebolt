"""Check the preserved failed native trial without a game or model call."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/independent-viewfinder-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
for name, digest in frozen['sources'].items():
    assert sha(OUT / 'source' / name) == digest, name
for name, digest in frozen['binaries'].items():
    assert sha(Path(frozen['bin']) / name) == digest, name
for name, digest in frozen['storage_before'].items():
    assert sha(OUT / 'storage-before' / name) == digest, name
assert sha(OUT / 'appmanifest-before.acf') == frozen['appmanifest_sha256']
result = read(OUT / 'result.json')
discovery = read(OUT / 'discovery.json')['state']
live = discovery['live']
assert result['workflow_finished'] and result['samples'] == len(live['frames']) == 15
assert result['jobs'] == [{'phase':'Completed', 'error':None}]
assert [i['key'] for i in result['evaluator_inputs']] == [68, 65, 68, 65]
assert all(i['planned_ms'] == 200 and i['end_ns'] > i['begin_ns'] for i in result['evaluator_inputs'])
assert not result['comparison'] and not result['plan_available']
assert result['manually_reviewed_measurement_images'] == 0
review = live['runtime']['naming']
assert review == result['review'] and review['status'] == 'unavailable' and review['batch'] is None
data = Path(frozen['data'])
receipt_path = data / 'runtime/naming' / review['evidence_id'] / 'receipt.json'
receipt = read(receipt_path)
inventory = read(receipt_path.with_name('inventory.json'))
assert receipt['report_sha256'] == review['report_sha256'] == inventory['report_sha256']
assert receipt['error'] == review['error'] and not receipt['stages'] and receipt['batch'] is None
assert receipt['model_digest'] == '22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643'
captures = [p for p in (data / 'captures').iterdir() if (p / 'frame-001.png').is_file()]
assert len(captures) == 1
for frame in live['frames']:
    assert sha(captures[0] / f"frame-{frame['number']:03}.png") == frame['sha256']
helpers, pauses = [], []
for log in (data / 'runtime').glob('*-*/**/stdout.jsonl'):
    events = [json.loads(line) for line in log.read_text().splitlines() if line.startswith('{')]
    assert events[-1]['stage'] == 'complete' and events[-1]['helper_absent'] and events[-1]['inspection_exit'] == 0
    report = read(log.with_name('report.json'))
    assert report['pid'] == live['process']['pid'] and report['session'] == 3 and not report['edit_authority']
    assert sha(log.with_name('report.json')) == inventory['report_sha256']
    pauses.extend(report['snapshots'])
    pauses.extend(w['discarded_snapshot'] for w in report['warmups'])
    helpers.append({'events_sha256':sha(log),'graph_complete':report['graph_complete'],'root_scan_complete':report['root_scan_complete']})
assert len(helpers) == 1
assert all(p['helper_allocations_while_paused'] == p['helper_deallocations_while_paused'] == 0 for p in pauses)
final = read(OUT / 'final-state.json')
assert not final['state']['runtime_trials'] and not final['state']['runtime_tests'] and not final['state']['freezes']
assert final['jobs']['fence'] is None and final['jobs']['in_flight'] is None
setup = read(OUT / 'setup.json')
assert setup['gameplay_observed'] and setup['setup_images_reviewed'] == 3
cleanup = read(OUT / 'cleanup.json')
assert cleanup['owned_processes_exited'] and cleanup['desktop_responsive'] and all(cleanup['protection'].values())
assert cleanup['new_reversebolt_detections'] == 0
after = read(OUT / 'storage-after.json')
for name in ['viewfinder_0.sav', 'viewfinder_0.bak']:
    assert after['hashes'][name] == frozen['storage_before'][name]
model_log = (OUT / 'model-request-tail.log').read_text(encoding='utf-8-sig')
assert 'task.n_tokens = 36168' in model_log and '1m30s' in model_log and 'cancel task' in model_log
summary = {'artifact_audit':'passed','native_trial':'failed_model_request','source_commit':frozen['source_commit'],
    'release':Path(frozen['bin']).name,'app_id':frozen['appid'],'installed_build':'20950646',
    'setup_images_reviewed':3,'automatic_images':15,'manually_reviewed_measurement_images':0,'input_holds':4,
    'types':len(inventory['types']),'fields':sum(len(t['fields']) for t in inventory['types']),
    'reachable_paths':inventory['reachable_paths'],'named_questions':0,'comparisons':0,'gameplay_edits':0,
    'failed_model_prompt_tokens':36168,'failed_model_request_seconds':90,'request_error':receipt['error'],
    'helpers':helpers,'pause_count':len(pauses),'max_stop_to_resume_us':max(p['stop_to_resume_us'] for p in pauses),
    'save_and_backup_unchanged':True,'normal_logs_and_analytics_changed':True,
    'frozen_manifest_sha256':sha(OUT/'frozen.json'),'receipt_sha256':sha(receipt_path),
    'model_log_sha256':sha(OUT/'model-request-tail.log'),'cleanup':cleanup,
    'limitation':'No named or verified gameplay control. Supports diagnosis of model request size, not generalization.'}
(ROOT/'research/independent-viewfinder/results.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:summary[k] for k in ['artifact_audit','native_trial','types','fields','reachable_paths','named_questions','comparisons','save_and_backup_unchanged']}))
