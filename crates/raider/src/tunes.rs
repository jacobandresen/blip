// Compiled by build.rs, and by the game for its tests and its few shared numbers.
#![allow(dead_code)]

use std::f32::consts::PI;

use blip::wav::{
    encode_pcm16_half, encode_pcm16_mono, encode_pcm16_music, halved, low_pass, mix_into, mix_into_f32,
    ms_to_samples, soft_limit_to_pcm16, tame, warm, Rng, MIX_KNEE, SAMPLE_RATE,
};
use blip::Asset;
use blip::xm::{heard, heard_hz, tempo, Part, HAT, KICK, SNARE};

// Rock music uses detuned saws or saw/square guitars, tanh drive and a low-pass cabinet filter.
// Voices mix in f32 and are limited once at the end.

/// Rock kick: tight and dry with a hard beater click that cuts through
/// distorted guitar; much less boom than `techno::kick`.
fn rock_kick(buf: &mut [f32], off: usize, vol: f32) {
    heard(off, KICK, Part::Drums);
    let sr = SAMPLE_RATE as f32;
    let n = (sr * 0.11) as usize;
    let click_n = (sr * 0.004) as usize;
    for i in 0..n {
        if off + i >= buf.len() {
            break;
        }
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(2.1);
        let freq = 48.0 + 95.0 * (-t / 0.028).exp(); // 143 Hz -> 48 Hz, fast
        let body = (2.0 * PI * freq * t).sin();
        let click = if i < click_n {
            let ce = 1.0 - i as f32 / click_n as f32;
            let cn = ((i as u32).wrapping_mul(2_654_435_761) as f32 / u32::MAX as f32) * 2.0 - 1.0;
            cn * ce * 0.8
        } else {
            0.0
        };
        mix_into_f32(
            buf,
            off + i,
            (body * 1.5 + click).tanh() * e * vol * 20_000.0,
        );
    }
}

/// Rock snare — a bright noise crack over two tuned shell modes, with a
/// real ringing decay (unlike the dry, choked `march_snare`): this is the
/// backbeat the whole groove leans on, so it needs to carry.
fn rock_snare(buf: &mut [f32], off: usize, rng: &mut Rng, vol: f32) {
    heard(off, SNARE, Part::Drums);
    let sr = SAMPLE_RATE as f32;
    let n = (sr * 0.15) as usize;
    let mut prev = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() {
            break;
        }
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.5);
        let white = rng.next_f32() * 2.0 - 1.0;
        let hp = white - prev; // crude 1st-order highpass -> "snap"
        prev = white;
        let shell = ((2.0 * PI * 185.0 * t).sin() + 0.6 * (2.0 * PI * 331.0 * t).sin()) * 0.3;
        mix_into_f32(
            buf,
            off + i,
            ((hp * 0.9 + shell) * e).tanh() * vol * 14_000.0,
        );
    }
}

/// Hi-hat / ride tick — bright filtered noise. `open` swaps the tight
/// closed-hat blip for a longer, washier decay.
fn rock_hat(buf: &mut [f32], off: usize, rng: &mut Rng, vol: f32, open: bool) {
    heard(off, HAT, Part::Drums);
    let n = (SAMPLE_RATE as f32 * if open { 0.19 } else { 0.038 }) as usize;
    let mut prev = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() {
            break;
        }
        let e = (1.0 - i as f32 / n as f32).powf(if open { 1.4 } else { 3.2 });
        let white = rng.next_f32() * 2.0 - 1.0;
        let hp = white - prev;
        prev = white;
        mix_into_f32(buf, off + i, hp * e * vol * 7_000.0);
    }
}

/// Crash cymbal — a long noise wash with a couple of inharmonic partials
/// for shimmer. Marks the top of a section.
fn crash(buf: &mut [f32], off: usize, rng: &mut Rng, vol: f32) {
    heard(off, HAT, Part::Drums);
    let sr = SAMPLE_RATE as f32;
    let n = (sr * 0.85) as usize;
    let mut prev = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() {
            break;
        }
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.7);
        let white = rng.next_f32() * 2.0 - 1.0;
        let hp = white - 0.7 * prev;
        prev = white;
        let shimmer = 0.15 * ((2.0 * PI * 5_300.0 * t).sin() + (2.0 * PI * 7_100.0 * t).sin());
        mix_into_f32(buf, off + i, (hp * 0.85 + shimmer) * e * vol * 6_500.0);
    }
}

/// Distorted root/fifth/octave power chord; `palm` selects muted or open articulation.
fn power_chord(buf: &mut [f32], off: usize, root: f32, ms: f32, vol: f32, palm: bool) {
    heard_hz(off, root, Part::Harmony);
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    if n == 0 {
        return;
    }
    let att = (sr * 0.0018) as usize + 1;
    let decay_pow = if palm { 2.6 } else { 0.7 };
    let drive = if palm { 7.5 } else { 11.0 };
    let cutoff = if palm { 2_500.0 } else { 3_400.0 };
    let alpha = 1.0 - (-2.0 * PI * cutoff / sr).exp();
    // root, fifth (3:2), octave — the fifth kept a touch quieter so the
    // chord has a root rather than a hollow parallel-fifths drone.
    const IVL: [(f32, f32); 3] = [(1.0, 1.0), (1.5, 0.7), (2.0, 0.9)];
    const DETUNE: f32 = 0.004;
    let mut ph = [0.0f32; 6];
    let mut lp = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() {
            break;
        }
        let a = if i < att {
            i as f32 / att as f32
        } else {
            (1.0 - (i - att) as f32 / (n - att).max(1) as f32).powf(decay_pow)
        };
        let mut raw = 0.0f32;
        let mut k = 0;
        for &(mult, w) in &IVL {
            for d in [-1.0f32, 1.0] {
                let f = root * mult * (1.0 + d * DETUNE);
                ph[k] += f / sr;
                ph[k] -= ph[k].floor();
                raw += (2.0 * ph[k] - 1.0) * w;
                k += 1;
            }
        }
        let driven = (raw / 5.0 * drive).tanh();
        lp += alpha * (driven - lp);
        mix_into_f32(buf, off + i, lp * a * vol * 15_000.0);
    }
}

/// Driven saw/square solo with delayed vibrato and optional attack bend in semitones.
fn lead_guitar(buf: &mut [f32], off: usize, freq: f32, ms: f32, vol: f32, bend: f32) {
    heard_hz(off, freq, Part::Tune);
    let mut freq = freq;
    while freq > 660.0 {
        freq *= 0.5;
    } // an octave down until it sings, not screams
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    if n == 0 {
        return;
    }
    let att = (sr * 0.006) as usize + 1;
    let bend_from = 2f32.powf(-bend / 12.0);
    let bend_n = ((sr * 0.065) as usize).max(1);
    let alpha = 1.0 - (-2.0 * PI * 3_200.0 / sr).exp();
    let mut ph = 0.0f32;
    let mut lp = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() {
            break;
        }
        let t = i as f32 / sr;
        let prog = i as f32 / n as f32;
        let a = if i < att {
            i as f32 / att as f32
        } else {
            let body = (1.0 - prog).powf(0.45);
            let bloom = 1.0 + 0.55 * prog.powf(3.0);
            (body * bloom).min(1.35)
        };
        let vib_on = ((t - 0.11) / 0.06).clamp(0.0, 1.0);
        let vib = 1.0 + vib_on * 0.014 * (2.0 * PI * 5.7 * t).sin();
        let b = if i < bend_n {
            let f = i as f32 / bend_n as f32;
            bend_from + (1.0 - bend_from) * (f * f) // ease-in, like a finger push
        } else {
            1.0
        };
        let f = freq * vib * b;
        ph += f / sr;
        ph -= ph.floor();
        let saw = 2.0 * ph - 1.0;
        let sq = if ph < 0.5 { 1.0 } else { -1.0 };
        let driven = ((saw * 0.7 + sq * 0.3) * 6.0).tanh();
        lp += alpha * (driven - lp);
        let sing = 0.12 * (2.0 * PI * 2.0 * f * t).sin(); // octave-up edge
        mix_into_f32(buf, off + i, (lp + sing) * a * vol * 12_000.0);
    }
}

