import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { openPage, startPage } from './lib/harness.mjs';

test('tape programs use uppercase source and unique ascending line numbers', () => {
  const tape = new URL('../web/TAPE/', import.meta.url);
  for (const file of readdirSync(tape).filter((name) => name.endsWith('.BLIP'))) {
    const source = readFileSync(new URL(file, tape), 'utf8');
    assert.equal(source, source.toUpperCase(), `${file} should be uppercase`);
    const lines = source.trim().split('\n').map((line) => Number(line.match(/^\s*(\d+)/)?.[1]));
    assert.ok(lines.every(Number.isFinite), `${file} should contain numbered lines`);
    assert.ok(lines.every((line, index) => index === 0 || line > lines[index - 1]), `${file} line numbers should be unique and ascending`);
  }
  const rally = readFileSync(new URL('RALLY.BLIP', tape), 'utf8');
  assert.match(rally, /KEY_ACTIVE\("I"\)/, 'Rally should let player 2 move up');
  assert.match(rally, /KEY_ACTIVE\("K"\)/, 'Rally should let player 2 move down');
  assert.match(rally, /ELSEIF STATE\$="PLAY" THEN\n4070     TARGET=HOME_Y/, 'the CPU plays only during a rally');
  assert.match(rally, /TARGET=BALL_Y\+BALL\/2-PAD_H\/2\+AI_ERR/);
  assert.match(rally, /BALL_SPEED=MIN\(BALL_SPEED\+SPEED_UP,TOP_SPEED\)/);
  assert.match(rally, /-0\.5,0\.5\)\*1\.1/);
  assert.match(rally, /AI_ERR=RND\(1\)\*PAD_H\*0\.7-PAD_H\*0\.35/);
  assert.match(rally, /DEF FNHITS_PADDLE\(X,Y\)=SEGMENT_RECT_OVERLAP/);
  const raider = readFileSync(new URL('RAIDER.BLIP', tape), 'utf8');
  assert.match(raider, /KILLS>=60\+15\*\(LEVEL-1\)/);
  assert.match(raider, /BOSS_IMAGE\$=ASSETS\$\+"BOSS_"\+STR\$\(LEVEL\)/);
  const assets = (game) => JSON.parse(readFileSync(new URL(`${game}/ASSETS/manifest.json`, tape), 'utf8'));
  for (let level = 1; level <= 7; level++) assert.ok(assets('RAIDER').includes(`BOSS_${level}.PNG`));
  assert.ok(assets('RALLY').includes('BALL.PNG'));
  assert.match(rally, /FILL_RECT\(BALL_X,BALL_Y,BALL,BALL,"BLIP_WHITE"\)/);
  const defender = readFileSync(new URL('DEFENDER.BLIP', tape), 'utf8');
  assert.ok(assets('DEFENDER').includes('BULLET.PNG'));
  assert.match(defender, /DRAW_TEXTURE\(ASSETS\$\+"BULLET\.PNG"/);
});

test('BASIC uses uppercase text and hides the monitor scrollbar without disabling scrolling', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.route('**/*.wasm', () => {});
  await page.goto(`${origin}/basic.html`);
  await page.locator('#basic-input').fill('10 print "hello world"');
  assert.equal(await page.locator('#basic-input').inputValue(), '10 PRINT "HELLO WORLD"');
  await page.evaluate(() => { BlipBasic.submit('10 print "hello world"'); BlipBasic.submit('SAVE lowercase'); });
  assert.equal(await page.evaluate(() => BlipBasic.lines.get(10)), 'PRINT "HELLO WORLD"');
  assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('blip-basic-filesystem')).files['/USER/LOWERCASE.BLIP']), '10 PRINT "HELLO WORLD"');
  assert.deepEqual(await page.locator('#basic-output').evaluate((el) => ({
    transform: getComputedStyle(el).textTransform,
    scrollbar: getComputedStyle(el).scrollbarWidth,
    overflowY: getComputedStyle(el).overflowY,
  })), { transform: 'uppercase', scrollbar: 'none', overflowY: 'auto' });
});

test('BASIC gameplay stays between the top bar and controls on desktop and phones', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.route('**/*.wasm', () => {});
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  for (const viewport of [{ width: 1000, height: 900 }, { width: 1440, height: 900 }, { width: 390, height: 844 }, { width: 844, height: 390 }, { width: 320, height: 568 }]) {
    await page.setViewportSize(viewport);
    for (const mode of ['data-playing', 'data-wasm-playing']) {
      await page.evaluate((mode) => {
        document.body.removeAttribute('data-playing');
        document.body.removeAttribute('data-wasm-playing');
        document.body.setAttribute(mode, '');
        window.dispatchEvent(new Event('resize'));
      }, mode);
      await page.waitForFunction(() => document.querySelector('.monitor').getAnimations().every((animation) => animation.playState === 'finished'));
      const bounds = await page.evaluate(() => {
        const box = (selector) => document.querySelector(selector).getBoundingClientRect().toJSON();
        return { bar: box('.top-marquee-bar'), monitor: box('.monitor'), screen: box('.screen'), stick: box('.stick-base'), buttons: box('.fire-buttons'), controls: Math.min(...['.amstrad-controls', '.stick-base', '.fire-buttons'].map((selector) => box(selector).top)) };
      });
      const label = `${viewport.width} × ${viewport.height}, ${mode}: ${JSON.stringify(bounds)}`;
      assert.ok(bounds.monitor.left >= -1 && bounds.monitor.right <= viewport.width + 1, label);
      assert.ok(bounds.monitor.top >= bounds.bar.bottom - 1, label);
      if (viewport.width >= 651 && viewport.width > viewport.height && viewport.height <= 500) {
        assert.ok(bounds.screen.left > bounds.stick.right - 1 && bounds.screen.right < bounds.buttons.left + 1, label);
      } else {
        assert.ok(bounds.screen.bottom <= bounds.controls + 1, label);
      }
    }
  }
});

test('the monitor fullscreen button expands and restores the monitor', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.route('**/*.wasm', () => {});
  await page.goto(`${origin}/basic.html`);
  const normal = await page.locator('.monitor').boundingBox();
  await page.locator('#monitor-fullscreen').click();
  await page.waitForFunction((width) => document.querySelector('.monitor').getBoundingClientRect().width > width * 1.35, normal.width);
  const expanded = await page.locator('.monitor').boundingBox();
  assert.equal(await page.locator('#monitor-fullscreen').getAttribute('aria-pressed'), 'true');
  assert.equal(await page.locator('.top-marquee-bar').isVisible(), false, 'the maximized monitor has no top bar over it');
  assert.ok(expanded.x >= -1 && expanded.y >= -1 && expanded.x + expanded.width <= 1001 && expanded.y + expanded.height <= 901);
  await page.locator('#monitor-fullscreen').click();
  await page.waitForFunction((width) => document.querySelector('.monitor').getBoundingClientRect().width < width * 1.1, normal.width);
  assert.equal(await page.locator('#monitor-fullscreen').getAttribute('aria-pressed'), 'false');
  assert.equal(await page.locator('.top-marquee-bar').isVisible(), true, 'restored, the top bar is back');
  await page.setViewportSize({ width: 390, height: 844 });
  await page.locator('#monitor-fullscreen').click();
  const mobile = await page.locator('.monitor').boundingBox();
  assert.ok(mobile.x >= -1 && mobile.y >= -1 && mobile.x + mobile.width <= 391 && mobile.y + mobile.height <= 845);
  await page.locator('#monitor-fullscreen').click();
});

test('the expanded monitor fills the screen during a BASIC game, without the control deck', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BOUNCER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BOUNCER.BLIP'));
  await page.evaluate(() => BlipBasic.submit('RUN'));
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
  await page.locator('#monitor-fullscreen').click();
  const layout = await page.evaluate(() => ({
    monitor: document.querySelector('.monitor').getBoundingClientRect().bottom,
    deck: getComputedStyle(document.querySelector('.amstrad-controls')).display,
    canvasFocused: document.querySelector('#basic-wasm-frame').contentDocument.activeElement?.id === 'glcanvas',
  }));
  assert.equal(layout.deck, 'none', 'the control deck goes away');
  assert.ok(layout.monitor > 850, `the monitor takes the room the deck had: ${JSON.stringify(layout)}`);
  assert.equal(layout.canvasFocused, true, 'the game keeps physical keyboard focus');
  await page.setViewportSize({ width: 844, height: 390 });
  await page.locator('#monitor-fullscreen').click();
  assert.equal(await page.locator('#monitor-fullscreen').getAttribute('aria-pressed'), 'false');
  assert.equal(await page.locator('.amstrad-controls').isVisible(), true, 'restored, the deck is back');
  await page.waitForFunction(() => document.querySelector('.monitor').getAnimations().every((animation) => animation.playState === 'finished'));
  const restored = await page.evaluate(() => ({
    screen: document.querySelector('.screen').getBoundingClientRect().toJSON(),
    bar: document.querySelector('.top-marquee-bar').getBoundingClientRect().bottom,
    controls: document.querySelector('.stick-base').getBoundingClientRect().top,
    stick: document.querySelector('.stick-base').getBoundingClientRect().right,
    buttons: document.querySelector('.fire-buttons').getBoundingClientRect().left,
  }));
  assert.ok(restored.screen.top >= restored.bar && restored.screen.left > restored.stick - 1 && restored.screen.right < restored.buttons + 1, `restored after rotation: ${JSON.stringify(restored)}`);
});

test('BASIC stores and runs a numbered program, then launches a BLIP BASIC game with LOAD and RUN', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.emulateMedia({ reducedMotion: 'reduce' });
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
  await page.evaluate(() => { BlipBasic.submit('NEW'); BlipBasic.submit('LOAD "TEST"'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /USER/TEST.BLIP'));
  await page.evaluate(() => BlipBasic.submit('SOURCE'));
  assert.match(await page.locator('#basic-output').textContent(), /20 PRINT "BLIP";I/);
  await page.evaluate(() => BlipBasic.submit('GAMES'));
  assert.match(await page.locator('#basic-output').textContent(), /UNKNOWN COMMAND: GAMES/);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SERPENT'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SERPENT.BLIP'));
  await page.evaluate(() => BlipBasic.submit('RUN'));
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
  assert.equal(new URL(page.url()).pathname, '/basic.html');
  assert.ok(await page.locator('.amstrad-controls').isVisible());
  await page.waitForFunction((initial) => document.querySelector('.monitor').getBoundingClientRect().width > initial + 200, startupMonitorWidth);
  assert.equal(await page.locator('.computer').evaluate((el) => getComputedStyle(el).display), 'none');
  assert.equal(await page.locator('#basic-keyboard').evaluate((el) => getComputedStyle(el).display), 'none');
  assert.equal(await page.locator('#cassette-deck').evaluate((el) => el.getBoundingClientRect().width), 0);
  assert.equal(await page.locator('#basic-wasm-frame').evaluate((el) => getComputedStyle(el).display), 'block');
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await page.evaluate(() => {
    const frame = document.querySelector('#basic-wasm-frame');
    frame.contentWindow.__gameKeys = [];
    frame.contentDocument.querySelector('#glcanvas').addEventListener('keydown', (event) => frame.contentWindow.__gameKeys.push(event.key));
  });
  await canvas.focus();
  await page.keyboard.down('ArrowRight');
  await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.__gameKeys.includes('ArrowRight'));
  await page.keyboard.up('ArrowRight');
  await page.setViewportSize({ width: 844, height: 390 });
  await page.waitForFunction(() => {
    const monitor = document.querySelector('.monitor').getBoundingClientRect();
    const controlsTop = document.querySelector('#stick-base').getBoundingClientRect().top;
    const screen = document.querySelector('.screen').getBoundingClientRect();
    const stickRight = document.querySelector('#stick-base').getBoundingClientRect().right;
    const buttonsLeft = document.querySelector('.fire-buttons').getBoundingClientRect().left;
    return screen.left > stickRight - 1 && screen.right < buttonsLeft + 1;
  });
  await page.setViewportSize({ width: 1000, height: 900 });
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').hidden);
  const tapeLine20 = readFileSync(new URL('../web/TAPE/SERPENT.BLIP', import.meta.url), 'utf8').match(/^20 (.*)$/m)[1];
  assert.equal(await page.evaluate(() => BlipBasic.lines.get(20)), tapeLine20, 'leaving the game keeps the program in memory');
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
  assert.equal(await page.locator('#basic-output').textContent(), 'READY\n');
  assert.equal(await page.evaluate(() => BlipBasic.lines.size), 3);
  await page.evaluate(() => BlipBasic.submit('PRINT A'));
  assert.match(await page.locator('#basic-output').textContent(), /\n0\nREADY/);
  await page.evaluate(() => BlipBasic.submit('LIST'));
  assert.match(await page.locator('#basic-output').textContent(), /10 A=42[\s\S]*20 PRINT A[\s\S]*30 END/);
  await page.reload();
  assert.match(await page.locator('#basic-output').textContent(), /3 PROGRAM LINES RECOVERED/);
});

test('BASIC supports the radians math used by Rust-aligned game physics', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('10 PRINT SQR(9)');
    BlipBasic.submit('20 PRINT SIN(0)');
    BlipBasic.submit('30 PRINT COS(0)');
    BlipBasic.submit('40 END');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('1'));
  assert.match(await page.locator('#basic-output').textContent(), /3[\s\S]*0[\s\S]*1/);
  await page.evaluate(() => { BlipBasic.submit('HELP'); });
  const help = await page.locator('#basic-output').textContent();
  assert.ok(help.includes('FILESYSTEM'), 'HELP should include filesystem commands');
  for (const command of ['LOAD FILE', 'SAVE [NAME]', 'DIR [PATH]', 'CD [PATH]', 'MKDIR NAME', 'DEL FILE', 'EDIT FILE', 'CHANGES AUTO-SAVE', 'ESC CLOSES', 'PAINT [FILE]', 'TUNE', 'TAB', '/TAPE IS READ-ONLY', '/USER PERSISTS']) assert.ok(help.includes(command), `HELP should document ${command}`);
  assert.doesNotMatch(help, /BLIP PRIMITIVES|COLOR\(|DRAW_TEXTURE\(|KEY_ACTIVE\(|PLAY_SFX|PLAY_MUSIC|STOP_MUSIC|\bSOUND\b|\bREST\b/);
});

test('CD navigates the game tapes and returns to USER', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => BlipBasic.submit('CD /TAPE'));
  await page.evaluate(() => BlipBasic.submit('DIR'));
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('DIRECTORY OF /TAPE\n<EMPTY>'));
  await page.evaluate(() => {
    BlipBasic.submit('CD /USER');
    BlipBasic.submit('CD /TAPE');
    BlipBasic.submit('CD');
  });
  const output = await page.locator('#basic-output').textContent();
  assert.match(output, /DIRECTORY OF \/TAPE\n<EMPTY>/);
  assert.doesNotMatch(output, /HELLOWORLD\.BLIP|TEAPOT\.BLIP|FUR ELISE\.BLIP/);
  assert.match(output, /\/TAPE[\s\S]*\/USER[\s\S]*\/TAPE[\s\S]*\/USER\nREADY\n$/);
});

