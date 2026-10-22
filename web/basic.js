(function () {
'use strict';

blipSetFrontend('basic');
document.getElementById('basic-to-cabinet').addEventListener('click', function () { blipSetFrontend('cabinet'); });
document.getElementById('basic-to-home').addEventListener('click', function () { blipSetFrontend('home'); });

var rows = [
  ['ESC','1','2','3','4','5','6','7','8','9','0','-','=','DEL'],
  ['TAB','Q','W','E','R','T','Y','U','I','O','P','[',']','RETURN'],
  ['CTRL','A','S','D','F','G','H','J','K','L',';',"'",'↵'],
  ['SHIFT','Z','X','C','V','B','N','M',',','.','/','SHIFT'],
  ['COPY','ALT','SPACE','ALT','←','↓','↑','→']
];
var keyboard = document.getElementById('basic-keyboard');
rows.forEach(function (row) {
  var el = document.createElement('div'); el.className = 'key-row';
  row.forEach(function (label) {
    var key = document.createElement('button'); key.type = 'button'; key.className = 'key'; key.textContent = label;
    key.dataset.keyLabel = label; key.setAttribute('aria-label', label === 'SPACE' ? 'Space' : label);
    if (label === 'SPACE') key.classList.add('space');
    if (label === 'RETURN') key.classList.add('wide', 'return');
    if (label === 'SHIFT') key.classList.add('wide', 'shift');
    if (label === 'CTRL') key.classList.add('ctrl');
    if (/^(ESC|TAB|CTRL|COPY)$/.test(label)) key.classList.add('green');
    if (/^(←|↓|↑|→)$/.test(label)) key.classList.add('blue');
    if (label === 'RETURN') key.classList.add('red');
    el.appendChild(key);
  });
  keyboard.appendChild(el);
});
function fitKeyLegends(){
  var compact=window.matchMedia('(max-width: 420px)').matches;
  var short={ESC:'⎋',TAB:'⇥',DEL:'⌫',RETURN:'ENT',CTRL:'CTL',SHIFT:'SHFT',COPY:'CPY'};
  keyboard.querySelectorAll('[data-key-label]').forEach(function(key){
    var label=key.dataset.keyLabel;
    key.textContent=compact&&short[label]?short[label]:label;
  });
}
fitKeyLegends();
window.addEventListener('resize',fitKeyLegends);

var output = document.getElementById('basic-output');
var form = document.getElementById('basic-form');
var input = document.getElementById('basic-input');
var editorPanel=document.getElementById('basic-editor'),editorFile=document.getElementById('editor-file'),editorCode=document.getElementById('editor-code'),editorMode=document.getElementById('editor-mode'),editorMessage=document.getElementById('editor-message');
if (blipHasTouch()) input.readOnly = true;
var canvas = document.getElementById('basic-graphics');
var wasmFrame=document.getElementById('basic-wasm-frame'),basicWasmPlaying=false,basicWasmReady=false,basicWasmSource='',basicWasmRunId=0;
const basicMathNames = [
  'MIN', 'MAX', 'ATN', 'ATAN2',
  'CLAMP', 'LERP', 'WRAP', 'DISTANCE', 'ANGLE_DIFF', 'SMOOTHSTEP01',
  'EASE_OUT_CUBIC', 'SCATTER', 'RECTS_OVERLAP',
  'SEGMENT_CIRCLE_OVERLAP', 'SEGMENT_RECT_OVERLAP',
];
const basicMathPattern = new RegExp('\\b(?:' + basicMathNames.join('|') + ')\\s*\\(');
let basicMathLoading = null;
let basicMathCommands = [];

function usesSharedMath(source) {
  return basicMathPattern.test(source.replace(/"(?:[^"]|"")*"/g, '""'));
}

function sharedMathReady() {
  return typeof wasmFrame.contentWindow?.wasm_exports?.blip_basic_math_clamp === 'function';
}

function needsSharedMath(command) {
  if (sharedMathReady()) return false;
  return usesSharedMath(command)
    || command === 'RUN' && Array.from(lines.values()).some(usesSharedMath);
}

function loadSharedMath() {
  return new Promise(function (resolve, reject) {
    const timer = setTimeout(() => finish(Error('Shared functions could not load')), 20000);
    function finish(error) {
      clearTimeout(timer);
      window.removeEventListener('message', loaded);
      if (error) reject(error);
      else resolve();
    }
    function loaded(event) {
      if (event.origin === location.origin && event.source === wasmFrame.contentWindow
          && event.data?.type === 'blip-basic-wasm-ready') finish();
    }
    window.addEventListener('message', loaded);
    if (!wasmFrame.hasAttribute('src')) wasmFrame.src = 'basic-runtime.html?v=18';
  });
}

function queueSharedMathCommand(command) {
  basicMathCommands.push(command);
  if (!basicMathLoading) {
    basicMathLoading = loadSharedMath().then(function () {
      const commands = basicMathCommands.splice(0);
      basicMathLoading = null;
      commands.forEach(submit);
    }).catch(function (error) {
      basicMathLoading = null;
      basicMathCommands = [];
      fail(error.message);
    });
  }
  return basicMathLoading;
}
var exitGameButton = document.getElementById('exit-game');
var monitorFullscreenButton = document.getElementById('monitor-fullscreen');
monitorFullscreenButton.addEventListener('click',function(){
  var expanded=document.body.toggleAttribute('data-monitor-expanded');
  monitorFullscreenButton.setAttribute('aria-pressed',String(expanded));
  monitorFullscreenButton.setAttribute('aria-label',expanded?'Restore monitor':'Maximize monitor');
  monitorFullscreenButton.title=expanded?'Restore monitor':'Maximize monitor';
  monitorFullscreenButton.textContent=expanded?'⤡':'⤢';
  updatePlayClearance();
  if(basicWasmPlaying){
    var gameCanvas=wasmFrame.contentDocument&&wasmFrame.contentDocument.querySelector('#glcanvas');
    if(gameCanvas)gameCanvas.focus({preventScroll:true});
  }else if(!blipHasTouch())input.focus({preventScroll:true});
});
var soundContext=null,soundCursor=0;
var tapeLoadingAudio=new Audio('blip-tape-load.mp3');
tapeLoadingAudio.preload='auto';
var controlDeck = document.querySelector('.amstrad-controls');
var controlStick = controlDeck.querySelector('.stick-base');
var controlFire = controlDeck.querySelector('.fire-buttons');
var controlSecond = document.getElementById('deck-p2');
var controlSticks = Array.from(controlDeck.querySelectorAll('.stick-base'));
var rallyDialP1 = document.getElementById('amstrad-rally-dial-p1');
var rallyDialP2 = document.getElementById('amstrad-rally-dial-p2');
function programTape() {
  var name = String(currentFile || '').split('/').pop().toUpperCase();
  var identity = Array.from(lines.values()).map(function (source) { return /^REM\s+GAME:\s*([A-Z_]+)\s*$/i.exec(source); }).find(Boolean);
  if (identity) name = identity[1].toUpperCase() + '.BLIP';
  return tapePrograms.find(function (item) { return item.file === name; });
}
function configureBasicControls() {
  BlipController.releaseAll();
  var tape = programTape();
  var game = tape && BLIP_GAMES[tape.slug] || {};
  document.body.toggleAttribute('data-rally', !!(tape && tape.slug === 'rally'));
  showBasicSecondPlayer(!!(tape && tape.players === 2));
  BlipController.init({
    canvas: { dispatchEvent: function (event) { sendBasicWasmKey(event.key, event.code, event.type === 'keydown'); } },
    gate: function () { return !basicWasmPlaying; },
    buttons: game.buttons,
    keys: game.keys,
    click: false,
    onInput: function () {
      controlSticks.forEach(function (stick, index) {
        var prefix = index ? 'p2' : '';
        stick.style.setProperty('--dx', Number(BlipController.isHeld(prefix + 'right')) - Number(BlipController.isHeld(prefix + 'left')));
        stick.style.setProperty('--dy', Number(BlipController.isHeld(prefix + 'down')) - Number(BlipController.isHeld(prefix + 'up')));
      });
    }
  });
  controlDeck.querySelectorAll('.fire-buttons').forEach(function (fire, player) {
    fire.querySelectorAll('.arcade-btn').forEach(function (button, index) {
      var spec = game.buttons && (game.buttons[index] || game.buttons[0]);
      button.classList.remove('spare');
      button.setAttribute('aria-label', 'Player ' + (player + 1) + ' ' + (spec && spec.label || 'Fire'));
      button.classList.toggle('alt', index === 1 && !!(game.buttons && game.buttons.length > 1));
    });
  });
}
function showBasicSecondPlayer(show) {
  controlSecond.hidden = !show;
  document.documentElement.toggleAttribute('data-open', show);
  document.documentElement.setAttribute('data-seats', show ? '2' : '1');
  document.documentElement.setAttribute('data-players', show ? '2' : '1');
}
showBasicSecondPlayer(false);
function releaseBasicRallyDials() { BlipController.releaseAll(); }
BlipController.init({ click: false });
controlSticks.forEach(function (stick, index) {
  var prefix = index ? 'p2' : '';
  BlipController.bindDpad(stick, { dead: .19, names: {
    left: prefix + 'left', right: prefix + 'right', up: prefix + 'up', down: prefix + 'down'
  } });
});
controlDeck.querySelectorAll('.fire-buttons').forEach(function (fire, player) {
  fire.querySelectorAll('.arcade-btn').forEach(function (button, index) {
    button.setAttribute('data-blip', (player ? 'p2' : '') + 'button' + (index + 1));
  });
  BlipController.bindButtons(fire);
});
BlipController.bindDial(rallyDialP1, {
  up: { key: 'ArrowUp', code: 'ArrowUp' }, down: { key: 'ArrowDown', code: 'ArrowDown' },
  tap: { key: ' ', code: 'Space' }
});
BlipController.bindDial(rallyDialP2, {
  up: { key: 'i', code: 'KeyI' }, down: { key: 'k', code: 'KeyK' }, tap: { key: 'i', code: 'KeyI' }
});
window.addEventListener('blur', function () { BlipController.releaseAll(); });
function updatePlayClearance(){
  var controls=document.body.hasAttribute('data-rally')?[rallyDialP1,rallyDialP2]:[controlDeck,controlStick,controlFire];
  var boxes=controls.map(function(el){return el.getBoundingClientRect();}).filter(function(box){return box.width>0&&box.height>0;});
  var top=Math.min.apply(Math,[document.documentElement.clientHeight].concat(boxes.map(function(box){return box.top;})));
  document.documentElement.style.setProperty('--amstrad-control-clearance',Math.max(0,document.documentElement.clientHeight-top)+'px');
}
updatePlayClearance();
window.addEventListener('resize',updatePlayClearance);
window.addEventListener('orientationchange',function(){requestAnimationFrame(updatePlayClearance);});
var gfx = canvas.getContext('2d');
var palette = ['#101da3','#fff5a4','#00d8e6','#ff5696','#a5ea6a','#ffad44','#e6e6e6','#131313'];
var pen = 1, paper = 0, gx = 0, gy = 0;
var lines = new Map();
var vars = Object.create(null);
var arrays = Object.create(null);
var lastKey = '';
var heldKeys=Object.create(null),pressedKeys=Object.create(null);
var run = null, pendingInput = null, history = [], historyAt = 0;
var DRAFT = 'blip-basic-draft';
var FS_STORE = 'blip-basic-filesystem';
var LEGACY_STORE = 'blip-basic-programs';
var CURRENT_FILE_STORE = 'blip-basic-current-file';
var tapeFiles=Object.create(null),gfxNames=Object.create(null),tapeNames=[],tapeLoadError=null,tapeBusy=false,tapeLoading=false;
var tapePrograms=[
  {file:'ADDER.BLIP',command:'ADDER',slug:'adder',title:'ADDER',players:2},
  {file:'BOUNCER.BLIP',command:'BOUNCER',slug:'bouncer',title:'BOUNCER'},
  {file:'BRAWLER.BLIP',command:'BRAWLER',slug:'brawler',title:'BRAWLER'},
  {file:'BUBBLER.BLIP',command:'BUBBLER',slug:'bubbler',title:'BUBBLER',players:2},
  {file:'DEFENDER.BLIP',command:'DEFENDER',slug:'galactic_defender',title:'GALACTIC DEFENDER'},
  {file:'METEORS.BLIP',command:'METEORS',slug:'meteors',title:'METEORS'},
  {file:'RALLY.BLIP',command:'RALLY',slug:'rally',title:'RALLY'},
  {file:'SERPENT.BLIP',command:'SERPENT',slug:'serpent',title:'SERPENT'},
  {file:'SKY_RAIDER.BLIP',command:'SKY_RAIDER',slug:'sky_raider',title:'SKY RAIDER'}
];
var tapeReady=fetch('TAPE/manifest.json').then(function(response){if(!response.ok)throw Error('TAPE DIRECTORY UNAVAILABLE');return response.json();}).then(function(names){if(!Array.isArray(names))throw Error('BAD TAPE MANIFEST');tapeNames=names.filter(function(name){return tapePrograms.some(function(tape){return tape.file===name;});});}).catch(function(error){tapeLoadError=error.message;});
var gfxReady=fetch('TAPE/GFX/manifest.json').then(function(response){if(!response.ok)throw Error('GFX directory unavailable');return response.json();}).then(function(games){if(games&&typeof games==='object')gfxNames=games;}).catch(function(){});
var romTunes=[];
var romTunesReady=fetch('ROM/TUNES/manifest.json').then(function(response){if(!response.ok)throw Error('ROM TUNES unavailable');return response.json();}).then(function(files){if(Array.isArray(files))romTunes=files.filter(function(name){return typeof name==='string'&&/\.xm$/i.test(name);});}).catch(function(){});
var currentDir = '/USER';
var currentFile = null;
var editor=null;
function readFilesystem(){
  var state={dirs:['/USER'],files:{}};
  try{
    var saved=JSON.parse(localStorage.getItem(FS_STORE)||'null');
    if(saved&&Array.isArray(saved.dirs)&&saved.files&&typeof saved.files==='object'){
      state.dirs=state.dirs.concat(saved.dirs.filter(function(path){return typeof path==='string'&&path.indexOf('/USER/')===0;}).map(function(path){return path.toUpperCase();}));
      Object.keys(saved.files).forEach(function(path){if(path.toUpperCase().indexOf('/USER/')===0&&typeof saved.files[path]==='string'){var value=saved.files[path];state.files[path.toUpperCase()]=value.indexOf('data:image/png;')===0?value:value.toUpperCase();}});
    }
    var legacy=JSON.parse(localStorage.getItem(LEGACY_STORE)||'{}');
    Object.keys(legacy).forEach(function(name){
      var safe=name.replace(/[^a-z0-9_-]/gi,'_');
      var path='/USER/'+safe.toUpperCase()+'.BLIP';
      if(!state.files[path]&&typeof legacy[name]==='string')state.files[path]=legacy[name].toUpperCase();
    });
  }catch(e){}
  return state;
}
var filesystem=readFilesystem();
function writeFilesystem(){
  localStorage.setItem(FS_STORE,JSON.stringify(filesystem));
}
function canonicalRoot(path){
  return path.replace(/^\/(rom|user)(?=\/|$)/i,function(match,root){return '/'+root.toUpperCase();});
}
function resolvePath(path,base){
  path=String(path||'').trim();
  if(!path)throw Error('Path expected');
  var parts=path.charAt(0)==='/'?[]:String(base||currentDir).split('/').filter(Boolean);
  path.split('/').forEach(function(part){
    if(!part||part==='.')return;
    if(part==='..'){parts.pop();return;}
    if(/[\\\\]/.test(part))throw Error('Bad path');
    parts.push(part.toUpperCase());
  });
  return canonicalRoot('/'+parts.join('/'));
}
function canonicalDirectory(path){
  var target=canonicalRoot(path).toLowerCase();
  var roots=mountedDirectories();
  return roots.find(function(dir){return dir.toLowerCase()===target;})||path;
}
function programPath(name){
  var path=resolvePath(name);
  if(!/\.blip$/i.test(path))path+='.blip';
  var parts=path.split('/').filter(Boolean);
  if(!parts.length||!parts.slice(0,-1).every(function(part){return /^[a-z0-9 _-]+$/i.test(part);})||! /^[a-z0-9 _-]+\.blip$/i.test(parts[parts.length-1]))throw Error('Bad program name');
  return path.slice(0,path.lastIndexOf('/')+1)+path.slice(path.lastIndexOf('/')+1).toUpperCase();
}
function findFile(path){
  var all=(insertedTape?['/TAPE/'+insertedTape.file]:[]).concat(Object.keys(filesystem.files));
  return all.find(function(file){return file.toLowerCase()===path.toLowerCase();})||path;
}
function mountedDirectories(){
  return ['/','/ROM','/ROM/TUNES','/TAPE','/USER'].concat(insertedTape?['/TAPE/GFX','/TAPE/GFX/'+insertedTape.slug.toUpperCase()]:[],filesystem.dirs);
}
function isFileAvailable(path){
  path=String(path).toUpperCase();
  if(path.indexOf('/TAPE/')!==0)return true;
  if(!insertedTape)return false;
  if(path==='/TAPE/'+insertedTape.file.toUpperCase())return true;
  var prefix='/TAPE/GFX/'+insertedTape.slug.toUpperCase()+'/';
  return path.indexOf(prefix)===0&&(gfxNames[insertedTape.slug.toUpperCase()]||[]).some(function(name){return prefix+name.toUpperCase()===path;});
}
function tapeForPath(path){return tapePrograms.find(function(tape){return ('/TAPE/'+tape.file).toLowerCase()===String(path).toLowerCase();})||null;}
var tapeOrder=tapePrograms.slice();
var tapeNodes=Object.create(null),insertedTape=null,insertedTapeNode=null,tapePlaceholder=null;
function makeTapeNode(tape,index){
  var game=BLIP_GAMES[tape.slug],item=document.createElement('div'),caseEl=document.createElement('span'),art=document.createElement('img'),label=document.createElement('span'),title=document.createElement('strong'),reels=document.createElement('span'),film=document.createElement('i');
  item.className='game-tape';item.dataset.tape=tape.command;item.tabIndex=0;item.setAttribute('role','button');item.setAttribute('aria-label','Load '+tape.title+' game tape');item.style.setProperty('--tape-accent','rgb('+game.accent+')');item.style.setProperty('--tape-index',String(index));
  caseEl.className='game-tape-case';art.className='game-tape-art';art.src=tape.slug+'/screenshot.png';art.alt='';label.className='game-tape-label';title.textContent=tape.title;reels.className='game-tape-reels';reels.innerHTML='<i class="tape-spool supply"></i><i class="tape-spool takeup"></i>';film.className='game-tape-film';
  label.append(title);caseEl.append(art,label,reels,film);item.append(caseEl);item.addEventListener('click',function(){if(!tapeDragged)startTape(tape,item);});item.addEventListener('keydown',function(event){if(event.key==='Enter'||event.key===' '){event.preventDefault();startTape(tape,item);}});return item;
}
function tapeAngle(index){return Math.sin((index+1)*1.618)*2.2+Math.sin(index*.73)*.65;}
function updateTapeStackGeometry(){
  var pile=document.getElementById('tape-pile');if(!pile)return;
  var vertical=window.matchMedia('(min-width:820px) and (min-height:501px)').matches,overlap=vertical?68:(window.matchMedia('(max-width:650px)').matches?116:24),previousDelta=0;
  Array.from(pile.children).forEach(function(item,index){
    var angle=tapeAngle(Number(item.style.getPropertyValue('--tape-index'))||index),radians=angle*Math.PI/180,width=item.offsetWidth,height=item.offsetHeight;
    var extent=vertical?height*Math.abs(Math.cos(radians))+width*Math.abs(Math.sin(radians)):width*Math.abs(Math.cos(radians))+height*Math.abs(Math.sin(radians));
    var delta=extent-(vertical?height:width),adjustment=(previousDelta+delta)/2-overlap;
    item.style.marginTop=vertical&&index?adjustment+'px':'0px';item.style.marginLeft=!vertical&&index?adjustment+'px':'0px';
    item.style.setProperty('--tape-shadow-x',(2+angle*.25).toFixed(2)+'px');item.style.setProperty('--tape-shadow-y',(3+Math.abs(angle)*.45).toFixed(2)+'px');item.style.setProperty('--tape-shadow-blur',(2+Math.abs(angle)*.18).toFixed(2)+'px');previousDelta=delta;
  });
}
function renderTapePile(){
  var pile=document.getElementById('tape-pile');if(!pile)return;
  pile.replaceChildren();
  tapeOrder.forEach(function(tape,index){
    var item=tapeNodes[tape.command]||(tapeNodes[tape.command]=makeTapeNode(tape,index));
    item.style.setProperty('--tape-index',String(index));
    item.style.setProperty('--tape-angle',tapeAngle(index).toFixed(3)+'deg');
    if(insertedTape===tape){if(tapePlaceholder)pile.append(tapePlaceholder);return;}
    if(tapePlaceholder&&tapePlaceholder.dataset.tape===tape.command){pile.append(tapePlaceholder);return;}
    pile.append(item);
  });
  updateTapeStackGeometry();
}
window.addEventListener('resize',function(){renderTapePile();seatBox(document.querySelector('.deck-bay'));});
function reducedTapeMotion(){return window.matchMedia&&window.matchMedia('(prefers-reduced-motion: reduce)').matches;}
function waitForMotion(ms){return new Promise(function(resolve){setTimeout(resolve,reducedTapeMotion()?0:ms);});}
function openDeck(){pressDeckKey('eject');document.body.setAttribute('data-deck-open','');return waitForMotion(380);}
function closeDeck(){document.body.removeAttribute('data-deck-open');return waitForMotion(300);}
function pressDeckKey(key){document.body.setAttribute('data-deck-key',key);setTimeout(function(){if(document.body.getAttribute('data-deck-key')===key)document.body.removeAttribute('data-deck-key');},180);}
function leavePile(tape,node){
  var pile=document.getElementById('tape-pile'),index=tapeOrder.indexOf(tape),next=tapeOrder.slice(index+1).map(function(nextTape){return tapeNodes[nextTape.command];}).find(function(nextNode){return nextNode&&nextNode.parentElement===pile;});
  tapePlaceholder=document.createElement('div');tapePlaceholder.className='game-tape tape-placeholder';tapePlaceholder.dataset.tape=tape.command;tapePlaceholder.setAttribute('aria-hidden','true');tapePlaceholder.style.cssText=node.style.cssText;pile.insertBefore(tapePlaceholder,next||null);
}
// Unrotated box of a tape: its centre on screen, its layout size and its tilt in the pile.
function tapeBox(node){var rect=node.getBoundingClientRect(),matrix=new DOMMatrixReadOnly(getComputedStyle(node).transform);return {x:rect.left+rect.width/2,y:rect.top+rect.height/2,w:node.offsetWidth,h:node.offsetHeight,angle:Math.atan2(matrix.b,matrix.a)*180/Math.PI};}
// Measure the seat inside the tilted computer, so the flying tape meets the actual bay.
function seatBox(bay){
  bay.style.setProperty('--tape-seat-width',Math.min(bay.clientWidth*.84,bay.clientHeight*.67*1.58)+'px');
  var rect=bay.querySelector('.deck-seat').getBoundingClientRect();
  return {x:rect.left+rect.width/2,y:rect.top+rect.height/2,w:rect.width,h:rect.height,angle:0};
}
function setFlightBox(node,box){
  node.classList.add('game-tape-flight');node.style.left=(box.x-box.w/2)+'px';node.style.top=(box.y-box.h/2)+'px';node.style.width=box.w+'px';node.style.height=box.h+'px';document.body.append(node);
}
function clearFlightBox(node){node.classList.remove('game-tape-flight');['left','top','width','height'].forEach(function(prop){node.style.removeProperty(prop);});}
// Each pose: offset from the flight box, scale, turn, tilt of the top edge towards the viewer, height above the desk (0-1).
function flyTape(node,poses,duration){
  if(reducedTapeMotion()||!node.animate)return waitForMotion(0);
  var caseEl=node.querySelector('.game-tape-case');
  var body=node.animate(poses.map(function(p){return {offset:p.at,easing:p.ease||'ease-in-out',transform:'translate('+p.x.toFixed(1)+'px,'+p.y.toFixed(1)+'px) rotate('+p.r.toFixed(2)+'deg) scale('+p.s.toFixed(4)+')'};}),{duration:duration,fill:'forwards'});
  var tilt=caseEl.animate(poses.map(function(p){var l=p.lift;return {offset:p.at,easing:p.ease||'ease-in-out',transform:'perspective(520px) rotateX('+p.tilt.toFixed(1)+'deg)',boxShadow:'inset 0 1px 2px #ffffff9c,inset 0 -4px 7px #000b,'+(l*7).toFixed(1)+'px '+(2+l*20).toFixed(1)+'px '+(3+l*22).toFixed(1)+'px rgba(0,0,0,'+(.75-l*.3).toFixed(2)+')'};}),{duration:duration,fill:'forwards'});
  return Promise.all([body.finished,tilt.finished]).catch(function(){}).then(function(){return [body,tilt];});
}
async function returnTapeToPile(keepOpen){
  if(!insertedTapeNode||!insertedTape)return;
  var tape=insertedTape,node=insertedTapeNode;
  document.body.setAttribute('data-tape-rewinding','');delete node.dataset.wound;await waitForMotion(1800);document.body.removeAttribute('data-tape-rewinding');
  var from=seatBox(node.parentElement);
  await openDeck();
  if(!tapePlaceholder||tapePlaceholder.dataset.tape!==tape.command)leavePile(tape,node);
  var to=tapeBox(tapePlaceholder),s0=from.w/to.w,dx=from.x-to.x,dy=from.y-to.y;
  insertedTape=null;insertedTapeNode=null;
  if(currentDir.indexOf('/TAPE/')===0)currentDir='/TAPE';
  node.classList.remove('deck-inserted-tape');setFlightBox(node,to);
  // The eject spring pops the top edge up, a hand tips the tape out by it and carries it back to its slot.
  var done=await flyTape(node,[
    {at:0,x:dx,y:dy,s:s0,r:0,tilt:0,lift:0,ease:'cubic-bezier(.2,1.6,.4,1)'},
    {at:.1,x:dx,y:dy-from.h*.05,s:s0*1.02,r:0,tilt:-14,lift:.15},
    {at:.26,x:dx,y:dy+from.h*.6,s:s0*1.08,r:0,tilt:-20,lift:.6},
    {at:.62,x:dx*.45+6,y:dy*.45-Math.max(40,to.h*.5),s:(s0+1)*.56,r:4,tilt:-10,lift:1},
    {at:.88,x:0,y:-12,s:1.06,r:to.angle,tilt:-3,lift:.55,ease:'cubic-bezier(.4,0,.6,1.4)'},
    {at:1,x:0,y:0,s:1,r:to.angle,tilt:0,lift:0}
  ],1350);
  node.style.transition='none';clearFlightBox(node);tapePlaceholder.replaceWith(node);tapePlaceholder=null;
  if(done)done.forEach(function(animation){animation.cancel();});
  node.tabIndex=0;node.setAttribute('role','button');node.setAttribute('aria-label','Load '+tape.title+' game tape');renderTapePile();void node.offsetWidth;node.style.removeProperty('transition');
  document.body.removeAttribute('data-deck-tape');
  if(!keepOpen)await closeDeck();
}
async function insertTapeFromPile(tape){
  var node=tapeNodes[tape.command],bay=document.querySelector('.deck-bay');if(!node||!bay)return;
  var lidReady=document.body.hasAttribute('data-deck-open')?Promise.resolve():openDeck();
  var from=tapeBox(node),to=seatBox(bay),scale=to.w/from.w,dx=to.x-from.x,dy=to.y-from.y;
  var approachX=from.y<to.y?dx+(from.x>=to.x?1:-1)*to.w*1.25:dx*.7;
  leavePile(tape,node);setFlightBox(node,from);
  // Keep the original cassette size through pickup; approach below the top hinge.
  var done=await flyTape(node,[
    {at:0,x:0,y:0,s:1,r:from.angle,tilt:4,lift:0,ease:'cubic-bezier(.25,.1,.35,1)'},
    {at:.22,x:-12,y:-30,s:1.04,r:from.angle*.5,tilt:-8,lift:.8},
    {at:.6,x:approachX,y:dy+to.h*.7,s:scale*1.05,r:-3,tilt:-12,lift:1},
    {at:.84,x:dx,y:dy+to.h*.7,s:scale,r:0,tilt:-6,lift:.5},
    {at:1,x:dx,y:dy+to.h*.42,s:scale,r:0,tilt:0,lift:.3}
  ],1900);
  await lidReady;
  clearFlightBox(node);node.classList.add('game-tape-inserting');bay.append(node);
  var settle=reducedTapeMotion()?null:node.animate([
    {transform:'translate(-50%,-8%)'},
    {transform:'translate(-50%,-50%)'}
  ],{duration:650,easing:'cubic-bezier(.3,.1,.35,1)',fill:'forwards'});
  if(done)done.forEach(function(animation){animation.cancel();});
  if(settle){await settle.finished;settle.cancel();}
  await waitForMotion(240);
  node.tabIndex=-1;node.setAttribute('role','img');node.setAttribute('aria-label',tape.title+' game tape loaded');
  insertedTape=tape;insertedTapeNode=node;document.body.setAttribute('data-deck-tape','');await closeDeck();
}
var tapeDrag=null,tapeDragged=false;
async function ejectTape(){
  if(tapeBusy||tapeLoading)throw Error('Tape transport busy');
  tapeBusy=true;
  try{
    if(editor)closeEditor();
    exitActiveProgram();
    await returnTapeToPile(false);
    document.body.removeAttribute('data-tape-loaded');
    document.body.removeAttribute('data-tape-name');
    print('Tape ejected');
  }finally{tapeBusy=false;}
}
document.querySelector('.tape-buttons [data-key=eject]').addEventListener('click',function(){ejectTape().then(ready).catch(function(error){fail(error.message);});});
function shuffleTapePile(){
  var old=tapeOrder.map(function(tape){return tape.file;}).join(',');
  do{for(var i=tapeOrder.length-1;i>0;i--){var j=Math.floor(Math.random()*(i+1));[tapeOrder[i],tapeOrder[j]]=[tapeOrder[j],tapeOrder[i]];}}while(tapeOrder.map(function(tape){return tape.file;}).join(',')===old);
  renderTapePile();
}
document.getElementById('tape-pile').addEventListener('pointerdown',function(event){
  var item=event.target.closest('.game-tape');if(!item||tapeBusy)return;
  var pile=document.getElementById('tape-pile');
  tapeDrag={id:event.pointerId,x:event.clientX,y:event.clientY,item:item,pile:pile,scroll:pile.scrollLeft,horizontal:!window.matchMedia('(min-width:820px) and (min-height:501px)').matches,scrolling:false};
  try{item.setPointerCapture(event.pointerId);}catch(e){}
});
document.getElementById('tape-pile').addEventListener('pointermove',function(event){
  if(!tapeDrag||event.pointerId!==tapeDrag.id)return;
  var dx=event.clientX-tapeDrag.x,dy=event.clientY-tapeDrag.y;
  if(tapeDrag.horizontal&&Math.abs(dx)>8&&Math.abs(dx)>Math.abs(dy)*1.2){
    tapeDrag.scrolling=true;tapeDragged=true;tapeDrag.item.style.removeProperty('--drag-x');tapeDrag.pile.scrollLeft=tapeDrag.scroll-dx;return;
  }
  if(tapeDrag.horizontal)return;
  if(Math.abs(dx)>8||Math.abs(dy)>8)tapeDrag.item.style.setProperty('--drag-x',Math.max(-90,Math.min(90,dx))+'px');
  if(Math.abs(dx)>34||Math.abs(dy)>34)tapeDragged=true;
});
['pointerup','pointercancel','lostpointercapture'].forEach(function(type){document.getElementById('tape-pile').addEventListener(type,function(event){
  if(!tapeDrag||event.pointerId!==tapeDrag.id)return;
  var dx=event.clientX-tapeDrag.x,dy=event.clientY-tapeDrag.y,moved=Math.abs(dx)>34||Math.abs(dy)>34;
  if(tapeDrag.horizontal&&Math.abs(dx)>34&&Math.abs(dx)>Math.abs(dy)*1.2){tapeDrag.scrolling=true;tapeDrag.pile.scrollLeft=tapeDrag.scroll-dx;}
  var scrolling=tapeDrag.scrolling,horizontal=tapeDrag.horizontal;
  tapeDrag.item.style.removeProperty('--drag-x');tapeDrag=null;
  if(moved){event.preventDefault();if(!horizontal&&!scrolling)shuffleTapePile();setTimeout(function(){tapeDragged=false;},80);}else setTimeout(function(){tapeDragged=false;},0);
});});
async function startTape(tape,item){
  if(tapeBusy)return;
  if(!blipMuted()){try{basicAudio();}catch(e){}}
  tapeBusy=true;document.body.setAttribute('data-tape-selecting','');
  document.querySelectorAll('.game-tape').forEach(function(control){control.setAttribute('aria-disabled','true');});
  try{
    if(editor)closeEditor();
    if(basicWasmPlaying||run)exitActiveProgram();
    submit('CD /TAPE');
    await new Promise(function(resolve){setTimeout(resolve,220);});
    var loaded=await submit('LOAD '+tape.command);
    if(loaded){
      await new Promise(function(resolve){setTimeout(resolve,1100);});
      submit('RUN');
    }
  }finally{
    tapeBusy=false;document.body.removeAttribute('data-tape-selecting');
    document.querySelectorAll('.game-tape').forEach(function(control){control.removeAttribute('aria-disabled');});
  }
}
renderTapePile();
function parseProgram(source){
  var program=new Map();
  source=String(source).toUpperCase();
  source.split(/\r?\n/).forEach(function(row){
    if(!row.trim())return;
    // One space ends the number; any more is the line's indentation.
    var match=/^(\d{1,5})(?:[ \t](.*))?$/.exec(row.trimEnd());
    if(match&&match[2]&&!match[2].trim())match[2]='';
    if(!match)throw Error('Bad program line: '+row);
    program.set(Number(match[1]),match[2]||'');
  });
  if(!program.size)throw Error('Program file is empty');
  return program;
}
function programSource(){
  return Array.from(lines).sort(function(a,b){return a[0]-b[0];}).map(function(row){return row[0]+' '+row[1].toUpperCase();}).join('\n');
}
function saveProgram(name){
  var path=name?(String(name).charAt(0)==='/'?programPath(name):programPath('/USER/'+name)):(currentFile&&currentFile.indexOf('/TAPE/')===0?'/USER/'+currentFile.slice(currentFile.lastIndexOf('/')+1).toUpperCase():(currentFile||'').toUpperCase());
  if(!path)throw Error('SAVE needs a name');
  if(path.indexOf('/USER/')!==0)throw Error('Only /USER is writable');
  var parent=path.slice(0,path.lastIndexOf('/'));
  if(!filesystem.dirs.some(function(dir){return dir.toLowerCase()===parent.toLowerCase();})&&parent!=='/USER')throw Error('Directory not found: '+parent);
  filesystem.files[path]=programSource();
  writeFilesystem();
  currentFile=path;
  print('Saved '+path);
}
function moveUserFile(from,to){
  if(!from||!to)throw Error('MOVE needs a source and destination');
  var source=canonicalRoot(resolvePath(from));
  if(source.indexOf('/USER/')!==0)throw Error('Only /USER files can be moved');
  if(!Object.prototype.hasOwnProperty.call(filesystem.files,source))throw Error('File not found: '+source);
  var destination=canonicalRoot(resolvePath(to));
  if(isDirectory(destination))destination+='/'+source.slice(source.lastIndexOf('/')+1);
  if(destination.indexOf('/USER/')!==0)throw Error('Only /USER files can be moved');
  var parent=destination.slice(0,destination.lastIndexOf('/'))||'/';
  if(!isDirectory(parent))throw Error('Directory not found: '+parent);
  if(source===destination)throw Error('Source and destination are the same');
  if(Object.prototype.hasOwnProperty.call(filesystem.files,destination)||isDirectory(destination))throw Error('Already exists: '+destination);
  filesystem.files[destination]=filesystem.files[source];
  delete filesystem.files[source];
  writeFilesystem();
  if(currentFile===source){currentFile=destination;storeDraft();}
  print('Moved '+source+' to '+destination);
}
function deleteUserFile(name){
  if(!name)throw Error('DEL needs a file');
  var path=canonicalRoot(resolvePath(name));
  if(!/\.[^/]+$/.test(path))path+='.BLIP';
  path=findFile(path);
  if(path.indexOf('/USER/')!==0)throw Error('Only /USER files can be deleted');
  if(!Object.prototype.hasOwnProperty.call(filesystem.files,path))throw Error('File not found: '+path);
  delete filesystem.files[path];
  writeFilesystem();
  if(currentFile&&currentFile.toUpperCase()===path.toUpperCase()){currentFile=null;storeDraft();}
  print('Deleted '+path);
}
async function readProgramSource(path){
  if(path.indexOf('/TAPE/')===0){
    await tapeReady;
    path=findFile(path);
    if(!isFileAvailable(path)||!tapeForPath(path))throw Error('TAPE NOT INSERTED');
    if(tapeFiles[path]===undefined){var tapeResponse=await fetch('TAPE/'+path.slice('/TAPE/'.length));if(!tapeResponse.ok)throw Error('TAPE NOT FOUND');tapeFiles[path]=await tapeResponse.text();}
    return tapeFiles[path];
  }
  return filesystem.files[path];
}
async function loadProgram(name){
  var path=programPath(name);
  if(path.indexOf('/TAPE/')===0)await Promise.all([tapeReady,gfxReady]);
  path=findFile(path);
  var tape=tapeForPath(path);
  if(tape&&!tapeNames.some(function(name){return ('/TAPE/'+name).toLowerCase()===path.toLowerCase();}))throw Error('TAPE NOT FOUND');
  if(tape&&tapeLoading)throw Error('Tape transport busy');
  if(tape)tapeLoading=true;
  var tapeLoad=tape?beginTapeLoad(tape):null;
  try{
    if(tapeLoad)await tapeLoad;
    var source=await readProgramSource(path);
    if(source===undefined)throw Error('Program not found: '+path);
    var program=parseProgram(source);
    lines.clear();program.forEach(function(value,key){lines.set(key,value);});
    vars=Object.create(null);run=null;pendingInput=null;currentFile=path;
    storeDraft();
    print('Loaded '+path);
    return true;
  }finally{if(tape){tapeLoading=false;document.body.removeAttribute('data-loading-tape');}}
}
function playTapeSeatClick(){
  if(blipMuted())return;
  try{
    var ctx=basicAudio(),out=typeof blipOut==='function'?blipOut(ctx):ctx.destination,now=ctx.currentTime;
    [[210,0.105,.16,'triangle'],[1180,0.028,.055,'square']].forEach(function(tone){
      var oscillator=ctx.createOscillator(),gain=ctx.createGain();
      oscillator.type=tone[3];oscillator.frequency.setValueAtTime(tone[0],now);oscillator.frequency.exponentialRampToValueAtTime(Math.max(45,tone[0]*.48),now+tone[1]);
      gain.gain.setValueAtTime(.0001,now);gain.gain.exponentialRampToValueAtTime(tone[2],now+.003);gain.gain.exponentialRampToValueAtTime(.0001,now+tone[1]);
      oscillator.connect(gain);gain.connect(out);oscillator.start(now);oscillator.stop(now+tone[1]+.005);
    });
  }catch(e){}
}
function beginTapeLoad(tape){
  document.body.removeAttribute('data-tape-loaded');
  if(!blipMuted()){try{basicAudio();}catch(e){}}
  var needsInsertion=insertedTape!==tape;
  var choreography=(needsInsertion?returnTapeToPile(true).then(function(){return insertTapeFromPile(tape);}):Promise.resolve());
  return choreography.then(function(){
    if(needsInsertion){document.body.setAttribute('data-tape-seated','');playTapeSeatClick();}
    return waitForMotion(needsInsertion?220:0).then(function(){
      document.body.removeAttribute('data-tape-seated');if(insertedTapeNode){insertedTapeNode.classList.remove('game-tape-inserting');insertedTapeNode.classList.add('deck-inserted-tape');insertedTapeNode.dataset.wound='';}document.body.setAttribute('data-loading-tape','');document.body.setAttribute('data-tape-name',tape.command);
      if(!blipMuted()){try{tapeLoadingAudio.volume=.9;tapeLoadingAudio.currentTime=0;var playback=tapeLoadingAudio.play();if(playback)playback.catch(function(){});}catch(e){}}
      return new Promise(function(resolve){setTimeout(function(){document.body.removeAttribute('data-loading-tape');document.body.setAttribute('data-tape-loaded','');resolve();},reducedTapeMotion()?0:7300);});
    });
  });
}
async function showSource(name){
  var source;
  if(name){
    var path=programPath(name);
    if(path.indexOf('/TAPE/')===0)await tapeReady;
    path=findFile(path);
    source=await readProgramSource(path);
    if(source===undefined)throw Error('Program not found: '+path);
    parseProgram(source);
  }else source=programSource();
  if(!source){print('No program');return;}
  source.split(/\r?\n/).forEach(print);
}
function editorEscapeHtml(value){return String(value).replace(/[&<>"']/g,function(ch){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[ch];});}
var editorKeywords=new Set(('PRINT INPUT LET IF THEN ELSE FOR TO STEP NEXT GOTO GOSUB RETURN SUB CALL EXIT LOCAL STATIC ELSEIF WHILE WEND DEF FN END STOP REM DATA READ RESTORE DIM AND OR NOT MOD ABS INT RND SQR SIN COS TAN ATN LOG EXP LEN STR$ VAL CHR$ ASC LEFT$ RIGHT$ MID$ CLEAR COLOR FILL_RECT DRAW_RECT DRAW_LINE DRAW_LINE_EX DRAW_POLYLINE DRAW_GLOW_LINE FX_CLEAR FX_UPDATE FX_DRAW FX_BURST FX_POPUP FX_RING CIRCLE DRAW_CIRCLE FILL_CIRCLE FILL_GLOW_CIRCLE FILL_CAPSULE FILL_SUNBURST DRAW_TEXT DRAW_CHAR DRAW_NUMBER DRAW_HUD DRAW_TEXTURE TEXT_CX KEY_ACTIVE KEY_HELD KEY_PRESSED LOAD SAVE DIR CD MKDIR DEL MOVE EJECT TUNE').split(' ').concat(basicMathNames));
function highlightEditorText(value){
  var pattern=/"[^\"]*"|\bREM\b.*$|\b[A-Z_][A-Z0-9_$]*\b|\b\d+(?:\.\d+)?\b/gi,out='',last=0,match;
  while((match=pattern.exec(value))){out+=editorEscapeHtml(value.slice(last,match.index));var token=match[0],upper=token.toUpperCase(),kind=token[0]==='"'?'editor-string':/^REM\b/i.test(token)?'editor-comment':/^\d/.test(token)?'editor-number':/^\b(?:BLIP_|NEON_)/i.test(token)?'editor-constant':editorKeywords.has(upper)?'editor-keyword':'';out+=kind?'<span class="'+kind+'">'+editorEscapeHtml(token)+'</span>':editorEscapeHtml(token);last=pattern.lastIndex;if(/^REM\b/i.test(token))break;}
  return out+editorEscapeHtml(value.slice(last));
}
var editorCompletions=('CALL DEF ELSE ELSEIF EXIT LOCAL RESTORE SUB WEND WHILE FX_CLEAR FX_UPDATE FX_DRAW FX_BURST FX_POPUP FX_RING DRAW_POLYLINE ABS AND CIRCLE DRAW_CIRCLE CD CLEAR COLOR COS DATA DEL DIR DRAW_CHAR DRAW_GLOW_LINE DRAW_HUD DRAW_LINE DRAW_LINE_EX DRAW_NUMBER DRAW_RECT DRAW_SUNBURST DRAW_TEXT DRAW_TEXTURE EJECT END EXP FILL_CAPSULE FILL_CIRCLE FILL_GLOW_CIRCLE FILL_RECT FILL_SUNBURST FOR GOSUB GOTO IF INPUT INT KEY_ACTIVE KEY_HELD KEY_PRESSED LEFT$ LEN LET LOAD LOG MID$ MKDIR MOD MOVE NEXT NOT OR PRINT READ RETURN RIGHT$ RND SAVE SIN SQR STEP STR$ TAN TEXT_CX THEN TO TUNE VAL').split(' ').concat(basicMathNames);
function editorCompletion(){
  var before=(editor.rows[editor.row]||'').slice(0,editor.column);
  if((before.match(/"/g)||[]).length%2)return null;
  var match=/([A-Z_][A-Z0-9_$]*)$/i.exec(before);if(!match||match[1].length<2)return null;
  var prefix=match[1].toUpperCase(),candidate=editorCompletions.find(function(word){return word.startsWith(prefix)&&word!==prefix;});
  return candidate?{prefix:match[1],suffix:candidate.slice(match[1].length),word:candidate}:null;
}
function renderEditor(message){
  if(!editor)return;
  editorPanel.hidden=false;document.body.setAttribute('data-editor','');
  if(editor.dirty){
    var source=editor.rows.join('\n').toUpperCase();
    filesystem.files[editor.path]=source;writeFilesystem();currentFile=editor.path;
    try{var parsed=source?parseProgram(source):new Map();lines.clear();parsed.forEach(function(value,line){lines.set(line,value);});storeDraft();}catch(e){}
    editor.dirty=false;editor.saved=true;
  }
  var suggestion=editorCompletion();
  editorFile.textContent=editor.path;
  editorMode.textContent='AUTO-SAVE';
  editorMessage.textContent=message||(suggestion?'TAB → '+suggestion.word:(editor.saved?'SAVED':'READY'));
  editorCode.innerHTML=editor.rows.map(function(row,index){
    var current=index===editor.row,content='',column=editor.column;
    if(current){
      content=highlightEditorText(row.slice(0,column))+'<span class="editor-caret">'+editorEscapeHtml(row.charAt(column)||' ')+'</span>'+highlightEditorText(row.slice(Math.min(row.length,column+1)));
      if(suggestion)content+='<span class="editor-completion">'+editorEscapeHtml(suggestion.suffix)+'</span>';
    }else content=highlightEditorText(row);
    return '<div class="editor-line'+(current?' current':'')+'" data-row="'+index+'"><span class="editor-gutter">'+String(index+1).padStart(3,' ')+'</span>'+content+'</div>';
  }).join('');
  var active=editorCode.children[editor.row];if(active){var top=active.offsetTop,bottom=top+active.offsetHeight;if(top<editorCode.scrollTop)editorCode.scrollTop=top;else if(bottom>editorCode.scrollTop+editorCode.clientHeight)editorCode.scrollTop=bottom-editorCode.clientHeight;}
  output.textContent=editor.rows.map(function(row,index){var marker=index===editor.row?'▶':' ';var value=index===editor.row?row.slice(0,editor.column)+'█'+row.slice(Math.min(row.length,editor.column+1)):row;return marker+String(index+1).padStart(3,' ')+' '+value;}).join('\n')+'\nAUTO-SAVE'+(message?' '+message:'');
  var caret=editorCode.querySelector('.editor-caret');if(caret){var box=caret.getBoundingClientRect(),view=editorCode.getBoundingClientRect();if(box.right>view.right)editorCode.scrollLeft+=box.right-view.right+8;else if(box.left<view.left)editorCode.scrollLeft-=view.left-box.left+8;}
}
function openEditor(name){
  var path=programPath(name),source=filesystem.files[path];
  if(source===undefined&&path.indexOf('/USER/')!==0)throw Error('Only /USER is writable');
  if(source===undefined){var parent=path.slice(0,path.lastIndexOf('/'));if(!isDirectory(parent))throw Error('Directory not found: '+parent);source='';}
  editor={path:path,rows:source?source.toUpperCase().split(/\r?\n/):[''],row:0,column:0,dirty:false,saved:false};
  input.value='';clear();renderEditor('');input.focus({preventScroll:true});
}
function editorInsertKey(key){
  if(key.length===1)key=key.toUpperCase();
  var row=editor.rows[editor.row]||'';
  if(key==='Escape'){closeEditor();return;}
  if(key==='Enter'){
    editor.rows[editor.row]=row.slice(0,editor.column);
    editor.rows.splice(editor.row+1,0,row.slice(editor.column));editor.row++;editor.column=0;editor.dirty=true;renderEditor();return;
  }
  if(key==='Backspace'){
    if(editor.column){editor.rows[editor.row]=row.slice(0,editor.column-1)+row.slice(editor.column);editor.column--;editor.dirty=true;}
    else if(editor.row){var previous=editor.rows[editor.row-1];editor.column=previous.length;editor.rows[editor.row-1]+=row;editor.rows.splice(editor.row,1);editor.row--;editor.dirty=true;}
    renderEditor();return;
  }
  if(key==='Delete'){
    if(editor.column<row.length){editor.rows[editor.row]=row.slice(0,editor.column)+row.slice(editor.column+1);editor.dirty=true;}
    else if(editor.row<editor.rows.length-1){editor.rows[editor.row]+=editor.rows[editor.row+1];editor.rows.splice(editor.row+1,1);editor.dirty=true;}
    renderEditor();return;
  }
  if(key==='ArrowLeft'){if(editor.column)editor.column--;else if(editor.row){editor.row--;editor.column=editor.rows[editor.row].length;}renderEditor();return;}
  if(key==='ArrowRight'){if(editor.column<row.length)editor.column++;else if(editor.row<editor.rows.length-1){editor.row++;editor.column=0;}renderEditor();return;}
  if(key==='ArrowUp'||key==='ArrowDown'){
    editor.row=Math.max(0,Math.min(editor.rows.length-1,editor.row+(key==='ArrowDown'?1:-1)));
    editor.column=Math.min(editor.column,editor.rows[editor.row].length);renderEditor();return;
  }
  if(key==='Home'){editor.column=0;renderEditor();return;}
  if(key==='End'){editor.column=row.length;renderEditor();return;}
  if(key==='Tab'){
    var completion=editorCompletion();
    if(completion){editor.rows[editor.row]=row.slice(0,editor.column)+completion.suffix+row.slice(editor.column);editor.column+=completion.suffix.length;editor.dirty=true;editor.saved=false;renderEditor('COMPLETED '+completion.word);return;}
    key='    ';
  }
  if(key.length===1||key==='    '){editor.rows[editor.row]=row.slice(0,editor.column)+key+row.slice(editor.column);editor.column+=key.length;editor.dirty=true;editor.saved=false;renderEditor();}
}
function closeEditor(){editor=null;editorPanel.hidden=true;document.body.removeAttribute('data-editor');clear();print('BLIP 64K COLOUR PERSONAL COMPUTER');print('BLIP BASIC 1.0');print('EDITOR CLOSED');ready();input.focus({preventScroll:true});}
function basicAudio(){
  var Audio=window.AudioContext||window.webkitAudioContext;
  if(!Audio)throw Error('Web Audio is not available');
  if(!soundContext)soundContext=new Audio();
  if(soundContext.state==='suspended')soundContext.resume().catch(function(){});
  return soundContext;
}
function sound(frequency,duration,volume){
  frequency=Number(frequency);duration=Number(duration);volume=volume===undefined?.18:Number(volume);
  if(!Number.isFinite(frequency)||frequency<0||frequency>12000)throw Error('SOUND frequency must be 0–12000');
  if(!Number.isFinite(duration)||duration<=0||duration>10)throw Error('SOUND duration must be over 0 and at most 10 seconds');
  if(!Number.isFinite(volume)||volume<0||volume>1)throw Error('SOUND volume must be 0–1');
  var ctx=basicAudio(),start=Math.max(ctx.currentTime,soundCursor),end=start+duration;
  soundCursor=end;
  if(frequency===0)return;
  var oscillator=ctx.createOscillator(),gain=ctx.createGain();
  oscillator.type='triangle';oscillator.frequency.setValueAtTime(frequency,start);
  gain.gain.setValueAtTime(0,start);gain.gain.linearRampToValueAtTime(volume,start+.012);
  gain.gain.setValueAtTime(volume,Math.max(start+.012,end-.035));gain.gain.linearRampToValueAtTime(0,end);
  oscillator.connect(gain);gain.connect(typeof blipOut==='function'?blipOut(ctx):ctx.destination);
  oscillator.start(start);oscillator.stop(end+.01);
}
function rest(duration){sound(0,duration,0);}
function directoryEntries(path){
  var dirs=[],files=[];
  if(path==='/')dirs=['/ROM/','/TAPE/','/USER/'];
  if(path==='/ROM')dirs.push('TUNES/');
  if(path==='/ROM/TUNES')files=files.concat(romTunes);
  if(path==='/TAPE'&&insertedTape){dirs.push('GFX/');files.push(insertedTape.file);}
  if(path==='/TAPE/GFX'&&insertedTape)dirs.push(insertedTape.slug.toUpperCase()+'/');
  var gfxGame=path.match(/^\/TAPE\/GFX\/([A-Z0-9_]+)$/);
  if(gfxGame&&insertedTape&&gfxGame[1]===insertedTape.slug.toUpperCase())files=files.concat(gfxNames[gfxGame[1]]||[]);
  filesystem.dirs.forEach(function(dir){
    if(dir==='/USER')return;
    var parent=dir.slice(0,dir.lastIndexOf('/'))||'/';
    if(parent.toLowerCase()===path.toLowerCase()&&dir!==path)dirs.push(dir.slice(dir.lastIndexOf('/')+1)+'/');
  });
  Object.keys(filesystem.files).forEach(function(file){
    var parent=file.slice(0,file.lastIndexOf('/'))||'/';
    if(parent.toLowerCase()===path.toLowerCase())files.push(file.slice(file.lastIndexOf('/')+1));
  });
  return dirs.concat(files).sort(function(a,b){return a.localeCompare(b);});
}
async function showDirectory(path){
  await tapeReady;
  if(path.toUpperCase().indexOf('/TAPE/GFX')===0)await gfxReady;
  if(path.toUpperCase().indexOf('/ROM/TUNES')===0)await romTunesReady;
  if(tapeLoadError)throw Error(tapeLoadError);
  path=canonicalDirectory(path||currentDir);
  if(!isDirectory(path))throw Error('Directory not found: '+path);
  print('Directory of '+path);
  var entries=directoryEntries(path);
  if(!entries.length)print('<empty>');
  else entries.forEach(print);
}
function isDirectory(path){
  return mountedDirectories().some(function(dir){return dir.toLowerCase()===path.toLowerCase();});
}
function makeDirectory(name){
  var path=canonicalRoot(resolvePath(name));
  if(path.indexOf('/USER/')!==0)throw Error('Only /USER is writable');
  if(isDirectory(path)||findFile(path)!==path)throw Error('Already exists: '+path);
  var parent=path.slice(0,path.lastIndexOf('/'))||'/';
  if(!isDirectory(parent))throw Error('Directory not found: '+parent);
  filesystem.dirs.push(path);writeFilesystem();print('Created '+path);
}
function pathArgument(value){
  value=value.trim();
  if(value.length>=2&&value.charAt(0)==='"'&&value.charAt(value.length-1)==='"')return value.slice(1,-1).replace(/""/g,'"');
  return value;
}
function storeDraft() {
  try {
    localStorage.setItem(DRAFT, JSON.stringify(Array.from(lines)));
    if(currentFile)localStorage.setItem(CURRENT_FILE_STORE,currentFile);
    else localStorage.removeItem(CURRENT_FILE_STORE);
  } catch (e) {}
}
function restoreDraft() {
  try {
    currentFile=(localStorage.getItem(CURRENT_FILE_STORE)||'').toUpperCase()||null;
    var draft = JSON.parse(localStorage.getItem(DRAFT) || '[]');
    if (Array.isArray(draft)) draft.forEach(function (entry) {
      if (Array.isArray(entry) && Number.isInteger(entry[0]) && typeof entry[1] === 'string') lines.set(entry[0], entry[1].toUpperCase());
    });
  } catch (e) {}
}

function print(s) {
  output.textContent += String(s).toUpperCase() + '\n';
  var all = output.textContent.split('\n');
  if (all.length > 180) output.textContent = all.slice(-180).join('\n');
  output.scrollTop = output.scrollHeight;
}
function clear() { output.textContent = ''; gfx.fillStyle = palette[paper]; gfx.fillRect(0, 0, 320, 200); }
function blipKeyCode(name){var key=String(name||'').toUpperCase();return ({LEFT:'ArrowLeft',RIGHT:'ArrowRight',UP:'ArrowUp',DOWN:'ArrowDown',SPACE:' ',FIRE:' ',BUTTON2:'z',ESC:'Escape',ENTER:'Enter'})[key]||(/^[A-Z]$/.test(key)?key.toLowerCase():key);}
function ready() { if (!run) print('Ready'); }
function fail(message) { run = null; pendingInput = null; document.body.removeAttribute('data-basic-running'); print('? ' + message); ready(); }
function stopProgram(){run=null;pendingInput=null;input.placeholder='';document.body.removeAttribute('data-basic-running');heldKeys=Object.create(null);pressedKeys=Object.create(null);clear();ready();input.focus({preventScroll:true});}
function splitStatements(s) {
  var out = [], part = '', quoted = false;
  for (var i=0;i<s.length;i++) {
    if (!part.trim() && /^(?:REM(?=\s|:|$)|')/i.test(s.slice(i).trimStart())) { part += s.slice(i); break; }
    if (s[i] === '"') quoted = !quoted;
    if (s[i] === ':' && !quoted && !/^IF\s+.+\s+THEN\s+/i.test(part.trim())) { out.push(part.trim()); part=''; } else part += s[i];
  }
  out.push(part.trim()); return out.filter(Boolean);
}
function splitArgs(s) {
  var out=[], part='', quoted=false, depth=0;
  for (var i=0;i<s.length;i++) {
    if (s[i] === '"') quoted = !quoted;
    if (!quoted && s[i] === '(') depth++;
    if (!quoted && s[i] === ')') depth--;
    if (!quoted && depth === 0 && (s[i] === ',' || s[i] === ';')) { out.push(part.trim()); part=''; } else part += s[i];
  }
  out.push(part.trim()); return out;
}
const consoleHost = {
  read(name) {
    if (name === 'INKEY$') {
      const key = lastKey;
      lastKey = '';
      return key;
    }
    return Object.hasOwn(vars, name) ? vars[name] : name.endsWith('$') ? '' : 0;
  },

  call(name, args) {
    if (basicMathNames.includes(name)) {
      const shared = wasmFrame.contentWindow?.wasm_exports?.['blip_basic_math_' + name.toLowerCase()];
      if (typeof shared !== 'function') throw Error('Shared functions are not loaded');
      if (args.length !== shared.length) throw Error(name + ' needs ' + shared.length + ' arguments');
      if (args.some((value) => typeof value !== 'number')) throw Error(name + ' needs numeric arguments');
      return shared(...args);
    }
    if (name === 'KEY_HELD' || name === 'KEY_ACTIVE' || name === 'KEY_PRESSED') {
      const key = blipKeyCode(args[0]);
      const held = !!heldKeys[key], pressed = !!pressedKeys[key];
      return Number(name === 'KEY_HELD' ? held : name === 'KEY_PRESSED' ? pressed : held || pressed);
    }
    if (Object.hasOwn(arrays, name)) {
      const key = args.map((index) => Math.floor(Number(index))).join(',');
      return arrays[name][key] || 0;
    }
    throw Error('Unknown function ' + name);
  },
};

function evalExpr(source) {
  return BlipBasicLanguage.evaluate(source, consoleHost);
}
function jump(number) {
  if(!run)throw Error('Not running');
  var i=run.numbers.indexOf(Number(number)); if(i<0)throw Error('Line '+number+' not found');
  run.index=i;run.statement=0;
}
var pendingGameCopy=null;
function ensureUserDirectory(path){
  var parts=path.split('/').filter(Boolean),current='';
  parts.forEach(function(part){current+='/'+part;if(current!=='/USER'&&!filesystem.dirs.some(function(dir){return dir.toUpperCase()===current;}))filesystem.dirs.push(current);});
}
function pngDataUrl(buffer){
  var bytes=new Uint8Array(buffer),binary='';
  for(var start=0;start<bytes.length;start+=0x8000)binary+=String.fromCharCode.apply(null,bytes.subarray(start,Math.min(start+0x8000,bytes.length)));
  return 'data:image/png;base64,'+btoa(binary);
}
function finishGameCopy(){
  var pending=pendingGameCopy;if(!pending)return;
  try{
    Object.keys(pending.assets).forEach(function(path){var name=path.slice(path.lastIndexOf('/')+1),alias=name.replace(/\.PNG$/,'');pending.source=pending.source.replaceAll('"'+alias+'"','"'+path+'"');});
    Object.keys(pending.assets).forEach(function(path){filesystem.files[path]=pending.assets[path];});
    filesystem.files[pending.destination]=pending.source;
    writeFilesystem();currentFile=pending.destination;storeDraft();
    pendingGameCopy=null;stopBouncerWasm();openEditor(currentFile);
  }catch(error){pendingGameCopy=null;stopBouncerWasm();print('? GAME COPY FAILED: '+error.message);}
}
function escapeFromBasicGame(){
  if(!basicWasmPlaying||pendingGameCopy)return;
  var tape=tapeForPath(currentFile),destination=currentFile;
  if(tape){destination='/USER/'+tape.command+'/'+tape.file;ensureUserDirectory('/USER/'+tape.command+'/GFX');}
  if(!destination||destination.indexOf('/USER/')!==0){stopBouncerWasm();return;}
  var source=filesystem.files[destination];
  if(source===undefined){
    source=programSource();
    if(tape)source=source.replaceAll('/TAPE/GFX/'+tape.slug.toUpperCase()+'/',destination.slice(0,destination.lastIndexOf('/'))+'/GFX/');
  }
  pendingGameCopy={destination:destination,source:source,assets:{}};
  if(tape&&filesystem.files[destination]===undefined&&basicWasmReady){
    var gameId={adder:1,bouncer:2,brawler:3,bubbler:4,galactic_defender:5,meteors:6,rally:7,serpent:8,sky_raider:9}[tape.slug]||0;
    wasmFrame.contentWindow.postMessage({type:'blip-basic-export-rom-assets',game:gameId},location.origin);
    return;
  }
  finishGameCopy();
}
function exitActiveProgram(){if(basicWasmPlaying)stopBouncerWasm();else if(run)stopProgram();}
function postBasicWasmSource(){if(basicWasmReady&&wasmFrame.contentWindow)wasmFrame.contentWindow.postMessage({type:'blip-basic-source',source:basicWasmSource,textures:typeof paintTexturePayload==='function'?paintTexturePayload():[],music:typeof blipBasicMusicPayload==='function'?blipBasicMusicPayload():[]},location.origin);}
function focusBasicWasmCanvas(){try{wasmFrame.contentWindow.focus();wasmFrame.contentDocument.getElementById('glcanvas').focus();}catch(e){}}
function runBlipInWasm(){
  configureBasicControls();
  basicWasmRunId++;
  basicWasmSource=Array.from(lines.keys()).sort(function(a,b){return a-b;}).map(function(n){return n+' '+lines.get(n);}).join('\n')+'\nREM RUN '+basicWasmRunId;
  basicWasmPlaying=true;document.body.setAttribute('data-wasm-playing','');updatePlayClearance();document.body.removeAttribute('data-wasm-error');document.body.removeAttribute('data-wasm-running');output.textContent='';clear();wasmFrame.hidden=false;
  if(!wasmFrame.hasAttribute('src')){wasmFrame.addEventListener('load',function loaded(){wasmFrame.removeEventListener('load',loaded);basicWasmReady=true;postBasicWasmSource();focusBasicWasmCanvas();},{once:true});wasmFrame.src='basic-runtime.html?v=18';}
  else {postBasicWasmSource();focusBasicWasmCanvas();}
}
var blipPrimitiveNames='canvas clear fill_rect draw_rect draw_line draw_line_ex draw_polyline draw_glow_line fx_clear fx_update fx_draw fx_burst fx_popup fx_ring fill_circle draw_circle fill_capsule fill_sunburst fill_glow_circle draw_texture draw_texture_region draw_texture_cell draw_texture_tinted draw_char draw_text draw_text_outlined draw_centered_outlined draw_number draw_centered draw_hud'.split(' ');
function programUsesBlipPrimitives(){
  var call=new RegExp('\\b(?:'+blipPrimitiveNames.join('|')+')\\s*\\(','i');
  return Array.from(lines.values()).some(function(source){return /\b(?:PLAY_SFX|PLAY_MUSIC|STOP_MUSIC|TEXT_CX)\b/i.test(source)||/\b(?:SUB|CALL|ELSE|ELSEIF|END\s+IF|WHILE|WEND|EXIT\s+FOR|DEF\s+FN\w*)\b|\bTHEN\s*$/i.test(source.replace(/"(?:[^"]|"")*"/g,'""'))||splitStatements(source).some(function(part){return call.test(part.trim());});});
}
function stopBouncerWasm(){if(!basicWasmPlaying)return;releaseBasicRallyDials();showBasicSecondPlayer(false);basicWasmSource='';postBasicWasmSource();basicWasmPlaying=false;document.body.removeAttribute('data-wasm-playing');document.body.removeAttribute('data-wasm-running');document.body.removeAttribute('data-rally');updatePlayClearance();wasmFrame.hidden=true;clear();output.textContent='';print('BLIP 64K COLOUR PERSONAL COMPUTER');print('BLIP BASIC 1.0');ready();input.focus({preventScroll:true});}
function sendBasicWasmKey(key,code,down){var message={type:'blip-basic-key',key:key,code:code,down:!!down};if(basicWasmReady&&wasmFrame.contentWindow)wasmFrame.contentWindow.postMessage(message,location.origin);}
window.addEventListener('message', function (event) {
  if(event.origin!==location.origin||!event.data)return;
  if(event.data.type==='blip-basic-wasm-ready'){basicWasmReady=true;postBasicWasmSource();if(basicWasmPlaying)focusBasicWasmCanvas();}
  if(event.data.type==='blip-basic-program-loaded'&&basicWasmPlaying){document.body.setAttribute('data-wasm-running','');focusBasicWasmCanvas();}
  if(event.data.type==='blip-basic-runtime-error')document.body.setAttribute('data-wasm-error',event.data.message||'error');
  if(event.data.type==='blip-basic-rom-asset'&&pendingGameCopy){var sourcePath=String(event.data.path||'').toUpperCase(),game=tapeForPath(currentFile),destinationPath=game?sourcePath.replace(/^\/TAPE\/GFX\/[^/]+\//,'/USER/'+game.command+'/GFX/'):'';if(destinationPath.indexOf('/USER/')===0)pendingGameCopy.assets[destinationPath]=pngDataUrl(event.data.bytes);}
  if(event.data.type==='blip-basic-rom-assets-done')finishGameCopy();
  if(event.data.type==='blip-basic-escape')escapeFromBasicGame();
  if(event.data.type==='blip-basic-stop')stopBouncerWasm();
});
exitGameButton.addEventListener('click', exitActiveProgram);
function statement(s) {
  var m, command=s.trim();
  var upper=command.toUpperCase();
  if(!command || /^REM(?:\s|$)/i.test(command) || command[0]==="'")return;
  if((m=/^(?:LET\s+)?([A-Z][A-Z0-9_]*\$?)\s*(?:\((.*)\))?\s*=\s*(.+)$/i.exec(command))){
    var value=evalExpr(m[3]),variable=m[1].toUpperCase();
    if(m[2]!==undefined){if(!arrays[variable])arrays[variable]=Object.create(null);arrays[variable][splitArgs(m[2]).map(function(index){return Math.floor(Number(evalExpr(index)));}).join(',')]=value;}
    else vars[variable]=value;
    return;
  }
  if((m=/^PRINT(?:\s+(.*))?$/i.exec(command))){print(m[1]?splitArgs(m[1]).map(function(x){return x?evalExpr(x):'';}).join(' '):'');return;}
  if((m=/^IF\s+(.+?)\s+THEN\s+(.+)$/i.exec(command))){if(evalExpr(m[1])){if(/^\d+$/.test(m[2].trim()))jump(m[2].trim());else splitStatements(m[2]).forEach(statement);}return;}
  if((m=/^GOTO\s+(.+)$/i.exec(command))){jump(evalExpr(m[1]));return;}
  if((m=/^GOSUB\s+(.+)$/i.exec(command))){if(!run)throw Error('Not running');run.calls.push([run.index,run.statement]);jump(evalExpr(m[1]));return;}
  if(upper==='RETURN'){if(!run||!run.calls.length)throw Error('RETURN without GOSUB');var addr=run.calls.pop();run.index=addr[0];run.statement=addr[1];return;}
  if((m=/^FOR\s+([A-Z][A-Z0-9_]*)\s*=\s*(.+?)\s+TO\s+(.+?)(?:\s+STEP\s+(.+))?$/i.exec(command))){
    if(!run)throw Error('FOR requires RUN');var name=m[1].toUpperCase();vars[name]=Number(evalExpr(m[2]));
    run.fors.push({name:name,end:Number(evalExpr(m[3])),step:m[4]?Number(evalExpr(m[4])):1,index:run.index,statement:run.statement});return;
  }
  if((m=/^NEXT(?:\s+([A-Z][A-Z0-9_]*))?$/i.exec(command))){
    if(!run||!run.fors.length)throw Error('NEXT without FOR');var f=run.fors[run.fors.length-1];
    if(m[1]&&m[1].toUpperCase()!==f.name)throw Error('NEXT variable mismatch');
    vars[f.name]+=f.step;if(f.step!==0&&(f.step>0?vars[f.name]<=f.end:vars[f.name]>=f.end)){run.index=f.index;run.statement=f.statement;}else run.fors.pop();return;
  }
  if(upper==='END'||upper==='STOP'){run=null;document.body.removeAttribute('data-basic-running');ready();return;}
  if(upper==='CLS'){clear();return;}
  if(upper==='CLEAR'){vars=Object.create(null);arrays=Object.create(null);pendingInput=null;clear();return;}
  if((m=/^SOUND\s+(.+)$/i.exec(command))){var tone=splitArgs(m[1]);if(tone.length<2||tone.length>3)throw Error('SOUND needs frequency,duration[,volume]');sound(evalExpr(tone[0]),evalExpr(tone[1]),tone.length===3?evalExpr(tone[2]):undefined);return;}
  if(/^PLAY_SFX\s+/i.test(command))throw Error('PLAY_SFX is provided by the Rust Blip runtime; enter it in a numbered program and RUN');
  if((m=/^REST\s+(.+)$/i.exec(command))){rest(evalExpr(m[1]));return;}
  if(new RegExp('\\b(?:'+blipPrimitiveNames.join('|')+')\\s*\\(','i').test(command))throw Error('BLIP drawing calls run through the Rust runtime; enter numbered lines and RUN');
  if((m=/^CIRCLE\s+(.+)$/i.exec(command))){
    var c=splitArgs(m[1]);if(c.length!==3)throw Error('CIRCLE needs x,y,r');
    var cx=Number(evalExpr(c[0])),cy=Number(evalExpr(c[1])),cr=Number(evalExpr(c[2]));
    if(cr<0)throw Error('Radius must be positive');
    gfx.strokeStyle=palette[pen];gfx.beginPath();gfx.arc(cx,199-cy,cr,0,Math.PI*2);gfx.stroke();return;
  }
  if((m=/^ELLIPSE\s+(.+)$/i.exec(command))){
    var e=splitArgs(m[1]);if(e.length!==4)throw Error('ELLIPSE needs x,y,rx,ry');
    var ex=Number(evalExpr(e[0])),ey=Number(evalExpr(e[1])),erx=Number(evalExpr(e[2])),ery=Number(evalExpr(e[3]));
    if(erx<0||ery<0)throw Error('Radius must be positive');
    gfx.strokeStyle=palette[pen];gfx.beginPath();gfx.ellipse(ex,199-ey,erx,ery,0,0,Math.PI*2);gfx.stroke();return;
  }
  if((m=/^FILLELLIPSE\s+(.+)$/i.exec(command))){
    var fe=splitArgs(m[1]);if(fe.length!==4)throw Error('FILLELLIPSE needs x,y,rx,ry');
    var fex=Number(evalExpr(fe[0])),fey=Number(evalExpr(fe[1])),ferx=Number(evalExpr(fe[2])),fery=Number(evalExpr(fe[3]));
    if(ferx<0||fery<0)throw Error('Radius must be positive');
    gfx.fillStyle=palette[pen];gfx.beginPath();gfx.ellipse(fex,199-fey,ferx,fery,0,0,Math.PI*2);gfx.fill();return;
  }
  if((m=/^FILLPOLY\s+(.+)$/i.exec(command))){
    var fp=splitArgs(m[1]);if(fp.length<6||fp.length%2)throw Error('FILLPOLY needs x,y pairs');
    gfx.fillStyle=palette[pen];gfx.beginPath();gfx.moveTo(Number(evalExpr(fp[0])),199-Number(evalExpr(fp[1])));
    for(var fi=2;fi<fp.length;fi+=2)gfx.lineTo(Number(evalExpr(fp[fi])),199-Number(evalExpr(fp[fi+1])));
    gfx.closePath();gfx.fill();return;
  }
  if((m=/^SPLINE\s+(.+)$/i.exec(command))){
    var sp=splitArgs(m[1]);if(sp.length!==8)throw Error('SPLINE needs x,y and two control points and an end point');
    var sv=sp.map(function(value){return Number(evalExpr(value));});
    gfx.strokeStyle=palette[pen];gfx.beginPath();gfx.moveTo(sv[0],199-sv[1]);gfx.bezierCurveTo(sv[2],199-sv[3],sv[4],199-sv[5],sv[6],199-sv[7]);gfx.stroke();return;
  }
  if((m=/^TEXT\s+(.+)$/i.exec(command))){
    var tx=splitArgs(m[1]);if(tx.length!==3)throw Error('TEXT needs x,y,string');
    gfx.fillStyle=palette[pen];gfx.font='10px "Courier New",monospace';gfx.textBaseline='top';gfx.fillText(String(evalExpr(tx[2])).toUpperCase(),Number(evalExpr(tx[0])),199-Number(evalExpr(tx[1])));return;
  }
  if((m=/^FILLSPLINE\s+(.+)$/i.exec(command))){
    var fs=splitArgs(m[1]);if(fs.length<8||(fs.length-2)%6)throw Error('FILLSPLINE needs a start point and cubic curve groups');
    var fv=fs.map(function(value){return Number(evalExpr(value));});
    gfx.fillStyle=palette[pen];gfx.beginPath();gfx.moveTo(fv[0],199-fv[1]);
    for(var si=2;si<fv.length;si+=6)gfx.bezierCurveTo(fv[si],199-fv[si+1],fv[si+2],199-fv[si+3],fv[si+4],199-fv[si+5]);
    gfx.closePath();gfx.fill();return;
  }
  if((m=/^ARC\s+(.+)$/i.exec(command))){
    var a=splitArgs(m[1]);if(a.length!==5)throw Error('ARC needs x,y,r,start,end');
    var ax=Number(evalExpr(a[0])),ay=Number(evalExpr(a[1])),ar=Number(evalExpr(a[2]));
    if(ar<0)throw Error('Radius must be positive');
    var start=-Number(evalExpr(a[3]))*Math.PI/180,end=-Number(evalExpr(a[4]))*Math.PI/180;
    gfx.strokeStyle=palette[pen];gfx.beginPath();gfx.arc(ax,199-ay,ar,start,end,false);gfx.stroke();return;
  }
  if((m=/^(?:PEN|INK)\s+(.+)$/i.exec(command))){pen=((Number(evalExpr(m[1]))%palette.length)+palette.length)%palette.length;return;}
  if((m=/^PAPER\s+(.+)$/i.exec(command))){paper=((Number(evalExpr(m[1]))%palette.length)+palette.length)%palette.length;clear();return;}
  if((m=/^PLOT\s+(.+)$/i.exec(command))){var a=splitArgs(m[1]);gx=Number(evalExpr(a[0]));gy=Number(evalExpr(a[1]));gfx.fillStyle=palette[pen];gfx.fillRect(Math.round(gx),199-Math.round(gy),1,1);return;}
  if((m=/^DRAW\s+(.+)$/i.exec(command))){var p=splitArgs(m[1]),x=Number(evalExpr(p[0])),y=Number(evalExpr(p[1]));gfx.strokeStyle=palette[pen];gfx.beginPath();gfx.moveTo(gx+.5,199-gy+.5);gfx.lineTo(x+.5,199-y+.5);gfx.stroke();gx=x;gy=y;return;}
  if((m=/^INPUT\s+(?:"([^"]*)"\s*[,;]\s*)?([A-Z][A-Z0-9_]*\$?)$/i.exec(command))){pendingInput=m[2].toUpperCase();print(m[1]||'?');input.placeholder=pendingInput+'?';input.focus();return;}
  if((m=/^WAIT\s+(.+)$/i.exec(command))){if(!run)throw Error('WAIT requires RUN');run.waitUntil=performance.now()+Math.max(0,Math.min(10000,Number(evalExpr(m[1]))));return;}
  throw Error('Unknown command: '+command.split(/\s/)[0]);
}
function tick() {
  var gameTick=run&&!pendingInput&&performance.now()>=run.waitUntil;
  if(gameTick){
    try {
      for(var n=0;n<1000&&run&&!pendingInput&&performance.now()>=run.waitUntil;n++){
        if(run.index>=run.numbers.length){run=null;document.body.removeAttribute('data-basic-running');ready();break;}
        var pieces=splitStatements(lines.get(run.numbers[run.index]));
        if(run.statement>=pieces.length){run.index++;run.statement=0;continue;}
        var code=pieces[run.statement++];statement(code);
      }
    }catch(e){fail(e.message);}
  }
  if(gameTick)pressedKeys=Object.create(null);
  requestAnimationFrame(tick);
}
function submit(raw) {
  if(editor){Array.from(raw).forEach(editorInsertKey);return;}
  var s=raw.trim().toUpperCase();if(!s)return;
  var numbered=/^(\d{1,5})(?:[ \t](.*))?$/.exec(s);
  if(numbered&&numbered[2]&&!numbered[2].trim())numbered[2]='';
  if (!numbered && !pendingInput && (basicMathLoading || needsSharedMath(s))) {
    return queueSharedMathCommand(s);
  }
  history.push(s);historyAt=history.length;
  print('> '+s);
  if(pendingInput){vars[pendingInput]=pendingInput.endsWith('$')?s:Number(s)||0;pendingInput=null;input.placeholder='';return;}
  if(numbered){var number=Number(numbered[1]);if(numbered[2])lines.set(number,numbered[2]);else lines.delete(number);storeDraft();return;}
  try {
    var m;
    if((m=/^PAINT(?:\s+(.+))?$/i.exec(s))){if(typeof startPaint!=='function')throw Error('PAINT is unavailable');if(run||basicWasmPlaying)exitActiveProgram();startPaint(m[1]?pathArgument(m[1]):null);return;}
    if((m=/^TUNE(?:\s+(.+))?$/i.exec(s))){if(m[1])throw Error('TUNE TAKES NO FILE; USE IMPORT XM');if(typeof startTune!=='function')throw Error('TUNE is unavailable');if(run||basicWasmPlaying)exitActiveProgram();startTune();return;}
    if(s.toUpperCase()==='RUN'){
      if(!lines.size)throw Error('No program');
      if(programUsesBlipPrimitives()){runBlipInWasm();return;}
      vars=Object.create(null);arrays=Object.create(null);run={numbers:Array.from(lines.keys()).sort(function(a,b){return a-b;}),index:0,statement:0,calls:[],fors:[],waitUntil:0};document.body.setAttribute('data-basic-running','');clear();input.blur();return;
    }
    if(s.toUpperCase()==='LIST'){Array.from(lines.keys()).sort(function(a,b){return a-b;}).forEach(function(k){print(k+' '+lines.get(k));});ready();return;}
    if((m=/^SOURCE(?:\s+(.+))?$/i.exec(s))){showSource(m[1]?pathArgument(m[1]):null).then(ready).catch(function(error){fail(error.message);});return;}
    if((m=/^EDIT\s+(.+)$/i.exec(s))){openEditor(pathArgument(m[1]));return;}
    if(s.toUpperCase()==='NEW'){run=null;document.body.removeAttribute('data-basic-running');lines.clear();vars=Object.create(null);arrays=Object.create(null);currentFile=null;storeDraft();clear();ready();return;}
    if(s.toUpperCase()==='HELP'){
      clear();
      print('BLIP BASIC · HELP');
      print('FILESYSTEM');
      print('LOAD FILE     LOAD A .BLIP PROGRAM FROM /ROM, /TAPE OR /USER');
      print('SAVE [NAME]   SAVE THE PROGRAM IN /USER');
      print('DIR [PATH]    LIST FILES');
      print('CD [PATH]     CHANGE FOLDER');
      print('MKDIR NAME    CREATE A FOLDER IN /USER');
      print('DEL FILE      DELETE A FILE IN /USER');
      print('MOVE FROM TO  MOVE A FILE IN /USER');
      print('EDIT FILE     EDIT A /USER FILE; CHANGES AUTO-SAVE; ESC CLOSES');
      print('PAINT [FILE]  OPEN THE PIXEL ART EDITOR');
      print('TUNE          OPEN THE MUSIC EDITOR');
      print('TAB           ACCEPT THE DIMMED EDITOR SUGGESTION');
      print('NAMES ARE UPPERCASE');
      print('/ROM IS READ-ONLY SYSTEM EXAMPLES');
      print('/TAPE IS READ-ONLY; INSERTED CASSETTE ONLY');
      print('EJECT         REMOVE THE CASSETTE');
      print('/USER PERSISTS IN THIS BROWSER');
      ready();return;
    }
    if(s==='EJECT')return ejectTape().then(ready).catch(function(error){fail(error.message);});
    if(s.toUpperCase()==='STOP'){run=null;pendingInput=null;document.body.removeAttribute('data-basic-running');ready();return;}
    if((m=/^DIR(?:\s+(.+))?$/i.exec(s))){showDirectory(m[1]?resolvePath(pathArgument(m[1])):currentDir).then(ready).catch(function(error){fail(error.message);});return;}
    if((m=/^CD(?:\s+(.+))?$/i.exec(s))){var dest=canonicalDirectory(m[1]?resolvePath(pathArgument(m[1])):'/USER');if(!isDirectory(dest))throw Error('Directory not found: '+dest);currentDir=dest;print(currentDir);ready();return;}
    if((m=/^MKDIR\s+(.+)$/i.exec(s))){makeDirectory(pathArgument(m[1]));ready();return;}
    if((m=/^SAVE(?:\s+(.+))?$/i.exec(s))){saveProgram(m[1]?pathArgument(m[1]):null);ready();return;}
    if((m=/^DEL(?:\s+(.+))?$/i.exec(s))){deleteUserFile(m[1]?pathArgument(m[1]):null);ready();return;}
    if((m=/^MOVE\s+("(?:[^"]|"")*"|\S+)\s+("(?:[^"]|"")*"|\S+)$/i.exec(s))){moveUserFile(pathArgument(m[1]),pathArgument(m[2]));ready();return;}
    if((m=/^LOAD\s+(.+)$/i.exec(s))){return loadProgram(pathArgument(m[1])).then(function(){ready();return true;}).catch(function(error){fail(error.message);return false;});}
    splitStatements(s).forEach(statement);ready();
  }catch(e){fail(e.message);}
}
form.addEventListener('submit',function(e){e.preventDefault();var value=input.value;input.value='';virtualCaret={start:0,end:0};submit(value);if(basicWasmPlaying||run&&!pendingInput)input.blur();else input.focus();});
input.addEventListener('keydown',function(e){
  if(editor){
    if(e.key==='Escape'){e.preventDefault();closeEditor();return;}
    if(e.ctrlKey||e.metaKey||e.altKey)return;
    if(e.key==='Enter'||e.key==='Backspace'||e.key==='Delete'||e.key==='Tab'||e.key.indexOf('Arrow')===0||e.key==='Home'||e.key==='End'||e.key.length===1){e.preventDefault();editorInsertKey(e.key);}
    return;
  }
  if(e.key==='ArrowUp'){e.preventDefault();historyAt=Math.max(0,historyAt-1);input.value=history[historyAt]||'';}
  if(e.key==='ArrowDown'){e.preventDefault();historyAt=Math.min(history.length,historyAt+1);input.value=history[historyAt]||'';}
  if(e.key==='Escape'){e.preventDefault();if(run)stopProgram();else input.value='';}
});
var shiftLatched=false,ctrlLatched=false;
var virtualCaret=null;
function rememberCaret(){virtualCaret={start:input.selectionStart==null?input.value.length:input.selectionStart,end:input.selectionEnd==null?input.value.length:input.selectionEnd};}
input.addEventListener('blur',rememberCaret);
input.addEventListener('input',function(){var start=input.selectionStart,end=input.selectionEnd,upper=input.value.toUpperCase();if(upper!==input.value){input.value=upper;input.setSelectionRange(start,end);}rememberCaret();});
input.addEventListener('select',rememberCaret);
function keyLatch(name,on){
  document.querySelectorAll('[data-key-label="'+name+'"]').forEach(function(key){key.classList.toggle('latched',on);key.setAttribute('aria-pressed',String(on));});
}
function insertAtCaret(value){
  var start=virtualCaret?virtualCaret.start:(input.selectionStart==null?input.value.length:input.selectionStart);
  var end=virtualCaret?virtualCaret.end:(input.selectionEnd==null?start:input.selectionEnd);
  input.focus({preventScroll:true});
  input.setRangeText(value,start,end,'end');
  rememberCaret();
}
function moveCaret(delta){
  var pos=virtualCaret?virtualCaret.start:(input.selectionStart==null?input.value.length:input.selectionStart);
  var next=Math.max(0,Math.min(input.value.length,pos+delta));
  input.focus({preventScroll:true});input.setSelectionRange(next,next);rememberCaret();
}
function virtualKey(label){
  if(label==='SHIFT'){shiftLatched=!shiftLatched;keyLatch(label,shiftLatched);return;}
  if(label==='CTRL'){ctrlLatched=!ctrlLatched;keyLatch(label,ctrlLatched);return;}
  if(editor){
    if(label==='ESC'){
      closeEditor();
      return;
    }
    var editorKeys={'SPACE':' ','RETURN':'Enter','↵':'Enter','DEL':'Backspace','TAB':'Tab','←':'ArrowLeft','→':'ArrowRight','↑':'ArrowUp','↓':'ArrowDown'};
    var editorChar=label.length===1?label:'';
    if(editorChar&&/[a-z]/i.test(editorChar))editorChar=editorChar.toUpperCase();
    else if(shiftLatched&&editorChar){var shiftedEditor={'1':'!','2':'"','3':'#','4':'$','5':'%','6':'&','7':'\'','8':'(','9':')','0':'_','-':'=','=':'+','[':'{',']':'}',';':':',"'":'"',',':'<','.':'>','/':'?'};editorChar=shiftedEditor[editorChar]||editorChar;}
    editorInsertKey(editorKeys[label]||editorChar);
    if(shiftLatched){shiftLatched=false;keyLatch('SHIFT',false);}
    return;
  }
  if(label==='ALT'){insertAtCaret('@');return;}
  if(basicWasmPlaying){
    if(label==='ESC'){escapeFromBasicGame();return;}
    var wasmKeys={'SPACE':[' ','Space'],'←':['ArrowLeft','ArrowLeft'],'→':['ArrowRight','ArrowRight'],'↑':['ArrowUp','ArrowUp'],'↓':['ArrowDown','ArrowDown']};
    if(wasmKeys[label]){sendBasicWasmKey(wasmKeys[label][0],wasmKeys[label][1],true);setTimeout(function(){sendBasicWasmKey(wasmKeys[label][0],wasmKeys[label][1],false);},35);return;}
  }
  if(run&&!pendingInput){
    var gameKeys={'SPACE':' ','←':'←','→':'→','↑':'↑','↓':'↓','RETURN':'Enter','↵':'Enter'};
    if(label==='ESC'){stopProgram();}
    else if(label.length===1){lastKey=label.toUpperCase();}
    else if(gameKeys[label])lastKey=gameKeys[label];
    if(shiftLatched){shiftLatched=false;keyLatch('SHIFT',false);}
    return;
  }
  if(ctrlLatched){
    ctrlLatched=false;keyLatch('CTRL',false);
    if(label.toUpperCase()==='A'){input.select();input.focus({preventScroll:true});return;}
    if(label.toUpperCase()==='X'||label.toUpperCase()==='C'){
      var selected=input.value.slice(input.selectionStart,input.selectionEnd);
      if(navigator.clipboard&&selected)navigator.clipboard.writeText(selected).catch(function(){});
      if(label.toUpperCase()==='X'&&selected)input.setRangeText('',input.selectionStart,input.selectionEnd,'end');
      input.focus({preventScroll:true});return;
    }
    if(label.toUpperCase()==='V'&&navigator.clipboard&&navigator.clipboard.readText){
      navigator.clipboard.readText().then(insertAtCaret).catch(function(){});return;
    }
  }
  if(label==='ESC'){if(run){stopProgram();}else input.value='';}
  else if(label==='DEL'){
    var start=virtualCaret?virtualCaret.start:(input.selectionStart==null?input.value.length:input.selectionStart),end=virtualCaret?virtualCaret.end:(input.selectionEnd==null?start:input.selectionEnd);
    if(start===end&&start>0)start--;
    input.focus({preventScroll:true});input.setRangeText('',start,end,'end');rememberCaret();
  }
  else if(label==='TAB')insertAtCaret('    ');
  else if(label==='RETURN'||label==='↵'){var value=input.value;input.value='';virtualCaret={start:0,end:0};submit(value);input.focus({preventScroll:true});}
  else if(label==='SPACE')insertAtCaret(' ');
  else if(label==='←')moveCaret(-1);
  else if(label==='→')moveCaret(1);
  else if(label==='↑'){historyAt=Math.max(0,historyAt-1);input.value=history[historyAt]||'';input.focus({preventScroll:true});input.setSelectionRange(input.value.length,input.value.length);rememberCaret();}
  else if(label==='↓'){historyAt=Math.min(history.length,historyAt+1);input.value=history[historyAt]||'';input.focus({preventScroll:true});input.setSelectionRange(input.value.length,input.value.length);rememberCaret();}
  else if(label==='COPY'){
    var pos=virtualCaret?virtualCaret.start:(input.selectionStart==null?input.value.length:input.selectionStart);
    var char=input.value.charAt(pos)||input.value.charAt(pos-1);
    if(char)insertAtCaret(char);
  }
  else {
    var shifted={'1':'!','2':'"','3':'#','4':'$','5':'%','6':'&','7':'\'','8':'(','9':')','0':'_','-':'=','=':'+','[':'{',']':'}',';':':',"'":'"',',':'<','.':'>','/':'?'};
    var character=label.length===1?label:'';
    if(character&&/[a-z]/i.test(character))character=character.toUpperCase();
    else if(shiftLatched&&shifted[character])character=shifted[character];
    if(character)insertAtCaret(character);
  }
  if(shiftLatched){shiftLatched=false;keyLatch('SHIFT',false);}
}
keyboard.addEventListener('click',function(event){
  var key=event.target.closest('[data-key-label]');
  if(key)virtualKey(key.dataset.keyLabel);
});
keyboard.addEventListener('pointerdown',function(event){if(event.target.closest('[data-key-label]'))event.preventDefault();});
window.addEventListener('keydown',function(e){if(basicWasmPlaying){if(e.key==='Escape'){e.preventDefault();escapeFromBasicGame();return;}sendBasicWasmKey(e.key,e.code,true);e.preventDefault();return;}if(e.target===input){if(run&&e.key==='Escape'){e.preventDefault();stopProgram();}return;}heldKeys[e.key]=true;if(!e.repeat)pressedKeys[e.key]=true;lastKey=e.key.length===1?e.key.toUpperCase():e.key==='ArrowLeft'?'A':e.key==='ArrowRight'?'D':e.key==='ArrowUp'?'W':e.key==='ArrowDown'?'S':'';if(run&&['ArrowLeft','ArrowRight','ArrowUp','ArrowDown',' '].includes(e.key))e.preventDefault();if(run&&e.key==='Escape'){e.preventDefault();stopProgram();}},true);
window.addEventListener('keyup',function(e){if(basicWasmPlaying){sendBasicWasmKey(e.key,e.code,false);return;}delete heldKeys[e.key];},true);
window.addEventListener('blur',function(){heldKeys=Object.create(null);pressedKeys=Object.create(null);});
document.querySelectorAll('[data-command]').forEach(function(button){button.addEventListener('click',function(){submit(button.dataset.command);if(button.dataset.command!=='RUN')input.focus();});});
document.getElementById('basic-screen').addEventListener('click',function(){input.focus();});
restoreDraft();clear();print('BLIP 64K COLOUR PERSONAL COMPUTER');print('BLIP BASIC 1.0');print('');if(lines.size)print(lines.size+' program lines recovered.');ready();
if(!blipHasTouch())input.focus();
requestAnimationFrame(tick);
window.BlipBasic={submit:submit,lines:lines,output:output,filesystem:filesystem,resolvePath:resolvePath,isDirectory:isDirectory,isFileAvailable:isFileAvailable,writeFilesystem:writeFilesystem,refreshTextures:postBasicWasmSource};
}());

// The book in the corner is picked up: it straightens and comes to where basic-book.html lays it, closed.
(function(){
var peek=document.getElementById('basic-book-peek');
if(!peek)return;
// Where basic-book.js puts the closed cover; it measures the same way (a 964x704 stage scaled to fit).
function landing(){
  var W=innerWidth,H=innerHeight;
  if(W>=861){
    var s=Math.max(.5,Math.min(1.4,(W-32)/964,(H-88)/734)),top=Math.max(24,(H-734*s-40)/2);
    return {x:(W-964*s)/2+241*s,y:top,w:482*s,h:704*s};
  }
  var w=Math.min(W-24,560);
  return {x:(W-w)/2,y:14,w:w,h:(w-12)*1.08*680/470+20};
}
function leave(){location.href=peek.href;}
peek.addEventListener('click',function(event){
  if(event.defaultPrevented||event.button||event.metaKey||event.ctrlKey||event.shiftKey||event.altKey)return;
  if(matchMedia('(prefers-reduced-motion: reduce)').matches)return;
  event.preventDefault();
  var m=new DOMMatrix(getComputedStyle(peek).transform),angle=Math.atan2(m.b,m.a)*180/Math.PI;
  var keep=peek.style.transform;peek.style.transform='none';
  var from=peek.getBoundingClientRect(),img=peek.querySelector('img'),full=img.getBoundingClientRect();
  peek.style.transform=keep;
  var to=landing(),k=full.width/to.w;
  var flight=document.createElement('div'),desk=document.createElement('div');
  flight.className='book-flight';desk.className='book-desk';
  flight.style.width=to.w+'px';flight.style.height=to.h+'px';
  flight.appendChild(img.cloneNode());
  // A phone shows a cropped corner of the cover; the crop opens as the book comes up.
  var cropR=Math.max(0,(full.width-from.width)/k),cropB=Math.max(0,(full.height-from.height)/k);
  document.body.append(desk,flight);
  peek.style.visibility='hidden';
  var ease='cubic-bezier(.3,.7,.2,1)',ms=760;
  desk.animate([{opacity:0},{opacity:1}],{duration:ms*.8,easing:'ease-in',fill:'forwards'});
  flight.animate([
    {transform:'translate('+from.left+'px,'+from.top+'px) rotate('+angle+'deg) scale('+k+')',clipPath:'inset(0 '+cropR+'px '+cropB+'px 0)'},
    {transform:'translate('+to.x+'px,'+to.y+'px) rotate(0deg) scale(1)',clipPath:'inset(0 0 0 0)'}
  ],{duration:ms,easing:ease,fill:'forwards'}).finished.then(leave,leave);
});
// Coming back from the book (it closes with history.back()), the page may be restored as it was left.
addEventListener('pageshow',function(event){
  if(!event.persisted)return;
  document.querySelectorAll('.book-flight,.book-desk').forEach(function(node){node.remove();});
  peek.style.visibility='';
});
}());
