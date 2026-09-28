//! Brawler assets: sounds only. The fighters are drawn by the game from the
//! same numbers as their hitboxes, so there is no sprite sheet to disagree
//! with the rules.

use crate::techno::{tame, warm, Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_mono, encode_pcm16_music, env, ms_to_samples,
    soft_limit_to_pcm16, SAMPLE_RATE};
use crate::Asset;

// ---- combat foley ---------------------------------------------------------
// Every sound here is a person. Flesh is heavily damped, so a body being hit
// never rings at a pitch (that is a drum): it is a short muffled thud of
// filtered noise, a smack of skin, the gi's cloth, and the fighter's own
// voice (a breath, a grunt, a winded groan), with a trace of the room.

const SRF: f32 = SAMPLE_RATE as f32;



/// Noise through a resonant band-pass (state-variable filter) at `center`
/// Hz, rising in `attack` and dying with time constant `decay` (seconds).
fn band(buf: &mut [f32], at: f32, center: f32, q: f32, amp: f32, attack: f32, decay: f32, rng: &mut Rng) {
    let off = (at * SRF) as usize;
    let n = ((attack + decay * 6.0) * SRF) as usize;
    let f = 2.0 * (std::f32::consts::PI * center / SRF).sin();
    let damp = 1.0 / q;
    let (mut low, mut bp) = (0.0f32, 0.0f32);
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / SRF;
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
    let extra = (0.06 * SRF) as usize;
    let dry = buf.clone();
    buf.resize(dry.len() + extra, 0.0);
    for (d, g) in taps {
        let off = (d * SRF) as usize;
        let mut lp = 0.0f32;
        for (i, x) in dry.iter().enumerate() {
            lp += (x - lp) * 0.35;
            buf[i + off] += lp * g * wet;
        }
    }
}

/// Two gentle low-pass poles at `hz`: air and a room take the fizz off.
fn soften(buf: &mut [f32], hz: f32) {
    let a = 1.0 - (-2.0 * std::f32::consts::PI * hz / SRF).exp();
    let (mut l1, mut l2) = (0.0f32, 0.0f32);
    for v in buf.iter_mut() { l1 += a * (*v - l1); l2 += a * (l1 - l2); *v = l2; }
}

fn render(mut buf: Vec<f32>, gain: f32) -> Vec<i16> {
    soften(&mut buf, 5000.0);
    room(&mut buf, 1.0);
    let n = buf.len();
    let fade = ((0.01 * SRF) as usize).min(n);
    for i in 0..fade { buf[n - fade + i] *= 1.0 - (i + 1) as f32 / fade as f32; }
    let scaled: Vec<f32> = buf.iter().map(|v| v * gain).collect();
    soft_limit_to_pcm16(&scaled, MIX_KNEE)
}





/// A very short damped sine: the pitch centre of a knock, dead in ~50 ms.
fn knock(buf: &mut [f32], at: f32, f: f32, amp: f32, tau: f32) {
    let off = (at * SRF) as usize;
    let n = ((tau * 5.0) * SRF) as usize;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / SRF;
        buf[off + i] += (2.0 * std::f32::consts::PI * f * t).sin() * amp * (-t / tau).exp() * (t / 0.0008).min(1.0);
    }
}

