var TOPBAR_H  = 56;

var loader    = document.getElementById('loader');
var barInner  = document.getElementById('bar-inner');
var statusEl  = document.getElementById('status');
var canvas    = document.getElementById('canvas');
var overlay   = document.getElementById('need-coin-overlay');
var totalDeps = 0;

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

// ---- Canvas scaling — centred in the area above the topbar ----

function fillCanvas() {
  var nw = canvas.width, nh = canvas.height;
  if (!nw || !nh) return;
  var tb = document.getElementById('topbar');
  TOPBAR_H = tb ? tb.offsetHeight : 56;
  var avH   = window.innerHeight - TOPBAR_H;
  var scale = Math.min(window.innerWidth / nw, avH / nh);
  var tx    = -(nw * scale / 2);
  var ty    = -(nh * scale / 2) - TOPBAR_H / 2;
  canvas.style.setProperty(
    'transform',
    'translate(' + tx + 'px,' + ty + 'px) scale(' + scale + ')',
    'important'
  );
}
window.addEventListener('resize', fillCanvas);
new MutationObserver(fillCanvas).observe(canvas, {
  attributes: true, attributeFilter: ['width', 'height']
});

canvas.addEventListener('webglcontextlost', function (e) {
  e.preventDefault();
  alert('WebGL context lost. Please reload the page.');
}, false);

// ---- Emscripten Module hooks ----

var Module = {
  canvas: canvas,
  setStatus: function (text) {
    if (!text) {
      loader.style.display = 'none';
      canvas.focus();
      return;
    }
    var m = text.match(/([^(]+)\((\d+(\.\d+)?)\/(\d+)\)/);
    if (m) {
      var pct = Math.round(parseInt(m[2]) / parseInt(m[4]) * 100);
      barInner.style.width = pct + '%';
      statusEl.textContent = 'LOADING ' + pct + '%';
    } else {
      statusEl.textContent = text.toUpperCase().slice(0, 24);
    }
  },
  monitorRunDependencies: function (left) {
    if (!totalDeps) totalDeps = left;
    if (totalDeps && left) {
      var pct = Math.round((totalDeps - left) / totalDeps * 100);
      barInner.style.width = pct + '%';
      statusEl.textContent = 'LOADING ' + pct + '%';
    }
  },
  print:    function () {},
  printErr: function (t) { console.warn(t); }
};

// ---- Touch controls — injected as keyboard events into the WASM game ----

if ('ontouchstart' in window || navigator.maxTouchPoints > 0) {
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

  var fire = document.getElementById('btn-fire');
  fire.addEventListener('touchstart',  function (e) { e.preventDefault(); pressBtn(fire); },  { passive: false });
  fire.addEventListener('touchend',    function (e) { e.preventDefault(); releaseBtn(fire); }, { passive: false });
  fire.addEventListener('touchcancel', function ()  { releaseBtn(fire); });
}
