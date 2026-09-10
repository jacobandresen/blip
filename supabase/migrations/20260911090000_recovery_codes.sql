-- Recovery codes for the BLIP arcade leaderboard.
--
-- A player's identity is an anonymous Supabase user, so clearing browser
-- storage would normally lose their handle forever. Instead, the first
-- time someone claims a handle they are given a one-time recovery code
-- (e.g. RTJ4-K9MC-P2XW). `restore_handle(code)` re-binds that handle —
-- and all its scores — onto whatever anonymous user is calling. No email,
-- no SMTP, no account. See docs/highscores.md.

-- ---------------------------------------------------------------------------
-- players: hold only a hash of the recovery code
-- ---------------------------------------------------------------------------
alter table public.players add column if not exists recovery_hash text;

-- codes carry ~59 bits of entropy, so a plain SHA-256 (no per-row salt) is
-- fine and lets restore_handle() do an indexed lookup instead of scanning.
create unique index if not exists players_recovery_hash_key
  on public.players (recovery_hash) where recovery_hash is not null;

-- The handle + its scores must be able to move to a different user id.
alter table public.scores drop constraint if exists scores_player_id_fkey;
alter table public.scores
  add constraint scores_player_id_fkey
  foreign key (player_id) references public.players (id)
  on update cascade on delete cascade;

-- ---------------------------------------------------------------------------
-- restore_attempts: brute-force guard for restore_handle()
-- ---------------------------------------------------------------------------
create table if not exists public.restore_attempts (
  id  bigint generated always as identity primary key,
  uid uuid not null,
  at  timestamptz not null default now()
);
alter table public.restore_attempts enable row level security;
-- no policies + no grants: only restore_handle() (security definer) touches it
revoke all on table public.restore_attempts from anon, authenticated;
create index if not exists restore_attempts_uid_at_idx
  on public.restore_attempts (uid, at desc);

-- ---------------------------------------------------------------------------
-- helpers (internal — not granted to anon/authenticated)
-- ---------------------------------------------------------------------------

-- A readable 12-char code in three dash-separated groups. Alphabet omits
-- 0/1/I/L/O/U to avoid transcription mistakes.
create or replace function public.blip_new_recovery_code()
returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
  alphabet constant text := '23456789ABCDEFGHJKMNPQRSTVWXYZ';
  raw bytea := extensions.gen_random_bytes(12);
  s   text := '';
  i   int;
begin
  for i in 0..11 loop
    s := s || substr(alphabet, (get_byte(raw, i) % length(alphabet)) + 1, 1);
  end loop;
  return substr(s, 1, 4) || '-' || substr(s, 5, 4) || '-' || substr(s, 9, 4);
end;
$$;
revoke all on function public.blip_new_recovery_code() from public, anon, authenticated;

-- Uppercase, strip everything but [0-9A-Z] so a code types loosely.
create or replace function public.blip_norm_code(p text)
returns text
language sql
immutable
set search_path = ''
as $$ select upper(regexp_replace(coalesce(p, ''), '[^0-9A-Za-z]', '', 'g')) $$;
revoke all on function public.blip_norm_code(text) from public, anon, authenticated;

-- Deterministic hash stored in players.recovery_hash.
create or replace function public.blip_code_hash(p_code text)
returns text
language sql
immutable
set search_path = ''
as $$ select encode(extensions.digest(public.blip_norm_code(p_code), 'sha256'), 'hex') $$;
revoke all on function public.blip_code_hash(text) from public, anon, authenticated;

-- ---------------------------------------------------------------------------
-- claim_handle(handle) -> jsonb  { ok, reason?, handle?, recovery_code? }
--   recovery_code is returned only when one is freshly minted (first claim,
--   or a backfill for a row that predates this migration).
-- ---------------------------------------------------------------------------
create or replace function public.claim_handle(p_handle text)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_uid  uuid := (select auth.uid());
  v_norm text := btrim(p_handle);
  v_row  public.players%rowtype;
  v_code text;
