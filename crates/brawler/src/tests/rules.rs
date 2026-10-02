//! The rules of a blow: guard heights, frame data, states, combos, throws, the dizzy spell and the uppercut.

use super::*;

// ---- blocking is a choice ------------------------------------------------

#[test]
pub(crate) fn a_standing_block_stops_a_mid_and_a_jump_in_but_not_a_sweep() {
    // The whole reason attacks have heights. If standing block covered
    // everything, there would be no reason to ever do anything else, and
    // no reason for the attacker to pick one move over another.
    assert!(blocks(Level::Mid, false), "a mid should be blockable standing");
    assert!(blocks(Level::Overhead, false), "a jump-in should be blockable standing");
    assert!(!blocks(Level::Low, false), "a sweep must beat a standing block");
}

#[test]
pub(crate) fn a_crouching_block_stops_a_sweep_but_not_a_jump_in() {
    assert!(blocks(Level::Low, true));
    assert!(blocks(Level::Mid, true));
    assert!(!blocks(Level::Overhead, true), "a jump-in must beat a crouching block");
}

#[test]
pub(crate) fn blocking_requires_holding_away_from_the_opponent() {
    // Holding toward them is walking into it. This is what makes a
    // player commit: you cannot advance and be safe at the same time.
    assert!(holding_back(back_input(1.0, false), 1.0));
    let mut forward = Input::default();
    forward.right = true;
    assert!(!holding_back(forward, 1.0), "holding forward is not a block");
}

#[test]
pub(crate) fn a_sweep_goes_through_a_standing_block_and_knocks_down() {
    let mut atk = at(0, 200.0, 1.0);
    let mut def = at(1, 240.0, -1.0);
    wind_to_active(&mut atk, MoveId::Sweep);
    let (dmg, blocked, knocked) = resolve_hit(&mut atk, &mut def, back_input(-1.0, false));
    assert!(!blocked, "a standing block stopped a low attack");
    assert!(dmg > 0);
    assert!(knocked && def.act == Act::Knockdown, "a sweep should put them on the floor");
}

#[test]
pub(crate) fn the_same_sweep_is_blocked_by_crouching() {
    let mut atk = at(0, 200.0, 1.0);
    let mut def = at(1, 240.0, -1.0);
    wind_to_active(&mut atk, MoveId::Sweep);
    let (dmg, blocked, _) = resolve_hit(&mut atk, &mut def, back_input(-1.0, true));
    assert!(blocked && dmg == 0, "crouch-blocking should stop a sweep");
    assert_eq!(def.health, FIGHTERS[def.who].health, "a blocked sweep should do no damage");
}

#[test]
pub(crate) fn a_jump_attack_beats_a_crouching_block() {
    let mut atk = at(0, 200.0, 1.0);
    atk.y = FLOOR_Y - 40.0;
    let mut def = at(1, 236.0, -1.0);
    wind_to_active(&mut atk, MoveId::JumpKick);
    let (dmg, blocked, _) = resolve_hit(&mut atk, &mut def, back_input(-1.0, true));
    assert!(!blocked && dmg > 0, "a crouch block should not cover an overhead");
}

// ---- frame data ----------------------------------------------------------

#[test]
pub(crate) fn an_attack_can_only_hit_during_its_active_window() {
    // Startup you can walk into; recovery you can punish. If a move
    // could hit across its whole animation, neither would exist.
    let mut f = at(0, 200.0, 1.0);
    let m = f.scaled(move_data(MoveId::HighKick));
    f.start_attack(MoveId::HighKick);

    f.t = m.startup * F * 0.5;
    assert!(f.hit_box().is_none(), "a kick hit during its startup");

    f.t = (m.startup + m.active * 0.5) * F;
    assert!(f.hit_box().is_some(), "a kick did not hit during its active frames");

    f.t = (m.startup + m.active + m.recovery * 0.5) * F;
    assert!(f.hit_box().is_none(), "a kick hit during its recovery");
}

