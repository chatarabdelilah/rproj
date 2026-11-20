const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const {JSDOM,VirtualConsole}=require(process.argv[2]?path.resolve(process.argv[2]):'jsdom');
const html=fs.readFileSync(path.join(__dirname,'review-update.html'),'utf8');

function fixture(t){
  const jobs=[],errors=[];
  const dom=new JSDOM(html,{runScripts:'dangerously',virtualConsole:new VirtualConsole().on('jsdomError',e=>errors.push(e.message)),
    beforeParse(w){
      w.setTimeout=fn=>jobs.push(fn);
      w.HTMLDialogElement.prototype.showModal=function(){this.open=true;};
      w.HTMLDialogElement.prototype.close=function(){this.open=false;};
    }});
  t.after(()=>{dom.window.close();assert.deepEqual(errors,[]);});
  const doc=dom.window.document,el=id=>doc.getElementById('ru-'+id);
  function click(id){el(id).focus();el(id).click();}
  function choose(id,value){el(id).value=value;el(id).dispatchEvent(new dom.window.Event('change'));}
  function load(value){choose('case',value);click('load');}
  function prepare(){click('prepare');assert.match(el('heading').textContent,/Preparing/);jobs.shift()();}
  function nav(value){[...doc.querySelectorAll('nav button')].find(b=>b.textContent===value).click();}
  return {doc,el,click,choose,load,prepare,nav,jobs};
}
test('initial comparison is readable before scripts or icons',()=>{
  const dom=new JSDOM(html);
  try{
    const d=dom.window.document;
    assert.match(d.querySelector('main').textContent,/Skyline.*Signals/s);
    assert.match(d.querySelector('#ru-note').textContent,/1.3.0 fits \^1.2.0/);
    assert.doesNotMatch(html,/\b(fetch|XMLHttpRequest|WebSocket)\s*\(|<script[^>]+src=/);
    for(const id of ['ru-dialog-title','ru-dialog-text'])assert.ok(d.getElementById(id));
    for(const e of d.querySelectorAll('input,select'))assert.ok(e.labels.length);
  }finally{dom.window.close();}
});
test('compatible, caret, exact and compound requirements expose only supported edits',t=>{
  const f=fixture(t);f.prepare();
  assert.match(f.el('effects').textContent,/Keep Signals.*sample\/task/s);assert.equal(f.el('apply').disabled,false);
  f.click('edit');f.choose('version','2.0.0');f.prepare();
  assert.match(f.el('effects').textContent,/− Signals = "sample\/signals@\^1.2.0"\n\+ Signals = "sample\/signals@\^2.0.0"/);
  f.load('exact');f.prepare();assert.match(f.el('effects').textContent,/sample\/signals@=2.0.0/);
  f.load('complex');assert.deepEqual([...f.el('version').options].map(o=>o.value),['1.3.0']);
});
test('direct and same-package alias changes block while transitive changes are reviewable',t=>{
  const f=fixture(t);
  for(const [sample,alias] of [['direct','server-dependencies.Storage'],['alias','dev-dependencies.TestSignals']]){
    f.load(sample);f.prepare();assert.equal(f.el('apply').disabled,true);
    assert.ok(f.el('note').textContent.includes(alias));assert.match(f.el('effects').textContent,/other direct dependency/);
    f.click('change');f.click('refresh');f.prepare();
    assert.match(f.el('note').textContent,new RegExp(alias,'u'));assert.equal(f.el('apply').disabled,true);
  }
});
test('Tools and Dependencies open at their target; tool pin, repository and collateral mismatches block',t=>{
  const f=fixture(t);f.nav('Tools');f.click('open');f.prepare();
  assert.match(f.el('effects').textContent,/rojo-rbx\/rojo@7.5.0/);assert.equal(f.el('apply').disabled,false);
  for(const sample of ['repo','pin','extra']){f.load(sample);f.prepare();assert.equal(f.el('apply').disabled,true);}
  f.nav('Dependencies');f.click('open');assert.match(f.el('owner').textContent,/Signals/);
});
test('unavailable discoveries explain refusal; changed inputs invalidate preparation and review',t=>{
  const f=fixture(t);
  for(const sample of ['current','ahead','unknown','unsupported','stale']){
    f.load(sample);assert.equal(f.el('prepare').disabled,true);assert.ok(f.el('note').textContent);
  }
  f.click('refresh');f.click('prepare');f.click('change');f.jobs.shift()();assert.equal(f.el('apply').hidden,true);
  f.click('refresh');f.prepare();f.click('change');assert.equal(f.el('apply').hidden,true);
});
test('same-project Watch requires acknowledged Stop; confirmation defaults to Keep and restores focus',t=>{
  const f=fixture(t);f.click('watched');f.prepare();assert.equal(f.el('apply').disabled,true);
  f.click('stop');f.click('watched');f.click('stop');f.jobs.shift()();
  assert.equal(f.el('apply').disabled,true);assert.match(f.el('watch').textContent,/Stopping/);
  f.jobs.shift()();f.click('apply');
  assert.equal(f.doc.activeElement,f.el('keep'));assert.match(f.el('dialog-text').textContent,/Signals.*1.3.0.*Skyline/);
  f.el('dialog').dispatchEvent(new f.doc.defaultView.Event('cancel',{cancelable:true}));
  assert.equal(f.doc.activeElement,f.el('apply'));f.click('apply');f.click('confirm');f.jobs.shift()();
  assert.match(f.el('heading').textContent,/Verified success/);assert.match(f.el('local').textContent,/Locked 1.3.0/);
  f.click('tool');f.click('package');assert.match(f.el('local').textContent,/Locked 1.3.0/);
  f.click('change');f.click('refresh');assert.match(f.el('heading').textContent,/Choose version/);
  assert.equal(f.el('apply').hidden,true);f.prepare();assert.match(f.el('heading').textContent,/Review changes/);
});
test('Activity retains origin, filter, height and collapse across browsing and completion',t=>{
  const f=fixture(t);f.prepare();f.click('apply');f.click('confirm');f.el('activity').open=false;
  f.el('output').style.height='215px';f.choose('project','Lantern');f.jobs.shift()();
  assert.match(f.el('summary').textContent,/Skyline · Signals · Verified/);assert.equal(f.el('activity').open,false);
  assert.equal(f.el('output').style.height,'215px');assert.match(f.el('watch').textContent,/Lantern · Watching/);
  f.el('filter').value='missing';f.el('filter').dispatchEvent(new f.doc.defaultView.Event('input'));
  assert.match(f.el('output').textContent,/No search matches/);
});
test('failure requires renewed preparation; uncertainty holds deferred close until confirmed outcome',t=>{
  const f=fixture(t);f.choose('outcome','failure');f.prepare();f.click('apply');f.click('confirm');f.jobs.shift()();
  assert.match(f.el('note').textContent,/Partial changes/);assert.equal(f.el('apply').hidden,true);f.click('refresh');
  f.choose('outcome','uncertain');f.prepare();f.click('apply');f.click('confirm');f.click('close');
  assert.equal(f.doc.activeElement,f.el('keep'));f.click('confirm');f.jobs.shift()();assert.equal(f.el('window').hidden,false);
  assert.match(f.el('heading').textContent,/uncertain/);f.click('finish');assert.equal(f.el('closed').hidden,false);
});
test('theme, responsive rule and selection survive resize',t=>{
  const f=fixture(t);f.choose('version','2.0.0');
  for(const theme of ['Dark','Light']){
    f.choose('theme',theme);assert.equal(f.doc.querySelector('#review-update').dataset.theme,theme);
    f.doc.defaultView.dispatchEvent(new f.doc.defaultView.Event('resize'));assert.equal(f.el('version').value,'2.0.0');
  }
  assert.match(html,/@container review-main \(min-width:600px\)/);assert.match(html,/:focus-visible/);
  const pairs=Object.fromEntries([...html.matchAll(/--(\w+):light-dark\(#(\w{6}),#(\w{6})\)/g)].map(m=>[m[1],m.slice(2)]));
  function lum(rgb){return rgb.match(/../g).map(c=>parseInt(c,16)/255).map(c=>c<=0.04045?c/12.92:((c+.055)/1.055)**2.4).reduce((sum,c,i)=>sum+c*[.2126,.7152,.0722][i],0);}
  for(const theme of [0,1])for(const fg of ['ink','quiet','blue'])for(const bg of ['paper','side','wash']){
    const [lo,hi]=[pairs[fg][theme],pairs[bg][theme]].map(lum).sort((a,b)=>a-b);assert.ok((hi+.05)/(lo+.05)>=4.5);
  }
});
test('completion while the close dialog is open respects Keep open and ignores duplicate delivery',t=>{
  const f=fixture(t);f.prepare();f.click('apply');f.click('confirm');f.click('close');
  const deliver=f.jobs.shift();deliver();deliver();
  assert.equal(f.doc.activeElement,f.el('keep'));assert.equal(f.el('window').hidden,false);
  f.click('keep');assert.equal(f.doc.activeElement,f.el('close'));assert.equal(f.el('window').hidden,false);
  assert.equal(f.el('output').textContent.match(/Verified 1\.3\.0/g).length,1);
});
