// Compiled by build.rs, and by the game for its tests: each uses part of this file.
#![allow(dead_code)]

use blip::cosy::{self, Voice, H as HOLD};
use blip::song::{major, minor, Chord, Groove, Song, H, R};
use blip::Asset;

// A spooky dungeon feel: D minor, bell lead, a low growl bass.
const DM: Chord = minor(50);
const GM: Chord = minor(55);
const AM: Chord = minor(57);
const FM: Chord = major(53);
const CM: Chord = major(48);
const BB: Chord = major(58);

/// The calm dungeon wander: stepwise, watchful, in D minor.
pub fn dungeon_song() -> Song<'static> {
    Song {
        bpm: 92.0,
        melody: &[
            [62, H, 65, H, 62, H, H, R],
            [60, H, 63, H, 60, H, H, R],
            [65, H, 69, H, 67, 65, 63, 62],
            [62, H, H, H, R, R, R, R],
            [62, H, 65, H, 69, H, 67, 65],
            [63, H, 67, H, 70, H, 67, 63],
            [65, H, 62, H, 60, H, 59, 57],
            [57, H, H, H, R, R, R, R],
            [69, H, 67, H, 65, H, 63, 62],
            [60, H, 62, H, 63, H, 60, 57],
            [62, H, 65, H, 62, H, H, R],
            [62, H, H, H, R, R, R, R],
        ],
        chords: &[
            [DM, DM],
            [GM, GM],
            [AM, DM],
            [DM, DM],
            [DM, DM],
            [GM, GM],
            [AM, GM],
            [DM, DM],
            [FM, FM],
            [GM, GM],
            [AM, DM],
            [DM, DM],
        ],
        lead: Voice::Bell,
        lead_vol: 0.5,
        harmony: Some((Voice::Glock, -12)),
        bass: [0, R, 7, R],
        groove: Groove::Brush,
        echo: 0.35,
        seed: 0x1a11_0c00,
        ..Song::DEFAULT
    }
}

/// The Warlock's chamber: tense, driving, in the same key.
pub fn tower_song() -> Song<'static> {
    Song {
        bpm: 126.0,
        melody: &[
            [62, 65, 69, 65, 62, H, 65, 69],
            [70, 69, 65, 69, 67, 65, 63, 62],
            [62, 65, 69, 72, 74, 72, 69, 65],
            [62, H, H, H, R, 65, 69, 72],
            [74, H, 72, H, 69, H, 65, H],
            [63, 67, 70, 67, 63, H, 67, 70],
            [71, 70, 67, 70, 69, 67, 65, 63],
            [62, H, H, H, R, R, R, R],
        ],
        chords: &[
            [DM, DM],
            [GM, DM],
            [BB, CM],
            [DM, DM],
            [AM, AM],
            [GM, GM],
            [AM, GM],
            [DM, DM],
        ],
        lead: Voice::Bell,
        lead_vol: 0.5,
        harmony: Some((Voice::Theremin, 0)),
        bass: [0, 12, 0, 12],
        groove: Groove::FourFloor,
        echo: 0.25,
        action: true,
        seed: 0x70_1e_2000,
        ..Song::DEFAULT
    }
}

// ---- effects --------------------------------------------------------------

/// A cast bolt: a quick bright pluck up a fifth.
fn bolt_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[72, 79], 0.04, Voice::Pluck, 0.65))
}

/// Reveal: a shimmering sweep, a rising glass run.
fn reveal_sfx() -> Vec<u8> {
    let run = cosy::run(&[72, 74, 77, 81, 84], 0.05, Voice::Glass, 0.55);
    cosy::sfx(&cosy::mix(run, &cosy::sparkle(0.3, 96, 4, 3), 0.08, 0.3))
}

/// A cleansed stone: a warm rising chord.
fn cleanse_sfx() -> Vec<u8> {
    cosy::jingle_with(&[62, 66, 69, 72, HOLD, 72, HOLD], 0.08, Voice::Bell, 0, Some((Voice::Glock, 1)))
}

/// An enemy defeated: a soft pop and a short fall.
fn kill_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[77, 60], 0.06, Voice::Growl, 0.5))
}

/// A lost life: a long, low descent.
fn death_sfx() -> Vec<u8> {
    cosy::jingle_with(&[69, 65, 62, 57, HOLD, HOLD, 55, HOLD], 0.14, Voice::Bell, 0, Some((Voice::Growl, -1)))
}

/// Game over: slow broken chord down the scale.
fn over_sfx() -> Vec<u8> {
    cosy::jingle_with(&[74, HOLD, 71, HOLD, 67, HOLD, 63, HOLD, 59, HOLD, 55, HOLD, HOLD, HOLD], 0.16, Voice::Bell, 0, None)
}

/// A chamber cleared: a bright ascending jingle.
fn chamber_sfx() -> Vec<u8> {
    cosy::jingle_with(&[62, 65, 69, 74, HOLD, 77, HOLD, 74, HOLD], 0.09, Voice::Bell, 0, Some((Voice::Glock, 1)))
}

/// A broken ward crystal: a sharp cracked-glass note.
fn ward_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::mix(
        cosy::run(&[84, 88], 0.03, Voice::Glass, 0.6),
        &cosy::sparkle(0.2, 110, 5, 2),
        0.02,
        0.4,
    ))
}

/// The Warlock's taunt: a low, guttural two-note growl.
fn warlock_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[55, 52, 55], 0.10, Voice::Growl, 0.6))
}

/// The effects, as `sounds/NAME.wav`.
pub fn sounds() -> Vec<Asset> {
    vec![
        ("sounds/bolt.wav", bolt_sfx()),
        ("sounds/reveal.wav", reveal_sfx()),
        ("sounds/cleanse.wav", cleanse_sfx()),
        ("sounds/kill.wav", kill_sfx()),
        ("sounds/death.wav", death_sfx()),
        ("sounds/over.wav", over_sfx()),
        ("sounds/chamber.wav", chamber_sfx()),
        ("sounds/ward.wav", ward_sfx()),
        ("sounds/warlock.wav", warlock_sfx()),
    ]
}

/// The music: each tune as a WAV to play and an XM of its notes for TUNE.
pub fn music() -> Vec<(String, Vec<u8>)> {
    blip::xm::tunes(&[
        ("DUNGEON", &|| dungeon_song().render()),
        ("TOWER", &|| tower_song().render()),
    ])
}