//! Shared techno/rave sound-design toolkit.
//!
//! Kick, hats, claps, an acid-style bass voice, and a bright lead-stab voice —
//! the building blocks every game's music module composes into its own
//! step-sequenced loop (its own BPM, pattern, and scale). Keeping the voices
//! here means every track shares one drum/bass "sound", while each game still
//! gets its own arrangement and energy level.
//!
//! Voices write into a shared `&mut [f32]` accumulation buffer (via
//! `mix_into_f32`) rather than clamping to i16 on every write — with a kick,
//! bass, and hats all landing on the same beat, per-voice clamping would
//! hard-clip into harsh digital distortion. Callers should render into a
//! `Vec<f32>` and convert once at the end with `soft_limit_to_pcm16`.

use std::f32::consts::PI;

use crate::wav::{env, mix_into_f32, SAMPLE_RATE};

/// Default knee for `wav::soft_limit_to_pcm16` when rendering a full track
/// built from these voices — tuned so a kick + bass + hats all landing on
/// the same beat compress gracefully instead of clipping.
pub const MIX_KNEE: f32 = 24_000.0;

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

/// Punchy pitch-swept kick drum, with a short high-frequency click on the
/// attack and gentle saturation — what makes a kick punch through a dense
/// mix instead of reading as a dull sine thump.
pub fn kick(buf: &mut [f32], off: usize, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * 0.15) as usize;
    let click_n = (sr * 0.0025) as usize;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.7);
        let freq = 42.0 + 130.0 * (-t / 0.045).exp(); // pitch sweep 172Hz -> 42Hz
        let body = (2.0 * PI * freq * t).sin();
        let click = if i < click_n {
            let ce = (1.0 - i as f32 / click_n as f32).powf(1.5);
            // Cheap deterministic pseudo-noise (no Rng dependency) for the
            // transient tick — reproducible builds, no extra state to thread.
            let cn = ((i as u32).wrapping_mul(2654435761) as f32 / u32::MAX as f32) * 2.0 - 1.0;
            cn * ce * 0.6
        } else {
            0.0
        };
        let s = (body + click).tanh();
        mix_into_f32(buf, off + i, s * e * vol * 22000.0);
    }
}

/// Closed hi-hat — short, bright noise tick.
pub fn hat(buf: &mut [f32], off: usize, rng: &mut Rng, vol: f32) {
    let n = (SAMPLE_RATE as f32 * 0.045) as usize;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let e = (1.0 - i as f32 / n as f32).powf(2.2);
        let noise = rng.next_f32() * 2.0 - 1.0;
        mix_into_f32(buf, off + i, noise * e * vol * 11000.0);
    }
}

/// Open hi-hat — longer decay, washier than the closed hat.
pub fn open_hat(buf: &mut [f32], off: usize, rng: &mut Rng, vol: f32) {
    let n = (SAMPLE_RATE as f32 * 0.16) as usize;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let e = (1.0 - i as f32 / n as f32).powf(1.3);
        let noise = rng.next_f32() * 2.0 - 1.0;
        mix_into_f32(buf, off + i, noise * e * vol * 9000.0);
    }
}

/// Clap — a few staggered noise bursts layered together, classic house/techno snap.
pub fn clap(buf: &mut [f32], off: usize, rng: &mut Rng, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let burst_n = (sr * 0.03) as usize;
    let spread = [0usize, (sr * 0.008) as usize, (sr * 0.018) as usize];
    for &s in &spread {
        for i in 0..burst_n {
            if off + s + i >= buf.len() { break; }
            let e = (1.0 - i as f32 / burst_n as f32).powf(1.8);
            let noise = rng.next_f32() * 2.0 - 1.0;
            mix_into_f32(buf, off + s + i, noise * e * vol * 9000.0);
        }
    }
    // Tail wash so the clap doesn't cut off too abruptly.
    let tail_n = (sr * 0.09) as usize;
    for i in 0..tail_n {
        if off + i >= buf.len() { break; }
        let e = (1.0 - i as f32 / tail_n as f32).powf(2.5);
        let noise = rng.next_f32() * 2.0 - 1.0;
        mix_into_f32(buf, off + i, noise * e * vol * 4000.0);
    }
}

