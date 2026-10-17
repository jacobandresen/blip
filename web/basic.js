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
if (blipHasTouch()) input.readOnly = true;
var canvas = document.getElementById('basic-graphics');
var gameFrame = document.getElementById('amstrad-game-frame');
var exitGameButton = document.getElementById('exit-game');
var maximizeScreenButton = document.getElementById('maximize-screen');
var catalogPanel = document.getElementById('basic-catalog');
var gameList = document.getElementById('basic-game-list');
var tapeLoader = document.getElementById('tape-loader');
var tapeLoaderMessage = document.getElementById('tape-loader-message');
var tapeLoaderName = document.getElementById('tape-loader-name');
var launchTimer = 0;
var controlDeck = document.querySelector('.amstrad-controls');
var controlStick = controlDeck.querySelector('.stick-base');
var controlBall = controlDeck.querySelector('.stick-ball');
var controlFire = controlDeck.querySelector('.fire-buttons');
var rallyDialP1 = document.getElementById('amstrad-rally-dial-p1');
var rallyDialP2 = document.getElementById('amstrad-rally-dial-p2');
var heldDirections = [];
function updatePlayClearance(){
  var controls=document.body.hasAttribute('data-rally')?[rallyDialP1,rallyDialP2]:[controlDeck,controlStick,controlFire];
  var top=Math.min.apply(Math,controls.map(function(el){return el.getBoundingClientRect().top;}));
  document.documentElement.style.setProperty('--amstrad-control-clearance',Math.max(0,document.documentElement.clientHeight-top)+'px');
}
updatePlayClearance();
window.addEventListener('resize',updatePlayClearance);
window.addEventListener('orientationchange',function(){requestAnimationFrame(updatePlayClearance);});
function gameController() {
  try { return gameFrame.contentWindow && gameFrame.contentWindow.BlipController; } catch (e) { return null; }
}
function bindRallyDials(){
  if(!document.body.hasAttribute('data-rally'))return;
  var controller=gameController();
  if(!controller||!controller.bindDial)return;
  [rallyDialP1,rallyDialP2].forEach(function(dial){
    var hand=dial.querySelector('.amstrad-dial-hand');
    var start=0,angle=0;
    function point(event){
      var box=dial.getBoundingClientRect();
      var next=Math.atan2(event.clientY-(box.top+box.height/2),event.clientX-(box.left+box.width/2));
      var delta=next-start;
      if(delta>Math.PI)delta-=Math.PI*2;
      if(delta< -Math.PI)delta+=Math.PI*2;
      angle+=delta;start=next;
      hand.style.transform='rotate('+angle+'rad)';
    }
    dial.addEventListener('pointerdown',function(event){start=Math.atan2(event.clientY-(dial.getBoundingClientRect().top+dial.offsetHeight/2),event.clientX-(dial.getBoundingClientRect().left+dial.offsetWidth/2));});
    dial.addEventListener('pointermove',function(event){if(event.buttons)point(event);});
  });
  function selectMode(mode){
    try { if(gameFrame.contentWindow.blipSetMode)gameFrame.contentWindow.blipSetMode(mode); } catch(e){}
  }
  controller.bindDial(rallyDialP1,{
    up:{key:'ArrowUp',code:'ArrowUp'},down:{key:'ArrowDown',code:'ArrowDown'},
    tap:{key:' ',code:'Space'},onTap:function(){selectMode(0);},onInteract:function(){selectMode(0);}
  });
  controller.bindDial(rallyDialP2,{
    up:{key:'i',code:'KeyI'},down:{key:'k',code:'KeyK'},
    tap:{key:'2',code:'Digit2'},onTap:function(){selectMode(1);},onInteract:function(){selectMode(1);}
  });
}
gameFrame.addEventListener('load',bindRallyDials);
function setGameInput(name, down) {
  var controller = gameController();
  if (controller) controller.set(name, down);
}
function releaseDeck() {
  heldDirections.forEach(function (name) { setGameInput(name, false); });
  heldDirections = [];
  setGameInput('button1', false);
  controlBall.style.transform = '';
}
controlStick.addEventListener('pointerdown', function (event) {
  event.preventDefault();
  try { controlStick.setPointerCapture(event.pointerId); } catch (e) {}
  moveStick(event);
});
function moveStick(event) {
  var rect = controlStick.getBoundingClientRect();
  var x = Math.max(-1, Math.min(1, (event.clientX - (rect.left + rect.width / 2)) / (rect.width * .34)));
  var y = Math.max(-1, Math.min(1, (event.clientY - (rect.top + rect.height / 2)) / (rect.height * .34)));
  var next = [];
  if (Math.abs(x) > .28) next.push(x < 0 ? 'left' : 'right');
  if (Math.abs(y) > .28) next.push(y < 0 ? 'up' : 'down');
  heldDirections.filter(function (name) { return next.indexOf(name) < 0; }).forEach(function (name) { setGameInput(name, false); });
  next.filter(function (name) { return heldDirections.indexOf(name) < 0; }).forEach(function (name) { setGameInput(name, true); });
  heldDirections = next;
  controlBall.style.transform = 'translate(' + (x * 12) + 'px,' + (y * 9) + 'px)';
}
controlStick.addEventListener('pointermove', function (event) { if (event.buttons) moveStick(event); });
['pointerup', 'pointercancel', 'lostpointercapture'].forEach(function (type) { controlStick.addEventListener(type, releaseDeck); });
controlFire.querySelectorAll('.arcade-btn').forEach(function (button) {
  button.setAttribute('data-blip', 'button1');
  button.setAttribute('aria-label', 'Fire');
  button.addEventListener('pointerdown', function (event) { event.preventDefault(); try { button.setPointerCapture(event.pointerId); } catch (e) {} setGameInput('button1', true); });
  ['pointerup', 'pointercancel', 'lostpointercapture'].forEach(function (type) { button.addEventListener(type, function () { setGameInput('button1', false); }); });
});
window.addEventListener('blur', releaseDeck);
// "Tape Player Sounds" by AldebaranCW, sourced through Pixabay; original is CC0.
var tapeLoadingAudio = new Audio('amstrad-tape-load.mp3');
tapeLoadingAudio.preload = 'auto';
var gfx = canvas.getContext('2d');
var palette = ['#101da3','#fff5a4','#00d8e6','#ff5696','#a5ea6a','#ffad44','#e6e6e6','#131313'];
var pen = 1, paper = 0, gx = 0, gy = 0;
var lines = new Map();
var vars = Object.create(null);
var lastKey = '';
var run = null, pendingInput = null, history = [], historyAt = 0;
var STORE = 'blip-basic-programs';
var DRAFT = 'blip-basic-draft';
function storeDraft() {
  try { localStorage.setItem(DRAFT, JSON.stringify(Array.from(lines))); } catch (e) {}
}
function restoreDraft() {
  try {
    var draft = JSON.parse(localStorage.getItem(DRAFT) || '[]');
    if (Array.isArray(draft)) draft.forEach(function (entry) {
      if (Array.isArray(entry) && Number.isInteger(entry[0]) && typeof entry[1] === 'string') lines.set(entry[0], entry[1]);
    });
  } catch (e) {}
}

