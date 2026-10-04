import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {readFile,writeFile} from 'node:fs/promises';
import {mkdtempSync,rmSync} from 'node:fs';
import {launch,evaluate,killAll,sleep} from './lib/cdp.mjs';
const server=createServer(async(req,res)=>{try{const file='web'+req.url.split('?')[0],bytes=await readFile(file);res.setHeader('Content-Type',file.endsWith('.html')?'text/html':file.endsWith('.css')?'text/css':file.endsWith('.js')?'text/javascript':file.endsWith('.svg')?'image/svg+xml':'image/png');res.end(bytes);}catch{res.statusCode=404;res.end();}});
await new Promise(resolve=>server.listen(0,resolve));
const profile=mkdtempSync('/tmp/blip-rom-physics-'),{proc,cdp}=await launch(18000+Math.floor(Math.random()*10000),[`--user-data-dir=${profile}`]);
try {
 for(const width of [1280,390,320]) {
  await cdp.send('Emulation.setDeviceMetricsOverride',{width,height:844,deviceScaleFactor:1,mobile:width<700});
  await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:width<700});
  await cdp.send('Page.navigate',{url:`http://127.0.0.1:${server.address().port}/index.html`});await sleep(1400);
  for(const returning of [false,true]) {
   await evaluate(cdp,`(()=>{const rom=document.querySelector('.card-focused .game-rom'),r=rom.getBoundingClientRect(),receiver=document.querySelector('.jukebox-receiver');receiver.style.transform='translate(-50%,0px)';receiver.style.clipPath='';receiver.classList.remove('stowed');const slot=receiver.querySelector('.jukebox-receiver-lip').getBoundingClientRect();window.physicsOptions={returning:${returning},original:rom,width:rom.offsetWidth,height:rom.offsetHeight,scale:r.height/rom.offsetHeight,source:{x:r.left+r.width/2,y:r.top+r.height/2},pivot:blipCardArmPivot(receiver),slot:{x:slot.left+slot.width/2,y:slot.top+slot.height/2},receiver,onPickup:()=>rom.closest('.card').classList.add('card-loaded'),onReturned:()=>rom.closest('.card').classList.remove('card-loaded')};window.physicsStage=blipTransferCard(physicsOptions);physicsStage.blipSeek(0);return true;})()`);
   const result=await evaluate(cdp,`(()=>{
    function overlap(a,b){for(const p of [a,b])for(let i=0;i<p.length;i++){const u=p[i],v=p[(i+1)%p.length],n={x:-(v.y-u.y),y:v.x-u.x},aa=a.map(q=>q.x*n.x+q.y*n.y),bb=b.map(q=>q.x*n.x+q.y*n.y);if(Math.max(...aa)<=Math.min(...bb)||Math.max(...bb)<=Math.min(...aa))return false;}return true;}
    const o=physicsOptions,stage=physicsStage,box=document.querySelector('.screen-bezel').getBoundingClientRect(),fascia=document.querySelector('.top-marquee-bar').getBoundingClientRect().bottom,failures=[];
    if(stage.querySelectorAll('[data-joint]').length!==2)failures.push({issue:'requires two driven rotary joints'});
    stage.blipSeek(0);const low=parseFloat(stage.querySelector('.jukebox-arm-pivot').style.top);
    stage.blipSeek(2100);const high=parseFloat(stage.querySelector('.jukebox-arm-pivot').style.top);
    if(Math.abs(low-high-12)>.01)failures.push({issue:'shoulder fails to elevate to ROM plane'});
    for(let ms=0;ms<=(${returning}?12300:14700);ms+=50){
     stage.blipSeek(ms);const wrist=stage.querySelector('.jukebox-arm-wrist'),card=wrist.querySelector('.jukebox-flight-card'),angle=parseFloat(wrist.style.transform.match(/[-.\\d]+/)[0])*Math.PI/180,x=parseFloat(wrist.style.left),y=parseFloat(wrist.style.top),feed=parseFloat(card.style.clipPath.match(/[.\\d]+/)[0])/100,w=o.width*o.scale,h=o.height*o.scale;
     const rom=[[-w/2,-h*(1-feed)],[w/2,-h*(1-feed)],[w/2,0],[-w/2,0]].map(([a,b])=>({x:x+a*Math.cos(angle)-b*Math.sin(angle),y:y+a*Math.sin(angle)+b*Math.cos(angle)}));
     const elbow=stage.querySelector('.jukebox-arm-elbow'),ex=parseFloat(elbow.style.left),ey=parseFloat(elbow.style.top);
     if(+getComputedStyle(wrist).zIndex<=+getComputedStyle(elbow).zIndex)failures.push({ms,issue:'ROM is below arm layer'});
     for(const selector of ['.jukebox-arm-pivot','.jukebox-arm-elbow','.jukebox-arm-bearing']){
      const joint=stage.querySelector(selector),cx=parseFloat(joint.style.left),cy=parseFloat(joint.style.top),r=joint.offsetWidth/2;
      const outline=Array.from({length:16},(_,i)=>({x:cx+r*Math.cos(i*Math.PI/8),y:cy+r*Math.sin(i*Math.PI/8)}));
      if(card.style.opacity!=='0'&&h*(1-feed)>1&&overlap(outline,rom))failures.push({ms,issue:'ROM hides '+selector});
      if(ms>=2100&&ms<9600&&(cy-r<fascia||cx-r<box.left||cx+r>box.right))failures.push({ms,issue:'joint is clipped',selector});
     }
     if(ey<fascia+15||ex<box.left+15||ex>box.right-15)failures.push({ms,issue:'elbow leaves cabinet',ex,ey});
     for(const selector of ['.jukebox-arm-shaft','.jukebox-arm-forearm','.jukebox-arm-tool']){
      const part=stage.querySelector(selector),px=parseFloat(part.style.left),py=parseFloat(part.style.top),length=parseFloat(part.style.width),radius=selector.endsWith('tool')?4:12,a=parseFloat(part.style.transform.match(/[-.\\d]+/)[0]);
      const poly=[[0,-radius],[length,-radius],[length,radius],[0,radius]].map(([u,v])=>({x:px+u*Math.cos(a)-v*Math.sin(a),y:py+u*Math.sin(a)+v*Math.cos(a)}));
      if(card.style.opacity!=='0'&&h*(1-feed)>1&&overlap(poly,rom))failures.push({ms,issue:'ROM intersects '+selector});
      if(selector!=='.jukebox-arm-tool'&&poly.some(p=>p.x<box.left||p.x>box.right||p.y<fascia))failures.push({ms,issue:'link leaves cabinet',selector});
     }
     const fore=stage.querySelector('.jukebox-arm-forearm'),fa=parseFloat(fore.style.transform.match(/[-.\\d]+/)[0]),fl=parseFloat(fore.style.width),fx=parseFloat(fore.style.left),fy=parseFloat(fore.style.top);
     const shoulder=stage.querySelector('.jukebox-arm-pivot'),sx=parseFloat(shoulder.style.left),sy=parseFloat(shoulder.style.top);
     if(Math.abs(Math.hypot(ex-sx,ey-sy)-parseFloat(stage.querySelector('.jukebox-arm-shaft').style.width))>.1||Math.hypot(fx-ex,fy-ey)>.1)failures.push({ms,issue:'detached joint'});
     for(const collar of stage.querySelectorAll('.jukebox-joint-lift'))if(Math.abs(parseFloat(collar.style.getPropertyValue('--joint-rise'))-12*Number(stage.querySelector('.articulated').dataset.elevation))>.02)failures.push({ms,issue:'joint collar out of sync'});
    }
    return failures.slice(0,12);
   })()`);
   assert.deepEqual(result,[],`${width}px ${returning?'return':'insert'} physics: ${JSON.stringify(result)}`);
   if(process.env.BLIP_ROM_SCREENSHOTS&&!returning){await evaluate(cdp,'physicsStage.blipSeek(5200);true');await sleep(100);const shot=await cdp.send('Page.captureScreenshot',{format:'png',fromSurface:true});await writeFile(`/tmp/blip-rom-physics-${width}.png`,Buffer.from(shot.data,'base64'));}
   await evaluate(cdp,'physicsStage.remove();true');
  }
  console.log(`${width}px: insertion and return sampled every 50 ms; ROM above linkage, all joints visible, no collisions or detached joints`);
 }
}finally{await cdp.send('Browser.close').catch(()=>{});cdp.close();killAll([proc]);server.close();rmSync(profile,{recursive:true,force:true,maxRetries:5,retryDelay:200});}
