//! The ball: the screwball curve, the pull toward the last bricks, its flight, its spin, and bouncing off the walls
//! and the paddle.

use super::*;

/// The screwball curve: the velocity turns (speed kept) at the spin's rate,
/// up to SCREW_MAX_CURVE, the spin bleeding off so the curve eases out
/// instead of looping.
pub(super) fn curve_ball(g: &mut Game, dt: f32) {
    if g.ball_spin.abs() > 0.001 {
        let remaining = (SCREW_MAX_CURVE - g.ball_curve_used).max(0.0);
        let mut ang = g.ball_spin * dt;
        if ang.abs() > remaining {
            ang = ang.signum() * remaining;
            g.ball_spin = 0.0;
        }
        g.ball_curve_used += ang.abs();
        let (s, c) = ang.sin_cos();
        let (vx, vy) = (g.ball_vx, g.ball_vy);
        g.ball_vx = vx * c - vy * s;
        g.ball_vy = vx * s + vy * c;
        g.ball_spin *= (1.0 - SCREW_SPIN_DECAY * dt).max(0.0);
    }
}

/// The pull: the flight steepens and quickens (see PULL).
pub(super) fn pull_ball(g: &mut Game, dt: f32) {
    let sp = g.ball_vx.hypot(g.ball_vy).max(1.0);
    let flat = (1.0 - (g.ball_vy.abs() / sp) / 0.4).clamp(0.0, 1.0);
    let dir = if g.ball_vy < 0.0 { -1.0 } else { 1.0 };
    g.ball_vy += dir * (PULL + PULL_FLAT * flat) * dt;
    if g.ball_spin.abs() > 0.001 || g.ball_curve_used > 0.0 {
        let k = 1.0 + SCREW_GAIN * dt;
        g.ball_vx *= k;
        g.ball_vy *= k;
    }
    let sp = g.ball_vx.hypot(g.ball_vy);
    let cap = BALL_SPEED_MAX * PULL_CAP;
    if sp > cap { g.ball_vx *= cap / sp; g.ball_vy *= cap / sp; }
}

/// One frame of the ball in hand: seeking, spin, pull, and the move itself
/// against walls, paddle and bricks. Returns true if it fell out the bottom.
pub(super) fn fly_ball(g: &mut Game, dt: f32, active_speed: f32, sfx: &Sounds) -> bool {
    seek_brick(g, dt);
    curve_ball(g, dt);
    pull_ball(g, dt);

    // Rolling-mark spin (visual only): angular rate = speed / radius, driven
    // by the horizontal component so a curving shot visibly spins faster.
    roll_ball(g, dt);

    // ---- integrate in substeps so a fast ball can't tunnel through a brick,
    //      the seam between two bricks, or the paddle in a single frame ----
    let speed = g.ball_vx.hypot(g.ball_vy).max(1.0);
    let substeps = ((speed * dt / (BALL_W as f32 * 0.4)).ceil() as i32).clamp(1, 8);
    let sdt = dt / substeps as f32;
    let pad_start_x = g.pad_x - g.pad_vx * dt;

    for step in 0..substeps {
        g.ball_x += g.ball_vx * sdt;
        g.ball_y += g.ball_vy * sdt;

        bounce_off_walls(g, sfx);
        let pad_x = pad_start_x + g.pad_vx * dt * (step as f32 + 0.5) / substeps as f32;
        bounce_off_paddle(g, active_speed, pad_x, sfx);
        hit_bricks(g, active_speed, sfx);

        if g.ball_y > WIN_H as f32 {
            g.fx.burst(g.ball_x + BALL_W as f32 / 2.0, WIN_H as f32 - 4.0, 14, 160.0, BLIP_RED);
            return true;
        }
    }
    false
}

/// While seeking, bend a rising ball toward the nearest brick.
pub(super) fn seek_brick(g: &mut Game, dt: f32) {
    if !g.seeking() || g.ball_vy >= 0.0 { return; }
    let Some((tx, ty)) = g.nearest_brick() else { return };
    let (cx, cy, _) = ball_circle(g);
    let want = (ty - cy).atan2(tx - cx);
    let have = g.ball_vy.atan2(g.ball_vx);
    let diff = blip::angle_diff(have, want);
    let turn = diff.clamp(-SEEK_RATE * dt, SEEK_RATE * dt);
    let (s, c) = turn.sin_cos();
    let (vx, vy) = (g.ball_vx, g.ball_vy);
    g.ball_vx = vx * c - vy * s;
    g.ball_vy = vx * s + vy * c;
}

pub(super) type Mat3 = [[f32; 3]; 3];
pub(super) const MAT3_ID: Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

pub(super) fn mat_mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut o = [[0.0; 3]; 3];
    for i in 0..3 { for j in 0..3 { for k in 0..3 { o[i][j] += a[i][k] * b[k][j]; } } }
    o
}
pub(super) fn rot_x(t: f32) -> Mat3 { let (s, c) = t.sin_cos(); [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]] }
pub(super) fn rot_y(t: f32) -> Mat3 { let (s, c) = t.sin_cos(); [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]] }
/// Rolls the ball without slipping: its surface drifts along the velocity
/// (axis = view-normal x v), plus the screwball spin about the view axis.
pub(super) fn roll_ball(g: &mut Game, dt: f32) {
    let r = BALL_W as f32 * 0.5;
    let w = [-g.ball_vy / r, g.ball_vx / r, g.ball_spin * 2.0];
    let ang = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt() * dt;
    if ang < 1e-6 { return; }
    let m = ang / dt;
    let (n0, n1, n2) = (w[0] / m, w[1] / m, w[2] / m);
    let (s, c) = ang.sin_cos();
    let t = 1.0 - c;
    let d: Mat3 = [
        [c + n0 * n0 * t,      n0 * n1 * t - n2 * s, n0 * n2 * t + n1 * s],
        [n1 * n0 * t + n2 * s, c + n1 * n1 * t,      n1 * n2 * t - n0 * s],
        [n2 * n0 * t - n1 * s, n2 * n1 * t + n0 * s, c + n2 * n2 * t],
    ];
    g.ball_rot = mat_mul(&d, &g.ball_rot);
    // Re-orthonormalise the rows so float drift never skews the sphere.
    let m = &mut g.ball_rot;
    for i in 0..3 {
        for j in 0..i {
            let dot: f32 = (0..3).map(|k| m[i][k] * m[j][k]).sum();
            // `k` walks two rows at once (mi and mj); the rows are split apart for the borrow checker.
            let (past, current) = m.split_at_mut(i);
            for (mi, mj) in current[0].iter_mut().zip(&past[j]) { *mi -= dot * *mj; }
        }
        let len = m[i].iter().map(|v| v * v).sum::<f32>().sqrt();
        for v in m[i].iter_mut() { *v /= len; }
    }
}

