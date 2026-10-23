"""Exactly one fresh run for each frozen case under the revised contract."""
import hashlib
import json
import subprocess
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/evidence-references-v2'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
assert all(sha(OUT / name) == digest for name, digest in frozen['binaries'].items())
for case in frozen['cases']:
    assert all(sha(Path(name)) == digest for name, digest in case['hashes'].items())
    destination = OUT / case['name']
    assert not destination.exists(), 'Never repeat a started case'
    with urllib.request.urlopen('http://127.0.0.1:11434/api/tags', timeout=10) as response:
        assert any(m['name'] == frozen['model']['name'] and m['digest'] == frozen['model']['digest']
                   for m in json.load(response)['models'])
    command = [str(OUT / ('review.exe' if case['kind'] == 'review' else 'naming.exe')),
               case['source'], str(destination), frozen['model']['name']]
    if case['variation']:
        command.append(case['variation'])
    begin = time.monotonic()
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               creationflags=subprocess.CREATE_NO_WINDOW)
    stdout, stderr = process.communicate(timeout=1200)
    saved = {'pid': process.pid, 'exit_code': process.returncode, 'seconds': time.monotonic() - begin,
             'stdout': stdout.decode('utf-8', 'replace'), 'stderr': stderr.decode('utf-8', 'replace')}
    (OUT / (case['name'] + '-process.json')).write_text(json.dumps(saved, indent=2) + '\n', encoding='utf-8')
    if process.returncode:
        raise RuntimeError(saved)
    receipt = read(destination / 'receipt.json')
    batch = receipt.get('batch')
    print(json.dumps({'case': case['name'], 'error': receipt['error'],
        'accepted': None if batch is None else len(batch['accepted']), 'rejected': None if batch is None else len(batch['rejected']),
        'assessment': receipt.get('assessment') is not None,
        'model_calls': len(receipt['stages']) if 'stages' in receipt else int(receipt['stage'] is not None)}), flush=True)
