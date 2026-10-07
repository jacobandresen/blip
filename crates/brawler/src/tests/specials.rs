//! Specials: bolts, the rush, the laser, the turtle call and the web.

use super::*;

// ---- specials ------------------------------------------------------------

#[test]
pub(crate) fn a_fireball_leaves_the_hand_pointing_the_way_the_fighter_is_facing() {
    // The one attack that keeps travelling after the animation ends, so
    // it is also the one that can be aimed backwards by a sign error and
    // look almost right while doing nothing.
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    g.p[0].facing = 1.0;
    g.spawn_bolt(0, 16);
    let bolt = g.bolts.iter().find(|b| b.active).expect("no bolt was spawned");
    assert!(bolt.vx > 0.0, "the fireball travelled backwards");
    assert!(bolt.x > g.p[0].x, "the fireball spawned behind the fighter who threw it");
    assert!(bolt.y < g.p[0].y, "the fireball spawned underground");

    // And the other way round.
    let mut g2 = Game::new();
    g2.pick = 0;
    g2.start_match(0);
    g2.p[0].facing = -1.0;
    g2.spawn_bolt(0, 16);
    let bolt = g2.bolts.iter().find(|b| b.active).unwrap();
    assert!(bolt.vx < 0.0 && bolt.x < g2.p[0].x);
}

#[test]
pub(crate) fn the_projectile_pool_cannot_be_overrun() {
    // A fighter holding the special down must not be able to allocate
    // forever; the pool is fixed and the overflow has to be dropped.
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    for _ in 0..50 { g.spawn_bolt(0, 16); }
    assert!(g.bolts.iter().filter(|b| b.active).count() <= g.bolts.len());
}

#[test]
pub(crate) fn every_special_is_something_the_shared_table_cannot_do() {
    // Each special breaks a rule the normals obey (the bolt leaves the
    // fighter, the rush crosses the stage, the talon kick goes airborne), or
    // fighters differ only in colour.
    let base = move_data(MoveId::Special);
    assert!(base.knockdown, "a special that does not knock down is just a slow kick");
    // It out-damages the pokes (it ends an exchange); the high kick
    // out-damages it and is meant to, a 14-frame overhead you must read.
    assert!(base.damage > move_data(MoveId::LowKick).damage);
    // And it costs more to miss than anything else in the table, which
    // is the only thing keeping it from being the whole game.
    for id in [MoveId::LowPunch, MoveId::LowKick, MoveId::HighKick, MoveId::HighPunch,
               MoveId::Sweep, MoveId::Throw, MoveId::FlyingKick] {
        assert!(base.recovery > move_data(id).recovery,
            "a special recovers faster than a {id:?} — it has to be punishable or it \
             is the only move anyone would throw");
    }
}

// ---- the turtle call and the web -------------------------------------------

/// A turtle against RYUKA, the call already made; runs the helpers until
/// they have all gone, the opponent holding `hold`.
pub(crate) fn turtle_call(hold: fn(f32) -> Input) -> (Game, Vec<(i32, bool, bool)>) {
    let turtle = FIGHTERS.iter().position(|a| a.build == Build::Turtle).unwrap();
    let mut g = Game::new();
    g.p = [at(turtle, 200.0, 1.0), at(0, 420.0, -1.0)];
    face_off(&mut g.p);
    g.call_turtles(0);
    let mut landed = vec![];
    for _ in 0..600 {
        let holds = [Input::default(), hold(g.p[1].facing)];
        landed.extend(update_helpers(&mut g, F, holds));
        advance(&mut g.p[1], F);
        if g.helpers.iter().all(|h| h.is_none()) { break; }
    }
    (g, landed)
}

