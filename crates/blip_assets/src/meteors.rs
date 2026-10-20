//! Meteors sound: music (see `song`) and effects (see `cosy`) on glass.

use crate::cosy::{self, Tone, Voice, H};
use crate::Asset;

#[path = "meteors/gfx.rs"]
pub mod gfx;

#[path = "meteors/tunes.rs"]
pub mod tunes;
pub use tunes::{drift_song, drift_wav, storm_song, storm_wav};

// ---- effects: glass ------------------------------------------------------
// Bubbler's warm, rounded style on Meteors' own instrument: glass, two sines
// a hair apart ringing long, in a floating E lydian.

/// The ship's shot: one clean glassy "pew", a pure sine sliding down and
/// fading to nothing (it fires fast, so it must end cleanly and quickly).
fn fire_zap() -> Vec<u8> {
    cosy::finish_warm(
        cosy::glide(0.09, 1400.0, 880.0, Tone::Sine, 1.6, 0.0),
        13_000.0,
    )
}

/// Thrust, retriggered while held: a soft low triangle hum.
fn thrust() -> Vec<u8> {
    cosy::finish_warm(cosy::glide(0.1, 82.0, 72.0, Tone::Tri, 0.6, 0.0), 10_000.0)
}

/// A rock breaking: a soft round thud falling away, deeper for a bigger
/// rock (`size` 0 large .. 2 small), with a faint glass chime on top.
fn bang(size: usize) -> Vec<u8> {
    let (f0, dur, chime) = [(170.0, 0.4, 64), (240.0, 0.28, 71), (360.0, 0.18, 76)][size];
    let thud = cosy::glide(dur, f0, f0 * 0.45, Tone::Sine, 1.3, 0.0);
    cosy::sfx(&cosy::mix(
        thud,
        &cosy::run(&[chime], 0.05, Voice::Glass, 0.18),
        0.0,
        1.0,
    ))
}

/// The saucers, heard in passing: a slow wobbling whistle, higher and
/// faster for the small one.
fn saucer(small: bool) -> Vec<u8> {
    let (f, depth, dur) = if small {
        (760.0, 110.0, 0.22)
    } else {
        (330.0, 45.0, 0.3)
    };
    cosy::finish_warm(cosy::glide(dur, f, f, Tone::Sine, 0.4, depth), 13_000.0)
}

/// The ship lost: a long glassy fall with a low chime under it.
fn ship_explosion() -> Vec<u8> {
    let fall = cosy::glide(1.2, 820.0, 90.0, Tone::Sine, 0.9, 18.0);
    cosy::sfx(&cosy::mix(
        fall,
        &cosy::run(&[52], 0.05, Voice::Glass, 0.5),
        0.0,
        1.0,
    ))
}

/// Hyperspace: a sweep up into a shower of glass.
fn hyperspace() -> Vec<u8> {
    let sweep = cosy::glide(0.22, 300.0, 1500.0, Tone::Sine, 0.6, 0.0);
    cosy::sfx(&cosy::mix(sweep, &cosy::sparkle(0.3, 88, 5, 5), 0.12, 0.8))
}

/// An extra ship: a glass fanfare in E lydian.
fn extra_life() -> Vec<u8> {
    cosy::jingle_with(
        &[76, 83, 88, H, 90, 88, 95, H, H],
        0.09,
        Voice::Glass,
        0,
        Some((Voice::Bell, -1)),
    )
}

pub fn generate() -> Vec<Asset> {
    let mut assets = gfx::generate();
    assets.extend(vec![
        ("sounds/fire.wav", fire_zap()),
        ("sounds/thrust.wav", thrust()),
        ("sounds/bang_large.wav", bang(0)),
        ("sounds/bang_medium.wav", bang(1)),
        ("sounds/bang_small.wav", bang(2)),
        ("sounds/saucer_big.wav", saucer(false)),
        ("sounds/saucer_small.wav", saucer(true)),
        ("sounds/hyperspace.wav", hyperspace()),
        ("sounds/extra_life.wav", extra_life()),
        ("sounds/ship_explosion.wav", ship_explosion()),
    ]);
    assets
}
