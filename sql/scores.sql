create table scores (
      id         bigserial primary key,
      game       text not null,
      initials   text not null check (char_length(initials) between 1 and 3),
      score      int  not null check (score > 0),
      created_at timestamptz default now()
  );
  create index on scores (game, score desc);
  alter table scores enable row level security;
  create policy "read all" on scores for select using (true);

  create or replace function submit_score(p_game text, p_initials text, p_score int)
  returns void language plpgsql security definer as $$
  declare v_count bigint; v_min int;
  begin
      select count(*), min(score) into v_count, v_min
      from (select score from scores where game = p_game order by score desc limit 10) t;
      if v_count < 10 or p_score > v_min then
          insert into scores (game, initials, score)
          values (p_game, upper(substr(p_initials, 1, 3)), p_score);
      end if;
  end;
  $$;
  grant execute on function submit_score(text, text, int) to anon;

