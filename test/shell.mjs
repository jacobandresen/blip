// The game shell in headless Chromium through Playwright: the
// high-score prompt takes taps and keys, fullscreen leaves a PC the top bar
// and the picture, coins need no mouse.
//
//   node --test --test-concurrency=1 test/shell.mjs

import test from 'node:test';
import assert from 'node:assert/strict';
import { evaluate, sleep, waitFor } from './lib/playwright-utils.mjs';
import { launchEngine } from './lib/engine.mjs';
import { createFileServer, listenOn } from './lib/harness.mjs';

let ORIGIN;

/** A fresh Playwright context for one shell case. */
async function browser(t, port, { touch = false, width = 1280, height = 800 } = {}) {
  const { browser, cdp } = await launchEngine('chromium', { hasTouch: touch, viewport: { width, height } });
  t.after(async () => {
    await browser.close().catch(() => {});
  });
  return cdp;
}

async function open(cdp, page, settle = 4000) {
  await cdp.send('Page.navigate', { url: `${ORIGIN}/${page}` });
  if (page.endsWith('/index.html')) {
    await waitFor(cdp, "document.documentElement.hasAttribute('data-game-ready')", 60000);
  }
  await sleep(settle);
}

async function key(cdp, k, code, vk) {
  for (const type of ['keyDown', 'keyUp']) {
    await cdp.send('Input.dispatchKeyEvent', { type, key: k, code, windowsVirtualKeyCode: vk });
    await sleep(100);
  }
}

const WALL = "document.getElementById('need-coin-overlay').classList.contains('visible')";

async function assertSelectedCardFits(cdp, label) {
  const selection = JSON.parse(await evaluate(cdp, `(function(){
    var selected = document.querySelector('.card-focused');
    if (!selected) return JSON.stringify({ missing: true });
    if (selected.id === 'manual-book') return JSON.stringify({ manual: true });
    var card = selected.getBoundingClientRect();
    var window = document.querySelector('#game-grid').getBoundingClientRect();
    return JSON.stringify({ manual: false, card: [card.left, card.right], window: [window.left, window.right] });
  })()`));
  assert.notEqual(selection.missing, true, `${label} has a selected stop`);
  if (selection.manual) return;
  assert.ok(selection.card[0] >= selection.window[0] - 1 && selection.card[1] <= selection.window[1] + 1,
    `${label} stays inside the cabinet window: ${JSON.stringify(selection)}`);
}

