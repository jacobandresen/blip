-- Per-game high scores for the BLIP arcade.
--
-- Identity is a Supabase *anonymous* user (auth.uid()); a player claims a
-- handle exactly once and the UNIQUE index + RLS stop anyone else posting
-- under it. See docs/highscores.md.

-- ---------------------------------------------------------------------------
-- players: one row per (anonymous) user, holding their chosen handle
-- ---------------------------------------------------------------------------
create table public.players (
  id         uuid primary key references auth.users (id) on delete cascade,
  handle     text not null check (handle ~ '^[A-Za-z0-9 _-]{2,14}$'),
  created_at timestamptz not null default now()
);

-- case-insensitive uniqueness without the citext extension
create unique index players_handle_lower_key on public.players (lower(handle));

alter table public.players enable row level security;

-- anyone (incl. the anon API key) may read handles — the leaderboard needs them
create policy players_read_all on public.players
  for select using (true);

-- you may only create / rename your own row
create policy players_insert_self on public.players
  for insert with check (id = (select auth.uid()));
create policy players_update_self on public.players
  for update using (id = (select auth.uid()))
             with check (id = (select auth.uid()));

-- ---------------------------------------------------------------------------
-- scores: best score per player per game
-- ---------------------------------------------------------------------------
create table public.scores (
  id         bigint generated always as identity primary key,
  game       text not null
             check (game in ('serpent','bouncer','galactic_defender','meteors','sky_raider')),
  player_id  uuid not null references public.players (id) on delete cascade,
  score      integer not null check (score >= 0 and score <= 10000000),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (game, player_id)
);

create index scores_game_score_idx on public.scores (game, score desc);

alter table public.scores enable row level security;

-- public read
create policy scores_read_all on public.scores
  for select using (true);

-- Writes go exclusively through submit_score() (security definer). No
-- insert/update/delete policy => the anon / authenticated roles cannot
-- touch the table directly, only through the function.

-- ---------------------------------------------------------------------------
-- leaderboard_top: the public read shape (rank per game)
-- ---------------------------------------------------------------------------
create view public.leaderboard_top
with (security_invoker = true) as
  select s.game,
         p.handle,
         s.score,
         s.updated_at,
         rank() over (partition by s.game order by s.score desc, s.updated_at asc) as rank
  from public.scores s
  join public.players p on p.id = s.player_id;

grant select on public.leaderboard_top to anon, authenticated;

-- ---------------------------------------------------------------------------
-- claim_handle(handle) -> jsonb  { ok, reason?, handle? }
-- ---------------------------------------------------------------------------
create function public.claim_handle(p_handle text)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_uid  uuid := (select auth.uid());
  v_norm text := btrim(p_handle);
begin
  if v_uid is null then
    return jsonb_build_object('ok', false, 'reason', 'no_session');
  end if;
  if v_norm !~ '^[A-Za-z0-9 _-]{2,14}$' then
    return jsonb_build_object('ok', false, 'reason', 'invalid');
  end if;

  insert into public.players (id, handle) values (v_uid, v_norm)
  on conflict (id) do update set handle = excluded.handle;

  return jsonb_build_object('ok', true, 'handle', v_norm);
exception
  when unique_violation then
    return jsonb_build_object('ok', false, 'reason', 'taken');
end;
$$;

grant execute on function public.claim_handle(text) to anon, authenticated;

-- ---------------------------------------------------------------------------
-- submit_score(game, score) -> jsonb
--   { ok, reason?, rank?, best?, board:[{handle,score,rank}] }
-- ---------------------------------------------------------------------------
create function public.submit_score(p_game text, p_score integer)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_uid    uuid := (select auth.uid());
  v_handle text;
  v_recent int;
  v_best   int;
  v_rank   int;
  v_board  jsonb;
begin
  if v_uid is null then
    return jsonb_build_object('ok', false, 'reason', 'no_session');
  end if;
  if p_game not in ('serpent','bouncer','galactic_defender','meteors','sky_raider') then
    return jsonb_build_object('ok', false, 'reason', 'bad_game');
  end if;
  if p_score is null or p_score < 0 or p_score > 10000000 then
    return jsonb_build_object('ok', false, 'reason', 'bad_score');
  end if;

  select handle into v_handle from public.players where id = v_uid;
  if v_handle is null then
    return jsonb_build_object('ok', false, 'reason', 'needs_handle');
  end if;

  -- crude rate limit: at most 30 score writes / minute / player
  select count(*) into v_recent
  from public.scores
  where player_id = v_uid and updated_at > now() - interval '1 minute';
  if v_recent > 30 then
    return jsonb_build_object('ok', false, 'reason', 'rate_limited');
  end if;

  insert into public.scores (game, player_id, score)
  values (p_game, v_uid, p_score)
  on conflict (game, player_id) do update
    set score = excluded.score, updated_at = now()
    where excluded.score > public.scores.score;

  select score into v_best from public.scores where game = p_game and player_id = v_uid;

  -- same ranking the public board shows (dense-ish rank() with an
  -- updated_at tiebreaker), so "you're #N" matches the player's row
  select rank into v_rank
  from public.leaderboard_top
  where game = p_game and handle = v_handle;

  select jsonb_agg(row_to_json(t)) into v_board from (
    select handle, score, rank
    from public.leaderboard_top
    where game = p_game
    order by rank
    limit 10
  ) t;

  return jsonb_build_object(
    'ok', true, 'rank', v_rank, 'best', v_best,
    'board', coalesce(v_board, '[]'::jsonb)
  );
end;
$$;

grant execute on function public.submit_score(text, integer) to anon, authenticated;

-- ---------------------------------------------------------------------------
-- Housekeeping: drop stale anonymous users that never claimed a handle.
-- pg_cron is available on Supabase (hosted + local); harmless if the job
-- can't be scheduled — comment this block out if `create extension` fails
-- on your plan.
-- ---------------------------------------------------------------------------
create extension if not exists pg_cron with schema pg_catalog;

select cron.schedule(
  'blip-prune-anon-users',
  '17 4 * * *',
  $$ delete from auth.users u
     where u.is_anonymous
       and u.created_at < now() - interval '30 days'
       and not exists (select 1 from public.players p where p.id = u.id) $$
);
