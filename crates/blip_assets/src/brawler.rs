//! Brawler assets: sounds only. The fighters are drawn by the game from the
//! same numbers as their hitboxes, so there is no sprite sheet to disagree
//! with the rules.

use crate::wav::{fade_out, low_pass, tame, warm, Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_half, encode_pcm16_mono, encode_pcm16_music, env, ms_to_samples,
    soft_limit_to_pcm16, SAMPLE_RATE};
use crate::Asset;

// ---- combat foley ---------------------------------------------------------

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
    fade_out(&mut buf, ms_to_samples(10.0));
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

/// Synthesize a formant-shaped voice from `f0` to `f1`; `breath` controls the noise mix.
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
    render(buf, 20_000.0)
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
    render(buf, 24_000.0)
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
    let mut buf = vec![0.0f32; (0.5 * SRF) as usize];
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
        a += 2.0 * std::f32::consts::PI * f / SRF;
        b += 2.0 * std::f32::consts::PI * f * 1.035 / SRF;
        let hiss = (rng.next_f32() * 2.0 - 1.0) * 0.18 * (1.0 - t);
        *out = (a.sin() * 0.5 + b.sin() * 0.4 + hiss) * e * 7000.0;
    }
    soften(&mut buf, 4200.0);
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// The ground giving under a giant: a long low rumble, the boards jumping
/// twice, and grit coming down after.
fn quake() -> Vec<i16> {
    let mut rng = Rng(0x9A4E);
    let mut buf = vec![0.0f32; (1.0 * SRF) as usize];
    band(&mut buf, 0.0, 62.0, 1.4, 4.0, 0.004, 0.16, &mut rng);
    band(&mut buf, 0.0, 120.0, 1.0, 2.2, 0.002, 0.07, &mut rng);
    knock(&mut buf, 0.0, 70.0, 1.6, 0.06);
    band(&mut buf, 0.14, 90.0, 1.2, 1.8, 0.004, 0.1, &mut rng);
    band(&mut buf, 0.0, 420.0, 0.9, 0.5, 0.001, 0.03, &mut rng);
    band(&mut buf, 0.2, 1500.0, 0.7, 0.12, 0.08, 0.12, &mut rng);
    soften(&mut buf, 1800.0);
    render(buf, 6_200.0)
}

/// A blow turned aside: steel on steel, bright and gone, over the knock of
/// the forearms. Nothing like a hit, nothing like a block.
fn parry() -> Vec<i16> {
    let mut rng = Rng(0x7E4C);
    let mut buf = vec![0.0f32; (0.5 * SRF) as usize];
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
    let mut buf = vec![0.0f32; (0.22 * SRF) as usize];
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
        let t = i as f32 / SRF;
        // 0.16 s on the low note, a quick slide, the rest on the high one.
        let k = ((t - 0.16) / 0.05).clamp(0.0, 1.0);
        let f = 1250.0 + 650.0 * k * k * (3.0 - 2.0 * k);
        ph += 2.0 * std::f32::consts::PI * f / SRF;
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
    let mut buf = vec![0.0f32; (0.36 * SRF) as usize];
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
    let mut buf = vec![0.0f32; (0.8 * SRF) as usize];
    voice(&mut buf, 0.0, 0.7, 96.0, 70.0, AH, 0.25, 1.3, &mut rng);
    voice(&mut buf, 0.01, 0.66, 49.0, 36.0, OO, 0.15, 0.9, &mut rng);
    band(&mut buf, 0.0, 300.0, 0.8, 0.25, 0.05, 0.12, &mut rng);
    soften(&mut buf, 2200.0);
    render(buf, 13_000.0)
}

