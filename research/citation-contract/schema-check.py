"""Independent JSON Schema checks of the compiled schema and retained responses."""
import copy
import hashlib
import itertools
import json
from pathlib import Path
from jsonschema import Draft202012Validator as Validator

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/citation-contract-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
assert sha(OUT / 'schemas.json') == frozen['schemas_sha256']
schemas = read(OUT / 'schemas.json')
for schema in schemas.values():
    Validator.check_schema(schema)
checks = 0
for name in ['hypothesis_citation', 'relevance_citation']:
    validator = Validator(schemas[name])
    for source, index in itertools.product(['field_name', 'type_name', 'object_text', 'unknown'], [None, 0, 1, -1, True, '0']):
        value = {'source': source, 'index': index, 'quote': 'Observed label'}
        expected = (source in ['field_name', 'type_name'] and index is None) or (
            source == 'object_text' and type(index) is int and index >= 0)
        assert validator.is_valid(value) == expected, (name, value)
        checks += 1
    base = {'source': 'object_text', 'index': 0, 'quote': 'Observed label'}
    for key in ['source', 'index', 'quote']:
        value = copy.deepcopy(base)
        del value[key]
        assert not validator.is_valid(value)
        checks += 1
    assert not validator.is_valid({**base, 'write': True})
    assert not validator.is_valid({**base, 'quote': ''})
    assert validator.is_valid({**base, 'quote': 'x' * 257}) == (name == 'relevance_citation')
    checks += 3
historical, rejected, valid = [], [], 0
roots = [ROOT / 'test-results/runtime-relevance-v1', ROOT / 'test-results/runtime-relevance-challenge-v1']
for root in roots:
    for path in sorted(root.glob('*/receipt.json')):
        receipt = read(path)
        stage = receipt.get('stage')
        if not stage or stage['response'].get('done_reason') != 'stop':
            continue
        old = stage['schema']['properties']['judgments']['items']['properties']['citations']['items']
        for judgment in json.loads(stage['response']['message']['content'])['judgments']:
            for citation in judgment['citations']:
                old_valid = Validator(old).is_valid(citation)
                new_valid = Validator(schemas['relevance_citation']).is_valid(citation)
                assert old_valid
                if new_valid:
                    valid += 1
                else:
                    rejected.append({'source': path.relative_to(ROOT).as_posix(), 'candidate': judgment['candidate'], 'citation': citation})
        historical.append({'path': str(path), 'sha256': sha(path)})
assert len(rejected) == 6
assert all(x['source'].endswith('balanced-1/receipt.json') and x['citation']['source'] == 'object_text'
           and x['citation']['index'] is None for x in rejected)
naming_valid = 0
for path in sorted((ROOT / 'test-results/bounded-type-selection-v1/revised').glob('*/receipt.json')):
    receipt = read(path)
    for stage in receipt['stages']:
        if 'hypotheses' not in stage['schema']['properties']:
            continue
        for hypothesis in json.loads(stage['response']['message']['content'])['hypotheses']:
            for citation in hypothesis['citations']:
                assert Validator(schemas['hypothesis_citation']).is_valid(citation)
                naming_valid += 1
    historical.append({'path': str(path), 'sha256': sha(path)})
result = {'passed': True, 'matrix_checks': checks, 'historical_valid_relevance_citations_preserved': valid,
          'historical_naming_citations_preserved': naming_valid, 'formerly_allowed_invalid_citations_rejected': rejected,
          'historical_receipts': historical, 'schemas_sha256': sha(OUT / 'schemas.json')}
(OUT / 'schema-check.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
print(json.dumps({k: v for k, v in result.items() if k not in ['historical_receipts', 'formerly_allowed_invalid_citations_rejected']}))
