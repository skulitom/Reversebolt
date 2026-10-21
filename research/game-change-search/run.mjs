// Evaluation only. No game-specific reader or control schedule enters the product.
import {createRequire} from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import crypto from 'node:crypto';
import {spawn, spawnSync} from 'node:child_process';
import readline from 'node:readline';
import {createOracle} from './oracle.mjs';
const require = createRequire(import.meta.url);
const {chromium} = require(process.env.REVERSEBOLT_PLAYWRIGHT);
const root = path.resolve(import.meta.dirname, '../..');
const config = JSON.parse(fs.readFileSync(path.join(import.meta.dirname, 'protocol-v1.json'), 'utf8'));
const [game, replicate, outArg] = process.argv.slice(2);
if (!['2048','tetris'].includes(game) || replicate!=='1') throw Error('Use 2048|tetris, replicate 1, new output directory');
const out = path.resolve(outArg), bin = config.binary_directory;
fs.mkdirSync(out, {recursive:false});
const write = (name, value) => fs.writeFileSync(path.join(out,name), JSON.stringify(value,null,2));
const sha = file => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
for (const [name, hash] of Object.entries(config.binaries)) if (sha(path.join(bin,name))!==hash) throw Error('Product binary changed');
for(const [name,hash] of Object.entries(config.method_sha256))if(sha(path.resolve(import.meta.dirname,name))!==hash)throw Error('Evaluation method changed: '+name);
const upstream = path.join(root,'test-results/independent-games-v1/upstream');
if(sha(path.join(upstream,'manifest.json'))!==config.upstream_manifest_sha256)throw Error('Upstream manifest changed');
const manifest = JSON.parse(fs.readFileSync(path.join(upstream,'manifest.json'),'utf8'));
const source = manifest.find(s=>s.name===game), sourceRoot=path.join(upstream,game);
for (const [file,hash] of Object.entries(source.files)) if(sha(path.join(sourceRoot,file))!==hash) throw Error('Upstream source changed: '+file);
const anode=path.join(process.env.LOCALAPPDATA,'Programs/Anode/anode.exe');
function cli(args) {
  const r=spawnSync(anode,args,{encoding:'utf8',windowsHide:true,timeout:20000});
  if(r.status!==0)throw Error(r.stderr||r.stdout||r.error?.message);
  return r.stdout;
}
let server, context, engine, mcp, input, polling=false, oracleTask, serial=0;
const pending=new Map(), states=[], oracles=[], delivered=[], injected=[], blocked=[], errors=[];
let lastStatus, geometry, browserProcesses, selected, browserVersion, scopeSelection;
let inputGeneration=0, searchOutcome='not attempted', readOracle;
let changeId, trialOutcome='not attempted', restoreAttempted=false;
const trialStages=[];
function stamp(stage){trialStages.push({stage,time_ms:Date.now()});write('trial-stages.json',trialStages);}
const workflow=[];
function rpc(method,params) {
  const id=++serial;
  return new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>{pending.delete(id);reject(Error('MCP response timed out; do not replay mutation'));},120000);
    pending.set(id,{resolve:v=>{clearTimeout(timer);resolve(v)},reject:e=>{clearTimeout(timer);reject(e)}});
    mcp.stdin.write(JSON.stringify({jsonrpc:'2.0',id,method,params})+'\n');
  });
}
async function call(name,args={},required=true) {
  const r=await rpc('tools/call',{name:'reversebolt_'+name,arguments:args});
  const v=JSON.parse(r.content.filter(c=>c.type==='text').at(-1).text);
  if(name!=='status'){workflow.push({name,args,ok:v.ok,error:v.error});write('commands.json',workflow);}
  if(required&&(r.isError||!v.ok))throw Error(name+': '+v.error);
  return v;
}
async function inputBurst(pulses) {
  const mode={generation:++inputGeneration,pulses};
  const target=path.join(out,'input-control.json'),temporary=target+'.pending';
  fs.writeFileSync(temporary,JSON.stringify(mode));fs.renameSync(temporary,target);
  fs.appendFileSync(path.join(out,'input-control.jsonl'),JSON.stringify({...mode,requested_ms:Date.now()})+'\n');
  const deadline=Date.now()+5000;
  while(Date.now()<deadline){if(injected.some(v=>v.kind==='burst_complete'&&v.generation===mode.generation))return;await sleep(20);}
  throw Error('Input burst completion absent; no replay');
}
async function jobControl(operation,job) {
  const state=await call('status'),ledger=state.jobs.history;
  return call('job_control',{request_id:(ledger.clients['mcp-agent']?.high_water??0)+1,boot:ledger.boot,binding:ledger.current,job,operation});
}
async function pauseJob(job,count) {
  await jobControl('pause',job);
  const deadline=Date.now()+15000;
  while(Date.now()<deadline){
    const state=await call('status');
    if(state.state.live.sample_count!==count)throw Error('An extra capture began before pause; preserve failed case');
    if(state.jobs.history.jobs[job].phase==='Paused')return;
    await sleep(50);
  }
  throw Error('Pause outcome unconfirmed; no replay');
}

