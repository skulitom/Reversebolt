"""Post-run audit of the frozen native workflow; no input or memory writes."""
import ctypes as C,hashlib,json,re,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=Path(sys.argv[1]).resolve();CASE=OUT/'case';DATA=CASE/'data'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json')
for name,digest in frozen['sources'].items():assert sha(ROOT/name)==digest,name
for name,digest in frozen['binaries'].items():assert sha(OUT/'bin'/name)==digest,name
assert frozen['binaries']['reversebolt_path_fixture.dll']==read(OUT/'fixture-build.json')['binary_sha256']
result=read(CASE/'result.json');assert result['passed'] and not result['external_input_actions']
complete=read(CASE/'complete.json');test=complete['test'];assert test['phase']=='complete' and test['assessment']['status']=='suppression_observed'
assert len(complete['executions'])==18
images=0;intents=[];workers=[]
tests=[read(p) for p in (DATA/'runtime-tests').glob('*.json')];assert len(tests)==3
for t in tests:
    assert t['phase'] in ('complete','stopped')
    for ci,condition in enumerate(t['conditions']):
        for fi,frame in enumerate(condition['frames']):
            assert sha(DATA/'runtime-tests'/frame['path'])==frame['sha256']
            assert sha(DATA/'runtime-tests'/frame['input_path'])==frame['input_sha256']
            assert frame['interval']['clock_id']==t['investigation']
            if fi:assert frame['input']['matches']
            images+=1
    for p in (DATA/'runtime-tests'/t['id']).glob('*.action.json'):
        plan=read(p);outcome=read(p.with_suffix('.outcome.json'))
        assert plan['test_id']==t['id'] and plan['target']['window']==t['window'] and plan['target']['session']==3
        assert outcome['plan_sha256']==sha(p)
        assert plan['action']==(None if plan['frame']<2 else t['actions'][plan['frame']-2])
        assert outcome['completed_unix_ms']>=outcome['started_unix_ms']
        assert outcome['keydown_accepted']==outcome['keyup_accepted']
        intents.append({'test':t['id'],'condition':plan['condition'],'frame':plan['frame'],'plan_sha256':sha(p),'outcome_sha256':sha(p.with_suffix('.outcome.json')),'accepted':outcome['keydown_accepted'],'error':outcome['error']})
        workers.append(outcome['worker_pid'])
assert images==20 and len(intents)==22 and sum(i['accepted'] for i in intents)==13
refused=read(CASE/'refused.json');assert refused['test']['phase']=='stopped'
assert refused['executions'][0]['outcome']['receipt']['keydown_accepted']==0
interrupted=read(CASE/'interrupted-after-restart.json');assert interrupted['test']['phase']=='stopped' and interrupted['test']['pending_capture']
assert len(interrupted['executions'])==3
release=read(CASE/'editor-exit-release.json');assert release['error'] and release['keydown_accepted']==release['keyup_accepted']==1
assert read(CASE/'original-oracle.json')['bytes']==read(CASE/'write.json')['data']['patch']['expected']
helpers=[];pauses=[];writes=0
for p in (DATA/'runtime').glob('*-*/**/stdout.jsonl'):
    events=[json.loads(s) for s in p.read_text(encoding='utf-8').splitlines() if s.startswith('{')]
    assert events[-1]['stage']=='complete' and events[-1]['helper_absent'] and events[-1]['inspection_exit']==0
    report=read(p.with_name('report.json'));pauses.extend(report.get('pauses',report.get('snapshots',[])))
    pauses.extend(w['discarded_snapshot'] for w in report.get('warmups',[]))
    writes+=int(report.get('patch',{}).get('state')=='written')
    helpers.append({'path':str(p.relative_to(OUT)),'sha256':sha(p),'report_sha256':sha(p.with_name('report.json'))})
balance=read(CASE/'balance.json');assert balance['starts']==balance['stops']==len(pauses)
assert all(p['helper_allocations_while_paused']==p['helper_deallocations_while_paused']==0 for p in pauses)
assert len(helpers)==result['helpers'] and writes==2
children=read(CASE/'children.json');assert all(c['exit_code'] is not None for c in children)
kernel=C.WinDLL('kernel32');kernel.OpenProcess.restype=C.c_void_p;kernel.GetExitCodeProcess.argtypes=[C.c_void_p,C.POINTER(C.c_ulong)];kernel.CloseHandle.argtypes=[C.c_void_p]
for pid in workers+[c['pid'] for c in children]:
    handle=kernel.OpenProcess(0x1000,False,pid)
    if handle:
        code=C.c_ulong();assert kernel.GetExitCodeProcess(handle,C.byref(code)) and code.value!=259,pid;kernel.CloseHandle(handle)
before=read(OUT/'desktop-before.json');after=read(OUT/'desktop-after.json')
for key in ('history','observer_model','runtime_trials','runtime_tests','changes','freezes'):assert before['state'].get(key)==after['state'].get(key)
assert before['pid']!=after['pid']
version=(ROOT/'dist/current.txt').read_text(encoding='utf-8-sig').strip()
for name,digest in frozen['binaries'].items():
    if name!='reversebolt_path_fixture.dll':assert sha(ROOT/'dist'/version/name)==digest
log=(ROOT/'test-results/planned-input-tests-final.log').read_text(encoding='utf-8-sig')
assert 'FAILED' not in log and sum(map(int,re.findall(r'test result: ok\. (\d+) passed',log)))==272
assert read(ROOT/'test-results/smoke-c76baf8a/report.json')['mcp_tools']==52
summary={'passed':True,'artifact_root':str(OUT.relative_to(ROOT)),'frozen_sha256':sha(OUT/'frozen.json'),
    'native_checks':len(result['assertions']),'workspace_tests':272,'mcp_tools':52,'comparison_images':18,'interruption_images':2,
    'execution_intents':len(intents),'completed_holds':12,'interrupted_holds':1,'external_gameplay_inputs':0,'manual_images':0,
    'assessment':test['assessment'],'focus_refusal':True,'editor_exit_releases_key':True,'restart_does_not_replay':True,
    'helpers':len(helpers),'helper_receipts':helpers,'balanced_pauses':len(pauses),'maximum_stop_to_resume_us':max(p['stop_to_resume_us'] for p in pauses),
    'helper_heap_while_paused':0,'writes_including_restore':writes,'input_workers_exited':len(workers),'children':children,
    'preview':version,'desktop_pid':after['pid'],'history_preserved':len(after['state']['history']),
    'limitations':['Owned explicitly labelled ABI/renderer fixture; no independent game or semantic/generalization credit.',
        'Evaluator selects the field and key plan. Product executes it but does not learn action meanings.',
        'Sampled input and rendered suppression do not establish physical quantities or full world recovery.',
        'Automatic input is opt-in, same-session foreground only, and restricted to letters, digits, arrows and Space.',
        'Focus checks are sampled; Windows input acceptance is not proof of game delivery.',
        'Human takeover wiring is retained; native interruption here is editor exit, not a desktop Take control click.',
        'Audit authored after the frozen run.']}
Path(__file__).with_name('results.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps({k:v for k,v in summary.items() if k not in ('helper_receipts','children')},indent=2))
