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

if ('ontouchstart' in window || navigator.maxTouchPoints > 0) {
  var isRally = window.location.pathname.indexOf('/rally/') !== -1;

  document.getElementById('touch-pad').style.display = 'block';

  var held = {};

  // Block game input while the overlay is up.
  window.addEventListener('keydown', function (e) {
    if (overlay.classList.contains('visible')) e.stopImmediatePropagation();
  }, true);

  function injectKey(key, code, type) {
    if (overlay.classList.contains('visible')) return;
    window.dispatchEvent(new KeyboardEvent(type, {
      bubbles: true, cancelable: true, key: key, code: code
    }));
  }

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
    // ---- Paddle dial ----
    document.getElementById('dpad').style.display = 'none';
    document.getElementById('btn-fire').style.display = 'none';
    var dial     = document.getElementById('paddle-dial');
    var dialHand = document.getElementById('dial-hand');
    dial.style.display = 'block';

    var dialAngle        = -Math.PI / 2; // hand points up initially
    var lastTouchAngle   = null;
    var totalDeltaAngle  = 0;
    var dialUpHeld = false, dialDownHeld = false;

    function touchAngleFromDial(touch) {
      var r = dial.getBoundingClientRect();
      return Math.atan2(touch.clientY - (r.top + r.height / 2),
                        touch.clientX - (r.left + r.width / 2));
    }

    function setDialDir(up, down) {
      if (up !== dialUpHeld) {
        dialUpHeld = up;
        injectKey('ArrowUp', 'ArrowUp', up ? 'keydown' : 'keyup');
      }
      if (down !== dialDownHeld) {
        dialDownHeld = down;
        injectKey('ArrowDown', 'ArrowDown', down ? 'keydown' : 'keyup');
      }
    }

    function stopDial() {
      setDialDir(false, false);
      dial.classList.remove('active');
      lastTouchAngle = null;
    }

    dial.addEventListener('touchstart', function (e) {
      e.preventDefault();
      lastTouchAngle  = touchAngleFromDial(e.touches[0]);
      totalDeltaAngle = 0;
      dial.classList.add('active');
    }, { passive: false });

    dial.addEventListener('touchmove', function (e) {
      e.preventDefault();
      if (lastTouchAngle === null) return;
      var a = touchAngleFromDial(e.touches[0]);
      var delta = a - lastTouchAngle;
      if (delta >  Math.PI) delta -= 2 * Math.PI;
      if (delta < -Math.PI) delta += 2 * Math.PI;
      lastTouchAngle = a;

      dialAngle       += delta;
      totalDeltaAngle += Math.abs(delta);
      dialHand.style.transform = 'rotate(' + (dialAngle + Math.PI / 2) + 'rad)';

      var DEAD = 0.018;
      if      (delta >  DEAD) setDialDir(false, true);
      else if (delta < -DEAD) setDialDir(true, false);
      else                    setDialDir(false, false);
    }, { passive: false });

    dial.addEventListener('touchend', function (e) {
      e.preventDefault();
      if (totalDeltaAngle < 0.08) {
        injectKey(' ', 'Space', 'keydown');
        injectKey(' ', 'Space', 'keyup');
      }
      stopDial();
    }, { passive: false });
    dial.addEventListener('touchcancel', stopDial);

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

})();
