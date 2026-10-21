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
  const raider = readFileSync(new URL('SKY_RAIDER.BLIP', tape), 'utf8');
  assert.match(raider, /KILLS>=60\+15\*\(LEVEL-1\)/);
  assert.match(raider, /BOSS_IMAGE\$="\/TAPE\/GFX\/SKY_RAIDER\/BOSS_"\+STR\$\(LEVEL\)/);
  const gfx = JSON.parse(readFileSync(new URL('GFX/manifest.json', tape), 'utf8'));
  for (let level = 1; level <= 7; level++) assert.ok(gfx.SKY_RAIDER.includes(`BOSS_${level}.PNG`));
  assert.ok(gfx.RALLY.includes('BALL.PNG'));
  assert.match(rally, /FILL_RECT\(BALL_X,BALL_Y,BALL,BALL,"BLIP_WHITE"\)/);
  const defender = readFileSync(new URL('DEFENDER.BLIP', tape), 'utf8');
  assert.ok(gfx.GALACTIC_DEFENDER.includes('BULLET.PNG'));
  assert.match(defender, /DRAW_TEXTURE\("\/TAPE\/GFX\/GALACTIC_DEFENDER\/BULLET\.PNG"/);
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
  assert.equal(await page.evaluate(() => BlipBasic.lines.get(20)), 'GAME_STATE=0');
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
    BlipBasic.lines.set(810, 'MOVE_DELAY=0.5:FRAME_TIME=0:FOOD_X=17:FOOD_Y=10');
    BlipBasic.lines.set(296, 'HEAD_X=NX:HEAD_Y=NY:BODY_X(1)=HEAD_X:BODY_Y(1)=HEAD_Y');
    BlipBasic.lines.set(389, 'IF HEAD_X=9 AND HEAD_Y=9 THEN QUEUE_OK=1');
    BlipBasic.lines.set(390, 'DRAW_HUD(SCORE,LIVES):DRAW_TEXT("LEVEL "+STR$(LEVEL),160,191,1,"BLIP_CYAN","CENTER"):IF QUEUE_OK=1 THEN DRAW_TEXT("TURN QUEUE OK",160,30,2,"BLIP_YELLOW","CENTER")');
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
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=3:BODY_LENGTH=4:HEAD_X=26:HEAD_Y=7:DIR_X=1:DIR_Y=0');
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
    BlipBasic.lines.set(400, 'FOOD_X=HEAD_X+DIR_X:FOOD_Y=HEAD_Y+DIR_Y:FOOD_BLOCKED=0');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=3:LEVEL=1:FOODS=4:OB_COUNT=0:BODY_LENGTH=4:HEAD_X=19:HEAD_Y=7:DIR_X=1:DIR_Y=0');
    BlipBasic.lines.set(810, 'MOVE_DELAY=0.75:FRAME_TIME=0:FOOD_X=20:FOOD_Y=7:GOSUB 860');
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
    return Array.from(context.getImageData(0, Math.floor(image.height * 0.9), image.width, Math.ceil(image.height * 0.1)).data);
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
      BlipBasic.lines.set(10, 'GOTO 800');
      BlipBasic.lines.set(800, `GAME_STATE=1:SCORE=0:LIVES=3:LEVEL=${n}:FOODS=0:OB_COUNT=0:BODY_LENGTH=4:HEAD_X=10:HEAD_Y=10:DIR_X=1:DIR_Y=0:STEPS_SINCE_FOOD=100:STREAK=0:BONUS_TIME=0`);
      BlipBasic.lines.set(810, 'MOVE_DELAY=10:FRAME_TIME=0:FOOD_X=17:FOOD_Y=10');
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
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=3:LEVEL=6:FOODS=0:OB_COUNT=0:BODY_LENGTH=25:HEAD_X=10:HEAD_Y=10:DIR_X=1:DIR_Y=0:STEPS_SINCE_FOOD=100:STREAK=0:BONUS_TIME=0:SHEARS_TIME=0');
    BlipBasic.lines.set(810, 'MOVE_DELAY=100:FRAME_TIME=100:FOOD_X=11:FOOD_Y=10');
    BlipBasic.lines.set(980, 'SHEARS_X=12:SHEARS_Y=10:SHEARS_TIME=7:MOVE_DELAY=100:FRAME_TIME=0:RETURN');
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
    const x = Math.round(image.naturalWidth * .538), y = Math.round(image.naturalHeight * .494);
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
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=3:LEVEL=6:FOODS=0:OB_COUNT=0:BODY_LENGTH=25:HEAD_X=10:HEAD_Y=10:DIR_X=1:DIR_Y=0:STEPS_SINCE_FOOD=100:STREAK=0:BONUS_TIME=0:SHEARS_TIME=0');
    BlipBasic.lines.set(810, 'MOVE_DELAY=100:FRAME_TIME=100:FOOD_X=11:FOOD_Y=10');
    BlipBasic.lines.set(980, 'SHEARS_X=12:SHEARS_Y=10:SHEARS_TIME=7:MOVE_DELAY=0.18:FRAME_TIME=0.18:RETURN');
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
    BlipBasic.lines.set(80, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=3:LEVEL=1:FOODS=2:OB_COUNT=0:BODY_LENGTH=4:HEAD_X=10:HEAD_Y=10:DIR_X=1:DIR_Y=0:STEPS_SINCE_FOOD=100:STREAK=0:BONUS_TIME=0');
    BlipBasic.lines.set(810, 'MOVE_DELAY=0.18:FRAME_TIME=0:FOOD_X=11:FOOD_Y=10:GOSUB 860');
    BlipBasic.lines.set(910, 'BONUS_X=12:BONUS_Y=10:BONUS_TIME=6:RETURN');
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
    BlipBasic.lines.set(10, 'MODE=1:GOTO 800');
    BlipBasic.lines.set(830, 'PLAY_MUSIC "ADDER_COIL":WAIT 100:ROUND=3:SEATS=4:GOTO 650');
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
    BlipBasic.submit('810 FOR I=1 TO 48:ROCK(I)=0:NEXT I:ROCK(1)=1:AX(1)=160:AY(1)=85:AR(1)=8:AVX(1)=0:AVY(1)=0:WAVE=1:GOTO 890');
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
    BlipBasic.submit('810 FOR I=1 TO 48:ROCK(I)=0:NEXT I:ROCK(1)=1:AX(1)=160:AY(1)=85:AR(1)=12:AVX(1)=0:AVY(1)=0:WAVE=1:GOTO 890');
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
    BlipBasic.submit('1480 RETURN');
    BlipBasic.submit('810 FOR I=1 TO 48:ROCK(I)=0:NEXT I:SAUCER=0:UFO_WAIT=1:UBULLET=0:INVULN=0:GOTO 890');
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
    BlipBasic.submit('10 GOTO 800');
    BlipBasic.submit('1480 RETURN');
    BlipBasic.submit('810 FOR I=1 TO 48:ROCK(I)=0:NEXT I:SAUCER=1:UX=160:UYS=85:UY=85:UBIG=1:UWIDTH=24:UHEIGHT=8:UFIRE=80:UBULLET=0:INVULN=0:GOTO 890');
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

test('SKY_RAIDER.BLIP runs three rounds with flight, shooting, graphics, and sound', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SKY_RAIDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SKY_RAIDER.BLIP'));
  for (let round = 1; round <= 3; round++) {
    await page.evaluate(() => BlipBasic.submit('RUN'));
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: Sky Raider should launch without errors`);
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

test('SKY_RAIDER drops an ACE weapon capsule and upgrades its volley', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SKY_RAIDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SKY_RAIDER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=99:PX=143:PY=160:VX=0:SPAWN=0:SCROLL=0:FIRE_WAIT=0:SPAWN_SLOT=0:HIT_WAIT=0:LEVEL=1:KILLS=0:BOSS_ACTIVE=0:WON=0:WEAPON=1:POWER_ACTIVE=0:PLAY_MUSIC "SKY_RAIDER_ALT"');
    BlipBasic.lines.set(820, 'FOR E=1 TO 8:ENEMY(E)=0:ESHOT(E)=0:NEXT E');
    BlipBasic.lines.set(830, 'ENEMY(1)=1:ETYPE(1)=3:EX(1)=143:EY(1)=120:EDIR(1)=0:ESHOT(1)=999:GOTO 100');
    BlipBasic.lines.set(150, 'DRAW_HUD(SCORE,LIVES):IF WEAPON=2 AND SCORE>=180 THEN FILL_RECT(4,40,8,8,"#00FF00")');
    BlipBasic.lines.set(308, 'IF ENEMY(1)=1 THEN SHOT(1)=1:BX(1)=EX(1)+15:BY(1)=EY(1)+10:BT(1)=50');
    BlipBasic.lines.set(309, 'IF POWER_ACTIVE=1 THEN POWER_X=PX:POWER_Y=PY:GOSUB 1850');
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

test('SKY_RAIDER reaches its Rust boss threshold, defeats the boss, and advances the wave', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SKY_RAIDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SKY_RAIDER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=99:PX=143:PY=160:VX=0:SPAWN=0:SCROLL=0:FIRE_WAIT=0:SPAWN_SLOT=0:HIT_WAIT=0:LEVEL=1:KILLS=60:BOSS_ACTIVE=0:WON=0:PLAY_MUSIC "SKY_RAIDER_ALT"');
    BlipBasic.lines.set(820, 'FOR E=1 TO 8:ENEMY(E)=0:ESHOT(E)=0:NEXT E');
    BlipBasic.lines.set(150, 'DRAW_HUD(SCORE,LIVES):IF BOSS_ACTIVE=1 THEN FILL_RECT(4,4,8,8,"#00FF00")');
    BlipBasic.lines.set(155, 'IF LEVEL=2 THEN FILL_RECT(4,4,8,8,"#FF00FF")');
    BlipBasic.lines.set(309, 'IF BOSS_ACTIVE=1 THEN SHOT(1)=1:BX(1)=BOSS_X+BOSS_W/2:BY(1)=BOSS_Y+5:BT(1)=50');
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

test('SKY_RAIDER.BLIP fires aimed enemy rounds with the original gun sound', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SKY_RAIDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SKY_RAIDER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=3:PX=143:PY=160:SPAWN=0:SCROLL=0:FIRE_WAIT=0:SPAWN_SLOT=0:HIT_WAIT=0:ENEMY(1)=1:EX(1)=143:EY(1)=80:ETYPE(1)=1:ESHOT(1)=1:PLAY_MUSIC "SKY_RAIDER_ALT"');
    BlipBasic.lines.set(820, 'FOR E=2 TO 8:ENEMY(E)=0:ESHOT(E)=0:NEXT E');
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

test('SKY_RAIDER.BLIP spawns enemy pairs like the Rust formations', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('CD /TAPE');
    BlipBasic.submit('LOAD SKY_RAIDER');
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SKY_RAIDER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=3:PX=143:PY=160:VX=0:SPAWN=80:SCROLL=0:FIRE_WAIT=0:SPAWN_SLOT=0:HIT_WAIT=0:PLAY_MUSIC "SKY_RAIDER_ALT"');
    BlipBasic.lines.set(295, 'IF ENEMY(1)+ENEMY(2)+ENEMY(3)+ENEMY(4)+ENEMY(5)+ENEMY(6)+ENEMY(7)+ENEMY(8)>=2 THEN FILL_RECT(100,40,8,8,"BLIP_YELLOW")');
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
    const scale = Math.min(width / 320, image.height / 200);
    const left = (width - 320 * scale) / 2, top = (image.height - 200 * scale) / 2;
    for (let y = Math.floor(top + 40 * scale); y < Math.ceil(top + 48 * scale); y++) for (let x = Math.floor(left + 100 * scale); x < Math.ceil(left + 108 * scale); x++) {
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

test('SKY_RAIDER.BLIP gives the plane hit grace against a touching formation', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD SKY_RAIDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/SKY_RAIDER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=3:PX=143:PY=160:VX=0:SPAWN=0:SCROLL=0:FIRE_WAIT=0:SPAWN_SLOT=0:HIT_WAIT=0');
    BlipBasic.lines.set(820, 'FOR E=1 TO 8:ENEMY(E)=0:ESHOT(E)=999:NEXT E:ENEMY(1)=1:EX(1)=143:EY(1)=159:ETYPE(1)=1:ENEMY(2)=1:EX(2)=143:EY(2)=159:ETYPE(2)=1');
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

test('BRAWLER.BLIP supports three rounds of movement and striking', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BRAWLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BRAWLER.BLIP'));
  for (let round = 1; round <= 3; round++) {
    await page.evaluate(() => {
      BlipBasic.lines.set(330, 'PLAYER_WAIT=PLAYER_WAIT-1:CPU_WAIT=999:ATTACK=ATTACK-1:PUNCH=0:KICK=0');
      BlipBasic.submit('RUN');
    });
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: Brawler should launch cleanly`);
    const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    await page.waitForTimeout(180);
    const title = await canvas.screenshot();
    if (round === 1) await import('node:fs/promises').then(({ writeFile }) => writeFile('/tmp/brawler-title.png', title));
    let titleAnimated = false;
    for (let frame = 0; frame < 16 && !titleAnimated; frame++) {
      await page.waitForTimeout(250);
      assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: title scene runtime`);
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
    await page.keyboard.up('ArrowRight');
    await page.keyboard.up('ArrowDown');
    await page.keyboard.up('Space');
    await page.keyboard.down('f');
    await page.waitForTimeout(100);
    await page.keyboard.up('f');
    await page.keyboard.down('z');
    await page.waitForTimeout(100);
    await page.keyboard.up('z');
    await page.waitForTimeout(250);
    await page.waitForTimeout(250);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: movement, jump, guard, punch, and kick should not fail`);
    let fight; let brightPixels = 0;
    for (let frame = 0; frame < 16 && brightPixels <= 500; frame++) {
      if (frame) await page.waitForTimeout(100);
      fight = await canvas.screenshot();
      brightPixels = await page.evaluate(async (base64) => {
        const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
        const sample = document.createElement('canvas'); sample.width = image.width; sample.height = image.height;
        const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
        const pixels = context.getImageData(image.width * 0.1, image.height * 0.1, image.width * 0.8, image.height * 0.8).data;
        let count = 0;
        for (let i = 0; i < pixels.length; i += 4) if (Math.max(pixels[i], pixels[i + 1], pixels[i + 2]) > 50) count++;
        return count;
      }, fight.toString('base64'));
    }
    assert.notDeepEqual(fight, title, `round ${round}: pressing Space should enter the fight scene`);
    assert.ok(brightPixels > 500, `round ${round}: the fight scene should contain visible stage and fighter pixels (got ${brightPixels})`);
    if (round === 1) await import('node:fs/promises').then(({ writeFile }) => writeFile('/tmp/brawler-fight.png', fight));
    assert.ok((await canvas.screenshot()).length > 1000);
    await page.keyboard.up('ArrowRight');
    await page.keyboard.up('Space');
    await page.keyboard.up('ArrowDown');
    await page.locator('#exit-game').click();
    await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
  }
});

