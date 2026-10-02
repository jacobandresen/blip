var MAX_COINS = 5;

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
// `buttons`: the deck's caps, one per button (key/code as injectKey() and the
// keyboard listener match them); omitted = one fire button (Space).
// `stick`: per-game stick geometry, all optional: engage / release (px from
// the floating pivot to catch a detent / fall back to neutral), maxR (how far
// the pivot trails the thumb), hyst (degrees past the 22.5 midline before the
// lock jumps a detent).
// `touch`: the game can be played on the touch strip instead of the pad.
// kind 'drag' puts the paddle / cannon under the finger (fire held while it
// is down), 'swipe' steers by flicks, 'paddles' gives each player a half to
// drag their bat up and down in, 'platform' fires button one on every
// touch, runs on a slide and presses button two on a swipe up. `hint` is printed on the strip,
// `mouseHint` instead for a touchscreen laptop's mouse.
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
  // A fighter: two caps, punch and kick (holding toward the opponent hits
  // high), and a stick that answers at once, since a sloppy neutral is a
  // dropped guard. `players: 2` builds the second station, revealed by
  // blip_set_mode; `keys` gives P1 WASD so the arrows are P2's.
  brawler:            { name: 'BRAWLER', accent: '220, 60, 40',
                         players: 2,
                         buttons: [{ key: 'f', code: 'KeyF', label: 'PUNCH' },
                                   { key: 'g', code: 'KeyG', label: 'KICK' }],
                         keys: { up:    { key: 'w', code: 'KeyW' },
                                 down:  { key: 's', code: 'KeyS' },
                                 left:  { key: 'a', code: 'KeyA' },
                                 right: { key: 'd', code: 'KeyD' },
                                 p2up:    { key: 'ArrowUp',    code: 'ArrowUp' },
                                 p2down:  { key: 'ArrowDown',  code: 'ArrowDown' },
                                 p2left:  { key: 'ArrowLeft',  code: 'ArrowLeft' },
                                 p2right: { key: 'ArrowRight', code: 'ArrowRight' },
                                 p2button1: { key: 'j', code: 'KeyJ' },
                                 p2button2: { key: 'k', code: 'KeyK' } },
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
                         keys: { up:    { key: 'w', code: 'KeyW' },
                                 down:  { key: 's', code: 'KeyS' },
                                 left:  { key: 'a', code: 'KeyA' },
                                 right: { key: 'd', code: 'KeyD' },
                                 p2up:    { key: 'ArrowUp',    code: 'ArrowUp' },
                                 p2down:  { key: 'ArrowDown',  code: 'ArrowDown' },
                                 p2left:  { key: 'ArrowLeft',  code: 'ArrowLeft' },
                                 p2right: { key: 'ArrowRight', code: 'ArrowRight' },
                                 p2button1: { key: 'j', code: 'KeyJ' },
                                 p2button2: { key: 'k', code: 'KeyK' } } },
  // A bullet-weaving shooter: fine nudges. A small dead zone, a long pivot
  // leash so re-centring neutralises, and a firm notch so a dodge holds.
  sky_raider:         { name: 'RAIDER', accent: '50, 100, 220',
                         stick: { engage: 11, release: 6, maxR: 58, hyst: 12 } }
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

function updateCoinsHud() {
  var n = getCoins(), icons = '';
  for (var i = 0; i < MAX_COINS; i++) icons += i < n ? '●' : '○';
  // The word is its own span: the narrowest phones drop it (kiosk.css).
  var html = '<span class="coins-word">COINS </span>' + icons;
  document.querySelectorAll('[data-coins-hud]').forEach(function (el) {
    el.innerHTML = html;
  });
}

/* ---- Sound on / off ---- The choice is stored ('blip-mute'). The games'
 * sounds are Howler's, muted there; the cabinet's own (coins, clicks) all
 * leave through blipOut(), one gain that is shut while muted. */
