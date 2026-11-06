//! How to play Rally: the left bat meets the ball where it will arrive, the
//! walls folded in, taking it on a different part of the face each volley
//! (a human-sized aiming error) so the returns vary.

use super::*;
use blip::bot::{predict, Goal::*, Pad, Playthrough};

/// What the bot must show it can do in Rally.
pub const PLAYTHROUGHS: &[Playthrough] = &[
    Playthrough {
        name: "the bat meets the ball and returns it ten times",
        goals: &[AtLeast("returns", 10.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it keeps a rally going for six returns",
        seconds: 240.0,
        goals: &[AtLeast("longest_rally", 6.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it wins a match to seven against the CPU",
        seconds: 600.0,
        goals: &[Ends("won")],
        ..Playthrough::DEFAULT
    },
];

/// What the bot remembers between frames.
#[derive(Default)]
pub struct Bot {
    /// Where on the face it means to take the next ball.
    aim_error: f32,
    last_vx: f32,
    rally: i32,
}

pub fn drive(g: &Game, b: &mut Bot) {
    let mut pad = Pad::new();
    match g.state {
        // Moving the left bat starts a one-player game and serves.
        State::Title | State::Serve => { pad.tap(BLIP_KEY_UP); }
        State::Over => blip::bot::finish(if g.score_l > g.score_r { "won" } else { "lost" }),
        State::Play => {
            count_returns(g, b);
            pad.climb(g.lpad_y + PAD_H * 0.5, arrival_y(g) + b.aim_error, 4.0);
        }
        State::Point => b.rally = 0,
    }
    pad.send();
}

/// The ball turning back toward the CPU is a return; a new volley coming in
/// gets a new spot on the face.
fn count_returns(g: &Game, b: &mut Bot) {
    if g.ball_vx > 0.0 && b.last_vx < 0.0 {
        b.rally += 1;
        blip::bot::add("returns", 1.0);
        blip::bot::set("longest_rally", (b.rally as f64).max(blip::bot::stat("longest_rally")));
    }
    if g.ball_vx < 0.0 && b.last_vx >= 0.0 {
        b.aim_error = blip::macroquad::rand::gen_range(-0.42, 0.42) * PAD_H;
    }
    b.last_vx = g.ball_vx;
}

/// Where the ball will reach the left bat's line; mid-court while it heads away.
fn arrival_y(g: &Game) -> f32 {
    if g.ball_vx >= 0.0 { return PLAY_T + PLAY_H * 0.5; }
    let r = BALL_SZ * 0.5;
    let t = (g.ball_x - (LPAD_X + PAD_W)) / -g.ball_vx;
    predict::bounce(g.ball_y + r + g.ball_vy * t, PLAY_T + r, PLAY_B - r)
}
