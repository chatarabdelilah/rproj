// Reproduce with Node and a local jsdom 26.1.0 module directory as argv[2].
const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const {Script} = require('node:vm');
const {JSDOM, VirtualConsole} = require(process.argv[2] ? path.resolve(process.argv[2]) : 'jsdom');
const html = fs.readFileSync(path.join(__dirname, 'catalog-browser.html'), 'utf8');
const scribe = 'local Scribe = require(game:GetService("ReplicatedStorage").packages.scribe)\nreturn {\n    Coins = Scribe.Number(0),\n    Spawn = Scribe.Vector3(Vector3.zero),\n}';
const profile = 'local ProfileStore = require(game:GetService("ServerScriptService").serverPackages.profilestore)\nlocal store = ProfileStore.New("ExampleProfiles", { Coins = 0 })\nprint(store.Name)';
// A full microtask turn also drains promises returned across jsdom's realm.
const settle = () => new Promise(resolve=>setImmediate(resolve));

function fixture(t, mode='absent') {
  const errors=[],tasks=[],copies=[],messages=[],scrolls=[];
  const vc=new VirtualConsole().on('jsdomError',e=>errors.push(e.message));
  const dom=new JSDOM(html,{runScripts:'dangerously',virtualConsole:vc,beforeParse(w){
    w.scrollTo=v=>scrolls.push(v.top);
    w.setTimeout=(fn,ms)=>{tasks.push({fn,ms});return tasks.length;};
    if(mode==='hostile') Object.defineProperty(w,'openai',{get(){throw Error('host unavailable');}});
    if(mode==='host') w.openai={widgetState:{privateContent:{catalogTheme:'dark'}},setWidgetState:v=>{messages.push(v);return Promise.resolve();}};
    if(mode==='clipboard') Object.defineProperty(w.navigator,'clipboard',{value:{writeText:async text=>{copies.push(text);}}});
  }});
  t.after(()=>{dom.window.close();assert.deepEqual(errors,[]);});
  const w=dom.window,doc=w.document,e=id=>doc.getElementById('cat-'+id);
  const click=id=>{assert.ok(e(id),id);e(id).click();};
  const change=(id,value)=>{e(id).value=value;e(id).dispatchEvent(new w.Event(id.includes('search')?'input':'change',{bubbles:true}));};
  const key=(id,key)=>e(id).dispatchEvent(new w.KeyboardEvent('keydown',{key,bubbles:true,cancelable:true}));
  const rows=()=>Array.from(e('results').querySelectorAll('button')).map(b=>b.dataset.entry);
  const owners=()=>[e('active').textContent,e('watch').textContent,e('drafts').textContent,e('activity').textContent,e('activity').open,e('output').style.height];
  return {w,doc,e,click,change,key,rows,owners,tasks,copies,messages,scrolls};
}