test('BOUNCER.BLIP plays a keyboard-controlled breakout game', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BOUNCER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BOUNCER.BLIP'));
  assert.ok(await page.evaluate(() => BlipBasic.lines.size > 70));
  for (let round = 1; round <= 3; round++) {
    await page.locator('#basic-input').fill('RUN');
    await page.locator('#basic-input').press('Enter');
    try {
      await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 5000 });
    } catch {
      const debug = await page.evaluate(() => ({ sourceLength: document.querySelector('#basic-wasm-frame').contentWindow?.blipBasicSource?.length, state: document.body.getAttribute('data-wasm-error'), src: document.querySelector('#basic-wasm-frame').getAttribute('src') }));
      assert.fail(`round ${round}: BASIC runtime did not load its program: ${JSON.stringify(debug)}`);
    }
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: ${await page.locator('body').getAttribute('data-wasm-error')}`);
    assert.ok(await page.locator('#exit-game').isVisible(), `round ${round}: an on-screen exit should be available during WASM games`);
    assert.equal(await page.locator('.quick-bar').count(), 0, `round ${round}: BASIC has no toolbar`);
    assert.equal(await page.locator('#exit-game').evaluate((el) => getComputedStyle(el).position), 'fixed', `round ${round}: the exit control stays above the game screen`);
    assert.equal(await page.locator('#basic-wasm-frame').isVisible(), true, `round ${round}: the Rust Macroquad canvas should take over the monitor`);
    assert.equal(await page.locator('#basic-wasm-frame').getAttribute('src') !== null, true);
    assert.equal(await page.evaluate(() => { const c=document.querySelector('#basic-wasm-frame').contentDocument.querySelector('#glcanvas');return !!(c.getContext('webgl2')||c.getContext('webgl')); }), true, `round ${round}: BASIC should render through Macroquad WebGL`);
    await page.keyboard.press('Space');
    await page.waitForTimeout(180);
    assert.notEqual(await page.evaluate(() => document.activeElement.id), 'basic-input', `round ${round}: the BASIC prompt must release physical keys`);
    await page.waitForTimeout(350);
    await page.keyboard.press('ArrowLeft');
    await page.keyboard.press('ArrowUp');
    await page.keyboard.press('ArrowRight');
    await page.keyboard.press('Space');
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') && !document.body.hasAttribute('data-wasm-error'));
    if (round === 1) {
      await page.locator('#exit-game').focus(); await page.keyboard.press('Escape');
      await page.waitForFunction(() => !document.querySelector('#basic-editor').hidden);
      assert.match(await page.locator('#editor-file').textContent(), /\/USER\/BOUNCER\/BOUNCER\.BLIP/);
      await page.keyboard.press('Escape');
    } else await page.locator('#exit-game').click();
    await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('READY'));
    assert.equal(await page.evaluate(() => document.activeElement.id), 'basic-input', `round ${round}: exit should return focus to BASIC`);
  }
});

test('SERPENT.BLIP starts, responds to steering, and survives three playtest rounds', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SERPENT'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SERPENT.BLIP'));
  assert.ok(await page.evaluate(() => BlipBasic.lines.size >= 60));

  for (let round = 1; round <= 3; round++) {
    await page.evaluate(() => BlipBasic.submit('RUN'));
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
    const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    const titleFrame = await canvas.screenshot();
    await page.waitForTimeout(250);
    assert.notDeepEqual(await canvas.screenshot(), titleFrame, `round ${round}: the title snake should animate`);
    await page.keyboard.press('Space');
    await page.waitForTimeout(250);
    const playingFrame = await canvas.screenshot();
    await page.keyboard.press('ArrowRight');
    await page.waitForTimeout(250);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: steering should not raise a BASIC runtime error`);
    const steeredFrame = await canvas.screenshot();
    assert.notDeepEqual(steeredFrame, playingFrame, `round ${round}: the snake should move and redraw after steering`);
    await page.locator('#exit-game').click();
    await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
    assert.equal(await page.locator('#basic-input').isVisible(), true, `round ${round}: exit should return to BASIC`);
  }
});

test('SERPENT queues two valid corner turns before the next step', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SERPENT'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SERPENT.BLIP'));
  await page.evaluate(() => {
    // From (11,11) heading right, up then left reaches (10,10) in two steps.
    BlipBasic.lines.set(175, 'IF STATE$="PLAY" THEN STEP_TIME=0.5');
    BlipBasic.lines.set(176, 'IF BODY_X(1)=10 AND BODY_Y(1)=10 THEN QUEUE_OK=1');
    BlipBasic.lines.set(165, 'IF QUEUE_OK THEN DRAW_TEXT("TURN QUEUE OK",320,30,2,"BLIP_YELLOW","CENTER")');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.focus();
  await page.waitForTimeout(250);
  await page.keyboard.press('Space');
  await page.waitForTimeout(150);
  await page.keyboard.down('ArrowUp');
  await page.waitForTimeout(120);
  await page.keyboard.up('ArrowUp');
  await page.keyboard.down('ArrowLeft');
  await page.waitForTimeout(120);
  await page.keyboard.up('ArrowLeft');
  await page.waitForTimeout(1400);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'queued steering should not cause an interpreter error');
  const image = await canvas.screenshot();
  const markerPixels = await canvas.evaluate(async (_el, encoded) => {
    const bitmap = new Image(); bitmap.src = `data:image/png;base64,${encoded}`; await bitmap.decode();
    const sample = document.createElement('canvas'); sample.width = bitmap.width; sample.height = bitmap.height;
    const context = sample.getContext('2d'); context.drawImage(bitmap, 0, 0);
    const { data, width, height } = context.getImageData(0, 0, sample.width, sample.height);
    let count = 0;
    for (let y = Math.floor(height * .12); y < height * .22; y++) for (let x = Math.floor(width * .25); x < width * .75; x++) {
      const i = (y * width + x) * 4;
      if (data[i] > 150 && data[i + 1] > 90 && data[i + 2] < 100) count++;
    }
    return count;
  }, image.toString('base64'));
  assert.ok(markerPixels > 10, `the snake should take up then left before moving into its body; found ${markerPixels} marker pixels`);
  await page.locator('#exit-game').click();
});

test('SERPENT.BLIP loses a life and resumes play after a collision', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SERPENT'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SERPENT.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(90, 'START_GAME:BODY_X(1)=20:LAST_TIME=TIME()');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await page.keyboard.press('Space');
  await page.waitForTimeout(1900);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  const respawned = await canvas.screenshot();
  await page.keyboard.press('ArrowUp');
  await page.waitForTimeout(200);
  assert.notDeepEqual(await canvas.screenshot(), respawned, 'the snake should move again after losing a life');
  await page.locator('#exit-game').click();
});

test('SERPENT.BLIP advances to a new level and draws its obstacles after five foods', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SERPENT'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SERPENT.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(90, 'START_GAME:FOODS=4:STEP_TIME=0.75:FOOD_X=BODY_X(1)+1:FOOD_Y=BODY_Y(1):LAST_TIME=TIME()');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  await page.keyboard.press('Space');
  await page.waitForTimeout(500);
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  const before = await canvas.screenshot();
  const bottomLabel = async (shot) => page.evaluate(async (base64) => {
    const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
    const crop = document.createElement('canvas'); crop.width = image.width; crop.height = image.height;
    const context = crop.getContext('2d'); context.drawImage(image, 0, 0);
    // The LEVEL label, below the board: game rows 372 to 396.
    const scale = Math.min(image.width / 640, image.height / 400), top = (image.height - 400 * scale) / 2;
    return Array.from(context.getImageData(0, Math.floor(top + 372 * scale), image.width, Math.ceil(24 * scale)).data);
  }, shot.toString('base64'));
  const originalLabel = await bottomLabel(before);
  let levelChanged = false;
  for (let frame = 0; frame < 20 && !levelChanged; frame++) {
    await page.waitForTimeout(100);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
    levelChanged = !(await bottomLabel(await canvas.screenshot())).every((pixel, index) => pixel === originalLabel[index]);
  }
  assert.ok(levelChanged, 'the level label should change after the fifth food');
  await page.locator('#exit-game').click();
});

test('SERPENT cycles the five Rust obstacle layouts', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SERPENT'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SERPENT.BLIP'));
  const frames = [];
  for (const level of [2, 3, 4, 5, 6]) {
    await page.evaluate((n) => {
      BlipBasic.lines.set(90, `START_GAME:LEVEL=${n}:RESET_SNAKE:STEP_TIME=10:FOOD_X=20:FOOD_Y=20:LAST_TIME=TIME()`);
      BlipBasic.submit('RUN');
    }, level);
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'));
    await page.waitForTimeout(180);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `level ${level} obstacle setup should run cleanly`);
    frames.push(await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').screenshot());
    await page.locator('#exit-game').click();
    await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
  }
  for (let i = 1; i < frames.length; i++) assert.notDeepEqual(frames[i], frames[i - 1], `level ${i + 2} should use a different obstacle pattern`);
});

