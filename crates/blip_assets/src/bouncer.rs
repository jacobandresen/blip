//! Bouncer (Breakout) assets: sprites, glockenspiel music (see `song`), and
//! effects from a toy box of rubber boings and glockenspiel (see `cosy`).

use crate::cosy::{self, Tone, Voice, H};
use crate::Asset;

#[path = "bouncer/gfx.rs"]
pub mod gfx;
pub use gfx::{BALL_PITCH_N, BALL_YAW_N, PADDLE_CAP};

#[path = "bouncer/tunes.rs"]
pub mod tunes;
pub use tunes::{bounce_song, bounce_wav, rebound_song, rebound_wav};

/// Bat cap width in source pixels; the game stretches only the slice
/// between the caps, so the ends keep their shape at any bat width.
// ---- effects --------------------------------------------------------------
// Pitch variants (`v`) keep repeated hits from sounding identical.
fn paddle_hit(v: usize) -> Vec<u8> {
    cosy::sfx(&cosy::boing(0.16, [196.0, 208.0, 220.0][v]))
}

/// A brick taking a hit without breaking: a muted glockenspiel tap.
fn brick_hit(v: usize) -> Vec<u8> {
    let mut tap = cosy::run(&[79 + 2 * v as i32], 0.05, Voice::Glock, 0.45);
    tap.truncate((0.25 * cosy::SR) as usize);
    cosy::sfx(&tap)
}

/// A brick breaking: a bright glockenspiel note on a soft pop.
fn brick_break(v: usize) -> Vec<u8> {
    let pop = cosy::snap(0.08, 1400.0 + 120.0 * v as f32, 0xB0 + v as u32);
    let bell = cosy::run(&[[84, 88, 91][v]], 0.05, Voice::Glock, 0.6);
    cosy::sfx(&cosy::mix(pop, &bell, 0.01, 1.0))
}

fn wall_hit() -> Vec<u8> {
    cosy::finish_warm(cosy::boing(0.1, 150.0), 11_000.0)
}

/// A ball lost: a rubbery sigh falling away.
fn life_lost() -> Vec<u8> {
    cosy::sfx(&cosy::glide(1.0, 520.0, 170.0, Tone::Sine, 0.8, 20.0))
}

fn win() -> Vec<u8> {
    cosy::jingle_with(
        &[72, 76, 79, 84, H, 79, 84, 88, H, H],
        0.09,
        Voice::Glock,
        0,
        Some((Voice::Bell, -1)),
    )
}

/// Pickups caught: a good one sparkles up the glockenspiel, the narrow bat
/// tumbles down it, and a life is a little fanfare.
fn pickup_good() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[84, 88, 91, 96], 0.045, Voice::Glock, 0.5))
}

fn pickup_bad() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[88, 84, 79, 72], 0.06, Voice::Glock, 0.45))
}

fn pickup_life() -> Vec<u8> {
    cosy::jingle_with(
        &[79, 84, 88, 91, H, 96, H, H],
        0.07,
        Voice::Glock,
        0,
        Some((Voice::Bell, -1)),
    )
}

/// A bomb brick going off: a soft pop over a low boom tumbling down. One
/// sound for the whole blast, not nine bricks at once.
fn bomb_sfx() -> Vec<u8> {
    let pop = cosy::snap(0.07, 700.0, 0xB0B1);
    cosy::sfx(&cosy::mix(
        pop,
        &cosy::bloop(0.42, 220.0, 55.0, 0xB0B2),
        0.01,
        1.0,
    ))
}

pub fn generate() -> Vec<Asset> {
    let mut assets = gfx::generate();
    assets.extend([
        ("sounds/paddle_hit_0.wav", paddle_hit(0)),
        ("sounds/paddle_hit_1.wav", paddle_hit(1)),
        ("sounds/paddle_hit_2.wav", paddle_hit(2)),
        ("sounds/brick_hit_0.wav", brick_hit(0)),
        ("sounds/brick_hit_1.wav", brick_hit(1)),
        ("sounds/brick_hit_2.wav", brick_hit(2)),
        ("sounds/brick_break_0.wav", brick_break(0)),
        ("sounds/brick_break_1.wav", brick_break(1)),
        ("sounds/brick_break_2.wav", brick_break(2)),
        ("sounds/wall_hit.wav", wall_hit()),
        ("sounds/bomb.wav", bomb_sfx()),
        ("sounds/life_lost.wav", life_lost()),
        ("sounds/win.wav", win()),
        ("sounds/pickup_good.wav", pickup_good()),
        ("sounds/pickup_bad.wav", pickup_bad()),
        ("sounds/pickup_life.wav", pickup_life()),
    ]);
    assets
}
