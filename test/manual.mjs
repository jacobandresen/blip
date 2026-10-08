// The manual: its book on the fascia, the chapters it opens to, and
// the printed leaves.
//
//   node --test test/manual.mjs        (after ./build_web.sh)

import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

for (const width of [1280, 390]) {
  test(`the book flies out of its pocket and back at ${width}px`, async (t) => {
    const { page, origin } = await openPage(t, 'chromium', { viewport: { width, height: 900 } });
    await page.goto(`${origin}/index.html`);
    const book = page.locator('#manual-book');
    const pocket = await book.boundingBox();
    assert.ok(pocket.x > width / 2, 'the book is stored on the right of the fascia');

    const flight = (keyframe) => page.locator('.manual-transit').evaluate((el, which) => {
      const frames = el.getAnimations()[0].effect.getKeyframes();
      return frames.at(which);
    }, keyframe);
    const atPocket = (frame) =>
      Math.abs(parseFloat(frame.left) - pocket.x) < 1 && Math.abs(parseFloat(frame.top) - pocket.y) < 1;

    await book.click();
    await page.waitForSelector('.manual-transit');
    assert.ok(atPocket(await flight(0)), 'it leaves from the pocket');
    await page.waitForFunction(() => !document.getElementById('manual-overlay').inert);
    assert.equal(await book.evaluate((el) => getComputedStyle(el).visibility), 'hidden', 'the pocket is empty while it is open');

    await page.getByRole('button', { name: 'Close manual', exact: true }).click();
    await page.waitForSelector('.manual-transit');
    assert.ok(atPocket(await flight(-1)), 'it returns to the pocket');
    await page.waitForFunction(() => document.getElementById('manual-overlay').hidden);
    assert.equal(await book.evaluate((el) => getComputedStyle(el).visibility), 'visible');
    assert.equal(await book.evaluate((el) => document.activeElement === el), true, 'focus returns to the book');
    assert.equal(await page.locator('.manual-transit').count(), 0);
  });
}

