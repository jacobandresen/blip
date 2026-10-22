// Compiled by build.rs, and by the game for its tests: each uses part of this file.
#![allow(dead_code)]

use blip::song::{major, minor, Chord, Groove, Song, H, R};

// ---- music: two tunes on the glockenspiel -------------------------------
// Synthesised on the device (see `song`). Octave leaps for the ball's
// bounce; the game alternates the two by level.
use blip::cosy::{self, Tone, Voice};
use blip::Asset;

const C: Chord = major(48);
const D: Chord = major(50);
const F: Chord = major(41);
const G: Chord = major(43);
const AM: Chord = minor(45);
const EM: Chord = minor(40);

/// Odd levels: C major, bouncing up the octave and skipping back down.
pub fn bounce_song() -> Song<'static> {
    Song {
        bpm: 124.0,
        melody: &[
            [72, 84, 79, H, 76, H, 79, H],
            [77, H, 76, H, 74, H, R, R],
            [71, 83, 79, H, 74, H, 77, H],
            [76, H, 74, H, 72, H, R, R],
            [72, 84, 79, H, 76, H, 79, 81],
            [84, H, 81, H, 77, H, 81, H],
            [79, H, 77, 76, 74, H, 71, H],
            [72, H, H, H, R, R, R, R],
            [81, H, 79, H, 77, H, 76, H],
            [74, H, 76, 77, 79, H, R, R],
            [79, H, 77, H, 76, H, 74, H],
            [72, H, 74, 76, 77, H, R, R],
            [76, 88, 84, H, 79, H, 84, H],
            [86, H, 84, H, 83, H, 79, H],
            [77, H, 76, H, 74, H, 79, H],
            [72, H, H, H, R, R, R, R],
        ],
        chords: &[
            [C, C],
            [F, G],
            [G, G],
            [C, C],
            [C, C],
            [F, F],
            [G, G],
            [C, C],
            [F, F],
            [G, G],
            [C, G],
            [AM, F],
            [C, C],
            [G, G],
            [F, G],
            [C, C],
        ],
        lead: Voice::Glock,
        lead_vol: 0.36,
        harmony: Some((Voice::Pluck, -12)),
        seed: 0xB0DE_1234,
        ..Song::DEFAULT
    }
}

pub fn bounce_wav() -> Vec<u8> {
    bounce_song().render()
}

/// Even levels: G major, quicker, climbing in thirds.
pub fn rebound_song() -> Song<'static> {
    Song {
        bpm: 138.0,
        melody: &[
            [79, H, 83, 86, H, 83, 79, H],
            [81, H, 84, H, 83, H, 81, H],
            [79, H, 83, 86, H, 88, 86, H],
            [84, H, 83, H, 81, H, R, R],
            [76, H, 79, 83, H, 79, 76, H],
            [78, H, 81, H, 84, H, 83, 81],
            [79, H, 78, H, 76, H, 74, H],
            [79, H, H, H, R, R, R, R],
            [86, H, 84, 83, 81, H, 79, H],
            [84, H, 83, 81, 79, H, 78, H],
            [79, 81, 83, 84, 86, H, 83, H],
            [81, H, H, H, R, R, R, R],
            [86, H, 91, H, 88, H, 86, H],
            [84, H, 88, H, 86, H, 84, H],
            [83, H, 81, H, 78, H, 74, H],
            [79, H, H, H, R, R, R, R],
        ],
        chords: &[
            [G, G],
            [D, D],
            [G, G],
            [C, D],
            [EM, EM],
            [D, D],
            [C, D],
            [G, G],
            [G, D],
            [C, D],
            [G, G],
            [D, D],
            [G, C],
            [C, C],
            [D, D],
            [G, G],
        ],
        lead: Voice::Glock,
        lead_vol: 0.34,
        harmony: Some((Voice::Pluck, -12)),
        groove: Groove::FourFloor,
        seed: 0xB0DE_5678,
        ..Song::DEFAULT
    }
}

pub fn rebound_wav() -> Vec<u8> {
    rebound_song().render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_tracks_render_the_exposed_song_data() {
        assert_eq!(bounce_wav(), bounce_song().render());
        assert_eq!(rebound_wav(), rebound_song().render());
    }
}

