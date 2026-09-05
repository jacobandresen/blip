//! Rally assets (music only — sprites are drawn at runtime).
//!
//! Direct port of `games/rally/assets/generate_assets.c`.

use crate::techno::{
    bass_note, clap, hat, kick, lead_stab, open_hat, riser, sidechain_duck, supersaw, Rng,
    MIX_KNEE,
};
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

/// Turbo Boost: faster and punchier than the title loop — a syncopated bass
/// hit pattern (landing off the beat in places, not the steady gallop) and
/// a bright `lead_stab` hook instead of `supersaw`, in a different key so
/// it doesn't just read as the title loop sped up.
fn music2() -> Vec<u8> {
    const BPM: f32 = 146.0;
    const STEPS_PER_BAR: usize = 16;
    const BARS: usize = 16;
    const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0x7080_0002);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 4);

    // C - Am, a two-chord vamp, one root every 2 bars.
    let bass_roots = [65.41_f32, 55.00]; // C2, A1
    const BASS_HIT: [bool; STEPS_PER_BAR] = [
        true, false, true, false, false, true, false, true,
        false, true, false, false, true, false, true, false,
    ];
    const HOOK: [f32; 3] = [523.25, 587.33, 659.25]; // C5 D5 E5

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;

        if pos % 4 == 0 {
            kick_offsets.push(off);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.5);
        }
        if pos % 2 == 1 {
            hat(&mut buf, off, &mut rng, 0.24);
        }
        if BASS_HIT[pos] {
            let root = bass_roots[(bar / 2) % 2];
            bass_note(&mut buf, off, root, step_ms * 0.55, 0.58);
        }
        if pos == 6 || pos == 7 || pos == 8 {
            lead_stab(&mut buf, off, HOOK[(pos - 6) as usize], step_ms * 1.6, 0.20);
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.55, step_ms * 0.8);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.95);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// Night Circuit: a laid-back cruising cut — half-time kick, long held bass
/// notes instead of a moving line, and a slow, spaced-out `lead_stab`
/// motif — the breather in Rally's rotation, deliberately roomier than the
/// other four rather than another up-tempo racer.
fn music3() -> Vec<u8> {
    const BPM: f32 = 108.0;
    const STEPS_PER_BAR: usize = 16;
    const BARS: usize = 12;
    const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0x7080_0003);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 8);

    // Dm - Bb - F - C, one held chord (as a bass note) per bar.
    let roots = [73.42_f32, 58.27, 87.31, 65.41]; // D2 Bb1 F2 C2
    const MOTIF: [f32; 4] = [587.33, 493.88, 698.46, 523.25]; // D5 B4 F5 C5

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;

        if pos == 0 || pos == 8 {
            kick_offsets.push(off);
        }
        if pos == 12 {
            clap(&mut buf, off, &mut rng, 0.30);
        }
        if pos % 4 == 2 {
            hat(&mut buf, off, &mut rng, 0.09);
        }
        if pos == 0 {
            bass_note(&mut buf, off, roots[bar % 4], step_ms * 12.0, 0.42);
        }
        if pos == 7 {
            lead_stab(&mut buf, off, MOTIF[bar % 4], step_ms * 5.0, 0.19);
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.3, step_ms * 2.0);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.75);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// Photo Finish: built around one big finish — a `riser` sweeps through the
/// last two bars into a hard cut back to the top, the closest thing Rally's
/// rotation has to an obvious "drop" moment, for the lap where the race is
/// close.
fn music4() -> Vec<u8> {
    const BPM: f32 = 140.0;
    const STEPS_PER_BAR: usize = 16;
    const BARS: usize = 18;
    const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0x7080_0004);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 4);

    // D - A, a two-chord vamp, one root every 2 bars.
    let bass_roots = [73.42_f32, 110.00]; // D2, A2
    const HOOK: [f32; 4] = [587.33, 698.46, 880.00, 698.46]; // D5 F5 A5 F5

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
            open_hat(&mut buf, off, &mut rng, if in_lift { 0.26 } else { 0.17 });
        }
        if pos % 4 == 0 {
            bass_note(&mut buf, off, bass_roots[(bar / 2) % 2], step_ms * 3.5, 0.55);
        }
        if bar % 2 == 0 || in_lift {
            if pos == 8 || pos == 9 || pos == 10 || pos == 11 {
                let note = HOOK[(pos - 8) as usize];
                supersaw(&mut buf, off, note, step_ms * 1.4, 0.15, 7.0, 0.009);
            }
        }
        if bar == BARS - 2 && pos == 0 {
            riser(&mut buf, off, step_ms * (STEPS_PER_BAR * 2) as f32, 0.3, &mut rng);
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.6, step_ms * 0.9);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.95);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// Pit Stop Groove: a funkier cut than the others — the bassline lands off
/// the beat instead of four-on-the-floor-locked, and a call-and-response
/// `lead_stab` hook (one phrase answered by a second) instead of one riff
/// repeated verbatim.
fn music5() -> Vec<u8> {
    const BPM: f32 = 122.0;
    const STEPS_PER_BAR: usize = 16;
    const BARS: usize = 16;
    const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0x7080_0005);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 4);

    // G mixolydian vamp: root off the beat, not on it.
    let root = 98.00_f32; // G2
    const BASS_HIT: [bool; 16] = [
        false, false, true, false, false, true, false, true,
        false, false, true, false, false, true, false, false,
    ];
    const CALL:     [f32; 3] = [587.33, 698.46, 783.99]; // D5 F5 G5
    const RESPONSE: [f32; 3] = [659.25, 587.33, 493.88]; // E5 D5 B4

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;

        if pos % 4 == 0 {
            kick_offsets.push(off);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.5);
        }
        if pos % 2 == 1 {
            hat(&mut buf, off, &mut rng, 0.19);
        }
        if bar % 4 == 3 && pos == 10 {
            open_hat(&mut buf, off, &mut rng, 0.2);
        }
        if BASS_HIT[pos] {
            bass_note(&mut buf, off, root * if pos == 7 || pos == 13 { 1.5 } else { 1.0 }, step_ms * 0.6, 0.55);
        }
        let phrase = if bar % 2 == 0 { &CALL } else { &RESPONSE };
        if pos == 9 || pos == 10 || pos == 11 {
            lead_stab(&mut buf, off, phrase[(pos - 9) as usize], step_ms * 1.3, 0.19);
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.5, step_ms * 0.9);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.9);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

pub fn generate() -> Vec<Asset> {
    vec![
        ("sounds/music.wav",  music()),
        ("sounds/music2.wav", music2()),
        ("sounds/music3.wav", music3()),
        ("sounds/music4.wav", music4()),
        ("sounds/music5.wav", music5()),
    ]
}
