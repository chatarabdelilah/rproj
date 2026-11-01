// Pass a local jsdom installation as argv[2], or install jsdom in Node's module path.
const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const {Script} = require('node:vm');
const {JSDOM, VirtualConsole} = require(process.argv[2] ? path.resolve(process.argv[2]) : 'jsdom');
const html = fs.readFileSync(path.join(__dirname, 'projects-browser.html'), 'utf8');
const SKY = 'C:/RobloxProjects/Skyline', LANTERN = 'C:/RobloxProjects/Lantern', AWAY = 'D:/Shared/Relay';

function fixture(t, hostile = false) {
  const tasks = [], errors = [], messages = [];
  const vc = new VirtualConsole().on('jsdomError', error => errors.push(error.message));
  const dom = new JSDOM(html, {runScripts:'dangerously', virtualConsole:vc, beforeParse(w) {
    w.setTimeout = (fn, delay) => {tasks.push({fn, delay}); return tasks.length;};
    if (hostile) Object.defineProperty(w, 'openai', {get() {throw Error('optional host unavailable');}});
    else w.openai = {setWidgetState: v => {messages.push(v); return Promise.resolve();}};
  }});
  t.after(() => {dom.window.close(); assert.deepEqual(errors, []);});
  const w = dom.window, doc = w.document;
  const e = id => doc.getElementById('pb-' + id);
  const click = id => e(id).click();
  const event = (id,type) => e(id).dispatchEvent(new w.Event(type, {bubbles:true}));
  const change = (id,value) => {e(id).value = value; event(id, id === 'search' ? 'input' : 'change');};
  const rows = () => Array.from(e('list').children);
  const row = p => rows().find(b => b.dataset.path === p);
  const run = (delay, last = false) => {
    const matches = tasks.filter(x => x.delay === delay);
    const job = matches[last ? matches.length-1 : 0];
    assert.ok(job, 'expected pending job at ' + delay); tasks.splice(tasks.indexOf(job),1); job.fn();
  };
  const key = (target,key) => target.dispatchEvent(new w.KeyboardEvent('keydown', {key,bubbles:true,cancelable:true}));
  const ownership = () => [e('active').textContent, e('activity').textContent, e('activity').open,
    doc.querySelector('.foot strong').textContent, doc.querySelector('.lab small').textContent];
  return {w,doc,e,click,change,rows,row,run,key,ownership,tasks,messages};
}
test('syntax and first paint stand alone without script, icons or host helpers', t => {
  const parsed = new JSDOM(html); t.after(() => parsed.window.close());
  const doc = parsed.window.document;
  new Script(doc.querySelector('script').textContent);
  assert.equal(doc.querySelector('#pb-name').textContent,'Lantern');
  assert.equal(doc.querySelectorAll('#pb-list button').length,8);
  assert.match(doc.querySelector('#pb-diagnostic').textContent,/Malformed/);
  assert.equal(doc.querySelector('#pb-browser').hidden,false);
  assert.equal(doc.querySelector('#pb-active').textContent,'Skyline');
  const f = fixture(t,true); f.change('theme','dark');
  assert.equal(f.doc.querySelector('#rp-projects').dataset.theme,'dark');
  assert.equal(f.row(LANTERN).getAttribute('aria-pressed'),'true');
});
test('shallow discovery, deterministic ordering, duplicate names and bad documents', t => {
  const f=fixture(t);
  assert.deepEqual(f.rows().map(b=>b.querySelector('strong').textContent),
    ['Canopy','Cinder','Lantern','North Point','Relay','Relay','Skyline','灯台 — Lighthouse with a long production project name']);
  assert.equal(f.rows().filter(b=>b.dataset.path.endsWith('/Relay')).length,2);
  assert.match(f.e('warnings').textContent,/Skipped 1 linked/);
  assert.ok(!f.e('list').textContent.match(/Nested|Linked|Notes|Harbor/));
  f.row(AWAY).click(); f.run(200); assert.match(f.e('facts').textContent,/Launch folder/);
  assert.equal(f.e('path').textContent,AWAY);
  f.rows().at(-1).click(); f.run(200);
  assert.match(f.e('diagnostic').textContent,/Malformed default.project.json/);
});
test('preview, late replies and arrow keys preserve ownership and focus',t=>{
  const f=fixture(t); f.e('activity').open=true; const before=f.ownership();
  f.row(AWAY).focus();f.row(AWAY).click();f.row(LANTERN).focus();f.row(LANTERN).click();
  assert.equal(f.e('open').disabled,true);f.run(200,true);f.run(200);
  assert.equal(f.e('path').textContent,LANTERN); assert.deepEqual(f.ownership(),before);
  assert.equal(f.doc.activeElement.dataset.path,LANTERN);
  f.key(f.doc.activeElement,'End'); f.run(200);
  assert.match(f.doc.activeElement.dataset.path,/灯台/);
  f.key(f.doc.activeElement,'Home');f.run(200);
  assert.equal(f.e('name').textContent,'Canopy');
  assert.deepEqual(f.ownership(),before);
});
test('search by name and path, no matches, refresh retains selected canonical path',t=>{
  const f=fixture(t); f.change('search','d:/SHARED'); f.run(200);
  assert.deepEqual(f.rows().map(x=>x.dataset.path),[AWAY]);assert.equal(f.e('name').textContent,'Relay');
  f.click('refresh');f.run(450);assert.equal(f.e('path').textContent,AWAY);
  f.change('search','灯台');f.run(200);assert.match(f.e('path').textContent,/Lighthouse/);
  f.change('search','no matches'); assert.match(f.e('list-note').textContent,/No projects match/);
  assert.equal(f.e('open').disabled,true); assert.equal(f.e('name').textContent,'No project selected');
});
test('Back and reopen preserve section, query, selection, list position and trigger focus',t=>{
  const f=fixture(t); f.click('back'); assert.equal(f.doc.activeElement,f.e('switch'));
  f.doc.querySelector('[data-section="Tools"]').click();
  f.click('switch');f.click('browse-menu');assert.equal(f.doc.activeElement,f.e('search'));
  f.change('search','Relay');f.run(200);f.row(AWAY).click();f.run(200);
  f.e('list').scrollTop=43;f.click('back');assert.equal(f.e('work-title').textContent,'Tools · Skyline');
  f.click('switch');f.click('browse-menu');assert.equal(f.e('search').value,'Relay');
  assert.equal(f.e('path').textContent,AWAY);assert.equal(f.e('list').scrollTop,43);
  f.key(f.e('search'),'Escape');assert.equal(f.doc.activeElement,f.e('switch'));
});
test('only Open activates preview in Overview, recent-only folder stays in switcher',t=>{
  const f=fixture(t); f.e('activity').open=true;
  f.click('switch');const recents=f.e('recents').textContent;f.key(f.e('browse-menu'),'Escape');
  f.row(AWAY).click();f.run(200);f.click('switch');assert.equal(f.e('recents').textContent,recents);
  f.key(f.e('browse-menu'),'Escape');f.click('open');
  assert.equal(f.e('work-title').textContent,'Overview · Relay');assert.equal(f.e('work-path').textContent,AWAY);
  assert.equal(f.e('activity').open,true);assert.match(f.ownership()[3],/Watching Skyline/);
  f.click('switch');assert.ok(f.e('recents').firstElementChild.textContent.includes(AWAY));
  f.e('recents').lastElementChild.click();assert.equal(f.e('work-path').textContent,'D:/Archive/Harbor');
  f.click('switch');f.click('browse-menu');assert.ok(!f.row('D:/Archive/Harbor'));
});
test('empty workspace entry returns focus without adopting the preview',t=>{
  const f=fixture(t);f.click('switch');f.click('close');assert.equal(f.e('work-title').textContent,'No project open');
  f.click('browse-empty');assert.equal(f.doc.activeElement,f.e('search'));
  f.row(LANTERN).click();f.run(200);f.click('back');
  assert.equal(f.doc.activeElement,f.e('browse-empty'));assert.equal(f.e('work-title').textContent,'No project open');
  assert.match(f.ownership()[3],/Watching Skyline/);
});
test('superseded scans coalesce and old results remain usable until latest scan finishes',t=>{
  const f=fixture(t);
  f.change('fixture','empty');f.change('fixture','missing');f.change('fixture','unreadable');
  assert.deepEqual(f.tasks.map(x=>x.delay),[450]);assert.equal(f.rows().length,8);
  f.run(450);assert.equal(f.rows().length,8);assert.deepEqual(f.tasks.map(x=>x.delay),[450]);
  f.run(450);assert.deepEqual(f.rows().map(b=>b.dataset.path),[AWAY]);
  assert.match(f.e('warnings').textContent,/could not be read/);assert.match(f.e('status').textContent,/no longer listed/);
  assert.equal(f.e('open').disabled,true);assert.equal(f.e('active').textContent,'Skyline');
});
test('empty, missing, and overlapping root/launch sources stay distinct',t=>{
  const f=fixture(t);f.change('fixture','empty');f.run(450);
  assert.equal(f.rows().length,0);assert.match(f.e('list-note').textContent,/No recognized/);
  f.change('fixture','missing');f.run(450);assert.match(f.e('warnings').textContent,/missing/);
  assert.deepEqual(f.rows().map(b=>b.dataset.path),[AWAY]);
  f.change('fixture','overlap');f.run(450);assert.equal(f.rows().filter(b=>b.dataset.path===SKY).length,1);
  f.row(SKY).click();f.run(200);assert.match(f.e('facts').textContent,/Projects folder \+ Launch folder/);
});
test('disappearance before Open or Refresh cannot switch projects',t=>{
  const f=fixture(t);f.click('vanish');f.click('open');
  assert.match(f.e('status').textContent,/disappeared/);assert.equal(f.e('active').textContent,'Skyline');
  assert.equal(f.doc.activeElement,f.e('refresh'));f.click('refresh');f.run(450);assert.ok(!f.row(LANTERN));
  f.click('reset');f.click('vanish');f.click('refresh');f.run(450);
  assert.equal(f.e('name').textContent,'No project selected');assert.equal(f.e('open').disabled,true);
  assert.match(f.e('status').textContent,/no longer listed/);
});
test('reset invalidates pending scan and preview, theme persists only presentation',t=>{
  const f=fixture(t);f.row(AWAY).click();f.change('fixture','empty');f.click('reset');f.run(450);f.run(200);
  assert.equal(f.rows().length,8);assert.equal(f.e('path').textContent,LANTERN);
  f.change('theme','dark');assert.deepEqual(JSON.parse(JSON.stringify(f.messages[0].privateContent)),{theme:'dark'});
  f.w.dispatchEvent(new f.w.CustomEvent('openai:set_globals',{detail:{globals:{widgetState:{privateContent:{theme:'light',active:AWAY}}}}}));
  assert.equal(f.doc.querySelector('#rp-projects').dataset.theme,'light');assert.equal(f.e('active').textContent,'Skyline');
});
test('an unavailable recent project does not clear a different preview',t=>{
  const f=fixture(t);
  f.click('open'); f.click('switch'); f.click('browse-menu');
  f.click('vanish'); f.row(AWAY).click(); f.run(200);
  f.click('switch'); f.e('recents').firstElementChild.click();
  assert.equal(f.e('path').textContent,AWAY); assert.equal(f.e('active').textContent,'Lantern');
  assert.match(f.e('status').textContent,/disappeared/);
});
test('late preview after a scan cannot resurrect a disappeared selection',t=>{
  const f=fixture(t);
  f.row(LANTERN).click(); f.change('fixture','missing'); f.run(450); f.run(200);
  assert.equal(f.e('name').textContent,'No project selected'); assert.equal(f.e('open').disabled,true);
  assert.equal(f.e('active').textContent,'Skyline');
});
test('accessible references, responsive geometry rules and readable theme pairs',t=>{
  const f=fixture(t);const ids=[...f.doc.querySelectorAll('[id]')].map(n=>n.id);
  assert.equal(ids.length,new Set(ids).size);
  for (const attr of ['for','aria-controls','aria-labelledby']) for(const n of f.doc.querySelectorAll('['+attr+']'))
    for(const id of n.getAttribute(attr).split(/\s+/)) assert.ok(f.doc.getElementById(id),id);
  assert.match(html,/@container pb-main \(width < 600px\)/);
  assert.match(html,/@container \(width < 550px\)/);
  assert.match(html,/outline:2px solid var\(--ink\)/);assert.match(html,/overflow-wrap: anywhere/);
  function luminance(hex) {
    const rgb=hex.match(/\w\w/g).map(s=>parseInt(s,16)/255).map(n=>n<=.04045?n/12.92:((n+.055)/1.055)**2.4);
    return .2126*rgb[0]+.7152*rgb[1]+.0722*rgb[2];
  }
  const tokens=Object.fromEntries([...html.matchAll(/--(\w+): light-dark\(#(\w{6}), #(\w{6})\)/g)].map(m=>[m[1],m.slice(2,4)]));
  for (const [fg,bg] of [['text','surface'],['text','side'],['quiet','surface'],['quiet','side'],['quiet','tint'],['ink','tint'],['warn','surface'],['ink','surface'],['ink','side']]) for(const theme of [0,1]) {
    const l=[luminance(tokens[fg][theme]),luminance(tokens[bg][theme])].sort((a,b)=>b-a);
    assert.ok((l[0]+.05)/(l[1]+.05)>=4.5,fg+' on '+bg);
  }
});
