//! How to play Bouncer: meet the ball where it will come down, a little off
//! centre to send it back inward, and fetch good drops while the ball climbs.

use super::*;
use blip::bot::{best, predict, Goal::*, Pad, Playthrough};

/// What the bot must show it can do in Bouncer.
pub const PLAYTHROUGHS: &[Playthrough] = &[
    Playthrough {
        name: "the paddle returns the ball and clears the first wall of bricks",
        seconds: 180.0,
        goals: &[AtLeast("bricks", 20.0), AtLeast("level", 2.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it catches the power-up drops",
        seconds: 240.0,
        goals: &[AtLeast("drops", 2.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "a bomb brick blasts its neighbours",
        level: BOMB_MIN_LEVEL as u32,
        seconds: 300.0,
        goals: &[AtLeast("bricks_blasted", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "steel bricks crack on the first hit and break on the second",
        level: STEEL_MIN_LEVEL as u32,
        seconds: 240.0,
        goals: &[AtLeast("steel_cracked", 1.0), AtLeast("level", STEEL_MIN_LEVEL as f64 + 1.0)],
        ..Playthrough::DEFAULT
    },
];

pub fn drive(g: &Game) {
    let mut pad = Pad::new();
    match g.state {
        State::Title | State::Launch => { pad.press_start(); }
        State::Play => {
            blip::bot::at(g.pad_x + g.pad_w / 2.0, PAD_Y as f32);
            pad.steer(g.pad_x + g.pad_w / 2.0, paddle_target(g), 5.0);
        }
        _ => {}
    }
    pad.send();
}

/// Where the paddle should be: under the first ball down, or under a good
/// drop while the ball is high and climbing.
fn paddle_target(g: &Game) -> f32 {
    let coming = g.extra.iter().copied().chain([g.ball()])
        .filter(|b| b.vy > 1.0 && b.y + (BALL_H as f32) < (PAD_Y + PAD_H) as f32)
        .map(|b| landing(&b));
    let landing = best(coming, |(_, t)| t);
    if let Some((x, _)) = landing {
        // Off-centre contact steers the ball: aim a little to send it inward.
        return x + if x < WIN_W as f32 / 2.0 { -g.pad_w * 0.15 } else { g.pad_w * 0.15 };
    }
    let ball_high = g.ball_y < PAD_Y as f32 - 220.0;
    let good_drop = pool_iter(&g.drops).filter(|d| d.kind != DropKind::Narrow).max_by(|a, b| a.y.total_cmp(&b.y));
    match good_drop {
        Some(d) if ball_high => d.x + DROP_W / 2.0,
        _ => ball_circle(g).0,
    }
}

/// Where ball `b` will cross the paddle line, the side walls folded in, and when.
fn landing(b: &Ball) -> (f32, f32) {
    let r = BALL_W as f32 / 2.0;
    let (cx, cy) = (b.x + r, b.y + r);
    predict::landing(cx, b.vx, cy + r, b.vy, PAD_Y as f32, (r, WIN_W as f32 - r))
}
