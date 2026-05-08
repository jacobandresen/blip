create table hi_scores (
      game        text primary key,
      score       int  not null default 0,
      updated_at  timestamptz default now()
  );

  -- Pre-seed all four games so reads always return a row
  insert into hi_scores (game, score) values
      ('bouncer',           0),
      ('serpent',           0),
      ('galactic_defender', 0),
      ('canaris',           0);

  -- RLS: public read, no direct writes (only via the function below)
  alter table hi_scores enable row level security;
  create policy "read all" on hi_scores for select using (true);

  -- Function enforces "only update if higher" — runs with table-owner rights
  create or replace function set_hi_score(p_game text, p_score int)
  returns void language plpgsql security definer as $$
  begin
      insert into hi_scores (game, score)
      values (p_game, p_score)
      on conflict (game) do update
          set score      = greatest(hi_scores.score, excluded.score),
              updated_at = now()
          where hi_scores.score < excluded.score;
  end;
  $$;

  -- Allow anon callers to invoke the function
  grant execute on function set_hi_score(text, int) to anon;
