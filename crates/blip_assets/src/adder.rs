//! Adder assets: the egg, effects as a dry rustle, and music on a pluck — the
//! voice no other cabinet tune leads on (see `cosy` and `song`).

use crate::cosy::{self, Tone, Voice, H as HOLD};
use crate::image::Image;
use crate::song::{major, minor, Chord, Groove, Song, H, R};
use crate::Asset;

const SPRITE: u32 = 32;

/// A cream egg: shell shading toward the rim, a highlight and a few speckles.
fn egg() -> Vec<u8> {
    let mut img = Image::new(SPRITE, SPRITE);
    let (w, h) = (SPRITE as i32, SPRITE as i32);
    let (cx, cy) = (w / 2, h / 2);
    let (rx, ry) = (w / 2 - 4, h / 2 - 3);
    for y in 0..h {
        for x in 0..w {
            let dx = (x - cx) as f32 / rx as f32;
            let mut dy = (y - cy) as f32 / ry as f32;
            if dy < 0.0 { dy *= 1.22; } // the narrow end
            let d = dx * dx + dy * dy;
            if d > 1.0 { continue; }
            if d > 0.80 {
                img.set(x, y, 168, 150, 110); // the rim
            } else {
                let lit = ((1.0 - d) * 26.0) as u8;
                img.set(x, y, 236 + lit / 4, 226 + lit / 4, 186);
            }
        }
    }
    img.set(cx - 3, cy - 5, 255, 250, 226);
    img.set(cx - 2, cy - 5, 255, 250, 226);
    img.set(cx - 3, cy - 4, 255, 250, 226);
    for (x, y) in [(cx - 5, cy + 2), (cx + 2, cy - 4), (cx + 5, cy + 3), (cx - 1, cy + 7), (cx + 4, cy - 7)] {
        img.set(x, y, 198, 178, 130);
    }
    img.encode_png()
}

// ---- music: two tunes on the pluck ---------------------------------------
// Synthesised on the device (see `song`): A minor pentatonic, the same notes
// Serpent winds through on a marimba, struck dry and earthy instead. COIL
// plays the early pits, HUNT from pit 4.

const AM: Chord = minor(45);
const C: Chord = major(48);
const DM: Chord = minor(50);
const EM: Chord = minor(40);
const F: Chord = major(41);
const G: Chord = major(43);

/// The slow tune: a coiled figure, hypnotic, the way a charmer's line hangs.
pub fn coil_wav() -> Vec<u8> {
    Song {
        bpm: 92.0,
        melody: &[
            [69, H, 72, H, 74, H, 72, H], [69, H, H, H, 67, H, R, R],
            [72, H, 74, H, 76, H, 74, H], [72, H, H, H, R, R, 69, H],
            [76, H, 74, H, 72, H, 74, H], [76, H, H, H, 79, H, R, R],
            [76, H, 74, H, 72, H, 69, H], [67, H, H, H, R, R, R, R],
            [81, H, 79, H, 76, H, 79, H], [81, H, H, H, 84, H, R, R],
            [84, H, 81, H, 79, H, 76, H], [79, H, H, H, R, R, 76, H],
            [74, H, 76, H, 79, H, 81, H], [79, H, H, H, 76, H, R, R],
            [74, H, 72, H, 69, H, 72, H], [69, H, H, H, R, R, R, R],
        ],
        chords: &[
            [AM, AM], [AM, G], [C, C], [G, G], [AM, AM], [F, F], [DM, EM], [AM, AM],
            [F, F], [C, C], [AM, AM], [EM, EM], [F, F], [G, G], [DM, EM], [AM, AM],
        ],
        lead: Voice::Pluck,
        lead_vol: 0.5,
        harmony: Some((Voice::Bell, -12)),
        bass: [0, R, 7, R],
        groove: Groove::Brush,
        seed: 0xADD0_0001,
        ..Song::DEFAULT
    }
    .render()
}

/// The fast tune, from pit 4: the same five notes run in eighths.
pub fn hunt_wav() -> Vec<u8> {
    Song {
        bpm: 138.0,
        melody: &[
            [69, 72, 74, 72, 69, 72, 74, 76], [79, H, 74, H, 72, H, 74, H],
            [76, 74, 72, 74, 76, 79, 81, H], [79, H, H, H, R, 76, 74, 72],
            [69, 72, 74, 76, 79, 76, 74, 72], [69, H, 67, H, 69, H, 72, H],
            [74, 76, 79, 81, 84, H, 81, H], [79, H, H, H, R, R, R, R],
            [81, 79, 76, 79, 81, 84, 81, H], [79, H, 76, H, 74, H, 72, H],
            [74, 76, 74, 72, 69, 72, 74, H], [76, H, H, H, R, 79, 81, H],
            [84, H, 81, H, 79, H, 81, H], [76, H, 74, H, 72, H, R, R],
            [74, 72, 69, 72, 74, 76, 79, H], [69, H, H, H, R, R, R, R],
        ],
        chords: &[
            [AM, AM], [AM, G], [C, C], [G, G], [AM, AM], [F, F], [DM, EM], [AM, AM],
            [F, F], [C, C], [AM, AM], [EM, EM], [F, F], [G, G], [DM, EM], [AM, AM],
        ],
        lead: Voice::Pluck,
        lead_vol: 0.36,
        harmony: Some((Voice::Bell, -12)),
        bass: [0, 12, 0, 12],
        groove: Groove::FourFloor,
        seed: 0xADD0_0002,
        ..Song::DEFAULT
    }
    .render()
}

// ---- effects: dry plucks and rustles -------------------------------------

/// Eating: two taps on the pluck, a fourth apart, low in the instrument.
fn eat_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[60, 67], 0.05, Voice::Pluck, 0.7))
}

/// The shed: a rustle that slides down out from under the tail.
fn shed_sfx() -> Vec<u8> {
    let rustle: Vec<f32> = cosy::glide(0.26, 1900.0, 620.0, Tone::Pulse(0.22), 1.7, 45.0)
        .iter().map(|v| v * 0.45).collect();
    cosy::sfx(&cosy::mix(rustle, &cosy::run(&[41, 45], 0.06, Voice::Pluck, 0.4), 0.0, 1.0))
}

/// The throttle going down: a short whip, played once per strike, not per step.
fn strike_sfx() -> Vec<u8> {
    let whip: Vec<f32> = cosy::glide(0.16, 1100.0, 260.0, Tone::Pulse(0.125), 1.1, 0.0)
        .iter().map(|v| v * 0.5).collect();
    cosy::sfx(&cosy::mix(whip, &cosy::run(&[72], 0.05, Voice::Pluck, 0.35), 0.0, 1.0))
}

/// A new pit: four plucked notes climbing, a bell under them.
fn pit_sfx() -> Vec<u8> {
    cosy::jingle_with(&[62, 66, 69, 74, HOLD, HOLD], 0.07, Voice::Pluck, 0, Some((Voice::Bell, -1)))
}

/// Game over: the phrase sinking, one long note at a time.
fn game_over_sfx() -> Vec<u8> {
    cosy::jingle_with(&[69, HOLD, 65, HOLD, 62, HOLD, 57, HOLD, HOLD, HOLD], 0.13, Voice::Pluck, 0, Some((Voice::Bell, -2)))
}

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/egg.png", egg()),
        ("sounds/eat.wav", eat_sfx()),
        ("sounds/shed.wav", shed_sfx()),
        ("sounds/strike.wav", strike_sfx()),
        ("sounds/pit.wav", pit_sfx()),
        ("sounds/game_over.wav", game_over_sfx()),
    ]
}