#[test]
pub(crate) fn a_slow_frame_cannot_step_over_the_active_window() {
    // One long frame (a slow phone) that starts before the active window
    // and would end after it must still stop in it.
    for id in [MoveId::LowPunch, MoveId::HighKick] {
        let mut f = at(0, 200.0, 1.0);
        let m = f.scaled(move_data(id));
        f.start_attack(id);
        advance(&mut f, (m.startup + m.active) * F + 0.01);
        assert!(f.hit_box().is_some(), "{id:?}: a long frame stepped over its active window");
    }
}

#[test]
pub(crate) fn one_attack_lands_one_hit() {
    // Four active frames is four chances to overlap. Without the latch
    // a kick would do four times its listed damage.
    let mut atk = at(0, 200.0, 1.0);
    let mut def = at(1, 240.0, -1.0);
    wind_to_active(&mut atk, MoveId::HighKick);
    let first = resolve_hit(&mut atk, &mut def, Input::default());
    let second = resolve_hit(&mut atk, &mut def, Input::default());
    assert!(first.0 > 0);
    assert_eq!(second.0, 0, "the same attack hit twice");
}

#[test]
pub(crate) fn the_slow_moves_are_the_punishable_ones() {
    // The core trade of the whole game: reach and damage are paid for in
    // recovery. If a move were both long and safe there would be nothing
    // to think about.
    let jab = move_data(MoveId::LowPunch);
    // The low kick is the poke this is about: longer than a jab and
    // slower to recover from, with the sweep costing more than either.
    let kick = move_data(MoveId::LowKick);
    let sweep = move_data(MoveId::Sweep);
    assert!(kick.reach > jab.reach && kick.recovery > jab.recovery,
        "the longer poke must also be the slower one to recover from");
    assert!(sweep.knockdown && sweep.recovery > kick.recovery,
        "a knockdown must cost more on whiff than a normal poke");
    assert!(jab.startup < kick.startup && jab.damage < kick.damage,
        "the fast move must be the weak one");
}

#[test]
pub(crate) fn hitstun_always_exceeds_blockstun() {
    // Otherwise blocking would be worse than being hit, and the game
    // would reward standing still and eating everything.
    for id in [MoveId::LowPunch, MoveId::LowKick, MoveId::HighKick, MoveId::HighPunch,
               MoveId::JumpPunch, MoveId::JumpKick, MoveId::FlyingKick, MoveId::Special] {
        let m = move_data(id);
        assert!(m.hitstun > m.blockstun, "{id:?} punishes blocking more than getting hit");
    }
}

// ---- states --------------------------------------------------------------

#[test]
pub(crate) fn you_cannot_act_while_you_are_busy() {
    // Being able to cancel recovery by pressing another button would
    // delete the entire punish game.
    let mut f = at(0, 200.0, 1.0);
    f.start_attack(MoveId::Sweep);
    let before = f.mv;
    let mut jab = Input::default();
    jab.punch_low = true;
    apply_input(&mut f, jab, false, F);
    assert_eq!(f.mv, before, "an attack was cancelled into another attack");
    assert_eq!(f.act, Act::Attack);

    f.act = Act::Hitstun;
    f.stun = 10.0 * F;
    apply_input(&mut f, jab, false, F);
    assert_eq!(f.act, Act::Hitstun, "a fighter attacked out of hitstun");
}

#[test]
pub(crate) fn a_knockdown_ends_with_a_moment_of_invulnerability() {
    // A knockdown that could be hit again on the way up is not a
    // knockdown, it is a loop — one sweep and the round is over.
    let mut f = at(0, 200.0, 1.0);
    f.act = Act::Knockdown;
    f.t = 0.2;
    assert!(!invulnerable(&f), "a fighter lying down should still be a target early");
    f.t = 1.0;
    assert!(invulnerable(&f), "there is no safe wakeup");

    // And it ends: the state cannot be held forever.
    f.t = 1.2;
    advance(&mut f, F);
    assert_eq!(f.act, Act::Idle, "a knockdown never got up");
}

#[test]
pub(crate) fn an_invulnerable_wakeup_cannot_be_hit() {
    let mut atk = at(0, 200.0, 1.0);
    let mut def = at(1, 240.0, -1.0);
    def.act = Act::Knockdown;
    def.t = 1.0;
    wind_to_active(&mut atk, MoveId::HighKick);
    let (dmg, _, _) = resolve_hit(&mut atk, &mut def, Input::default());
    assert_eq!(dmg, 0, "a waking fighter was hit through their invulnerability");
}

