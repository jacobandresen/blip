import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

test('BASIC stores and runs a numbered program, then returns from a BLIP game', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.route('**/*.wasm', () => {});
  await page.goto(`${origin}/basic.html`);
  const startupMonitorWidth = await page.locator('.monitor').evaluate((el) => el.getBoundingClientRect().width);
  assert.ok(startupMonitorWidth >= 600, 'the front-page monitor uses the available desktop width');
  assert.notEqual(await page.evaluate(() => document.activeElement.id), 'stick-base');
  await page.evaluate(() => {
    BlipBasic.submit('10 FOR I=1 TO 3');
    BlipBasic.submit('20 PRINT "BLIP";I');
    BlipBasic.submit('30 NEXT I');
    BlipBasic.submit('40 END');
    BlipBasic.submit('SAVE "TEST"');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('BLIP 3'));
  assert.match(await page.locator('#basic-output').textContent(), /BLIP 1[\s\S]*BLIP 2[\s\S]*BLIP 3/);
  await page.evaluate(() => { BlipBasic.submit('NEW'); BlipBasic.submit('LOAD "TEST"'); BlipBasic.submit('LIST'); });
  assert.match(await page.locator('#basic-output').textContent(), /20 PRINT "BLIP";I/);
  await page.evaluate(() => BlipBasic.submit('GAMES'));
  assert.match(await page.locator('#basic-output').textContent(), /RUN "SERPENT"/);
  await page.evaluate(() => BlipBasic.submit('RUN "SERPENT"'));
  assert.equal(await page.locator('body').getAttribute('data-loading-tape'), '');
  assert.equal(await page.locator('#loading-tape').evaluate((el) => getComputedStyle(el).animationName), 'tape-insert');
  assert.match(await page.locator('#loading-tape-label').textContent(), /SERPENT/);
  await page.waitForFunction(() => {
    const frame = document.querySelector('#amstrad-game-frame');
    return frame.contentDocument?.documentElement?.hasAttribute('data-amstrad') === true;
  });
  assert.equal(new URL(page.url()).pathname, '/basic.html');
  assert.match(await page.locator('#amstrad-game-frame').getAttribute('src'), /^serpent\/\?amstrad=1$/);
  assert.equal(await page.locator('body').getAttribute('data-playing'), '');
  assert.ok(await page.locator('.amstrad-controls').isVisible());
  assert.ok(await page.locator('.monitor').evaluate((el, initial) => el.getBoundingClientRect().width > initial + 200, startupMonitorWidth));
  assert.equal(await page.locator('.computer').evaluate((el) => getComputedStyle(el).display), 'none');
  assert.equal(await page.locator('#basic-keyboard').evaluate((el) => getComputedStyle(el).display), 'none');
  assert.equal(await page.locator('#cassette-deck').evaluate((el) => el.getBoundingClientRect().width), 0);
  assert.equal(await page.locator('#amstrad-game-frame').evaluate((el) => getComputedStyle(el).display), 'block');
  const stick = await page.locator('.amstrad-controls #stick-base').boundingBox();
  await page.mouse.move(stick.x + stick.width / 2, stick.y + stick.height / 2);
  await page.mouse.down();
  await page.mouse.move(stick.x + stick.width * .86, stick.y + stick.height / 2);
  assert.equal(await page.evaluate(() => document.querySelector('#amstrad-game-frame').contentWindow.BlipController.isHeld('right')), true);
  await page.mouse.up();
  assert.equal(await page.evaluate(() => document.querySelector('#amstrad-game-frame').contentWindow.BlipController.isHeld('right')), false);
  await page.setViewportSize({ width: 844, height: 390 });
  await page.waitForFunction(() => {
    const monitor = document.querySelector('.monitor').getBoundingClientRect();
    const controlsTop = Math.min(document.querySelector('#stick-base').getBoundingClientRect().top, document.querySelector('#fire-buttons').getBoundingClientRect().top);
    return monitor.bottom <= controlsTop + 1;
  });
  await page.setViewportSize({ width: 1000, height: 900 });
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => document.querySelector('#amstrad-game-frame').hidden);
  assert.equal(await page.evaluate(() => BlipBasic.lines.get(20)), 'PRINT "BLIP";I');
});

