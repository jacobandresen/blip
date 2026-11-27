//! The rock field: rocks placed clear of the ship, drifting and wrapping, split by shots into smaller, faster ones.

use super::*;

/// A point at least SAFE_RADIUS from (cx, cy), measured across the wrap.
pub(super) fn find_clear_spot(cx: f32, cy: f32) -> (f32, f32) {
    let (w, h) = (PLAY_W as f32, PLAY_H as f32);
    loop {
        let x = rand_range_f32(0.0, w);
        let y = rand_range_f32(PLAY_Y0, PLAY_Y0 + h);
        let dx = (x - cx).abs().min(w - (x - cx).abs());
        let dy = (y - cy).abs().min(h - (y - cy).abs());
        if dx * dx + dy * dy > SAFE_RADIUS * SAFE_RADIUS {
            return (x, y);
        }
    }
}

/// A rock's speed for its size, in any direction.
pub(super) fn random_velocity(size: ASize) -> (f32, f32) {
    let (smin, smax) = size.speed_range();
    let speed = rand_range_f32(smin, smax);
    let dir = rand_range_f32(0.0, PI * 2.0);
    (dir.cos() * speed, dir.sin() * speed)
}

pub(super) fn spawn_rock(g: &mut Game, x: f32, y: f32, size: ASize, vx: f32, vy: f32) {
    let mut jag = [1.0f32; 10];
    for j in jag.iter_mut() { *j = rand_range_f32(0.75, 1.25); }
    pool_spawn(&mut g.asteroids, Asteroid {
        active: true, x, y,
        vx, vy,
        size, rot: rand_range_f32(0.0, PI * 2.0),
        spin: rand_range_f32(-1.5, 1.5),
        jag,
    });
}

/// Fragments leave along opposite sides of a random axis, on top of the
/// parent's velocity (plus a share of the impactor's), so momentum carries through.
pub(super) fn split_rock(g: &mut Game, x: f32, y: f32, size: ASize, pvx: f32, pvy: f32) {
    let Some(child) = size.child() else { return };
    let axis = rand_range_f32(0.0, PI * 2.0);
    let (ax, ay) = (axis.cos(), axis.sin());
    let (smin, smax) = child.speed_range();
    for sign in [1.0_f32, -1.0] {
        let kick = rand_range_f32(smin, smax) * 0.6;
        spawn_rock(g, x + ax * sign * 6.0, y + ay * sign * 6.0, child,
            pvx + ax * sign * kick, pvy + ay * sign * kick);
    }
}

/// Debris tumbles out and fades; the rocks drift and turn, wrapping round the field.
pub(super) fn drift_field(g: &mut Game, dt: f32) {
    for d in pool_iter_mut(&mut g.debris) {
        d.x += d.vx * dt;
        d.y += d.vy * dt;
        d.rot += d.spin * dt;
        d.ttl -= dt;
        if d.ttl <= 0.0 { d.active = false; }
    }
    for a in pool_iter_mut(&mut g.asteroids) {
        a.x = wrap(a.x + a.vx * dt, 0.0, PLAY_W as f32);
        a.y = wrap(a.y + a.vy * dt, PLAY_Y0, PLAY_Y0 + PLAY_H as f32);
        a.rot += a.spin * dt;
    }
}

/// The ship's shots against the rocks that were there when the frame began: a hit splits the rock (with a share of
/// the shot's momentum), scores by the streak, and now and then drops a crystal.
pub(super) fn shoot_rocks(g: &mut Game, dt: f32, sfx: &Sounds, rocks_at_start: &[bool; MAX_ASTEROIDS]) {
    for bi in 0..MAX_BULLETS {
        if !g.bullets[bi].active || !g.bullets[bi].from_player { continue; }
        for (ai, was) in rocks_at_start.iter().enumerate() {
            if !*was || !g.asteroids[ai].active { continue; }
            let (b, a) = (g.bullets[bi], g.asteroids[ai]);
            if !wrapped_motion_overlap((b.x, b.y), (a.x, a.y), (b.vx * dt, b.vy * dt), (a.vx * dt, a.vy * dt), a.size.radius()) { continue; }
            g.bullets[bi].active = false;
            g.asteroids[ai].active = false;
            let size = a.size;
            let (pvx, pvy) = (a.vx + b.vx * 0.04, a.vy + b.vy * 0.04);
            let c = rock_color(&a, g.sess.level, blip::now());
            burst(g, a.x, a.y, a.vx, a.vy, 5, size.radius() * 0.5, 60.0, c);
            let times = g.streak.hit();
            let pts = size.points() * times;
            if times > 1 { g.fx.popup(a.x, a.y - 10.0, &format!("{pts} X{times}"), BLIP_GOLD); }
            award(g, sfx, pts);
            blip::bot::add(if size == ASize::Small { "rocks_dust" } else { "rocks_split" }, 1.0);
            match size {
                ASize::Large  => play_sfx(&sfx.bang_large),
                ASize::Medium => play_sfx(&sfx.bang_medium),
                ASize::Small  => play_sfx(&sfx.bang_small),
            }
            split_rock(g, a.x, a.y, size, pvx, pvy);
            if size != ASize::Small && g.crystals.len() < MAX_CRYSTALS && rand_range_f32(0.0, 1.0) < CRYSTAL_ODDS {
                g.crystals.push(Crystal { x: a.x, y: a.y, vx: pvx * 0.3, vy: pvy * 0.3, ttl: CRYSTAL_TTL });
            }
            break;
        }
    }
}