var _blipOut = null;
function blipMuted() { try { return localStorage.getItem('blip-mute') === '1'; } catch (e) { return false; } }
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

// A coin is two sounds: the inharmonic clink through the chute, then the
// register's "credit accepted" chime. Keep in sync with shell.js's copy.
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

  [3000, 4550, 6100].forEach(function (freq, i) {
    var osc = ctx.createOscillator(), gain = ctx.createGain();
    osc.type = 'triangle'; osc.frequency.value = freq;
    osc.connect(gain); gain.connect(blipOut(ctx));
    var start = t + i * 0.006;
    gain.gain.setValueAtTime(0.16 / (i + 1), start);
    gain.gain.exponentialRampToValueAtTime(0.0008, start + 0.1);
    osc.start(start); osc.stop(start + 0.11);
  });

  [{ freq: 1047, start: 0.1 }, { freq: 1319, start: 0.155 }].forEach(function(note) {
    var osc  = ctx.createOscillator();
    var gain = ctx.createGain();
    osc.connect(gain);
    gain.connect(blipOut(ctx));
    osc.type = 'square';
    osc.frequency.value = note.freq;
    gain.gain.setValueAtTime(0.22, t + note.start);
    gain.gain.exponentialRampToValueAtTime(0.001, t + note.start + 0.11);
    osc.start(t + note.start);
    osc.stop(t + note.start + 0.12);
  });
}

