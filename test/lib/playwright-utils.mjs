export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

export async function evaluate(handle, expression) {
  const page = handle.page || handle;
  return page.evaluate((source) => eval(source), expression);
}

export async function waitFor(handle, expression, timeoutMs = 15000, intervalMs = 100) {
  const start = Date.now();
  while (Date.now() - start <= timeoutMs) {
    const value = await evaluate(handle, expression);
    if (value) return value;
    await sleep(intervalMs);
  }
  throw new Error(`timed out waiting for truthy: ${expression}`);
}