#[test]
pub(crate) fn health_stops_at_zero() {
    let mut atk = at(1, 200.0, 1.0); // Brutus, the hardest hitter
    let mut def = at(2, 236.0, -1.0);
    def.health = 3;
    wind_to_active(&mut atk, MoveId::HighKick);
    resolve_hit(&mut atk, &mut def, Input::default());
    assert_eq!(def.health, 0, "health went negative and the bar would draw backwards");
}

// ---- the stage -----------------------------------------------------------

#[test]
pub(crate) fn nobody_leaves_the_stage() {
    let mut p = [at(0, WALL_MARGIN, 1.0), at(1, WIN_W as f32 - WALL_MARGIN, -1.0)];
    push_apart(&mut p, 40.0);
    for f in p.iter() {
        assert!(f.x >= WALL_MARGIN - 0.01 && f.x <= WIN_W as f32 - WALL_MARGIN + 0.01,
            "a fighter was pushed off the stage to {}", f.x);
    }
}

#[test]
pub(crate) fn a_cornered_fighter_pushes_the_attacker_back_instead() {
    // Corner pressure only exists because the pushback has to go
    // somewhere. Without this the corner is not a place, and half the
    // reason to advance disappears.
    let corner = WIN_W as f32 - WALL_MARGIN;
    let mut p = [at(0, corner - 40.0, 1.0), at(1, corner, -1.0)];
    let attacker_before = p[0].x;
    push_apart(&mut p, 12.0);
    assert!((p[1].x - corner).abs() < 0.01, "the cornered fighter slid out of the corner");
    assert!(p[0].x < attacker_before, "the attacker was not pushed back by the corner");
}

#[test]
pub(crate) fn fighters_cannot_stand_inside_each_other() {
    let mut p = [at(0, 300.0, 1.0), at(1, 302.0, -1.0)];
    separate(&mut p);
    assert!((p[1].x - p[0].x).abs() >= BODY_W * 0.8 - 0.01,
        "two fighters occupied the same space");
}

// ---- combos --------------------------------------------------------------

#[test]
pub(crate) fn a_landed_jab_can_be_cancelled_into_something_heavier() {
    // The reward for seeing your own hit land. Without it a jab is six
    // damage and a shrug, and there is no reason to ever throw the fast
    // move rather than the big one.
    let mut atk = at(0, 280.0, 1.0);
    let mut def = at(1, 320.0, -1.0);
    wind_to_active(&mut atk, MoveId::LowPunch);
    let (dmg, _, _) = resolve_hit(&mut atk, &mut def, Input::default());
    assert!(dmg > 0);
    assert!(atk.can_cancel(), "a landed jab opened no follow-up window");

    let mut kick = Input::default();
    kick.kick_high = true;
    apply_input(&mut atk, kick, false, F);
    assert_eq!(atk.mv, MoveId::HighKick, "the follow-up never came out");
    assert!(atk.chained, "the follow-up was not marked as a chain");
}

#[test]
pub(crate) fn a_blocked_poke_buys_nothing() {
    // Pressing buttons into a guard must not be a combo. If it were,
    // blocking would be the losing option at every range.
    let mut atk = at(0, 280.0, 1.0);
    let mut def = at(1, 320.0, -1.0);
    wind_to_active(&mut atk, MoveId::LowPunch);
    let mut back = Input::default();
    back.right = true; // away from the attacker on the left
    back.down = true;  // and low, because the low punch is low
    let (_, blocked, _) = resolve_hit(&mut atk, &mut def, back);
    assert!(blocked);
    assert!(!atk.can_cancel(), "a blocked poke opened a follow-up window");
}

