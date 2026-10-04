import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { evaluate, sleep } from './lib/playwright-utils.mjs';
import { startPage } from './lib/harness.mjs';

const shots = process.env.BLIP_JUKEBOX_SHOTS || '/tmp/blip-jukebox-rounds';
const { browser, cdp, origin, close } = await startPage('chromium', { viewport:{width:1280,height:800} });
await cdp.send('Emulation.setDeviceMetricsOverride', { width:1280, height:800, deviceScaleFactor:1, mobile:false });
const open = async (url, wait=3800) => { await cdp.send('Page.navigate', { url:`${origin}/${url}` }); await sleep(wait); };
const key = async (keyName, code, vk) => {
  for (const type of ['keyDown','keyUp']) { await cdp.send('Input.dispatchKeyEvent', { type, key:keyName, code, windowsVirtualKeyCode:vk }); await sleep(80); }
};
const shot = async (name) => {
  await mkdir(shots, { recursive:true });
  try {
    const png = await cdp.send('Page.captureScreenshot', { format:'png', fromSurface:true,
      clip:{ x:0, y:0, width:1280, height:800, scale:0.5 } });
    await writeFile(path.join(shots, `${name}.png`), Buffer.from(png.data, 'base64'));
  } catch (error) { process.stderr.write(`screenshot ${name}: ${error.message}\n`); }
};

