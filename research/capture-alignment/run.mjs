// Evaluation only. No game-specific reader or control schedule enters the product.
import {createRequire} from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import crypto from 'node:crypto';
import {spawn, spawnSync} from 'node:child_process';
import readline from 'node:readline';
const require = createRequire(import.meta.url);
const {chromium} = require(process.env.REVERSEBOLT_PLAYWRIGHT);
const root = path.resolve(import.meta.dirname, '../..');
const config = JSON.parse(fs.readFileSync(path.join(import.meta.dirname, 'protocol-v1.json'), 'utf8'));
const [game, replicate, outArg] = process.argv.slice(2);
if (!['2048', 'tetris'].includes(game) || !['1','2'].includes(replicate)) throw Error('Use game, replicate 1|2, new output directory');
const out = path.resolve(outArg), bin = config.binary_directory;
fs.mkdirSync(out, {recursive:false});
const write = (name, value) => fs.writeFileSync(path.join(out,name), JSON.stringify(value,null,2));
const sha = file => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
for (const [name, hash] of Object.entries(config.binaries)) if (sha(path.join(bin,name))!==hash) throw Error('Product binary changed');
const upstream = path.join(root,'test-results/independent-games-v1/upstream');
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
const workflow=[];
function rpc(method,params) {
  const id=++serial;
  return new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>{pending.delete(id);reject(Error('MCP response timed out; do not replay mutation'));},120000);
    pending.set(id,{resolve:v=>{clearTimeout(timer);resolve(v)},reject:e=>{clearTimeout(timer);reject(e)}});
    mcp.stdin.write(JSON.stringify({jsonrpc:'2.0',id,method,params})+'\n');
  });
}
async function call(name,args={}) {
  const r=await rpc('tools/call',{name:'reversebolt_'+name,arguments:args});
  const v=JSON.parse(r.content.filter(c=>c.type==='text').at(-1).text);
  if(name!=='status'){workflow.push({name,args,ok:v.ok,error:v.error});write('commands.json',workflow);}
  if(r.isError||!v.ok)throw Error(name+': '+v.error);
  return v;
}
async function oracle(page) {
  const before=Date.now();
  const value=await page.evaluate(game=>{
    const region=(selector,numeric)=>{
      const e=document.querySelector(selector), b=e.getBoundingClientRect();
      return {value:Number(numeric?e.firstChild.textContent:e.textContent),bounds:[b.x,b.y,b.width,b.height]};
    };
    const common={time_ms:Date.now(),viewport:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},focused:document.hasFocus()};
    if(game==='2048')return {...common,values:{score:region('.score-container',true),best:region('.best-container',false)},
      tiles:[...document.querySelectorAll('.tile-inner')].map(e=>({value:Number(e.textContent),rect:JSON.parse(JSON.stringify(e.getBoundingClientRect()))})),
      ended:document.querySelector('.game-message').classList.contains('game-over')};
    return {...common,values:{score:region('#score',false),rows:region('#rows',false)},
      actual_score:window.score,display_score:window.vscore,playing:window.playing,
      current:window.current?{x:current.x,y:current.y,dir:current.dir,color:current.type.color,shape:current.type.blocks[current.dir]}:null,
      next:window.next?{color:next.type.color,shape:next.type.blocks[next.dir]}:null};
  },game);
  oracles.push({request_ms:before,response_ms:Date.now(),...value});
}
try {
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
  const tools=await rpc('tools/list',{});if(tools.tools.length!==30)throw Error('Unexpected MCP tool contract');
  selected=(await call('list_processes')).state.processes.find(p=>p.pid===browserPid&&p.title.includes(game==='2048'?'2048':'Javascript Tetris'));
  if(!selected)throw Error('Selected browser not in Reversebolt inventory');
  await call('set_observer_model',{model:config.model});
  polling=true;
  oracleTask=(async()=>{while(polling){try{await oracle(page)}catch(e){errors.push('oracle: '+e.message)}await sleep(config.oracle_interval_ms)}})();
  lastStatus=await call('attach_process',{pid:selected.pid,created:selected.created,window:selected.window,budget_mib:config.budget_mib,auto_observe:false});
  write('attached.json',lastStatus);
  const plan={...config.input[game],session:Number(process.env.REVERSEBOLT_TEST_SESSION),parent_session:Number(process.env.REVERSEBOLT_PARENT_SESSION),
    pid:selected.pid,window:selected.window,max_seconds:config.input_seconds,output:out,stop_file:path.join(out,'stop-input')};
  write('input-config.json',plan);
  input=spawn(process.env.REVERSEBOLT_PYTHON,[path.resolve(import.meta.dirname,'../independent-games/input.py'),path.join(out,'input-config.json')],{windowsHide:true,stdio:['ignore','pipe','pipe']});
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
    memory_pid:candidates[0].id,memory_created:candidates[0].identity.created,budget_mib:config.budget_mib,auto_observe:false});
  write('attached.json',lastStatus);
  let count=0;
  async function collect(amount,compare) {
    const ledger=lastStatus.jobs.history;
    const started=await call('start_observation_job',{request_id:(ledger.clients['mcp-agent']?.high_water??0)+1,
      boot:ledger.boot,binding:ledger.current,count:amount,interval_ms:1000,mode:'full',compare});
    const job=started.data.ack.job;
    const deadline=Date.now()+config.run_timeout_seconds*1000;
    while(Date.now()<deadline){
      lastStatus=await call('status');
      const live=lastStatus.state.live;
      if(live.sample_count>count){count=live.sample_count;states.push(lastStatus);write('states.json',states);
        console.log(JSON.stringify({game,frame:count,memory_pid:live.memory_process.pid,matches:live.tracks.reduce((n,t)=>n+t.matches.length,0)}));}
      const phase=lastStatus.jobs.history.jobs[job].phase;
      if(['Completed','Failed','Cancelled','Interrupted'].includes(phase)&&!lastStatus.state.analysis){
        if(phase!=='Completed')throw Error('Observation job '+phase);
        if(states.length&&states.at(-1).state.live.sample_count===count){states[states.length-1]=lastStatus;write('states.json',states);}
        return;
      }
      if(input.exitCode!==null)throw Error('Input driver stopped before collection finished.');
      await sleep(200);
    }
    throw Error('Observation budget exhausted; original job evidence retained.');
  }
  await collect(12,true);
  write('training-status.json',lastStatus);
  const target=lastStatus.state.live.tracks.filter(t=>t.active&&t.matches.length).sort((a,b)=>a.id-b.id)[0];
  if(target){
    lastStatus=await call('start_prediction_check',{investigation_id:lastStatus.state.live.investigation_id,
      batch:lastStatus.state.live.coverage.batch,after_frame:12,source:'ocr',track:target.id});
    write('frozen-status.json',lastStatus);
  } else write('no-hypothesis.json',{reason:'No active OCR track had matches after 12 captures.'});
  await collect(8,false);
  write('final-status.json',lastStatus);
  browserProcesses=(await cdp.send('SystemInfo.getProcessInfo')).processInfo;
  try {write('after-archive.json',await call('advance_memory',{investigation_id:lastStatus.state.live.investigation_id,
      batch:lastStatus.state.live.coverage.batch,auto_observe:false}));}
  catch(error){write('archive-error.json',{error:String(error)});}

} catch(error) {
  errors.push(error.stack||String(error));process.exitCode=1;
} finally {
  polling=false;
  fs.writeFileSync(path.join(out,'stop-input'),'stop');
  if(input&&input.exitCode===null)await Promise.race([new Promise(resolve=>input.once('exit',resolve)),sleep(3000)]);
  // The guarded driver releases every press in finally; it also has its own deadline.
  if(input&&input.exitCode===null){errors.push('Input driver did not stop promptly');process.exitCode=1;}
  if(oracleTask)await oracleTask;
  if(lastStatus)write('final-status.json',lastStatus);
  write('oracle.json',oracles);write('delivered-keys.json',delivered);
  write('run.json',{game,replicate,started_protocol_sha256:sha(path.join(import.meta.dirname,'protocol-v1.json')),
    source,selected,scopeSelection,browserVersion,browserProcesses,geometry,blocked,errors,binaries:config.binaries,
    captures:states.length,oracle_samples:oracles.length,injected_pulses:injected.filter(v=>v.kind==='pulse').length,delivered_keys:delivered.length});
  if(mcp)mcp.kill();if(engine)engine.kill();
  if(context)await context.close();
  if(server)await new Promise(resolve=>server.close(resolve));
  console.log(JSON.stringify({game,replicate,finished:true,errors:errors.length,captures:states.length,oracle_samples:oracles.length}));
}