#[test]
pub(crate) fn a_chain_cannot_chain_again() {
    // One cancel per attack. Jab into jab into jab comes out faster than
    // the hitstun it causes, which is an infinite, which is not a game.
    let mut atk = at(0, 280.0, 1.0);
    let mut def = at(1, 320.0, -1.0);
    wind_to_active(&mut atk, MoveId::LowPunch);
    resolve_hit(&mut atk, &mut def, Input::default());
    let mut jab = Input::default();
    jab.punch_low = true;
    apply_input(&mut atk, jab, false, F);          // the one cancel
    assert!(atk.chained);

    wind_to_active(&mut atk, MoveId::LowPunch);
    atk.chained = true; // as start_attack would have left it
    let mut def2 = at(1, 320.0, -1.0);
    resolve_hit(&mut atk, &mut def2, Input::default());
    assert!(!atk.can_cancel(), "a chained attack could chain again");
}

#[test]
pub(crate) fn the_second_hit_of_a_combo_is_worth_less_than_the_first() {
    // Otherwise the reward for one read is the whole round.
    assert!(combo_scale(1) < combo_scale(0));
    assert!(combo_scale(2) < combo_scale(1));
    assert!(combo_scale(9) > 0.0, "a long combo should still do something");

    let mut atk = at(0, 280.0, 1.0);
    let mut def = at(1, 320.0, -1.0);
    wind_to_active(&mut atk, MoveId::HighKick);
    let (first, _, _) = resolve_hit(&mut atk, &mut def, Input::default());
    let mut atk2 = at(0, 280.0, 1.0);
    wind_to_active(&mut atk2, MoveId::HighKick);
    let (second, _, _) = resolve_hit(&mut atk2, &mut def, Input::default());
    assert!(second < first, "the second hit landed for full value ({second} vs {first})");
}

#[test]
pub(crate) fn getting_out_of_a_combo_resets_it() {
    let mut f = at(0, 280.0, 1.0);
    f.combo = 3;
    f.act = Act::Hitstun;
    f.stun = F;
    advance(&mut f, F * 2.0);
    assert_eq!(f.act, Act::Idle);
    assert_eq!(f.combo, 0, "the combo counter survived the recovery that ended it");
}

// ---- throws --------------------------------------------------------------

#[test]
pub(crate) fn a_throw_goes_through_any_guard() {
    // The third corner. Block beats attack, throw beats block, attack
    // beats throw — without this the game has a stalemate in it, and it
    // had a measurable one.
    for crouch in [false, true] {
        let mut atk = at(0, 300.0, 1.0);
        let mut def = at(1, 330.0, -1.0);
        wind_to_active(&mut atk, MoveId::Throw);
        let (dmg, blocked, knock) = resolve_hit(&mut atk, &mut def, back_input(-1.0, crouch));
        assert!(!blocked, "a throw was blocked (crouching: {crouch})");
        assert!(dmg > 0 && knock, "a throw should land hard and put them down");
    }
}

#[test]
pub(crate) fn a_throw_cannot_catch_someone_in_the_air() {
    // Jumping is an answer to a throw, as attacking is. Without that the
    // triangle collapses into "walk in and throw".
    let mut atk = at(0, 300.0, 1.0);
    let mut def = at(1, 330.0, -1.0);
    def.y = FLOOR_Y - 50.0;
    wind_to_active(&mut atk, MoveId::Throw);
    let (dmg, _, _) = resolve_hit(&mut atk, &mut def, Input::default());
    assert_eq!(dmg, 0, "a throw plucked someone out of the air");
}

#[test]
pub(crate) fn a_throw_only_comes_out_from_throw_range() {
    // Punch is punch at arm's length and a throw up close; at range the
    // player must still get the move they expect.
    let f = at(0, 300.0, 1.0);
    let mut punch = Input::default();
    punch.punch_low = true;
    assert_eq!(pressed_move(&f, punch, true), Some(MoveId::Throw));
    assert_eq!(pressed_move(&f, punch, false), Some(MoveId::LowPunch));
    // And crouching punch is never a throw — you cannot throw from
    // down. Holding down asks for the uppercut whichever punch button
    // found it, the same way it asks for the sweep.
    punch.down = true;
    assert_eq!(pressed_move(&f, punch, true), Some(MoveId::Uppercut));
    let mut high = Input::default();
    high.punch_high = true;
    high.down = true;
    assert_eq!(pressed_move(&f, high, false), Some(MoveId::Uppercut));
}

