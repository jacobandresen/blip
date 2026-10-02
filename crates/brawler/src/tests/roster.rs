//! The fighters themselves: what power, reach and size do, and the one nothing hurts.

use super::*;

// ---- archetypes ----------------------------------------------------------

#[test]
pub(crate) fn the_fighters_are_actually_different() {
    // Palette swaps would not be fighters. Somebody has to be the best at
    // each thing, and pay for it.
    // (The invincible one is outside every trade, on purpose.)
    let fair = || FIGHTERS.iter().enumerate().filter(|(_, a)| !a.invincible);
    let fastest = fair().max_by(|a, b| a.1.walk.total_cmp(&b.1.walk)).unwrap().0;
    let strongest = fair().max_by(|a, b| a.1.power.total_cmp(&b.1.power)).unwrap().0;
    let toughest = fair().max_by_key(|(_, a)| a.health).unwrap().0;
    assert_ne!(fastest, strongest, "the fastest fighter is also the strongest");
    // The heavies pay in speed for what they hit with and what they take.
    for heavy in [strongest, toughest] {
        assert!(FIGHTERS[heavy].walk < 100.0, "{} is heavy and quick", FIGHTERS[heavy].name);
    }

    // And the fast one pays for it.
    let fast = FIGHTERS[fastest];
    assert!(fast.power < 1.0 && fast.health < 100, "the fast fighter pays no price");
    assert!(fast.reach > 1.0, "the fast fighter has nothing to poke with");

    // No two fighters share a name or a full set of numbers.
    for (i, a) in FIGHTERS.iter().enumerate() {
        for b in &FIGHTERS[i + 1..] {
            assert_ne!(a.name, b.name);
            assert!(a.walk != b.walk || a.power != b.power || a.health != b.health,
                "{} and {} play the same", a.name, b.name);
        }
    }
    // Every special is somebody's.
    for sp in [Special::ChiBolt, Special::BullRush, Special::TalonKick, Special::LaserVision] {
        assert!(FIGHTERS.iter().any(|a| a.special == sp), "nobody has {sp:?}");
    }
}

#[test]
pub(crate) fn power_and_reach_scale_the_shared_move_table() {
    // The archetype has to reach into the numbers, or "power 1.35" is
    // decoration.
    let brutus = at(1, 0.0, 1.0);
    let kestrel = at(2, 0.0, 1.0);
    let base = move_data(MoveId::HighKick);
    assert!(brutus.scaled(base).damage > base.damage);
    assert!(kestrel.scaled(base).damage < base.damage);
    assert!(kestrel.scaled(base).reach > brutus.scaled(base).reach,
        "the lighter fighter should out-range the heavy one");
    // Size scales the whole fighter: a turtle half the height has half the
    // body to hit and little more than half the reach, the giant half as
    // much again of both.
    let (small, big) = (at(3, 0.0, 1.0), at(8, 0.0, 1.0));
    assert!(small.height() < kestrel.height() * 0.55 && big.height() > kestrel.height() * 1.45);
    assert!(small.scaled(base).reach < brutus.scaled(base).reach);
    assert!(big.scaled(base).reach > kestrel.scaled(base).reach);
}

// ---- sizes ---------------------------------------------------------------

#[test]
pub(crate) fn nobody_strikes_over_the_head_of_a_smaller_opponent() {
    // Blows are aimed at the body in front of them: every grounded attack
    // of every fighter, thrown from just inside its range, touches every
    // other standing fighter, however small.
    let mut bad = vec![];
    for a in 0..FIGHTERS.len() {
        for d in 0..FIGHTERS.len() {
            for mv in [MoveId::LowPunch, MoveId::HighPunch, MoveId::LowKick, MoveId::HighKick,
                       MoveId::Sweep, MoveId::Throw] {
                let mut p = [at(a, 200.0, 1.0), at(d, 300.0, -1.0)];
                face_off(&mut p);
                p[1].x = p[0].x + attack_range(&p[0], mv) - 3.0;
                wind_to_active(&mut p[0], mv);
                let [atk, def] = &mut p;
                let (dmg, _, _) = resolve_hit(atk, def, Input::default());
                if dmg == 0 {
                    bad.push(format!("{}'s {mv:?} misses {}", FIGHTERS[a].name, FIGHTERS[d].name));
                }
            }
        }
    }
    assert!(bad.is_empty(), "attacks that cannot land:\n  {}", bad.join("\n  "));
}

