//! Whole rounds: the simulated fight, the real round loop driven by scripts, and the attract mode.

use super::*;

// ---- a whole fight -------------------------------------------------------

/// Run the simulation the way update_fight() does, minus the sound and
/// the scoring. Two fighters, two input streams, for as many frames as
/// asked — enough to prove the loop terminates rather than jamming.
pub(crate) fn spar(a_who: usize, b_who: usize, gap: f32, frames: usize,
        mut inputs: impl FnMut(usize, &[Fighter; 2]) -> [Input; 2]) -> [Fighter; 2] {
    let mid = WIN_W as f32 / 2.0;
    let mut p = [Fighter::new(a_who, mid - gap / 2.0, 1.0),
                 Fighter::new(b_who, mid + gap / 2.0, -1.0)];
    for frame in 0..frames {
        let ins = inputs(frame, &p);
        for i in 0..2 {
            let other = p[1 - i].x;
            if p[i].free() && !p[i].airborne() {
                p[i].facing = if other >= p[i].x { 1.0 } else { -1.0 };
            }
        }
        let close = face_off(&mut p);
        for i in 0..2 {
            apply_input(&mut p[i], ins[i], close[i], F);
            advance(&mut p[i], F);
            p[i].x = clamp(p[i].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
        }
        separate(&mut p);
        for atk in 0..2 {
            let def = 1 - atk;
            let (mut a, mut d) = (p[atk], p[def]);
            let (dmg, blocked, _) = resolve_hit(&mut a, &mut d, ins[def]);
            p[atk] = a;
            p[def] = d;
            if dmg > 0 || blocked { push_apart(&mut p, if blocked { 6.0 } else { 9.0 }); }
        }
    }
    p
}

#[test]
pub(crate) fn two_fighters_left_alone_actually_hurt_each_other() {
    let _sim = simulating(0x5EED01);
    // The loop has to make progress. A fight where nothing connects is
    // the failure mode that every individual rule above can pass while
    // the game is still unplayable.
    let mut forward_and_punch = |frame: usize, _p: &[Fighter; 2]| {
        let mut a = Input::default();
        let mut b = Input::default();
        a.right = true;
        b.left = true;
        if frame % 24 < 2 { a.punch_low = true; }
        if frame % 30 < 2 { b.kick_low = true; }
        [a, b]
    };
    let p = spar(0, 1, 240.0, 60 * 12, &mut forward_and_punch);
    assert!(p[0].health < FIGHTERS[p[0].who].health, "nobody ever landed a hit on P1");
    assert!(p[1].health < FIGHTERS[p[1].who].health, "nobody ever landed a hit on P2");
}

#[test]
pub(crate) fn a_fighter_who_blocks_correctly_takes_far_less_than_one_who_does_not() {
    // The single claim the whole design rests on: defence is worth
    // something. Same attacker, same attack, one defender holding back
    // and crouching, the other standing still.
    let mut sweeping = |frame: usize, _p: &[Fighter; 2]| {
        let mut a = Input::default();
        a.down = true;
        if frame % 40 < 2 { a.kick_low = true; } // sweep on a loop
        [a, Input::default()]
    };
    // Inside sweep range from the first frame: this test is about the
    // block, not about walking in.
    let open = spar(0, 1, 74.0, 60 * 10, &mut sweeping);
    let damage_taken_standing = FIGHTERS[open[1].who].health - open[1].health;

    let mut sweeping_vs_block = |frame: usize, _p: &[Fighter; 2]| {
        let mut a = Input::default();
        a.down = true;
        if frame % 40 < 2 { a.kick_low = true; }
        let mut b = Input::default();
        b.right = true; // away from P1, who is on the left
        b.down = true;  // and low, because a sweep is low
        [a, b]
    };
    let guarded = spar(0, 1, 74.0, 60 * 10, &mut sweeping_vs_block);
    let damage_taken_blocking = FIGHTERS[guarded[1].who].health - guarded[1].health;

    assert!(damage_taken_standing > 0, "the attacker never landed anything at all");
    assert!(damage_taken_blocking * 3 < damage_taken_standing,
        "blocking low against a sweep saved almost nothing: {damage_taken_blocking} vs \
         {damage_taken_standing} damage");
}

#[test]
pub(crate) fn nobody_gets_stuck_in_a_state_they_cannot_leave() {
    // Twelve seconds of someone mashing every button at once. Whatever
    // that produces, both fighters must still be able to act at the end
    // — a state machine with a dead end is a game that freezes.
    let mut mash = |frame: usize, _p: &[Fighter; 2]| {
        let mut a = Input::default();
        a.punch_low = frame % 3 == 0;
        a.kick_high = frame % 5 == 0;
        a.down = frame % 7 < 3;
        a.up = frame % 31 == 0;
        a.right = frame % 11 < 5;
        a.left = frame % 13 < 4;
        a.special = frame % 97 == 0;
        [a, a]
    };
    let mut p = spar(0, 2, 200.0, 60 * 12, &mut mash);
    for f in p.iter_mut() {
        // Give each one a second of no input; they should come to rest.
        for _ in 0..60 { advance(f, F); }
        assert!(f.free() || f.act == Act::Attack,
            "a fighter ended up stuck in {:?} with no way out", f.act);
        assert!(f.y <= FLOOR_Y + 0.01, "a fighter ended up below the floor");
    }
}

// ---- the real round loop ------------------------------------------------------
// `step_fight` is the game's own frame, driven here by scripted inputs.

/// A round under way between `a` and `b`, a body-length and a half apart.
pub(crate) fn round(a: usize, b: usize) -> Game {
    let mut g = Game::new();
    g.mode = Mode::Versus;
    g.state = State::Fight;
    g.p = [at(a, 200.0, 1.0), at(b, 380.0, -1.0)];
    g.fruit_due = false;
    g
}

/// Run `frames` of it, each side pressing what its script says for the frame.
pub(crate) fn play(g: &mut Game, frames: usize, mut keys: impl FnMut(usize, &Game) -> [Input; 2]) {
    for frame in 0..frames {
        if g.state != State::Fight { break; }
        let [a, b] = keys(frame, g);
        step_fight(g, F, a, b);
    }
}

pub(crate) fn special() -> Input { Input { special: true, ..Input::default() } }

/// A round with ZENITH hovering `up` over `x`, staring from the first frame;
/// RYUKA presses `keys` for three frames from frame `delay`. Returns what
/// health that left.
pub(crate) fn under_the_stare(x: f32, up: f32, delay: usize, keys: Input) -> i32 {
    let zenith = FIGHTERS.iter().position(|a| a.invincible).unwrap();
    let mut g = round(zenith, 0);
    g.p[0].x = x;
    g.p[0].y = FLOOR_Y - up;
    g.p[0].act = Act::Air;
    play(&mut g, 150, |f, _| [if f == 0 { special() } else { Input::default() },
        if (delay..delay + 3).contains(&f) { keys } else { Input::default() }]);
    g.p[1].health
}

#[test]
pub(crate) fn the_laser_from_the_air_burns_where_he_looked_and_a_jump_gets_clear() {
    let full = FIGHTERS[0].health;
    let up = Input { up: true, ..Input::default() };
    // RYUKA faces left, so away is right.
    let back = Input { up: true, right: true, ..Input::default() };
    assert_eq!(under_the_stare(200.0, 140.0, 0, Input::default()), 0, "standing in it did not end the round");
    // Jumping as the eyes light, or a moment after: it does not follow.
    for delay in [0, 6, 12] {
        assert_eq!(under_the_stare(200.0, 140.0, delay, up), full, "a jump {delay} frames in was followed");
    }
    // Jumping away works later still, and with him low and 80px off, where
    // the beam lands on frame 17.
    for (x, h, latest) in [(200.0, 140.0, 18), (300.0, 60.0, 12)] {
        for delay in (0..=latest).step_by(6) {
            assert_eq!(under_the_stare(x, h, delay, back), full,
                "a jump back {delay} frames in, from under ({x}, {h} up), was caught");
        }
    }
}

#[test]
pub(crate) fn the_web_shot_wraps_whoever_it_reaches_and_two_blows_cut_them_out() {
    let webber = FIGHTERS.iter().position(|a| a.build == Build::Spider).unwrap();
    let mut g = round(webber, 0);
    play(&mut g, 90, |f, _| [if f == 0 { special() } else { Input::default() }, Input::default()]);
    assert!(g.p[1].webbed > 0.0, "the web did not take");
    assert!(g.sounds.iter().any(|&(s, _)| s == Sfx::Thwip));

    // Wrapped, the stick does nothing: holding away neither walks nor guards.
    let x = g.p[1].x;
    let away = back_input(g.p[1].facing, false);
    play(&mut g, 30, |_, _| [Input::default(), away]);
    assert_eq!(g.p[1].x, x, "a webbed fighter walked");

    // Two kicks and it is off.
    let hp = g.p[1].health;
    let kick = Input { kick_low: true, ..Input::default() };
    let toward = Input { right: true, ..Input::default() };
    play(&mut g, 240, |f, g| {
        let far = (g.p[1].x - g.p[0].x).abs() > attack_range(&g.p[0], MoveId::LowKick) - 6.0;
        [if far { toward } else if f % 6 < 2 { kick } else { Input::default() }, away]
    });
    assert_eq!(g.p[1].webbed, 0.0, "still wrapped after a quarter of a minute of kicking");
    assert!(g.p[1].health < hp);
    assert!(g.p[1].web_hits >= WEB_HITS || g.p[1].web_rest > 0.0);
}

#[test]
pub(crate) fn the_laser_ends_the_round_for_whoever_stands_in_it_and_only_costs_a_guard() {
    let zenith = FIGHTERS.iter().position(|a| a.invincible).unwrap();
    // Standing there: gone.
    let mut g = round(zenith, 0);
    play(&mut g, 120, |f, _| [if f == 0 { special() } else { Input::default() }, Input::default()]);
    assert_eq!(g.p[1].health, 0);
    assert_eq!(g.state, State::RoundEnd);
    assert!(g.sounds.iter().any(|&(s, _)| s == Sfx::Laser) && g.sounds.iter().any(|&(s, _)| s == Sfx::Ko));

    // Guarding: it costs the chip and no more.
    let mut g = round(zenith, 0);
    let guard = back_input(g.p[1].facing, false);
    play(&mut g, 120, |f, _| [if f == 0 { special() } else { Input::default() }, guard]);
    assert_eq!(g.p[1].health, FIGHTERS[0].health - LASER_CHIP);

    // Sitting down: it goes over.
    let mut g = round(zenith, 0);
    let duck = Input { down: true, ..Input::default() };
    play(&mut g, 120, |f, _| [if f == 0 { special() } else { Input::default() }, duck]);
    assert_eq!(g.p[1].health, FIGHTERS[0].health, "the level laser caught a croucher");
}

#[test]
pub(crate) fn a_called_turtle_gang_plays_out_in_the_real_round() {
    let turtle = FIGHTERS.iter().position(|a| a.build == Build::Turtle).unwrap();
    let mut g = round(turtle, 0);
    play(&mut g, 400, |f, _| [if f == 0 { special() } else { Input::default() }, Input::default()]);
    assert!(g.sounds.iter().any(|&(s, _)| s == Sfx::Whistle), "nobody whistled");
    assert!(g.helpers.iter().all(|h| h.is_none()), "a helper is still on the stage");
    assert!(g.p[1].health < FIGHTERS[0].health, "three turtles and not a mark on him");
    assert_eq!(g.p[0].health, FIGHTERS[turtle].health);

    // And all of it is stopped by sitting behind a low guard.
    let mut g = round(turtle, 0);
    let low = back_input(g.p[1].facing, true);
    play(&mut g, 400, |f, _| [if f == 0 { special() } else { Input::default() }, low]);
    assert_eq!(g.p[1].health, FIGHTERS[0].health);
}

// ---- the attract mode ---------------------------------------------------------

#[test]
pub(crate) fn the_demo_is_a_round_between_two_different_fair_fighters_and_both_of_them_fight() {
    let _sim = simulating(0xD3A0);
    for _ in 0..20 {
        let mut g = Game::new();
        g.start_demo();
        assert!(g.demo && g.state == State::RoundIntro);
        let (a, b) = (g.p[0].who, g.p[1].who);
        assert_ne!(a, b, "a fighter was drawn against themselves");
        assert!(!FIGHTERS[a].invincible && !FIGHTERS[b].invincible);
        assert_eq!(g.stage, home_of(b));
    }

    let mut g = Game::new();
    g.start_demo();
    g.state = State::Fight;
    let (a, b) = (g.p[0].who, g.p[1].who);
    let mut pressed = [false; 2];
    for _ in 0..(ROUND_SECS / F) as usize + 600 {
        if g.state != State::Fight { break; }
        let ins = [cpu_turn(&mut g, 0, F), cpu_turn(&mut g, 1, F)];
        // Thinking for the first fighter leaves the two where they were.
        assert_eq!((g.p[0].who, g.p[1].who), (a, b));
        for side in 0..2 {
            let i = ins[side];
            pressed[side] |= i.punch_low || i.punch_high || i.kick_low || i.kick_high || i.special;
        }
        step_fight(&mut g, F, ins[0], ins[1]);
    }
    assert!(pressed[0] && pressed[1], "one side of the demo never threw anything: {pressed:?}");
    assert_eq!(g.state, State::RoundEnd, "the demo round never ended");
    assert!(g.p[0].health < FIGHTERS[a].health || g.p[1].health < FIGHTERS[b].health,
        "a whole round and nobody was touched");
}

/// One CPU-against-CPU round on the real loop. Returns the health each has
/// left, as a share of full, and how long it took.
pub(crate) fn cpu_round(a: usize, b: usize, difficulty: f32) -> ([f32; 2], f32) {
    let mut g = Game::new();
    g.demo = true;
    g.difficulty = difficulty;
    g.p = [at(a, 200.0, 1.0), at(b, 440.0, -1.0)];
    g.fruit_due = false;
    g.state = State::Fight;
    let mut secs = 0.0;
    while g.state == State::Fight && secs < ROUND_SECS + 20.0 {
        let ins = [cpu_turn(&mut g, 0, F), cpu_turn(&mut g, 1, F)];
        step_fight(&mut g, F, ins[0], ins[1]);
        secs += F;
    }
    ([g.p[0].health as f32 / FIGHTERS[a].health as f32, g.p[1].health as f32 / FIGHTERS[b].health as f32], secs)
}

#[test]
#[ignore = "diagnostic"]
pub(crate) fn diagnose_cpu_against_cpu() {
    let _sim = simulating(0xBA1A);
    let fair: Vec<usize> = (0..FIGHTERS.len()).filter(|&w| !FIGHTERS[w].invincible).collect();
    const N: usize = 30;
    println!("{:8}  {}  | won   secs", "", fair.iter().map(|&w| format!("{:>4.4}", FIGHTERS[w].name)).collect::<Vec<_>>().join(" "));
    for &a in &fair {
        let (mut row, mut total, mut time) = (vec![], 0.0, 0.0);
        for &b in &fair {
            if a == b { row.push("   -".to_string()); continue; }
            let mut won = 0.0;
            for k in 0..N {
                // Each side of the stage in turn.
                let (h, secs) = if k % 2 == 0 { cpu_round(a, b, 0.4) } else { let (h, s) = cpu_round(b, a, 0.4); ([h[1], h[0]], s) };
                won += if h[0] > h[1] { 1.0 } else if h[0] == h[1] { 0.5 } else { 0.0 };
                time += secs;
            }
            total += won;
            row.push(format!("{:>4.0}", won / N as f32 * 100.0));
        }
        println!("{:8}  {}  | {:>3.0}%  {:>4.1}", FIGHTERS[a].name, row.join(" "),
            total / (N * (fair.len() - 1)) as f32 * 100.0, time / (N * (fair.len() - 1)) as f32);
    }
}

#[test]
pub(crate) fn nobody_on_the_roster_is_hopeless_or_unbeatable() {
    // CPU against CPU on the real loop, everyone against everyone. The giant
    // is the ladder's second-to-last fight and is meant to be the strongest;
    // even he has to lose sometimes.
    let _sim = simulating(0xBA1A);
    let fair: Vec<usize> = (0..FIGHTERS.len()).filter(|&w| !FIGHTERS[w].invincible).collect();
    const N: usize = 10;
    for &a in &fair {
        let mut won = 0.0;
        for &b in fair.iter().filter(|&&b| b != a) {
            for k in 0..N {
                let h = if k % 2 == 0 { cpu_round(a, b, 0.4).0 } else { let h = cpu_round(b, a, 0.4).0; [h[1], h[0]] };
                won += if h[0] > h[1] { 1.0 } else if h[0] == h[1] { 0.5 } else { 0.0 };
            }
        }
        let share = won / (N * (fair.len() - 1)) as f32;
        let top = if FIGHTERS[a].build == Build::Giant { 0.92 } else { 0.70 };
        assert!((0.24..=top).contains(&share), "{} wins {:.0}% of its rounds", FIGHTERS[a].name, share * 100.0);
    }
}

#[test]
pub(crate) fn the_second_bonus_round_drops_ten_barrels_that_roll_away_if_nobody_stops_them() {
    let mut g = after_winning(BONUS_AFTER[1]);
    assert_eq!(g.bonus_total, BARRELS);
    assert!(g.barrels.iter().all(|b| b.rolls && b.hp == 1 && b.y < 0.0));
    // They come one at a time, land, and leave by the nearer edge.
    let mut most_at_once = 0;
    for _ in 0..(BONUS_SECS / F) as usize {
        if g.bonus_done > 0.0 { break; }
        update_bonus(&mut g, F, Input::default());
        let out = g.barrels.iter().filter(|b| b.hp > 0 && b.wait <= 0.0).count();
        most_at_once = most_at_once.max(out);
        assert!(g.barrels.iter().all(|b| b.y <= FLOOR_Y + 0.01), "a barrel went through the floor");
    }
    assert!((1..=5).contains(&most_at_once), "{most_at_once} barrels on the stage at once");
    assert_eq!(g.bonus[1], 0, "an untouched barrel was counted as broken");

    // One that has landed in front of the fighter goes at a single blow.
    let mut g = after_winning(BONUS_AFTER[1]);
    g.barrels[0] = Barrel { x: g.p[0].x + 44.0, y: FLOOR_Y, wait: 0.0, vx: 0.0, ..g.barrels[0] };
    let kick = Input { kick_low: true, ..Input::default() };
    for f in 0..40 { update_bonus(&mut g, F, if f < 2 { kick } else { Input::default() }); }
    assert!(g.barrels[0].broke_t >= 0.0, "a kick did not break it");
}

#[test]
pub(crate) fn a_challenger_stops_the_solo_fight_and_takes_both_to_the_select_screen() {
    let mut g = Game { pick: 4, ..Game::new() };
    g.start_match(3);
    g.state = State::Fight;
    g.sess.add_score(900);
    challenge(&mut g);
    assert_eq!(g.state, State::Challenger);
    run_phase(&mut g, update_challenger);
    assert_eq!((g.state, g.mode), (State::Select, Mode::Versus));
    assert_eq!(g.pick, 4, "player one lost the fighter they were playing");
    assert_ne!(g.pick2, g.pick);
    assert_eq!(g.locked, [false; 2]);
    assert_eq!((g.opponent_index, g.sess.score), (0, 0), "the solo run came along");
}
