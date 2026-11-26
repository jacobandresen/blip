# Fun log

What was tried to make the games more fun, what the measures said (`node tools/fun_report.mjs`, six seeds of 60 s
unless noted), and whether it stayed. The method is in [FUN.md](FUN.md). It is no longer kept up: a change's
measured reason goes in a comment beside the code.

## Takeaways

1. **Reward skill, not luck.** Streaks that random play never builds raised fun most: Raider's no-damage streak
   (skill 0.14 → 0.27, +6.4), Meteors' accuracy streak (+1.6), Defender's streak broken by a lost cannon (+2.0).
   Rewards random play can stumble into (Raider grazing, -2.3) lower it.
2. **Remove unfair hits.** The meter's frames found Raider's phantom hits (hits 8.5 → 2.8 a minute) and point-blank
   flak (→ 1.2, +5.8), and Warlock fireballs burning from a cell away. Look at a hit's frame before tuning anything.
3. **Tuning opponents does not help.** Brawler's per-fighter dials and Adder's short-sighted worms changed nothing or
   made it worse: difficulty lives in matchups and kits, which are design questions for a person.
4. **Measure the same thing on both sides.** Three gate fixes came from the measures, not the games: compare only
   measures both runs take; do not count near misses while invulnerable; a bot run ends at game over unless
   `BLIP_BOT_CONTINUES` is set (random play took every continue, Bouncer read -16.9).
5. **Continues** (Bubbler, Raider, Warlock, Rounds, Bouncer, Brawler, all on `blip::Continue`) let players see later
   levels; the measures cannot see them (time after game over). Score-attack games keep one run.
6. **A person's reaction time** (`BLIP_BOT_REACT`) tests the bots more than the games; `Pad::arrive` now anticipates
   the delay (Rounds at 250 ms: 1-2 → 4 deliveries).
7. **Bot changes are measure changes.** Warlock's bot backing off (close calls 0.08 → 0.51) shows the game a careful
   player sees; Raider's bot looking ahead with a stick change cut its hits.

## Attempts

| Game | Tried | Measured | Kept? |
|---|---|---|---|
| Raider | Sweep a round from where it is (phantom hits from across the screen) | hits 8.5 → 2.8 a minute, fun +3.5 | yes |
| Raider | Flak holds fire inside 110 px (a shell's 0.46 s flight leaves time to dodge) | hits 2.8 → 1.2, fun +5.8 ± 2.1 | yes |
| Raider | Grazing pays 10 points | fun -2.3 ± 1.1: random play grazes by blundering | no |
| Raider | Holding fire plays the 7.6 s take-off 4x as fast | busy 0.80 → 0.90 | yes |
| Raider | No-damage streak, every 5 kills +1 multiple up to x4 | skill 0.14 → 0.27, fun +6.4 ± 0.5 | yes |
| Raider | Bot: shared rollout cost; 1.2 s look-ahead | more hits | no |
| Raider | Bot: one stick change in the look-ahead | fewer hits over 9 seeds | yes |
| Meteors | Accuracy streak, every 5 hits without a wasted shot +1 up to x4 | skill 0.56 → 0.58, fun +1.6 ± 0.4 | yes |
| Defender | A lost cannon breaks the streak too | skill 0.32 → 0.36, fun +2.0 | yes |
| Serpent | A lost life breaks the quick-meal streak | unchanged (consistency) | yes |
| Warlock | Monsters walk a tile a step (path finder returned the target) | a bug fix; playthroughs pass | yes |
| Warlock | Fireballs burn by drawn position | a fairness fix, not measurable | yes |
| Warlock | Bot steps away from monsters (`grid::influence`) | close calls 0.08 → 0.51, busy 0.79 → 0.97 | yes |
| Warlock | Slower monsters | not tried: deaths are the bot walking into them | no |
| Brawler | Per-fighter CPU dials; +1 dial for the two weakest | ladder curve no smoother | no |
| Adder | CPU worms see less far early | fun -13 ± 13 | no |
| Adder | Hit-box hook | square boxes on 2 px trails overcount | no |
| Bubbler | Monsters pause before bumping the dragon | slow players lose sooner | no |
| Bubbler, Raider, Warlock, Rounds, Bouncer | Continues on shared `blip::Continue` | playthroughs and unit tests pass | yes |
| Rounds | Manual spread shortened (it overflowed its pages) | spread test passes | yes |
| all | Hit boxes from one named source per game, used by the collision code and the meter | identical runs | yes |
| all | `Pad::arrive` anticipates the reaction delay | no change without a delay | yes |
| Warlock | Bot backs off from a wraith that has shown itself to pounce (counts 2 in the danger map); deaths now record the killer (`death_<kind>`) | wraiths killed on every seed (7 of 17 lives); now 1 of 10; skill 0.74 → 0.87, fun +3 ± 7.5. At 250 ms wraith deaths 6 → 2: the 0.35 s tell is fair | yes (bot) |
| all | `BLIP_BOT_NEAR`/`SHOTS` folders are made when missing (saving a frame panicked) | a bug fix | yes |
| Brawler | Busy reads 0.74: the idle stretches are the 4.5 s between rounds (K.O. 2.2 s, the bow 2.3 s) and the Vs screen | genre pacing; a bot pressing through it would only fool the measure | no change |
