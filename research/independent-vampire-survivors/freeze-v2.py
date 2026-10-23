"""Freeze an unchanged product/evaluator for an independent installed game."""
import hashlib,json,shutil,subprocess,urllib.request
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/independent-vampire-survivors-v2'
BIN=ROOT/'dist/0.1.0-20261023-074606'
GAME=Path('D:/Steam/steamapps/common/Vampire Survivors')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
with urllib.request.urlopen('http://127.0.0.1:11434/api/tags',timeout=10) as response:models=json.load(response)['models']
model=next(m for m in models if m['name']=='qwen3.8:latest')
assert model['digest']=='22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643'
assert (ROOT/'dist/current.txt').read_text(encoding='utf-8-sig').strip()==BIN.name
prior=json.loads((ROOT/'test-results/independent-vampire-survivors-v1/frozen.json').read_text(encoding='utf-8'))
assert all(sha(GAME/name)==digest for name,digest in prior['game_files'].items()), 'Installed game changed; retain setup failure before selecting another build'
OUT.mkdir();(OUT/'source').mkdir()
sources=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
sources += [ROOT/p for p in ['Cargo.toml','Cargo.lock','rust-toolchain.toml','research/independent-runtime/run.py',
    'research/independent-vampire-survivors/PROTOCOL-v2.md','research/independent-vampire-survivors/freeze-v2.py']]
digests={}
for path in sources:
    name=path.relative_to(ROOT).as_posix();dest=OUT/'source'/name;dest.parent.mkdir(parents=True,exist_ok=True)
    shutil.copy2(path,dest);digests[name]=sha(dest)
manifest=Path('D:/Steam/steamapps/appmanifest_1794680.acf')
shutil.copy2(manifest,OUT/'appmanifest-before.acf')
storage=[Path.home()/'AppData/Roaming/Vampire_Survivors',Path('D:/Steam/userdata/95209006/1794680'),Path.home()/'AppData/LocalLow/poncle/Vampire Survivors']
snapshots=[]
for i,path in enumerate(storage):
    assert path.is_dir()
    snapshots.append({'path':str(path),'backup':f'storage-before/{i}',
        'hashes':{p.relative_to(path).as_posix():sha(p) for p in path.rglob('*') if p.is_file()}})
    shutil.copytree(path,OUT/f'storage-before/{i}')
frozen={'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        'bin':str(BIN),'data':str(OUT/'data'),'appid':'1794680','game':str(GAME),'sources':digests,
        'binaries':{p.name:sha(p) for p in BIN.iterdir() if p.suffix in ['.exe','.dll']},
        'model':{k:model[k] for k in ['name','digest','size']},'storage':snapshots,
        'appmanifest_sha256':sha(manifest),'game_files':{name:sha(GAME/name) for name in ['VampireSurvivors.exe','GameAssembly.dll','UnityPlayer.dll']},
        'maximum_setup_images':6,'maximum_setup_actions':8,'maximum_comparisons':1,'main_editor_pid':98332,'seat_session':3}
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'output':str(OUT),'sources':len(digests),'storage_files':sum(len(s['hashes']) for s in snapshots)}))
