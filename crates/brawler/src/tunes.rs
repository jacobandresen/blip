// Compiled by build.rs, and by the game for its tests: each uses part of this file.
#![allow(dead_code)]

use blip::wav::{
    encode_pcm16_half, encode_pcm16_mono, encode_pcm16_music, env, fade_out, fold_tail, limit_pcm16, low_pass,
    ms_to_samples, soft_limit_to_pcm16, tame, warm, Rng, MIX_KNEE, SAMPLE_RATE,
};
use blip::Asset;
use blip::cosy::SR;


/// Plucked string using Karplus–Strong averaging; upper harmonics decay first.
pub(super) fn pluck(buf: &mut [f32], off: usize, freq: f32, ms: f32, gain: f32, rng: &mut Rng) {
    let freq = tame(freq);
    let period = (SAMPLE_RATE as f32 / freq).max(2.0) as usize;
    let n = ms_to_samples(ms).min(buf.len().saturating_sub(off));
    let mut line: Vec<f32> = (0..period).map(|_| rng.next_f32() * 2.0 - 1.0).collect();
    // A little brightness taken off the initial burst: a koto is
    // plucked with a plectrum, not struck with a hammer.
    for i in 1..period {
        line[i] = line[i] * 0.6 + line[i - 1] * 0.4;
    }
    // A long (low) string has few trips round the line to lose its hiss in,
    // so it starts rounder: a bass note should be a note, not a thwack.
    for _ in 0..period / 150 {
        let first = line[0];
        for i in 0..period - 1 {
            line[i] = (line[i] + line[i + 1]) * 0.5;
        }
        line[period - 1] = (line[period - 1] + first) * 0.5;
    }
    for i in 0..n {
        let idx = i % period;
        let nxt = (i + 1) % period;
        let out = line[idx];
        line[idx] = (line[idx] + line[nxt]) * 0.5 * 0.996;
        // A slow overall fade so notes do not pile up into a drone.
        let e = (-(i as f32) / n as f32 * 3.2).exp();
        buf[off + i] += out * e * gain * 7600.0;
    }
}

/// A taiko: a low body tone falling in pitch as the head relaxes, a noise
/// crack for the stick, and a room-length tail.
pub(super) fn taiko(buf: &mut [f32], off: usize, freq: f32, gain: f32, rng: &mut Rng) {
    let n = ms_to_samples(460.0).min(buf.len().saturating_sub(off));
    let mut phase = 0.0f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let e = (-t * 5.5).exp();
        let f = freq * (1.0 - 0.42 * t);
        phase += 2.0 * std::f32::consts::PI * f / SAMPLE_RATE as f32;
        let skin = (rng.next_f32() * 2.0 - 1.0) * (-t * 46.0).exp() * 0.55;
        buf[off + i] += (phase.sin() * 0.95 + skin) * e * gain * 5400.0;
    }
}

/// The rim — a dry wooden click, the sound of the stick on the hoop.
pub(super) fn rim(buf: &mut [f32], off: usize, gain: f32, rng: &mut Rng) {
    let n = ms_to_samples(45.0).min(buf.len().saturating_sub(off));
    let mut prev = 0.0f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let x = rng.next_f32() * 2.0 - 1.0;
        // A one-sample difference: all click, no body.
        let hp = x - prev;
        prev = x;
        buf[off + i] += hp * (-t * 14.0).exp() * gain * 3400.0;
    }
}

/// A short, metallic hat: filtered noise with inharmonic cymbal partials.
fn hi_hat(buf: &mut [f32], off: usize, gain: f32, rng: &mut Rng) {
    let n = ms_to_samples(38.0).min(buf.len().saturating_sub(off));
    let (mut prev, mut phase) = (0.0f32, 0.0f32);
    for i in 0..n {
        let t = i as f32 / SR;
        let noise = rng.next_f32() * 2.0 - 1.0;
        let high = noise - prev;
        prev = noise;
        phase += std::f32::consts::TAU / SR;
        let metal = (phase * 5_317.0).sin() + (phase * 7_691.0).sin() + (phase * 9_211.0).sin();
        buf[off + i] += (high * 0.24 + metal * 0.12) * (-t * 72.0).exp() * gain * 2500.0;
    }
}

/// Five-note scales in semitones from the root. Hirajoshi's minor second is
/// what makes it sound Japanese; yo is the bright folk scale without one;
/// in is the dark one; the minor pentatonic is the one a rock band plays.
pub(super) const HIRAJOSHI: [f32; 5] = [0.0, 1.0, 5.0, 7.0, 8.0];
pub(super) const YO: [f32; 5] = [0.0, 2.0, 5.0, 7.0, 9.0];
const IN: [f32; 5] = [0.0, 1.0, 5.0, 7.0, 10.0];
const MINOR: [f32; 5] = [0.0, 3.0, 5.0, 7.0, 10.0];
/// The major pentatonic, open and heroic; and the minor with its fifth
/// flattened, which is the blues.
const MAJOR: [f32; 5] = [0.0, 2.0, 4.0, 7.0, 9.0];
const BLUES: [f32; 5] = [0.0, 3.0, 5.0, 6.0, 10.0];

pub(super) fn degree(scale: &[f32; 5], root: f32, step: i32) -> f32 {
    let oct = step.div_euclid(5);
    let d = scale[step.rem_euclid(5) as usize];
    root * 2.0f32.powf((d + 12.0 * oct as f32) / 12.0)
}

/// A shakuhachi: a breathy tone that swells in, wavers once it is held and
/// dies away, with the air of the player's breath round it.
pub(super) fn flute(buf: &mut [f32], off: usize, freq: f32, ms: f32, gain: f32, rng: &mut Rng) {
    let n = ms_to_samples(ms).min(buf.len().saturating_sub(off));
    let (mut ph, mut air) = (0.0f32, 0.0f32);
    for i in 0..n {
        let t = i as f32 / SR;
        let k = i as f32 / n as f32;
        // Blown from just under the note, as the player's lip finds it.
        let scoop = 1.0 - 0.03 * (-t / 0.05).exp();
        let vib = 1.0 + 0.007 * (t / 0.35).min(1.0) * (2.0 * std::f32::consts::PI * 5.2 * t).sin();
        ph += 2.0 * std::f32::consts::PI * freq * scoop * vib / SR;
        air += ((rng.next_f32() * 2.0 - 1.0) - air) * 0.25;
        let e = (t / 0.09).min(1.0) * (1.0 - k).powf(0.7);
        let tone = ph.sin() + 0.28 * (2.0 * ph).sin() + 0.10 * (3.0 * ph).sin();
        buf[off + i] += (tone + air * 0.5) * e * gain * 2600.0;
    }
}

/// A struck glass: one clear partial and its octave, gone in half a second.
/// Not tamed like the koto, so it can sit above the tune.
fn chime(buf: &mut [f32], off: usize, freq: f32, gain: f32) {
    let n = ms_to_samples(520.0).min(buf.len().saturating_sub(off));
    for i in 0..n {
        let t = i as f32 / SR;
        let ph = 2.0 * std::f32::consts::PI * freq * t;
        buf[off + i] += (ph.sin() + 0.3 * (2.0 * ph).sin()) * (-t * 7.0).exp() * gain * 2400.0;
    }
}

/// What a theme is made of. Phrases are four bars of sixteenths in scale
/// degrees, so a tune cannot leave the scale; `bass` is a degree a bar.
struct Theme<'a> {
    bpm: f32,
    /// The lowest note of the tune. Degrees 0..=5 must stay under A4, where
    /// `tame` starts folding notes down an octave and bends the melody.
    root: f32,
    scale: &'a [f32; 5],
    phrases: &'a [&'a [i32]],
    bass: &'a [&'a [i32]],
    /// Which phrase each four-bar slot plays; phrase 3 is the hushed one.
    order: &'a [usize],
    /// 0..1: how hard the drums push.
    drive: f32,
    /// What the sticks do between the drum beats.
    feel: Feel,
}

/// A theme's percussion besides the big drum.
#[derive(Copy, Clone)]
enum Feel {
    Rim,
    March,
    Push,
    Chime,
    HipHop,
    /// 1970s kung fu film funk for THE PAGODA: a syncopated break, swung sixteenths, bongos on the off-beats, a
    /// slap bass jumping octaves, choked guitar stabs on the backbeat and a gong at the top of each phrase.
    Funk,
}

