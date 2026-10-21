"""Recheck retained product trials without contacting or changing a target."""
import hashlib, json, struct, subprocess, sys
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
BASE=Path(sys.argv[1]).resolve(); RUN=BASE/'fixture'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def digest(value, ordered=False):
    return hashlib.sha256(json.dumps(value,sort_keys=not ordered,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
manifest=read(BASE/'frozen.json')
for name,expected in manifest['binaries'].items(): assert sha(BASE/'bin'/name)==expected
for path,hashes in manifest['sources'].items():
    data=subprocess.check_output(['git','show',manifest['revision']+':'+path],cwd=ROOT)
    assert hashlib.sha256(data).hexdigest()==hashes['git_sha256']
    assert hashes['working_sha256'] in {hashlib.sha256(d).hexdigest() for d in (data,data.replace(b'\n',b'\r\n'),data.replace(b'\r\n',b'\n'))}
assert sha(BASE/'bin/reversebolt_path_fixture.dll')==sha(ROOT/'test-results/runtime-paths-v2/bin/reversebolt_path_fixture.dll')
summary=read(RUN/'summary.json'); assert summary['passed']
assert summary['screenshots']==summary['gameplay_input']==summary['verified_controls']==0
assert summary['target']==read(RUN/'ready.json')
pauses=0; maximum=0; states={}
for entry in summary['evidence']:
    directory=Path(entry['path']); report=read(directory/'report.json')
    assert sha(directory/'report.json')==entry['report_sha256'] and sha(directory/'stdout.jsonl')==entry['receipt_sha256']
    receipts=[json.loads(line) for line in (directory/'stdout.jsonl').read_text(encoding='utf-8').splitlines()]
    assert receipts[0]['stage']=='validated' and receipts[-1]['stage']=='complete' and receipts[-1]['helper_absent']
    assert report.get('scope', {k:report.get(k) for k in ('pid','created','session')})==summary['target']
    shots=report.get('pauses',report.get('snapshots',[])+[w['discarded_snapshot'] for w in report.get('warmups',[])])
    for shot in shots:
        assert shot['helper_allocations_while_paused']==shot['helper_deallocations_while_paused']==0
        maximum=max(maximum,shot['stop_to_resume_us'])
    pauses+=len(shots)
    states[str(directory/'report.json')]=report
assert summary['operations']==len(states)==19 and pauses==summary['pauses']==44
assert read(RUN/'balance.json')==dict(starts=pauses,stops=pauses)
written=refused=0
records=[read(p) for p in (RUN/'data/runtime-trials').glob('*.json')]
assert len(records)==summary['trials']==8
for trial in records:
    base=trial['baseline']; assert digest(base['report'])==base['report_sha256']
    assert base['report']==read(Path(base['path']))
    assert base['report']['checks'][0]['bytes']==trial['patch']['expected']
    assert trial['context']['process']==summary['target']
    for side in ('write','restoration'):
        attempt=trial[side]
        if attempt is None: continue
        patch=attempt['patch']; evidence=attempt['evidence']; report=evidence['report']
        assert digest(report)==evidence['report_sha256'] and states[evidence['path']]==report
        assert digest(patch,True)==report['patch_id']
        assert report['checks'][0]['path_id']==trial['candidate_id']
        receipt=report['patch']; assert attempt['outcome']==receipt['state']
        if attempt['outcome']=='written':
            written+=1
            assert receipt['attempted'] and receipt['transferred']==len(patch['replacement']) and receipt['observed_after']==patch['replacement']
            check=report['checks'][0]
            assert check['bytes']==patch['expected'] and check['type_code']==patch['type_code']
            assert int(check['field_address'],16)==patch['field_address']
            assert [int(a,16) for a in check['snapshot_object_addresses']]==patch['object_addresses']
        else:
            refused+=1; assert not receipt['attempted'] and receipt['transferred']==0 and receipt['observed_after'] is None
        if side=='restoration':
            assert patch['expected']==trial['patch']['replacement'] and patch['replacement']==trial['patch']['expected']
            assert all(patch[k]==trial['patch'][k] for k in ('path','type_code','field_address','object_addresses'))
assert written==6 and refused==2
oracles={p.stem:read(p) for p in RUN.glob('*-oracle.json')}
assert len(oracles)==9
for oracle in oracles.values(): assert oracle['bytes']==list(struct.pack('<f',oracle['expected']))
assert oracles['restore-refused-oracle']['expected']==4.5 and oracles['cleanup-oracle']['expected']==3.5
assert read(RUN/'write-replay.json')['data']==read(RUN/'write.json')['data']
assert read(RUN/'restore-replay.json')['data']==read(RUN/'restore.json')['data']
assert read(RUN/'stale.json')['data']['phase']=='expired'
assert not read(RUN/'scope-refusal.json')['ok'] and not read(RUN/'restart-refusal.json')['ok']
restart=read(RUN/'restarted.json')['state']; assert restart['live'] is None
assert {t['id']:t for t in restart['runtime_trials']}=={t['id']:t for t in records}
before=(RUN/'ui-before.txt').read_text(encoding='utf-8')
after=(RUN/'ui-after.txt').read_text(encoding='utf-8')
assert all(s in before for s in ('Review edit','Restore value','Learning','4.5'))
assert 'Original value restored' in after and 'Learning' in after
ui=read(RUN/'ui-restored-status.json')['state']
assert ui['live']['runtime']['cards'][0]['value']=='3.5'
assert any(e['source']=='desktop' and 'Original bytes were observed' in e['text'] for e in ui['history'])
execution=read(RUN/'execution.json'); assert len(execution)==4 and all(p['exit'] is not None for p in execution)
assert next(p['exit'] for p in execution if p['pid']==summary['target']['pid'])==0
smoke_path=Path(sys.argv[2]).resolve(); smoke=read(smoke_path)
assert smoke['mcp_tools']==42 and smoke['shared_session']==smoke['auth_and_origin']=='passed'
assert all(p['edit_restore']=='passed' for p in smoke['practice']) and smoke['stale_restore']=='rejected'
result={'audit':'passed','revision':manifest['revision'],'frozen_sha256':sha(BASE/'frozen.json'),
    'binaries':manifest['binaries'],'source_files':len(manifest['sources']),'cases':len(summary['cases']),
    'helper_operations':len(states),'balanced_pauses':pauses,'max_stop_to_resume_us':maximum,
    'confirmed_numeric_writes':written,'refusals':refused,'trials':len(records),'oracle_checks':len(oracles),
    'ui_restore':'invoked through accessibility; shared result confirmed','execution':execution,
    'smoke':{'path':str(smoke_path),'sha256':sha(smoke_path),'result':smoke},'verified_controls':0,
    'scope':'Owned ABI fixture. No real-game semantics, persistent actor identity or complete gameplay rollback proven.'}
(BASE/'audit.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
print(json.dumps({k:v for k,v in result.items() if k not in ('binaries','execution','smoke')}))