test('BRAWLER punch lands once after its Rust startup frames', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BRAWLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BRAWLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'PICK=1:GOSUB 1490:PLAYER_REACH=REACH(1):PLAYER_POWER=POWER(1):PLAYER_MAXHP=MAXHP(1):PLAYER_SPEED=SPEED(1):PLAYER_JUMP=JUMP(1):GOTO 800');
    BlipBasic.lines.set(190, 'CX=CX');
    BlipBasic.lines.set(200, 'CX=CX');
    BlipBasic.lines.set(810, 'GOSUB 1500:PLAYER_HP=PLAYER_MAXHP:CPU_HP=CPU_MAXHP:PX=120:CX=155:PY=117:CY=117:PVY=0:PLAYER_WAIT=0:CPU_WAIT=999:CPU_ATTACK=0:CPU_HIT=0:ATTACK=0:ATTACK_KIND=0:HIT_DONE=1:PUNCH_BUFFER=0:KICK_BUFFER=0:RESULT_SET=0:ROUND_TIME=45');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.focus();
  await page.waitForTimeout(150);
  const before = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  await page.keyboard.press('Space');
  await page.waitForTimeout(350);
  const after = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  assert.equal(after, before + 2, `a close punch should play its windup and exactly one hit sound (${before} -> ${after})`);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the timed punch should not cause an interpreter error');
  await page.locator('#exit-game').click();
});

test('BRAWLER tape visits the Rust ladder order for all nine matches', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.route('**/basic-runtime.html*', async (route) => {
    const response = await route.fetch();
    const body = (await response.text()).replace('imports.env.blip_basic_sound=playBasicTone;',
      'imports.env.blip_basic_sound=function(f,s,v){window.__ladderOpponent=f;playBasicTone(f,s,v);};');
    await route.fulfill({ response, body });
  });
  const expected = [4, 2, 5, 3, 6, 8, 7, 9, 10];
  const seen = [];
  for (let rung = 1; rung <= expected.length; rung++) {
    await page.goto(`${origin}/basic.html`);
    await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BRAWLER'); });
    await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BRAWLER.BLIP'));
    await page.evaluate((rung) => {
      BlipBasic.lines.set(10, `TEST_RUNG=${rung}`);
      BlipBasic.lines.set(75, 'MATCH_NO=TEST_RUNG:GOSUB 1850');
      // Report the actual opponent through the existing SOUND host call.
      BlipBasic.lines.set(146, 'SOUND CPU_PICK,0,0');
      BlipBasic.lines.set(360, 'WAIT 150:GOTO 370');
      BlipBasic.submit('RUN');
    }, rung);
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'));
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
    await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').press('Space', { delay: 100 });
    await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.__ladderOpponent !== undefined);
    seen.push(await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.__ladderOpponent));
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
    await page.locator('#exit-game').click();
    await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
  }
  assert.deepEqual(seen, expected, 'the BASIC search should match Rust ordering and keep Gamma then Zenith last');
});

test('BRAWLER roster specials land across projectile, rush, kick and laser fighters', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });

  for (const pick of [1, 2, 3, 8, 10]) {
    await page.goto(`${origin}/basic.html`);
    await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BRAWLER'); });
    await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BRAWLER.BLIP'));
    await page.evaluate((fighter) => {
      BlipBasic.lines.set(10, `PICK=${fighter}:GOSUB 1490:PLAYER_SUIT$=SUIT$(PICK):PLAYER_TRIM$=TRIM$(PICK):PLAYER_MAXHP=MAXHP(PICK):PLAYER_SPEED=SPEED(PICK):PLAYER_POWER=POWER(PICK):PLAYER_REACH=REACH(PICK):PLAYER_JUMP=JUMP(PICK):PLAYER_SPECIAL=SPEC_KIND(PICK):PLAYER_SPECIAL_SPEED=SPEC_SPEED(PICK):GOTO 800`);
      BlipBasic.lines.set(190, 'CX=CX');
      BlipBasic.lines.set(200, 'CX=CX');
      BlipBasic.lines.set(810, 'GOSUB 1500:PLAYER_HP=PLAYER_MAXHP:CPU_HP=CPU_MAXHP:PX=120:CX=165:PY=117:CY=117:PVY=0:PLAYER_WAIT=0:CPU_WAIT=999:CPU_ATTACK=0:CPU_HIT=0:ATTACK=0:ATTACK_KIND=0:HIT_DONE=1:PUNCH_BUFFER=0:KICK_BUFFER=0:RESULT_SET=0:ROUND_TIME=45');
      BlipBasic.submit('RUN');
    }, pick);
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 15000 });
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `fighter ${pick} should start`);
    const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    await canvas.focus();
    let rendered = false;
    for (let frame = 0; frame < 20 && !rendered; frame++) {
      await page.waitForTimeout(100);
      const image = await canvas.screenshot();
      rendered = await page.evaluate(async (encoded) => {
        const image = new Image(); image.src = `data:image/png;base64,${encoded}`; await image.decode();
        const sample = document.createElement('canvas'); sample.width = image.width; sample.height = image.height;
        const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
        const pixels = context.getImageData(0, 0, image.width, image.height).data;
        let count = 0;
        for (let i = 0; i < pixels.length; i += 4) if (Math.max(pixels[i], pixels[i + 1], pixels[i + 2]) > 50) count++;
        return count > 500;
      }, image.toString('base64'));
    }
    assert.ok(rendered, `fighter ${pick} should render the fight before input`);
    const beforeSound = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents);
    await page.keyboard.down('f');
    await page.waitForTimeout(35);
    await page.keyboard.down('g');
    await page.waitForTimeout(55);
    await page.keyboard.up('f');
    await page.keyboard.up('g');
    await page.waitForTimeout(800);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `fighter ${pick} special should run`);
    const afterSound = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents);
    assert.ok(afterSound >= beforeSound + 2, `fighter ${pick} special should play its startup and hit sounds (${beforeSound} -> ${afterSound})`);
    await page.locator('#exit-game').click();
    await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
  }
});

