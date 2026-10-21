(async ({root, pid, index, stage}) => {
 if (!/^C:\\DEV\\Reversebolt\\test-results\\runtime-alternatives-v1$/.test(root) || ![0,1].includes(index) || !["baseline","changed","restored"].includes(stage)) throw Error("Invalid frozen condition");
 const base=root+"\\candidate-"+index, frames=base+"\\frames", log=base+"\\"+stage+"-actions.json";
 const quote=s=>"'"+s.replaceAll("'","''")+"'";
 const pre=await tools.exec_command({cmd:"if ((Test-Path -LiteralPath "+quote(log)+") -or (Get-ChildItem -LiteralPath "+quote(frames)+" -Filter "+quote(stage+"-*")+")) { throw 'Condition already started; never replay input' }",workdir:"C:\\DEV\\Reversebolt",max_output_tokens:500});
 if(pre.exit_code!==0) throw Error("Condition is not new");
 const record={pid,index,stage,started:new Date().toISOString(),status:"running",actions:[]};
 const save=async()=>{const r=await tools.mcp__filesystem__write_file({path:log,content:JSON.stringify(record,null,2)});if(r.isError)throw Error("Cannot save action receipt");};
 const foreground=async()=>{const r=await tools.mcp__anode__seat_windows({pid});if(r.isError||!r.structuredContent?.windows?.some(w=>w.pid===pid&&w.foreground))throw Error("Expected game is not foreground");};
 const capture=async label=>{
   await foreground();
   const r=await tools.mcp__anode__seat_screenshot({format:"png",maxWidth:640});
   const im=r.content?.find(c=>c.type==="image");
   if(r.isError||!im||im.mimeType!=="image/png")throw Error("Capture failed");
   const saved=await tools.mcp__filesystem__write_file({path:frames+"\\"+label+".b64",content:im.data});
   if(saved.isError)throw Error("Capture save failed");
   record.actions.push({action:"screenshot",label,at:new Date().toISOString(),metadata:r.content.filter(c=>c.type==="text").map(c=>c.text)});await save();
 };
 await save();
 try {
   await capture(stage+"-idle");await new Promise(r=>setTimeout(r,250));await capture(stage+"-0");
   for(const [i,key] of ["d","a","d","a"].entries()){
     await foreground();const r=await tools.mcp__anode__seat_key({keys:key,holdMs:200});
     record.actions.push({action:"key",key,holdMs:200,at:new Date().toISOString(),result:r.content});await save();
     if(r.isError||r.content?.find(c=>c.type==="text")?.text!=="ok")throw Error("Input outcome uncertain; never replay");
     await new Promise(r=>setTimeout(r,250));await capture(stage+"-"+(i+1));
   }
   const decoded=await tools.exec_command({cmd:"Get-ChildItem -LiteralPath "+quote(frames)+" -Filter "+quote(stage+"-*.b64")+" | ForEach-Object { $captureTarget=[IO.Path]::ChangeExtension($_.FullName,'png'); $captureBytes=[Convert]::FromBase64String([IO.File]::ReadAllText($_.FullName)); $captureStream=[IO.File]::Open($captureTarget,[IO.FileMode]::CreateNew); try { $captureStream.Write($captureBytes,0,$captureBytes.Length) } finally { $captureStream.Dispose() } }",workdir:"C:\\DEV\\Reversebolt",max_output_tokens:500});
   if(decoded.exit_code!==0)throw Error("Frame decode failed");
   record.status="completed";
 } catch(error){record.status="stopped";record.error=String(error);throw error;}
 finally {record.finished=new Date().toISOString();await save();}
 return {index,stage,status:record.status,frames:6,pulses:4};
})