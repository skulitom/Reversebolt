"""Audit the original terminal naming failure, even after a separate continuation."""
import hashlib
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/runtime-siblings-sable-v1'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json');job=read(OUT/'anode-job.json')
assert job['finished'] and job['exitCode']==1
for name,digest in frozen['sources'].items():assert sha(OUT/'source'/name)==digest,name
for name,digest in frozen['binaries'].items():assert sha(Path(frozen['bin'])/name)==digest,name
state=read(OUT/'discovery.json');runtime=state['state']['live']['runtime'];review=runtime['naming']
assert review['status']=='unavailable' and review['batch'] is None
assert state['jobs']['in_flight'] is None and state['jobs']['fence'] is None
assert not state['state']['runtime_trials'] and not state['state']['runtime_tests']
assert not list(OUT.glob('observation-*.json')) and not (OUT/'plan').exists()
receipt=read(Path(frozen['data'])/'runtime/naming'/review['evidence_id']/'receipt.json')
assert receipt['error']==review['error'] and receipt['incomplete_stage']['phase']=='request'
stage=receipt['incomplete_stage']['stage']
assert hashlib.sha256(stage['prompt'].encode()).hexdigest()==stage['prompt_sha256']
report=Path(runtime['catalog_source']['path']);events=[json.loads(line) for line in report.with_name('stdout.jsonl').read_text().splitlines()]
assert events[-1]['stage']=='complete' and events[-1]['helper_absent'] and events[-1]['inspection_exit']==0
failure=read(OUT/'failure.json')
assert [i['key'] for i in failure['discovery_inputs']]==[68,65,68,65]
assert read(OUT/'child.json')['exit_code']==0
result={'audited':True,'original_run_succeeded':False,'failure_stage':'first naming proposal after type selection',
    'error':review['error'],'completed_metadata_requests':len(receipt['stages']),
    'completed_metadata_ms':sum(s['response']['total_duration'] for s in receipt['stages'])/1e6,
    'failed_prompt_utf8_bytes':len(stage['prompt'].encode()),'failed_request_options':stage['options'],
    'selected_types':len(receipt['selection']['selected_types']),'selected_paths':len(receipt['selection']['rows']),
    'observed_types':len(read(Path(frozen['data'])/'runtime/naming'/review['evidence_id']/'inventory.json')['types']),
    'automatic_images':state['state']['live']['sample_count'],'setup_images_reviewed':read(OUT/'setup.json')['setup_images_reviewed'],
    'discovery_input_holds':len(failure['discovery_inputs']),'measurement_input_holds':0,'sibling_reads':0,
    'gameplay_field_writes':0,'manually_reviewed_measurement_images':0,'native_discovery_helper_unloaded':True,
    'elapsed_ms':job['elapsedMs'],'progress_percent':36}
(OUT/'audit-failure.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