test('literal initial screen contains guide, examples and every section without JavaScript',t=>{
  const dom=new JSDOM(html);t.after(()=>dom.window.close());const d=dom.window.document;
  new Script(d.querySelector('script').textContent);
  assert.equal(d.querySelector('#cat-name').textContent,'Scribe');
  assert.equal(d.querySelector('#cat-code').textContent,scribe);
  assert.equal(d.querySelectorAll('#cat-results button').length,8);
  assert.equal(d.querySelectorAll('#cat-scope option').length,7);
  assert.equal(d.querySelector('#cat-browser').hidden,false);
  assert.match(d.querySelector('#cat-sections').textContent,/server lifecycle/);
  assert.doesNotMatch(html,/document\.currentScript|<html|<head>|<body>|<svg|fetch\(|XMLHttpRequest|WebSocket/);
  assert.ok(Buffer.byteLength(html)<1024*1024);
});

test('host and icons are optional; only compatible appearance restores without saving on load',t=>{
  fixture(t,'absent');const hostile=fixture(t,'hostile');hostile.change('theme','light');
  assert.equal(hostile.doc.querySelector('#rp-catalog').dataset.theme,'light');
  const f=fixture(t,'host');assert.equal(f.e('theme').value,'dark');assert.equal(f.messages.length,0);
  f.change('theme','light');assert.equal(f.messages.length,1);
  f.w.dispatchEvent(new f.w.CustomEvent('openai:set_globals',{detail:{globals:{widgetState:{privateContent:{catalogTheme:'system'}}}}}));
  assert.equal(f.e('theme').value,'system');assert.equal(f.messages.length,1);
  f.w.dispatchEvent(new f.w.CustomEvent('openai:set_globals',{detail:{globals:{widgetState:{privateContent:{catalogTheme:'invalid'}}}}}));
  assert.equal(f.e('theme').value,'system');assert.ok(JSON.stringify(f.messages).length<16384);
});

test('all-library search covers names, keys, descriptions and category labels with independent scope',t=>{
  const f=fixture(t);
  for(const [q,expected] of [['SCRIBE',['package-scribe','tool-scribe-studio']],['scribe-studio',['tool-scribe-studio']],['session locking',['package-profilestore']],['Data & profiles',['package-scribe','package-profilestore']],['Code quality',['capability-format','file-stylua']]]){
    f.change('search',q);assert.deepEqual(f.rows(),expected);
  }
  f.change('search','');f.change('scope','Tools');assert.deepEqual(f.rows(),['tool-scribe-studio','tool-stylua']);
  f.change('search','not found');assert.deepEqual(f.rows(),[]);assert.match(f.e('results').textContent,/No entries match/);
  assert.equal(f.e('name').textContent,'Scribe');assert.match(f.e('count').textContent,/outside these results/);
  f.change('search','');f.change('scope','all');assert.equal(f.rows().length,8);
});

test('same-name tool and settings have unique identity and relevant sections',t=>{
  const f=fixture(t);f.change('search','stylua');
  assert.ok(f.rows().includes('tool-stylua'));assert.ok(f.rows().includes('settings-stylua'));
  f.click('entry-tool-stylua');assert.match(f.e('kind').textContent,/Tools/);assert.match(f.e('identity').textContent,/Provider/);
  assert.equal(f.e('example-section').hidden,true);assert.equal(f.e('code').textContent,scribe);
  f.click('entry-settings-stylua');assert.match(f.e('kind').textContent,/Configurable settings/);assert.match(f.e('identity').textContent,/Destination/);
  assert.match(f.e('sections').textContent,/column_width/);assert.match(f.e('sections').textContent,/catalog suggestions, not values read/);
  assert.equal(f.e('doc-link').href,'https://github.com/JohnnyMorganz/StyLua#options');
});

test('Scribe to Studio to Back restores filters, focus and position without changing owners',t=>{
  const f=fixture(t);f.e('activity').open=true;f.e('output').style.height='220px';const owners=f.owners();
  Object.defineProperty(f.w,'scrollY',{value:330,configurable:true});
  f.change('scope','Packages');f.change('search','scribe');f.e('related-tool-scribe-studio').focus();f.click('related-tool-scribe-studio');
  assert.equal(f.e('name').textContent,'Scribe Studio');assert.match(f.e('sections').textContent,/manual install step/);
  assert.match(f.e('sections').textContent,/cannot detect/);assert.match(f.e('sections').textContent,/does not add the Scribe library/);
  assert.equal(f.e('store-link').href,'https://create.roblox.com/store/asset/113609038046646/Scribe-Studio');
  assert.deepEqual(f.owners(),owners);f.click('back');
  assert.equal(f.e('name').textContent,'Scribe');assert.equal(f.e('search').value,'scribe');assert.equal(f.e('scope').value,'Packages');
  assert.equal(f.doc.activeElement,f.e('related-tool-scribe-studio'));assert.deepEqual(f.scrolls,[330]);assert.equal(f.e('back').disabled,true);
  assert.deepEqual(f.owners(),owners);
});

test('ProfileStore example and guidance preserve the server realm and lifecycle caveat',t=>{
  const f=fixture(t);f.click('related-package-profilestore');assert.equal(f.e('code').textContent,profile);
  assert.match(f.e('identity').textContent,/Server realm/);assert.match(f.e('sections').textContent,/Server Script only/);
  assert.match(f.e('sections').textContent,/StartSessionAsync failure/);assert.match(f.e('sections').textContent,/EndSession/);
  assert.equal(f.e('doc-link').href,'https://madstudioroblox.github.io/ProfileStore/');
});

test('sample examples and docs match authoritative local catalog fields',t=>{
  const f=fixture(t);const repo=path.resolve(__dirname,'../..');
  const source=fs.readFileSync(path.join(repo,'crates/rproj-core/src/catalog/wally_packages.rs'),'utf8');
  const guides=fs.readFileSync(path.join(repo,'crates/rproj-core/src/catalog/package_usage.rs'),'utf8');
  for(const [id,key,expected] of [['package-scribe','scribe',scribe],['package-profilestore','profilestore',profile]]){
    f.click('entry-'+id);const fields=source.slice(source.indexOf('key: "'+key+'"')).split('},')[0];
    const url=fields.match(/docs_url: "([^"]+)"/)[1];assert.equal(f.e('doc-link').href,url);
    const usage=guides.slice(guides.indexOf('"'+key+'" => (')).split('\n        ),')[0];
    const literals=Array.from(usage.matchAll(/"(?:[^"\\]|\\.)*"/g),m=>JSON.parse(m[0]));
    assert.ok(literals.includes(expected.slice(expected.indexOf('\n')+1)),'canonical example body: '+key);
    assert.equal(f.e('code').textContent,expected);
  }
});