test('BRAWLER guard blocks the CPU attack without chip damage', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BRAWLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BRAWLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'PICK=1:GOSUB 1490:PLAYER_REACH=REACH(1):PLAYER_POWER=POWER(1):PLAYER_MAXHP=MAXHP(1):PLAYER_SPEED=SPEED(1):PLAYER_JUMP=JUMP(1):GOTO 800');
    BlipBasic.lines.set(190, 'CX=CX');
    BlipBasic.lines.set(200, 'CX=CX');
    BlipBasic.lines.set(810, 'GOSUB 1500:PLAYER_HP=PLAYER_MAXHP:CPU_HP=CPU_MAXHP:PX=120:CX=145:PY=117:CY=117:PVY=0:PLAYER_WAIT=0:CPU_WAIT=20:CPU_ATTACK=0:CPU_HIT=0:ATTACK=0:ATTACK_KIND=0:HIT_DONE=1:PUNCH_BUFFER=0:KICK_BUFFER=0:RESULT_SET=0:ROUND_TIME=45');
    BlipBasic.lines.set(350, BlipBasic.lines.get(350).replace('IF ', 'IF KEY_HELD("DOWN")=1 AND '));
    BlipBasic.lines.set(351, 'IF CPU_HIT=1 AND ABS(CX-PX)<27 AND PY>108 AND (KEY_HELD("DOWN")=1 OR KEY_HELD("S")=1) THEN CPU_HIT=0:CPU_WAIT=999:BLOCK_FLASH=60:PLAY_SFX "BRAWLER/BLOCK"');
    BlipBasic.lines.set(353, 'IF BLOCK_FLASH>0 AND PLAYER_HP<>PLAYER_MAXHP THEN DRAW_TEXTURE("CHIP DAMAGE",0,0,1,1)');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.focus();
  await page.waitForTimeout(150);
  const before = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  await page.keyboard.down('ArrowDown');
  await page.waitForFunction((initial) => (document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0) >= initial + 2, before, { timeout: 3000 });
  const blockFrame = await canvas.screenshot();
  await import('node:fs/promises').then(({ writeFile }) => writeFile('/tmp/brawler-block.png', blockFrame));
  await page.keyboard.up('ArrowDown');
  const after = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  assert.ok(after >= before + 2, `the CPU should wind up and be blocked (${before} -> ${after})`);
  const blockPixels = await page.evaluate(async (base64) => {
    const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const x = Math.round(image.naturalWidth * .37), y = Math.round(image.naturalHeight * .64);
    const { data } = context.getImageData(x, y, Math.round(image.naturalWidth * .09), Math.round(image.naturalHeight * .12));
    let count = 0;
    for (let i = 0; i < data.length; i += 4) if (data[i + 1] > 125 && data[i + 2] > 155 && data[i + 1] > data[i] * 1.35) count++;
    return count;
  }, blockFrame.toString('base64'));
  assert.ok(blockPixels > 4, `guard impact should flash cyan at the player's hands, got ${blockPixels} pixels`);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'blocking should not cause an interpreter error');
  await page.locator('#exit-game').click();
});

test('BRAWLER unguarded CPU hits show a red impact cue', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BRAWLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BRAWLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'PICK=1:GOSUB 1490:PLAYER_REACH=REACH(1):PLAYER_POWER=POWER(1):PLAYER_MAXHP=MAXHP(1):PLAYER_SPEED=SPEED(1):PLAYER_JUMP=JUMP(1):GOTO 800');
    BlipBasic.lines.set(190, 'CX=CX');
    BlipBasic.lines.set(200, 'CX=CX');
    BlipBasic.lines.set(351, 'IF CPU_HIT=1 AND ABS(CX-PX)<27 AND PY>108 AND (KEY_HELD("DOWN")=1 OR KEY_HELD("S")=1) THEN CPU_HIT=0:CPU_WAIT=999:BLOCK_FLASH=60:PLAY_SFX "BRAWLER/BLOCK"');
    BlipBasic.lines.set(352, 'IF CPU_HIT=1 THEN CPU_HIT=0:CPU_WAIT=999:IF ABS(CX-PX)<27 AND PY>108 THEN PLAYER_HP=PLAYER_HP-5*CPU_POWER:PLAYER_HIT_FLASH=60:PLAY_SFX "BRAWLER/HIT_LIGHT2"');
    BlipBasic.lines.set(810, 'GOSUB 1500:PLAYER_HP=PLAYER_MAXHP:CPU_HP=CPU_MAXHP:PX=120:CX=145:PY=117:CY=117:PVY=0:PLAYER_WAIT=0:CPU_WAIT=20:CPU_ATTACK=0:CPU_HIT=0:ATTACK=0:ATTACK_KIND=0:HIT_DONE=1:PUNCH_BUFFER=0:KICK_BUFFER=0:RESULT_SET=0:ROUND_TIME=45');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.focus();
  await page.waitForTimeout(150);
  const before = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  await page.waitForFunction((initial) => (document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0) >= initial + 2, before, { timeout: 3000 });
  const hitFrame = await canvas.screenshot();
  const hitPixels = await page.evaluate(async (base64) => {
    const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const x = Math.round(image.naturalWidth * .37), y = Math.round(image.naturalHeight * .64);
    const { data } = context.getImageData(x, y, Math.round(image.naturalWidth * .09), Math.round(image.naturalHeight * .12));
    let count = 0;
    for (let i = 0; i < data.length; i += 4) if (data[i] > 125 && data[i] > data[i + 1] * 1.35 && data[i] > data[i + 2] * 1.15) count++;
    return count;
  }, hitFrame.toString('base64'));
  assert.ok(hitPixels > 4, `unguarded CPU hit should flash red between the fighters, got ${hitPixels} pixels`);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the damage animation should not cause an interpreter error');
  await page.locator('#exit-game').click();
});

