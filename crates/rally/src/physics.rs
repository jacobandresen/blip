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

/// The court's measurements, so the BASIC tape's court can be traced too.
#[derive(Clone, Copy, Debug)]
pub struct Court {
    pub top: f32,
    pub bottom: f32,
    pub width: f32,
    pub lpad_x: f32,
    pub rpad_x: f32,
    pub pad_w: f32,
    pub pad_h: f32,
    pub ball: f32,
    pub speed_up: f32,
    pub top_speed: f32,
}

pub const COURT: Court = Court {
    top: PLAY_T,
    bottom: PLAY_B,
    width: WIN_W as f32,
    lpad_x: LPAD_X,
    rpad_x: RPAD_X,
    pad_w: PAD_W,
    pad_h: PAD_H,
    ball: BALL_SZ,
    speed_up: BALL_INC,
    top_speed: BALL_MAX,
};

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
    c: &Court,
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
    let substeps = ((speed * dt / (c.ball * 0.5)).ceil() as i32).clamp(1, 8);
    let sdt = dt / substeps as f32;

    for step in 0..substeps {
        let t = (step as f32 + 0.5) / substeps as f32;
        let lpad = lpad_y - lpad_vy * dt * (1.0 - t);
        let rpad = rpad_y - rpad_vy * dt * (1.0 - t);
        let (prev_x, prev_y) = (g.ball_x + c.ball * 0.5, g.ball_y + c.ball * 0.5);
        g.ball_x += g.ball_vx * sdt;
        g.ball_y += g.ball_vy * sdt;
        let (next_x, next_y) = (g.ball_x + c.ball * 0.5, g.ball_y + c.ball * 0.5);

        // Reflect the overshoot rather than clamp, so no speed is shaved off.
        if g.ball_y < c.top {
            g.ball_y = 2.0 * c.top - g.ball_y;
            g.ball_vy = g.ball_vy.abs();
            events.push(Event::Wall);
        }
        if g.ball_y + c.ball > c.bottom {
            g.ball_y = 2.0 * (c.bottom - c.ball) - g.ball_y;
            g.ball_vy = -g.ball_vy.abs();
            events.push(Event::Wall);
        }

        if g.ball_vx < 0.0
            && segment_rect_overlap(
                prev_x,
                prev_y,
                next_x,
                next_y,
                c.lpad_x - c.ball * 0.5,
                lpad - c.ball * 0.5,
                c.pad_w + c.ball,
                c.pad_h + c.ball,
            )
        {
            g.ball_x = c.lpad_x + c.pad_w;
            bounce_paddle(c, g, lpad, lpad_vy, 1.0);
            events.push(Event::LeftHit);
        }

        if g.ball_vx > 0.0
            && segment_rect_overlap(
                prev_x,
                prev_y,
                next_x,
                next_y,
                c.rpad_x - c.ball * 0.5,
                rpad - c.ball * 0.5,
                c.pad_w + c.ball,
                c.pad_h + c.ball,
            )
        {
            g.ball_x = c.rpad_x - c.ball;
            bounce_paddle(c, g, rpad, rpad_vy, -1.0);
            events.push(Event::RightHit);
        }

        if g.ball_x + c.ball < 0.0 {
            events.push(Event::Point(false));
            return events;
        }
        if g.ball_x > c.width {
            events.push(Event::Point(true));
            return events;
        }
    }
    events
}

pub fn bounce_paddle(c: &Court, g: &mut Ball, pad_y: f32, pad_vy: f32, dir: f32) {
    let rel = ((g.ball_y + c.ball * 0.5 - pad_y) / c.pad_h - 0.5).clamp(-0.5, 0.5);
    g.ball_spd = (g.ball_spd + c.speed_up).min(c.top_speed);
    let a = rel * 1.1;
    g.ball_vx = dir * g.ball_spd * a.cos();
    g.ball_vy = g.ball_spd * a.sin() + pad_vy * 0.25;
    // Renormalise so |v| stays exactly on the speed ramp after the english.
    let m = g.ball_vx.hypot(g.ball_vy).max(1.0);
    g.ball_vx = g.ball_vx / m * g.ball_spd;
    g.ball_vy = g.ball_vy / m * g.ball_spd;
}
