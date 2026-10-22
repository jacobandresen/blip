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

pub fn move_ball(
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

        bounce_off_walls(c, g, &mut events);

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
            bounce(c, g, lpad, lpad_vy, 1.0);
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
            bounce(c, g, rpad, rpad_vy, -1.0);
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

/// Reflect the overshoot rather than clamp, so no speed is shaved off.
fn bounce_off_walls(c: &Court, g: &mut Ball, events: &mut Vec<Event>) {
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
}

pub fn bounce(c: &Court, g: &mut Ball, pad_y: f32, pad_vy: f32, dir: f32) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ball(x: f32, y: f32, vx: f32, vy: f32) -> Ball {
        Ball { ball_x: x, ball_y: y, ball_vx: vx, ball_vy: vy, ball_spd: vx.hypot(vy) }
    }

    /// Paddles parked at the bottom, out of the ball's way.
    fn fly(b: &mut Ball, dt: f32) -> Vec<Event> {
        move_ball(&COURT, b, dt, PAD_YMAX, PAD_YMAX, 0.0, 0.0)
    }

    #[test]
    fn a_wall_reflects_the_overshoot_and_keeps_the_speed() {
        let mut b = ball(200.0, PLAY_T + 2.0, 0.0, -300.0);
        let events = fly(&mut b, 0.02);
        assert_eq!(events, vec![Event::Wall]);
        assert!(b.ball_vy > 0.0, "the ball should now head down");
        assert!(b.ball_y >= PLAY_T, "the ball should be back in the court");
        assert_eq!(b.ball_vy.abs(), 300.0);
    }

    #[test]
    fn the_bottom_wall_sends_the_ball_up() {
        let mut b = ball(200.0, PLAY_B - BALL_SZ - 1.0, 0.0, 300.0);
        assert_eq!(fly(&mut b, 0.02), vec![Event::Wall]);
        assert!(b.ball_vy < 0.0);
        assert!(b.ball_y + BALL_SZ <= PLAY_B);
    }

    #[test]
    fn a_fast_ball_cannot_pass_through_a_paddle_in_one_frame() {
        // Far enough in one long frame to cross the paddle's whole width.
        let pad_y = 200.0;
        let mut b = ball(LPAD_X + PAD_W + 30.0, pad_y + PAD_H / 2.0 - BALL_SZ / 2.0, -BALL_MAX, 0.0);
        let events = move_ball(&COURT, &mut b, 0.1, pad_y, PAD_YMAX, 0.0, 0.0);
        assert_eq!(events, vec![Event::LeftHit]);
        assert!(b.ball_vx > 0.0, "the ball should come back off the paddle");
    }

    #[test]
    fn a_ball_past_an_edge_is_the_other_sides_point() {
        let mut b = ball(-BALL_SZ + 1.0, 300.0, -300.0, 0.0);
        assert_eq!(fly(&mut b, 0.02), vec![Event::Point(false)]);
        let mut b = ball(WIN_W as f32 - 1.0, 300.0, 300.0, 0.0);
        assert_eq!(fly(&mut b, 0.02), vec![Event::Point(true)]);
    }

    #[test]
    fn a_bounce_speeds_the_ball_up_to_the_cap_and_no_further() {
        let mut b = ball(LPAD_X + PAD_W, 200.0, -BALL_SPD0, 0.0);
        bounce(&COURT, &mut b, 200.0 - PAD_H / 2.0, 0.0, 1.0);
        assert_eq!(b.ball_spd, BALL_SPD0 + BALL_INC);
        let mut fast = ball(LPAD_X + PAD_W, 200.0, -BALL_MAX, 0.0);
        bounce(&COURT, &mut fast, 200.0 - PAD_H / 2.0, 0.0, 1.0);
        assert_eq!(fast.ball_spd, BALL_MAX);
    }

    #[test]
    fn spin_bends_the_ball_without_speeding_it_up() {
        let mut b = ball(LPAD_X + PAD_W, 200.0, -BALL_SPD0, 0.0);
        bounce(&COURT, &mut b, 200.0 - PAD_H / 2.0, 600.0, 1.0);
        assert!(b.ball_vy > 0.0, "a paddle moving down should throw the ball down");
        assert!((b.ball_vx.hypot(b.ball_vy) - b.ball_spd).abs() < 0.01);
    }

    #[test]
    fn the_paddles_edge_sends_the_ball_at_an_angle() {
        // Off the top of the paddle, the ball leaves upward; off the middle, flat.
        let mut top = ball(LPAD_X + PAD_W, 200.0, -BALL_SPD0, 0.0);
        bounce(&COURT, &mut top, 200.0, 0.0, 1.0);
        assert!(top.ball_vy < 0.0);
        let mut mid = ball(LPAD_X + PAD_W, 200.0, -BALL_SPD0, 0.0);
        bounce(&COURT, &mut mid, 200.0 + BALL_SZ / 2.0 - PAD_H / 2.0, 0.0, 1.0);
        assert!(mid.ball_vy.abs() < 1.0);
    }
}