test('SERPENT draws long-tail shears in the first eligible even wave', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SERPENT'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SERPENT.BLIP'));
  await page.evaluate(() => {
    // Six levels in, a long snake is offered shears; these sit at cell (13,11).
    BlipBasic.lines.set(90, 'START_GAME:LEVEL=6:RESET_SNAKE:STEP_TIME=100:SHEARS_X=13:SHEARS_Y=11:SHEARS_LEFT=7:LAST_TIME=TIME()');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'));
  await page.waitForTimeout(100);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the shears wave should run without BASIC errors');
  const screenshot = await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').screenshot();
  const pixels = await page.evaluate(async (base64) => {
    const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const scale = Math.min(image.naturalWidth / 640, image.naturalHeight / 400);
    const x = Math.round((image.naturalWidth - 640 * scale) / 2 + 360 * scale), y = Math.round((image.naturalHeight - 400 * scale) / 2 + 216 * scale);
    const { data } = context.getImageData(x - 18, y - 18, 36, 36);
    let count = 0;
    for (let i = 0; i < data.length; i += 4) if (data[i + 2] > data[i] + 55 && data[i + 1] > data[i] + 35) count++;
    return count;
  }, screenshot.toString('base64'));
  assert.ok(pixels > 4, `the pale blue shears should appear in their cell, got ${pixels} blue pixels`);
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('SERPENT shears trim the tail when collected', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SERPENT'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SERPENT.BLIP'));
  const before = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  await page.evaluate(() => {
    // The shears one step ahead, then food: two sounds.
    BlipBasic.lines.set(90, 'START_GAME:LEVEL=6:RESET_SNAKE:SHEARS_X=BODY_X(1)+1:SHEARS_Y=BODY_Y(1):SHEARS_LEFT=7:FOOD_X=BODY_X(1)+2:FOOD_Y=BODY_Y(1):LAST_TIME=TIME()');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'));
  await page.waitForFunction((count) => {
    const frame = document.querySelector('#basic-wasm-frame');
    return document.body.hasAttribute('data-wasm-error') || (frame.contentWindow.blipBasicSoundEvents || 0) >= count + 2;
  }, before, { timeout: 5000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'shears pickup should run without BASIC errors');
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('SERPENT.BLIP offers and collects its timed bonus fruit', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SERPENT'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SERPENT.BLIP'));
  await page.evaluate(() => {
    // The third food offers the bonus; it is moved into the snake's path to be eaten.
    BlipBasic.lines.set(90, 'START_GAME:FOODS=2:FOOD_X=BODY_X(1)+1:FOOD_Y=BODY_Y(1):LAST_TIME=TIME()');
    BlipBasic.lines.set(175, 'IF BONUS_LEFT>0 AND NOT MOVED THEN BONUS_X=BODY_X(1)+2:BONUS_Y=BODY_Y(1):MOVED=1');
    window.serpentSoundBaseline = document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0;
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const frame = page.locator('#basic-wasm-frame');
  const beforeSound = await page.evaluate(() => window.serpentSoundBaseline);
  try {
    await page.waitForFunction(before => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents >= before + 3, beforeSound, { timeout: 3000 });
  } catch {
    const debug = await page.evaluate(() => ({ events: document.querySelector('#basic-wasm-frame').contentWindow?.blipBasicSoundEvents, error: document.body.getAttribute('data-wasm-error'), output: document.querySelector('#basic-output').textContent }));
    assert.fail(`bonus fruits should trigger three original sounds: ${JSON.stringify(debug)}`);
  }
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'bonus spawn, collection, and score should run without BASIC errors');
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('RALLY.BLIP runs matches and moves both player paddles', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD RALLY'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/RALLY.BLIP'));
  const original = await page.evaluate(() => BlipBasic.lines.get(80));
  const turnDial = async (selector, direction) => {
    const box = await page.locator(selector).boundingBox();
    const centerX = box.x + box.width / 2, centerY = box.y + box.height / 2;
    await page.mouse.move(centerX + box.width * .34, centerY); await page.mouse.down();
    await page.mouse.move(centerX, centerY + (direction === 'down' ? 1 : -1) * box.height * .34, { steps: 5 });
    await page.waitForTimeout(120); await page.mouse.up();
  };
  for (let round = 1; round <= 3; round++) {
    const soundEventsBefore = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
    await page.evaluate(([testRound, original]) => {
      BlipBasic.lines.set(80, testRound === 1 ? 'PLAYERS=2:START_GAME' : original);
      BlipBasic.submit('RUN');
    }, [round, original]);
    try {
      await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
    } catch {
      const debug = await page.evaluate(() => ({ sourceLength: document.querySelector('#basic-wasm-frame').contentWindow?.blipBasicSource?.length, runtimeReady: typeof document.querySelector('#basic-wasm-frame').contentWindow?.wasm_memory, error: document.body.getAttribute('data-wasm-error'), src: document.querySelector('#basic-wasm-frame').getAttribute('src') }));
      assert.fail(`round ${round}: BASIC runtime did not load Rally: ${JSON.stringify(debug)}`);
    }
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: Rally source should run without errors`);
    if (round > 1) await page.waitForFunction((before) => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents > before, soundEventsBefore, { timeout: 3000 });
    const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    if (round === 1) {
      await page.waitForTimeout(100);
      const paddleCenter = async () => {
        const image = await canvas.screenshot();
        return canvas.evaluate(async (_el, encoded) => {
          const bitmap = new Image(); bitmap.src = `data:image/png;base64,${encoded}`; await bitmap.decode();
          const sample = document.createElement('canvas'); sample.width = bitmap.width; sample.height = bitmap.height;
          const context = sample.getContext('2d'); context.drawImage(bitmap, 0, 0);
          const { data, width, height } = context.getImageData(0, 0, sample.width, sample.height);
          let weightedY = 0, count = 0;
          const scale = Math.min(width / 640, height / 400), left = (width - 640 * scale) / 2;
          for (let y = Math.floor(height * .2); y < height * .9; y++) for (let x = Math.ceil(left + 596 * scale); x < left + 608 * scale; x++) {
            const i = (y * width + x) * 4;
            // The paddle's grey, dimmed by the curved glass toward the edge.
            if (data[i] >= 140 && data[i] <= 225 && Math.abs(data[i] - data[i + 1]) < 8 && Math.abs(data[i] - data[i + 2]) < 8) { weightedY += y; count++; }
          }
          return count > 0 ? weightedY / count / height : null;
        }, image.toString('base64'));
      };
      let before = await paddleCenter();
      for (let attempt = 0; attempt < 6 && before === null; attempt++) {
        await page.waitForTimeout(100);
        before = await paddleCenter();
      }
      assert.notEqual(before, null, 'both paddles should render before measuring player two input');
      await turnDial('#amstrad-rally-dial-p2', 'down');
      let after = null;
      for (let attempt = 0; attempt < 6 && after === null; attempt++) {
        await page.waitForTimeout(100);
        after = await paddleCenter();
      }
      const inputError = await page.locator('body').getAttribute('data-wasm-error');
      assert.ok(before !== null && after !== null && after - before > .015, `player 2 should move the right paddle down (${before} -> ${after}); ${inputError || 'no interpreter error'}`);
    } else {
      if (round === 2) await turnDial('#amstrad-rally-dial-p1', 'up');
      else await canvas.press('ArrowUp');
      await page.waitForTimeout(100);
      await canvas.press('Space');
      const inFlight = await canvas.screenshot();
      await page.waitForTimeout(250);
      assert.notDeepEqual(await canvas.screenshot(), inFlight, `round ${round}: the served ball should travel between paddles`);
      const served = await canvas.screenshot();
      await canvas.press('ArrowUp');
      await page.waitForTimeout(200);
      assert.notDeepEqual(await canvas.screenshot(), served, `round ${round}: Rally should animate after serve and paddle input`);
    }
    await page.locator('#exit-game').click();
    await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
  }
});

test('RALLY.BLIP awards a point and resets the field after a missed paddle', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.route('**/basic-runtime.html*', async (route) => {
    const response = await route.fetch();
    await route.fulfill({ response, body: (await response.text()).replace('imports.env.blip_basic_sound=playBasicTone;',
      'imports.env.blip_basic_sound=function(f,s,v){window.__pointReset=f;};') });
  });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(async () => {
    await BlipBasic.submit('LOAD /TAPE/RALLY');
    BlipBasic.lines.set(90, 'PLAYERS=2:START_GAME:STATE$="PLAY":BALL_X=0:BALL_Y=60:BALL_VX=-360:BALL_VY=0:LPAD_Y=334:LAST_TIME=TIME()');
    BlipBasic.lines.set(175, 'IF RIGHT_SCORE=1 AND STATE$="POINT" AND BALL_X=314 AND BALL_Y=210 THEN SOUND 1234,0,0');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.__pointReset === 1234);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  await page.locator('#exit-game').click();
});

test('ADDER.BLIP starts and responds to turning in three playtests', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD ADDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/ADDER.BLIP'));
  for (let round = 1; round <= 3; round++) {
    await page.evaluate(() => BlipBasic.submit('RUN'));
    try {
      await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
    } catch {
      const debug = await page.evaluate(() => ({ sourceLength: document.querySelector('#basic-wasm-frame').contentWindow?.blipBasicSource?.length, error: document.body.getAttribute('data-wasm-error'), src: document.querySelector('#basic-wasm-frame').getAttribute('src') }));
      assert.fail(`round ${round}: BASIC runtime did not load Adder: ${JSON.stringify(debug)}`);
    }
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: Adder should run without errors`);
    const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    const countAttract = async (screenshot) => page.evaluate(async base64 => {
      const image = new Image();
      image.src = `data:image/png;base64,${base64}`;
      await image.decode();
      const sample = document.createElement('canvas');
      sample.width = image.width;
      sample.height = image.height;
      const context = sample.getContext('2d');
      context.drawImage(image, 0, 0);
      const { data, width, height } = context.getImageData(0, 0, sample.width, sample.height);
      let green = 0, purple = 0, cyan = 0;
      for (let y = Math.floor(height * .24); y < Math.floor(height * .7); y++) for (let x = 0; x < width; x++) {
        const i = (y * width + x) * 4;
        if (data[i + 1] > data[i] * 1.08 && data[i + 1] > data[i + 2] * 1.5) green++;
        if (data[i + 2] > data[i] * 1.1 && data[i + 2] > data[i + 1] * 1.2) purple++;
        if (data[i + 1] > data[i] * 1.2 && data[i + 2] > data[i] * 1.3 && data[i + 2] > data[i + 1] * 0.95) cyan++;
      }
      return { green, purple, cyan };
    }, screenshot.toString('base64'));
    let title, attract = { green: 0, purple: 0, cyan: 0 };
    for (let frame = 0; frame < 8 && (attract.green <= 30 || attract.purple <= 30 || attract.cyan <= 30); frame++) {
      if (frame) await page.waitForTimeout(100);
      assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: attract scene runtime`);
      title = await canvas.screenshot();
      attract = await countAttract(title);
    }
    assert.ok(attract.green > 30 && attract.purple > 30 && attract.cyan > 30, `the three-viper attract scene should show green, purple, and cyan snakes (${JSON.stringify(attract)})`);
    await page.waitForTimeout(250);
    assert.notDeepEqual(await canvas.screenshot(), title, `round ${round}: the attract vipers should move on the title screen`);
    await page.keyboard.press('Space');
    const opening = await canvas.screenshot();
    await page.keyboard.press('ArrowRight');
    await page.waitForTimeout(200);
    assert.notDeepEqual(await canvas.screenshot(), opening, `round ${round}: the vipers should move and draw`);
    await page.locator('#exit-game').click();
    await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
  }
});

test('ADDER changes to its Rust hunt track on round four', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('CD /TAPE');
    BlipBasic.submit('LOAD ADDER');
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/ADDER.BLIP'));
  await page.evaluate(() => {
    // Round three ends at once, so round four begins on the next frame.
    BlipBasic.lines.set(85, 'START_GAME 1:ROUND=3:ROUND_WAIT=0.01');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents >= 2, undefined, { timeout: 3000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  const events = await page.frameLocator('#basic-wasm-frame').locator('body').evaluate(body => body.ownerDocument.defaultView.blipBasicSoundEvents);
  assert.ok(events >= 2, `round four should switch from the coil to the hunt music (got ${events} music events)`);
  await page.locator('#exit-game').click();
});

test('METEORS.BLIP runs three rounds of vector flight and shooting', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD METEORS'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/METEORS.BLIP'));
  for (let round = 1; round <= 3; round++) {
    await page.evaluate(() => BlipBasic.submit('RUN'));
    try {
      await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
    } catch {
      const debug = await page.evaluate(() => ({ sourceLength: document.querySelector('#basic-wasm-frame').contentWindow?.blipBasicSource?.length, runtimeReady: typeof document.querySelector('#basic-wasm-frame').contentWindow?.wasm_memory, error: document.body.getAttribute('data-wasm-error'), src: document.querySelector('#basic-wasm-frame').getAttribute('src') }));
      assert.fail(`round ${round}: BASIC runtime did not load Meteors: ${JSON.stringify(debug)}`);
    }
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: Meteors should run without errors`);
    const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    await page.waitForTimeout(180);
    const title = await canvas.screenshot();
    const horizonPixels = await page.evaluate(async (base64) => {
      const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
      const sample = document.createElement('canvas'); sample.width = image.width; sample.height = image.height;
      const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
      const { data, width, height } = context.getImageData(0, 0, sample.width, sample.height); let purple = 0;
      for (let y = Math.floor(height * .34); y < height; y++) for (let x = 0; x < width; x++) {
        const i = (y * width + x) * 4;
        if (data[i + 2] > data[i] * 1.25 && data[i + 2] > data[i + 1] * 1.25 && data[i + 2] > 12) purple++;
      }
      return purple;
    }, title.toString('base64'));
    assert.ok(horizonPixels > 20, `the Rust-style purple perspective grid should be visible (${horizonPixels} pixels)`);
    await page.waitForTimeout(250);
    assert.notDeepEqual(await canvas.screenshot(), title, `round ${round}: the rocks and ship should animate on the title screen`);
    await page.keyboard.press('Space');
    await page.keyboard.down('ArrowUp');
    await page.keyboard.down('ArrowLeft');
    await page.keyboard.down('Space');
    await page.waitForTimeout(250);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: flight and fire controls should work`);
    await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents > 0, undefined, { timeout: 2500 });
    assert.ok((await canvas.screenshot()).length > 1000);
    await page.keyboard.up('ArrowUp');
    await page.keyboard.up('ArrowLeft');
    await page.keyboard.up('Space');
    await page.locator('#exit-game').click();
    await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
  }
});

test('METEORS.BLIP starts the next wave after the final small asteroid', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('CD /TAPE');
    BlipBasic.submit('LOAD METEORS');
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/METEORS.BLIP'));
  await page.evaluate(() => {
    // START_GAME's first wave is one small rock straight ahead of the ship.
    BlipBasic.submit('2100 CLEAR_ROCKS:SPAWN_ROCK 3,320,150,0,0');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 20000 });
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await page.keyboard.press('Space');
  await page.waitForTimeout(100);
  const before = await canvas.screenshot();
  await page.keyboard.press('Space');
  await page.waitForTimeout(700);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'splitting and wave spawning should run without BASIC errors');
  assert.notDeepEqual(await canvas.screenshot(), before, 'the cleared rock should be replaced by the next wave');
  await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents > 0, undefined, { timeout: 2500 });
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('METEORS.BLIP splits a medium asteroid into two small rocks', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD METEORS'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/METEORS.BLIP'));
  await page.evaluate(() => {
    BlipBasic.submit('2100 CLEAR_ROCKS:SPAWN_ROCK 2,320,150,0,0');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 20000 });
  const frame = page.locator('#basic-wasm-frame');
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await page.keyboard.press('Space');
  await page.waitForTimeout(100);
  const before = await canvas.screenshot();
  const beforeSound = await frame.evaluate(el => el.contentWindow.blipBasicSoundEvents);
  await page.keyboard.press('Space');
  await page.waitForFunction(before => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents >= before + 2, beforeSound);
  await page.waitForTimeout(100);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'medium asteroid splitting should run without BASIC errors');
  assert.notDeepEqual(await canvas.screenshot(), before, 'the medium rock should split into smaller moving rocks');
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('METEORS.BLIP launches an animated saucer and aims its shot', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD METEORS'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/METEORS.BLIP'));
  await page.evaluate(() => {
    // An empty field that stays empty, and a saucer due at once.
    BlipBasic.submit('4915 EXIT SUB');
    BlipBasic.submit('2100 CLEAR_ROCKS:SAUCER_WAIT=0.05');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 20000 });
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await page.keyboard.press('Space');
  await page.waitForTimeout(100);
  await page.waitForTimeout(160);
  const saucer = await canvas.screenshot();
  await page.waitForTimeout(220);
  assert.notDeepEqual(await canvas.screenshot(), saucer, 'the saucer should travel across the empty playfield');
  await page.waitForTimeout(1200);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'saucer firing and aimed shots should run without BASIC errors');
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('METEORS.BLIP awards the big saucer hit and plays its original sound', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD METEORS'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/METEORS.BLIP'));
  await page.evaluate(() => {
    // Straight into play, with a big saucer parked ahead of the ship.
    BlipBasic.submit('85 START_GAME');
    BlipBasic.submit('4915 EXIT SUB');
    BlipBasic.submit('2100 CLEAR_ROCKS:SPAWN_SAUCER:SAUCER_X=320:SAUCER_Y=150:SAUCER_VX=0:SAUCER_BIG=1');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 20000 });
  const frame = page.locator('#basic-wasm-frame');
  const beforeSound = await frame.evaluate(el => el.contentWindow.blipBasicSoundEvents);
  await page.keyboard.press('Space');
  await page.waitForFunction(before => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents >= before + 2, beforeSound, { timeout: 3000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the saucer hit and original audio should run without BASIC errors');
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('RAIDER.BLIP runs three rounds with flight, shooting, graphics, and sound', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD RAIDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/RAIDER.BLIP'));
  for (let round = 1; round <= 3; round++) {
    await page.evaluate(() => BlipBasic.submit('RUN'));
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: Raider should launch without errors`);
    const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    await page.waitForTimeout(180);
    const title = await canvas.screenshot();
    await page.waitForTimeout(250);
    const nextTitle = await canvas.screenshot();
    if (!nextTitle.equals(title)) {
      const { writeFile } = await import('node:fs/promises');
      await writeFile('/tmp/blip-raider-title-before.png', title);
      await writeFile('/tmp/blip-raider-title-after.png', nextTitle);
    }
    assert.ok(nextTitle.equals(title), `round ${round}: the native title's sea and cloud art should stay stable`);
    await page.keyboard.press('Space');
    await page.keyboard.down('ArrowRight');
    await page.keyboard.down('Space');
    await page.waitForTimeout(250);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: flight and fire should work`);
    await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents > 0, undefined, { timeout: 2500 });
    assert.ok((await canvas.screenshot()).length > 1000, `round ${round}: the monitor should render the scrolling game scene`);
    await page.keyboard.up('ArrowRight');
    await page.keyboard.up('Space');
    await page.locator('#exit-game').click();
    await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
  }
});

test('RAIDER drops an ACE weapon capsule and upgrades its volley', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD RAIDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/RAIDER.BLIP'));
  await page.evaluate(() => {
    // One ACE that never fires, a round put into it each frame, and its capsule
    // carried to the plane; the marker shows tier two and 120+60 points.
    BlipBasic.lines.set(85, 'START_GAME:LIVES=99:SPAWN_WAIT=999:SPAWN_ENEMY 3,295,120,0:ENEMY_FIRE(1)=999');
    BlipBasic.lines.set(175, 'IF ENEMY(1) THEN SHOT(1)=1:SHOT_X(1)=ENEMY_X(1)+20:SHOT_Y(1)=ENEMY_Y(1)+10');
    BlipBasic.lines.set(176, 'IF CAPSULE_ON THEN CAPSULE_X=PLANE_X+18:CAPSULE_Y=PLANE_Y+15');
    BlipBasic.lines.set(165, 'IF WEAPON=2 AND SCORE>=180 THEN FILL_RECT(8,60,24,24,"#00FF00")');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  let upgraded = false;
  for (let frame = 0; frame < 15 && !upgraded; frame++) {
    await page.waitForTimeout(100);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'ACE drops and weapon upgrades should run without interpreter errors');
    const screenshot = await canvas.screenshot();
    upgraded = await page.evaluate(async encoded => {
      const image = new Image(); image.src = `data:image/png;base64,${encoded}`; await image.decode();
      const sample = document.createElement('canvas'); sample.width = image.width; sample.height = image.height;
      const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
      const { data, width, height } = context.getImageData(0, 0, sample.width, sample.height);
      let green = 0;
      for (let y = Math.floor(height * .18); y < Math.floor(height * .30); y++) for (let x = Math.floor(width * .01); x < Math.floor(width * .06); x++) {
        const i = (y * width + x) * 4;
        if (data[i + 1] > 100 && data[i + 1] > data[i] * 2 && data[i + 1] > data[i + 2] * 2) green++;
      }
      return green > 8;
    }, screenshot.toString('base64'));
  }
  if (!upgraded) {
    const debug = await canvas.screenshot();
    await import('node:fs/promises').then(({ writeFile }) => writeFile('/tmp/sky-raider-powerup.png', debug));
  }
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'weapon upgrade playtest should not stop on a BASIC runtime error');
  assert.ok(upgraded, 'the ACE capsule should raise the player to weapon tier two');
  assert.ok((await canvas.screenshot()).length > 1000);
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('RAIDER reaches its Rust boss threshold, defeats the boss, and advances the wave', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD RAIDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/RAIDER.BLIP'));
  await page.evaluate(() => {
    // Sixty kills already: the boss comes at once and takes a round each frame.
    BlipBasic.lines.set(85, 'START_GAME:LIVES=99:KILLS=60');
    BlipBasic.lines.set(175, 'IF BOSS_ON AND BOSS_IN THEN SHOT(1)=1:SHOT_X(1)=BOSS_X+BOSS_W/2:SHOT_Y(1)=BOSS_Y+5');
    BlipBasic.lines.set(165, 'IF BOSS_ON THEN FILL_RECT(8,4,24,22,"#00FF00")');
    BlipBasic.lines.set(166, 'IF LEVEL=2 THEN FILL_RECT(8,4,24,22,"#FF00FF")');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'boss spawn and hit logic should run without interpreter errors');
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  const levelMarker = async () => page.evaluate(async encoded => {
    const image = new Image(); image.src = `data:image/png;base64,${encoded}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.width; sample.height = image.height;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const { data, width, height } = context.getImageData(0, 0, sample.width, sample.height);
    let count = 0;
    for (let y = Math.floor(height * .08); y < Math.floor(height * .14); y++) for (let x = Math.floor(width * .01); x < Math.floor(width * .06); x++) {
      const i = (y * width + x) * 4;
      if (data[i] > 100 && data[i + 1] < 140 && data[i + 2] > 100) count++;
    }
    return count;
  }, (await canvas.screenshot()).toString('base64'));
  let marker = 0;
  for (let frame = 0; frame < 35 && marker < 8; frame++) {
    await page.waitForTimeout(200);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'boss bullets and wave progression should remain error-free');
    marker = await levelMarker();
  }
  const lastFrame = await canvas.screenshot();
  if (marker < 8) await import('node:fs/promises').then(({ writeFile }) => writeFile('/tmp/sky-raider-basic-boss.png', lastFrame));
  assert.ok(marker >= 8, `the original boss should take hits and advance from level one to level two (marker ${marker}, events ${await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents)})`);
  assert.ok((await canvas.screenshot()).length > 1000, 'the next wave should keep rendering');
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('RAIDER.BLIP fires aimed enemy rounds with the original gun sound', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD RAIDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/RAIDER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(85, 'START_GAME:SPAWN_WAIT=999:SPAWN_ENEMY 1,295,80,0:ENEMY_FIRE(1)=0.05');
  });
  await page.evaluate(() => BlipBasic.submit('RUN'));
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'aimed enemy fire should run without BASIC or runtime errors');
  const beforeSound = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  await page.waitForTimeout(1500);
  const soundState = await page.evaluate(() => ({ events: document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents, error: document.body.getAttribute('data-wasm-error'), output: document.querySelector('#basic-output').textContent }));
  assert.ok(soundState.events > beforeSound, `enemy fire should play its registered sound (before ${beforeSound}, after ${soundState.events}, error ${soundState.error}, output ${soundState.output})`);
  await page.waitForTimeout(350);
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  assert.ok((await canvas.screenshot()).length > 1000, 'enemy fire should remain visible in the flight scene');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the aimed shot update should remain stable');
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('RAIDER.BLIP spawns enemy pairs like the Rust formations', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('CD /TAPE');
    BlipBasic.submit('LOAD RAIDER');
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/RAIDER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(85, 'START_GAME:SPAWN_WAIT=0');
    BlipBasic.lines.set(165, 'IF (ENEMY(1)>0)+(ENEMY(2)>0)+(ENEMY(3)>0)+(ENEMY(4)>0)>=2 THEN FILL_RECT(200,80,16,16,"BLIP_YELLOW")');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  const readMarker = async () => page.evaluate(async encoded => {
    const image = new Image(); image.src = `data:image/png;base64,${encoded}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.width; sample.height = image.height;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const { data, width } = context.getImageData(0, 0, sample.width, sample.height);
    let pixels = 0;
    const scale = Math.min(width / 640, image.height / 400);
    const left = (width - 640 * scale) / 2, top = (image.height - 400 * scale) / 2;
    for (let y = Math.floor(top + 82 * scale); y < Math.ceil(top + 94 * scale); y++) for (let x = Math.floor(left + 202 * scale); x < Math.ceil(left + 214 * scale); x++) {
      const i = (y * width + x) * 4;
      if (data[i] > 150 && data[i + 1] > 110 && data[i + 2] < 100) pixels++;
    }
    return { pixels, width, height: image.height };
  }, (await canvas.screenshot()).toString('base64'));
  let marker = { pixels: 0, width: 0, height: 0 };
  for (let frame = 0; frame < 25 && marker.pixels <= 8; frame++) {
    await page.waitForTimeout(150);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'enemy formation logic should run cleanly');
    marker = await readMarker();
  }
  assert.ok(marker.pixels > 8, `two active enemies should trigger the formation marker; got ${JSON.stringify(marker)}`);
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('RAIDER.BLIP gives the plane hit grace against a touching formation', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD RAIDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/RAIDER.BLIP'));
  await page.evaluate(() => {
    // Two quiet grunts on top of the plane, without its spawn grace. No music
    // starts, so the only sound is the hit.
    BlipBasic.lines.set(80, 'REM NO TITLE MUSIC');
    BlipBasic.lines.set(85, 'LIVES=3:LEVEL=1:WEAPON=1:CLEAR_SKY:NEW_PLANE:STATE$="PLAY":SPAWN_WAIT=999:GRACE=0');
    BlipBasic.lines.set(86, 'SPAWN_ENEMY 1,PLANE_X,PLANE_Y,0:SPAWN_ENEMY 1,PLANE_X,PLANE_Y,0:ENEMY_FIRE(1)=999:ENEMY_FIRE(2)=999');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'));
  await page.waitForTimeout(250);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'contact grace should run without BASIC errors');
  const events = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents);
  assert.equal(events, 1, `a formation contact should consume one life and play one hit sound, got ${events} events`);
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

const soundCount = (page) => page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
const exitTape = async (page) => {
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
};
/** Ryuka against Giotto, a fixed distance apart; the rival stands still. */
const brawlerFight = (gap, setup = '') => ({
  80: 'REM NO MUSIC, SO THE SOUND COUNT IS ONLY THE FIGHT',
  85: `PICK=1:START_GAME:RIVAL_X=YOU_X+${gap}:SPEED(RIVAL)=0${setup}`,
});

test('BRAWLER.BLIP supports three rounds of movement and striking', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (let round = 1; round <= 3; round++) {
    const canvas = await openTape(page, origin, 'BRAWLER', { 166: 'RIVAL_WAIT=999' });
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: Brawler should launch cleanly`);
    await page.waitForTimeout(180);
    const title = await canvas.screenshot();
    let titleAnimated = false;
    for (let frame = 0; frame < 16 && !titleAnimated; frame++) {
      await page.waitForTimeout(250);
      titleAnimated = !(await canvas.screenshot()).equals(title);
    }
    assert.ok(titleAnimated, `round ${round}: the title fighters and spotlights should animate`);
    await canvas.focus();
    await page.keyboard.down('Space');
    await page.waitForTimeout(350);
    await page.keyboard.up('Space');
    await page.keyboard.down('ArrowRight');
    await page.keyboard.down('Space');
    await page.keyboard.press('ArrowUp');
    await page.keyboard.down('ArrowDown');
    await page.keyboard.press('z');
    await page.waitForTimeout(1200);
    for (const key of ['ArrowRight', 'ArrowDown', 'Space']) await page.keyboard.up(key);
    await page.keyboard.press('f', { delay: 100 });
    await page.keyboard.press('z', { delay: 100 });
    await page.waitForTimeout(500);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: movement, jump, guard, punch, and kick should not fail`);
    let fight; let lit = 0;
    for (let frame = 0; frame < 16 && lit <= 2000; frame++) {
      if (frame) await page.waitForTimeout(100);
      fight = await canvas.screenshot();
      lit = await countGamePixels(page, fight, [64, 40, 512, 320], 'lit');
    }
    assert.notDeepEqual(fight, title, `round ${round}: pressing Space should enter the fight scene`);
    assert.ok(lit > 2000, `round ${round}: the fight scene should show its stage and fighters (got ${lit})`);
    await exitTape(page);
  }
});

test('BRAWLER punch lands once after its Rust startup frames', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const canvas = await openTape(page, origin, 'BRAWLER', brawlerFight(70, ':RIVAL_WAIT=999'));
  await waitForPixels(page, canvas, [52, 24, 196, 12], 'cyan', 200);
  await canvas.focus();
  const before = await soundCount(page);
  await page.keyboard.press('Space');
  await page.waitForTimeout(350);
  const after = await soundCount(page);
  assert.equal(after, before + 2, `a close punch should play its windup and exactly one hit sound (${before} -> ${after})`);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the timed punch should not cause an interpreter error');
  await exitTape(page);
});

test('BRAWLER tape visits the Rust ladder order for all nine matches', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  // blip::ordered_ladder for Ryuka (1) and Gamma (9), one-based.
  for (const [pick, expected] of [[1, [4, 2, 5, 3, 6, 8, 7, 9, 10]], [9, [1, 4, 2, 5, 3, 6, 8, 7, 10]]]) {
    const same = expected.map((who, i) => `LADDER(${i + 1})=${who}`).join(' AND ');
    const canvas = await openTape(page, origin, 'BRAWLER', { 85: `PICK=${pick}:BUILD_LADDER:STATE$="TITLE"`, 165: statusMarker(same) });
    assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20) > 20, `pick ${pick}: the BASIC search should match Rust ordering and keep Gamma then Zenith last`);
    await exitTape(page);
  }
});

test('BRAWLER roster specials land across projectile, rush, kick and laser fighters', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const pick of [1, 2, 3, 8, 10]) {
    const canvas = await openTape(page, origin, 'BRAWLER', {
      80: 'REM NO MUSIC',
      85: `PICK=${pick}:START_GAME:RIVAL_X=YOU_X+110:SPEED(RIVAL)=0:RIVAL_WAIT=999`,
      165: statusMarker('SPECIAL_HIT=1 AND RIVAL_HP<MAX_HP(RIVAL)'),
    });
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `fighter ${pick} should start`);
    await canvas.focus();
    await page.waitForTimeout(300);
    const beforeSound = await soundCount(page);
    await page.keyboard.down('f');
    await page.waitForTimeout(35);
    await page.keyboard.down('g');
    await page.waitForTimeout(55);
    await page.keyboard.up('f');
    await page.keyboard.up('g');
    await page.waitForTimeout(800);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `fighter ${pick} special should run`);
    const afterSound = await soundCount(page);
    assert.ok(afterSound >= beforeSound + 2, `fighter ${pick} special should play its startup and hit sounds (${beforeSound} -> ${afterSound})`);
    assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20) > 20, `fighter ${pick} special should land`);
    await exitTape(page);
  }
});

test('BRAWLER guard blocks the CPU attack without chip damage', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const canvas = await openTape(page, origin, 'BRAWLER', {
    ...brawlerFight(40, ':RIVAL_WAIT=20'),
    165: 'IF BLOCK_FLASH>0 THEN BLOCK_FLASH=99',
    166: statusMarker('YOUR_HP=MAX_HP(PICK)'),
  });
  await canvas.focus();
  await page.waitForTimeout(150);
  const before = await soundCount(page);
  await page.keyboard.down('ArrowDown');
  await page.waitForFunction((initial) => (document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0) >= initial + 2, before, { timeout: 3000 });
  await page.waitForTimeout(150);
  const frame = await canvas.screenshot();
  await page.keyboard.up('ArrowDown');
  assert.ok(await countGamePixels(page, frame, [166, 268, 28, 28], 'cyan') > 4, "guard impact should flash cyan at the player's hands");
  assert.ok(await countGamePixels(page, frame, STATUS_MARKER, 'green') > 20, 'a blocked blow should do no damage');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'blocking should not cause an interpreter error');
  await exitTape(page);
});

test('BRAWLER unguarded CPU hits show a red impact cue', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const canvas = await openTape(page, origin, 'BRAWLER', {
    ...brawlerFight(40, ':RIVAL_WAIT=20'),
    165: 'IF HURT_FLASH>0 THEN HURT_FLASH=99',
    166: statusMarker('YOUR_HP<MAX_HP(PICK)'),
  });
  await canvas.focus();
  await page.waitForTimeout(150);
  const before = await soundCount(page);
  await page.waitForFunction((initial) => (document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0) >= initial + 2, before, { timeout: 3000 });
  await page.waitForTimeout(150);
  const frame = await canvas.screenshot();
  assert.ok(await countGamePixels(page, frame, [166, 268, 28, 28], 'red') > 4, 'an unguarded CPU hit should flash red between the fighters');
  assert.ok(await countGamePixels(page, frame, STATUS_MARKER, 'green') > 20, 'the hit should cost health');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the damage animation should not cause an interpreter error');
  await exitTape(page);
});

test('BRAWLER ends a 45-second round on time and awards the healthier fighter', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  let canvas = await openTape(page, origin, 'BRAWLER', { 85: 'PICK=1:START_GAME:RIVAL_WAIT=999' });
  assert.ok(await waitForPixels(page, canvas, [52, 24, 196, 12], 'cyan', 200) > 200, 'your health bar should sit on the left');
  const fight = await canvas.screenshot();
  assert.ok(await countGamePixels(page, fight, [392, 24, 196, 12], 'red') > 200, "the rival's health bar should sit on the right");
  assert.ok(await countGamePixels(page, fight, [294, 18, 52, 26], 'white') > 10, 'the round clock should be visible between them');
  await exitTape(page);
  canvas = await openTape(page, origin, 'BRAWLER', {
    85: 'PICK=1:START_GAME:YOUR_HP=90:RIVAL_HP=55:RIVAL_WAIT=999:ROUND_TIME=0.2',
    165: statusMarker('STATE$="ROUND" AND TIME_UP AND YOUR_ROUNDS=1 AND RIVAL_ROUNDS=0'),
  });
  await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the time-up path should not stop with a BASIC error');
  const result = await canvas.screenshot();
  assert.ok(await countGamePixels(page, result, [240, 66, 160, 26], 'yellow') > 12, 'the round should display TIME UP');
  assert.ok(await countGamePixels(page, result, STATUS_MARKER, 'green') > 20, 'the higher-health player should win the timed round');
  await exitTape(page);
});

test('BRAWLER.BLIP draws the turtle shell and mask build', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const canvas = await openTape(page, origin, 'BRAWLER', { 85: 'PICK=4:START_GAME' });
  const mask = await waitForPixels(page, canvas, [140, 240, 40, 12], 'blue', 10);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'shell, mask and fight drawing should run without BASIC errors');
  assert.ok(mask > 10, 'the turtle should wear its blue mask');
  await exitTape(page);
});

/** The numbers of the tape lines whose code is `code`, so a test can replace a line inside a SUB. */
function tapeLines(tape, code) {
  const source = readFileSync(new URL(`../web/TAPE/${tape}.BLIP`, import.meta.url), 'utf8');
  const found = source.split('\n').filter((line) => line.replace(/^\d+\s+/, '').trim() === code);
  assert.ok(found.length > 0, `${tape} has a line ${code}`);
  return found.map((line) => Number(line.match(/^\d+/)[0]));
}

function tapeLine(tape, code) {
  const [number, ...more] = tapeLines(tape, code);
  assert.equal(more.length, 0, `${tape} has one line ${code}`);
  return number;
}

/** Round one in play at once, with no music, so the sound count is only the game's. */
/** Bubbler keeps the Rust game's 624x600 arena. */
const BUBBLER_SIZE = [624, 600];
const BUBBLER_PLAY = 'START_GAME:SHOW "PLAY"';
const quietBubbler = (setup, lines = {}) => ({ 80: 'REM NO MUSIC', 85: setup, ...lines });

test('BUBBLER.BLIP runs three rounds of platforming and bubble attacks', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (let round = 1; round <= 3; round++) {
    const canvas = await openTape(page, origin, 'BUBBLER');
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: Bubbler should launch cleanly`);
    await waitForPixels(page, canvas, [0, 0, 624, 600], 'lit', 2000, { size: BUBBLER_SIZE });
    const title = await canvas.screenshot();
    await page.waitForTimeout(250);
    assert.notDeepEqual(await canvas.screenshot(), title, `round ${round}: the bubbles and dragons should animate on the title screen`);
    await canvas.focus();
    await page.keyboard.press('Space');
    await page.keyboard.down('ArrowRight');
    await page.keyboard.press('ArrowUp');
    await page.keyboard.down('Space');
    await page.waitForTimeout(250);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: jump and bubble controls should not fail`);
    await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents > 0, undefined, { timeout: 2500 });
    await page.keyboard.up('ArrowRight');
    await page.keyboard.up('Space');
    await exitTape(page);
  }
});

test('BUBBLER advances its preserved score into a faster next round', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  const canvas = await openTape(page, origin, 'BUBBLER', quietBubbler(
    'TWO=1:START_GAME:SCORE(1)=1234:SCORE(2)=567:LIVES(2)=7:SHOW "PLAY":FOR I=1 TO TOY_COUNT:TOY_ON(I)=0:NEXT I',
    { 165: statusMarker('ROUND=2 AND PACE>0.8 AND SCORE(1)=1234 AND SCORE(2)=567 AND LIVES(1)=6 AND LIVES(2)=7') }));
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20, { timeout: 9000, size: BUBBLER_SIZE }) > 20,
    'clearing a round should keep the scores, top up lives to seven, and start a quicker round two');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  await exitTape(page);
});

test('BUBBLER completes after its eighth Rust level', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  const canvas = await openTape(page, origin, 'BUBBLER', quietBubbler(
    'START_GAME:LOAD_ROUND 8:SHOW "PLAY":FOR I=1 TO TOY_COUNT:TOY_ON(I)=0:NEXT I',
    { 165: statusMarker('STATE$="OVER" AND WON') }));
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20, { timeout: 9000, size: BUBBLER_SIZE }) > 20, 'clearing round eight should win the game');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the final round and win screen should run cleanly');
  await exitTape(page);
});

test('BUBBLER chains five trapped enemies into the Rust score and five candy drops', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  const canvas = await openTape(page, origin, 'BUBBLER', quietBubbler(
    `${BUBBLER_PLAY}:FOR I=1 TO 5:BUB_ON(I)=1:BUB_HOLDS(I)=I:TOY_ON(I)=0:BUB_X(I)=200+I*20:BUB_Y(I)=200:NEXT I:POP_CHAIN 1,1`,
    { 165: statusMarker('SCORE(1)=31000 AND CANDY_ON(5)=1 AND CANDY_KIND(5)=3 AND BUB_ON(5)=0 AND CHAIN_SIZE=5') }));
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20, { size: BUBBLER_SIZE }) > 20, 'five toys should score 1000+2000+4000+8000+16000 and drop five candies');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  await exitTape(page);
});

test('BUBBLER trapped enemies are defeated when a player pops their bubble', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  const canvas = await openTape(page, origin, 'BUBBLER', quietBubbler(
    `${BUBBLER_PLAY}:BUB_ON(1)=1:BUB_HOLDS(1)=1:TOY_ON(1)=0:BUB_PHASE(1)=FLOATING:BUB_X(1)=PX(1)+14:BUB_Y(1)=PY(1)+11`,
    { 165: statusMarker('SCORE(1)>=1000 AND BUB_ON(1)=0 AND TOY_ON(1)=0') }));
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20, { size: BUBBLER_SIZE }) > 20, 'touching a caught toy should pop it for points');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'popping a trapped enemy should run without a BASIC error');
  await exitTape(page);
});

test('BUBBLER physical controls trap a monster, pop it, and collect its candy', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const canvas = await openTape(page, origin, 'BUBBLER', quietBubbler(
    `${BUBBLER_PLAY}:FOR I=2 TO TOY_COUNT:TOY_ON(I)=0:NEXT I:TOY_X(1)=PX(1)+20:TOY_Y(1)=PY(1)+2:TOY_KIND(1)=1:PACE=0.001`,
    // The candy hops back to the dragon instead of off to a random side.
    { [tapeLine('BUBBLER', 'CANDY_VX(I)=RND(220)-110')]: 'CANDY_VX(I)=-60', 165: statusMarker('SCORE(1)=1500') }));
  await canvas.focus();
  await page.waitForTimeout(300);
  const before = await soundCount(page);
  await page.keyboard.press('f');
  const scored = await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20, { size: BUBBLER_SIZE });
  const after = await soundCount(page);
  assert.ok(after >= before + 4, `F should blow, trap, pop and the dragon pick up candy (sound events ${before} -> ${after})`);
  assert.ok(scored > 20, 'the toy and its bonbon should score 1000 + 500');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'trapping, popping and picking up candy should run without a BASIC error');
  await exitTape(page);
});

test('BUBBLER trapped enemies escape and return to play when their bubbles expire', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  const canvas = await openTape(page, origin, 'BUBBLER', quietBubbler(
    `${BUBBLER_PLAY}:BUB_ON(1)=1:BUB_HOLDS(1)=1:TOY_ON(1)=0:BUB_PHASE(1)=FLOATING:BUB_X(1)=300:BUB_Y(1)=200:BUB_AGE(1)=TRAP_LIFE-0.2`,
    { 165: statusMarker('TOY_ON(1)=1 AND TOY_ANGRY(1)=1 AND BUB_ON(1)=0') }));
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20, { size: BUBBLER_SIZE }) > 20, 'the toy should break out of its bubble, angry');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'an escaped enemy should return without a BASIC error');
  await exitTape(page);
});

test('BUBBLER changes its palette and platform layout through round eight', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  const rounds = [];
  for (let round = 1; round <= 8; round++) {
    const canvas = await openTape(page, origin, 'BUBBLER', quietBubbler(`START_GAME:LOAD_ROUND ${round}`, { 165: statusMarker(`ROUND=${round}`) }));
    assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20, { size: BUBBLER_SIZE }) > 20, `round ${round} should load`);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round} palette and platforms should draw`);
    rounds.push(await countGamePixels(page, await canvas.screenshot(), [24, 48, 576, 480], 'lit', BUBBLER_SIZE));
    await exitTape(page);
  }
  assert.equal(new Set(rounds).size, 8, `each round should look its own (${rounds})`);
});

