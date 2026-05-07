(function () {
'use strict';

var TOPBAR_H  = 56;
var PAD       = 16; // padding around the canvas on all sides (= bezel width)

var loader    = document.getElementById('loader');
var barInner  = document.getElementById('bar-inner');
var statusEl  = document.getElementById('status');
var canvas    = document.getElementById('glcanvas');
var overlay   = document.getElementById('need-coin-overlay');

updateCoinsHud();

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
function playCoinInsert() {
  var ctx = getUiAudio(), t = ctx.currentTime;
  [{ freq: 1047, start: 0 }, { freq: 1319, start: 0.055 }].forEach(function (note) {
    var osc = ctx.createOscillator(), gain = ctx.createGain();
    osc.connect(gain); gain.connect(ctx.destination);
    osc.type = 'square'; osc.frequency.value = note.freq;
    gain.gain.setValueAtTime(0.22, t + note.start);
    gain.gain.exponentialRampToValueAtTime(0.001, t + note.start + 0.11);
    osc.start(t + note.start); osc.stop(t + note.start + 0.12);
  });
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
  ['coins-hud', 'insert-coin-btn'].forEach(function (id) {
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
  flashCoinBar();
  overlay.classList.remove('visible');
});

// ---- Canvas sizing ----
// The kiosk bar is position:fixed;bottom:0. We leave PAD px on each side
// plus full clearance for the bar so it never overlaps the canvas.

function fillCanvas() {
  var tb = document.getElementById('topbar');
  // clamp to 56 so a mis-read of 0 before layout doesn't eat the whole screen
  TOPBAR_H = tb ? Math.max(tb.offsetHeight, 56) : 56;
  var w = window.innerWidth  - PAD * 2;
  var h = window.innerHeight - TOPBAR_H - PAD * 2;
  canvas.style.setProperty('width',  w + 'px', 'important');
  canvas.style.setProperty('height', h + 'px', 'important');
  canvas.style.setProperty('top',    PAD + 'px', 'important');
  canvas.style.setProperty('left',   PAD + 'px', 'important');
  canvas.style.setProperty('transform', 'none', 'important');
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

if ('ontouchstart' in window || navigator.maxTouchPoints > 0) {
  var isRally = window.location.pathname.indexOf('/rally/') !== -1;

  document.getElementById('touch-pad').style.display = 'block';

  var held = {};

  function pressBtn(el) {
    var code = el.dataset.code;
    if (held[code]) return;
    held[code] = true;
    el.classList.add('active');
    injectKey(el.dataset.key, code, 'keydown');
  }

  function releaseBtn(el) {
    var code = el.dataset.code;
    if (!held[code]) return;
    held[code] = false;
    el.classList.remove('active');
    injectKey(el.dataset.key, code, 'keyup');
  }

  function releaseAll() {
    document.querySelectorAll('[data-key].active').forEach(function (el) {
      releaseBtn(el);
    });
  }

  if (isRally) {
    // ---- Dual paddle dials (P1 left = Arrow keys, P2 right = I/K) ----
    document.getElementById('dpad').style.display = 'none';
    document.getElementById('btn-fire').style.display = 'none';

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

  } else {
    var dpad = document.getElementById('dpad');
    dpad.addEventListener('touchstart', function (e) {
      e.preventDefault();
      for (var i = 0; i < e.changedTouches.length; i++) {
        var t = e.changedTouches[i];
        var el = document.elementFromPoint(t.clientX, t.clientY);
        if (el && el.dataset.code) pressBtn(el);
      }
    }, { passive: false });

    dpad.addEventListener('touchmove', function (e) {
      e.preventDefault();
      releaseAll();
      for (var i = 0; i < e.touches.length; i++) {
        var t = e.touches[i];
        var el = document.elementFromPoint(t.clientX, t.clientY);
        if (el && el.dataset.code) pressBtn(el);
      }
    }, { passive: false });

    dpad.addEventListener('touchend',    function (e) { e.preventDefault(); releaseAll(); }, { passive: false });
    dpad.addEventListener('touchcancel', releaseAll);
  }

  var fire = document.getElementById('btn-fire');
  fire.addEventListener('touchstart',  function (e) { e.preventDefault(); pressBtn(fire); },  { passive: false });
  fire.addEventListener('touchend',    function (e) { e.preventDefault(); releaseBtn(fire); }, { passive: false });
  fire.addEventListener('touchcancel', function ()  { releaseBtn(fire); });

  var fire2 = document.getElementById('btn-fire2');
  if (fire2) {
    fire2.addEventListener('touchstart',  function (e) { e.preventDefault(); pressBtn(fire2); },  { passive: false });
    fire2.addEventListener('touchend',    function (e) { e.preventDefault(); releaseBtn(fire2); }, { passive: false });
    fire2.addEventListener('touchcancel', function ()  { releaseBtn(fire2); });
  }

  // ---- Tilt (DeviceOrientation) controls ----
  (function () {
    var DEAD       = 12;   // degrees dead zone
    var tiltActive = false;
    var tiltBase   = null; // calibrated on first sample after enable
    var tiltHeld   = { up: false, down: false, left: false, right: false };

    function tiltKey(key, code, want, held) {
      if (want !== held) injectKey(key, code, want ? 'keydown' : 'keyup');
      return want;
    }

    function releaseAllTilt() {
      if (tiltHeld.up)    injectKey('ArrowUp',    'ArrowUp',    'keyup');
      if (tiltHeld.down)  injectKey('ArrowDown',  'ArrowDown',  'keyup');
      if (tiltHeld.left)  injectKey('ArrowLeft',  'ArrowLeft',  'keyup');
      if (tiltHeld.right) injectKey('ArrowRight', 'ArrowRight', 'keyup');
      tiltHeld = { up: false, down: false, left: false, right: false };
    }

    function onOrientation(e) {
      var g = e.gamma || 0, b = e.beta || 0;
      if (!tiltBase) tiltBase = { g: g, b: b };
      var dg = g - tiltBase.g, db = b - tiltBase.b;
      if (isRally) {
        // Left-right tilt (gamma) → P1 paddle up / down
        tiltHeld.up   = tiltKey('ArrowUp',    'ArrowUp',    dg < -DEAD, tiltHeld.up);
        tiltHeld.down = tiltKey('ArrowDown',  'ArrowDown',  dg >  DEAD, tiltHeld.down);
      } else {
        // Left-right tilt → ArrowLeft / ArrowRight
        // Forward-back tilt (beta delta) → ArrowUp / ArrowDown
        tiltHeld.left  = tiltKey('ArrowLeft',  'ArrowLeft',  dg < -DEAD, tiltHeld.left);
        tiltHeld.right = tiltKey('ArrowRight', 'ArrowRight', dg >  DEAD, tiltHeld.right);
        tiltHeld.up    = tiltKey('ArrowUp',    'ArrowUp',    db < -DEAD, tiltHeld.up);
        tiltHeld.down  = tiltKey('ArrowDown',  'ArrowDown',  db >  DEAD, tiltHeld.down);
      }
    }

    // Tilted phone with bilateral rotation arcs
    var ICON = '<svg width="22" height="22" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">'
      + '<rect x="6" y="3" width="8" height="14" rx="1.5" transform="rotate(-18 10 10)"/>'
      + '<path d="M2.5,9 C2,5 5,1.5 9,1"/>'
      + '<polyline points="7.5,0 9,1 8.5,2.5"/>'
      + '<path d="M17.5,11 C18,15 15,18.5 11,19"/>'
      + '<polyline points="12.5,20 11,19 11.5,17.5"/>'
      + '</svg>';

    var btn = document.createElement('button');
    btn.id        = 'tilt-btn';
    btn.title     = 'Toggle tilt controls';
    btn.innerHTML = ICON;
    var corner = document.createElement('div');
    corner.className = 'tilt-corner';
    corner.appendChild(btn);
    document.body.appendChild(corner);

    btn.addEventListener('click', function () {
      if (tiltActive) {
        tiltActive = false;
        tiltBase   = null;
        btn.classList.remove('tilt-on');
        window.removeEventListener('deviceorientation', onOrientation);
        releaseAllTilt();
        return;
      }
      function enable() {
        tiltActive = true;
        btn.classList.add('tilt-on');
        window.addEventListener('deviceorientation', onOrientation);
      }
      if (typeof DeviceOrientationEvent !== 'undefined' &&
          typeof DeviceOrientationEvent.requestPermission === 'function') {
        // iOS 13+ requires a permission prompt on a user gesture
        DeviceOrientationEvent.requestPermission()
          .then(function (s) { if (s === 'granted') enable(); })
          .catch(function () {});
      } else {
        enable();
      }
    });
  }());
}

  // ---- Gamepad support ----
  (function () {
    var DEADZONE = 0.25;
    var gpHeld   = {};

    var BTN_MAP = [
      { idx: 0,  code: 'Space'       },  // A / Cross   — Button 1
      { idx: 1,  code: 'KeyZ'        },  // B / Circle  — Button 2
      { idx: 2,  code: 'Space'       },
      { idx: 3,  code: 'Space'       },
      { idx: 9,  code: 'Space'       },
      { idx: 12, code: 'ArrowUp'    },
      { idx: 13, code: 'ArrowDown'  },
      { idx: 14, code: 'ArrowLeft'  },
      { idx: 15, code: 'ArrowRight' },
    ];

    function keyOf(code) {
      if (code === 'Space') return ' ';
      if (code === 'KeyZ')  return 'z';
      return code;
    }

    var polling = false;

    function poll() {
      var pads = navigator.getGamepads ? navigator.getGamepads() : [];
      var pad = null;
      for (var i = 0; i < pads.length; i++) {
        if (pads[i] && pads[i].connected) { pad = pads[i]; break; }
      }
      if (!pad) { polling = false; return; }

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

      var code;
      for (code in want) {
        if (!gpHeld[code]) { gpHeld[code] = true;  injectKey(keyOf(code), code, 'keydown'); }
      }
      for (code in gpHeld) {
        if (gpHeld[code] && !want[code]) { gpHeld[code] = false; injectKey(keyOf(code), code, 'keyup'); }
      }

      requestAnimationFrame(poll);
    }

    window.addEventListener('gamepadconnected', function () {
      if (!polling) { polling = true; requestAnimationFrame(poll); }
    });
  }());

})();
