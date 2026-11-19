const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const {JSDOM, VirtualConsole} = require(process.argv[2] ? path.resolve(process.argv[2]) : 'jsdom');
const html = fs.readFileSync(path.join(__dirname,'update-discovery.html'),'utf8');

function sample(t) {
  const timers = [], errors = [];
  const dom = new JSDOM(html,{
    runScripts:'dangerously',
    virtualConsole:new VirtualConsole().on('jsdomError', error => errors.push(error.message)),
    beforeParse(window) {
      window.setTimeout = callback => timers.push(callback);
      window.fetch = () => { throw Error('Unexpected network request'); };
    }
  });
  t.after(() => { dom.window.close(); assert.deepEqual(errors,[]); });
  const doc = dom.window.document;
  const el = id => doc.getElementById('ud-' + id);
  function click(element) { element.focus(); element.click(); }
  function choose(id,value,event='change') {
    el(id).value = value;
    el(id).dispatchEvent(new dom.window.Event(event,{bubbles:true}));
  }
  const group = name => click(doc.querySelector('[data-group="' + name + '"]'));
  const section = name => click(doc.querySelector('[data-section="' + name + '"]'));
  const rows = () => [...el('list').querySelectorAll('button')];
  function select(alias) {
    const row = rows().find(button => button.firstChild.textContent === alias);
    assert.ok(row,alias); click(row);
  }
  return {timers,doc,el,click,choose,group,section,rows,select};
}