test('BUBBLER round two mixes walkers, hoppers, and floating ghosts', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  const canvas = await openTape(page, origin, 'BUBBLER', quietBubbler('START_GAME:LOAD_ROUND 2:SHOW "PLAY"',
    { 165: statusMarker('TOY_KIND(1)=3 AND TOY_KIND(3)=1 AND TOY_KIND(5)=2 AND TOY_ON(1)') }));
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20, { size: BUBBLER_SIZE }) > 20, 'round two should hold ghosts, walkers and hoppers');
  const first = await canvas.screenshot();
  await page.waitForTimeout(450);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'walker, hopper, and ghost updates should run cleanly');
  assert.notDeepEqual(await canvas.screenshot(), first, 'ghost bobbing and enemy motion should animate the round');
  await exitTape(page);
});

test('BUBBLER co-op gives player two their own movement, jump, and bubble attack', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  const canvas = await openTape(page, origin, 'BUBBLER', { 165: statusMarker('JOINED(2) AND PX(2)<SPAWN_X(2)') });
  await canvas.focus();
  await page.keyboard.press('j');
  await page.waitForTimeout(2200);
  await page.keyboard.down('ArrowLeft');
  await page.waitForTimeout(200);
  await page.keyboard.up('ArrowLeft');
  await page.keyboard.press('k');
  await page.keyboard.press('j');
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20, { size: BUBBLER_SIZE }) > 20, 'J should start a game for two, and the arrows move player two');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'co-op key reads and the second player update should run cleanly');
  assert.ok(await soundCount(page) > 0, 'player two jump and bubble actions should use the original sound effects');
  await exitTape(page);
});

