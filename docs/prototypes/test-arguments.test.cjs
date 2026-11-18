// Existing jsdom installation only. This harness does not launch native windows.
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const {Script}=require('node:vm');
const {JSDOM,VirtualConsole}=require(process.argv[2]?path.resolve(process.argv[2]):'jsdom');
const html=fs.readFileSync(path.join(__dirname,'test-arguments.html'),'utf8');
const fixtures=JSON.parse(fs.readFileSync(path.join(__dirname,'test-arguments.fixtures.json'),'utf8'));
function fixture(t){
 const errors=[];
 const dom=new JSDOM(html,{runScripts:'dangerously',virtualConsole:new VirtualConsole().on('jsdomError',error=>errors.push(error.message)),beforeParse(w){
  // jsdom has no native dialog behavior. This tests handlers, not platform modality.
  w.HTMLDialogElement.prototype.showModal=function(){this.setAttribute('open','');};
  w.HTMLDialogElement.prototype.close=function(){this.removeAttribute('open');};
  Object.defineProperty(w,'openai',{get(){throw Error('Optional host unavailable');}});
 }});
 t.after(()=>{dom.window.close();assert.deepEqual(errors,[]);});
 const w=dom.window,d=w.document,e=id=>d.getElementById('ta-'+id),api=d.getElementById('rp-test').testArguments;
 const click=id=>{assert.ok(e(id),id);e(id).focus();e(id).click();};
 const input=(id,value)=>{e(id).value=value;e(id).dispatchEvent(new w.Event('input',{bubbles:true}));};
 const change=(id,value)=>{e(id).value=value;e(id).dispatchEvent(new w.Event('change',{bubbles:true}));};
 const check=id=>{e(id).checked=!e(id).checked;e(id).dispatchEvent(new w.Event('change',{bubbles:true}));};
 const snap=()=>JSON.parse(JSON.stringify(api.snapshot()));
 const escape=()=>e('dialog').dispatchEvent(new w.Event('cancel',{cancelable:true}));
 const scenario=id=>{change('case',id);click('load');};
 const finish=kind=>{change('result',kind);click('complete');};
 return {w,d,e,api,click,input,change,check,snap,escape,scenario,finish};
}
test('useful literal startup, syntax, bounded fragment and no external execution',t=>{
 const dom=new JSDOM(html);t.after(()=>dom.window.close());const d=dom.window.document;
 assert.equal(d.getElementById('ta-flow').hidden,false);assert.match(d.getElementById('ta-argv').textContent,/--passWithNoTests/);assert.equal(d.getElementById('ta-rows').querySelectorAll('input').length,2);
 new Script(d.querySelector('script').textContent);assert.ok(Buffer.byteLength(html)<1024*1024);
 assert.doesNotMatch(html,/fetch\(|XMLHttpRequest|WebSocket|navigator\.clipboard|document\.currentScript|<html|<body>|<head>/);
 const f=fixture(t);assert.equal(f.e('run').disabled,false);assert.match(f.e('effects').textContent,/refresh Jest/);
});
test('exact runner invocation fixtures, not shell parsing',t=>{
 const f=fixture(t);for(const sample of fixtures.cases){const actual=JSON.parse(JSON.stringify(f.api.invocation(sample.runner,sample.arguments)));assert.deepEqual(actual,{program:sample.program,argv:sample.argv},sample.name);}
});
test('literal argument editing, NUL refusal and empty consent',t=>{
 const f=fixture(t);f.input('arg-1','保存 "quoted" $HOME; echo hello');f.click('run');assert.equal(f.snap().run.argv[2],'保存 "quoted" $HOME; echo hello');f.finish('success');
 f.click('add');assert.equal(f.e('run').disabled,true);f.check('empty-2');assert.equal(f.e('run').disabled,false);f.click('run');assert.equal(f.snap().run.argv.at(-1),'');f.finish('success');
 f.input('arg-2','bad\0value');assert.equal(f.e('run').disabled,true);assert.match(f.e('problem').textContent,/NUL/);assert.equal(f.api.start(),false);
 f.input('arg-2','');assert.equal(f.e('run').disabled,true);assert.equal(f.e('empty-2').checked,false);
});
test('escaped controls preserve exact CR LF and tabs from editor through submission',t=>{
 const f=fixture(t);const exact='first\r\nsecond\nthird\rend\t';f.change('format-1','json');assert.equal(f.e('discard').disabled,true);f.input('arg-1',JSON.stringify(exact));assert.equal(f.snap().rows[1].value,exact);assert.match(f.e('argv').textContent,/\\r\\n/);f.click('run');assert.equal(f.snap().run.argv[2],exact);f.finish('success');f.change('format-1','literal');assert.equal(f.e('format-1').value,'json');assert.equal(f.snap().rows[1].value,exact);
});
test('multiline paste switches to escaped editing without losing line endings',t=>{
 const f=fixture(t);const input=f.e('arg-1');input.setSelectionRange(0,input.value.length);const event=new f.w.Event('paste',{bubbles:true,cancelable:true});Object.defineProperty(event,'clipboardData',{value:{getData:()=> 'first\r\nsecond\rthird\n'}});input.dispatchEvent(event);assert.equal(event.defaultPrevented,true);assert.equal(f.e('format-1').value,'json');f.click('run');assert.equal(f.snap().run.argv[2],'first\r\nsecond\rthird\n');
});
test('invalid JSON remains unsubmitted and cannot silently replace the valid value',t=>{
 const f=fixture(t);f.change('format-1','json');f.input('arg-1','[1]');assert.equal(f.e('run').disabled,true);assert.equal(f.snap().rows[1].value,'inventory');f.change('format-1','literal');assert.equal(f.e('format-1').value,'json');f.click('close');assert.equal(f.e('keep').textContent,'Keep draft');f.escape();assert.equal(f.e('arg-1').value,'[1]');f.click('discard');f.click('confirm');assert.equal(f.e('arg-1').value,'inventory');assert.equal(f.e('run').disabled,false);
});
test('ordered controls move and remove exact values with focus',t=>{
 const f=fixture(t);assert.equal(f.e('up-0').disabled,true);assert.equal(f.e('down-1').disabled,true);f.click('down-0');assert.deepEqual(f.snap().rows.map(x=>x.value),['inventory','-t']);assert.equal(f.d.activeElement,f.e('arg-1'));
 f.click('up-1');assert.deepEqual(f.snap().rows.map(x=>x.value),['-t','inventory']);f.click('remove-0');f.click('remove-0');assert.match(f.e('rows').textContent,/No extra/);assert.equal(f.d.activeElement,f.e('add'));f.click('run');assert.deepEqual(f.snap().run.argv,['--passWithNoTests']);
});
test('default Run tests ignores retained custom arguments and uses configured runner',t=>{
 const f=fixture(t);f.input('arg-1','unsent');f.click('checks');f.click('default');assert.deepEqual(f.snap().run.argv,['--passWithNoTests']);assert.match(f.e('dirty').textContent,/Unsubmitted/);f.finish('success');
 f.click('resume');f.click('discard');f.click('confirm');f.scenario('testez');f.click('checks');f.click('default');assert.equal(f.snap().run.program,'lute');assert.deepEqual(f.snap().run.argv,['test']);
});
test('captured ownership survives project switch, closure and app navigation',t=>{
 const f=fixture(t);f.input('arg-1','retained');f.click('switch');f.click('lantern');assert.equal(f.snap().owner,'Skyline');f.click('catalog');f.click('resume');assert.match(f.e('owner').textContent,/Skyline/);assert.equal(f.e('arg-1').value,'retained');
 f.click('switch');f.click('close-project');assert.equal(f.snap().active,null);f.click('checks');assert.equal(f.e('default').disabled,true);f.click('resume');assert.equal(f.e('run').disabled,false);f.click('run');assert.equal(f.snap().run.owner,'Skyline');assert.match(f.snap().run.path,/Skyline$/);assert.equal(f.snap().watchOwner,'Skyline');
});
test('replacement defaults Keep draft and captures new owner only on confirmation',t=>{
 const f=fixture(t);f.input('arg-1','retained');f.click('switch');f.click('lantern');f.click('checks');f.click('enter');assert.equal(f.d.activeElement,f.e('keep'));f.escape();assert.equal(f.snap().owner,'Skyline');assert.equal(f.d.activeElement,f.e('enter'));f.click('enter');f.click('confirm');assert.equal(f.snap().owner,'Lantern');assert.deepEqual(f.snap().rows,[]);assert.match(f.e('path').textContent,/保存/);
});
test('reverted edits restore clean state and default Keep protects discard',t=>{
 const f=fixture(t);f.input('arg-1','changed');f.click('discard');assert.equal(f.e('keep').textContent,'Keep draft');f.escape();assert.equal(f.e('arg-1').value,'changed');f.click('discard');f.click('confirm');assert.equal(f.e('arg-1').value,'inventory');assert.equal(f.e('discard').disabled,true);f.click('close');assert.equal(f.snap().closed,true);
});
test('context changes retain old invocation until explicit refresh',t=>{
 const f=fixture(t);f.input('arg-1','retained');f.click('source');assert.equal(f.e('run').disabled,true);assert.equal(f.snap().runner,'jest');f.click('refresh');f.escape();assert.equal(f.snap().captured,0);f.click('refresh');f.click('confirm');assert.equal(f.snap().runner,'testez');assert.equal(f.e('arg-1').value,'retained');assert.equal(f.e('run').disabled,false);assert.match(f.e('argv').textContent,/\[0\] "test"/);
});
test('all refused sample contexts explain unavailable execution',t=>{
 const f=fixture(t);for(const kind of ['no-testing','incompatible','malformed','worker','prerequisite']){f.scenario(kind);assert.equal(f.e('run').disabled,true,kind);assert.ok(f.e('problem').textContent.length>20);assert.equal(f.api.start(),false);assert.equal(f.snap().run,null);}
 f.scenario('testez');assert.equal(f.e('run').disabled,false);assert.match(f.e('effects').textContent,/None/);
});
test('missing worker also blocks default tests after browsing another project',t=>{
 const f=fixture(t);f.scenario('worker');f.click('switch');f.click('lantern');f.click('checks');assert.equal(f.e('default').disabled,true);assert.match(f.e('default-reason').textContent,/every project/);assert.equal(f.api.start(false),false);assert.equal(f.snap().run,null);
});
test('Watch allows steady Watching for either owner and blocks transitional states',t=>{
 const f=fixture(t);for(const owner of ['Skyline','Lantern']){if(f.snap().watchOwner!==owner)f.click('watch-owner');for(const [state,result]of Object.entries(fixtures.watch)){f.change('watch-state',state);assert.equal(f.e('run').disabled,result==='blocked',owner+' '+state);}}
 f.change('watch-state','Watching');f.click('stop');assert.equal(f.e('run').disabled,true);f.click('stop-ack');assert.equal(f.e('run').disabled,false);assert.equal(f.snap().watchOwner,'Lantern');
});
test('serialized operations and competing-operation refusal preserve edits',t=>{
 const f=fixture(t);f.input('arg-1','unsent');f.click('busy');assert.equal(f.api.start(),false);assert.equal(f.snap().baseline[1].value,'inventory');f.click('busy');f.click('run');const id=f.snap().run.id;assert.equal(f.api.start(),false);assert.equal(f.snap().run.id,id);assert.equal(f.e('load').disabled,true);
});
test('submitted invocation is immutable; later edits remain unsubmitted',t=>{
 const f=fixture(t);f.click('run');f.input('arg-1','next run');assert.equal(f.snap().run.argv[2],'inventory');f.click('advance');assert.equal(f.snap().run.stage,'Running');f.click('switch');f.click('lantern');f.finish('failure');assert.equal(f.snap().result.owner,'Skyline');assert.match(f.e('summary').textContent,/Skyline.*Runner failed/);f.click('resume');assert.equal(f.e('arg-1').value,'next run');assert.match(f.e('dirty').textContent,/Unsubmitted/);
});
test('stale and duplicate results cannot replace a newer operation',t=>{
 const f=fixture(t);f.click('run');const old=f.snap().run.id;assert.equal(f.api.complete(old,'unknown'),false);assert.equal(f.api.complete(old,'cancelled'),false);assert.equal(f.api.complete(old,'success'),true);assert.equal(f.api.complete(old,'failure'),false);f.click('run');const current=f.snap().run.id;f.click('obsolete');assert.equal(f.snap().run.id,current);assert.equal(f.api.complete(old,'failure'),false);assert.equal(f.snap().result,null);
});
test('cancellation stays pending until confirmed and completion race preserves success',t=>{
 const f=fixture(t);f.click('run');f.click('cancel');assert.equal(f.snap().run.stage,'Cancellation pending');assert.equal(f.e('cancel').disabled,true);assert.equal(f.e('run').disabled,true);f.click('teardown');assert.equal(f.snap().result.label,'Cancelled');f.click('run');f.click('cancel');f.finish('success');assert.equal(f.snap().result.label,'Succeeded');
});
test('Activity keeps owner, bounded output, filter and manual collapse/height',t=>{
 const f=fixture(t);f.click('run');assert.equal(f.snap().expanded,true);assert.match(f.e('output').textContent,/Waiting/);f.e('output').style.height='240px';f.click('activity-toggle');for(let i=0;i<110;i++)f.click('line');assert.equal(f.snap().logs.length,100);assert.equal(f.snap().omitted,10);assert.equal(f.snap().expanded,false);f.finish('success');assert.equal(f.snap().expanded,false);f.click('activity-toggle');assert.equal(f.e('output').style.height,'240px');f.input('filter','not present');assert.match(f.e('output').textContent,/No search matches/);f.input('filter','Skyline');assert.match(f.e('output').textContent,/Skyline/);f.click('clear');assert.match(f.e('output').textContent,/no retained output/);
});
test('new failure expands Activity once; ordinary updates never reopen it',t=>{
 const f=fixture(t);f.click('run');const id=f.snap().run.id;f.click('activity-toggle');f.finish('failure');assert.equal(f.snap().expanded,true);f.click('activity-toggle');f.api.complete(id,'failure');assert.equal(f.snap().expanded,false);
});
test('idle close protects unsubmitted edits and Escape restores focus',t=>{
 const f=fixture(t);f.input('arg-1','dirty');f.click('close');assert.equal(f.d.activeElement,f.e('keep'));assert.equal(f.e('keep').textContent,'Keep draft');f.escape();assert.equal(f.snap().closed,false);assert.equal(f.snap().closeAfter,false);assert.equal(f.d.activeElement,f.e('close'));assert.equal(f.e('arg-1').value,'dirty');f.click('close');f.click('confirm');assert.equal(f.snap().closed,true);
});
test('close waits for cancellation and then protects independent dirty drafts',t=>{
 const f=fixture(t);f.check('other-dirty');f.click('run');f.click('close');assert.equal(f.e('keep').textContent,'Keep open');assert.match(f.e('dialog-text').textContent,/Skyline/);f.click('confirm');assert.equal(f.snap().closed,false);assert.equal(f.snap().run.cancel,true);f.click('teardown');assert.equal(f.snap().closed,false);assert.equal(f.e('keep').textContent,'Keep draft');f.escape();assert.equal(f.snap().otherDirty,true);assert.equal(f.snap().closeAfter,false);
});
test('completion while close dialog open waits for choice; Keep cancels prior close intent',t=>{
 const f=fixture(t);f.click('run');f.click('close');f.click('confirm');f.click('close');f.finish('success');assert.equal(f.snap().closed,false);assert.equal(f.e('dialog').open,true);f.escape();assert.equal(f.snap().closed,false);assert.equal(f.snap().closeAfter,false);assert.equal(f.d.activeElement,f.e('close'));
});
test('confirmed close after raced completion protects edits made during run',t=>{
 const f=fixture(t);f.click('run');f.input('arg-1','later');f.click('close');f.finish('success');f.click('confirm');assert.equal(f.snap().closed,false);assert.equal(f.e('keep').textContent,'Keep draft');assert.match(f.e('dialog-title').textContent,/unsubmitted/);f.click('confirm');assert.equal(f.snap().closed,true);
});
test('uncertain teardown blocks close and runs until explicit confirmation',t=>{
 const f=fixture(t);f.click('run');f.click('close');f.click('confirm');f.finish('uncertain');assert.equal(f.snap().closed,false);assert.equal(f.e('run').disabled,true);f.click('close');assert.match(f.e('notice').textContent,/unconfirmed/);f.click('teardown');assert.equal(f.snap().closed,true);assert.match(f.snap().result.label,/outcome uncertain/);
});
test('switcher Escape and simulated picker cancellation restore trigger focus',t=>{
 const f=fixture(t);f.click('switch');f.e('skyline').dispatchEvent(new f.w.KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true}));assert.equal(f.e('menu').hidden,true);assert.equal(f.d.activeElement,f.e('switch'));f.click('switch');f.click('open');f.escape();assert.equal(f.snap().active,'Skyline');assert.equal(f.d.activeElement,f.e('switch'));
});
test('unique IDs, accessibility references, labeled controls and responsive contracts',t=>{
 const f=fixture(t);f.click('add');const ids=[...f.d.querySelectorAll('[id]')].map(x=>x.id);assert.equal(new Set(ids).size,ids.length);
 for(const node of f.d.querySelectorAll('[aria-controls],[aria-labelledby],[aria-describedby],label[for]'))for(const attr of ['aria-controls','aria-labelledby','aria-describedby','for'])for(const id of (node.getAttribute(attr)||'').split(' ').filter(Boolean))assert.ok(f.d.getElementById(id),id);
 for(const node of f.d.querySelectorAll('input,select'))assert.ok(node.labels.length||node.getAttribute('aria-label'));
 assert.match(html,/@container test-main \(min-width:600px\)/);assert.match(html,/@container test-shell \(max-width:550px\)/);assert.match(html,/white-space:pre-wrap/);assert.match(html,/:focus-visible/);
 for(const theme of ['light','dark','system']){f.change('theme',theme);assert.equal(f.d.getElementById('rp-test').dataset.theme,theme);assert.equal(f.e('flow').hidden,false);}
});
test('Soft text colors meet WCAG AA contrast in both themes',()=>{
 const colors=Object.fromEntries([...html.matchAll(/--([a-z]+):light-dark\((#[0-9a-f]{6}),(#[0-9a-f]{6})\)/g)].map(m=>[m[1],[m[2],m[3]]]));
 const luminance=hex=>hex.slice(1).match(/../g).map(v=>parseInt(v,16)/255).map(v=>v<=0.04045?v/12.92:((v+0.055)/1.055)**2.4).reduce((sum,v,i)=>sum+v*[0.2126,0.7152,0.0722][i],0);
 for(let theme=0;theme<2;theme++)for(const foreground of ['text','quiet','blue','error'])for(const background of ['surface','side','tint']){const a=luminance(colors[foreground][theme]),b=luminance(colors[background][theme]);assert.ok((Math.max(a,b)+0.05)/(Math.min(a,b)+0.05)>=4.5,`${foreground}/${background}/${theme}`);}
});
