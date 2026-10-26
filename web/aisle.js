/* Walking the arcade's aisle: where you stand (a decimal, in machines), how fast you walk, where you
 * stop, and how loud each machine is from there. No DOM, so test/aisle.test.mjs runs it in node. */
var BlipAisle = (function () {
  var W = 9;        // the settling spring: a tap from rest arrives in about 0.8 s and never overshoots
  var RAMP = 0.13;  // a held key reaches walking speed in about 0.4 s
  var HOLD = 0.2;   // a press held this long (s) walks; a shorter one is a tap
  var STEP = 1 / 120, MAX_DT = 0.25;

  function mod(a, n) { return ((a % n) + n) % n; }

  function create(n, pos) {
    pos = pos || 0;
    return { n: n, pos: pos, v: 0, target: pos, settled: true, drag: null };
  }

  // The machine a coasting walk stops at: the nearest one ahead that it can reach without overshooting.
  function coastTarget(s) {
    if (s.v > 0.05) return Math.ceil(s.pos + s.v / W - 1e-6);
    if (s.v < -0.05) return Math.floor(s.pos + s.v / W + 1e-6);
    return Math.round(s.pos);
  }

  // A tap: one machine on from where the walk was going to stop.
  function press(s, dir) {
    var base = s.target != null ? s.target : Math.round(s.pos);
    var next = base + (dir < 0 ? -1 : 1);
    if (Math.abs(next - Math.round(s.pos)) <= s.n) s.target = next;
    s.settled = false;
  }

  function integrate(s, input, h) {
    var dir = input.dir || 0;
    // While a key is down the walk speeds up evenly; whether it was a tap is known only when it is let go.
    if (dir) {
      if ((input.heldFor || 0) >= HOLD) s.target = null;
      s.v += (dir * (input.speed || 1) - s.v) * (1 - Math.exp(-h / RAMP));
      s.pos += s.v * h;
      return;
    }
    if (s.target == null) s.target = coastTarget(s);
    s.v += (-W * W * (s.pos - s.target) - 2 * W * s.v) * h;
    s.pos += s.v * h;
  }

  // Advances the walk by dt seconds; input is {dir, heldFor, speed}. A long gap (a tab coming back) counts as a quarter second.
  function step(s, input, dt) {
    if (s.drag) return s;
    var left = Math.min(Math.max(dt || 0, 0), MAX_DT);
    while (left > 1e-9) {
      var h = Math.min(left, STEP);
      integrate(s, input || {}, h);
      left -= h;
    }
    if (s.target != null && Math.abs(s.pos - s.target) < 4e-3 && Math.abs(s.v) < 3e-2) {
      s.target = mod(s.target, s.n);
      s.pos = s.target;
      s.v = 0;
      s.settled = true;
    } else {
      s.settled = false;
    }
    return s;
  }

  // A finger on the aisle moves it directly; letting go keeps the last 0.1 s of its speed.
  function dragStart(s, now) {
    s.drag = { from: s.pos, samples: [[now, s.pos]] };
    s.target = null;
    s.v = 0;
    s.settled = false;
  }
  function dragTo(s, pos, now) {
    if (!s.drag) return;
    s.pos = pos;
    var samples = s.drag.samples;
    samples.push([now, pos]);
    while (samples.length > 2 && now - samples[0][0] > 100) samples.shift();
  }
  // snapBack: the drag was too short to count, so the aisle returns to where it began.
  function dragEnd(s, now, snapBack) {
    if (!s.drag) return;
    var samples = s.drag.samples, first = samples[0], last = samples[samples.length - 1];
    var from = Math.round(s.drag.from), span = (last[0] - first[0]) / 1000;
    s.drag = null;
    s.v = span > 0.008 && now - last[0] < 100 ? Math.max(-6, Math.min(6, (last[1] - first[1]) / span)) : 0;
    if (snapBack) { s.v = 0; s.target = from; return; }
    s.target = coastTarget(s);
    var dir = Math.sign(s.pos - from);
    if (dir) {
      // Let the distance dragged set the farthest stop. A phone flick can have
      // high release velocity without the finger ever passing a second machine.
      var furthest = from + dir * Math.max(1, Math.round(Math.abs(s.pos - from)));
      if (dir * (s.target - furthest) > 0 || s.target === from) s.target = furthest;
    }
  }

  // Where machine i stands from you, in machines: -n/2 up to n/2.
  function offset(i, pos, n) { return mod(i - pos + n / 2, n) - n / 2; }
  function index(pos, n) { return mod(Math.round(pos), n); }

  // How far a room layer that repeats every `period` px has scrolled when you have walked `travel` machines,
  // `spacing` px apart, the layer moving at `rate` times the machines' pace: a translateX in (-period, 0].
  function layerOffset(travel, spacing, rate, period) {
    var x = -mod(travel * spacing * rate, period);
    return x === 0 ? 0 : x;
  }

  function machineLevel(d) { return 1 / (1 + d * d); }
  function machinePan(off) { return Math.max(-1, Math.min(1, off / 2)); }

  return {
    HOLD: HOLD, create: create, press: press, step: step,
    dragStart: dragStart, dragTo: dragTo, dragEnd: dragEnd,
    offset: offset, index: index, layerOffset: layerOffset, machineLevel: machineLevel, machinePan: machinePan
  };
}());
if (typeof module !== 'undefined') module.exports = BlipAisle;
