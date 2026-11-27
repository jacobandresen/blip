// Compiled by build.rs, and by the game for its tests: each uses part of this file.
#![allow(dead_code)]

use blip::song::{Groove, Song, H, R};

// ---- music: our own tunes ----------------------------------------------------------------------------------------
// Original melodies only (user, 2026-11-27), short and hummable: one for the title and one for each stretch of levels,
// rising in pace with the snake (main.rs, TUNE_FROM). Each is a hook of eight bars, looped: the tune on one instrument
// over a plain bass and light drums, no arpeggio, fills or doubling voice. Synthesised on the device (see `song`).
use blip::cosy::{self, Voice, H as HOLD};
use blip::Asset;
use blip::song::chords::{AM, C, DM, E, F, G};

/// What every Serpent tune leaves out, so the melody carries it.
const PLAIN: Song<'static> = Song { arp: false, fills: false, harmony: None, echo: 0.15, ..Song::DEFAULT };

/// Meadow, over the title: a music-box lullaby in C, the snake asleep in the grass.
pub fn meadow_song() -> Song<'static> {
    Song {
        bpm: 112.0,
        melody: &[
            [72, H, 76, 79, 77, H, 76, 74],
            [72, H, 74, 76, 74, H, H, R],
            [72, H, 76, 79, 81, H, 79, 77],
            [76, 74, 72, 74, 72, H, H, R],
            [79, H, 77, 76, 77, H, 76, 74],
            [76, H, 74, 72, 74, H, H, R],
            [72, H, 76, 79, 81, H, 83, 84],
            [79, H, 74, 76, 72, H, H, R],
        ],
        chords: &[[C, C], [C, G], [F, F], [G, C], [C, C], [F, G], [F, F], [G, C]],
        lead: Voice::Glock,
        lead_vol: 0.5,
        bass: [0, R, R, R],
        groove: Groove::None,
        echo: 0.25,
        seed: 0xE115_0000,
        ..PLAIN
    }
}

/// Stroll, levels 1–2: an easy hop up the scale and a step back, on marimba with a soft beat. C major.
pub fn stroll_song() -> Song<'static> {
    Song {
        bpm: 120.0,
        melody: &[
            [67, 72, H, 72, 74, 76, H, 72],
            [74, H, 72, H, 69, H, 67, R],
            [67, 72, H, 72, 74, 76, H, 79],
            [77, H, 76, H, 74, H, H, R],
            [67, 72, H, 72, 74, 76, H, 72],
            [74, H, 72, H, 69, H, 67, H],
            [69, H, 72, H, 74, H, 76, 74],
            [72, H, H, H, R, R, R, R],
        ],
        chords: &[[C, C], [F, C], [C, C], [G, G], [C, C], [F, C], [F, G], [C, C]],
        lead: Voice::Marimba,
        lead_vol: 0.6,
        bass: [0, R, 7, R],
        groove: Groove::Brush,
        seed: 0x5111_7000,
        ..PLAIN
    }
}

/// Burrow, levels 3–4: a sneaky plucked figure in A minor over a backbeat, asking twice and answering.
pub fn burrow_song() -> Song<'static> {
    Song {
        bpm: 128.0,
        melody: &[
            [69, H, 72, 69, 76, H, 74, 72],
            [71, H, 74, 71, 76, H, H, R],
            [69, H, 72, 69, 76, H, 79, 77],
            [76, 74, 72, 71, 69, H, H, R],
            [69, H, 72, 69, 76, H, 74, 72],
            [71, H, 74, 71, 76, H, H, R],
            [77, 76, 74, 72, 71, 72, 74, H],
            [69, H, H, H, R, R, R, R],
        ],
        chords: &[[AM, AM], [E, E], [AM, AM], [E, AM], [AM, AM], [E, E], [DM, E], [AM, AM]],
        lead: Voice::Pluck,
        lead_vol: 0.6,
        bass: [0, R, 7, R],
        groove: Groove::Backbeat,
        seed: 0x9A01_3000,
        ..PLAIN
    }
}

/// Picnic, levels 5–6: a bright marching tune in C that climbs a step each phrase.
pub fn picnic_song() -> Song<'static> {
    Song {
        bpm: 140.0,
        melody: &[
            [72, H, 76, 74, 72, H, 79, H],
            [77, H, 76, 74, 76, H, H, R],
            [74, H, 77, 76, 74, H, 81, H],
            [79, H, 77, 76, 77, H, H, R],
            [76, H, 79, 77, 76, H, 84, H],
            [83, H, 81, 79, 81, H, 79, H],
            [77, 76, 74, 76, 72, H, 71, H],
            [72, H, H, H, R, R, R, R],
        ],
        chords: &[[C, C], [F, C], [DM, DM], [G, G], [C, C], [AM, G], [F, G], [C, C]],
        lead: Voice::Lead,
        lead_vol: 0.34,
        bass: [0, 7, 0, 7],
        groove: Groove::Backbeat,
        seed: 0x0DE0_9000,
        ..PLAIN
    }
}

/// Scamper, levels 7–8: quick running arpeggios on the bell, a step a sixteenth. A minor.
pub fn scamper_song() -> Song<'static> {
    Song {
        bpm: 176.0,
        melody: &[
            [69, 72, 76, 72, 69, 72, 76, 79],
            [77, 76, 74, 72, 74, H, H, R],
            [69, 72, 76, 72, 69, 72, 76, 81],
            [79, 77, 76, 74, 76, H, H, R],
            [72, 76, 79, 76, 72, 76, 79, 84],
            [83, 81, 79, 77, 79, H, H, R],
            [77, 76, 74, 72, 71, 72, 74, 71],
            [69, H, H, H, R, R, R, R],
        ],
        chords: &[[AM, AM], [DM, G], [AM, AM], [E, E], [C, C], [G, G], [DM, E], [AM, AM]],
        lead: Voice::Bell,
        lead_vol: 0.4,
        bass: [0, R, 7, R],
        groove: Groove::Backbeat,
        seed: 0x7A2C_0000,
        ..PLAIN
    }
}

/// Finale, level 9 on: a galloping see-saw on the accordion, a kick on every beat. C major.
pub fn finale_song() -> Song<'static> {
    Song {
        bpm: 150.0,
        melody: &[
            [79, H, 76, H, 79, H, 76, H],
            [77, 76, 74, 76, 77, H, 74, H],
            [77, H, 74, H, 77, H, 74, H],
            [76, 74, 72, 74, 76, H, H, R],
            [79, H, 76, H, 79, H, 84, H],
            [83, 81, 79, 81, 83, H, 79, H],
            [81, 79, 77, 76, 74, 76, 77, 74],
            [72, H, H, H, R, R, R, R],
        ],
        chords: &[[C, C], [G, G], [G, G], [C, C], [C, C], [G, G], [F, G], [C, C]],
        lead: Voice::Accordion,
        lead_vol: 0.45,
        bass: [0, 12, 0, 12],
        groove: Groove::FourFloor,
        seed: 0xF6E2_9000,
        ..PLAIN
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
        ("MEADOW", &|| meadow_song().render()),
        ("STROLL", &|| stroll_song().render()),
        ("BURROW", &|| burrow_song().render()),
        ("PICNIC", &|| picnic_song().render()),
        ("SCAMPER", &|| scamper_song().render()),
        ("FINALE", &|| finale_song().render()),
    ])
}
