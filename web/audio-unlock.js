// iOS Safari suspends AudioContext until a user gesture. Wrap the constructor
// to track every instance and resume them all on the first interaction.
(function () {
  var Orig = window.AudioContext || window.webkitAudioContext;
  if (!Orig) return;

  var ctxs = [], fired = false;

  function Wrapped(opts) {
    var ctx = new Orig(opts);
    ctxs.push(ctx);
    if (fired) ctx.resume();
    return ctx;
  }
  Wrapped.prototype = Orig.prototype;
  window.AudioContext = window.webkitAudioContext = Wrapped;

  function unlock() {
    if (fired) return;
    fired = true;
    ctxs.forEach(function (c) { if (c.state === 'suspended') c.resume(); });
  }
  ['touchstart', 'click', 'keydown'].forEach(function (e) {
    document.addEventListener(e, unlock, { passive: true });
  });
}());
