//! WAV encoding (16-bit PCM, mono) and the basics every sound shares: a
//! seeded random source, the master low-pass and the soft limiter.

use std::f32::consts::PI;

pub const SAMPLE_RATE: u32 = 44_100;

/// Default knee for `soft_limit_to_pcm16` on a full track: a kick, bass and
/// hats landing on one beat compress gracefully instead of clipping.
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

/// The master warmth for a music track: two gentle low-pass poles round
/// 3 kHz take the fizz off hats, saws and noise without dulling the tune.
pub fn warm(buf: &mut [f32]) { low_pass(buf, 3000.0); }

/// Two gentle low-pass poles at `hz`.
pub fn low_pass(buf: &mut [f32], hz: f32) {
    let a = 1.0 - (-2.0 * PI * hz / SAMPLE_RATE as f32).exp();
    let (mut l1, mut l2) = (0.0f32, 0.0f32);
    for v in buf.iter_mut() {
        l1 += a * (*v - l1);
        l2 += a * (l1 - l2);
        *v = l2;
    }
}

/// The action games' master: music is stored at half rate, so this only
/// keeps what would alias out of it. The edge of a saw or a snare stays.
pub fn bright(buf: &mut [f32]) { low_pass(buf, 7500.0); }

/// Keep a melodic voice out of the shrill register: anything above A4 drops
/// by octaves, keeping the tune's shape where it is easy on the ears and a
/// phone speaker.
pub fn tame(freq: f32) -> f32 {
    let mut f = freq;
    while f > 440.0 { f *= 0.5; }
    f
}

/// Encode a buffer of i16 mono samples at `SAMPLE_RATE` as a WAV file.
pub fn encode_pcm16_mono(samples: &[i16]) -> Vec<u8> {
    encode_at(samples, SAMPLE_RATE)
}

/// The same at half the rate, for music: after the 3 kHz `warm` pass there
/// is nothing above 11 kHz to lose, and it halves the memory a loop takes.
/// Averaging each pair before dropping one is the anti-aliasing low-pass.
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
