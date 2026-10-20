//! Brawler assets: sounds only. The fighters are drawn by the game from the
//! same numbers as their hitboxes, so there is no sprite sheet to disagree
//! with the rules.

use crate::wav::{fade_out, warm, Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_half, encode_pcm16_mono, encode_pcm16_music, env, ms_to_samples,
    soft_limit_to_pcm16, SAMPLE_RATE};
use crate::Asset;

#[path = "brawler/gfx.rs"]
pub mod gfx;

#[path = "brawler/tunes.rs"]
pub mod tunes;
pub use tunes::{ambience_wav, crowd_wav, theme_wav};
use tunes::{degree, flute, pluck, rim, taiko, HIRAJOSHI, YO};

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

pub fn generate() -> Vec<Asset> {
    let mut assets = gfx::generate();
    assets.extend(vec![
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
    ]);
    assets
}