#[test]
pub(crate) fn a_turtles_first_special_of_the_round_calls_the_other_three() {
    let turtle = FIGHTERS.iter().position(|a| a.build == Build::Turtle).unwrap();
    let mut f = at(turtle, 200.0, 1.0);
    f.start_attack(MoveId::Special);
    assert!(f.calling && f.called);
    assert!(f.hit_box().is_none(), "the caller struck a blow of his own");
    // The second one is his own special again.
    f.act = Act::Idle;
    f.start_attack(MoveId::Special);
    assert!(!f.calling, "he called twice in a round");
    // Nobody else calls anybody.
    let mut other = at(0, 200.0, 1.0);
    other.start_attack(MoveId::Special);
    assert!(!other.calling);

    let (g, landed) = turtle_call(|_| Input::default());
    assert!(g.helpers.iter().all(|h| h.is_none()), "a helper never left the stage");
    assert_eq!(landed.len(), 3, "the three did not each land a blow: {landed:?}");
    assert!(landed.iter().all(|&(dmg, blocked, _)| dmg > 0 && !blocked));
    assert!(landed[2].2, "the last of them did not put the opponent down");
    assert!(g.p[1].health < FIGHTERS[0].health);
}

#[test]
pub(crate) fn the_helpers_are_three_different_turtles_and_never_the_caller() {
    for turtle in (0..FIGHTERS.len()).filter(|&w| FIGHTERS[w].build == Build::Turtle) {
        let mut g = Game::new();
        g.p[0] = at(turtle, 200.0, 1.0);
        g.call_turtles(0);
        let mut who: Vec<usize> = g.helpers.iter().flatten().map(|h| h.f.who).collect();
        who.sort();
        who.dedup();
        assert_eq!(who.len(), 3);
        assert!(!who.contains(&turtle));
        assert!(who.iter().all(|&w| FIGHTERS[w].build == Build::Turtle));
    }
}

#[test]
pub(crate) fn against_a_turtle_only_the_two_not_fighting_answer_the_call() {
    let turtles: Vec<usize> = (0..FIGHTERS.len()).filter(|&w| FIGHTERS[w].build == Build::Turtle).collect();
    let mut g = Game::new();
    g.p = [at(turtles[0], 200.0, 1.0), at(turtles[1], 420.0, -1.0)];
    // Both call: neither set of helpers takes the other's place.
    g.call_turtles(0);
    g.call_turtles(1);
    for side in 0..2 {
        let mine: Vec<Helper> = g.helpers.iter().flatten().filter(|h| h.side == side).copied().collect();
        let who: Vec<usize> = mine.iter().map(|h| h.f.who).collect();
        assert_eq!(who, [turtles[2], turtles[3]], "side {side} called the wrong turtles");
        assert!(mine.last().unwrap().mv == MoveId::Sweep, "the last one in does not sweep");
    }
}

#[test]
pub(crate) fn one_low_guard_answers_the_whole_call() {
    let (g, landed) = turtle_call(|facing| back_input(facing, true));
    assert_eq!(landed.len(), 3);
    assert!(landed.iter().all(|&(dmg, blocked, _)| dmg == 0 && blocked), "{landed:?}");
    assert_eq!(g.p[1].health, FIGHTERS[0].health);
}

#[test]
pub(crate) fn a_web_holds_for_five_seconds_or_two_blows() {
    let mut f = at(0, 300.0, 1.0);
    assert!(f.web());
    assert_eq!(f.webbed, WEB_SECS);
    assert_eq!(WEB_SECS, 5.0);
    // One blow leaves it on; the second takes it off.
    f.hurt(5, 1.0);
    assert!(f.webbed > 0.0);
    f.hurt(5, 1.0);
    assert_eq!(f.webbed, 0.0, "two blows did not break the web");
    // And another does not hold straight away.
    assert!(!f.web(), "webbed again the moment it came off");
    for _ in 0..(WEB_REST / F) as usize + 2 { advance(&mut f, F); }
    assert!(f.web());
    // Left alone it lets go by itself.
    for _ in 0..(WEB_SECS / F) as usize + 2 { advance(&mut f, F); }
    assert_eq!(f.webbed, 0.0);
    assert!(f.web_rest > 0.0);
}

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
