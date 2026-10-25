import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const Aisle = createRequire(import.meta.url)('../web/aisle.js');
const N = 9;

function run(s, input, seconds, dt = 1 / 60) {
  for (let t = 0; t < seconds; t += dt) Aisle.step(s, input, dt);
  return s;
}
function settle(s, limit = 5) {
  for (let t = 0; t < limit && !s.settled; t += 1 / 60) Aisle.step(s, {}, 1 / 60);
  assert.ok(s.settled, `settled at ${s.pos}`);
  return s;
}

test('a tap from rest settles one machine on, from every machine and across the wrap', () => {
  for (let start = 0; start < N; start++) {
    for (const dir of [1, -1]) {
      const s = Aisle.create(N, start);
      Aisle.press(s, dir);
      settle(s);
      assert.equal(s.pos, (start + dir + N) % N);
    }
  }
});

test('a tap arrives within a second and never overshoots', () => {
  const s = Aisle.create(N, 2);
  Aisle.press(s, 1);
  let most = 2, t = 0;
  while (!s.settled) { Aisle.step(s, {}, 1 / 60); t += 1 / 60; most = Math.max(most, s.pos); }
  assert.ok(t < 1, `took ${t.toFixed(2)} s`);
  assert.ok(most <= 3 + 2e-3, `reached ${most}`);
});

test('two quick taps move exactly two', () => {
  const s = Aisle.create(N, 0);
  Aisle.press(s, 1);
  run(s, {}, 0.1);
  Aisle.press(s, 1);
  assert.equal(settle(s).pos, 2);
});

test('a tap the other way while walking turns back', () => {
  const s = Aisle.create(N, 4);
  Aisle.press(s, 1);
  run(s, {}, 0.2);
  Aisle.press(s, -1);
  assert.equal(settle(s).pos, 4);
});

test('holding walks about a machine a second, and letting go stops on a machine ahead', () => {
  const s = Aisle.create(N, 0);
  Aisle.press(s, 1);
  let held = 0;
  for (; held < 3; held += 1 / 60) Aisle.step(s, { dir: 1, heldFor: held }, 1 / 60);
  assert.ok(Math.abs(s.v - 1) < 0.02, `speed ${s.v}`);
  assert.ok(s.pos > 2.5 && s.pos < 3.2, `walked ${s.pos}`);
  const released = s.pos;
  settle(s);
  // The next machine, or the one after when the next is too close to stop at gently.
  assert.ok([Math.ceil(released) % N, (Math.ceil(released) + 1) % N].includes(s.pos), `${released} -> ${s.pos}`);
});

test('a held walk picks its speed', () => {
  const slow = Aisle.create(N, 0), fast = Aisle.create(N, 0);
  run(slow, { dir: 1, heldFor: 1, speed: 0.4 }, 2);
  run(fast, { dir: 1, heldFor: 1, speed: 1.5 }, 2);
  assert.ok(fast.pos > slow.pos * 2.5, `${fast.pos} vs ${slow.pos}`);
});

test('a flick settles on a whole machine ahead within a second', () => {
  const s = Aisle.create(N, 0);
  Aisle.dragStart(s, 0);
  for (let t = 0; t <= 150; t += 16) Aisle.dragTo(s, t / 1000 * 4, t);
  Aisle.dragEnd(s, 150);
  assert.ok(s.v > 3, `kept ${s.v}`);
  const from = s.pos;
  let t = 0;
  while (!s.settled) { Aisle.step(s, {}, 1 / 60); t += 1 / 60; }
  assert.ok(t < 1, `took ${t.toFixed(2)} s`);
  assert.ok(Number.isInteger(s.pos));
  assert.ok((s.pos - Math.floor(from) + N) % N >= 1, `${from} -> ${s.pos}`);
});

test('a slow drag past half a machine still moves one; a short one springs back', () => {
  const on = Aisle.create(N, 3);
  Aisle.dragStart(on, 0);
  Aisle.dragTo(on, 3.3, 500);
  Aisle.dragTo(on, 3.3, 900);
  Aisle.dragEnd(on, 1000);
  assert.equal(settle(on).pos, 4);
  const back = Aisle.create(N, 3);
  Aisle.dragStart(back, 0);
  Aisle.dragTo(back, 3.05, 50);
  Aisle.dragEnd(back, 60, true);
  assert.equal(settle(back).pos, 3);
});

test('a long gap or none at all leaves the walk sound', () => {
  const s = Aisle.create(N, 0);
  Aisle.press(s, 1);
  Aisle.step(s, {}, 0);
  Aisle.step(s, {}, 2);
  Aisle.step(s, { dir: 1, heldFor: 5 }, 2);
  assert.ok(Number.isFinite(s.pos) && Number.isFinite(s.v));
  settle(s);
});

