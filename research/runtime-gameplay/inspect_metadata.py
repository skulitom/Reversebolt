"""Offline metadata/observation coverage query; never reads a live target or game source."""
import argparse
import collections
import hashlib
import json
from pathlib import Path


def inspect(report, query):
    terms = query.casefold().split()
    if not terms or len(terms) > 12 or len(query) > 96:
        raise ValueError('Use 1-12 terms, at most 96 characters')
    classes = {c['id']: c for c in report['classes']}
    if len(classes) != len(report['classes']):
        raise ValueError('Duplicate class identity')
    nodes = collections.Counter()
    reads = collections.Counter()
    roots = collections.defaultdict(list)
    for shot in report['snapshots']:
        for node in shot['nodes']:
            if node['class'] is not None:
                nodes[node['class']] += 1
        for reading in shot['reads']:
            if reading['read_ok'] and reading['value'] is not None:
                reads[(reading['declaring_class'], reading['field'])] += 1
        for root in shot['roots']:
            roots[root['class']].append({
                'field_id': root['field'], 'read_ok': root['read_ok'],
                'null_reference': root['read_ok'] and int(root['reference'], 16) == 0,
                'snapshot_id': shot['snapshot_id'],
            })
    matches = []
    for class_id, klass in classes.items():
        for field in klass['fields']:
            searchable = ' '.join((klass['key']['type_name'], klass['image'], field['name'])).casefold()
            if all(term in searchable for term in terms):
                matches.append({
                    'type': klass['key'], 'field_id': field['id'], 'field_name': field['name'],
                    'type_code': field['type_code'], 'width': field['width'],
                    'reference': field['reference'], 'static': bool(field['flags'] & 0x10),
                    'observed_node_occurrences': nodes[class_id],
                    'successful_scalar_read_occurrences': reads[(class_id, field['id'])],
                    'roots_declared_on_type': roots[class_id],
                })
    return {'query': query, 'matches': len(matches), 'fields': matches[:100],
            'truncated': len(matches) > 100, 'source_scope': {k: report[k] for k in ('pid', 'created', 'session')},
            'limitation': 'Metadata names are hypotheses. Zero observed occurrences means not seen within the bounded retained traversal, not absence from the process. A root declared on a type need not point to an instance of that type. No effect or edit authority is established.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('report', type=Path)
    parser.add_argument('query')
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    data = args.report.read_bytes()
    if len(data) > 64 * 1024 * 1024:
        raise ValueError('Report exceeds 64 MiB')
    result = inspect(json.loads(data), args.query)
    result['source_path'] = str(args.report.resolve())
    result['source_sha256'] = hashlib.sha256(data).hexdigest()
    with args.output.open('x', encoding='utf-8') as file:
        json.dump(result, file, indent=2)
    print(json.dumps({'query': result['query'], 'matches': result['matches'], 'truncated': result['truncated']}))
