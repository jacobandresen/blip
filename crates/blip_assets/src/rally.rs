//! Rally assets (music only — sprites are drawn at runtime).
//!
//! Direct port of `games/rally/assets/generate_assets.c`.

use crate::wav::{encode_pcm16_mono, mix_into, SAMPLE_RATE};
use crate::Asset;

fn note(buf: &mut [i16], off: usize, freq: f32, ms: f32, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    let att = SAMPLE_RATE as usize / 200;
    let rel = (n / 5).max(1);
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let env = if i < att {
            i as f32 / att as f32
        } else if i + rel > n {
            (n - i) as f32 / rel as f32
        } else {
            1.0
        };
        let phase = (freq * i as f32 / sr).rem_euclid(1.0);
        let w = if phase < 0.5 { 1.0 } else { -1.0 };
        mix_into(buf, off + i, w * env * vol * 16000.0);
    }
}

fn music() -> Vec<u8> {
    let bpm = 110.0_f32;
    let beat_ms = 60_000.0 / bpm;
    let beats = 8 * 4;
    let total = (SAMPLE_RATE as f32 * beat_ms / 1000.0 * beats as f32) as usize
        + SAMPLE_RATE as usize;
    let mut buf = vec![0i16; total];

    let bass = [82.41_f32, 110.0, 123.47, 110.0];
    let stab = 164.81_f32;

    for b in 0..beats {
        let bar = b / 4;
        let beat = b % 4;
        let off = (SAMPLE_RATE as f32 * beat_ms / 1000.0 * b as f32) as usize;

        note(&mut buf, off, bass[beat], beat_ms * 0.65, 0.38);

        if beat == 0 && bar % 2 == 0 {
            note(&mut buf, off, stab, beat_ms * 0.18, 0.20);
        }
        if beat == 2 && bar % 2 == 1 {
            note(&mut buf, off, stab, beat_ms * 0.12, 0.14);
        }
    }

    encode_pcm16_mono(&buf)
}

pub fn generate() -> Vec<Asset> {
    vec![("sounds/music.wav", music())]
}
