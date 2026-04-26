// iOS Safari suspends AudioContext until a user gesture and requires
// actually playing a buffer (not just resume()) to fully unlock hardware.
(function () {
  var Orig = window.AudioContext || window.webkitAudioContext;
  if (!Orig) return;

  var ctxs = [];
  var unlocked = false;

  function playSilentBuffer(ctx) {
    try {
      var buf = ctx.createBuffer(1, 1, 22050);
      var src = ctx.createBufferSource();
      src.buffer = buf;
      src.connect(ctx.destination);
      src.start(0);
    } catch (e) {}
  }

  function tryUnlock(ctx) {
    if (ctx.state === 'running') return;
    var p = ctx.resume();
    if (p && p.then) {
      p.then(function () { playSilentBuffer(ctx); }).catch(function () {});
    } else {
      playSilentBuffer(ctx);
    }
  }

  function Wrapped(opts) {
    var ctx = new Orig(opts);
    ctxs.push(ctx);
    tryUnlock(ctx);
    return ctx;
  }
  Wrapped.prototype = Orig.prototype;
  window.AudioContext = window.webkitAudioContext = Wrapped;

  function unlock() {
    ctxs.forEach(tryUnlock);
    // Once all contexts are running, reduce listener overhead.
    if (ctxs.length && ctxs.every(function (c) { return c.state === 'running'; })) {
      unlocked = true;
      EVENTS.forEach(function (e) {
        document.removeEventListener(e, unlock, { capture: true });
      });
    }
  }

  var EVENTS = ['touchstart', 'touchend', 'pointerdown', 'click', 'keydown'];

  // capture:true fires before any handler that calls preventDefault
  EVENTS.forEach(function (e) {
    document.addEventListener(e, unlock, { passive: true, capture: true });
  });

  // Re-unlock when tab is foregrounded — iOS suspends contexts on hide
  document.addEventListener('visibilitychange', function () {
    if (!document.hidden) ctxs.forEach(tryUnlock);
  });
}());
