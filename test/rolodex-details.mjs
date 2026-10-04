import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import {evaluate,sleep} from './lib/playwright-utils.mjs';
import {startPage} from './lib/harness.mjs';
const {cdp,origin,close}=await startPage('chromium',{hasTouch:true,viewport:{width:1280,height:844}});
try {
  for(const width of [1280,768,390,320]) {
    await cdp.send('Emulation.setDeviceMetricsOverride',{width,height:844,deviceScaleFactor:1,mobile:width<620});
    await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:width<620});
    await cdp.send('Page.navigate',{url:`${origin}/index.html`});
    await sleep(1500);
    await evaluate(cdp,'document.fonts.ready.then(()=>true)');
    assert.equal(await evaluate(cdp,"getComputedStyle(document.body).userSelect"),'none',`${width}px landing page prevents accidental text selection`);
    const stack=await evaluate(cdp,"(function(){var cards=Array.from(document.querySelectorAll('#game-grid .card')),book=document.querySelector('#manual-book'),bezel=document.querySelector('.screen-bezel').getBoundingClientRect(),bar=document.querySelector('#kiosk-bar').getBoundingClientRect(),box=book.getBoundingClientRect();return {inside:cards.every(function(card){var r=card.getBoundingClientRect();return r.left>=bezel.left&&r.right<=bezel.right&&r.top>=bezel.top&&r.bottom<=bezel.bottom}),bookOnShelf:box.left>=bar.left&&box.right<=bar.right&&box.top>=bar.top-12&&box.bottom<=bar.bottom&&box.width<150&&box.height<38,upright:cards.every(function(card){return parseFloat(card.style.getPropertyValue('--arc-turn'))===0&&parseFloat(card.style.getPropertyValue('--arc-tilt'))===0}),scrollWidth:document.querySelector('#game-grid').scrollWidth,clientWidth:document.querySelector('#game-grid').clientWidth,bookHint:book.getAttribute('aria-label').toLowerCase().includes('lift'),hiddenGame:getComputedStyle(document.querySelector('.jukebox-game-screen')).visibility==='hidden'};})()");
    assert.equal(stack.inside,true,`${width}px every complete game card remains inside the cabinet`);
    assert.equal(stack.bookOnShelf,true,`${width}px compact field manual sits on the bottom bar shelf`);
    assert.equal(stack.upright,true,`${width}px cards stand upright in their stack`);
    assert.equal(stack.bookHint,true,`${width}px physical manual shows the pull-tab pickup hint`);
    assert.ok(stack.scrollWidth<=stack.clientWidth,`${width}px card rack does not extend beyond the screen`);
    assert.equal(stack.hiddenGame,true,`${width}px stored game cannot steal focus behind the Rolodex`);
    let fixedRail;
    for(let index=0;index<8;index++) {
      if(index)await evaluate(cdp,"document.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowRight',code:'ArrowRight',bubbles:true}));true");
      await sleep(1100);
      const state=await evaluate(cdp,`(function(){var card=document.querySelectorAll('.card')[${index}],track=document.querySelector('#game-grid'),box=card.getBoundingClientRect(),bezel=document.querySelector('.screen-bezel').getBoundingClientRect(),copy=card.querySelector('.card-desc'),text=[card.querySelector('.card-title'),card.querySelector('.card-genre'),copy],ranges=text.flatMap(function(el){var range=document.createRange();range.selectNodeContents(el);return Array.from(range.getClientRects()).filter(function(r){return r.width>0})}),stays=document.querySelector('.rolodex-spindle .stay-metal').getAttribute('d');return {rail:document.querySelector('.rail-metal').getAttribute('d'),tabsInside:Array.from(document.querySelectorAll('.card-index-tab')).every(function(tab){var r=tab.getBoundingClientRect();return r.left>=bezel.left&&r.right<=bezel.right&&r.top>=bezel.top&&r.bottom<=bezel.bottom;}),title:card.querySelector('.card-title').textContent,font:parseFloat(getComputedStyle(copy).fontSize),focused:card.classList.contains('card-focused')&&card.style.getPropertyValue('--arc-pitch')==='0deg',opaque:getComputedStyle(card).opacity==='1'&&getComputedStyle(card).backgroundColor!=='rgba(0, 0, 0, 0)',supported:(stays.match(/M/g)||[]).length===2&&(stays.match(/L/g)||[]).length===2,cardInside:box.left>=bezel.left&&box.right<=bezel.right&&box.bottom<=bezel.bottom,box:{left:box.left,right:box.right,top:box.top,bottom:box.bottom},frame:{left:bezel.left,right:bezel.right,top:bezel.top,bottom:bezel.bottom},scroll:{left:track.scrollLeft,width:track.scrollWidth,client:track.clientWidth,offset:card.offsetLeft,cardWidth:card.offsetWidth},textInside:ranges.every(function(r){return r.left>=box.left&&r.right<=box.right&&r.top>=box.top&&r.bottom<=box.bottom}),shown:[copy].every(function(el){return getComputedStyle(el).display!=='none'&&el.getClientRects().length>0}),height:card.offsetHeight};})()`);
      assert.ok(state.font>=14&&state.focused,`${width}px ${state.title}: card text remains readable`);
      assert.equal(state.cardInside,true,`${width}px ${state.title}: card inside bezel (${JSON.stringify(state)})`);
      assert.equal(state.textInside,true,`${width}px ${state.title}: all text fits on the card`);
      assert.equal(state.shown,true,`${width}px ${state.title}: printed description is shown`);
      assert.equal(state.opaque,true,`${width}px ${state.title}: laminated stock stays opaque`);
      assert.equal(state.supported,true,`${width}px ${state.title}: arch rail has two side stays`);
      assert.equal(state.tabsInside,true,`${width}px ${state.title}: physical index tabs fit inside the cabinet`);
      if(fixedRail)assert.equal(state.rail,fixedRail,'the arch remains rigid while the cards pivot');
      fixedRail=state.rail;
      assert.ok(state.height>=490,`${width}px ${state.title}: card is enlarged (${state.height}px)`);
    }
    console.log(`${width}px: all eight game cards fit in the upright stack with readable details`);
    if(width===320 || width===1280) {
      await evaluate(cdp,"var card=document.querySelector('.card-brawler'),track=document.querySelector('#game-grid');card.dispatchEvent(new Event('pointerenter'));track.scrollLeft=card.offsetLeft+card.offsetWidth/2-track.clientWidth/2;true");
      await sleep(400);
      await mkdir('/tmp/blip-details',{recursive:true});
      try{const shot=await cdp.send('Page.captureScreenshot',{format:'png',fromSurface:false});await writeFile(`/tmp/blip-details/${width===320?'phone':'desktop'}.png`,Buffer.from(shot.data,'base64'));}catch(error){console.log(`screenshot: ${error.message}`);}
    }
  }
  for(const page of ['about','controls','history']) {
    await cdp.send('Page.navigate',{url:`${origin}/${page}.html`});await sleep(1800);
    assert.equal(await evaluate(cdp,"!document.querySelector('#manual-overlay').hidden"),true,`${page} opens as a chapter in the physical manual`);
    const expectedTitle=page==='about'?'about blip':page;
    assert.equal((await evaluate(cdp,"document.querySelector('#manual-book-title').textContent")).toLowerCase(),expectedTitle,`${page} direct address opens to its printed chapter`);
  }
  console.log('About, Controls, History: direct addresses open the combined manual');
  for(const [width,height] of [[320,844],[390,844],[844,390],[1280,844]]) {
    const mobile=width<950;
    await cdp.send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile});
    await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:mobile});
    for(const page of ['index','about','controls','history','api','galactic_defender/index']) {
      await cdp.send('Page.navigate',{url:`${origin}/${page}.html`});await sleep(1000);
      const state=await evaluate(cdp,"(function(){var title=document.querySelector('#marquee-name');return {title:getComputedStyle(title).visibility,utilityButtons:document.querySelectorAll('#fullscreen-btn,#mute-btn').length};})()");
      assert.equal(state.title,mobile?'hidden':'visible',`${page} at ${width}px title`);
      assert.equal(state.utilityButtons,0,`${page} at ${width}px omits fullscreen and sound buttons`);
    }
    console.log(`${width}×${height}: title visibility passed on all six pages`);
  }
  await cdp.send('Page.navigate',{url:`${origin}/index.html?manual=about`});await sleep(2200);
  const book=await evaluate(cdp,"(function(){var overlay=document.querySelector('#manual-overlay'),page=document.querySelector('.manual-page-content'),leaf=page.querySelector('.manual-leaf-sheet'),rect=overlay.querySelector('.manual-open-book').getBoundingClientRect(),frame=overlay.getBoundingClientRect();return {open:!overlay.hidden,front:parseInt(getComputedStyle(overlay).zIndex)>=10000,centerError:rect.left+rect.width/2-(frame.left+frame.width/2),rect:{left:rect.left,width:rect.width},scrolling:getComputedStyle(page).overflow!=='hidden'||page.scrollHeight>page.clientHeight+1,leafFits:leaf&&leaf.scrollHeight<=leaf.clientHeight+1,pageCount:document.querySelector('#manual-page-count').textContent,index:Array.from(document.querySelectorAll('.manual-contents li b')).map(function(e){return e.textContent})};})()");
  assert.equal(book.open,true,'deep chapter opens as the physical manual');
  assert.equal(book.front,true,'the open book sits in front of the cabinet');
  assert.ok(Math.abs(book.centerError)<2,`the opened manual is centered in front of the player: ${JSON.stringify(book.rect)}`);
  assert.equal(book.scrolling,false,'manual pages have no scroll area');
  assert.equal(book.leafFits,true,'first printed leaf fits its paper without clipping');
    assert.ok(book.index.length>=3,'printed contents include the chapter page entries');
  assert.equal(book.index[0],'01','index shows the actual first page number for About');
  const total=Number(book.pageCount.split('/')[1]);
  assert.ok(total>3,'long chapters are laid out as multiple real pages');
  await mkdir('/tmp/blip-details',{recursive:true});
  try{const shot=await cdp.send('Page.captureScreenshot',{format:'png',fromSurface:false});await writeFile('/tmp/blip-details/manual.png',Buffer.from(shot.data,'base64'));}catch(error){console.log(`manual screenshot: ${error.message}`);}
  let portraitsMatch=false;
  for(let page=1;page<total;page++) {
    await evaluate(cdp,"document.querySelector('#manual-next').click();true");await sleep(80);
    const state=await evaluate(cdp,"(function(){var content=document.querySelector('.manual-page-content'),leaf=content.querySelector('.manual-leaf-sheet'),imgs=Array.from(content.querySelectorAll('.profile-avatar'));return {fits:!!leaf&&leaf.scrollHeight<=leaf.clientHeight+1&&getComputedStyle(content).overflow==='hidden',pictures:imgs.map(function(e){var r=e.getBoundingClientRect();return [e.offsetWidth,e.offsetHeight,getComputedStyle(e).filter]})};})()");
    assert.equal(state.fits,true,`printed manual leaf ${page+1} fits without scrolling or clipping`);
    if(state.pictures.length)portraitsMatch=state.pictures.every(function(p){return p[0]===52&&p[1]===52&&p[2].includes('grayscale(1)')});
  }
  assert.equal(portraitsMatch,true,'operator portraits share a monochrome printed size');
  await evaluate(cdp,"document.querySelector('#manual-close').click();true");
  console.log(`Field Manual: ${total} typeset leaves, no clipped text or scrolling`);
}finally {
  await close();
}