test('BUBBLER round four ghosts launch aimed sparks with the original sound', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  const canvas = await openTape(page, origin, 'BUBBLER', quietBubbler(
    'START_GAME:LOAD_ROUND 4:SHOW "PLAY":HURRY_UP:FOR I=1 TO TOY_COUNT:TOY_SHOT_WAIT(I)=0:NEXT I',
    { 165: 'FOR I=1 TO MAX_SHOTS:IF SHOT_ON(I) AND SHOT_SPARK(I) THEN SPARKED=1', 166: 'NEXT I', 167: statusMarker('SPARKED') }));
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20, { size: BUBBLER_SIZE }) > 20, 'angry ghosts should aim sparks from round four');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'aimed sparks should run without BASIC or runtime errors');
  await exitTape(page);
});

test('BUBBLER round three walkers roll rocks at players on their platform', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  const canvas = await openTape(page, origin, 'BUBBLER', quietBubbler(
    'START_GAME:LOAD_ROUND 3:SHOW "PLAY":TOY_X(1)=PX(1)+150:TOY_Y(1)=PY(1)+2:TOY_KIND(1)=1:TOY_SHOT_WAIT(1)=0',
    { 165: 'FOR I=1 TO MAX_SHOTS:IF SHOT_ON(I) AND NOT SHOT_SPARK(I) THEN ROLLED=1', 166: 'NEXT I', 167: statusMarker('ROLLED') }));
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20, { size: BUBBLER_SIZE }) > 20, 'a walker in line with a player should wind up and roll a rock');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'rolling rocks should run without BASIC or runtime errors');
  await exitTape(page);
});

test('BUBBLER enemies jump between platform heights', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  const canvas = await openTape(page, origin, 'BUBBLER', quietBubbler('START_GAME:LOAD_ROUND 2:SHOW "PLAY"',
    { 165: 'IF TOY_VY(5)<-200 THEN JUMPED=1', 166: statusMarker('JUMPED') }));
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 20, { timeout: 6000, size: BUBBLER_SIZE }) > 20, 'a hopper should crouch and spring into the air');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'enemy jumps and platform landings should run without BASIC errors');
  await exitTape(page);
});

test('BOUNCER aims the ball off the paddle and returns it upward', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const canvas = await openTape(page, origin, 'BOUNCER', {
    85: 'START_GAME:STATE$="PLAY":BALL_X(1)=PAD_X+20:BALL_Y(1)=PAD_Y-30:BALL_VX(1)=0:BALL_VY(1)=150',
    165: statusMarker('BALL_VY(1)<0 AND BALL_VX(1)<0'),
  });
  assert.match(await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSource),
    /AIM=CLAMP\(\(\(BALL_X\(K\)\+BALL\/2-PAD_X\)\/PAD_W-0\.5\)\*2,-1,1\)\*1\.15/);
  await page.waitForTimeout(400);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 40) > 40, 'a ball landing left of centre goes back up and to the left');
  await page.locator('#exit-game').click();
});

test('BOUNCER seeks the nearest brick after five seconds without a break', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const canvas = await openTape(page, origin, 'BOUNCER', { 85: 'START_GAME:STATE$="PLAY":SINCE_BREAK=6:BALL_X(1)=300:BALL_Y(1)=300:BALL_VX(1)=40:BALL_VY(1)=-60' });
  let outline = 0;
  for (let attempt = 0; attempt < 8 && outline <= 8; attempt++) {
    await page.waitForTimeout(80);
    outline = await countGamePixels(page, await canvas.screenshot(), [250, 118, 110, 32], 'yellow');
  }
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'nearest-brick steering should run cleanly');
  assert.ok(outline > 8, `the nearest live brick should get a pulsing aim outline (${outline} gold pixels)`);
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('BOUNCER later levels render the Rust bomb and steel bricks', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  // Level four, with a bomb at column 1 and steel at column 2 of the second row.
  const canvas = await openTape(page, origin, 'BOUNCER', { 85: 'START_GAME:LEVEL=4:START_LEVEL:BRICK(1,1)=2:BRICK(2,1)=3' });
  await page.waitForTimeout(300);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  const shot = await canvas.screenshot();
  const bombs = await countGamePixels(page, shot, [169, 64, 36, 14], 'orange');
  const steel = await countGamePixels(page, shot, [207, 64, 36, 14], 'gray');
  assert.ok(steel > 100, `steel-brick pixels: ${steel}`);
  assert.ok(bombs > 10, `bomb-brick pixels: ${bombs}`);
});

test('BOUNCER bomb hits clear the bricks around them', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  // A bomb at (4,2) with a clear way up to it from below.
  const canvas = await openTape(page, origin, 'BOUNCER', {
    85: 'START_GAME:STATE$="PLAY":BRICK(4,2)=2:BRICK(4,3)=0:BRICK(4,4)=0:BRICK(4,5)=0:BALL_X(1)=FNBRICK_X(4)+11:BALL_Y(1)=200:BALL_VX(1)=0:BALL_VY(1)=-200',
    165: statusMarker('BRICK(4,2)=0 AND BRICK(3,2)=0 AND BRICK(5,1)=0 AND BRICK(3,3)=0'),
  });
  await page.waitForTimeout(900);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 40) > 40, 'the bomb and its eight neighbours should be gone');
});

test('BOUNCER multi-ball pickup splits into three balls', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const canvas = await openTape(page, origin, 'BOUNCER', {
    85: 'START_GAME:STATE$="PLAY":BALL_Y(1)=250:BALL_VY(1)=-200:CATCH 4',
    165: statusMarker('BALL_ON(1)+BALL_ON(2)+BALL_ON(3)=3'),
  });
  await page.waitForTimeout(300);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 40) > 40, 'three balls should be in play');
});

test('BOUNCER wide, narrow, slow, and life pickups take effect', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  // One pickup of each kind falling onto the paddle; a drop holds its kind plus one.
  const drops = [1, 2, 3, 4].map((kind) => `DROP(${kind})=${kind}:DROP_X(${kind})=PAD_X+10:DROP_Y(${kind})=PAD_Y-30-${kind * 4}`).join(':');
  const canvas = await openTape(page, origin, 'BOUNCER', {
    85: `START_GAME:STATE$="PLAY":BALL_Y(1)=200:BALL_VY(1)=-100:${drops}`,
    165: statusMarker('LIVES=4 AND SLOW_LEFT>0 AND WIDTH_LEFT>0'),
  });
  await page.waitForTimeout(900);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  assert.ok(await waitForPixels(page, canvas, STATUS_MARKER, 'green', 40) > 40, 'every pickup should have been caught');
});

test('BOUNCER cleared and game-over screens keep rendering while they wait', async (t) => {
  for (const [screen, lines, colour] of [
    ['cleared', 'START_GAME:STATE$="CLEAR":WAIT_LEFT=99', 'green'],
    ['over', 'START_GAME:STATE$="OVER":WAIT_LEFT=2', 'red'],
  ]) {
    const { page, origin } = await openPage(t, 'chromium');
    await page.emulateMedia({ reducedMotion: 'reduce' });
    const canvas = await openTape(page, origin, 'BOUNCER', { 85: lines });
    await page.waitForTimeout(900);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `${screen}: no interpreter error`);
    const coloured = await countGamePixels(page, await canvas.screenshot(), [140, 100, 360, 150], colour);
    assert.ok(coloured > 100, `${screen}: expected visible coloured title text, found ${coloured} pixels`);
  }
});

test('BOUNCER clears its last brick and advances to the next level automatically', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const canvas = await openTape(page, origin, 'BOUNCER', {
    85: 'START_GAME:FOR R=0 TO 5:FOR C=0 TO 9:BRICK(C,R)=0:NEXT C:NEXT R:BRICK(5,5)=1:BRICKS_LEFT=1:STATE$="PLAY":BALL_X(1)=FNBRICK_X(5)+11:BALL_Y(1)=200:BALL_VX(1)=0:BALL_VY(1)=-250',
  });
  await page.waitForTimeout(700);
  const cleared = await canvas.screenshot();
  const wall = [131, 48, 378, 96];
  const before = await countGamePixels(page, cleared, wall, 'lit');
  assert.ok(await countGamePixels(page, cleared, [140, 160, 360, 80], 'green') > 100, 'the cleared screen should show');
  let after = 0;
  for (let attempt = 0; attempt < 20 && after <= before * 3; attempt++) {
    await page.waitForTimeout(150);
    after = await countGamePixels(page, await canvas.screenshot(), wall, 'lit');
  }
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'BOUNCER should keep running');
  assert.ok(after > before * 3, `level two should rebuild its brick wall: ${before} -> ${after}`);
});

