# Per-game high scores (Supabase free tier)

Status: **live** — schema is on a free-tier hosted Supabase project,
anonymous sign-ins are on, and the full flow (claim handle, submit,
leaderboard read, recovery codes, RLS) is verified end-to-end. The web
client is wired; set the `SUPABASE_URL` / `SUPABASE_ANON_KEY` repo secrets
so the Pages deploy picks them up (see [Deploy](#deploy)). The project ref
and anon key are never committed to this repo.

## Goal

A shared high-score table for each of the six cabinet games, shown on the
game-over screen. A player picks a handle **once**; nobody else can post a
score under that handle. No password, no "create an account" wall — the
first play is friction-free.

Non-goals: anti-cheat beyond casual plausibility limits, cross-game
aggregate ranking, seasons/resets (all easy to add later).

## How the "no login" identity works

Supabase **anonymous sign-in**. On first load `blip_scores.js` calls
`auth.signInAnonymously()` — no UI. That mints a real Supabase user with a
stable `auth.uid()` and a JWT that lives in `localStorage`
(`blip-sb-auth`). Every subsequent visit re-uses it.

The handle is protected by two database facts, not by a login:

1. `players.handle` has a **`UNIQUE` constraint** (case-insensitive). Once
   `RETROJACK` is claimed, no other user — anonymous or not — can insert
   or rename to it.
2. Row-level security ties every write to the caller's token:
   `players` rows can only be inserted/updated where `id = auth.uid()`,
   and `scores` rows only where `player_id = auth.uid()`.

So to post a score as `RETROJACK` you would need `RETROJACK`'s JWT, which
never leaves their browser. Clearing browser storage would normally lose
the handle — recovery codes are the fix.

### Recovery codes — move a name to another device

No email, no account. The first time a player claims a handle,
`claim_handle` mints a one-time **recovery code** (e.g. `RTJ4-K9MC-P2XW`)
and returns it once; only a SHA-256 hash is stored (`players.recovery_hash`,
unique index). The game-over panel shows it under **SAVE THIS CODE** with a
COPY button.

On a new device the player taps **RESTORE**, enters the code, and
`restore_handle(code)`:

1. looks the code hash up (indexed, O(1)),
2. deletes the caller's throwaway `players` row (cascading its scores),
3. `UPDATE players SET id = <caller uid>` on the matched row — the handle
   *and its scores* follow via `scores_player_id_fkey ON UPDATE CASCADE`.

The old anonymous user is left orphaned and gets pruned by the
`blip-prune-anon-users` cron job. Brute force is bounded by
`restore_attempts` (≤ 10/hour/caller) on top of GoTrue's anon-signup limit;
against a ~59-bit code that's ~10¹⁵ years.

`new_recovery_code()` rotates the code (the **MY CODE** button), which also
back-fills one for handles claimed before this feature existed.

## Data model

`supabase/migrations/` — `*_highscores.sql` then `*_recovery_codes.sql`

```
players
  id             uuid  PK  -> auth.users(id) on delete cascade  (moved by restore_handle)
  handle         text  NOT NULL  CHECK (handle ~ '^[A-Za-z0-9 _-]{2,14}$')
                 UNIQUE on lower(handle)
  recovery_hash  text  UNIQUE (partial, where not null)  -- sha256 of the recovery code
  created_at     timestamptz default now()

scores
  id          bigint identity PK
  game        text  NOT NULL  CHECK (game IN ('serpent','bouncer','galactic_defender','meteors','sky_raider'))
  player_id   uuid  NOT NULL  -> players(id) ON UPDATE CASCADE ON DELETE CASCADE
  score       integer NOT NULL CHECK (score >= 0 AND score <= 10000000)
  created_at / updated_at  timestamptz default now()
  UNIQUE (game, player_id)          -- one row per player per game: keep only their best

restore_attempts
  id  bigint identity PK,  uid uuid,  at timestamptz    -- restore_handle() brute-force guard
```

`rally` is excluded on purpose — it is first-to-N two-player pong, there
is no single score to rank.

### Reads — public, via a view

```
leaderboard_top  (security_invoker view)
  game, handle, score, created_at, rank  -- rank() over (partition by game order by score desc)
```

RLS: `scores` and `players` grant `SELECT` to `anon`; the view is how the
client reads. `GET /rest/v1/leaderboard_top?game=eq.serpent&limit=10`.

### Writes — through RPCs

All `security definer`, `search_path = ''`, granted to `anon` +
`authenticated`:

`submit_score(p_game text, p_score int)`:

* resolves `auth.uid()` (401 if somehow absent),
* requires a claimed handle (returns `needs_handle` so the client knows to prompt),
* `INSERT ... ON CONFLICT (game, player_id) DO UPDATE ... WHERE excluded.score > scores.score` — only improves,
* rate-limits: rejects if the player has written > 30 score rows in the last minute,
* returns the player's new rank + the top 10 as JSON, so game-over needs one round trip.

`claim_handle(p_handle text)` — inserts the `players` row for `auth.uid()`,
or renames it; maps the unique-violation to `{ ok:false, reason:'taken' }`.
On a first claim (or a null-hash backfill) it also mints a recovery code
and returns `recovery_code`.

`new_recovery_code()` — rotates the caller's recovery code, returns it.

`restore_handle(p_code text)` — moves the handle + scores that the code
unlocks onto the caller (see [Recovery codes](#recovery-codes--move-a-name-to-another-device)).

The client calls these directly through PostgREST
(`supabase.rpc('submit_score', …)`) — `SECURITY DEFINER` + RLS is the
whole enforcement boundary, so no Edge Function is needed and the deploy
is just `db push`.

### Edge Function (optional, not implemented)

If you later want per-request checks the database can't easily do
(HMAC-signed run tokens, IP-based limits, a Turnstile check), add
`supabase/functions/submit-score/` as a thin wrapper that verifies the
JWT and calls the RPC, then point `blip_scores.js` at
`/functions/v1/submit-score` instead of `.rpc()`. Free tier: 500K
invocations/mo.

## Client integration

| File | Role |
|---|---|
| `web/vendor/supabase.js` | vendored `@supabase/supabase-js` UMD build (CSP + offline SW forbid a CDN) |
| `web/blip_config.js` | `window.BLIP_SUPABASE = { url, anonKey }` — **git-ignored**; the Pages deploy writes it from the `SUPABASE_URL` / `SUPABASE_ANON_KEY` repo secrets (`.github/workflows/pages.yml`). Blank / absent ⇒ feature disabled. |
| `web/blip_scores.js` | `window.blipScores`: `onGameOver(game, score)`, `promptHandle()`, `promptRestore()`, `showMyCode()`, `dismissBoard()` |
| `web/blip_config.example.js` | template |

Flow on game over:

1. WASM enters `State::Over` → `blip::web::report_score(session.score)` →
   `blip_game_over(score)` FFI → `window.blipGameOver(score)` in `shell.js`.
2. `blipScores.onGameOver(score)`:
   * if not configured → just update the local best (`localStorage
     blip-best-<game>`) and draw nothing new.
   * ensure anon session, then `submit_score`.
   * `needs_handle` → show the name panel; on a first claim, then show
     **SAVE THIS CODE** (the recovery code) before continuing.
   * render the score board — an HTML overlay above the canvas, the
     player's row highlighted, with **MY CODE** / **RESTORE** buttons.
3. Any d-pad / button press or tap dismisses the board, same as the
   WASM game-over screen's "press to continue".

Degrades safely: offline, project paused, RLS error, network failure —
all fall through to local-best-only and log a warning. The games never
block on the network.

### FFI additions

* `crates/blip/src/web.rs` — `extern "C" { fn blip_game_over(score: i32); }` + `pub fn report_score(score: i32)`
* `web/blip_bridge.js` — `importObject.env.blip_game_over`
* `web/shell.js` — `window.blipGameOver`
* each game's `main.rs` — one line at the `LifeResult::GameOver` transition:
  `blip::web::report_score(g.sess.score);`

### Service worker

`web/sw.js` — add `vendor/supabase.js`, `blip_scores.js`, `blip_config.js`
to `ASSETS` and bump `CACHE`. Supabase calls are cross-origin so the
existing fetch handler already passes them straight to the network.

## Free-tier operations

* **7-day pause.** A free project sleeps after a week with no API
  traffic. `.github/workflows/keep-warm.yml` pings
  `/rest/v1/leaderboard_top?limit=1` daily. (Cold resume is ~1 min if it
  ever does pause.)
* **Anonymous user build-up.** Anon users count toward the 50K MAU cap
  and pile up in `auth.users`. `pg_cron` job in the migration:
  daily `DELETE FROM auth.users WHERE is_anonymous AND created_at <
  now() - interval '30 days' AND id NOT IN (SELECT id FROM players)`.
* **Size.** 500 MB DB, ~2 rows/player. Fine for a hobby arcade.
* **Backups.** `supabase db dump` in the keep-warm workflow, artifact
  retention 30 days.

## Deploy

```bash
# one time — create the project (free tier) then:
npx supabase login                       # opens browser for an access token
npx supabase link --project-ref <ref>    # <ref> is in the dashboard URL
npx supabase db push                     # applies supabase/migrations/*
```

Turn **Anonymous sign-ins** on in Dashboard > Authentication > Sign In /
Providers (or `supabase config push` — but review `supabase config diff`
first, the generated `config.toml` also carries localhost defaults).

Then add the project's **URL** and **anon key** (Dashboard > Project
Settings > API) as GitHub **repo secrets** `SUPABASE_URL` and
`SUPABASE_ANON_KEY` (Settings > Secrets and variables > Actions). The
Pages workflow writes `web/blip_config.js` from them at deploy — the key
is never committed. Bump `CACHE` in `web/sw.js` when shipping client
changes.

No email/SMTP config is needed — identity is anonymous + recovery codes.

### What the anon key exposes

Keeping it out of the repo stops fork/clone reuse, but the deployed page
still serves it to every visitor (view-source) — unavoidable for a
browser Supabase client. That's acceptable because **every write goes
through `SECURITY DEFINER` + RLS**: a copied key cannot read anything the
leaderboard doesn't already show, nor post a score under a handle it
doesn't hold. The residual risk is nuisance traffic — junk handles/scores
and burning the 50 K free-tier MAU with throwaway anon users (rate-limited
to 30/hour/IP). If that ever bites, add the optional
[Edge Function](#edge-function-optional-not-implemented) with a Turnstile
or HMAC run-token check and point the client at it instead of `.rpc()`.

## Local development

```bash
npx supabase start        # Postgres + Auth + REST + Edge runtime in Docker
npx supabase db reset     # apply migrations + seed
node test/highscores.mjs  # end-to-end: anon sign-in, claim, submit, read
```

`supabase/seed.sql` inserts a few fake players/scores so the board is not
empty in dev.

### In-game high-score display

Each game shows the current record and its holder:

* **HUD** (during play): a centred `HI 180940 JACOB` between SCORE and
  LIVES, the name in a fluorescent glow — `Blip::draw_hud` in
  `crates/blip/src/ctx.rs`. It sizes itself to the clear gap between the
  SCORE and LIVES clusters.
* **Title / game-over screens**: `HI 180940 JACOB` and
  `BEST 180940 JACOB` / `NEW BEST!`.

The value comes from `blip::web::high_score() -> HighScore { score, name }`
(FFI `blip_high_score` + `blip_high_name`), which the shell resolves from
`localStorage['blip-top-<slug>']` (the cached leaderboard #1), falling back
to this browser's `blip-best-<slug>` + `blip-handle`.

On native builds `high_score()` is normally empty; set
`BLIP_HI=180940` or `BLIP_HI=180940:JACOB` to fake a record for a
screenshot.
