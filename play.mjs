import { chromium } from 'playwright';
import http from 'node:http'; import fs from 'node:fs'; import path from 'node:path';
const WEB = '/Users/jacob/Projects/blip/web', PORT = 8099;
const OUT = '/private/tmp/claude-501/-Users-jacob-Projects-blip/1c90f0d3-8b99-409a-8bb6-507dc80bb1f8/scratchpad/shots';
fs.mkdirSync(OUT, { recursive: true });
const TYPES = { '.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.css':'text/css','.png':'image/png','.json':'application/json','.svg':'image/svg+xml' };
export function serve() {
  const s = http.createServer((req,res)=>{ const p=path.join(WEB, decodeURIComponent(req.url.split('?')[0]));
    fs.readFile(p,(e,d)=>{ if(e){res.writeHead(404);res.end();return;} res.writeHead(200,{'Content-Type':TYPES[path.extname(p)]||'application/octet-stream'}); res.end(d); }); });
  return new Promise(r=>s.listen(PORT,'127.0.0.1',()=>r(s)));
}
export async function open(game,{width=900,height=700}={}) {
  const browser = await chromium.launch({ args:['--use-gl=swiftshader','--enable-unsafe-swiftshader','--disable-renderer-backgrounding','--disable-background-timer-throttling'] });
  const page = await browser.newPage({ viewport:{width,height} });
  page.on('pageerror', e=>console.log(`[${game} pageerror] ${e.message}`));
  await page.goto(`http://127.0.0.1:${PORT}/${game}/index.html`);
  await page.waitForFunction("document.readyState==='complete'");
  await page.waitForFunction("(function(){var c=document.getElementById('glcanvas');return !!(c&&c.width>0);})()");
  await page.evaluate("(function(){var o=document.getElementById('need-coin-overlay'); if(o&&o.classList.contains('visible'))o.click();})()");
  await page.waitForTimeout(1200);
  return { browser, page };
}
export const down = (page,c,k)=>page.evaluate(([c,k])=>document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keydown',{bubbles:true,cancelable:true,key:k,code:c})),[c,k]);
export const up = (page,c,k)=>page.evaluate(([c,k])=>document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keyup',{bubbles:true,cancelable:true,key:k,code:c})),[c,k]);
export async function hold(page,c,k,ms){ await down(page,c,k); await page.waitForTimeout(ms); await up(page,c,k); }
export const tap = (page,c,k)=>hold(page,c,k,70);
export const shot = async (page,n)=>{ const p=`${OUT}/${n}.png`; await page.screenshot({path:p}); return p; };