try {
  if(!process.env.ANODE_AGENT_ID||!process.env.ANODE_LEASE_TOKEN)throw Error('Anode CLI identity/lease absent; no game launch');
  // Validate the child-session identity before any headed launch.
  const guard=spawnSync(process.env.REVERSEBOLT_PYTHON,['-c',
    'import ctypes,os; s=ctypes.c_ulong(); assert ctypes.windll.kernel32.ProcessIdToSessionId(os.getpid(),ctypes.byref(s)); assert s.value==int(os.environ["REVERSEBOLT_TEST_SESSION"]) and s.value!=int(os.environ["REVERSEBOLT_PARENT_SESSION"])'],{windowsHide:true,encoding:'utf8'});
  if(guard.status!==0)throw Error('Anode session guard failed: '+guard.stderr);
  server=http.createServer((req,res)=>{
    let requested;
    try{requested=decodeURIComponent(new URL(req.url,'http://localhost').pathname);}catch{res.writeHead(400).end();return;}
    const file=path.resolve(sourceRoot,'.'+(requested.endsWith('/')?requested+'index.html':requested));
    if(!file.startsWith(sourceRoot+path.sep)){res.writeHead(403).end();return;}
    const types={'.html':'text/html','.js':'application/javascript','.css':'text/css','.png':'image/png','.jpg':'image/jpeg','.svg':'image/svg+xml'};
    fs.readFile(file,(error,bytes)=>{if(error){res.writeHead(404).end();return;}res.writeHead(200,{'Content-Type':types[path.extname(file)]||'application/octet-stream','Cache-Control':'no-store'});res.end(bytes);});
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const url='http://127.0.0.1:'+server.address().port+'/';
  context=await chromium.launchPersistentContext(path.join(out,'browser-profile'),{executablePath:config.browser,
    headless:false,viewport:null,args:['--window-position=40,30','--window-size=1120,650','--force-device-scale-factor=1','--no-first-run']});
  await context.route('**/*',route=>{
    if(route.request().url().startsWith(url))return route.continue();
    blocked.push(route.request().url());return route.abort();
  });
  const page=context.pages()[0]||await context.newPage();
  await page.exposeBinding('__evaluationKey',(_,value)=>delivered.push(value));
  await page.addInitScript(()=>{
    document.addEventListener('keydown',e=>window.__evaluationKey({time_ms:Date.now(),kind:'down',code:e.code,key:e.key,trusted:e.isTrusted,repeat:e.repeat}),true);
    document.addEventListener('keyup',e=>window.__evaluationKey({time_ms:Date.now(),kind:'up',code:e.code,key:e.key,trusted:e.isTrusted}),true);
  });
  await page.goto(url,{waitUntil:'networkidle'});
  readOracle=await createOracle(context,page,config.oracle_selectors[game]);
  await page.bringToFront();
  const cdp=await context.browser().newBrowserCDPSession();
  browserProcesses=(await cdp.send('SystemInfo.getProcessInfo')).processInfo;
  browserVersion=await cdp.send('Browser.getVersion');
  const browserPid=browserProcesses.find(p=>p.type==='browser').id;
  const windows=JSON.parse(cli(['windows','--pid',String(browserPid),'--json'])).windows;
  const window=windows.find(w=>w.title.includes(game==='2048'?'2048':'Javascript Tetris'));
  if(!window)throw Error('Owned game window absent');
  cli(['window',window.windowId,'focus']);
  engine=spawn(path.join(bin,'Reversebolt.exe'),['--headless','--data-dir',path.join(out,'engine')],{windowsHide:true});
  const endpointFile=path.join(out,'engine/endpoint.json');
  for(let i=0;i<200&&!fs.existsSync(endpointFile);i++){if(engine.exitCode!==null)throw Error('Engine startup failed');await sleep(50);}
  if(!fs.existsSync(endpointFile))throw Error('Engine startup timed out');
  mcp=spawn(path.join(bin,'reversebolt-mcp.exe'),['--data-dir',path.join(out,'engine')],{windowsHide:true,stdio:['pipe','pipe','pipe']});
  mcp.stderr.on('data',data=>fs.appendFileSync(path.join(out,'mcp-stderr.log'),data));
  readline.createInterface({input:mcp.stdout}).on('line',line=>{
    try{const r=JSON.parse(line),p=pending.get(r.id);if(p){pending.delete(r.id);r.error?p.reject(Error(JSON.stringify(r.error))):p.resolve(r.result)}}catch(e){errors.push(String(e))}
  });
  await rpc('initialize',{protocolVersion:'2025-11-25',capabilities:{},clientInfo:{name:'independent-game-evaluation',version:'1'}});
  mcp.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n');
  const tools=await rpc('tools/list',{});if(tools.tools.length!==35)throw Error('Unexpected MCP tool contract');
  selected=(await call('list_processes')).state.processes.find(p=>p.pid===browserPid&&p.title.includes(game==='2048'?'2048':'Javascript Tetris'));
  if(!selected)throw Error('Selected browser not in Reversebolt inventory');
  await call('set_observer_model',{model:config.model});
  polling=true;
  oracleTask=(async()=>{while(polling){try{oracles.push(await readOracle())}catch(e){errors.push('oracle: '+e.message)}await sleep(config.oracle_interval_ms)}})();
  lastStatus=await call('attach_process',{pid:selected.pid,created:selected.created,window:selected.window,budget_mib:config.budget_mib,auto_observe:false});
  write('attached.json',lastStatus);
  const plan={...config.input[game],session:Number(process.env.REVERSEBOLT_TEST_SESSION),parent_session:Number(process.env.REVERSEBOLT_PARENT_SESSION),
    pid:selected.pid,window:selected.window,max_seconds:config.input_seconds,output:out,stop_file:path.join(out,'stop-input'),control_file:path.join(out,'input-control.json')};
  write('input-config.json',plan);
  write('input-control.json',{generation:0,pulses:1});
  input=spawn(process.env.REVERSEBOLT_PYTHON,[path.resolve(import.meta.dirname,'input.py'),path.join(out,'input-config.json')],{windowsHide:true,stdio:['ignore','pipe','pipe']});
  input.stderr.on('data',data=>fs.appendFileSync(path.join(out,'input-stderr.log'),data));
  readline.createInterface({input:input.stdout}).on('line',line=>{const value=JSON.parse(line);injected.push(value);if(value.kind==='geometry')geometry=value;fs.appendFileSync(path.join(out,'input.jsonl'),line+'\n')});
  const cpuBefore=(await cdp.send('SystemInfo.getProcessInfo')).processInfo;
  await sleep(config.cpu_probe_ms);
  const cpuAfter=(await cdp.send('SystemInfo.getProcessInfo')).processInfo;
  const sources=(await call('list_memory_processes')).data;
  if(sources.error)throw Error(sources.error);
  const candidates=cpuAfter.filter(p=>p.type==='renderer').map(p=>({
    ...p,delta:p.cpuTime-(cpuBefore.find(q=>q.id===p.id)?.cpuTime??p.cpuTime),
    identity:sources.processes.find(q=>q.pid===p.id)
  })).filter(p=>p.identity&&p.delta>0).sort((a,b)=>b.delta-a.delta||a.id-b.id);
  if(!candidates.length)throw Error('No active accessible renderer; no fallback selection.');
  scopeSelection={rule:'Largest positive renderer CPU delta; PID tie-break',cpuBefore,cpuAfter,sources,candidates,chosen:candidates[0]};
  write('scope-selection.json',scopeSelection);
  lastStatus=await call('attach_process',{pid:selected.pid,created:selected.created,window:selected.window,
    memory_pid:candidates[0].id,memory_created:candidates[0].identity.created,budget_mib:config.budget_mib,auto_observe:true});
  write('attached.json',lastStatus);
  async function collect(stage,total=16,jobOverride) {
    let count=0;
    const job=jobOverride??lastStatus.data.automatic_observation.ack.job;
    const batch=lastStatus.state.live.coverage.batch;
    const deadline=Date.now()+config.run_timeout_seconds*1000;
    while(Date.now()<deadline){
      lastStatus=await call('status');
      let current=lastStatus;
      if(stage==='selection'&&lastStatus.state.live.coverage.batch===batch+1){
        const archived=lastStatus.state.live.coverage.batches.find(b=>b.number===batch);
        if(!archived)throw Error('Automatic transition lost its selection archive');
        const saved=JSON.parse(fs.readFileSync(archived.archive,'utf8'));
        current={...lastStatus,state:{...lastStatus.state,live:saved.state},selection_state_from_archive:true};
      }
      if(current.state.live.coverage.batch!==batch)throw Error('Unexpected batch change during collection');
      current={...current,evaluation_phase:stage};
      const live=current.state.live;
      if(live.sample_count>count){
        if(live.sample_count!==count+1)throw Error('Capture skipped by evaluator; no replacement');
        count=live.sample_count;
        if(count<total){
          await pauseJob(job,count);
          if(stage==='selection'||count<12)await inputBurst(4);
          if(stage!=='selection'&&count===14)await inputBurst(12);
          await jobControl('resume',job);
        }
        states.push(current);write('states.json',states);
        console.log(JSON.stringify({game,replicate,stage,frame:count,memory_pid:live.memory_process.pid,matches:live.tracks.reduce((n,t)=>n+t.matches.length,0)}));
      }
      const phase=lastStatus.jobs.history.jobs[job].phase;
      if(['Completed','Failed','Cancelled','Interrupted'].includes(phase)&&!lastStatus.state.analysis){
        if(phase!=='Completed')throw Error('Observation job '+phase);
        if(count!==total)throw Error('Automatic capture count differed');
        states[states.length-1]=current;write('states.json',states);
        write(stage+'-completed.json',current);
        return;
      }
      if(input.exitCode!==null)throw Error('Input driver stopped before collection finished.');
      await sleep(100);
    }
    throw Error('Observation budget exhausted; original job evidence retained.');
  }
  await collect('initial');
  const live=lastStatus.state.live;
  const eligible=live.tracks.filter(t=>{
    const valid=t.samples.filter(Number.isFinite).sort((a,b)=>a-b);
    return t.active && Number.isFinite(t.samples.at(-1)) && valid.length>=10 && valid.at(-2)-valid[1]>2.2;
  }).sort((a,b)=>a.id-b.id);
  write('track-selection.json',{rule:'Lowest ID active current OCR track; >=10 valid readings; second highest minus second lowest >2.2',eligible,selected:eligible[0]??null});
  if(!eligible.length)searchOutcome='abstained: no eligible changing current OCR track';
  else {
    const started=await call('start_change_search',{investigation_id:live.investigation_id,batch:live.coverage.batch,after_frame:16,track:eligible[0].id},false);
    write('started.json',started);lastStatus=started;
    if(!started.ok)searchOutcome='start refused: '+started.error;
    else {
      await collect('selection',4);
      const deadline=Date.now()+120000;
      let summary;
      while(Date.now()<deadline){
        lastStatus=await call('status');summary=lastStatus.state.live.change_searches.at(-1);
        if(summary.status!=='collecting')break;
        await sleep(100);
      }
      if(summary.status==='collecting')throw Error('Selection did not settle; no replay');
      write('search-summary.json',summary);
      if(fs.existsSync(summary.evidence))fs.copyFileSync(summary.evidence,path.join(out,'selection.json'));
      if(summary.status!=='completed')searchOutcome='selection '+summary.status+': '+(summary.reason??lastStatus.state.error??'no fresh batch');
      else {
        const targetPrefix=lastStatus.state.live.investigation_id+'/'+lastStatus.state.live.coverage.batch+'/';
        const deadline=Date.now()+15000;let children=[];
        while(Date.now()<deadline){
          lastStatus=await call('status');
          children=Object.values(lastStatus.jobs.history.jobs).filter(j=>j.binding.target.startsWith(targetPrefix));
          if(children.length)break;
          await sleep(50);
        }
        if(children.length!==1)throw Error('Expected exactly one automatic fresh job');
        write('fresh-start.json',lastStatus);
        await collect('focused',16,children[0].id);
        searchOutcome='completed four-moment selection and fresh sixteen-moment batch';
        const fresh=lastStatus.state.live;
        const trialChecks=fresh.prediction_checks.filter(c=>c.source==='ocr').sort((a,b)=>a.track-b.track);
        const choice=trialChecks.flatMap(check=>check.candidates.map((candidate,index)=>({check,candidate,index})))
          .find(({candidate})=>candidate.samples.length===4&&candidate.samples.every(s=>s.outcome==='agrees'));
        if(!choice)trialOutcome='abstained: no candidate agreed on all four fresh moments';
        else {
          const observed=fresh.tracks.find(t=>t.id===choice.check.track)?.samples.at(-1);
          if(!Number.isFinite(observed))trialOutcome='abstained: no current readable value';
          else {
            write('trial-selection.json',{rule:'Lowest OCR check track ID, lowest candidate index with exactly four agreements',track:choice.check.track,prediction_id:choice.check.id,candidate:choice.index,observed,requested:observed+10});
            stamp('before_prepare');
            const prepared=await call('prepare_probe',{investigation_id:fresh.investigation_id,batch:fresh.coverage.batch,
              track:choice.check.track,prediction_id:choice.check.id,candidate:choice.index,value:observed+10},false);
            write('prepared.json',prepared);stamp('after_prepare');
            if(!prepared.ok||prepared.data.phase!=='prepared')trialOutcome='preparation refused; no fallback';
            else {
              const change=prepared.data;changeId=change.id;
              if(change.scope.process.pid!==fresh.memory_process.pid||change.scope.process.created!==fresh.memory_process.created)throw Error('Trial process identity mismatch');
              if(change.probe.selected!==choice.index||change.patch.after.length<1||change.patch.after.length>8)throw Error('Trial differs from selected candidate');
              stamp('before_apply');
              const applied=await call('apply_change',{change_id:changeId},false);write('applied.json',applied);stamp('after_apply');
              if(applied.ok&&applied.data.phase==='written'){
                await sleep(750);
                stamp('before_observe');
                write('checked.json',await call('observe_change',{change_id:changeId},false));stamp('after_observe');
                restoreAttempted=true;
                stamp('before_restore');
                const restored=await call('restore_change',{change_id:changeId},false);write('restored.json',restored);stamp('after_restore');
                if(restored.ok&&restored.data.phase==='restored'){
                  await sleep(750);stamp('before_restore_observe');
                  write('checked-restore.json',await call('observe_change',{change_id:changeId},false));stamp('after_restore_observe');
                }
                trialOutcome='written; restoration '+(restored.data?.phase??'failed');
              }else trialOutcome='application '+(applied.data?.phase??'failed');
            }
          }
        }
      }
    }
  }
  lastStatus=await call('status');write('final-status.json',lastStatus);
  browserProcesses=(await cdp.send('SystemInfo.getProcessInfo')).processInfo;
  const finalLive=lastStatus.state.live;
  if(lastStatus.state.changes.some(c=>['writing','written','uncertain','restoring'].includes(c.phase)))
    write('archive-skipped.json',{reason:'An applied or unresolved trial remains; keep the current investigation.'});
  else write('archived.json',await call('advance_memory',{investigation_id:finalLive.investigation_id,batch:finalLive.coverage.batch,auto_observe:false},false));

} catch(error) {
  errors.push(error.stack||String(error));process.exitCode=1;
} finally {
  if(changeId&&!restoreAttempted){
    try{
      const current=await call('status');const change=current.state.changes.find(c=>c.id===changeId);
      if(change?.phase==='written'){
        restoreAttempted=true;stamp('cleanup_restore');write('cleanup-restore.json',await call('restore_change',{change_id:changeId},false));
      }
      lastStatus=await call('status');
    }catch(error){errors.push('Checked cleanup: '+String(error));}
  }
  polling=false;
  fs.writeFileSync(path.join(out,'stop-input'),'stop');
  if(input&&input.exitCode===null)await Promise.race([new Promise(resolve=>input.once('exit',resolve)),sleep(3000)]);
  // The guarded driver releases every press in finally; it also has its own deadline.
  if(input&&input.exitCode===null){errors.push('Input driver did not stop promptly');process.exitCode=1;}
  if(oracleTask)await oracleTask;
  if(lastStatus)write('final-status.json',lastStatus);
  write('oracle.json',oracles);write('delivered-keys.json',delivered);
  write('run.json',{game,replicate,started_protocol_sha256:sha(path.join(import.meta.dirname,'protocol-v1.json')),
    source,selected,scopeSelection,browserVersion,browserProcesses,geometry,blocked,errors,binaries:config.binaries,searchOutcome,trialOutcome,changeId,trialStages,ownedProcesses:{engine:engine?.pid,mcp:mcp?.pid,input:input?.pid},
    captures:states.length,oracle_samples:oracles.length,injected_pulses:injected.filter(v=>v.kind==='pulse').length,delivered_keys:delivered.length});
  if(mcp)mcp.kill();if(engine)engine.kill();
  if(context)await context.close();
  if(server)await new Promise(resolve=>server.close(resolve));
  console.log(JSON.stringify({game,replicate,finished:true,errors:errors.length,captures:states.length,oracle_samples:oracles.length}));
}
