//! Shared techno/rave sound-design toolkit.
//!
//! Kick, hats, claps, an acid-style bass voice, and a bright lead-stab voice —
//! the building blocks every game's music module composes into its own
//! step-sequenced loop (its own BPM, pattern, and scale). Keeping the voices
//! here means every track shares one drum/bass "sound", while each game still
//! gets its own arrangement and energy level.

use std::f32::consts::PI;

use crate::wav::{env, mix_into, SAMPLE_RATE};

/// Small deterministic PRNG — no external dependency, reproducible builds.
pub struct Rng(pub u32);
impl Rng {
    pub fn next_f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / 16_777_216.0 // 0..1
    }
}

/// Punchy pitch-swept kick drum.
pub fn kick(buf: &mut [i16], off: usize, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * 0.15) as usize;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.7);
        let freq = 42.0 + 130.0 * (-t / 0.045).exp(); // pitch sweep 172Hz -> 42Hz
        let s = (2.0 * PI * freq * t).sin();
        mix_into(buf, off + i, s * e * vol * 22000.0);
    }
}

/// Closed hi-hat — short, bright noise tick.
pub fn hat(buf: &mut [i16], off: usize, rng: &mut Rng, vol: f32) {
    let n = (SAMPLE_RATE as f32 * 0.045) as usize;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let e = (1.0 - i as f32 / n as f32).powf(2.2);
        let noise = rng.next_f32() * 2.0 - 1.0;
        mix_into(buf, off + i, noise * e * vol * 11000.0);
    }
}

/// Open hi-hat — longer decay, washier than the closed hat.
pub fn open_hat(buf: &mut [i16], off: usize, rng: &mut Rng, vol: f32) {
    let n = (SAMPLE_RATE as f32 * 0.16) as usize;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let e = (1.0 - i as f32 / n as f32).powf(1.3);
        let noise = rng.next_f32() * 2.0 - 1.0;
        mix_into(buf, off + i, noise * e * vol * 9000.0);
    }
}

/// Clap — a few staggered noise bursts layered together, classic house/techno snap.
pub fn clap(buf: &mut [i16], off: usize, rng: &mut Rng, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let burst_n = (sr * 0.03) as usize;
    let spread = [0usize, (sr * 0.008) as usize, (sr * 0.018) as usize];
    for &s in &spread {
        for i in 0..burst_n {
            if off + s + i >= buf.len() { break; }
            let e = (1.0 - i as f32 / burst_n as f32).powf(1.8);
            let noise = rng.next_f32() * 2.0 - 1.0;
            mix_into(buf, off + s + i, noise * e * vol * 9000.0);
        }
    }
    // Tail wash so the clap doesn't cut off too abruptly.
    let tail_n = (sr * 0.09) as usize;
    for i in 0..tail_n {
        if off + i >= buf.len() { break; }
        let e = (1.0 - i as f32 / tail_n as f32).powf(2.5);
        let noise = rng.next_f32() * 2.0 - 1.0;
        mix_into(buf, off + i, noise * e * vol * 4000.0);
    }
}

/// Acid-style bass voice — a fat two-harmonic saw-ish tone, short and punchy.
pub fn bass_note(buf: &mut [i16], off: usize, freq: f32, ms: f32, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    let att = (sr * 0.003) as usize;
    let rel = (n / 4).max(1);
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let e = env(i, n, att.max(1), rel);
        let w = (2.0 * PI * freq * t).sin()
            - 0.5 * (2.0 * PI * freq * 2.0 * t).sin()
            + 0.25 * (2.0 * PI * freq * 3.0 * t).sin();
        mix_into(buf, off + i, w * e * vol * 13000.0);
    }
}

/// Bright additive lead/stab voice, for hooks and tension hits.
pub fn lead_stab(buf: &mut [i16], off: usize, freq: f32, ms: f32, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    let att = (sr * 0.01) as usize;
    let rel = (n * 3 / 4).max(1);
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let e = env(i, n, att.max(1), rel);
        let w = (2.0 * PI * freq * t).sin()
            + (1.0 / 3.0) * (2.0 * PI * freq * 3.0 * t).sin()
            + (1.0 / 5.0) * (2.0 * PI * freq * 5.0 * t).sin();
        mix_into(buf, off + i, w * e * vol * 9000.0);
    }
}