function print(s) {
  output.textContent += String(s) + '\n';
  var all = output.textContent.split('\n');
  if (all.length > 180) output.textContent = all.slice(-180).join('\n');
  output.scrollTop = output.scrollHeight;
}
function clear() { output.textContent = ''; gfx.fillStyle = palette[paper]; gfx.fillRect(0, 0, 320, 200); }
function ready() { if (!run) print('Ready'); }
function fail(message) { run = null; pendingInput = null; print('? ' + message); ready(); }
function catalog() {
  catalogPanel.hidden = false;
  print('BLIP GAMES');
  Object.keys(BLIP_GAMES).forEach(function (slug) { print('  ' + BLIP_GAMES[slug].name.padEnd(12) + ' RUN "' + slug.toUpperCase() + '"'); });
  print('Type RUN "name" to play.');
}
function stored() { try { return JSON.parse(localStorage.getItem(STORE) || '{}') || {}; } catch (e) { return {}; } }
function save(name) {
  if (!/^[a-z0-9_ -]{1,32}$/i.test(name)) throw Error('Bad program name');
  var all = stored(); all[name.toUpperCase()] = Array.from(lines).sort(function(a,b){return a[0]-b[0];}).map(function(x){return x[0]+' '+x[1];}).join('\n');
  localStorage.setItem(STORE, JSON.stringify(all)); print('Saved ' + name.toUpperCase());
}
function load(name) {
  var source = stored()[name.toUpperCase()];
  if (source === undefined) throw Error('Program not found: ' + name);
  lines.clear(); source.split('\n').forEach(function (s) { var m = /^(\d+)\s+(.*)$/.exec(s); if (m) lines.set(Number(m[1]), m[2]); });
  storeDraft();
  print('Loaded ' + name.toUpperCase());
}
function splitStatements(s) {
  var out = [], part = '', quoted = false;
  for (var i=0;i<s.length;i++) {
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
function tokenize(s) {
  var out=[], re=/\s*(?:([0-9]+(?:\.[0-9]*)?|\.[0-9]+)|("(?:[^"]|"")*")|([A-Za-z_][A-Za-z0-9_$]*)|(<=|>=|<>|[+\-*/(),=<>]))/gy;
  var i=0;
  while(i<s.length) {
    re.lastIndex=i; var m=re.exec(s);
    if(!m) { if(/^\s*$/.test(s.slice(i))) break; throw Error('Syntax near '+s.slice(i, i+12)); }
    out.push(m[1]!==undefined?{kind:'num',value:Number(m[1])}:m[2]!==undefined?{kind:'str',value:m[2].slice(1,-1).replace(/""/g,'"')}:{kind:'sym',value:(m[3]||m[4]).toUpperCase()});
    i=re.lastIndex;
  }
  return out;
}
function evalExpr(source) {
  var ts=tokenize(source), at=0;
  function peek(){return ts[at]&&ts[at].value;}
  function eat(v){if(peek()===v){at++;return true;}return false;}
  function need(v){if(!eat(v)) throw Error('Expected '+v);}
  function atom(){
    var t=ts[at++]; if(!t) throw Error('Expression expected');
    if(t.kind==='num'||t.kind==='str') return t.value;
    if(t.value==='('){var v=expr(0);need(')');return v;}
    if(t.value==='-') return -Number(atom());
    if(t.value==='+') return Number(atom());
    if(t.value==='NOT') return Number(!Number(atom()));
    if(t.kind==='sym' && /^[A-Z_]/.test(t.value)) {
      var name=t.value;
      if(eat('(')) {
        var args=[];if(!eat(')')){do{args.push(expr(0));}while(eat(','));need(')');}
        if(name==='INT')return Math.floor(Number(args[0]));
        if(name==='ABS')return Math.abs(Number(args[0]));
        if(name==='RND')return Math.random()*(args.length?Number(args[0]):1);
        if(name==='LEN')return String(args[0]).length;
        if(name==='CHR$')return String.fromCharCode(Number(args[0]));
        throw Error('Unknown function '+name);
      }
      if(name==='INKEY$'){var k=lastKey;lastKey='';return k;}
      return Object.prototype.hasOwnProperty.call(vars,name)?vars[name]:(name.endsWith('$')?'':0);
    }
    throw Error('Expression expected');
  }
  var prec={OR:1,AND:2,'=':3,'<>':3,'<':3,'>':3,'<=':3,'>=':3,'+':4,'-':4,'*':5,'/':5};
  function expr(min){
    var left=atom();
    while(prec[peek()]>=min && peek()!==undefined){
      var op=ts[at++].value,right=expr(prec[op]+1);
      if(op==='+') left=typeof left==='string'||typeof right==='string'?String(left)+String(right):Number(left)+Number(right);
      else if(op==='-')left=Number(left)-Number(right);
      else if(op==='*')left=Number(left)*Number(right);
      else if(op==='/'){if(Number(right)===0)throw Error('Division by zero');left=Number(left)/Number(right);}
      else if(op==='=')left=Number(left===right);
      else if(op==='<>')left=Number(left!==right);
      else if(op==='<')left=Number(left<right);
      else if(op==='>')left=Number(left>right);
      else if(op==='<=')left=Number(left<=right);
      else if(op==='>=')left=Number(left>=right);
      else if(op==='AND')left=Number(Boolean(left)&&Boolean(right));
      else if(op==='OR')left=Number(Boolean(left)||Boolean(right));
    }
    return left;
  }
  var result=expr(0);if(at!==ts.length)throw Error('Syntax near '+ts[at].value);return result;
}
function jump(number) {
  if(!run)throw Error('Not running');
  var i=run.numbers.indexOf(Number(number)); if(i<0)throw Error('Line '+number+' not found');
  run.index=i;run.statement=0;
}
function playTapeLoading() {
  if (blipMuted()) return;
  try {
    tapeLoadingAudio.volume = .9;
    tapeLoadingAudio.currentTime = 0;
    var playback = tapeLoadingAudio.play();
    if (playback) playback.catch(function () {});
  } catch (e) {}
}
function launch(name) {
  if (launchTimer) return;
  var q=String(name).trim().toLowerCase().replace(/\s+/g,'_');
  var slug=Object.keys(BLIP_GAMES).find(function (s) { return s===q || BLIP_GAMES[s].name.toLowerCase()===q; });
  if(!slug)throw Error('Game not found: '+name);
  catalogPanel.hidden = true;
  blipSetFrontend('basic');
  tapeLoaderName.textContent = '"' + BLIP_GAMES[slug].name + '"';
  document.getElementById('loading-tape-label').lastChild.textContent = BLIP_GAMES[slug].name;
  tapeLoaderMessage.textContent = 'LOADING';
  tapeLoader.hidden = false;
  document.body.setAttribute('data-loading-tape', '');
  playTapeLoading();
  input.blur();
  launchTimer = setTimeout(function () {
    tapeLoader.hidden = true;
    if(slug==='rally')document.body.setAttribute('data-rally','');
    else document.body.removeAttribute('data-rally');
    document.body.setAttribute('data-playing', '');
    updatePlayClearance();
    gameFrame.hidden = false;
    gameFrame.src = slug + '/?amstrad=1';
    launchTimer = 0;
  }, 7900);
}
function exitGame() {
  if (gameFrame.hidden) return;
  gameFrame.src = 'about:blank';
  gameFrame.hidden = true;
  document.body.removeAttribute('data-loading-tape');
  document.body.removeAttribute('data-playing');
  document.body.removeAttribute('data-rally');
  updatePlayClearance();
  input.focus({ preventScroll: true });
}
window.addEventListener('message', function (event) {
  if (event.origin === location.origin && event.data && event.data.type === 'blip-amstrad-exit') exitGame();
});
exitGameButton.addEventListener('click', exitGame);
function syncMaximizeScreen(){
  var maximized=document.documentElement.hasAttribute('data-fullscreen');
  maximizeScreenButton.setAttribute('aria-pressed',String(maximized));
  maximizeScreenButton.setAttribute('aria-label',maximized?'Restore Amstrad screen':'Maximize Amstrad screen');
  maximizeScreenButton.title=maximized?'Restore screen':'Maximize screen';
}
maximizeScreenButton.addEventListener('click',function(){
  var root=document.documentElement;
  var maximized=root.hasAttribute('data-fullscreen');
  blipSetFullscreen(!maximized);
  syncMaximizeScreen();
  if(!maximized){
    var enter=root.requestFullscreen||root.webkitRequestFullscreen;
    if(enter){try{var result=enter.call(root);if(result&&result.catch)result.catch(function(){});}catch(e){}}
  }else{
    var leave=document.exitFullscreen||document.webkitExitFullscreen;
    if((document.fullscreenElement||document.webkitFullscreenElement)&&leave){try{var result=leave.call(document);if(result&&result.catch)result.catch(function(){});}catch(e){}}
  }
});
['fullscreenchange','webkitfullscreenchange'].forEach(function(name){document.addEventListener(name,syncMaximizeScreen);});
syncMaximizeScreen();
function statement(s) {
  var m, command=s.trim(), upper=command.toUpperCase();
  if(!command || /^REM(?:\s|$)/i.test(command) || command[0]==="'")return;
  if((m=/^(?:LET\s+)?([A-Z][A-Z0-9_]*\$?)\s*=\s*(.+)$/i.exec(command))){vars[m[1].toUpperCase()]=evalExpr(m[2]);return;}
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
  if(upper==='END'||upper==='STOP'){run=null;ready();return;}
  if(upper==='CLS'){clear();return;}
  if(upper==='CLEAR'){vars=Object.create(null);pendingInput=null;clear();return;}
  if((m=/^(?:PEN|INK)\s+(.+)$/i.exec(command))){pen=((Number(evalExpr(m[1]))%palette.length)+palette.length)%palette.length;return;}
  if((m=/^PAPER\s+(.+)$/i.exec(command))){paper=((Number(evalExpr(m[1]))%palette.length)+palette.length)%palette.length;clear();return;}
  if((m=/^PLOT\s+(.+)$/i.exec(command))){var a=splitArgs(m[1]);gx=Number(evalExpr(a[0]));gy=Number(evalExpr(a[1]));gfx.fillStyle=palette[pen];gfx.fillRect(Math.round(gx),199-Math.round(gy),1,1);return;}
  if((m=/^DRAW\s+(.+)$/i.exec(command))){var p=splitArgs(m[1]),x=Number(evalExpr(p[0])),y=Number(evalExpr(p[1]));gfx.strokeStyle=palette[pen];gfx.beginPath();gfx.moveTo(gx+.5,199-gy+.5);gfx.lineTo(x+.5,199-y+.5);gfx.stroke();gx=x;gy=y;return;}
  if((m=/^INPUT\s+(?:"([^"]*)"\s*[,;]\s*)?([A-Z][A-Z0-9_]*\$?)$/i.exec(command))){pendingInput=m[2].toUpperCase();print(m[1]||'?');input.placeholder=pendingInput+'?';input.focus();return;}
  if((m=/^RUN\s+(.+)$/i.exec(command))){launch(evalExpr(m[1]));return;}
  if((m=/^WAIT\s+(.+)$/i.exec(command))){if(!run)throw Error('WAIT requires RUN');run.waitUntil=performance.now()+Math.max(0,Math.min(10000,Number(evalExpr(m[1]))));return;}
  throw Error('Unknown command: '+command.split(/\s/)[0]);
}
function tick() {
  if(run&&!pendingInput&&performance.now()>=run.waitUntil){
    try {
      for(var n=0;n<250&&run&&!pendingInput&&performance.now()>=run.waitUntil;n++){
        if(run.index>=run.numbers.length){run=null;ready();break;}
        var pieces=splitStatements(lines.get(run.numbers[run.index]));
        if(run.statement>=pieces.length){run.index++;run.statement=0;continue;}
        var code=pieces[run.statement++];statement(code);
      }
    }catch(e){fail(e.message);}
  }
  requestAnimationFrame(tick);
}
function submit(raw) {
  var s=raw.trim();if(!s)return;
  history.push(s);historyAt=history.length;
  print('> '+s);
  if(pendingInput){vars[pendingInput]=pendingInput.endsWith('$')?s:Number(s)||0;pendingInput=null;input.placeholder='';return;}
  var numbered=/^(\d{1,5})(?:\s+(.*))?$/.exec(s);
  if(numbered){var number=Number(numbered[1]);if(numbered[2])lines.set(number,numbered[2]);else lines.delete(number);storeDraft();return;}
  try {
    var m;
    if(s.toUpperCase()==='RUN'){
      if(!lines.size)throw Error('No program');
      vars=Object.create(null);run={numbers:Array.from(lines.keys()).sort(function(a,b){return a-b;}),index:0,statement:0,calls:[],fors:[],waitUntil:0};clear();input.blur();return;
    }
    if(s.toUpperCase()==='LIST'){Array.from(lines.keys()).sort(function(a,b){return a-b;}).forEach(function(k){print(k+' '+lines.get(k));});ready();return;}
    if(s.toUpperCase()==='NEW'){run=null;lines.clear();vars=Object.create(null);storeDraft();clear();ready();return;}
    if(s.toUpperCase()==='GAMES'){catalog();ready();return;}
    if(s.toUpperCase()==='HELP'){print('Line numbers store a program. RUN plays it.');print('LIST  CLEAR  NEW  SAVE "name"  LOAD "name"');print('CLEAR clears the screen and variables; NEW deletes the program.');print('PRINT INPUT IF THEN GOTO FOR NEXT');print('PLOT DRAW PEN PAPER CLS INKEY$ WAIT');print('GAMES lists BLIP games; RUN "SERPENT" starts one.');ready();return;}
    if(s.toUpperCase()==='STOP'){run=null;pendingInput=null;ready();return;}
    if((m=/^SAVE\s+"([^"]+)"$/i.exec(s))){save(m[1]);ready();return;}
    if((m=/^LOAD\s+"([^"]+)"$/i.exec(s))){load(m[1]);ready();return;}
    splitStatements(s).forEach(statement);ready();
  }catch(e){fail(e.message);}
}
form.addEventListener('submit',function(e){e.preventDefault();var value=input.value;input.value='';virtualCaret={start:0,end:0};submit(value);input.focus();});
input.addEventListener('keydown',function(e){
  if(e.key==='ArrowUp'){e.preventDefault();historyAt=Math.max(0,historyAt-1);input.value=history[historyAt]||'';}
  if(e.key==='ArrowDown'){e.preventDefault();historyAt=Math.min(history.length,historyAt+1);input.value=history[historyAt]||'';}
  if(e.key==='Escape'){input.value='';}
});
var shiftLatched=false,ctrlLatched=false;
var virtualCaret=null;
function rememberCaret(){virtualCaret={start:input.selectionStart==null?input.value.length:input.selectionStart,end:input.selectionEnd==null?input.value.length:input.selectionEnd};}
input.addEventListener('blur',rememberCaret);
input.addEventListener('input',rememberCaret);
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
  if(label==='ALT'){insertAtCaret('@');return;}
  if(run&&!pendingInput){
    var gameKeys={'SPACE':' ','←':'←','→':'→','↑':'↑','↓':'↓','RETURN':'Enter','↵':'Enter'};
    if(label==='ESC'){run=null;ready();}
    else if(label.length===1){lastKey=shiftLatched?label.toLowerCase():label.toUpperCase();}
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
  if(label==='ESC'){if(run){run=null;pendingInput=null;ready();}else input.value='';}
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
    if(character&&/[a-z]/i.test(character))character=shiftLatched?character.toLowerCase():character.toUpperCase();
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
document.addEventListener('keydown',function(e){if(e.target!==input){lastKey=e.key.length===1?e.key.toUpperCase():e.key==='ArrowLeft'?'←':e.key==='ArrowRight'?'→':e.key==='ArrowUp'?'↑':e.key==='ArrowDown'?'↓':'';if(run&&['ArrowLeft','ArrowRight','ArrowUp','ArrowDown',' '].includes(e.key))e.preventDefault();if(run&&e.key==='Escape'){run=null;ready();}}});
document.querySelectorAll('[data-command]').forEach(function(button){button.addEventListener('click',function(){submit(button.dataset.command);if(button.dataset.command!=='RUN')input.focus();});});
Object.keys(BLIP_GAMES).forEach(function(slug){var button=document.createElement('button');button.type='button';button.textContent=BLIP_GAMES[slug].name;button.addEventListener('click',function(){launch(slug);});gameList.appendChild(button);});
document.getElementById('basic-screen').addEventListener('click',function(){input.focus();});
restoreDraft();clear();print('BLIP 64K COLOUR PERSONAL COMPUTER');print('BLIP BASIC 1.0');print('');if(lines.size)print(lines.size+' program lines recovered.');ready();
if(!blipHasTouch())input.focus();
requestAnimationFrame(tick);
window.BlipBasic={submit:submit,lines:lines,output:output};
}());
