//! Meteors music — a driving four-on-the-floor techno loop.
//! Kick + hat + a syncopated acid-style bassline + tension stabs, all synthesized.

use std::f32::consts::PI;

use crate::techno::{bass_note, hat, kick, lead_stab, Rng};
use crate::wav::{encode_pcm16_mono, env, mix_into, SAMPLE_RATE};
use crate::Asset;

const BPM: f32 = 128.0;
const STEPS_PER_BAR: usize = 16; // 16th-note grid
const BARS: usize = 8;
const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;

fn music() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0i16; total];
    let mut rng = Rng(0xC0FF_EE42);

    // A-phrygian bassline roots, one per 2-bar section: A1 - C2 - D2 - C2.
    let bass_roots = [55.00_f32, 65.41, 73.42, 65.41];
    // 16-step syncopated acid pattern: which steps trigger a bass hit.
    const BASS_HIT: [bool; STEPS_PER_BAR] = [
        true, false, true, false, false, true, false, true,
        true, false, false, true, false, true, false, true,
    ];
    // Octave-jump accent on some hits, for movement (acid-line character).
    const BASS_OCT: [f32; STEPS_PER_BAR] = [
        1.0, 1.0, 2.0, 1.0, 1.0, 1.0, 1.0, 2.0,
        1.0, 1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 1.0,
    ];
    // Tension stab notes cycling every 2 bars (A4, C5, D5, Bb4 — Phrygian bite).
    let stab_notes = [440.00_f32, 523.25, 587.33, 466.16];

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;

        // Four-on-the-floor kick.
        if pos % 4 == 0 {
            kick(&mut buf, off, 0.95);
        }
        // Off-beat closed hat, plus a 16th-note roll into the last beat of every 4th bar.
        if pos % 4 == 2 {
            hat(&mut buf, off, &mut rng, 0.30);
        }
        if bar % 4 == 3 && pos >= 12 {
            hat(&mut buf, off, &mut rng, 0.22);
        }
        // Acid bassline.
        if BASS_HIT[pos] {
            let root = bass_roots[(bar / 2) % bass_roots.len()];
            bass_note(&mut buf, off, root * BASS_OCT[pos], step_ms * 0.85, 0.34);
        }
        // Tension stab on the downbeat of every odd bar.
        if bar % 2 == 1 && pos == 0 {
            let note = stab_notes[(bar / 2) % stab_notes.len()];
            lead_stab(&mut buf, off, note, step_ms * 7.0, 0.20);
        }
    }

    encode_pcm16_mono(&buf)
}

/// Laser-zap fire sound: a fast downward pitch sweep with a bright buzzy
/// saw/square blend on top, plus a touch of noise sizzle for bite.
fn fire_zap() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let dur_ms = 110.0_f32;
    let n = (sr * dur_ms / 1000.0) as usize;
    let att = (sr * 0.002) as usize;
    let rel = (n * 2 / 3).max(1);
    let mut buf = vec![0i16; n];
    let mut rng = Rng(0xFEED_0001);
    let mut phase = 0.0_f32;
    for i in 0..n {
        let t = i as f32 / sr;
        let f = 1900.0 * (-t / 0.045).exp() + 420.0; // sweep 2320Hz -> ~420Hz
        phase += f / sr;
        let ph = phase.fract();
        // Blend a sine (body) with a saw (bite) for a brighter buzz than a pure tone.
        let sine = (2.0 * PI * phase).sin();
        let saw = 2.0 * ph - 1.0;
        let sizzle = (rng.next_f32() * 2.0 - 1.0) * 0.12;
        let e = env(i, n, att.max(1), rel);
        let s = (sine * 0.6 + saw * 0.3 + sizzle) * e * 11000.0;
        mix_into(&mut buf, i, s);
    }
    encode_pcm16_mono(&buf)
}

/// Ship-destruction explosion: sub-bass thump, a broadband noise blast that
/// darkens over time (simple one-pole low-pass), and a short bright crackle on top.
fn ship_explosion() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let dur_ms = 650.0_f32;
    let n = (sr * dur_ms / 1000.0) as usize;
    let mut buf = vec![0i16; n];
    let mut rng = Rng(0xB00B_1E5);

    // Sub-bass thump: fast downward pitch sweep, punchy attack.
    for i in 0..n {
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.4);
        let freq = 30.0 + 90.0 * (-t / 0.09).exp();
        let s = (2.0 * PI * freq * t).sin();
        mix_into(&mut buf, i, s * e * 18000.0);
    }

    // Broadband noise blast, low-passed with a decaying cutoff so it darkens
    // from a sharp crack into a dull rumble as the explosion dies out.
    let mut lp = 0.0_f32;
    for i in 0..n {
        let progress = i as f32 / n as f32;
        let e = (1.0 - progress).powf(1.7);
        let cutoff = 0.55 - 0.45 * progress; // one-pole coefficient, closes over time
        let white = rng.next_f32() * 2.0 - 1.0;
        lp += (white - lp) * cutoff;
        mix_into(&mut buf, i, lp * e * 14000.0);
    }

    // Bright crackle on top for the first ~90ms — the initial "crack" of debris.
    let crackle_n = (sr * 0.09) as usize;
    for i in 0..crackle_n.min(n) {
        let e = (1.0 - i as f32 / crackle_n as f32).powf(2.5);
        let noise = rng.next_f32() * 2.0 - 1.0;
        mix_into(&mut buf, i, noise * e * 9000.0);
    }

    encode_pcm16_mono(&buf)
}

pub fn generate() -> Vec<Asset> {
    vec![
        ("sounds/techno.wav", music()),
        ("sounds/fire.wav", fire_zap()),
        ("sounds/ship_explosion.wav", ship_explosion()),
    ]
}