#[test]
pub(crate) fn a_throw_is_offered_only_where_it_reaches() {
    // "Close enough to throw" is each fighter's own distance: a turtle has
    // to be nearer than a giant does.
    for a in 0..FIGHTERS.len() {
        for d in 0..FIGHTERS.len() {
            let mut p = [at(a, 200.0, 1.0), at(d, 300.0, -1.0)];
            face_off(&mut p);
            p[1].x = p[0].x + throw_range(&p[0]) - 0.5;
            assert!(face_off(&mut p)[0]);
            assert!(throw_range(&p[0]) < attack_range(&p[0], MoveId::Throw),
                "{} is offered a throw that cannot reach {}", FIGHTERS[a].name, FIGHTERS[d].name);
            // And it is a distance the two can actually stand at.
            assert!(throw_range(&p[0]) > (p[0].width() + p[1].width()) / 2.0 * 0.82,
                "{} can never get close enough to throw {}", FIGHTERS[a].name, FIGHTERS[d].name);
        }
    }
    let (small, big) = (at(3, 0.0, 1.0), at(8, 0.0, 1.0));
    assert!(throw_range(&small) < throw_range(&big));
}

// ---- the one nothing hurts -----------------------------------------------

#[test]
pub(crate) fn only_the_giant_hurts_the_invincible_fighter_but_blows_still_land() {
    let who = FIGHTERS.iter().position(|a| a.invincible).expect("somebody is invincible");
    let full = FIGHTERS[who].health;
    for mv in [MoveId::LowPunch, MoveId::HighKick, MoveId::Sweep, MoveId::Throw, MoveId::Special] {
        let mut p = [at(1, 200.0, 1.0), at(who, 300.0, -1.0)];
        face_off(&mut p);
        p[1].x = p[0].x + attack_range(&p[0], mv) - 3.0;
        wind_to_active(&mut p[0], mv);
        let [atk, def] = &mut p;
        let (dmg, blocked, _) = resolve_hit(atk, def, Input::default());
        assert!(dmg > 0 && !blocked, "{mv:?} did not land");
        assert_eq!(def.health, full, "{mv:?} hurt the fighter nothing hurts");
        assert!(matches!(def.act, Act::Hitstun | Act::Knockdown), "{mv:?} did not move them");
    }
    // Chip through a guard costs nothing either.
    let mut p = [at(1, 200.0, 1.0), at(who, 300.0, -1.0)];
    face_off(&mut p);
    p[1].x = p[0].x + attack_range(&p[0], MoveId::Special) - 3.0;
    wind_to_active(&mut p[0], MoveId::Special);
    let [atk, def] = &mut p;
    resolve_hit(atk, def, back_input(-1.0, false));
    assert_eq!(def.health, full);

    // Only someone bigger gets through, and only a little.
    let giant = FIGHTERS.iter().position(|a| a.size > FIGHTERS[who].size).unwrap();
    let mut p = [at(giant, 200.0, 1.0), at(who, 300.0, -1.0)];
    face_off(&mut p);
    p[1].x = p[0].x + attack_range(&p[0], MoveId::HighKick) - 3.0;
    wind_to_active(&mut p[0], MoveId::HighKick);
    let [atk, def] = &mut p;
    let (dmg, _, _) = resolve_hit(atk, def, Input::default());
    let lost = full - def.health;
    assert!(lost > 0 && lost * 3 < dmg, "the giant's {dmg} cost them {lost}");
}

#[test]
pub(crate) fn the_level_laser_hits_whoever_stands_and_goes_over_a_crouch() {
    let who = FIGHTERS.iter().position(|a| a.special == Special::LaserVision).unwrap();
    for foe in [0, 3] {
        let mut g = Game::new();
        g.p = [at(who, 200.0, 1.0), at(foe, 400.0, -1.0)];
        face_off(&mut g.p);
        g.spawn_bolt(0, 16);
        let b = g.bolts.iter().find(|b| b.active).unwrap();
        let (bx, by) = (0.0, b.y - LASER_THICK / 2.0);
        let stand = g.p[1].hurt_box();
        assert!(rects_overlap(bx, by, 20.0, LASER_THICK, 0.0, stand.1, stand.2, stand.3),
            "the laser misses a standing {}", FIGHTERS[foe].name);
        g.p[1].act = Act::Crouch;
        let duck = g.p[1].hurt_box();
        assert!(!rects_overlap(bx, by, 20.0, LASER_THICK, 0.0, duck.1, duck.2, duck.3),
            "{} cannot duck the laser", FIGHTERS[foe].name);
    }
}

#[test]
pub(crate) fn the_laser_fired_from_the_air_is_aimed_at_the_fighter() {
    // Level, it would pass over everyone's head. From anywhere in the air,
    // either side, it has to reach whoever it was fired at, big or small.
    let who = FIGHTERS.iter().position(|a| a.special == Special::LaserVision).unwrap();
    for foe in [0, 3, 8] {
        for (x, up) in [(120.0, 140.0), (300.0, 60.0), (520.0, 150.0), (330.0, 140.0)] {
            let mut g = Game::new();
            g.p = [at(who, x, 1.0), at(foe, 320.0, -1.0)];
            g.p[0].y = FLOOR_Y - up;
            g.p[0].act = Act::Air;
            face_off(&mut g.p);
            g.spawn_bolt(0, 16);
            let mut b = *g.bolts.iter().find(|b| b.active).unwrap();
            let (hx, hy, hw, hh) = g.p[1].hurt_box();
            let mut hit = false;
            for _ in 0..120 {
                b.x += b.vx * F;
                b.y += b.vy * F;
                if rects_overlap(b.x - 10.0, b.y - 8.0, 20.0, 16.0, hx, hy, hw, hh) { hit = true; break; }
                if b.y > FLOOR_Y { break; }
            }
            assert!(hit, "fired from ({x}, {up} up) the laser missed {}", FIGHTERS[foe].name);
        }
    }
}

