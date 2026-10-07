//! Balance, measured: scripted styles against the CPU, and CPU against CPU.

use super::*;

// ---- balance, measured ---------------------------------------------------

/// A whole round fought headlessly, the real CPU against a crude rusher,
/// turtle or poker, reported as numbers: how the game treats players who are
/// not good at it is most of whether it is fun.
pub(crate) struct RoundStats {
    pub(crate) seconds: f32,
    pub(crate) player_health: i32,
    pub(crate) cpu_health: i32,
    pub(crate) player_hits: i32,
    pub(crate) cpu_hits: i32,
    pub(crate) timed_out: bool,
}

/// Every test that runs the CPU takes this first: the RNG is process-global
/// and `cargo test` runs in parallel, so without the lock (and a seed inside
/// it) the balance tests pass or fail by scheduling.
pub(crate) static SIMULATION: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn simulating(seed: u64) -> std::sync::MutexGuard<'static, ()> {
    // A poisoned lock just means some other simulation test already
    // failed its assertion; this one can still run.
    let guard = SIMULATION.lock().unwrap_or_else(|e| e.into_inner());
    blip::rand_seed(seed);
    guard
}

pub(crate) fn fight_round(pick: usize, foe_index: usize, style: fn(usize, &[Fighter; 2]) -> Input) -> RoundStats {
    fight_round_seen(pick, foe_index, style, &mut |_, _, _| {})
}

