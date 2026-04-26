// iOS Safari suspends AudioContext until a user gesture. Wrap the constructor
// to track every instance and resume them all on every interaction.
(function () {
  var Orig = window.AudioContext || window.webkitAudioContext;
  if (!Orig) return;

  var ctxs = [];

  function tryResume(ctx) {
    if (ctx.state !== 'running') ctx.resume().catch(function () {});
  }

  function Wrapped(opts) {
    var ctx = new Orig(opts);
    ctxs.push(ctx);
    tryResume(ctx); // works when created inside a gesture; silent no-op otherwise
    return ctx;
  }
  Wrapped.prototype = Orig.prototype;
  window.AudioContext = window.webkitAudioContext = Wrapped;

  function unlock() {
    ctxs.forEach(tryResume);
  }

  // capture:true ensures we fire before any game handler that calls preventDefault
  ['touchstart', 'touchend', 'pointerdown', 'click', 'keydown'].forEach(function (e) {
    document.addEventListener(e, unlock, { passive: true, capture: true });
  });

  // Re-resume after the tab is foregrounded (iOS suspends contexts on hide)
  document.addEventListener('visibilitychange', function () {
    if (!document.hidden) ctxs.forEach(tryResume);
  });
}());
