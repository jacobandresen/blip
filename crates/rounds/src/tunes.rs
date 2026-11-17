// Compiled by build.rs, and by the game for its tests: each uses part of this file.
#![allow(dead_code)]

// ---- music: an accordion on a morning round ---------------------------------
// A folk tune in G for the round, the same tune hurried for hard streets (market day, Sunday, every week after
// the first), and a whistled tune for the Canal Run.
use blip::cosy::{self, Tone, Voice, H as HOLD, SR};
use blip::song::chords::{AM, C, D, DM, EM, F, G};
use blip::song::{Groove, Song, H, R};
use blip::wav::Rng;
use blip::Asset;

const MORNING: &[[i32; 8]] = &[
    [67, H, H, 71, 74, H, H, H],
    [76, H, 74, H, 71, H, H, H],
    [72, H, H, 76, 79, H, 76, H],
    [74, H, H, H, R, R, R, R],
    [67, H, H, 71, 74, H, 79, H],
    [78, H, 76, H, 71, H, H, H],
    [72, H, 71, H, 69, H, 66, H],
    [67, H, H, H, R, R, R, R],
    [76, H, H, H, 79, H, 76, H],
    [74, H, H, H, 71, H, H, H],
    [72, H, H, 74, 76, H, 72, H],
    [74, H, H, H, R, R, R, R],
    [76, H, 74, H, 72, H, 76, H],
    [74, H, 71, H, 67, H, 71, H],
    [69, H, H, 72, 71, H, 69, H],
    [67, H, H, H, R, R, R, R],
];

const MORNING_CHORDS: &[[blip::song::Chord; 2]] = &[
    [G, G], [EM, EM], [C, C], [D, D], [G, G], [EM, EM], [AM, D], [G, G],
    [C, C], [G, G], [AM, AM], [D, D], [C, C], [G, EM], [AM, D], [G, G],
];

/// The round: unhurried and quiet under the play: an accordion in long notes with room between the phrases, a soft
/// marimba an octave down, a plain bass and brushes. No arpeggio running under it.
pub fn morning_song() -> Song<'static> {
    Song {
        bpm: 92.0,
        melody: MORNING,
        chords: MORNING_CHORDS,
        lead: Voice::Accordion,
        lead_vol: 0.2,
        harmony: Some((Voice::Marimba, -12)),
        bass: [0, R, 7, R],
        arp: false,
        groove: Groove::Brush,
        fills: false,
        echo: 0.16,
        seed: 0x0D0B_A6E1,
        ..Song::DEFAULT
    }
}

/// The same round a little quicker, for the hard streets: still brushes, a walking bass.
pub fn rush_song() -> Song<'static> {
    Song { bpm: 106.0, bass: [0, 7, 12, 7], seed: 0x0D0B_F457, ..morning_song() }
}

/// Nyhavn: soft bells over the water, slow.
pub fn canal_song() -> Song<'static> {
    Song {
        bpm: 84.0,
        melody: &[
            [72, H, H, 76, 79, H, H, H],
            [77, H, 76, H, 74, H, H, H],
            [76, H, H, 79, 84, H, 83, H],
            [81, H, H, H, 79, H, H, H],
            [72, H, 74, 76, 77, H, 76, 74],
            [76, H, H, H, 72, H, H, H],
            [74, H, 76, 77, 79, H, 77, 76],
            [72, H, H, H, R, R, R, R],
        ],
        chords: &[[C, C], [F, G], [AM, AM], [F, G], [F, F], [C, AM], [DM, G], [C, C]],
        lead: Voice::Bell,
        lead_vol: 0.24,
        harmony: None,
        bass: [0, R, R, 7],
        arp: false,
        groove: Groove::None,
        echo: 0.22,
        seed: 0xCA7A_1000,
        ..Song::DEFAULT
    }
}

// ---- effects ------------------------------------------------------------------
// Soft and wooden: the letterbox flap, the paper's thump, the bike bell, a friendly woof.

/// `secs` of noise through a one-pole low-pass at `lp` (0..1, lower is darker), shaped by `env(k)`.
fn noise(secs: f32, lp: f32, seed: u32, env: impl Fn(f32) -> f32) -> Vec<f32> {
    let mut rng = Rng(seed | 1);
    let n = (secs * SR) as usize;
    let mut y = 0.0f32;
    (0..n).map(|i| {
        y += ((rng.next_f32() * 2.0 - 1.0) - y) * lp;
        y * env(i as f32 / n as f32)
    }).collect()
}

/// A throw: a quick breath of air.
fn throw_sfx() -> Vec<u8> {
    let air = noise(0.14, 0.35, 11, |k| (k * 6.0).min(1.0) * (1.0 - k).powf(2.0) * 0.8);
    cosy::sfx(&cosy::mix(air, &cosy::glide(0.12, 520.0, 300.0, Tone::Sine, 2.0, 0.0), 0.0, 0.18))
}