/// A theme sequenced by `order` rather than looped: eight slots of four bars
/// outlast the round, so no seam is heard.
impl Theme<'_> {
    fn pcm(&self) -> Vec<i16> {
        let Theme { bpm, root, scale, phrases, bass, order, drive, feel } = *self;
        let step = (SAMPLE_RATE as f32 * 60.0 / bpm / 4.0) as usize; // 16ths
        let bars = 4 * order.len();
        let steps = bars * 16;
        // Room past the end for the last notes to ring out in, which is
        // then folded back onto the top — see `fold_tail`.
        let body = step * steps;
        let n = body + ms_to_samples(1400.0);
        let mut buf = vec![0.0f32; n];
        let mut rng = Rng(0x9E3B);
        let beat_ms = 60.0 * 1000.0 / bpm;
        for s in 0..steps {
            let off = s * step;
            let slot = s / 64; // which four-bar phrase
            let melody = phrases[order[slot] % phrases.len()];
            let line = bass[order[slot] % bass.len()];
            let within = s % 64; // step inside the phrase
            let bar = within / 16;
            let b = s % 16;
            // Each phrase has a quiet bar, and a whole phrase now and then
            // is quiet throughout — a theme with no let-up in it has
            // nothing to come back from.
            let hush = order[slot] == 3;
            let lull = bar == 2 || hush;
            let last_bar = bar == 3;
            let last_slot = slot + 1 == order.len();

            // The fight under the tune, as hard as the stage drives: a low drum
            // on every beat, a bass string plucked on every eighth (the octave
            // on the off-beats), and sticks on the sixteenths at full drive.
            let ground = line[bar % line.len()];
            if drive >= 0.4 && !hush {
                if b % 4 == 0 && b != 0 {
                    taiko(&mut buf, off, 84.0, 0.42 * drive, &mut rng);
                }
                if b % 2 == 0 && !(lull && b >= 8) {
                    let f = degree(scale, root, ground) * if b % 4 == 2 { 0.5 } else { 0.25 };
                    pluck(&mut buf, off, f, beat_ms * 0.45, 0.5 * drive, &mut rng);
                }
                if drive > 0.8 && b % 2 == 1 && !lull {
                    rim(&mut buf, off, 0.14, &mut rng);
                }
            }

            // The pulse: a heavy taiko on one, an answer past the middle.
            if b == 0 {
                taiko(&mut buf, off, 96.0, if bar == 0 && !hush { 1.0 } else { 0.8 }, &mut rng);
            }
            if b == 8 && !lull {
                taiko(&mut buf, off, 104.0, 0.7, &mut rng);
            }
            if b == 12 && bar % 2 == 1 && !hush {
                taiko(&mut buf, off, 122.0, 0.6, &mut rng);
            }
            // Every phrase fills into the next one; the last one fills
            // hardest, because that is the seam back to the top.
            if last_bar && matches!(b, 10 | 12 | 14) {
                let g = if last_slot { 0.7 } else { 0.5 } + 0.15 * drive;
                taiko(&mut buf, off, 92.0 + (b as f32 - 10.0) * 14.0, g, &mut rng);
            }
            match feel {
                // A parade ground: the rim on every beat, and a roll into each
                // new phrase.
                Feel::March => {
                    if b % 4 == 0 && !hush {
                        rim(&mut buf, off, 0.36, &mut rng);
                    }
                    if b % 4 == 2 {
                        rim(&mut buf, off, 0.5, &mut rng);
                    }
                    if last_bar && b >= 12 {
                        rim(&mut buf, off, 0.22 + 0.06 * (b - 12) as f32, &mut rng);
                    }
                }
                // The city: every rim pushed a sixteenth ahead of where it is
                // expected, and a soft drum in the gap.
                Feel::Push => {
                    if b % 4 == 3 && !(lull && b == 11) {
                        rim(&mut buf, off, 0.46, &mut rng);
                    }
                    if b == 6 && !lull {
                        taiko(&mut buf, off, 132.0, 0.4, &mut rng);
                    }
                }
                // Ice: no sticks at all, and a chime answering the drum from
                // high above the tune.
                Feel::Chime => {
                    if b % 8 == 4 {
                        let d = line[bar % line.len()];
                        chime(&mut buf, off, degree(scale, root, d + 4) * 2.0, if lull { 0.35 } else { 0.5 });
                    }
                }
                // A slow backbeat, shuffled hi-hat and deep kick for THE HOOD.
                Feel::HipHop => {
                    if b == 0 || b == 10 { taiko(&mut buf, off, 58.0, if b == 0 { 0.9 } else { 0.55 }, &mut rng); }
                    if b == 8 { rim(&mut buf, off, 0.72, &mut rng); }
                    if matches!(b, 2 | 6 | 14) { rim(&mut buf, off, 0.16, &mut rng); }
                    if b % 2 == 0 && !hush {
                        let swung = b % 4 == 2;
                        // Half a step rounds back at 92 BPM; 0.6 survives XM's quantization.
                        hi_hat(&mut buf, off + if swung { step * 3 / 5 } else { 0 },
                            if lull { 0.12 } else if swung { 0.24 } else { 0.16 }, &mut rng);
                    }
                }
                Feel::Funk => {
                    // The break: kick on one and the and-of-two, the snare cracking on two and four.
                    if b == 0 || b == 6 || (b == 10 && bar % 2 == 1) { taiko(&mut buf, off, 62.0, if b == 0 { 0.95 } else { 0.7 }, &mut rng); }
                    if b == 4 || b == 12 { rim(&mut buf, off, 0.78, &mut rng); }
                    // Ghost notes either side of the snare.
                    if matches!(b, 7 | 15) && !hush { rim(&mut buf, off, 0.14, &mut rng); }
                    // Sixteenth hats, swung, the off ones lighter.
                    if !hush {
                        let swung = b % 2 == 1;
                        hi_hat(&mut buf, off + if swung { step / 4 } else { 0 }, if swung { 0.10 } else { 0.18 }, &mut rng);
                    }
                    // Bongos answering on the off-beats.
                    if matches!(b, 3 | 11) && !lull { taiko(&mut buf, off, 236.0, 0.32, &mut rng); }
                    if b == 14 && !lull { taiko(&mut buf, off, 212.0, 0.26, &mut rng); }
                    // The gong opens every phrase.
                    if bar == 0 && b == 0 { chime(&mut buf, off, 98.0, 0.55); taiko(&mut buf, off, 49.0, 0.6, &mut rng); }
                    // The slap bass: root, octave jump, a run back down.
                    let d = line[bar % line.len()];
                    let root_hz = degree(scale, root, d) * 0.5;
                    match b {
                        0 => pluck(&mut buf, off, root_hz, beat_ms * 0.5, 0.75, &mut rng),
                        3 => pluck(&mut buf, off, root_hz * 2.0, beat_ms * 0.25, 0.55, &mut rng),
                        6 => pluck(&mut buf, off, root_hz, beat_ms * 0.3, 0.6, &mut rng),
                        10 => pluck(&mut buf, off, degree(scale, root, d + 2) * 0.5, beat_ms * 0.3, 0.5, &mut rng),
                        13 => pluck(&mut buf, off, degree(scale, root, d + 1) * 0.5, beat_ms * 0.25, 0.45, &mut rng),
                        _ => {}
                    }
                    // Choked guitar stabs on the backbeat: three strings at once, cut dead.
                    if (b == 4 || b == 12) && !lull {
                        for up in [0, 2, 4] { pluck(&mut buf, off, degree(scale, root, d + up) * 2.0, beat_ms * 0.12, 0.16, &mut rng); }
                    }
                }
                // Rim on the off-beats, denser on a driving stage.
                Feel::Rim => {
                    if b % 4 == 2 && !(lull && b == 10) {
                        rim(&mut buf, off, 0.5, &mut rng);
                    }
                    if b % 8 == 7 && !hush {
                        rim(&mut buf, off, 0.32, &mut rng);
                    }
                    if drive > 0.5 && b % 4 == 3 && !lull {
                        rim(&mut buf, off, 0.18, &mut rng);
                    }
                }
            }

            // The koto line.
            let note = melody[within % melody.len()];
            if note > -90 {
                let gain = if lull { 0.62 } else { 0.85 };
                pluck(&mut buf, off, degree(scale, root, note), beat_ms * 1.6, gain, &mut rng);
                // Doubled an octave up on the last bar of a phrase, so the
                // tune has a top to it once every four bars.
                if last_bar && b == 0 {
                    pluck(&mut buf, off, degree(scale, root, note + 5), beat_ms * 1.2, 0.4, &mut rng);
                }
                // A second koto answers the tune three steps behind it, on the
                // phrases after the first: the tune comes back with company.
                if order[slot] != 0 && !lull && s + 3 < steps {
                    pluck(&mut buf, off + 3 * step, degree(scale, root, note), beat_ms * 0.9, 0.22, &mut rng);
                }
            }

            // Each phrase opens on a strummed chord: three strings a moment apart.
            if within == 0 && !hush {
                let d = line[0];
                for (j, up) in [0, 2, 4].into_iter().enumerate() {
                    pluck(
                        &mut buf,
                        off + j * ms_to_samples(14.0),
                        degree(scale, root, d + up),
                        beat_ms * 2.4,
                        0.3,
                        &mut rng,
                    );
                }
            }

            // The flute answers where the koto leaves room: over the quiet bar
            // and the cadence, and all through a hushed phrase. It holds a note
            // of the bar's chord an octave above the tune.
            let d = line[bar % line.len()];
            if b == 0 && (lull || last_bar) {
                let up = if last_bar { 0 } else { 2 };
                flute(
                    &mut buf,
                    off,
                    degree(scale, root, d + up) * 2.0,
                    beat_ms * 3.4,
                    if hush { 0.8 } else { 0.6 },
                    &mut rng,
                );
            }

            // The bass walks a note a bar.
            if b == 0 {
                pluck(&mut buf, off, degree(scale, root, d) * 0.5, beat_ms * 3.6, 0.55, &mut rng);
            }
            if b == 10 && !lull {
                pluck(&mut buf, off, degree(scale, root, d + 2) * 0.5, beat_ms * 1.2, 0.3, &mut rng);
            }
        }
        let mut buf = fold_tail(buf, body);
        // Fight music: brighter than the house warmth, and as loud as the other
        // games' (the koto alone sat 12 dB under them).
        low_pass(&mut buf[..body], 6500.0);
        for v in buf[..body].iter_mut() {
            *v *= 2.6;
        }
        soft_limit_to_pcm16(&buf[..body], MIX_KNEE)
    }
}

