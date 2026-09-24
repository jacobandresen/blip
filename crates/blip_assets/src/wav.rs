//! WAV (16-bit signed PCM, mono, 44.1 kHz) helpers and tone synthesis.

pub const SAMPLE_RATE: u32 = 44_100;

/// Encode a buffer of i16 mono samples at `SAMPLE_RATE` as a WAV file.
pub fn encode_pcm16_mono(samples: &[i16]) -> Vec<u8> {
    encode_at(samples, SAMPLE_RATE)
}

/// The same, at half the rate, for music.
///
/// Music is the only thing here long enough for its size to matter: a
/// sound effect is a tenth of a second and a loop is several seconds,
/// and the two stage themes were most of the game's download. Halving
/// the rate halves the bytes, and a taiko and a plucked string have
/// nothing above eleven kilohertz to lose — which buys twice the loop
/// for the same money, and a loop twice as long is the cheapest
/// variety there is.
///
/// Averaging each pair before dropping one of them is the low-pass
/// that stops whatever *is* up there folding back down as aliasing.
pub fn encode_pcm16_music(samples: &[i16]) -> Vec<u8> {
    let half: Vec<i16> = samples
        .chunks(2)
        .map(|c| if c.len() == 2 { ((c[0] as i32 + c[1] as i32) / 2) as i16 } else { c[0] })
        .collect();
    encode_at(&half, SAMPLE_RATE / 2)
}

fn encode_at(samples: &[i16], rate: u32) -> Vec<u8> {
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
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes()); // byte rate
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

/// Plain (non-clamping) add into an f32 accumulation buffer. Use this for
/// multi-voice mixes (drums + bass + leads all landing on the same beat)
/// where clamping per-voice, per-sample would hard-clip into harsh digital
/// distortion; clamp once at the end instead, with `soft_limit_to_pcm16`.
pub fn mix_into_f32(buf: &mut [f32], off: usize, sample: f32) {
    if off >= buf.len() {
        return;
    }
    buf[off] += sample;
}

/// Convert an f32 accumulation buffer to 16-bit PCM through a soft (tanh)
/// limiter, so loud collisions between voices compress gracefully instead of
/// flat-topping. `knee` is roughly "the level at which compression starts to
/// bite" — signals well under it pass through close to linear.
pub fn soft_limit_to_pcm16(buf: &[f32], knee: f32) -> Vec<i16> {
    buf.iter()
        .map(|&s| ((s / knee).tanh() * 31_000.0) as i16)
        .collect()
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