/// Through the letterbox: the paper's rustle, the flap's double clack and a little brass ring.
fn flap_sfx() -> Vec<u8> {
    let rustle = noise(0.06, 0.5, 21, |k| (1.0 - k) * 0.5);
    let a = cosy::mix(rustle, &cosy::snap(0.05, 1300.0, 22), 0.03, 0.8);
    let b = cosy::mix(a, &cosy::snap(0.05, 900.0, 23), 0.11, 0.6);
    cosy::sfx(&cosy::mix(b, &cosy::ding(1760.0, 0.12), 0.12, 0.25))
}

/// On the doorstep: a soft, papery thump.
fn thump_sfx() -> Vec<u8> {
    let body = cosy::glide(0.13, 190.0, 85.0, Tone::Sine, 2.0, 0.0);
    cosy::sfx(&cosy::mix(body, &noise(0.08, 0.25, 31, |k| (1.0 - k).powf(3.0) * 0.6), 0.0, 1.0))
}

/// Into the flower bed.
fn plop_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::mix(cosy::glide(0.09, 320.0, 210.0, Tone::Tri, 1.5, 0.0), &noise(0.07, 0.4, 41, |k| (1.0 - k) * 0.3), 0.0, 1.0))
}

/// A sample in its box: a bright chirp and a sparkle.
fn sample_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::mix(cosy::chirp(0.12, 880.0, 1320.0), &cosy::sparkle(0.25, 84, 3, 4), 0.08, 0.5))
}

/// Water: a puddle, the canal.
fn splash_sfx() -> Vec<u8> {
    let water = noise(0.38, 0.3, 51, |k| (k * 20.0).min(1.0) * (1.0 - k).powf(1.6) * 0.9);
    cosy::sfx(&cosy::mix(water, &cosy::bloop(0.16, 260.0, 520.0, 52), 0.02, 0.4))
}

/// A tumble: a thud, the bell's jangle and papers everywhere.
fn crash_sfx() -> Vec<u8> {
    let mut buf = cosy::glide(0.25, 140.0, 55.0, Tone::Sine, 1.5, 0.0);
    buf = cosy::mix(buf, &noise(0.3, 0.3, 61, |k| (1.0 - k).powf(2.0) * 0.7), 0.0, 1.0);
    for (i, f) in [2093.0, 2217.0, 1976.0].iter().enumerate() {
        buf = cosy::mix(buf, &cosy::ding(*f, 0.08), 0.05 + i as f32 * 0.06, 0.22);
    }
    cosy::sfx(&cosy::mix(buf, &noise(0.35, 0.6, 62, |k| (1.0 - k) * 0.25), 0.12, 1.0))
}

/// Ten fresh papers from the newsstand cart.
fn refill_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::mix(cosy::sparkle(0.35, 79, 5, 4), &cosy::ding(1568.0, 0.2), 0.3, 0.4))
}

/// A heart lost: the accordion sighs down a third.
fn heart_lost_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[69, 65], 0.13, Voice::Accordion, 0.5))
}

/// A subscriber gives up the paper.
fn cancelled_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[72, 68, 65, 60], 0.12, Voice::Accordion, 0.45))
}

/// The bike bell: ring-ring.
fn bell_sfx() -> Vec<u8> {
    let ring = |f: f32| cosy::mix(cosy::ding(f, 0.18), &cosy::ding(f * 1.5, 0.1), 0.0, 0.4);
    cosy::sfx(&cosy::mix(ring(2093.0), &ring(2093.0), 0.15, 1.0))
}

/// A car's reverse beeper: three soft beeps, its tell.
fn reverse_sfx() -> Vec<u8> {
    let beep = cosy::glide(0.11, 1046.0, 1046.0, Tone::Pulse(0.5), 0.3, 0.0);
    let mut buf = Vec::new();
    for i in 0..3 { buf = cosy::mix(buf, &beep, i as f32 * 0.24, 0.35); }
    cosy::sfx(&buf)
}

/// A garden gate rattling on its latch, the dog's tell.
fn rattle_sfx() -> Vec<u8> {
    let mut buf = Vec::new();
    for i in 0..6 { buf = cosy::mix(buf, &cosy::snap(0.04, 1900.0 + i as f32 * 140.0, 70 + i), i as f32 * 0.075, 0.45); }
    cosy::sfx(&buf)
}

/// A friendly woof, twice.
fn bark_sfx() -> Vec<u8> {
    let woof = |f: f32| cosy::glide(0.13, f, f * 0.62, Tone::Pulse(0.3), 1.2, 0.0);
    cosy::sfx(&cosy::mix(woof(430.0), &woof(470.0), 0.19, 0.8))
}

/// A sprinkler waking: a rising hiss.
fn hiss_sfx() -> Vec<u8> {
    let mut rng = Rng(81);
    let n = (0.7 * SR) as usize;
    let mut prev = 0.0f32;
    let buf: Vec<f32> = (0..n).map(|i| {
        let w = rng.next_f32() * 2.0 - 1.0;
        let hp = w - prev;
        prev = w;
        let k = i as f32 / n as f32;
        hp * k.min(1.0) * (1.0 - k).powf(0.3) * 0.35
    }).collect();
    cosy::sfx(&buf)
}