// Game tests start a tape mid-game through line 85, draw a status marker
// through line 165 (before the frame's WAIT), and read pixels in the 640x400
// game's own coordinates.
async function openTape(page, origin, tape, lines = {}) {
  await page.goto(`${origin}/basic.html`);
  await page.evaluate((tape) => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD ' + tape); }, tape);
  await page.waitForFunction((tape) => document.querySelector('#basic-output').textContent.includes(`LOADED /TAPE/${tape}.BLIP`), tape);
  await page.evaluate((lines) => {
    for (const [n, code] of Object.entries(lines)) BlipBasic.lines.set(Number(n), code);
    BlipBasic.submit('RUN');
  }, lines);
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'));
  return page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
}

const openDefender = (page, origin, lines) => openTape(page, origin, 'DEFENDER', lines);

/** Green when `condition` holds, red when it does not, at game (580,250). */
const statusMarker = (condition) => `FILL_RECT(580,250,40,24,COLOR(1-(${condition}),${condition},0))`;
const STATUS_MARKER = [580, 250, 40, 24];
const defenderMarker = statusMarker;
const DEFENDER_MARKER = STATUS_MARKER;

/** Screenshots until `rect` holds more than `least` pixels of `colour`; returns that count. */
async function waitForPixels(page, canvas, rect, colour, least, { timeout = 4000, size } = {}) {
  const until = Date.now() + timeout;
  let count = 0;
  do {
    count = await countGamePixels(page, await canvas.screenshot(), rect, colour, size);
    if (count > least) break;
    await page.waitForTimeout(100);
  } while (Date.now() < until);
  return count;
}

/** `rect` is in game pixels, on a screen `size` big: 640x400 unless the tape picks another. */
async function countGamePixels(page, png, rect, colour, size = [640, 400]) {
  return page.evaluate(async ([base64, rect, colour, [wide, high]]) => {
    const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const scale = Math.min(sample.width / wide, sample.height / high);
    const left = (sample.width - wide * scale) / 2, top = (sample.height - high * scale) / 2;
    const { data } = context.getImageData(Math.floor(left + rect[0] * scale), Math.floor(top + rect[1] * scale),
      Math.max(1, Math.ceil(rect[2] * scale)), Math.max(1, Math.ceil(rect[3] * scale)));
    const is = {
      red: (r, g, b) => r > 120 && r > g * 1.35 && r > b * 1.2,
      green: (r, g, b) => g > 120 && g > r * 1.35 && g > b * 1.2,
      orange: (r, g, b) => r > 120 && g > 45 && g < 190 && b < 110,
      white: (r, g, b) => r > 190 && g > 190 && b > 190,
      yellow: (r, g, b) => r > 130 && g > 100 && b < 110,
      lit: (r, g, b) => r + g + b > 180,
      cyan: (r, g, b) => g > 120 && b > 120 && r < g * 0.75,
      blue: (r, g, b) => b > 140 && b > r * 1.5 && b > g * 1.2,
      gray: (r, g, b) => r > 110 && g > 110 && b > 110 && Math.max(r, g, b) - Math.min(r, g, b) < 60,
    }[colour];
    let count = 0;
    for (let i = 0; i < data.length; i += 4) if (is(data[i], data[i + 1], data[i + 2])) count++;
    return count;
  }, [png.toString('base64'), rect, colour, size]);
}

test('DEFENDER.BLIP runs a keyboard-controlled game through Rust Blip primitives', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => BlipBasic.submit('CD /TAPE'));
  await page.evaluate(() => BlipBasic.submit('LOAD DEFENDER'));
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/DEFENDER.BLIP'));
  await page.evaluate(() => BlipBasic.submit('DIR'));
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('DEFENDER.BLIP'));
  assert.ok(await page.evaluate(() => BlipBasic.lines.size > 70));
  await page.locator('#basic-input').fill('RUN');
  await page.locator('#basic-input').press('Enter');
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
  assert.ok(await page.locator('#exit-game').isVisible());
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.press('Space');
  await page.waitForTimeout(120);
  await canvas.press('ArrowLeft');
  await canvas.press('ArrowUp');
  await canvas.press('Space');
  await page.waitForTimeout(500);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the BASIC source should not produce a Rust interpreter error');
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('READY'));
  assert.equal(await page.evaluate(() => document.activeElement.id), 'basic-input');
});

test('DEFENDER renders its alien formation in five fresh playtests', async (t) => {
  for (let round = 1; round <= 5; round++) {
    const handle = await startPage('chromium');
    const { page, origin } = handle;
    try {
      await page.emulateMedia({ reducedMotion: 'reduce' });
      const canvas = await openDefender(page, origin);
      await canvas.press('Space');
      await page.waitForTimeout(180);
      await canvas.focus();
      await page.keyboard.down('Space');
      // Level one: four rows of ten, 44 by 28 pixels apart, from (104,58).
      const formation = async (png) => {
        let cells = 0;
        for (let row = 0; row < 4; row++) for (let col = 0; col < 10; col++) {
          if (await countGamePixels(page, png, [104 + col * 44, 58 + row * 28, 34, 22], 'lit') > 8) cells++;
        }
        return cells;
      };
      // Wait for the first frame of play with a shot in flight, not for a clock.
      let playerShot, occupiedCells = 0, playerShotPixels = 0;
      for (let attempt = 0; attempt < 40 && (occupiedCells < 20 || playerShotPixels < 5); attempt++) {
        await page.waitForTimeout(50);
        playerShot = await canvas.screenshot();
        occupiedCells = await formation(playerShot);
        playerShotPixels = await countGamePixels(page, playerShot, [220, 60, 200, 300], 'white');
      }
      // An explosion lasts under half a second: watch for one as the shots land.
      let explosionPixels = 0;
      for (let attempt = 0; attempt < 30 && explosionPixels < 4; attempt++) {
        await page.waitForTimeout(60);
        explosionPixels = await countGamePixels(page, await canvas.screenshot(), [40, 40, 560, 220], 'orange');
      }
      await page.keyboard.up('Space');
      await page.waitForTimeout(1500);
      let alienShotPixels = 0;
      for (let attempt = 0; attempt < 20 && alienShotPixels < 5; attempt++) {
        await page.waitForTimeout(100);
        alienShotPixels = await countGamePixels(page, await canvas.screenshot(), [40, 60, 560, 290], 'orange');
      }
      const visual = { occupiedCells, playerShotPixels, explosionPixels, alienShotPixels };
      assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: BASIC runtime error`);
      assert.ok(occupiedCells >= 20, `round ${round}: expected a visible 4-by-10 alien formation, got ${JSON.stringify(visual)}`);
      assert.ok(playerShotPixels >= 5, `round ${round}: player shot should be visible, got ${JSON.stringify(visual)}`);
      assert.ok(explosionPixels >= 4, `round ${round}: alien hit burst should be visible, got ${JSON.stringify(visual)}`);
      assert.ok(alienShotPixels >= 5, `round ${round}: alien shot should be visible, got ${JSON.stringify(visual)}`);
      await page.locator('#exit-game').click();
      await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
    } finally { await handle.close(); }
  }
});

test('DEFENDER shots remove individual shield blocks', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const canvas = await openDefender(page, origin, {
    85: 'START_GAME:BOMB_WAIT=99:UFO_WAIT=99:SHIP_X=500:SHOT_ON=1:SHOT_X=SHIELD_X(1)+2:SHOT_Y=SHIELD_Y+40',
    165: defenderMarker('SHIELD(1,3,1)=1') + ':' + defenderMarker('SHIELD(1,2,1)=1').replace('580,250', '580,280'),
  });
  await page.waitForTimeout(250);
  const shot = await canvas.screenshot();
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  assert.ok(await countGamePixels(page, shot, DEFENDER_MARKER, 'red') > 40, 'one shot should remove the bunker block it meets');
  assert.ok(await countGamePixels(page, shot, [580, 280, 40, 24], 'green') > 40, 'the block above it should stay');
  await page.locator('#exit-game').click();
});

test('DEFENDER bomb collisions follow the tapered player ship', async (t) => {
  // A bomb by the ship's empty top corner: a ship sliding away is missed, a still one is hit at its base.
  async function markerFor(moving) {
    const handle = await startPage('chromium');
    const { page, origin } = handle;
    try {
      const canvas = await openDefender(page, origin, {
        85: 'START_GAME:BOMB_WAIT=99:UFO_WAIT=99:SHIP_X=200:BOMB(1)=1:BOMB_X(1)=203:BOMB_Y(1)=SHIP_Y-10' + (moving ? ':SHIP_VX=200' : ''),
        165: defenderMarker('LIVES=3'),
        9720: moving ? 'STICK=1' : 'STICK=0',
      });
      await page.waitForTimeout(400);
      const shot = await canvas.screenshot();
      return { red: await countGamePixels(page, shot, DEFENDER_MARKER, 'red'), green: await countGamePixels(page, shot, DEFENDER_MARKER, 'green') };
    } finally { await handle.close(); }
  }
  const cornerMiss = await markerFor(true);
  const bodyHit = await markerFor(false);
  assert.ok(cornerMiss.green > 40, `a bomb through the empty top corner of a ship moving away should miss: ${JSON.stringify(cornerMiss)}`);
  assert.ok(bodyHit.red > 40, `the same bomb reaches a still ship's body: ${JSON.stringify(bodyHit)}`);
});

test('DEFENDER renders four simultaneous Rust-sized alien bombs', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const xs = [40, 140, 240, 460];
  const canvas = await openDefender(page, origin, {
    85: 'START_GAME:BOMB_WAIT=99:UFO_WAIT=99:BOMB_SPEED=0:' + xs.map((x, i) => `BOMB(${i + 1})=1:BOMB_X(${i + 1})=${x}:BOMB_Y(${i + 1})=${200 + i * 10}`).join(':'),
  });
  await page.waitForTimeout(150);
  const shot = await canvas.screenshot();
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  const counts = [];
  for (const [i, x] of xs.entries()) counts.push(await countGamePixels(page, shot, [x - 2, 195 + i * 10, 8, 20], 'orange'));
  counts.forEach((count, i) => assert.ok(count > 3, `alien bomb ${i + 1} should be visible, orange pixels: ${counts}`));
  await page.locator('#exit-game').click();
});

test('DEFENDER UFO laser clears its column and fires a beam', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const canvas = await openDefender(page, origin, {
    85: 'START_GAME:BOMB_WAIT=99:SHIP_X=520:UFO_ON=1:UFO_MODE$="CHARGING":UFO_TIMER=0.01:UFO_X=SHIELD_X(2):UFO_DIR=1',
    6260: 'REM HOLD THE BEAM FOR THE SCREENSHOT',
    165: defenderMarker('SHIELD(2,1,1)=1'),
  });
  await page.waitForTimeout(200);
  const shot = await canvas.screenshot();
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  const beam = await countGamePixels(page, shot, [242, 60, 12, 300], 'white');
  assert.ok(beam > 200, `the UFO should fire the white core of its beam, got ${beam} pixels`);
  assert.ok(await countGamePixels(page, shot, DEFENDER_MARKER, 'red') > 40, 'the beam should clear the shield cells in its column');
  await page.locator('#exit-game').click();
});

test('DEFENDER UFO drops its delayed bomb and the bomb breaks shields', async (t) => {
  async function render(lines, rect, colour, wait) {
    const handle = await startPage('chromium');
    const { page, origin } = handle;
    try {
      const canvas = await openDefender(page, origin, lines);
      await page.waitForTimeout(wait);
      assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
      return await countGamePixels(page, await canvas.screenshot(), rect, colour);
    } finally { await handle.close(); }
  }
  const flying = await render({ 85: 'START_GAME:BOMB_WAIT=99:SHIP_X=520:UFO_ON=1:UFO_MODE$="FLYING":UFO_DIR=1:UFO_X=200:UFO_TIMER=0.01' }, [196, 40, 90, 70], 'orange', 120);
  const shield = await render({
    85: 'START_GAME:BOMB_WAIT=99:SHIP_X=520:UFO_BOMB=1:UFO_BOMB_X=SHIELD_X(3)+12:UFO_BOMB_Y=SHIELD_Y-30',
    165: defenderMarker('SHIELD(3,1,2)=1'),
  }, DEFENDER_MARKER, 'red', 400);
  assert.ok(flying > 6, `the UFO should drop its orange bomb, got ${flying} pixels`);
  assert.ok(shield > 40, `the UFO bomb should remove a shield cell, got ${shield} red pixels`);
});

test('DEFENDER higher waves initialize each formation theme', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const level of [2, 3, 4, 5]) {
    const canvas = await openDefender(page, origin, { 85: `START_GAME:LEVEL=${level}:START_WAVE:BOMB_WAIT=99:UFO_WAIT=99` });
    let lit = 0;
    for (let attempt = 0; attempt < 6 && lit <= 800; attempt++) {
      await page.waitForTimeout(attempt === 0 ? 300 : 250);
      lit = await countGamePixels(page, await canvas.screenshot(), [40, 50, 560, 200], 'lit');
    }
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `level ${level} should run without an interpreter error`);
    assert.ok(lit > 800, `level ${level} should render its formation, got ${lit} lit pixels`);
    await page.locator('#exit-game').click();
    await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
  }
});

test('DEFENDER starts a fifth-wave mothership with a visible health bar', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.route('**/basic-runtime.html*', async (route) => {
    const response = await route.fetch();
    await route.fulfill({ response, body: (await response.text()).replace('imports.env.blip_basic_sound=playBasicTone;',
      'imports.env.blip_basic_sound=function(f,s,v){if(f===1777)window.__mothershipVolley=true;else playBasicTone(f,s,v);};') });
  });
  // The fifth wave with its formation gone: the mothership comes; the frame
  // freezes once its first volley is clear of its hull.
  const canvas = await openDefender(page, origin, {
    85: 'START_GAME:LEVEL=5:START_WAVE:FOR I=1 TO ROWS*COLS:ALIVE(I)=0:NEXT I:ALIENS_LEFT=0:SHIP_X=0:UFO_WAIT=99',
    165: 'IF BOSS_ON AND BOMB(1) AND BOMB(2) AND BOMB(3) AND BOMB_Y(1)>FNBOSS_Y()+BOSS_H+20 THEN SOUND 1777,0,0:WAIT 0:GOTO 9990',
    9990: 'WAIT 0:GOTO 9990',
  });
  await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.__mothershipVolley, undefined, { timeout: 10000 });
  const shot = await canvas.screenshot();
  const hull = await countGamePixels(page, shot, [0, 40, 640, 130], 'red');
  const bombs = await countGamePixels(page, shot, [0, 120, 640, 250], 'orange');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the boss encounter should not produce an interpreter error');
  assert.ok(hull > 100, `the mothership should render its red hull and health bar, got ${hull} red pixels`);
  assert.ok(bombs > 8, `the mothership should fire a three-shot volley, got ${bombs} orange pixels`);
  await page.locator('#exit-game').click();
});

test('DEFENDER telegraphs and resolves its wave-three diver', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  // Wave three is three rows of eleven from (82,58): the last alien, at (522,114), shakes and flashes.
  const canvas = await openDefender(page, origin, { 85: 'START_GAME:LEVEL=3:START_WAVE:BOMB_WAIT=99:UFO_WAIT=99:DIVE_PICK=ROWS*COLS:DIVE_TELL=3' });
  let flash = 0;
  for (let attempt = 0; attempt < 20 && flash <= 10; attempt++) {
    await page.waitForTimeout(60);
    flash = await countGamePixels(page, await canvas.screenshot(), [400, 100, 240, 90], 'white');
  }
  assert.ok(flash > 10, `the selected alien should flash white before diving: ${flash} pixels`);
  // Then it dives, and the game carries on.
  await page.waitForTimeout(3500);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the diver state should run without an interpreter error');
  await page.locator('#exit-game').click();
});