// The coin that drops into the slot (kiosk.css #coin-drop-anim): a real
// element so a class toggle can animate it, injected once for every non-game
// page.
var coinDropAnim = null;
(function () {
  var btn = document.getElementById('kiosk-insert-btn');
  if (!btn) return;
  coinDropAnim = document.createElement('span');
  coinDropAnim.id = 'coin-drop-anim';
  coinDropAnim.setAttribute('aria-hidden', 'true');
  btn.appendChild(coinDropAnim);
}());
function dropCoinAnimation() {
  if (!coinDropAnim) return;
  // Measured, like shell.js's copy: the slot is the centre of the coin plate,
  // padding-right plus half the plate, and both change across breakpoints.
  var btn = coinDropAnim.parentElement;
  var padRight = parseFloat(getComputedStyle(btn).paddingRight) || 8;
  var plateW = parseFloat(getComputedStyle(btn, '::after').width) || 18;
  coinDropAnim.style.setProperty('--slot-x', (padRight + plateW / 2) + 'px');
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

function flashCoins() {
  ['insert-coin', 'kiosk-insert-btn'].forEach(function(id) {
    var el = document.getElementById(id);
    if (!el) return;
    el.classList.remove('coin-flash');
    void el.offsetWidth;
    el.classList.add('coin-flash');
    el.addEventListener('animationend', function() { el.classList.remove('coin-flash'); }, { once: true });
  });
}

function insertCoin() {
  var n = getCoins();
  if (n < MAX_COINS) {
    saveCoins(n + 1);
    playCoinInsert();
    dropCoinAnimation();
    updateCoinsHud();
    updateCoinBeckon();
    flashCoins();
    if (typeof window.onCoinInserted === 'function') window.onCoinInserted();
  } else {
    playNoRoom();
    ['insert-coin', 'kiosk-insert-btn'].forEach(function(id) {
      var el = document.getElementById(id);
      if (!el) return;
      el.classList.remove('shake');
      void el.offsetWidth;
      el.classList.add('shake');
      el.addEventListener('animationend', function() { el.classList.remove('shake'); }, { once: true });
    });
  }
}

// While the landing page (.game-grid) shows zero credits, the COINS button
// throbs (.needs-coin) until the first coin.
function updateCoinBeckon() {
  if (!document.querySelector('.game-grid')) return;
  var btn = document.getElementById('kiosk-insert-btn');
  if (btn) btn.classList.toggle('needs-coin', getCoins() <= 0);
}
window.addEventListener('load', updateCoinBeckon);

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

/* ---- Shared gamepad polling ---- Reports logical button changes ('ArrowUp'
 * | 'ArrowDown' | 'ArrowLeft' | 'ArrowRight' | 'Space' | 'KeyZ') from the
 * first gamepad to onDown / onUp; game pages turn them into key events, the
 * landing page moves the card selection. */
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

  // Polled unconditionally: Chrome on Linux never fires 'gamepadconnected'
  // for many generic joysticks, and rAF polling costs nothing with none
  // attached. Station one's lamp: off with no pad, dim when idle for IDLE_MS,
  // lit in use.
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
  document.addEventListener('touchstart', unlockAudio, { passive: true, capture: true });
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

/* ---- Fullscreen ---- One button beside every coin slot sets <html
 * data-fullscreen> (each page's CSS decides what that hides) and stores it.
 * A page load drops real fullscreen; the first key or click asks again. */
(function () {
  var root = document.documentElement;
  var KEY = 'blip-fullscreen';
  var enter = root.requestFullscreen || root.webkitRequestFullscreen;
  var leave = document.exitFullscreen || document.webkitExitFullscreen;
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

  var btn = null;
  function apply(on) {
    root.toggleAttribute('data-fullscreen', on);
    wakeCursor();
    if (btn) {
      btn.setAttribute('aria-pressed', on ? 'true' : 'false');
      btn.title = on ? 'Exit fullscreen' : 'Fullscreen';
      btn.setAttribute('aria-label', btn.title);
    }
    if (typeof window.onBlipFullscreenChange === 'function') {
      try { window.onBlipFullscreenChange(on); } catch (e) {}
    }
  }
  function set(on) {
    try { localStorage.setItem(KEY, on ? '1' : '0'); } catch (e) {}
    apply(on);
  }
  // Before first paint where this script is in <head>.
  if (wanted()) root.setAttribute('data-fullscreen', '');

  // Left of the coin slot. On a phone a game's name reaches that far: one
  // button then goes right of the logo, and the name is centred in what is
  // left between it and the slot, its letters closed up until it fits.
  var mute = null;
  function place() {
    var coin = document.getElementById('insert-coin-btn') || document.getElementById('kiosk-insert-btn');
    var name = document.getElementById('marquee-name');
    var logo = document.querySelector('.blip-logo');
    if (!mute || !coin) return;
    var bar = name && name.parentNode;
    if (bar) {
      bar.style.paddingLeft = bar.style.paddingRight = '';
      name.style.letterSpacing = name.style.visibility = '';
    }
    mute.hidden = false;
    var first = btn || mute;   // the one that stays where there is room for one
    var slot = coin.getBoundingClientRect();
    // As tall as the coin slot, which is as tall as the page's top bar (28,
    // 40 or 46px), so the icon sits on the bar's middle line.
    var h = coin.offsetHeight + 'px';
    var w = first.offsetWidth || 34;
    var left = slot.left - w - 2;
    var cramped = !!(name && logo && slot.width > 0 &&
      name.getBoundingClientRect().right > left - (btn ? w : 0));
    if (cramped) left = logo.getBoundingClientRect().right + 2;
    first.style.height = h;
    first.style.left = Math.round(left) + 'px';
    if (btn) {
      mute.hidden = cramped;
      mute.style.height = h;
      mute.style.left = Math.round(left - w) + 'px';
    }
    if (!cramped) {
      // The bulb strips beside the name stop short of the buttons; the same
      // on both sides, so the name stays in the middle.
      if (bar && logo) {
        var edge = Math.max(logo.getBoundingClientRect().right, window.innerWidth - (left - (btn ? w : 0)));
        bar.style.paddingLeft = bar.style.paddingRight = Math.round(edge + 4) + 'px';
      }
      return;
    }
    var from = left + w, room = slot.left - 4 - from;
    bar.style.paddingLeft = Math.round(from) + 'px';
    bar.style.paddingRight = Math.round(window.innerWidth - slot.left + 4) + 'px';
    var gap = parseFloat(getComputedStyle(name).letterSpacing) || 0;
    while (gap > 1 && name.getBoundingClientRect().width > room) {
      gap -= 1;
      name.style.letterSpacing = gap + 'px';
    }
    if (name.getBoundingClientRect().width > room) name.style.visibility = 'hidden';
  }

  function build() {
    var coin = document.getElementById('insert-coin-btn') || document.getElementById('kiosk-insert-btn');
    if (!coin) return;
    // A touch screen keeps its whole layout, so without real fullscreen
    // (iPhone) the button would do nothing: the sound button stands alone.
    if (!blipHasTouch() || enter) {
      btn = document.createElement('button');
      btn.id = 'fullscreen-btn';
      btn.type = 'button';
      btn.innerHTML = '<svg viewBox="0 0 16 16" aria-hidden="true">' +
        '<path class="fs-enter" d="M1.5 6V1.5H6M10 1.5h4.5V6M14.5 10v4.5H10M6 14.5H1.5V10"/>' +
        '<path class="fs-exit" d="M1.5 6H6V1.5M10 1.5V6h4.5M14.5 10H10v4.5M6 14.5V10H1.5"/></svg>';
      document.body.appendChild(btn);
    }
    mute = document.createElement('button');
    mute.id = 'mute-btn';
    mute.type = 'button';
    mute.innerHTML = '<svg viewBox="0 0 16 16" aria-hidden="true">' +
      '<path d="M1.5 6h2.7L8 3v10L4.2 10H1.5z"/>' +
      '<path class="snd-on" d="M10.4 5.6c1.1 1.2 1.1 3.6 0 4.8M12.4 3.8c2.1 2.2 2.1 6.2 0 8.4"/>' +
      '<path class="snd-off" d="M10.6 6l4 4M14.6 6l-4 4"/></svg>';
    function label() {
      mute.title = blipMuted() ? 'Sound on' : 'Sound off';
      mute.setAttribute('aria-label', mute.title);
      mute.setAttribute('aria-pressed', blipMuted() ? 'true' : 'false');
    }
    document.body.appendChild(mute);
    label();
    mute.addEventListener('click', function () { blipSetMuted(!blipMuted()); label(); });
    // M does the same, for a cabinet with no mouse (no game uses the key).
    window.addEventListener('keydown', function (e) {
      if ((e.key !== 'm' && e.key !== 'M') || e.repeat || e.ctrlKey || e.metaKey || e.altKey) return;
      if (e.target && e.target.closest && e.target.closest('input, textarea')) return;
      blipSetMuted(!blipMuted());
      label();
    }, true);
    if (btn) btn.addEventListener('click', function () {
      var on = !root.hasAttribute('data-fullscreen');
      set(on);
      if (on) request();
      else if (inBrowserFullscreen() && leave) { try { leave.call(document); } catch (e) {} }
    });
    window.addEventListener('resize', place);
    // The logo and the slot settle after their fonts and boot animation.
    if (typeof ResizeObserver === 'function') {
      var settle = new ResizeObserver(place);
      ['.blip-logo', '#insert-coin-btn', '#kiosk-insert-btn', '#marquee-name'].forEach(function (sel) {
        var el = document.querySelector(sel);
        if (el) settle.observe(el);
      });
    }
    var logo = document.querySelector('.blip-logo');
    if (logo) logo.addEventListener('animationend', place);
    apply(wanted());
    place();
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
      place();
    });
  });
  // Back in real fullscreen on the first key or click of a page.
  ['keydown', 'mousedown', 'pointerup', 'touchend'].forEach(function (ev) {
    window.addEventListener(ev, function (e) {
      if (!wanted() || inBrowserFullscreen()) return;
      if (e.target && e.target.closest && e.target.closest('#fullscreen-btn')) return;
      if (e.key === 'Escape') return;
      request();
    }, true);
  });
}());