test('capability, file and topic guides omit empty example/docs sections without invented URLs',t=>{
  const f=fixture(t);
  for(const id of ['capability-format','file-stylua','topic-ci']){
    f.click('entry-'+id);assert.equal(f.e('example-section').hidden,true);assert.equal(f.e('docs').children.length,0);assert.ok(f.e('sections').children.length>=3);
  }
  f.click('entry-capability-format');assert.match(f.e('sections').textContent,/\.gitattributes/);assert.match(f.e('sections').textContent,/No other capability/);
  f.click('entry-file-stylua');assert.match(f.e('sections').textContent,/whether its file exists/);
});

test('navigation, project closure and long paths retain catalog state and all other owners',t=>{
  const f=fixture(t);f.change('search','stylua');f.click('entry-settings-stylua');const owners=f.owners();
  Object.defineProperty(f.w,'scrollY',{value:215,configurable:true});
  f.click('overview');assert.equal(f.e('browser').hidden,true);f.click('library');
  assert.deepEqual(f.scrolls,[215]);
  assert.equal(f.e('name').textContent,'StyLua');assert.match(f.e('kind').textContent,/Configurable settings/);assert.equal(f.e('search').value,'stylua');assert.deepEqual(f.owners(),owners);
  f.click('switch');f.click('project-lantern');f.click('overview');assert.match(f.e('workspace-path').textContent,/灯台/);assert.match(f.e('workspace-path').textContent,/long shared project folder/);
  assert.equal(f.e('watch').textContent,owners[1]);assert.equal(f.e('drafts').textContent,owners[2]);
  f.click('library');f.click('switch');f.click('close-project');assert.equal(f.e('active').textContent,'No project');assert.equal(f.e('browser').hidden,false);assert.equal(f.e('name').textContent,'StyLua');
  f.click('overview');assert.equal(f.e('workspace-name').textContent,'No project open');f.click('empty-catalog');assert.equal(f.e('name').textContent,'StyLua');
});

test('known dependency deep link and unknown inspection never invent guidance',t=>{
  const f=fixture(t);f.click('dependencies');const text=f.e('workspace-body').textContent;
  for(const label of ['Recorded composition','Wally declaration','Resolved version','Installed files','Manual Wally declaration','no catalog guide'])assert.ok(text.includes(label),label);
  assert.equal(f.e('workspace-body').querySelectorAll('button').length,1);f.click('view-catalog');assert.equal(f.e('name').textContent,'Scribe');assert.equal(f.e('browser').hidden,false);
  f.click('missing');assert.equal(f.e('name').textContent,'Guide unavailable');assert.equal(f.e('docs').children.length,0);assert.equal(f.e('example-section').hidden,true);
  f.click('back');assert.equal(f.e('name').textContent,'Scribe');assert.equal(f.doc.activeElement,f.e('missing'));
});

test('project menu Escape and outside dismissal restore focus; folder sample preserves selection',t=>{
  const f=fixture(t);f.click('switch');assert.equal(f.doc.activeElement,f.e('project-skyline'));f.key('project-skyline','Escape');
  assert.equal(f.e('menu').hidden,true);assert.equal(f.doc.activeElement,f.e('switch'));
  f.click('switch');f.e('name').click();assert.equal(f.e('menu').hidden,true);assert.equal(f.doc.activeElement,f.e('switch'));
  const owners=f.owners();f.click('switch');f.click('open-folder');assert.deepEqual(f.owners(),owners);assert.match(f.e('notice').textContent,/selection has not changed/);
});