test('DEFENDER shots award the double value for a diver kill', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const canvas = await openDefender(page, origin, {
    85: 'START_GAME:LEVEL=3:START_WAVE:BOMB_WAIT=99:UFO_WAIT=99:DIVER=1:DIVER_KIND=0:DIVER_X=300:DIVER_Y=200:SHOT_ON=1:SHOT_X=314:SHOT_Y=240',
    165: defenderMarker('SCORE=180'),
    // The popup holds still, so the check does not race its 0.8 second life.
    ...Object.fromEntries(tapeLines('DEFENDER', 'FX_UPDATE(DT)').map((n) => [n, 'REM'])),
  });
  assert.ok(await waitForPixels(page, canvas, DEFENDER_MARKER, 'green', 40) > 40, 'a squid diver at level three should score 30x3x2');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'a diver hit should not produce an interpreter error');
  const yellow = await countGamePixels(page, await canvas.screenshot(), [230, 140, 180, 90], 'yellow');
  assert.ok(yellow > 10, `the diver kill should show its bonus score, got ${yellow} yellow pixels`);
  await page.locator('#exit-game').click();
});

test('DEFENDER divers smash through shield blocks', async (t) => {
  async function shieldMarker(diving) {
    const handle = await startPage('chromium');
    const { page, origin } = handle;
    try {
      const canvas = await openDefender(page, origin, {
        85: 'START_GAME:LEVEL=4:START_WAVE:BOMB_WAIT=99:UFO_WAIT=99:SHIP_X=600' + (diving ? ':DIVER=1:DIVER_KIND=1:DIVER_X=SHIELD_X(2):DIVER_Y=SHIELD_Y-40' : ''),
        165: defenderMarker('SHIELD(2,1,2)=1'),
      });
      await page.waitForTimeout(400);
      assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the shield dive should not produce an interpreter error');
      const shot = await canvas.screenshot();
      return { red: await countGamePixels(page, shot, DEFENDER_MARKER, 'red'), green: await countGamePixels(page, shot, DEFENDER_MARKER, 'green') };
    } finally { await handle.close(); }
  }
  const intact = await shieldMarker(false);
  const broken = await shieldMarker(true);
  assert.ok(intact.green > 40, `the intact shield should report green, got ${JSON.stringify(intact)}`);
  assert.ok(broken.red > 40, `the diver should smash the shield and report red, got ${JSON.stringify(broken)}`);
});

test('EDIT fills the monitor, highlights and completes code, auto-saves edits, and exits with ESC', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => BlipBasic.submit('EDIT SONG'));
  assert.equal(await page.locator('#basic-editor').isVisible(), true);
  const bounds = await page.evaluate(() => {
    const editor = document.querySelector('#basic-editor').getBoundingClientRect();
    const screen = document.querySelector('#basic-screen').getBoundingClientRect();
    return { editor, screen };
  });
  assert.ok(bounds.editor.width >= bounds.screen.width - 2, 'editor should fill the monitor width');
  assert.ok(bounds.editor.height >= bounds.screen.height - 2, 'editor should fill the monitor height');
  assert.equal(await page.locator('#basic-editor button').count(), 0, 'ESC is the editor exit control');
  assert.equal(await page.locator('.front-nav').isVisible(), false, 'navigation should not provide another way out');
  await page.keyboard.type('1');
  assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('blip-basic-filesystem')).files['/USER/SONG.BLIP']), '1', 'the first typed key should be saved');
  await page.keyboard.type('0 PRINT "HELLO"');
  assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('blip-basic-filesystem')).files['/USER/SONG.BLIP']), '10 PRINT "HELLO"', 'each key should persist immediately');
  assert.equal(await page.locator('#editor-code .editor-keyword').first().textContent(), 'PRINT');
  assert.equal(await page.locator('#editor-code .editor-string').first().textContent(), '"HELLO"');
  await page.keyboard.press('Enter');
  await page.keyboard.type('20 FILL_SUNB');
  assert.equal(await page.locator('.editor-completion').textContent(), 'URST');
  await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('blip-basic-filesystem')).files['/USER/SONG.BLIP']), '10 PRINT "HELLO"\n20 FILL_SUNBURST');
  assert.match(await page.locator('#basic-output').textContent(), /AUTO-SAVE/);
  await page.keyboard.press('Escape');
  assert.equal(await page.locator('#basic-editor').isVisible(), false, 'Escape should be the editor exit key');
  assert.match(await page.locator('#basic-output').textContent(), /EDITOR CLOSED/);
  await page.evaluate(() => BlipBasic.submit('LOAD SONG'));
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /USER/SONG.BLIP'));
  assert.equal(await page.evaluate(() => BlipBasic.lines.get(20)), 'FILL_SUNBURST');
  await page.evaluate(() => BlipBasic.submit('EDIT SONG'));
  await page.locator('[data-key-label="ESC"]').click();
  assert.equal(await page.locator('#basic-editor').isVisible(), false, 'on-screen ESC should exit the editor too');
});

test('USER directories and saved .blip files persist across reloads', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('MKDIR games');
    BlipBasic.submit('CD games');
    BlipBasic.submit('10 PRINT "TEAPOT"');
    BlipBasic.submit('20 END');
    BlipBasic.submit('SAVE /USER/games/teapot');
  });
  assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('blip-basic-filesystem')).files['/USER/GAMES/TEAPOT.BLIP']), '10 PRINT "TEAPOT"\n20 END');
  await page.reload();
  await page.evaluate(() => BlipBasic.submit('CD /USER/games'));
  await page.evaluate(() => BlipBasic.submit('DIR'));
  assert.match(await page.locator('#basic-output').textContent(), /DIRECTORY OF \/USER\/GAMES[\s\S]*TEAPOT\.BLIP/);
  await page.evaluate(() => { BlipBasic.submit('NEW'); BlipBasic.submit('LOAD teapot.blip'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /USER/GAMES/TEAPOT.BLIP'));
  await page.reload();
  await page.evaluate(() => BlipBasic.submit('SAVE'));
  assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('blip-basic-filesystem')).files['/USER/GAMES/TEAPOT.BLIP']), '10 PRINT "TEAPOT"\n20 END');
  await page.evaluate(() => BlipBasic.submit('RUN'));
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('TEAPOT'));
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('SAVE forbidden'); });
  assert.match(await page.locator('#basic-output').textContent(), /SAVED \/USER\/FORBIDDEN\.BLIP/);
  assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('blip-basic-filesystem')).files['/USER/FORBIDDEN.BLIP']), '10 PRINT "TEAPOT"\n20 END');
});

test('DEL removes USER files and protects the tapes', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.route('**/*.wasm', () => {});
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('10 PRINT "TEMP"');
    BlipBasic.submit('SAVE temp');
    BlipBasic.submit('DEL temp');
    BlipBasic.submit('DEL /TAPE/BOUNCER');
  });
  const output = await page.locator('#basic-output').textContent();
  assert.match(output, /DELETED \/USER\/TEMP\.BLIP/);
  assert.match(output, /ONLY \/USER FILES CAN BE DELETED/);
  assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('blip-basic-filesystem')).files['/USER/TEMP.BLIP']), undefined);
  await page.evaluate(() => BlipBasic.submit('DIR'));
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('READY'));
  assert.match(await page.locator('#basic-output').textContent(), /DIRECTORY OF \/USER\n<EMPTY>/);
});

test('DIR and CD expose the inserted tape\'s files under TAPE/<GAME>/ASSETS', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => BlipBasic.submit('LOAD /TAPE/BOUNCER'));
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BOUNCER.BLIP'));
  await page.evaluate(() => { BlipBasic.submit('DIR /TAPE/BOUNCER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('ASSETS/'));
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE/BOUNCER/ASSETS'); BlipBasic.submit('DIR'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('PADDLE.PNG'));
  const output = await page.locator('#basic-output').textContent();
  assert.match(output, /\/TAPE\/BOUNCER[\s\S]*\/TAPE\/BOUNCER\/ASSETS[\s\S]*BALL\.PNG[\s\S]*BRICK_RED\.PNG/);
  await page.evaluate(() => {
    BlipBasic.submit('10 CLEAR("BLIP_BLACK")');
    BlipBasic.submit('20 DRAW_TEXTURE("/TAPE/BOUNCER/ASSETS/BALL.PNG",40,40,16,16)');
    BlipBasic.submit('30 WAIT 16');
    BlipBasic.submit('40 GOTO 10');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  assert.match(await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSource), /\/TAPE\/BOUNCER\/ASSETS\/BALL\.PNG/);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  await page.locator('#exit-game').click();
});

test('the BASIC book turns one readable page at a time on a phone', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 390, height: 844 } });
  await page.goto(`${origin}/basic.html`);
  const bookLink = page.locator('#basic-book-peek');
  assert.equal(await bookLink.getAttribute('aria-label'), 'Open the BLIP BASIC programming book');
  assert.ok(await bookLink.locator('img').evaluate((img) => img.complete && img.naturalWidth > 0));
  const homeCoverSrc = await bookLink.locator('img').getAttribute('src');
  assert.equal(homeCoverSrc, 'blip-basic-cover.svg', 'the corner shows the book\'s own front cover');
  await bookLink.click();
  await page.waitForURL(`${origin}/basic-book.html`);
  await page.waitForFunction(() => document.body.dataset.bookState !== undefined);
  await page.waitForTimeout(250);
  assert.equal(await page.evaluate(() => document.body.dataset.bookState), 'closed', 'the book waits closed so its front cover can be seen');
  const opening = await page.locator('.opening-spread').screenshot();
  await page.locator('.book-cover').click();
  await page.waitForFunction(() => document.body.dataset.bookState === 'open', undefined, { timeout: 5000 });
  const opened = await page.locator('.opening-spread').screenshot();
  assert.notDeepEqual(opened, opening, 'the same hard cover should swing open to reveal the printed contents');
  assert.equal(await page.locator('a').count(), 0, 'the programming manual is a book, without browser-style links');
  assert.equal(await page.locator('h1').textContent(), 'CONTENTS');
  assert.equal(await page.locator('.cover-front img').getAttribute('src'), 'blip-basic-cover.svg');
  assert.ok(await page.locator('.cover-front img').evaluate((img) => img.complete && img.naturalWidth > 0));
  assert.equal(await page.locator('.contents-list li').count(), 20);
  assert.equal(await page.locator('.reader-page:visible').count(), 0);
  await page.locator('[data-turn="next"]').click();
  assert.equal(await page.locator('.reader-page:visible').count(), 1);
  assert.match(await page.locator('.reader-page:visible').innerText(), /YOUR FIRST PROGRAM/);
  for (let turn = 0; turn < 20 && !(await page.locator('.reader-page:visible').innerText()).includes('DRAW WITH BLIP'); turn += 1) await page.locator('[data-turn="next"]').click();
  assert.equal(await page.locator('.reader-page:visible').count(), 1);
  assert.equal(await page.locator('.reader-page:visible .primitive-list > div').count(), 16);
  for (let turn = 0; turn < 20 && !(await page.locator('.reader-page:visible').innerText()).includes('OPEN A GAME'); turn += 1) await page.locator('[data-turn="next"]').click();
  assert.match(await page.locator('.reader-page:visible').innerText(), /LOAD \/TAPE\/BOUNCER\.BLIP[\s\S]*RUN/);
  await page.locator('[data-turn="next"]').click();
  assert.match(await page.locator('.reader-page:visible').innerText(), /SOURCE[\s\S]*SAVE MY_BOUNCER[\s\S]*EDIT \/USER\/MY_BOUNCER\.BLIP[\s\S]*RUN/);
  await page.locator('[data-turn="next"]').click();
  assert.match(await page.locator('.reader-page:visible').innerText(), /ARROW KEYS[\s\S]*HOME \/ END[\s\S]*TAB[\s\S]*ESCAPE[\s\S]*saved to the open/);
  await page.locator('[data-turn="prev"]').click();
  assert.match(await page.locator('.reader-page:visible').innerText(), /READ BEFORE YOU CHANGE[\s\S]*MAKE YOUR OWN COPY/);
  assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
});

test('the BASIC book shows only its current two-page spread on desktop', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1180, height: 900 } });
  await page.goto(`${origin}/basic-book.html`);
  await page.waitForTimeout(800);
  assert.equal(await page.evaluate(() => document.body.dataset.bookState), 'closed');
  await page.locator('[data-turn="next"]').click();
  await page.waitForFunction(() => document.body.dataset.bookState === 'open');
  assert.equal(await page.locator('.opening-spread:visible .page').count(), 2);
  assert.match(await page.locator('.blank-page').innerText(), /Welcome to your programming adventure\.[\s\S]*Kind regards,[\s\S]*Jacob Andresen/);
  assert.equal(await page.locator('.reader-page:visible').count(), 0);
  await page.locator('[data-turn="next"]').click();
  assert.equal(await page.locator('.reader-spread > .reader-page:visible').count(), 2);
  assert.match(await page.locator('.reader-spread').innerText(), /YOUR FIRST PROGRAM[\s\S]*NUMBERS, CHOICES & LOOPS/);
  await page.waitForTimeout(1000);
  await page.locator('[data-turn="next"]').click();
  await page.waitForTimeout(1000);
  assert.equal(await page.locator('.reader-spread > .reader-page:visible').count(), 2);
  assert.match(await page.locator('.reader-spread').innerText(), /NAME YOUR ROUTINES\.[\s\S]*BLOCKS, LOOPS AND DATA\./);
  const fit = await page.locator('.reader-spread > .reader-page').evaluateAll((pages) => pages.map((sheet) => ({ overflow: getComputedStyle(sheet).overflowY, fits: sheet.scrollHeight <= sheet.clientHeight + 1, scroll: sheet.scrollHeight, height: sheet.clientHeight, chapter: sheet.firstElementChild && sheet.firstElementChild.getBoundingClientRect().height })));
  assert.ok(fit.every((sheet) => sheet.overflow === 'hidden' && sheet.fits), JSON.stringify(fit));
  for (let spread = 0; spread < 8; spread += 1) {
    await page.locator('[data-turn="next"]').click();
    await page.waitForTimeout(700);
    const pages = await page.locator('.reader-spread > .reader-page').evaluateAll((sheets) => sheets.map((sheet) => ({ fits: sheet.scrollHeight <= sheet.clientHeight + 1, scroll: sheet.scrollHeight, height: sheet.clientHeight, chapter: sheet.firstElementChild && sheet.firstElementChild.id, zoom: sheet.firstElementChild && getComputedStyle(sheet.firstElementChild).zoom, box: sheet.firstElementChild && sheet.firstElementChild.getBoundingClientRect().height })));
    assert.equal(pages.length, 2);
    assert.ok(pages.every((sheet) => sheet.fits), JSON.stringify(pages));
  }
  assert.equal(await page.locator('[data-close-book]').innerText(), '▣ CLOSE BOOK');
  await page.locator('[data-close-book]').click();
  await page.waitForURL(`${origin}/basic.html`);
});

test('the BLIP BASIC book peeks from beneath the desktop tape pile', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1180, height: 820 } });
  await page.goto(`${origin}/basic.html`);
  const boxes = await page.evaluate(() => {
    const rect = (selector) => { const r = document.querySelector(selector).getBoundingClientRect(); return { left:r.left, top:r.top, right:r.right, bottom:r.bottom }; };
    return { book:rect('#basic-book-peek'), rack:rect('.tape-rack'), nav:rect('.front-nav') };
  });
  assert.ok(boxes.book.top > boxes.nav.bottom, 'the cover sits in the lower screen area');
  assert.ok(boxes.book.left > boxes.rack.left, 'the cover peeks out from the lower-right side of the tape pile');
  assert.ok(boxes.book.right > 1180 && boxes.book.bottom > 820, 'only the cover corner is on screen');
  assert.ok(await page.locator('#basic-book-peek').isVisible());
  assert.equal(await page.locator('.front-nav a[href="basic-book.html"]').count(), 0, 'the physical cover on the computer page opens the book');
});