test('with reduced motion the book opens and closes without a flight', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 900 } });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/index.html`);
  await page.locator('#manual-book').click();
  assert.equal(await page.locator('.manual-transit').count(), 0);
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => document.getElementById('manual-overlay').hidden);
  assert.equal(await page.locator('#manual-book').evaluate((el) => getComputedStyle(el).visibility), 'visible');
});

for (const [slug, title] of [['about', 'about blip'], ['controls', 'controls'], ['history', 'history']]) {
  test(`/${slug}.html opens the manual at its ${slug} chapter`, async (t) => {
    const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 844 } });
    await page.goto(`${origin}/${slug}.html`);
    await page.waitForFunction(() => !document.getElementById('manual-overlay').hidden);
    assert.equal((await page.locator('#manual-book-title').textContent()).toLowerCase(), title);
  });
}

const SLUGS = ['adder', 'bouncer', 'brawler', 'bubbler', 'galactic_defender', 'meteors', 'sky_raider', 'rally', 'serpent'];

async function openManual(t, query, { width = 1280, height = 844 } = {}) {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: width < 700, viewport: { width, height } });
  await page.goto(`${origin}/index.html?manual=${query}`);
  await page.waitForFunction(() => document.querySelector('.manual-page-content .manual-leaf-sheet')
    && document.getElementById('manual-page-count').textContent.split('/')[1].trim() !== '01'
    && !document.getElementById('manual-overlay').inert);
  return page;
}

for (const [width, height] of [[1280, 844], [390, 844], [320, 720]]) {
  test(`every printed leaf fits its paper at ${width}px`, async (t) => {
    const page = await openManual(t, 'about', { width, height });
    const wide = width > 700;
    const book = await page.evaluate(() => {
      const open = document.querySelector('.manual-open-book').getBoundingClientRect();
      return {
        centered: Math.abs(open.left + open.width / 2 - innerWidth / 2) < 2,
        inFront: Number(getComputedStyle(document.getElementById('manual-overlay')).zIndex) >= 10000,
        leaves: Number(document.getElementById('manual-page-count').textContent.split('/')[1]),
      };
    });
    assert.deepEqual({ centered: book.centered, inFront: book.inFront }, { centered: true, inFront: true });
    assert.ok(book.leaves > 3, `the manual is split into leaves, got ${book.leaves}`);

    while (await page.locator('#manual-prev').isEnabled()) await page.locator('#manual-prev').click();

    let portraits = 0, shown = 0;
    for (;;) {
      await page.waitForTimeout(wide ? 120 : 60);
      const spread = await page.evaluate(() => [...document.querySelectorAll('.manual-open-page')].flatMap((pageEl) => {
        const sheet = pageEl.querySelector('.manual-leaf-sheet:not(.manual-endpaper)');
        if (!sheet || pageEl.closest('.manual-flip, .manual-flip-cover') || getComputedStyle(pageEl).display === 'none') return [];
        const content = sheet.parentElement, box = sheet.getBoundingClientRect(), range = document.createRange();
        range.selectNodeContents(sheet);
        const text = [...range.getClientRects()].filter((r) => r.width && r.height);
        return [{
          scrolls: sheet.scrollHeight > sheet.clientHeight + 1 || content.scrollHeight > content.clientHeight + 1,
          textInside: text.every((r) => r.left >= box.left - 1 && r.right <= box.right + 1 && r.bottom <= box.bottom + 1),
          portraits: [...content.querySelectorAll('img.operator-photo')].map((img) =>
            `${getComputedStyle(img).filter.includes('grayscale(1)')}`),
        }];
      }));
      for (const leaf of spread) {
        assert.equal(leaf.scrolls, false, 'a leaf scrolls');
        assert.equal(leaf.textInside, true, 'a leaf\'s text leaves the page');
        for (const portrait of leaf.portraits) assert.equal(portrait, 'true', 'operator photos are black and white');
        portraits += leaf.portraits.length;
        shown++;
      }
      if (!(await page.locator('#manual-next').isEnabled())) break;
      await page.locator('#manual-next').click();
    }
    const expected = wide ? book.leaves - 1 : await page.evaluate(() => manualPages.filter((leaf) => !leaf.filler).length);
    assert.ok(shown >= expected, `every leaf was seen: ${shown} of ${expected}`);
    assert.ok(portraits > 0, 'the operators appear in the manual');
  });
}

test('the contents are inside the front cover only; turning a page leaves content on both sides', async (t) => {
  const page = await openManual(t, 'about');
  const left = () => page.evaluate(() => ({
    contents: getComputedStyle(document.querySelector('.manual-left-contents')).display !== 'none',
    leaf: document.querySelector('#manual-left-content .manual-leaf-sheet') !== null,
    folio: document.getElementById('manual-left-folio').textContent,
  }));
  assert.equal((await left()).contents, false, 'a chapter spread has no contents');
  assert.equal((await left()).leaf, true, 'its left page is a leaf');

  await page.locator('#manual-contents-tab').click();
  await page.waitForFunction(() => document.getElementById('manual-page-count').textContent.startsWith('01'));
  assert.deepEqual((await left()).contents, true, 'the Contents tab goes back to the front cover');

  await page.locator('#manual-next').click();
  await page.waitForFunction(() => document.querySelectorAll('.manual-flip').length === 0);
  const turned = await left();
  assert.equal(turned.contents, false, 'turning the page takes the contents away');
  assert.equal(turned.leaf && Number(turned.folio) > 0, true, 'a numbered leaf takes its place');
});

for (const slug of SLUGS) {
  test(`${slug} gets a spread: its screen in black and white and a blurb on the left, its controls on the right`, async (t) => {
    const page = await openManual(t, 'controls');
    await page.evaluate((game) => showManualPage(manualGameStarts[game], 0), slug);
    await page.waitForFunction((game) => document.querySelector(`#manual-left-content [data-manual-game="${game}"]`), slug);
    const spread = await page.evaluate(() => {
      const figure = document.querySelector('#manual-left-content .manual-figure img');
      const blurb = document.querySelector('#manual-left-content .manual-game-blurb');
      return {
        screenshot: figure && figure.naturalWidth > 0,
        grey: figure && getComputedStyle(figure).filter.includes('grayscale(1)'),
        blurb: blurb ? blurb.textContent.length : 0,
        caption: document.querySelector('#manual-left-content figcaption')?.textContent.startsWith('Fig. '),
        controls: document.querySelectorAll('#manual-page-content [data-side="right"] .keytable tr').length,
        sides: [document.querySelector('#manual-left-content [data-side]')?.dataset.side,
          document.querySelector('#manual-page-content [data-side]')?.dataset.side],
      };
    });
    assert.deepEqual({ ...spread, blurb: spread.blurb > 60, controls: spread.controls > 0 },
      { screenshot: true, grey: true, blurb: true, caption: true, controls: true, sides: ['left', 'right'] });
  });
}

