-- Realtime Broadcast authorization for two-device Rally pairing
-- (docs/multiplayer.md). This is signaling only — the brief SDP
-- offer/answer + ICE candidate exchange needed before an RTCPeerConnection
-- can open its own direct DataChannel. No gameplay traffic ever touches
-- Supabase; once the DataChannel is open, this channel is abandoned.
--
-- No new tables: Realtime Broadcast is pure pub/sub over the built-in
-- `realtime.messages` table, gated by RLS the same as any other table.
-- Channels are used as *private* (`config: { private: true }` client-side)
-- so this RLS is actually enforced — a plain (non-private) channel would
-- bypass it under this project's "Allow public access" Realtime setting,
-- which is a project-wide toggle this migration shouldn't have to assume
-- one way or the other.
--
-- Scoped to the `blip-room-` topic prefix (see web/blip_net.js) rather
-- than opening every broadcast topic to anon — a room code is 4 digits
-- (10,000 possibilities) and the payload is single-use per match, so the
-- exposure here is "a stranger could guess a live room code and either
-- eavesdrop on or spoof someone else's pairing handshake", not anything
-- that reaches a database row or a player identity. Low stakes, still
-- scoped rather than left wide open.

create policy "anon can read blip-room broadcast"
on "realtime"."messages"
for select
to anon
using (
  realtime.messages.extension = 'broadcast'
  and (select realtime.topic()) like 'blip-room-%'
);

create policy "anon can send blip-room broadcast"
on "realtime"."messages"
for insert
to anon
with check (
  realtime.messages.extension = 'broadcast'
  and (select realtime.topic()) like 'blip-room-%'
);
