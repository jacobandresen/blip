/* High-score Supabase config (see docs/highscores.md). The anon key is public; writes use RLS and SECURITY DEFINER functions.
 * Leave URL/key blank to disable remote scores; configure them from Project Settings > API and bump `CACHE` in `sw.js`.
 * For local Supabase, use the CLI URL/key while testing and don't commit that config. */
window.BLIP_SUPABASE = {
  url: '',
  anonKey: ''
};
