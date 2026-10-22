"""Audit both frozen local replays without model requests or game access.

This is an artifact consistency audit, not an accuracy or generalization pass.
The Rust check command additionally validates v2's exact typed input digest.
"""
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
CASES = ['original', 'reversed', 'opaque', 'other_activity']


def read(path):
    return json.loads(path.read_text(encoding='utf-8'))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


native_path = ROOT/'test-results/viewfinder-followup-v1/comparison-2.json'
native = read(native_path)['test']
assert native['assessment']['status'] == 'suppression_observed'
positive_path = native['candidate']['id']
results = []
identities = set()
for version in (1, 2):
    directory = ROOT/f'test-results/comparison-priority-v{version}'
    frozen = read(directory/'frozen.json')
    assert frozen['cases'] == CASES
    assert read(directory/'result.json') == [dict(case=c, exit_code=0) for c in CASES]
    assert sha(directory/'chosen-plan.json') == frozen['source_plan_sha256']
    for name, expected in frozen['sources'].items():
        assert sha(directory/'source'/name) == expected, name
    binary = directory/'evaluator.exe'
    if version == 2:
        assert binary.resolve() == Path(frozen['evaluator']).resolve()
        assert sha(binary) == frozen['evaluator_sha256']
    # V1 recorded a hash but did not retain its executable. Do not check the
    # mutable target/ path as though it were the frozen binary.
    plan = read(directory/'chosen-plan.json')['plan']
    initial = dict(report_sha256=plan['report_sha256'], activity=plan['activity_context'],
                   observed_actions=plan['observed_actions'],
                   candidates=[dict(row=c['selected_candidate'], proposed_value=c['value'])
                               for c in plan['choices']])
    code = (directory/'source/crates/reversebolt-observer/src/runtime_priority.rs').read_text()
    intro = json.loads(re.search(r'pub const PROMPT: &str = ("(?:[^"\\]|\\.)*");', code)[1])
    cases = []
    for case in CASES:
        ip, rp = directory/case/'input.json', directory/case/'receipt.json'
        saved = read(ip)
        assert saved['source_sha256'] == frozen['source_plan_sha256']
        assert saved['variation'] == case
        expected = deepcopy(initial)
        if case == 'reversed':
            expected['candidates'].reverse()
        elif case == 'opaque':
            for i, candidate in enumerate(expected['candidates']):
                candidate['row'].update(type_name='Container', field_name=f'F{i}', object_text=[])
        elif case == 'other_activity':
            for name in ('motion', 'appearances', 'regions'):
                expected['activity'].pop(name, None)
            expected['activity']['observation'].update(
                description='In both pairs the visible surfaces become brighter. Their shapes and positions appear unchanged.',
                uncertainty='The source of the brightness change is unknown; no movement is established.')
        assert saved['input'] == expected, (version, case, 'input changed')
        receipt = read(rp)
        assert receipt['error'] is None and not receipt['cancelled']
        assert receipt['model'] == frozen['model']
        identities.add((receipt['model'], receipt['model_digest'], receipt['server_version']))
        stage = receipt['stage']
        assert hashlib.sha256(stage['prompt'].encode()).hexdigest() == stage['prompt_sha256']
        assert stage['prompt'].startswith(intro+'\nEvidence: ')
        evidence = json.loads(stage['prompt'][len(intro+'\nEvidence: '):])
        assert evidence['observation'] == expected['activity']['observation']
        assert evidence['basis'] == expected['activity'].get('basis', 'interpreted')
        assert evidence['candidates'] == [dict(candidate=i, field_name=c['row']['field_name'],
            type_name=c['row']['type_name'], object_text=c['row']['object_text'],
            observed_value=c['row']['value'], proposed_value=c['proposed_value'])
            for i, c in enumerate(expected['candidates'])]
        symbols, sequence = [], []
        for action in expected['observed_actions']['actions']:
            if action['keys'] not in symbols:
                symbols.append(action['keys'])
            sequence.append(dict(symbol=symbols.index(action['keys']), hold_ms=action['hold_ms']))
        assert evidence['repeat_sequence'] == sequence
        assert evidence['reviewed_pair_symbol'] == symbols.index([expected['activity']['source']['key']])
        if version == 2:
            source = expected['activity']['source']
            assert evidence['measured_image_difference'] == dict(
                action_pairs=[p['difference'] for p in source['pairs']],
                quiet=source['quiet_difference'], units='image intensity difference, not physical motion')
        motion = None if case == 'other_activity' else plan['activity_motion_comparison']
        if version == 2 and motion is not None:
            motion = dict(relation=motion['relation'], pairs=[
                {k:p[k] for k in ('displacement', 'selected', 'matched', 'matched_cells', 'error')}
                for p in motion['pairs']])
        assert evidence['motion_comparison'] == motion
        expected_keys = {'basis', 'candidates', 'motion_comparison', 'observation',
                         'repeat_sequence', 'reviewed_pair_symbol'}
        if version == 2:
            expected_keys.add('measured_image_difference')
        assert set(evidence) == expected_keys
        response = stage['response']
        assert response['done'] and response['done_reason'] == 'stop'
        assert response['message']['role'] == 'assistant'
        assert not response['message'].get('tool_calls')
        assert 0 < response['prompt_eval_count'] <= 16384-800-512
        assert stage['options'] == dict(temperature=0, seed=42, num_ctx=16384, num_predict=800)
        proposal = json.loads(response['message']['content'])
        assert set(proposal) == {'order', 'reason', 'uncertainty'}
        preference = receipt['preference']
        assert preference['proposal'] == proposal
        assert preference['input_sha256'] == receipt['input_sha256']
        assert preference['policy'] == 'observed_response_priority_v1'
        order = proposal['order']
        assert len(order) <= 8 and len(order) == len(set(order))
        assert all(type(i) is int and 0 <= i < len(expected['candidates']) for i in order)
        assert all(0 < len(proposal[k]) <= 400 for k in ('reason', 'uncertainty'))
        full_order = order+[i for i in range(len(expected['candidates'])) if i not in order]
        paths = [expected['candidates'][i]['row']['path_id'] for i in full_order]
        replay_sha = None
        if version == 2:
            checked_path = directory/f'check-{case}.json'
            checked = read(checked_path)
            assert checked == dict(checked=True, input_sha256=receipt['input_sha256'],
                ordered_paths=paths, model_calls=0, field_writes=0, new_images=0)
            replay_sha = sha(checked_path)
        cases.append(dict(case=case, abstained=not order, nominated_indices=order,
            effective_order_paths=paths, known_positive_position=paths.index(positive_path)+1,
            elapsed_ms=stage['elapsed_ms'], prompt_tokens=response['prompt_eval_count'],
            input_sha256=sha(ip), receipt_sha256=sha(rp), typed_replay_sha256=replay_sha))
    results.append(dict(version=version, frozen_sha256=sha(directory/'frozen.json'),
        evaluator_sha256=frozen['evaluator_sha256'],
        evaluator_preserved=version == 2, cases=cases,
        summed_model_stage_ms=sum(c['elapsed_ms'] for c in cases),
        same_effective_order_after_reversal=cases[0]['effective_order_paths']==cases[1]['effective_order_paths']))
assert len(identities) == 1
result = dict(artifact_audit='passed', default_desktop_integration=False,
    accuracy_claim=False, development_case=True, progress_percent=36,
    model_identity=list(identities)[0], source_comparison_sha256=sha(native_path),
    known_positive_path=positive_path, model_calls=8, new_images=0,
    field_writes=0, runs=results,
    limitations=['V1 executable was not preserved; its recorded hash is historical only.',
        'V1 exact typed input digest requires its frozen implementation; this audit checks input transformations and prompt evidence.',
        'Reversal has one run per ordering, so order sensitivity is not separated from server nondeterminism.',
        'Brightness-only and opaque inputs are synthetic checks, not new gameplay evidence.',
        'The audit target label comes from an existing development trial and is never supplied to inference.'])
(Path(__file__).parent/'results.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k != 'runs'}, indent=2))
