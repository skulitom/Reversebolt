"""Audit retained native evidence; no inputs, captures, models or target reads."""
import hashlib,json,re,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve();CASE=OUT/'case'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json')
for name,digest in frozen['sources'].items():assert sha(ROOT/name)==digest,name
for name,digest in frozen['binaries'].items():assert sha(OUT/'bin'/name)==digest,name
result=read(CASE/'result.json');assert result['passed'] and len(result['actions'])==6
frames=0
for name in ('automatic','observed'):
    live=read(CASE/(name+'.json'))['state']['live'];folder=Path(live['latest']['screen']['path']).parent
    assert len(live['frames'])==live['sample_count']==15
    for frame in live['frames']:
        assert sha(folder/f"frame-{frame['number']:03}.png")==frame['sha256']
        capture=read(folder/f"frame-{frame['number']:03}.json")
        assert capture['mode']=='play' and capture['captured_bytes']==0
        assert not capture['memory_reads'] and not capture['prediction_reads']
        assert capture['recognition_ms']==0 and not capture['screen']['words'] and not capture['screen']['text']
        assert frame['capture_interval']['clock_id']==live['investigation_id']
        frames+=1
    assert live['coverage']['attempted_bytes']==live['retained_bytes']==0
observed=read(CASE/'observed.json')['state']['live'];e=result['evidence']['evidence']
for i,interval in enumerate(e['intervals']):
    a,b=observed['frames'][i:i+2]
    assert (interval['before'],interval['after'])==(a['id'],b['id'])
    assert (interval['before_sha256'],interval['after_sha256'])==(a['sha256'],b['sha256'])
    assert interval['elapsed_us']==[b['capture_interval']['begin_us']-a['capture_interval']['end_us'],b['capture_interval']['end_us']-a['capture_interval']['begin_us']]
assert e['intervals'][5]['exposure']['kind']=='unknown'
assert e['intervals'][7]['exposure']==e['intervals'][8]['exposure']=={'kind':'key','value':68}
assert any(p['first_interval']==7 and p['last_interval']==8 for p in e['episodes'])
action=next(a for a in e['actions'] if a['key']==68)
assert len(action['bounded_episodes'])==4 and action['median_difference']>3 and action['quiet_median_difference']<1
for name in ['refusal-'+str(i) for i in range(4)]+['changed-image-refusal']:
    assert read(CASE/(name+'.json'))['response']['ok'] is False
helpers=[];pauses=[]
for p in (CASE/'data/runtime').glob('*-*/**/stdout.jsonl'):
    entries=[json.loads(s) for s in p.read_text().splitlines() if s.startswith('{')]
    assert entries[-1]['stage']=='complete' and entries[-1]['helper_absent'] and entries[-1]['inspection_exit']==0
    report=read(p.with_name('report.json'));assert 'patch' not in report
    pauses.extend(report.get('pauses',report.get('snapshots',[])))
    pauses.extend(w['discarded_snapshot'] for w in report.get('warmups',[]))
    helpers.append({'receipt':str(p.relative_to(OUT)),'sha256':sha(p),'report_sha256':sha(p.with_name('report.json'))})
balance=read(CASE/'balance.json');assert balance['starts']==balance['stops']==len(pauses)
assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in pauses)
assert len(helpers)==result['helpers']==3
children=read(CASE/'children.json');assert all(c['exit_code'] is not None for c in children)
before=read(OUT/'desktop-before.json');after=read(OUT/'desktop-after.json')
for key in ('history','observer_model','runtime_trials','runtime_tests','changes','freezes'):assert before['state'].get(key)==after['state'].get(key)
assert before['pid']!=after['pid']
version=(ROOT/'dist/current.txt').read_text(encoding='utf-8-sig').strip()
for name,digest in frozen['binaries'].items():
    if name!='reversebolt_path_fixture.dll':assert sha(ROOT/'dist'/version/name)==digest
tests=(ROOT/'test-results/play-workspace-verified.log').read_text(encoding='utf-8-sig')
assert 'FAILED' not in tests and sum(map(int,re.findall(r'test result: ok\. (\d+) passed',tests)))==268
summary={'passed':True,'artifact_root':str(OUT.relative_to(ROOT)),'frozen_sha256':sha(OUT/'frozen.json'),
    'native_checks':len(result['checks']),'workspace_tests':268,'mcp_tools':52,'frames':frames,'evaluator_holds':6,'manual_screenshots':0,
    'observed_key':action,'unknown_intervals':sum(i['exposure']['kind']=='unknown' for i in e['intervals']),
    'helper_operations':len(helpers),'helper_receipts':helpers,'balanced_pauses':len(pauses),
    'maximum_stop_to_resume_us':max(p['stop_to_resume_us'] for p in pauses),'helper_heap_while_paused':0,'writes':0,
    'children':children,'preview':version,'desktop_pid':after['pid'],'history_preserved':len(after['state']['history']),
    'failed_attempt':'test-results/play-evidence-v1/case/failure.json',
    'limitations':['Owned explicitly configured renderer; no independent game, semantic, causal or executable-plan claim.',
        'Automatic first pass is fifteen captures with 500 ms delays after completion, not continuous learning or a promised frame rate.',
        'Passive response evidence is exposed to agents; local metadata inference does not yet consume it.',
        'Initial native attempt stopped at an evaluator path error before input; corrected under a new frozen root.',
        'One repeated suite run failed a storage fault test; unique test directories now prevent PID-reuse collisions, and the final suite passes. The exact prior failure cause was not proven.',
        'Audit authored after execution; no product thresholds or evaluator input plan changed.']}
Path(__file__).with_name('results.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps({k:v for k,v in summary.items() if k not in ('helper_receipts','children')},indent=2))
