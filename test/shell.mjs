// The game shell in a real (headless) Chromium, driven over CDP: the
// high-score prompt takes taps and keys, fullscreen leaves a PC the top bar
// and the picture, coins need no mouse. Needs only /usr/bin/chromium (see
// lib/chromium-binary.mjs), not Playwright.
//
//   node --test --test-concurrency=1 test/shell.mjs

import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { launch, evaluate, killAll, sleep } from './lib/cdp.mjs';

const WEB = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'web');
const PORT = 8096;
const ORIGIN = `http://127.0.0.1:${PORT}`;
const MIME = {
  '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css',
  '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png',
  '.svg': 'image/svg+xml', '.woff2': 'font/woff2',
};

function serve() {
  const server = createServer(async (req, res) => {
    const file = path.join(WEB, decodeURIComponent(req.url.split('?')[0]));
    try {
      const body = await readFile(file);
      res.writeHead(200, { 'content-type': MIME[path.extname(file)] || 'application/octet-stream' });
      res.end(body);
    } catch { res.writeHead(404); res.end(); }
  });
  return new Promise((ok) => server.listen(PORT, () => ok(server)));
}

/** A browser on `port` with its own profile, closed with the test. */
async function browser(t, port, { touch = false, width = 1280, height = 800 } = {}) {
  const args = [`--window-size=${width},${height}`, `--user-data-dir=${mkdtempSync(path.join(tmpdir(), 'blip-shell-'))}`];
  if (touch) args.push('--touch-events=enabled');
  const { proc, cdp } = await launch(port, args);
  t.after(() => killAll([proc]));
  await cdp.send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: touch });
  if (touch) await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: true });
  return cdp;
}

async function open(cdp, page, settle = 4000) {
  await cdp.send('Page.navigate', { url: `${ORIGIN}/${page}` });
  await sleep(settle);
}

async function key(cdp, k, code, vk) {
  for (const type of ['keyDown', 'keyUp']) {
    await cdp.send('Input.dispatchKeyEvent', { type, key: k, code, windowsVirtualKeyCode: vk });
    await sleep(100);
  }
}

const WALL = "document.getElementById('need-coin-overlay').classList.contains('visible')";
const centre = (sel) => `(function(){var r=document.querySelector('${sel}').getBoundingClientRect();return Math.round((r.top+r.height/2)*2)/2;})()`;