/// Picked electric bass — a mildly overdriven saw with a reinforcing sine
/// at the fundamental, locked to the rhythm-guitar root. Medium decay, a
/// little pick grit; sits under the guitars without fighting them.
fn bass_guitar(buf: &mut [f32], off: usize, freq: f32, ms: f32, vol: f32) {
    heard_hz(off, freq, Part::Bass);
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    if n == 0 {
        return;
    }
    let att = (sr * 0.003) as usize + 1;
    let alpha = 1.0 - (-2.0 * PI * 1_700.0 / sr).exp();
    let mut ph = 0.0f32;
    let mut lp = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() {
            break;
        }
        let t = i as f32 / sr;
        let a = if i < att {
            i as f32 / att as f32
        } else {
            (1.0 - (i - att) as f32 / (n - att).max(1) as f32).powf(1.4)
        };
        ph += freq / sr;
        ph -= ph.floor();
        let driven = ((2.0 * ph - 1.0) * 2.3).tanh();
        lp += alpha * (driven - lp);
        let sub = (2.0 * PI * freq * t).sin();
        mix_into_f32(buf, off + i, (lp * 0.8 + sub * 0.5) * a * vol * 16_000.0);
    }
}

/// Guitar dive bomb — the whammy-bar drop that ends a solo: pitch craters
/// from `freq` toward nothing while the note blooms once and then chokes.
fn dive_bomb(buf: &mut [f32], off: usize, freq: f32, ms: f32, vol: f32) {
    heard_hz(off, freq, Part::Tune);
    let (freq, vol) = (tame(freq), vol * 0.6);
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    if n == 0 {
        return;
    }
    let alpha = 1.0 - (-2.0 * PI * 2_600.0 / sr).exp();
    let mut ph = 0.0f32;
    let mut lp = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() {
            break;
        }
        let prog = i as f32 / n as f32;
        let f = (freq * (1.0 - prog).powf(2.2)).max(18.0);
        let e = (1.0 - prog).powf(1.3) * (1.0 + 0.4 * prog);
        ph += f / sr;
        ph -= ph.floor();
        let saw = 2.0 * ph - 1.0;
        let sq = if ph < 0.5 { 1.0 } else { -1.0 };
        let driven = ((saw * 0.6 + sq * 0.4) * 7.0).tanh();
        lp += alpha * (driven - lp);
        mix_into_f32(buf, off + i, lp * e * vol * 12_000.0);
    }
}

/// Core rock beat step; `busy` doubles eighth-note hats to sixteenths for the solo.
fn rock_beat_step(buf: &mut [f32], off: usize, pos: usize, rng: &mut Rng, busy: bool) {
    const KICK: [bool; 16] = [
        true, false, false, false, false, false, true, false, true, false, false, false, false,
        false, true, false,
    ];
    if KICK[pos] {
        rock_kick(buf, off, 0.9);
    }
    if pos == 4 || pos == 12 {
        rock_snare(buf, off, rng, 0.6);
    }
    if busy || pos.is_multiple_of(2) {
        rock_hat(
            buf,
            off,
            rng,
            if pos.is_multiple_of(4) { 0.22 } else { 0.16 },
            false,
        );
    }
    if pos == 14 {
        rock_hat(buf, off, rng, 0.16, true);
    }
}

/// A one-bar snare fill: rising 16th-note hits across the back half of the
/// bar, capped with a crash on the downbeat that follows (left to the
/// caller). The march form's crescendo roll, re-scored for a kit.
fn drum_fill(buf: &mut [f32], bar_off: usize, step_samples: usize, rng: &mut Rng) {
    for h in 0..8 {
        let off = bar_off + step_samples * (8 + h);
        rock_snare(buf, off, rng, 0.22 + 0.055 * h as f32);
    }
}

/// Main Raider theme in E minor: intro, verse, chorus, solo, chorus.
/// The fixed gallop hook anchors the verse; the chorus moves Em–C–G–D.
pub fn loop1_wav() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let bpm = 150.0_f32;
    let steps_per_bar = 16usize;
    tempo(bpm);
    let step_ms = 60_000.0 / bpm / 4.0; // 16th note = 100 ms
    let step_samples = (sr * step_ms / 1000.0) as usize;

    // intro(1) + verse(8) + chorus(4) + solo(8) + chorus(4)
    const INTRO: usize = 1;
    const VERSE: usize = INTRO + 8;
    const CHORUS: usize = VERSE + 4;
    const SOLO: usize = CHORUS + 8;
    const BARS: usize = SOLO + 4; // 25

    let total_steps = BARS * steps_per_bar;
    let total = step_samples * total_steps + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0x5217_9111);

    // E minor. Low roots for the rhythm guitar and bass.
    const E2: f32 = 82.41;
    const G2: f32 = 98.00;
    const A2: f32 = 110.00;
    const B2: f32 = 123.47;
    const C3: f32 = 130.81;
    const D3: f32 = 146.83;

    // The verse hook: a 2-bar phrase, (step-in-phrase, root, dur-in-steps,
    // palm-muted?). Gallop of open-E chugs, then the triad answer.
    const RIFF: [(usize, f32, f32, bool); 20] = [
        (0, E2, 1.3, true),
        (2, E2, 1.3, true),
        (3, E2, 1.3, true),
        (4, E2, 1.3, true),
        (6, E2, 1.3, true),
        (7, E2, 1.3, true),
        (8, E2, 1.3, true),
        (10, E2, 1.3, true),
        (11, E2, 1.3, true),
        (12, G2, 2.0, false),
        (14, A2, 2.0, false),
        (16, E2, 1.3, true),
        (18, E2, 1.3, true),
        (19, E2, 1.3, true),
        (20, E2, 1.3, true),
        (22, E2, 1.3, true),
        (23, E2, 1.3, true),
        (24, B2, 2.0, false),
        (26, A2, 2.0, false),
        (30, E2, 4.0, false),
    ];
    // Bass under the verse — root notes, a little detached.
    const RIFF_BASS: [(usize, f32); 11] = [
        (0, E2),
        (4, E2),
        (8, E2),
        (12, G2),
        (14, A2),
        (16, E2),
        (20, E2),
        (24, B2),
        (26, A2),
        (28, G2),
        (30, E2),
    ];

    // Chorus changes, one chord per bar, plus the lead line sitting on top.
    const CHORDS: [f32; 4] = [E2, C3, G2, D3]; // Em - C - G - D
    const CH_LEAD: [(f32, f32); 4] = [
        (493.88, 0.0), // B4
        (523.25, 0.0), // C5
        (587.33, 0.0), // D5
        (493.88, 2.0), // B4, bent up
    ];

    // The solo, as (absolute-step-from-solo-start, freq, dur-in-steps, bend).
    const LEAD: [(usize, f32, f32, f32); 34] = [
        // bar 0 — pickup, then bend up a tone into a held D5
        (0, 440.00, 2.0, 0.0),
        (2, 493.88, 2.0, 0.0),
        (4, 587.33, 12.0, 2.0),
        // bar 1 — pentatonic run down
        (16, 659.25, 2.0, 0.0),
        (18, 587.33, 2.0, 0.0),
        (20, 493.88, 2.0, 0.0),
        (22, 440.00, 2.0, 0.0),
        (24, 392.00, 2.0, 0.0),
        (26, 329.63, 6.0, 0.0),
        // bar 2 — call, ending on a bent E5
        (32, 493.88, 3.0, 0.0),
        (35, 587.33, 3.0, 0.0),
        (38, 659.25, 10.0, 2.0),
        // bar 3 — answer
        (48, 587.33, 3.0, 0.0),
        (51, 493.88, 3.0, 0.0),
        (54, 440.00, 3.0, 0.0),
        (57, 493.88, 7.0, 0.0),
        // bar 4 — fast run up the scale to a bent A5
        (64, 329.63, 1.0, 0.0),
        (65, 392.00, 1.0, 0.0),
        (66, 440.00, 1.0, 0.0),
        (67, 493.88, 1.0, 0.0),
        (68, 587.33, 1.0, 0.0),
        (69, 659.25, 1.0, 0.0),
        (70, 783.99, 1.0, 0.0),
        (71, 880.00, 8.0, 1.0),
        // bar 5 — rhythmic top-note phrase
        (80, 659.25, 2.0, 0.0),
        (82, 659.25, 2.0, 0.0),
        (84, 783.99, 2.0, 0.0),
        (86, 659.25, 2.0, 0.0),
        (88, 587.33, 2.0, 0.0),
        (90, 493.88, 6.0, 0.0),
        // bar 6 — climax: a huge held bent B5, vibrato and feedback bloom
        (96, 987.77, 16.0, 2.0),
        // bar 7 — resolve down (the dive bomb is placed separately)
        (112, 880.00, 2.0, 0.0),
        (114, 783.99, 2.0, 0.0),
        (116, 659.25, 2.0, 0.0),
    ];

    for step in 0..total_steps {
        let bar = step / steps_per_bar;
        let pos = step % steps_per_bar;
        let off = step * step_samples;
        let bar_off = bar * steps_per_bar * step_samples;

        let in_intro = bar < INTRO;
        let in_verse = (INTRO..VERSE).contains(&bar);
        let in_chorus = (VERSE..CHORUS).contains(&bar) || (SOLO..BARS).contains(&bar);
        let in_solo = (CHORUS..SOLO).contains(&bar);

        // Section-top crash.
        if pos == 0 && (bar == VERSE || bar == CHORUS || bar == SOLO || bar == BARS - 4) {
            crash(&mut buf, off, &mut rng, 0.5);
        }

        // ---- Drums ----
        if in_intro {
            // count-in fill only
            if pos >= 8 {
                rock_snare(&mut buf, off, &mut rng, 0.20 + 0.05 * (pos - 8) as f32);
            }
        } else {
            let last_of_section = (in_verse && bar == VERSE - 1)
                || (in_solo && bar == SOLO - 1)
                || (in_chorus && (bar == CHORUS - 1 || bar == BARS - 1));
            if last_of_section {
                if pos == 0 {
                    drum_fill(&mut buf, bar_off, step_samples, &mut rng);
                }
                // keep the kick pulse under the fill
                if pos == 0 || pos == 8 {
                    rock_kick(&mut buf, off, 0.85);
                }
            } else {
                rock_beat_step(&mut buf, off, pos, &mut rng, in_solo);
            }
        }

        // ---- Rhythm guitar + bass ----
        if in_intro {
            if pos == 0 {
                power_chord(&mut buf, off, E2, step_ms * 16.0, 0.42, false);
                bass_guitar(&mut buf, off, E2, step_ms * 14.0, 0.5);
            }
        } else if in_verse {
            let phase_step = ((bar - INTRO) % 2) * steps_per_bar + pos;
            for &(s, root, dur, palm) in &RIFF {
                if s == phase_step {
                    power_chord(
                        &mut buf,
                        off,
                        root,
                        step_ms * dur,
                        if palm { 0.5 } else { 0.44 },
                        palm,
                    );
                }
            }
            for &(s, root) in &RIFF_BASS {
                if s == phase_step {
                    bass_guitar(&mut buf, off, root, step_ms * 3.0, 0.5);
                }
            }
        } else if in_chorus {
            let ch = if bar < SOLO { bar - VERSE } else { bar - SOLO };
            let root = CHORDS[ch % 4];
            if pos == 0 {
                power_chord(&mut buf, off, root, step_ms * 15.5, 0.5, false);
            }
            if pos == 8 {
                power_chord(&mut buf, off, root, step_ms * 7.5, 0.42, false);
            }
            if pos.is_multiple_of(2) {
                bass_guitar(&mut buf, off, root, step_ms * 1.7, 0.5);
            }
            if pos == 0 {
                let (f, bnd) = CH_LEAD[ch % 4];
                lead_guitar(&mut buf, off, f, step_ms * 12.0, 0.42, bnd);
            }
        } else if in_solo {
            // gallop under the solo; last four bars walk the chorus changes
            let sbar = bar - CHORUS;
            let root = if sbar < 4 { E2 } else { CHORDS[(sbar - 4) % 4] };
            const GALLOP: [usize; 12] = [0, 2, 3, 4, 6, 7, 8, 10, 11, 12, 14, 15];
            if GALLOP.contains(&pos) {
                power_chord(&mut buf, off, root, step_ms * 1.3, 0.42, true);
            }
            if pos == 0 || pos == 4 || pos == 8 || pos == 12 {
                bass_guitar(&mut buf, off, root, step_ms * 3.0, 0.48);
            }
        }

        // ---- Lead solo ----
        if in_solo {
            let solo_step = (bar - CHORUS) * steps_per_bar + pos;
            for &(s, f, dur, bend) in &LEAD {
                if s == solo_step {
                    lead_guitar(&mut buf, off, f, step_ms * dur, 0.6, bend);
                }
            }
            if solo_step == 120 {
                dive_bomb(&mut buf, off, 659.25, step_ms * 7.0, 0.55);
            }
        }
    }

    // A soft sustained low-E drone under the whole piece, glueing the loop.
    for bar in 0..BARS {
        let bar_off = bar * steps_per_bar * step_samples;
        power_chord(
            &mut buf,
            bar_off,
            E2,
            step_ms * steps_per_bar as f32 * 1.02,
            0.06,
            false,
        );
    }

    // a rock band, not a lullaby: the guitars keep their bite
    low_pass(&mut buf, 6500.0);
    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// Second Raider loop: 176 BPM drop-D thrash in D minor, with a chromatic break and solo.
