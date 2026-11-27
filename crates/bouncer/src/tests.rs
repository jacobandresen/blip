use super::*;

/// A polygon's area (shoelace).
fn area(pts: &[(f32, f32)]) -> f32 {
    (0..pts.len()).map(|k| { let (a, b) = (pts[k], pts[(k + 1) % pts.len()]); a.0 * b.1 - b.0 * a.1 }).sum::<f32>().abs() / 2.0
}

#[test]
fn a_shattered_brick_is_three_to_five_shards_that_make_up_the_whole_brick() {
    for seed in 1..40u32 {
        let mut rng = ShardRng(seed.wrapping_mul(2654435761) | 1);
        let hit = (100.0 + (seed % 7) as f32 * 6.0, 50.0 + (seed % 3) as f32 * 6.0);
        let pieces = shatter(100.0, 50.0, hit, (0.0, -200.0), BLIP_RED, &mut rng);
        let shards: Vec<&Shard> = pieces.iter().collect();
        assert!((3..=5).contains(&shards.len()), "seed {seed}: {} shards", shards.len());
        let total: f32 = shards.iter().map(|s| area(&s.pts)).sum();
        let brick = (BRICK_W * BRICK_H) as f32;
        assert!((total - brick).abs() < brick * 0.01, "seed {seed}: shards cover {total} of {brick}");
    }
}

#[test]
fn a_continue_plays_on_against_the_bricks_left_with_fresh_lives() {
    let mut g = Game::new();
    g.start_game();
    g.bricks[0].alive = false;
    g.sess.score = 700;
    g.sess.lives = 1;
    assert!(g.sess.lose_life() == LifeResult::GameOver);
    g.cont.start(CONTINUE_SECS);
    g.state = State::Over;
    g.continue_play();
    assert!(!g.bricks[0].alive, "the bricks already broken stay broken");
    assert!(g.state == State::Launch);
    assert_eq!((g.sess.lives, g.sess.score, g.sess.run_best()), (LIVES_START, 0, 700));
}

fn rot_z(t: f32) -> Mat3 { let (s, c) = t.sin_cos(); [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]] }

#[test]
fn pose_decomposition_reconstructs_orientation() {
    let r = mat_mul(&rot_z(0.7), &mat_mul(&rot_x(0.4), &rot_y(1.1)));
    let pitch = r[2][1].asin();
    let yaw = (-r[2][0]).atan2(r[2][2]);
    let roll = r[1][0].atan2(r[0][0]) - (pitch.sin() * yaw.sin()).atan2(yaw.cos());
    assert!((pitch - 0.4).abs() < 1e-4 && (yaw - 1.1).abs() < 1e-4 && (roll - 0.7).abs() < 1e-4);
}

#[test]
fn rolling_right_drifts_surface_right() {
    let mut g = Game::new();
    g.ball_vx = 100.0;
    roll_ball(&mut g, 0.05);
    // The view-facing point (0,0,1) should move toward +x.
    assert!(g.ball_rot[0][2] > 0.0 && g.ball_rot[1][2].abs() < 1e-4);
}

#[test]
fn a_multi_makes_three_balls_at_the_same_speed_none_of_them_flat() {
    let mut g = Game::new();
    (g.ball_x, g.ball_y, g.ball_vx, g.ball_vy) = (200.0, 300.0, 240.0, -20.0);
    g.split_ball();
    assert_eq!(1 + g.extra.len(), MULTI_BALLS);
    let speed = g.ball_vx.hypot(g.ball_vy);
    for b in &g.extra {
        assert!((b.x, b.y) == (200.0, 300.0), "a copy started somewhere else");
        assert!(b.vy.abs() >= speed * 0.3 - 0.01, "a copy left along the paddle line: vy {}", b.vy);
        assert!(b.vx.hypot(b.vy) < speed * 1.2, "a copy left faster than the ball: {}", b.vx.hypot(b.vy));
    }
    // A second pickup with three in play adds none.
    g.split_ball();
    assert_eq!(1 + g.extra.len(), MULTI_BALLS);
    // A new ball starts alone.
    g.new_ball();
    assert!(g.extra.is_empty());
}

#[test]
fn swapping_a_ball_in_and_out_leaves_both_as_they_were() {
    let mut g = Game::new();
    (g.ball_x, g.ball_vx) = (10.0, 1.0);
    g.extra.push(Ball { x: 99.0, vx: -7.0, ..g.ball() });
    g.swap_ball(0);
    assert_eq!((g.ball_x, g.ball_vx, g.extra[0].x, g.extra[0].vx), (99.0, -7.0, 10.0, 1.0));
    g.swap_ball(0);
    assert_eq!((g.ball_x, g.ball_vx, g.extra[0].x, g.extra[0].vx), (10.0, 1.0, 99.0, -7.0));
}

fn layout(level: i32) -> Game {
    let mut g = Game::new();
    g.sess.level = level;
    g.build_bricks();
    g
}

#[test]
fn the_brick_grid_sits_inside_the_screen_without_overlaps() {
    let (x0, y0) = brick_rect(0);
    let (x1, y1) = brick_rect(BRICK_TOTAL - 1);
    assert!(x0 >= 0.0 && x1 + BRICK_W as f32 <= WIN_W as f32);
    assert!(y0 > HUD_H as f32 && y1 + (BRICK_H as f32) < WIN_H as f32 / 2.0);
    assert_eq!(brick_rect(1).0 - x0, (BRICK_W + BRICK_GAP) as f32);
}

#[test]
fn every_layout_has_bricks_and_can_be_cleared() {
    for level in 1..=16 {
        let g = layout(level);
        let alive: Vec<_> = g.bricks.iter().filter(|b| b.alive).collect();
        assert!(!alive.is_empty(), "level {level} has no bricks");
        assert!(alive.iter().all(|b| b.hp >= 1 && b.hp <= 2), "level {level}: a brick that never breaks");
        assert!(alive.iter().filter(|b| b.kind == BRICK_BOMB).count() <= BOMB_MAX, "level {level}: too many bombs");
    }
}

#[test]
fn steel_and_bombs_arrive_only_at_their_levels() {
    for _ in 0..20 {
        assert!(layout(1).bricks.iter().all(|b| b.kind != BRICK_STEEL && b.kind != BRICK_BOMB));
        assert!(layout(STEEL_MIN_LEVEL - 1).bricks.iter().all(|b| b.kind != BRICK_STEEL));
    }
}

#[test]
fn the_layouts_cycle_after_eight_levels() {
    let shape = |level| layout(level).bricks.iter().map(|b| b.alive).collect::<Vec<_>>();
    assert_eq!(shape(2), shape(10));
    assert_ne!(shape(1), shape(2));
}

#[test]
fn the_nearest_brick_is_the_one_closest_to_the_ball() {
    let mut g = layout(1);
    let (x, y) = brick_rect(BRICK_TOTAL - 1);
    (g.ball_x, g.ball_y) = (x, y + 40.0);
    let (bx, by) = g.nearest_brick().unwrap();
    assert_eq!((bx, by), (x + BRICK_W as f32 / 2.0, y + BRICK_H as f32 / 2.0));
    for b in g.bricks.iter_mut() { b.alive = false; }
    assert!(g.nearest_brick().is_none());
}
