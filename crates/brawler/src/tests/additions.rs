//! Counter hits, rage, guard impact, the bonus fruit, clashing bolts, the shell and the rest of the ten additions.

use super::*;

// ---- ten additions --------------------------------------------------------

/// `a` at full extension of `mv`, just inside its range of `d`.
pub(crate) fn in_range(a: usize, d: usize, mv: MoveId) -> [Fighter; 2] {
    let mut p = [at(a, 200.0, 1.0), at(d, 300.0, -1.0)];
    face_off(&mut p);
    p[1].x = p[0].x + attack_range(&p[0], mv) - 3.0;
    wind_to_active(&mut p[0], mv);
    p
}

#[test]
pub(crate) fn hitting_someone_out_of_their_attack_is_a_counter() {
    let plain = {
        let [mut a, mut d] = in_range(0, 2, MoveId::LowKick);
        resolve_hit(&mut a, &mut d, Input::default()).0
    };
    // Caught winding up: more damage, more stun, and it is announced.
    let [mut a, mut d] = in_range(0, 2, MoveId::LowKick);
    d.start_attack(MoveId::HighKick);
    let (dmg, _, _) = resolve_hit(&mut a, &mut d, Input::default());
    assert!(a.countered && dmg > plain, "a counter did {dmg} against a plain {plain}");
    assert!(d.stun > move_data(MoveId::LowKick).hitstun * F);
    // Caught recovering is a punish, not a counter.
    let [mut a, mut d] = in_range(0, 2, MoveId::LowKick);
    d.start_attack(MoveId::HighKick);
    d.t = 0.5;
    let (dmg, _, _) = resolve_hit(&mut a, &mut d, Input::default());
    assert!(!a.countered && dmg == plain);
}

#[test]
pub(crate) fn the_last_quarter_of_the_bar_hits_harder() {
    let base = move_data(MoveId::HighKick);
    let mut f = at(0, 200.0, 1.0);
    let fresh = f.scaled(base).damage;
    f.health = FIGHTERS[0].health / 4;
    assert!(f.enraged() && f.scaled(base).damage > fresh);
    f.health = FIGHTERS[0].health / 4 + 1;
    assert!(!f.enraged() && f.scaled(base).damage == fresh);
    // Out is out, and the fighter nothing hurts has nothing to be angry about.
    f.health = 0;
    assert!(!f.enraged());
    let mut z = at(FIGHTERS.iter().position(|a| a.invincible).unwrap(), 200.0, 1.0);
    z.health = 1;
    assert!(!z.enraged());
}

#[test]
pub(crate) fn a_tap_toward_at_the_last_moment_turns_the_blow() {
    let toward = Input { left: true, ..Default::default() }; // the defender faces left
    // Tapped just before it lands: no damage, and the attacker is left open.
    let [mut a, mut d] = in_range(0, 1, MoveId::HighKick);
    apply_input(&mut d, toward, false, F);
    let full = d.health;
    let (dmg, blocked, _) = resolve_hit(&mut a, &mut d, toward);
    assert!(dmg == 0 && blocked && d.health == full && a.turned);
    assert!(a.act == Act::Hitstun && a.stun >= IMPACT_STUN - 0.001);
    assert!(d.free(), "the defender is not free to answer");
    // Held, not tapped: walking into a kick is just being kicked.
    let [mut a, mut d] = in_range(0, 1, MoveId::HighKick);
    for _ in 0..12 { apply_input(&mut d, toward, false, F); }
    assert!(resolve_hit(&mut a, &mut d, toward).0 > 0);
    // A throw is not a blow, and cannot be turned.
    let [mut a, mut d] = in_range(0, 1, MoveId::Throw);
    apply_input(&mut d, toward, false, F);
    assert!(resolve_hit(&mut a, &mut d, toward).0 > 0);
}