#[test]
pub(crate) fn a_whiffed_throw_is_the_worst_thing_you_can_do() {
    // It has to be, or throwing would simply be correct at close range.
    let throw = move_data(MoveId::Throw);
    for id in [MoveId::LowPunch, MoveId::LowKick, MoveId::HighPunch] {
        assert!(throw.recovery > move_data(id).recovery,
            "a missed throw recovers faster than a {id:?}");
    }
}

// ---- seeing stars -----------------------------------------------------------

#[test]
pub(crate) fn a_flurry_leaves_a_fighter_dizzy_once_a_round_and_a_jab_now_and_then_never_does() {
    let land = |p: &mut [Fighter; 2], id: MoveId| {
        p[1].act = Act::Idle;
        p[1].x = p[0].x + 40.0;
        wind_to_active(&mut p[0], id);
        let [atk, def] = p;
        resolve_hit(atk, def, Input::default()).0
    };
    // Heavy blows one after another.
    let mut p = [at(0, 300.0, 1.0), at(1, 340.0, -1.0)];
    face_off(&mut p);
    let mut blows = 0;
    while p[1].dizzy <= 0.0 && blows < 8 {
        assert!(land(&mut p, MoveId::HighKick) > 0);
        blows += 1;
    }
    assert!((2..=4).contains(&blows), "dizzy after {blows} heavy blows");
    assert!(p[1].stun >= DIZZY_SECS, "a dizzy fighter could act at once");
    // The next blow wakes them, and there is no second spell this round.
    land(&mut p, MoveId::LowPunch);
    assert_eq!(p[1].dizzy, 0.0);
    for _ in 0..8 { land(&mut p, MoveId::HighKick); }
    assert_eq!(p[1].dizzy, 0.0, "dizzy twice in a round");

    // A jab a second never adds up.
    let mut p = [at(0, 300.0, 1.0), at(1, 340.0, -1.0)];
    face_off(&mut p);
    for _ in 0..12 {
        land(&mut p, MoveId::LowPunch);
        for _ in 0..60 { advance(&mut p[1], F); }
        assert_eq!(p[1].dizzy, 0.0);
    }
    // Nor does anything make the one nothing hurts see stars.
    let boss = FIGHTERS.iter().position(|a| a.invincible).unwrap();
    let mut p = [at(8, 300.0, 1.0), at(boss, 340.0, -1.0)];
    face_off(&mut p);
    for _ in 0..8 { land(&mut p, MoveId::HighKick); }
    assert_eq!(p[1].dizzy, 0.0);
}

// ---- the uppercut ------------------------------------------------------------

#[test]
pub(crate) fn down_and_punch_is_the_uppercut_and_it_is_for_whoever_jumps_in() {
    let mut down_punch = Input::default();
    down_punch.down = true;
    down_punch.punch_low = true;
    assert_eq!(grounded_move(down_punch, false), Some(MoveId::Uppercut));

    let upper = |set: fn(&mut Fighter)| {
        let mut p = [at(0, 300.0, 1.0), at(1, 344.0, -1.0)];
        face_off(&mut p);
        set(&mut p[1]);
        wind_to_active(&mut p[0], MoveId::Uppercut);
        let [atk, def] = &mut p;
        let (dmg, _, floored) = resolve_hit(atk, def, Input::default());
        (dmg, floored)
    };
    // Out of the air, and onto the floor.
    let (dmg, floored) = upper(|f| { f.y = FLOOR_Y - 46.0; f.act = Act::Air; });
    assert!(dmg > 0 && floored, "a jump-in went through the uppercut");
    // Standing in front of it, the same.
    assert!(upper(|_| {}).0 > 0);
    // It goes clean over someone sitting down.
    assert_eq!(upper(|f| f.act = Act::Crouch).0, 0, "the uppercut caught a croucher");

    // And a miss is paid for: longer on the hook than any other normal.
    let u = move_data(MoveId::Uppercut);
    for other in [MoveId::LowPunch, MoveId::HighPunch, MoveId::LowKick, MoveId::HighKick, MoveId::Sweep] {
        assert!(u.recovery > move_data(other).recovery, "{other:?} recovers slower");
    }
}
