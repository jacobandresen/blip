-- Two-device Rally settled on QR-code-only signaling (docs/multiplayer.md)
-- — no network involved in pairing at all, so the Realtime Broadcast
-- relay from 20260911120000_multiplayer_signaling.sql is unused. Drop
-- the anon grant on `realtime.messages` rather than leave an unused
-- permission sitting on the project.

drop policy if exists "anon can read blip-room broadcast" on "realtime"."messages";
drop policy if exists "anon can send blip-room broadcast" on "realtime"."messages";