test('the game shell', async (t) => {
  const server = createFileServer();
  await listenOn(server, 0);
  ORIGIN = `http://127.0.0.1:${server.address().port}`;
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

  await t.test('fullscreen on a PC is the top bar and the picture, and comes off again', async (t) => {
    const cdp = await browser(t, 9402);
    await open(cdp, 'meteors/index.html', 17000);
    const state = `JSON.stringify((function(){var c=document.getElementById('glcanvas'),r=c.getBoundingClientRect();
      var topbar=document.getElementById('topbar');
      return {topbarDisplay:getComputedStyle(topbar).display,panel:getComputedStyle(topbar.querySelector('.deck-panel')).display,
              barHeight:topbar.getBoundingClientRect().height,box:[r.left,r.top,r.width,r.height],buf:[c.width,c.height],
              marqueeBottom:document.getElementById('marquee-bar').getBoundingClientRect().bottom};})())`;
    const before = JSON.parse(await evaluate(cdp, state));
    assert.notEqual(before.panel, 'none');

    await evaluate(cdp, "blipSetFullscreen(true); true");
    await sleep(600);
    const on = JSON.parse(await evaluate(cdp, state));
    assert.equal(on.panel, 'none', 'the deck panel is still shown');
    assert.equal(on.barHeight, 0, 'the collapsed bar still takes up screen space');
    assert.deepEqual(on.box, [0, on.marqueeBottom, 1280, 800 - on.marqueeBottom], 'the picture does not run edge to edge under the bar');
    // No window resize happened: the drawing buffer has to follow anyway.
    assert.deepEqual(on.buf, [on.box[2], on.box[3]], 'the drawing buffer kept its old size');
    await evaluate(cdp, "blipSetFullscreen(false); true");
    await sleep(600);
    assert.deepEqual(JSON.parse(await evaluate(cdp, state)), before, 'turning it off did not restore the layout');
  });

  await t.test('the choice carries to the next page, where the cabinet drops its deck', async (t) => {
    const cdp = await browser(t, 9403, { width: 1920, height: 1080 });
    await open(cdp, 'index.html');
    await evaluate(cdp, "blipSetFullscreen(true); true");
    await sleep(400);
    assert.equal(await evaluate(cdp, "getComputedStyle(document.querySelector('#kiosk-bar .deck-panel')).display"), 'none');
    assert.equal(await evaluate(cdp, "document.getElementById('kiosk-bar').getBoundingClientRect().height"), 0);
    assert.equal(await evaluate(cdp, 'document.documentElement.scrollHeight <= innerHeight'), true,
      'the catalogue needs scrolling on a 1080p kiosk');
    await open(cdp, 'serpent/index.html', 5000);
    assert.ok(await evaluate(cdp, "document.documentElement.hasAttribute('data-fullscreen')"), 'the game page forgot the choice');
    assert.equal(await evaluate(cdp, "getComputedStyle(document.querySelector('#topbar .deck-panel')).display"), 'none');

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
    const cols = await evaluate(cdp, `(function(){var c=document.querySelectorAll('.card'),t=c[0].offsetTop,n=0;
      while(n<c.length&&c[n].offsetTop===t)n++;return n;})()`);
    const n = await evaluate(cdp, "document.querySelectorAll('.card').length");
    assert.equal(cols, n, `Rolodex cards should form one horizontal fan`);
    await key(cdp, 'ArrowRight', 'ArrowRight', 39);
    await waitFor(cdp, `${lit} === 1`, 1500);
    assert.equal(await evaluate(cdp, lit), 1);
    await key(cdp, 'ArrowDown', 'ArrowDown', 40);
    await waitFor(cdp, `${lit} === 2`, 1500);
    assert.equal(await evaluate(cdp, lit), 2);
    await key(cdp, 'ArrowUp', 'ArrowUp', 38);
    await waitFor(cdp, `${lit} === 1`, 1500);
    assert.equal(await evaluate(cdp, lit), 1);
    // Up and down step through the same single Rolodex row.
    await key(cdp, 'ArrowUp', 'ArrowUp', 38);
    await waitFor(cdp, `${lit} === 0`, 1500);
    assert.equal(await evaluate(cdp, lit), 0);
  });

  await t.test('the mechanical title stays in place and flips through every game', async (t) => {
    const cdp = await browser(t, 9410, { width: 1280, height: 800 });
    await open(cdp, 'index.html');
    const layout = `JSON.stringify((function(){var n=document.querySelector('#marquee-name'),r=n.getBoundingClientRect();
      return {box:[r.left,r.top,r.width,r.height],cells:n.querySelectorAll('.marquee-letter').length,
        dots:n.querySelectorAll('.marquee-dot').length,text:Array.prototype.map.call(n.querySelectorAll('.marquee-letter'),function(x){return x.textContent}).join(''),
        bulbs:Array.prototype.map.call(document.querySelectorAll('.top-marquee-bar .marquee-bulbs'),function(x){var b=x.getBoundingClientRect();return [b.left,b.top,b.width,b.height]})};})())`;
    const empty = JSON.parse(await evaluate(cdp, layout));
    assert.equal(empty.cells, 8);
    assert.equal(empty.dots, 0);
    assert.equal(empty.text, '\u00a0'.repeat(8));
    assert.equal(await evaluate(cdp, "document.querySelector('.top-marquee-bar').getBoundingClientRect().height"), 60,
      'the fascia is tall enough for the large title flipper');
    assert.equal(await evaluate(cdp, "document.querySelectorAll('.top-marquee-bar .marquee-bulbs').length"), 0,
      'the cabinet fascia has no dot strips');
    assert.ok(await evaluate(cdp, "document.querySelector('.card-focused .card-title').textContent === 'RALLY' && document.querySelector('.card-focused .card-desc').textContent.length > 20"),
      'each selected game carries its own description');
    assert.ok(await evaluate(cdp, `(function(){var grid=document.getElementById('game-grid'),card=document.querySelector('.card-focused');
      return card.offsetHeight >= Math.min(500,grid.clientHeight-50)-1;})()`),
      'the selected card fills the available space for readable details');

    const catalogue = JSON.parse(await evaluate(cdp, `JSON.stringify((function(){var cards=[].slice.call(document.querySelectorAll('.card'));
      return {count:cards.length,codes:cards.map(function(c){return c.getAttribute('data-card-code')}),
        rack:document.querySelectorAll('.jukebox-module').length,slugs:cards.map(function(c){return c.getAttribute('data-game-url').split('/')[0]})};})())`));
    assert.equal(catalogue.count, 9);
    assert.equal(catalogue.rack, 9);
    assert.equal(new Set(catalogue.codes).size, 9, 'every laminated selector card has a unique code');
    assert.equal(await evaluate(cdp, "document.querySelectorAll('.lang-switcher,.left-badges').length"), 0,
      'the cabinet has no language or page badges');
    assert.equal(await evaluate(cdp, "document.querySelectorAll('.rolodex-page-card').length"), 0,
      'the current selector contains game cards only');
    for (const slug of catalogue.slugs) {
      assert.equal(await evaluate(cdp, `document.querySelector('.jukebox-module[data-slug="${slug}"]') !== null`), true, `${slug} has a stored card`);
      await evaluate(cdp, `document.querySelector('.card[data-game-url^="${slug}/"]').click(); true`);
      assert.equal(await evaluate(cdp, `document.querySelector('.card[data-game-url^="${slug}/"]').classList.contains('rejected')`), true, `${slug} card responds while awaiting credit`);
      await evaluate(cdp, `document.querySelector('.card[data-game-url^="${slug}/"]').classList.remove('rejected'); true`);
    }
    // The manual is a tenth stop after Adder.
    for (let i = 0; i < 10; i++) {
      await key(cdp, 'ArrowRight', 'ArrowRight', 39);
      await sleep(700);
      await assertSelectedCardFits(cdp, `desktop stop ${i + 1}`);
    }
    await cdp.send('Emulation.setDeviceMetricsOverride', { width:390, height:844, deviceScaleFactor:1, mobile:true });
    await open(cdp, 'index.html', 1100);
    for (let i = 0; i < 10; i++) {
      await key(cdp, 'ArrowRight', 'ArrowRight', 39);
      await sleep(700);
      await assertSelectedCardFits(cdp, `phone stop ${i + 1}`);
    }
    await cdp.send('Emulation.setDeviceMetricsOverride', { width:1280, height:800, deviceScaleFactor:1, mobile:false });
    await open(cdp, 'index.html', 800);

    // Enter from the cabinet itself so the first title change is a real
    // front-page selection, not a direct load of the shell.
    await evaluate(cdp, "document.getElementById('kiosk-insert-btn').click(); true");
    await sleep(300);
    await key(cdp, 'ArrowRight', 'ArrowRight', 39);
    assert.equal(await evaluate(cdp, "document.querySelector('.card-focused .card-title').textContent"), 'BOUNCER');
    await key(cdp, ' ', 'Space', 32);
    await sleep(1450);
    await sleep(18000);
    assert.equal(await evaluate(cdp, 'location.pathname'), '/bouncer/index.html');
    assert.equal(await evaluate(cdp, "document.querySelector('.jukebox-receiver').classList.contains('loaded')"), true,
      'the selected card seats in the receiver before the title changes');
    assert.equal(await evaluate(cdp, "getComputedStyle(document.querySelector('.jukebox-seated-card')).opacity"), '1',
      'the selected game card is visible in the receiver');
    const titles = {
      serpent: 'SERPENT', bouncer: 'BOUNCER', galactic_defender: 'DEFENDER', rally: 'RALLY',
      meteors: 'METEORS', sky_raider: 'RAIDER', brawler: 'BRAWLER', bubbler: 'BUBBLER',
      adder: 'ADDER',
    };
    for (const [slug, title] of Object.entries(titles)) {
      await open(cdp, `${slug}/index.html`, 18000);
      await evaluate(cdp, `blipSetMarquee(${JSON.stringify(title)}, true); true`);
      await waitFor(cdp, `Array.from(document.querySelectorAll('#marquee-name .marquee-letter'), x => x.textContent).join('') === ${JSON.stringify(title.padEnd(8, '\u00a0'))}`, 5000);
      const state = JSON.parse(await evaluate(cdp, `JSON.stringify((function(){var n=document.querySelector('#marquee-name'),r=n.getBoundingClientRect();
        return {name:n.getAttribute('aria-label'),box:[r.left,r.top,r.width,r.height],cells:n.querySelectorAll('.marquee-letter').length,
          dots:n.querySelectorAll('.marquee-dot').length,text:Array.prototype.map.call(n.querySelectorAll('.marquee-letter'),function(x){return x.textContent}).join(''),
          duration:getComputedStyle(n.querySelector('.marquee-letter')).animationDuration,
          delay:getComputedStyle(n.querySelector('.marquee-letter:last-child')).animationDelay};})())`));
      assert.equal(state.name, title, `${slug} accessible title`);
      assert.equal(state.cells, 8, `${slug} has eight mechanical cells`);
      assert.equal(state.dots, 0, `${slug} has no dot dividers`);
      assert.equal(state.text, title.padEnd(8, '\u00a0'), `${slug} title cells`);
      assert.deepEqual(state.box, empty.box, `${slug} title panel position and size`);
      assert.equal(state.duration, '1s', `${slug} slow flap duration`);
      assert.equal(state.delay, '1.33s', `${slug} final cell stagger`);
    }
  });

  await t.test('mobile hides the game title and has no fascia utility buttons', async (t) => {
    const cdp = await browser(t, 9408, { touch: true, width: 390, height: 844 });
    for (const [width,height] of [[390,844],[320,700],[844,390]]) {
      await cdp.send('Emulation.setDeviceMetricsOverride', {width,height,deviceScaleFactor:1,mobile:true});
      for (const page of ['index.html','about.html','controls.html','history.html','api.html','galactic_defender/index.html']) {
        await open(cdp,page,1200);
        const expectedTitle = page.includes('/') && width <= 620 ? 'visible' : 'hidden';
        await waitFor(cdp, `getComputedStyle(document.querySelector('#marquee-name')).visibility === '${expectedTitle}'`, 5000);
        assert.equal(await evaluate(cdp, "getComputedStyle(document.querySelector('#marquee-name')).visibility"), expectedTitle,
          `${page} at ${width}px has the intended title visibility`);
        assert.equal(await evaluate(cdp, "!document.querySelector('#fullscreen-btn, #mute-btn')"), true, `${page} has no fascia utility buttons`);
      }
    }
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

  await t.test('M toggles sound and the choice carries to the next page', async (t) => {
    const cdp = await browser(t, 9405);
    await open(cdp, 'meteors/index.html', 6000);
    const muted = "JSON.stringify([Howler._muted, document.documentElement.hasAttribute('data-muted'), localStorage.getItem('blip-mute')])";
    assert.equal(await evaluate(cdp, muted), '[false,false,null]');
    await key(cdp, 'm', 'KeyM', 77);
    assert.equal(await evaluate(cdp, muted), '[true,true,"1"]');
    // The cabinet's own sounds leave through one gain, shut with it.
    assert.equal(await evaluate(cdp, 'blipOut(getKioskAudio()).gain.value'), 0);

    await open(cdp, 'index.html');
    assert.equal(await evaluate(cdp, muted), '[true,true,"1"]', 'the next page forgot it');
    await key(cdp, 'm', 'KeyM', 77);
    assert.equal(await evaluate(cdp, muted), '[false,false,"0"]', 'M did not turn the sound back on');

  });

  await t.test('a PC arrives on the game with focus and a wall that says what to press', async (t) => {
    const cdp = await browser(t, 9406);
    await open(cdp, 'meteors/index.html', 6000);
    assert.equal(await evaluate(cdp, "document.activeElement.id"), 'glcanvas', 'the game does not have focus');
    const shown = await evaluate(cdp, "[...document.querySelectorAll('#need-coin-overlay .nco-go span')].find((s) => s.offsetParent)?.textContent");
    assert.equal(shown, 'PRESS SPACE TO INSERT A COIN');
  });

  await t.test('coins need no mouse: fire at the wall, 5 at any time', async (t) => {
    const cdp = await browser(t, 9404);
    await open(cdp, 'meteors/index.html', 6000);
    await evaluate(cdp, "document.getElementById('glcanvas').focus(); true");
    assert.equal(await evaluate(cdp, WALL), true, 'no coin wall on arrival with no credit');
    await key(cdp, ' ', 'Space', 32);
    await sleep(400);
    assert.deepEqual([await evaluate(cdp, WALL), await evaluate(cdp, 'getCoins()')], [false, 0], 'fire at the wall did not pay the owed start');
    await key(cdp, '5', 'Digit5', 53);
    assert.equal(await evaluate(cdp, 'getCoins()'), 1);

    // And Backspace is the way back to the cabinet.
    await key(cdp, 'Backspace', 'Backspace', 8);
    await sleep(1500);
    assert.equal(await evaluate(cdp, 'location.pathname'), '/index.html');
  });
});
