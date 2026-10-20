//! Rally assets: techno music (see `techno`) and plucky effects (see `cosy`);
//! the sprites are drawn at runtime. The music is synthesised on the device.

use crate::cosy::{self, Voice, H};
use crate::Asset;

#[path = "rally/gfx.rs"]
pub mod gfx;

#[path = "rally/tunes.rs"]
pub mod tunes;
pub use tunes::{music, music2, music3, music4, music5};

// ---- effects: a pluck ----------------------------------------------------
// Bubbler's warm, rounded style on Rally's own instrument: a thin plucked
// pulse, quick and bright, in G major pentatonic.

/// Ball off the side rail: a tiny pluck.
fn wall() -> Vec<u8> {
    cosy::finish_warm(cosy::run(&[74], 0.035, Voice::Marimba, 0.28), 9_000.0)
}

/// Ball off a paddle: a short boing with a pluck on top, a step higher for
/// the right paddle so the rally ping-pongs.
fn hit(right: bool) -> Vec<u8> {
    let (f, n) = if right { (262.0, 74) } else { (220.0, 71) };
    cosy::sfx(&cosy::mix(
        cosy::boing(0.09, f),
        &cosy::run(&[n], 0.03, Voice::Pluck, 0.35),
        0.0,
        1.0,
    ))
}

/// A point to the left player (the one at the controls): a bright run up.
fn point_won() -> Vec<u8> {
    cosy::jingle_with(
        &[79, 83, 86, 91, H],
        0.06,
        Voice::Pluck,
        0,
        Some((Voice::Bell, -1)),
    )
}

/// A point to the right: a short fall.
fn point_lost() -> Vec<u8> {
    cosy::jingle_with(
        &[74, H, 71, H, 67, H, H],
        0.07,
        Voice::Pluck,
        -1,
        Some((Voice::Bell, -2)),
    )
}

pub fn generate() -> Vec<Asset> {
    let mut assets = gfx::generate();
    assets.extend(vec![
        ("sounds/wall.wav", wall()),
        ("sounds/hit_l.wav", hit(false)),
        ("sounds/hit_r.wav", hit(true)),
        ("sounds/score_l.wav", point_won()),
        ("sounds/score_r.wav", point_lost()),
    ]);
    assets
}
