// Compiled by build.rs, and by the game for its tests: each uses part of this file.
#![allow(dead_code)]

use blip::cosy::{self, Tone, Voice};
use blip::song::{major, minor, Chord, Groove, Song, H, R};
use blip::Asset;

const EM: Chord = minor(40);
const AM: Chord = minor(45);
const DM: Chord = minor(50);
const B: Chord = major(47);
const E: Chord = major(40);
const C: Chord = major(48);
const D: Chord = major(50);
const F: Chord = major(41);
const G: Chord = major(43);

/// The invasion: E minor, a march, the flat sixth (F) for the alien touch.
pub fn invasion_song() -> Song<'static> {
    Song {
        bpm: 126.0,
        melody: &[
            [76, H, H, 79, 78, H, 76, H],
            [71, H, H, H, R, R, 74, 76],
            [77, H, H, 76, 74, H, 72, H],
            [71, H, H, H, R, R, R, R],
            [76, H, H, 79, 83, H, 81, 79],
            [78, H, 76, H, 75, H, 71, H],
            [72, H, 74, H, 75, H, 78, H],
            [76, H, H, H, R, R, R, R],
            [83, H, H, 81, 79, H, 78, H],
            [76, H, H, H, 71, H, R, R],
            [81, H, H, 79, 77, H, 76, H],
            [74, H, H, H, R, R, R, R],
            [76, H, 79, H, 83, H, 88, H],
            [87, H, H, 83, 81, H, 78, H],
            [79, H, 78, H, 75, H, 71, H],
            [76, H, H, H, R, R, R, R],
        ],
        chords: &[
            [EM, EM],
            [B, B],
            [F, F],
            [B, B],
            [EM, EM],
            [B, B],
            [AM, B],
            [EM, EM],
            [G, D],
            [EM, EM],
            [F, F],
            [G, G],
            [EM, EM],
            [B, B],
            [C, B],
            [EM, EM],
        ],
        lead: Voice::Saw,
        lead_vol: 0.30,
        harmony: Some((Voice::Theremin, -12)),
        bass: [0, 0, 7, 0],
        bass_voice: Voice::Growl,
        groove: Groove::FourFloor,
        action: true,
        seed: 0xDEF0_1001,
        ..Song::DEFAULT
    }
}

/// The mothership: A minor, flat out, circling the leading note.
pub fn mothership_song() -> Song<'static> {
    Song {
        bpm: 158.0,
        melody: &[
            [69, H, 72, 69, 76, H, 75, 76],
            [77, H, 76, 74, 72, H, 71, H],
            [69, H, 72, 69, 76, H, 79, H],
            [77, H, 76, H, 75, H, R, R],
            [81, H, 80, 81, 84, H, 81, H],
            [77, H, 76, 74, 76, H, 71, H],
            [72, H, 74, H, 75, H, 76, H],
            [69, H, H, H, R, R, R, R],
        ],
        chords: &[[AM, AM], [F, E], [AM, AM], [F, E], [AM, AM], [DM, E], [F, E], [AM, AM]],
        lead: Voice::Saw,
        lead_vol: 0.30,
        harmony: Some((Voice::Saw, -12)),
        bass: [0, 0, 12, 0],
        bass_voice: Voice::Growl,
        groove: Groove::Drive,
        action: true,
        seed: 0xDEF0_2002,
        ..Song::DEFAULT
    }
}

/// The saucer's call: a theremin warbling round one note, 1.2 s.
fn ufo_siren() -> Vec<u8> {
    cosy::sfx(&cosy::glide(1.2, 560.0, 560.0, Tone::Sine, 0.25, 70.0))
}

/// The saucer charging: a theremin sliding up two octaves, trembling
/// harder as it nears the top, 1.6 s.
fn laser_charge_sfx() -> Vec<u8> {
    let rise = cosy::glide(1.6, 180.0, 720.0, Tone::Sine, 0.1, 18.0);
    let under = cosy::glide(1.6, 90.0, 360.0, Tone::Tri, 0.1, 0.0);
    cosy::sfx(&cosy::mix(rise, &under, 0.0, 0.4))
}

/// The beam: a warbling theremin falling away over a low triangle, 0.55 s.
fn laser_blast_sfx() -> Vec<u8> {
    let fall = cosy::glide(0.55, 900.0, 220.0, Tone::Sine, 1.0, 45.0);
    let low = cosy::glide(0.55, 110.0, 80.0, Tone::Tri, 0.8, 0.0);
    cosy::sfx(&cosy::mix(fall, &low, 0.0, 0.6))
}

/// One march step: a soft triangle thud falling onto `base_freq`. Four
/// descending ones cycle as the aliens advance.
fn march_thump(base_freq: f32) -> Vec<u8> {
    cosy::finish_warm(cosy::glide(0.16, base_freq * 1.4, base_freq, Tone::Tri, 1.4, 0.0), 24_000.0)
}

// Device-synthesized action tracks use saw lead, theremin and galloping bass; boss levels use the faster theme.

/// Game over: the theremin walking down A minor, slowly.
fn game_over_sfx() -> Vec<u8> {
    cosy::jingle_with(&[81, H, 77, H, 76, H, 72, H, 69, H, H, H], 0.14, Voice::Theremin, -1, Some((Voice::Bell, -2)))
}

/// Wave cleared: the theremin climbing A minor to a held note.
fn level_clear_sfx() -> Vec<u8> {
    cosy::jingle_with(&[69, 72, 76, 81, H, 79, 81, H, H], 0.1, Voice::Theremin, -1, Some((Voice::Bell, -2)))
}

/// The player's shot: a quick theremin "pew" falling an octave.
fn shoot_sfx() -> Vec<u8> {
    cosy::finish_warm(cosy::glide(0.11, 1300.0, 620.0, Tone::Sine, 1.2, 25.0), 15_000.0)
}

/// A diver leaving the formation: the theremin swooping down a ninth, with
/// a wobble, so the dive is heard as well as seen.
fn dive_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::glide(0.55, 1100.0, 480.0, Tone::Sine, 0.9, 30.0))
}

/// An alien hit: a soft pop and a bloop tumbling down, no hiss.
fn explosion_sfx() -> Vec<u8> {
    let pop = cosy::snap(0.06, 900.0, 0xDEF1);
    cosy::sfx(&cosy::mix(pop, &cosy::bloop(0.3, 360.0, 80.0, 0xDEF2), 0.02, 0.9))
}

/// The effects, as `sounds/NAME.wav`.
pub fn sounds() -> Vec<Asset> {
    vec![
        ("sounds/shoot.wav", shoot_sfx()),
        ("sounds/explosion.wav", explosion_sfx()),
        ("sounds/dive.wav", dive_sfx()),
        ("sounds/game_over.wav", game_over_sfx()),
        ("sounds/march1.wav", march_thump(98.0)),
        ("sounds/march2.wav", march_thump(87.0)),
        ("sounds/march3.wav", march_thump(78.0)),
        ("sounds/march4.wav", march_thump(70.0)),
        ("sounds/level_clear.wav", level_clear_sfx()),
        ("sounds/ufo_siren.wav", ufo_siren()),
        ("sounds/laser_charge.wav", laser_charge_sfx()),
        ("sounds/laser_blast.wav", laser_blast_sfx()),
    ]
}

/// The music: each tune as a WAV to play and an XM of its notes for TUNE.
pub fn music() -> Vec<(String, Vec<u8>)> {
    blip::xm::tunes(&[("INVASION", &|| invasion_song().render()), ("MOTHERSHIP", &|| mothership_song().render())])
}
