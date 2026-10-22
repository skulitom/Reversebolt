"""Frozen sequence replay using original saved pixels, without any live target."""
import copy, hashlib, json, shutil, subprocess, time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'test-results/appearance-sequence-v1'
OUT.mkdir()
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
    with p.open('rb') as f: return hashlib.file_digest(f, 'sha256').hexdigest()
def save(p, value):
    with p.open('x', encoding='utf-8') as f: f.write(json.dumps(value, indent=2) + '\n')
controls_path = ROOT / 'test-results/region-appearance-v1/controls/cases.json'
controls = read(controls_path)
a, b = controls[0]['frames']
replacement = controls[4]['frames'][1]
patterns = [[a,b,a,a], [a,b,replacement,a], [a,b,a], [a]*10, [a]*10]
cases = []
for index, pattern in enumerate(patterns):
    case = {'name':f'sequence-{index}', 'investigation_id':f'sequence-{index}', 'frames':[], 'visual_tracks':[]}
    numbers = [1,2,4] if index == 2 else list(range(1,len(pattern)+1))
    for number, image in zip(numbers, pattern, strict=True):
        frame = copy.deepcopy(image)
        frame['reference'].update(id=f'sequence-{index}-frame-{number}', number=number,
            started_unix_ms=number, capture_interval={'clock_id':case['investigation_id'], 'begin_us':number*1_000_000, 'end_us':number*1_000_000+1000})
        case['frames'].append(frame)
    for source in range(4 if index == 3 else 1):
        track = copy.deepcopy(controls[0]['visual_tracks'][0])
        observation = track['observations'][0]
        track.update(id=source, key=f'appearance-{source}')
        observation['input'].update(investigation_id=case['investigation_id'],frame_id=case['frames'][0]['reference']['id'],
            key=track['key'], bounds=[64-source%2,80-source//2,64+source%2,64+source//2])
        track['observations']=[observation]+[None]*(max(numbers)-1)
        case['visual_tracks'].append(track)
    cases.append(case)
truth = [
    {'name':'sequence-0','accepted':[3],'attempts':3,'stop':'end_of_requested_frames','deltas':[[160,64],[-160,-64],[0,0]]},
    {'name':'sequence-1','accepted':[1],'attempts':2,'stop':'unresolved_appearance'},
    {'name':'sequence-2','accepted':[1],'attempts':1,'stop':'omitted_capture'},
    {'name':'sequence-3','accepted':[3,3,3,3],'attempts':12,'stop':'analysis_budget'},
    {'name':'sequence-4','accepted':[6],'attempts':6,'stop':'step_limit'},
]
prior_manifest = ROOT / 'test-results/observed-regions-v1/cases.json'
prior_result = ROOT / 'test-results/region-appearance-v1/result.json'
cases += read(prior_manifest)
manifest = OUT / 'cases.json';save(manifest,cases);save(OUT/'truth.json',truth)
inputs = [manifest, OUT/'truth.json', controls_path, prior_manifest, prior_result]
for case in cases:
    for frame in case['frames']:
        p=Path(frame['file']);assert sha(p)==frame['reference']['sha256'];inputs.append(p)
save(OUT/'inputs-frozen.json',{str(p):sha(p) for p in inputs})
binary=OUT/'observed-region-replay.exe'
shutil.copy2(ROOT/'target/debug/examples/observed-region-replay.exe',binary)
sources=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
sources += [ROOT/'Cargo.toml',ROOT/'Cargo.lock',Path(__file__),Path(__file__).with_name('PROTOCOL.md'),Path(__file__).with_name('audit.py')]
hashes={}
for p in sources:
    relative=p.relative_to(ROOT).as_posix();target=OUT/'source'/relative
    target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,target);hashes[relative]=sha(target)
save(OUT/'source-frozen.json',{'parent':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    'sources':hashes,'binaries':{str(binary):sha(binary)}})
start=time.monotonic()
r=subprocess.run([str(binary),str(manifest),str(OUT/'result.json')],cwd=ROOT,capture_output=True,text=True,
    timeout=60,creationflags=subprocess.CREATE_NO_WINDOW)
receipt={'exit_code':r.returncode,'seconds':time.monotonic()-start,'stdout':r.stdout,'stderr':r.stderr}
save(OUT/'process.json',receipt);print(json.dumps(receipt));assert r.returncode==0