#[test]
pub(crate) fn the_fruit_comes_once_a_round_and_feeds_whoever_reaches_it() {
    let mut g = Game::new();
    g.start_match(0);
    g.state = State::Fight;
    assert!(fruit_step(&mut g, F).is_none() && g.fruit.ttl <= 0.0, "it is there from the bell");
    g.clock = FRUIT_AT - 0.01;
    fruit_step(&mut g, F);
    assert!(g.fruit.ttl > 0.0, "it never appeared");
    assert!((g.fruit.x - g.p[0].x).abs() > 60.0 && (g.fruit.x - g.p[1].x).abs() > 60.0);
    // Jumping over it is not eating it.
    g.p[0].health = 40;
    g.p[0].x = g.fruit.x;
    g.p[0].y = FLOOR_Y - 60.0;
    assert!(fruit_step(&mut g, F).is_none());
    // Walking onto it is, and it gives some health back and is gone.
    g.p[0].y = FLOOR_Y;
    assert_eq!(fruit_step(&mut g, F), Some(0));
    assert!(g.p[0].health > 40 && g.p[0].health < FIGHTERS[g.p[0].who].health);
    assert!(g.fruit.ttl <= 0.0 && g.sess.score > 0);
    assert!(fruit_step(&mut g, F).is_none() && g.fruit.ttl <= 0.0, "a second helping");
    // It never fills past full, and the next round serves the next one.
    let first = g.fruit.kind;
    g.round += 1;
    g.start_round();
    g.clock = FRUIT_AT - 0.01;
    g.p[1].x = WIN_W as f32 / 2.0;
    assert_eq!(fruit_step(&mut g, F), Some(1));
    assert_eq!(g.p[1].health, FIGHTERS[g.p[1].who].health);
    assert_ne!(g.fruit.kind, first);
    // Left alone it goes away.
    g.round += 1;
    g.start_round();
    g.clock = FRUIT_AT - 0.01;
    for _ in 0..((FRUIT_STAYS / F) as usize + 5) { fruit_step(&mut g, F); }
    assert!(g.fruit.ttl <= 0.0);
}

#[test]
pub(crate) fn two_bolts_cancel_and_the_laser_burns_through() {
    let bolt = |x: f32, vx: f32, owner: usize| Bolt { x, y: 276.0, vx, vy: 0.0, owner, active: true, damage: 16 };
    let mut g = Game::new();
    g.p = [at(0, 100.0, 1.0), at(3, 540.0, -1.0)];
    g.bolts[0] = bolt(300.0, 300.0, 0);
    g.bolts[1] = bolt(312.0, -300.0, 1);
    assert!(clash_bolts(&mut g).is_some());
    assert!(g.bolts.iter().all(|b| !b.active), "a bolt survived the clash");
    // Two from the same hand pass each other by.
    g.bolts[0] = bolt(300.0, 300.0, 0);
    g.bolts[1] = bolt(312.0, 300.0, 0);
    assert!(clash_bolts(&mut g).is_none());
    // The laser eats the bolt and carries on.
    let laser = FIGHTERS.iter().position(|a| a.special == Special::LaserVision).unwrap();
    g.p[1] = at(laser, 540.0, -1.0);
    g.bolts[1] = bolt(312.0, -600.0, 1);
    assert!(clash_bolts(&mut g).is_some());
    assert!(!g.bolts[0].active && g.bolts[1].active);
}

#[test]
pub(crate) fn a_turtle_in_its_shell_cannot_be_chipped() {
    let chip = |who: usize, crouch: bool| {
        let [mut a, mut d] = in_range(1, who, MoveId::Special);
        let full = d.health;
        let (_, blocked, _) = resolve_hit(&mut a, &mut d, back_input(-1.0, crouch));
        assert!(blocked);
        (full - d.health, d.shelled())
    };
    assert_eq!(chip(3, true), (0, true), "the shell was chipped");
    assert!(chip(3, false).0 > 0 && !chip(3, false).1, "a standing turtle is not in its shell");
    assert!(chip(0, true).0 > 0, "only turtles have a shell");
}

#[test]
pub(crate) fn the_announcer_calls_a_perfect_and_a_great() {
    let mut g = Game::new();
    g.p = [at(0, 200.0, 1.0), at(1, 440.0, -1.0)];
    g.result = RoundResult::P1;
    assert_eq!(round_call(&g), "PERFECT");
    g.p[0].health = FIGHTERS[0].health - 1;
    assert_eq!(round_call(&g), "");
    g.p[0].health = FIGHTERS[0].health / 10;
    assert_eq!(round_call(&g), "GREAT");
    g.result = RoundResult::Draw;
    assert_eq!(round_call(&g), "");
    // Untouched is nothing to shout about when nothing can touch you.
    let z = FIGHTERS.iter().position(|a| a.invincible).unwrap();
    g.p[1] = at(z, 440.0, -1.0);
    g.result = RoundResult::P2;
    assert_eq!(round_call(&g), "");
}

