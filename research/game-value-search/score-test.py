"""Owned synthetic receipts test scoring exclusions, never the discovery product."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile

spec=importlib.util.spec_from_file_location('scorer',Path(__file__).with_name('score.py'))
scorer=importlib.util.module_from_spec(spec)
spec.loader.exec_module(scorer)

with tempfile.TemporaryDirectory() as tmp:
    directory=Path(tmp)
    def save(name,value):
        (directory/name).write_text(json.dumps(value),encoding='utf-8')
    states=[]; oracle=[]
    outcomes=['correct','wrong','conflicting','abstention','unscorable','unscorable']
    for index,outcome in enumerate(outcomes):
        start=1000+index*1000
        for offset in [-30,40]:
            stamp=start+offset
            oracle.append({'time_ms':stamp,'request_ms':stamp-1,'response_ms':stamp+1,
                'geometry_supported':index!=5,'viewport':{'width':800,'height':600,'dpr':1},
                'values':{key:{'value':128,'bounds':[20,30,100,40],
                    'raw_html':'<div>128<div>+'+str(offset)+'</div></div>'} for key in ['score','best']}})
        if index==4:
            oracle[-2]['request_ms']-=300
            oracle[-2]['time_ms']=(oracle[-2]['request_ms']+oracle[-2]['response_ms'])/2
        values={'correct':[128],'wrong':[127],'conflicting':[127,128],'abstention':[None],'unscorable':[128]}[outcome]
        live={'sample_count':index+1,'coverage':{'batch':1},
            'latest':{'screen':{'width':800,'height':600},'screen_started_unix_ms':start,'screen_capture_ms':10},
            'tracks':[{'id':i,'label':'Number','bounds':[20,30,100,40],'active':True,'samples':[value],'matches':[]} for i,value in enumerate(values)]}
        states.append({'evaluation_phase':'initial','state':{'live':live}})
    save('states.json',states);save('oracle.json',oracle);save('delivered-keys.json',[])
    save('final-status.json',states[-1]);save('run.json',{'game':'2048','replicate':'test','errors':[],
        'geometry':{'width':800,'height':600},'injected_pulses':0,'searchOutcome':'test'})
    with contextlib.redirect_stdout(io.StringIO()):result=scorer.score(directory)
    actual=[row['status'] for row in result['rows'] if row['counter']=='score']
    assert actual==outcomes,(actual,outcomes)
    assert result['captured']==6
    print(json.dumps({'scoring_outcomes':actual,'changing_child_markup':'ignored','passed':True}))