test('BRAWLER ends a 45-second round on time and awards the healthier fighter', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BRAWLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BRAWLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'PICK=1:GOSUB 1490:PLAYER_SUIT$="#315BCB":PLAYER_TRIM$="#2345AA":NAME$(1)="RYUKA":GOTO 800');
    BlipBasic.lines.set(810, 'GOSUB 1500:PLAYER_HP=90:CPU_HP=55:PX=70:CX=232:PY=117:CY=117:PVY=0:PLAYER_WAIT=0:CPU_WAIT=999:CPU_ATTACK=0:CPU_HIT=0:ATTACK=0:ATTACK_KIND=0:HIT_DONE=1:PUNCH_BUFFER=0:KICK_BUFFER=0:RESULT_SET=0:ROUND_TIME=45');
    BlipBasic.lines.set(740, 'IF PLAYER_ROUNDS=1 AND CPU_ROUNDS=0 THEN DRAW_TEXT("TIME SCORE OK",160,108,2,"BLIP_YELLOW","CENTER")');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await page.waitForTimeout(100);
  const hudImage = await canvas.screenshot();
  const hud = await canvas.evaluate(async (_element, encoded) => {
    const image = new Image(); image.src = `data:image/png;base64,${encoded}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.width; sample.height = image.height;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const { data, width, height } = context.getImageData(0, 0, sample.width, sample.height);
    let player = 0, rival = 0, clock = 0;
    for (let y = Math.floor(height * .14); y < Math.floor(height * .25); y++) for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 4;
      if (x < width * .42 && data[i + 2] > 100 && data[i + 1] > 80 && data[i + 2] > data[i] * 1.3) player++;
      if (x > width * .58 && data[i] > 150 && data[i] > data[i + 1] * 1.3) rival++;
      if (x > width * .45 && x < width * .55 && ((data[i] > 150 && data[i] > data[i + 1] * 1.3) || (data[i] > 160 && data[i + 1] > 160 && data[i + 2] > 160))) clock++;
    }
    return { player, rival, clock };
  }, hudImage.toString('base64'));
  assert.ok(hud.player > 20 && hud.rival > 20, `the Rust-spaced health bars should stay on their own sides (${JSON.stringify(hud)})`);
  assert.ok(hud.clock > 5, `the Rust-style central round clock should be visible (${JSON.stringify(hud)})`);
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
  await page.evaluate(() => {
    BlipBasic.lines.set(810, 'GOSUB 1500:PLAYER_HP=90:CPU_HP=55:PX=70:CX=232:PY=117:CY=117:PVY=0:PLAYER_WAIT=0:CPU_WAIT=999:CPU_ATTACK=0:CPU_HIT=0:ATTACK=0:ATTACK_KIND=0:HIT_DONE=1:PUNCH_BUFFER=0:KICK_BUFFER=0:RESULT_SET=0:ROUND_TIME=0');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  await page.waitForTimeout(350);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the time-up path should not stop with a BASIC error');
  const image = await canvas.screenshot();
  const markers = await canvas.evaluate(async (_element, encoded) => {
    const bitmap = new Image(); bitmap.src = `data:image/png;base64,${encoded}`; await bitmap.decode();
    const sample = document.createElement('canvas'); sample.width = bitmap.width; sample.height = bitmap.height;
    const context = sample.getContext('2d'); context.drawImage(bitmap, 0, 0);
    const { data, width, height } = context.getImageData(0, 0, sample.width, sample.height);
    const yellowPixels = (from, to) => {
      let count = 0;
      for (let y = Math.floor(height * from); y < Math.floor(height * to); y++) for (let x = 0; x < width; x++) {
        const i = (y * width + x) * 4;
        if (data[i] > 160 && data[i + 1] > 100 && data[i + 2] < 100) count++;
      }
      return count;
    };
    return { timeout: yellowPixels(.12, .25), winner: yellowPixels(.45, .62) };
  }, image.toString('base64'));
  assert.ok(markers.timeout > 12, `the round should display TIME UP (${markers.timeout} yellow pixels)`);
  assert.ok(markers.winner > 12, `the higher-health player should win the timed round (${markers.winner} yellow pixels)`);
  await page.locator('#exit-game').click();
});

test('BRAWLER.BLIP draws the turtle shell and mask build', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BRAWLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BRAWLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(15, 'IF PICK=0 THEN PICK=4:CPU_COLOR$="#A3343B":RIVAL$="BRUTUS":NAME$(4)="GIOTTO":SUIT$(4)="#41704A":TRIM$(4)="#2852AC":GOSUB 1490');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the turtle selection should initialize');
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.focus();
  await page.keyboard.press('Space');
  await page.waitForTimeout(500);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'shell, mask and fight drawing should run without BASIC errors');
  assert.ok((await canvas.screenshot()).length > 1000, 'the turtle fight should render');
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('BUBBLER.BLIP runs three rounds of platforming and bubble attacks', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BUBBLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  for (let round = 1; round <= 3; round++) {
    await page.evaluate(() => BlipBasic.submit('RUN'));
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: Bubbler should launch cleanly`);
    const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    await page.waitForTimeout(180);
    const title = await canvas.screenshot();
    await page.waitForTimeout(250);
    assert.notDeepEqual(await canvas.screenshot(), title, `round ${round}: the bubbles and dragons should animate on the title screen`);
    await page.keyboard.press('Space');
    await page.keyboard.down('ArrowRight');
    await page.keyboard.press('ArrowUp');
    await page.keyboard.down('Space');
    await page.waitForTimeout(250);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round}: jump and bubble controls should not fail`);
    await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents > 0, undefined, { timeout: 2500 });
    assert.ok((await canvas.screenshot()).length > 1000);
    await page.keyboard.up('ArrowRight');
    await page.keyboard.up('Space');
    await page.locator('#exit-game').click();
    await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
  }
});

test('BUBBLER advances its preserved score into a faster next wave', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('CD /TAPE');
    BlipBasic.submit('LOAD BUBBLER');
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(895, 'FOR E=1 TO ENEMY_COUNT:MONSTER(E)=0:NEXT E:GOTO 100');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=1234:LIVES=5:ROUND=1:P2SCORE=567:P2LIVES=7:TWO=1:GOTO 810');
    BlipBasic.lines.set(810, BlipBasic.lines.get(810)+':IF ROUND=2 THEN GOSUB 2090');
    BlipBasic.lines.set(2090, 'IF SCORE<>1234 OR P2SCORE<>567 OR LIVES<>6 OR P2LIVES<>7 THEN DRAW_TEXTURE("BAD ROUND REWARD",0,0,1,1)');
    BlipBasic.lines.set(2091, 'SOUND 880,0.01,0.1:RETURN');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.focus();
  await page.keyboard.press('Space');
  const firstWave = await canvas.screenshot();
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-error') || document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundTimes.length > 0, undefined, { timeout: 15000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  assert.notDeepEqual(await canvas.screenshot(), firstWave, 'wave number and score should advance when the enemies are cleared');
  await page.locator('#exit-game').click();
});

test('BUBBLER completes after its eighth Rust level', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('CD /TAPE');
    BlipBasic.submit('LOAD BUBBLER');
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=5:ROUND=8:TWO=0:GOTO 810');
    BlipBasic.lines.set(895, 'FOR E=1 TO ENEMY_COUNT:MONSTER(E)=0:NEXT E:GOTO 100');
    BlipBasic.lines.set(690, BlipBasic.lines.get(690)+':SOUND 880,0.01,0.1');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 15000 });
  const events = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  const before = await canvas.screenshot();
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-error') || document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundTimes.length > 0, undefined, { timeout: 15000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the final wave and win screen should run cleanly');
  const afterEvents = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  assert.ok(afterEvents > events, 'clearing round eight should play the completion sound');
  assert.notDeepEqual(await canvas.screenshot(), before, 'the final wave should transition to its win screen');
  await page.locator('#exit-game').click();
});

test('BUBBLER chains five trapped enemies into the Rust score and five fruit drops', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BUBBLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(5, 'CLEAR("BLIP_BLACK"):SCORE=0');
    BlipBasic.lines.set(6, 'FOR I=1 TO 5:BUBBLE(I)=2:TRAP(I)=I:MONSTER(I)=1:ETRAPPED(I)=1:QX(I)=100+I*5:QY(I)=80:NEXT I');
    BlipBasic.lines.set(7, 'B=1:SCORER=1:GOSUB 2000');
    BlipBasic.lines.set(8, 'IF SCORE<>31000 OR CHAIN_KILLS<>5 OR FRUIT(5)<>1 OR ETRAPPED(5)<>0 THEN DRAW_TEXTURE("BAD CHAIN",0,0,1,1)');
    BlipBasic.lines.set(9, 'PLAY_SFX "SERPENT/EAT":WAIT 16:GOTO 9');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-error') || document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents > 0);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
});

test('BUBBLER trapped enemies are defeated when a player pops their bubble', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BUBBLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=5:ROUND=1:TWO=0:GOTO 810');
    BlipBasic.lines.set(890, 'FOR B=1 TO 6:BUBBLE(B)=0:QOWNER(B)=1:NEXT B:BUBBLE(1)=2:TRAP(1)=1:ETRAPPED(1)=1:QX(1)=PX+8:QY(1)=PY+8:BT(1)=500');
  });
  const before = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  await page.evaluate(() => BlipBasic.submit('RUN'));
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 15000 });
  await page.waitForFunction((initial) => (document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0) > initial, before, { timeout: 3000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'popping a trapped enemy should run without a BASIC error');
  await page.locator('#exit-game').click();
});

test('BUBBLER physical controls trap a monster, pop it, and collect its candy', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('CD /TAPE');
    BlipBasic.submit('LOAD BUBBLER');
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(870, 'FOR E=1 TO ENEMY_COUNT:MONSTER(E)=0:ETRAPPED(E)=0:NEXT E:EX(1)=32:EY(1)=176:ETYPE(1)=1:MONSTER(1)=1:ETRAPPED(1)=0:EGROUND(1)=1:EDIR(1)=0:EMIN(1)=13:EMAX(1)=292:MSPEED=0');
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(817, 'STOP_MUSIC');
    BlipBasic.lines.set(2042, 'FRUIT(FRUIT_SLOT)=1:FX(FRUIT_SLOT)=QX(C)-8:FY(FRUIT_SLOT)=QY(C)-8:FRUIT_HOME(FRUIT_SLOT)=FY(FRUIT_SLOT):FVX(FRUIT_SLOT)=0:FVY(FRUIT_SLOT)=0-1.7:FRUIT_KIND(FRUIT_SLOT)=CHAIN_KILLS-1');
    BlipBasic.lines.set(1886, 'FRUIT(F)=0:PLAY_SFX "BUBBLER/FRUIT":IF SCORE<>1500 THEN DRAW_TEXTURE("BAD FRUIT SCORE",0,0,1,1)');
    BlipBasic.lines.set(1887, 'SOUND 880,0.01,0.1:RETURN');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 15000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'Bubbler should launch without a BASIC error');
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.focus();
  await page.waitForTimeout(200);
  const initialSounds = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  await page.keyboard.press('f');
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-error') || document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundTimes.length > 0, undefined, { timeout: 5000 });
  const finalSounds = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  assert.ok(finalSounds >= initialSounds + 4, `F should blow, trap, pop and pick up candy (sound events ${initialSounds} → ${finalSounds}; error ${await page.locator('body').getAttribute('data-wasm-error')})`);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'trapping, popping and picking up candy should run without a BASIC error');
  await page.locator('#exit-game').click();
});

test('BUBBLER trapped enemies escape and return to play when their bubbles expire', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BUBBLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=5:ROUND=1:TWO=0:GOTO 810');
    BlipBasic.lines.set(890, 'FOR B=1 TO 6:BUBBLE(B)=0:QOWNER(B)=1:NEXT B:BUBBLE(1)=2:TRAP(1)=1:ETRAPPED(1)=1:QX(1)=240:QY(1)=80:BT(1)=2');
  });
  const before = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  await page.evaluate(() => BlipBasic.submit('RUN'));
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 15000 });
  await page.waitForFunction((initial) => (document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0) > initial, before, { timeout: 3000 });
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'an escaped enemy should return without a BASIC error');
  await page.locator('#exit-game').click();
});

test('BUBBLER changes its palette and platform layout through round eight', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BUBBLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  const playRound = async (round) => {
    await page.evaluate((level) => {
      BlipBasic.lines.set(10, 'GOTO 800');
      BlipBasic.lines.set(800, `GAME_STATE=1:SCORE=0:LIVES=5:ROUND=${level}:TWO=0:GOTO 810`);
      BlipBasic.submit('RUN');
    }, round);
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 15000 });
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round} should initialize`);
    const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    await canvas.focus();
    await page.keyboard.press('Space');
    await page.waitForTimeout(250);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `round ${round} palette and platforms should draw`);
    const image = await canvas.screenshot();
    await page.locator('#exit-game').click();
    await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
    return image;
  };
  const rounds = [];
  for (let round = 1; round <= 8; round++) rounds.push(await playRound(round));
  for (let round = 1; round < 8; round++) {
    assert.notDeepEqual(rounds[round], rounds[0], `round ${round + 1} should use its Rust-aligned palette and platform arrangement`);
  }
});

test('BUBBLER round two mixes walkers, hoppers, and floating ghosts', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BUBBLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=5:ROUND=2:GOTO 810');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 15000 });
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  const first = await canvas.screenshot();
  await page.waitForTimeout(450);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'walker, hopper, and ghost updates should run cleanly');
  assert.notDeepEqual(await canvas.screenshot(), first, 'ghost bobbing and enemy motion should animate the round');
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('BUBBLER co-op gives player two their own movement, jump, and bubble attack', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BUBBLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 15000 });
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await page.keyboard.press('j');
  await page.waitForTimeout(120);
  await page.keyboard.down('ArrowLeft');
  await page.waitForTimeout(120);
  await page.keyboard.up('ArrowLeft');
  await page.keyboard.press('k');
  await page.keyboard.press('j');
  await page.waitForTimeout(160);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'co-op key reads and the second player update should run cleanly');
  assert.ok((await canvas.screenshot()).length > 1000, 'both players and the Bubbler arena should render');
  const sounds = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  assert.ok(sounds > 0, 'player two jump and bubble actions should use the original sound effects');
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('BUBBLER round four ghosts launch aimed sparks with the original sound', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BUBBLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=5:ROUND=4:GOTO 810');
    BlipBasic.lines.set(1135, 'FIRE_CD(E)=0');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 15000 });
  const before = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  await page.waitForTimeout(500);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'aimed sparks should run without BASIC or runtime errors');
  const after = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  assert.ok(after > before, `ghost fire should trigger the registered sparkle sound (${before} -> ${after})`);
  assert.ok((await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').screenshot()).length > 1000);
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('BUBBLER round three walkers roll rocks at players on their platform', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BUBBLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(10, 'GOTO 800');
    BlipBasic.lines.set(800, 'GAME_STATE=1:SCORE=0:LIVES=5:ROUND=3:GOTO 810');
    BlipBasic.lines.set(810, 'PX=156:PY=112:VY=0:GROUND=1:FACE=1:HIT_WAIT=0:BLOW_WAIT=0:GAMEOVER=0');
    BlipBasic.lines.set(870, 'EX(1)=45:EY(1)=112:EMIN(1)=25:EMAX(1)=96:EDIR(1)=1:ETYPE(1)=1');
    BlipBasic.lines.set(889, 'JUMP_CD(1)=999');
    BlipBasic.lines.set(1135, 'FIRE_CD(E)=0');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 15000 });
  const before = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  await page.waitForTimeout(350);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'rolling rocks should run without BASIC or runtime errors');
  const after = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents || 0);
  assert.ok(after > before, `the walker rock should play the original bounce sound (${before} -> ${after})`);
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('BUBBLER enemies jump between platform heights', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BUBBLER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(1101, 'IF ETRAPPED(E)=0 AND EGROUND(E)=1 THEN EVY(E)=0-3.2:EGROUND(E)=0');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await page.keyboard.press('Space');
  await page.waitForTimeout(250);
  const firstJump = await canvas.screenshot();
  await page.waitForTimeout(250);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'enemy jumps and platform landings should run without BASIC errors');
  assert.notDeepEqual(await canvas.screenshot(), firstJump, 'enemies should move through the air between platforms');
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('BOUNCER evaluates paddle steering and screwball math during play', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('CD /TAPE');
    BlipBasic.submit('LOAD BOUNCER');
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BOUNCER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(80, 'LV=1:SC=0:L=3:GOSUB 1000');
    BlipBasic.lines.set(90, 'GOTO 160');
    BlipBasic.lines.set(160, 'CLS:PW=60:PX=130:PPX=130:BX=132:BY=167:VX=0:VY=2:SPIN=0:CURVE=0:T=0:ROT=0:REST=0:CHAIN=0:SERVE=0:EXTRA=0:EXTRA2=0:FT=0:POPT=0:LTAP=0:RTAP=0:DASH=0:DT=0');
    BlipBasic.lines.set(309, 'VX=0:VY=2:BX=132:BY=167');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  assert.match(await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSource), /160 CLS:PW=60:PX=130:PPX=130:BX=132:BY=167/);
  assert.match(await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSource), /315 IF HITPAD=1 THEN VY=0-ABS\(SP\*COS\(REL\*1\.15\)\)/);
  await page.waitForTimeout(200);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  await page.locator('#exit-game').click();
});

