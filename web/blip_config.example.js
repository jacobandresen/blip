/* BLIP backend config — high-score leaderboard (see docs/highscores.md).
 *
 * The anon key is a PUBLIC key: it is meant to ship in the browser. All
 * writes go through row-level security + SECURITY DEFINER functions, so a
 * leaked anon key cannot forge a score under someone else's handle.
 *
 * Leave url/anonKey blank to disable the leaderboard entirely — the games
 * then keep a local-only best in localStorage and nothing hits the network.
 *
 * Fill these from: Supabase dashboard > Project Settings > API, then bump
 * CACHE in web/sw.js. For local `supabase start`, paste the CLI's
 * "API URL" and "anon key" here while testing (and don't commit that).
 */
window.BLIP_SUPABASE = {
  url: '',
  anonKey: ''
};