begin
  if v_uid is null then
    return jsonb_build_object('ok', false, 'reason', 'no_session');
  end if;
  if v_norm !~ '^[A-Za-z0-9 _-]{2,14}$' then
    return jsonb_build_object('ok', false, 'reason', 'invalid');
  end if;

  select * into v_row from public.players where id = v_uid;

  if v_row.id is null then
    v_code := public.blip_new_recovery_code();
    insert into public.players (id, handle, recovery_hash)
      values (v_uid, v_norm, public.blip_code_hash(v_code));
    return jsonb_build_object('ok', true, 'handle', v_norm, 'recovery_code', v_code);
  end if;

  update public.players set handle = v_norm where id = v_uid;

  if v_row.recovery_hash is null then
    v_code := public.blip_new_recovery_code();
    update public.players set recovery_hash = public.blip_code_hash(v_code) where id = v_uid;
    return jsonb_build_object('ok', true, 'handle', v_norm, 'recovery_code', v_code);
  end if;

  return jsonb_build_object('ok', true, 'handle', v_norm);
exception
  when unique_violation then
    return jsonb_build_object('ok', false, 'reason', 'taken');
end;
$$;

grant execute on function public.claim_handle(text) to anon, authenticated;

-- ---------------------------------------------------------------------------
-- new_recovery_code() -> jsonb  { ok, reason?, recovery_code? }
--   Rotate (or first-time mint) the calling player's code — for a "show my
--   code again" button. Invalidates any previously issued code.
-- ---------------------------------------------------------------------------
create or replace function public.new_recovery_code()
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_uid  uuid := (select auth.uid());
  v_code text;
begin
  if v_uid is null then
    return jsonb_build_object('ok', false, 'reason', 'no_session');
  end if;
  if not exists (select 1 from public.players where id = v_uid) then
    return jsonb_build_object('ok', false, 'reason', 'needs_handle');
  end if;

  v_code := public.blip_new_recovery_code();
  update public.players set recovery_hash = public.blip_code_hash(v_code) where id = v_uid;
  return jsonb_build_object('ok', true, 'recovery_code', v_code);
end;
$$;

grant execute on function public.new_recovery_code() to anon, authenticated;

-- ---------------------------------------------------------------------------
-- restore_handle(code) -> jsonb  { ok, reason?, handle? }
--   Move the handle (and its scores) that the code unlocks onto the caller.
-- ---------------------------------------------------------------------------
create or replace function public.restore_handle(p_code text)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_uid    uuid := (select auth.uid());
  v_norm   text := public.blip_norm_code(p_code);
  v_target public.players%rowtype;
  v_recent int;
begin
  if v_uid is null then
    return jsonb_build_object('ok', false, 'reason', 'no_session');
  end if;
  if length(v_norm) < 8 then
    return jsonb_build_object('ok', false, 'reason', 'bad_code');
  end if;

  -- at most 10 attempts / hour / caller (each also needs a fresh anon
  -- sign-in, itself rate-limited by GoTrue)
  select count(*) into v_recent from public.restore_attempts
    where uid = v_uid and at > now() - interval '1 hour';
  if v_recent >= 10 then
    return jsonb_build_object('ok', false, 'reason', 'rate_limited');
  end if;
  insert into public.restore_attempts (uid) values (v_uid);

  select * into v_target from public.players
    where recovery_hash = public.blip_code_hash(v_norm);

  if v_target.id is null then
    return jsonb_build_object('ok', false, 'reason', 'bad_code');
  end if;
  if v_target.id = v_uid then
    return jsonb_build_object('ok', true, 'handle', v_target.handle);
  end if;

  -- drop the caller's throwaway row (cascades its scores), then move the
  -- recovered handle + scores onto the caller's uid
  delete from public.players where id = v_uid;
  update public.players set id = v_uid where id = v_target.id;

  return jsonb_build_object('ok', true, 'handle', v_target.handle);
end;
$$;

grant execute on function public.restore_handle(text) to anon, authenticated;

-- ---------------------------------------------------------------------------
-- housekeeping
-- ---------------------------------------------------------------------------
select cron.schedule(
  'blip-prune-restore-attempts',
  '23 4 * * *',
  $$ delete from public.restore_attempts where at < now() - interval '2 days' $$
);