pub fn loop2_wav() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let bpm = 176.0_f32;
    let steps_per_bar = 16usize;
    tempo(bpm);
    let step_ms = 60_000.0 / bpm / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;

    const INTRO: usize = 1;
    const RIFF_A: usize = INTRO + 6;
    const SOLO: usize = RIFF_A + 6;
    const BARS: usize = SOLO + 3; // 16

    let total_steps = BARS * steps_per_bar;
    let total = step_samples * total_steps + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0x5217_9222);

    const D2: f32 = 73.42;
    const EB2: f32 = 77.78;
    const F2: f32 = 87.31;
    const C3: f32 = 130.81;

    // 1-bar drop-D riff: tremolo chug on the low D, then a
    // chromatic F–D–C–D–Eb–D breakdown accent.
    const RIFF: [(usize, f32, f32, bool); 15] = [
        (0, D2, 1.1, true),
        (1, D2, 1.1, true),
        (2, D2, 1.1, true),
        (3, D2, 1.1, true),
        (4, D2, 1.1, true),
        (5, D2, 1.1, true),
        (6, D2, 1.1, true),
        (7, D2, 1.1, true),
        (8, F2, 2.0, false),
        (10, D2, 1.1, true),
        (11, C3, 1.5, false),
        (12, D2, 1.1, true),
        (13, EB2, 1.3, false),
        (14, D2, 1.1, true),
        (15, D2, 1.1, true),
    ];

    // The shred solo, (absolute-step-from-solo-start, freq, dur-in-steps, bend).
    const LEAD: [(usize, f32, f32, f32); 28] = [
        // bar 0 — bend up into a held D5
        (0, 440.00, 2.0, 0.0),
        (2, 523.25, 2.0, 0.0),
        (4, 587.33, 10.0, 2.0),
        // bar 1 — fast run down D minor pentatonic
        (16, 587.33, 1.0, 0.0),
        (17, 523.25, 1.0, 0.0),
        (18, 440.00, 1.0, 0.0),
        (19, 392.00, 1.0, 0.0),
        (20, 349.23, 1.0, 0.0),
        (21, 293.66, 1.0, 0.0),
        (22, 349.23, 2.0, 0.0),
        (24, 293.66, 8.0, 0.0),
        // bar 2 — bend up into a held F5
        (32, 440.00, 2.0, 0.0),
        (34, 587.33, 2.0, 0.0),
        (36, 698.46, 10.0, 3.0),
        // bar 3 — tremolo alternation, then a held D5
        (48, 587.33, 1.0, 0.0),
        (49, 698.46, 1.0, 0.0),
        (50, 587.33, 1.0, 0.0),
        (51, 698.46, 1.0, 0.0),
        (52, 880.00, 1.0, 0.0),
        (53, 698.46, 1.0, 0.0),
        (54, 587.33, 1.0, 0.0),
        (55, 440.00, 2.0, 0.0),
        (57, 587.33, 7.0, 0.0),
        // bar 4 — pinch-harmonic screams way up top
        (64, 880.00, 6.0, 1.0),
        (70, 1174.66, 6.0, 2.0),
        // bar 5 — descend into the dive (placed separately)
        (80, 1174.66, 1.0, 0.0),
        (81, 880.00, 1.0, 0.0),
        (82, 698.46, 1.0, 0.0),
    ];

    for step in 0..total_steps {
        let bar = step / steps_per_bar;
        let pos = step % steps_per_bar;
        let off = step * step_samples;
        let bar_off = bar * steps_per_bar * step_samples;

        let in_intro = bar < INTRO;
        let in_solo = (RIFF_A..SOLO).contains(&bar);
        let last_bar = bar == RIFF_A - 1 || bar == SOLO - 1 || bar == BARS - 1;

        if pos == 0 && (bar == INTRO || bar == RIFF_A || bar == SOLO) {
            crash(&mut buf, off, &mut rng, 0.5);
        }

        // ---- Drums: straight 8th kicks, backbeat, busy hats ----
        if in_intro {
            if pos >= 8 {
                rock_snare(&mut buf, off, &mut rng, 0.22 + 0.05 * (pos - 8) as f32);
            }
        } else if last_bar {
            if pos == 0 {
                drum_fill(&mut buf, bar_off, step_samples, &mut rng);
            }
            if pos.is_multiple_of(4) {
                rock_kick(&mut buf, off, 0.85);
            }
        } else {
            if pos.is_multiple_of(2) {
                rock_kick(&mut buf, off, 0.88);
            }
            if pos == 4 || pos == 12 {
                rock_snare(&mut buf, off, &mut rng, 0.6);
            }
            rock_hat(
                &mut buf,
                off,
                &mut rng,
                if pos.is_multiple_of(4) { 0.2 } else { 0.14 },
                false,
            );
        }

        // ---- Rhythm guitar + bass ----
        if in_intro {
            if pos == 0 {
                power_chord(&mut buf, off, D2, step_ms * 16.0, 0.42, false);
                bass_guitar(&mut buf, off, D2, step_ms * 14.0, 0.5);
            }
        } else if !last_bar || in_solo {
            for &(s, root, dur, palm) in &RIFF {
                if s == pos {
                    power_chord(
                        &mut buf,
                        off,
                        root,
                        step_ms * dur,
                        if palm { 0.5 } else { 0.44 },
                        palm,
                    );
                }
            }
            if pos.is_multiple_of(2) {
                bass_guitar(&mut buf, off, D2, step_ms * 1.4, 0.5);
            }
        } else {
            // the fill bar still needs the downbeat chord
            if pos == 0 {
                power_chord(&mut buf, off, D2, step_ms * 4.0, 0.48, false);
            }
        }

        // ---- Lead ----
        if in_solo {
            let solo_step = (bar - RIFF_A) * steps_per_bar + pos;
            for &(s, f, dur, bend) in &LEAD {
                if s == solo_step {
                    lead_guitar(&mut buf, off, f, step_ms * dur, 0.6, bend);
                }
            }
            if solo_step == 83 {
                dive_bomb(&mut buf, off, 587.33, step_ms * 12.0, 0.55);
            }
        }
    }

    for bar in 0..BARS {
        let bar_off = bar * steps_per_bar * step_samples;
        power_chord(
            &mut buf,
            bar_off,
            D2,
            step_ms * steps_per_bar as f32 * 1.02,
            0.06,
            false,
        );
    }

    // a rock band, not a lullaby: the guitars keep their bite
    low_pass(&mut buf, 6500.0);
    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

