-- Adder keeps a board: one player, one score per run. Same table and RPC as
-- the other games, with its slug added to the check and to the allowed list.

alter table public.scores drop constraint scores_game_check;
alter table public.scores add constraint scores_game_check
  check (game in ('serpent','bouncer','galactic_defender','meteors','sky_raider','bubbler','brawler','adder'));

create or replace function public.submit_score(p_game text, p_score integer)
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
  if p_game not in ('serpent','bouncer','galactic_defender','meteors','sky_raider','bubbler','brawler','adder') then
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
