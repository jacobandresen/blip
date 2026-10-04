import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, writeFile } from 'node:fs/promises';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { launch, evaluate, killAll, sleep } from './lib/cdp.mjs';

const web = path.resolve('web');
const mime = { '.html':'text/html', '.css':'text/css', '.js':'application/javascript', '.svg':'image/svg+xml', '.png':'image/png' };
const server = createServer(async (req,res) => {
  try { const file=path.join(web,req.url.split('?')[0]),bytes=await readFile(file);res.writeHead(200,{'content-type':mime[path.extname(file)]||'application/octet-stream'});res.end(bytes); }
  catch { if(!res.headersSent)res.writeHead(404);res.end(); }
});
await new Promise(resolve=>server.listen(0,resolve));
const profile=mkdtempSync(path.join(tmpdir(),'blip-manual-'));
const {proc,cdp}=await launch(26000+Math.floor(Math.random()*12000),[`--user-data-dir=${profile}`]);
try {
  for(const [width,height] of [[1280,844],[390,844],[320,720]]) {
    await cdp.send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:width<700});
    await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:width<700});
    await cdp.send('Page.navigate',{url:`http://127.0.0.1:${server.address().port}/index.html?manual=about&size=${width}`});
    await sleep(1300);
    for(let attempt=0;attempt<40 && !(await evaluate(cdp,"!!document.querySelector('.manual-page-content .manual-leaf-sheet')"));attempt++)await sleep(100);
    const first=await evaluate(cdp,"(function(){var o=document.querySelector('#manual-overlay'),b=o.querySelector('.manual-open-book'),p=o.querySelector('.manual-page-content'),l=p.querySelector('.manual-leaf-sheet'),paper=document.querySelector('.manual-open-left'),range=document.createRange();range.selectNodeContents(paper);var text=Array.from(range.getClientRects()).filter(x=>x.width&&x.height),r=paper.getBoundingClientRect();return {open:!o.hidden,z:getComputedStyle(o).zIndex,centered:Math.abs(b.getBoundingClientRect().left+b.getBoundingClientRect().width/2-innerWidth/2)<2,count:document.querySelector('#manual-page-count').textContent,index:Array.from(document.querySelectorAll('.manual-contents li b')).map(e=>e.textContent),scroll:l?[p.scrollHeight,p.clientHeight,l.scrollHeight,l.clientHeight]:null,indexFits:text.every(x=>x.left>=r.left-1&&x.right<=r.right+1&&x.top>=r.top-1&&x.bottom<=r.bottom+1)};})()");
    assert.equal(first.open,true,`${width}px manual opens above the cabinet`);
    assert.ok(first.centered,`${width}px open book is centered`);
    assert.ok(first.scroll&&first.scroll[2]<=first.scroll[3]+1,`${width}px first paper leaf fits`);
    assert.equal(first.indexFits,true,`${width}px printed index stays inside its page margins`);
    const total=Number(first.count.split('/')[1]);
    assert.ok(total>3,`${width}px chapters are split into leaves: ${first.count}`);
    if(width===1280&&process.env.BLIP_MANUAL_SCREENSHOT){const screenshot=await cdp.send('Page.captureScreenshot',{format:'png',fromSurface:false});await writeFile('/tmp/blip-manual.png',Buffer.from(screenshot.data,'base64'));}
    let portraitsSeen=false;
    for(let index=1;index<=total;index++) {
      if(index>1){await evaluate(cdp,"document.querySelector('#manual-next').click();true");await sleep(100);}
      const page=await evaluate(cdp,"(function(){var p=document.querySelector('.manual-page-content'),l=p.querySelector('.manual-leaf-sheet'),r=l.getBoundingClientRect(),range=document.createRange();range.selectNodeContents(l);var text=Array.from(range.getClientRects()).filter(x=>x.width&&x.height);var imgs=Array.from(l.querySelectorAll('.profile-avatar')).map(x=>[x.offsetWidth,x.offsetHeight,getComputedStyle(x).filter]);return {scroll:[p.scrollHeight,p.clientHeight,l.scrollHeight,l.clientHeight],bounds:[r.top,r.bottom],textFits:text.every(x=>x.top>=r.top-1&&x.bottom<=r.bottom+1),images:imgs,title:document.querySelector('#manual-book-title').textContent};})()");
      assert.ok(page.scroll[2]<=page.scroll[3]+1,`${width}px leaf ${index}/${total} fits its paper (scroll ${page.scroll[2]}/${page.scroll[3]}, ${page.title})`);
      assert.equal(page.textFits,true,`${width}px all printed text on leaf ${index}/${total} stays inside the margins`);
      assert.equal(page.images.every(img=>img[0]===52&&img[1]===52&&img[2].includes('grayscale(1)')),true,`${width}px portraits on leaf ${index}/${total} share a black-and-white size`);
      if(page.images.length)portraitsSeen=true;
    }
    assert.ok(portraitsSeen,`${width}px Codex and the operators appear in the manual`);
    console.log(`${width}px: ${total} printed pages fit without scrolling or clipped text`);
  }
} finally {
  await cdp.send('Browser.close').catch(()=>{});cdp.close();killAll([proc]);server.close();rmSync(profile,{recursive:true,force:true,maxRetries:5,retryDelay:200});
}