#[test]
pub(crate) fn the_finishing_blow_plays_in_slow_motion_and_the_bar_drains_after_it() {
    let mut g = Game::new();
    g.start_match(0);
    g.state = State::RoundEnd;
    g.phase.start(2.2);
    g.p[0].act = Act::Victory;
    g.p[0].t = 0.0;
    g.slow = SLOW_MO_SECS;
    for _ in 0..12 { update_round_end(&mut g, F); }
    assert!(g.p[0].t < 12.0 * F * 0.5, "the fighters ran at full speed: {:.3}", g.p[0].t);
    for _ in 0..60 { update_round_end(&mut g, F); }
    assert!(g.slow <= 0.0, "the slow motion never ended");
    // A bar's ghost falls to the health under it and never past.
    g.p[1].health = 20;
    g.ghost[1] = 100.0;
    drain_ghosts(&mut g, 0.1);
    assert!(g.ghost[1] < 100.0 && g.ghost[1] > 20.0);
    for _ in 0..100 { drain_ghosts(&mut g, 0.1); }
    assert_eq!(g.ghost[1], 20.0);
}

#[test]
pub(crate) fn the_giants_rage_jump_floors_anyone_standing_anywhere() {
    let giant = FIGHTERS.iter().position(|a| a.build == Build::Giant).unwrap();
    // A whole jump: up, the kick in the air, and down again.
    let jump = |g: &mut Game, kick: bool| {
        apply_input(&mut g.p[0], Input { up: true, ..Default::default() }, false, F);
        let mut shook = false;
        for frame in 0..120 {
            let inp = Input { kick_low: kick && frame == 12, ..Default::default() };
            apply_input(&mut g.p[0], inp, false, F);
            let was_air = g.p[0].airborne();
            advance(&mut g.p[0], F);
            if was_air && !g.p[0].airborne() { shook = land_quake(g, 0); break; }
        }
        shook
    };
    let fresh = |foe_x: f32| {
        let mut g = Game::new();
        g.p = [at(giant, 80.0, 1.0), at(0, foe_x, -1.0)];
        g
    };
    // From the far end of the stage, a fighter on the floor goes down.
    let mut g = fresh(WIN_W as f32 - WALL_MARGIN);
    assert!(jump(&mut g, true), "the floor did not shake");
    assert_eq!(g.p[1].act, Act::Knockdown);
    assert!(g.p[1].health < FIGHTERS[0].health && g.quake_t > 0.0 && g.shake > 0.2);
    // Being in the air when he lands is the answer.
    let mut g = fresh(500.0);
    g.p[1].y = FLOOR_Y - 40.0;
    assert!(jump(&mut g, true));
    assert_ne!(g.p[1].act, Act::Knockdown);
    // A plain jump shakes nothing, and neither does anybody else's kick.
    let mut g = fresh(500.0);
    assert!(!jump(&mut g, false) && g.p[1].act != Act::Knockdown);
    let mut g = fresh(500.0);
    g.p[0] = at(1, 80.0, 1.0);
    assert!(!jump(&mut g, true));
    // The CPU knows the answer, and at the top of the ladder usually finds it.
    let _sim = simulating(0x57031);
    let mut g = fresh(500.0);
    g.difficulty = 0.75;
    g.p[0].y = FLOOR_Y - 80.0;
    g.p[0].stomp = true;
    let jumps = (0..200).filter(|_| cpu_think(&mut g) == CpuPlan::Jump).count();
    assert!(jumps > 120, "the CPU jumped a rage jump {jumps} times in 200");
    // Knocked out of the air, he does not get his landing.
    let mut g = fresh(500.0);
    g.p[0].stomp = true;
    g.p[0].act = Act::Knockdown;
    assert!(!land_quake(&mut g, 0) && !g.p[0].stomp);
}