/// A human voice: a vocal-cord buzz (harmonics falling off like a real
/// glottal pulse, with jitter and shimmer) plus breath, shaped by three
/// formants into a vowel `(F1, F2, F3)`. Pitch glides `f0`..`f1`; `breath`
/// 0..1 is how much of it is air rather than tone.
fn voice(buf: &mut [f32], at: f32, dur: f32, f0: f32, f1: f32, vowel: (f32, f32, f32), breath: f32,
         amp: f32, rng: &mut Rng) {
    let off = (at * SRF) as usize;
    let n = (dur * SRF) as usize;
    let formants = [(vowel.0, 80.0, 1.0), (vowel.1, 110.0, 0.55), (vowel.2, 160.0, 0.3)];
    let mut state = [(0.0f32, 0.0f32); 3];
    let (mut ph, mut jit) = (0.0f32, 0.0f32);
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let k = i as f32 / n as f32;
        jit = jit * 0.995 + (rng.next_f32() - 0.5) * 0.004;
        let f = (f0 + (f1 - f0) * k) * (1.0 + jit);
        ph += f / SRF;
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
            let fq = 2.0 * (std::f32::consts::PI * fc / SRF).sin();
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
const OO: (f32, f32, f32) = (380.0, 900.0, 2300.0);  // "oof", winded
const AH: (f32, f32, f32) = (760.0, 1150.0, 2500.0); // "ah", the cry going down

/// A blow landing on a body. `thud` is the centre of the muffled flesh thud
/// (the combo climbs it); `weight` 0..1 runs from a jab to a heavy kick:
/// a deeper, longer thud, more cloth, and a voiced grunt instead of a breath.
fn hit(thud: f32, weight: f32, seed: u32) -> Vec<i16> {
    let mut rng = Rng(seed);
    let mut buf = vec![0.0f32; (0.34 * SRF) as usize];
    // the smack: skin on skin, a crack and a crackle
    band(&mut buf, 0.0, 2400.0, 1.2, 0.9, 0.0003, 0.003 + 0.002 * weight, &mut rng);
    band(&mut buf, 0.0021, 1600.0, 1.5, 0.35, 0.0002, 0.0025, &mut rng);
    // the flesh thud: a muffled knock with a pitch centre and no ring (a
    // 12 ms core dies in about 50 ms; a drum rings for hundreds)
    band(&mut buf, 0.0, thud, 4.0, 1.8, 0.0006, 0.016 + 0.01 * weight, &mut rng);
    knock(&mut buf, 0.0, thud, 0.9, 0.012);
    // the body mass behind it, deep and dead
    band(&mut buf, 0.001, 110.0, 0.8, 0.8 + 1.4 * weight, 0.002, 0.02 + 0.025 * weight, &mut rng);
    // the gi
    band(&mut buf, 0.003, 950.0, 0.8, 0.2 + 0.25 * weight, 0.002, 0.03 + 0.02 * weight, &mut rng);
    // the fighter taking it: a forced breath on a light hit, a grunt on a heavy one
    if weight > 0.5 {
        voice(&mut buf, 0.012, 0.22, 150.0, 105.0, UH, 0.35, 0.9, &mut rng);
    } else {
        // each gasp of a combo a little tighter than the last
        let tight = thud / 300.0;
        let v = (UH.0 * tight, UH.1 * tight, UH.2);
        voice(&mut buf, 0.01, 0.09, 140.0 * tight, 120.0 * tight, v, 1.0, 0.25, &mut rng);
    }
    render(buf, 11_000.0 + 5_000.0 * weight)
}

/// Blocked: forearm on forearm, a dull dry knock and the sleeves slapping,
/// and the blocker's short exhale. No skin smack, no body thud behind it,
/// so it never sounds like a hit landing.
fn block_sfx() -> Vec<i16> {
    let mut rng = Rng(0x51A7);
    let mut buf = vec![0.0f32; (0.2 * SRF) as usize];
    band(&mut buf, 0.0, 520.0, 1.6, 1.2, 0.0004, 0.008, &mut rng);
    band(&mut buf, 0.0, 1300.0, 1.0, 0.6, 0.0005, 0.01, &mut rng);
    band(&mut buf, 0.004, 800.0, 0.8, 0.3, 0.002, 0.02, &mut rng);
    voice(&mut buf, 0.008, 0.07, 150.0, 140.0, UH, 0.9, 0.25, &mut rng);
    render(buf, 12_000.0)
}

/// A body going down on the boards: hips then shoulders landing as dull deep
/// thuds, a short dead knock of the planks, the gi, and a winded "oof".
fn crunch() -> Vec<i16> {
    let mut rng = Rng(0x5A);
    let mut buf = vec![0.0f32; (0.6 * SRF) as usize];
    band(&mut buf, 0.0, 85.0, 0.9, 3.0, 0.002, 0.05, &mut rng);
    band(&mut buf, 0.0, 240.0, 1.2, 0.9, 0.001, 0.012, &mut rng);
    band(&mut buf, 0.0, 700.0, 1.0, 0.35, 0.001, 0.02, &mut rng);
    band(&mut buf, 0.09, 80.0, 0.9, 2.2, 0.002, 0.045, &mut rng);
    band(&mut buf, 0.09, 220.0, 1.2, 0.6, 0.001, 0.01, &mut rng);
    band(&mut buf, 0.05, 600.0, 0.8, 0.2, 0.01, 0.07, &mut rng);
    voice(&mut buf, 0.03, 0.34, 125.0, 88.0, OO, 0.3, 0.7, &mut rng);
    soften(&mut buf, 1400.0); // a body is soft: the fall is all low
    render(buf, 16_000.0)
}

/// Boots on boards: a short dead knock with a heel click.
fn land(seed: u32) -> Vec<i16> {
    let mut rng = Rng(seed);
    let mut buf = vec![0.0f32; (0.16 * SRF) as usize];
    band(&mut buf, 0.0, 150.0, 1.0, 1.4, 0.001, 0.02, &mut rng);
    band(&mut buf, 0.0, 420.0, 1.4, 0.5, 0.0005, 0.008, &mut rng);
    band(&mut buf, 0.0, 3000.0, 1.5, 0.3, 0.0003, 0.002, &mut rng);
    render(buf, 10_000.0)
}

/// The knockout: the finishing blow, a long cry falling away, and the body
/// hitting the boards.
fn ko() -> Vec<i16> {
    let mut rng = Rng(0xF157);
    let mut buf = vec![0.0f32; (1.1 * SRF) as usize];
    band(&mut buf, 0.0, 2400.0, 1.2, 1.0, 0.0003, 0.005, &mut rng);
    band(&mut buf, 0.0, 300.0, 2.0, 1.8, 0.0006, 0.03, &mut rng);
    band(&mut buf, 0.001, 110.0, 0.8, 2.2, 0.002, 0.05, &mut rng);
    voice(&mut buf, 0.02, 0.75, 210.0, 95.0, AH, 0.3, 1.1, &mut rng);
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
        let f = 2.0 * (std::f32::consts::PI * center / SRF).sin();
        let x = rng.next_f32() * 2.0 - 1.0;
        let high = x - low - 0.55 * bp;
        bp += f * high;
        low += f * bp;
        let flutter = 1.0 + 0.18 * (k * 60.0).sin();
        *out = bp * (k * std::f32::consts::PI).sin().powf(1.3) * flutter * vol;
    }
    soften(&mut buf, 2200.0);
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
        let w = 1.0
            + 0.22 * (t * 37.0).sin()
            + 0.15 * (t * 23.0 + 1.7).sin()
            + 0.10 * (t * 61.0 + 0.4).sin();
        *out = voices * w * e * vol * 5200.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}


