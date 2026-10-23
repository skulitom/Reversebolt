"""Audit the four context probes and the separate current-code catalog replay."""
import hashlib
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
CONTEXT=ROOT/'test-results/sable-context-v1'
context_frozen=read(CONTEXT/'frozen.json')
for name,digest in context_frozen['source_sha256'].items():
    assert sha(ROOT/name)==digest
assert context_frozen['contexts']==[32768,32768,16384,16384]
base=read(CONTEXT/'base-request.json')
assert hashlib.sha256(base['messages'][0]['content'].encode()).hexdigest()==context_frozen['prompt_sha256']
context_results=[]
for i,context in enumerate(context_frozen['contexts']):
    expected=json.loads(json.dumps(base));expected['options']['num_ctx']=context
    assert read(CONTEXT/f'request-{i}.json')==expected
    result=read(CONTEXT/f'result-{i}.json')
    assert result['context']==context and result['error'] is None
    response=result['response']
    assert response['done'] and response['done_reason']=='stop'
    assert response['prompt_eval_count']==801 and response['message']['role']=='assistant'
    assert json.loads(response['message']['content'])=={'hypotheses':[]}
    model=next(m for m in result['loaded_after']['models'] if m['name']==context_frozen['model'])
    assert model['digest']==context_frozen['model_digest'] and model['context_length']==context
    context_results.append(dict(context=context,elapsed_ms=result['elapsed_ms'],
        load_ms=response['load_duration']/1e6,prompt_tokens=response['prompt_eval_count'],
        output_tokens=response['eval_count'],abstained=True,reported_model_vram=model['size_vram'],
        result_sha256=sha(CONTEXT/f'result-{i}.json')))

OUT=ROOT/'test-results/sable-naming-replay-v1'
frozen=read(OUT/'frozen.json')
assert sha(OUT/'report.json')==frozen['report_sha256']
assert sha(OUT/'evaluator.exe')==frozen['evaluator_sha256']
for name,digest in frozen['sources'].items():
    assert sha(OUT/'source'/name)==digest
assert read(OUT/'exit.json')['exit_code']==0
receipt=read(OUT/'output/receipt.json')
inventory=read(OUT/'output/input.json')['inventory']
assert receipt['report_sha256']==inventory['report_sha256']==frozen['report_sha256']
assert receipt['model']==frozen['model'] and receipt['model_digest']==frozen['model_digest']
assert receipt['error'] is None and not receipt['cancelled']
assert receipt.get('incomplete_stage') is None and receipt['activity'] is None
original={t['id']:t for t in inventory['types']}
expected,seen_rows,selection_stages=set(original),{},[]
for number,round_ in enumerate(receipt['paged_selection']['rounds']):
    seen,selected=set(),set()
    for page in round_['pages']:
        stage=receipt['stages'][page['stage_index']]
        rows=json.loads(stage['prompt'].split('\nObserved types and path-scoped text examples: ',1)[1])['types']
        assert len(stage['prompt'].encode())<=12000
        assert [r['id'] for r in rows]==page['type_ids']
        for row in rows:
            i=row['id'];assert i in expected and i not in seen;seen.add(i)
            assert row['field_labels']==[f['name'] for f in original[i]['fields']]
            if number:assert row==seen_rows[i]
            else:seen_rows[i]=row
        ids=json.loads(stage['response']['message']['content'])['types']
        assert ids==page['selected'] and len(ids)==len(set(ids))<=8
        assert set(ids)<=set(page['type_ids']);selected.update(ids)
        selection_stages.append(page['stage_index'])
    assert seen==expected
    expected=selected
selection=receipt['selection']
assert expected==set(selection['selected_types'])
assert selection_stages==list(range(len(selection_stages)))
proposals=[]
request_summary=[]
for i,stage in enumerate(receipt['stages']):
    assert hashlib.sha256(stage['prompt'].encode()).hexdigest()==stage['prompt_sha256']
    response=stage['response'];options=stage['options']
    assert response['done'] and response['done_reason']=='stop'
    assert response['prompt_eval_count'] <= options['num_ctx']-options['num_predict']-512
    if i < len(selection_stages):
        assert options['num_ctx']==16384
    else:
        group=i-len(selection_stages)
        type_id=selection['selected_types'][group]
        rows=[r for r in selection['rows'] if r['type_id']==type_id]
        table=json.loads(stage['prompt'].split('\nObserved paths: ',1)[1])
        assert table==[dict(candidate=r['id'],field_name=r['field_name'],type_name=r['type_name'],
            object_text=r['object_text'],observed_value=r['value']) for r in rows]
        quota=2 if group<12-len(selection['selected_types']) else 1
        assert options['num_predict']==quota*850
        expected_context=16384 if len(stage['prompt'].encode())+quota*850+512<=16384 else 32768
        assert options['num_ctx']==expected_context
        supplied={r['id'] for r in rows}
        nominations=json.loads(response['message']['content'])['hypotheses']
        assert len(nominations)<=quota
        for p in nominations:
            assert p['candidate'] in supplied and set(p['alternatives'])<=supplied
        proposals.extend(nominations)
    request_summary.append(dict(stage=i,context=options['num_ctx'],prompt_bytes=len(stage['prompt'].encode()),
        prompt_tokens=response['prompt_eval_count'],elapsed_ms=stage['elapsed_ms']))
assert len(receipt['stages'])==len(selection_stages)+len(selection['selected_types'])
assert not receipt['batch']['rejected']
assert proposals==[h['proposal'] for h in receipt['batch']['accepted']]
rows={r['id']:r for r in selection['rows']}
for h in receipt['batch']['accepted']:
    candidate=h['candidate'];assert candidate==rows[h['proposal']['candidate']]
    assert h['alternatives']==[rows[i] for i in h['proposal']['alternatives']]
    for citation in h['proposal']['citations']:
        text=candidate[citation['source']]
        if citation['source']=='object_text':text=text[citation['index']]
        assert text==citation['quote']
summary=dict(artifact_audit='passed',development_replay=True,native_timeout_resolved=False,
    new_images=0,field_writes=0,context_probe_frozen_sha256=sha(CONTEXT/'frozen.json'),
    context_probes=context_results,replay_frozen_sha256=sha(OUT/'frozen.json'),
    receipt_sha256=sha(OUT/'output/receipt.json'),selected_types=selection['selected_types'],
    accepted=[h['proposal']['name'] for h in receipt['batch']['accepted']],rejected=0,
    stages=request_summary,total_stage_ms=sum(s['elapsed_ms'] for s in request_summary),
    limitation='The game was closed. Names are hypotheses. Both legacy-context and reduced-context diagnostic requests succeeded; this is not a causal diagnosis of the native timeout or gameplay validation.')
(Path(__file__).parent/'replay-results.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:summary[k] for k in ('artifact_audit','accepted','total_stage_ms','native_timeout_resolved','new_images','field_writes')}))
