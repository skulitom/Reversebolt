"""One fixed synthetic batch per request. Never retry or tune after results."""
import hashlib
import json
import subprocess
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/runtime-relevance-challenge-v1'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen = read(OUT / 'frozen.json')
binary = OUT / 'review.exe'
assert sha(binary) == frozen['binary_sha256']
for case in frozen['cases']:
    source = Path(case['source'])
    assert sha(source) == case['source_sha256']
    destination = OUT / case['name']
    assert not destination.exists(), 'Never rerun a started case'
    with urllib.request.urlopen('http://127.0.0.1:11434/api/tags', timeout=10) as response:
        assert any(m['name'] == frozen['model']['name'] and m['digest'] == frozen['model']['digest']
                   for m in json.load(response)['models'])
    begin = time.monotonic()
    process = subprocess.Popen([str(binary), str(source), str(destination), frozen['model']['name'], 'synthetic'],
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=subprocess.CREATE_NO_WINDOW)
    stdout, stderr = process.communicate(timeout=180)
    saved = {'pid': process.pid, 'exit_code': process.returncode, 'seconds': time.monotonic() - begin,
             'stdout': stdout.decode('utf-8', 'replace'), 'stderr': stderr.decode('utf-8', 'replace')}
    (OUT / (case['name'] + '-process.json')).write_text(json.dumps(saved, indent=2) + '\n', encoding='utf-8')
    if process.returncode:
        raise RuntimeError(saved)
    receipt = read(destination / 'receipt.json')
    print(json.dumps({'case': case['name'], 'error': receipt['error'],
        'categories': None if receipt['assessment'] is None else [j['category'] for j in receipt['assessment']['judgments']]}), flush=True)
