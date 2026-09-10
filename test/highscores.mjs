/* End-to-end check of the high-score backend against a local
 * `supabase start` stack. Not part of `npm test` (needs Docker + the
 * stack running). Run:
 *
 *   npx supabase start
 *   node test/highscores.mjs        # reads the URL + anon key from `supabase status`
 *
 * Or point it elsewhere with SUPABASE_URL / SUPABASE_ANON_KEY.
 */
import { createClient } from '@supabase/supabase-js';
import assert from 'node:assert/strict';
import { execSync } from 'node:child_process';

// Pull the local stack's URL + anon key from the CLI rather than hard-coding
// the well-known demo JWT (which secret scanners flag).
function fromSupabaseStatus() {
  try {
    const out = execSync('npx --no-install supabase status -o env', {
      encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'],
    });
    const get = (k) => (out.match(new RegExp('^' + k + '="?([^"\\n]+)', 'm')) || [])[1];
    return { url: get('API_URL'), anon: get('ANON_KEY') };
  } catch { return {}; }
}

const local = fromSupabaseStatus();
const URL = process.env.SUPABASE_URL || local.url || 'http://127.0.0.1:54321';
const ANON = process.env.SUPABASE_ANON_KEY || local.anon;

if (!ANON) {
  console.error('No anon key — run `npx supabase start` first, or set SUPABASE_ANON_KEY.');
  process.exit(2);
}

const rnd = () => 'T' + Math.random().toString(36).slice(2, 8).toUpperCase();
const fresh = () => createClient(URL, ANON, { auth: { persistSession: false } });

let failed = 0;
async function step(name, fn) {
  try { await fn(); console.log('  ok  ', name); }
  catch (e) { failed++; console.error('  FAIL', name, '\n       ', e.message); }
}

// -- player A: anonymous sign-in, claim a handle, submit a score ----------
const a = fresh();
const handleA = rnd();
let scoreRes;

await step('anonymous sign-in', async () => {
  const { data, error } = await a.auth.signInAnonymously();
  assert.equal(error, null);
  assert.ok(data.session.access_token);
});

await step('submit before claiming a handle -> needs_handle', async () => {
  const { data, error } = await a.rpc('submit_score', { p_game: 'serpent', p_score: 1234 });
  assert.equal(error, null);
  assert.equal(data.reason, 'needs_handle');
});

let codeA;
await step('claim_handle succeeds and mints a recovery code', async () => {
  const { data } = await a.rpc('claim_handle', { p_handle: handleA });
  assert.equal(data.ok, true, JSON.stringify(data));
  assert.equal(data.handle, handleA);
  assert.match(data.recovery_code || '', /^[2-9A-HJKMNP-Z]{4}-[2-9A-HJKMNP-Z]{4}-[2-9A-HJKMNP-Z]{4}$/);
  codeA = data.recovery_code;
});

await step('re-claiming the same handle does not re-issue a code', async () => {
  const { data } = await a.rpc('claim_handle', { p_handle: handleA });
  assert.equal(data.ok, true);
  assert.equal(data.recovery_code, undefined);
});

await step('submit_score returns rank + board', async () => {
  const { data, error } = await a.rpc('submit_score', { p_game: 'serpent', p_score: 999999 });
  assert.equal(error, null);
  assert.equal(data.ok, true, JSON.stringify(data));
  assert.equal(data.best, 999999);
  assert.equal(data.rank, 1);
  assert.ok(Array.isArray(data.board) && data.board.length >= 1);
  assert.equal(data.board[0].handle, handleA);
  scoreRes = data;
});

await step('a lower score does not replace the best', async () => {
  const { data } = await a.rpc('submit_score', { p_game: 'serpent', p_score: 5 });
  assert.equal(data.best, 999999);
});

// -- player B: a different anon user cannot steal handle A ---------------
const b = fresh();
await step('second anon user', async () => {
  const { error } = await b.auth.signInAnonymously();
  assert.equal(error, null);
});

await step('claiming a taken handle -> {ok:false, reason:"taken"}', async () => {
  const { data } = await b.rpc('claim_handle', { p_handle: handleA.toLowerCase() });
  assert.equal(data.ok, false);
  assert.equal(data.reason, 'taken');
});

await step('B claims its own handle and appears below A', async () => {
  const hb = rnd();
  await b.rpc('claim_handle', { p_handle: hb });
  const { data } = await b.rpc('submit_score', { p_game: 'serpent', p_score: 500000 });
  assert.equal(data.ok, true);
  assert.ok(data.rank >= 2);
});

// -- direct table writes are refused (RLS) ------------------------------
await step('direct INSERT into scores is blocked by RLS', async () => {
  const { data: u } = await a.auth.getUser();
  const { error } = await a.from('scores')
    .insert({ game: 'serpent', player_id: u.user.id, score: 10000000 });
  assert.ok(error, 'expected an RLS error');
});

await step('cannot rename over a taken handle', async () => {
  const { data } = await b.rpc('claim_handle', { p_handle: handleA });
  assert.equal(data.ok, false);
  assert.equal(data.reason, 'taken');
});

// -- public read shape -------------------------------------------------
await step('leaderboard_top is publicly readable and ranked', async () => {
  const pub = fresh();
  const { data, error } = await pub
    .from('leaderboard_top').select('handle,score,rank')
    .eq('game', 'serpent').order('rank').limit(5);
  assert.equal(error, null);
  assert.ok(data.length >= 2);
  assert.equal(data[0].rank, 1);
  assert.ok(data[0].score >= data[1].score);
});

await step('bad game name is rejected', async () => {
  const { data } = await a.rpc('submit_score', { p_game: 'pong', p_score: 1 });
  assert.equal(data.ok, false);
  assert.equal(data.reason, 'bad_game');
});

// -- recovery codes: player C restores handle A onto a fresh anon user ---
const c = fresh();
await step('third anon user', async () => {
  const { error } = await c.auth.signInAnonymously();
  assert.equal(error, null);
});

await step('restore_handle rejects a wrong code', async () => {
  const { data } = await c.rpc('restore_handle', { p_code: 'XXXX-XXXX-XXXX' });
  assert.equal(data.ok, false);
  assert.equal(data.reason, 'bad_code');
});

await step('restore_handle moves handle A (loose typing ok)', async () => {
  const loose = codeA.toLowerCase().replace(/-/g, ' ');
  const { data } = await c.rpc('restore_handle', { p_code: loose });
  assert.equal(data.ok, true, JSON.stringify(data));
  assert.equal(data.handle, handleA);
});

await step("A's score followed the handle to C", async () => {
  const { data } = await c.rpc('submit_score', { p_game: 'serpent', p_score: 1 });
  assert.equal(data.ok, true);
  assert.equal(data.best, 999999);          // kept — not overwritten by the 1
});

await step('the original user A can no longer post as handle A', async () => {
  const { data } = await a.rpc('submit_score', { p_game: 'serpent', p_score: 7 });
  assert.equal(data.reason, 'needs_handle');   // A's players row was moved away
});

await step('new_recovery_code rotates and invalidates the old one', async () => {
  const { data: rot } = await c.rpc('new_recovery_code');
  assert.equal(rot.ok, true);
  assert.notEqual(rot.recovery_code, codeA);
  const { data: old } = await c.rpc('restore_handle', { p_code: codeA });
  assert.equal(old.reason, 'bad_code');
});

console.log(failed ? `\n${failed} check(s) failed` : '\nall checks passed');
process.exit(failed ? 1 : 0);