/// A swing with a body's whole weight behind it: lower and longer than the
/// ordinary one, and it ends in a clap of air.
fn heavy_swing() -> Vec<i16> {
    let mut rng = Rng(0x5E11);
    let mut buf = vec![0.0f32; (0.42 * SRF) as usize];
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
    let mut buf = vec![0.0f32; (0.5 * SRF) as usize];
    knock(&mut buf, 0.0, 170.0, 1.4, 0.02);
    band(&mut buf, 0.0, 420.0, 1.0, 1.6, 0.001, 0.03, &mut rng);
    for k in 0..6 {
        let at = 0.012 + 0.022 * k as f32;
        band(&mut buf, at, 1100.0 + 380.0 * ((k * 5) % 4) as f32, 2.2, 0.9 - 0.1 * k as f32, 0.0004, 0.006, &mut rng);
    }
    for (at, f) in [(0.16f32, 1900.0f32), (0.24, 2300.0)] { knock(&mut buf, at, f, 0.25, 0.03); }
    render(buf, 8_000.0)
}

/// Birds round the head: four chirps, each a quick slide up.
fn tweet() -> Vec<i16> {
    let n = ms_to_samples(520.0);
    let mut buf = vec![0.0f32; n];
    let mut ph = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / SRF;
        let k = (t / 0.13).fract();
        let f = 1500.0 + 700.0 * k + 120.0 * ((t / 0.13).floor() % 2.0);
        ph += 2.0 * std::f32::consts::PI * f / SRF;
        let e = (k / 0.1).min(1.0) * (1.0 - k).powf(1.5) * (1.0 - t / 0.52);
        *out = ph.sin() * e * 3900.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// A wood block: the menu moving a step.
fn tick() -> Vec<i16> {
    let mut rng = Rng(0x71C4);
    let mut buf = vec![0.0f32; (0.09 * SRF) as usize];
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
    for (freq, amp, tau) in [(110.0f32, 1.0f32, 0.55f32), (164.0, 0.7, 0.5), (233.0, 0.55, 0.42),
                             (331.0, 0.4, 0.34), (447.0, 0.25, 0.26), (452.0, 0.2, 0.26)] {
        for (i, out) in buf.iter_mut().enumerate() {
            let t = i as f32 / SRF;
            // The shimmer blooms a moment after the stroke.
            let swell = (t / (0.02 + 60.0 / freq / 100.0)).min(1.0);
            *out += (2.0 * std::f32::consts::PI * freq * t).sin() * (-t / tau).exp() * swell * amp * 3200.0;
        }
    }
    let mut thump = vec![0.0f32; n];
    band(&mut thump, 0.0, 140.0, 1.0, 1.0, 0.001, 0.02, &mut rng);
    for (out, x) in buf.iter_mut().zip(&thump) { *out += x * 5000.0; }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// Plucked string using Karplus–Strong averaging; upper harmonics decay first.
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
    // A long (low) string has few trips round the line to lose its hiss in,
    // so it starts rounder: a bass note should be a note, not a thwack.
    for _ in 0..period / 150 {
        let first = line[0];
        for i in 0..period - 1 { line[i] = (line[i] + line[i + 1]) * 0.5; }
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

/// Five-note scales in semitones from the root. Hirajoshi's minor second is
/// what makes it sound Japanese; yo is the bright folk scale without one;
/// in is the dark one; the minor pentatonic is the one a rock band plays.
const HIRAJOSHI: [f32; 5] = [0.0, 1.0, 5.0, 7.0, 8.0];
const YO: [f32; 5] = [0.0, 2.0, 5.0, 7.0, 9.0];
const IN: [f32; 5] = [0.0, 1.0, 5.0, 7.0, 10.0];
const MINOR: [f32; 5] = [0.0, 3.0, 5.0, 7.0, 10.0];
/// The major pentatonic, open and heroic; and the minor with its fifth
/// flattened, which is the blues.
const MAJOR: [f32; 5] = [0.0, 2.0, 4.0, 7.0, 9.0];
const BLUES: [f32; 5] = [0.0, 3.0, 5.0, 6.0, 10.0];

fn degree(scale: &[f32; 5], root: f32, step: i32) -> f32 {
    let oct = step.div_euclid(5);
    let d = scale[step.rem_euclid(5) as usize];
    root * 2.0f32.powf((d + 12.0 * oct as f32) / 12.0)
}

/// A shakuhachi: a breathy tone that swells in, wavers once it is held and
/// dies away, with the air of the player's breath round it.
fn flute(buf: &mut [f32], off: usize, freq: f32, ms: f32, gain: f32, rng: &mut Rng) {
    let n = ms_to_samples(ms).min(buf.len().saturating_sub(off));
    let (mut ph, mut air) = (0.0f32, 0.0f32);
    for i in 0..n {
        let t = i as f32 / SRF;
        let k = i as f32 / n as f32;
        // Blown from just under the note, as the player's lip finds it.
        let scoop = 1.0 - 0.03 * (-t / 0.05).exp();
        let vib = 1.0 + 0.007 * (t / 0.35).min(1.0) * (2.0 * std::f32::consts::PI * 5.2 * t).sin();
        ph += 2.0 * std::f32::consts::PI * freq * scoop * vib / SRF;
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
        let t = i as f32 / SRF;
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
enum Feel { Rim, March, Push, Chime }

/// A theme sequenced by `order` rather than looped: eight slots of four bars
/// outlast the round, so no seam is heard.
impl Theme<'_> {
fn pcm(&self) -> Vec<i16> {
    let Theme { bpm, root, scale, phrases, bass, order, drive, feel } = *self;
    let step = (SAMPLE_RATE as f32 * 60.0 / bpm / 4.0) as usize; // 16ths
    let bars = 4 * order.len();
    let steps = bars * 16;
    // Room past the end for the last notes to ring out in, which is
    // then folded back onto the top — see `wrap_tail`.
    let body = step * steps;
    let n = body + ms_to_samples(1400.0);
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

        // The fight under the tune, as hard as the stage drives: a low drum
        // on every beat, a bass string plucked on every eighth (the octave
        // on the off-beats), and sticks on the sixteenths at full drive.
        let ground = line[bar % line.len()];
        if drive >= 0.4 && !hush {
            if b % 4 == 0 && b != 0 { taiko(&mut buf, off, 84.0, 0.42 * drive, &mut rng); }
            if b % 2 == 0 && !(lull && b >= 8) {
                let f = degree(scale, root, ground) * if b % 4 == 2 { 0.5 } else { 0.25 };
                pluck(&mut buf, off, f, beat_ms * 0.45, 0.5 * drive, &mut rng);
            }
            if drive > 0.8 && b % 2 == 1 && !lull { rim(&mut buf, off, 0.14, &mut rng); }
        }

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
        match feel {
            // A parade ground: the rim on every beat, and a roll into each
            // new phrase.
            Feel::March => {
                if b % 4 == 0 && !hush { rim(&mut buf, off, 0.36, &mut rng); }
                if b % 4 == 2 { rim(&mut buf, off, 0.5, &mut rng); }
                if last_bar && b >= 12 { rim(&mut buf, off, 0.22 + 0.06 * (b - 12) as f32, &mut rng); }
            }
            // The city: every rim pushed a sixteenth ahead of where it is
            // expected, and a soft drum in the gap.
            Feel::Push => {
                if b % 4 == 3 && !(lull && b == 11) { rim(&mut buf, off, 0.46, &mut rng); }
                if b == 6 && !lull { taiko(&mut buf, off, 132.0, 0.4, &mut rng); }
            }
            // Ice: no sticks at all, and a chime answering the drum from
            // high above the tune.
            Feel::Chime => {
                if b % 8 == 4 {
                    let d = line[bar % line.len()];
                    chime(&mut buf, off, degree(scale, root, d + 4) * 2.0, if lull { 0.35 } else { 0.5 });
                }
            }
            // Rim on the off-beats, denser on a driving stage.
            Feel::Rim => {
                if b % 4 == 2 && !(lull && b == 10) { rim(&mut buf, off, 0.5, &mut rng); }
                if b % 8 == 7 && !hush { rim(&mut buf, off, 0.32, &mut rng); }
                if drive > 0.5 && b % 4 == 3 && !lull { rim(&mut buf, off, 0.18, &mut rng); }
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
                pluck(&mut buf, off + j * ms_to_samples(14.0), degree(scale, root, d + up),
                    beat_ms * 2.4, 0.3, &mut rng);
            }
        }

        // The flute answers where the koto leaves room: over the quiet bar
        // and the cadence, and all through a hushed phrase. It holds a note
        // of the bar's chord an octave above the tune.
        let d = line[bar % line.len()];
        if b == 0 && (lull || last_bar) {
            let up = if last_bar { 0 } else { 2 };
            flute(&mut buf, off, degree(scale, root, d + up) * 2.0, beat_ms * 3.4,
                if hush { 0.8 } else { 0.6 }, &mut rng);
        }

        // The bass walks a note a bar.
        if b == 0 {
            pluck(&mut buf, off, degree(scale, root, d) * 0.5, beat_ms * 3.6, 0.55, &mut rng);
        }
        if b == 10 && !lull {
            pluck(&mut buf, off, degree(scale, root, d + 2) * 0.5, beat_ms * 1.2, 0.3, &mut rng);
        }
    }
    wrap_tail(&mut buf, body);
    // Fight music: brighter than the house warmth, and as loud as the other
    // games' (the koto alone sat 12 dB under them).
    low_pass(&mut buf[..body], 6500.0);
    for v in buf[..body].iter_mut() { *v *= 2.6; }
    soft_limit_to_pcm16(&buf[..body], MIX_KNEE)
}
}

/// Fold what rings out past the end of the loop onto its start, so tails
/// carry over the loop point instead of the music stopping dead.
fn wrap_tail(buf: &mut [f32], body: usize) {
    for i in body..buf.len() {
        buf[i - body] += buf[i];
    }
}


/// Device-synthesized themes by stage: docks, temple, select, air base, bath house, river, fortress, rooftop.
pub fn theme_wav(which: usize) -> Vec<u8> {
    // The docks, the temple and the select screen are on hirajoshi and differ
    // in pace and density; each later stage has a scale of its own.
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

    // -- the air base: a march on the minor pentatonic -------------------
    let a_a = [0, R, 0, R, 2, R, 3, R, 4, R, R, 3, R, R, 2, R,
               0, R, 0, R, 2, R, 3, R, 5, R, R, 4, R, R, 3, R,
               4, R, 4, R, 5, R, 6, R, 5, R, R, 4, R, R, 3, R,
               2, R, 3, R, 4, R, R, 2, 0, R, R, R, R, R, R, R];
    let a_b = [5, R, R, R, 4, R, 5, R, 6, R, R, R, 5, R, 4, R,
               3, R, R, R, 2, R, 3, R, 4, R, R, R, R, R, 2, R,
               5, R, R, R, 4, R, 5, R, 6, R, R, 5, R, R, 4, R,
               3, R, 2, R, 3, R, 4, R, 5, R, R, R, R, R, R, R];
    let a_c = [0, R, R, 2, R, R, 3, R, 4, R, R, R, 3, R, 2, R,
               0, R, R, 2, R, R, 3, R, 2, R, R, R, R, R, R, R,
               3, R, R, 4, R, R, 5, R, 6, R, R, R, 5, R, 4, R,
               3, R, R, 2, R, R, 0, R, 0, R, R, R, R, R, R, R];
    let a_d = [0, R, R, R, R, R, R, R, 3, R, R, R, R, R, R, R,
               2, R, R, R, R, R, R, R, 0, R, R, R, R, R, R, R,
               4, R, R, R, R, R, R, R, 3, R, R, R, R, R, R, R,
               2, R, R, R, R, R, 3, R, 0, R, R, R, R, R, R, R];

    // -- the bath house: bright and easy, on yo --------------------------
    let h_a = [2, R, R, 3, 4, R, R, R, 3, R, 2, R, 0, R, R, R,
               2, R, R, 3, 4, R, 5, R, 4, R, R, R, R, R, 3, R,
               2, R, R, 3, 4, R, R, R, 5, R, 4, R, 3, R, R, 2,
               3, R, 2, R, 1, R, R, 2, 0, R, R, R, R, R, R, R];
    let h_b = [5, R, R, R, 4, R, 3, R, 4, R, R, R, 2, R, R, R,
               3, R, R, R, 2, R, 1, R, 2, R, R, R, R, R, R, R,
               5, R, R, R, 4, R, 3, R, 4, R, R, 5, 6, R, R, R,
               5, R, 4, R, 3, R, 2, R, 0, R, R, R, R, R, R, R];
    let h_c = [0, R, 2, R, 3, R, 2, R, 0, R, R, R, R, R, 1, R,
               0, R, 2, R, 3, R, 4, R, 3, R, R, R, R, R, R, R,
               4, R, 3, R, 2, R, 3, R, 4, R, R, R, 5, R, R, R,
               4, R, R, 3, R, R, 2, R, 0, R, R, R, R, R, R, R];
    let h_d = [2, R, R, R, R, R, R, R, 3, R, R, R, R, R, R, R,
               4, R, R, R, R, R, R, R, 2, R, R, R, R, R, R, R,
               3, R, R, R, R, R, R, R, 1, R, R, R, R, R, R, R,
               0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R];

    // -- the river village: slow and heavy, on in ------------------------
    let v_a = [0, R, R, R, 0, R, 1, R, 2, R, R, R, R, R, R, R,
               3, R, R, R, 2, R, 1, R, 0, R, R, R, R, R, R, R,
               0, R, R, R, 0, R, 1, R, 2, R, R, R, 3, R, R, R,
               4, R, R, 3, R, R, 2, R, 0, R, R, R, R, R, R, R];
    let v_b = [4, R, R, R, 3, R, R, R, 4, R, R, R, 5, R, R, R,
               4, R, R, 3, R, R, 2, R, 3, R, R, R, R, R, R, R,
               2, R, R, R, 3, R, R, R, 4, R, R, R, 6, R, R, R,
               5, R, R, 4, R, R, 3, R, 2, R, R, R, R, R, R, R];
    let v_c = [0, R, 2, R, R, R, 3, R, R, R, 2, R, 0, R, R, R,
               0, R, 2, R, R, R, 3, R, R, R, 4, R, 3, R, R, R,
               5, R, R, R, 4, R, R, R, 3, R, R, R, 2, R, R, R,
               1, R, R, R, 0, R, R, R, 0, R, R, R, R, R, R, R];
    let v_d = [0, R, R, R, R, R, R, R, R, R, R, R, 1, R, R, R,
               2, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R,
               3, R, R, R, R, R, R, R, 2, R, R, R, 1, R, R, R,
               0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R];

    // -- the crystal fortress: slow and open, on the major pentatonic ------
    let f_a = [0, R, R, R, 3, R, R, R, R, R, R, R, 4, R, 3, R,
               5, R, R, R, R, R, R, R, 4, R, R, R, 3, R, R, R,
               0, R, R, R, 3, R, R, R, R, R, R, R, 4, R, 5, R,
               6, R, R, R, R, R, 5, R, 5, R, R, R, R, R, R, R];
    let f_b = [5, R, R, R, 4, R, R, R, 3, R, R, R, R, R, 2, R,
               3, R, R, R, R, R, R, R, 2, R, R, R, 0, R, R, R,
               5, R, R, R, 4, R, R, R, 3, R, R, R, 4, R, 5, R,
               6, R, R, R, R, R, R, R, 5, R, R, R, R, R, R, R];
    let f_c = [2, R, R, R, 3, R, R, R, 4, R, R, R, R, R, R, R,
               3, R, R, R, 4, R, R, R, 5, R, R, R, R, R, R, R,
               4, R, R, R, 5, R, R, R, 6, R, R, R, R, R, 5, R,
               4, R, R, R, 3, R, R, R, 0, R, R, R, R, R, R, R];
    let f_d = [0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R,
               3, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R,
               2, R, R, R, R, R, R, R, 4, R, R, R, R, R, R, R,
               0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R];

    // -- the rooftop: pushed off the beat, on the blues -------------------
    let q_a = [0, R, R, 0, R, R, 1, R, 2, R, 3, R, 2, R, 1, R,
               0, R, R, R, R, R, 4, R, R, R, 3, R, 2, R, R, R,
               0, R, R, 0, R, R, 1, R, 2, R, 3, R, 4, R, 5, R,
               4, R, R, 3, R, R, 2, R, 0, R, R, R, R, R, R, R];
    let q_b = [5, R, R, R, 4, R, R, 5, R, R, 4, R, 3, R, 2, R,
               4, R, R, R, 3, R, R, 4, R, R, 3, R, 2, R, 1, R,
               5, R, R, R, 4, R, R, 5, R, R, 6, R, 5, R, 4, R,
               3, R, 2, R, 1, R, 2, R, 0, R, R, R, R, R, R, R];
    let q_c = [2, R, 2, R, R, R, 3, R, 2, R, R, R, 1, R, 0, R,
               2, R, 2, R, R, R, 3, R, 4, R, R, R, R, R, R, R,
               5, R, 5, R, R, R, 4, R, 3, R, R, R, 2, R, 1, R,
               0, R, R, 1, R, R, 2, R, 0, R, R, R, R, R, R, R];
    let q_d = [0, R, R, R, R, R, R, R, 2, R, R, R, R, R, 1, R,
               0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R,
               3, R, R, R, R, R, R, R, 2, R, R, R, R, R, 1, R,
               0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R];

    // A note a bar under each, so the ground moves.
    let b0 = [0, 0, 3, 2];
    let b1 = [2, 3, 4, 2];
    let b2 = [0, 4, 3, 0];
    let b3 = [0, 0, 0, 0];

    // Eight slots of four bars: a rondo, so the tune keeps coming back
    // without the piece repeating. Slot 3 is the hushed one.
    let order = [0usize, 1, 0, 3, 2, 1, 0, 1];
    let bass: [&[i32]; 4] = [&b0, &b1, &b2, &b3];
    let stage = |bpm, root, scale, phrases: [&[i32]; 4], drive, feel| Theme {
        bpm, root, scale, phrases: &phrases, bass: &bass, order: &order, drive, feel,
    }.pcm();
    encode_pcm16_music(&match which {
        0 => stage(148.0, 196.00, &HIRAJOSHI, [&d_a, &d_b, &d_c, &d_d], 1.0, Feel::Rim),
        1 => stage(128.0, 220.00, &HIRAJOSHI, [&t_a, &t_b, &t_c, &t_d], 0.6, Feel::Rim),
        3 => stage(156.0, 174.61, &MINOR, [&a_a, &a_b, &a_c, &a_d], 1.0, Feel::March),
        4 => stage(136.0, 196.00, &YO, [&h_a, &h_b, &h_c, &h_d], 0.7, Feel::Rim),
        5 => stage(124.0, 164.81, &IN, [&v_a, &v_b, &v_c, &v_d], 0.8, Feel::Rim),
        6 => stage(120.0, 196.00, &MAJOR, [&f_a, &f_b, &f_c, &f_d], 0.5, Feel::Chime),
        7 => stage(142.0, 185.00, &BLUES, [&q_a, &q_b, &q_c, &q_d], 0.9, Feel::Push),
        _ => Theme { bpm: 118.0, root: 207.65, scale: &HIRAJOSHI, phrases: &[&s_a, &s_b],
                     bass: &[&b2, &b0], order: &[0, 1, 0, 1], drive: 0.0, feel: Feel::Rim }.pcm(),
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
            let crowd = (lp - hp) * (1.0 + 0.25 * turn(3.0, 0.0) + 0.18 * turn(7.0, 0.3) + 0.12 * turn(11.0, 0.7)) * 5200.0;
            let secs = t * 4.0;
            crowd + match kind {
                // Crickets: a high note chopped into chirps, three to a burst.
                2 => {
                    let burst = (secs * 2.5).fract() < 0.45;
                    let chop = (secs * 31.0).fract() < 0.5;
                    if burst && chop { (std::f32::consts::TAU * 3900.0 * secs).sin() * 420.0 } else { 0.0 }
                }
                // The city: a low drone that beats slowly against itself.
                3 => ((std::f32::consts::TAU * 58.0 * secs).sin() + (std::f32::consts::TAU * 61.25 * secs).sin()) * 800.0,
                _ => 0.0,
            }
        } else {
            // Wind: a narrow band whose pitch rises and falls with the gusts.
            let gust = 0.5 + 0.5 * turn(1.0, 0.0) * turn(3.0, 0.2);
            let f = 2.0 * (std::f32::consts::PI * (260.0 + 420.0 * gust) / SRF).sin();
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

/// A heart going hard: two low thuds and a rest, to loop while a fighter is
/// nearly out.
fn heartbeat() -> Vec<i16> {
    let mut rng = Rng(0x4EA7);
    let mut buf = vec![0.0f32; (0.86 * SRF) as usize];
    for (at, amp) in [(0.0f32, 1.0f32), (0.21, 0.7)] {
        knock(&mut buf, at, 58.0, amp * 1.6, 0.05);
        band(&mut buf, at, 90.0, 1.2, amp * 1.2, 0.004, 0.03, &mut rng);
    }
    soften(&mut buf, 900.0);
    let scaled: Vec<f32> = buf.iter().map(|v| v * 3300.0).collect();
    soft_limit_to_pcm16(&scaled, MIX_KNEE)
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
        ("sounds/hit_light.wav",  encode_pcm16_half(&hit(300.0, 0.15, 0x11))),
        ("sounds/hit_light2.wav", encode_pcm16_half(&hit(380.0, 0.2, 0x1b))),
        ("sounds/hit_light3.wav", encode_pcm16_half(&hit(470.0, 0.25, 0x2f))),
        // A second take of each, a shade lower and from another seed: the
        // same blow twice running should not be the same recording twice.
        ("sounds/hit_light_b.wav",  encode_pcm16_half(&hit(284.0, 0.18, 0x51))),
        ("sounds/hit_light2_b.wav", encode_pcm16_half(&hit(360.0, 0.23, 0x5b))),
        ("sounds/hit_light3_b.wav", encode_pcm16_half(&hit(446.0, 0.28, 0x6f))),
        ("sounds/hit_heavy_b.wav",  encode_pcm16_half(&hit(216.0, 1.0, 0x62))),
        ("sounds/hit_heavy.wav",  encode_pcm16_half(&hit(230.0, 1.0, 0x22))),
        // A knockdown: a body on the boards, lower and longer than any hit.
        ("sounds/crunch.wav",    encode_pcm16_half(&crunch())),
        ("sounds/land.wav",      encode_pcm16_mono(&land(0x71))),
        ("sounds/whoosh.wav",    encode_pcm16_half(&whoosh(150.0, 0.7, 0x33))),
        ("sounds/gi.wav",        encode_pcm16_mono(&gi_snap(0x6C1D))),
        ("sounds/block.wav",     encode_pcm16_mono(&block_sfx())),
        ("sounds/bell.wav",      encode_pcm16_half(&bell())),
        ("sounds/ko.wav",        encode_pcm16_half(&ko())),
        ("sounds/projectile.wav", encode_pcm16_half(&projectile())),
        // Stored at half the rate: everything whose energy above 10 kHz is
        // 33 dB or more under the rest (render() rolls off from 5 kHz). The
        // snaps, whistles and swishes keep the full rate.
        ("sounds/laser.wav",     encode_pcm16_half(&laser())),
        ("sounds/quake.wav",     encode_pcm16_music(&quake())),
        ("sounds/parry.wav",     encode_pcm16_half(&parry())),
        ("sounds/counter.wav",   encode_pcm16_half(&counter())),
        ("sounds/fruit.wav",     encode_pcm16_mono(&run_up(&[0, 2, 3, 5], 220.0, 55.0, 420.0, 0xF2))),
        ("sounds/tick.wav",      encode_pcm16_half(&tick())),
        ("sounds/gong.wav",      encode_pcm16_music(&gong())),
        ("sounds/bellow.wav",    encode_pcm16_music(&bellow())),
        ("sounds/swing.wav",     encode_pcm16_mono(&heavy_swing())),
        ("sounds/drums.wav",     encode_pcm16_music(&drum_call())),
        ("sounds/smash.wav",     encode_pcm16_half(&smash())),
        ("sounds/tweet.wav",     encode_pcm16_half(&tweet())),
        ("sounds/heart.wav",     encode_pcm16_music(&heartbeat())),
        ("sounds/whistle.wav",   encode_pcm16_mono(&whistle())),
        ("sounds/thwip.wav",     encode_pcm16_mono(&thwip())),
        ("sounds/win.wav",       encode_pcm16_music(&verdict(true))),
        ("sounds/lose.wav",      encode_pcm16_music(&verdict(false))),
    ]
}