test('CLEAR resets the screen and variables but keeps recoverable program lines', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('10 A=42');
    BlipBasic.submit('20 PRINT A');
    BlipBasic.submit('30 END');
    BlipBasic.submit('A=7');
    BlipBasic.submit('CLEAR');
  });
  assert.equal(await page.locator('#basic-output').textContent(), 'Ready\n');
  assert.equal(await page.evaluate(() => BlipBasic.lines.size), 3);
  await page.evaluate(() => BlipBasic.submit('PRINT A'));
  assert.match(await page.locator('#basic-output').textContent(), /\n0\nReady/);
  await page.evaluate(() => BlipBasic.submit('LIST'));
  assert.match(await page.locator('#basic-output').textContent(), /10 A=42[\s\S]*20 PRINT A[\s\S]*30 END/);
  await page.reload();
  assert.match(await page.locator('#basic-output').textContent(), /3 program lines recovered/);
});

test('the desktop Amstrad screen can be maximized and restored', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.goto(`${origin}/basic.html`);
  const maximize = page.locator('#maximize-screen');
  assert.ok(await maximize.isVisible());
  await maximize.click();
  assert.equal(await page.locator('html').getAttribute('data-fullscreen'), '');
  assert.equal(await maximize.getAttribute('aria-pressed'), 'true');
  assert.equal(await page.locator('.computer').evaluate(el => getComputedStyle(el).display), 'none');
  await maximize.click();
  assert.equal(await page.locator('html').getAttribute('data-fullscreen'), null);
  assert.equal(await maximize.getAttribute('aria-pressed'), 'false');
});

test('the Amstrad keyboard types, edits and submits commands by touch', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 390, height: 844 }, hasTouch: true });
  await page.goto(`${origin}/basic.html`);
  assert.equal(await page.locator('#basic-input').getAttribute('readonly'), '');
  await page.setViewportSize({ width: 320, height: 568 });
  assert.deepEqual(await page.locator('.key').evaluateAll(keys => keys.filter(key => key.scrollWidth > key.clientWidth + 1).map(key => key.dataset.keyLabel)), []);
  await page.setViewportSize({ width: 390, height: 844 });
  for (const key of ['H', 'E', 'P']) await page.locator(`[data-key-label="${key}"]`).tap();
  await page.locator('[data-key-label="←"]').tap();
  await page.locator('[data-key-label="L"]').tap();
  await page.locator('[data-key-label="X"]').tap();
  await page.locator('[data-key-label="DEL"]').tap();
  assert.equal(await page.locator('#basic-input').inputValue(), 'HELP');
  await page.locator('[data-key-label="RETURN"]').tap();
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('Line numbers store a program.'));
  assert.equal(await page.locator('#basic-input').inputValue(), '');
  await page.evaluate(() => {
    BlipBasic.submit('10 K$=INKEY$');
    BlipBasic.submit('20 IF K$="A" THEN 50');
    BlipBasic.submit('30 GOTO 10');
    BlipBasic.submit('50 PRINT "TOUCH KEY"');
    BlipBasic.submit('60 END');
    BlipBasic.submit('RUN');
  });
  await page.locator('[data-key-label="A"]').tap();
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('TOUCH KEY'));
});

test('Rally gets two working paddle dials on the Amstrad deck', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.route('**/*.wasm', () => {});
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => BlipBasic.submit('RUN "RALLY"'));
  await page.waitForFunction(() => {
    const frame = document.querySelector('#amstrad-game-frame');
    return frame.contentDocument?.documentElement?.hasAttribute('data-amstrad') === true;
  });
  assert.ok(await page.locator('#amstrad-rally-dial-p1').isVisible());
  assert.ok(await page.locator('#amstrad-rally-dial-p2').isVisible());
  assert.equal(await page.locator('.deck-side').first().evaluate(el => getComputedStyle(el).display), 'none');
  await page.evaluate(() => {
    const win = document.querySelector('#amstrad-game-frame').contentWindow;
    win.__dialKeys = [];
    win.document.querySelector('#glcanvas').addEventListener('keydown', event => win.__dialKeys.push(event.code));
  });
  const p1 = await page.locator('#amstrad-rally-dial-p1').boundingBox();
  const cx = p1.x + p1.width / 2, cy = p1.y + p1.height / 2, radius = p1.width * .35;
  await page.mouse.move(cx + radius, cy);
  await page.mouse.down();
  await page.mouse.move(cx + radius * Math.cos(.3), cy + radius * Math.sin(.3), { steps: 3 });
  await page.waitForFunction(() => document.querySelector('#amstrad-game-frame').contentWindow.__dialKeys.includes('ArrowDown'));
  assert.equal(await page.evaluate(() => document.querySelector('#amstrad-game-frame').contentWindow.document.documentElement.hasAttribute('data-cpu')), true);
  await page.mouse.up();
  const p2 = await page.locator('#amstrad-rally-dial-p2').boundingBox();
  await page.mouse.click(p2.x + p2.width / 2, p2.y + p2.height / 2);
  assert.equal(await page.evaluate(() => document.querySelector('#amstrad-game-frame').contentWindow.document.documentElement.hasAttribute('data-cpu')), false);
});

