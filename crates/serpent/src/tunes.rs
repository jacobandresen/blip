// Compiled by build.rs, and by the game for its tests: each uses part of this file.
#![allow(dead_code)]

use blip::song::{Groove, Song, H, R};

// ---- music: themes everyone knows, our own arrangements --------------------------------------------------------
// Public-domain classics, so a player hums along from the first bar, one for the title and one for each stretch of
// levels, rising in pace with the snake (main.rs, TUNE_FROM): Für Elise on a music box over the title, Grieg's
// Morning Mood, his Hall of the Mountain King, Beethoven's Ode to Joy, Mozart's Turkish March and Offenbach's
// Can-can. Synthesised on the device (see `song`).
use blip::cosy::{self, Voice, H as HOLD};
use blip::Asset;
use blip::song::chords::{AM, B, C, CSM, DM, E, EM, FS, G, GSM};

/// Für Elise (Beethoven, 1810): the opening, a step a sixteenth, on a music box, the way a jewellery box plays it.
pub fn elise_song() -> Song<'static> {
    Song {
        bpm: 168.0,
        melody: &[
            [76, 75, 76, 75, 76, 71, 74, 72],
            [69, H, 60, 64, 69, 71, H, 64],
            [68, 71, 72, H, 64, 76, 75, 76],
            [75, 76, 71, 74, 72, 69, H, 60],
            [64, 69, 71, H, 64, 72, 71, 69],
            [H, H, H, R, R, R, R, R],
            [76, 75, 76, 75, 76, 71, 74, 72],
            [69, H, 60, 64, 69, 71, H, 64],
            [68, 71, 72, H, 64, 76, 75, 76],
            [75, 76, 71, 74, 72, 69, H, 60],
            [64, 69, 71, H, 64, 72, 71, 69],
            [H, H, H, R, R, R, R, R],
        ],
        chords: &[
            [AM, AM], [AM, E], [E, AM], [E, AM], [E, AM], [AM, AM],
            [AM, AM], [AM, E], [E, AM], [E, AM], [E, AM], [AM, AM],
        ],
        lead: Voice::Glock,
        lead_vol: 0.5,
        bass: [0, R, R, R],
        arp: false,
        groove: Groove::None,
        fills: false,
        echo: 0.25,
        seed: 0xE115_0000,
        ..Song::DEFAULT
    }
}

/// Morning Mood (Grieg, 1875): the flute's sunrise, sol-mi-re-do-re-mi, on marimba and glockenspiel. E major
/// pentatonic, rising; the middle phrase turns to the minor and back.
pub fn morning_song() -> Song<'static> {
    Song {
        bpm: 104.0,
        melody: &[
            [83, H, 80, 78, 76, 78, 80, H],
            [83, H, 80, 78, 76, 78, 80, H],
            [78, 80, 83, 80, 83, 85, 80, 85],
            [83, H, 80, H, 78, H, 76, H],
            [83, H, 80, 78, 76, 78, 80, H],
            [83, H, 80, 78, 76, 78, 80, H],
            [78, 80, 83, 80, 83, 85, 80, 85],
            [88, H, H, H, R, R, R, R],
            [80, H, 76, 75, 73, 75, 76, H],
            [80, H, 76, 75, 73, 75, 76, H],
            [75, 76, 80, 76, 80, 81, 76, 81],
            [80, H, 76, H, 75, H, 73, H],
            [83, H, 80, 78, 76, 78, 80, H],
            [83, H, 80, 78, 76, 78, 80, H],
            [78, 80, 83, 80, 83, 85, 80, 85],
            [83, H, 80, H, 76, H, H, H],
        ],
        chords: &[
            [E, E], [E, E], [E, CSM], [B, E],
            [E, E], [E, E], [E, CSM], [E, E],
            [GSM, GSM], [GSM, GSM], [GSM, CSM], [GSM, GSM],
            [E, E], [E, E], [E, CSM], [B, E],
        ],
        lead: Voice::Marimba,
        lead_vol: 0.6,
        harmony: Some((Voice::Glock, -12)),
        bass: [0, R, 7, R],
        groove: Groove::Brush,
        echo: 0.3,
        seed: 0x5111_7000,
        ..Song::DEFAULT
    }
}

