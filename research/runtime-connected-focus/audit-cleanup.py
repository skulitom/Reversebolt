"""Verify that only exact self-references changed in the separate replay."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'test-results/runtime-connected-focus-v1'
OUT = BASE / 'cleanup-replay'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
for name, digest in frozen['sources'].items():
    assert sha(OUT / 'source' / name) == digest
assert sha(OUT / 'hypothesis_cleanup.exe') == frozen['binary_sha256']
assert sha(BASE / 'review/receipt.json') == frozen['original_receipt_sha256']
assert read(OUT / 'terminal.json')['exit_code'] == 0
original = read(BASE / 'review/receipt.json')
result = read(OUT / 'normalized.json')
assert result['source_receipt_sha256'] == frozen['original_receipt_sha256']
assert result['new_inference_calls'] == 0 and result['edit_authority'] is False
cleanup = result['cleanup']
assert cleanup['policy'] == 'remove_exact_self_alternative_v1'
assert cleanup['original_batch'] == original['batch']
raw = []
for stage in original['stages']:
    if 'hypotheses' in stage['schema']['properties']:
        raw.extend(json.loads(stage['response']['message']['content'])['hypotheses'])
normalized = json.loads(json.dumps(raw))
for change in cleanup['removed']:
    p = normalized[change['proposal_index']]
    assert p['candidate'] == change['candidate'] == p['alternatives'][change['alternative_index']]
    p['alternatives'].pop(change['alternative_index'])
accepted = result['batch']['accepted']
assert len(accepted) == len(raw) and result['batch']['rejected'] == []
rows = {r['id']: r for r in original['selection']['rows']}
for hypothesis, proposal in zip(accepted, normalized):
    assert hypothesis['proposal'] == proposal
    assert hypothesis['candidate'] == rows[proposal['candidate']]
    assert hypothesis['alternatives'] == [rows[i] for i in proposal['alternatives']]
    assert all(i != proposal['candidate'] for i in proposal['alternatives'])
summary = {
    'passed': True, 'original_accepted': len(original['batch']['accepted']),
    'original_rejected': len(original['batch']['rejected']),
    'removed_self_references': len(cleanup['removed']),
    'normalized_accepted': len(accepted), 'normalized_rejected': len(result['batch']['rejected']),
    'questions': [h['proposal']['name'] for h in accepted],
    'new_inference_calls': 0, 'new_native_calls': 0, 'semantics_verified': False,
}
(OUT / 'audit.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
print(json.dumps(summary))
