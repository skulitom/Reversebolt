import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {leadingNumber,createOracle} from './oracle.mjs';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.REVERSEBOLT_PLAYWRIGHT);
assert.equal(leadingNumber('<div class="score">128<div>+4</div></div>'),128);
assert.equal(leadingNumber('<span>0</span>'),0);
assert.throws(()=>leadingNumber('<div>unknown 50</div>'));
const browser=await chromium.launch({headless:true,executablePath:process.env.REVERSEBOLT_BROWSER});
try {
  const context=await browser.newContext({viewport:{width:800,height:600}});
  const page=await context.newPage();
  await page.setContent('<style>body{margin:0}.counter{position:absolute;left:20px;top:30px;width:100px;height:40px}</style><div class="counter">128<div>+4</div></div>');
  const sample=await createOracle(context,page,{score:'.counter'});
  const first=await sample();
  assert.equal(first.values.score.value,128);assert.deepEqual(first.values.score.bounds,[20,30,100,40]);
  assert.equal(first.viewport.width,800);assert.equal(first.viewport.height,600);assert.equal(first.geometry_supported,true);
  assert(first.request_ms<=first.time_ms&&first.time_ms<=first.response_ms);
  await page.locator('.counter').evaluate(e=>e.firstChild.textContent='256');
  assert.equal((await sample()).values.score.value,256);
  console.log(JSON.stringify({leading_text:'passed',native_dom_text_and_geometry:'passed',changed_text:'passed'}));
} finally {await browser.close();}
