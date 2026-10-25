import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';
import { instrumentBasicSound } from './lib/basic-runtime.mjs';

async function fight(t, { setup = '', cpu = false, menu = false, initialPick, knockAt = 0, sample = 'X(1)/100', volume = 'HEALTH(2)/MAX(1,MAX_HP(WHO(2)))', probe = '1000+MV(1)+16*(ACT$(1)="ATTACK")+32*(FACING(1)<0)+64*(Y(1)<FLOOR_Y)', view } = {}) {
  const handle = await openPage(t, 'chromium', { viewport: view || { width: 1000, height: 900 }, hasTouch: !!view });
  const { page, origin } = handle;
  await instrumentBasicSound(page, `function(f,s,v){
        window.__brawler={bits:Math.round(f-1000),x:s*100,hp:v,samples:(window.__brawler?.samples||0)+1};
        window.__brawler.probe=f;window.__brawler.sample=s;
        window.__brawlerSeen=(window.__brawlerSeen||0)|(1<<(Math.round(f-1000)&15));
      }`);
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(async ({ setup, cpu, probe, menu, sample, volume, initialPick, knockAt }) => {
    await BlipBasic.submit('LOAD /TAPE/BRAWLER');
    if (initialPick) BlipBasic.lines.set(18, `PICK=${initialPick}`);
    if (!cpu) {
      let inside = false;
      for (const [n, line] of [...BlipBasic.lines]) {
        if (line.trim() === 'SUB UPDATE_CPU') { inside = true; continue; }
        if (inside && line.trim() === 'END SUB') { inside = false; continue; }
        if (inside) BlipBasic.lines.delete(n);
      }
    }
    if (knockAt > 0) {
      BlipBasic.lines.set(2421, `IF DRIVEBY_ACTIVE(1) AND DRIVEBY_T(1)>=${knockAt} AND TEST_DOWN=0 THEN`);
      BlipBasic.lines.set(2422, 'ACT$(1)="KNOCKDOWN":STUN(1)=240:TEST_DOWN=1');
      BlipBasic.lines.set(2423, 'END IF');
    }
    if (!menu) BlipBasic.lines.set(90, `START_GAME:INTRO=0:X(1)=220:X(2)=278:${setup || 'CPU_DELAY=999'}`);
    const wait = [...BlipBasic.lines].find(([, line]) => line.trim() === 'WAIT 0')[0];
    BlipBasic.lines.set(wait - 1, `SOUND ${probe},${sample},${volume}`);
    BlipBasic.submit('RUN');
  }, { setup, cpu, probe, menu, sample, volume, initialPick, knockAt });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'));
  const frame = page.frames().find((f) => f.url().includes('basic-runtime.html'));
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  try {
    await frame.waitForFunction(() => window.__brawler?.samples > 2, undefined, { timeout: 6000 });
  } catch {
    assert.fail(`Brawler did not reach its frame probe: ${await page.locator('body').getAttribute('data-wasm-error')}; ${await page.locator('#basic-output').textContent()}`);
  }
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.focus();
  return {
    ...handle, frame, canvas,
    state: () => frame.evaluate(() => window.__brawler),
    async press(key, ms = 70) {
      await page.keyboard.down(key);
      await page.waitForTimeout(ms);
      await page.keyboard.up(key);
    },
    async seen(move) {
      await frame.waitForFunction((m) => (window.__brawlerSeen & (1 << m)) !== 0, move, { timeout: 5000 });
      assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
    },
  };
}

