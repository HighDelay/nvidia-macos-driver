const fs = require('fs');
const vm = require('vm');
const assert = require('assert');
const elements = new Map();
const sent = [];
function element(id) {
  if (!elements.has(id)) elements.set(id, { hidden: false, disabled: false, value: '', textContent: '', innerHTML: '', options: [], classList: { toggle() {} }, insertAdjacentHTML() {}, querySelectorAll() { return [] } });
  return elements.get(id);
}
const sandbox = {document: {getElementById: element, querySelectorAll() {return []}}, window: {webkit: {messageHandlers: {nm: {postMessage(m) {sent.push(m)}}}}}, confirm: () => true, setTimeout};
vm.createContext(sandbox);
vm.runInContext(fs.readFileSync(__dirname + '/../app/Resources/app.js', 'utf8'), sandbox);
const event = (event, data) => sandbox.window.NM.on({event, data});
event('upd', {state:'checked', newer:true, latest:'1.0.9', installed:'1.0.8'});
assert.equal(element('upddrv').disabled,false);
event('dl', {state:'start'});
for (const name of ['dl','updchk','upddrv']) assert.equal(element(name).disabled,true);
event('dl', {state:'error', why:'network failed'});
for (const name of ['dl','updchk','upddrv']) assert.equal(element(name).disabled,false);
event('dl', {state:'start'});
event('dl', {state:'done',path:'/tmp/driver.tar.gz'});
assert.equal(element('updchk').disabled,false);
assert.equal(sent.at(-1).act,'scan');
event('upd', {state:'checked', newer:false, latest:'1.0.8', installed:'1.0.8'});
assert.equal(element('upddrv').hidden,true);
assert.equal(element('upddrv').disabled,true);
console.log('Update download failure, retry, completion and version controls passed.');

const machine = (major) => ({version:'1.1', macos:String(major), major, arch:'x86_64', translated:false,
  gpus:[{vendor:'10DE', name:'NVIDIA RTX 5060', supported:true, tested:true}],
  kexts:4, metal:['NVIDIA RTX 5060'], opencore:'1.0.6', files:true,
  packages:[{ok:'yes',path:'/tmp/driver.tar.gz'}]});
event('scan',machine(26));
assert.equal(element('upd').disabled,true);
assert(element('verdict').innerHTML.includes('macOS 26 is already installed'));
event('scan',machine(15));
assert.equal(element('upd').disabled,false);
const unsupported=machine(15);unsupported.opencore='';
event('scan',unsupported);assert.equal(element('upd').disabled,true);
console.log('Tahoe preparation is enabled only for a supported macOS 15 upgrade.');