/// Pigeons taking off: wing claps and a coo.
fn coo_sfx() -> Vec<u8> {
    let wings = noise(0.45, 0.4, 91, |k| ((k * 90.0).sin().abs()) * (1.0 - k) * 0.6);
    cosy::sfx(&cosy::mix(wings, &cosy::glide(0.28, 520.0, 400.0, Tone::Sine, 0.8, 18.0), 0.1, 0.35))
}

/// Soaked by a sprinkler.
fn soggy_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::mix(cosy::bloop(0.25, 320.0, 170.0, 101), &noise(0.3, 0.5, 102, |k| (1.0 - k) * 0.25), 0.0, 1.0))
}

/// Church bells over the rooftops: a big bell tolling, a smaller one between.
fn church_sfx() -> Vec<u8> {
    let mut buf = Vec::new();
    for i in 0..4 {
        let big = cosy::mix(cosy::ding(196.0, 1.2), &cosy::ding(392.0 * 1.19, 0.6), 0.0, 0.4);
        buf = cosy::mix(buf, &big, i as f32 * 0.9, 0.7);
        buf = cosy::mix(buf, &cosy::ding(294.0, 0.7), i as f32 * 0.9 + 0.45, 0.35);
    }
    cosy::sfx(&buf)
}

/// Another cyclist's bell, passing close: a quick single ring.
fn ting_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::mix(cosy::ding(2349.0, 0.12), &cosy::ding(3520.0, 0.06), 0.0, 0.3))
}

/// A Canal Run target: two bells.
fn target_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::mix(cosy::ding(1568.0, 0.25), &cosy::ding(2093.0, 0.3), 0.07, 0.8))
}

/// Nothing left in the box.
fn empty_sfx() -> Vec<u8> {
    cosy::sfx(&cosy::chirp(0.14, 300.0, 220.0))
}

fn accordion(notes: &[i32], step: f32, double: Voice) -> Vec<u8> {
    cosy::jingle_with(notes, step, Voice::Accordion, 0, Some((double, -1)))
}

/// The tunes: the day starting, the day done, a perfect day, a second rider, the end.
fn day_start_sfx() -> Vec<u8> { accordion(&[67, 71, 74, 79, HOLD, 74, 79, HOLD], 0.1, Voice::Bell) }
fn day_end_sfx() -> Vec<u8> { accordion(&[79, 78, 76, 74, 76, 79, HOLD, HOLD], 0.11, Voice::Bell) }
fn perfect_sfx() -> Vec<u8> { accordion(&[74, 79, 83, 86, HOLD, 83, 86, 91, HOLD, HOLD], 0.09, Voice::Glock) }
fn join_sfx() -> Vec<u8> { accordion(&[67, 74, 79, HOLD], 0.08, Voice::Bell) }
fn game_over_sfx() -> Vec<u8> {
    accordion(&[79, HOLD, 76, HOLD, 74, HOLD, 71, HOLD, 72, HOLD, 67, HOLD, HOLD, HOLD], 0.13, Voice::Bell)
}

/// The effects, as `sounds/NAME.wav`.
pub fn sounds() -> Vec<Asset> {
    vec![
        ("sounds/throw.wav", throw_sfx()),
        ("sounds/flap.wav", flap_sfx()),
        ("sounds/thump.wav", thump_sfx()),
        ("sounds/plop.wav", plop_sfx()),
        ("sounds/sample.wav", sample_sfx()),
        ("sounds/splash.wav", splash_sfx()),
        ("sounds/crash.wav", crash_sfx()),
        ("sounds/refill.wav", refill_sfx()),
        ("sounds/heart_lost.wav", heart_lost_sfx()),
        ("sounds/cancelled.wav", cancelled_sfx()),
        ("sounds/bell.wav", bell_sfx()),
        ("sounds/reverse.wav", reverse_sfx()),
        ("sounds/rattle.wav", rattle_sfx()),
        ("sounds/bark.wav", bark_sfx()),
        ("sounds/hiss.wav", hiss_sfx()),
        ("sounds/coo.wav", coo_sfx()),
        ("sounds/soggy.wav", soggy_sfx()),
        ("sounds/target.wav", target_sfx()),
        ("sounds/church.wav", church_sfx()),
        ("sounds/ting.wav", ting_sfx()),
        ("sounds/empty.wav", empty_sfx()),
        ("sounds/day_start.wav", day_start_sfx()),
        ("sounds/day_end.wav", day_end_sfx()),
        ("sounds/perfect.wav", perfect_sfx()),
        ("sounds/join.wav", join_sfx()),
        ("sounds/game_over.wav", game_over_sfx()),
    ]
}

/// The music: each tune as a WAV to play.
pub fn music() -> Vec<(String, Vec<u8>)> {
    blip::wav::tunes(&[
        ("MORNING", &|| morning_song().render()),
        ("RUSH", &|| rush_song().render()),
        ("CANAL", &|| canal_song().render()),
    ])
}