/// Device-synthesized themes by stage: docks, temple, select, air base, bath house, river, fortress, rooftop.
pub fn theme_wav(which: usize) -> Vec<u8> {
    // The docks, the temple and the select screen are on hirajoshi and differ
    // in pace and density; each later stage has a scale of its own.
    // Four-bar phrases, sequenced; -99 is a rest.
    const R: i32 = -99;

    // -- the docks ------------------------------------------------------
    let d_a = [
        0, R, R, 2, 1, R, 0, R, 3, R, 2, R, 0, R, R, 1, 4, R, 3, R, 2, R, R, 0, 1, R, 0, R, R, 2, R, R, 0, R, R, 2, 1,
        R, 0, R, 3, R, 4, R, 3, R, R, 2, 4, R, 5, R, 4, R, 3, R, 2, R, R, 1, 0, R, R, R,
    ];
    let d_b = [
        2, R, 3, R, 4, R, R, 3, 2, R, R, 4, 5, R, R, R, 4, R, 3, R, 2, R, 1, R, 0, R, R, 2, 1, R, R, R, 3, R, R, 4, 5,
        R, 4, R, 3, R, R, 2, 1, R, 2, R, 0, R, 1, R, 2, R, R, 3, 2, R, 1, R, 0, R, R, R,
    ];
    let d_c = [
        5, R, R, R, 4, R, R, R, 3, R, R, R, 2, R, R, R, 4, R, R, 3, R, R, 2, R, R, R, 1, R, R, R, R, R, 0, R, R, 1, 2,
        R, R, 3, 4, R, R, R, 3, R, R, R, 2, R, R, R, 1, R, R, R, 0, R, R, R, R, R, R, R,
    ];
    let d_d = [
        0, R, R, R, R, R, 2, R, R, R, 1, R, R, R, R, R, 0, R, R, R, R, R, R, R, 2, R, R, R, R, R, R, R, 3, R, R, R, R,
        R, 2, R, R, R, R, R, 1, R, R, R, 0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R,
    ];

    // -- the temple -----------------------------------------------------
    let t_a = [
        3, R, R, R, 2, R, 4, R, 3, R, R, 1, 0, R, R, R, 1, R, 2, R, R, 3, R, 2, 0, R, R, R, R, R, 1, R, 3, R, R, R, 4,
        R, R, 3, 5, R, R, 4, 3, R, R, R, 2, R, R, 1, R, R, 2, R, 0, R, R, R, R, R, R, R,
    ];
    let t_b = [
        5, R, R, 4, R, R, 3, R, R, R, 4, R, 5, R, R, R, 4, R, R, R, 3, R, R, 2, R, R, 3, R, R, R, R, R, 2, R, R, 3, 4,
        R, R, 5, R, R, 4, R, 3, R, R, R, 1, R, R, R, 2, R, R, R, 0, R, R, R, R, R, R, R,
    ];
    let t_c = [
        0, R, R, R, 1, R, R, R, 2, R, R, R, 3, R, R, R, 2, R, R, 1, R, R, 0, R, R, R, R, R, 1, R, R, R, 4, R, R, R, 3,
        R, R, R, 2, R, R, 1, R, R, R, R, 0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R,
    ];
    let t_d = [
        3, R, R, R, R, R, R, R, 2, R, R, R, R, R, R, R, 1, R, R, R, R, R, R, R, 0, R, R, R, R, R, R, R, 2, R, R, R, R,
        R, R, R, 1, R, R, R, R, R, R, R, 0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R,
    ];

    // -- the select screen ----------------------------------------------
    let s_a = [
        0, R, R, R, 3, R, 2, R, R, R, 1, R, 0, R, R, R, 2, R, R, R, 1, R, R, 0, R, R, R, R, 3, R, R, R, 4, R, R, 3, R,
        R, 2, R, R, R, 3, R, 4, R, R, R, 2, R, R, R, 0, R, R, 1, R, R, R, R, R, R, R, R,
    ];
    let s_b = [
        3, R, R, R, 4, R, R, R, 5, R, R, 4, 3, R, R, R, 2, R, R, 3, R, R, 4, R, R, R, 3, R, R, R, R, R, 1, R, R, R, 2,
        R, R, 1, 0, R, R, R, R, R, 2, R, 1, R, R, R, R, R, 0, R, R, R, R, R, R, R, R, R,
    ];

    // -- the air base: a march on the minor pentatonic -------------------
    let a_a = [
        0, R, 0, R, 2, R, 3, R, 4, R, R, 3, R, R, 2, R, 0, R, 0, R, 2, R, 3, R, 5, R, R, 4, R, R, 3, R, 4, R, 4, R, 5,
        R, 6, R, 5, R, R, 4, R, R, 3, R, 2, R, 3, R, 4, R, R, 2, 0, R, R, R, R, R, R, R,
    ];
    let a_b = [
        5, R, R, R, 4, R, 5, R, 6, R, R, R, 5, R, 4, R, 3, R, R, R, 2, R, 3, R, 4, R, R, R, R, R, 2, R, 5, R, R, R, 4,
        R, 5, R, 6, R, R, 5, R, R, 4, R, 3, R, 2, R, 3, R, 4, R, 5, R, R, R, R, R, R, R,
    ];
    let a_c = [
        0, R, R, 2, R, R, 3, R, 4, R, R, R, 3, R, 2, R, 0, R, R, 2, R, R, 3, R, 2, R, R, R, R, R, R, R, 3, R, R, 4, R,
        R, 5, R, 6, R, R, R, 5, R, 4, R, 3, R, R, 2, R, R, 0, R, 0, R, R, R, R, R, R, R,
    ];
    let a_d = [
        0, R, R, R, R, R, R, R, 3, R, R, R, R, R, R, R, 2, R, R, R, R, R, R, R, 0, R, R, R, R, R, R, R, 4, R, R, R, R,
        R, R, R, 3, R, R, R, R, R, R, R, 2, R, R, R, R, R, 3, R, 0, R, R, R, R, R, R, R,
    ];

    // -- the bath house: bright and easy, on yo --------------------------
    let h_a = [
        2, R, R, 3, 4, R, R, R, 3, R, 2, R, 0, R, R, R, 2, R, R, 3, 4, R, 5, R, 4, R, R, R, R, R, 3, R, 2, R, R, 3, 4,
        R, R, R, 5, R, 4, R, 3, R, R, 2, 3, R, 2, R, 1, R, R, 2, 0, R, R, R, R, R, R, R,
    ];
    let h_b = [
        5, R, R, R, 4, R, 3, R, 4, R, R, R, 2, R, R, R, 3, R, R, R, 2, R, 1, R, 2, R, R, R, R, R, R, R, 5, R, R, R, 4,
        R, 3, R, 4, R, R, 5, 6, R, R, R, 5, R, 4, R, 3, R, 2, R, 0, R, R, R, R, R, R, R,
    ];
    let h_c = [
        0, R, 2, R, 3, R, 2, R, 0, R, R, R, R, R, 1, R, 0, R, 2, R, 3, R, 4, R, 3, R, R, R, R, R, R, R, 4, R, 3, R, 2,
        R, 3, R, 4, R, R, R, 5, R, R, R, 4, R, R, 3, R, R, 2, R, 0, R, R, R, R, R, R, R,
    ];
    let h_d = [
        2, R, R, R, R, R, R, R, 3, R, R, R, R, R, R, R, 4, R, R, R, R, R, R, R, 2, R, R, R, R, R, R, R, 3, R, R, R, R,
        R, R, R, 1, R, R, R, R, R, R, R, 0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R,
    ];

    // -- the river village: slow and heavy, on in ------------------------
    let v_a = [
        0, R, R, R, 0, R, 1, R, 2, R, R, R, R, R, R, R, 3, R, R, R, 2, R, 1, R, 0, R, R, R, R, R, R, R, 0, R, R, R, 0,
        R, 1, R, 2, R, R, R, 3, R, R, R, 4, R, R, 3, R, R, 2, R, 0, R, R, R, R, R, R, R,
    ];
    let v_b = [
        4, R, R, R, 3, R, R, R, 4, R, R, R, 5, R, R, R, 4, R, R, 3, R, R, 2, R, 3, R, R, R, R, R, R, R, 2, R, R, R, 3,
        R, R, R, 4, R, R, R, 6, R, R, R, 5, R, R, 4, R, R, 3, R, 2, R, R, R, R, R, R, R,
    ];
    let v_c = [
        0, R, 2, R, R, R, 3, R, R, R, 2, R, 0, R, R, R, 0, R, 2, R, R, R, 3, R, R, R, 4, R, 3, R, R, R, 5, R, R, R, 4,
        R, R, R, 3, R, R, R, 2, R, R, R, 1, R, R, R, 0, R, R, R, 0, R, R, R, R, R, R, R,
    ];
    let v_d = [
        0, R, R, R, R, R, R, R, R, R, R, R, 1, R, R, R, 2, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R, 3, R, R, R, R,
        R, R, R, 2, R, R, R, 1, R, R, R, 0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R,
    ];

    // -- the crystal fortress: slow and open, on the major pentatonic ------
    let f_a = [
        0, R, R, R, 3, R, R, R, R, R, R, R, 4, R, 3, R, 5, R, R, R, R, R, R, R, 4, R, R, R, 3, R, R, R, 0, R, R, R, 3,
        R, R, R, R, R, R, R, 4, R, 5, R, 6, R, R, R, R, R, 5, R, 5, R, R, R, R, R, R, R,
    ];
    let f_b = [
        5, R, R, R, 4, R, R, R, 3, R, R, R, R, R, 2, R, 3, R, R, R, R, R, R, R, 2, R, R, R, 0, R, R, R, 5, R, R, R, 4,
        R, R, R, 3, R, R, R, 4, R, 5, R, 6, R, R, R, R, R, R, R, 5, R, R, R, R, R, R, R,
    ];
    let f_c = [
        2, R, R, R, 3, R, R, R, 4, R, R, R, R, R, R, R, 3, R, R, R, 4, R, R, R, 5, R, R, R, R, R, R, R, 4, R, R, R, 5,
        R, R, R, 6, R, R, R, R, R, 5, R, 4, R, R, R, 3, R, R, R, 0, R, R, R, R, R, R, R,
    ];
    let f_d = [
        0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R, 3, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R, 2, R, R, R, R,
        R, R, R, 4, R, R, R, R, R, R, R, 0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R,
    ];

    // -- the rooftop: pushed off the beat, on the blues -------------------
    let q_a = [
        0, R, R, 0, R, R, 1, R, 2, R, 3, R, 2, R, 1, R, 0, R, R, R, R, R, 4, R, R, R, 3, R, 2, R, R, R, 0, R, R, 0, R,
        R, 1, R, 2, R, 3, R, 4, R, 5, R, 4, R, R, 3, R, R, 2, R, 0, R, R, R, R, R, R, R,
    ];
    let q_b = [
        5, R, R, R, 4, R, R, 5, R, R, 4, R, 3, R, 2, R, 4, R, R, R, 3, R, R, 4, R, R, 3, R, 2, R, 1, R, 5, R, R, R, 4,
        R, R, 5, R, R, 6, R, 5, R, 4, R, 3, R, 2, R, 1, R, 2, R, 0, R, R, R, R, R, R, R,
    ];
    let q_c = [
        2, R, 2, R, R, R, 3, R, 2, R, R, R, 1, R, 0, R, 2, R, 2, R, R, R, 3, R, 4, R, R, R, R, R, R, R, 5, R, 5, R, R,
        R, 4, R, 3, R, R, R, 2, R, 1, R, 0, R, R, 1, R, R, 2, R, 0, R, R, R, R, R, R, R,
    ];
    let q_d = [
        0, R, R, R, R, R, R, R, 2, R, R, R, R, R, 1, R, 0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R, 3, R, R, R, R,
        R, R, R, 2, R, R, R, R, R, 1, R, 0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R,
    ];

    // A note a bar under each, so the ground moves.
    let b0 = [0, 0, 3, 2];
    let b1 = [2, 3, 4, 2];
    let b2 = [0, 4, 3, 0];
    let b3 = [0, 0, 0, 0];

    // Eight slots of four bars: a rondo, so the tune keeps coming back
    // without the piece repeating. Slot 3 is the hushed one.
    let order = [0usize, 1, 0, 3, 2, 1, 0, 1];
    let bass: [&[i32]; 4] = [&b0, &b1, &b2, &b3];
    let stage = |bpm, root, scale, phrases: [&[i32]; 4], drive, feel| {
        Theme { bpm, root, scale, phrases: &phrases, bass: &bass, order: &order, drive, feel }.pcm()
    };
    encode_pcm16_music(&match which {
        0 => stage(148.0, 196.00, &HIRAJOSHI, [&d_a, &d_b, &d_c, &d_d], 1.0, Feel::Rim),
        1 => stage(128.0, 220.00, &HIRAJOSHI, [&t_a, &t_b, &t_c, &t_d], 0.6, Feel::Rim),
        3 => stage(156.0, 174.61, &MINOR, [&a_a, &a_b, &a_c, &a_d], 1.0, Feel::March),
        4 => stage(136.0, 196.00, &YO, [&h_a, &h_b, &h_c, &h_d], 0.7, Feel::Rim),
        5 => stage(124.0, 164.81, &IN, [&v_a, &v_b, &v_c, &v_d], 0.8, Feel::Rim),
        6 => stage(120.0, 196.00, &MAJOR, [&f_a, &f_b, &f_c, &f_d], 0.5, Feel::Chime),
        7 => stage(142.0, 185.00, &BLUES, [&q_a, &q_b, &q_c, &q_d], 0.9, Feel::Push),
        // Original West Coast-flavoured hip-hop groove: swung bass, dry rim and warm keys.
        8 => stage(92.0, 110.00, &MINOR, [&q_a, &v_a, &q_c, &v_d], 0.8, Feel::HipHop),
        // THE PAGODA: kung fu funk on the blues scale, the tune borrowed from the city's phrases.
        9 => stage(112.0, 146.83, &BLUES, [&q_a, &a_c, &q_c, &a_d], 1.0, Feel::Funk),
        // THE COLOSSEUM: the duel, slow and tense: a march held back in a minor key, the airbase's phrases low.
        10 => stage(96.0, 130.81, &MINOR, [&a_a, &a_d, &a_b, &a_d], 0.5, Feel::March),
        _ => Theme {
            bpm: 118.0,
            root: 207.65,
            scale: &HIRAJOSHI,
            phrases: &[&s_a, &s_b],
            bass: &[&b2, &b0],
            order: &[0, 1, 0, 1],
            drive: 0.0,
            feel: Feel::Rim,
        }
        .pcm(),
    })
}

