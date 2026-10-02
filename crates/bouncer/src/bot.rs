//! Native-only autopilot (BLIP_BOT=1): meet the ball where it will cross the
//! paddle line, walls folded in; fetch good drops while the ball climbs.

use super::*;
use blip::input::{BLIP_KEY_LEFT, BLIP_KEY_RIGHT};

/// Where ball `b` will cross the paddle line, walls folded in, and when.
fn landing(b: &Ball) -> (f32, f32) {
    let r = BALL_W as f32 / 2.0;
    let (cx, cy) = (b.x + r, b.y + r);
    let t = ((PAD_Y as f32 - (cy + r)) / b.vy).max(0.0);
    let (lo, hi) = (r, WIN_W as f32 - r);
    let span = hi - lo;
    let mut x = (cx + b.vx * t - lo).rem_euclid(2.0 * span);
    if x > span { x = 2.0 * span - x; }
    (x + lo, t)
}

pub fn drive(g: &Game, t: f32) {
    let keys: &[_] = match g.state {
        State::Title | State::Launch => {
            // Alternate so the fire key is a fresh press each time.
            if (t * 10.0) as i32 % 2 == 0 { &[BLIP_KEY_SPACE] } else { &[] }
        }
        State::Play => {
            let centre = g.pad_x + g.pad_w / 2.0;
            // The ball that comes down first, of however many are in play.
            let falling = g.extra.iter().copied().chain([g.ball()])
                .filter(|b| b.vy > 1.0 && b.y + (BALL_H as f32) < (PAD_Y + PAD_H) as f32).map(|b| landing(&b))
                .min_by(|p, q| p.1.total_cmp(&q.1));
            let mut target = falling.map_or(ball_circle(g).0, |f| f.0);
            // Off-centre contact steers; aim a little to send it inward.
            if falling.is_some() { target += if target < WIN_W as f32 / 2.0 { -g.pad_w * 0.15 } else { g.pad_w * 0.15 }; }
            let rising_far = falling.is_none() && g.ball_y < PAD_Y as f32 - 220.0;
            if rising_far {
                if let Some(d) = pool_iter(&g.drops).filter(|d| d.kind != DropKind::Narrow)
                    .max_by(|a, b| a.y.partial_cmp(&b.y).unwrap()) {
                    target = d.x + DROP_W / 2.0;
                }
            }
            let dead = 5.0;
            if target < centre - dead { &[BLIP_KEY_LEFT] } else if target > centre + dead { &[BLIP_KEY_RIGHT] } else { &[] }
        }
        _ => &[],
    };
    blip::bot::hold(keys);
}
