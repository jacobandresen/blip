//! Rally assets (music only — sprites are drawn at runtime).
//!
//! Direct port of `games/rally/assets/generate_assets.c`.

use crate::techno::{bass_note, clap, hat, kick, sidechain_duck, supersaw, Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_mono, soft_limit_to_pcm16, SAMPLE_RATE};
use crate::Asset;

const BPM: f32 = 132.0;
const STEPS_PER_BAR: usize = 16;
const BARS: usize = 16;
const LIFT_BAR: usize = BARS / 2;
const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;

/// A fast, driving arcade-rally banger: four-on-the-floor kick, backbeat
/// claps, tight hats, a relentless galloping bassline, and one catchy hook
/// riff repeated every bar over a simple Em-Am vamp. The back half
/// (~every 30s) adds an octave-up hook harmony and an extra hat roll.
fn music() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0xFA57_CA12);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 4);

    // E - A, a two-chord vamp, one root every 2 bars.
    let bass_roots = [82.41_f32, 110.0]; // E2, A2
    // Galloping 16th pattern: hit, hit, rest, hit — repeated across the bar.
    const BASS_HIT: [bool; STEPS_PER_BAR] = [
        true, true, false, true, true, true, false, true,
        true, true, false, true, true, true, false, true,
    ];
    // The hook: a driving 4-note riff, identical every bar.
    const HOOK: [f32; 4] = [329.63, 392.00, 493.88, 392.00]; // E4 G4 B4 G4

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;
        let lifted = bar >= LIFT_BAR;

        if pos % 4 == 0 {
            kick_offsets.push(off);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.45);
        }
        if pos % 2 == 1 {
            hat(&mut buf, off, &mut rng, 0.22);
        }
        if lifted && pos == 6 {
            hat(&mut buf, off, &mut rng, 0.16);
        }
        if BASS_HIT[pos] {
            let root = bass_roots[(bar / 2) % bass_roots.len()];
            bass_note(&mut buf, off, root, step_ms * 0.6, 0.60);
        }
        if pos % 4 == 0 {
            supersaw(&mut buf, off, HOOK[pos / 4], step_ms * 3.0, 0.18, 6.0, 0.008);
            if lifted {
                supersaw(&mut buf, off, HOOK[pos / 4] * 2.0, step_ms * 3.0, 0.09, 6.0, 0.008);
            }
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.55, step_ms * 0.85);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.95);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

pub fn generate() -> Vec<Asset> {
    vec![("sounds/music.wav", music())]
}
