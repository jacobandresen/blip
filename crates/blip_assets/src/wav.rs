//! WAV (16-bit signed PCM, mono, 44.1 kHz) helpers and tone synthesis.

pub const SAMPLE_RATE: u32 = 44_100;

/// Encode a buffer of i16 mono samples at `SAMPLE_RATE` as a WAV file.
pub fn encode_pcm16_mono(samples: &[i16]) -> Vec<u8> {
    let n = samples.len();
    let data_bytes = (n * 2) as u32;
    let mut out = Vec::with_capacity(44 + n * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// Number of samples for a given duration in milliseconds.
pub fn ms_to_samples(ms: f32) -> usize {
    (SAMPLE_RATE as f32 * ms / 1000.0) as usize
}

/// Saturating add into a buffer of i16 samples.
pub fn mix_into(buf: &mut [i16], off: usize, sample: f32) {
    if off >= buf.len() {
        return;
    }
    let v = buf[off] as i32 + sample as i32;
    buf[off] = v.clamp(-32_767, 32_767) as i16;
}

/// Linear ADSR-ish envelope: attack-ramp / sustain / release-ramp.
#[inline]
pub fn env(i: usize, n: usize, attack: usize, release: usize) -> f32 {
    if i < attack {
        i as f32 / attack as f32
    } else if i + release > n {
        (n - i) as f32 / release as f32
    } else {
        1.0
    }
}
