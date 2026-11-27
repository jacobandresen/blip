use super::*;

#[test]
fn a_wrapping_shot_hits_rocks_on_both_sides_of_the_seam() {
    assert!(wrapped_segment_circle_overlap(
        (PLAY_W as f32 - 3.0, 200.0),
        (PLAY_W as f32 + 5.0, 200.0),
        (2.0, 200.0),
        4.0,
    ));
    assert!(wrapped_segment_circle_overlap(
        (300.0, PLAY_Y0 + 3.0),
        (300.0, PLAY_Y0 - 5.0),
        (300.0, PLAY_Y0 + PLAY_H as f32 - 2.0),
        4.0,
    ));
}

#[test]
fn a_wrapping_shot_does_not_hit_through_the_middle_of_the_screen() {
    assert!(!wrapped_segment_circle_overlap(
        (PLAY_W as f32 - 3.0, 200.0),
        (PLAY_W as f32 + 5.0, 200.0),
        (PLAY_W as f32 / 2.0, 200.0),
        4.0,
    ));
}

fn rocks(g: &Game) -> Vec<ASize> {
    g.asteroids.iter().filter(|a| a.active).map(|a| a.size).collect()
}

#[test]
fn a_jump_lands_clear_of_every_rock() {
    let mut g = Game::new();
    g.asteroids = [ASTEROID_OFF; MAX_ASTEROIDS];
    for (x, y) in [(100.0, 150.0), (300.0, 300.0), (500.0, 200.0), (200.0, 400.0)] {
        spawn_rock(&mut g, x, PLAY_Y0 + y - 100.0, ASize::Large, 0.0, 0.0);
    }
    for _ in 0..200 {
        let (x, y) = landing_spot(&g);
        for a in g.asteroids.iter().filter(|a| a.active) {
            assert!((x - a.x).hypot(y - a.y) > a.size.radius() + SHIP_RADIUS,
                "landed on a rock at ({x}, {y})");
        }
    }
}

#[test]
fn a_jump_lands_clear_of_the_saucer() {
    let mut g = Game::new();
    g.asteroids = [ASTEROID_OFF; MAX_ASTEROIDS];
    g.saucer = Saucer { active: true, x: PLAY_W as f32 / 2.0, y: PLAY_Y0 + PLAY_H as f32 / 2.0, vx: 0.0, wave_t: 0.0, fire_t: 9.0, big: true };
    for _ in 0..200 {
        let (x, y) = landing_spot(&g);
        assert!((x - g.saucer.x).hypot(y - g.saucer.y) > saucer_radius(true) + SHIP_RADIUS, "landed on the saucer at ({x}, {y})");
    }
}

#[test]
fn a_rock_splits_into_two_of_the_next_size_and_the_smallest_into_none() {
    let mut g = Game::new();
    g.asteroids = [ASTEROID_OFF; MAX_ASTEROIDS];
    split_rock(&mut g, 300.0, 300.0, ASize::Large, 10.0, 0.0);
    assert!(rocks(&g) == vec![ASize::Medium, ASize::Medium]);
    g.asteroids = [ASTEROID_OFF; MAX_ASTEROIDS];
    split_rock(&mut g, 300.0, 300.0, ASize::Small, 10.0, 0.0);
    assert!(rocks(&g).is_empty());
}

#[test]
fn the_halves_fly_apart_on_top_of_the_parents_drift() {
    let mut g = Game::new();
    g.asteroids = [ASTEROID_OFF; MAX_ASTEROIDS];
    split_rock(&mut g, 300.0, 300.0, ASize::Large, 40.0, -20.0);
    let halves: Vec<_> = g.asteroids.iter().filter(|a| a.active).collect();
    // Each half's kick is along the same axis, in opposite directions.
    let k0 = (halves[0].vx - 40.0, halves[0].vy + 20.0);
    let k1 = (halves[1].vx - 40.0, halves[1].vy + 20.0);
    assert!(k0.0 * k1.0 + k0.1 * k1.1 < 0.0, "the halves should part");
}

#[test]
fn a_ship_and_rock_touching_across_the_edge_collide() {
    let r = ASize::Large.radius() + SHIP_RADIUS;
    assert!(wrapped_motion_overlap((2.0, 300.0), (PLAY_W as f32 - 2.0, 300.0), (0.0, 0.0), (0.0, 0.0), r));
    assert!(!wrapped_motion_overlap((2.0, 300.0), (PLAY_W as f32 / 2.0, 300.0), (0.0, 0.0), (0.0, 0.0), r));
}

#[test]
fn saucers_come_and_shoot_more_often_each_level_down_to_a_floor() {
    blip::testing::never_rises("saucer cooldown", 1..=8, saucer_fire_cooldown);
    blip::testing::never_rises("saucer spawn", 1..=8, |l| saucer_spawn_range(l).0);
    assert!(saucer_fire_cooldown(50) >= 0.7);
    assert!(saucer_spawn_range(50).0 >= 4.0);
}
