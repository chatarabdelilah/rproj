// Node + jsdom 26.1.0; optional argv[2] names an existing jsdom module directory.
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const {Script}=require('node:vm');
const {JSDOM,VirtualConsole}=require(process.argv[2]?path.resolve(process.argv[2]):'jsdom');
const html=fs.readFileSync(path.join(__dirname,'project-tool-setup.html'),'utf8');
const fixtures=JSON.parse(fs.readFileSync(path.join(__dirname,'project-tool-setup.fixtures.json'),'utf8'));
function fixture(t,hostile=false){
 const errors=[];
 const dom=new JSDOM(html,{runScripts:'dangerously',virtualConsole:new VirtualConsole().on('jsdomError',e=>errors.push(e.message)),beforeParse(w){
  // jsdom has no native dialog implementation. Exercise our handlers, not a real focus trap.
  w.HTMLDialogElement.prototype.showModal=function(){this.setAttribute('open','');};
  w.HTMLDialogElement.prototype.close=function(){this.removeAttribute('open');};
  if(hostile)Object.defineProperty(w,'openai',{get(){throw Error('host unavailable');}});
 }});
 t.after(()=>{dom.window.close();assert.deepEqual(errors,[]);});
 const w=dom.window,d=w.document,e=id=>d.getElementById('pt-'+id),api=d.getElementById('rp-tool').projectTool;
 const click=id=>{assert.ok(e(id),id);e(id).focus();e(id).click();};
 const change=(id,value)=>{e(id).value=value;e(id).dispatchEvent(new w.Event(['filter','search'].includes(id)?'input':'change',{bubbles:true}));};
 const scenario=value=>{change('case',value);click('load');click('confirm');};
 const tool=value=>{const b=d.querySelector('[data-tool="'+value+'"]');assert.ok(b,value);b.focus();b.click();};
 const key=(id,key,shiftKey=false)=>e(id).dispatchEvent(new w.KeyboardEvent('keydown',{key,shiftKey,bubbles:true,cancelable:true}));
 const snapshot=()=>JSON.parse(JSON.stringify(api.snapshot()));
 const start=()=>{click('prepare');click('apply');click('confirm');assert.ok(snapshot().run);};
 return {w,d,e,api,click,change,scenario,tool,key,snapshot,start};
}
test('literal useful startup, valid syntax and no external execution dependency',t=>{
 const dom=new JSDOM(html);t.after(()=>dom.window.close());const d=dom.window.document;
 assert.equal(d.getElementById('pt-selection').hidden,false);assert.equal(d.getElementById('pt-tool-name').textContent,'Asphalt');
 assert.match(d.getElementById('pt-prerequisites').textContent,/Rokit/);assert.equal(d.getElementById('pt-path').textContent,fixtures.projects.Skyline);
 new Script(d.querySelector('script').textContent);assert.ok(Buffer.byteLength(html)<1024*1024);
 assert.doesNotMatch(html,/fetch\(|XMLHttpRequest|WebSocket|navigator\.clipboard|document\.currentScript|<html|<body>|<head>/);
 fixture(t,true).change('theme','dark');
});
test('fixture inventory, named tools, search and extension/CLI distinction',t=>{
 const f=fixture(t);assert.deepEqual(Array.from(f.e('case').options,o=>o.value),Object.keys(fixtures.cases));
 assert.equal(f.e('list').querySelectorAll('button').length,13);
 f.change('search','luau lsp');assert.equal(f.e('list').children.length,2);f.tool('luau-lsp');assert.match(f.e('kind').textContent,/Machine-wide/);
 f.tool('luau-lsp-cli');assert.match(f.e('kind').textContent,/Project Rokit/);
 f.change('search','not-a-tool');assert.match(f.e('list').textContent,/No matching/);assert.equal(f.snapshot().tool,'luau-lsp-cli');
});
test('ordered Asphalt effects, explicit unknown version and manual requirements',t=>{
 const f=fixture(t);f.click('prepare');const r=f.snapshot().review;
 assert.match(r.items[0],/Initialize missing rokit/);assert.match(r.items[1],/Trust source.*downloads\/shared cache.*unknown/);
 assert.match(r.items[2],/asphalt.toml.*assets\/\*\*\/\*.png/);assert.match(r.config,/strip_extensions = true/);
 assert.match(r.config,/output_basename = "assets"/);assert.match(r.notes,/creator.id.*credentials/);assert.match(r.notes,/No asset upload/);
});
test('custom and existing pins, existing files and both asset source layouts',t=>{
 const f=fixture(t);
 for(const key of ['pin','custom','existing']){f.scenario(key);f.click('prepare');const r=f.snapshot().review;assert.match(r.items[1],/Preserve existing.*no version update/);assert.doesNotMatch(r.items.join(' '),/Trust source/);}
 assert.equal(f.snapshot().review.config,'');assert.match(f.e('effects').textContent,/byte-for-byte/);
 for(const tool of ['asphalt','tungsten']){f.scenario('figma');f.tool(tool);f.click('prepare');assert.match(f.snapshot().review.config,/figma\/exports\/\*\*\/\*.png/);}
 assert.match(f.snapshot().review.config,/strip_extension = true/);assert.match(f.snapshot().review.config,/output_path = "src\/shared\/assets.luau"/);
});
test('Blender missing/existing scenes and guidance avoid invented binary diffs',t=>{
 const f=fixture(t);f.tool('blender');f.click('prepare');assert.match(f.e('effects').textContent,/0.28 meter/);assert.equal(f.snapshot().review.config,'');
 f.scenario('scene');f.click('prepare');assert.match(f.e('effects').textContent,/Preserve existing.*need not run/);
 f.scenario('no-blender');assert.equal(f.e('prepare').disabled,true);assert.match(f.e('problem').textContent,/Blender is unavailable/);
 f.tool('scribe-studio');f.click('prepare');assert.equal(f.e('apply').hidden,true);assert.equal(f.snapshot().review.items.length,1);
 f.click('tool-step');f.click('handoff');assert.equal(f.snapshot().view,'Machine Setup');assert.equal(f.snapshot().watch,'Lantern');
});
test('plain Rojo works without record/setup; unsafe and missing prerequisites refuse review',t=>{
 const f=fixture(t);f.scenario('plain');f.click('prepare');assert.equal(f.e('apply').disabled,false);
 for(const key of ['no-rokit','no-project','malformed','linked','unreadable']){f.scenario(key);assert.equal(f.e('prepare').disabled,true,key);assert.equal(f.e('problem').hidden,false,key);}
 f.scenario('fresh');assert.equal(f.e('prepare').disabled,false);
});
test('confirmation defaults Keep; Escape, Tab and dismissal restore trigger focus',t=>{
 const f=fixture(t);f.click('prepare');f.click('apply');assert.equal(f.d.activeElement,f.e('keep'));assert.equal(f.e('keep').textContent,'Keep reviewing');
 assert.match(f.e('dialog-text').textContent,/Skyline.*C:\/Samples.*Initialize/);f.key('keep','Tab',true);assert.equal(f.d.activeElement,f.e('confirm'));
 f.key('confirm','Tab');assert.equal(f.d.activeElement,f.e('keep'));f.key('keep','Escape');assert.equal(f.e('dialog').open,false);assert.equal(f.d.activeElement,f.e('apply'));assert.equal(f.snapshot().run,null);
});
test('changing a reviewed tool defaults Keep review and preserves the captured owner',t=>{
 const f=fixture(t);f.click('prepare');f.tool('tungsten');assert.equal(f.e('keep').textContent,'Keep review');assert.equal(f.d.activeElement,f.e('keep'));
 f.key('keep','Escape');assert.equal(f.snapshot().tool,'asphalt');assert.ok(f.snapshot().review);assert.equal(f.d.activeElement.dataset.tool,'tungsten');
 f.tool('tungsten');f.click('confirm');assert.equal(f.snapshot().tool,'tungsten');assert.equal(f.snapshot().review,null);assert.equal(f.snapshot().owner,'Skyline');assert.equal(f.snapshot().watch,'Lantern');
});
test('stale sources and competing operations invalidate confirmation; refresh is explicit',t=>{
 for(const action of ['source-change','busy','watch-owner']){const f=fixture(t);f.click('prepare');f.click('apply');f.click(action);f.click('confirm');assert.equal(f.snapshot().run,null,action);assert.match(f.e('output').textContent,/expired/);}
 const f=fixture(t);f.click('prepare');f.click('source-change');assert.equal(f.e('apply').disabled,true);f.click('refresh');assert.equal(f.e('apply').disabled,false);
});
test('same-project Watch waits for separate Stop acknowledgement; other Watch remains untouched',t=>{
 const f=fixture(t);f.click('prepare');f.click('watch-owner');assert.equal(f.e('apply').disabled,true);f.click('stop');assert.equal(f.e('apply').disabled,true);
 assert.equal(f.snapshot().watch,'Skyline');f.click('finish');assert.equal(f.snapshot().watch,null);assert.equal(f.e('apply').disabled,false);
 const g=fixture(t);g.start();g.api.finish(g.snapshot().run.id,'success');assert.equal(g.snapshot().watch,'Lantern');
});
test('browsing and closing projects retains review; replacement defaults Keep and captures long path',t=>{
 const f=fixture(t);f.click('prepare');f.click('switch');f.click('lantern');assert.equal(f.snapshot().owner,'Skyline');
 f.click('tools');f.click('enter');assert.equal(f.e('keep').textContent,'Keep review');f.key('keep','Escape');assert.equal(f.snapshot().owner,'Skyline');
 f.click('enter');f.click('confirm');assert.equal(f.snapshot().owner,'Lantern');assert.equal(f.e('path').textContent,fixtures.projects.Lantern);
 f.click('switch');f.click('close-project');assert.equal(f.snapshot().owner,'Lantern');assert.equal(f.snapshot().active,null);assert.equal(f.snapshot().watch,'Lantern');
});
test('switcher Escape/outside dismissal and simulated folder cancellation preserve active project',t=>{
 const f=fixture(t);f.click('switch');f.key('skyline','Escape');assert.equal(f.d.activeElement,f.e('switch'));assert.equal(f.e('menu').hidden,true);
 f.click('switch');f.click('owner');assert.equal(f.e('menu').hidden,true);assert.equal(f.d.activeElement,f.e('switch'));
 f.click('switch');f.click('open');f.click('keep');assert.equal(f.snapshot().active,'Skyline');assert.equal(f.d.activeElement,f.e('switch'));
});
test('serialized running operation, per-item progress, navigation and captured Activity',t=>{
 const f=fixture(t);f.start();const id=f.snapshot().run.id;assert.match(f.e('items').textContent,/Running.*Pending/);
 f.click('advance');assert.match(f.e('items').textContent,/Completed.*Running.*Pending/);
 f.click('switch');f.click('lantern');f.click('tools');assert.equal(f.e('enter').disabled,true);assert.equal(f.e('load').disabled,true);
 f.api.finish(id,'success');assert.match(f.e('summary').textContent,/Skyline.*Completed/);assert.match(f.e('output').textContent,/Skyline/);assert.equal(f.snapshot().owner,'Skyline');
 f.click('resume');assert.match(f.e('items').textContent,/Manual action required/);
});
test('failed addition plus created files remains Failed; retries require renewed review',t=>{
 const f=fixture(t);f.start();f.api.finish(f.snapshot().run.id,'failure');assert.equal(f.e('outcome').textContent,'Failed');
 assert.match(f.e('items').textContent,/Failed · Trust/);assert.match(f.e('items').textContent,/Completed · Create asphalt/);assert.match(f.e('result-note').textContent,/No rollback/);
 f.click('review-step');assert.equal(f.e('apply').disabled,true);f.click('refresh');assert.equal(f.e('apply').disabled,false);
});
test('existing files report Already present; pre-change failure reports skipped effects',t=>{
 const f=fixture(t);f.scenario('existing');f.start();f.api.finish(f.snapshot().run.id,'success');assert.match(f.e('items').textContent,/Already present/);
 f.click('retry');f.click('apply');f.click('confirm');f.api.finish(f.snapshot().run.id,'before');assert.match(f.e('items').textContent,/Skipped/);assert.equal(f.e('outcome').textContent,'Failed');
});
test('obsolete, duplicate and invalid completions cannot replace a later operation',t=>{
 const f=fixture(t);f.start();const first=f.snapshot().run.id;f.api.finish(first+5,'success');assert.equal(f.snapshot().run.id,first);
 f.api.finish(first,'invalid');assert.equal(f.snapshot().run.id,first);f.api.finish(first,'success');const text=f.e('summary').textContent;
 f.api.finish(first,'failure');assert.equal(f.e('summary').textContent,text);f.click('retry');f.click('apply');f.click('confirm');const second=f.snapshot().run.id;
 f.api.finish(first,'failure');assert.equal(f.snapshot().run.id,second);assert.match(f.e('summary').textContent,/Running/);
});
test('uncertainty blocks closing and mutations until teardown, then requires explicit refresh',t=>{
 const f=fixture(t);f.start();f.api.finish(f.snapshot().run.id,'uncertain');f.click('close');assert.equal(f.snapshot().closed,false);assert.equal(f.e('retry').disabled,true);
 f.click('drained');assert.equal(f.snapshot().uncertain,true);f.click('tool-step');assert.equal(f.snapshot().page,'results');f.tool('tungsten');assert.equal(f.snapshot().tool,'asphalt');
 assert.equal(f.e('apply').disabled,true);f.click('retry');assert.equal(f.snapshot().uncertain,false);assert.equal(f.e('apply').disabled,false);
});
test('close after setup waits for completion and then protects independent dirty drafts',t=>{
 const f=fixture(t);f.e('dirty').checked=true;f.e('dirty').dispatchEvent(new f.w.Event('change'));f.start();f.click('close');assert.equal(f.e('keep').textContent,'Keep open');
 f.click('confirm');assert.equal(f.snapshot().closed,false);f.api.finish(f.snapshot().run.id,'success');assert.equal(f.e('dialog').open,true);assert.equal(f.e('keep').textContent,'Keep draft');
 f.key('keep','Escape');assert.equal(f.snapshot().closed,false);assert.equal(f.snapshot().dirty,true);
 f.click('close');f.click('confirm');assert.equal(f.snapshot().closed,true);assert.equal(f.snapshot().watch,'Lantern');
});
test('completion while close dialog is open preserves Keep focus and choice',t=>{
 for(const kind of ['success','uncertain']){const f=fixture(t);f.start();f.click('close');f.api.finish(f.snapshot().run.id,kind);assert.equal(f.d.activeElement,f.e('keep'));
  f.click('confirm');assert.equal(f.snapshot().closed,kind==='success');}
 const f=fixture(t);f.start();f.click('close');f.api.finish(f.snapshot().run.id,'success');f.key('keep','Escape');assert.equal(f.d.activeElement,f.e('close'));assert.equal(f.snapshot().closed,false);
});
test('idle read-only review adds no dirty-close prompt and no cancellation action',t=>{
 const f=fixture(t);f.click('prepare');f.click('close');assert.equal(f.snapshot().closed,true);assert.equal(f.e('dialog').open,false);
 assert.equal(Array.from(f.d.querySelectorAll('button')).some(b=>/cancel/i.test(b.textContent)),false);
});
test('Keep open or Escape withdraws an earlier close-after-setup choice, including completion races',t=>{
 for(const action of ['keep','escape']){
  const f=fixture(t);f.start();f.click('close');f.click('confirm');assert.equal(f.snapshot().closeAfter,true);
  f.click('close');if(action==='keep')f.click('keep');else f.key('keep','Escape');assert.equal(f.snapshot().closeAfter,false);
  f.api.finish(f.snapshot().run.id,'success');assert.equal(f.snapshot().closed,false);
 }
 const f=fixture(t);f.start();f.click('close');f.click('confirm');f.click('close');f.api.finish(f.snapshot().run.id,'success');
 assert.equal(f.snapshot().closed,false);assert.equal(f.d.activeElement,f.e('keep'));f.click('keep');assert.equal(f.snapshot().closeAfter,false);
});
test('app-level navigation retains the setup review, Watch and independent draft state',t=>{
 const f=fixture(t);f.click('prepare');f.e('dirty').checked=true;f.e('dirty').dispatchEvent(new f.w.Event('change'));const review=f.snapshot().review;
 for(const id of ['machine','setups','template','catalog']){f.click(id);assert.deepEqual(f.snapshot().review,review);assert.equal(f.snapshot().watch,'Lantern');assert.equal(f.snapshot().dirty,true);f.click('resume');assert.equal(f.e('review').hidden,false);}
});
test('bounded searchable output, manual collapse and height survive completion/navigation',t=>{
 const f=fixture(t);f.start();f.e('output').style.height='230px';f.click('activity-toggle');f.click('advance');assert.equal(f.snapshot().expanded,false);
 f.api.finish(f.snapshot().run.id,'success');assert.equal(f.snapshot().expanded,false);f.click('tools');f.click('resume');f.click('activity-toggle');assert.equal(f.e('output').style.height,'230px');
 f.change('filter','not-in-output');assert.equal(f.e('output').textContent,'No search matches');f.change('filter','Skyline');assert.match(f.e('output').textContent,/Skyline/);
 for(let i=0;i<105;i++)f.click('watch-output');assert.equal(f.snapshot().logs.length,100);f.click('clear');assert.equal(f.e('output').textContent,'No output recorded');
});
test('unique IDs, label/ARIA references and responsive/focus source rules',t=>{
 const f=fixture(t);for(const key of Object.keys(fixtures.cases)){f.scenario(key);const ids=Array.from(f.d.querySelectorAll('[id]'),x=>x.id);assert.equal(new Set(ids).size,ids.length);
  for(const el of f.d.querySelectorAll('[for],[aria-labelledby],[aria-describedby],[aria-controls]'))for(const attr of ['for','aria-labelledby','aria-describedby','aria-controls'])for(const id of (el.getAttribute(attr)||'').split(' ').filter(Boolean))assert.ok(f.d.getElementById(id),id);
 }
 assert.match(html,/@container tool-main \(min-width:600px\)/);assert.match(html,/@container tool-shell \(max-width:550px\)/);assert.match(html,/:focus-visible/);assert.match(html,/overflow-wrap:anywhere/);assert.match(html,/min-height:44px/);
 f.change('theme','light');assert.equal(f.d.getElementById('rp-tool').dataset.theme,'light');f.change('theme','dark');assert.equal(f.d.getElementById('rp-tool').dataset.theme,'dark');
});
test('Light/Dark content and action text retain 4.5:1 contrast',()=>{
 const luminance=h=>{const rgb=h.match(/\w\w/g).map(v=>parseInt(v,16)/255).map(v=>v<=0.04045?v/12.92:((v+0.055)/1.055)**2.4);return rgb[0]*0.2126+rgb[1]*0.7152+rgb[2]*0.0722;};
 const palette={};for(const match of html.matchAll(/--(\w+):light-dark\(#([0-9a-f]{6}),#([0-9a-f]{6})\)/g))palette[match[1]]=[match[2],match[3]];
 for(let theme=0;theme<2;theme++)for(const fg of ['text','quiet','blue','error'])for(const bg of ['surface','side','tint']){const a=luminance(palette[fg][theme]),b=luminance(palette[bg][theme]);assert.ok((Math.max(a,b)+0.05)/(Math.min(a,b)+0.05)>=4.5,`${fg}/${bg}/${theme}`);}
});
