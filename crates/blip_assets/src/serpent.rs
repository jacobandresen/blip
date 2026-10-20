//! Serpent sounds and tunes.

use crate::cosy::{self, Voice, H as HOLD};
use crate::Asset;

#[path = "serpent/gfx.rs"]
pub mod gfx;
#[path = "serpent/tunes.rs"]
pub mod tunes;
pub use tunes::{frenzy_song, frenzy_wav, slither_song, slither_wav};

// ---- effects: a marimba --------------------------------------------------
// Bubbler's warm, rounded style on Serpent's own instrument: wooden bars in
// A minor pentatonic, low and earthy.

/// Eating: two quick taps on the marimba, up a fourth.
fn eat_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[76, 81], 0.05, Voice::Marimba, 0.7))
}

/// A bonus fruit appearing: three soft taps climbing the pentatonic.
fn bonus_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[69, 72, 76], 0.07, Voice::Marimba, 0.45))
}

/// A bonus fruit eaten: a quick run up the scale with a glassy sparkle over it.
fn bonus_eat_sfx() -> Vec<u8> {
    let run = cosy::run(&[69, 72, 74, 76, 79, 81, 84], 0.045, Voice::Marimba, 0.6);
    cosy::sfx(&cosy::mix(run, &cosy::sparkle(0.3, 93, 4, 3), 0.18, 0.35))
}

/// Next level: a marimba phrase with a bell under it.
fn level_sfx() -> Vec<u8> {
    cosy::jingle_with(
        &[69, 72, 76, 81, HOLD, 76, 81, HOLD],
        0.09,
        Voice::Marimba,
        0,
        Some((Voice::Bell, -1)),
    )
}

/// Game over: the phrase walking back down, slowly.
fn game_over_sfx() -> Vec<u8> {
    cosy::jingle_with(
        &[
            81, HOLD, 79, HOLD, 76, HOLD, 74, HOLD, 72, HOLD, 69, HOLD, HOLD, HOLD,
        ],
        0.12,
        Voice::Marimba,
        0,
        Some((Voice::Bell, -1)),
    )
}

pub fn generate() -> Vec<Asset> {
    let mut assets = gfx::generate();
    assets.extend(vec![
        ("sounds/eat.wav", eat_sfx()),
        ("sounds/game_over.wav", game_over_sfx()),
        ("sounds/bonus.wav", bonus_sfx()),
        ("sounds/bonus_eat.wav", bonus_eat_sfx()),
        ("sounds/level.wav", level_sfx()),
    ]);
    assets
}