test('BASIC Start opens selection and a held Start cannot also confirm RYUKA', async (t) => {
  const p = await fight(t, { menu: true, probe: '1000+PICK+16*(STATE$="SELECT")+32*(STATE$="PLAY")' });
  assert.equal((await p.state()).bits, 1, 'start on the title');
  await p.press('ArrowRight');
  assert.equal((await p.state()).bits, 1, 'right arrow does not choose a fighter from the title');
  await p.page.keyboard.down('f');
  await p.page.waitForTimeout(650);
  assert.equal((await p.state()).bits, 17, 'held Start stays on fighter selection');
  await p.page.keyboard.up('f');
  await p.page.waitForTimeout(180);
  await p.press('d');
  await p.frame.waitForFunction(() => window.__brawler?.bits === 18, undefined, { timeout: 1500 });
  assert.equal((await p.state()).bits, 18, 'right highlights BRUTUS');
  await p.page.waitForTimeout(180);
  await p.press('ArrowDown');
  await p.frame.waitForFunction(() => window.__brawler?.bits === 26, undefined, { timeout: 1500 });
  assert.equal((await p.state()).bits, 26, 'down selects the fighter below in the other row');
  await p.page.waitForTimeout(180);
  await p.press('ArrowUp');
  await p.frame.waitForFunction(() => window.__brawler?.bits === 18, undefined, { timeout: 1500 });
  assert.equal((await p.state()).bits, 18, 'up returns to the same column');
  await p.press('f');
  await p.frame.waitForFunction(() => (window.__brawler.bits & 32) !== 0);
  assert.equal((await p.state()).bits & 15, 2, 'the selected fighter starts the fight');
});

test('BASIC Brawler lets both players choose distinct fighters and control the match', async (t) => {
  const p = await fight(t, {
    menu: true,
    probe: '1000+1*(STATE$="TITLE")+2*(STATE$="SELECT")+4*(STATE$="PLAY")+8*(MODE=2)+16*LOCKED(1)+32*LOCKED(2)+64*(ACT$(2)="ATTACK")+128*(ACT$(1)="ATTACK")+256*(ACT$(1)="WALK")+512*(ACT$(2)="WALK")',
    sample: '(PICK+16*PICK2)/256',
    volume: '(WHO(1)+16*WHO(2))/256',
  });

  await p.press('ArrowDown');
  assert.equal((await p.state()).bits & 8, 8, 'Down selects local 2-player mode');
  await p.press('f');
  await p.frame.waitForFunction(() => (window.__brawler.bits & 2) !== 0);
  await p.press('d');
  await p.page.waitForTimeout(180);
  await p.press('ArrowLeft');
  assert.equal(Math.round((await p.state()).x / 100 * 256), 2 + 14 * 16, 'the two cursors can choose different fighters');

  await p.page.waitForTimeout(180);
  await p.press('j');
  await p.page.waitForTimeout(180);
  assert.equal((await p.state()).bits & 32, 32, 'P2 locks in with J');
  await p.press('f');
  await p.frame.waitForFunction(() => (window.__brawler.bits & 4) !== 0);
  assert.equal(Math.round((await p.state()).hp * 256), 2 + 14 * 16, 'the match starts with the selected fighters');

  await p.page.waitForTimeout(1350);
  await p.page.keyboard.down('a');
  await p.page.keyboard.down('ArrowRight');
  await p.frame.waitForFunction(() => (window.__brawler.bits & 768) === 768, undefined, { timeout: 2000 });
  await p.page.keyboard.up('a');
  await p.page.keyboard.up('ArrowRight');
  await p.press('f');
  await p.frame.waitForFunction(() => (window.__brawler.bits & 128) !== 0);
  await p.press('j');
  await p.frame.waitForFunction(() => (window.__brawler.bits & 64) !== 0);
});

test('BASIC touch D-pad selects a fighter without starting from the title', async (t) => {
  const p = await fight(t, {
    menu: true,
    view: { width: 390, height: 844 },
    probe: '1000+PICK+16*(STATE$="SELECT")+32*(STATE$="PLAY")',
  });
  const tap = async (selector) => {
    const box = await p.page.locator(selector).boundingBox();
    assert.ok(box && box.width > 0 && box.height > 0, `${selector} is visible on the touch deck`);
    await p.page.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2);
    await p.page.waitForTimeout(140);
  };
  await tap('#snes-pad .snes-btn:first-child');
  assert.equal((await p.state()).bits, 17, 'punch opens selection');
  await tap('#snes-pad .dp-right');
  assert.equal((await p.state()).bits, 18, 'right highlights BRUTUS without starting play');
  await tap('#snes-pad .snes-btn:first-child');
  await p.frame.waitForFunction(() => (window.__brawler.bits & 32) !== 0);
  assert.equal((await p.state()).bits & 15, 2, 'touch punch confirms the highlighted fighter');
});