test('turning a page lifts a leaf that comes to rest, and the book is whole afterwards', async (t) => {
  const page = await openManual(t, 'about');
  await page.locator('#manual-prev').click();
  await page.waitForSelector('.manual-flip');
  await page.waitForFunction(() => document.querySelectorAll('.manual-flip, .manual-flip-cover').length === 0);
  assert.equal(await page.locator('#manual-page-content').count(), 1, 'one right-hand page');
  assert.equal(await page.locator('.manual-open-page').count(), 2, 'two pages, no leftovers of the turn');
});

test('a swipe across the open book turns its pages, left for next and right for previous', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: true, viewport: { width: 390, height: 844 } });
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 2, mobile: true });
  await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: true });
  await page.goto(`${origin}/index.html?manual=about`);
  await page.waitForFunction(() => document.querySelector('.manual-page-content .manual-leaf-sheet')
    && document.getElementById('manual-page-count').textContent.split('/')[1].trim() !== '01'
    && !document.getElementById('manual-overlay').inert);
  const leaf = async () => Number((await page.locator('#manual-page-count').textContent()).split('/')[0]);
  const box = await page.locator('.manual-open-book').boundingBox();
  const y = box.y + box.height / 2;
  const swipe = async (from, to) => {
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ id: 1, x: box.x + box.width * from, y }] });
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ id: 1, x: box.x + box.width * to, y }] });
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [{ id: 1, x: box.x + box.width * to, y }] });
    await page.waitForTimeout(400);
  };
  const first = await leaf();
  await swipe(0.8, 0.2);
  assert.equal(await leaf(), first + 1, 'a left swipe turns to the next page');
  await swipe(0.2, 0.8);
  assert.equal(await leaf(), first, 'a right swipe turns back');
});

test('the second notes page carries a handwritten note, taped near the foot and signed Jacob Andresen', async (t) => {
  const page = await openManual(t, 'controls');
  await page.evaluate(() => {
    const second = manualPages.map((leaf, index) => (leaf.filler ? index : -1)).filter((index) => index >= 0)[1];
    showManualPage(second, 0);
  });
  await page.waitForSelector('.manual-open-page .manual-note-card');
  await page.evaluate(() => document.fonts.ready);
  const note = await page.evaluate(() => {
    const card = document.querySelector('.manual-note-card').getBoundingClientRect();
    const sheet = document.querySelector('.manual-note-card').closest('.manual-open-page').getBoundingClientRect();
    return {
      body: document.querySelector('.manual-note-body').textContent,
      regards: document.querySelector('.manual-note-regards').textContent,
      signature: document.querySelector('.manual-signature').textContent,
      lines: Math.round(document.querySelector('.manual-note-body').getBoundingClientRect().height / 34),
      nearFoot: sheet.bottom - card.bottom < 110 && card.top > sheet.top + sheet.height * 0.4,
      insidePage: card.left >= sheet.left && card.right <= sheet.right,
      fonts: [document.fonts.check("25px Caveat"), document.fonts.check("58px 'Mrs Saint Delafield'", 'Jacob Andresen')],
      notes: document.querySelectorAll('.manual-note-card').length,
    };
  });
  assert.deepEqual(note, {
    body: 'Blip Arcade is a tribute to the golden age of arcade games. I hope you enjoy this as much as I enjoyed playing the old games.',
    regards: 'Kind regards,',
    signature: 'Jacob Andresen',
    lines: 3,
    nearFoot: true,
    insidePage: true,
    fonts: [true, true],
    notes: 1,
  });
});

