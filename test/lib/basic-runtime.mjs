/** Replace BASIC's audio callback in tests that use tones as a trace channel. */
export async function instrumentBasicSound(page, callbackSource) {
  await page.route('**/basic-runtime.html*', async (route) => {
    const response = await route.fetch();
    const runtime = await response.text();
    const marker = 'imports.env.blip_basic_sound=playBasicTone;';
    if (!runtime.includes(marker)) throw new Error('BASIC runtime sound hook was not found');
    await route.fulfill({ response, body: runtime.replace(marker, `imports.env.blip_basic_sound=${callbackSource};`) });
  });
}
