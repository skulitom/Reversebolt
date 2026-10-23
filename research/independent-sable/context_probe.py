"""Local-only, four predeclared requests reconstructed from frozen Rust source."""
import hashlib
import json
from pathlib import Path
import re
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
NATIVE = ROOT/'test-results/independent-sable-v1'
OUT = ROOT/'test-results/sable-context-v1'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
def request(path, payload=None, timeout=90):
    data = None if payload is None else json.dumps(payload).encode()
    req = urllib.request.Request('http://127.0.0.1:11434'+path, data=data,
        headers={'Content-Type':'application/json'})
    with urllib.request.urlopen(req, timeout=timeout) as response:
        raw = response.read(128*1024+1)
        if len(raw)>128*1024:
            raise ValueError('Response exceeds budget')
        return json.loads(raw)

OUT.mkdir()
receipt_path = next((NATIVE/'data/runtime/naming').glob('*/receipt.json'))
receipt = read(receipt_path)
source_path = NATIVE/'source/crates/reversebolt-observer/src/runtime_naming.rs'
core_path = NATIVE/'source/crates/reversebolt-core/src/runtime_hypotheses.rs'
source, core = source_path.read_text(), core_path.read_text()
intro = json.loads(re.search(r'pub const PROPOSE_PROMPT: &str = ("(?:[^"\\]|\\.)*");',source)[1])
template = json.loads(re.search(r'let prompt = format!\(\s*("(?:[^"\\]|\\.)*Observed paths: \{\}")',source)[1])
body = core.split('pub fn proposal_schema()',1)[1].split('#[cfg(test)]',1)[0]
schema = json.loads(re.search(r'serde_json::json!\((\{.*\})\)',body,re.S)[1])
selection = receipt['selection']
quota = 2 if 0 < 12-len(selection['selected_types']) else 1
rows = [r for r in selection['rows'] if r['type_id']==selection['selected_types'][0]]
table = [dict(candidate=r['id'],field_name=r['field_name'],type_name=r['type_name'],
              object_text=r['object_text'],observed_value=r['value']) for r in rows]
prompt = template.replace('{PROPOSE_PROMPT}',intro).replace('{quota}',str(quota)).replace(
    '{}',json.dumps(table,sort_keys=True,separators=(',',':'),ensure_ascii=False))
schema['properties']['hypotheses']['maxItems'] = quota
base = dict(model=receipt['model'],stream=False,keep_alive='2m',format=schema,think=False,
    options=dict(temperature=0,seed=42,num_ctx=32768,num_predict=quota*850),
    messages=[dict(role='user',content=prompt,images=[])])
contexts = [32768,32768,16384,16384]
frozen = dict(native_receipt_sha256=sha(receipt_path), native_frozen_sha256=sha(NATIVE/'frozen.json'),
    source_sha256={str(p.relative_to(ROOT)):sha(p) for p in (source_path,core_path,Path(__file__),Path(__file__).with_name('CONTEXT-PROTOCOL.md'))},
    model=receipt['model'],model_digest=receipt['model_digest'],contexts=contexts,
    first_group_type=selection['selected_types'][0],row_count=len(rows),prompt_bytes=len(prompt.encode()),
    prompt_sha256=hashlib.sha256(prompt.encode()).hexdigest(),
    limitation='Reconstructed failed request, not a retained native HTTP body; no gameplay proof.')
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2)+'\n')
(OUT/'base-request.json').write_text(json.dumps(base,indent=2)+'\n')
for number, context in enumerate(contexts):
    before = request('/api/ps')
    model = next(m for m in request('/api/tags')['models'] if m['name']==receipt['model'])
    assert model['digest']==receipt['model_digest']
    base['options']['num_ctx'] = context
    (OUT/f'request-{number}.json').write_text(json.dumps(base,indent=2)+'\n')
    start=time.monotonic()
    response=error=None
    try:
        response=request('/api/chat',base)
    except Exception as failure:
        error=repr(failure)
    elapsed=round((time.monotonic()-start)*1000)
    after=request('/api/ps')
    result=dict(context=context,elapsed_ms=elapsed,error=error,response=response,
        loaded_before=before,loaded_after=after)
    (OUT/f'result-{number}.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(case=number,context=context,elapsed_ms=elapsed,error=error,
        prompt_tokens=(response or {}).get('prompt_eval_count'),
        output_tokens=(response or {}).get('eval_count'),done_reason=(response or {}).get('done_reason'))),flush=True)