test('Copy writes exactly the selected sample and reports missing clipboard honestly',async t=>{
  const f=fixture(t,'clipboard');f.click('copy');await settle();assert.deepEqual(f.copies,[scribe]);assert.equal(f.e('copy-status').textContent,'Example copied.');
  f.click('related-package-profilestore');f.click('copy');await settle();assert.deepEqual(f.copies,[scribe,profile]);
  const fallback=fixture(t);fallback.click('copy');await settle();assert.match(fallback.e('copy-status').textContent,/Clipboard access is unavailable/);assert.equal(fallback.w.getSelection().toString(),scribe);
});

test('late clipboard completion cannot report success on a different guide',async t=>{
  const f=fixture(t);let complete;
  Object.defineProperty(f.w.navigator,'clipboard',{value:{writeText:()=>new Promise(resolve=>{complete=resolve;})}});
  f.click('copy');f.click('related-package-profilestore');complete();await settle();assert.equal(f.e('copy-status').textContent,'');
});

test('Activity filtering, manual collapse and retained size survive browsing and Stop acknowledgement',t=>{
  const f=fixture(t);f.e('output').style.height='240px';f.e('activity').open=true;f.change('output-search','passed');assert.equal(f.e('output').textContent,'Skyline / Checks: All sample checks passed');
  f.change('output-search','absent');assert.match(f.e('output').textContent,/No output lines match/);f.change('output-search','');f.e('activity').open=false;
  f.click('related-tool-scribe-studio');assert.equal(f.e('activity').open,false);f.click('stop');assert.match(f.e('watch').textContent,/Stopping/);assert.equal(f.e('stop').disabled,true);
  assert.equal(f.tasks.length,1);f.tasks.shift().fn();assert.match(f.e('watch').textContent,/Stopped/);assert.equal(f.e('activity').open,false);
  f.click('watch-output');assert.equal(f.e('activity').open,true);assert.equal(f.e('output').style.height,'240px');assert.match(f.e('output').textContent,/Skyline \/ Watch: Stop acknowledged/);
});

function references(doc) {
  const ids=Array.from(doc.querySelectorAll('[id]'),e=>e.id);assert.equal(new Set(ids).size,ids.length);
  for(const node of doc.querySelectorAll('[aria-controls],[aria-labelledby],[aria-describedby],label[for]'))for(const attr of ['aria-controls','aria-labelledby','aria-describedby','for']){
    for(const id of (node.getAttribute(attr)||'').split(' ').filter(Boolean))assert.ok(doc.getElementById(id),attr+'='+id);
  }
  for(const b of doc.querySelectorAll('button'))assert.ok(b.textContent.trim()||b.getAttribute('aria-label'),'named button');
  for(const a of doc.querySelectorAll('a')){assert.match(a.href,/^https:\/\//);assert.equal(a.rel,'noopener noreferrer');}
}
test('unique IDs, native keyboard controls and valid accessibility references across all entries',t=>{
  const f=fixture(t);for(const id of f.rows()){f.click('entry-'+id);references(f.doc);}
  f.click('dependencies');references(f.doc);assert.equal(f.doc.querySelector('[tabindex]'),null);
  assert.doesNotMatch(html,/outline\s*:\s*(none|0)|:focus[^}]*outline/);
  assert.match(html,/@container catalog-main \(min-width:600px\)/);assert.match(html,/@container catalog-shell \(max-width:550px\)/);
  assert.match(html,/white-space:pre-wrap/);assert.match(html,/overflow-wrap:anywhere/);assert.match(html,/min-height:44px/);
});

function luminance(hex){return hex.match(/\w\w/g).map(v=>parseInt(v,16)/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((s,v,i)=>s+v*[.2126,.7152,.0722][i],0);}
test('Light/Dark palette: 18 text comparisons meet WCAG AA normal text contrast',()=>{
  const palettes=[{bg:['fcfbf8','f1f0eb','e6eefb'],fg:['282925','605f58','2459b8']},{bg:['1e1e1d','272725','2d3c52'],fg:['efeee8','bbb9ae','97bcff']}];
  for(const palette of palettes)for(const bg of palette.bg)for(const fg of palette.fg){const [a,b]=[luminance(bg),luminance(fg)].sort((a,b)=>b-a);assert.ok((a+.05)/(b+.05)>=4.5,fg+' on '+bg);assert.ok(html.includes('#'+bg));assert.ok(html.includes('#'+fg));}
});