test('every machine stands within half the aisle of you', () => {
  for (let pos = 0; pos < N; pos += 0.01) {
    const seen = [];
    for (let i = 0; i < N; i++) {
      const off = Aisle.offset(i, pos, N);
      assert.ok(off >= -N / 2 && off < N / 2, `${i} at ${pos}: ${off}`);
      seen.push(Math.round(off * 100));
    }
    assert.equal(new Set(seen).size, N);
  }
  assert.equal(Aisle.offset(0, 8, N), 1);
  assert.equal(Aisle.index(8.6, N), 0);
});

test('a room layer scrolls within one repeat of its pattern, the way the old formulas did', () => {
  const spacing = 414;
  const old = (pos, rate, period) => -((pos * spacing * rate) % period + period) % period;
  for (const [rate, period] of [[0.5, 120], [1.3, 86], [0.3, 480]]) {
    for (let travel = -20; travel <= 20; travel += 0.013) {
      const x = Aisle.layerOffset(travel, spacing, rate, period);
      assert.ok(x > -period && x <= 0, `${travel}: ${x}`);
      assert.ok(Math.abs(x - old(travel, rate, period)) < 1e-9 || Math.abs(Math.abs(x - old(travel, rate, period)) - period) < 1e-9,
        `${travel}: ${x} vs ${old(travel, rate, period)}`);
      // One repeat of the pattern later, in machines walked, the layer looks the same.
      const repeat = period / (spacing * rate);
      const later = Aisle.layerOffset(travel + repeat, spacing, rate, period);
      const apart = Math.abs(later - x);
      assert.ok(apart < 1e-6 || Math.abs(apart - period) < 1e-6, `${travel} and one repeat on: ${x}, ${later}`);
    }
  }
  assert.ok(Object.is(Aisle.layerOffset(0, spacing, 1, 120), 0), 'no negative zero');
  assert.ok(Object.is(Aisle.layerOffset(-2, 60, 1, 120), 0), 'no negative zero walking back');
  // Walking on moves the layer left until it wraps; walking back moves it right.
  const a = Aisle.layerOffset(0.1, spacing, 0.5, 120), b = Aisle.layerOffset(0.15, spacing, 0.5, 120);
  assert.ok(b < a, `${a} then ${b}`);
  assert.ok(Aisle.layerOffset(-0.15, spacing, 0.5, 120) > Aisle.layerOffset(-0.1, spacing, 0.5, 120));
});

test('the faced machine is loudest and the others fall away and pan apart', () => {
  assert.equal(Aisle.machineLevel(0), 1);
  assert.equal(Aisle.machineLevel(1), 0.5);
  for (let d = 0; d < 4; d += 0.25) assert.ok(Aisle.machineLevel(d + 0.25) < Aisle.machineLevel(d));
  for (let off = -5; off <= 5; off += 0.5) {
    assert.equal(Aisle.machinePan(-off), -Aisle.machinePan(off));
    assert.ok(Math.abs(Aisle.machinePan(off)) <= 1);
  }
});

test('every machine has an attract strip of twelve frames, small enough to fetch as you walk, cached offline', async () => {
  const { readFileSync, statSync } = await import('node:fs');
  const web = new URL('../web/', import.meta.url);
  const sw = readFileSync(new URL('sw.js', web), 'utf8');
  for (const slug of ['rally', 'bouncer', 'defender', 'bubbler', 'raider', 'meteors', 'serpent', 'brawler', 'adder']) {
    const file = new URL(`${slug}/attract.webp`, web);
    const bytes = readFileSync(file);
    assert.equal(bytes.toString('latin1', 8, 12), 'WEBP', `${slug} is a WebP`);
    assert.ok(statSync(file).size <= 60 * 1024, `${slug}: ${statSync(file).size} bytes`);
    assert.ok(sw.includes(`"/blip/${slug}/attract.webp"`), `${slug}'s strip is in sw.js`);
    const png = readFileSync(new URL(`${slug}/attract.png`, web));
    assert.equal(png.toString('latin1', 1, 4), 'PNG', `${slug} has a PNG fallback`);
    assert.ok(png.length <= 256 * 1024, `${slug}: PNG fallback is ${png.length} bytes`);
    assert.equal(png.readUInt32BE(20) % 12, 0, `${slug}'s PNG contains twelve equal-height frames`);
    assert.ok(sw.includes(`"/blip/${slug}/attract.png"`), `${slug}'s PNG fallback is cached offline`);
  }
});