/// In the Hall of the Mountain King (Grieg, 1875): the trolls creeping, plucked, a backbeat under them. In A minor,
/// the second half the same theme a fifth up, as Grieg has it.
pub fn mountain_song() -> Song<'static> {
    Song {
        bpm: 128.0,
        melody: &[
            [69, 71, 72, 74, 76, 72, 76, H],
            [75, 71, 75, H, 74, 70, 74, H],
            [69, 71, 72, 74, 76, 72, 76, 81],
            [79, 76, 72, 76, 79, H, H, R],
            [69, 71, 72, 74, 76, 72, 76, H],
            [75, 71, 75, H, 74, 70, 74, H],
            [69, 71, 72, 74, 76, 72, 76, 81],
            [79, 76, 72, 76, 79, H, H, R],
            [76, 78, 79, 81, 83, 79, 83, H],
            [82, 78, 82, H, 81, 77, 81, H],
            [76, 78, 79, 81, 83, 79, 83, 88],
            [86, 83, 79, 83, 86, H, H, R],
            [69, 71, 72, 74, 76, 72, 76, H],
            [75, 71, 75, H, 74, 70, 74, H],
            [69, 71, 72, 74, 76, 72, 76, 81],
            [76, H, 72, H, 69, H, R, R],
        ],
        chords: &[
            [AM, AM], [E, DM], [AM, AM], [C, C],
            [AM, AM], [E, DM], [AM, AM], [C, C],
            [EM, EM], [FS, DM], [EM, EM], [G, G],
            [AM, AM], [E, DM], [AM, AM], [E, AM],
        ],
        lead: Voice::Pluck,
        lead_vol: 0.55,
        harmony: Some((Voice::Marimba, -12)),
        bass: [0, 7, 12, 7],
        groove: Groove::Backbeat,
        seed: 0x9A01_3000,
        ..Song::DEFAULT
    }
}

/// Ode to Joy (Beethoven's Ninth, 1824): the tune everyone can whistle, marching, on a bright lead with a backbeat.
/// C major: the theme, its answer, the middle phrase and the theme again.
pub fn ode_song() -> Song<'static> {
    Song {
        bpm: 136.0,
        melody: &[
            [76, H, 76, H, 77, H, 79, H],
            [79, H, 77, H, 76, H, 74, H],
            [72, H, 72, H, 74, H, 76, H],
            [76, H, H, 74, 74, H, H, H],
            [76, H, 76, H, 77, H, 79, H],
            [79, H, 77, H, 76, H, 74, H],
            [72, H, 72, H, 74, H, 76, H],
            [74, H, H, 72, 72, H, H, H],
            [74, H, 74, H, 76, H, 72, H],
            [74, H, 76, 77, 76, H, 72, H],
            [74, H, 76, 77, 76, H, 74, H],
            [72, H, 74, H, 67, H, H, H],
            [76, H, 76, H, 77, H, 79, H],
            [79, H, 77, H, 76, H, 74, H],
            [72, H, 72, H, 74, H, 76, H],
            [74, H, H, 72, 72, H, H, H],
        ],
        chords: &[
            [C, C], [G, C], [C, C], [G, G],
            [C, C], [G, C], [C, C], [G, C],
            [G, C], [G, C], [G, C], [C, G],
            [C, C], [G, C], [C, C], [G, C],
        ],
        lead: Voice::Lead,
        lead_vol: 0.34,
        harmony: Some((Voice::Marimba, -12)),
        bass: [0, 7, 0, 7],
        groove: Groove::Backbeat,
        seed: 0x0DE0_9000,
        ..Song::DEFAULT
    }
}

