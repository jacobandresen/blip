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

/// Fade the last `samples` to silence.
pub fn fade_out(buf: &mut [f32], samples: usize) {
    let n = samples.min(buf.len());
    let start = buf.len() - n;
    for (i, sample) in buf[start..].iter_mut().enumerate() {
        *sample *= 1.0 - (i + 1) as f32 / n as f32;
    }
}

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

/// Half the rate through a proper low-pass, flat to 8 kHz and gone by 12:
/// for effects whose top octave is already 30 dB down. The pair averaging
/// above would take another 2 dB off what they have at 10 kHz.
pub fn encode_pcm16_half(samples: &[i16]) -> Vec<u8> {
    const TAPS: usize = 63;
    let fc = 10_000.0 / SAMPLE_RATE as f32;
    let mid = (TAPS / 2) as isize;
    // A Blackman-windowed sinc.
    let mut kernel = [0.0f32; TAPS];
    for (i, k) in kernel.iter_mut().enumerate() {
        let n = (i as isize - mid) as f32;
        let sinc = if n == 0.0 { 2.0 * fc } else { (2.0 * PI * fc * n).sin() / (PI * n) };
        let x = i as f32 / (TAPS - 1) as f32;
        *k = sinc * (0.42 - 0.5 * (2.0 * PI * x).cos() + 0.08 * (4.0 * PI * x).cos());
    }
    let sum: f32 = kernel.iter().sum();
    let half: Vec<i16> = (0..samples.len().div_ceil(2))
        .map(|j| {
            let mut acc = 0.0f32;
            for (i, k) in kernel.iter().enumerate() {
                let at = 2 * j as isize + i as isize - mid;
                if at >= 0 && (at as usize) < samples.len() { acc += k * samples[at as usize] as f32; }
            }
            (acc / sum).round().clamp(-32767.0, 32767.0) as i16
        })
        .collect();
    encode_at(&half, SAMPLE_RATE / 2)
}

/// `encode_pcm16_half` for a sound that is already a full-rate WAV.
pub fn halved(wav: &[u8]) -> Vec<u8> {
    let pcm: Vec<i16> = wav[44..].chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
    encode_pcm16_half(&pcm)
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

/// Add to an f32 mix buffer without clipping; limit once with `soft_limit_to_pcm16`.
pub fn mix_into_f32(buf: &mut [f32], off: usize, sample: f32) {
    if off >= buf.len() {
        return;
    }
    buf[off] += sample;
}

/// Convert an f32 mix to 16-bit PCM through a tanh limiter.
/// `knee` sets the level where compression becomes noticeable.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// RMS of a full-scale sine at `hz` after `encode_pcm16_half`, against the sine's own.
    fn through_half(hz: f32) -> f32 {
        let tone: Vec<i16> = (0..SAMPLE_RATE as usize / 2)
            .map(|i| ((2.0 * PI * hz * i as f32 / SAMPLE_RATE as f32).sin() * 20_000.0) as i16)
            .collect();
        let wav = encode_pcm16_half(&tone);
        let out: Vec<f32> = wav[44..].chunks(2).map(|b| i16::from_le_bytes([b[0], b[1]]) as f32).collect();
        let rms = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
        rms(&out[100..out.len() - 100]) / (20_000.0 / 2.0f32.sqrt())
    }

    #[test]
    fn half_rate_keeps_what_is_under_8_khz_and_drops_what_would_alias() {
        for hz in [200.0, 3000.0, 6000.0, 8000.0] {
            let db = 20.0 * through_half(hz).log10();
            assert!(db.abs() < 0.5, "{hz} Hz came through at {db:.2} dB");
        }
        // 15 kHz would fold down to 7 kHz.
        let db = 20.0 * through_half(15_000.0).log10();
        assert!(db < -50.0, "15 kHz came through at {db:.1} dB");
    }

    #[test]
    fn fade_out_is_linear_and_clamps_to_the_buffer() {
        let mut samples = [1.0; 4];
        fade_out(&mut samples, 2);
        assert_eq!(samples, [1.0, 1.0, 0.5, 0.0]);

        fade_out(&mut samples, usize::MAX);
        assert_eq!(samples[3], 0.0);

        let mut unchanged = [0.25; 2];
        fade_out(&mut unchanged, 0);
        assert_eq!(unchanged, [0.25; 2]);
    }
}
