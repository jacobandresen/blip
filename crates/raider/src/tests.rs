use super::*;

#[test]
fn every_fall_style_ends_in_the_sea_near_where_it_began() {
    // A wreck that flies off the screen or goes NaN is a plane that
    // vanished instead of going in.
    let e = Game::new().enemies[0];
    let e = Enemy { x: 200.0, y: 200.0, heading: 0.3, active: true, ..e };
    let mut seen = Vec::new();
    for _ in 0..400 {
        let mut w = new_wreck(&e);
        if !seen.contains(&(w.fall as u8)) { seen.push(w.fall as u8); }
        while w.t < w.dur {
            w.t += F;
            fly_wreck(&mut w, 1.0, F);
            assert!(w.x.is_finite() && w.y.is_finite() && w.heading.is_finite());
        }
        assert!((w.x - e.x).abs() < 260.0 && (w.y - e.y).abs() < 360.0,
            "a {:?} fall ended at ({:.0}, {:.0})", w.fall as u8, w.x, w.y);
    }
    assert_eq!(seen.len(), 5, "not every fall style turned up");
}

#[test]
fn a_wreck_carries_on_the_way_the_plane_was_flying() {
    // Momentum: the first moments of any fall continue the plane's own
    // velocity, not a new direction picked for the fall.
    let e = Game::new().enemies[0];
    for heading in [0.0f32, 0.8, -1.2, 2.5] {
        let e = Enemy { x: 200.0, y: 200.0, heading, speed: 120.0, active: true, ..e };
        for _ in 0..50 {
            let mut w = new_wreck(&e);
            for _ in 0..6 { w.t += F; fly_wreck(&mut w, 0.0, F); }
            let moved = (w.y - e.y).atan2(w.x - e.x);
            let flew = heading.cos().atan2(heading.sin());
            let off = (moved - flew + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            assert!(off.abs() < 0.25, "a {} fall set off {off:.2} rad from the plane's heading", w.fall as u8);
            assert!(((w.x - e.x).hypot(w.y - e.y) - 120.0 * 6.0 * F).abs() < 3.0, "it did not keep the plane's speed");
        }
    }
}

const F: f32 = 1.0 / 60.0;

#[test]
fn each_gun_tier_is_at_least_the_last_one() {
    blip::testing::never_falls("guns", 1..=MAX_WEAPON_LEVEL, |l| weapon_guns(l).0);
    blip::testing::never_falls("fan", 1..=MAX_WEAPON_LEVEL, |l| weapon_guns(l).1);
    blip::testing::never_falls("range", 1..=MAX_WEAPON_LEVEL, |l| weapon_guns(l).2);
    blip::testing::never_rises("cooldown", 1..=MAX_WEAPON_LEVEL, weapon_cooldown);
}

#[test]
fn waves_grow_and_the_last_is_the_long_flight_in() {
    blip::testing::never_falls("wave size", 1..MAX_LEVEL, wave_target_for);
    assert!(wave_target_for(MAX_LEVEL) > wave_target_for(MAX_LEVEL - 1));
    assert!(wave_target_for(40) <= 160, "the cap holds before the last level");
}

#[test]
fn flights_come_closer_together_each_level_and_never_overlap_their_range() {
    for level in 1..10 {
        let (lo, hi) = spawn_interval_range(level);
        assert!(lo > 0.0 && lo < hi, "level {level}");
    }
    blip::testing::never_rises("longest gap", 1..=10, |l| spawn_interval_range(l).1);
}

#[test]
fn no_two_planes_in_a_formation_touch() {
    use Formation::*;
    for f in [Vic, Pair, Echelon, Abreast, FingerFour, Solo] {
        let s = slots(f);
        for (i, a) in s.iter().enumerate() {
            for b in &s[i + 1..] {
                let apart = (a.0 - b.0).abs() >= ENEMY_W as f32 || (a.1 - b.1).abs() >= ENEMY_H as f32 * 0.85;
                assert!(apart, "two slots overlap: {a:?} and {b:?}");
            }
        }
    }
}

#[test]
fn a_slot_turns_with_its_leader() {
    // Flying down the screen, "behind" is up it.
    let (x, y) = slot_pos(100.0, 100.0, 0.0, (0.0, 50.0));
    assert!((x - 100.0).abs() < 1e-4 && (y - 50.0).abs() < 1e-4);
    // Any heading keeps the wingman the same distance from the leader.
    for k in 0..12 {
        let (x, y) = slot_pos(0.0, 0.0, k as f32 * 0.5, (62.0, 50.0));
        assert!((x.hypot(y) - 62.0f32.hypot(50.0)).abs() < 1e-3);
    }
}

#[test]
fn a_flak_gun_fires_only_once_it_has_reloaded() {
    let mut gun = FlakGun { angle: 0.0, reload: 0.5, flash: 0.0 };
    assert!(!aim_flak_gun(&mut gun, 0.0, 0.0, 100.0, 100.0, 0.1, (1.0, 2.0)));
    assert!(aim_flak_gun(&mut gun, 0.0, 0.0, 100.0, 100.0, 0.5, (1.0, 2.0)));
    assert!((1.0..=2.0).contains(&gun.reload), "reload {}", gun.reload);
    assert!(gun.flash > 0.0);
}

#[test]
fn a_straight_leg_holds_its_heading_and_stays_above_the_stall() {
    let mut f = Game::new().flights[0];
    f = Flight { active: true, x: 200.0, y: 0.0, heading: 0.2, bank: 0.0, speed: 115.0, trim: 115.0,
        legs: [Leg::Straight(5.0); 4], n: 1, leg: 0, leg_t: 0.0, turned: 0.0, ..f };
    for _ in 0..120 { update_flight(&mut f, F, (200.0, 400.0, 0.0)); }
    assert!((f.heading - 0.2).abs() < 1e-4);
    assert!(f.speed >= FLIGHT_STALL);
}