test('BOUNCER seeks the nearest brick after five seconds without a break', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BOUNCER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BOUNCER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(80, 'LV=1:SC=0:L=3:GOSUB 1000');
    BlipBasic.lines.set(90, 'GOTO 160');
    BlipBasic.lines.set(160, 'CLS:PW=60:PX=130:PPX=130:BX=132:BY=195:VX=0.2:VY=-0.3:SPIN=0:CURVE=0:T=0:ROT=0:REST=0:CHAIN=0:SERVE=0:EXTRA=0:EXTRA2=0:FT=0:POPT=0:LTAP=0:RTAP=0:DASH=0:DT=0:QUIET=5.1');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 15000 });
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await page.waitForTimeout(250);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'nearest-brick steering should run cleanly');
  const shot = await canvas.screenshot();
  const outlinePixels = await page.evaluate(async (base64) => {
    const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.width; sample.height = image.height;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const { data, width, height } = context.getImageData(0, 0, sample.width, sample.height);
    let count = 0;
    const x0 = Math.floor(width * .38), x1 = Math.floor(width * .50);
    const y0 = Math.floor(height * .45), y1 = Math.floor(height * .49);
    for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
      const i = (y * width + x) * 4;
      if (data[i] > 130 && data[i + 1] > 110 && data[i + 2] < data[i + 1] * .85) count++;
    }
    return count;
  }, shot.toString('base64'));
  assert.ok(outlinePixels > 8, `the nearest live brick should get a pulsing aim outline (${outlinePixels} gold pixels)`);
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});

test('BOUNCER later levels render the Rust bomb and steel bricks', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BOUNCER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BOUNCER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(80, 'LV=4:SC=0:L=3:GOSUB 1600');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
  await page.keyboard.press('Space');
  await page.waitForTimeout(500);
  await page.keyboard.press('Space');
  await page.waitForTimeout(250);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  const shot = await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').screenshot();
  const counts = await page.evaluate(async (base64) => {
    const image = new Image();
    image.src = `data:image/png;base64,${base64}`;
    await image.decode();
    const sample = document.createElement('canvas');
    sample.width = image.width;
    sample.height = image.height;
    const context = sample.getContext('2d');
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, sample.width, sample.height).data;
    let steel = 0;
    let bombs = 0;
    for (let y = Math.floor(sample.height * .19); y < Math.floor(sample.height * .55); y++) {
      for (let x = Math.floor(sample.width * .02); x < Math.floor(sample.width * .98); x++) {
        const i = (y * sample.width + x) * 4;
        const [r, g, b] = pixels.slice(i, i + 3);
        if (r > 110 && g > 110 && b > 110 && Math.max(r, g, b) - Math.min(r, g, b) < 80) steel++;
        if (r > 160 && g > 50 && g < 180 && b < 100) bombs++;
      }
    }
    return { steel, bombs };
  }, shot.toString('base64'));
  assert.ok(counts.steel > 500, `steel-brick pixels: ${counts.steel}`);
  assert.ok(counts.bombs > 500, `bomb-brick pixels: ${counts.bombs}`);
});

test('BOUNCER bomb hits clear surrounding bricks and return from their subroutine', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BOUNCER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BOUNCER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(80, 'LV=1:SC=0:L=3:GOSUB 1600');
    BlipBasic.lines.set(90, 'GOSUB 1000:B(4,2)=2:B(3,2)=1:B(5,2)=1');
    BlipBasic.lines.set(160, 'CLS:PX=133:BX=140:BY=74:VX=0:VY=-2.7:T=0:ROT=0:REST=0:CHAIN=0:SERVE=0');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
  await page.keyboard.press('Space');
  await page.waitForTimeout(900);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  assert.equal(await page.locator('body').getAttribute('data-wasm-running'), '');
});

test('BOUNCER multi-ball pickup splits into three balls and updates all of them', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BOUNCER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BOUNCER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(80, 'LV=1:SC=0:L=3:GOSUB 1600');
    BlipBasic.lines.set(90, 'GOSUB 1000:DA(0)=1:DK(0)=4:DX(0)=145:DY(0)=168');
    BlipBasic.lines.set(160, 'CLS:PW=60:PX=130:PPX=130:BX=160:BY=169:VX=1.6:VY=0:T=0:ROT=0:REST=0:CHAIN=0:SERVE=0:EXTRA=0:EXTRA2=0:FT=0:POPT=0');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.press('Space');
  await page.waitForTimeout(250);
  await canvas.press('Space');
  await page.waitForTimeout(500);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  const visibleBallClusters = async () => {
    const shot = await canvas.screenshot();
    return page.evaluate(async (base64) => {
      const image = new Image();
      image.src = `data:image/png;base64,${base64}`;
      await image.decode();
      const sample = document.createElement('canvas');
      sample.width = image.width;
      sample.height = image.height;
      const context = sample.getContext('2d');
      context.drawImage(image, 0, 0);
      const pixels = context.getImageData(0, 0, sample.width, sample.height).data;
      const tile = 8;
      const columns = Math.ceil(sample.width / tile);
      const rows = Math.ceil(sample.height / tile);
      const mask = new Uint16Array(columns * rows);
      for (let y = Math.floor(sample.height * .05); y < Math.floor(sample.height * .95); y++) {
        for (let x = 0; x < sample.width; x++) {
          const i = (y * sample.width + x) * 4;
          if (pixels[i] > 100 && pixels[i] > pixels[i + 1] * 1.3 && pixels[i] > pixels[i + 2] * 1.2) {
            mask[Math.floor(y / tile) * columns + Math.floor(x / tile)]++;
          }
        }
      }
      const seen = new Uint8Array(mask.length);
      let count = 0;
      for (let i = 0; i < mask.length; i++) {
        if (mask[i] <= 2 || seen[i]) continue;
        const queue = [i];
        seen[i] = 1;
        let tiles = 0, minX = columns, maxX = 0, minY = rows, maxY = 0;
        while (queue.length) {
          const at = queue.pop();
          const tx = at % columns;
          const ty = Math.floor(at / columns);
          tiles++;
          minX = Math.min(minX, tx); maxX = Math.max(maxX, tx);
          minY = Math.min(minY, ty); maxY = Math.max(maxY, ty);
          for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
            const x = tx + dx;
            const y = ty + dy;
            const next = y * columns + x;
            if (x >= 0 && x < columns && y >= 0 && y < rows && !seen[next] && mask[next] > 2) {
              seen[next] = 1;
              queue.push(next);
            }
          }
        }
        if (tiles <= 12 && maxX - minX <= 6 && maxY - minY <= 6) count++;
      }
      return count;
    }, shot.toString('base64'));
  };
  let balls = 0;
  for (let attempt = 0; attempt < 6 && balls < 3; attempt++) {
    balls = await visibleBallClusters();
    if (balls < 3) await page.waitForTimeout(100);
  }
  assert.ok(balls >= 3, `visible ball clusters: ${balls}`);
});

test('BOUNCER wide, narrow, slow, and life pickups run without errors', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BOUNCER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BOUNCER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(80, 'LV=1:SC=0:L=3:GOSUB 1600');
    BlipBasic.lines.set(90, 'GOSUB 1000:DA(0)=1:DK(0)=0:DX(0)=145:DY(0)=168:DA(1)=1:DK(1)=1:DX(1)=145:DY(1)=168:DA(2)=1:DK(2)=2:DX(2)=145:DY(2)=168:DA(3)=1:DK(3)=3:DX(3)=145:DY(3)=168');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
  await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').press('Space');
  await page.waitForTimeout(900);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  assert.equal(await page.locator('body').getAttribute('data-wasm-running'), '');
});

test('BOUNCER win and game-over screens keep rendering while they wait', async (t) => {
  for (const screen of ['win', 'over']) {
    const { page, origin } = await openPage(t, 'chromium');
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto(`${origin}/basic.html`);
    await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BOUNCER'); });
    await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BOUNCER.BLIP'));
    await page.evaluate((screen) => {
      BlipBasic.lines.set(80, 'LV=1:SC=0:L=3:GOSUB 1600');
      BlipBasic.lines.set(160, screen === 'win'
        ? 'GOTO 800'
        : 'CLS:PW=60:PX=130:PPX=130:BX=160:BY=208:VX=1.6:VY=-2.7:T=0:ROT=0:REST=0:CHAIN=0:SERVE=0:EXTRA=0:FT=0:POPT=0:L=1');
      BlipBasic.submit('RUN');
    }, screen);
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
    await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').press('Space');
    await page.waitForTimeout(900);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `${screen}: no interpreter error`);
    let colored = 0;
    for (let attempt = 0; attempt < 8 && colored <= 100; attempt++) {
      if (attempt > 0) await page.waitForTimeout(100);
      const shot = await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').screenshot();
      colored = await page.evaluate(async ([base64, screen]) => {
        const image = new Image();
        image.src = `data:image/png;base64,${base64}`;
        await image.decode();
        const sample = document.createElement('canvas');
        sample.width = image.width;
        sample.height = image.height;
        const context = sample.getContext('2d');
        context.drawImage(image, 0, 0);
        const pixels = context.getImageData(0, 0, sample.width, sample.height).data;
        let count = 0;
        for (let y = Math.floor(sample.height * .2); y < Math.floor(sample.height * .7); y++) {
          for (let x = Math.floor(sample.width * .1); x < Math.floor(sample.width * .9); x++) {
            const i = (y * sample.width + x) * 4;
            const [r, g, b] = pixels.slice(i, i + 3);
            if (screen === 'win' ? g > 80 && g > r * 1.5 && g > b * 1.2 : r > 100 && r > g * 1.5 && r > b * 1.5) count++;
          }
        }
        return count;
      }, [shot.toString('base64'), screen]);
    }
    assert.ok(colored > 100, `${screen}: expected visible colored title text, found ${colored} pixels`);
  }
});

