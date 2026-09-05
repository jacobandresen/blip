//! Meteors music — a driving four-on-the-floor techno loop.
//! Kick + hat + a syncopated acid-style bassline + tension stabs, all synthesized.

use std::f32::consts::PI;

use crate::techno::{
    bass_note, clap, hat, kick, lead_stab, open_hat, riser, sidechain_duck, supersaw, Rng,
    MIX_KNEE,
};
use crate::wav::{encode_pcm16_mono, env, mix_into, soft_limit_to_pcm16, SAMPLE_RATE};
use crate::Asset;

const BPM: f32 = 128.0;
const STEPS_PER_BAR: usize = 16; // 16th-note grid
const BARS: usize = 16;
const LIFT_BAR: usize = BARS / 2;
const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;

fn music() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0xC0FF_EE42);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 4);

    // A - C, a two-chord vamp, one root every 2 bars.
    let bass_roots = [55.00_f32, 65.41]; // A1, C2
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
    // The hook: a 4-note Phrygian-bite riff, identical every bar.
    const HOOK: [f32; 4] = [440.00, 466.16, 523.25, 466.16]; // A4 Bb4 C5 Bb4

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;
        let lifted = bar >= LIFT_BAR;

        // Four-on-the-floor kick.
        if pos % 4 == 0 {
            kick_offsets.push(off);
        }
        // Off-beat closed hat, plus a 16th-note roll into the last beat of every 4th bar.
        if pos % 4 == 2 {
            hat(&mut buf, off, &mut rng, 0.30);
        }
        if (bar % 4 == 3 && pos >= 12) || (lifted && pos % 4 == 0) {
            hat(&mut buf, off, &mut rng, 0.16);
        }
        // Acid bassline.
        if BASS_HIT[pos] {
            let root = bass_roots[(bar / 2) % bass_roots.len()];
            bass_note(&mut buf, off, root * BASS_OCT[pos], step_ms * 0.85, 0.60);
        }
        // Hook riff on the downbeat of every odd bar; back half adds an
        // octave-up harmony for a small lift (~every 30s).
        if bar % 2 == 1 && pos % 4 == 0 {
            supersaw(&mut buf, off, HOOK[pos / 4], step_ms * 3.5, 0.20, 8.0, 0.008);
            if lifted {
                supersaw(&mut buf, off, HOOK[pos / 4] * 2.0, step_ms * 3.5, 0.10, 8.0, 0.008);
            }
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.55, step_ms * 0.85);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.95);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// Deep-space drift: a slow, atmospheric breather from the acid runs —
/// half-time kick, a low sub-bass drone instead of a moving bassline, and a
/// long-attack `supersaw` pad standing in for a lead (no plucked hook here)
/// so it reads as open space rather than another driving loop. A `riser`
/// sweeps through every 4th bar like a distant meteor shower building and
/// passing.
fn music2() -> Vec<u8> {
    const BPM: f32 = 90.0;
    const STEPS_PER_BAR: usize = 16;
    const BARS: usize = 12;
    const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0xD817_0002);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 8);

    // D - Bb sub-bass drone, one root every 2 bars.
    let drone_roots = [36.71_f32, 29.14]; // D1, Bb0
    // Pad notes: a wide, slow-moving 2-note figure in D dorian.
    const PAD: [f32; 2] = [293.66, 349.23]; // D4, F4

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;

        if pos == 0 || pos == 8 {
            kick_offsets.push(off);
        }
        if pos % 8 == 4 {
            open_hat(&mut buf, off, &mut rng, 0.08);
        }
        if pos == 0 {
            bass_note(&mut buf, off, drone_roots[bar % 2], step_ms * 14.0, 0.5);
        }
        if pos == 0 || pos == 8 {
            supersaw(&mut buf, off, PAD[(bar + pos / 8) % 2], step_ms * 7.0, 0.14, 220.0, 0.02);
        }
        if bar % 4 == 3 && pos == 0 {
            riser(&mut buf, off, step_ms * (STEPS_PER_BAR * 4) as f32 * 0.9, 0.22, &mut rng);
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.35, step_ms * 3.0);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.7);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// Asteroid Rush: harder and faster than the title loop — a rolling
/// 16th-note acid bassline (notes on nearly every step, not `music()`'s
/// syncopated gaps) under a sparse, bright `lead_stab` hook instead of
/// `supersaw` — a different lead timbre keeps this from just sounding like
/// `music()` sped up.
fn music3() -> Vec<u8> {
    const BPM: f32 = 144.0;
    const STEPS_PER_BAR: usize = 16;
    const BARS: usize = 16;
    const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0xA57E_0003);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 4);

    // F#m rolling 16ths, a fourth lower every other bar.
    const ACID_HI: [f32; STEPS_PER_BAR] = [
        92.50, 92.50, 0.0, 92.50, 110.00, 92.50, 0.0, 92.50,
        92.50, 0.0, 92.50, 116.54, 0.0, 92.50, 103.83, 0.0,
    ];
    const ACID_LO: [f32; STEPS_PER_BAR] = [
        69.30, 69.30, 0.0, 69.30, 82.41, 69.30, 0.0, 69.30,
        69.30, 0.0, 69.30, 87.31, 0.0, 69.30, 77.78, 0.0,
    ];
    const HOOK: [f32; 3] = [740.00, 830.61, 987.77]; // F#5 G#5 B5

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;

        if pos % 4 == 0 {
            kick_offsets.push(off);
        }
        hat(&mut buf, off, &mut rng, if pos % 2 == 0 { 0.14 } else { 0.27 });
        if bar % 2 == 1 && pos == 14 {
            open_hat(&mut buf, off, &mut rng, 0.20);
        }
        let acid = if bar % 2 == 0 { &ACID_HI } else { &ACID_LO };
        if acid[pos] > 0.0 {
            bass_note(&mut buf, off, acid[pos], step_ms * 0.8, 0.55);
        }
        if bar % 2 == 0 && pos == 6 {
            lead_stab(&mut buf, off, HOOK[(bar / 2) % 3], step_ms * 2.2, 0.18);
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.6, step_ms * 0.8);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.95);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// Warp Lift: a trance-style build around one obvious drop — a `riser`
/// sweeps through the final two bars into a hard cut back to the top of the
/// loop, the one meteors track besides the title loop built around a climax
/// moment rather than a steady groove throughout.
fn music4() -> Vec<u8> {
    const BPM: f32 = 136.0;
    const STEPS_PER_BAR: usize = 16;
    const BARS: usize = 18;
    const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0x11F7_0004);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 4);

    // Bb - Gm, a two-chord vamp, one root every 2 bars.
    let bass_roots = [58.27_f32, 49.00]; // Bb1, G1
    const HOOK: [f32; 4] = [466.16, 587.33, 698.46, 587.33]; // Bb4 D5 F5 D5

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;
        let in_lift = bar >= BARS - 2;

        if pos % 4 == 0 {
            kick_offsets.push(off);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.5);
        }
        if pos % 2 == 1 {
            open_hat(&mut buf, off, &mut rng, if in_lift { 0.27 } else { 0.18 });
        }
        if pos % 4 == 0 {
            bass_note(&mut buf, off, bass_roots[(bar / 2) % 2], step_ms * 3.5, 0.55);
        }
        if bar % 2 == 0 || in_lift {
            if pos == 8 || pos == 9 || pos == 10 || pos == 11 {
                let note = HOOK[(pos - 8) as usize];
                supersaw(&mut buf, off, note, step_ms * 1.4, 0.16, 8.0, 0.01);
            }
        }
        if bar == BARS - 2 && pos == 0 {
            riser(&mut buf, off, step_ms * (STEPS_PER_BAR * 2) as f32, 0.32, &mut rng);
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.6, step_ms * 0.9);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.95);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// Silent Drift: the moody one — a half-time kick landing off-symmetry (not
/// a clean two-hits-per-bar split), a C minor sub-bass sliding down a half
/// step to B (an unresolved, alien wobble rather than a normal chord
/// change), and a sparse 4-bar `lead_stab` motif with real silence between
/// hits instead of a running melody — the quietest, roomiest track in the
/// rotation.
fn music5() -> Vec<u8> {
    const BPM: f32 = 96.0;
    const STEPS_PER_BAR: usize = 16;
    const BARS: usize = 14;
    const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0x5170_0005);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 4);

    let bass_roots = [65.41_f32, 61.74]; // C2, B1
    const STAB: [f32; 4] = [311.13, 349.23, 415.30, 293.66]; // D#4 F4 G#4 D4

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;

        if pos == 0 || pos == 9 {
            kick_offsets.push(off);
        }
        if pos == 13 {
            open_hat(&mut buf, off, &mut rng, 0.09);
        }
        if pos == 0 {
            bass_note(&mut buf, off, bass_roots[bar % 2], step_ms * 6.0, 0.5);
        }
        if bar % 2 == 1 && pos == 7 {
            lead_stab(&mut buf, off, STAB[(bar / 2) % 4], step_ms * 3.2, 0.16);
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.62, step_ms * 1.6);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.85);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
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
        ("sounds/techno2.wav", music2()),
        ("sounds/techno3.wav", music3()),
        ("sounds/techno4.wav", music4()),
        ("sounds/techno5.wav", music5()),
        ("sounds/fire.wav", fire_zap()),
        ("sounds/ship_explosion.wav", ship_explosion()),
    ]
}
