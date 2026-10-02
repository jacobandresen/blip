// High-score entry, end to end, in every game that keeps a board: the real
// wasm is played until it ends, and the name is entered three ways (typed,
// with a stick and fire, by touch) against a stand-in backend
// (lib/fake-supabase.js) that answers as supabase/migrations do.
//
//   node --test --test-concurrency=1 test/highscore-entry.mjs     (after ./build_web.sh)
//   GAMES=serpent,brawler node --test test/highscore-entry.mjs
//
// A game is left to lose on its own, so the run takes ten minutes or so.

import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { launch, evaluate, killAll, sleep, waitFor } from './lib/cdp.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const WEB = path.join(HERE, '..', 'web');
const PORT = 8098;
const ORIGIN = `http://127.0.0.1:${PORT}`;
const MIME = {
  '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css',
  '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png',
  '.svg': 'image/svg+xml', '.woff2': 'font/woff2',
};
// The backend's two files are replaced; everything else is the built site.
const FAKES = {
  '/vendor/supabase.js': () => readFile(path.join(HERE, 'lib', 'fake-supabase.js')),
  '/blip_config.js': async () => "window.BLIP_SUPABASE = { url: 'http://fake.invalid', anonKey: 'fake' };",
};

const SPACE = { key: ' ', code: 'Space', vk: 32 };
const F = { key: 'f', code: 'KeyF', vk: 70 };
// Fire is pressed once a second until the game takes its coin, then
// `presses` times more, and then the game is left alone to lose. Bouncer
// needs a launch after every lost ball; Brawler has screens to get through,
// and must not be pressed through its CONTINUE countdown.
const ALL = {
  serpent:           { fire: SPACE, presses: 1 },
  bouncer:           { fire: SPACE, presses: Infinity },
  galactic_defender: { fire: SPACE, presses: 1 },
  meteors:           { fire: SPACE, presses: 1 },
  sky_raider:        { fire: SPACE, presses: 2 },
  brawler:           { fire: F, presses: 7 },
  bubbler:           { fire: F, presses: 2 },
};
const only = (process.env.GAMES || '').split(',').filter(Boolean);
const GAMES = Object.entries(ALL).filter(([slug]) => !only.length || only.includes(slug));

function serve() {
  const server = createServer(async (req, res) => {
    const url = decodeURIComponent(req.url.split('?')[0]);
    try {
      const body = FAKES[url] ? await FAKES[url]() : await readFile(path.join(WEB, url));
      res.writeHead(200, { 'content-type': MIME[path.extname(url)] || 'application/octet-stream' });
      res.end(body);
    } catch { res.writeHead(404); res.end(); }
  });
  return new Promise((ok) => server.listen(PORT, () => ok(server)));
}

async function browser(t, port, { touch = false, width = 1280, height = 800 } = {}) {
  const profile = mkdtempSync(path.join(tmpdir(), 'blip-hs-'));
  const args = [`--window-size=${width},${height}`, `--user-data-dir=${profile}`];
  if (touch) args.push('--touch-events=enabled');
  const { proc, cdp } = await launch(port, args);
  // A profile is tens of megabytes, and /tmp is often memory: take it away.
  t.after(async () => {
    killAll([proc]);
    await sleep(300);
    rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  });
  await cdp.send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: touch });
  if (touch) await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: true });
  return cdp;
}

/** Load a game with five coins in, and note the score it reports. */
async function openGame(cdp, slug) {
  await cdp.send('Page.navigate', { url: `${ORIGIN}/${slug}/index.html` });
  await waitFor(cdp, "typeof window.blipGameOver === 'function' && document.getElementById('glcanvas').width > 0", 30000);
  await sleep(4000);
  assert.equal(await evaluate(cdp, 'window.blipScores && window.blipScores.enabled'), true, 'the board is not enabled');
  await evaluate(cdp, `(function () {
    for (var i = 0; i < 5; i++) document.getElementById('insert-coin-btn').click();
    var real = window.blipGameOver;
    window.blipGameOver = function (s) { if (window.__over === undefined) window.__over = s; return real(s); };
    document.getElementById('glcanvas').focus();
    return getCoins();
  })()`);
}

/** A real key press, as a keyboard or a cabinet's encoder sends it. */
async function key(cdp, k, { text } = {}) {
  const base = { key: k.key, code: k.code, windowsVirtualKeyCode: k.vk };
  await cdp.send('Input.dispatchKeyEvent', { type: 'keyDown', ...base, ...(text ? { text } : {}) });
  await sleep(80);     // held across a frame, as a finger holds it
  await cdp.send('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
  await sleep(80);
}
const ENTER = { key: 'Enter', code: 'Enter', vk: 13 };
const arrow = (d) => ({ key: 'Arrow' + d, code: 'Arrow' + d, vk: { Up: 38, Down: 40, Left: 37, Right: 39 }[d] });
function charKey(c) {
  if (c === ' ') return SPACE;
  if (/[0-9]/.test(c)) return { key: c, code: 'Digit' + c, vk: c.charCodeAt(0) };
  return { key: c, code: 'Key' + c.toUpperCase(), vk: c.toUpperCase().charCodeAt(0) };
}

async function tap(cdp, sel) {
  const at = JSON.parse(await evaluate(cdp, `(function(){var r=document.querySelector(${JSON.stringify(sel)}).getBoundingClientRect();
    return JSON.stringify({x:r.left+r.width/2,y:r.top+r.height/2});})()`));
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [at] });
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  await sleep(350);
}

