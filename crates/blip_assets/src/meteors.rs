//! Meteors sound: music (see `song`) and effects (see `cosy`) on glass.


use crate::cosy::{self, Tone, Voice, H};
use crate::song::{major, minor, Chord, Groove, Song, R};
use crate::Asset;

// ---- music: two tunes on glass --------------------------------------------
// Synthesised on the device (see `song`). E lydian, whose raised fourth
// (A sharp) sounds like floating; DRIFT plays the first waves, STORM from
// wave 5.

const E: Chord = major(40);
const FS: Chord = major(42);
const GSM: Chord = minor(44);
const CSM: Chord = minor(37);
const B: Chord = major(47);

/// Slow and spacious: long glass notes that hang in the air.
pub fn drift_wav() -> Vec<u8> {
    Song {
        bpm: 96.0,
        melody: &[
            [76, H, H, 83, 82, H, 80, H], [78, H, H, H, R, R, 76, 78],
            [80, H, H, 82, 83, H, 87, H], [85, H, H, H, R, R, R, R],
            [76, H, H, 83, 82, H, 80, H], [78, H, 80, H, 82, H, 83, H],
            [85, H, 83, H, 82, H, 78, H], [76, H, H, H, R, R, R, R],
            [88, H, H, 87, 83, H, H, H], [85, H, 83, H, 82, H, R, R],
            [80, H, H, 82, 83, H, 85, H], [87, H, H, H, R, R, R, R],
            [88, H, 87, H, 83, H, 80, H], [82, H, H, 80, 78, H, H, H],
            [80, H, 78, H, 75, H, 71, H], [76, H, H, H, R, R, R, R],
        ],
        chords: &[
            [E, E], [FS, FS], [GSM, B], [CSM, CSM], [E, E], [FS, FS], [CSM, FS], [E, E],
            [E, B], [CSM, FS], [GSM, CSM], [B, B], [E, GSM], [FS, FS], [GSM, B], [E, E],
        ],
        lead: Voice::Glass,
        lead_vol: 0.40,
        harmony: Some((Voice::Bell, -12)),
        bass: [0, R, 7, R],
        groove: Groove::Brush,
        seed: 0x3E7E_0001,
        ..Song::DEFAULT
    }
    .render()
}

/// Driving: the same scale in a rolling figure over a steady kick.
pub fn storm_wav() -> Vec<u8> {
    Song {
        bpm: 140.0,
        melody: &[
            [76, 83, 80, 83, 76, 83, 82, 83], [78, 85, 82, 85, 78, 85, 83, 85],
            [80, 87, 83, 87, 80, 87, 85, 87], [88, H, 87, H, 85, H, 83, H],
            [76, 83, 80, 83, 76, 83, 82, 83], [78, 85, 82, 85, 78, 85, 83, 85],
            [85, H, 83, H, 82, H, 78, H], [76, H, H, H, R, R, R, R],
        ],
        chords: &[
            [E, E], [FS, FS], [GSM, GSM], [CSM, B], [E, E], [FS, FS], [CSM, FS], [E, E],
        ],
        lead: Voice::Glass,
        lead_vol: 0.30,
        harmony: Some((Voice::Saw, -12)),
        arp: false,
        bass: [0, 12, 0, 12],
        bass_voice: Voice::Growl,
        groove: Groove::Drive,
        action: true,
        seed: 0x3E7E_0002,
        ..Song::DEFAULT
    }
    .render()
}

// ---- effects: glass ------------------------------------------------------
// Bubbler's warm, rounded style on Meteors' own instrument: glass, two sines
// a hair apart ringing long, in a floating E lydian.

/// The ship's shot: one clean glassy "pew", a pure sine sliding down and
/// fading to nothing (it fires fast, so it must end cleanly and quickly).
fn fire_zap() -> Vec<u8> {
    cosy::finish_warm(cosy::glide(0.09, 1400.0, 880.0, Tone::Sine, 1.6, 0.0), 13_000.0)
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
    cosy::sfx(&cosy::mix(thud, &cosy::run(&[chime], 0.05, Voice::Glass, 0.18), 0.0, 1.0))
}

/// The saucers, heard in passing: a slow wobbling whistle, higher and
/// faster for the small one.
fn saucer(small: bool) -> Vec<u8> {
    let (f, depth, dur) = if small { (760.0, 110.0, 0.22) } else { (330.0, 45.0, 0.3) };
    cosy::finish_warm(cosy::glide(dur, f, f, Tone::Sine, 0.4, depth), 13_000.0)
}

/// The ship lost: a long glassy fall with a low chime under it.
fn ship_explosion() -> Vec<u8> {
    let fall = cosy::glide(1.2, 820.0, 90.0, Tone::Sine, 0.9, 18.0);
    cosy::sfx(&cosy::mix(fall, &cosy::run(&[52], 0.05, Voice::Glass, 0.5), 0.0, 1.0))
}

/// Hyperspace: a sweep up into a shower of glass.
fn hyperspace() -> Vec<u8> {
    let sweep = cosy::glide(0.22, 300.0, 1500.0, Tone::Sine, 0.6, 0.0);
    cosy::sfx(&cosy::mix(sweep, &cosy::sparkle(0.3, 88, 5, 5), 0.12, 0.8))
}

/// An extra ship: a glass fanfare in E lydian.
fn extra_life() -> Vec<u8> {
    cosy::jingle_with(&[76, 83, 88, H, 90, 88, 95, H, H], 0.09, Voice::Glass, 0, Some((Voice::Bell, -1)))
}

pub fn generate() -> Vec<Asset> {
    vec![
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
    ]
}