/// Split the orientation into sheet cell (yaw, pitch) and a roll angle:
/// R = Rz(roll) * Rx(pitch) * Ry(yaw).
pub(super) fn ball_pose(m: &Mat3) -> (u32, u32, f32) {
    let pitch = m[2][1].clamp(-1.0, 1.0).asin();
    let yaw = (-m[2][0]).atan2(m[2][2]);
    let roll = m[1][0].atan2(m[0][0]) - (pitch.sin() * yaw.sin()).atan2(yaw.cos());
    let period = 2.0 * PI / 3.0;
    let col = ((yaw.rem_euclid(period) / period * BALL_YAW_N as f32).round() as u32) % BALL_YAW_N;
    let row = (((pitch + PI / 2.0) / PI * (BALL_PITCH_N - 1) as f32).round() as u32).min(BALL_PITCH_N - 1);
    (col, row, roll)
}

/// The ball as a circle: centre and radius.
#[inline]
pub(super) fn ball_circle(g: &Game) -> (f32, f32, f32) {
    let r = BALL_W as f32 / 2.0;
    (g.ball_x + r, g.ball_y + r, r)
}

/// Left / right / top walls. The overshoot past the wall is reflected back
/// rather than clamped flat, so no speed is lost to the ball "sticking".
pub(super) fn bounce_off_walls(g: &mut Game, sfx: &Sounds) {
    if g.ball_x < 0.0 {
        g.ball_x = -g.ball_x;
        g.ball_vx = g.ball_vx.abs();
        play_sfx_volume(&sfx.wall_hit, 0.55);
    }
    let right = (WIN_W - BALL_W) as f32;
    if g.ball_x > right {
        g.ball_x = 2.0 * right - g.ball_x;
        g.ball_vx = -g.ball_vx.abs();
        play_sfx_volume(&sfx.wall_hit, 0.55);
    }
    let top = HUD_H as f32;
    if g.ball_y < top {
        g.ball_y = 2.0 * top - g.ball_y;
        g.ball_vy = g.ball_vy.abs();
        play_sfx_volume(&sfx.wall_hit, 0.55);
    }
}

pub(super) fn bounce_off_paddle(g: &mut Game, speed: f32, pad_x: f32, sfx: &Sounds) {
    if g.ball_vy <= 0.0 {
        return;
    }
    if !rects_overlap(
        g.ball_x, g.ball_y, BALL_W as f32, BALL_H as f32,
        pad_x, PAD_Y as f32, g.pad_w, PAD_H as f32,
    ) {
        return;
    }
    play_variant(&sfx.paddle_hit, speed);
    web::haptic();
    g.chain = 0;
    g.fx.ring(g.ball_x + BALL_W as f32 / 2.0, PAD_Y as f32, 22.0, 0.25, BlipColor { r: 0.6, g: 0.85, b: 1.0, a: 0.8 });
    g.pad_kick_v = 40.0 + 70.0 * (speed / BALL_SPEED_MAX).min(1.0);
    let incoming_vx = g.ball_vx;

    // Where it landed across the face, -1 (left tip) .. +1 (right tip). The
    // classic "aim by where you hit" english, plus a slice of the paddle's
    // own sideways speed carried into the ball the way a moving bat drags it.
    let rel = ((g.ball_x + BALL_W as f32 / 2.0 - pad_x) / g.pad_w - 0.5) * 2.0;
    let steer = rel.clamp(-1.0, 1.0) * 1.15; // radians
    let nvx = speed * steer.sin() + g.pad_vx * 0.18;
    let mut nvy = -(speed * steer.cos()).abs();
    if nvy > -speed * 0.32 {
        nvy = -speed * 0.32; // keep it from grazing along just over the paddle
    }
    // Renormalise: the english + paddle drag changed |v|; the ball's speed
    // stays exactly on the ramp.
    let m = (nvx * nvx + nvy * nvy).sqrt().max(1.0);
    g.ball_vx = nvx / m * speed;
    g.ball_vy = nvy / m * speed;
    g.ball_y = (PAD_Y - BALL_H) as f32 - 0.5;

    // Screwball spin from the slip between the paddle's surface and the
    // ball's — their *relative* horizontal velocity, not the paddle's alone.
    let rel_vx = g.pad_vx - incoming_vx;
    g.ball_spin = if rel_vx.abs() > SCREW_MIN_PAD_SPEED {
        let t = (rel_vx.abs() / (PAD_SPEED + BALL_SPEED_MAX)).min(1.0);
        rel_vx.signum() * t * SCREW_MAX_SPIN_RATE
    } else {
        0.0
    };
    g.ball_curve_used = 0.0;
}