const SR_F: f32 = SAMPLE_RATE as f32;

// ---------------------------------------------------------------------- //
// Tone / noise helpers (self-contained, same idiom as the other games)     //
// ---------------------------------------------------------------------- //

fn gen_tone(freq: f32, dur_ms: f32, amp: f32) -> Vec<i16> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(dur_ms);
    let fade = SAMPLE_RATE as usize / 200;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let mut e = 1.0_f32;
        if i < fade { e = i as f32 / fade as f32; }
        if i + fade > n { e = (n - i) as f32 / fade as f32; }
        let fund = (2.0 * std::f32::consts::PI * freq * t).sin();
        let third = (2.0 * std::f32::consts::PI * freq * 3.0 * t).sin() / 3.0;
        let shaped = (fund * 0.8 + third * 0.3).tanh();
        s.push((e * amp * 27000.0 * shaped) as i16);
    }
    s
}

/// LCG for deterministic noise.
struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12345) & 0x7FFF_FFFF;
        self.0
    }
}

fn gen_noise(dur_ms: f32, amp: f32) -> Vec<i16> {
    let n = ms_to_samples(dur_ms);
    let fade = SAMPLE_RATE as usize / 200;
    let mut rng = Lcg(7);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let mut e = 1.0_f32;
        if i < fade { e = i as f32 / fade as f32; }
        if i + fade > n { e = (n - i) as f32 / fade as f32; }
        let decay = 1.0 - i as f32 / n as f32;
        let r = rng.next() % 65536;
        let noise = (r as f32 - 32768.0) / 32768.0;
        s.push((e * amp * decay * 32000.0 * noise) as i16);
    }
    s
}

/// Notes one after another, each overlapping the next (`step_ms < dur_ms`):
/// the power-up chimes, more notes and higher for a bigger tier.
fn ascending_run(notes: &[f32], step_ms: f32, dur_ms: f32, amp: f32) -> Vec<i16> {
    let n = ms_to_samples(step_ms * notes.len() as f32 + dur_ms);
    let mut buf = vec![0i16; n];
    for (i, f) in notes.iter().enumerate() {
        let t = gen_tone(*f, dur_ms, amp);
        let off = ms_to_samples(step_ms * i as f32);
        for (j, s) in t.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    }
    buf
}

/// The grand finale for maxing out the weapon: a fast four-note ascending run
/// into a held bright chord, with a fifth harmony under the last note —
/// meant to feel like a proper "fanfare" next to the plain pickup chimes.
fn max_power_sfx() -> Vec<u8> {
    let notes = [523.25, 659.25, 783.99, 1046.50]; // C5 E5 G5 C6
    let step_ms = 90.0;
    let chord_ms = 260.0;
    let n = ms_to_samples(step_ms * notes.len() as f32 + chord_ms + 150.0);
    let mut buf = vec![0i16; n];
    for (i, f) in notes.iter().enumerate() {
        let dur = if i == notes.len() - 1 { chord_ms } else { step_ms * 1.4 };
        let t = gen_tone(*f, dur, 0.5);
        let off = ms_to_samples(step_ms * i as f32);
        for (j, s) in t.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    }
    // A fifth harmony under the held final chord, for extra sparkle.
    let harmony = gen_tone(1318.51, chord_ms, 0.3); // E6
    let off = ms_to_samples(step_ms * (notes.len() - 1) as f32);
    for (j, s) in harmony.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    encode_pcm16_mono(&buf)
}

/// A short rising/falling alarm wail — plays once as each boss makes its
/// entrance, tier 1 through 7 alike (the boss's own name banner is what
/// signals which one it is).
fn boss_warning_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let dur_ms = 500.0;
    let n = ms_to_samples(dur_ms);
    let mut s = Vec::with_capacity(n);
    let mut phase = 0.0f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let freq = 500.0 + (t * std::f32::consts::PI * 2.0).sin() * 220.0; // wobbles ~280-720Hz
        phase += freq / sr;
        let env = if t < 0.05 { t / 0.05 } else if t > 0.85 { (1.0 - t) / 0.15 } else { 1.0 };
        let wave = (2.0 * std::f32::consts::PI * phase).sin();
        s.push((env * 0.4 * 27000.0 * wave) as i16);
    }
    encode_pcm16_mono(&s)
}

/// The big finish: clearing all seven waves. A fast run up through an octave
/// into a sustained major triad — the biggest fanfare in the game, longer and
/// grander than the "reached max power" one.
fn victory_sfx() -> Vec<u8> {
    let notes = [392.00, 493.88, 587.33, 698.46, 783.99, 987.77]; // G4 B4 D5 F5 G5 B5
    let step_ms = 85.0;
    let chord_ms = 900.0;
    let n = ms_to_samples(step_ms * notes.len() as f32 + chord_ms + 300.0);
    let mut buf = vec![0i16; n];
    for (i, f) in notes.iter().enumerate() {
        let dur = if i == notes.len() - 1 { chord_ms } else { step_ms * 1.5 };
        let t = gen_tone(*f, dur, 0.5);
        let off = ms_to_samples(step_ms * i as f32);
        for (j, s) in t.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    }
    // Two more notes layered under the held final chord, for a full triad.
    let off = ms_to_samples(step_ms * (notes.len() - 1) as f32);
    for extra in [1174.66, 1567.98] { // D6, G6
        let t = gen_tone(extra, chord_ms, 0.28);
        for (j, s) in t.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    }
    encode_pcm16_mono(&buf)
}