/// Acid-style bass voice — a sawtooth run through a low-pass filter whose
/// cutoff sweeps shut over the note (the classic rolling TB-303-style acid
/// motion, not a static harmonic stack), driven into gentle saturation for
/// grit, over a clean reinforcing sub-octave sine. The sub layer is what
/// makes it read as a prominent, loud bassline rather than a mid-range
/// pluck; because the caller mixes into an f32 buffer and soft-limits once
/// at the end, this can be driven hot without hard-clipping the rest of the
/// mix.
pub fn bass_note(buf: &mut [f32], off: usize, freq: f32, ms: f32, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    let att = (sr * 0.003) as usize;
    let rel = (n / 4).max(1);
    let cutoff_hi = (freq * 16.0).min(sr * 0.45);
    let cutoff_lo = freq * 2.2;
    let mut lp = 0f32;
    let mut phase = 0f32;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let e = env(i, n, att.max(1), rel);
        let close = (i as f32 / n as f32).powf(1.5);
        let cutoff = cutoff_hi + (cutoff_lo - cutoff_hi) * close;
        let alpha = (1.0 - (-2.0 * PI * cutoff / sr).exp()).clamp(0.0, 1.0);

        phase += freq / sr;
        phase -= phase.floor();
        let saw = 2.0 * phase - 1.0;
        lp += alpha * (saw - lp);
        let driven = (lp * 1.6).tanh();

        let sub = (2.0 * PI * freq * 0.5 * t).sin();
        mix_into_f32(buf, off + i, (driven * 0.85 + sub * 0.9) * e * vol * 20000.0);
    }
}

/// Sidechain "pump" — dips the buffer's level right after each kick and lets
/// it recover, the four-on-the-floor ducking that makes house/techno feel
/// like it's breathing in time with the kick instead of just stacking
/// everything on top of it. Call this on the bass/pad/hat layer *before*
/// mixing the kick hits themselves in, so the kick's own transient isn't
/// ducked by its own hit.
pub fn sidechain_duck(buf: &mut [f32], kick_offsets: &[usize], depth: f32, release_ms: f32) {
    let sr = SAMPLE_RATE as f32;
    let rel_n = ((sr * release_ms / 1000.0) as usize).max(1);
    for &off in kick_offsets {
        for i in 0..rel_n {
            if off + i >= buf.len() { break; }
            let frac = i as f32 / rel_n as f32;
            let duck = 1.0 - depth * (1.0 - frac).powf(2.5);
            buf[off + i] *= duck;
        }
    }
}

/// Bright additive lead/stab voice, for hooks and tension hits.
pub fn lead_stab(buf: &mut [f32], off: usize, freq: f32, ms: f32, vol: f32) {
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
        mix_into_f32(buf, off + i, w * e * vol * 9000.0);
    }
}

/// Supersaw — a stack of seven detuned sawtooth oscillators, the wide,
/// chorus-y trance/EDM lead sound. `att_ms` controls character: short
/// (~10ms) reads as a plucked lead/arp note, long (~200ms+) reads as a
/// swelling breakdown pad. `detune` is the spread as a fraction of `freq`
/// (0.006-0.01 is a classic supersaw width; wider gets dissonant/chorusy).
pub fn supersaw(buf: &mut [f32], off: usize, freq: f32, ms: f32, vol: f32, att_ms: f32, detune: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    let att = ((sr * att_ms / 1000.0) as usize).max(1);
    let rel = (n / 3).max(1);
    const SPREAD: [f32; 7] = [-1.0, -0.66, -0.33, 0.0, 0.33, 0.66, 1.0];
    let mut phases = [0f32; 7];
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let e = env(i, n, att, rel);
        let mut s = 0f32;
        for (v, ph) in SPREAD.iter().zip(phases.iter_mut()) {
            let f = freq * (1.0 + v * detune);
            *ph += f / sr;
            *ph -= ph.floor();
            s += 2.0 * *ph - 1.0;
        }
        s /= SPREAD.len() as f32;
        mix_into_f32(buf, off + i, s * e * vol * 16000.0);
    }
}

/// Buildup riser — filtered noise that sweeps its cutoff upward and swells
/// in volume over `dur_ms`, the classic trance transition from breakdown
/// into the drop. Meant to span a bar or several, ending right as the drop
/// hits.
pub fn riser(buf: &mut [f32], off: usize, dur_ms: f32, vol: f32, rng: &mut Rng) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * dur_ms / 1000.0) as usize;
    let mut lp = 0f32;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let frac = i as f32 / n as f32;
        let e = frac.powf(1.5);
        let cutoff = 200.0 + 9000.0 * frac.powf(1.8);
        let alpha = (1.0 - (-2.0 * PI * cutoff / sr).exp()).clamp(0.0, 1.0);
        let white = rng.next_f32() * 2.0 - 1.0;
        lp += alpha * (white - lp);
        mix_into_f32(buf, off + i, lp * e * vol * 14000.0);
    }
}