test('BASIC touch Punch lands an attack during a match', async (t) => {
  const p = await fight(t, {
    view: { width: 390, height: 844 },
    setup: 'CPU_DELAY=999',
    probe: '1000+MV(1)+16*(ACT$(1)="ATTACK")',
    sample: 'HEALTH(2)/MAX(1,MAX_HP(WHO(2)))',
  });
  const button = p.page.locator('#snes-pad .snes-btn:first-child');
  const box = await button.boundingBox();
  assert.ok(box && box.width > 0 && box.height > 0, 'Punch is visible on the touch deck');
  await p.page.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2);
  await p.frame.waitForFunction(() => (window.__brawler?.bits & 16) !== 0,
    undefined, { timeout: 5000 });
  await p.frame.waitForFunction(() => window.__brawler?.hp < 1,
    undefined, { timeout: 5000 });
  assert.ok((await p.state()).hp < 1, 'the touch Punch damaged the opponent');
});

test('BASIC simultaneous touch Punch and Kick trigger MACK’s drive-by', async (t) => {
  const p = await fight(t, {
    view: { width: 390, height: 844 },
    setup: 'WHO(1)=15:X(1)=240:X(2)=500:CPU_DELAY=999',
    probe: '1000+MV(1)+16*(ACT$(1)="ATTACK")+32*DRIVEBY_ACTIVE(1)',
  });
  const caps = p.page.locator('#snes-pad .snes-btn');
  const punch = await caps.nth(0).boundingBox();
  const kick = await caps.nth(1).boundingBox();
  assert.ok(punch?.width && kick?.width, 'both face buttons are visible on the touch pad');
  const session = await p.page.context().newCDPSession(p.page);
  await session.send('Input.dispatchTouchEvent', {
    type: 'touchStart',
    touchPoints: [
      { id: 1, x: punch.x + punch.width / 2, y: punch.y + punch.height / 2 },
      { id: 2, x: kick.x + kick.width / 2, y: kick.y + kick.height / 2 },
    ],
  });
  await p.frame.waitForFunction(() => (window.__brawler?.bits & 32) !== 0,
    undefined, { timeout: 3000 });
  const bits = (await p.state()).bits;
  await session.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  await session.detach();
  assert.equal(bits, 53, 'MACK should start special move 5 and its independent drive-by timeline');
});

test('BASIC title attract rotates its featured fighter pair', async (t) => {
  const p = await fight(t, { menu: true, probe: '1000+INT(NOW/4) MOD FIGHTERS' });
  const firstPair = (await p.state()).bits;
  await p.frame.waitForFunction((first) => window.__brawler && window.__brawler.bits !== first,
    firstPair, { timeout: 6000 });
  assert.notEqual((await p.state()).bits, firstPair,
    'the title should feature a new pair after its four-second turn');
});

test('BASIC selection shows each fighter power and flags ZENITH as OVERPOWERED', async (t) => {
  const p = await fight(t, {
    menu: true,
    initialPick: 10,
    probe: '1000+PICK+16*(STATE$="SELECT")+32*(POWER(PICK)=3)+64*(POWER_DESC$(PICK)="OVERPOWERED")+128*BOSS(PICK)',
  });
  assert.equal((await p.state()).bits, 234, 'the title shows Zenith stats without starting the fight');
  await p.press('f');
  await p.frame.waitForFunction(() => (window.__brawler.bits & 16) !== 0);
  assert.equal((await p.state()).bits, 250, 'ZENITH shows 3x power and the OVERPOWERED tag while still in selection');
});

test('BASIC selects MACK with his named special and THE HOOD home stage', async (t) => {
  const p = await fight(t, {
    menu: true,
    initialPick: 15,
    probe: '1000+PICK+16*(STATE$="SELECT")+32*(SPECIAL_KIND(PICK)=5)+64*(SPECIAL_NAME$(PICK)="YO! HOMIES!")+128*(HOME(PICK)=8)+256*(POWER_DESC$(PICK)="LOWRIDER DRIVE-BY")',
  });
  await p.press('f');
  assert.equal((await p.state()).bits, 511, 'selection must identify MACK, his drive-by, and THE HOOD');
  await p.press('f');
  await p.frame.waitForFunction(() => (window.__brawler.bits & 16) === 0);
  assert.equal((await p.state()).bits & 15, 15, 'MACK starts as the chosen fighter');
});