test('BASIC can draw graphics and the other front ends can select it', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 390, height: 844 } });
  await page.goto(`${origin}/home.html`);
  assert.equal(await page.locator('#basic-switch img').getAttribute('alt'), 'BLIP');
  await page.locator('#basic-switch').click();
  await page.waitForURL(`${origin}/basic.html`);
  assert.equal(await page.locator('.front-nav [aria-current="page"]').textContent(), 'BLIP');
  const pixel = await page.evaluate(() => {
    BlipBasic.submit('PLOT 10,10'); BlipBasic.submit('DRAW 30,30');
    return [...document.querySelector('#basic-graphics').getContext('2d').getImageData(10,189,1,1).data];
  });
  assert.ok(pixel[0] > 200, 'PLOT changes the colour screen');
  assert.equal(await page.evaluate(() => blipFrontend()), 'basic');
  await page.locator('#basic-to-cabinet').click();
  await page.waitForURL(`${origin}/index.html?cabinet`);
  assert.equal(await page.evaluate(() => blipFrontend()), 'cabinet');
  await page.locator('#basic-switch-btn').click();
  await page.waitForURL(`${origin}/basic.html`);
  assert.equal(await page.evaluate(() => blipFrontend()), 'basic');
  await page.locator('#basic-to-home').click();
  await page.waitForURL(`${origin}/home.html`);
  assert.equal(await page.evaluate(() => blipFrontend()), 'home');
  assert.equal(await page.locator('#dock-arcade').getAttribute('aria-label'), 'BLIP Home');
  assert.match(await page.locator('#dock-arcade img').getAttribute('src'), /icon\.svg/);
  await page.locator('#dock-arcade').click();
  await page.waitForURL(`${origin}/index.html?cabinet`);
  assert.equal(await page.evaluate(() => blipFrontend()), 'cabinet');
});

test('BASIC waits for a key while a program is running', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('10 K$=INKEY$');
    BlipBasic.submit('20 IF K$="X" THEN 50');
    BlipBasic.submit('30 WAIT 10');
    BlipBasic.submit('40 GOTO 10');
    BlipBasic.submit('50 PRINT "WIN"');
    BlipBasic.submit('60 END');
    BlipBasic.submit('RUN');
  });
  await page.keyboard.press('x');
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('WIN'));
});

test('IF controls every statement after THEN on the line', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('10 IF 0 THEN PRINT "BAD":PRINT "BAD"');
    BlipBasic.submit('20 IF 1 THEN PRINT "GOOD":PRINT "ALSO GOOD"');
    BlipBasic.submit('30 END');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('ALSO GOOD'));
  const output = await page.locator('#basic-output').textContent();
  assert.doesNotMatch(output, /BAD/);
});

test('BASIC and cabinet switches fit a small phone', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 320, height: 568 }, hasTouch: true });
  await page.goto(`${origin}/basic.html`);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), 0);
  assert.ok(await page.locator('[data-command="GAMES"]').isVisible());
  assert.equal(await page.locator('.direction-buttons').count(), 0);
  assert.equal(await page.locator('[data-command="LIST"], [data-command="RUN"]').count(), 0);
  await page.goto(`${origin}/index.html?cabinet`);
  const positions = await page.evaluate(() => {
    const a = document.querySelector('#home-switch-btn').getBoundingClientRect();
    const b = document.querySelector('#basic-switch-btn').getBoundingClientRect();
    return { a: { x:a.x, right:a.right, y:a.y, bottom:a.bottom }, b:{ x:b.x, right:b.right, y:b.y, bottom:b.bottom } };
  });
  assert.ok(positions.a.right < positions.b.x);
  assert.ok(positions.b.right <= 320);
});
