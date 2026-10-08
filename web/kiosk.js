var MAX_COINS = 5;

// A held finger must never raise a context menu (Android does, and it cancels the touch).
document.addEventListener('contextmenu', function (e) {
  if (!(e.target.closest && e.target.closest('input, textarea, [contenteditable]'))) e.preventDefault();
});
var TWO_PLAYER_KEYS = {
  up: { key: 'w', code: 'KeyW' }, down: { key: 's', code: 'KeyS' },
  left: { key: 'a', code: 'KeyA' }, right: { key: 'd', code: 'KeyD' },
  p2up: { key: 'ArrowUp', code: 'ArrowUp' }, p2down: { key: 'ArrowDown', code: 'ArrowDown' },
  p2left: { key: 'ArrowLeft', code: 'ArrowLeft' }, p2right: { key: 'ArrowRight', code: 'ArrowRight' },
  p2button1: { key: 'j', code: 'KeyJ' }, p2button2: { key: 'k', code: 'KeyK' }
};

/* ---- Layout: upright or landscape ----
 * One rule for every page's deck, applied as <html data-layout> and kept
 * current on resize, so the landing page and a game lay the bar out alike. */
function blipLandscape() {
  return window.innerHeight <= 520 && window.innerWidth > window.innerHeight * 1.25;
}
function blipApplyLayout() {
  document.documentElement.setAttribute('data-layout', blipLandscape() ? 'landscape' : 'upright');
}
blipApplyLayout();
window.addEventListener('resize', blipApplyLayout);
window.addEventListener('orientationchange', blipApplyLayout);

/** The controller in use: 'pad', 'stick' or 'touch' (the strip, only on a
 * game that has one). */
function blipControls() {
  var root = document.documentElement;
  if (root.hasAttribute('data-touch')) return 'touch';
  return root.getAttribute('data-controls') === 'stick' ? 'stick' : 'pad';
}

/** The pad or stick chosen, which a game without a touch strip keeps using
 * while touch is chosen. */
function blipPhysicalControls() {
  var m = 'pad';
  try { m = localStorage.getItem('blip-controls') || 'pad'; } catch (e) {}
  return m === 'stick' ? 'stick' : 'pad';
}

/** A touch screen: the only place TOUCH is offered. */
function blipHasTouch() {
  return ('ontouchstart' in window) || navigator.maxTouchPoints > 0;
}

/** On a touch screen: touch was chosen, or no choice has been made yet. */
function blipTouchChosen() {
  if (!blipHasTouch()) return false;
  var v = null;
  try { v = localStorage.getItem('blip-touch'); } catch (e) {}
  return v !== '0';
}

/** Apply the stored choice as <html data-controls> (+ data-touch). Touch
 * borrows the pad's bar so the picture is fitted the same. */
function blipApplyControls() {
  var root = document.documentElement;
  var game = blipGameFromPath(window.location.pathname);
  var touch = !!(game && game.touch) && blipTouchChosen();
  root.setAttribute('data-controls', touch ? 'pad' : blipPhysicalControls());
  if (touch) root.setAttribute('data-touch', game.touch.kind);
  else root.removeAttribute('data-touch');
}

function blipSetControls(mode) {
  try {
    if (mode === 'touch') localStorage.setItem('blip-touch', '1');
    else {
      localStorage.setItem('blip-touch', '0');
      if (mode === 'pad' || mode === 'stick') localStorage.setItem('blip-controls', mode);
    }
  } catch (e) {}
  blipApplyControls();
  if (typeof window.onBlipControlsChange === 'function') {
    try { window.onBlipControlsChange(blipControls()); } catch (e) {}
  }
}

/* ---- Per-game cabinet identity ---- Card colours on the landing page,
 * marquee and bezel glow on each game page. `accent` is an "r, g, b" triple
 * so CSS can build solid and translucent colours. */
// `buttons` map deck caps; `stick` accepts engage/release and maxR pixels plus hysteresis degrees.
// `touch.kind`: drag follows a finger, swipe steers by flick, paddles split bats, platform runs/blows/jumps.
// `hint` labels the touch strip; `mouseHint` labels a touchscreen laptop's mouse controls.
var BLIP_GAMES = {
  serpent:            { name: 'SERPENT',  accent: '50, 200, 50',
                         touch: { kind: 'swipe', hint: 'Swipe to steer', mouseHint: 'Click and drag to steer' } },
  bouncer:            { name: 'BOUNCER',  accent: '0, 200, 200',
                         touch: { kind: 'drag', hint: 'Tap to launch', mouseHint: 'Point to move &middot; Click to launch' } },
  galactic_defender:  { name: 'DEFENDER', accent: '200, 50, 200',
                         touch: { kind: 'drag', hint: 'Hold to fire', mouseHint: 'Point to move &middot; Hold the button to fire' } },
  rally:              { name: 'RALLY',    accent: '220, 50, 50',
                         touch: { kind: 'paddles', hint: 'Drag up or down', mouseHint: 'Point up or down &middot; Click to serve' } },
  meteors:            { name: 'METEORS',  accent: '180, 180, 180',
                         buttons: [{ key: ' ', code: 'Space' }, { key: 'z', code: 'KeyZ' }] },
  // Two caps; holding toward hits high. `players: 2` enables the second station; `keys` maps P1 to WASD.
  brawler:            { name: 'BRAWLER', accent: '220, 60, 40',
                         players: 2,
                         buttons: [{ key: 'f', code: 'KeyF', label: 'PUNCH' },
                                   { key: 'g', code: 'KeyG', label: 'KICK' }],
                         keys: TWO_PLAYER_KEYS,
                         stick: { engage: 10, release: 6, maxR: 54, hyst: 10 } },
  // A tribute to Bubble Bobble. Two caps: bubble (fire) and jump; up
  // jumps too. Like Brawler, player one is WASD so the arrows are player
  // two's, who drops in with their own bubble or jump.
  bubbler:            { name: 'BUBBLER', accent: '120, 210, 255',
                         players: 2,
                         touch: { kind: 'platform', hint: 'Touch to bubble &middot; slide to run &middot; swipe up to jump',
                                  mouseHint: 'Drag to run &middot; click to bubble &middot; drag up to jump' },
                         buttons: [{ key: 'f', code: 'KeyF', label: 'BUBBLE' },
                                   { key: 'g', code: 'KeyG', label: 'JUMP' }],
                         keys: TWO_PLAYER_KEYS },
  // A bullet-weaving shooter: fine nudges. A small dead zone, a long pivot
  // leash so re-centring neutralises, and a firm notch so a dodge holds.
  sky_raider:         { name: 'RAIDER', accent: '50, 100, 220',
                         stick: { engage: 11, release: 6, maxR: 58, hyst: 12 } },
  // Viper-inspired trail arena: relative left/right turns, passable dim gaps.
  // Space only starts/restarts; J joins player two. Drag steers by horizontal position.
  adder:              { name: 'ADDER', accent: '224, 160, 52',
                         players: 2,
                         buttons: [{ key: ' ', code: 'Space', label: 'START' }],
                         keys: TWO_PLAYER_KEYS,
                         touch: { kind: 'drag', hint: 'Drag left/right to steer &middot; tap to start',
                                  mouseHint: 'Point left/right to steer &middot; click to start' } }
};

// Pick out the game slug from a shell-page URL, e.g. "/blip/serpent/index.html"
// or "/serpent/" both resolve to "serpent". Returns null off the game pages.
function blipGameFromPath(pathname) {
  var m = /\/([a-z_]+)\/(?:index\.html)?$/i.exec(pathname || '');
  var g = m && BLIP_GAMES[m[1]];
  return g ? { slug: m[1], name: g.name, accent: g.accent, buttons: g.buttons,
               keys: g.keys, players: g.players || 1, stick: g.stick,
               touch: g.touch || null } : null;
}

// Before first paint, so no controller flashes. data-has-touch shows the
// TOUCH badges on the landing page.
blipApplyControls();
if (blipHasTouch()) document.documentElement.setAttribute('data-has-touch', '');

if ('serviceWorker' in navigator) {
  var _manifest = document.querySelector('link[rel=manifest]');
  if (_manifest) {
    var _swUrl = new URL('sw.js', new URL(_manifest.href, location.href)).href;
    navigator.serviceWorker.register(_swUrl);
  }
}


function getCoins() {
  try {
    var n = parseInt(sessionStorage.getItem('blip-coins') || '0', 10);
    return isNaN(n) ? 0 : Math.min(Math.max(n, 0), MAX_COINS);
  } catch (e) { return 0; }
}

function saveCoins(n) {
  try { sessionStorage.setItem('blip-coins', n); } catch (e) {}
}