try {
  await open('index.html');
  assert.equal(await evaluate(cdp, "document.querySelectorAll('.jukebox-transfer.idle .jukebox-arm-mount').length"),1,'the folded arm is present before selection');
  const parkedArm=JSON.parse(await evaluate(cdp,`JSON.stringify((function(){var stage=document.querySelector('.jukebox-transfer.idle'),bar=stage.querySelector('.jukebox-mechanics-bar').getBoundingClientRect(),arm=stage.querySelector('.jukebox-transfer-arm'),shaft=arm.querySelector('.jukebox-arm-shaft'),forearm=arm.querySelector('.jukebox-arm-forearm'),wrist=arm.querySelector('.jukebox-arm-wrist');function end(el){var m=new DOMMatrix(getComputedStyle(el).transform),x=parseFloat(el.style.left),y=parseFloat(el.style.top);return{x:x+m.a*el.offsetWidth,y:y+m.b*el.offsetWidth};}var a=end(shaft),b=end(forearm),joint={x:parseFloat(forearm.style.left),y:parseFloat(forearm.style.top)},tip={x:parseFloat(wrist.style.left),y:parseFloat(wrist.style.top)};return{bar:{top:bar.top,bottom:bar.bottom},joint:joint,errors:[Math.hypot(a.x-joint.x,a.y-joint.y),Math.hypot(b.x-tip.x,b.y-tip.y)]};})())`));
  assert.ok(parkedArm.joint.y>=parkedArm.bar.top&&parkedArm.joint.y<=parkedArm.bar.bottom,'parked arm elbows stay on the visible mechanism bar');
  assert.ok(parkedArm.errors.every(error=>error<1),`parked links close cleanly at their joints: ${JSON.stringify(parkedArm)}`);
  assert.ok(await evaluate(cdp, "parseFloat(getComputedStyle(document.querySelector('.card-focused .card-desc')).fontSize)>=14"),'focused game details are readable on their card');
  await shot('round-01-rolodex');
  await evaluate(cdp, "document.getElementById('kiosk-insert-btn').click(); true");
  await sleep(300);


  await evaluate(cdp, `var transfer=window.blipTransferCard; window.blipTransferCard=function(options){if(!options.original)return transfer(options);window.__pickupBox=options.original.getBoundingClientRect().toJSON(); window.__pickupStyle={transform:getComputedStyle(options.original).transform,width:options.original.offsetWidth,height:options.original.offsetHeight,computedHeight:getComputedStyle(options.original).height,scale:options.scale};window.__heldNav=options.onDone; options.onDone=function(){}; var stage=transfer(options);stage.blipSeek(0);return stage;}; true`);
  const sequence = [1,2,3,4,5,6,7,0,7,0];
  const deckLength = 11;
  const games = ['rally','bouncer','galactic_defender','bubbler','sky_raider','meteors','serpent','brawler'];
  const labels = { rally:'RALLY', bouncer:'BOUNCER', galactic_defender:'DEFENDER', bubbler:'BUBBLER', sky_raider:'RAIDER', meteors:'METEORS', serpent:'SERPENT', brawler:'BRAWLER' };
  let selectedIndex = 0;
  for (let round=0; round<sequence.length; round++) {
    const target = sequence[round];
    const available=await evaluate(cdp, "Array.from(document.querySelectorAll('.card')).map((card,index)=>card.classList.contains('card-loaded')?-1:index).filter(index=>index>=0)");
    selectedIndex=await evaluate(cdp, "Array.from(document.querySelectorAll('.card')).indexOf(document.querySelector('.card-focused'))");
    const current=available.indexOf(selectedIndex), desired=available.indexOf(target);
    assert.ok(desired>=0,'target game card is available in the Rolodex');
    const rightSteps=(desired-current+available.length)%available.length;
    const leftSteps=(current-desired+available.length)%available.length;
    const direction=rightSteps<=leftSteps?'ArrowRight':'ArrowLeft';
    for(let step=0;step<Math.min(rightSteps,leftSteps);step++) await key(direction,direction,direction==='ArrowRight'?39:37);
    selectedIndex=target;
    const picked = await evaluate(cdp, "document.querySelector('.card-focused .card-title').textContent.trim()");
    assert.equal(picked, labels[games[target]], `round ${round+1} highlighted title`);
    if(round===1) await evaluate(cdp, `window.__cycles=[];var originalTransfer=blipTransferCard;window.blipTransferCard=function(options){window.__cycles.push(options.returning?'return':options.idle?'idle':'insert');return originalTransfer(options);};true`);
    if(round===1) await evaluate(cdp,"document.querySelector('#fire-buttons .arcade-btn').dispatchEvent(new PointerEvent('pointerdown',{bubbles:true,cancelable:true,pointerType:'mouse'}));true");
    else await key(' ', 'Space', 32);
    if(round===0) {
      await sleep(1000);
      assert.equal(await evaluate(cdp, "document.querySelector('.jukebox-transfer').dataset.phase"),'grip','arm begins moving immediately after selection');
      await shot('selected-card-reading');

    }
    if (round === 0) {
      await sleep(1000);
      await evaluate(cdp, "document.querySelector('.jukebox-transfer').blipSeek(2100); true");
      const handoff = JSON.parse(await evaluate(cdp, `JSON.stringify({before:window.__pickupBox,styles:window.__pickupStyle,after:document.querySelector('.jukebox-flight-card').getBoundingClientRect().toJSON()})`));
      process.stdout.write(`handoff ${JSON.stringify(handoff)}\n`);
      for (const axis of ['x','y','width','height']) assert.ok(Math.abs(handoff.before[axis]-handoff.after[axis])<1, `pickup preserves the card's ${axis}`);
      const armState = await evaluate(cdp, `JSON.stringify((function(){var a=document.querySelector('.jukebox-transfer-arm'),c=document.querySelector('.jukebox-flight-card');
        function state(n){if(!n)return null;var r=n.getBoundingClientRect(),s=getComputedStyle(n);return {box:[r.left,r.top,r.width,r.height],size:[n.offsetWidth,n.offsetHeight],opacity:s.opacity,display:s.display,transform:s.transform};}
        var module=document.querySelector('.card[href="bouncer/index.html"]');
        return {stage:state(a&&a.parentElement),arm:state(a),card:state(c),module:state(module),modulePicked:module&&getComputedStyle(module).visibility==='hidden',child:a&&a.contains(c),receiver:state(document.querySelector('.jukebox-receiver')),bezel:state(document.querySelector('#screen-bezel'))};})())`);
      process.stdout.write(`picker state ${armState}\n`);
      const parsed = JSON.parse(armState);
      assert.ok(parsed.arm && parsed.card && parsed.child, 'the selected card is mechanically attached to the swinging arm');
      assert.deepEqual(parsed.card.size, [handoff.styles.width,handoff.styles.height], 'the arm carries the enlarged card at its physical size');
      assert.equal(parsed.modulePicked, true, 'the selected storage bay clears as the arm lifts its card');
      const geometry=JSON.parse(await evaluate(cdp, `JSON.stringify((function(){var stage=document.querySelector('.jukebox-transfer'), arm=stage.querySelector('.jukebox-transfer-arm'), last=null,maxJump=0,maxError=0;for(var ms=2100;ms<=14700;ms+=10){stage.blipSeek(ms);var a=arm.querySelector('.jukebox-arm-shaft'), b=arm.querySelector('.jukebox-arm-forearm'), w=arm.querySelector('.jukebox-arm-wrist');var points=[a,b].map(function(el){var m=new DOMMatrix(getComputedStyle(el).transform),x=parseFloat(el.style.left),y=parseFloat(el.style.top);return {x:x,y:y,endX:x+m.a*el.offsetWidth,endY:y+m.b*el.offsetWidth};});maxError=Math.max(maxError,Math.hypot(points[0].endX-points[1].x,points[0].endY-points[1].y),Math.hypot(points[1].endX-parseFloat(w.style.left),points[1].endY-parseFloat(w.style.top)));if(last)maxJump=Math.max(maxJump,Math.hypot(points[1].x-last.x,points[1].y-last.y));last=points[1];}return {maxJump:maxJump,maxError:maxError};})())`));
      assert.ok(geometry.maxError<1, `rigid links meet their joints: ${JSON.stringify(geometry)}`);
      assert.ok(geometry.maxJump<15, `elbow follows a continuous path: ${JSON.stringify(geometry)}`);
      await evaluate(cdp, "window.__visualStage=document.querySelector('.jukebox-transfer'); window.__visualStage.blipSeek(5100); true");
      assert.equal(await evaluate(cdp, "document.querySelectorAll('.rolodex-empty-bay').length"),1,'the actual Rolodex slot is left empty');
      await shot('round-01-picker-swing');
      await evaluate(cdp, 'window.__visualStage.blipSeek(8350); true'); await shot('round-01-feeding-card');
      await evaluate(cdp, 'window.__visualStage.blipSeek(13700); true');
      assert.ok(await evaluate(cdp, "Math.abs(new DOMMatrix(getComputedStyle(document.querySelector('.jukebox-mechanics-bar')).transform).m42 + 30)<1"),'mechanics bar is halfway retracted after 1000 ms');
      await shot('round-01-bar-retracting');
      await evaluate(cdp, 'window.__visualStage.blipSeek(14700); true'); await shot('round-01-seated-card');
      await evaluate(cdp, 'window.__heldNav(); true');
    }
    if(round===1) {
      let cycles=[];
      for(let attempt=0;attempt<70;attempt++) {
        cycles=await evaluate(cdp, 'window.__cycles || []');
        if(cycles.includes('return')&&cycles.includes('insert')) break;
        await sleep(500);
      }
      const transferState=await evaluate(cdp,"JSON.stringify({cycles:window.__cycles,loaded:sessionStorage.getItem('blip-loaded-card'),focused:document.querySelector('.card-focused')?.getAttribute('href'),focusedLoaded:document.querySelector('.card-focused')?.className,cabinetReturn:document.body.className,deploying:document.querySelector('.jukebox-transfer[data-deploying]')?.dataset.deploying,flight:!!document.querySelector('.jukebox-transfer.active:not(.idle)'),inFlight:typeof cardInFlight==='undefined'?null:cardInFlight,coins:sessionStorage.getItem('blip-coins')})");
      assert.equal(cycles[0],'return',`previous game card returns to the Rolodex first: ${transferState}`);
      assert.ok(cycles.includes('insert'),'new pickup starts after returning the old card');
      await sleep(18500);
    } else await sleep(round===0?4000:33000);
    assert.equal(await evaluate(cdp, 'location.pathname.split("/")[1]'), games[target], `round ${round+1} game route`);
    let actual = '';
    for (let attempt=0; attempt<15 && !actual; attempt++) {
      actual = await evaluate(cdp, "document.querySelector('#marquee-name')?.getAttribute('aria-label') || ''");
      if (!actual) await sleep(200);
    }
    if (!actual) process.stderr.write('page state: ' + await evaluate(cdp, "JSON.stringify({url:location.href,title:document.title,body:document.body.innerHTML.slice(0,600)})") + '\n');
    assert.equal(actual, labels[games[target]], `round ${round+1} marquee title`);
    assert.equal(await evaluate(cdp, "document.querySelector('.jukebox-receiver').classList.contains('loaded')"), true, `round ${round+1} receiver closed`);
    assert.equal(await evaluate(cdp, "getComputedStyle(document.querySelector('.jukebox-seated-card')).opacity"), '0', `round ${round+1} card fully seated`);
    assert.equal(await evaluate(cdp, "document.querySelectorAll('.jukebox-transfer').length"),0,'game area has no transfer overlay');
    assert.equal(await evaluate(cdp, "getComputedStyle(document.querySelector('.jukebox-receiver')).visibility"),'hidden','receiver retracts before the game screen appears');
    await shot(`round-${String(round+1).padStart(2,'0')}-game`);
    process.stdout.write(`round ${round+1}: ${games[target]} selected, seated, title revealed\n`);
    if (round < sequence.length-1) await open('index.html', 4500);
  }

  await evaluate(cdp, "document.querySelector('.blip-logo').click(); true");
  await sleep(500);
  assert.equal(await evaluate(cdp, "document.body.classList.contains('cabinet-return')"),true,'BLIP logo brings the Rolodex back slowly');
  assert.equal(await evaluate(cdp, "document.querySelector('.jukebox-transfer.idle')?.dataset.deploying"),'true','mechanics bar is deploying on return');
  await shot('return-mechanics-opening');
  await sleep(4300);
  assert.equal(await evaluate(cdp, "!!document.querySelector('.jukebox-transfer[data-deploying]')"),false,'mechanics bar finishes opening');

  for (const [index, page] of ['about','controls','history'].entries()) {
    await evaluate(cdp, "sessionStorage.removeItem('blip-last-game'); sessionStorage.setItem('blip-coins','0'); true");
    await open('index.html', 4500);
    let guideCard = '';
    for (let step=0; step<12 && guideCard !== `${page}.html`; step++) {
      guideCard = await evaluate(cdp, "document.querySelector('.card-focused')?.getAttribute('href') || ''");
      if (guideCard !== `${page}.html`) await key('ArrowRight', 'ArrowRight', 39);
    }
    guideCard = await evaluate(cdp, "document.querySelector('.card-focused')?.getAttribute('href') || ''");
    assert.equal(guideCard, `${page}.html`, `${page} card sits behind all games`);
    await key(' ', 'Space', 32);
    await sleep(1200);
    assert.equal(await evaluate(cdp, 'location.pathname.split("/").pop()'), `${page}.html`, `${page} opens without a credit`);
    assert.equal(await evaluate(cdp, "document.querySelectorAll('.lang-switcher,.left-badges').length"), 0, `${page} has no badges`);
    assert.equal(await evaluate(cdp, 'document.documentElement.lang'), 'en', `${page} stays in English`);
    assert.equal(await evaluate(cdp, "document.querySelectorAll('.jukebox-transfer.idle .jukebox-arm-mount').length"),1,`${page} shares the parked arm component`);
    process.stdout.write(`${page}: rear Rolodex card opened without a credit\n`);
  }
} finally {
    await close();
}