test('BASIC MACK drive-by lands a three-shot burst without a knockdown lockout', async (t) => {
  const p = await fight(t, {
    setup: 'WHO(1)=15:HEALTH(1)=108:X(1)=220:X(2)=410',
    sample: 'HEALTH(2)/MAX(1,MAX_HP(WHO(2)))',
  });
  assert.equal((await p.state()).hp, 1);
  await p.page.keyboard.down('f');
  await p.page.waitForTimeout(35);
  await p.page.keyboard.down('g');
  await p.page.waitForTimeout(55);
  await p.page.keyboard.up('f');
  await p.page.keyboard.up('g');
  await p.seen(5);
  await p.frame.waitForFunction(() => window.__brawler.hp <= 0.94, undefined, { timeout: 5000 });
  const remaining = (await p.state()).hp;
  assert.ok(remaining >= 0.85 && remaining <= 0.94, `three pellets should add up to about 15 damage; got ${(1 - remaining) * 108}`);
});

test('BASIC MACK parks, bounces, fires in three beats, then exits the stage', async (t) => {
  const p = await fight(t, {
    setup: 'WHO(1)=15:X(1)=240:X(2)=500:CPU_DELAY=999',
    probe: '1000+T(1)+1000*DRIVEBY_SHOTS(1)+5000*(ACT$(1)="ATTACK")',
    sample: 'CAR_Y/400',
    volume: 'MIN(1,MAX(0,CAR_X/1000))',
  });
  const state = async () => {
    const s = await p.state();
    const n = Math.round(s.probe - 1000);
    return { attack: Math.floor(n / 5000) === 1, tick: n % 1000,
      burst: Math.floor(n / 1000) % 5, carX: s.hp * 1000, carY: s.sample * 400 };
  };
  const until = (condition, message) => p.frame.waitForFunction((c) => {
    const s = window.__brawler;
    if (!s) return false;
    const n = Math.round(s.probe - 1000);
    const attack = Math.floor(n / 5000) === 1, tick = n % 1000, burst = Math.floor(n / 1000) % 5;
    return (!c.attack || attack) && tick >= c.min && tick < c.max &&
      (c.burst === null || burst === c.burst);
  }, condition, { timeout: 5000 }).catch(() => assert.fail(message));

  await p.page.keyboard.down('f');
  await p.page.waitForTimeout(35);
  await p.page.keyboard.down('g');
  await p.page.waitForTimeout(55);
  await p.page.keyboard.up('f');
  await p.page.keyboard.up('g');
  await until({ attack: true, min: 0, max: 1000, burst: null }, 'MACK never entered the drive-by');
  await until({ attack: true, min: 32, max: 56, burst: 0 },
    'the car did not pause before firing');
  const parked = await state();
  assert.ok(Math.abs(parked.carX - 110) < 1, `the car should hold beside MACK; got x=${parked.carX}`);
  await until({ attack: true, min: 44, max: 56, burst: 0 },
    'the car left its pre-shot pause early');
  const bounced = await state();
  assert.ok(Math.abs(bounced.carX - parked.carX) < 1, 'the car should bounce in place before firing');
  assert.ok(Math.abs(bounced.carY - parked.carY) > 5, 'the hydraulic bounce should move the car visibly');

  await until({ attack: true, min: 56, max: 71, burst: 1 },
    'the first shot did not fire');
  const first = await state();
  assert.ok(first.tick >= 56 && first.tick < 71, `shot one fired on tick ${first.tick}`);
  await until({ attack: true, min: 71, max: 86, burst: 2 },
    'the second shot did not fire');
  const second = await state();
  assert.ok(second.tick >= 71 && second.tick < 86, `shot two fired on tick ${second.tick}`);
  await until({ attack: true, min: 86, max: 102, burst: 3 },
    'the third shot did not fire');
  const third = await state();
  assert.ok(third.tick >= 86 && third.tick < 102, `shot three fired on tick ${third.tick}`);
  assert.ok(Math.abs(third.carX - parked.carX) < 1, 'the lowrider should remain parked for the whole burst');

  await until({ attack: true, min: 174, max: 183, burst: 3 },
    'the car did not drive across the stage after its burst');
  const exiting = await state();
  assert.ok(exiting.carX > 704, `the car should clear the far edge before recovery ends; got x=${exiting.carX}`);
});

