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
        let mut who: Vec<usize> = g.helpers.iter().map(|h| h.unwrap().f.who).collect();
        who.sort();
        who.dedup();
        assert_eq!(who.len(), 3);
        assert!(!who.contains(&turtle));
        assert!(who.iter().all(|&w| FIGHTERS[w].build == Build::Turtle));
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