/// Bat cap width in source pixels; the game stretches only the slice
/// between the caps, so the ends keep their shape at any bat width.
// ---- effects --------------------------------------------------------------
// Pitch variants (`v`) keep repeated hits from sounding identical.
fn paddle_hit(v: usize) -> Vec<u8> {
    cosy::sfx(&cosy::boing(0.16, [196.0, 208.0, 220.0][v]))
}

/// A brick taking a hit without breaking: a muted glockenspiel tap.
fn brick_hit(v: usize) -> Vec<u8> {
    let mut tap = cosy::run(&[79 + 2 * v as i32], 0.05, Voice::Glock, 0.45);
    tap.truncate((0.25 * cosy::SR) as usize);
    cosy::sfx(&tap)
}

/// A brick breaking: a bright glockenspiel note on a soft pop.
fn brick_break(v: usize) -> Vec<u8> {
    let pop = cosy::snap(0.08, 1400.0 + 120.0 * v as f32, 0xB0 + v as u32);
    let bell = cosy::run(&[[84, 88, 91][v]], 0.05, Voice::Glock, 0.6);
    cosy::sfx(&cosy::mix(pop, &bell, 0.01, 1.0))
}

fn wall_hit() -> Vec<u8> {
    cosy::finish_warm(cosy::boing(0.1, 150.0), 11_000.0)
}

/// A ball lost: a rubbery sigh falling away.
fn life_lost() -> Vec<u8> {
    cosy::sfx(&cosy::glide(1.0, 520.0, 170.0, Tone::Sine, 0.8, 20.0))
}

fn win() -> Vec<u8> {
    cosy::jingle_with(
        &[72, 76, 79, 84, H, 79, 84, 88, H, H],
        0.09,
        Voice::Glock,
        0,
        Some((Voice::Bell, -1)),
    )
}

/// Pickups caught: a good one sparkles up the glockenspiel, the narrow bat
/// tumbles down it, and a life is a little fanfare.
fn pickup_good() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[84, 88, 91, 96], 0.045, Voice::Glock, 0.5))
}

fn pickup_bad() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[88, 84, 79, 72], 0.06, Voice::Glock, 0.45))
}

fn pickup_life() -> Vec<u8> {
    cosy::jingle_with(
        &[79, 84, 88, 91, H, 96, H, H],
        0.07,
        Voice::Glock,
        0,
        Some((Voice::Bell, -1)),
    )
}

/// A bomb brick going off: a soft pop over a low boom tumbling down. One
/// sound for the whole blast, not nine bricks at once.
fn bomb_sfx() -> Vec<u8> {
    let pop = cosy::snap(0.07, 700.0, 0xB0B1);
    cosy::sfx(&cosy::mix(
        pop,
        &cosy::bloop(0.42, 220.0, 55.0, 0xB0B2),
        0.01,
        1.0,
    ))
}

/// The effects, as `sounds/NAME.wav`.
pub fn sounds() -> Vec<Asset> {
    vec![
        ("sounds/paddle_hit_0.wav", paddle_hit(0)),
        ("sounds/paddle_hit_1.wav", paddle_hit(1)),
        ("sounds/paddle_hit_2.wav", paddle_hit(2)),
        ("sounds/brick_hit_0.wav", brick_hit(0)),
        ("sounds/brick_hit_1.wav", brick_hit(1)),
        ("sounds/brick_hit_2.wav", brick_hit(2)),
        ("sounds/brick_break_0.wav", brick_break(0)),
        ("sounds/brick_break_1.wav", brick_break(1)),
        ("sounds/brick_break_2.wav", brick_break(2)),
        ("sounds/wall_hit.wav", wall_hit()),
        ("sounds/bomb.wav", bomb_sfx()),
        ("sounds/life_lost.wav", life_lost()),
        ("sounds/win.wav", win()),
        ("sounds/pickup_good.wav", pickup_good()),
        ("sounds/pickup_bad.wav", pickup_bad()),
        ("sounds/pickup_life.wav", pickup_life()),
    ]
}

/// The music: each tune as a WAV to play and an XM of its notes for TUNE.
pub fn music() -> Vec<(String, Vec<u8>)> {
    blip::xm::tunes(&[
        ("BOUNCE", &bounce_wav),
        ("REBOUND", &rebound_wav),
    ])
}
