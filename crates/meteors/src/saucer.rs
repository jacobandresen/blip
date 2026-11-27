//! The saucer: when it comes, how it flies and fires, and shooting it down.

use super::*;

/// Saucers spawn more often, shoot more often, and skew toward the harder
/// "small" variant as the level climbs, so late levels keep escalating even
/// after the asteroid count itself caps out.
pub(super) fn saucer_spawn_range(level: i32) -> (f32, f32) {
    let lv = (level - 1).min(6) as f32;
    ((SAUCER_SPAWN_MIN - lv * 0.6).max(4.0), (SAUCER_SPAWN_MAX - lv * 1.2).max(8.0))
}

pub(super) fn saucer_fire_cooldown(level: i32) -> f32 {
    let lv = (level - 1).min(6) as f32;
    (SAUCER_FIRE_CD - lv * 0.12).max(0.7)
}

pub(super) fn spawn_saucer(g: &mut Game) {
    let small_prob = (0.33 + (g.sess.level - 1) as f32 * 0.05).min(0.75);
    let big = g.sess.score < 5000 || rand_range_f32(0.0, 1.0) > small_prob;
    let from_left = rand_int(0, 1) == 0;
    let x = if from_left { -20.0 } else { PLAY_W as f32 + 20.0 };
    let y = rand_range_f32(PLAY_Y0 + 40.0, PLAY_Y0 + PLAY_H as f32 - 40.0);
    let vx = if from_left { SAUCER_SPEED } else { -SAUCER_SPEED };
    let fire_cd = saucer_fire_cooldown(g.sess.level);
    g.saucer = Saucer { active: true, x, y, vx, wave_t: 0.0, fire_t: fire_cd * 0.5, big };
}

/// The saucer crosses on a wave, firing at the ship (the big one wildly, the
/// small one true); when gone, the next is on its way.
pub(super) fn fly_saucer(g: &mut Game, dt: f32) {
    if g.saucer.active {
        g.saucer.x += g.saucer.vx * dt;
        g.saucer.wave_t += dt;
        g.saucer.y += (g.saucer.wave_t * 2.0).sin() * 24.0 * dt;
        g.saucer.fire_t -= dt;
        if g.saucer.fire_t <= 0.0 && g.ship_alive {
            let dx = g.ship.x - g.saucer.x;
            let dy = g.ship.y - g.saucer.y;
            let spread = if g.saucer.big { 0.35 } else { 0.04 };
            let jitter = rand_range_f32(-spread, spread);
            let base = dy.atan2(dx) + jitter;
            let speed = 260.0;
            spawn_bullet(&mut g.bullets, g.saucer.x, g.saucer.y, base.cos() * speed, base.sin() * speed, false);
            g.saucer.fire_t = saucer_fire_cooldown(g.sess.level);
        }
        if g.saucer.x < -40.0 || g.saucer.x > PLAY_W as f32 + 40.0 {
            g.saucer.active = false;
        }
    } else {
        g.saucer_cd -= dt;
        if g.saucer_cd <= 0.0 {
            spawn_saucer(g);
            let (lo, hi) = saucer_spawn_range(g.sess.level);
            g.saucer_cd = rand_range_f32(lo, hi);
        }
    }
}

/// The ship's shots against the saucer, swept with it from where it began the frame (or, new this frame, against
/// where it is): the small one is worth five of the big.
pub(super) fn shoot_saucer(g: &mut Game, dt: f32, sfx: &Sounds, bullet_starts: &[(f32, f32); MAX_BULLETS], saucer_start: Option<(f32, f32)>) {
    let r = saucer_radius(g.saucer.big);
    let at = (g.saucer.x, g.saucer.y);
    // `bi` pairs the bullet slot with its swept start.
    for (bi, &from) in bullet_starts.iter().enumerate() {
        let b = g.bullets[bi];
        if !b.active || !b.from_player { continue; }
        let hit = match saucer_start {
            Some(s0) => wrapped_motion_overlap((b.x, b.y), at, (b.vx * dt, b.vy * dt), (at.0 - s0.0, at.1 - s0.1), r),
            None => wrapped_segment_circle_overlap(from, (from.0 + b.vx * dt, from.1 + b.vy * dt), at, r),
        };
        if !hit { continue; }
        g.bullets[bi].active = false;
        g.saucer.active = false;
        blip::bot::add("saucers", 1.0);
        let (sx, sy, svx) = (g.saucer.x, g.saucer.y, g.saucer.vx);
        burst(g, sx, sy, svx, 0.0, 6, r, 90.0, BLIP_HEART);
        let times = g.streak.hit();
        let pts = if g.saucer.big { 200 } else { 1000 } * times;
        g.fx.popup(sx, sy - 14.0, &if times > 1 { format!("{pts} X{times}") } else { format!("{pts}") }, BLIP_GOLD);
        award(g, sfx, pts);
        if g.saucer.big { play_sfx(&sfx.saucer_big); } else { play_sfx(&sfx.saucer_small); }
        break;
    }
}