test('the BASIC home has no game toolbar and uses the BLIP 64 page title', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.goto(`${origin}/basic.html`);
  assert.equal(await page.title(), 'BLIP 64');
  // The arcade's own top bar: the BLIP badge and Home Screen.
  assert.equal(await page.locator('#basic-to-cabinet.blip-logo.led-logo .blip-led-display').count(), 1);
  assert.equal(await page.locator('#basic-to-home.home-switch').getAttribute('aria-label'), 'Switch to the home screen');
  assert.equal(await page.locator('.top-marquee-bar').isVisible(), true, 'the arcade’s top bar');
  assert.equal(await page.locator('.quick-bar, #maximize-screen, [data-command="GAMES"]').count(), 0);
  assert.equal(await page.locator('#exit-game').isVisible(), false);
});

test('the game tapes read as a physical overlapping cassette stack', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.goto(`${origin}/basic.html`);
  await page.waitForLoadState('networkidle');
  const pile = await page.locator('.game-tape').evaluateAll((tapes) => ({
    count: tapes.length,
    overlap: tapes.slice(0, -1).every((tape, index) => tapes[index + 1].getBoundingClientRect().top < tape.getBoundingClientRect().bottom),
    labels: tapes.every((tape) => tape.querySelector('.game-tape-label strong')?.textContent),
    artwork: tapes.every((tape) => tape.querySelector('.game-tape-art')?.naturalWidth > 0),
    reelWindows: tapes.every((tape) => getComputedStyle(tape.querySelector('.game-tape-reels')).height !== '0px'),
    visibleControls: document.querySelectorAll('.tape-rack button, .tape-rack h1, .tape-rack h2').length,
    screwHeads: getComputedStyle(tapes[0].querySelector('.game-tape-case')).backgroundImage.includes('radial-gradient'),
    angles: tapes.map((tape) => Number.parseFloat(getComputedStyle(tape).getPropertyValue('--tape-angle'))),
    shadows: tapes.every((tape) => getComputedStyle(tape).getPropertyValue('--tape-shadow-blur').endsWith('px')),
    images: tapes.map((tape) => ({ src: tape.querySelector('.game-tape-art').getAttribute('src'), naturalWidth: tape.querySelector('.game-tape-art').naturalWidth, complete: tape.querySelector('.game-tape-art').complete })),
  }));
  assert.equal(pile.count, 9);
  assert.equal(pile.overlap, true);
  assert.equal(pile.labels, true);
  assert.ok(pile.artwork, JSON.stringify(pile.images));
  assert.equal(pile.reelWindows, true);
  assert.equal(pile.visibleControls, 0);
  assert.equal(pile.screwHeads, true);
  assert.ok(pile.angles.some((angle) => angle < -0.5) && pile.angles.some((angle) => angle > 0.5));
  assert.equal(pile.shadows, true);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForTimeout(60);
  assert.equal(await page.locator('.game-tape').evaluateAll((tapes) => tapes.slice(0, -1).every((tape, index) => tapes[index + 1].getBoundingClientRect().left < tape.getBoundingClientRect().right)), true);
});

test('the same cassette is returned to its pile before another tape is loaded', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.goto(`${origin}/basic.html`);
  const defenderTape = page.locator('.game-tape[data-tape="DEFENDER"]:not(.tape-placeholder)');
  const defenderHandle = await defenderTape.elementHandle();
  await defenderTape.click({ position: { x: 100, y: 24 } });
  await page.waitForFunction(() => document.querySelector('.game-tape-inserting') !== null, undefined, { timeout: 12000 });
  assert.deepEqual(await defenderTape.evaluate((node) => ({
    staysInBay: node.parentElement.classList.contains('deck-bay'),
    position: getComputedStyle(node).position,
    tapeLayer: Number(getComputedStyle(node).zIndex),
    lidLayer: Number(getComputedStyle(document.querySelector('.deck-lid')).zIndex),
  })), { staysInBay: true, position: 'absolute', tapeLayer: 2, lidLayer: 4 });
  await page.waitForFunction(() => document.body.hasAttribute('data-loading-tape'), undefined, { timeout: 12000 });
  assert.deepEqual(await defenderTape.evaluate((node) => {
    const tape = node.getBoundingClientRect(), bay = document.querySelector('.deck-bay').getBoundingClientRect();
    return {
      fullySeated: tape.left >= bay.left && tape.top >= bay.top && tape.right <= bay.right && tape.bottom <= bay.bottom,
      centered: Math.abs((tape.left + tape.right - bay.left - bay.right) / 2) < 2,
      lidClosed: !document.body.hasAttribute('data-deck-open'),
      beneathLid: Number(getComputedStyle(node).zIndex) < Number(getComputedStyle(document.querySelector('.deck-lid')).zIndex),
    };
  }), { fullySeated: true, centered: true, lidClosed: true, beneathLid: true });
  const reel = page.locator('.deck-inserted-tape .tape-spool.supply');
  const loadingRotation = await reel.evaluate((el) => getComputedStyle(el).transform);
  await page.waitForTimeout(260);
  assert.notEqual(await reel.evaluate((el) => getComputedStyle(el).transform), loadingRotation);
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 25000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  assert.match(await page.locator('#basic-wasm-frame').getAttribute('src'), /basic-runtime\.html/);
  const gameFrame = page.frameLocator('#basic-wasm-frame');
  await gameFrame.locator('#glcanvas').waitFor({ state: 'visible' });
  assert.equal(await defenderTape.evaluate((node, original) => node === original && node.parentElement.classList.contains('deck-bay'), defenderHandle), true);
  await gameFrame.locator('#glcanvas').evaluate((canvas) => canvas.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'Escape', code: 'Escape' })));
  await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').hidden);

  const bouncerTape = page.locator('.game-tape[data-tape="BOUNCER"]:not(.tape-placeholder)');
  await bouncerTape.click({ position: { x: 100, y: 30 } });
  await page.waitForFunction(() => document.body.hasAttribute('data-tape-rewinding'), undefined, { timeout: 12000 });
  assert.deepEqual(await page.locator('.deck-bay').evaluate((el) => ({ closed: !document.body.hasAttribute('data-deck-open'), tapePresent: !!el.querySelector('.deck-inserted-tape') })), { closed: true, tapePresent: true });
  const rewindRotation = await reel.evaluate((el) => getComputedStyle(el, '::before').transform);
  await page.waitForTimeout(260);
  assert.notEqual(await reel.evaluate((el) => getComputedStyle(el, '::before').transform), rewindRotation);
  await page.waitForFunction(() => document.querySelector('.game-tape-flight[data-tape="DEFENDER"]') !== null, undefined, { timeout: 12000 });
  assert.equal(await defenderTape.evaluate((node) => !document.querySelector('.deck-bay').contains(node) && !document.querySelector('#tape-pile').contains(node)), true);
  await page.waitForFunction(() => document.body.hasAttribute('data-loading-tape'), undefined, { timeout: 12000 });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 25000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  assert.equal(await defenderTape.evaluate((node) => document.querySelector('#tape-pile').contains(node)), true);
  assert.equal(await bouncerTape.evaluate((node) => node.parentElement.classList.contains('deck-bay')), true);
});

test('a game tape launches its BLIP BASIC program on the home computer', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.goto(`${origin}/basic.html`);
  const tape = page.locator('.game-tape[data-tape="DEFENDER"]');
  await tape.click({ position: { x: 100, y: 24 } });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 25000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  const gameFrame = page.frameLocator('#basic-wasm-frame');
  await gameFrame.locator('#glcanvas').waitFor({ state: 'visible' });
  assert.match(await page.locator('#basic-wasm-frame').getAttribute('src'), /basic-runtime\.html/);
  assert.equal(await page.evaluate(() => Array.from(BlipBasic.lines.values()).some((line) => line.startsWith('REM DEFENDER'))), true);
  assert.equal(await page.locator('#basic-keyboard').evaluate((el) => getComputedStyle(el).display), 'none');
  await page.evaluate(() => {
    const frame = document.querySelector('#basic-wasm-frame');
    const canvas = frame.contentDocument.querySelector('#glcanvas');
    frame.contentWindow.__amstradKeys = [];
    canvas.addEventListener('keydown', (event) => frame.contentWindow.__amstradKeys.push(event.code));
    document.querySelector('.fire-buttons .arcade-btn').dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, pointerId: 1 }));
  });
  await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.__amstradKeys.includes('Space'));
  await gameFrame.locator('#glcanvas').evaluate((canvas) => canvas.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'Escape', code: 'Escape' })));
  await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').hidden);
  assert.equal(await page.locator('#basic-input').isVisible(), true);
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
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('BLIP BASIC · HELP'));
  assert.equal(await page.locator('#basic-input').inputValue(), '');
  await page.evaluate(() => {
    BlipBasic.submit('10 K$=INKEY$');
    BlipBasic.submit('20 IF K$="A" THEN 50');
    BlipBasic.submit('30 WAIT 1');
    BlipBasic.submit('40 GOTO 10');
    BlipBasic.submit('50 PRINT "TOUCH KEY"');
    BlipBasic.submit('60 END');
    BlipBasic.submit('RUN');
  });
  await page.locator('[data-key-label="A"]').tap();
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('TOUCH KEY'));
});

test('Rally gets two working paddle dials on the Amstrad deck', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD RALLY'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/RALLY.BLIP'));
  await page.evaluate(() => BlipBasic.submit('RUN'));
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
  assert.ok(await page.locator('#amstrad-rally-dial-p1').isVisible());
  assert.ok(await page.locator('#amstrad-rally-dial-p2').isVisible());
  assert.equal(await page.locator('body').getAttribute('data-rally'), '');
  assert.equal(await page.locator('.deck-side').first().evaluate(el => getComputedStyle(el).display), 'none');
  await page.evaluate(() => {
    const frame = document.querySelector('#basic-wasm-frame'), win = frame.contentWindow;
    win.__dialKeys = [];
    frame.contentDocument.querySelector('#glcanvas').addEventListener('keydown', event => win.__dialKeys.push(event.code));
  });
  const p1 = await page.locator('#amstrad-rally-dial-p1').boundingBox();
  const cx = p1.x + p1.width / 2, cy = p1.y + p1.height / 2, radius = p1.width * .35;
  await page.mouse.move(cx + radius, cy);
  await page.mouse.down();
  await page.mouse.move(cx + radius * Math.cos(.3), cy + radius * Math.sin(.3), { steps: 3 });
  await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.__dialKeys.includes('ArrowDown'));
  await page.mouse.up();
  const p2 = await page.locator('#amstrad-rally-dial-p2').boundingBox();
  await page.mouse.move(p2.x + p2.width / 2 + p2.width * .3, p2.y + p2.height / 2);
  await page.mouse.down();
  await page.mouse.move(p2.x + p2.width / 2 + p2.width * .3 * Math.cos(.3), p2.y + p2.height / 2 + p2.width * .3 * Math.sin(.3), { steps: 3 });
  await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.__dialKeys.includes('KeyK'));
  await page.mouse.up();
  await page.locator('#exit-game').click();
});

test('BASIC routes drawing calls through the Rust Blip crate and front ends can select it', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 390, height: 844 } });
  await page.goto(`${origin}/home.html`);
  assert.equal(await page.locator('#basic-switch img').getAttribute('alt'), 'BLIP');
  await page.locator('#basic-switch').click();
  await page.waitForURL(`${origin}/basic.html`);
  assert.equal(await page.locator('.front-nav a').count(), 2);
  assert.equal(await page.locator('.front-nav [aria-current="page"]').count(), 0);
  const commentOutput = await page.evaluate(() => {
    BlipBasic.submit('COMMENT_VALUE=7:REM INPUT: COMMENT_VALUE=99');
    BlipBasic.submit("COMMENT_VALUE=COMMENT_VALUE+1:'STATE: COMMENT_VALUE=999");
    BlipBasic.submit('PRINT COMMENT_VALUE');
    return BlipBasic.output.textContent;
  });
  assert.match(commentOutput, /\n8\nREADY/);
  const plotPixel = await page.evaluate(() => {
    BlipBasic.submit('PLOT 10,10'); BlipBasic.submit('DRAW 30,30');
    return [...document.querySelector('#basic-graphics').getContext('2d').getImageData(10,189,1,1).data];
  });
  assert.ok(plotPixel[0] > 200, 'PLOT changes the colour screen');
  await page.evaluate(() => {
    BlipBasic.submit("5 'STATE: A=2:GOTO 9999");
    BlipBasic.submit('8 REM INPUT: B=2:GOTO 9999');
    BlipBasic.submit('10 clear("BLIP_BLACK"):REM FRAME: GOTO 9999');
    BlipBasic.submit('20 fill_rect(11,12,3,4,"BLIP_RED")');
    BlipBasic.submit('30 A=TEXT_CX("HI",2)');
    BlipBasic.submit('40 WAIT 16');
    BlipBasic.submit('50 GOTO 10');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
  assert.equal(await page.locator('#basic-wasm-frame').isVisible(), true, 'crate calls switch execution to the Rust interpreter');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'BASIC comments must be accepted by the Rust interpreter');
  assert.match(await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSource), /FILL_RECT\(11,12,3,4/);
  assert.match(await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSource), /TEXT_CX\("HI",2\)/);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') && !document.body.hasAttribute('data-wasm-error'));
  await page.locator('#exit-game').click();
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

test('REST pauses Rust-backed BASIC execution between tones', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('10 CLEAR("BLIP_BLACK")');
    BlipBasic.submit('20 SOUND 440,0.05,0.3');
    BlipBasic.submit('30 REST 0.25');
    BlipBasic.submit('40 SOUND 550,0.05,0.3');
    BlipBasic.submit('50 WAIT 16');
    BlipBasic.submit('60 GOTO 50');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents >= 2);
  const soundTimes = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundTimes);
  assert.ok(soundTimes[1] - soundTimes[0] >= 220, `REST should delay the second tone for 0.25s, got ${soundTimes[1] - soundTimes[0]}ms`);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  await page.locator('#exit-game').click();
});

test('Rust BASIC preserves conditional actions through calls, loops, and waits', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    const source = [
      '10 CLEAR("BLIP_BLACK"):X=0:IF 0 THEN X=10:X=20',
      '15 NEXT_FRAME=2:PRINTED=3:VALUES("É")=8',
      '16 IF NEXT_FRAME+PRINTED+VALUES("É")<>13 THEN DRAW_TEXTURE("BAD ASSIGNMENT",0,0,1,1)',
      '20 IF X<>0 THEN DRAW_TEXTURE("BAD CONDITION",0,0,1,1)',
      '25 IF NOT 1 THEN DRAW_TEXTURE("BAD NOT",0,0,1,1)',
      '26 IF NOT(0)<>1 THEN DRAW_TEXTURE("BAD NOT GROUP",0,0,1,1)',
      '30 IF 1 THEN GOSUB 100:IF X<>1 THEN DRAW_TEXTURE("BAD RETURN",0,0,1,1)',
      '40 IF 1 THEN FOR I=1 TO 3:X=X+1:NEXT I',
      '50 IF X<>4 THEN DRAW_TEXTURE("BAD LOOP",0,0,1,1)',
      '60 IF 1 THEN WAIT 50:GOSUB 100:IF X<>5 THEN DRAW_TEXTURE("BAD WAIT",0,0,1,1)',
      '70 PLAY_SFX "/TAPE/SERPENT/ASSETS/EAT.WAV"',
      '80 WAIT 16:GOTO 80',
      '100 X=X+1:RETURN',
    ];
    for (const line of source) BlipBasic.submit(line);
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-error') || document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents > 0);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
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
  assert.equal(await page.locator('.quick-bar, #maximize-screen, [data-command="GAMES"]').count(), 0);
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