/// Stage ambience loops: 0 crowd, 1 wind, 2 crowd and crickets, 3 crowd and city hum.
pub fn ambience_wav(kind: usize) -> Vec<u8> {
    let body = ms_to_samples(4000.0);
    let fade = ms_to_samples(400.0);
    let mut buf = vec![0.0f32; body + fade];
    let mut rng = Rng(0xA3B1 + kind as u32);
    let (mut lp, mut hp, mut bp, mut low) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / body as f32;
        let turn = |cycles: f32, at: f32| (std::f32::consts::TAU * (cycles * t + at)).sin();
        let noise = rng.next_f32() * 2.0 - 1.0;
        *out = if kind != 1 {
            // Voices: the middle of the band, swelling here and there.
            lp += (noise - lp) * 0.30;
            hp += (lp - hp) * 0.030;
            let crowd =
                (lp - hp) * (1.0 + 0.25 * turn(3.0, 0.0) + 0.18 * turn(7.0, 0.3) + 0.12 * turn(11.0, 0.7)) * 5200.0;
            let secs = t * 4.0;
            crowd
                + match kind {
                    // Crickets: a high note chopped into chirps, three to a burst.
                    2 => {
                        let burst = (secs * 2.5).fract() < 0.45;
                        let chop = (secs * 31.0).fract() < 0.5;
                        if burst && chop {
                            (std::f32::consts::TAU * 3900.0 * secs).sin() * 420.0
                        } else {
                            0.0
                        }
                    }
                    // The city: a low drone that beats slowly against itself.
                    3 => {
                        ((std::f32::consts::TAU * 58.0 * secs).sin() + (std::f32::consts::TAU * 61.25 * secs).sin())
                            * 800.0
                    }
                    _ => 0.0,
                }
        } else {
            // Wind: a narrow band whose pitch rises and falls with the gusts.
            let gust = 0.5 + 0.5 * turn(1.0, 0.0) * turn(3.0, 0.2);
            let f = 2.0 * (std::f32::consts::PI * (260.0 + 420.0 * gust) / SR).sin();
            let high = noise - low - 0.35 * bp;
            bp += f * high;
            low += f * bp;
            bp * (0.35 + 0.65 * gust) * 9000.0
        };
    }
    // The tail is laid over the head, each fading as the other comes in, so
    // the loop has no seam.
    for i in 0..fade {
        let k = i as f32 / fade as f32;
        buf[i] = buf[i] * k + buf[body + i] * (1.0 - k);
    }
    buf.truncate(body);
    encode_pcm16_music(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// The crowd, built at startup: 2.5 s of PCM is a quarter of a megabyte for a
/// few lines of filtered noise.
pub fn crowd_wav(long: bool) -> Vec<u8> {
    let s = if long { crowd(1600.0, 1.0, 0xFEED) } else { crowd(900.0, 0.85, 0xC0DE) };
    encode_pcm16_mono(&s)
}

// ---- combat foley ---------------------------------------------------------

/// Noise through a resonant band-pass (state-variable filter) at `center`
/// Hz, rising in `attack` and dying with time constant `decay` (seconds).
fn band(buf: &mut [f32], at: f32, center: f32, q: f32, amp: f32, attack: f32, decay: f32, rng: &mut Rng) {
    let off = (at * SR) as usize;
    let n = ((attack + decay * 6.0) * SR) as usize;
    let f = 2.0 * (std::f32::consts::PI * center / SR).sin();
    let damp = 1.0 / q;
    let (mut low, mut bp) = (0.0f32, 0.0f32);
    for i in 0..n.min(buf.len().saturating_sub(off)) {
        let t = i as f32 / SR;
        let x = rng.next_f32() * 2.0 - 1.0;
        let high = x - low - damp * bp;
        bp += f * high;
        low += f * bp;
        let e = if t < attack { t / attack.max(1e-4) } else { (-(t - attack) / decay).exp() };
        buf[off + i] += bp * amp * e;
    }
}

/// A few early reflections off the walls, low-passed: places the sound in
/// a room instead of in your ear.
fn room(buf: &mut Vec<f32>, wet: f32) {
    let taps = [(0.011, 0.30), (0.017, 0.22), (0.026, 0.15), (0.037, 0.10), (0.051, 0.06)];
    let extra = (0.06 * SR) as usize;
    let dry = buf.clone();
    buf.resize(dry.len() + extra, 0.0);
    for (d, g) in taps {
        let off = (d * SR) as usize;
        let mut lp = 0.0f32;
        for (i, x) in dry.iter().enumerate() {
            lp += (x - lp) * 0.35;
            buf[i + off] += lp * g * wet;
        }
    }
}

fn render(mut buf: Vec<f32>, gain: f32) -> Vec<i16> {
    low_pass(&mut buf, 5000.0);
    room(&mut buf, 1.0);
    fade_out(&mut buf, ms_to_samples(10.0));
    limit_pcm16(&buf, gain)
}

/// A very short damped sine: the pitch centre of a knock, dead in ~50 ms.
fn knock(buf: &mut [f32], at: f32, f: f32, amp: f32, tau: f32) {
    let off = (at * SR) as usize;
    let n = ((tau * 5.0) * SR) as usize;
    for i in 0..n.min(buf.len().saturating_sub(off)) {
        let t = i as f32 / SR;
        buf[off + i] += (2.0 * std::f32::consts::PI * f * t).sin() * amp * (-t / tau).exp() * (t / 0.0008).min(1.0);
    }
}

/// Synthesize a formant-shaped voice from `f0` to `f1`; `breath` controls the noise mix.
fn voice(
    buf: &mut [f32],
    at: f32,
    dur: f32,
    f0: f32,
    f1: f32,
    vowel: (f32, f32, f32),
    breath: f32,
    amp: f32,
    rng: &mut Rng,
) {
    let off = (at * SR) as usize;
    let n = (dur * SR) as usize;
    let formants = [(vowel.0, 80.0, 1.0), (vowel.1, 110.0, 0.55), (vowel.2, 160.0, 0.3)];
    let mut state = [(0.0f32, 0.0f32); 3];
    let (mut ph, mut jit) = (0.0f32, 0.0f32);
    for i in 0..n.min(buf.len().saturating_sub(off)) {
        let k = i as f32 / n as f32;
        jit = jit * 0.995 + (rng.next_f32() - 0.5) * 0.004;
        let f = (f0 + (f1 - f0) * k) * (1.0 + jit);
        ph += f / SR;
        let mut src = 0.0f32;
        let harmonics = ((3800.0 / f) as usize).max(1);
        for h in 1..=harmonics {
            src += (2.0 * std::f32::consts::PI * ph * h as f32).sin() / (h as f32).powf(1.3);
        }
        let shimmer = 1.0 + (rng.next_f32() - 0.5) * 0.12;
        let air = rng.next_f32() * 2.0 - 1.0;
        let x = src * (1.0 - breath) * shimmer * 0.35 + air * breath * 1.6;
        let mut y = 0.0f32;
        for (j, &(fc, bw, g)) in formants.iter().enumerate() {
            let fq = 2.0 * (std::f32::consts::PI * fc / SR).sin();
            let damp = bw / fc * 2.0;
            let (low, bp) = &mut state[j];
            let high = x - *low - damp * *bp;
            *bp += fq * high;
            *low += fq * *bp;
            y += *bp * g;
        }
        let e = (k / 0.06).min(1.0) * (1.0 - k).powf(1.4);
        buf[off + i] += y * e * amp;
    }
}

// Vowels as formant triples.
const UH: (f32, f32, f32) = (640.0, 1190.0, 2390.0); // "uh", the grunt of a blow
const OO: (f32, f32, f32) = (380.0, 900.0, 2300.0); // "oof", winded
const AH: (f32, f32, f32) = (760.0, 1150.0, 2500.0); // "ah", the cry going down

/// A blow landing on a body. `thud` is the centre of the muffled flesh thud
/// (the combo climbs it); `weight` 0..1 runs from a jab to a heavy kick:
/// a deeper, longer thud, more cloth, and a voiced grunt instead of a breath.
fn hit(thud: f32, weight: f32, seed: u32) -> Vec<i16> { blow(thud, weight, false, seed) }

/// A blow landing; a kick (`shin`) is a shin or instep, not knuckles: less
/// crack, more of the body and the cloth. The one hit says nothing: the
/// fighter taking it has their own voice (`hurt`).
fn blow(thud: f32, weight: f32, shin: bool, seed: u32) -> Vec<i16> {
    let mut rng = Rng(seed);
    let mut buf = vec![0.0f32; (0.34 * SR) as usize];
    let (crack, body, cloth) = if shin { (0.55, 1.35, 1.8) } else { (1.0, 1.0, 1.0) };
    // the smack: skin on skin, a crack and a crackle
    band(&mut buf, 0.0, if shin { 1700.0 } else { 2400.0 }, 1.2, 0.9 * crack, 0.0003, 0.003 + 0.002 * weight, &mut rng);
    band(&mut buf, 0.0021, 1600.0, 1.5, 0.35 * crack, 0.0002, 0.0025, &mut rng);
    // the flesh thud: a muffled knock with a pitch centre and no ring (a
    // 12 ms core dies in about 50 ms; a drum rings for hundreds)
    band(&mut buf, 0.0, thud, 4.0, 1.8, 0.0006, 0.016 + 0.01 * weight, &mut rng);
    knock(&mut buf, 0.0, thud, 0.9, 0.012);
    // the body mass behind it, deep and dead
    band(&mut buf, 0.001, 110.0, 0.8, (0.8 + 1.4 * weight) * body, 0.002, 0.02 + 0.025 * weight, &mut rng);
    // the gi
    band(&mut buf, 0.003, 950.0, 0.8, (0.2 + 0.25 * weight) * cloth, 0.002, 0.03 + 0.02 * weight, &mut rng);
    render(buf, 13_000.0 + 5_000.0 * weight)
}

/// Who is shouting: the pitch of the voice and how far up its vowels sit
/// (a shorter throat lifts them). A man, a deep voice, a woman, a turtle, and DRAKE's high, keening one.
#[derive(Copy, Clone)]
pub struct Throat { pub f0: f32, pub size: f32 }
pub const THROATS: [Throat; 6] = [
    Throat { f0: 120.0, size: 1.0 }, Throat { f0: 86.0, size: 0.86 },
    Throat { f0: 215.0, size: 1.17 }, Throat { f0: 285.0, size: 1.3 },
    Throat { f0: 165.0, size: 1.08 },
    // LANCER: a baritone a little under the first man's, fuller in the chest.
    Throat { f0: 105.0, size: 0.94 },
];

/// A falsetto cry built from its harmonics, to the measure of three reference yells (20 ms frames): a short low
/// burst from the chest (the 2nd and 3rd harmonics as strong as the 1st, with breath), then a leap into falsetto,
/// nearly a pure tone: the 2nd harmonic 25 dB down, the 3rd 32, the 4th 40, breath 40. `contour` is points of
/// (0..1 through the note, multiple of the throat's f0, loudness in dB, how open: 0 pure to 1 `BRIGHT`); `chest` is how much of the note the burst
/// takes. (Formant filters on a buzz, which this replaced, put the 2nd harmonic at -3 dB and breath at -22: a hen.)
fn wail(buf: &mut [f32], at: f32, dur: f32, t: Throat, contour: &[(f32, f32, f32, f32)], chest: f32, rng: &mut Rng) {
    const CHEST: [f32; 6] = [0.0, 3.0, 2.0, -6.0, -8.0, -12.0];
    const HEAD: [f32; 6] = [0.0, -25.0, -32.0, -40.0, -46.0, -52.0];
    // The open "aah" a cry can flip into before it thins (the long reference: its 2nd harmonic 8 to 10 dB over the
    // 1st just after the flip, falling to -20 by three quarters through).
    const BRIGHT: [f32; 6] = [0.0, 8.0, -12.0, -16.0, -20.0, -26.0];
    let off = (at * SR) as usize;
    let n = (dur * SR) as usize;
    let mut ph = 0.0f32;
    let (mut wander, mut swell) = (0.0f32, 0.0f32);
    for i in 0..n.min(buf.len().saturating_sub(off)) {
        let k = i as f32 / n as f32;
        let j = contour.iter().rposition(|p| p.0 <= k).unwrap_or(0);
        let (a, b) = (contour[j], contour[(j + 1).min(contour.len() - 1)]);
        let u = if b.0 > a.0 { (k - a.0) / (b.0 - a.0) } else { 0.0 };
        let u = u * u * (3.0 - 2.0 * u);
        // A wander of about two per cent: the reference holds drift that much frame to frame, no faster vibrato.
        wander += ((rng.next_f32() * 2.0 - 1.0) - wander) * 0.0015;
        let f = t.f0 * (a.1 + (b.1 - a.1) * u) * (1.0 + 0.02 * wander);
        let gain = 10f32.powf((a.2 + (b.2 - a.2) * u) / 20.0);
        let open = a.3 + (b.3 - a.3) * u;
        ph += f / SR;
        let head = ((k - chest) / 0.02).clamp(0.0, 1.0);
        let mut src = 0.0f32;
        for h in 0..6 {
            if f * (h + 1) as f32 > 7000.0 { break; }
            let d = HEAD[h] + (BRIGHT[h] - HEAD[h]) * open;
            src += 10f32.powf((CHEST[h] + (d - CHEST[h]) * head) / 20.0) * (std::f32::consts::TAU * ph * (h + 1) as f32).sin();
        }
        // Breath under the falsetto at about -55 dB, and the loudness wandering a couple of dB, as the reference's does.
        let breath = (rng.next_f32() * 2.0 - 1.0) * (0.15 * (1.0 - head) + 0.05);
        swell += ((rng.next_f32() * 2.0 - 1.0) - swell) * 0.002;
        let gain = gain * (1.0 + 0.25 * swell);
        let e = (k / 0.02).min(1.0) * ((1.0 - k) / 0.08).min(1.0);
        buf[off + i] += (src + breath) * gain * e * 0.5;
    }
}

/// DRAKE struck: a short falsetto yelp, or with `heavy` the cry falling away (the reference's at 49 s, quick).
fn yelp(t: Throat, take: u32, heavy: bool) -> Vec<i16> {
    let mut rng = Rng(0x7E1 ^ take.wrapping_mul(7_919));
    let mut buf = vec![0.0f32; (0.6 * SR) as usize];
    if heavy {
        wail(&mut buf, 0.0, 0.42, t, &[(0.0, 1.6, -4.0, 0.0), (0.12, 1.7, -2.0, 0.0), (0.16, 3.9, 0.0, 0.4), (0.4, 3.6, -2.0, 0.0), (1.0, 2.2, -14.0, 0.0)], 0.14, &mut rng);
    } else {
        let top = if take == 0 { 3.6 } else { 3.3 };
        wail(&mut buf, 0.0, 0.16, t, &[(0.0, 1.7, -4.0, 0.0), (0.3, 1.8, -2.0, 0.0), (0.38, top, 0.0, 0.5), (1.0, top - 0.7, -8.0, 0.0)], 0.36, &mut rng);
    }
    hall(&mut buf, 0.25, 0.4);
    render(buf, 5_200.0)
}

/// The Colosseum's kitten, to the measure of a recorded kitten (20 ms frames): one tone that rises from 1250 Hz to
/// 1550 in the first tenth of a second and glides down to 650, bright at the start (the 2nd to 4th harmonics as loud
/// as the 1st, the "mi"), purer through the fall (the 2nd 10 to 20 dB down), opening a little at the end ("ow"), and
/// fading 15 dB. (A voice through human vowel resonances, which this replaced, sounded like a person saying "meow".)
fn meow() -> Vec<i16> {
    const PURE: [f32; 5] = [0.0, -14.0, -22.0, -28.0, -34.0];
    const BRIGHT: [f32; 5] = [0.0, 5.0, 0.0, 2.0, -6.0];
    // (0..1 through the call, Hz, dB, brightness)
    const CALL: [(f32, f32, f32, f32); 6] =
        [(0.0, 1250.0, -6.0, 1.0), (0.15, 1560.0, 0.0, 1.0), (0.35, 1400.0, -2.0, 0.45), (0.6, 1100.0, -5.0, 0.15),
         (0.85, 800.0, -10.0, 0.4), (1.0, 660.0, -16.0, 0.6)];
    let mut rng = Rng(0xCA7);
    let dur = 0.62;
    let n = (dur * SR) as usize;
    let mut buf = vec![0.0f32; (0.9 * SR) as usize];
    let mut ph = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate().take(n) {
        let k = i as f32 / n as f32;
        let j = CALL.iter().rposition(|p| p.0 <= k).unwrap_or(0);
        let (a, b) = (CALL[j], CALL[(j + 1).min(CALL.len() - 1)]);
        let u = if b.0 > a.0 { (k - a.0) / (b.0 - a.0) } else { 0.0 };
        let u = u * u * (3.0 - 2.0 * u);
        // A slight waver, as a small throat holds a note.
        let f = (a.1 + (b.1 - a.1) * u) * (1.0 + 0.012 * (i as f32 / SR * 7.0 * std::f32::consts::TAU).sin());
        let gain = 10f32.powf((a.2 + (b.2 - a.2) * u) / 20.0);
        let bright = a.3 + (b.3 - a.3) * u;
        ph += f / SR;
        let mut s = 0.0;
        for h in 0..5 {
            if f * (h + 1) as f32 > 7500.0 { break; }
            let db = PURE[h] + (BRIGHT[h] - PURE[h]) * bright;
            s += 10f32.powf(db / 20.0) * (std::f32::consts::TAU * ph * (h + 1) as f32).sin();
        }
        let e = (k / 0.05).min(1.0) * ((1.0 - k) / 0.12).min(1.0);
        *out = (s * 0.35 + (rng.next_f32() * 2.0 - 1.0) * 0.004) * gain * e;
    }
    hall(&mut buf, 0.25, 0.5);
    render(buf, 9_000.0)
}

/// DRAKE knocked out: the long falsetto cry, held and falling away to nothing.
fn keen_down(t: Throat) -> Vec<i16> {
    let mut rng = Rng(0xD0E);
    let mut buf = vec![0.0f32; (1.0 * SR) as usize];
    wail(&mut buf, 0.0, 0.85, t, &[(0.0, 1.6, -4.0, 0.0), (0.08, 1.7, -2.0, 0.0), (0.1, 4.1, 0.0, 0.6), (0.35, 4.0, 0.0, 0.2), (0.7, 2.8, -6.0, 0.0), (1.0, 1.9, -18.0, 0.0)], 0.09, &mut rng);
    hall(&mut buf, 0.35, 0.7);
    render(buf, 5_200.0)
}

/// A hall's tail: four feedback combs into an allpass, `secs` to die away 60 dB, mixed in at `wet`. The reference
/// yells (dubbed in a studio) carry one: their floor between harmonics is 20 to 40 dB down, where a dry voice is 60.
fn hall(buf: &mut Vec<f32>, wet: f32, secs: f32) {
    let dry = buf.clone();
    buf.resize(dry.len() + (secs * SR) as usize, 0.0);
    let mut tail = vec![0.0f32; buf.len()];
    for d in [0.0297f32, 0.0371, 0.0411, 0.0437] {
        let n = (d * SR) as usize;
        let g = 10f32.powf(-3.0 * d / secs);
        let mut line = vec![0.0f32; buf.len()];
        let mut lp = 0.0f32;
        for i in 0..buf.len() {
            let back = if i >= n { line[i - n] } else { 0.0 };
            lp += (back - lp) * 0.4;
            line[i] = dry.get(i).copied().unwrap_or(0.0) + lp * g;
            tail[i] += back * 0.25;
        }
    }
    let n = (0.005 * SR) as usize;
    let mut ap = vec![0.0f32; buf.len()];
    for i in 0..buf.len() {
        let back = if i >= n { ap[i - n] } else { 0.0 };
        ap[i] = tail[i] + back * 0.5;
        buf[i] += (back - 0.5 * ap[i]) * wet;
    }
}

/// DRAKE's shout: a high falsetto cry, modelled on reference yells measured for this (pitches for his 165 Hz
/// throat): "ha-AAAH" climbing, 590 to 740 Hz over most of a second; "wa-TAAH", a leap from a low burst to 580,
/// rising to 640, a catch, then up to 770; "AAAOOOH", held at 660 and falling away to 340 as it fades; a short
/// "HAH"; and a held cry that ends on a clipped second.
fn keen(t: Throat, take: u32) -> Vec<i16> {
    let mut rng = Rng(0xD7A ^ take.wrapping_mul(7_919));
    let mut buf = vec![0.0f32; (1.05 * SR) as usize];
    match take % 5 {
        // "HAH!": a burst and a short high hold, open, falling.
        3 => wail(&mut buf, 0.0, 0.34, t, &[(0.0, 1.6, -6.0, 0.0), (0.14, 1.7, -3.0, 0.0), (0.18, 3.6, 0.0, 0.6), (0.6, 3.8, 0.0, 0.3), (1.0, 3.2, -6.0, 0.0)], 0.16, &mut rng),
        // "AAAOOOH" (the reference at 49 s): held pure at 660, then falling away to 340 as it fades.
        4 => wail(&mut buf, 0.0, 0.70, t, &[(0.0, 3.7, -2.0, 0.1), (0.1, 4.05, 0.0, 0.0), (0.35, 3.95, 0.0, 0.0), (0.6, 3.0, -5.0, 0.0), (1.0, 2.05, -15.0, 0.0)], 0.0, &mut rng),
        // "ha-AAAH" (at 55 s): flips open at 595 Hz, steps up to 680 and climbs to 740, thinning to pure.
        0 => wail(&mut buf, 0.0, 1.0, t, &[(0.0, 3.3, -6.0, 0.0), (0.18, 3.35, -4.0, 0.0), (0.21, 3.6, 0.0, 1.0), (0.45, 3.62, 0.0, 0.75), (0.5, 4.1, 0.0, 0.65), (0.68, 4.25, 0.0, 0.6), (0.76, 4.4, 0.0, 0.0), (0.88, 4.45, 0.0, 0.0), (1.0, 4.35, -4.0, 0.0)], 0.2, &mut rng),
        // "wa-TAAH" (at 25 s): burst, leap, rise, catch, up again; nearly pure.
        1 => wail(&mut buf, 0.0, 0.62, t, &[(0.0, 1.6, -6.0, 0.0), (0.08, 1.65, -3.0, 0.0), (0.11, 3.0, 0.0, 0.3), (0.2, 3.5, 0.0, 0.2), (0.5, 3.9, 0.0, 0.1), (0.6, 3.75, -4.0, 0.1), (0.72, 4.55, 0.0, 0.1), (1.0, 4.7, -3.0, 0.0)], 0.1, &mut rng),
        // A held cry, then a clipped second, lower.
        _ => {
            wail(&mut buf, 0.0, 0.48, t, &[(0.0, 1.6, -6.0, 0.0), (0.1, 1.7, -3.0, 0.0), (0.13, 3.7, 0.0, 0.7), (1.0, 4.1, -2.0, 0.0)], 0.12, &mut rng);
            wail(&mut buf, 0.52, 0.26, t, &[(0.0, 3.8, -3.0, 0.4), (0.4, 3.9, -3.0, 0.2), (1.0, 3.2, -9.0, 0.0)], 0.0, &mut rng);
        }
    }
    hall(&mut buf, 0.35, 0.7);
    // (5,200: at 11,000, as the other voices are rendered, the pure falsetto sat 10 dB over the other fighters' shouts,
    // -14 dB mean against -25, and pierced; now about 6 dB over, still the loudest voice in the fight.)
    render(buf, 5_200.0)
}
const EH: (f32, f32, f32) = (530.0, 1840.0, 2480.0);
const EE: (f32, f32, f32) = (300.0, 2200.0, 2950.0);

fn throat_vowel(v: (f32, f32, f32), t: Throat) -> (f32, f32, f32) { (v.0 * t.size, v.1 * t.size, v.2 * t.size) }

/// Taking a blow: the air knocked out in a short grunt, three ways (`take`),
/// or with `heavy` a longer cry that breaks and falls.
fn hurt(t: Throat, take: u32, heavy: bool) -> Vec<i16> {
    let mut rng = Rng(0x4A11 ^ take.wrapping_mul(7919) ^ t.f0 as u32);
    let mut buf = vec![0.0f32; (0.42 * SR) as usize];
    // the breath forced out ahead of the voice
    band(&mut buf, 0.0, 1500.0 * t.size, 1.0, 0.35, 0.001, 0.014, &mut rng);
    if heavy {
        voice(&mut buf, 0.01, 0.20, t.f0 * 1.7, t.f0 * 1.2, throat_vowel(AH, t), 0.3, 0.9, &mut rng);
        voice(&mut buf, 0.17, 0.16, t.f0 * 1.15, t.f0 * 0.8, throat_vowel(UH, t), 0.45, 0.6, &mut rng);
    } else {
        let (v, dur, rise, breath) = match take % 3 {
            0 => (UH, 0.10, 1.25, 0.45),
            1 => (EH, 0.08, 1.35, 0.55),
            _ => (OO, 0.13, 1.1, 0.4),
        };
        voice(&mut buf, 0.008, dur, t.f0 * rise, t.f0 * 0.95, throat_vowel(v, t), breath, 0.75, &mut rng);
    }
    render(buf, 12_000.0)
}

/// The shout behind a big blow (kiai): "ha!", "hyah!" and "tah!", sharp at
/// the front and gone in a fifth of a second.
fn kiai(t: Throat, take: u32) -> Vec<i16> {
    let mut rng = Rng(0x7A1 ^ take.wrapping_mul(104_729) ^ t.f0 as u32);
    let mut buf = vec![0.0f32; (0.36 * SR) as usize];
    let ah = throat_vowel(AH, t);
    match take % 3 {
        0 => {
            band(&mut buf, 0.0, 1300.0 * t.size, 0.9, 0.5, 0.004, 0.012, &mut rng);
            voice(&mut buf, 0.018, 0.17, t.f0 * 1.9, t.f0 * 1.35, ah, 0.22, 1.0, &mut rng);
        }
        1 => {
            band(&mut buf, 0.0, 1800.0 * t.size, 1.0, 0.4, 0.003, 0.01, &mut rng);
            voice(&mut buf, 0.012, 0.06, t.f0 * 1.7, t.f0 * 2.0, throat_vowel(EE, t), 0.25, 0.7, &mut rng);
            voice(&mut buf, 0.05, 0.17, t.f0 * 2.0, t.f0 * 1.3, ah, 0.2, 1.0, &mut rng);
        }
        _ => {
            band(&mut buf, 0.0, 3200.0, 1.4, 0.9, 0.0003, 0.003, &mut rng);
            voice(&mut buf, 0.006, 0.13, t.f0 * 1.8, t.f0 * 1.45, ah, 0.18, 1.0, &mut rng);
        }
    }
    render(buf, 11_000.0)
}

/// Cut the silent tail a buffer was sized for: 28 voice takes ship in the wasm.
fn trimmed(mut s: Vec<i16>) -> Vec<i16> {
    let end = s.iter().rposition(|v| v.unsigned_abs() > 60).map_or(0, |i| i + 1);
    s.truncate((end + ms_to_samples(4.0)).min(s.len()));
    s
}

/// The knockout cry in a fighter's own voice: long, falling away.
fn ko_cry(t: Throat) -> Vec<i16> {
    let mut rng = Rng(0xC1 ^ t.f0 as u32);
    let mut buf = vec![0.0f32; (0.9 * SR) as usize];
    voice(&mut buf, 0.0, 0.75, t.f0 * 1.75, t.f0 * 0.8, throat_vowel(AH, t), 0.3, 1.1, &mut rng);
    render(buf, 12_000.0)
}

/// Blocked: forearm on forearm, a dull dry knock and the sleeves slapping,
/// and the blocker's short exhale. No skin smack, no body thud behind it,
/// so it never sounds like a hit landing.
fn block_sfx() -> Vec<i16> { block_take(0x51A7) }

/// Another take of the block, from its own seed.
fn block_take(seed: u32) -> Vec<i16> {
    let mut rng = Rng(seed);
    let mut buf = vec![0.0f32; (0.2 * SR) as usize];
    band(&mut buf, 0.0, 520.0, 1.6, 1.2, 0.0004, 0.008, &mut rng);
    band(&mut buf, 0.0, 1300.0, 1.0, 0.6, 0.0005, 0.01, &mut rng);
    band(&mut buf, 0.004, 800.0, 0.8, 0.3, 0.002, 0.02, &mut rng);
    voice(&mut buf, 0.008, 0.07, 150.0, 140.0, UH, 0.9, 0.25, &mut rng);
    render(buf, 20_000.0)
}

/// A body going down on the boards: hips then shoulders landing as dull deep
/// thuds, a short dead knock of the planks, the gi, and a winded "oof".
fn crunch() -> Vec<i16> {
    let mut rng = Rng(0x5A);
    let mut buf = vec![0.0f32; (0.6 * SR) as usize];
    band(&mut buf, 0.0, 85.0, 0.9, 3.0, 0.002, 0.05, &mut rng);
    band(&mut buf, 0.0, 240.0, 1.2, 0.9, 0.001, 0.012, &mut rng);
    band(&mut buf, 0.0, 700.0, 1.0, 0.35, 0.001, 0.02, &mut rng);
    band(&mut buf, 0.09, 80.0, 0.9, 2.2, 0.002, 0.045, &mut rng);
    band(&mut buf, 0.09, 220.0, 1.2, 0.6, 0.001, 0.01, &mut rng);
    band(&mut buf, 0.05, 600.0, 0.8, 0.2, 0.01, 0.07, &mut rng);
    low_pass(&mut buf, 1400.0); // a body is soft: the fall is all low
    render(buf, 26_000.0)
}

/// A footfall on the walk: the sole set down, a soft knock and the brush of it, quiet under everything.
fn step(seed: u32) -> Vec<i16> {
    let mut rng = Rng(seed);
    let mut buf = vec![0.0f32; (0.09 * SR) as usize];
    band(&mut buf, 0.0, 180.0, 1.0, 0.9, 0.002, 0.012, &mut rng);
    band(&mut buf, 0.004, 2200.0, 0.9, 0.22, 0.004, 0.012, &mut rng);
    render(buf, 18_000.0)
}

/// Boots on boards: a short dead knock with a heel click.
fn land(seed: u32) -> Vec<i16> {
    let mut rng = Rng(seed);
    let mut buf = vec![0.0f32; (0.16 * SR) as usize];
    band(&mut buf, 0.0, 150.0, 1.0, 1.4, 0.001, 0.02, &mut rng);
    band(&mut buf, 0.0, 420.0, 1.4, 0.5, 0.0005, 0.008, &mut rng);
    band(&mut buf, 0.0, 3000.0, 1.5, 0.3, 0.0003, 0.002, &mut rng);
    render(buf, 24_000.0)
}

/// The knockout: the finishing blow, a long cry falling away, and the body
/// hitting the boards.
fn ko() -> Vec<i16> {
    let mut rng = Rng(0xF157);
    let mut buf = vec![0.0f32; (1.1 * SR) as usize];
    band(&mut buf, 0.0, 2400.0, 1.2, 1.0, 0.0003, 0.005, &mut rng);
    band(&mut buf, 0.0, 300.0, 2.0, 1.8, 0.0006, 0.03, &mut rng);
    band(&mut buf, 0.001, 110.0, 0.8, 2.2, 0.002, 0.05, &mut rng);
    band(&mut buf, 0.72, 85.0, 0.9, 2.6, 0.002, 0.05, &mut rng);
    band(&mut buf, 0.8, 80.0, 0.9, 1.8, 0.002, 0.045, &mut rng);
    band(&mut buf, 0.74, 600.0, 0.8, 0.25, 0.01, 0.06, &mut rng);
    render(buf, 14_000.0)
}

/// The swing, heard before anything connects: air through a resonant band
/// that sweeps up as the limb passes and back down, with the sleeve
/// fluttering in it.
fn whoosh(ms: f32, vol: f32, seed: u32) -> Vec<i16> {
    let mut rng = Rng(seed);
    let n = ms_to_samples(ms);
    let mut buf = vec![0.0f32; n];
    let (mut low, mut bp) = (0.0f32, 0.0f32);
    for (i, out) in buf.iter_mut().enumerate() {
        let k = i as f32 / n as f32;
        let center = 260.0 + 900.0 * (k * std::f32::consts::PI).sin().powf(1.5);
        let f = 2.0 * (std::f32::consts::PI * center / SR).sin();
        let x = rng.next_f32() * 2.0 - 1.0;
        let high = x - low - 0.55 * bp;
        bp += f * high;
        low += f * bp;
        let flutter = 1.0 + 0.18 * (k * 60.0).sin();
        *out = bp * (k * std::f32::consts::PI).sin().powf(1.3) * flutter * vol;
    }
    low_pass(&mut buf, 2200.0);
    render(buf, 16_000.0)
}

/// A crowd, all at once: a wide band of noise that swells and falls, with a
/// slow wobble so it reads as many voices, and no pitch (a pitched cheer is a
/// chord).
fn crowd(ms: f32, vol: f32, seed: u32) -> Vec<i16> {
    let n = ms_to_samples(ms);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(seed);
    let mut lp = 0.0f32;
    let mut hp = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / n as f32;
        // Up fast, down slow: a room reacting, then settling.
        let e = if t < 0.16 { t / 0.16 } else { (1.0 - (t - 0.16) / 0.84).powf(1.7) };
        let noise = rng.next_f32() * 2.0 - 1.0;
        // Open enough to stay a cheer. Filtered down to a narrow band
        // it came out as a rumble — measurably lower in pitch than a
        // punch, which is not what a room full of people sounds like.
        lp += (noise - lp) * 0.78;
        hp += (lp - hp) * 0.020;
        let voices = lp - hp;
        // Several slow wobbles at once — individual voices coming and
        // going out of the mass.
        let w = 1.0 + 0.22 * (t * 37.0).sin() + 0.15 * (t * 23.0 + 1.7).sin() + 0.10 * (t * 61.0 + 0.4).sin();
        *out = voices * w * e * vol * 5200.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// Fast cloth snap from an upward-opening noise band and quieter return crack.
fn gi_snap(seed: u32) -> Vec<i16> {
    let ms = 150.0;
    let n = ms_to_samples(ms);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(seed);
    let per_ms = ms_to_samples(1.0) as f32;
    // Each crack gets its own filter sweep; sharing one made the second the
    // louder.
    for (at_ms, gain) in [(0.0f32, 1.0f32), (62.0, 0.38)] {
        let from = ms_to_samples(at_ms);
        let (mut lp, mut prev) = (0.0f32, 0.0f32);
        for (i, slot) in buf.iter_mut().enumerate().skip(from) {
            let d = (i - from) as f32 / per_ms;
            let noise = rng.next_f32() * 2.0 - 1.0;
            // The band opens as the slack runs out.
            let k = (0.20 + 0.030 * d).min(0.95);
            lp += (noise - lp) * k;
            let bright = lp - prev;
            prev = lp;
            // Almost all attack: up in two milliseconds, gone in fifty.
            let e = (d / 2.5).min(1.0) * (-d * 0.062).exp();
            *slot += bright * e * gain * 19000.0;
        }
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// The bell that opens a round.
fn bell() -> Vec<i16> {
    let n = ms_to_samples(900.0);
    let mut buf = vec![0.0f32; n];
    // Three inharmonic partials: a struck bell, not a note.
    for (freq, amp) in [(880.0f32, 1.0f32), (886.0, 0.45), (1320.0, 0.55), (2093.0, 0.3), (440.0, 0.35)] {
        for (i, out) in buf.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;
            let decay = (-3.2 * t).exp();
            let phase = 2.0 * std::f32::consts::PI * freq * t;
            *out += phase.sin() * decay * amp * 3800.0;
        }
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// A fireball leaving the hands: the shout that throws it, the air catching
/// light in a band that sweeps up, and a low body under both.
fn projectile() -> Vec<i16> {
    let mut rng = Rng(0x2B01);
    let mut buf = vec![0.0f32; (0.5 * SR) as usize];
    voice(&mut buf, 0.0, 0.3, 190.0, 240.0, AH, 0.25, 0.8, &mut rng);
    for k in 0..10 {
        let t = k as f32 / 10.0;
        band(&mut buf, 0.03 * k as f32, 420.0 + 1500.0 * t, 2.2, 0.5 * (1.0 - 0.5 * t), 0.01, 0.03, &mut rng);
    }
    band(&mut buf, 0.0, 130.0, 1.0, 1.2, 0.004, 0.08, &mut rng);
    render(buf, 11_000.0)
}

/// Eye beams: two tones a hair apart falling fast, so they beat, over a hiss
/// of scorched air.
fn laser() -> Vec<i16> {
    let n = ms_to_samples(380.0);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(0x1A5E);
    let (mut a, mut b) = (0.0f32, 0.0f32);
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / n as f32;
        let e = env(i, n, ms_to_samples(3.0), ms_to_samples(240.0));
        let f = 1500.0 * (1.0 - t).powf(1.6) + 260.0;
        a += 2.0 * std::f32::consts::PI * f / SR;
        b += 2.0 * std::f32::consts::PI * f * 1.035 / SR;
        let hiss = (rng.next_f32() * 2.0 - 1.0) * 0.18 * (1.0 - t);
        *out = (a.sin() * 0.5 + b.sin() * 0.4 + hiss) * e * 7000.0;
    }
    low_pass(&mut buf, 4200.0);
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// The ground giving under a giant: a long low rumble, the boards jumping
/// twice, and grit coming down after.
fn quake() -> Vec<i16> {
    let mut rng = Rng(0x9A4E);
    let mut buf = vec![0.0f32; (1.0 * SR) as usize];
    band(&mut buf, 0.0, 62.0, 1.4, 4.0, 0.004, 0.16, &mut rng);
    band(&mut buf, 0.0, 120.0, 1.0, 2.2, 0.002, 0.07, &mut rng);
    knock(&mut buf, 0.0, 70.0, 1.6, 0.06);
    band(&mut buf, 0.14, 90.0, 1.2, 1.8, 0.004, 0.1, &mut rng);
    band(&mut buf, 0.0, 420.0, 0.9, 0.5, 0.001, 0.03, &mut rng);
    band(&mut buf, 0.2, 1500.0, 0.7, 0.12, 0.08, 0.12, &mut rng);
    low_pass(&mut buf, 1800.0);
    render(buf, 6_200.0)
}

/// A blow turned aside: steel on steel, bright and gone, over the knock of
/// the forearms. Nothing like a hit, nothing like a block.
fn parry() -> Vec<i16> {
    let mut rng = Rng(0x7E4C);
    let mut buf = vec![0.0f32; (0.5 * SR) as usize];
    for (f, amp, tau) in [(1568.0f32, 0.8f32, 0.09f32), (2637.0, 0.55, 0.06), (3951.0, 0.3, 0.035)] {
        knock(&mut buf, 0.0, f, amp, tau);
    }
    band(&mut buf, 0.0, 3200.0, 2.0, 0.5, 0.0002, 0.004, &mut rng);
    band(&mut buf, 0.0, 520.0, 1.6, 0.7, 0.0004, 0.008, &mut rng);
    render(buf, 9_000.0)
}

/// Caught swinging: a dry crack laid over the hit, sharper than any of them.
fn counter() -> Vec<i16> {
    let mut rng = Rng(0xC0A7);
    let mut buf = vec![0.0f32; (0.22 * SR) as usize];
    band(&mut buf, 0.0, 3400.0, 1.4, 1.3, 0.0002, 0.004, &mut rng);
    band(&mut buf, 0.0015, 1900.0, 1.6, 0.8, 0.0002, 0.006, &mut rng);
    knock(&mut buf, 0.0, 190.0, 1.2, 0.02);
    band(&mut buf, 0.0, 95.0, 0.8, 1.6, 0.002, 0.04, &mut rng);
    render(buf, 7_500.0)
}

/// Notes of the koto one after another, rising: the bonus taken.
fn run_up(steps: &[i32], root: f32, gap_ms: f32, ring_ms: f32, seed: u32) -> Vec<i16> {
    let gap = ms_to_samples(gap_ms);
    let mut buf = vec![0.0f32; gap * steps.len() + ms_to_samples(ring_ms)];
    let mut rng = Rng(seed);
    for (i, &d) in steps.iter().enumerate() {
        pluck(&mut buf, i * gap, degree(&HIRAJOSHI, root, d), ring_ms, 0.9, &mut rng);
    }
    render(buf, 1.0)
}

/// Fingers in the mouth: two notes, the second higher and held, with the
/// breath that makes them.
fn whistle() -> Vec<i16> {
    let n = ms_to_samples(520.0);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(0x3157);
    let (mut ph, mut air) = (0.0f32, 0.0f32);
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / SR;
        // 0.16 s on the low note, a quick slide, the rest on the high one.
        let k = ((t - 0.16) / 0.05).clamp(0.0, 1.0);
        let f = 1250.0 + 650.0 * k * k * (3.0 - 2.0 * k);
        ph += 2.0 * std::f32::consts::PI * f / SR;
        air += ((rng.next_f32() * 2.0 - 1.0) - air) * 0.5;
        let gap = if (0.13..0.16).contains(&t) { 0.25 } else { 1.0 };
        let e = (t / 0.015).min(1.0) * ((0.52 - t) / 0.12).clamp(0.0, 1.0) * gap;
        *out = (ph.sin() + air * 0.35) * e * 3000.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// A web leaving the wrist and landing: a spit of air that rises, then the
/// wet slap of it sticking.
fn thwip() -> Vec<i16> {
    let mut rng = Rng(0x7419);
    let mut buf = vec![0.0f32; (0.36 * SR) as usize];
    for k in 0..8 {
        let t = k as f32 / 8.0;
        band(&mut buf, 0.012 * k as f32, 900.0 + 3200.0 * t, 3.0, 0.7 * (1.0 - 0.4 * t), 0.004, 0.012, &mut rng);
    }
    band(&mut buf, 0.13, 700.0, 1.2, 0.9, 0.001, 0.02, &mut rng);
    band(&mut buf, 0.13, 2200.0, 1.5, 0.5, 0.0005, 0.006, &mut rng);
    render(buf, 10_000.0)
}

/// The match decided, on the koto and the flute: a run up to a held chord
/// for the winner, three notes falling onto the drum for the loser.
fn verdict(won: bool) -> Vec<i16> {
    let mut rng = Rng(if won { 0x0BA1 } else { 0x10B5 });
    let mut buf = vec![0.0f32; ms_to_samples(if won { 2300.0 } else { 2100.0 })];
    let root = 220.0;
    let at = |ms: f32| ms_to_samples(ms);
    if won {
        for (i, d) in [0, 2, 3, 4].into_iter().enumerate() {
            pluck(&mut buf, at(95.0 * i as f32), degree(&YO, root, d), 500.0, 0.8, &mut rng);
        }
        for (d, g) in [(0, 0.8f32), (2, 0.7), (3, 0.7), (5, 0.9)] {
            pluck(&mut buf, at(420.0), degree(&YO, root, d), 1700.0, g, &mut rng);
        }
        taiko(&mut buf, at(420.0), 96.0, 1.0, &mut rng);
        flute(&mut buf, at(520.0), degree(&YO, root, 5) * 2.0, 1500.0, 0.9, &mut rng);
    } else {
        for (i, d) in [4, 3, 1].into_iter().enumerate() {
            pluck(&mut buf, at(330.0 * i as f32), degree(&HIRAJOSHI, root, d), 900.0, 0.8, &mut rng);
        }
        pluck(&mut buf, at(990.0), degree(&HIRAJOSHI, root, 0) * 0.5, 1000.0, 0.9, &mut rng);
        taiko(&mut buf, at(990.0), 80.0, 0.9, &mut rng);
        flute(&mut buf, at(990.0), degree(&HIRAJOSHI, root, 0), 1000.0, 0.6, &mut rng);
    }
    fade_out(&mut buf, ms_to_samples(120.0));
    warm(&mut buf);
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// The giant coming down: a roar from the chest, low and long, with the
/// breath of it.
fn bellow() -> Vec<i16> {
    let mut rng = Rng(0x6A44);
    let mut buf = vec![0.0f32; (0.8 * SR) as usize];
    voice(&mut buf, 0.0, 0.7, 96.0, 70.0, AH, 0.25, 1.3, &mut rng);
    voice(&mut buf, 0.01, 0.66, 49.0, 36.0, OO, 0.15, 0.9, &mut rng);
    band(&mut buf, 0.0, 300.0, 0.8, 0.25, 0.05, 0.12, &mut rng);
    low_pass(&mut buf, 2200.0);
    render(buf, 13_000.0)
}

/// A swing with a body's whole weight behind it: lower and longer than the
/// ordinary one, and it ends in a clap of air.
fn heavy_swing() -> Vec<i16> {
    let mut rng = Rng(0x5E11);
    let mut buf = vec![0.0f32; (0.42 * SR) as usize];
    for k in 0..10 {
        let t = k as f32 / 10.0;
        let up = 1.0 - (2.0 * t - 1.0).abs();
        band(&mut buf, 0.022 * k as f32, 260.0 + 900.0 * up, 1.6, 0.6 + 0.6 * up, 0.012, 0.03, &mut rng);
    }
    band(&mut buf, 0.2, 110.0, 1.0, 1.4, 0.003, 0.05, &mut rng);
    render(buf, 11_000.0)
}

/// The round called: three strokes on the big drum, closing up, and a rim
/// shot to finish.
fn drum_call() -> Vec<i16> {
    let mut rng = Rng(0xD0D0);
    let mut buf = vec![0.0f32; ms_to_samples(1100.0)];
    for (ms, f, g) in [(0.0f32, 92.0f32, 0.9f32), (300.0, 98.0, 0.95), (520.0, 104.0, 1.0)] {
        taiko(&mut buf, ms_to_samples(ms), f, g, &mut rng);
    }
    rim(&mut buf, ms_to_samples(680.0), 0.9, &mut rng);
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// A barrel going: staves cracking one after another over the knock of the
/// blow, and the hoops ringing on the boards.
fn smash() -> Vec<i16> {
    let mut rng = Rng(0x5A4A);
    let mut buf = vec![0.0f32; (0.5 * SR) as usize];
    knock(&mut buf, 0.0, 170.0, 1.4, 0.02);
    band(&mut buf, 0.0, 420.0, 1.0, 1.6, 0.001, 0.03, &mut rng);
    for k in 0..6 {
        let at = 0.012 + 0.022 * k as f32;
        band(&mut buf, at, 1100.0 + 380.0 * ((k * 5) % 4) as f32, 2.2, 0.9 - 0.1 * k as f32, 0.0004, 0.006, &mut rng);
    }
    for (at, f) in [(0.16f32, 1900.0f32), (0.24, 2300.0)] {
        knock(&mut buf, at, f, 0.25, 0.03);
    }
    render(buf, 8_000.0)
}

/// Birds round the head: four chirps, each a quick slide up.
fn tweet() -> Vec<i16> {
    let n = ms_to_samples(520.0);
    let mut buf = vec![0.0f32; n];
    let mut ph = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let k = (t / 0.13).fract();
        let f = 1500.0 + 700.0 * k + 120.0 * ((t / 0.13).floor() % 2.0);
        ph += 2.0 * std::f32::consts::PI * f / SR;
        let e = (k / 0.1).min(1.0) * (1.0 - k).powf(1.5) * (1.0 - t / 0.52);
        *out = ph.sin() * e * 3900.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// A wood block: the menu moving a step.
fn tick() -> Vec<i16> {
    let mut rng = Rng(0x71C4);
    let mut buf = vec![0.0f32; (0.09 * SR) as usize];
    knock(&mut buf, 0.0, 940.0, 1.0, 0.007);
    knock(&mut buf, 0.0, 1430.0, 0.4, 0.004);
    band(&mut buf, 0.0, 2600.0, 1.5, 0.3, 0.0002, 0.002, &mut rng);
    render(buf, 9_000.0)
}

/// A gong: the mallet's thump, then partials that do not line up swelling
/// out of it and beating against each other as they die.
fn gong() -> Vec<i16> {
    let n = ms_to_samples(1500.0);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(0x6046);
    for (freq, amp, tau) in [
        (110.0f32, 1.0f32, 0.55f32),
        (164.0, 0.7, 0.5),
        (233.0, 0.55, 0.42),
        (331.0, 0.4, 0.34),
        (447.0, 0.25, 0.26),
        (452.0, 0.2, 0.26),
    ] {
        for (i, out) in buf.iter_mut().enumerate() {
            let t = i as f32 / SR;
            // The shimmer blooms a moment after the stroke.
            let swell = (t / (0.02 + 60.0 / freq / 100.0)).min(1.0);
            *out += (2.0 * std::f32::consts::PI * freq * t).sin() * (-t / tau).exp() * swell * amp * 3200.0;
        }
    }
    let mut thump = vec![0.0f32; n];
    band(&mut thump, 0.0, 140.0, 1.0, 1.0, 0.001, 0.02, &mut rng);
    for (out, x) in buf.iter_mut().zip(&thump) {
        *out += x * 5000.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// A heart going hard: two low thuds and a rest, to loop while a fighter is
/// nearly out.
fn heartbeat() -> Vec<i16> {
    let mut rng = Rng(0x4EA7);
    let mut buf = vec![0.0f32; (0.86 * SR) as usize];
    for (at, amp) in [(0.0f32, 1.0f32), (0.21, 0.7)] {
        knock(&mut buf, at, 58.0, amp * 1.6, 0.05);
        band(&mut buf, at, 90.0, 1.2, amp * 1.2, 0.004, 0.03, &mut rng);
    }
    low_pass(&mut buf, 900.0);
    limit_pcm16(&buf, 3300.0)
}

/// The effects, as `sounds/NAME.wav`.
pub fn sounds() -> Vec<Asset> {
    vec![
        // Three light hits, the thud climbing a combo step each, so a string
        // sounds like it is going somewhere.
        ("sounds/hit_light.wav", encode_pcm16_half(&hit(300.0, 0.15, 0x11))),
        ("sounds/hit_light2.wav", encode_pcm16_half(&hit(380.0, 0.2, 0x1b))),
        ("sounds/hit_light3.wav", encode_pcm16_half(&hit(470.0, 0.25, 0x2f))),
        // A second take of each, a shade lower and from another seed: the
        // same blow twice running should not be the same recording twice.
        ("sounds/hit_light_b.wav", encode_pcm16_half(&hit(284.0, 0.18, 0x51))),
        ("sounds/hit_light2_b.wav", encode_pcm16_half(&hit(360.0, 0.23, 0x5b))),
        ("sounds/hit_light3_b.wav", encode_pcm16_half(&hit(446.0, 0.28, 0x6f))),
        ("sounds/hit_heavy_b.wav", encode_pcm16_half(&hit(216.0, 1.0, 0x62))),
        ("sounds/hit_heavy.wav", encode_pcm16_half(&hit(230.0, 1.0, 0x22))),
        // Kicks: shin and instep, two takes light and two heavy.
        ("sounds/kick_light.wav", encode_pcm16_half(&blow(260.0, 0.3, true, 0x91))),
        ("sounds/kick_light_b.wav", encode_pcm16_half(&blow(240.0, 0.35, true, 0x93))),
        ("sounds/kick_heavy.wav", encode_pcm16_half(&blow(190.0, 1.0, true, 0x95))),
        ("sounds/kick_heavy_b.wav", encode_pcm16_half(&blow(176.0, 1.0, true, 0x97))),
        // A knockdown: a body on the boards, lower and longer than any hit.
        ("sounds/crunch.wav", encode_pcm16_half(&crunch())),
        ("sounds/land.wav", encode_pcm16_mono(&land(0x71))),
        ("sounds/whoosh.wav", encode_pcm16_half(&whoosh(150.0, 0.7, 0x33))),
        ("sounds/gi.wav", encode_pcm16_mono(&gi_snap(0x6C1D))),
        ("sounds/block.wav", encode_pcm16_mono(&block_sfx())),
        ("sounds/block_b.wav", encode_pcm16_mono(&block_take(0x51B3))),
        ("sounds/step.wav", encode_pcm16_half(&step(0x57E1))),
        ("sounds/step_b.wav", encode_pcm16_half(&step(0x57E9))),
        ("sounds/bell.wav", encode_pcm16_half(&bell())),
        ("sounds/ko.wav", encode_pcm16_half(&ko())),
        ("sounds/projectile.wav", encode_pcm16_half(&projectile())),
        // Stored at half the rate: everything whose energy above 10 kHz is
        // 33 dB or more under the rest (render() rolls off from 5 kHz). The
        // snaps, whistles and swishes keep the full rate.
        ("sounds/laser.wav", encode_pcm16_half(&laser())),
        ("sounds/quake.wav", encode_pcm16_music(&quake())),
        ("sounds/parry.wav", encode_pcm16_half(&parry())),
        ("sounds/counter.wav", encode_pcm16_half(&counter())),
        ("sounds/fruit.wav", encode_pcm16_mono(&run_up(&[0, 2, 3, 5], 220.0, 55.0, 420.0, 0xF2))),
        ("sounds/tick.wav", encode_pcm16_half(&tick())),
        ("sounds/gong.wav", encode_pcm16_music(&gong())),
        ("sounds/bellow.wav", encode_pcm16_music(&bellow())),
        ("sounds/swing.wav", encode_pcm16_mono(&heavy_swing())),
        ("sounds/drums.wav", encode_pcm16_music(&drum_call())),
        ("sounds/smash.wav", encode_pcm16_half(&smash())),
        ("sounds/tweet.wav", encode_pcm16_half(&tweet())),
        ("sounds/meow.wav", encode_pcm16_half(&trimmed(meow()))),
        ("sounds/heart.wav", encode_pcm16_music(&heartbeat())),
        ("sounds/whistle.wav", encode_pcm16_mono(&whistle())),
        ("sounds/thwip.wav", encode_pcm16_mono(&thwip())),
        ("sounds/win.wav", encode_pcm16_music(&verdict(true))),
        ("sounds/lose.wav", encode_pcm16_music(&verdict(false))),
        // The fighters' voices, four throats: two grunts, a cry, three shouts and a knockout each.
        ("sounds/hurt0_0.wav", encode_pcm16_half(&trimmed(hurt(THROATS[0], 0, false)))),
        ("sounds/hurt0_1.wav", encode_pcm16_half(&trimmed(hurt(THROATS[0], 1, false)))),
        ("sounds/hurt0_2.wav", encode_pcm16_half(&trimmed(hurt(THROATS[0], 0, true)))),
        ("sounds/hurt1_0.wav", encode_pcm16_half(&trimmed(hurt(THROATS[1], 0, false)))),
        ("sounds/hurt1_1.wav", encode_pcm16_half(&trimmed(hurt(THROATS[1], 2, false)))),
        ("sounds/hurt1_2.wav", encode_pcm16_half(&trimmed(hurt(THROATS[1], 0, true)))),
        ("sounds/hurt2_0.wav", encode_pcm16_half(&trimmed(hurt(THROATS[2], 1, false)))),
        ("sounds/hurt2_1.wav", encode_pcm16_half(&trimmed(hurt(THROATS[2], 2, false)))),
        ("sounds/hurt2_2.wav", encode_pcm16_half(&trimmed(hurt(THROATS[2], 0, true)))),
        ("sounds/hurt3_0.wav", encode_pcm16_half(&trimmed(hurt(THROATS[3], 0, false)))),
        ("sounds/hurt3_1.wav", encode_pcm16_half(&trimmed(hurt(THROATS[3], 1, false)))),
        ("sounds/hurt3_2.wav", encode_pcm16_half(&trimmed(hurt(THROATS[3], 0, true)))),
        ("sounds/kiai0_0.wav", encode_pcm16_half(&trimmed(kiai(THROATS[0], 0)))),
        ("sounds/kiai0_1.wav", encode_pcm16_half(&trimmed(kiai(THROATS[0], 1)))),
        ("sounds/kiai0_2.wav", encode_pcm16_half(&trimmed(kiai(THROATS[0], 2)))),
        ("sounds/kiai1_0.wav", encode_pcm16_half(&trimmed(kiai(THROATS[1], 0)))),
        ("sounds/kiai1_1.wav", encode_pcm16_half(&trimmed(kiai(THROATS[1], 1)))),
        ("sounds/kiai1_2.wav", encode_pcm16_half(&trimmed(kiai(THROATS[1], 2)))),
        ("sounds/kiai2_0.wav", encode_pcm16_half(&trimmed(kiai(THROATS[2], 0)))),
        ("sounds/kiai2_1.wav", encode_pcm16_half(&trimmed(kiai(THROATS[2], 1)))),
        ("sounds/kiai2_2.wav", encode_pcm16_half(&trimmed(kiai(THROATS[2], 2)))),
        ("sounds/kiai3_0.wav", encode_pcm16_half(&trimmed(kiai(THROATS[3], 0)))),
        ("sounds/kiai3_1.wav", encode_pcm16_half(&trimmed(kiai(THROATS[3], 1)))),
        ("sounds/kiai3_2.wav", encode_pcm16_half(&trimmed(kiai(THROATS[3], 2)))),
        ("sounds/cry0.wav", encode_pcm16_half(&trimmed(ko_cry(THROATS[0])))),
        ("sounds/cry5.wav", encode_pcm16_half(&trimmed(ko_cry(THROATS[5])))),
        ("sounds/kiai5_0.wav", encode_pcm16_half(&trimmed(kiai(THROATS[5], 0)))),
        ("sounds/kiai5_1.wav", encode_pcm16_half(&trimmed(kiai(THROATS[5], 1)))),
        ("sounds/kiai5_2.wav", encode_pcm16_half(&trimmed(kiai(THROATS[5], 2)))),
        ("sounds/hurt5_0.wav", encode_pcm16_half(&trimmed(hurt(THROATS[5], 0, false)))),
        ("sounds/hurt5_1.wav", encode_pcm16_half(&trimmed(hurt(THROATS[5], 1, false)))),
        ("sounds/hurt5_2.wav", encode_pcm16_half(&trimmed(hurt(THROATS[5], 0, true)))),
        ("sounds/cry1.wav", encode_pcm16_half(&trimmed(ko_cry(THROATS[1])))),
        ("sounds/cry2.wav", encode_pcm16_half(&trimmed(ko_cry(THROATS[2])))),
        ("sounds/cry3.wav", encode_pcm16_half(&trimmed(ko_cry(THROATS[3])))),
        ("sounds/hurt4_0.wav", encode_pcm16_half(&trimmed(yelp(THROATS[4], 0, false)))),
        ("sounds/hurt4_1.wav", encode_pcm16_half(&trimmed(yelp(THROATS[4], 1, false)))),
        ("sounds/hurt4_2.wav", encode_pcm16_half(&trimmed(yelp(THROATS[4], 0, true)))),
        ("sounds/kiai4_0.wav", encode_pcm16_half(&trimmed(keen(THROATS[4], 0)))),
        ("sounds/kiai4_1.wav", encode_pcm16_half(&trimmed(keen(THROATS[4], 1)))),
        ("sounds/kiai4_2.wav", encode_pcm16_half(&trimmed(keen(THROATS[4], 2)))),
        ("sounds/kiai4_3.wav", encode_pcm16_half(&trimmed(keen(THROATS[4], 3)))),
        ("sounds/kiai4_4.wav", encode_pcm16_half(&trimmed(keen(THROATS[4], 4)))),
        ("sounds/cry4.wav", encode_pcm16_half(&trimmed(keen_down(THROATS[4])))),
        // The crowd, and the four stages' ambience under the music.
        ("sounds/cheer.wav", crowd_wav(false)),
        ("sounds/roar.wav", crowd_wav(true)),
        ("sounds/ambience_crowd.wav", ambience_wav(0)),
        ("sounds/ambience_wind.wav", ambience_wav(1)),
        ("sounds/ambience_crickets.wav", ambience_wav(2)),
        ("sounds/ambience_city.wav", ambience_wav(3)),
    ]
}

/// Original walkout: swung breakbeat, sub bass and a sparse, minor-key piano hook.
pub fn walkout_wav() -> Vec<u8> {
    const BPM: f32 = 96.0;
    const BARS: usize = 16;
    const HOOK: [(usize, i32); 8] = [(0, 62), (3, 69), (7, 65), (10, 67),
        (16, 60), (22, 65), (26, 64), (30, 57)];
    let step = (SR * 60.0 / BPM / 4.0) as usize;
    let body = step * BARS * 16;
    let mut buf = vec![0.0; body + ms_to_samples(1800.0)];
    let mut rng = Rng(0x2026);
    let hz = |note: i32| 440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0);
    for tick in 0..BARS * 16 {
        let beat = tick % 16;
        let swing = if tick % 2 == 1 { step / 7 } else { 0 };
        let off = tick * step + swing;
        if matches!(beat, 0 | 6 | 10) || beat == 15 && tick / 16 % 2 == 1 {
            taiko(&mut buf, off, 75.0, 1.15, &mut rng);
        }
        if matches!(beat, 4 | 12) {
            rim(&mut buf, off, 0.9, &mut rng);
            let n = ms_to_samples(135.0);
            for i in 0..n {
                let t = i as f32 / n as f32;
                buf[off + i] += (rng.next_f32() * 2.0 - 1.0) * (-t * 6.0).exp() * 3700.0;
            }
        }
        if tick % 2 == 0 {
            let n = ms_to_samples(26.0);
            for i in 0..n {
                let t = i as f32 / n as f32;
                buf[off + i] += (rng.next_f32() * 2.0 - 1.0) * (-t * 9.0).exp() * 650.0;
            }
        }
        if matches!(beat, 0 | 7 | 10) {
            let note = [38, 38, 36, 41][tick / 16 % 4];
            let freq = hz(note);
            let n = step * if beat == 0 { 6 } else { 3 };
            for i in 0..n {
                let phase = std::f32::consts::TAU * freq * i as f32 / SR;
                let attack = (i as f32 / (SR * 0.008)).min(1.0);
                let decay = (1.0 - i as f32 / n as f32).powf(1.4);
                buf[off + i] += (phase.sin() + 0.18 * (phase * 2.0).sin()) * attack * decay * 5400.0;
            }
        }
        for &(at, note) in &HOOK {
            if tick % 32 != at { continue; }
            pluck(&mut buf, off, hz(note), 520.0, 0.72, &mut rng);
            pluck(&mut buf, off + step * 3, hz(note), 420.0, 0.15, &mut rng);
        }
    }
    encode_pcm16_music(&soft_limit_to_pcm16(&fold_tail(buf, body), MIX_KNEE))
}

/// The music: each tune as a WAV to play.
pub fn music() -> Vec<(String, Vec<u8>)> {
    blip::wav::tunes(&[
        ("DOCK", &|| theme_wav(0)),
        ("TEMPLE", &|| theme_wav(1)),
        ("SELECT", &|| theme_wav(2)),
        ("AIRBASE", &|| theme_wav(3)),
        ("BATHHOUSE", &|| theme_wav(4)),
        ("VILLAGE", &|| theme_wav(5)),
        ("FORTRESS", &|| theme_wav(6)),
        ("ROOFTOP", &|| theme_wav(7)),
        ("HOOD", &|| theme_wav(8)),
        ("WALKOUT", &walkout_wav),
        ("PAGODA", &|| theme_wav(9)),
        ("COLOSSEUM", &|| theme_wav(10)),
    ])
}