/// The same, calling `seen(attacker, their fighter, damage)` for every blow
/// that lands.
pub(crate) fn fight_round_seen(pick: usize, foe_index: usize, style: fn(usize, &[Fighter; 2]) -> Input,
                    seen: &mut dyn FnMut(usize, &Fighter, i32)) -> RoundStats {
    let mut g = Game::new();
    g.pick = pick;
    g.start_match(foe_index);
    g.state = State::Fight;

    let mut stats = RoundStats {
        seconds: 0.0, player_health: 0, cpu_health: 0,
        player_hits: 0, cpu_hits: 0, timed_out: false,
    };

    let mut frame = 0usize;
    while g.p[0].health > 0 && g.p[1].health > 0 && stats.seconds < ROUND_SECS {
        if g.hitstop > 0.0 { g.hitstop -= F; frame += 1; continue; }
        stats.seconds += F;

        let p_in = style(frame, &g.p);
        g.cpu_delay -= F;
        if g.cpu_delay <= 0.0 {
            g.cpu_plan = cpu_think(&mut g);
            g.cpu_delay = 0.38 - 0.18 * g.difficulty;
        }
        let c_in = cpu_input(&g);
        let ins = [p_in, c_in];

        for i in 0..2 {
            let other = g.p[1 - i].x;
            if g.p[i].free() && !g.p[i].airborne() {
                g.p[i].facing = if other >= g.p[i].x { 1.0 } else { -1.0 };
            }
            let close = face_off(&mut g.p)[i];
            apply_input(&mut g.p[i], ins[i], close, F);
            advance(&mut g.p[i], F);
            g.p[i].x = clamp(g.p[i].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
        }
        separate(&mut g.p);

        for a in 0..2 {
            let d = 1 - a;
            let (mut atk, mut def) = (g.p[a], g.p[d]);
            let (dmg, blocked, knock) = resolve_hit(&mut atk, &mut def, ins[d]);
            g.p[a] = atk;
            g.p[d] = def;
            if dmg > 0 {
                if a == 0 { stats.player_hits += 1; } else { stats.cpu_hits += 1; }
                seen(a, &g.p[a], dmg);
                g.hitstop = hitstop_for(dmg, knock, false);
                push_apart(&mut g.p, if knock { 34.0 } else { 9.0 });
            } else if blocked {
                g.hitstop = hitstop_for(0, false, true);
                push_apart(&mut g.p, 6.0);
            }
        }
        frame += 1;
    }
    stats.timed_out = g.p[0].health > 0 && g.p[1].health > 0;
    stats.player_health = g.p[0].health;
    stats.cpu_health = g.p[1].health;
    stats
}

/// Walks forward and mashes punch — the way everybody plays a fighting
/// game for the first sixty seconds of their life.
pub(crate) fn rusher(frame: usize, p: &[Fighter; 2]) -> Input {
    let mut i = Input::default();
    if p[1].x > p[0].x { i.right = true; } else { i.left = true; }
    i.punch_low = frame % 8 < 2;
    i
}

/// Holds back and blocks, and does nothing else.
pub(crate) fn turtle(_frame: usize, p: &[Fighter; 2]) -> Input {
    let mut i = Input::default();
    if p[1].x > p[0].x { i.left = true; } else { i.right = true; }
    i.down = true;
    i
}

/// Keeps its distance, crouch-blocks (standing into sweeps is a hole the game
/// punishes), punishes whiffs, and pokes from range.
pub(crate) fn poker(frame: usize, p: &[Fighter; 2]) -> Input {
    let mut i = Input::default();
    let dist = (p[1].x - p[0].x).abs();
    let toward = p[1].x > p[0].x;
    let (fwd, back) = if toward { (&mut i.right as *mut bool, &mut i.left as *mut bool) }
                      else { (&mut i.left as *mut bool, &mut i.right as *mut bool) };
    // SAFETY-free alternative would need two branches everywhere; these
    // are two distinct fields of a local struct.
    let (fwd, back) = unsafe { (&mut *fwd, &mut *back) };

    // The safe poke is the yardstick: the low kick. (Poking with the high
    // kick made the poker throw its most punishable move as its jab.)
    let my_kick = attack_range(&p[0], MoveId::LowKick);
    let whiffed = p[1].act == Act::Attack && p[1].hit_done;

    if whiffed && dist < my_kick {
        i.kick_high = true;                 // punish
    } else if dist > my_kick * 1.15 {
        *fwd = true;                        // close the gap
    } else if frame % 30 < 3 {
        i.kick_low = true;                  // poke from the edge
    } else if frame % 30 < 6 {
        i.down = true;
        i.kick_low = true;                  // and mix in a low
    } else {
        // Otherwise hold guard, and change which: one guard held all round
        // ate every high kick.
        *back = true;
        i.down = frame % 96 < 52;
    }
    i
}

#[test]
pub(crate) fn every_round_is_decided_by_something_that_happened() {
    // A round may run the clock out, but must not often end with both
    // fighters largely untouched: that is a stalemate the rules cannot break.
    // Asserted as a rate over several seeds, since one seed is a coin flip in
    // a chaotic simulation (the rate held at 2-3% over 180 rounds).
    let styles: [(&str, fn(usize, &[Fighter; 2]) -> Input); 3] =
        [("rusher", rusher), ("poker", poker), ("turtle", turtle)];
    let mut stalemates = vec![];
    let mut decided_fast = 0;
    let mut rounds = 0;

    for s in 0..4u64 {
        let _sim = simulating(0xB4A17E + s * 0x9E37);
        for pick in (0..FIGHTERS.len()).filter(|&i| !FIGHTERS[i].invincible) {
            for foe in [0, RUNGS - 3] {
                for (name, style) in styles {
                    let r = fight_round(pick, foe, style);
                    rounds += 1;
                    if !r.timed_out { decided_fast += 1; }
                    let cpu_who = Game { pick, ..Game::new() }.ladder()[foe];
                    let both_healthy = r.player_health as f32
                            > FIGHTERS[pick].health as f32 * 0.55
                        && r.cpu_health as f32 > FIGHTERS[cpu_who].health as f32 * 0.55;
                    if r.timed_out && both_healthy {
                        stalemates.push(format!("{} vs opponent {foe} [{name}] seed {s}: {} / {}",
                            FIGHTERS[pick].name, r.player_health, r.cpu_health));
                    }
                }
            }
        }
    }

    assert!(stalemates.len() * 12 <= rounds,
        "{} of {rounds} rounds ended with nothing having happened: {stalemates:?}",
        stalemates.len());
    assert!(decided_fast * 2 >= rounds,
        "only {decided_fast} of {rounds} rounds ended in a knockout — the clock is refereeing");
}

#[test]
pub(crate) fn playing_well_beats_playing_badly() {
    let _sim = simulating(0x5EED02);
    // The claim the whole design rests on. Poking and blocking at range
    // has to do measurably better against the same CPU than walking
    // forward and mashing, or none of the rules above are load-bearing.
    let mut rush_margin = 0;
    let mut poke_margin = 0;
    // Between fighters who have a range to keep: a half-size one is built
    // to leap in behind every blow, on either side of the fight.
    let spaces = |who: usize| !FIGHTERS[who].invincible && FIGHTERS[who].size >= 1.0;
    for pick in (0..FIGHTERS.len()).filter(|&i| spaces(i)) {
        // Every rung with such an opponent on it: two rungs alone are
        // mostly turtles.
        for foe in 0..RUNGS - 1 {
            if !spaces(Game { pick, ..Game::new() }.ladder()[foe]) { continue; }
            let r = fight_round(pick, foe, rusher);
            rush_margin += r.player_health - r.cpu_health;
            let p = fight_round(pick, foe, poker);
            poke_margin += p.player_health - p.cpu_health;
        }
    }
    assert!(poke_margin > rush_margin,
        "spacing and blocking ({poke_margin}) did no better than mashing ({rush_margin})");
}

#[test]
pub(crate) fn turtling_does_not_win_on_its_own() {
    // Blocking has to be worth doing and not worth doing *only*: a player
    // who crouch-blocks all round lasts it out twice as often as one who
    // stands there, and never takes it.
    fn idle(_frame: usize, _p: &[Fighter; 2]) -> Input { Input::default() }
    let rounds_survived = |style: fn(usize, &[Fighter; 2]) -> Input| {
        let _sim = simulating(0x5EED03);
        let (mut survived, mut won, mut rounds) = (0, 0, 0);
        for pick in (0..FIGHTERS.len()).filter(|&i| !FIGHTERS[i].invincible) {
            for foe in 0..RUNGS - 1 {
                let r = fight_round(pick, foe, style);
                rounds += 1;
                if r.player_health > 0 { survived += 1; }
                if r.cpu_health <= 0 { won += 1; }
            }
        }
        (survived, won, rounds)
    };
    let (guarded, won, rounds) = rounds_survived(turtle);
    let (stood, _, _) = rounds_survived(idle);
    assert_eq!(won, 0, "a fighter who never attacked won {won} rounds");
    assert!(guarded >= stood * 2 && guarded * 4 >= rounds,
        "crouch-blocking survived {guarded} of {rounds} rounds, standing still {stood}: blocking is not working");
}

