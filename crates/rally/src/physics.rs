//! Ball motion, swept paddle hits, wall reflection, and spin without audio or input.

use blip::segment_rect_overlap;

pub const WIN_W: i32 = 480;
pub const WIN_H: i32 = 540;
pub const HUD_H: i32 = 28;
pub const PLAY_T: f32 = HUD_H as f32;
pub const PLAY_B: f32 = WIN_H as f32;
pub const PLAY_H: f32 = PLAY_B - PLAY_T;

// ---- objects ----------------------------------------------------------
pub const PAD_W: f32 = 12.0;
pub const PAD_H: f32 = 72.0;
pub const BALL_SZ: f32 = 12.0;
pub const PAD_OFF: f32 = 28.0;

// ---- tuning -----------------------------------------------------------
pub const SCORE_WIN: i32 = 7;
pub const POINT_PAUSE: f32 = 1.2;
pub const PAD_SPEED: f32 = 300.0;
pub const BALL_SPD0: f32 = 275.0;
pub const BALL_INC: f32 = 15.0;
pub const BALL_MAX: f32 = 450.0;
pub const AI_SPD: f32 = 145.0;
// A finger drags its bat relatively, geared up so a thumb's sweep of about
// two thirds of the picture covers the whole field.
pub const TOUCH_GAIN: f32 = 1.6;

// ---- derived ----------------------------------------------------------
pub const LPAD_X: f32 = PAD_OFF;
pub const RPAD_X: f32 = WIN_W as f32 - PAD_OFF - PAD_W;
pub const PAD_YMIN: f32 = PLAY_T + 2.0;
pub const PAD_YMAX: f32 = PLAY_B - PAD_H - 2.0;

#[derive(Clone, Copy, Debug)]
pub struct Ball {
    pub ball_x: f32,
    pub ball_y: f32,
    pub ball_vx: f32,
    pub ball_vy: f32,
    pub ball_spd: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Wall,
    LeftHit,
    RightHit,
    Point(bool),
}

pub fn step(
    g: &mut Ball,
    dt: f32,
    lpad_y: f32,
    rpad_y: f32,
    lpad_vy: f32,
    rpad_vy: f32,
) -> Vec<Event> {
    let dt = dt.max(1e-4);
    let mut events = Vec::new();
    // Integrate in substeps so a fast ball can't skip past a paddle or clip
    // through a wall inside one frame (worst case: a frame-rate dip).
    let speed = g.ball_vx.hypot(g.ball_vy).max(1.0);
    let substeps = ((speed * dt / (BALL_SZ * 0.5)).ceil() as i32).clamp(1, 8);
    let sdt = dt / substeps as f32;

    for step in 0..substeps {
        let t = (step as f32 + 0.5) / substeps as f32;
        let lpad = lpad_y - lpad_vy * dt * (1.0 - t);
        let rpad = rpad_y - rpad_vy * dt * (1.0 - t);
        let (prev_x, prev_y) = (g.ball_x + BALL_SZ * 0.5, g.ball_y + BALL_SZ * 0.5);
        g.ball_x += g.ball_vx * sdt;
        g.ball_y += g.ball_vy * sdt;
        let (next_x, next_y) = (g.ball_x + BALL_SZ * 0.5, g.ball_y + BALL_SZ * 0.5);

        // Reflect the overshoot rather than clamp, so no speed is shaved off.
        if g.ball_y < PLAY_T {
            g.ball_y = 2.0 * PLAY_T - g.ball_y;
            g.ball_vy = g.ball_vy.abs();
            events.push(Event::Wall);
        }
        if g.ball_y + BALL_SZ > PLAY_B {
            g.ball_y = 2.0 * (PLAY_B - BALL_SZ) - g.ball_y;
            g.ball_vy = -g.ball_vy.abs();
            events.push(Event::Wall);
        }

        if g.ball_vx < 0.0
            && segment_rect_overlap(
                prev_x,
                prev_y,
                next_x,
                next_y,
                LPAD_X - BALL_SZ * 0.5,
                lpad - BALL_SZ * 0.5,
                PAD_W + BALL_SZ,
                PAD_H + BALL_SZ,
            )
        {
            g.ball_x = LPAD_X + PAD_W;
            bounce_paddle(g, lpad, lpad_vy, 1.0);
            events.push(Event::LeftHit);
        }

        if g.ball_vx > 0.0
            && segment_rect_overlap(
                prev_x,
                prev_y,
                next_x,
                next_y,
                RPAD_X - BALL_SZ * 0.5,
                rpad - BALL_SZ * 0.5,
                PAD_W + BALL_SZ,
                PAD_H + BALL_SZ,
            )
        {
            g.ball_x = RPAD_X - BALL_SZ;
            bounce_paddle(g, rpad, rpad_vy, -1.0);
            events.push(Event::RightHit);
        }

        if g.ball_x + BALL_SZ < 0.0 {
            events.push(Event::Point(false));
            return events;
        }
        if g.ball_x > WIN_W as f32 {
            events.push(Event::Point(true));
            return events;
        }
    }
    events
}

pub fn bounce_paddle(g: &mut Ball, pad_y: f32, pad_vy: f32, dir: f32) {
    let rel = ((g.ball_y + BALL_SZ * 0.5 - pad_y) / PAD_H - 0.5).clamp(-0.5, 0.5);
    g.ball_spd = (g.ball_spd + BALL_INC).min(BALL_MAX);
    let a = rel * 1.1;
    g.ball_vx = dir * g.ball_spd * a.cos();
    g.ball_vy = g.ball_spd * a.sin() + pad_vy * 0.25;
    // Renormalise so |v| stays exactly on the speed ramp after the english.
    let m = g.ball_vx.hypot(g.ball_vy).max(1.0);
    g.ball_vx = g.ball_vx / m * g.ball_spd;
    g.ball_vy = g.ball_vy / m * g.ball_spd;
}
