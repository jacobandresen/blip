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

/// Which note of a four-note hook to play, given the bar and the position
/// within it — the difference between a riff and a phrase.
///
/// Every track here repeated its hook identically in every bar, for
/// sixteen bars. That is what makes a loop recognisable, and also what
/// makes it wear out: nothing ever arrives or resolves, so there is no
/// reason to keep listening past the second pass.
///
/// This answers the riff on the fourth bar of each four-bar phrase, in
/// the oldest form there is: the same notes, backwards and a fifth
/// higher. Four bars is the phrase length the chord changes already
/// imply, so the variation lands where the ear is expecting the phrase to
/// close rather than sounding like the melody wandered off. A perfect
/// fifth is diatonic for every hook in these games — each is built from
/// scale degrees whose fifths are also in the scale — so the answer stays
/// in key without any per-track tuning.
///
/// The tune is still the same four notes throughout, so nothing becomes
/// less recognisable; it just stops being flat.
pub fn phrase_note(hook: &[f32; 4], bar: usize, idx: usize) -> f32 {
    debug_assert!(idx < 4);
    if bar % 4 == 3 {
        hook[3 - idx] * 1.5
    } else {
        hook[idx]
    }
}

/// A sixteenth-note hat roll climbing across the second half of a bar —
/// the standard "something is about to change" cue, for the bar before a
/// section lifts. Without it the busier half simply appears, which reads
/// as the loop restarting rather than as the track going somewhere.
pub fn lift_fill(buf: &mut [f32], bar_start_off: usize, step_samples: usize, rng: &mut Rng, vol: f32) {
    for step in 8..16 {
        let off = bar_start_off + step * step_samples;
        if off >= buf.len() {
            break;
        }
        let ramp = (step - 8) as f32 / 8.0;
        hat(buf, off, rng, vol * (0.45 + 0.55 * ramp));
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

#[cfg(test)]
mod tests {
    use super::*;

    const HOOK: [f32; 4] = [523.25, 659.25, 783.99, 659.25]; // C5 E5 G5 E5

    #[test]
    fn the_first_three_bars_of_a_phrase_play_the_hook_unchanged() {
        // The tune has to stay the tune: a listener should recognise it
        // on every pass, which is the whole reason the riff repeats.
        for bar in [0, 1, 2, 4, 5, 6, 8] {
            for idx in 0..4 {
                assert_eq!(phrase_note(&HOOK, bar, idx), HOOK[idx], "bar {bar} note {idx}");
            }
        }
    }

    #[test]
    fn the_fourth_bar_answers_it_backwards_and_a_fifth_up() {
        for bar in [3, 7, 11, 15] {
            for idx in 0..4 {
                assert_eq!(phrase_note(&HOOK, bar, idx), HOOK[3 - idx] * 1.5, "bar {bar} note {idx}");
            }
        }
    }

    #[test]
    fn the_answer_stays_inside_an_octave_of_the_riff() {
        // A fifth up is deliberate; anything that lands more than an
        // octave from the original would read as a different instrument
        // rather than as the phrase closing.
        for idx in 0..4 {
            let answer = phrase_note(&HOOK, 3, idx);
            let lowest = HOOK.iter().cloned().fold(f32::MAX, f32::min);
            assert!(answer > lowest, "answer {answer} fell below the riff");
            assert!(answer < lowest * 4.0, "answer {answer} is more than two octaves up");
        }
    }

    #[test]
    fn the_lift_fill_only_touches_the_second_half_of_its_bar() {
        // It is a run-up to the next bar, not a change to this one: the
        // first half has to stay exactly as the arrangement wrote it.
        let step = 1000;
        let mut buf = vec![0f32; step * 16 * 2];
        let mut rng = Rng(1);
        lift_fill(&mut buf, 0, step, &mut rng, 0.3);
        let first_half: f32 = buf[..step * 8].iter().map(|v| v.abs()).sum();
        let second_half: f32 = buf[step * 8..step * 16].iter().map(|v| v.abs()).sum();
        assert_eq!(first_half, 0.0, "the fill wrote into the first half of the bar");
        assert!(second_half > 0.0, "the fill wrote nothing at all");
    }

    #[test]
    fn the_lift_fill_climbs() {
        // The point of the cue is that it builds; a flat run of hats
        // reads as a glitch rather than as an announcement.
        let step = 1000;
        let mut buf = vec![0f32; step * 16];
        let mut rng = Rng(7);
        lift_fill(&mut buf, 0, step, &mut rng, 0.3);
        let energy = |s: usize| -> f32 { buf[s * step..(s + 1) * step].iter().map(|v| v.abs()).sum() };
        let early = energy(8) + energy(9);
        let late = energy(14) + energy(15);
        assert!(late > early, "the fill did not get louder ({early} -> {late})");
    }

    #[test]
    fn the_lift_fill_stays_inside_the_buffer() {
        // Called with the last bar's offset on a short track, it must
        // clip rather than panic.
        let step = 1000;
        let mut buf = vec![0f32; step * 4];
        let mut rng = Rng(3);
        lift_fill(&mut buf, step * 2, step, &mut rng, 0.3);
    }
}
