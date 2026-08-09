//! Rally assets (music only — sprites are drawn at runtime).
//!
//! Direct port of `games/rally/assets/generate_assets.c`.

use crate::techno::{bass_note, clap, hat, kick, lead_stab, Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_mono, soft_limit_to_pcm16, SAMPLE_RATE};
use crate::Asset;

const BPM: f32 = 132.0;
const STEPS_PER_BAR: usize = 16;
const BARS: usize = 8;
const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;

/// A fast, driving arcade-rally banger: four-on-the-floor kick, backbeat
/// claps, tight hats, and a relentless galloping bassline.
fn music() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0xFA57_CA12);

    // E-minor galloping roots, one per 2-bar section: E2 - A2 - B2 - A2.
    let bass_roots = [82.41_f32, 110.0, 123.47, 110.0];
    // Galloping 16th pattern: hit, hit, rest, hit — repeated across the bar.
    const BASS_HIT: [bool; STEPS_PER_BAR] = [
        true, true, false, true, true, true, false, true,
        true, true, false, true, true, true, false, true,
    ];
    let stab = 164.81_f32; // B3

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;

        if pos % 4 == 0 {
            kick(&mut buf, off, 0.95);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.45);
        }
        if pos % 2 == 1 {
            hat(&mut buf, off, &mut rng, 0.22);
        }
        if BASS_HIT[pos] {
            let root = bass_roots[(bar / 2) % bass_roots.len()];
            bass_note(&mut buf, off, root, step_ms * 0.6, 0.60);
        }
        if bar % 2 == 0 && pos == 0 {
            lead_stab(&mut buf, off, stab, step_ms * 3.0, 0.16);
        }
        if bar % 2 == 1 && pos == 8 {
            lead_stab(&mut buf, off, stab * 1.5, step_ms * 2.0, 0.12);
        }
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

pub fn generate() -> Vec<Asset> {
    vec![("sounds/music.wav", music())]
}
