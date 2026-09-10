-- Dev-only seed data so the game-over leaderboard isn't empty under
-- `supabase start`. Never runs against a linked project.
--
-- These insert fake auth.users directly (only possible locally).

do $$
declare
  ids uuid[] := array[
    gen_random_uuid(), gen_random_uuid(), gen_random_uuid(),
    gen_random_uuid(), gen_random_uuid(), gen_random_uuid()
  ];
  handles text[] := array['RETROJACK','PIXELPETE','QBERT','NOVA','ZAP','ADA'];
  games   text[] := array['serpent','bouncer','galactic_defender','meteors','sky_raider'];
  i int; g text;
begin
  for i in 1 .. array_length(ids, 1) loop
    insert into auth.users (id, aud, role, is_anonymous, created_at, updated_at,
                            instance_id, raw_app_meta_data, raw_user_meta_data)
    values (ids[i], 'authenticated', 'authenticated', true, now(), now(),
            '00000000-0000-0000-0000-000000000000', '{}', '{}')
    on conflict (id) do nothing;

    insert into public.players (id, handle) values (ids[i], handles[i])
    on conflict (id) do nothing;

    foreach g in array games loop
      insert into public.scores (game, player_id, score)
      values (g, ids[i], (random() * 50000)::int)
      on conflict (game, player_id) do nothing;
    end loop;
  end loop;
end $$;
