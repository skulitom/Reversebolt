"""Offline source/request/receipt checks; proposal correctness needs review."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/test-design-v1'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=read(OUT/'frozen.json')
for name,digest in frozen['sources'].items():assert sha(OUT/'source'/name)==digest,name
exe=OUT/'evaluator.exe';assert sha(exe)==frozen['evaluator_sha256']
for name,digest in frozen['plans'].items():assert sha(OUT/f'{name}-plan.json')==digest
for name,digest in frozen['source_plans'].items():assert sha(OUT/f'{name}-source.json')==digest
results=read(OUT/'result.json')
assert [r['case'] for r in results]==list(frozen['plans']) and all(r['exit_code']==0 for r in results)
rows=[];identities=set()
for case in frozen['plans']:
    plan=read(OUT/f'{case}-plan.json')['plan']
    assert len(list((OUT/case).iterdir()))==len(plan['choices'])
    for i,choice in enumerate(plan['choices']):
        directory=OUT/case/str(i);input_=read(directory/'input.json');receipt=read(directory/'receipt.json')
        evidence=input_['evidence']
        # Context omits absent optional measurements when serialized by Rust.
        expected_activity={k:v for k,v in plan['activity_context'].items()
                           if k not in ('motion','appearances','regions') or v is not None}
        assert evidence['activity']==expected_activity
        assert evidence['observed_actions']==plan['observed_actions']
        assert evidence['report_sha256']==plan['report_sha256']
        assert evidence['candidates']==[{'row':choice['selected_candidate'],'proposed_value':choice['value']}]
        assert input_['question']==choice['question'] and input_['explanation']==choice['explanation']
        assert input_['selection_role']==choice['selection_role']
        stage=receipt['stage'];assert hashlib.sha256(stage['prompt'].encode()).hexdigest()==stage['prompt_sha256']
        data=json.loads(stage['prompt'].split('\nEvidence: ',1)[1])
        assert data['fallible_visual_interpretation']==plan['activity_context']['observation']
        assert data['unexecuted_metadata_clues']['field_name']==choice['selected_candidate']['field_name']
        assert data['unexecuted_metadata_clues']['object_text']==choice['selected_candidate']['object_text']
        assert 'audio recording' in data['available_experiment']['unavailable']
        identities.add((receipt['model'],receipt['model_digest'],receipt['server_version']))
        check=subprocess.run([str(exe),'check',str(directory)],cwd=ROOT,text=True,capture_output=True)
        if receipt['error'] is None and not receipt['cancelled']:
            assert check.returncode==0,check.stderr
            assert read(directory/'receipt.json')['design']==json.loads(check.stdout)['design']
        else:
            assert receipt['design'] is None and check.returncode!=0
        design=(receipt.get('design') or {}).get('proposal') or {}
        rows.append(dict(case=case,index=i,field=choice['selected_candidate']['field_name'],role=choice['selection_role'],
            path_id=choice['selected_candidate']['path_id'],error=receipt['error'],status=design.get('status'),
            readout=design.get('readout'),prediction=design.get('prediction'),
            required_condition=design.get('required_condition'),condition_support=design.get('condition_support'),
            reason=design.get('reason'),next_observation=design.get('next_observation'),uncertainty=design.get('uncertainty'),
            elapsed_ms=stage['elapsed_ms'],prompt_tokens=stage['response'].get('prompt_eval_count'),
            receipt_sha256=sha(directory/'receipt.json')))
assert len(rows)==frozen['requests']==24 and len(identities)==1
lookup={(r['case'],r['index']):r for r in rows}
audio=[lookup[('viewfinder',i)] for i in (1,4,6,8)]+[lookup[('sable',i)] for i in (0,3,6)]
summary=dict(artifact_audit='passed',research_only=True,model_identity=list(next(iter(identities))),
    requests=len(rows),accepted=sum(r['error'] is None for r in rows),
    proposed_current_probes=[dict(case=r['case'],index=r['index'],field=r['field']) for r in rows if r['status']=='current_probe'],
    known_positive_proposed=lookup[('viewfinder',2)]['status']=='current_probe',
    audio_questions_proposed_as_visual_probes=sum(r['status']=='current_probe' for r in audio),
    opaque_status=lookup[('opaque',0)]['status'],brightness_status=lookup[('brightness',0)]['status'],
    metadata_event_statuses={case:lookup[(case,0)]['status'] for case in ('viewfinder_metadata_event','sable_metadata_event')},
    total_stage_ms=sum(r['elapsed_ms'] for r in rows),new_images=0,field_writes=0,
    frozen_sha256=sha(OUT/'frozen.json'),rows=rows,
    limitation='Known development cases plus synthetic transformations. Quoted text and valid categories do not prove a proposed experiment is useful. No native comparison or gameplay validation was performed.')
(Path(__file__).parent/'results.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k!='rows'},indent=2))
