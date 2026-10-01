//! Serpent (Snake) assets: the food sprite, and music and effects on a
//! marimba (see `cosy` and `song`).


use crate::image::Image;
use crate::cosy::{self, Voice, H as HOLD};
use crate::song::{major, minor, Chord, Groove, Song, H, R};
use crate::Asset;

const SPRITE: u32 = 24;

fn food() -> Vec<u8> {
    let mut img = Image::new(SPRITE, SPRITE);
    let (w, h) = (SPRITE as i32, SPRITE as i32);
    let cx = w / 2;
    let cy = h / 2;
    let r = w / 2 - 3;
    for y in 0..h {
        for x in 0..w {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= r * r {
                img.set(x, y, 220, 50, 50);
            }
        }
    }
    img.set(cx - 2, cy - 2, 255, 150, 150);
    img.set(cx - 1, cy - 2, 255, 200, 200);
    img.set(cx, 0, 80, 50, 20);
    img.set(cx + 1, 1, 80, 50, 20);
    img.set(cx + 2, 0, 40, 160, 40);
    img.set(cx + 3, 1, 40, 160, 40);
    img.encode_png()
}

// ---- music: two tunes on the marimba -------------------------------------
// Synthesised on the device (see `song`): A minor pentatonic, the notes a
// snake winds through. SLITHER plays the early levels, FRENZY from level 5.

const AM: Chord = minor(45);
const C: Chord = major(48);
const DM: Chord = minor(50);
const EM: Chord = minor(40);
const F: Chord = major(41);
const G: Chord = major(43);

/// The calm tune: stepwise, winding, a little sly.
pub fn slither_wav() -> Vec<u8> {
    Song {
        bpm: 108.0,
        melody: &[
            [69, H, 72, 74, 76, H, 74, 72], [69, H, H, 67, 69, H, R, R],
            [72, H, 74, 76, 79, H, 76, 74], [76, H, H, H, R, R, 74, 72],
            [69, H, 72, 74, 76, H, 79, 81], [79, H, 76, H, 74, H, 72, H],
            [74, 76, 74, 72, 69, H, 67, H], [69, H, H, H, R, R, R, R],
            [81, H, 79, 76, 79, H, 76, 74], [76, H, 74, 72, 74, H, R, R],
            [72, H, 74, 76, 74, 72, 69, H], [67, H, 69, H, 72, H, R, R],
            [81, H, 79, H, 76, H, 79, 81], [84, H, 81, H, 79, H, 76, H],
            [74, 76, 79, 76, 74, 72, 67, H], [69, H, H, H, R, R, R, R],
        ],
        chords: &[
            [AM, AM], [AM, G], [C, C], [G, G], [AM, AM], [C, G], [DM, EM], [AM, AM],
            [F, F], [C, C], [AM, AM], [G, G], [F, F], [G, G], [DM, EM], [AM, AM],
        ],
        lead: Voice::Marimba,
        lead_vol: 0.6,
        harmony: Some((Voice::Marimba, -12)),
        bass: [0, R, 7, R],
        groove: Groove::Brush,
        seed: 0x5111_7000,
        ..Song::DEFAULT
    }
    .render()
}

/// The fast tune: the same scale, running and leaping.
pub fn frenzy_wav() -> Vec<u8> {
    Song {
        bpm: 136.0,
        melody: &[
            [81, 79, 76, H, 74, 76, 79, H], [81, H, 84, H, 81, 79, 76, H],
            [74, 72, 69, H, 72, 74, 76, H], [74, H, 72, H, 69, H, R, R],
            [81, 79, 76, H, 74, 76, 79, H], [84, H, 86, H, 84, 81, 79, H],
            [76, 79, 81, 79, 76, 74, 72, 74], [69, H, H, H, R, 76, 79, 81],
            [84, H, 81, H, 79, H, 81, H], [76, H, 79, H, 74, H, R, R],
            [72, 74, 76, 79, 81, H, 79, 76], [79, H, H, H, R, R, R, R],
            [84, H, 81, H, 79, H, 81, 84], [86, H, 84, H, 81, H, 79, H],
            [76, 79, 81, 79, 76, 74, 72, 74], [69, H, H, H, R, R, R, R],
        ],
        chords: &[
            [AM, AM], [F, G], [DM, DM], [EM, AM], [AM, AM], [F, G], [C, G], [AM, AM],
            [F, F], [C, C], [AM, AM], [G, G], [F, F], [G, G], [C, EM], [AM, AM],
        ],
        lead: Voice::Marimba,
        lead_vol: 0.42,
        harmony: Some((Voice::Bell, -12)),
        bass: [0, 12, 0, 12],
        groove: Groove::FourFloor,
        seed: 0xF6E2_9000,
        ..Song::DEFAULT
    }
    .render()
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
    cosy::jingle_with(&[69, 72, 76, 81, HOLD, 76, 81, HOLD], 0.09, Voice::Marimba, 0, Some((Voice::Bell, -1)))
}

/// Game over: the phrase walking back down, slowly.
fn game_over_sfx() -> Vec<u8> {
    cosy::jingle_with(&[81, HOLD, 79, HOLD, 76, HOLD, 74, HOLD, 72, HOLD, 69, HOLD, HOLD, HOLD], 0.12, Voice::Marimba, 0, Some((Voice::Bell, -1)))
}

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/food.png", food()),
        ("sounds/eat.wav", eat_sfx()),
        ("sounds/game_over.wav", game_over_sfx()),
        ("sounds/bonus.wav", bonus_sfx()),
        ("sounds/bonus_eat.wav", bonus_eat_sfx()),
        ("sounds/level.wav", level_sfx()),
    ]
}