test('BASIC MACK keeps the drive-by going when he is knocked down', async (t) => {
  const p = await fight(t, {
    setup: 'WHO(1)=15:X(1)=240:X(2)=500:CPU_DELAY=999',
    knockAt: 30,
    probe: '1000+DRIVEBY_T(1)+1000*DRIVEBY_SHOTS(1)+5000*DRIVEBY_ACTIVE(1)',
    sample: 'MIN(1,MAX(0,CAR_X/1000))',
    volume: '(ACT$(1)="KNOCKDOWN")',
  });
  const snap = () => p.state().then((s) => {
    const n = Math.round(s.probe - 1000);
    return { active: Math.floor(n / 5000) === 1, tick: n % 1000,
      shots: Math.floor(n / 1000) % 5, carX: s.sample * 1000, down: s.hp === 1 };
  });
  const waitFor = (condition, message) => p.frame.waitForFunction((c) => {
    const s = window.__brawler;
    if (!s) return false;
    const n = Math.round(s.probe - 1000);
    const actual = { active: Math.floor(n / 5000) === 1, tick: n % 1000,
      shots: Math.floor(n / 1000) % 5, down: s.hp === 1 };
    return Object.entries(c).every(([key, value]) => actual[key] === value ||
      (Array.isArray(value) && actual[key] >= value[0] && actual[key] < value[1]));
  }, condition, { timeout: 5000 }).catch(() => assert.fail(message));

  await p.page.keyboard.down('f');
  await p.page.waitForTimeout(35);
  await p.page.keyboard.down('g');
  await p.page.waitForTimeout(55);
  await p.page.keyboard.up('f');
  await p.page.keyboard.up('g');
  await waitFor({ active: true, tick: [30, 56], down: true }, 'MACK did not get knocked down during the call');
  for (const [shots, min, max] of [[1, 56, 71], [2, 71, 86], [3, 86, 102]]) {
    await waitFor({ active: true, tick: [min, max], shots, down: true },
      `the drive-by stopped after MACK went down (shot ${shots})`);
  }
  await waitFor({ active: true, tick: [174, 183], shots: 3, down: true },
    'the lowrider did not finish its exit while MACK was down');
  const exiting = await snap();
  assert.ok(exiting.carX > 704, `the lowrider should clear the far edge; got x=${exiting.carX}`);
});

test('BASIC Brawler lands a jab once, then recovers', async (t) => {
  const p = await fight(t);
  await p.press('f');
  await p.seen(1);
  await p.frame.waitForFunction(() => window.__brawler.hp < 1);
  const hp = (await p.state()).hp;
  await p.page.waitForTimeout(550);
  assert.equal((await p.state()).hp, hp, 'one active window must not hit twice');
  assert.equal((await p.state()).bits & 16, 0, 'the jab must finish');
});

test('BASIC fighters can damage OVERPOWERED ZENITH', async (t) => {
  const p = await fight(t, {
    setup: 'WHO(2)=10:HEALTH(2)=200:X(1)=220:X(2)=278',
    sample: 'HEALTH(2)/100',
  });
  assert.equal((await p.state()).hp, 1);
  await p.press('f');
  await p.frame.waitForFunction(() => window.__brawler.hp < 1);
  assert.ok((await p.state()).hp > 0, 'the 3x boss loses health but remains a challenge');
});

test('BASIC Brawler faces left and hits an opponent on that side', async (t) => {
  const p = await fight(t, { setup: 'X(1)=278:X(2)=220' });
  assert.equal((await p.state()).bits & 32, 32);
  await p.press('f');
  await p.frame.waitForFunction(() => window.__brawler.hp < 1);
});

test('BASIC Brawler accepts a special with the two buttons a few frames apart', async (t) => {
  const p = await fight(t, { setup: 'X(2)=400' });
  await p.page.keyboard.down('f');
  await p.page.waitForTimeout(35);
  await p.page.keyboard.down('g');
  await p.page.waitForTimeout(55);
  await p.page.keyboard.up('f');
  await p.page.keyboard.up('g');
  await p.seen(5);
  await p.frame.waitForFunction(() => window.__brawler.hp < 1);
});

