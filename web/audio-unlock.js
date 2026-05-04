// iOS Safari suspends AudioContext until a user gesture.
(function () {
  var Orig = window.AudioContext || window.webkitAudioContext;
  if (!Orig) return;

  var ctxs = [];

  function tryResume(ctx) {
    if (ctx.state === 'running') return;
    ctx.resume().catch(function () {});
  }

  function Wrapped(opts) {
    var ctx = new Orig(opts);
    ctxs.push(ctx);
    tryResume(ctx);
    return ctx;
  }
  Wrapped.prototype = Orig.prototype;
  window.AudioContext = window.webkitAudioContext = Wrapped;

  function unlock() {
    ctxs.forEach(tryResume);
  }

  var EVENTS = ['touchstart', 'touchend', 'pointerdown', 'click', 'keydown'];

  // capture:true, passive:false — must be non-passive so iOS counts this as user activation
  EVENTS.forEach(function (e) {
    document.addEventListener(e, unlock, { passive: false, capture: true });
  });

  // Re-resume when tab is foregrounded — iOS suspends contexts on hide
  document.addEventListener('visibilitychange', function () {
    if (!document.hidden) ctxs.forEach(tryResume);
  });
}());
