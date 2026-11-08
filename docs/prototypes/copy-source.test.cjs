// Node + jsdom 26.1.0. Optional argv[2] is the installed jsdom module directory.
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const {Script}=require('node:vm');
const {JSDOM,VirtualConsole}=require(process.argv[2]?path.resolve(process.argv[2]):'jsdom');
const html=fs.readFileSync(path.join(__dirname,'copy-source.html'),'utf8');
const source=JSON.parse(fs.readFileSync(path.join(__dirname,'copy-source.fixtures.json'),'utf8'));
function fixture(t,host='none') {
  const tasks=[],errors=[],writes=[],scrolls=[],messages=[];
  const dom=new JSDOM(html,{runScripts:'dangerously',virtualConsole:new VirtualConsole().on('jsdomError',e=>errors.push(e.message)),beforeParse(w){
    w.TextEncoder=TextEncoder;w.setTimeout=fn=>{tasks.push(fn);return tasks.length;};
    w.HTMLElement.prototype.scrollIntoView=function(){scrolls.push(this.id);};
    Object.defineProperty(w.navigator,'clipboard',{value:{writeText:text=>writes.push(text)}});
    if(host==='hostile')Object.defineProperty(w,'openai',{get(){throw Error('not available');}});
    if(host==='working')w.openai={widgetState:{privateContent:{copyTheme:'dark'}},setWidgetState:v=>{messages.push(v);return Promise.resolve();}};
  }});
  t.after(()=>{dom.window.close();assert.deepEqual(errors,[]);assert.deepEqual(writes,[],'no OS clipboard requests');});
  const w=dom.window,d=w.document,e=id=>d.getElementById('cp-'+id),click=id=>{assert.ok(e(id),id);e(id).focus();e(id).click();};
  const change=(id,value)=>{e(id).value=value;e(id).dispatchEvent(new w.Event(id==='filter'?'input':'change',{bubbles:true}));};
  const api=d.getElementById('rp-copy').copySource,snapshot=()=>JSON.parse(JSON.stringify(api.snapshot()));
  const flush=()=>{const fn=tasks.shift();assert.ok(fn,'queued completion');fn();};
  const scenario=value=>{change('scenario',value);click('load-scenario');click('confirm');flush();};
  const key=(id,key)=>e(id).dispatchEvent(new w.KeyboardEvent('keydown',{key,bubbles:true,cancelable:true}));
  return {w,d,e,click,change,api,snapshot,tasks,flush,scenario,key,scrolls,messages};
}
test('useful literal startup, valid script, no host/clipboard/network dependency',t=>{
  const dom=new JSDOM(html);t.after(()=>dom.window.close());const d=dom.window.document;
  assert.equal(d.getElementById('cp-review').hidden,false);assert.equal(d.getElementById('cp-owner').textContent,'Skyline');
  assert.equal(d.querySelectorAll('#cp-files button').length,3);assert.match(d.getElementById('cp-text').textContent,/print\("Skyline"\)/);
  assert.equal(d.getElementById('cp-text').textContent,source.expected.replace(/\r\n/g,'\n'));assert.equal(d.getElementById('cp-bytes').textContent,Buffer.byteLength(source.expected)+' UTF-8 bytes');
  new Script(d.querySelector('script').textContent);assert.ok(Buffer.byteLength(html)<1024*1024);
  assert.doesNotMatch(html,/document\.currentScript|navigator\.clipboard|fetch\(|XMLHttpRequest|WebSocket|<html|<head>|<body>/);
});
test('exact independent payload fixture includes empty and non-Luau files; byte count is UTF-8',t=>{
  const f=fixture(t),s=f.snapshot().review;
  assert.deepEqual(s.files,source.files);assert.equal(s.text,source.expected);
  assert.equal(f.api.payload(source.files),source.expected);assert.equal(f.e('bytes').textContent,Buffer.byteLength(source.expected)+' UTF-8 bytes');
  assert.notEqual(Buffer.byteLength(source.expected),source.expected.length);
  f.click('copy');assert.equal(f.e('copy').disabled,true);f.flush();assert.equal(f.snapshot().clipboard,source.expected);assert.match(f.e('status').textContent,/Copy requested/);
});
test('file navigation selects and scrolls without changing payload',t=>{
  const f=fixture(t);f.click('file-2');assert.deepEqual(f.scrolls,['cp-source-2']);assert.equal(f.snapshot().selected,2);
  f.click('copy');f.flush();assert.equal(f.snapshot().clipboard,source.expected);assert.equal(f.e('file-2').getAttribute('aria-pressed'),'true');
});
test('status completions retain file focus and DOM; successful refresh restores surviving focus',t=>{
  const f=fixture(t);f.click('copy');f.e('file-1').focus();const button=f.e('file-1'),row=f.e('source-1');f.flush();
  assert.equal(f.d.activeElement,button);assert.equal(f.e('source-1'),row);
  f.click('refresh');f.e('file-1').focus();f.flush();assert.equal(f.d.activeElement,f.e('file-1'));
  f.change('scenario','empty');f.click('refresh');f.e('file-1').focus();f.flush();assert.equal(f.d.activeElement,f.e('refresh'));
});
test('display escapes controls and preserves exact raw copied characters',t=>{
  const f=fixture(t);f.scenario('controls');const s=f.snapshot().review;
  assert.deepEqual(s.files.at(-1),source.controlFile);assert.ok(f.e('text').textContent.includes('\\u001b'));assert.ok(f.e('text').textContent.includes('\\u202e'));assert.ok(f.e('text').textContent.includes('⟦CRLF⟧'));
  assert.equal(f.e('text').querySelector('script'),null);assert.equal(f.e('text').querySelectorAll('.number').length,s.text.split('\n').length-1);
  f.click('copy');f.flush();assert.equal(f.snapshot().clipboard,source.expected+f.api.payload([source.controlFile]));
});
test('partial collection names skips and only requests included files',t=>{
  const f=fixture(t);f.scenario('partial');assert.equal(f.e('copy').textContent,'Copy included files');assert.equal(f.e('skips').hidden,false);
  for(const word of ['access denied','Not UTF-8','Linked path'])assert.ok(f.e('skips').textContent.includes(word));
  f.click('copy');f.flush();assert.equal(f.snapshot().clipboard,source.expected);assert.match(f.e('status').textContent,/3 skipped; only included/);assert.doesNotMatch(f.e('status').textContent,/all source/i);
});
test('source change does not change copy; explicit refresh replaces snapshot',t=>{
  const f=fixture(t);f.click('source-change');f.click('copy');f.flush();assert.equal(f.snapshot().clipboard,source.expected);
  f.click('refresh');assert.equal(f.e('review').getAttribute('aria-busy'),'true');assert.equal(f.e('copy').disabled,true);f.flush();
  assert.match(f.snapshot().review.text,/changed sample 1/);assert.equal(f.e('copy').disabled,false);
});
test('failed refresh retains previous snapshot and allows explicit old snapshot copy',t=>{
  const f=fixture(t);f.change('scenario','removed');f.click('refresh');f.flush();assert.match(f.e('snapshot-label').textContent,/previous snapshot/);
  assert.equal(f.snapshot().review.text,source.expected);assert.equal(f.e('copy').disabled,false);assert.match(f.e('problem').textContent,/no longer available/);
  f.click('copy');f.flush();assert.equal(f.snapshot().clipboard,source.expected);
});
test('empty versus missing/unreadable/refused preparation, recovery never truncates',t=>{
  const f=fixture(t);f.scenario('empty');assert.match(f.e('status').textContent,/empty/);assert.equal(f.e('copy').disabled,true);assert.equal(f.snapshot().review.text,'');
  for(const scenario of ['missing','not-directory','unreadable','traversal','linked','large','removed']){
    f.scenario(scenario);assert.equal(f.snapshot().review,null,scenario);assert.equal(f.e('copy').disabled,true);assert.equal(f.e('problem').hidden,false);
  }
  f.scenario('ready');assert.equal(f.snapshot().review.text,source.expected);assert.equal(f.e('problem').hidden,true);
});
test('browse/close project retain review, Watch, drafts and snapshot; replacement defaults Keep',t=>{
  const f=fixture(t),watch=f.e('watch').textContent,drafts=f.e('drafts').textContent;
  f.click('switch');f.click('project-lantern');assert.equal(f.snapshot().active,'lantern');assert.equal(f.snapshot().owner,'skyline');
  assert.equal(f.e('review').hidden,false);assert.equal(f.e('watch').textContent,watch);f.click('overview');f.click('enter');assert.equal(f.d.activeElement,f.e('keep'));
  assert.match(f.e('dialog-body').textContent,/Skyline/);assert.match(f.e('dialog-body').textContent,/Lantern/);f.key('keep','Escape');assert.equal(f.d.activeElement,f.e('enter'));assert.equal(f.snapshot().owner,'skyline');
  f.click('resume');f.click('switch');f.click('close-project');assert.equal(f.snapshot().active,null);assert.equal(f.snapshot().owner,'skyline');assert.equal(f.e('watch').textContent,watch);assert.equal(f.e('drafts').textContent,drafts);
});
test('explicit replacement captures destination and never moves Watch',t=>{
  const f=fixture(t),watch=f.e('watch').textContent;f.click('switch');f.click('project-lantern');f.click('overview');f.click('enter');f.click('confirm');f.flush();
  assert.equal(f.snapshot().owner,'lantern');assert.match(f.e('path').textContent,/長い名前/);assert.match(f.snapshot().review.text,/Lantern/);assert.equal(f.e('watch').textContent,watch);
});
test('late preparation cannot replace a newer captured review',t=>{
  const f=fixture(t);f.click('refresh');f.click('switch');f.click('project-lantern');f.click('overview');f.click('enter');f.click('confirm');
  f.flush();assert.equal(f.snapshot().review,null);f.flush();assert.equal(f.snapshot().review.owner,'lantern');
});
test('late clipboard result is ignored after refresh, replacement or window closure',t=>{
  for(const action of ['refresh','replace','close']){
    const f=fixture(t);f.click('copy');
    if(action==='refresh')f.click('refresh');else if(action==='replace'){f.click('switch');f.click('project-lantern');f.click('overview');f.click('enter');f.click('confirm');}else f.click('close-window');
    f.flush();assert.equal(f.snapshot().clipboard,null);assert.doesNotMatch(f.e('status').textContent,/Copy requested/);
  }
});
test('rejected/unavailable clipboard preserve snapshot and allow retry without a success claim',t=>{
  const f=fixture(t);for(const mode of ['rejected','unavailable']){f.change('clipboard-mode',mode);f.click('copy');f.flush();assert.equal(f.snapshot().clipboard,null);assert.equal(f.e('copy').disabled,false);assert.match(f.e('status').textContent,/retained/i);}
  f.change('clipboard-mode','ok');f.click('copy');f.flush();assert.equal(f.snapshot().clipboard,source.expected);
});
test('Activity retains captured metadata, bounds output and respects manual collapse',t=>{
  const f=fixture(t);f.e('output').style.height='250px';f.click('copy');assert.equal(f.e('activity').open,true);f.e('activity').open=false;
  f.click('switch');f.click('project-lantern');f.click('dependencies');f.flush();assert.equal(f.e('activity').open,false);assert.match(f.e('output').textContent,/Skyline/);assert.doesNotMatch(f.e('output').textContent,/print\(|Café/);
  f.change('filter','missing result');assert.equal(f.e('output').textContent,'No output matches this search.');f.click('clear');assert.match(f.e('output').textContent,/Copy requested/);
  f.click('watch-output');assert.equal(f.e('output').style.height,'250px');f.e('activity').open=false;f.click('watch-output');assert.equal(f.e('output').style.height,'250px');
  f.click('resume');for(let i=0;i<35;i++){f.click('copy');f.flush();}assert.equal(f.snapshot().logs.length,60);
});
test('clipboard failure expands Activity once; no-operation and waiting states are distinct',t=>{
  const f=fixture(t);assert.equal(f.e('output').textContent,'No operations yet');f.change('clipboard-mode','rejected');f.click('copy');assert.match(f.e('output').textContent,/pending/);f.e('activity').open=false;f.flush();assert.equal(f.e('activity').open,true);
});
test('separate Stop acknowledges without altering the source review',t=>{
  const f=fixture(t);f.click('stop');assert.match(f.e('watch').textContent,/Stopping/);assert.equal(f.e('stop').disabled,true);f.flush();assert.equal(f.e('watch').textContent,'Watch · Stopped');assert.equal(f.snapshot().review.text,source.expected);
});
test('menu and folder-picker cancellation restore focus without changing selection',t=>{
  const f=fixture(t);f.click('switch');assert.equal(f.d.activeElement,f.e('project-skyline'));f.key('project-skyline','Escape');assert.equal(f.d.activeElement,f.e('switch'));
  f.click('switch');f.click('open-project');assert.equal(f.d.activeElement,f.e('keep'));f.key('keep','Escape');assert.equal(f.d.activeElement,f.e('switch'));assert.equal(f.snapshot().active,'skyline');
  f.click('switch');f.click('overview');assert.equal(f.e('menu').hidden,true);
});
test('read-only close has no draft prompt; pending preparation cannot revive a closed window',t=>{
  const f=fixture(t);f.click('refresh');f.click('close-window');assert.equal(f.snapshot().closed,true);assert.equal(f.e('dialog').hidden,true);f.flush();assert.equal(f.snapshot().closed,true);assert.equal(f.snapshot().clipboard,null);f.click('reopen');assert.equal(f.e('copy').disabled,false);
});
test('existing draft and operation close protections remain default Keep',t=>{
  const f=fixture(t);f.change('guard','dirty');f.click('close-window');assert.equal(f.d.activeElement,f.e('keep'));assert.equal(f.e('keep').textContent,'Keep draft');f.key('keep','Escape');assert.equal(f.snapshot().closed,false);
  f.change('guard','operation');f.click('close-window');assert.equal(f.e('confirm').hidden,true);f.click('keep');assert.equal(f.snapshot().closed,false);
  f.change('guard','dirty');f.click('close-window');f.click('confirm');assert.equal(f.snapshot().closed,true);
});
test('optional host state restores only appearance and never includes source text',t=>{
  const hostile=fixture(t,'hostile');hostile.change('theme','light');assert.equal(hostile.d.getElementById('rp-copy').dataset.theme,'light');
  const f=fixture(t,'working');assert.equal(f.e('theme').value,'dark');assert.deepEqual(f.messages,[]);f.change('theme','light');assert.equal(f.messages.length,1);assert.doesNotMatch(JSON.stringify(f.messages),/Skyline|print|Café/);
  f.w.dispatchEvent(new f.w.CustomEvent('openai:set_globals',{detail:{globals:{widgetState:{privateContent:{copyTheme:'system'}}}}}));assert.equal(f.e('theme').value,'system');assert.equal(f.messages.length,1);
});
function references(d){const ids=Array.from(d.querySelectorAll('[id]'),e=>e.id);assert.equal(ids.length,new Set(ids).size);for(const node of d.querySelectorAll('[aria-controls],[aria-labelledby],[aria-describedby],label[for]'))for(const attr of ['aria-controls','aria-labelledby','aria-describedby','for'])for(const id of (node.getAttribute(attr)||'').split(' ').filter(Boolean))assert.ok(d.getElementById(id),attr+'='+id);for(const button of d.querySelectorAll('button'))assert.ok(button.textContent.trim()||button.getAttribute('aria-label'));}
test('accessibility references across scenarios; responsive/wrapped layouts remain in source',t=>{
  const f=fixture(t);for(const scenario of ['ready','partial','empty','controls','large']){f.scenario(scenario);references(f.d);}assert.equal(f.d.querySelector('[tabindex]'),null);
  for(const rule of ['copy-main (min-width:600px)','copy-shell (max-width:550px)','white-space:pre-wrap','overflow-wrap:anywhere','min-height:44px'])assert.ok(html.includes(rule),rule);
  assert.doesNotMatch(html,/outline\s*:\s*none/);
});
function luminance(hex){return hex.match(/\w\w/g).map(v=>parseInt(v,16)/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((s,v,i)=>s+v*[.2126,.7152,.0722][i],0);}
test('Light/Dark text, muted, accent and error contrast exceeds 4.5:1',()=>{
  const themes=[{bg:['fcfbf8','f1f0eb','e6eefb'],fg:['282925','605f58','2459b8','a12621']},{bg:['1e1e1d','272725','2d3c52'],fg:['efeee8','bbb9ae','97bcff','ffb4ab']}];
  for(const theme of themes)for(const bg of theme.bg)for(const fg of theme.fg){const a=luminance(bg),b=luminance(fg);assert.ok((Math.max(a,b)+.05)/(Math.min(a,b)+.05)>=4.5,bg+'/'+fg);assert.ok(html.includes('#'+bg));assert.ok(html.includes('#'+fg));}
});