/// Generate a wing-gun report; `pitch` and `seed` vary the takes for autofire.
fn gun_report(seed: u32, pitch: f32) -> Vec<f32> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(120.0);
    let mut rng = Rng(seed | 1);
    let mut dry = vec![0.0f32; n];
    let (mut lo, mut bp_lo, mut bp_hi, mut phase) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for (i, out) in dry.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let w = rng.next_f32() * 2.0 - 1.0;
        // crack: noise band-passed round 1.5-5 kHz, driven hard, ~4ms
        bp_hi += (w - bp_hi) * 0.55 * pitch.min(1.2);
        bp_lo += (bp_hi - bp_lo) * 0.18;
        let crack = ((bp_hi - bp_lo) * 6.0).tanh() * (-t / 0.004).exp();
        // body: low-passed blast, ~25ms
        lo += (w - lo) * 0.09 * pitch;
        let body = (lo * 5.0).tanh() * (-t / 0.025).exp();
        // thump: 180 -> 90 Hz, saturated so a phone speaker keeps it
        phase += pitch * (90.0 + 90.0 * (-t / 0.01).exp()) / sr;
        let thump = ((2.0 * PI * phase).sin() * 4.0).tanh() * (-t / 0.02).exp();
        // bolt: two short metallic clacks
        let mut bolt = 0.0;
        for (at, amp) in [(0.022f32, 1.0f32), (0.034, 0.6)] {
            let ct = t - at / pitch;
            if ct > 0.0 { bolt += amp * (2.0 * PI * 2900.0 * pitch * ct).sin() * (-ct / 0.0018).exp(); }
        }
        *out = crack * 1.0 + body * 0.7 + thump * 0.55 + bolt * 0.12;
    }
    // Early reflections, darker than the direct sound.
    let mut buf = dry.clone();
    for (ms, g) in [(14.0f32, 0.30f32), (27.0, 0.16)] {
        let d = ms_to_samples(ms);
        let mut lp = 0.0f32;
        for i in d..n {
            lp += (dry[i - d] - lp) * 0.25;
            buf[i] += lp * g;
        }
    }
    buf
}

/// `guns` guns firing one burst: each report a few ms off the others and a
/// little off pitch, summed and then limited, so more guns come out denser
/// and louder rather than the same shot turned up.
fn gun_burst(guns: usize, seed: u32) -> Vec<i16> {
    let mut rng = Rng(seed | 1);
    let pad = ms_to_samples(16.0);
    let mut mix = vec![0.0f32; ms_to_samples(120.0) + pad];
    for g in 0..guns {
        let delay = if g == 0 { 0 } else { (rng.next_f32() * pad as f32) as usize };
        let pitch = 0.9 + rng.next_f32() * 0.2;
        let shot = gun_report(seed.wrapping_add((g as u32).wrapping_mul(0x9E37_79B9)), pitch);
        for (i, v) in shot.iter().enumerate() { mix[delay + i] += v * 16_000.0; }
    }
    soft_limit_to_pcm16(&mix, MIX_KNEE)
}

/// An enemy fighter's burst, heard from a distance: three rounds from a
/// pair of guns, lower and duller than the player's (further off, smaller
/// calibre on most of them), with the highs rolled off by the air between.
fn enemy_gun_sfx() -> Vec<u8> {
    let mut rng = Rng(0xE6_6E1);
    let n = ms_to_samples(360.0);
    let mut mix = vec![0.0f32; n];
    for k in 0..3 {
        for g in 0..2 {
            let off = ms_to_samples(k as f32 * 80.0 + g as f32 * 9.0 + rng.next_f32() * 6.0);
            let shot = gun_report(0x1234 + (k * 2 + g) as u32 * 977, 0.78 + rng.next_f32() * 0.08);
            for (i, v) in shot.iter().enumerate() {
                if off + i < n { mix[off + i] += v * 12_000.0; }
            }
        }
    }
    warm(&mut mix);
    warm(&mut mix);
    encode_pcm16_mono(&soft_limit_to_pcm16(&mix, MIX_KNEE))
}

/// A backfire from an engine starved near a stall: a sharp pop in the
/// exhaust, a low thump, and a crackle as it clears.
fn backfire_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(280.0);
    let mut rng = Rng(0xBAC_F1);
    let mut buf = vec![0.0f32; n];
    let (mut lp, mut crack) = (0.0f32, 0.0f32);
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let w = rng.next_f32() * 2.0 - 1.0;
        lp += (w - lp) * 0.2;
        let pop = lp * (-t / 0.012).exp();
        let thump = (2.0 * PI * (70.0 + 60.0 * (-t / 0.02).exp()) * t).sin() * (-t / 0.05).exp();
        if rng.next_f32() < 0.004 * (1.0 - t / 0.28) { crack = 1.0; }
        crack *= 0.9;
        *out = (pop * 1.6 + thump * 0.9 + lp * crack * 0.8) * 20_000.0;
    }
    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// A ricochet: a hard metallic tick where the round strikes, then the
/// classic "pyeww" of a spinning slug tearing away, a whistle falling from
/// `f0` with a wavering pitch, a little air noise round it.
fn ricochet_sfx(f0: f32, seed: u32) -> Vec<i16> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(420.0);
    let mut rng = Rng(seed | 1);
    let mut buf = vec![0.0f32; n];
    let (mut phase, mut hp_prev) = (0.0f32, 0.0f32);
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let w = rng.next_f32() * 2.0 - 1.0;
        let hp = w - hp_prev;
        hp_prev = w;
        let tick = hp * (-t / 0.003).exp() + (2.0 * PI * 3100.0 * t).sin() * (-t / 0.006).exp() * 0.5;
        // whistle: falls to ~40% of f0 and wavers as the slug tumbles
        let f = f0 * (0.4 + 0.6 * (-t / 0.16).exp()) * (1.0 + 0.035 * (2.0 * PI * 23.0 * t).sin());
        phase += f / sr;
        let env = (t / 0.012).min(1.0) * (-t / 0.13).exp();
        let whistle = (2.0 * PI * phase).sin() * env;
        let air = w * env * 0.12;
        *out = (tick * 0.7 + whistle * 0.55 + air) * 20_000.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// A round striking the fuselage: a sharp tick, a short metallic ring from
/// two inharmonic partials, and a dull knock underneath.
fn hit_sfx() -> Vec<i16> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(160.0);
    let mut rng = Rng(0x417C_0DE5);
    let mut buf = vec![0.0f32; n];
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let w = rng.next_f32() * 2.0 - 1.0;
        let tick = w * (-t / 0.004).exp();
        let ring = ((2.0 * PI * 1180.0 * t).sin() + 0.6 * (2.0 * PI * 1730.0 * t).sin()) * (-t / 0.035).exp();
        let knock = (2.0 * PI * 180.0 * t).sin() * (-t / 0.02).exp();
        *out = (tick * 0.35 + ring * 0.35 + knock * 0.8) * 20_000.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// An explosion's shape: how long, how deep, how much goes off after.
#[derive(Clone, Copy)]
struct Blast {
    secs: f32,
    /// The boom's starting pitch; it falls to a third. Lower is bigger.
    boom_hz: f32,
    /// Fuel and ammunition cooking off after the main blast.
    secondaries: usize,
    /// Metal pinging away as debris.
    debris: usize,
    amp: f32,
    seed: u32,
}

