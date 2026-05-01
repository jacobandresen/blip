(function () {
'use strict';

var TOPBAR_H  = 56;
var PAD       = 10; // padding around the canvas on all sides

var loader    = document.getElementById('loader');
var barInner  = document.getElementById('bar-inner');
var statusEl  = document.getElementById('status');
var canvas    = document.getElementById('glcanvas');
var overlay   = document.getElementById('need-coin-overlay');

updateCoinsHud();

// ---- UI audio (coin insert / no room) ----

var uiAudio = null;
function getUiAudio() {
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

    // Tap anywhere on the canvas to send Space (start game / launch ball).
    // stopPropagation is NOT called so macroquad still gets the event.
    canvas.addEventListener('touchstart', function (e) {
      e.preventDefault();
      injectKey(' ', 'Space', 'keydown');
      injectKey(' ', 'Space', 'keyup');
    }, { passive: false });

    function makeDial(dialEl, handEl, upKey, upCode, downKey, downCode, tapKey, tapCode) {
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
          injectKey(tapKey, tapCode, 'keydown');
          injectKey(tapKey, tapCode, 'keyup');
        }
        stop();
      }, { passive: false });

      dialEl.addEventListener('touchcancel', stop);
    }

    makeDial(
      dialP1, document.getElementById('dial-hand'),
      'ArrowUp', 'ArrowUp', 'ArrowDown', 'ArrowDown',
      ' ', 'Space'
    );
    // P2 dial: tap sends '2' (selects 2-player on title screen)
    makeDial(
      dialP2, document.getElementById('dial-hand-p2'),
      'i', 'KeyI', 'k', 'KeyK',
      '2', 'Digit2'
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
}

  // ---- Gamepad support ----
  (function () {
    var DEADZONE = 0.25;
    var gpHeld   = {};

    var BTN_MAP = [
      { idx: 0,  code: 'Space'       },
      { idx: 1,  code: 'Space'       },
      { idx: 2,  code: 'Space'       },
      { idx: 3,  code: 'Space'       },
      { idx: 9,  code: 'Space'       },
      { idx: 12, code: 'ArrowUp'    },
      { idx: 13, code: 'ArrowDown'  },
      { idx: 14, code: 'ArrowLeft'  },
      { idx: 15, code: 'ArrowRight' },
    ];

    function keyOf(code) { return code === 'Space' ? ' ' : code; }

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