test('BOUNCER clears its last brick and advances to the next level automatically', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD BOUNCER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BOUNCER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(90, 'GOSUB 1000:FOR R=0 TO 5:FOR C=0 TO 9:B(C,R)=0:NEXT C:NEXT R:B(5,0)=1:LEFT=1');
    BlipBasic.lines.set(160, BlipBasic.lines.get(160)
      .replace('BX=160:BY=169:VX=1.6:VY=-2.7', 'BX=171:BY=49:VX=0:VY=-3')
      .replace('SERVE=1', 'SERVE=0'));
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.press('Space');
  await page.waitForTimeout(600);
  const win = await canvas.screenshot();
  await page.waitForTimeout(1300);
  async function colors(base64) {
    return page.evaluate(async (encoded) => {
      const image = new Image(); image.src = `data:image/png;base64,${encoded}`; await image.decode();
      const sample = document.createElement('canvas'); sample.width = image.width; sample.height = image.height;
      const ctx = sample.getContext('2d'); ctx.drawImage(image, 0, 0);
      const pixels = ctx.getImageData(0, 0, image.width, image.height).data;
      let green = 0, bricks = 0;
      for (let y = Math.floor(image.height * .15); y < Math.floor(image.height * .55); y += 2) {
        for (let x = Math.floor(image.width * .03); x < Math.floor(image.width * .97); x += 2) {
          const i = (y * image.width + x) * 4, r = pixels[i], g = pixels[i + 1], b = pixels[i + 2];
          if (g > 80 && g > r * 1.4 && g > b * 1.1) green++;
          if (r + g + b > 210) bricks++;
        }
      }
      return { green, bricks };
    }, base64);
  }
  const colorCounts = { win: await colors(win.toString('base64')), next: null };
  for (let attempt = 0; attempt < 20; attempt++) {
    colorCounts.next = await colors((await canvas.screenshot()).toString('base64'));
    if (colorCounts.next.bricks > colorCounts.win.bricks * 3) break;
    await page.waitForTimeout(100);
  }
  const runtime = await page.evaluate(() => ({ running: document.body.hasAttribute('data-wasm-running'), playing: document.body.hasAttribute('data-wasm-playing'), output: document.querySelector('#basic-output').textContent }));
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `BOUNCER should keep running: ${JSON.stringify({ colorCounts, runtime })}`);
  assert.ok(colorCounts.win.green > 100, `cleared screen should be visible: ${JSON.stringify(colorCounts)}`);
  assert.ok(colorCounts.next.bricks > colorCounts.win.bricks * 3, `level two should rebuild its brick wall: ${JSON.stringify({ colorCounts, runtime })}`);
});

test('DEFENDER.BLIP runs a keyboard-controlled game through Rust Blip primitives', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => BlipBasic.submit('CD /TAPE'));
  await page.evaluate(() => BlipBasic.submit('LOAD DEFENDER'));
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/DEFENDER.BLIP'));
  await page.evaluate(() => BlipBasic.submit('DIR'));
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('DEFENDER.BLIP'));
  assert.match(await page.locator('#basic-output').textContent(), /DEFENDER\.BLIP/);
  assert.ok(await page.evaluate(() => BlipBasic.lines.size > 70));
  await page.locator('#basic-input').fill('RUN');
  await page.locator('#basic-input').press('Enter');
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
  assert.ok(await page.locator('#exit-game').isVisible());
  await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').press('Space');
  await page.waitForTimeout(120);
  await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').press('ArrowLeft');
  await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').press('ArrowUp');
  await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').press('Space');
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
    await page.goto(`${origin}/basic.html`);
    await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD DEFENDER'); });
    await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/DEFENDER.BLIP'));
    await page.evaluate(() => BlipBasic.submit('RUN'));
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
    const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    await canvas.press('Space');
    await page.waitForTimeout(180);
    await canvas.focus();
    await page.keyboard.down('Space');
    await page.waitForTimeout(260);
    const playerShot = await canvas.screenshot();
    await page.keyboard.up('Space');
    if (round === 1) await import('node:fs/promises').then(({ writeFile }) => writeFile('/tmp/blip-defender-shot.png', playerShot));
    await page.waitForTimeout(600);
    const hitShot = await canvas.screenshot();
    if (round === 1) await import('node:fs/promises').then(({ writeFile }) => writeFile('/tmp/blip-defender-hit.png', hitShot));
    await page.waitForTimeout(2000);
    let alienShot, alienShotPixels = 0;
    for (let attempt = 0; attempt < 20 && alienShotPixels < 5; attempt++) {
      await page.waitForTimeout(100);
      alienShot = await canvas.screenshot();
      alienShotPixels = await page.evaluate(async (base64) => {
        const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
        const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
        const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
        const { width, height, data } = context.getImageData(0, 0, sample.width, sample.height);
        let count = 0;
        for (let y = Math.floor(height * .2); y < Math.floor(height * .9); y++) for (let x = Math.floor(width * .08); x < Math.floor(width * .92); x++) {
          const i = (y * width + x) * 4;
          if (data[i] > 140 && data[i + 1] > 55 && data[i + 1] < 180 && data[i + 2] < 110) count++;
        }
        return count;
      }, alienShot.toString('base64'));
    }
    if (round === 1) await import('node:fs/promises').then(({ writeFile }) => writeFile('/tmp/blip-defender-alien-shot.png', alienShot));
    const visual = await page.evaluate(async ([playerBase64, alienBase64, hitBase64]) => {
      async function pixelsFrom(base64) {
      const image = new Image();
      image.src = `data:image/png;base64,${base64}`;
      await image.decode();
      const sample = document.createElement('canvas');
      sample.width = image.naturalWidth; sample.height = image.naturalHeight;
      const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
      return { width: sample.width, height: sample.height, data: context.getImageData(0, 0, sample.width, sample.height).data };
      }
      const playerFrame = await pixelsFrom(playerBase64), enemyFrame = await pixelsFrom(alienBase64);
      const { width, height, data: pixels } = playerFrame;
      let occupiedCells = 0;
      const left = width * .26, top = height * .16;
      const cellWidth = width * .0469, cellHeight = height * .065;
      for (let row = 0; row < 4; row++) for (let col = 0; col < 10; col++) {
        let bright = 0;
        const x0 = Math.floor(left + col * cellWidth), y0 = Math.floor(top + row * cellHeight);
        for (let y = y0; y < y0 + cellHeight * .78; y++) {
          for (let x = x0; x < x0 + cellWidth * .78; x++) {
            const i = (y * width + x) * 4;
            if (pixels[i] + pixels[i + 1] + pixels[i + 2] > 180) bright++;
          }
        }
        if (bright > 8) occupiedCells++;
      }
      let playerShotPixels = 0, alienShotPixels = 0;
      for (let y = Math.floor(height * .2); y < Math.floor(height * .9); y++) for (let x = Math.floor(width * .35); x < Math.floor(width * .65); x++) {
        const i = (y * width + x) * 4;
        if (pixels[i] > 100 && pixels[i + 1] > 100 && pixels[i + 2] > 100) playerShotPixels++;
      }
      const enemyPixels = enemyFrame.data;
      for (let y = Math.floor(height * .2); y < Math.floor(height * .9); y++) for (let x = Math.floor(width * .08); x < Math.floor(width * .92); x++) {
        const i = (y * width + x) * 4;
        if (enemyPixels[i] > 140 && enemyPixels[i + 1] > 55 && enemyPixels[i + 1] < 180 && enemyPixels[i + 2] < 110) alienShotPixels++;
      }
      let explosionPixels = 0;
      const hitPixels = (await pixelsFrom(hitBase64)).data;
      for (let y = Math.floor(height * .15); y < Math.floor(height * .7); y++) for (let x = Math.floor(width * .08); x < Math.floor(width * .92); x++) {
        const i = (y * width + x) * 4;
        if (hitPixels[i] > 80 && hitPixels[i + 1] > 35 && hitPixels[i + 1] < 165 && hitPixels[i + 2] < 80) explosionPixels++;
      }
      return { occupiedCells, playerShotPixels, alienShotPixels, explosionPixels, width, height };
    }, [playerShot.toString('base64'), alienShot.toString('base64'), hitShot.toString('base64')]);
    const runtimeError = await page.locator('body').getAttribute('data-wasm-error');
    assert.equal(runtimeError, null, `round ${round}: BASIC runtime error: ${runtimeError}`);
    assert.ok(visual.occupiedCells >= 20, `round ${round}: expected a visible 4-by-10 alien formation, got ${JSON.stringify(visual)}`);
    assert.ok(visual.playerShotPixels >= 5, `round ${round}: player shot should be visible, got ${JSON.stringify(visual)}`);
    assert.ok(visual.explosionPixels >= 4, `round ${round}: alien hit burst should be visible, got ${JSON.stringify(visual)}`);
    assert.ok(visual.alienShotPixels >= 5, `round ${round}: alien shot should be visible, got ${JSON.stringify(visual)}`);
    await page.locator('#exit-game').click();
    await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
    } finally { await handle.close(); }
  }
});

test('DEFENDER shots remove individual shield blocks', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD DEFENDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/DEFENDER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(190, 'GOTO 200');
    BlipBasic.lines.set(311, 'BB=0:BB2=0:BB3=0:B=1:BX=56:BY=160:PX=46:START=1:REST=0:T=0:AF=0:UFO=0:UT=0:UM=500:MT=0:DROP=0:TURN=0:STREAK=0:MS=0:BOSS=0:DV=0:DIVCD=420:DTELL=0:DPT=0');
    BlipBasic.lines.set(598, 'fill_rect(140,130,18,10,COLOR(1-SH(0),SH(0),0))');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await page.waitForTimeout(180);
  const shot = await canvas.screenshot();
  const pixels = await page.evaluate(async (base64) => {
    const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const { data } = context.getImageData(0, 0, sample.width, sample.height);
    let red = 0, green = 0;
    for (let y = Math.floor(sample.height * .60); y < Math.floor(sample.height * .70); y++) {
      for (let x = Math.floor(sample.width * .43); x < Math.floor(sample.width * .54); x++) {
        const i = (y * sample.width + x) * 4;
        if (data[i] > 120 && data[i] > data[i + 1] * 1.35 && data[i] > data[i + 2] * 1.2) red++;
        if (data[i + 1] > 120 && data[i + 1] > data[i] * 1.35 && data[i + 1] > data[i + 2] * 1.2) green++;
      }
    }
    return { red, green };
  }, shot.toString('base64'));
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  assert.ok(pixels.red > 40, `one shot should remove a bunker block, pixels: ${JSON.stringify(pixels)}`);
  await page.locator('#exit-game').click();
});