// A fresh cabinet visit powers down; travelling between its pages keeps credit.
(function(){
  if(!document.getElementById('game-grid')||new URLSearchParams(location.search).has('manual'))return;
  var navigation=performance.getEntriesByType('navigation')[0],internal=false;
  try{var previous=new URL(document.referrer),scope=new URL('.',location.href);internal=previous.origin===scope.origin&&previous.pathname.startsWith(scope.pathname);}catch(e){}
  if((navigation&&navigation.type==='reload')||(!internal&&(!navigation||navigation.type!=='back_forward')))saveCoins(0);
}());
document.documentElement.toggleAttribute('data-credit',getCoins()>0);
function buildPowerIndicators() {
  if (!document.body || document.querySelector('.blip-power-indicator')) return;
  var lamp=document.createElement('div');
  lamp.className='blip-power-indicator';
  lamp.setAttribute('role','status');
  lamp.setAttribute('aria-live','polite');
  lamp.innerHTML='<span class="power-label">POWER</span><i class="power-bulb" aria-hidden="true"><svg viewBox="0 0 10 16"><path d="M6.2 0 1 9h3.3L3.8 16 9 6.7H5.7z"/></svg></i>';
  document.body.appendChild(lamp);
  function place() {
    var topBar=document.querySelector('.top-marquee-bar, #marquee-bar')||document.querySelector('.blip-logo');
    if(!topBar)return;
    var barBox=topBar.getBoundingClientRect();
    lamp.style.left='';
    lamp.style.right='max('+Math.max(8,innerWidth-barBox.right+8)+'px,calc(env(safe-area-inset-right,0px) + 8px))';
    lamp.style.top=barBox.top+(barBox.height-lamp.offsetHeight)/2+'px';
  }
  function sync() {
    var on=document.documentElement.hasAttribute('data-credit');
    lamp.setAttribute('aria-label','Power '+(on?'on':'off'));
    lamp.title='Power '+(on?'on':'off');
    place();
  }
  sync();
  window.addEventListener('resize',place);
  if(typeof ResizeObserver==='function'){
    var resizeObserver=new ResizeObserver(place);
    var bar=document.querySelector('.top-marquee-bar, #marquee-bar')||document.querySelector('.blip-logo');
    if(bar)resizeObserver.observe(bar);
  }
  new MutationObserver(sync).observe(document.documentElement,{attributes:true,attributeFilter:['data-credit']});
}
if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',buildPowerIndicators,{once:true});
else buildPowerIndicators();
var cabinetPowerTimer=null;
function updateCoinsHud() {
  var root=document.documentElement,powered=getCoins()>0,wasPowered=root.hasAttribute('data-credit');
  root.toggleAttribute('data-credit',powered);
  if(powered&&!wasPowered){
    clearTimeout(cabinetPowerTimer);root.setAttribute('data-power-up','');
    cabinetPowerTimer=setTimeout(function(){root.removeAttribute('data-power-up');},1200);
  }else if(!powered){clearTimeout(cabinetPowerTimer);root.removeAttribute('data-power-up');}
  blipUpdateCabinetHum();
}

/* ---- Sound on / off ---- The choice is stored ('blip-mute'). The games'
 * sounds are Howler's, muted there; the cabinet's own (coins, clicks) all
 * leave through blipOut(), one gain that is shut while muted. */
var _blipOut = null;
function blipMuted() { if(document.documentElement.hasAttribute('data-cabinet-screen')) return true; try { return localStorage.getItem('blip-mute') === '1'; } catch (e) { return false; } }
function blipOut(ctx) {
  if (!_blipOut || _blipOut.context !== ctx) {
    _blipOut = ctx.createGain();
    _blipOut.connect(ctx.destination);
  }
  _blipOut.gain.value = blipMuted() ? 0 : 1;
  return _blipOut;
}
function blipSetMuted(on) {
  try { localStorage.setItem('blip-mute', on ? '1' : '0'); } catch (e) {}
  if (typeof Howler !== 'undefined') Howler.mute(on);
  if (_blipOut) _blipOut.gain.value = on ? 0 : 1;
  document.documentElement.toggleAttribute('data-muted', on);
}
if (blipMuted()) {
  document.documentElement.setAttribute('data-muted', '');
  if (typeof Howler !== 'undefined') Howler.mute(true);
}

/* ---- Audio ---- */
var _kioskAudioCtx = null;

function getKioskAudio() {
  if (typeof Howler !== 'undefined' && Howler.ctx) {
    if (Howler.ctx.state === 'suspended') Howler.ctx.resume();
    return Howler.ctx;
  }
  if (!_kioskAudioCtx) _kioskAudioCtx = new (window.AudioContext || window.webkitAudioContext)();
  if (_kioskAudioCtx.state === 'suspended') _kioskAudioCtx.resume();
  return _kioskAudioCtx;
}

var cabinetHum=null;
function blipUpdateCabinetHum(unlock) {
  var powered=getCoins()>0&&!document.hidden&&!document.documentElement.hasAttribute('data-cabinet-screen');
  if(!powered) {
    if(cabinetHum&&!cabinetHum.offTimer) {
      var fading=cabinetHum;
      fading.output.gain.setTargetAtTime(0,fading.ctx.currentTime,.22);
      fading.offTimer=setTimeout(function(){
        fading.sources.forEach(function(source){source.stop();source.disconnect();});
        fading.nodes.forEach(function(node){node.disconnect();});
        if(cabinetHum===fading)cabinetHum=null;
      },1400);
    }
    return;
  }
  try {
    var existing=_kioskAudioCtx||(typeof Howler!=='undefined'&&Howler.ctx);
    if(!existing&&!unlock)return;
    var ctx=getKioskAudio();
    if(ctx.state!=='running') {ctx.resume().then(function(){blipUpdateCabinetHum();}).catch(function(){});return;}
    if(!cabinetHum) {
      var output=ctx.createGain(),filter=ctx.createBiquadFilter(),sources=[],nodes=[output,filter];
      filter.type='lowpass';filter.frequency.value=430;filter.Q.value=.65;
      output.gain.value=0;filter.connect(output);output.connect(blipOut(ctx));
      [[60,'sawtooth',.6],[120,'sine',.28],[93,'triangle',.12]].forEach(function(voice){
        var oscillator=ctx.createOscillator(),level=ctx.createGain();
        oscillator.type=voice[1];oscillator.frequency.value=voice[0];level.gain.value=voice[2];
        oscillator.connect(level);level.connect(filter);oscillator.start();sources.push(oscillator);nodes.push(level);
      });
      var buffer=ctx.createBuffer(1,ctx.sampleRate*2,ctx.sampleRate),data=buffer.getChannelData(0),smoothed=0;
      for(var i=0;i<data.length;i++){smoothed=.97*smoothed+.03*(Math.random()*2-1);data[i]=smoothed;}
      var fan=ctx.createBufferSource(),air=ctx.createGain();fan.buffer=buffer;fan.loop=true;air.gain.value=.55;
      fan.connect(air);air.connect(filter);fan.start();sources.push(fan);nodes.push(air);
      cabinetHum={ctx:ctx,output:output,sources:sources,nodes:nodes,offTimer:null};
    }
    clearTimeout(cabinetHum.offTimer);cabinetHum.offTimer=null;
    cabinetHum.output.gain.setTargetAtTime(.09,cabinetHum.ctx.currentTime,.35);
  }catch(e){}
}
['pointerdown','keydown','touchstart'].forEach(function(event){
  document.addEventListener(event,function(){blipUpdateCabinetHum(true);},{capture:true,passive:true});
});
document.addEventListener('visibilitychange',function(){blipUpdateCabinetHum();});
window.addEventListener('pagehide',function(){
  if(!cabinetHum)return;
  clearTimeout(cabinetHum.offTimer);
  cabinetHum.sources.forEach(function(source){source.stop();source.disconnect();});
  cabinetHum.nodes.forEach(function(node){node.disconnect();});cabinetHum=null;
});

// Chute clink, collection-box rattle, then the credit chime.
function playCoinInsert() {
  var ctx = getKioskAudio();
  var t   = ctx.currentTime;

  var noiseBuf = ctx.createBuffer(1, Math.ceil(ctx.sampleRate * 0.045), ctx.sampleRate);
  var noiseData = noiseBuf.getChannelData(0);
  for (var i = 0; i < noiseData.length; i++) noiseData[i] = Math.random() * 2 - 1;
  var noise = ctx.createBufferSource();
  noise.buffer = noiseBuf;
  var noiseFilter = ctx.createBiquadFilter();
  noiseFilter.type = 'bandpass';
  noiseFilter.frequency.value = 4200;
  noiseFilter.Q.value = 1.1;
  var noiseGain = ctx.createGain();
  noiseGain.gain.setValueAtTime(0.4, t);
  noiseGain.gain.exponentialRampToValueAtTime(0.001, t + 0.045);
  noise.connect(noiseFilter); noiseFilter.connect(noiseGain); noiseGain.connect(blipOut(ctx));
  noise.start(t); noise.stop(t + 0.045);

  [2400, 3550, 4900].forEach(function (freq, i) {
    var osc = ctx.createOscillator(), gain = ctx.createGain();
    osc.type = 'triangle'; osc.frequency.value = freq;
    osc.connect(gain); gain.connect(blipOut(ctx));
    var start = t + i * 0.006;
    gain.gain.setValueAtTime(0.11 / (i + 1), start);
    gain.gain.exponentialRampToValueAtTime(0.0008, start + 0.085);
    osc.start(start); osc.stop(start + 0.09);
  });

  function boxHit(at, strength) {
    var size=Math.ceil(ctx.sampleRate*.035),buffer=ctx.createBuffer(1,size,ctx.sampleRate),data=buffer.getChannelData(0);
    for(var i=0;i<size;i++)data[i]=(Math.random()*2-1)*(1-i/size);
    var noise=ctx.createBufferSource(),filter=ctx.createBiquadFilter(),gain=ctx.createGain();
    noise.buffer=buffer;filter.type='bandpass';filter.frequency.value=2600;filter.Q.value=1.4;
    gain.gain.setValueAtTime(strength,at);gain.gain.exponentialRampToValueAtTime(.001,at+.035);
    noise.connect(filter);filter.connect(gain);gain.connect(blipOut(ctx));noise.start(at);noise.stop(at+.036);
    [[920,.12,.19],[1470,.075,.15],[2240,.045,.105]].forEach(function(partial){
      var osc=ctx.createOscillator(),ring=ctx.createGain();osc.type='triangle';osc.frequency.value=partial[0];
      ring.gain.setValueAtTime(partial[1]*strength,at);ring.gain.exponentialRampToValueAtTime(.0005,at+partial[2]);
      osc.connect(ring);ring.connect(blipOut(ctx));osc.start(at);osc.stop(at+partial[2]);
    });
  }
  boxHit(t+.66,.9);
  boxHit(t+.73,.42);

  [{ freq: 1047, start: 0.86 }, { freq: 1319, start: 0.915 }].forEach(function(note) {
    [[1, 0.13, 0.17], [2.73, 0.028, 0.11]].forEach(function(partial) {
      var osc = ctx.createOscillator(), gain = ctx.createGain();
      var start = t + note.start, end = start + partial[2];
      osc.type = 'sine';
      osc.frequency.setValueAtTime(note.freq * partial[0], start);
      osc.frequency.exponentialRampToValueAtTime(note.freq * partial[0] * 0.985, end);
      gain.gain.setValueAtTime(partial[1], start);
      gain.gain.exponentialRampToValueAtTime(0.0008, end);
      osc.connect(gain); gain.connect(blipOut(ctx));
      osc.start(start); osc.stop(end + 0.005);
      osc.onended = function() { osc.disconnect(); gain.disconnect(); };
    });
  });
}

