//! How it feels in the hands: buffering, turning, response times.

use super::*;

// ---- feel ----------------------------------------------------------------

#[test]
pub(crate) fn an_attack_pressed_during_recovery_comes_out_when_recovery_ends() {
    // The difference between "I was a frame early" and "this game
    // ignored me". A player pressing punch at the end of a sweep is
    // asking for a punch, and should get one.
    let mut f = at(0, 200.0, 1.0);
    f.start_attack(MoveId::Sweep);
    let sweep = f.scaled(move_data(MoveId::Sweep));
    let total = (sweep.startup + sweep.active + sweep.recovery) * F;

    let mut punch = Input::default();
    punch.punch_low = true;
    // Press with a few frames of recovery still to run.
    f.t = total - 4.0 * F;
    apply_input(&mut f, punch, false, F);
    assert_eq!(f.mv, MoveId::Sweep, "the press interrupted the sweep");
    assert_eq!(f.buffered, Some(MoveId::LowPunch), "the press was thrown away");

    // Let the sweep finish, then hold nothing at all.
    for _ in 0..5 { advance(&mut f, F); }
    apply_input(&mut f, Input::default(), false, F);
    assert_eq!(f.act, Act::Attack, "the buffered punch never came out");
    assert_eq!(f.mv, MoveId::LowPunch);
}

#[test]
pub(crate) fn a_buffered_attack_is_forgotten_if_it_waits_too_long() {
    // A buffer that never expires fires attacks the player asked for
    // seconds ago, which is its own kind of not listening.
    let mut f = at(0, 200.0, 1.0);
    f.act = Act::Hitstun;
    f.stun = 1.0;
    let mut punch = Input::default();
    punch.punch_low = true;
    apply_input(&mut f, punch, false, F);
    assert!(f.buffered.is_some());

    for _ in 0..20 { apply_input(&mut f, Input::default(), false, F); }
    assert_eq!(f.buffered, None, "the buffer held an input far past its welcome");
}

#[test]
pub(crate) fn a_buffered_move_remembers_the_stance_it_was_asked_for_in() {
    // Down+kick pressed in recovery is a sweep when it comes out, whatever
    // the stick does meanwhile.
    let mut f = at(0, 200.0, 1.0);
    f.start_attack(MoveId::HighKick);
    let mut low = Input::default();
    low.down = true;
    low.kick_low = true;
    apply_input(&mut f, low, false, F);
    assert_eq!(f.buffered, Some(MoveId::Sweep));
}

#[test]
pub(crate) fn the_heavier_the_hit_the_longer_the_world_stops() {
    // Hitstop is how weight is communicated. If every hit froze for the
    // same time, a jab and a knockdown would feel identical.
    let block = hitstop_for(0, false, true);
    let light = hitstop_for(6, false, false);
    let heavy = hitstop_for(13, false, false);
    let knock = hitstop_for(13, true, false);
    assert!(block < light, "a blocked hit should feel lighter than a landed one");
    assert!(light < heavy, "a jab should not land like a kick");
    assert!(heavy < knock, "a knockdown should be the heaviest thing in the game");
    assert!(knock < 0.2, "the freeze is long enough to be a pause rather than a hit");
}

#[test]
pub(crate) fn a_knockdown_throws_the_two_fighters_apart() {
    // Landing one has to be a reward. Standing over the wakeup is not a
    // reward — it is a guess against invulnerability, and on the
    // receiving end it reads as being stomped.
    let mut p = [at(0, 280.0, 1.0), at(1, 330.0, -1.0)];
    let before = (p[1].x - p[0].x).abs();
    push_apart(&mut p, 34.0);
    let after = (p[1].x - p[0].x).abs();
    assert!(after > before + 50.0,
        "a knockdown left them {after:.0}px apart, barely more than the {before:.0} they started at");
}
