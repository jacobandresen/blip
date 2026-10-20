use std::f32::consts::PI;

use crate::wav::{
    encode_pcm16_mono, low_pass, mix_into_f32, soft_limit_to_pcm16, tame, Rng, MIX_KNEE,
    SAMPLE_RATE,
};

// Rock music uses detuned saws or saw/square guitars, tanh drive and a low-pass cabinet filter.
// Voices mix in f32 and are limited once at the end.

/// Rock kick: tight and dry with a hard beater click that cuts through
/// distorted guitar; much less boom than `techno::kick`.
fn rock_kick(buf: &mut [f32], off: usize, vol: f32) {
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
pub fn music() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let bpm = 150.0_f32;
    let steps_per_bar = 16usize;
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
pub fn music2() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let bpm = 176.0_f32;
    let steps_per_bar = 16usize;
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