/// Cloth snapping taut, the sound of a gi when a leg goes out fast: the
/// whoosh is air moving, this is speed. Noise through a band opening upward
/// as the leg extends, a one-sample difference to strip the bottom, an
/// envelope almost all attack, then a quieter second crack as the leg comes
/// back.
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
        for i in from..n {
            let d = (i - from) as f32 / per_ms;
            let noise = rng.next_f32() * 2.0 - 1.0;
            // The band opens as the slack runs out.
            let k = (0.20 + 0.030 * d).min(0.95);
            lp += (noise - lp) * k;
            let bright = lp - prev;
            prev = lp;
            // Almost all attack: up in two milliseconds, gone in fifty.
            let e = (d / 2.5).min(1.0) * (-d * 0.062).exp();
            buf[i] += bright * e * gain * 19000.0;
        }
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}


/// The bell that opens a round.
fn bell() -> Vec<i16> {
    let n = ms_to_samples(900.0);
    let mut buf = vec![0.0f32; n];
    // Three inharmonic partials: a struck bell, not a note.
    for (freq, amp) in [(880.0f32, 1.0f32), (1320.0, 0.55), (2093.0, 0.3)] {
        for (i, out) in buf.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;
            let decay = (-3.2 * t).exp();
            let phase = 2.0 * std::f32::consts::PI * freq * t;
            *out += phase.sin() * decay * amp * 5000.0;
        }
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}