test('chapter 2 is called The golden age of arcade games', async (t) => {
  const page = await openManual(t, 'history');
  const names = await page.evaluate(() => ({
    contents: document.querySelector('.manual-back-contents [data-chapter="history"] span').textContent.replace(/\s+/g, ' '),
    heading: document.querySelector('.manual-subchapter-title')?.textContent,
  }));
  assert.deepEqual(names, { contents: '2 The golden age of arcade games', heading: 'THE GOLDEN AGE OF ARCADE GAMES' });
});

for (const [width, height] of [[1920, 1080], [1440, 900], [1366, 768], [1280, 844], [1280, 720], [1180, 820], [1024, 600], [768, 1024], [390, 844], [360, 640], [320, 568]]) {
  test(`the operators fill exactly one page at ${width}x${height}: six cards, the last for you, and a pitch`, async (t) => {
    const page = await openManual(t, 'about', { width, height });
    const index = await page.evaluate(() => {
      const found = manualPages.map((leaf, i) => (leaf.operators ? i : -1)).filter((i) => i >= 0);
      showManualPage(found[0], 0);
      return { found, last: manualPages.length - 1 };
    });
    assert.deepEqual(index.found, [index.last], 'the operators are the last page, and the only one');
    await page.waitForSelector('.manual-operators');
    const roster = await page.evaluate(() => {
      const section = document.querySelector('.manual-operators');
      const sheet = section.closest('.manual-leaf-sheet');
      return {
        title: section.querySelector('.manual-operators-title').textContent,
        introShown: getComputedStyle(section.querySelector('.manual-operators-intro')).display !== 'none',
        cards: [...section.querySelectorAll('td.operator .operator-name')].map((name) => name.textContent),
        pitch: section.querySelector('.operators-pitch').textContent,
        scrolls: sheet.scrollHeight > sheet.clientHeight + 1,
        photos: [...section.querySelectorAll('.operator-photo')].every((photo) => {
          const r = photo.getBoundingClientRect(), card = photo.closest('.operator').getBoundingClientRect();
          return r.width > 8 && r.width <= card.width / 2 && r.bottom <= card.bottom;
        }),
        textFits: [...section.querySelectorAll('.operator, .operators-pitch')].every((box) => {
          const r = box.getBoundingClientRect(), page = sheet.getBoundingClientRect();
          return r.left >= page.left && r.right <= page.right && r.bottom <= page.bottom;
        }),
      };
    });
    assert.equal(roster.title, 'The operators');
    if (height >= 844) assert.equal(roster.introShown, true, 'an introduction sits under the title');
    assert.deepEqual(roster.cards, ['Jacob Andresen', 'Björn Harrtell', 'Claude', 'Copilot', 'Codex', 'You?']);
    assert.match(roster.pitch, /HELP WANTED/);
    assert.match(roster.pitch, /github\.com\/jacobandresen\/blip/);
    assert.deepEqual({ scrolls: roster.scrolls, textFits: roster.textFits, photos: roster.photos }, { scrolls: false, textFits: true, photos: true },
      'every card, photo and the pitch sit inside the one page');
  });
}

test('the contents list The operators at its page', async (t) => {
  const page = await openManual(t, 'about');
  const entry = await page.evaluate(() => {
    const button = document.querySelector('[data-chapter="operators"]');
    return { enabled: !button.disabled, page: Number(button.dataset.page), last: manualPages.length - 1 };
  });
  assert.deepEqual(entry, { enabled: true, page: entry.last, last: entry.last });
});