#[test]
pub(crate) fn the_flier_hangs_in_the_air_until_told_to_land() {
    let who = FIGHTERS.iter().position(|a| a.build == Build::Caped).unwrap();
    let mut f = at(who, 300.0, 1.0);
    let hold = |up: bool, down: bool, right: bool| Input { up, down, right, ..Default::default() };
    let step = |f: &mut Fighter, inp: Input, frames: usize| {
        for _ in 0..frames { apply_input(f, inp, false, F); advance(f, F); }
    };
    step(&mut f, hold(true, false, false), 30);
    let high = FLOOR_Y - f.y;
    assert!(high > 60.0, "half a second of up lifted him {high:.0}px");
    // Hands off: he stays exactly where he is.
    step(&mut f, Input::default(), 90);
    assert!((FLOOR_Y - f.y - high).abs() < 0.5 && f.act == Act::Air, "he did not hover");
    // The stick moves him without gravity, and never above the ceiling.
    let x = f.x;
    step(&mut f, hold(true, false, true), 120);
    assert!(f.x > x + 100.0, "he did not fly forward");
    assert!(FLOOR_Y - f.y <= CEILING + 0.5, "he flew off the top of the screen");
    // Down brings him back to the boards, free to act.
    step(&mut f, hold(false, true, false), 120);
    assert!(!f.airborne() && f.free(), "he never landed: {:?} at {:.0}", f.act, FLOOR_Y - f.y);
    // Nobody else flies.
    let mut g = at(0, 300.0, 1.0);
    step(&mut g, hold(true, false, false), 5);
    step(&mut g, Input::default(), 120);
    assert!(!g.airborne());
}

#[test]
pub(crate) fn the_fliers_kick_is_the_laser_and_his_punch_hits_three_times_as_hard() {
    let who = FIGHTERS.iter().position(|a| a.build == Build::Caped).unwrap();
    let f = at(who, 300.0, 1.0);
    for kick in [Input { kick_low: true, ..Default::default() },
                 Input { kick_high: true, ..Default::default() },
                 Input { kick_low: true, down: true, ..Default::default() }] {
        assert_eq!(pressed_move(&f, kick, true), Some(MoveId::Special));
    }
    let punch = Input { punch_low: true, ..Default::default() };
    assert_eq!(pressed_move(&f, punch, true), Some(MoveId::HighPunch), "he throws, or jabs");
    let base = move_data(MoveId::HighPunch).damage;
    assert_eq!(f.scaled(move_data(MoveId::HighPunch)).damage, base * 3);
    // And it sends them flying.
    let mut p = [f, at(1, 300.0, -1.0)];
    face_off(&mut p);
    p[1].x = p[0].x + attack_range(&p[0], MoveId::HighPunch) - 3.0;
    wind_to_active(&mut p[0], MoveId::HighPunch);
    let [atk, def] = &mut p;
    let (dmg, _, floored) = resolve_hit(atk, def, Input::default());
    assert!(dmg == base * 3 && floored && def.act == Act::Knockdown);
}

#[test]
pub(crate) fn the_last_fight_is_won_by_being_on_your_feet_at_the_bell() {
    // Nobody but the giant can knock out the fighter nothing hurts, so a
    // round against him goes to the player who lasts it out.
    let boss = FIGHTERS.iter().position(|a| a.invincible).unwrap();
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(RUNGS - 1);
    assert_eq!(g.p[1].who, boss);
    g.p[0].health = 7;
    assert!(survived(&g));
    assert_eq!(round_result(&g, false), RoundResult::P1, "time ran out and he still took the round");
    // Knocked out is knocked out.
    g.p[0].health = 0;
    assert_eq!(round_result(&g, true), RoundResult::P2);
    // Against anyone else the bell goes to whoever has more left.
    g.start_match(0);
    g.p[0].health = 7;
    assert!(!survived(&g));
    assert_eq!(round_result(&g, false), RoundResult::P2);
    // And two players settle it on health, whoever they picked.
    g.mode = Mode::Versus;
    g.p[1] = at(boss, 440.0, -1.0);
    assert!(!survived(&g));
}

#[test]
pub(crate) fn the_ladder_gets_harder_all_the_way_up() {
    let mut g = Game { pick: 0, ..Game::new() };
    let mut last = f32::MIN;
    for rung in 0..RUNGS {
        g.start_match(rung);
        assert!(g.difficulty > last, "fight {} is no harder than the one before", rung + 1);
        last = g.difficulty;
    }
    assert!((last - LAST_RUNG).abs() < 1e-4);
}