/// The Turkish March (Mozart's Rondo alla Turca, 1783): its running opening, a step a sixteenth, quick and bright,
/// on the bell with a kick on every beat, twice over (the harmony joins the second time). A minor.
pub fn march_song() -> Song<'static> {
    Song {
        bpm: 228.0,
        melody: &[
            [71, 69, 68, 69, 72, H, H, H],
            [74, 72, 71, 72, 76, H, H, H],
            [77, 76, 75, 76, 83, 81, 80, 81],
            [83, 81, 80, 81, 84, H, H, H],
            [81, 83, 84, 83, 81, 80, 81, 76],
            [77, 74, 72, 71, 69, H, H, R],
            [71, 69, 68, 69, 72, H, H, H],
            [74, 72, 71, 72, 76, H, H, H],
            [77, 76, 75, 76, 83, 81, 80, 81],
            [83, 81, 80, 81, 84, H, H, H],
            [81, 83, 84, 83, 81, 80, 81, 76],
            [77, 74, 72, 71, 69, H, R, R],
            [71, 69, 68, 69, 72, H, H, H],
            [74, 72, 71, 72, 76, H, H, H],
            [77, 76, 75, 76, 83, 81, 80, 81],
            [83, 81, 80, 81, 84, H, H, H],
            [81, 83, 84, 83, 81, 80, 81, 76],
            [77, 74, 72, 71, 69, H, H, R],
            [71, 69, 68, 69, 72, H, H, H],
            [74, 72, 71, 72, 76, H, H, H],
            [77, 76, 75, 76, 83, 81, 80, 81],
            [83, 81, 80, 81, 84, H, H, H],
            [81, 83, 84, 83, 81, 80, 81, 76],
            [77, 74, 72, 71, 69, H, R, R],
        ],
        chords: &[
            [AM, AM], [AM, C], [E, AM], [E, AM], [AM, E], [DM, AM],
            [AM, AM], [AM, C], [E, AM], [E, AM], [AM, E], [DM, AM],
            [AM, AM], [AM, C], [E, AM], [E, AM], [AM, E], [DM, AM],
            [AM, AM], [AM, C], [E, AM], [E, AM], [AM, E], [DM, AM],
        ],
        lead: Voice::Bell,
        lead_vol: 0.36,
        harmony: Some((Voice::Marimba, -12)),
        bass: [0, 12, 0, 12],
        groove: Groove::FourFloor,
        seed: 0x7A2C_0000,
        ..Song::DEFAULT
    }
}

/// The Can-can (Offenbach's Galop infernal, 1858): high kicks on the accordion, a kick on every beat. C major.
pub fn cancan_song() -> Song<'static> {
    Song {
        bpm: 150.0,
        melody: &[
            [72, H, H, H, 74, 77, 76, 74],
            [79, H, 79, H, 79, 81, 76, 77],
            [74, H, 74, H, 74, 77, 76, 74],
            [72, 84, 83, 81, 79, 77, 76, 74],
            [72, H, H, H, 74, 77, 76, 74],
            [79, H, 79, H, 79, 81, 76, 77],
            [74, H, 74, H, 74, 77, 76, 74],
            [72, 79, 74, 76, 72, H, R, R],
            [72, H, H, H, 74, 77, 76, 74],
            [79, H, 79, H, 79, 81, 76, 77],
            [74, H, 74, H, 74, 77, 76, 74],
            [72, 84, 83, 81, 79, 77, 76, 74],
            [72, H, H, H, 74, 77, 76, 74],
            [79, H, 79, H, 79, 81, 76, 77],
            [74, H, 74, H, 74, 77, 76, 74],
            [72, 79, 74, 76, 72, H, R, R],
        ],
        chords: &[
            [C, C], [C, C], [G, G], [C, G],
            [C, C], [C, C], [G, G], [G, C],
            [C, C], [C, C], [G, G], [C, G],
            [C, C], [C, C], [G, G], [G, C],
        ],
        lead: Voice::Accordion,
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
        ("ELISE", &|| elise_song().render()),
        ("MORNING", &|| morning_song().render()),
        ("MOUNTAIN", &|| mountain_song().render()),
        ("ODE", &|| ode_song().render()),
        ("MARCH", &|| march_song().render()),
        ("CANCAN", &|| cancan_song().render()),
    ])
}