const has = (sel) => `!!document.querySelector(${JSON.stringify(sel)})`;
const PROMPT = '.blip-hs-input';
const CODE = "Array.prototype.some.call(document.querySelectorAll('.blip-hs-modal'), function (m) { return m.textContent.indexOf('ABCD-EFGH-JKLM') >= 0; })";
const MY_ROW = `(function(){var li=document.querySelector('.blip-hs-list .blip-hs-item.me');
  return li ? JSON.stringify([li.querySelector('.hs-name').textContent, li.querySelector('.hs-score').textContent]) : 'none';})()`;
const db = async (cdp) => JSON.parse(await evaluate(cdp, "localStorage.getItem('fake-sb')"));
// As the board prints it, in the browser's own locale.
const shown = (cdp, n) => evaluate(cdp, `Number(${n}).toLocaleString()`);

/** Forget the name, as on a cabinet's next player, and end a game at `score`. */
async function nextPlayer(cdp, score) {
  await evaluate(cdp, `(function () {
    var d = JSON.parse(localStorage.getItem('fake-sb'));
    delete d.players.u1; d.scores = {}; d.calls = [];
    localStorage.setItem('fake-sb', JSON.stringify(d));
    localStorage.removeItem('blip-handle');
    window.blipGameOver(${score});
    return true;
  })()`);
  await waitFor(cdp, has(PROMPT), 8000);
  await waitFor(cdp, `document.activeElement === document.querySelector('${PROMPT}')`, 3000);
  await sleep(600);     // the prompt takes no keys in its first half second
}