test('literal Skyline comparison survives absent scripts and host helpers', () => {
  const dom = new JSDOM(html);
  try {
    const d = dom.window.document;
    assert.match(d.querySelector('main').textContent,/Skyline/);
    assert.equal(d.querySelector('#ud-view').hidden,false);
    assert.match(d.querySelector('#ud-detail').textContent,/1\.3\.0 fits.*2\.0\.0 requires/);
    assert.match(d.querySelector('#ud-list').textContent,/Rojo/);
    assert.doesNotMatch(html,/\b(fetch|WebSocket|XMLHttpRequest)\s*\(|<script[^>]+src=/);
  } finally { dom.window.close(); }
});

test('both entry points open the same view with the relevant group and focus, without checking', t => {
  const f = sample(t);
  for (const [section,group] of [['Tools','Tools'],['Dependencies','Packages']]) {
    f.section(section); assert.equal(f.el('view').hidden,true);
    f.click(f.el('open')); assert.equal(f.el('view').hidden,false);
    assert.equal(f.doc.activeElement.dataset.group,group);
    assert.ok(f.rows().every(row => row.textContent.includes(group === 'Tools' ? 'Tool ·' : 'Package ·')));
  }
  f.group('All'); assert.equal(f.rows().length,5);
  assert.deepEqual(f.timers,[]);
});

test('declared, locked, compatible, latest, and source appear separately for manual dependencies', t => {
  const f = sample(t);
  const fields = () => Object.fromEntries([...f.el('detail').querySelectorAll('dt')].map(dt => [dt.textContent,dt.nextSibling.textContent]));
  assert.deepEqual(fields(),{Package:'sample/signal',Origin:'Manual Wally declaration',Declared:'^1.2.0',
    Locked:'1.2.0','Package files':'Present',Registry:'github.com/UpliftGames/wally-index','Within requirement':'1.3.0','Latest stable':'2.0.0'});
  f.select('ServerSignals'); assert.equal(fields().Registry,'registry.example/studio');
  assert.equal(fields()['Latest stable'],'1.2.1'); assert.match(f.el('detail').textContent,/matches the latest/);
  f.select('TestSignals'); assert.equal(fields().Locked,'Missing'); assert.equal(fields()['Package files'],'Missing');
  assert.match(f.el('detail').textContent,/No locked version/);
  f.select('Rojo'); assert.equal(fields()['Configured pin'],'7.4.0'); assert.equal(fields()['Executable on PATH'],'Detected');
  f.select('StudioRojo'); assert.match(fields().Repository,/sample-studio/); assert.equal(fields()['Latest stable'],'Unknown');
});

test('explicit checks cover hidden groups, retain partial history, and recover', t => {
  const f = sample(t);
  f.group('Packages'); f.choose('outcome','partial'); f.click(f.el('check'));
  assert.equal(f.el('check').disabled,true);
  assert.match(f.el('status').textContent,/Checking/);
  f.timers[0](); assert.match(f.el('status').textContent,/Partial.*09:01/);
  f.group('Tools'); f.select('Rojo');
  assert.match(f.el('detail').textContent,/Latest stableUnknown.*Last observed at 09:00 UTC7.5.0/);
  f.choose('outcome','ok'); f.click(f.el('check')); f.timers[1]();
  assert.match(f.el('detail').textContent,/Latest stable7\.5\.0/);
  assert.doesNotMatch(f.el('detail').textContent,/Last observed/);
});

test('offline and rate limits retain the last successful timestamp and never certify an unknown source', t => {
  const f = sample(t);
  for (const [index,outcome] of ['offline','limited'].entries()) {
    f.choose('outcome',outcome); f.click(f.el('check')); f.timers[index]();
    assert.match(f.el('status').textContent,/Showing previous results.*09:00/);
    assert.match(f.el('detail').textContent,/2\.0\.0/);
  }
  f.choose('project','Lantern');
  assert.match(f.el('status').textContent,/Never checked/);
  f.click(f.el('check')); f.timers[2](); assert.match(f.el('status').textContent,/Rate limited.*No successful check/);
  assert.match(f.el('detail').textContent,/No saved release information/);
  assert.doesNotMatch(f.el('detail').textContent,/latest stable release from/);
});

test('late and duplicate results retain their owner through switch, close, and reopen', t => {
  const f = sample(t);
  f.click(f.el('check')); f.choose('project','Lantern');
  f.click(f.el('check')); f.timers[0]();
  assert.match(f.el('context').textContent,/Lantern/); assert.match(f.el('status').textContent,/Checking.*Never checked/);
  f.choose('project',''); f.timers[1](); assert.equal(f.el('title').textContent,'No project open');
  f.choose('project','Lantern'); f.section('Tools'); f.click(f.el('open'));
  assert.match(f.el('status').textContent,/09:02/); assert.match(f.el('detail').textContent,/matches the latest/);
  f.group('Packages'); assert.match(f.el('detail').textContent,/Multiple lockfile matches/);
  f.click(f.el('check')); f.timers[1](); assert.equal(f.el('check').disabled,true); f.timers[2]();
  assert.match(f.el('status').textContent,/09:03/);
  f.choose('project','Skyline'); assert.match(f.el('status').textContent,/09:01/);
  assert.match(f.doc.querySelector('footer').textContent,/Watch · Skyline · Watching/);
});

test('Activity retains owner, height, filter and manual collapse after successful checks', t => {
  const f = sample(t);
  f.click(f.el('check')); assert.equal(f.el('activity').open,true); f.el('activity').open = false;
  f.el('output').style.height = '220px'; f.choose('project','Lantern'); f.timers[0]();
  assert.equal(f.el('activity').open,false); assert.equal(f.el('output').style.height,'220px');
  f.choose('filter','lantern','input'); assert.match(f.el('output').textContent,/No search matches/);
  f.choose('filter','complete','input'); assert.equal(f.el('output').textContent,'Skyline · Check complete');
});

test('responsive selection, native control labels, visible focus and readable theme pairs', t => {
  const f = sample(t);
  const ids = [...f.doc.querySelectorAll('[id]')].map(e => e.id);
  assert.equal(new Set(ids).size,ids.length);
  for (const control of f.doc.querySelectorAll('select,input')) assert.ok(control.labels.length,control.id);
  f.select('ServerSignals'); f.doc.defaultView.dispatchEvent(new f.doc.defaultView.Event('resize'));
  assert.equal(f.doc.activeElement.firstChild.textContent,'ServerSignals');
  for (const theme of ['Light','Dark']) {
    f.choose('theme',theme); assert.equal(f.doc.querySelector('#rp-updates').dataset.theme,theme.toLowerCase());
    assert.match(f.el('detail').textContent,/ServerSignals/);
  }
  assert.match(html,/@container updates-main \(min-width:600px\)/);
  assert.match(html,/:focus-visible\{outline:2px/);
  const colors = Object.fromEntries([...html.matchAll(/--(\w+):light-dark\((#\w+),(#\w+)\)/g)].map(m => [m[1],m.slice(2)]));
  function luminance(hex) {
    const linear = hex.slice(1).match(/../g).map(c => parseInt(c,16)/255)
      .map(c => c <= 0.04045 ? c/12.92 : ((c+0.055)/1.055)**2.4);
    return linear[0]*0.2126 + linear[1]*0.7152 + linear[2]*0.0722;
  }
  for (const theme of [0,1]) for (const fg of ['ink','muted','blue']) for (const bg of ['paper','side','wash']) {
    const [low,high] = [colors[fg][theme],colors[bg][theme]].map(luminance).sort((a,b) => a-b);
    assert.ok((high+0.05)/(low+0.05) >= 4.5,fg+' on '+bg);
  }
});