/// A fireball leaving the hand: noise and a rising tone.
fn projectile() -> Vec<i16> {
    let n = ms_to_samples(420.0);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(0x2B01);
    let mut phase = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / n as f32;
        let e = env(i, n, ms_to_samples(6.0), ms_to_samples(300.0));
        let f = 180.0 + 520.0 * t;
        phase += 2.0 * std::f32::consts::PI * f / SAMPLE_RATE as f32;
        let noise = (rng.next_f32() * 2.0 - 1.0) * 0.5;
        *out = (phase.sin() * 0.7 + noise) * e * 8000.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// A plucked string: koto, or shamisen struck harder. Karplus-Strong: a
/// period-long delay line of noise read round and round, averaging
/// neighbours, so the highs die first and a bright pluck decays to a pure
/// tone, like a real string.
fn pluck(buf: &mut [f32], off: usize, freq: f32, ms: f32, gain: f32, rng: &mut Rng) {
    let freq = tame(freq);
    let period = (SAMPLE_RATE as f32 / freq).max(2.0) as usize;
    let n = ms_to_samples(ms).min(buf.len().saturating_sub(off));
    let mut line: Vec<f32> = (0..period).map(|_| rng.next_f32() * 2.0 - 1.0).collect();
    // A little brightness taken off the initial burst: a koto is
    // plucked with a plectrum, not struck with a hammer.
    for i in 1..period {
        line[i] = line[i] * 0.6 + line[i - 1] * 0.4;
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
fn taiko(buf: &mut [f32], off: usize, freq: f32, gain: f32, rng: &mut Rng) {
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
fn rim(buf: &mut [f32], off: usize, gain: f32, rng: &mut Rng) {
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

/// The hirajoshi scale in semitones from the root: root, minor second,
/// fourth, fifth, minor sixth. The minor second is what makes it sound
/// Japanese.
const HIRAJOSHI: [f32; 5] = [0.0, 1.0, 5.0, 7.0, 8.0];

fn degree(root: f32, step: i32) -> f32 {
    let oct = step.div_euclid(5);
    let d = HIRAJOSHI[step.rem_euclid(5) as usize];
    root * 2.0f32.powf((d + 12.0 * oct as f32) / 12.0)
}

/// A theme built from four-bar phrases, sequenced by `order` rather than
/// looped: eight slots of four bars outlast the round, so no seam is heard.
/// Phrases are scale degrees, so a tune cannot leave the scale.
fn theme(bpm: f32, root: f32, phrases: &[&[i32]], bass: &[&[i32]], order: &[usize],
         drive: f32) -> Vec<i16> {
    let step = (SAMPLE_RATE as f32 * 60.0 / bpm / 4.0) as usize; // 16ths
    let bars = 4 * order.len();
    let steps = bars * 16;
    // Room past the end for the last notes to ring out in, which is
    // then folded back onto the top — see `wrap_tail`.
    let body = step * steps;
    let n = body + ms_to_samples(900.0);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(0x9E3B);
    let beat_ms = 60.0 * 1000.0 / bpm;
    for s in 0..steps {
        let off = s * step;
        let slot = s / 64;                 // which four-bar phrase
        let melody = phrases[order[slot] % phrases.len()];
        let line = bass[order[slot] % bass.len()];
        let within = s % 64;               // step inside the phrase
        let bar = within / 16;
        let b = s % 16;
        // Each phrase has a quiet bar, and a whole phrase now and then
        // is quiet throughout — a theme with no let-up in it has
        // nothing to come back from.
        let hush = order[slot] == 3;
        let lull = bar == 2 || hush;
        let last_bar = bar == 3;
        let last_slot = slot + 1 == order.len();

        // The pulse: a heavy taiko on one, an answer past the middle.
        if b == 0 {
            taiko(&mut buf, off, 96.0, if bar == 0 && !hush { 1.0 } else { 0.8 }, &mut rng);
        }
        if b == 8 && !lull { taiko(&mut buf, off, 104.0, 0.7, &mut rng); }
        if b == 12 && bar % 2 == 1 && !hush { taiko(&mut buf, off, 122.0, 0.6, &mut rng); }
        // Every phrase fills into the next one; the last one fills
        // hardest, because that is the seam back to the top.
        if last_bar && matches!(b, 10 | 12 | 14) {
            let g = if last_slot { 0.7 } else { 0.5 } + 0.15 * drive;
            taiko(&mut buf, off, 92.0 + (b as f32 - 10.0) * 14.0, g, &mut rng);
        }
        // Rim on the off-beats, denser on a driving stage.
        if b % 4 == 2 && !(lull && b == 10) { rim(&mut buf, off, 0.5, &mut rng); }
        if b % 8 == 7 && !hush { rim(&mut buf, off, 0.32, &mut rng); }
        if drive > 0.5 && b % 4 == 3 && !lull { rim(&mut buf, off, 0.18, &mut rng); }

        // The koto line.
        let note = melody[within % melody.len()];
        if note > -90 {
            let gain = if lull { 0.62 } else { 0.85 };
            pluck(&mut buf, off, degree(root, note), beat_ms * 1.6, gain, &mut rng);
            // Doubled an octave up on the last bar of a phrase, so the
            // tune has a top to it once every four bars.
            if last_bar && b == 0 {
                pluck(&mut buf, off, degree(root, note + 5), beat_ms * 1.2, 0.4, &mut rng);
            }
        }

        // The bass walks a note a bar.
        if b == 0 {
            let d = line[bar % line.len()];
            pluck(&mut buf, off, degree(root, d) * 0.5, beat_ms * 3.6, 0.55, &mut rng);
        }
        if b == 10 && !lull {
            let d = line[bar % line.len()];
            pluck(&mut buf, off, degree(root, d + 2) * 0.5, beat_ms * 1.2, 0.3, &mut rng);
        }
    }
    wrap_tail(&mut buf, body);
    warm(&mut buf[..body]);
    soft_limit_to_pcm16(&buf[..body], MIX_KNEE)
}

/// Fold what rings out past the end of the loop onto its start, so tails
/// carry over the loop point instead of the music stopping dead.
fn wrap_tail(buf: &mut [f32], body: usize) {
    for i in body..buf.len() {
        buf[i - body] += buf[i];
    }
}


/// The three themes, synthesised at startup rather than baked in: a
/// round-length theme is a megabyte of PCM, times three, while the code is a
/// couple of hundred lines. 0 and 1 are the stages, 2 the select screen.
pub fn theme_wav(which: usize) -> Vec<u8> {
    // All on hirajoshi, differing in pace, register and density: the docks
    // fast and hard on the drum, the temple slower and higher with room
    // between notes, the select screen between them and out of the way.
    // Four-bar phrases, sequenced; -99 is a rest.
    const R: i32 = -99;

    // -- the docks ------------------------------------------------------
    let d_a = [0, R, R, 2, 1, R, 0, R, 3, R, 2, R, 0, R, R, 1,
               4, R, 3, R, 2, R, R, 0, 1, R, 0, R, R, 2, R, R,
               0, R, R, 2, 1, R, 0, R, 3, R, 4, R, 3, R, R, 2,
               4, R, 5, R, 4, R, 3, R, 2, R, R, 1, 0, R, R, R];
    let d_b = [2, R, 3, R, 4, R, R, 3, 2, R, R, 4, 5, R, R, R,
               4, R, 3, R, 2, R, 1, R, 0, R, R, 2, 1, R, R, R,
               3, R, R, 4, 5, R, 4, R, 3, R, R, 2, 1, R, 2, R,
               0, R, 1, R, 2, R, R, 3, 2, R, 1, R, 0, R, R, R];
    let d_c = [5, R, R, R, 4, R, R, R, 3, R, R, R, 2, R, R, R,
               4, R, R, 3, R, R, 2, R, R, R, 1, R, R, R, R, R,
               0, R, R, 1, 2, R, R, 3, 4, R, R, R, 3, R, R, R,
               2, R, R, R, 1, R, R, R, 0, R, R, R, R, R, R, R];
    let d_d = [0, R, R, R, R, R, 2, R, R, R, 1, R, R, R, R, R,
               0, R, R, R, R, R, R, R, 2, R, R, R, R, R, R, R,
               3, R, R, R, R, R, 2, R, R, R, R, R, 1, R, R, R,
               0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R];

    // -- the temple -----------------------------------------------------
    let t_a = [3, R, R, R, 2, R, 4, R, 3, R, R, 1, 0, R, R, R,
               1, R, 2, R, R, 3, R, 2, 0, R, R, R, R, R, 1, R,
               3, R, R, R, 4, R, R, 3, 5, R, R, 4, 3, R, R, R,
               2, R, R, 1, R, R, 2, R, 0, R, R, R, R, R, R, R];
    let t_b = [5, R, R, 4, R, R, 3, R, R, R, 4, R, 5, R, R, R,
               4, R, R, R, 3, R, R, 2, R, R, 3, R, R, R, R, R,
               2, R, R, 3, 4, R, R, 5, R, R, 4, R, 3, R, R, R,
               1, R, R, R, 2, R, R, R, 0, R, R, R, R, R, R, R];
    let t_c = [0, R, R, R, 1, R, R, R, 2, R, R, R, 3, R, R, R,
               2, R, R, 1, R, R, 0, R, R, R, R, R, 1, R, R, R,
               4, R, R, R, 3, R, R, R, 2, R, R, 1, R, R, R, R,
               0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R];
    let t_d = [3, R, R, R, R, R, R, R, 2, R, R, R, R, R, R, R,
               1, R, R, R, R, R, R, R, 0, R, R, R, R, R, R, R,
               2, R, R, R, R, R, R, R, 1, R, R, R, R, R, R, R,
               0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R];

    // -- the select screen ----------------------------------------------
    let s_a = [0, R, R, R, 3, R, 2, R, R, R, 1, R, 0, R, R, R,
               2, R, R, R, 1, R, R, 0, R, R, R, R, 3, R, R, R,
               4, R, R, 3, R, R, 2, R, R, R, 3, R, 4, R, R, R,
               2, R, R, R, 0, R, R, 1, R, R, R, R, R, R, R, R];
    let s_b = [3, R, R, R, 4, R, R, R, 5, R, R, 4, 3, R, R, R,
               2, R, R, 3, R, R, 4, R, R, R, 3, R, R, R, R, R,
               1, R, R, R, 2, R, R, 1, 0, R, R, R, R, R, 2, R,
               1, R, R, R, R, R, 0, R, R, R, R, R, R, R, R, R];

    // A note a bar under each, so the ground moves.
    let b0 = [0, 0, 3, 2];
    let b1 = [2, 3, 4, 2];
    let b2 = [0, 4, 3, 0];
    let b3 = [0, 0, 0, 0];

    // Eight slots of four bars: a rondo, so the tune keeps coming back
    // without the piece repeating. Slot 3 is the hushed one.
    let order = [0usize, 1, 0, 3, 2, 1, 0, 1];
    match which {
        0 => encode_pcm16_music(&theme(132.0, 293.66,
            &[&d_a, &d_b, &d_c, &d_d], &[&b0, &b1, &b2, &b3], &order, 1.0)),
        1 => encode_pcm16_music(&theme(108.0, 220.0,
            &[&t_a, &t_b, &t_c, &t_d], &[&b0, &b1, &b2, &b3], &order, 0.2)),
        _ => encode_pcm16_music(&theme(118.0, 246.94,
            &[&s_a, &s_b], &[&b2, &b0], &[0, 1, 0, 1], 0.0)),
    }
}

/// The crowd, built at startup: 2.5 s of PCM is a quarter of a megabyte for a
/// few lines of filtered noise.
pub fn crowd_wav(long: bool) -> Vec<u8> {
    let s = if long { crowd(1600.0, 1.0, 0xFEED) } else { crowd(900.0, 0.85, 0xC0DE) };
    encode_pcm16_mono(&s)
}

pub fn generate() -> Vec<Asset> {
    vec![
        // Three light hits, the thud climbing a combo step each, so a string
        // sounds like it is going somewhere.
        ("sounds/hit_light.wav",  encode_pcm16_mono(&hit(300.0, 0.15, 0x11))),
        ("sounds/hit_light2.wav", encode_pcm16_mono(&hit(380.0, 0.2, 0x1b))),
        ("sounds/hit_light3.wav", encode_pcm16_mono(&hit(470.0, 0.25, 0x2f))),
        ("sounds/hit_heavy.wav",  encode_pcm16_mono(&hit(230.0, 1.0, 0x22))),
        // A knockdown: a body on the boards, lower and longer than any hit.
        ("sounds/crunch.wav",    encode_pcm16_mono(&crunch())),
        ("sounds/land.wav",      encode_pcm16_mono(&land(0x71))),
        ("sounds/whoosh.wav",    encode_pcm16_mono(&whoosh(150.0, 0.7, 0x33))),
        ("sounds/gi.wav",        encode_pcm16_mono(&gi_snap(0x6C1D))),
        ("sounds/block.wav",     encode_pcm16_mono(&block_sfx())),
        ("sounds/bell.wav",      encode_pcm16_mono(&bell())),
        ("sounds/ko.wav",        encode_pcm16_mono(&ko())),
        ("sounds/projectile.wav", encode_pcm16_mono(&projectile())),
    ]
}
