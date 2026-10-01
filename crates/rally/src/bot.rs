//! Native-only autopilot (BLIP_BOT=1): the left bat meets the ball where it
//! will arrive, walls folded in, with a human-sized aiming error.

use super::*;

fn arrival_y(g: &Game) -> f32 {
    let cy = g.ball_y + BALL_SZ * 0.5;
    if g.ball_vx >= 0.0 { return PLAY_T + PLAY_H * 0.5; }
    let t = (g.ball_x - (LPAD_X + PAD_W)) / -g.ball_vx;
    let (lo, hi) = (PLAY_T + BALL_SZ * 0.5, PLAY_B - BALL_SZ * 0.5);
    let span = hi - lo;
    let mut y = (cy + g.ball_vy * t - lo).rem_euclid(2.0 * span);
    if y > span { y = 2.0 * span - y; }
    y + lo
}

pub struct Bot { pub err: f32, pub last_vx: f32, pub hits: i32 }

pub fn drive(g: &Game, b: &mut Bot, t: f32) {
    let tap = (t * 10.0) as i32 % 2 == 0;
    let keys: &[_] = match g.state {
        State::Title | State::Serve => if tap { &[BLIP_KEY_UP] } else { &[] },
        State::Over => {
            blip::bot::set("score_bot", g.score_l as f64);
            blip::bot::set("score_cpu", g.score_r as f64);
            blip::bot::finish(if g.score_l > g.score_r { "won" } else { "lost" });
            &[]
        }
        State::Play => {
            if g.ball_vx < 0.0 && b.last_vx >= 0.0 {
                // A new volley coming in: pick where on the face to take it.
                b.err = blip::macroquad::rand::gen_range(-0.42, 0.42) * PAD_H;
                b.hits += 1;
            }
            b.last_vx = g.ball_vx;
            let target = arrival_y(g) + b.err;
            let centre = g.lpad_y + PAD_H * 0.5;
            if target < centre - 4.0 { &[BLIP_KEY_UP] } else if target > centre + 4.0 { &[BLIP_KEY_DOWN] } else { &[] }
        }
        State::Point => {
            if b.hits > 0 { blip::bot::add(&format!("rally_len{}", (b.hits / 3) * 3), 1.0); }
            b.hits = 0;
            &[]
        }
    };
    blip::bot::hold(keys);
}