// A switch latch and the muted clack of a card seating in its guide.
function playCardMechanism() {
  try {
    var ctx = getKioskAudio(), t = ctx.currentTime;
    var size = Math.ceil(ctx.sampleRate * 0.026), buffer = ctx.createBuffer(1, size, ctx.sampleRate);
    var data = buffer.getChannelData(0);
    for (var i = 0; i < size; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / size);
    var noise = ctx.createBufferSource(), filter = ctx.createBiquadFilter(), snap = ctx.createGain();
    noise.buffer = buffer; filter.type = 'bandpass'; filter.frequency.value = 1750; filter.Q.value = 1.3;
    snap.gain.setValueAtTime(0.12, t); snap.gain.exponentialRampToValueAtTime(0.0001, t + 0.027);
    noise.connect(filter); filter.connect(snap); snap.connect(blipOut(ctx));
    noise.start(t); noise.stop(t + 0.028);
    var osc = ctx.createOscillator(), body = ctx.createGain();
    osc.type = 'triangle'; osc.frequency.setValueAtTime(230, t + 0.004);
    osc.frequency.exponentialRampToValueAtTime(105, t + 0.065);
    body.gain.setValueAtTime(0.0001, t); body.gain.exponentialRampToValueAtTime(0.12, t + 0.006);
    body.gain.exponentialRampToValueAtTime(0.0001, t + 0.085);
    osc.connect(body); body.connect(blipOut(ctx)); osc.start(t); osc.stop(t + 0.09);
  } catch (e) {}
}

function playRolodexMove(direction) {
  try {
    var ctx=getKioskAudio(), start=ctx.currentTime, sign=direction<0?-1:1;
    var count=3, buffer=ctx.createBuffer(1,Math.ceil(ctx.sampleRate*.19),ctx.sampleRate), data=buffer.getChannelData(0);
    for(var i=0;i<data.length;i++) data[i]=(Math.random()*2-1)*Math.exp(-i/(ctx.sampleRate*.055));
    var noise=ctx.createBufferSource(), filter=ctx.createBiquadFilter(), scrape=ctx.createGain();
    noise.buffer=buffer;filter.type='bandpass';filter.frequency.value=780;filter.Q.value=.65;
    scrape.gain.setValueAtTime(.0001,start);scrape.gain.exponentialRampToValueAtTime(.035,start+.025);scrape.gain.exponentialRampToValueAtTime(.0001,start+.2);
    noise.connect(filter);filter.connect(scrape);scrape.connect(blipOut(ctx));noise.start(start);noise.stop(start+.2);
    for(var tick=0;tick<count;tick++) {
      var at=start+.018+tick*.052, osc=ctx.createOscillator(), gain=ctx.createGain();
      osc.type='triangle';osc.frequency.setValueAtTime(155+tick*13,at);osc.frequency.exponentialRampToValueAtTime(82+tick*8,at+.035);
      gain.gain.setValueAtTime(.0001,at);gain.gain.exponentialRampToValueAtTime(.075,at+.004);gain.gain.exponentialRampToValueAtTime(.0001,at+.045);
      osc.connect(gain);gain.connect(blipOut(ctx));osc.start(at);osc.stop(at+.05);
    }
    // The card settling on the stack, as the shuffle drops it behind.
    var land=start+.42, thud=ctx.createOscillator(), thudGain=ctx.createGain();
    thud.type='sine';thud.frequency.setValueAtTime(128,land);thud.frequency.exponentialRampToValueAtTime(62,land+.07);
    thudGain.gain.setValueAtTime(.0001,land);thudGain.gain.exponentialRampToValueAtTime(.09,land+.006);thudGain.gain.exponentialRampToValueAtTime(.0001,land+.09);
    thud.connect(thudGain);thudGain.connect(blipOut(ctx));thud.start(land);thud.stop(land+.1);
    thud.onended=function(){thud.disconnect();thudGain.disconnect();};
  } catch(e) {}
}

function playManualPaper(lift) {
  try {
    var ctx=getKioskAudio(),duration=lift?.38:.25,size=Math.ceil(ctx.sampleRate*duration);
    var buffer=ctx.createBuffer(1,size,ctx.sampleRate),data=buffer.getChannelData(0);
    for(var i=0;i<size;i++){var p=i/size;data[i]=(Math.random()*2-1)*Math.pow(Math.sin(Math.PI*p),1.3)*(.65+.35*Math.sin(p*31));}
    var paper=ctx.createBufferSource(),filter=ctx.createBiquadFilter(),gain=ctx.createGain();
    paper.buffer=buffer;filter.type='bandpass';filter.frequency.value=lift?1150:1800;filter.Q.value=.45;gain.gain.value=.14;
    paper.connect(filter);filter.connect(gain);gain.connect(blipOut(ctx));paper.start();
    paper.onended=function(){paper.disconnect();filter.disconnect();gain.disconnect();};
  }catch(e){}
}

// The coin that drops into the slot (kiosk.css #coin-drop-anim): a real
// element so a class toggle can animate it, injected once for every non-game
// page.
var coinDropAnim = null;
(function () {
  var btn = document.getElementById('kiosk-insert-btn');
  if (!btn) return;
  coinDropAnim = btn.querySelector('#coin-drop-anim');
  if(!coinDropAnim) {
    coinDropAnim = document.createElement('span');
    coinDropAnim.id = 'coin-drop-anim';
    coinDropAnim.setAttribute('aria-hidden', 'true');
    btn.appendChild(coinDropAnim);
  }
}());
function dropCoinAnimation() {
  if (!coinDropAnim) return;
  // Measured, like shell.js's copy: the slot is the centre of the coin plate,
  // padding-right plus half the plate, and both change across breakpoints.
  var btn = coinDropAnim.parentElement;
  var padRight = parseFloat(getComputedStyle(btn).paddingRight) || 8;
  var plateW = parseFloat(getComputedStyle(btn, '::after').width) || 18;
  coinDropAnim.style.setProperty('--slot-x', (btn.classList.contains('deck-coin-slot')?btn.offsetWidth/2:padRight+plateW/2)+'px');
  coinDropAnim.classList.remove('dropping');
  void coinDropAnim.offsetWidth;
  coinDropAnim.classList.add('dropping');
  coinDropAnim.addEventListener('animationend', function () {
    coinDropAnim.classList.remove('dropping');
  }, { once: true });
}

function playNoRoom() {
  var ctx  = getKioskAudio();
  var t    = ctx.currentTime;
  var osc  = ctx.createOscillator();
  var gain = ctx.createGain();
  osc.connect(gain);
  gain.connect(blipOut(ctx));
  osc.type = 'sawtooth';
  osc.frequency.setValueAtTime(200, t);
  osc.frequency.exponentialRampToValueAtTime(65, t + 0.38);
  gain.gain.setValueAtTime(0.32, t);
  gain.gain.exponentialRampToValueAtTime(0.001, t + 0.38);
  osc.start(t);
  osc.stop(t + 0.39);
}

function insertCoin() {
  var n = getCoins();
  if (n < MAX_COINS) {
    saveCoins(n + 1);
    playCoinInsert();
    dropCoinAnimation();
    updateCoinsHud();
    updateCoinPrompt();
    if (typeof window.onCoinInserted === 'function') window.onCoinInserted();
  } else {
    playNoRoom();
  }
}

function wireCabinetLogo() {
  document.querySelectorAll('.led-logo').forEach(function(logo){
    if(logo.querySelector('.logo-power-wire'))return;
    var wire=document.createElementNS('http://www.w3.org/2000/svg','svg');
    wire.setAttribute('class','logo-power-wire');wire.setAttribute('viewBox','0 0 32 40');wire.setAttribute('aria-hidden','true');
    wire.innerHTML='<circle cx="27" cy="11" r="4.5" fill="#211c16" stroke="#9e8764" stroke-width="1.5"/>'+
      '<path class="wire-shadow" d="M0 22 H7 C14 22 10 34 20 34 C30 34 27 23 27 11"/>'+
      '<path class="wire-jacket" d="M0 20 H7 C14 20 10 32 20 32 C30 32 27 21 27 11"/>'+
      '<path class="wire-highlight" d="M0 19 H7 C14 19 10 31 20 31 C30 31 27 20 27 11"/>'+
      '<path class="wire-current" d="M0 19 H7 C14 19 10 31 20 31 C30 31 27 20 27 11"/>'+
      '<rect x="-2" y="16" width="7" height="8" rx="1.5" fill="#40372b" stroke="#aa9270" stroke-width=".8"/>';
    logo.appendChild(wire);
    logo.querySelectorAll('.led-on').forEach(function(lamp){lamp.style.setProperty('--led-bank',3-Math.floor(Number(lamp.getAttribute('cx'))/24));});
    var display=logo.querySelector('.blip-led-display'),brand=document.createElementNS('http://www.w3.org/2000/svg','g');
    brand.setAttribute('class','led-blip');
    while(display.firstChild)brand.appendChild(display.firstChild);display.appendChild(brand);
    var prompt=document.createElementNS('http://www.w3.org/2000/svg','g');prompt.setAttribute('class','led-coin-prompt');
    var glyphs={I:['11111','00100','00100','00100','00100','00100','11111'],N:['10001','11001','11001','10101','10011','10011','10001'],S:['01111','10000','10000','01110','00001','00001','11110'],E:['11111','10000','10000','11110','10000','10000','11111'],R:['11110','10001','10001','11110','10100','10010','10001'],T:['11111','00100','00100','00100','00100','00100','00100'],C:['01111','10000','10000','10000','10000','10000','01111'],O:['01110','10001','10001','10001','10001','10001','01110']};
    ['INSERT','COIN'].forEach(function(word,line){
      var left=(100-(word.length*6-2)*2.5)/2;
      Array.from(word).forEach(function(letter,index){glyphs[letter].forEach(function(row,y){Array.from(row).forEach(function(dot,x){
        if(dot==='0')return;var lamp=document.createElementNS('http://www.w3.org/2000/svg','circle');
        lamp.setAttribute('cx',left+(index*6+x)*2.5);lamp.setAttribute('cy',3+line*16+y*1.7);lamp.setAttribute('r','.66');prompt.appendChild(lamp);
      });});});
    });display.appendChild(prompt);
  });
}
function wireCoinSlot() {
  wireCabinetLogo();
  var slot=document.getElementById('kiosk-insert-btn');
  if(slot)slot.addEventListener('click',insertCoin);
  updateCoinsHud();
}
if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',wireCoinSlot);
else wireCoinSlot();

