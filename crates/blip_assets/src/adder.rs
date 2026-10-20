//! Adder sounds and tunes.

use crate::cosy::{self, Tone, Voice, H as HOLD};
use crate::Asset;

#[path = "adder/gfx.rs"]
pub mod gfx;
#[path = "adder/tunes.rs"]
pub mod tunes;
pub use tunes::{coil_song, coil_wav, hunt_song, hunt_wav};

// ---- effects: dry plucks and rustles -------------------------------------

/// Eating: two taps on the pluck, a fourth apart, low in the instrument.
fn eat_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[60, 67], 0.05, Voice::Pluck, 0.7))
}

/// The shed: a rustle that slides down out from under the tail.
fn shed_sfx() -> Vec<u8> {
    let rustle: Vec<f32> = cosy::glide(0.26, 1900.0, 620.0, Tone::Pulse(0.22), 1.7, 45.0)
        .iter()
        .map(|v| v * 0.45)
        .collect();
    cosy::sfx(&cosy::mix(
        rustle,
        &cosy::run(&[41, 45], 0.06, Voice::Pluck, 0.4),
        0.0,
        1.0,
    ))
}

/// The throttle going down: a short whip, played once per strike, not per step.
fn strike_sfx() -> Vec<u8> {
    let whip: Vec<f32> = cosy::glide(0.16, 1100.0, 260.0, Tone::Pulse(0.125), 1.1, 0.0)
        .iter()
        .map(|v| v * 0.5)
        .collect();
    cosy::sfx(&cosy::mix(
        whip,
        &cosy::run(&[72], 0.05, Voice::Pluck, 0.35),
        0.0,
        1.0,
    ))
}

/// Through a hole: a rising slip, which is what getting away with it sounds
/// like.
fn through_sfx() -> Vec<u8> {
    let slip: Vec<f32> = cosy::glide(0.22, 320.0, 1500.0, Tone::Tri, 1.4, 30.0)
        .iter()
        .map(|v| v * 0.4)
        .collect();
    cosy::sfx(&cosy::mix(
        slip,
        &cosy::run(&[81, 88], 0.05, Voice::Pluck, 0.45),
        0.02,
        1.0,
    ))
}

/// A new pit: four plucked notes climbing, a bell under them.
fn pit_sfx() -> Vec<u8> {
    cosy::jingle_with(
        &[62, 66, 69, 74, HOLD, HOLD],
        0.07,
        Voice::Pluck,
        0,
        Some((Voice::Bell, -1)),
    )
}

/// Game over: the phrase sinking, one long note at a time.
fn game_over_sfx() -> Vec<u8> {
    cosy::jingle_with(
        &[69, HOLD, 65, HOLD, 62, HOLD, 57, HOLD, HOLD, HOLD],
        0.13,
        Voice::Pluck,
        0,
        Some((Voice::Bell, -2)),
    )
}

pub fn generate() -> Vec<Asset> {
    let mut assets = gfx::generate();
    assets.extend(vec![
        ("sounds/eat.wav", eat_sfx()),
        ("sounds/through.wav", through_sfx()),
        ("sounds/shed.wav", shed_sfx()),
        ("sounds/strike.wav", strike_sfx()),
        ("sounds/pit.wav", pit_sfx()),
        ("sounds/game_over.wav", game_over_sfx()),
    ]);
    assets
}