test('DEFENDER bomb collisions follow the tapered player ship', async (t) => {
  async function markerFor(bombX) {
    const handle = await startPage('chromium');
    const { page, origin } = handle;
    try {
      await page.goto(`${origin}/basic.html`);
      await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD DEFENDER'); });
      await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/DEFENDER.BLIP'));
      await page.evaluate((x) => {
        BlipBasic.lines.set(190, 'GOTO 200');
        BlipBasic.lines.set(311, 'BB=1:BBX='+x+':BBY=173:BB2=0:BB3=0:BB4=0:B=0:BY=0:START=1:REST=0:PX=148:T=0:AF=0:UFO=0:UMODE=0:UFB=0:UT=0:UM=500:MT=0:DROP=0:TURN=0:STREAK=0:MS=0:BOSS=0:DV=0:DIVCD=420:DTELL=0:DPT=0');
        BlipBasic.lines.set(715, 'fill_rect(170,130,18,10,COLOR(1-(L=3),(L=3),0))');
        BlipBasic.submit('RUN');
      }, bombX);
      await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
      await page.waitForTimeout(90);
      const shot = await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').screenshot();
      return await page.evaluate(async (base64) => {
        const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
        const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
        const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
        const { data } = context.getImageData(0, 0, sample.width, sample.height);
        let red = 0, green = 0;
        for (let y = Math.floor(sample.height * .60); y < Math.floor(sample.height * .70); y++) {
          for (let x = Math.floor(sample.width * .52); x < Math.floor(sample.width * .62); x++) {
            const i = (y * sample.width + x) * 4;
            if (data[i] > data[i + 1] * 1.35 && data[i] > 100) red++;
            if (data[i + 1] > data[i] * 1.35 && data[i + 1] > 100) green++;
          }
        }
        return { red, green };
      }, shot.toString('base64'));
    } finally { await handle.close(); }
  }
  const topMiss = await markerFor(138);
  const bodyHit = await markerFor(158);
  assert.ok(topMiss.green > 40, `a bomb beside the ship's empty top corners should miss: ${JSON.stringify(topMiss)}`);
  assert.ok(bodyHit.red > 40, `a bomb through the ship body should hit: ${JSON.stringify(bodyHit)}`);
});

test('DEFENDER renders four simultaneous Rust-sized alien bombs', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD DEFENDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/DEFENDER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(190, 'GOTO 200');
    BlipBasic.lines.set(311, 'BB=1:BBX=20:BBY=100:BB2=1:BBX2=70:BBY2=110:BB3=1:BBX3=120:BBY3=120:BB4=1:BBX4=230:BBY4=130:B=0:START=1:REST=0:PX=148:T=0:AF=0:UFO=0:UT=0:UM=500:MT=0:DROP=0:TURN=0:STREAK=0:MS=0:BOSS=0:DV=0:DIVCD=420:DTELL=0:DPT=0');
    BlipBasic.lines.set(693, 'FILL_RECT(BBX,BBY,5,6,"BLIP_ORANGE"):FILL_RECT(BBX2,BBY2,5,6,"BLIP_ORANGE"):FILL_RECT(BBX3,BBY3,5,6,"BLIP_ORANGE"):FILL_RECT(BBX4,BBY4,5,6,"BLIP_ORANGE")');
    BlipBasic.lines.set(694, 'GOTO 697');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await page.waitForTimeout(100);
  const shot = await canvas.screenshot();
  const counts = await page.evaluate(async (base64) => {
    const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const { data } = context.getImageData(0, 0, sample.width, sample.height);
    return [20,70,120,230].map((x) => {
      let count = 0;
      const center = x * sample.width / 320;
      for (let py = Math.floor(sample.height * .15); py < Math.floor(sample.height * .60); py++) {
        for (let px = Math.floor(center-10); px < Math.floor(center+13); px++) {
          const i = (py * sample.width + px) * 4;
          if (data[i] > 120 && data[i+1] > 45 && data[i+1] < 180 && data[i+2] < 90) count++;
        }
      }
      return count;
    });
  }, shot.toString('base64'));
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  counts.forEach((count, i) => assert.ok(count > 3, `alien bomb ${i+1} should be visible, orange pixels: ${counts}`));
  await page.locator('#exit-game').click();
});

test('DEFENDER UFO laser clears its column and fires a beam', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD DEFENDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/DEFENDER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(190, 'GOTO 200');
    BlipBasic.lines.set(311, 'BB=0:BB2=0:BB3=0:BB4=0:B=0:BY=0:START=1:REST=0:PX=280:T=0:AF=0:UFO=1:UMODE=2:ULUSED=1:USTART=0:ULX=0:UX=120:UD=1:UTR=0:UCH=1:UFIRE=0:PSTILL=120:UT=0:UM=500:MT=0:DROP=0:TURN=0:STREAK=0:MS=0:BOSS=0:DV=0:DIVCD=420:DTELL=0:DPT=0');
    BlipBasic.lines.set(1590, 'REM hold laser for screenshot');
    BlipBasic.lines.set(680, 'fill_rect(170,130,18,10,COLOR(1-SH(12),SH(12),0)):fill_rect(200,130,18,10,COLOR(UMODE/3,1-UMODE/3,0))');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  await page.waitForTimeout(20);
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  const shot = await canvas.screenshot();
  const pixels = await page.evaluate(async (base64) => {
    const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const { data } = context.getImageData(0, 0, sample.width, sample.height);
    let beam = 0, block = 0;
    for (let y = Math.floor(sample.height * .05); y < Math.floor(sample.height * .90); y++) {
      for (let x = Math.floor(sample.width * .38); x < Math.floor(sample.width * .46); x++) {
        const i = (y * sample.width + x) * 4;
        if (data[i] > 190 && data[i+1] > 190 && data[i+2] > 190) beam++;
      }
    }
    for (let y = Math.floor(sample.height * .60); y < Math.floor(sample.height * .70); y++) {
      for (let x = Math.floor(sample.width * .52); x < Math.floor(sample.width * .62); x++) {
        const i = (y * sample.width + x) * 4;
        if (data[i] > 120 && data[i] > data[i+1] * 1.35 && data[i] > data[i+2] * 1.2) block++;
      }
    }
    return { beam, block };
  }, shot.toString('base64'));
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  assert.ok(pixels.beam > 200, `the UFO should fire the white core of its beam, got ${pixels.beam} pixels`);
  assert.ok(pixels.block > 40, `the beam should clear shield cells across its column, got ${pixels.block} red pixels`);
  await page.locator('#exit-game').click();
});

test('DEFENDER UFO drops its delayed bomb and the bomb breaks shields', async (t) => {
  async function render(kind) {
    const handle = await startPage('chromium');
    const { page, origin } = handle;
    try {
      await page.goto(`${origin}/basic.html`);
      await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD DEFENDER'); });
      await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/DEFENDER.BLIP'));
      await page.evaluate((scenario) => {
        BlipBasic.lines.set(190, 'GOTO 200');
        const flying = scenario === 'flying';
        BlipBasic.lines.set(311, 'BB=0:BB2=0:BB3=0:BB4=0:B=0:BY=0:START=1:REST=0:PX=280:T=0:AF=0:UFO='+Number(flying)+':UMODE=0:ULUSED=1:USTART=0:ULX=0:UX=105:UD=1:UTR=0:UCH=0:UFIRE=0:PSTILL=0:UFB='+Number(!flying)+':UFBX='+(flying?0:115)+':UFBY='+(flying?0:157)+':UFT='+(flying?180:0)+':UFDROP=0:UT=0:UM=500:MT=0:DROP=0:TURN=0:STREAK=0:MS=0:BOSS=0:DV=0:DIVCD=420:DTELL=0:DPT=0');
        BlipBasic.lines.set(flying ? 680 : 699, flying
          ? 'fill_rect(170,130,18,10,COLOR(UFB,1-UFB,0))'
          : 'fill_rect(170,130,18,10,COLOR(1-SH(12),SH(12),0))');
        BlipBasic.submit('RUN');
      }, kind);
      await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
      await page.waitForTimeout(30);
      assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
      const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
      const shot = await canvas.screenshot();
      return await page.evaluate(async ([base64, scenario]) => {
        const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
        const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
        const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
        const { data } = context.getImageData(0, 0, sample.width, sample.height);
        let orange = 0, red = 0;
        for (let y = Math.floor(sample.height * .05); y < Math.floor(sample.height * .18); y++) {
          for (let x = Math.floor(sample.width * .32); x < Math.floor(sample.width * .42); x++) {
            const i = (y * sample.width + x) * 4;
            if (data[i] > 120 && data[i+1] > 45 && data[i+1] < 180 && data[i+2] < 90) orange++;
          }
        }
        for (let y = Math.floor(sample.height * .60); y < Math.floor(sample.height * .70); y++) {
          for (let x = Math.floor(sample.width * .52); x < Math.floor(sample.width * .62); x++) {
            const i = (y * sample.width + x) * 4;
            if (data[i] > 120 && data[i] > data[i+1] * 1.35 && data[i] > data[i+2] * 1.2) red++;
          }
        }
        return scenario === 'flying' ? { orange } : { red };
      }, [shot.toString('base64'), kind]);
    } finally { await handle.close(); }
  }
  const flying = await render('flying');
  const shield = await render('shield');
  assert.ok(flying.orange > 6, `the UFO should drop its orange bomb after three seconds, got ${flying.orange} pixels`);
  assert.ok(shield.red > 40, `the UFO bomb should remove a shield cell, got ${shield.red} red pixels`);
});

