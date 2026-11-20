// Compiled by build.rs, and by the game for its tests: each uses part of this file.
#![allow(dead_code)]

use blip::song::{Groove, Song, H, R};

// ---- music: two tunes on the marimba -------------------------------------
// Synthesised on the device (see `song`): A minor pentatonic, the notes a
// snake winds through. SLITHER plays the early levels, PROWL from level 3, FRENZY from level 5.
use blip::cosy::{self, Voice, H as HOLD};
use blip::Asset;
use blip::song::chords::{AM, C, DM, EM, F, G};


/// The calm tune: stepwise, winding, a little sly.
pub fn slither_song() -> Song<'static> {
    Song {
        bpm: 108.0,
        melody: &[
            [69, H, 72, 74, 76, H, 74, 72],
            [69, H, H, 67, 69, H, R, R],
            [72, H, 74, 76, 79, H, 76, 74],
            [76, H, H, H, R, R, 74, 72],
            [69, H, 72, 74, 76, H, 79, 81],
            [79, H, 76, H, 74, H, 72, H],
            [74, 76, 74, 72, 69, H, 67, H],
            [69, H, H, H, R, R, R, R],
            [81, H, 79, 76, 79, H, 76, 74],
            [76, H, 74, 72, 74, H, R, R],
            [72, H, 74, 76, 74, 72, 69, H],
            [67, H, 69, H, 72, H, R, R],
            [81, H, 79, H, 76, H, 79, 81],
            [84, H, 81, H, 79, H, 76, H],
            [74, 76, 79, 76, 74, 72, 67, H],
            [69, H, H, H, R, R, R, R],
        ],
        chords: &[
            [AM, AM],
            [AM, G],
            [C, C],
            [G, G],
            [AM, AM],
            [C, G],
            [DM, EM],
            [AM, AM],
            [F, F],
            [C, C],
            [AM, AM],
            [G, G],
            [F, F],
            [G, G],
            [DM, EM],
            [AM, AM],
        ],
        lead: Voice::Marimba,
        lead_vol: 0.6,
        harmony: Some((Voice::Marimba, -12)),
        bass: [0, R, 7, R],
        groove: Groove::Brush,
        seed: 0x5111_7000,
        ..Song::DEFAULT
    }
}

/// The middle tune: the snake on the hunt, a backbeat under it and the melody answering itself, a step between the
/// calm and the fast one.
pub fn prowl_song() -> Song<'static> {
    Song {
        bpm: 122.0,
        melody: &[
            [69, H, 72, H, 76, 74, 72, H],
            [74, H, 76, H, 79, H, 76, H],
            [81, H, 79, 76, 74, H, 72, 74],
            [76, H, H, H, R, R, 72, 74],
            [69, H, 72, H, 76, 74, 72, H],
            [74, H, 76, H, 79, H, 81, H],
            [84, H, 81, 79, 76, H, 74, 72],
            [69, H, H, H, R, R, R, R],
            [81, 81, R, 79, 81, H, 84, H],
            [79, H, 76, H, 74, H, 76, H],
            [72, 72, R, 74, 76, H, 79, H],
            [76, H, 74, H, 72, H, R, R],
            [69, H, 72, H, 76, 74, 72, H],
            [74, H, 76, H, 79, H, 76, 79],
            [81, H, 84, H, 81, 79, 76, 74],
            [69, H, H, H, R, R, R, R],
        ],
        chords: &[
            [AM, AM],
            [G, C],
            [F, F],
            [EM, EM],
            [AM, AM],
            [G, C],
            [F, G],
            [AM, AM],
            [F, F],
            [C, C],
            [DM, DM],
            [EM, EM],
            [AM, AM],
            [G, C],
            [F, G],
            [AM, AM],
        ],
        lead: Voice::Marimba,
        lead_vol: 0.5,
        harmony: Some((Voice::Marimba, -12)),
        bass: [0, 7, 12, 7],
        groove: Groove::Backbeat,
        seed: 0x9A01_3000,
        ..Song::DEFAULT
    }
}

/// The fast tune: the same scale, running and leaping.
pub fn frenzy_song() -> Song<'static> {
    Song {
        bpm: 136.0,
        melody: &[
            [81, 79, 76, H, 74, 76, 79, H],
            [81, H, 84, H, 81, 79, 76, H],
            [74, 72, 69, H, 72, 74, 76, H],
            [74, H, 72, H, 69, H, R, R],
            [81, 79, 76, H, 74, 76, 79, H],
            [84, H, 86, H, 84, 81, 79, H],
            [76, 79, 81, 79, 76, 74, 72, 74],
            [69, H, H, H, R, 76, 79, 81],
            [84, H, 81, H, 79, H, 81, H],
            [76, H, 79, H, 74, H, R, R],
            [72, 74, 76, 79, 81, H, 79, 76],
            [79, H, H, H, R, R, R, R],
            [84, H, 81, H, 79, H, 81, 84],
            [86, H, 84, H, 81, H, 79, H],
            [76, 79, 81, 79, 76, 74, 72, 74],
            [69, H, H, H, R, R, R, R],
        ],
        chords: &[
            [AM, AM],
            [F, G],
            [DM, DM],
            [EM, AM],
            [AM, AM],
            [F, G],
            [C, G],
            [AM, AM],
            [F, F],
            [C, C],
            [AM, AM],
            [G, G],
            [F, F],
            [G, G],
            [C, EM],
            [AM, AM],
        ],
        lead: Voice::Marimba,
        lead_vol: 0.42,
        harmony: Some((Voice::Bell, -12)),
        bass: [0, 12, 0, 12],
        groove: Groove::FourFloor,
        seed: 0xF6E2_9000,
        ..Song::DEFAULT
    }
}

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

/// The golden mouse hopping away: a tiny squeak, two quick chirps sliding up.
fn squeak_sfx() -> Vec<u8> {
    let chirp = cosy::glide(0.05, 1700.0, 2500.0, cosy::Tone::Sine, 0.5, 0.0);
    cosy::sfx(&cosy::mix(chirp.clone(), &chirp, 0.07, 0.8))
}

/// The effects, as `sounds/NAME.wav`.
pub fn sounds() -> Vec<Asset> {
    vec![
        ("sounds/eat.wav", eat_sfx()),
        ("sounds/game_over.wav", game_over_sfx()),
        ("sounds/bonus.wav", bonus_sfx()),
        ("sounds/bonus_eat.wav", bonus_eat_sfx()),
        ("sounds/level.wav", level_sfx()),
        ("sounds/squeak.wav", squeak_sfx()),
    ]
}

/// The music: each tune as a WAV to play.
pub fn music() -> Vec<(String, Vec<u8>)> {
    blip::wav::tunes(&[
        ("SLITHER", &|| slither_song().render()),
        ("PROWL", &|| prowl_song().render()),
        ("FRENZY", &|| frenzy_song().render()),
    ])
}