test('BASIC Brawler has a rising uppercut and a low sweep', async (t) => {
  const p = await fight(t);
  await p.page.keyboard.down('s');
  await p.press('f');
  await p.seen(8);
  await p.frame.waitForFunction(() => (window.__brawler.bits & 64) !== 0);
  await p.page.waitForTimeout(650);
  await p.press('g');
  await p.seen(7);
  await p.page.keyboard.up('s');
});

test('BASIC a low guard stops a jab but loses to an overhead', async (t) => {
  const p = await fight(t, { setup: 'GUARD(2)=1:LOW(2)=1' });
  await p.press('f');
  await p.seen(1);
  await p.page.waitForTimeout(500);
  assert.equal((await p.state()).hp, 1, 'a crouching guard stops a low punch');
  await p.page.keyboard.down('d');
  await p.press('f');
  await p.page.keyboard.up('d');
  await p.seen(2);
  await p.frame.waitForFunction(() => window.__brawler.hp < 1);
});

test('BASIC a standing guard stops a high kick but loses to a sweep', async (t) => {
  const p = await fight(t, { setup: 'GUARD(2)=1:LOW(2)=0' });
  await p.page.keyboard.down('d');
  await p.press('g');
  await p.page.keyboard.up('d');
  await p.seen(4);
  await p.page.waitForTimeout(800);
  assert.equal((await p.state()).hp, 1, 'standing guard stops the high kick');
  await p.page.keyboard.down('s');
  await p.press('g');
  await p.page.keyboard.up('s');
  await p.seen(7);
  await p.frame.waitForFunction(() => window.__brawler.hp < 1);
});

test('BASIC two active punches trade rather than favouring player one', async (t) => {
  const p = await fight(t, {
    setup: 'X(2)=272:START_ATTACK 1,1:START_ATTACK 2,1',
    probe: '1000+HEALTH(1)',
  });
  await p.page.waitForTimeout(400);
  assert.ok((await p.state()).bits < 100, 'player one should take the answering hit');
  assert.ok((await p.state()).hp < 1, 'player two should take the first hit');
});

test('BASIC remembers a kick pressed just before jab recovery ends', async (t) => {
  const p = await fight(t, { sample: 'T(1)/100' });
  await p.press('f');
  await p.frame.waitForFunction(() => window.__brawler.x >= 8 && window.__brawler.x <= 12);
  await p.press('g');
  await p.seen(3);
});

test('BASIC the CPU can finish a round against an idle player', async (t) => {
  const p = await fight(t, {
    cpu: true, setup: 'CPU_DELAY=0',
    probe: '1000+HEALTH(1)+512*(STATE$<>"PLAY")',
  });
  await p.frame.waitForFunction(() => window.__brawler.bits < 100);
  await p.frame.waitForFunction(() => window.__brawler.bits >= 512, undefined, { timeout: 50000 });
  assert.equal((await p.state()).bits, 512, 'the round ends on a knockout');
});

for (const [pick, name] of [[11, 'JADE'], [12, 'REGENT'], [13, 'KITE'], [14, 'AURELIA']]) {
  test(`BASIC ${name} moves, jumps, attacks and uses a special`, async (t) => {
    const p = await fight(t, { setup: `PICK=${pick}:START_GAME:INTRO=0:X(1)=220:X(2)=350` });
    await p.page.keyboard.down('d');
    await p.page.waitForTimeout(200);
    await p.page.keyboard.up('d');
    assert.ok((await p.state()).x > 230, `${name} should walk`);
    await p.press('w');
    await p.frame.waitForFunction(() => (window.__brawler.bits & 64) !== 0);
    await p.press('g');
    await p.seen(6);
    await p.page.waitForTimeout(800);
    await p.page.keyboard.down('f');
    await p.page.keyboard.down('g');
    await p.page.waitForTimeout(80);
    await p.page.keyboard.up('f');
    await p.page.keyboard.up('g');
    await p.seen(5);
  });
}