/// Generate a seeded explosion from blast, roar, secondary, debris and echo layers.
fn explosion_sfx(b: Blast) -> Vec<i16> {
    let sr = SAMPLE_RATE as f32;
    let n = (b.secs * sr) as usize;
    let mut rng = Rng(b.seed | 1);
    let mut buf = vec![0.0f32; n + (0.3 * sr) as usize];
    let (mut lp1, mut lp2, mut phase, mut turb, mut turb_lp) = (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for i in 0..n {
        let t = i as f32 / sr;
        let k = t / b.secs;
        let attack = (t / 0.002).min(1.0);
        let decay = (1.0 - k).max(0.0).powf(1.6) * (-t / (b.secs * 0.4)).exp();
        let w = rng.next_f32() * 2.0 - 1.0;
        // the fireball's roar: dark noise whose filter closes as it dies,
        // its level wandering at a few hertz the way turbulence does
        let cut = 0.22 * (1.0 - k * 0.85) + 0.015;
        lp1 += (w - lp1) * cut;
        lp2 += (lp1 - lp2) * cut;
        turb += (rng.next_f32() * 2.0 - 1.0 - turb) * 0.0015;
        turb_lp += (turb - turb_lp) * 0.002;
        let roar = lp2 * 3.4 * (0.65 + 4.0 * turb_lp.abs());
        // the boom: a saturated low sine falling from boom_hz to a third
        phase += b.boom_hz * (0.33 + 0.67 * (-t / 0.08).exp()) / sr;
        let boom = ((2.0 * PI * phase).sin() * 3.0).tanh() * (-t / (b.secs * 0.22)).exp();
        // the blast front: a hard crack of bright noise in the first ms
        let front = w * (-t / 0.0025).exp() * 1.4 + lp1 * (-t / 0.015).exp() * 1.2;
        buf[i] += (roar * decay + boom * 0.8 * decay + front) * attack * b.amp;
    }
    // secondary explosions: smaller, later, each its own crack and thump
    for _ in 0..b.secondaries {
        let at = (b.secs * (0.12 + 0.5 * rng.next_f32()) * sr) as usize;
        let size = 0.35 + 0.4 * rng.next_f32();
        let f = b.boom_hz * (1.3 + 0.6 * rng.next_f32());
        let mut lp = 0.0f32;
        for i in 0..(0.18 * sr) as usize {
            let j = at + i;
            if j >= buf.len() { break; }
            let t = i as f32 / sr;
            let w = rng.next_f32() * 2.0 - 1.0;
            lp += (w - lp) * 0.25;
            buf[j] += size * b.amp * ((2.0 * PI * f * t).sin() * (-t / 0.05).exp() + lp * 1.5 * (-t / 0.03).exp());
        }
    }
    // debris: short metal clinks scattering away, two inharmonic partials
    // each so they read as torn panels rather than tones
    for _ in 0..b.debris {
        let at = (b.secs * (0.08 + 0.6 * rng.next_f32()) * sr) as usize;
        let f = 1200.0 + 2200.0 * rng.next_f32();
        let ring = 0.004 + 0.008 * rng.next_f32();
        let vol = 0.10 + 0.10 * rng.next_f32();
        for i in 0..(ring * 6.0 * sr) as usize {
            let j = at + i;
            if j >= buf.len() { break; }
            let t = i as f32 / sr;
            let tone = (2.0 * PI * f * t).sin() + 0.6 * (2.0 * PI * f * 2.37 * t).sin();
            buf[j] += tone * (-t / ring).exp() * vol * b.amp;
        }
    }
    // the sea and the sky give a little back: two soft, darker echoes
    let dry = buf.clone();
    for &(delay, gain) in &[(0.11f32, 0.28f32), (0.24, 0.16)] {
        let d = (delay * sr) as usize;
        let mut lp = 0.0f32;
        for i in d..buf.len() {
            lp += (dry[i - d] - lp) * 0.15;
            buf[i] += lp * gain;
        }
    }
    let end = buf.iter().rposition(|v| v.abs() > 1e-4).map_or(1, |i| i + 1);
    buf.truncate(end);
    let fade = (0.02 * sr) as usize;
    let len = buf.len();
    for i in 0..fade.min(len) { buf[len - 1 - i] *= i as f32 / fade as f32; }
    let scaled: Vec<f32> = buf.iter().map(|v| v * 22_000.0).collect();
    soft_limit_to_pcm16(&scaled, MIX_KNEE)
}

/// Takes of each explosion size: enemy planes and boats, the player, bosses.
const ENEMY_BLASTS: [Blast; 4] = [
    Blast { secs: 0.65, boom_hz: 100.0, secondaries: 1, debris: 3, amp: 0.8, seed: 0xE1E1 },
    Blast { secs: 0.75, boom_hz: 88.0, secondaries: 2, debris: 2, amp: 0.8, seed: 0xE2E2 },
    Blast { secs: 0.55, boom_hz: 112.0, secondaries: 0, debris: 4, amp: 0.75, seed: 0xE3E3 },
    Blast { secs: 0.8, boom_hz: 92.0, secondaries: 1, debris: 1, amp: 0.85, seed: 0xE4E4 },
];
const PLAYER_BLASTS: [Blast; 2] = [
    Blast { secs: 1.2, boom_hz: 70.0, secondaries: 3, debris: 5, amp: 1.0, seed: 0x9A7E },
    Blast { secs: 1.3, boom_hz: 64.0, secondaries: 2, debris: 4, amp: 1.0, seed: 0x9B7F },
];
const BOSS_BLASTS: [Blast; 2] = [
    Blast { secs: 2.0, boom_hz: 55.0, secondaries: 6, debris: 8, amp: 1.0, seed: 0xB055 },
    Blast { secs: 2.2, boom_hz: 50.0, secondaries: 7, debris: 6, amp: 1.0, seed: 0xB156 },
];

/// Descending 3-tone "power-down" sting for game over.
fn game_over_sfx() -> Vec<u8> {
    let notes = [440.0, 330.0, 220.0];
    let step_ms = 180.0;
    let n = ms_to_samples(step_ms * notes.len() as f32 + 200.0);
    let mut buf = vec![0i16; n];
    for (i, f) in notes.iter().enumerate() {
        let t = gen_tone(*f, step_ms * 1.3, 0.55);
        let off = ms_to_samples(step_ms * i as f32);
        for (j, s) in t.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    }
    encode_pcm16_mono(&buf)
}

/// Ascending 3-tone fanfare for a stage clear.
fn stage_clear_sfx() -> Vec<u8> {
    let notes = [523.25, 659.25, 783.99]; // C5 E5 G5
    let step_ms = 130.0;
    let n = ms_to_samples(step_ms * notes.len() as f32 + 300.0);
    let mut buf = vec![0i16; n];
    for (i, f) in notes.iter().enumerate() {
        let t = gen_tone(*f, step_ms * 1.6, 0.55);
        let off = ms_to_samples(step_ms * i as f32);
        for (j, s) in t.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    }
    encode_pcm16_mono(&buf)
}

// ---------------------------------------------------------------------- //
// Guns and engine                                                          //
// ---------------------------------------------------------------------- //

/// Turret cannon shot — a low tonal thump plus a burst of noise, so it reads
/// as artillery rather than the player's laser-y "pew".
fn turret_fire_sfx() -> Vec<u8> {
    let dur_ms = 140.0;
    let n = ms_to_samples(dur_ms);
    let mut buf = vec![0i16; n];
    let tone = gen_tone(130.0, dur_ms, 0.5);
    let noise = gen_noise(55.0, 0.6);
    for (j, s) in tone.iter().enumerate() { mix_into(&mut buf, j, *s as f32); }
    for (j, s) in noise.iter().enumerate() { mix_into(&mut buf, j, *s as f32); }
    encode_pcm16_mono(&buf)
}

/// Hermite smoothstep, clamped to 0..1: ramps the engine's pitch and chop
/// through the start-up.
fn smooth01(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The carrier launch start-up in one shot: the starter whirring, the engine
/// catching on a couple of uneven chugs, then the prop spinning up to idle.
/// ~1.9s, played as the launch begins over the engine loops.
fn engine_start_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let dur_s = 1.9_f32;
    let n = ms_to_samples(dur_s * 1000.0);
    let mut rng = Lcg(0x0E17_0011);
    let mut s = vec![0f32; n];
    for i in 0..n {
        let t = i as f32 / sr;
        let k = i as f32 / n as f32;

        // Starter motor: a low buzz with a fast amplitude wobble (the
        // electric crank), fading out as the engine takes over.
        let starter_env = (1.0 - (k / 0.62).min(1.0)).powf(1.4);
        let crank_f = 52.0 + 12.0 * k;
        let crank_wob = 0.6 + 0.4 * (2.0 * PI * 9.0 * t).sin();
        let crank_noise = ((rng.next() % 4096) as f32 / 4096.0 - 0.5) * 0.5;
        let starter = ((2.0 * PI * crank_f * t).sin() * crank_wob + crank_noise) * starter_env * 0.5;

        // The engine: pitch and blade-chop rate ramp up from a lumpy
        // catch to a steady idle over the second half of the clip.
        let eng_env = (k / 0.30).min(1.0);
        let spin = smooth01((k - 0.22) / 0.62);
        let f0 = 34.0 + 46.0 * spin;
        let chop_hz = 5.0 + 20.0 * spin;
        let chop = 0.4 + 0.6 * ((2.0 * PI * chop_hz * t).sin() * 0.5 + 0.5).powf(1.7);
        let tone = (2.0 * PI * f0 * t).sin()
            + 0.5 * (2.0 * PI * f0 * 2.0 * t).sin()
            + 0.25 * (2.0 * PI * f0 * 3.0 * t).sin();
        let engine = (tone * 0.4).tanh() * chop * eng_env * 0.9;

        // Two or three uneven "chug" thumps as the cylinders first fire.
        let mut chug = 0.0f32;
        for &(ct, cv) in &[(0.60_f32, 0.9_f32), (0.76, 0.7), (0.88, 0.5)] {
            let d = t - ct * dur_s;
            if (0.0..0.10).contains(&d) {
                let ce = (1.0 - d / 0.10).powf(1.5);
                chug += (2.0 * PI * 44.0 * d).sin() * ce * cv;
            }
        }

        s[i] = starter + engine + chug * 0.55;
    }
    let f = ms_to_samples(8.0);
    for i in 0..f {
        let g = i as f32 / f as f32;
        s[i] *= g;
        s[n - 1 - i] *= g;
    }
    let pcm: Vec<i16> = s.iter().map(|&v| (v.tanh() * 24_000.0) as i16).collect();
    half_rate(&pcm)
}

// Spitfire recordings set the 26 Hz cruise / 16 Hz idle crank rates and six firings per crank turn.
// Vary cylinder levels and add exhaust rasp; sine-only tones sounded like a generator.

/// Crankshaft turns per second at cruise.
const CRANK_HZ: f32 = 26.0;
/// How loud each of the twelve cylinders fires, in firing order: the six of
/// one turn differ most, so the loudness pulses at the crank rate as in the
/// recordings, and the two turns differ a little.
const CYLINDERS: [f32; 12] = [1.0, 0.72, 0.88, 0.66, 0.94, 0.78, 0.97, 0.70, 0.90, 0.64, 0.92, 0.80];
/// Each cylinder fires a touch early or late (fraction of a firing gap),
/// the same every cycle, so the harmonics stay sharp lines.
const CYL_TIMING: [f32; 12] = [0.0, 0.05, -0.03, 0.04, -0.05, 0.02, 0.01, 0.06, -0.04, 0.03, -0.02, 0.05];
/// Four blades geared down: two blade passes per crank turn.
const BLADE_PER_CRANK: f32 = 2.0;

/// One cylinder's exhaust pop into `buf` at sample `at`: a burst of dark
/// noise and a short low thump. Higher `rpm` is a harder, brighter bark.
fn exhaust_pop(buf: &mut [f32], at: usize, amp: f32, rpm: f32, rng: &mut Rng) {
    let len = (0.03 * SR_F) as usize;
    let body = 95.0 + 45.0 * rpm;
    let bright = 0.13 + 0.10 * rpm;
    let mut lp = 0.0f32;
    for i in 0..len {
        let j = at + i;
        if j >= buf.len() { break; }
        let t = i as f32 / SR_F;
        let w = rng.next_f32() * 2.0 - 1.0;
        lp += (w - lp) * bright;
        // the exhaust stack rings at two pipe modes
        let thump = (2.0 * PI * body * t).sin() * (-t / 0.011).exp()
            + 0.5 * (2.0 * PI * body * 2.1 * t).sin() * (-t / 0.006).exp();
        buf[j] += amp * (lp * 1.3 * (-t / 0.005).exp() + thump * 1.2);
    }
}

/// The propeller: a soft roar of low noise, swelling at the blade-pass rate.
fn prop_roar(buf: &mut [f32], blade_hz: f32, level: f32, rng: &mut Rng) {
    let (mut l1, mut l2) = (0.0f32, 0.0f32);
    for (i, v) in buf.iter_mut().enumerate() {
        let t = i as f32 / SR_F;
        let w = rng.next_f32() * 2.0 - 1.0;
        l1 += (w - l1) * 0.04;
        l2 += (l1 - l2) * 0.04;
        let swell = 0.55 + 0.45 * (2.0 * PI * blade_hz * t).sin();
        *v += l2 * 9.0 * swell * level;
    }
}

/// One-second engine loop at `rpm` (1.0 = cruise); even crank rates keep the 12-cylinder loop seamless.
/// `sputter` omits firings to model a labouring engine.
fn engine_loop(rpm: f32, sputter: bool) -> Vec<u8> {
    let n = SAMPLE_RATE as usize;
    let crank = ((CRANK_HZ * rpm / 2.0).round() * 2.0).max(4.0);
    let firing = (crank * 6.0) as usize; // firings per second
    let mut rng = Rng(0x0E61_4E55 ^ firing as u32);
    let mut buf = vec![0.0f32; n + (0.05 * SR_F) as usize];
    for k in 0..firing {
        let at = (((k as f32 + CYL_TIMING[k % 12]) / firing as f32).max(0.0) * SR_F) as usize;
        let mut amp = CYLINDERS[k % 12] * (0.75 + 0.25 * rpm.min(1.2));
        if sputter {
            let r = rng.next_f32();
            if r < 0.35 { continue; }            // a misfire
            if r > 0.96 { amp *= 2.2; }          // a pop in the exhaust
        }
        exhaust_pop(&mut buf, at, amp, rpm, &mut rng);
    }
    prop_roar(&mut buf, BLADE_PER_CRANK * crank, 0.35 + 0.25 * rpm, &mut rng);
    let tail = buf.split_off(n);
    for (i, v) in tail.iter().enumerate() { buf[i] += v; }
    let scaled: Vec<f32> = buf.iter().map(|v| v * 11_000.0).collect();
    half_rate(&soft_limit_to_pcm16(&scaled, MIX_KNEE))
}

/// Seconds into `engine_splutter_sfx` the engine dies, and catches again.
/// The game ducks the engine loops between them (see raider's main.rs).
pub const SPLUTTER_DIES: f32 = 0.15;
pub const SPLUTTER_CATCHES: f32 = 1.2;
pub const SPLUTTER_SECS: f32 = 1.6;

/// Bad fuel: the cruising engine misses more and more, nearly dies with the
/// prop windmilling and two coughs, then catches with a pop and runs again.
/// Rendered at the cruise loop's level, so it crossfades with it.
fn engine_splutter_sfx() -> Vec<u8> {
    let n = (SPLUTTER_SECS * SR_F) as usize;
    let crank = CRANK_HZ;
    let firing = crank * 6.0;
    let mut rng = Rng(0x5B1A_7731);
    let mut buf = vec![0.0f32; n + (0.05 * SR_F) as usize];
    let count = (SPLUTTER_SECS * firing) as usize;
    for k in 0..count {
        let t = k as f32 / firing;
        // the chance a firing is missed: rising, nearly dead, recovering
        let miss = if t < 0.35 { 0.1 + 0.6 * t / 0.35 }
            else if t < 1.05 { 0.93 }
            else if t < 1.4 { 0.93 * (1.0 - (t - 1.05) / 0.35) }
            else { 0.0 };
        if rng.next_f32() < miss { continue; }
        let mut amp = CYLINDERS[k % 12];
        if (0.4..1.1).contains(&t) && rng.next_f32() < 0.3 { amp *= 2.4; } // a cough
        // the engine slows as it starves, and the pops get duller
        let rpm = if t < 1.05 { 1.0 - 0.35 * (t / 1.05) } else { 0.65 + 0.35 * ((t - 1.05) / 0.35).min(1.0) };
        let at = ((t + CYL_TIMING[k % 12] / firing) * SR_F) as usize;
        exhaust_pop(&mut buf, at, amp, rpm, &mut rng);
    }
    // two coughs as it tries to catch, and the pop as it does
    for &(t, a) in &[(0.55f32, 3.2f32), (0.83, 2.6), (1.08, 3.6)] {
        for k in 0..3 { exhaust_pop(&mut buf, ((t + k as f32 * 0.012) * SR_F) as usize, a, 0.7, &mut rng); }
    }
    let mut roar = vec![0.0f32; buf.len()];
    prop_roar(&mut roar, BLADE_PER_CRANK * crank, 0.6, &mut rng);
    for (v, r) in buf.iter_mut().zip(&roar) { *v += r; }
    buf.truncate(n);
    // fade out under the returning loop
    let f0 = ((SPLUTTER_CATCHES + 0.05) * SR_F) as usize;
    for (i, v) in buf.iter_mut().enumerate().skip(f0) {
        *v *= 1.0 - (i - f0) as f32 / (n - f0) as f32;
    }
    let scaled: Vec<f32> = buf.iter().map(|v| v * 11_000.0).collect();
    half_rate(&soft_limit_to_pcm16(&scaled, MIX_KNEE))
}

/// A flak shell bursting at altitude: the hard crack of the charge, a dull
/// "crump" of a body, and a short rattle of fragments; a smaller cousin of
/// explosion_sfx, heard from a little way off.
fn flak_burst_sfx() -> Vec<u8> {
    let b = Blast { secs: 0.45, boom_hz: 140.0, secondaries: 0, debris: 3, amp: 0.7, seed: 0xF1A4 };
    half_rate(&explosion_sfx(b))
}

/// A laser turret charging, 1.3 s: a whine climbing two octaves, its
/// tremolo quickening, so the ear counts down with the eye.
fn laser_charge_sfx() -> Vec<u8> {
    let n = (1.3 * SR_F) as usize;
    let mut ph = 0.0f32;
    let mut buf = vec![0.0f32; n];
    for (i, v) in buf.iter_mut().enumerate() {
        let k = i as f32 / n as f32;
        let t = i as f32 / SR_F;
        ph += (320.0 * 4f32.powf(k)) / SR_F;
        let trem = 0.6 + 0.4 * (2.0 * PI * (6.0 + 30.0 * k) * t).sin();
        let tone = (2.0 * PI * ph).sin() + 0.3 * (2.0 * PI * ph * 2.0).sin();
        *v = tone * trem * (0.25 + 0.75 * k) * (t / 0.02).min(1.0) * 9_000.0;
    }
    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// A laser turret's beam, 0.7 s: a harsh electric buzz with a crackle,
/// cutting off.
fn laser_fire_sfx() -> Vec<u8> {
    let n = (0.7 * SR_F) as usize;
    let mut rng = Rng(0x1A5E_F12E);
    let mut buf = vec![0.0f32; n];
    let mut lp = 0.0f32;
    for (i, v) in buf.iter_mut().enumerate() {
        let t = i as f32 / SR_F;
        let k = i as f32 / n as f32;
        let saw = |f: f32| 2.0 * (t * f).fract() - 1.0;
        let w = rng.next_f32() * 2.0 - 1.0;
        lp += (w - lp) * 0.3;
        let buzz = saw(110.0) * 0.6 + saw(221.0) * 0.4 + saw(330.5) * 0.2;
        let env = (t / 0.01).min(1.0) * (1.0 - k).powf(0.3);
        *v = (buzz * 0.8 + lp * 0.7) * env * 14_000.0;
    }
    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// The laser barrier's drone, looped and ridden by distance in the game:
/// the 110 Hz fundamental, its harmonics and the tremolo all complete whole
/// cycles in the buffer, so it loops without a seam.
fn barrier_hum_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let dur_ms = 400.0;
    let n = ms_to_samples(dur_ms);
    let f0 = 110.0_f32; // A2 — 44 whole cycles over 400ms
    let trem_hz = 5.0_f32; // 2 whole cycles over 400ms
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let fund = (2.0 * std::f32::consts::PI * f0 * t).sin();
        let harm = (2.0 * std::f32::consts::PI * f0 * 2.0 * t).sin() * 0.4;
        let buzz = (2.0 * std::f32::consts::PI * f0 * 3.0 * t).sin() * 0.22;
        let trem = 0.75 + 0.25 * (2.0 * std::f32::consts::PI * trem_hz * t).sin();
        let shaped = ((fund + harm + buzz) * 0.5).tanh();
        s.push((shaped * trem * 16_000.0) as i16);
    }
    encode_pcm16_mono(&s)
}

/// The barrier's close-range buzz, crossfaded in as the beam nears the
/// player: an octave up with a fifth, and a hard 12.5 Hz throb. Every part
/// fits whole cycles into 400 ms, so it loops without a click.
fn barrier_hum2_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(400.0);
    let tau = 2.0 * std::f32::consts::PI;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let tone = (tau * 220.0 * t).sin() + 0.5 * (tau * 330.0 * t).sin() + 0.25 * (tau * 110.0 * t).sin();
        let throb = 0.55 + 0.45 * (tau * 12.5 * t).sin().max(0.0);
        s.push(((tone * 0.6).tanh() * throb * 15_000.0) as i16);
    }
    encode_pcm16_mono(&s)
}