test('DEFENDER higher waves initialize each formation theme', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD DEFENDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/DEFENDER.BLIP'));
  const countFormationPixels = async (canvas) => {
    const screenshot = await canvas.screenshot();
    return page.evaluate(async (base64) => {
      const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
      const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
      const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
      const { data } = context.getImageData(0, 0, sample.width, sample.height);
      let count = 0;
      for (let y = Math.floor(sample.height * .16); y < Math.floor(sample.height * .55); y++) {
        for (let x = Math.floor(sample.width * .06); x < Math.floor(sample.width * .94); x++) {
          const i = (y * sample.width + x) * 4;
          if (data[i] + data[i + 1] + data[i + 2] > 180) count++;
        }
      }
      return count;
    }, screenshot.toString('base64'));
  };
  for (const level of [2, 3, 4, 5]) {
    await page.evaluate((n) => BlipBasic.lines.set(20, `LV=${n}:SC=0:L=3`), level);
    await page.evaluate(() => BlipBasic.submit('RUN'));
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
    const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    await canvas.press('Space');
    let coloredPixels = 0;
    for (let attempt = 0; attempt < 6 && coloredPixels <= 800; attempt++) {
      await page.waitForTimeout(attempt === 0 ? 500 : 250);
      coloredPixels = await countFormationPixels(canvas);
    }
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `level ${level} should run without an interpreter error`);
    assert.ok(coloredPixels > 800, `level ${level} should render its formation, got ${coloredPixels} lit pixels`);
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
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD DEFENDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/DEFENDER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(20, 'LV=5:SC=0:L=3');
    BlipBasic.lines.set(211, 'MAXI=ROWS*COLS-1:LEFT=0');
    BlipBasic.lines.set(759, 'IF BB=1 AND BB2=1 AND BB3=1 AND BBY>85 AND BBY<150 THEN SOUND 1777,0,0:WAIT 0:GOTO 3000');
    BlipBasic.lines.set(3000, 'WAIT 0:GOTO 3000');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.press('Space');
  await page.waitForFunction(() => document.querySelector('#basic-wasm-frame').contentWindow.__mothershipVolley, undefined, { timeout: 10000 });
  const shot = await canvas.screenshot();
  const bossPixels = await page.evaluate(async (base64) => {
    const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const { data } = context.getImageData(0, 0, sample.width, sample.height);
    let hull = 0, bombs = 0;
    for (let y = Math.floor(sample.height * .12); y < Math.floor(sample.height * .34); y++) {
      for (let x = Math.floor(sample.width * .05); x < Math.floor(sample.width * .55); x++) {
        const i = (y * sample.width + x) * 4;
        if (data[i] > 130 && data[i] > data[i + 1] * 1.6 && data[i] > data[i + 2] * 1.4) hull++;
      }
    }
    for (let y = Math.floor(sample.height * .32); y < Math.floor(sample.height * .88); y++) {
      for (let x = Math.floor(sample.width * .04); x < Math.floor(sample.width * .96); x++) {
        const i = (y * sample.width + x) * 4;
        if (data[i] > 150 && data[i + 1] > 55 && data[i + 1] < 190 && data[i + 2] < 110) bombs++;
      }
    }
    return { hull, bombs };
  }, shot.toString('base64'));
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the boss encounter should not produce an interpreter error');
  assert.ok(bossPixels.hull > 100, `the mothership should render its red hull and health bar, got ${bossPixels.hull} red pixels`);
  assert.ok(bossPixels.bombs > 8, `the mothership should fire a three-shot volley, got ${bossPixels.bombs} orange pixels`);
  await page.locator('#exit-game').click();
});

test('DEFENDER telegraphs and resolves its wave-three diver', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD DEFENDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/DEFENDER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(20, 'LV=3:SC=0:L=3');
    BlipBasic.lines.set(311, 'BB=0:BB2=0:BB3=0:B=0:BY=0:START=1:REST=0:T=0:AF=0:UFO=0:UT=0:UM=500:MT=0:DROP=0:TURN=0:STREAK=0:MS=0:BOSS=0:DV=1:DIDX=21:DVT=1:DXD=AX(21):DYD=AY(21):DIVCD=420:DTELL=18:DPT=0');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.press('Space');
  await page.waitForTimeout(90);
  const tell = await canvas.screenshot();
  await page.waitForTimeout(450);
  const pixels = await page.evaluate(async (tell64) => {
    const image = new Image(); image.src = `data:image/png;base64,${tell64}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const { width, height, data } = context.getImageData(0, 0, sample.width, sample.height);
    let warningRed = 0;
    for (let y = Math.floor(height * .16); y < Math.floor(height * .34); y++) {
      for (let x = Math.floor(width * .74); x < Math.floor(width * .99); x++) {
        const i = (y * width + x) * 4, [r,g,b] = data.slice(i, i + 3);
        if (r > 100 && r > g * 1.45 && r > b * 1.35) warningRed++;
      }
    }
    return { warningRed };
  }, tell.toString('base64'));
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the diver state should run without an interpreter error');
  assert.ok(pixels.warningRed > 10, `the selected alien should flash red before diving: ${pixels.warningRed} pixels`);
  await page.locator('#exit-game').click();
});

test('DEFENDER shots award the double value for a diver kill', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD DEFENDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/DEFENDER.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(20, 'LV=3:SC=0:L=3');
    BlipBasic.lines.set(311, 'BB=0:BB2=0:BB3=0:B=1:BX=145:BY=125:START=1:REST=0:T=0:AF=0:UFO=0:UT=0:UM=500:MT=0:DROP=0:TURN=0:STREAK=0:MS=0:BOSS=0:DV=2:DVT=0:DXD=140:DYD=120:DIVCD=420:DTELL=0:DPT=0');
    BlipBasic.lines.set(607, 'IF HITD=1 THEN DPT=600:IF SC<>180 OR DPTS<>180 THEN DRAW_TEXTURE("BAD DIVER SCORE",0,0,1,1)');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.press('Space');
  await page.waitForTimeout(160);
  const shot = await canvas.screenshot();
  const yellow = await page.evaluate(async (base64) => {
    const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
    const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
    const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
    const { data } = context.getImageData(0, 0, sample.width, sample.height);
    let count = 0;
    for (let y = Math.floor(sample.height * .45); y < Math.floor(sample.height * .75); y++) {
      for (let x = Math.floor(sample.width * .25); x < Math.floor(sample.width * .65); x++) {
        const i = (y * sample.width + x) * 4;
        if (data[i] > 130 && data[i + 1] > 100 && data[i + 2] < 110) count++;
      }
    }
    return count;
  }, shot.toString('base64'));
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'a diver hit should not produce an interpreter error');
  assert.ok(yellow > 10, `the diver kill should show its bonus score, got ${yellow} yellow pixels`);
  await page.locator('#exit-game').click();
});

test('DEFENDER divers smash through shield blocks', async (t) => {
  async function shieldPixels(diving) {
    const handle = await startPage('chromium');
    const { page, origin } = handle;
    try {
      await page.goto(`${origin}/basic.html`);
      await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD DEFENDER'); });
      await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/DEFENDER.BLIP'));
      await page.evaluate((active) => {
        BlipBasic.lines.set(20, 'LV=4:SC=0:L=3');
        BlipBasic.lines.set(598, 'B=0:fill_rect(140,130,18,10,COLOR(1-SH(14),SH(14),0))');
        const diver = active ? ':DV=2:DVT=0:DXD=125:DYD=158' : ':DV=0:DXD=125:DYD=158';
        BlipBasic.lines.set(311, 'BB=0:BB2=0:BB3=0:B=0:BY=0:START=1:REST=0:PX=280:T=0:AF=0:UFO=0:UT=0:UM=500:MT=0:DROP=0:TURN=0:STREAK=0:MS=0:BOSS=0:DIVCD=420:DTELL=0:DPT=0'+diver);
        BlipBasic.submit('RUN');
      }, diving);
      await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
      assert.match(await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSource), /598 B=0:fill_rect/);
      const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
      await canvas.press('Space');
      await page.waitForTimeout(180);
      assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the shield dive should not produce an interpreter error');
      const shot = await canvas.screenshot();
      return await page.evaluate(async (base64) => {
        const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
        const sample = document.createElement('canvas'); sample.width = image.naturalWidth; sample.height = image.naturalHeight;
        const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
        const { data } = context.getImageData(0, 0, sample.width, sample.height);
        let red = 0, green = 0;
        for (let y = Math.floor(sample.height * .60); y < Math.floor(sample.height * .70); y++) {
          for (let x = Math.floor(sample.width * .43); x < Math.floor(sample.width * .54); x++) {
            const i = (y * sample.width + x) * 4;
            if (data[i] > 120 && data[i] > data[i + 1] * 1.35 && data[i] > data[i + 2] * 1.2) red++;
            if (data[i + 1] > 120 && data[i + 1] > data[i] * 1.35 && data[i + 1] > data[i + 2] * 1.2) green++;
          }
        }
        return { red, green };
      }, shot.toString('base64'));
    } finally { await handle.close(); }
  }
  const intact = await shieldPixels(false);
  const broken = await shieldPixels(true);
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

test('DIR and CD expose the inserted tape\'s game art under TAPE/GFX', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => BlipBasic.submit('LOAD /TAPE/BOUNCER'));
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BOUNCER.BLIP'));
  await page.evaluate(() => { BlipBasic.submit('DIR /TAPE/GFX'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('BOUNCER/'));
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE/GFX/BOUNCER'); BlipBasic.submit('DIR'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('PADDLE.PNG'));
  const output = await page.locator('#basic-output').textContent();
  assert.match(output, /\/TAPE\/GFX[\s\S]*\/TAPE\/GFX\/BOUNCER[\s\S]*BALL\.PNG[\s\S]*BRICK_RED\.PNG/);
  await page.evaluate(() => {
    BlipBasic.submit('10 CLEAR("BLIP_BLACK")');
    BlipBasic.submit('20 DRAW_TEXTURE("/TAPE/GFX/BOUNCER/BALL.PNG",40,40,16,16)');
    BlipBasic.submit('30 WAIT 16');
    BlipBasic.submit('40 GOTO 10');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  assert.match(await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSource), /\/TAPE\/GFX\/BOUNCER\/BALL\.PNG/);
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
  assert.equal(await page.locator('.contents-list li').count(), 14);
  assert.equal(await page.locator('.reader-page:visible').count(), 0);
  await page.locator('[data-turn="next"]').click();
  assert.equal(await page.locator('.reader-page:visible').count(), 1);
  assert.match(await page.locator('.reader-page:visible').innerText(), /YOUR FIRST PROGRAM/);
  for (let turn = 0; turn < 8 && !(await page.locator('.reader-page:visible').innerText()).includes('DRAW WITH BLIP'); turn += 1) await page.locator('[data-turn="next"]').click();
  assert.equal(await page.locator('.reader-page:visible').count(), 1);
  assert.equal(await page.locator('.reader-page:visible .primitive-list > div').count(), 15);
  for (let turn = 0; turn < 8 && !(await page.locator('.reader-page:visible').innerText()).includes('OPEN A GAME'); turn += 1) await page.locator('[data-turn="next"]').click();
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
  assert.match(await page.locator('.reader-spread').innerText(), /NAME YOUR ROUTINES\.[\s\S]*READ INPUT\. UPDATE\. DRAW\./);
  const fit = await page.locator('.reader-spread > .reader-page').evaluateAll((pages) => pages.map((sheet) => ({ overflow: getComputedStyle(sheet).overflowY, fits: sheet.scrollHeight <= sheet.clientHeight + 1, scroll: sheet.scrollHeight, height: sheet.clientHeight, chapter: sheet.firstElementChild && sheet.firstElementChild.getBoundingClientRect().height })));
  assert.ok(fit.every((sheet) => sheet.overflow === 'hidden' && sheet.fits), JSON.stringify(fit));
  for (let spread = 0; spread < 5; spread += 1) {
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
  assert.equal(await page.evaluate(() => Array.from(BlipBasic.lines.values()).some((line) => line.includes('GALACTIC'))), true);
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
      '70 PLAY_SFX "SERPENT/EAT"',
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