test('the game shell', async (t) => {
  const server = await serve();
  t.after(() => new Promise((ok) => server.close(ok)));

  await t.test('the name prompt takes a tap while touch play is on, and its keys while the coin wall is up', async (t) => {
    const cdp = await browser(t, 9401, { touch: true, width: 390, height: 844 });
    await open(cdp, 'bouncer/index.html');
    assert.equal(await evaluate(cdp, "document.documentElement.getAttribute('data-touch')"), 'drag');

    // Coins left, so touch play is live: every tap that is not a button plays.
    await evaluate(cdp, "document.getElementById('need-coin-overlay').classList.remove('visible'); window.blipScores.promptHandle('test'); true");
    await sleep(300);
    await evaluate(cdp, 'document.activeElement && document.activeElement.blur(); true');
    const at = JSON.parse(await evaluate(cdp, "(function(){var b=document.querySelector('.blip-hs-input').getBoundingClientRect();return JSON.stringify({x:b.left+b.width/2,y:b.top+b.height/2});})()"));
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [at] });
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    await sleep(300);
    assert.ok(await evaluate(cdp, "document.activeElement === document.querySelector('.blip-hs-input')"),
      'a tap on the name field did not focus it');

    // Out of coins the wall swallows keys for the game, not the prompt's.
    await evaluate(cdp, "document.getElementById('need-coin-overlay').classList.add('visible'); true");
    await key(cdp, 'Escape', 'Escape', 27);
    assert.ok(await evaluate(cdp, "!document.querySelector('.blip-hs-modal')"), 'Escape did not close the prompt');
  });

  await t.test('a name can be entered with a stick and the fire button', async (t) => {
    const cdp = await browser(t, 9406);
    await open(cdp, 'meteors/index.html', 6000);
    await evaluate(cdp, "localStorage.removeItem('blip-handle'); window.blipScores.promptHandle('test'); true");
    await sleep(800);     // the prompt takes no keys in its first half second
    const name = "document.querySelector('.blip-hs-input').value";
    const press = async (...keys) => {
      for (const k of keys) await key(cdp, k, k, { ArrowUp: 38, ArrowDown: 40, ArrowLeft: 37, ArrowRight: 39 }[k]);
    };
    await press('ArrowUp', 'ArrowUp', 'ArrowUp');           // A, B, C
    assert.equal(await evaluate(cdp, name), 'C');
    await press('ArrowRight', 'ArrowDown');                  // CA, then back round to C_
    assert.equal(await evaluate(cdp, name), 'C_');
    await press('ArrowLeft');
    assert.equal(await evaluate(cdp, name), 'C');
    // Fire accepts: one letter is too short, and the prompt says so.
    await key(cdp, ' ', 'Space', 32);
    assert.equal(await evaluate(cdp, name), 'C', 'fire typed a space instead of accepting');
    assert.match(await evaluate(cdp, "document.querySelector('.blip-hs-err').textContent"), /2-14/);
  });

  await t.test('fullscreen on a PC is the top bar and the picture, and comes off again', async (t) => {
    const cdp = await browser(t, 9402);
    await open(cdp, 'meteors/index.html', 6000);
    const state = `JSON.stringify((function(){var c=document.getElementById('glcanvas'),r=c.getBoundingClientRect();
      return {deck:getComputedStyle(document.getElementById('topbar')).display,box:[r.left,r.top,r.width,r.height],buf:[c.width,c.height],
              bar:document.getElementById('marquee-bar').getBoundingClientRect().bottom};})())`;
    const before = JSON.parse(await evaluate(cdp, state));
    assert.notEqual(before.deck, 'none');

    await evaluate(cdp, "document.getElementById('fullscreen-btn').click(); true");
    await sleep(600);
    const on = JSON.parse(await evaluate(cdp, state));
    assert.equal(on.deck, 'none', 'the deck is still shown');
    assert.deepEqual(on.box, [0, on.bar, 1280, 800 - on.bar], 'the picture does not run edge to edge under the bar');
    // No window resize happened: the drawing buffer has to follow anyway.
    assert.deepEqual(on.buf, [on.box[2], on.box[3]], 'the drawing buffer kept its old size');
    assert.equal(await evaluate(cdp, centre('#fullscreen-btn svg')), await evaluate(cdp, centre('#marquee-bar')),
      'the button is off the middle of the bar');

    await evaluate(cdp, "document.getElementById('fullscreen-btn').click(); true");
    await sleep(600);
    assert.deepEqual(JSON.parse(await evaluate(cdp, state)), before, 'turning it off did not restore the layout');
  });

  await t.test('the choice carries to the next page, where the cabinet drops its deck', async (t) => {
    const cdp = await browser(t, 9403, { width: 1920, height: 1080 });
    await open(cdp, 'index.html');
    await evaluate(cdp, "document.getElementById('fullscreen-btn').click(); true");
    await sleep(400);
    assert.equal(await evaluate(cdp, "getComputedStyle(document.getElementById('kiosk-bar')).display"), 'none');
    assert.equal(await evaluate(cdp, 'document.documentElement.scrollHeight <= innerHeight'), true,
      'the catalogue needs scrolling on a 1080p kiosk');
    assert.equal(await evaluate(cdp, centre('#fullscreen-btn svg')), await evaluate(cdp, centre('.top-marquee-bar')));

    await open(cdp, 'serpent/index.html', 5000);
    assert.ok(await evaluate(cdp, "document.documentElement.hasAttribute('data-fullscreen')"), 'the game page forgot the choice');
    assert.equal(await evaluate(cdp, "getComputedStyle(document.getElementById('topbar')).display"), 'none');

    // A mouse left lying loses its pointer.
    await cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 400, y: 400 });
    assert.equal(await evaluate(cdp, "document.documentElement.hasAttribute('data-cursor-idle')"), false);
    await sleep(3400);
    assert.equal(await evaluate(cdp, "document.documentElement.hasAttribute('data-cursor-idle')"), true);
  });

  await t.test('on the cabinet, up and down move the selection a row', async (t) => {
    const cdp = await browser(t, 9407);
    await open(cdp, 'index.html');
    const lit = "Array.prototype.findIndex.call(document.querySelectorAll('.card'), function (c) { return c.classList.contains('card-focused'); })";
    const cols = await evaluate(cdp, `(function(){var c=document.querySelectorAll('.card'),t=c[0].getBoundingClientRect().top,n=0;
      while(n<c.length&&Math.abs(c[n].getBoundingClientRect().top-t)<2)n++;return n;})()`);
    const n = await evaluate(cdp, "document.querySelectorAll('.card').length");
    assert.ok(cols > 1 && cols < n, `expected several rows of several cards, got ${cols} across of ${n}`);
    await key(cdp, 'ArrowRight', 'ArrowRight', 39);
    assert.equal(await evaluate(cdp, lit), 1);
    await key(cdp, 'ArrowDown', 'ArrowDown', 40);
    assert.equal(await evaluate(cdp, lit), 1 + cols);
    await key(cdp, 'ArrowUp', 'ArrowUp', 38);
    assert.equal(await evaluate(cdp, lit), 1);
    // Up from the top row goes round to the bottom of the same column.
    await key(cdp, 'ArrowUp', 'ArrowUp', 38);
    const to = await evaluate(cdp, lit);
    assert.ok(to % cols === 1 && to + cols >= n, `went to card ${to}`);
  });

  await t.test("on a phone the game's name fits between the buttons and the coin slot", async (t) => {
    const cdp = await browser(t, 9408, { touch: true, width: 390, height: 844 });
    await open(cdp, 'galactic_defender/index.html', 5000);   // the longest name
    const edges = (sel) => `(function(){var r=document.querySelector('${sel}').getBoundingClientRect();return JSON.stringify([r.left,r.right]);})()`;
    const at = async (sel) => JSON.parse(await evaluate(cdp, edges(sel)));
    const [name, slot, fs] = [await at('#marquee-name'), await at('#insert-coin-btn'), await at('#fullscreen-btn')];
    assert.ok(name[0] >= fs[1] && name[1] <= slot[0], `name ${name} between button ${fs} and slot ${slot}`);
    assert.equal(await evaluate(cdp, "getComputedStyle(document.getElementById('marquee-name')).visibility"), 'visible');

    // At 320px the slot drops its word to leave the name room.
    await cdp.send('Emulation.setDeviceMetricsOverride', { width: 320, height: 700, deviceScaleFactor: 1, mobile: true });
    await open(cdp, 'galactic_defender/index.html', 5000);
    const [narrow, slot320] = [await at('#marquee-name'), await at('#insert-coin-btn')];
    assert.equal(await evaluate(cdp, "getComputedStyle(document.getElementById('marquee-name')).visibility"), 'visible');
    assert.ok(narrow[1] <= slot320[0] && slot320[1] <= 320, `name ${narrow}, slot ${slot320}`);
    await cdp.send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 1, mobile: true });

    // An iPhone has no fullscreen to offer: the sound button stands alone.
    await cdp.send('Page.addScriptToEvaluateOnNewDocument', {
      source: 'delete Element.prototype.requestFullscreen; delete Element.prototype.webkitRequestFullscreen;',
    });
    await open(cdp, 'galactic_defender/index.html', 5000);
    assert.equal(await evaluate(cdp, "!!document.getElementById('fullscreen-btn')"), false);
    const [name2, mute] = [await at('#marquee-name'), await at('#mute-btn')];
    assert.ok(mute[1] > mute[0], 'no sound button');
    assert.ok(name2[0] >= mute[1] && name2[1] <= slot[0], `name ${name2} between button ${mute} and slot ${slot}`);
  });

  await t.test('a game that traps says so and goes back to the cabinet', async (t) => {
    const cdp = await browser(t, 9410);
    await open(cdp, 'serpent/index.html', 5000);
    await evaluate(cdp, "window.dispatchEvent(new ErrorEvent('error', { error: new WebAssembly.RuntimeError('unreachable') })); true");
    assert.equal(await evaluate(cdp, "document.getElementById('status').textContent"), 'GAME STOPPED');
    assert.notEqual(await evaluate(cdp, "getComputedStyle(document.getElementById('loader')).display"), 'none');
    await sleep(3500);
    assert.equal(await evaluate(cdp, 'location.pathname'), '/index.html');

    // A lost GL context loads the game again, with no dialog to hang on;
    // nor do the runtime's own alerts raise one.
    await open(cdp, 'serpent/index.html', 5000);
    assert.equal(await evaluate(cdp, "alert('x'); 'no dialog'"), 'no dialog');
    await evaluate(cdp, "window.__mark = 1; document.getElementById('glcanvas').dispatchEvent(new Event('webglcontextlost', { cancelable: true })); true");
    assert.equal(await evaluate(cdp, "document.getElementById('status').textContent"), 'ONE MOMENT');
    await sleep(4000);
    assert.deepEqual([await evaluate(cdp, 'location.pathname'), await evaluate(cdp, 'window.__mark || 0')], ['/serpent/index.html', 0]);
  });

  await t.test('no page is wider than a 360px phone', async (t) => {
    const cdp = await browser(t, 9409, { touch: true, width: 360, height: 760 });
    for (const page of ['index.html', 'controls.html', 'about.html', 'history.html']) {
      await open(cdp, page, 2500);
      // A page that overflows makes a phone lay it out wider and zoom out.
      assert.deepEqual(JSON.parse(await evaluate(cdp, 'JSON.stringify([innerWidth, document.documentElement.scrollWidth])')),
        [360, 360], page);
    }
  });

  await t.test('sound goes off and on from the button or M, and stays off across pages', async (t) => {
    const cdp = await browser(t, 9405);
    await open(cdp, 'meteors/index.html', 6000);
    const muted = "JSON.stringify([Howler._muted, document.documentElement.hasAttribute('data-muted'), localStorage.getItem('blip-mute')])";
    assert.equal(await evaluate(cdp, muted), '[false,false,null]');
    await evaluate(cdp, "document.getElementById('mute-btn').click(); true");
    assert.equal(await evaluate(cdp, muted), '[true,true,"1"]');
    // The cabinet's own sounds leave through one gain, shut with it.
    assert.equal(await evaluate(cdp, 'blipOut(getKioskAudio()).gain.value'), 0);

    await open(cdp, 'index.html');
    assert.equal(await evaluate(cdp, muted), '[true,true,"1"]', 'the next page forgot it');
    await key(cdp, 'm', 'KeyM', 77);
    assert.equal(await evaluate(cdp, muted), '[false,false,"0"]', 'M did not turn the sound back on');

    // Both buttons sit on the bar's middle line, the sound one left of the other.
    const x = (sel) => `document.querySelector('${sel}').getBoundingClientRect().left`;
    assert.equal(await evaluate(cdp, centre('#mute-btn svg')), await evaluate(cdp, centre('.top-marquee-bar')));
    assert.ok(await evaluate(cdp, `${x('#mute-btn')} < ${x('#fullscreen-btn')} && ${x('#fullscreen-btn')} < ${x('#kiosk-insert-btn')}`));
  });

  await t.test('coins need no mouse: fire at the wall, 5 at any time', async (t) => {
    const cdp = await browser(t, 9404);
    await open(cdp, 'meteors/index.html', 6000);
    await evaluate(cdp, "document.getElementById('glcanvas').focus(); true");
    assert.equal(await evaluate(cdp, WALL), true, 'no coin wall on arrival with no credit');
    await key(cdp, ' ', 'Space', 32);
    await sleep(400);
    assert.deepEqual([await evaluate(cdp, WALL), await evaluate(cdp, 'getCoins()')], [false, 1], 'fire at the wall did not pay it');
    await key(cdp, '5', 'Digit5', 53);
    assert.equal(await evaluate(cdp, 'getCoins()'), 2);

    // And Backspace is the way back to the cabinet.
    await key(cdp, 'Backspace', 'Backspace', 8);
    await sleep(1500);
    assert.equal(await evaluate(cdp, 'location.pathname'), '/index.html');
  });
});