/// A rumble (an explosion, an engine) encoded at half rate: little of it
/// lives above 8 kHz, and these are most of the game's download.
fn half_rate(pcm: &[i16]) -> Vec<u8> {
    let mut f: Vec<f32> = pcm.iter().map(|&v| v as f32).collect();
    low_pass(&mut f, 8000.0);
    let s: Vec<i16> = f.iter().map(|&v| v.clamp(-32767.0, 32767.0) as i16).collect();
    encode_pcm16_music(&s)
}

/// The effects, as `sounds/NAME.wav`.
pub fn sounds() -> Vec<Asset> {
    vec![
        ("sounds/turret_fire.wav",    turret_fire_sfx()),
        ("sounds/barrier_hum.wav",    barrier_hum_sfx()),
        ("sounds/barrier_hum2.wav",   barrier_hum2_sfx()),
        ("sounds/engine_start.wav",   engine_start_sfx()),
        ("sounds/enemy_gun.wav",      halved(&enemy_gun_sfx())),
        ("sounds/backfire.wav",       backfire_sfx()),
        ("sounds/engine_splutter.wav", engine_splutter_sfx()),
        ("sounds/flak_burst.wav",     flak_burst_sfx()),
        ("sounds/laser_charge.wav",   halved(&laser_charge_sfx())),
        ("sounds/laser_fire.wav",     laser_fire_sfx()),
        ("sounds/engine0.wav",        engine_loop(0.4, true)),
        ("sounds/engine1.wav",        engine_loop(0.65, false)),
        ("sounds/engine2.wav",        engine_loop(1.0, false)),
        ("sounds/engine3.wav",        engine_loop(1.2, false)),
        ("sounds/engine4.wav",        engine_loop(1.42, false)),
        ("sounds/burst1_1.wav", encode_pcm16_mono(&gun_burst(2, 0x5a0f82d5))),
        ("sounds/burst1_2.wav", encode_pcm16_mono(&gun_burst(2, 0x5a0f8698))),
        ("sounds/burst2_1.wav", encode_pcm16_mono(&gun_burst(3, 0x5a0f83d6))),
        ("sounds/burst2_2.wav", encode_pcm16_mono(&gun_burst(3, 0x5a0f8799))),
        ("sounds/burst3_1.wav", encode_pcm16_mono(&gun_burst(5, 0x5a0f84d7))),
        ("sounds/burst3_2.wav", encode_pcm16_mono(&gun_burst(5, 0x5a0f889a))),
        ("sounds/burst4_1.wav", encode_pcm16_mono(&gun_burst(6, 0x5a0f85d8))),
        ("sounds/burst4_2.wav", encode_pcm16_mono(&gun_burst(6, 0x5a0f899b))),
        ("sounds/burst5_1.wav", encode_pcm16_mono(&gun_burst(8, 0x5a0f86d9))),
        ("sounds/burst5_2.wav", encode_pcm16_mono(&gun_burst(8, 0x5a0f8a9c))),
        ("sounds/enemy_explode0.wav", half_rate(&explosion_sfx(ENEMY_BLASTS[0]))),
        ("sounds/enemy_explode1.wav", half_rate(&explosion_sfx(ENEMY_BLASTS[1]))),
        ("sounds/enemy_explode2.wav", half_rate(&explosion_sfx(ENEMY_BLASTS[2]))),
        ("sounds/enemy_explode3.wav", half_rate(&explosion_sfx(ENEMY_BLASTS[3]))),
        ("sounds/player_explode0.wav", half_rate(&explosion_sfx(PLAYER_BLASTS[0]))),
        ("sounds/player_explode1.wav", half_rate(&explosion_sfx(PLAYER_BLASTS[1]))),
        ("sounds/ricochet1.wav",      encode_pcm16_mono(&ricochet_sfx(3300.0, 0x51C0_C4E7))),
        ("sounds/ricochet2.wav",      encode_pcm16_mono(&ricochet_sfx(2700.0, 0x2B1E_77A3))),
        ("sounds/player_hit.wav",     encode_pcm16_mono(&hit_sfx())),
        ("sounds/boss_explode0.wav",  half_rate(&explosion_sfx(BOSS_BLASTS[0]))),
        ("sounds/boss_explode1.wav",  half_rate(&explosion_sfx(BOSS_BLASTS[1]))),
        ("sounds/boss_warning.wav",   halved(&boss_warning_sfx())),
        ("sounds/powerup2.wav",       encode_pcm16_half(&ascending_run(&[880.00, 1318.51], 55.0, 85.0, 0.5))),
        ("sounds/powerup3.wav",       encode_pcm16_half(&ascending_run(&[987.77, 1479.98], 50.0, 90.0, 0.5))),
        ("sounds/powerup4.wav",       encode_pcm16_half(&ascending_run(&[740.00, 987.77, 1318.51], 55.0, 95.0, 0.5))),
        ("sounds/max_power.wav",      halved(&max_power_sfx())),
        ("sounds/health_pickup.wav",  encode_pcm16_half(&ascending_run(&[392.00, 523.25, 659.25], 55.0, 90.0, 0.42))),
        ("sounds/stage_clear.wav",    halved(&stage_clear_sfx())),
        ("sounds/victory.wav",        halved(&victory_sfx())),
        ("sounds/game_over.wav",      halved(&game_over_sfx())),
    ]
}

/// The music: each tune as a WAV to play and an XM of its notes for TUNE.
pub fn music() -> Vec<(String, Vec<u8>)> {
    blip::xm::tunes(&[
        ("LOOP1", &loop1_wav),
        ("LOOP2", &loop2_wav),
    ])
}
