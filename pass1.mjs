import { serve, open, hold, tap, down, up, shot } from './play.mjs';
const server = await serve();
const { browser, page } = await open('brawler');
await tap(page,'Space',' '); await page.waitForTimeout(400);
await tap(page,'Space',' '); await page.waitForTimeout(2700);
// Walk into range and throw a jab; capture the frames around contact.
await hold(page,'ArrowRight','ArrowRight',900);
for (let i=0;i<6;i++){
  await tap(page,'Space',' ');
  await page.waitForTimeout(60);
  await shot(page,`p1-jab-${i}`);
  await page.waitForTimeout(140);
}
// Press punch repeatedly during recovery of a sweep — does anything come out?
await down(page,'ArrowDown','ArrowDown'); await tap(page,'KeyZ','z'); await up(page,'ArrowDown','ArrowDown');
for (let i=0;i<8;i++){ await tap(page,'Space',' '); await page.waitForTimeout(40); }
await shot(page,'p1-after-buffer-test');
await browser.close(); server.close();
