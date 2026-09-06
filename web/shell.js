(function () {
'use strict';

var TOPBAR_H   = 56;
var MARQUEE_H  = 28; // reserved header height once #marquee-bar exists
var PAD        = 16; // padding around the canvas on all sides (= bezel width)

var loader    = document.getElementById('loader');
var barInner  = document.getElementById('bar-inner');
var statusEl  = document.getElementById('status');
var canvas    = document.getElementById('glcanvas');
var overlay   = document.getElementById('need-coin-overlay');

updateCoinsHud();

// ---- Per-game marquee + cabinet accent ----
// Builds the backlit name sign in JS (rather than requiring it in every
// game's HTML) so rally's hand-maintained page picks it up for free, same as
// the shell-generated ones.
(function () {
  var game = (typeof blipGameFromPath === 'function') ? blipGameFromPath(window.location.pathname) : null;
  if (!game) return;
  document.documentElement.style.setProperty('--cab', game.accent);
  var bar = document.createElement('div');
  bar.id = 'marquee-bar';
  bar.innerHTML =
    '<span class="marquee-bulbs"></span>' +
    '<span id="marquee-name">' + game.name + '</span>' +
    '<span class="marquee-bulbs"></span>';
  document.body.insertBefore(bar, document.body.firstChild);

  var logo = document.querySelector('.blip-logo');
  if (logo) {
    logo.classList.add('boot');
    logo.addEventListener('animationend', function () {
      logo.classList.remove('boot');
    }, { once: true });
  }
}());

// Stop all audio when navigating away (pagehide is reliable on iOS Safari / PWA)
window.addEventListener('pagehide', function () {
  if (typeof Howler !== 'undefined') Howler.stop();
});

// ---- UI audio (coin insert / no room) ----

var uiAudio = null;
function getUiAudio() {
  if (typeof Howler !== 'undefined' && Howler.ctx) {
    if (Howler.ctx.state === 'suspended') Howler.ctx.resume();
    return Howler.ctx;
  }
  if (!uiAudio) uiAudio = new (window.AudioContext || window.webkitAudioContext)();
  if (uiAudio.state === 'suspended') uiAudio.resume();
  return uiAudio;
}
// A real coin makes two distinct sounds in sequence: the metallic clink
// of it dropping through the chute (bright, inharmonic, near-instant —
// real metal doesn't ring in tidy octaves the way a synth voice does),
// then the register's own electronic "credit accepted" chime a beat
// later. Modelling both, rather than just the chime alone, is what reads
// as an actual coin rather than a UI beep.
function playCoinInsert() {
  var ctx = getUiAudio(), t = ctx.currentTime;

  // The clink: filtered noise for the transient "tik" of metal on metal,
  // plus a few short inharmonic tones for the coin's own brief ring.
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
  noise.connect(noiseFilter); noiseFilter.connect(noiseGain); noiseGain.connect(ctx.destination);
  noise.start(t); noise.stop(t + 0.045);

  [3000, 4550, 6100].forEach(function (freq, i) {
    var osc = ctx.createOscillator(), gain = ctx.createGain();
    osc.type = 'triangle'; osc.frequency.value = freq;
    osc.connect(gain); gain.connect(ctx.destination);
    var start = t + i * 0.006;
    gain.gain.setValueAtTime(0.16 / (i + 1), start);
    gain.gain.exponentialRampToValueAtTime(0.0008, start + 0.1);
    osc.start(start); osc.stop(start + 0.11);
  });

  // The credit chime, arriving just after the coin lands.
  [{ freq: 1047, start: 0.1 }, { freq: 1319, start: 0.155 }].forEach(function (note) {
    var osc = ctx.createOscillator(), gain = ctx.createGain();
    osc.connect(gain); gain.connect(ctx.destination);
    osc.type = 'square'; osc.frequency.value = note.freq;
    gain.gain.setValueAtTime(0.22, t + note.start);
    gain.gain.exponentialRampToValueAtTime(0.001, t + note.start + 0.11);
    osc.start(t + note.start); osc.stop(t + note.start + 0.12);
  });
}

// The coin that visually drops into #insert-coin-btn's slot on a
// successful insert — a real element (not a pseudo-element) so a class
// toggle can animate it on demand, injected here rather than duplicated
// across shell.html and all six per-game index.html files.
var coinDropAnim = null;
(function () {
  var btn = document.getElementById('insert-coin-btn');
  if (!btn) return;
  coinDropAnim = document.createElement('span');
  coinDropAnim.id = 'coin-drop-anim';
  coinDropAnim.setAttribute('aria-hidden', 'true');
  btn.appendChild(coinDropAnim);
}());
function dropCoinAnimation() {
  if (!coinDropAnim) return;
  // The slot (#insert-coin-btn::after) sits flush against the button's
  // content-box edge, so its centre is padding-right + half its own 6px
  // width in from the button's outer edge — read padding back from the
  // computed style (it changes across the responsive breakpoints) rather
  // than guessing a constant, so the coin actually lands on the slot
  // instead of just somewhere near it.
  var btn = coinDropAnim.parentElement;
  var padRight = parseFloat(getComputedStyle(btn).paddingRight) || 8;
  coinDropAnim.style.setProperty('--slot-x', (padRight + 3) + 'px');
  coinDropAnim.classList.remove('dropping');
  void coinDropAnim.offsetWidth;
  coinDropAnim.classList.add('dropping');
  coinDropAnim.addEventListener('animationend', function () {
    coinDropAnim.classList.remove('dropping');
  }, { once: true });
}
function playNoRoom() {
  var ctx = getUiAudio(), t = ctx.currentTime;
  var osc = ctx.createOscillator(), gain = ctx.createGain();
  osc.connect(gain); gain.connect(ctx.destination);
  osc.type = 'sawtooth';
  osc.frequency.setValueAtTime(200, t);
  osc.frequency.exponentialRampToValueAtTime(65, t + 0.38);
  gain.gain.setValueAtTime(0.32, t);
  gain.gain.exponentialRampToValueAtTime(0.001, t + 0.38);
  osc.start(t); osc.stop(t + 0.39);
}
function flashCoinBar() {
  ['insert-coin-btn'].forEach(function (id) {
    var el = document.getElementById(id);
    if (!el) return;
    el.classList.remove('coin-flash');
    void el.offsetWidth;
    el.classList.add('coin-flash');
    el.addEventListener('animationend', function () { el.classList.remove('coin-flash'); }, { once: true });
  });
}

// Called from WASM on game-over restart — spend a coin or block if empty.
window.blipSpendCoin = function () {
  var n = getCoins();
  if (n <= 0) {
    overlay.classList.add('visible');
    return 0;
  }
  saveCoins(n - 1);
  updateCoinsHud();
  return 1;
};

// Called from WASM when game mode is chosen on the title screen.
// mode 0 = 1-player (CPU controls right paddle), 1 = 2-player.
window.blipSetMode = function (mode) {
  var d = document.getElementById('paddle-dial-p2');
  if (!d) return;
  var isCpu = (mode === 0);
  d.classList.toggle('cpu-mode', isCpu);
  var label = document.getElementById('dial-label-p2');
  if (label) label.textContent = isCpu ? 'CPU' : '2P';
};

overlay.addEventListener('click', function () {
  var n = getCoins();
  if (n >= MAX_COINS) return;
  saveCoins(n + 1);
  updateCoinsHud();
  playCoinInsert();
  dropCoinAnimation();
  flashCoinBar();
  overlay.classList.remove('visible');
});

document.getElementById('insert-coin-btn').addEventListener('click', function () {
  var n = getCoins();
  if (n >= MAX_COINS) {
    playNoRoom();
    var btn = this;
    btn.classList.remove('shake');
    void btn.offsetWidth;
    btn.classList.add('shake');
    btn.addEventListener('animationend', function () { btn.classList.remove('shake'); }, { once: true });
    return;
  }
  saveCoins(n + 1);
  updateCoinsHud();
  playCoinInsert();
  dropCoinAnimation();
  flashCoinBar();
  overlay.classList.remove('visible');
});

// Mirror the overlay's visibility onto <body> so CSS can key off it
// (body.need-coin flashes the top-right coin button, kiosk.css/shell.css)
// without every show/hide site having to know.
new MutationObserver(function () {
  document.body.classList.toggle('need-coin', overlay.classList.contains('visible'));
}).observe(overlay, { attributes: true, attributeFilter: ['class'] });

// Landing on a game page with no credits — a fresh session, a shared deep
// link — is walking up to a cold cabinet: you put a coin in before you
// play. Same overlay the in-game "continue?" prompt uses; it blocks
// injectKey() until a coin goes in, so the title screen can't be started
// for free.
if (getCoins() <= 0) overlay.classList.add('visible');

// ---- Canvas sizing ----
// The kiosk bar is position:fixed;bottom:0. We leave PAD px on each side
// plus full clearance for the bar so it never overlaps the canvas.

function fillCanvas() {
  var tb = document.getElementById('topbar');
  // clamp to 56 so a mis-read of 0 before layout doesn't eat the whole screen
  TOPBAR_H = tb ? Math.max(tb.offsetHeight, 56) : 56;
  var mb = document.getElementById('marquee-bar');
  MARQUEE_H = mb ? Math.max(mb.offsetHeight, 28) : 0;
  var w = window.innerWidth  - PAD * 2;
  var h = window.innerHeight - TOPBAR_H - MARQUEE_H - PAD * 2;
  canvas.style.setProperty('width',  w + 'px', 'important');
  canvas.style.setProperty('height', h + 'px', 'important');
  canvas.style.setProperty('top',    (MARQUEE_H + PAD) + 'px', 'important');
  canvas.style.setProperty('left',   PAD + 'px', 'important');
  canvas.style.setProperty('transform', 'none', 'important');
  // The touch controls (dpad, fire buttons, paddle dials — all plain CSS,
  // positioned "bottom: calc(--topbar-h + ...)") key off this custom
  // property instead of a hardcoded 56px, so they stay clear of the bar
  // even when it renders taller than 56px (e.g. its content wrapping or
  // growing on a narrow phone) instead of overlapping it.
  document.documentElement.style.setProperty('--topbar-h', TOPBAR_H + 'px');
}
window.addEventListener('resize', fillCanvas);
fillCanvas();

function hideLoader() {
  if (loader && loader.style.display !== 'none') {
    loader.style.display = 'none';
    fillCanvas();
    canvas.focus();
  }
}

(function waitForCanvas() {
  if (canvas.width > 0 && canvas.height > 0) { hideLoader(); return; }
  setTimeout(waitForCanvas, 50);
})();
setTimeout(hideLoader, 3000);

canvas.addEventListener('webglcontextlost', function (e) {
  e.preventDefault();
  alert('WebGL context lost. Please reload the page.');
}, false);

// ---- Touch controls — injected as keyboard events into the WASM game ----

// Block game input while the overlay is up.
window.addEventListener('keydown', function (e) {
  if (overlay.classList.contains('visible')) e.stopImmediatePropagation();
}, true);

function injectKey(key, code, type) {
  if (overlay.classList.contains('visible')) return;
  canvas.dispatchEvent(new KeyboardEvent(type, {
    bubbles: true, cancelable: true, key: key, code: code
  }));
}

var isRally = window.location.pathname.indexOf('/rally/') !== -1;

// ---- Visual feedback: reflect the live input state — keyboard, touch, or
// gamepad — onto the on-screen stick/buttons. Keyboard, touch, and gamepad
// all funnel through injectKey()'s synthetic KeyboardEvent dispatched on
// the canvas, which bubbles up to this listener exactly like a real
// keypress would — so one listener covers every input source without the
// stick/button code below needing to know or care which one is driving
// it. Always wired up, not just on touch devices, so a keyboard player
// sees their own presses reflected too.
if (!isRally) (function () {
  var DIR_FOR_CODE = {
    ArrowUp: 'up', KeyW: 'up',
    ArrowDown: 'down', KeyS: 'down',
    ArrowLeft: 'left', KeyA: 'left',
    ArrowRight: 'right', KeyD: 'right',
  };
  var held  = { up: false, down: false, left: false, right: false };
  // Set on #stick-base (not #stick-handle) so the --dx/--dy custom
  // properties are visible to both #stick-handle's lean AND
  // #stick-base::before's contact-shadow drift (kiosk.css) — both are
  // its children, but siblings of each other, so the value has to live
  // on their shared parent to reach them both via inheritance.
  var stick = document.getElementById('stick-base');

  function updateStick() {
    if (!stick) return;
    var x = (held.right ? 1 : 0) - (held.left ? 1 : 0);
    var y = (held.down  ? 1 : 0) - (held.up   ? 1 : 0);
    stick.style.setProperty('--dx', x);
    stick.style.setProperty('--dy', y);
  }

  document.addEventListener('keydown', function (e) { reflect(e.code, true);  });
  document.addEventListener('keyup',   function (e) { reflect(e.code, false); });

  function reflect(code, down) {
    var dir = DIR_FOR_CODE[code];
    if (dir) { held[dir] = down; updateStick(); return; }
    var btn = document.querySelector('.arcade-btn[data-code="' + code + '"]');
    if (btn) btn.classList.toggle('active', down);
  }
}());

if (isRally) {
  if ('ontouchstart' in window || navigator.maxTouchPoints > 0) (function () {
    // ---- Dual paddle dials (P1 left = Arrow keys, P2 right = I/K) ----
    document.getElementById('stick-base').style.display = 'none';
    document.getElementById('fire-buttons').style.display = 'none';

    var dialP1 = document.getElementById('paddle-dial');
    var dialP2 = document.getElementById('paddle-dial-p2');
    dialP1.style.display = 'block';
    dialP2.style.display = 'block';

    // null = title screen (no mode chosen yet); 0 = 1P; 1 = 2P.
    var rallyMode = null;

    function applyRallyMode(mode) {
      rallyMode = mode;
      window.blipSetMode(mode);
    }

    // Tap anywhere on the canvas = start 1P (or launch ball / any-key during play).
    // Only update the mode indicator when we're still on the title screen.
    canvas.addEventListener('touchstart', function (e) {
      e.preventDefault();
      injectKey(' ', 'Space', 'keydown');
      injectKey(' ', 'Space', 'keyup');
      if (rallyMode === null) applyRallyMode(0);
    }, { passive: false });

    // onTap is called when a tap gesture completes, before the key is injected.
    // onInteract is called on the first touch contact (before any keys fly).
    function makeDial(dialEl, handEl, upKey, upCode, downKey, downCode, tapKey, tapCode, onTap, onInteract) {
      var angle       = -Math.PI / 2;
      var lastAngle   = null;
      var totalDelta  = 0;
      var touchId     = null;
      var upHeld      = false;
      var downHeld    = false;

      function angleFrom(touch) {
        var r = dialEl.getBoundingClientRect();
        return Math.atan2(touch.clientY - (r.top  + r.height / 2),
                          touch.clientX - (r.left + r.width  / 2));
      }

      function findTouch(list, id) {
        for (var i = 0; i < list.length; i++) {
          if (list[i].identifier === id) return list[i];
        }
        return null;
      }

      function setDir(up, down) {
        if (up !== upHeld) {
          upHeld = up;
          injectKey(upKey, upCode, up ? 'keydown' : 'keyup');
        }
        if (down !== downHeld) {
          downHeld = down;
          injectKey(downKey, downCode, down ? 'keydown' : 'keyup');
        }
      }

      function stop() {
        setDir(false, false);
        dialEl.classList.remove('active');
        lastAngle = null;
        touchId   = null;
      }

      dialEl.addEventListener('touchstart', function (e) {
        e.preventDefault();
        e.stopPropagation(); // don't also fire the canvas Space handler
        if (touchId !== null) return;
        if (onInteract) onInteract();
        var t   = e.changedTouches[0];
        touchId    = t.identifier;
        lastAngle  = angleFrom(t);
        totalDelta = 0;
        dialEl.classList.add('active');
      }, { passive: false });

      dialEl.addEventListener('touchmove', function (e) {
        e.preventDefault();
        if (lastAngle === null) return;
        var t = findTouch(e.touches, touchId);
        if (!t) return;
        var a = angleFrom(t);
        var d = a - lastAngle;
        if (d >  Math.PI) d -= 2 * Math.PI;
        if (d < -Math.PI) d += 2 * Math.PI;
        lastAngle    = a;
        angle       += d;
        totalDelta  += Math.abs(d);
        handEl.style.transform = 'rotate(' + (angle + Math.PI / 2) + 'rad)';
        var DEAD = 0.018;
        if      (d >  DEAD) setDir(false, true);
        else if (d < -DEAD) setDir(true, false);
        else                setDir(false, false);
      }, { passive: false });

      dialEl.addEventListener('touchend', function (e) {
        e.preventDefault();
        if (!findTouch(e.changedTouches, touchId)) return;
        if (totalDelta < 0.08) {
          if (onTap) onTap();
          injectKey(tapKey, tapCode, 'keydown');
          injectKey(tapKey, tapCode, 'keyup');
        }
        stop();
      }, { passive: false });

      dialEl.addEventListener('touchcancel', stop);
    }

    // P1 dial tap = Space = 1P mode (when still on title screen)
    makeDial(
      dialP1, document.getElementById('dial-hand'),
      'ArrowUp', 'ArrowUp', 'ArrowDown', 'ArrowDown',
      ' ', 'Space',
      function () { if (rallyMode === null) applyRallyMode(0); },
      function () { if (rallyMode === null) applyRallyMode(0); }
    );
    // P2 dial tap = '2' = 2P mode (when still on title screen)
    makeDial(
      dialP2, document.getElementById('dial-hand-p2'),
      'i', 'KeyI', 'k', 'KeyK',
      '2', 'Digit2',
      function () { if (rallyMode === null) applyRallyMode(1); },
      function () { if (rallyMode === null) applyRallyMode(1); }
    );
  }());

} else {
  // ---- Analog stick: drag with mouse, touch, or pen (Pointer Events unify
  // all three) — this only turns drag position into ArrowUp/Down/Left/Right
  // key state; moving the ball itself is handled by reflect()/updateStick()
  // above, the same code path a real keypress drives. ----
  (function () {
    var base   = document.getElementById('stick-base');
    var fire   = document.getElementById('fire-buttons');
    // Bind the drag to #topbar, not #stick-base: #stick-base sits inside
    // the deck's 3D tilt (.deck-panel's rotateX), which leaves small,
    // maddening seams in its hit-test where a touch lands "on" the stick
    // visually but misses the element. #topbar carries no transform and is
    // the common ancestor of the stick and the buttons, so a pointerdown
    // anywhere on the deck bubbles to it cleanly — we then just ask "is
    // this touch in the stick half or the button half?".
    var bar = document.getElementById('topbar');
    if (!base || !bar) return;

    // A comfortable throw with hysteresis: a direction ENGAGES once the
    // drag is past ON px from the pivot, and only DISENGAGES back inside
    // OFF px. The gap between the two stops it chattering on/off when your
    // thumb hovers near the threshold, and lets small drift while you hold
    // a direction not drop it — the "gets stuck / too twitchy" complaints.
    var ON  = 22; // px from pivot to start moving — a deliberate push
    var OFF = 13; // px to stop (must be < ON)
    var activeId = null;
    var wantDir = { up: false, down: false, left: false, right: false };
    var CODE_FOR = { up: 'ArrowUp', down: 'ArrowDown', left: 'ArrowLeft', right: 'ArrowRight' };

    function setDir(dir, want) {
      if (wantDir[dir] === want) return;
      wantDir[dir] = want;
      var code = CODE_FOR[dir];
      injectKey(code, code, want ? 'keydown' : 'keyup');
    }
    // one axis with hysteresis: v is the signed offset in px
    function axis(negDir, posDir, v) {
      setDir(negDir, wantDir[negDir] ? v < -OFF : v < -ON);
      setDir(posDir, wantDir[posDir] ? v >  OFF : v >  ON);
    }

    // Floating pivot: wherever the thumb first lands becomes "centre", and
    // the drag is measured from there — not from the ball's fixed rest
    // position. The hit-region is tall (it has to cover the ball floating
    // above the bar), so an absolute pivot made a thumb resting anywhere
    // but the exact ball centre read as a hard direction the instant it
    // touched down. This is the standard mobile virtual-stick feel: grab
    // anywhere, push from there. The ball itself still leans to show the
    // direction (via reflect()/updateStick()), it just no longer has to be
    // the thing you aim for.
    var pivot = null;

    function apply(e) {
      axis('left', 'right', e.clientX - pivot.x);
      axis('up',   'down',  e.clientY - pivot.y);
    }

    function release() {
      activeId = null;
      pivot = null;
      setDir('left', false); setDir('right', false);
      setDir('up',   false); setDir('down',  false);
      window.removeEventListener('pointermove', onMove, true);
      window.removeEventListener('pointerup', onEnd, true);
      window.removeEventListener('pointercancel', onEnd, true);
    }

    // A touch belongs to the stick unless it landed on (or right of) the
    // fire buttons. Measured per press so it tracks the responsive layout.
    function isStickTouch(e) {
      if (fire && e.target && e.target.closest && e.target.closest('#fire-buttons')) return false;
      if (fire) {
        var fr = fire.getBoundingClientRect();
        if (fr.width && e.clientX >= fr.left - 6) return false;
      }
      return true;
    }

    function onMove(e) {
      if (e.pointerId !== activeId || !pivot) return;
      e.preventDefault();
      apply(e);
    }
    function onEnd(e) {
      // Any up/cancel for our pointer ends the drag. Don't be fussy about
      // the id on a cancel — iOS fires pointercancel with a mismatched (or
      // reused) id when a second finger lands or it steals the gesture,
      // and a missed release is exactly the "stuck moving forever" bug.
      if (e.type === 'pointercancel' || e.pointerId === activeId) release();
    }

    bar.addEventListener('pointerdown', function (e) {
      // Heal any stuck state from a swallowed release before starting.
      if (activeId !== null) release();
      if (!isStickTouch(e)) return;
      e.preventDefault();
      activeId = e.pointerId;
      pivot = { x: e.clientX, y: e.clientY };
      // Listen on window (not via setPointerCapture): capture is silently
      // dropped by iOS in enough situations that relying on it is what let
      // the stick get stuck. Window listeners always see the up/cancel.
      window.addEventListener('pointermove', onMove, true);
      window.addEventListener('pointerup', onEnd, true);
      window.addEventListener('pointercancel', onEnd, true);
      // No direction yet — the first move off this point is what steers.
    });

    // Last-resort safety nets: if the page loses focus or is hidden mid-
    // drag (a call comes in, you switch apps), let go of everything.
    window.addEventListener('blur', release);
    document.addEventListener('visibilitychange', function () {
      if (document.hidden) release();
    });
  }());

  // ---- Fire buttons: one per game.buttons entry (default: just fire),
  // built here instead of hand-written per game, so a two-button game like
  // Meteors only has to declare it once in BLIP_GAMES (kiosk.js). ----
  (function () {
    var host = document.getElementById('fire-buttons');
    if (!host) return;
    var game = (typeof blipGameFromPath === 'function') ? blipGameFromPath(window.location.pathname) : null;
    var specs = (game && game.buttons) || [{ key: ' ', code: 'Space' }];
    // Always render two buttons, even for a single-action game, so the
    // panel looks and sits the same on every cabinet — the second one just
    // duplicates the first action rather than sitting there dead.
    if (specs.length < 2) specs = specs.concat(specs[0]);

    specs.forEach(function (spec) {
      var btn = document.createElement('div');
      btn.className = 'arcade-btn';
      btn.dataset.key  = spec.key;
      btn.dataset.code = spec.code;
      btn.innerHTML = '<span class="arcade-btn-cap"></span>';
      host.appendChild(btn);

      var activeId = null;
      btn.addEventListener('pointerdown', function (e) {
        e.preventDefault();
        activeId = e.pointerId;
        btn.setPointerCapture(activeId);
        injectKey(spec.key, spec.code, 'keydown');
      });
      function endPointer(e) {
        if (e.pointerId !== activeId) return;
        activeId = null;
        injectKey(spec.key, spec.code, 'keyup');
      }
      btn.addEventListener('pointerup',     endPointer);
      btn.addEventListener('pointercancel', endPointer);
    });
  }());
}

// ---- Gamepad support ----
  // Polling loop lives in kiosk.js (pollGamepad, shared with the kiosk
  // landing page); here we just turn logical button changes into the same
  // synthetic keyboard events the touch controls use.
  (function () {
    function keyOf(code) {
      if (code === 'Space') return ' ';
      if (code === 'KeyZ')  return 'z';
      return code;
    }
    pollGamepad(
      function (code) { injectKey(keyOf(code), code, 'keydown'); },
      function (code) { injectKey(keyOf(code), code, 'keyup'); }
    );
  }());

})();