test('high-score entry in every game', async (t) => {
  const server = await serve();
  t.after(() => new Promise((ok) => server.close(ok)));

  for (const [i, [slug, { fire, presses }]] of GAMES.entries()) {
    await t.test(`${slug}: played to the end, a name typed and one from the stick`, async (t) => {
      const cdp = await browser(t, 9420 + i);
      await openGame(cdp, slug);

      // Fire starts the game; then it is left to lose.
      const pressFire = `(function () {
        if (window.__over !== undefined) return;
        var c = document.getElementById('glcanvas');
        var o = { bubbles: true, cancelable: true, key: ${JSON.stringify(fire.key)}, code: '${fire.code}' };
        c.dispatchEvent(new KeyboardEvent('keydown', o));
        setTimeout(function () { c.dispatchEvent(new KeyboardEvent('keyup', o)); }, 80);
      })()`;
      const began = Date.now();
      for (let more = 0; ;) {
        if (await evaluate(cdp, 'window.__over !== undefined')) break;
        assert.ok(Date.now() - began < 420000, 'the game did not end in seven minutes');
        const started = (await evaluate(cdp, 'getCoins()')) < 5;
        if (!started || more < presses) {
          await evaluate(cdp, pressFire);
          if (started) more++;
        }
        await sleep(1000);
      }
      const over = await evaluate(cdp, 'window.__over');
      t.diagnostic(`${slug}: game over after ${Math.round((Date.now() - began) / 1000)}s with ${over}`);
      assert.ok(Number.isInteger(over) && over >= 0, `score reported: ${over}`);

      // A game that scored nothing shows the board and asks no name; the
      // entry is then checked with a score of 7.
      let first = over;
      if (over === 0) {
        await waitFor(cdp, has('.blip-hs-list'), 8000);
        assert.equal(await evaluate(cdp, has(PROMPT)), false, 'a name was asked for a score of 0');
        await sleep(1300);
        await key(cdp, fire);
        first = 7;
        await nextPlayer(cdp, first);
      }

      // 1. Typed. The letters are ones the shell also listens for (M mutes,
      //    5 is a coin, F and space are fire): the name must get them all.
      await waitFor(cdp, has(PROMPT), 8000);
      assert.equal(await evaluate(cdp, "document.querySelector('.blip-hs-title').textContent"), 'ENTER YOUR NAME');
      await waitFor(cdp, `document.activeElement === document.querySelector('${PROMPT}')`, 3000);
      // Fire hammered as the prompt comes up does nothing, and no name
      // starts with a space.
      await key(cdp, fire);
      await sleep(600);
      await key(cdp, SPACE);
      assert.equal(await evaluate(cdp, `document.querySelector('${PROMPT}').value`), '');
      const before = await evaluate(cdp, 'JSON.stringify([getCoins(), blipMuted(), location.pathname])');
      for (const c of 'am 5f') await key(cdp, charKey(c), { text: c });
      assert.equal(await evaluate(cdp, `document.querySelector('${PROMPT}').value`), 'am 5f');
      assert.equal(await evaluate(cdp, 'JSON.stringify([getCoins(), blipMuted(), location.pathname])'), before,
        'typing a name dropped a coin, muted the sound or left the page');
      await key(cdp, ENTER);
      await waitFor(cdp, CODE, 8000);          // the recovery code is shown once
      await sleep(300);
      await key(cdp, ENTER);
      await waitFor(cdp, has('.blip-hs-list'), 8000);
      assert.equal(await evaluate(cdp, MY_ROW), JSON.stringify(['am 5f', await shown(cdp, first)]));
      assert.deepEqual((await db(cdp)).calls, [
        { name: 'submit_score', args: { p_game: slug, p_score: first }, out: 'needs_handle' },
        { name: 'claim_handle', args: { p_handle: 'am 5f' }, out: 'ok' },
        { name: 'submit_score', args: { p_game: slug, p_score: first }, out: 'ok' },
      ]);
      assert.equal(await evaluate(cdp, "localStorage.getItem('blip-handle')"), 'am 5f');
      assert.equal(await evaluate(cdp, `localStorage.getItem('blip-best-${slug}')`), String(first));
      await sleep(1300);
      await key(cdp, fire);                      // any key puts the board away
      assert.equal(await evaluate(cdp, has('.blip-hs-modal')), false, 'the board stayed up');

      // 2. From the stick, by the next player. Left on an empty name skips,
      //    and nothing is claimed.
      await nextPlayer(cdp, first + 5);
      await key(cdp, arrow('Left'));
      assert.equal(await evaluate(cdp, has('.blip-hs-modal')), false, 'left did not skip the prompt');
      assert.deepEqual((await db(cdp)).calls.map((c) => c.name), ['submit_score']);
      //    B, then BA, then BB; fire accepts.
      await nextPlayer(cdp, first + 10);
      for (const d of ['Up', 'Up', 'Right', 'Up']) await key(cdp, arrow(d));
      assert.equal(await evaluate(cdp, `document.querySelector('${PROMPT}').value`), 'BB');
      await key(cdp, fire);
      await waitFor(cdp, CODE, 8000);
      await sleep(300);
      await key(cdp, fire);
      await waitFor(cdp, has('.blip-hs-list'), 8000);
      assert.equal(await evaluate(cdp, MY_ROW), JSON.stringify(['BB', await shown(cdp, first + 10)]));
      assert.equal((await db(cdp)).scores[slug].u1, first + 10);
      await sleep(1300);
      await key(cdp, fire);
      assert.equal(await evaluate(cdp, has('.blip-hs-modal')), false, 'the board stayed up');

      // The same name next time: no prompt, straight to the board.
      await evaluate(cdp, `window.blipGameOver(${first + 20}); true`);
      await waitFor(cdp, has('.blip-hs-list'), 8000);
      assert.equal(await evaluate(cdp, has(PROMPT)), false);
      assert.equal(await evaluate(cdp, MY_ROW), JSON.stringify(['BB', await shown(cdp, first + 20)]));
      await sleep(1300);
      await key(cdp, fire);

      // And the game still answers the keyboard, with nobody clicking the
      // picture first: fire starts another game, which takes a coin.
      assert.equal(await evaluate(cdp, 'document.activeElement && document.activeElement.id'), 'glcanvas',
        'the keyboard was not handed back to the game');
      const coins = await evaluate(cdp, 'getCoins()');
      assert.ok(coins >= 2, `coins left: ${coins}`);
      let spent = false;
      for (let n = 0; n < 12 && !spent; n++) {
        await key(cdp, fire);
        await sleep(900);
        spent = (await evaluate(cdp, 'getCoins()')) < coins;
      }
      assert.ok(spent, 'fire no longer reaches the game after the board');
    });

    await t.test(`${slug}: a name entered by touch`, async (t) => {
      const cdp = await browser(t, 9440 + i, { touch: true, width: 390, height: 844 });
      await openGame(cdp, slug);
      await evaluate(cdp, 'window.blipGameOver(4321); true');
      await waitFor(cdp, has(PROMPT), 8000);
      await evaluate(cdp, 'document.activeElement && document.activeElement.blur(); true');
      await tap(cdp, PROMPT);
      assert.ok(await evaluate(cdp, `document.activeElement === document.querySelector('${PROMPT}')`), 'a tap did not focus the name');
      await cdp.send('Input.insertText', { text: 'TAP' });
      await tap(cdp, '.blip-hs-panel .blip-hs-btn:not(.ghost)');      // OK
      await waitFor(cdp, CODE, 8000);
      await tap(cdp, '.blip-hs-panel .blip-hs-btn.ghost');            // I SAVED IT
      await waitFor(cdp, has('.blip-hs-list'), 8000);
      assert.equal(await evaluate(cdp, MY_ROW), JSON.stringify(['TAP', await shown(cdp, 4321)]));
      await sleep(1300);
      await tap(cdp, '.blip-hs-panel .blip-hs-btn:not(.ghost)');      // CONTINUE
      assert.equal(await evaluate(cdp, has('.blip-hs-modal')), false, 'the board stayed up');
    });
  }
});