// Mark the coin control while the landing page has no credits.
function updateCoinPrompt() {
  if (!document.querySelector('.game-grid')) return;
  var btn = document.getElementById('kiosk-insert-btn');
  if (btn) btn.classList.toggle('needs-coin', getCoins() <= 0);
}
window.addEventListener('load', updateCoinPrompt);

/* ---- BLIP logo overcharge glitch ---- Now and then one letter arcs as if
 * surged, the rest flicker, with a zap on the speaker. Every page loads
 * kiosk.js; deferred to 'load' because game pages load it in <head>. */
(function () {
  var reduceMotion = !!(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  if (reduceMotion) return; // no schedule at all — this is pure motion, nothing informational

  window.addEventListener('load', function () {
    var logo = document.querySelector('.blip-logo');
    if (!logo || !logo.firstChild || logo.firstChild.nodeType !== Node.TEXT_NODE) return;

    // One span per letter of "BLIP" (the part of the wordmark always shown)
    // so one can be struck.
    var letters = logo.firstChild.textContent.split('');
    var frag = document.createDocumentFragment();
    letters.forEach(function (ch) {
      var span = document.createElement('span');
      span.className = 'lg-ch';
      span.textContent = ch;
      frag.appendChild(span);
    });
    logo.replaceChild(frag, logo.firstChild);
    var chars = logo.querySelectorAll('.lg-ch');
    if (!chars.length) return;

    // An electrical crackle (filtered noise burst) under a fast descending
    // zap (a sawtooth sweeping from a shriek down to a thud) — the same
    // procedural-audio recipe as playCoinInsert/playNoRoom above.
    function playZap() {
      var ctx = getKioskAudio();
      var t = ctx.currentTime;

      // A crackle burst (filtered noise), sharper and louder than the
      // coin sfx's — this has to carry across a whole room, not just
      // confirm a tap.
      function crackle(start, dur, gain) {
        var buf = ctx.createBuffer(1, Math.ceil(ctx.sampleRate * dur), ctx.sampleRate);
        var data = buf.getChannelData(0);
        for (var i = 0; i < data.length; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / data.length);
        var noise = ctx.createBufferSource();
        noise.buffer = buf;
        var filt = ctx.createBiquadFilter();
        filt.type = 'highpass';
        filt.frequency.value = 2200;
        var ng = ctx.createGain();
        ng.gain.setValueAtTime(gain, start);
        ng.gain.exponentialRampToValueAtTime(0.001, start + dur);
        noise.connect(filt); filt.connect(ng); ng.connect(blipOut(ctx));
        noise.start(start); noise.stop(start + dur);
      }
      crackle(t, 0.16, 0.65);
      crackle(t + 0.22, 0.09, 0.4);   // a second, smaller sputter as it flickers back
      crackle(t + 0.36, 0.06, 0.25);

      // The main arc: a fast descending sawtooth sweep, shriek to thud.
      var osc = ctx.createOscillator();
      var og = ctx.createGain();
      osc.type = 'sawtooth';
      osc.frequency.setValueAtTime(3200, t);
      osc.frequency.exponentialRampToValueAtTime(70, t + 0.2);
      og.gain.setValueAtTime(0.3, t);
      og.gain.exponentialRampToValueAtTime(0.001, t + 0.22);
      osc.connect(og); og.connect(blipOut(ctx));
      osc.start(t); osc.stop(t + 0.23);

      // A low undertone thump, like the sign's transformer took the hit.
      var thump = ctx.createOscillator();
      var tg = ctx.createGain();
      thump.type = 'triangle';
      thump.frequency.setValueAtTime(110, t);
      thump.frequency.exponentialRampToValueAtTime(45, t + 0.3);
      tg.gain.setValueAtTime(0.28, t + 0.02);
      tg.gain.exponentialRampToValueAtTime(0.001, t + 0.32);
      thump.connect(tg); tg.connect(blipOut(ctx));
      thump.start(t + 0.02); thump.stop(t + 0.33);
    }

    function zap(forced) {
      if (document.hidden && !forced) return;
      var el = chars[Math.floor(Math.random() * chars.length)];
      logo.classList.add('lg-surge');
      el.classList.add('lg-zap');
      playZap();
      var done = function () {
        el.classList.remove('lg-zap');
        logo.classList.remove('lg-surge');
      };
      el.addEventListener('animationend', done, { once: true });
      setTimeout(done, 900); // in case the tab was hidden mid-animation and it never fired
    }

    // The next strike is 45s-3min out, re-armed after each one (and while
    // hidden) rather than on an interval, so a backgrounded tab does not fire
    // a burst of overdue zaps.
    function scheduleNext() {
      var delay = 45000 + Math.random() * 135000;
      setTimeout(function () {
        if (document.hidden) { scheduleNext(); return; }
        zap();
        scheduleNext();
      }, delay);
    }
    scheduleNext();

    // Manual trigger for testing/demoing, since the real thing is 45s-3min
    // apart by design: run `blipZapLogo()` in the console to fire one now.
    window.blipZapLogo = function () { zap(true); };
  });
}());

/* Poll the first gamepad and report logical button edges to games or the cabinet picker. */
function pollGamepad(onDown, onUp) {
  if (!navigator.getGamepads) return;

  var DEADZONE = 0.25;
  var held = {};

  var BTN_MAP = [
    { idx: 0,  code: 'Space'      },  // A / Cross   — Button 1
    { idx: 1,  code: 'KeyZ'       },  // B / Circle  — Button 2
    { idx: 2,  code: 'KeyZ'       },  // X / Square  — Button 2
    { idx: 3,  code: 'Space'      },  // Y / Triangle— Button 1
    { idx: 4,  code: 'KeyZ'       },  // L shoulder  — Button 2
    { idx: 5,  code: 'KeyZ'       },  // R shoulder  — Button 2
    { idx: 8,  code: 'Select'     },  // Select / Back — arcade menu
    { idx: 9,  code: 'Start'      },  // Start / Options — start the game
    { idx: 12, code: 'ArrowUp'    },
    { idx: 13, code: 'ArrowDown'  },
    { idx: 14, code: 'ArrowLeft'  },
    { idx: 15, code: 'ArrowRight' },
  ];

  function findPad() {
    var pads = navigator.getGamepads();
    for (var i = 0; i < pads.length; i++) {
      if (pads[i] && pads[i].connected) return pads[i];
    }
    return null;
  }

  // Poll continuously because some Linux gamepads omit `gamepadconnected`; lamp dims after `IDLE_MS`.
  var IDLE_MS = 10000;
  var root = document.documentElement;
  var lastInput = -Infinity, lamp = '';
  function setLamp(state) {
    if (state === lamp) return;
    lamp = state;
    if (state) root.setAttribute('data-pad', state);
    else root.removeAttribute('data-pad');
  }

  function tick() {
    var pad = findPad();
    if (!pad) setLamp('');
    if (pad) {
      var want = {};
      for (var j = 0; j < BTN_MAP.length; j++) {
        var m = BTN_MAP[j];
        var b = pad.buttons[m.idx];
        if (b && (b.pressed || b.value > 0.5)) want[m.code] = true;
      }
      var ax = pad.axes[0] || 0, ay = pad.axes[1] || 0;
      if (ax < -DEADZONE) want['ArrowLeft']  = true;
      if (ax >  DEADZONE) want['ArrowRight'] = true;
      if (ay < -DEADZONE) want['ArrowUp']    = true;
      if (ay >  DEADZONE) want['ArrowDown']  = true;

      var now = performance.now();
      for (var any in want) { lastInput = now; break; }
      setLamp(now - lastInput < IDLE_MS ? 'active' : 'idle');
      var code;
      for (code in want) {
        if (!held[code]) { held[code] = true; onDown(code); }
      }
      for (code in held) {
        if (held[code] && !want[code]) { held[code] = false; onUp(code); }
      }
    }
    requestAnimationFrame(tick);
  }

  requestAnimationFrame(tick);
}

// iOS suspends AudioContext on load and re-suspends after backgrounding.
// Howler's autoUnlock is disabled (it calls unload() on non-44100 Hz devices, destroying
// all WASM sounds), so we resume Howler.ctx manually on any gesture and on tab refocus.
(function () {
  function unlockAudio() {
    if (typeof Howler === 'undefined') return;
    if (!Howler.ctx) {
      // Force Howler to create its AudioContext now, while inside a user gesture.
      // On iOS, a context created outside a gesture starts suspended; inside one it starts running.
      // Howler.volume() triggers _setupAudioContext() internally via `ctx || _()`.
      Howler.volume();
    }
    if (Howler.ctx && Howler.ctx.state !== 'running') {
      Howler.ctx.resume();
    }
  }
  // iOS counts a touch's end, not its start, as the gesture that may start sound,
  // and the touch strip suppresses the click that would follow.
  ['touchstart', 'touchend', 'pointerup'].forEach(function (type) {
    document.addEventListener(type, unlockAudio, { passive: true, capture: true });
  });
  document.addEventListener('click',      unlockAudio, { capture: true });
  document.addEventListener('visibilitychange', function () {
    if (!document.hidden) unlockAudio();
  });
}());

/* The deck itself is built by deck.js, the same on every page. */

/* The side badges and flags sit 16px above the page's deck, whose height
   changes with the controller, layout and player count, or 16px off the
   screen's edge when there is none (a touch screen's front page). */
(function () {
  var GAP = 16;
  function place() {
    var top = Infinity;
    document.querySelectorAll('.kiosk-bar').forEach(function (bar) {
      var r = bar.getBoundingClientRect();
      if (r.height > 0 && r.top < top) top = r.top;
    });
    var v = top === Infinity
      ? 'calc(' + GAP + 'px + env(safe-area-inset-bottom, 0px))'
      : Math.round(window.innerHeight - top + GAP) + 'px';
    document.documentElement.style.setProperty('--side-badges-bottom', v);
  }
  window.addEventListener('resize', place);
  document.addEventListener('DOMContentLoaded', function () {
    place();
    if (typeof ResizeObserver === 'function') {
      document.querySelectorAll('.kiosk-bar').forEach(function (bar) { new ResizeObserver(place).observe(bar); });
    }
  });
}());

/* Restore the saved fullscreen layout after navigation. */
(function () {
  var root = document.documentElement;
  var KEY = 'blip-fullscreen';
  var enter = root.requestFullscreen || root.webkitRequestFullscreen;
  function wanted() { try { return localStorage.getItem(KEY) === '1'; } catch (e) { return false; } }
  function inBrowserFullscreen() {
    return !!(document.fullscreenElement || document.webkitFullscreenElement);
  }
  function request() {
    if (!enter || inBrowserFullscreen()) return;
    try {
      var p = enter.call(root);
      // Refused (an embedded page, no gesture yet): the layout still holds.
      if (p && p.catch) p.catch(function () {});
    } catch (e) {}
  }
  // In fullscreen a mouse left lying loses its pointer after three seconds.
  var cursorT = null;
  function wakeCursor() {
    root.removeAttribute('data-cursor-idle');
    clearTimeout(cursorT);
    if (!root.hasAttribute('data-fullscreen')) return;
    cursorT = setTimeout(function () { root.setAttribute('data-cursor-idle', ''); }, 3000);
  }
  ['mousemove', 'mousedown'].forEach(function (ev) {
    window.addEventListener(ev, wakeCursor, { passive: true });
  });

  function apply(on) {
    root.toggleAttribute('data-fullscreen', on);
    wakeCursor();
    if (typeof window.onBlipFullscreenChange === 'function') {
      try { window.onBlipFullscreenChange(on); } catch (e) {}
    }
  }
  function set(on) {
    try { localStorage.setItem(KEY, on ? '1' : '0'); } catch (e) {}
    apply(on);
  }
  window.blipSetFullscreen = set;
  // Before first paint where this script is in <head>.
  if (wanted()) root.setAttribute('data-fullscreen', '');

  function build() {
    window.addEventListener('keydown', function (e) {
      if ((e.key !== 'm' && e.key !== 'M') || e.repeat || e.ctrlKey || e.metaKey || e.altKey) return;
      if (e.target && e.target.closest && e.target.closest('input, textarea')) return;
      if (root.hasAttribute('data-cabinet-screen') && window.parent !== window) {
        try {
          window.parent.blipSetMuted(!window.parent.blipMuted());
          return;
        } catch (e) {}
      }
      blipSetMuted(!blipMuted());
    }, true);
    apply(wanted());
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', build);
  else build();

  // Esc, or the browser's own way out, turns the mode off. Leaving the page
  // drops real fullscreen too, and that must not count: the choice has to
  // survive into the next page.
  var leaving = false;
  ['beforeunload', 'pagehide'].forEach(function (ev) {
    window.addEventListener(ev, function () { leaving = true; });
  });
  ['fullscreenchange', 'webkitfullscreenchange'].forEach(function (ev) {
    document.addEventListener(ev, function () {
      if (leaving) return;
      if (!inBrowserFullscreen() && root.hasAttribute('data-fullscreen')) set(false);
    });
  });
  // Back in real fullscreen on the first key or click of a page.
  ['keydown', 'mousedown', 'pointerup', 'touchend'].forEach(function (ev) {
    window.addEventListener(ev, function (e) {
      if (!wanted() || inBrowserFullscreen()) return;
      if (e.key === 'Escape') return;
      request();
    }, true);
  });
}());

// The telescoping boom keeps its level clamp clear of the receiver.
window.blipTransferCard = function (options) {
  var stage = document.querySelector('.jukebox-transfer.idle') || document.createElement('div');
  stage.className = 'jukebox-transfer active' + (options.idle ? ' idle' : '');
  stage.setAttribute('aria-hidden', 'true');
  var cabinet=document.querySelector('.screen-bezel');
  var cabinetBox=cabinet&&cabinet.getBoundingClientRect();
  if(cabinetBox) {
    var fascia=document.querySelector('.top-marquee-bar, #marquee-bar'),fasciaBottom=fascia?fascia.getBoundingClientRect().bottom:40;
    stage.style.clipPath='inset('+Math.max(0,fasciaBottom)+'px '+Math.max(0,innerWidth-cabinetBox.right)+'px '+
      Math.max(0,innerHeight-cabinetBox.bottom)+'px '+Math.max(0,cabinetBox.left)+'px)';
  }
  stage.innerHTML = '<div class="jukebox-mechanics-bar"></div><div class="jukebox-transfer-arm articulated"><i class="jukebox-arm-anchor"></i><i class="jukebox-arm-mount"></i><i class="jukebox-arm-shaft"></i><i class="jukebox-arm-forearm"></i><i class="jukebox-arm-pivot"></i><div class="jukebox-arm-wrist"><div class="jukebox-flight-card"><img alt=""><strong></strong><small></small></div><i class="jukebox-arm-grip"></i></div></div>';
  var returning=!!options.returning;
  stage.dataset.cycle=returning?'return':'insert';
  var totalTime=returning?12300:14700;
  var arm = stage.querySelector('.jukebox-transfer-arm'), card = arm.querySelector('.jukebox-flight-card');
  card.querySelector('img').src = options.art;
  card.querySelector('strong').textContent = options.name;
  card.querySelector('small').textContent = options.code;
  var width = options.width, height = options.height, scale = options.scale || 1;
  if (options.original) {
    var copy = options.original.cloneNode(true);
    var originals = [options.original].concat(Array.from(options.original.querySelectorAll('*')));
    var copies = [copy].concat(Array.from(copy.querySelectorAll('*')));
    originals.forEach(function (original,index) {
      var styles=getComputedStyle(original);
      Array.from(styles).forEach(function (key) { copies[index].style.setProperty(key,styles.getPropertyValue(key)); });
    });
    copies.slice(1).forEach(function(part){part.style.visibility='visible';});
    copy.className='jukebox-flight-card'; copy.removeAttribute('href');
    copy.style.minWidth='0'; copy.style.minHeight='0'; copy.style.maxWidth='none'; copy.style.maxHeight='none';
    copy.style.position='absolute'; copy.style.margin='0'; copy.style.transition='none';
    copy.style.left='0'; copy.style.top='0'; copy.style.zIndex='5';
    copy.style.transformOrigin='50% 100%';
    copy.style.transform='translate(-50%,-100%) scale('+scale+')';
    card.replaceWith(copy); card=copy;
  }
  var cardOpacity=parseFloat(card.style.opacity || '1');
  var physicalHeight=height*scale,physicalWidth=width*scale;
  var exposedEdge=Math.min(18,physicalHeight*.2),feedTravel=physicalHeight-exposedEdge;
  card.style.visibility='hidden';
  card.style.width = width + 'px'; card.style.height = height + 'px';
  var pivot = options.pivot, source = options.source, slot = options.slot;
  var initialAngle=options.angle || 0, radians=initialAngle*Math.PI/180;
  var start = options.idle ? {x:pivot.x-58,y:pivot.y} :
    {x:source.x-Math.sin(radians)*physicalHeight/2, y:source.y+Math.cos(radians)*physicalHeight/2};
  var approach = {x:slot.x, y:slot.y + physicalHeight};
  var park = {x:pivot.x-58,y:pivot.y};
  var feedStart=returning?2100:7100;
  var gripDrop=returning?-exposedEdge-6:14-physicalHeight;
  var sourceGrip={x:start.x-Math.sin(radians)*gripDrop,y:start.y+Math.cos(radians)*gripDrop};
  var pickupGrip=returning?{x:slot.x,y:slot.y+exposedEdge+gripDrop}:sourceGrip;
  var toolSide=physicalWidth/2+38,bridge=toolSide-20;
  var restingRom=null;
  if(options.original&&!options.idle) {
    restingRom=card.cloneNode(true);restingRom.className='jukebox-resting-rom';
    restingRom.style.position='fixed';restingRom.style.zIndex='21';
    restingRom.style.left=start.x+'px';restingRom.style.top=start.y+'px';
    restingRom.style.transform='translate(-50%,-100%) rotate('+initialAngle+'deg) scale('+scale+')';
    restingRom.style.visibility='visible';restingRom.style.pointerEvents='none';
    arm.appendChild(restingRom);
  }
  var shaft = arm.querySelector('.jukebox-arm-shaft'), forearm = arm.querySelector('.jukebox-arm-forearm');
  var wrist = arm.querySelector('.jukebox-arm-wrist');
  wrist.dataset.depth='12';
  var grip=arm.querySelector('.jukebox-arm-grip');arm.appendChild(grip);
  grip.innerHTML='<i class="jukebox-claw-rail"></i><i class="jukebox-claw-jaw left"><b></b></i><i class="jukebox-claw-jaw right"><b></b></i><i class="jukebox-claw-lock"></i>';
  grip.style.setProperty('--rom-half',(physicalWidth/2)+'px');
  var tool=document.createElement('i');tool.className='jukebox-arm-tool';arm.appendChild(tool);
  var bearing=document.createElement('i');bearing.className='jukebox-arm-bearing';arm.appendChild(bearing);
  var shoulderLift=document.createElement('i');shoulderLift.className='jukebox-joint-lift shoulder';arm.appendChild(shoulderLift);
  shoulderLift.style.left=pivot.x+'px';shoulderLift.style.top=pivot.y+'px';
  var mechanicsBar=stage.querySelector('.jukebox-mechanics-bar');
  var rotaryPivot=arm.querySelector('.jukebox-arm-pivot'),rotor={x:pivot.x,y:pivot.y};
  rotaryPivot.dataset.joint='shoulder';arm.dataset.drive='telescopic';
  wrist.dataset.layer='rom';
  arm.querySelector('.jukebox-arm-mount').dataset.layer='chassis';
  tool.dataset.layer='gripper';
  [arm.querySelector('.jukebox-arm-mount'), arm.querySelector('.jukebox-arm-pivot')].forEach(function (part) {
    part.style.left = pivot.x + 'px'; part.style.top = pivot.y + 'px';
  });
  var anchor=arm.querySelector('.jukebox-arm-anchor'),guide=document.querySelector('.rolodex-guide.right');
  anchor.style.left=pivot.x+'px';anchor.style.top=pivot.y+'px';
  anchor.style.width=(guide?Math.max(30,guide.getBoundingClientRect().left+5-pivot.x):40)+'px';
  function ease(t) { return t*t*t*(10+t*(-15+6*t)); }
  function mix(a,b,t) { return a+(b-a)*t; }
  function link(el,a,b,length) {
    el.style.left = a.x + 'px'; el.style.top = a.y + 'px';
    el.style.width = length + 'px';
    el.style.transform = 'rotate(' + Math.atan2(b.y-a.y,b.x-a.x) + 'rad)';
  }
  var picked = false, seated = false, lifted = false, extracted = false, drive = null, guideClicks = 0;
  var jointSound=null,loadSound=null,lastPose=null,soundEvents={};
  function stopSound() {
    if(jointSound){jointSound.stop();jointSound=null;}
    if(loadSound){loadSound.stop();loadSound=null;}
    if(drive){drive.stop();drive=null;}
  }
  function drawer(stow) {
    arm.style.transform='translateY('+(-60*stow)+'px)';
    mechanicsBar.style.transform='translateY('+(-60*stow)+'px)';
  }
  function render(ms) {
    var x=start.x, y=start.y, roll=options.idle?0:initialAngle, feed=0;
    if (ms>=2100 && ms<3100) {
      y=start.y-12*ease((ms-2100)/1000);
    } else if (ms >= 3100 && ms < 7100) {
      var t = ease((ms-3100)/4000);
      x=mix(start.x,approach.x,t); y=mix(start.y-12,approach.y,t)-Math.sin(t*Math.PI)*8;
      roll=mix(initialAngle,0,t)+Math.sin(t*Math.PI)*-8;
    } else if (ms >= 7100) {
      feed=ease(Math.min(1,(ms-7100)/2500));
      x=slot.x; y=approach.y-feedTravel*feed; roll=0;
    }
    if(!paused && !options.idle) {
      if(ms>=feedStart && ms<feedStart+2500) {
        if(!drive) drive=blipCardDrive();
        if(drive) drive.update((ms-feedStart)/2500);
        var clicks=Math.floor((ms-feedStart)/700);
        if(clicks>guideClicks) {guideClicks=clicks;playCardLatch(true);}
      } else if(drive) {drive.stop();drive=null;}
    }
    if(returning) {
      roll=0; feed=1;
      if(ms<2100) {x=slot.x;y=slot.y+exposedEdge;}
      else if(ms<4600) {
        feed=1-ease((ms-2100)/2500);x=slot.x;y=slot.y+exposedEdge+feedTravel*(1-feed);
      } else if(ms<8600) {
        var home=ease((ms-4600)/4000); feed=0;
        x=mix(approach.x,start.x,home);y=mix(approach.y,start.y-12,home)-Math.sin(home*Math.PI)*8;
        roll=mix(0,initialAngle,home)+Math.sin(home*Math.PI)*-8;
      } else if(ms<9800) {
        feed=0;x=start.x;y=start.y-12*(1-ease(Math.min(1,(ms-8600)/1000)));roll=initialAngle;
      } else {
        feed=0;roll=0;
      }
    }
    var toolAngle=roll*Math.PI/180;
    var carrierY=y+(returning?0:feedTravel*feed);
    var withdraw=returning?0:ease(Math.max(0,Math.min(1,(ms-7400)/800)))*
      (1-ease(Math.max(0,Math.min(1,(ms-10200)/2500))));
    var carrierX=x+12*withdraw;carrierY+=32*withdraw;
    var gripPoint={x:carrierX-Math.sin(toolAngle)*gripDrop,y:carrierY+Math.cos(toolAngle)*gripDrop};
    if(ms<1500) {
      if(ms<900) {
        var reach=ease(ms/900);
        gripPoint={x:mix(park.x,pickupGrip.x,reach),y:mix(park.y,pickupGrip.y-36,reach)};
      } else gripPoint={x:pickupGrip.x,y:mix(pickupGrip.y-36,pickupGrip.y,ease((ms-900)/600))};
    }
    if(returning&&ms>=9800) {
      if(ms<10700) {
        var rise=ease((ms-9800)/900);
        gripPoint={x:sourceGrip.x,y:sourceGrip.y-36*rise};
        roll=mix(initialAngle,0,rise);toolAngle=roll*Math.PI/180;
      } else {
        var retreat=ease(Math.min(1,(ms-10700)/1600));
        gripPoint={x:mix(sourceGrip.x,park.x,retreat),y:mix(sourceGrip.y-36,park.y,retreat)};
      }
    } else if(!returning&&ms>=10200) {
      var retreat=ease(Math.min(1,(ms-10200)/2500));
      gripPoint={x:mix(slot.x+12,park.x,retreat),y:mix(slot.y+46,park.y,retreat)};
    }
    if(options.idle) gripPoint={x:park.x,y:park.y};
    var toolPoint={x:gripPoint.x+Math.cos(toolAngle)*toolSide,y:gripPoint.y+Math.sin(toolAngle)*toolSide};
    var toolStart={x:gripPoint.x+Math.cos(toolAngle)*bridge,y:gripPoint.y+Math.sin(toolAngle)*bridge};
    var elevation=ms<1500?0:ms<2100?ease((ms-1500)/600):
      1-ease(Math.max(0,Math.min(1,(ms-(returning?9800:10200))/2500)));
    if(options.idle)elevation=0;
    // Elevation changes depth; the shoulder stays centred in its chassis bay.
    arm.dataset.elevation=elevation.toFixed(3);
    arm.dataset.depth=(16+12*elevation).toFixed(2);
    var distance=Math.hypot(toolPoint.x-rotor.x,toolPoint.y-rotor.y),shoulderAngle=Math.atan2(toolPoint.y-rotor.y,toolPoint.x-rotor.x);
    var housing=Math.min(52,distance),overlap=Math.min(14,housing),slide=Math.max(0,housing-overlap);
    var carriage={x:rotor.x+Math.cos(shoulderAngle)*slide,y:rotor.y+Math.sin(shoulderAngle)*slide};
    link(shaft,rotor,toolPoint,housing);link(forearm,carriage,toolPoint,distance-slide);
    shoulderLift.style.setProperty('--joint-rise',(12*elevation)+'px');
    // A 20px standoff keeps the wrist outside the open jaw's sweep.
    link(tool,toolStart,toolPoint,20);
    bearing.style.left=toolPoint.x+'px';bearing.style.top=toolPoint.y+'px';
    bearing.style.setProperty('--bearing-angle',roll+'deg');
    rotaryPivot.style.setProperty('--bearing-angle',shoulderAngle+'rad');
    var jawClose=ease(Math.max(0,Math.min(1,(ms-1500)/600)))*
      (1-ease(Math.max(0,Math.min(1,(ms-(returning?9300:7100))/300))));
    grip.style.setProperty('--jaw-travel',(12*jawClose)+'px');
    grip.style.setProperty('--jaw-close',jawClose);
    grip.dataset.holding=String(jawClose>.999&&(returning?ms>=2100&&ms<9300:ms>=2100&&ms<=7100));
    var gripDepth=16+12*elevation-16*jawClose;
    grip.dataset.depth=gripDepth.toFixed(2);
    grip.style.filter='drop-shadow(0 '+Math.max(0,(gripDepth-12)*.4)+'px 2px #000a)';
    if(!paused&&!options.idle) {
      if(!jointSound)jointSound=blipArmJointSound();
      if(lastPose&&ms>lastPose.ms&&jointSound) {
        var dt=(ms-lastPose.ms)/1000;
        function angularSpeed(a,b){return Math.abs(Math.atan2(Math.sin(a-b),Math.cos(a-b)))/dt;}
        jointSound.update(angularSpeed(shoulderAngle,lastPose.shoulder),Math.abs(distance-lastPose.reach)/(70*dt),Math.abs(elevation-lastPose.elevation)/dt);
      }
      lastPose={ms:ms,shoulder:shoulderAngle,reach:distance,elevation:elevation};
      [[1500,'valve'],[2100,'attach'],[returning?4600:9600,returning?'unseat':'seat'],[returning?9600:7400,'release'],[returning?12300:12700,'vent']].forEach(function(event){
        if(ms>=event[0]&&!soundEvents[event[1]]){soundEvents[event[1]]=true;playRomCoupling(event[1]);}
      });
    }
    wrist.style.left=x+'px'; wrist.style.top=y+'px'; wrist.style.transform='rotate('+roll+'deg)';
    var clearance=returning?1-ease(Math.max(0,Math.min(1,(ms-8600)/1000))):ease(Math.max(0,Math.min(1,(ms-2100)/1000)));
    wrist.style.filter='drop-shadow(0 '+(9*clearance)+'px '+(4*clearance)+'px #000b)';
    grip.style.left=(gripPoint.x-16)+'px';grip.style.top=(gripPoint.y-11)+'px';grip.style.transform='rotate('+roll+'deg)';
    card.style.clipPath='inset('+(feed*feedTravel/physicalHeight*100)+'% 0 0 0)';
    if(returning&&ms>=2100)options.receiver.classList.remove('loaded');
    card.style.visibility=ms>=2100 && ms<9600?'visible':'hidden';
    card.style.opacity=ms>=2100 && ms<9600?cardOpacity:0;
    card.classList.toggle('seated',ms>=9600);
    if(restingRom)restingRom.style.display=(returning?ms>=9600:ms<2100)?'flex':'none';
    arm.classList.toggle('clamped',ms>=1500 && ms<(returning?9600:9800));
    if(ms>=2100 && !picked) {
      picked=true; card.style.visibility='visible';
      if(!returning && options.onPickup) options.onPickup();
    }
    var stow=returning?0:ease(Math.max(0,Math.min(1,(ms-12700)/2000)));
    drawer(stow);
    if(ms>=14700) options.receiver.classList.add('stowed');
    if(returning && ms>=4600 && !extracted) {extracted=true;options.receiver.classList.remove('loaded');if(options.onExtract) options.onExtract();}
    if(!returning && ms>=3100 && !lifted) {lifted=true;if(options.onLift) options.onLift();}
    stage.dataset.phase = ms<2100?'grip':ms<3100?'lift':ms<7100?'swing':ms<9600?'feed':ms<10200?'release':ms<12700?'return':'stow';
    if (ms >= 9600 && !seated) {
      seated=true; card.classList.add('seated');
      if(returning)options.receiver.classList.remove('loaded');
      else {
        blipSeatRom(options.receiver,options);
        if(!paused&&!options.idle)loadSound=playRomLoading(Math.min(4.8,(totalTime-ms)/1000-.1));
      }
      if(returning && options.onReturned) options.onReturned();
      else if (options.onSeat) options.onSeat();
    }
  }
  document.body.appendChild(stage);
  var paused=false;
  render(0);
  var began=performance.now();
  // A deterministic pose is useful when inspecting the joints in screenshots.
  stage.blipSeek=function(ms) { paused=true; stopSound(); render(ms); };
  function frame(now) {
    if (paused) return;
    var ms=now-began; render(ms);
    if(ms<totalTime) requestAnimationFrame(frame);
    else { stopSound();stage.remove(); if(options.onDone) options.onDone(); }
  }
  if (!options.idle && typeof playCardMechanism==='function') playCardMechanism();
  if(options.idle) {
    paused=true; options.receiver.classList.remove('stowed');
    if(options.deploy) {
      stage.dataset.deploying='true'; drawer(1);
      var deployStart=performance.now();
      function deploy(now) {
        var t=Math.min(1,(now-deployStart)/2000); drawer(1-ease(t));
        if(t<1) requestAnimationFrame(deploy);
        else { delete stage.dataset.deploying; stage.dispatchEvent(new Event('blip-deployed')); }
      }
      requestAnimationFrame(deploy);
    }
  } else requestAnimationFrame(frame);
  return stage;
};

function blipCardReceiver(bar) {
  var receiver=bar.querySelector('.jukebox-receiver');
  if (!receiver) {
    var game=blipGameFromPath(location.pathname),interactive=game&&!document.documentElement.hasAttribute('data-cabinet-screen');
    receiver=document.createElement(interactive?'a':'div'); receiver.className='jukebox-receiver';
    if(interactive){receiver.href='../index.html';receiver.setAttribute('aria-label','Return to the cabinet');receiver.title='Return to the cabinet';}
    else receiver.setAttribute('aria-hidden','true');
    receiver.innerHTML='<div class="jukebox-receiver-lip"><i></i><i></i></div><div class="jukebox-seated-card"><i></i><i></i></div>';
    bar.appendChild(receiver);
    try {
      var loaded=JSON.parse(sessionStorage.getItem('blip-loaded-card')||'null');
      if(loaded)blipSeatRom(receiver,loaded);
    }catch(e){}
  }
  return receiver;
}

function blipSeatRom(receiver,rom) {
  receiver.dataset.rom=rom.slug||rom.name||'';
  receiver.classList.add('loaded');
}

function blipCardArmPivot(receiver) {
  var box=receiver.getBoundingClientRect(),bar=receiver.closest('.top-marquee-bar, #marquee-bar');
  var top=bar?bar.getBoundingClientRect().bottom:box.top;
  var guide=document.querySelector('.rolodex-guide.right'),center=box.left+box.width/2;
  var cabinetWidth=Math.min(960,document.documentElement.clientWidth-32);
  var x=guide?guide.getBoundingClientRect().left-30:center+Math.min(205,(cabinetWidth-26)/2)-40;
  var layerHeight=parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--mechanics-layer-height'))||54;
  return {x:x,y:top+layerHeight/2};
}

function blipParkMechanism(track, deploy) {
  var bar=document.querySelector('.top-marquee-bar');
  if (!bar) return null;
  var receiver=blipCardReceiver(bar), housing=receiver.getBoundingClientRect(), barBox=bar.getBoundingClientRect();
  var slot=receiver.querySelector('.jukebox-receiver-lip').getBoundingClientRect();
  var area=track ? track.getBoundingClientRect() : {left:0,width:innerWidth,top:110,bottom:440};
  return blipTransferCard({
    idle:true,deploy:!!deploy,name:'',code:'',art:'',width:134,height:258,
    reach:Math.hypot(housing.right+34-(area.left+area.width/2),area.bottom-housing.top)+60,
    source:{x:area.left+area.width/2,y:area.top+150},
    pivot:blipCardArmPivot(receiver),
    slot:{x:slot.left+slot.width/2,y:slot.top+slot.height/2},receiver:receiver
  });
}

function blipBuildGuideMechanism() {
  if (!/\/(about|controls|history|api)\.html$/.test(location.pathname)) return;
  document.documentElement.setAttribute('data-cabinet-mechanism','');
  var deploy=!!document.referrer && new URL(document.referrer).origin===location.origin;
  function park() {
    if(document.querySelector('.jukebox-transfer[data-deploying]')) return;
    blipParkMechanism(null,deploy); deploy=false;
  }
  requestAnimationFrame(park); window.addEventListener('resize',park);
}
if(document.readyState==='loading') document.addEventListener('DOMContentLoaded',blipBuildGuideMechanism);
else blipBuildGuideMechanism();

function blipArmJointSound() {
  try {
    var ctx=getKioskAudio(),output=ctx.createGain(),nodes=[],sources=[],motors=[];
    output.gain.value=1.5;output.connect(blipOut(ctx));nodes.push(output);
    [79,113].forEach(function(base){
      var motor=ctx.createOscillator(),tone=ctx.createBiquadFilter(),volume=ctx.createGain();
      motor.type='sawtooth';motor.frequency.value=base;
      tone.type='lowpass';tone.frequency.value=640;tone.Q.value=1.2;
      volume.gain.value=0;motor.connect(tone);tone.connect(volume);volume.connect(output);
      var cog=ctx.createOscillator(),teeth=ctx.createGain();
      cog.type='triangle';cog.frequency.value=19;teeth.gain.value=0;
      cog.connect(teeth);teeth.connect(volume.gain);
      motor.start();cog.start();sources.push(motor,cog);nodes.push(tone,volume,teeth);
      motors.push({motor: motor,tone:tone,volume:volume,cog:cog,teeth:teeth,base:base});
    });
    var buffer=ctx.createBuffer(1,ctx.sampleRate,ctx.sampleRate),data=buffer.getChannelData(0);
    for(var i=0;i<data.length;i++)data[i]=Math.random()*2-1;
    var air=ctx.createBufferSource(),valve=ctx.createBiquadFilter(),pressure=ctx.createGain();
    air.buffer=buffer;air.loop=true;valve.type='bandpass';valve.frequency.value=2400;valve.Q.value=.65;
    pressure.gain.value=0;air.connect(valve);valve.connect(pressure);pressure.connect(output);
    air.start();sources.push(air);nodes.push(valve,pressure);
    var stopped=false,timer=setTimeout(stop,18000);
    function stop(){
      if(stopped)return;stopped=true;clearTimeout(timer);window.removeEventListener('pagehide',stop);
      output.gain.setTargetAtTime(0,ctx.currentTime,.015);
      sources.forEach(function(source){source.stop(ctx.currentTime+.1);});
      setTimeout(function(){sources.concat(nodes).forEach(function(node){node.disconnect();});},150);
    }
    window.addEventListener('pagehide',stop,{once:true});
    return {stop:stop,update:function(shoulder,elbow,lift){
      if(stopped)return;
      var t=ctx.currentTime;
      [shoulder,elbow].forEach(function(speed,index){
        var voice=motors[index],motion=Math.min(1,speed/.9);
        voice.motor.frequency.setTargetAtTime(voice.base+motion*125,t,.04);
        voice.tone.frequency.setTargetAtTime(400+motion*1100,t,.04);
        voice.volume.gain.setTargetAtTime(motion*.036,t,.025);
        voice.teeth.gain.setTargetAtTime(motion*.012,t,.025);
        voice.cog.frequency.setTargetAtTime(16+motion*67,t,.04);
      });
      var flow=Math.min(1,lift/2.2);
      pressure.gain.setTargetAtTime(flow*.11,t,.025);
      valve.frequency.setTargetAtTime(1600+flow*2100,t,.04);
    }};
  }catch(e){return null;}
}

function playRomLoading(duration) {
  if(duration<=1.2)return null;
  try {
    var ctx=getKioskAudio(),start=ctx.currentTime+.1,end=start+duration,nodes=[],sources=[];
    var output=ctx.createGain(),tone=ctx.createBiquadFilter();
    output.gain.setValueAtTime(.0001,start);output.gain.exponentialRampToValueAtTime(.19,start+.18);
    output.gain.setValueAtTime(.19,end-.95);output.gain.exponentialRampToValueAtTime(.0001,end);
    tone.type='lowpass';tone.frequency.value=1900;tone.Q.value=.55;
    tone.connect(output);output.connect(blipOut(ctx));nodes.push(tone,output);
    var wave=ctx.createPeriodicWave(new Float32Array(6),new Float32Array([0,1,.36,.2,.12,.07]));
    var motor=ctx.createOscillator();motor.setPeriodicWave(wave);
    motor.frequency.setValueAtTime(90,start);motor.frequency.exponentialRampToValueAtTime(260,start+.7);
    motor.frequency.linearRampToValueAtTime(235,end-.95);motor.frequency.exponentialRampToValueAtTime(65,end);
    motor.connect(tone);sources.push(motor);
    var rotor=ctx.createOscillator(),ripple=ctx.createGain();rotor.type='sine';
    rotor.frequency.setValueAtTime(12,start);rotor.frequency.linearRampToValueAtTime(34,start+.7);
    rotor.frequency.linearRampToValueAtTime(9,end);ripple.gain.value=7;
    rotor.connect(ripple);ripple.connect(motor.frequency);sources.push(rotor);nodes.push(ripple);
    var buffer=ctx.createBuffer(1,ctx.sampleRate,ctx.sampleRate),data=buffer.getChannelData(0);
    for(var i=0;i<data.length;i++)data[i]=Math.random()*2-1;
    var air=ctx.createBufferSource(),filter=ctx.createBiquadFilter(),breath=ctx.createGain();
    air.buffer=buffer;air.loop=true;filter.type='bandpass';filter.frequency.value=2100;filter.Q.value=.6;breath.gain.value=.18;
    air.connect(filter);filter.connect(breath);breath.connect(output);sources.push(air);nodes.push(filter,breath);
    sources.forEach(function(source){source.start(start);source.stop(end+.02);});
    var stopped=false,timer=setTimeout(stop,(duration+.2)*1000);
    function stop(){
      if(stopped)return;stopped=true;clearTimeout(timer);window.removeEventListener('pagehide',stop);
      output.gain.cancelScheduledValues(ctx.currentTime);output.gain.setTargetAtTime(.0001,ctx.currentTime,.025);
      sources.forEach(function(source){try{source.stop(ctx.currentTime+.1);}catch(e){}});
      setTimeout(function(){sources.concat(nodes).forEach(function(node){node.disconnect();});},150);
    }
    window.addEventListener('pagehide',stop,{once:true});return {stop:stop};
  }catch(e){return null;}
}

function playRomCoupling(kind) {
  try {
    var ctx=getKioskAudio(),now=ctx.currentTime,isValve=kind==='valve'||kind==='vent';
    var offsets=isValve?[0]:kind==='attach'?[0,.045,.095]:kind==='release'?[0,.055]:[0,.025,.08];
    offsets.forEach(function(offset,index){
      var t=now+offset,duration=isValve?.22:.12;
      var buffer=ctx.createBuffer(1,Math.ceil(ctx.sampleRate*duration),ctx.sampleRate),data=buffer.getChannelData(0);
      for(var i=0;i<data.length;i++)data[i]=(Math.random()*2-1)*Math.exp(-i/(ctx.sampleRate*(isValve?.05:.008)));
      var noise=ctx.createBufferSource(),filter=ctx.createBiquadFilter(),snap=ctx.createGain();
      noise.buffer=buffer;filter.type='bandpass';filter.frequency.value=isValve?3300:kind==='release'?2100:1250+index*480;filter.Q.value=.8;
      snap.gain.value=isValve?.22:(kind==='attach'?.42:.23)/(1+index*.3);
      noise.connect(filter);filter.connect(snap);snap.connect(blipOut(ctx));noise.start(t);
      noise.onended=function(){noise.disconnect();filter.disconnect();snap.disconnect();};
      if(isValve)return;
      [kind==='attach'?96:kind==='seat'?94:146,327+index*91,713].forEach(function(frequency,partial){
        var metal=ctx.createOscillator(),ring=ctx.createGain();metal.type='sine';
        metal.frequency.setValueAtTime(frequency,t);metal.frequency.exponentialRampToValueAtTime(frequency*.82,t+.09);
        ring.gain.setValueAtTime((kind==='attach'?.28:.16)/(1+partial+index),t);ring.gain.exponentialRampToValueAtTime(.0001,t+.14);
        metal.connect(ring);ring.connect(blipOut(ctx));metal.start(t);metal.stop(t+.15);
        metal.onended=function(){metal.disconnect();ring.disconnect();};
      });
    });
  }catch(e){}
}

function blipCardDrive(duration) {
  try {
    var ctx=getKioskAudio(), out=ctx.createGain(), filter=ctx.createBiquadFilter();
    filter.type='lowpass'; filter.frequency.value=850;
    filter.connect(out); out.connect(blipOut(ctx)); out.gain.value=0;
    var motor=ctx.createOscillator(), harmonic=ctx.createOscillator();
    motor.type='sine'; harmonic.type='triangle';
    var overtone=ctx.createGain(); overtone.gain.value=.22;
    motor.connect(filter); harmonic.connect(overtone); overtone.connect(filter);
    var buffer=ctx.createBuffer(1,ctx.sampleRate*.4,ctx.sampleRate), data=buffer.getChannelData(0);
    for(var i=0;i<data.length;i++) data[i]=Math.random()*2-1;
    var friction=ctx.createBufferSource(), rub=ctx.createGain();
    friction.buffer=buffer; friction.loop=true; rub.gain.value=.12;
    friction.connect(rub); rub.connect(filter);
    motor.start(); harmonic.start(); friction.start();
    var stopped=false;
    function stop() {
      if(stopped) return; stopped=true;
      out.gain.setTargetAtTime(0,ctx.currentTime,.015);
      [motor,harmonic,friction].forEach(function(node){node.stop(ctx.currentTime+.08);});
      setTimeout(function(){out.disconnect();filter.disconnect();},150);
    }
    window.addEventListener('pagehide',stop,{once:true});
    setTimeout(stop,duration || 3500);
    return {update:function(progress){
      var speed=Math.sin(progress*Math.PI), now=ctx.currentTime;
      motor.frequency.setTargetAtTime(62+speed*74,now,.025);
      harmonic.frequency.setTargetAtTime(124+speed*148,now,.025);
      out.gain.setTargetAtTime(.04+.105*speed,now,.025);
    },stop:stop};
  } catch(e) {return null;}
}

function playRolodexDock(attached) {
  try {
    var ctx=getKioskAudio(),start=ctx.currentTime,output=ctx.createGain(),nodes=[output];
    output.gain.value=.85;output.connect(blipOut(ctx));
    // Both catches engage together; a short second strike is the latch's rebound.
    [0,attached ? .065 : .032].forEach(function(offset,index){
      var at=start+offset,buffer=ctx.createBuffer(1,Math.ceil(ctx.sampleRate*.09),ctx.sampleRate),data=buffer.getChannelData(0);
      for(var i=0;i<data.length;i++)data[i]=(Math.random()*2-1)*Math.exp(-i/(ctx.sampleRate*.009));
      var snap=ctx.createBufferSource(),filter=ctx.createBiquadFilter(),gain=ctx.createGain();
      snap.buffer=buffer;filter.type='highpass';filter.frequency.value=950;gain.gain.value=index?.28:.9;
      snap.connect(filter);filter.connect(gain);gain.connect(output);snap.start(at);nodes.push(snap,filter,gain);
      (attached ? [92,287,741] : [163,431,1187]).forEach(function(frequency,partial){
        var tone=ctx.createOscillator(),body=ctx.createGain();tone.type='triangle';tone.frequency.value=frequency;
        body.gain.setValueAtTime((index?.07:.18)/(partial+1),at);body.gain.exponentialRampToValueAtTime(.0001,at+.12);
        tone.connect(body);body.connect(output);tone.start(at);tone.stop(at+.14);nodes.push(tone,body);
      });
    });
    setTimeout(function(){nodes.forEach(function(node){node.disconnect();});},300);
  } catch(e) {}
}

function playCardLatch(guide) {
  try {
    var ctx=getKioskAudio(), t=ctx.currentTime;
    var buffer=ctx.createBuffer(1,Math.ceil(ctx.sampleRate*.06),ctx.sampleRate), data=buffer.getChannelData(0);
    for(var i=0;i<data.length;i++) data[i]=(Math.random()*2-1)*Math.exp(-i/(ctx.sampleRate*.012));
    var noise=ctx.createBufferSource(), filter=ctx.createBiquadFilter(), gain=ctx.createGain();
    noise.buffer=buffer; filter.type='bandpass'; filter.frequency.value=guide?1500:950; filter.Q.value=.7;
    gain.gain.value=guide ? .045 : .16;
    noise.connect(filter); filter.connect(gain); gain.connect(blipOut(ctx)); noise.start(t);
    (guide?[270]:[118,283,517]).forEach(function(frequency,index){
      var osc=ctx.createOscillator(), body=ctx.createGain(); osc.type='sine'; osc.frequency.value=frequency;
      var at=t+(index===0?0:.014);
      body.gain.setValueAtTime(guide ? .025 : .1/(index+1),at);
      body.gain.exponentialRampToValueAtTime(.0001,at+(guide ? .04 : .16));
      osc.connect(body);body.connect(blipOut(ctx));osc.start(at);osc.stop(at+.18);
    });
  } catch(e) {}
}

function blipSetMarquee(name, animate) {
  var sign=document.getElementById('marquee-name');
  if(!sign) return;
  var tiles=sign.querySelectorAll('.marquee-letter');
  sign.setAttribute('aria-label',name || 'No game selected');
  sign.classList.remove('marquee-engage');
  tiles.forEach(function(tile,index){
    tile.textContent=animate?'\u00a0':name[index] || '\u00a0';
    tile.style.setProperty('--flap-index',index);
  });
  if(animate) {
    void sign.offsetWidth; sign.classList.add('marquee-engage');
    Array.from(name).forEach(function(char,index){
      setTimeout(function(){tiles[index].textContent=char;playCardLatch(true);},index*190);
    });
  }
}

function blipBuildCabinetStars() {
  if (!document.body.classList.contains('cabinet-page') ||
      document.documentElement.hasAttribute('data-cabinet-screen')) return;
  var stars = document.createElement('div');
  stars.className = 'cabinet-stars';
  stars.setAttribute('aria-hidden', 'true');
  // Fixed seed keeps the cabinet's speckles in place across page navigation.
  var seed = 1977;
  function random() {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    return seed / 4294967296;
  }
  for (var i = 0; i < 120; i++) {
    var star = document.createElement('i');
    star.style.left = random() * 100 + '%';
    star.style.top = random() * 100 + '%';
    star.style.opacity = (0.12 + random() * 0.23).toFixed(2);
    stars.appendChild(star);
  }
  document.body.prepend(stars);
}
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', blipBuildCabinetStars, { once:true });
} else blipBuildCabinetStars();
