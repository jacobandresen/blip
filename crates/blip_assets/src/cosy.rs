//! Bubbler's sound, shared: sine-led tones, a pulse softened with a sine,
//! rounded pitch sweeps, bell dings and glassy sparkles, and jingles of a
//! lead doubled by a bell. Everything is soft-limited, never hard noise.

use std::f32::consts::PI;

use crate::wav::{warm, Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_mono, encode_pcm16_music, soft_limit_to_pcm16, SAMPLE_RATE};

pub const SR: f32 = SAMPLE_RATE as f32;
/// Hold the previous note, in a jingle.
pub const H: i32 = -2;

pub fn midi_hz(n: i32) -> f32 { 440.0 * 2f32.powf((n - 69) as f32 / 12.0) }

/// Pulse wave with duty `d`, blended with a sine so it is bright without
/// being harsh on a phone speaker.
pub fn pulse(phase: f32, d: f32) -> f32 {
    let p = phase.fract();
    let sq = if p < d { 1.0 } else { -1.0 };
    sq * 0.8 + (2.0 * PI * p).sin() * 0.2
}

pub fn saw(phase: f32) -> f32 { 2.0 * phase.fract() - 1.0 }

pub fn tri(phase: f32) -> f32 {
    let p = phase.fract();
    4.0 * (p - 0.5).abs() - 1.0
}

/// Lead (pulse and sine, vibrato, quick attack, gentle decay), bass
/// (triangle with a pulse edge), arpeggio blip, or bell: Bubbler's band.
/// The rest give each game its own instrument in the same warm family:
/// marimba (Serpent), glockenspiel (Bouncer), theremin (Defender), glass
/// (Meteors) and pluck (Rally). Saw and growl are the action songs' lead and
/// bass: detuned sawtooths with their edge left on.
#[derive(Clone, Copy)]
pub enum Voice { Lead, Bass, Arp, Bell, Marimba, Glock, Theremin, Glass, Pluck, Saw, Growl }

/// A note into `buf` at `start`, held `len` samples.
pub fn note(buf: &mut [f32], start: usize, len: usize, midi: i32, vol: f32, v: Voice) {
    let f = midi_hz(midi);
    let mut ph = 0.0f32;
    let tail = match v {
        Voice::Lead => 0.06, Voice::Bass => 0.03, Voice::Arp => 0.01, Voice::Bell => 0.4,
        Voice::Marimba => 0.25, Voice::Glock => 0.6, Voice::Theremin => 0.12, Voice::Glass => 0.9, Voice::Pluck => 0.02,
        Voice::Saw => 0.05, Voice::Growl => 0.02,
    };
    let n = len + (tail * SR) as usize;
    for i in 0..n {
        let j = start + i;
        if j >= buf.len() { break; }
        let t = i as f32 / SR;
        let held = i < len;
        let env = match v {
            Voice::Lead => {
                let a = (t / 0.008).min(1.0);
                let d = 0.75 + 0.25 * (-t / 0.18).exp();
                if held { a * d } else { d * (1.0 - (i - len) as f32 / (tail * SR)) }
            }
            Voice::Bass => {
                let a = (t / 0.004).min(1.0) * (0.6 + 0.4 * (-t / 0.09).exp());
                if held { a } else { a * (1.0 - (i - len) as f32 / (tail * SR)) }
            }
            Voice::Arp => {
                let a = (t / 0.002).min(1.0) * (-t / 0.05).exp();
                if held { a } else { 0.0 }
            }
            Voice::Bell => (t / 0.003).min(1.0) * (-t / 0.25).exp(),
            Voice::Marimba => (t / 0.002).min(1.0) * (-t / 0.13).exp(),
            Voice::Glock => (t / 0.001).min(1.0) * (-t / 0.35).exp(),
            Voice::Theremin => {
                let a = (t / 0.03).min(1.0);
                if held { a } else { a * (1.0 - (i - len) as f32 / (tail * SR)) }
            }
            Voice::Glass => (t / 0.004).min(1.0) * (-t / 0.55).exp(),
            Voice::Pluck => (t / 0.001).min(1.0) * (-t / 0.09).exp(),
            Voice::Saw => {
                let a = (t / 0.004).min(1.0) * (0.7 + 0.3 * (-t / 0.12).exp());
                if held { a } else { a * (1.0 - (i - len) as f32 / (tail * SR)) }
            }
            Voice::Growl => {
                let a = (t / 0.002).min(1.0) * (0.45 + 0.55 * (-t / 0.06).exp());
                if held { a } else { a * (1.0 - (i - len) as f32 / (tail * SR)) }
            }
        };
        let vib = match v {
            Voice::Lead => 1.0 + 0.004 * (2.0 * PI * 5.5 * t).sin() * (t / 0.15).min(1.0),
            Voice::Theremin => 1.0 + 0.012 * (2.0 * PI * 6.0 * t).sin() * ((t - 0.06) / 0.12).clamp(0.0, 1.0),
            Voice::Saw => 1.0 + 0.006 * (2.0 * PI * 6.0 * t).sin() * ((t - 0.12) / 0.1).clamp(0.0, 1.0),
            _ => 1.0,
        };
        ph += f * vib / SR;
        let s = match v {
            Voice::Lead => pulse(ph, 0.5) * 0.55 + (2.0 * PI * ph).sin() * 0.45,
            Voice::Bass => tri(ph) * 0.8 + pulse(ph, 0.5) * 0.2,
            Voice::Arp => pulse(ph, 0.125),
            Voice::Bell => (2.0 * PI * ph).sin() + 0.35 * (2.0 * PI * ph * 2.0).sin() + 0.15 * (2.0 * PI * ph * 3.01).sin(),
            // a wooden bar: triangle body and a quick knock at four times the pitch
            Voice::Marimba => tri(ph) * 0.8 + (2.0 * PI * ph * 4.0).sin() * 0.35 * (-t / 0.018).exp(),
            // a steel bar: the glockenspiel's inharmonic 2.76 partial
            Voice::Glock => (2.0 * PI * ph).sin() + 0.3 * (2.0 * PI * ph * 2.76).sin() * (-t / 0.08).exp(),
            Voice::Theremin => (2.0 * PI * ph).sin() + 0.12 * (2.0 * PI * ph * 2.0).sin(),
            // two sines a hair apart, beating slowly, and a faint fifth above
            Voice::Glass => (2.0 * PI * ph * 0.996).sin() * 0.5 + (2.0 * PI * ph * 1.004).sin() * 0.5
                + 0.2 * (2.0 * PI * ph * 3.0).sin() * (-t / 0.2).exp(),
            Voice::Pluck => pulse(ph, 0.25) * 0.35 + (2.0 * PI * ph).sin() * 0.65,
            // two sawtooths a few cents apart over a square an octave down
            Voice::Saw => saw(ph * 0.997) * 0.4 + saw(ph * 1.003) * 0.4 + pulse(ph * 0.5, 0.5) * 0.2,
            // a sawtooth with a sine under it for weight
            Voice::Growl => saw(ph) * 0.55 + (2.0 * PI * ph).sin() * 0.6,
        };
        buf[j] += s * env * vol;
    }
}

/// A short tune: `notes` in MIDI, `H` holds, a negative value rests.
/// The lead plays an octave below the written note with a bell doubling
/// it an octave lower still. `step` is seconds per note.
pub fn jingle(notes: &[i32], step: f32) -> Vec<u8> {
    let st = (step * SR) as usize;
    let mut buf = vec![0.0f32; st * (notes.len() + 6)];
    let mut k = 0;
    while k < notes.len() {
        let n = notes[k];
        if n >= 0 {
            let mut len = 1;
            while k + len < notes.len() && notes[k + len] == H { len += 1; }
            note(&mut buf, k * st, len * st, n - 12, 0.34, Voice::Lead);
            note(&mut buf, k * st, len * st, n - 24, 0.12, Voice::Bell);
            k += len;
        } else { k += 1; }
    }
    finish(&buf, 21_000.0)
}

/// A game's own jingle: `lead` at the written pitch shifted `lead_oct`
/// octaves, doubled by `double` (if any) `double_oct` octaves.
pub fn jingle_with(notes: &[i32], step: f32, lead: Voice, lead_oct: i32, double: Option<(Voice, i32)>) -> Vec<u8> {
    let st = (step * SR) as usize;
    let mut buf = vec![0.0f32; st * (notes.len() + 8)];
    let mut k = 0;
    while k < notes.len() {
        let n = notes[k];
        if n >= 0 {
            let mut len = 1;
            while k + len < notes.len() && notes[k + len] == H { len += 1; }
            note(&mut buf, k * st, len * st, n + 12 * lead_oct, 0.36, lead);
            if let Some((d, o)) = double { note(&mut buf, k * st, len * st, n + 12 * o, 0.14, d); }
            k += len;
        } else { k += 1; }
    }
    buf.resize(buf.len() + (SR * 1.2) as usize, 0.0); // room for a long ring
    let end = buf.iter().rposition(|x| x.abs() > 1e-3).map_or(1, |i| i + 1);
    buf.truncate(end);
    fade_out(&mut buf, 0.03);
    finish_warm(buf, 21_000.0)
}

/// Notes one after another on `v` (MIDI, `step` seconds apart), as f32.
pub fn run(notes: &[i32], step: f32, v: Voice, vol: f32) -> Vec<f32> {
    let st = (step * SR) as usize;
    let mut buf = vec![0.0f32; st * notes.len() + (SR * 2.5) as usize];
    for (k, n) in notes.iter().enumerate() {
        if *n >= 0 { note(&mut buf, k * st, st, *n, vol, v); }
    }
    let end = buf.iter().rposition(|x| x.abs() > 1e-3).map_or(1, |i| i + 1);
    buf.truncate(end);
    fade_out(&mut buf, 0.02);
    buf
}

/// Fade the last `secs` to silence, so a cut-off ring never clicks.
pub fn fade_out(buf: &mut [f32], secs: f32) {
    let n = ((secs * SR) as usize).min(buf.len());
    let len = buf.len();
    for i in 0..n { buf[len - n + i] *= 1.0 - (i + 1) as f32 / n as f32; }
}

/// The tone a sweep is drawn with.
#[derive(Clone, Copy)]
pub enum Tone { Sine, Pulse(f32), Tri }

fn tone(t: Tone, ph: f32) -> f32 {
    match t {
        Tone::Sine => (2.0 * PI * ph).sin(),
        Tone::Pulse(d) => pulse(ph, d),
        Tone::Tri => tri(ph),
    }
}

/// A pitch glide from `f0` to `f1` Hz (exponential) over `dur` seconds,
/// fading as (1 - k)^`fade`, with an optional `wobble` (Hz depth at 7 Hz).
pub fn glide(dur: f32, f0: f32, f1: f32, t: Tone, fade: f32, wobble: f32) -> Vec<f32> {
    let n = (dur * SR) as usize;
    let mut ph = 0.0f32;
    (0..n).map(|i| {
        let tt = i as f32 / SR;
        let k = i as f32 / n as f32;
        let f = f0 * (f1 / f0).powf(k) + wobble * (2.0 * PI * 7.0 * tt).sin();
        ph += f / SR;
        let a = (tt / 0.003).min(1.0) * (1.0 - k).powf(fade);
        tone(t, ph) * a
    }).collect()
}

/// A round wet "bloop": a sine rising (or falling) with a breath of air.
pub fn bloop(dur: f32, f0: f32, f1: f32, seed: u32) -> Vec<f32> {
    let mut rng = Rng(seed | 1);
    let mut v = glide(dur, f0, f1, Tone::Sine, 1.5, 0.0);
    let n = v.len() as f32;
    for (i, s) in v.iter_mut().enumerate() {
        *s = *s * 0.85 + (rng.next_f32() * 2.0 - 1.0) * 0.06 * (1.0 - i as f32 / n);
    }
    v
}

/// A bell ding at `freq`, ringing for about `ring` seconds.
pub fn ding(freq: f32, ring: f32) -> Vec<f32> {
    let n = ((ring * 4.0).max(0.15) * SR) as usize;
    let mut ph = 0.0f32;
    (0..n).map(|i| {
        let t = i as f32 / SR;
        ph += freq / SR;
        ((2.0 * PI * ph).sin() + 0.4 * (2.0 * PI * ph * 2.0).sin() + 0.12 * (2.0 * PI * ph * 3.01).sin())
            * (t / 0.002).min(1.0) * (-t / ring).exp() * 0.6
    }).collect()
}

/// A soft rubbery boing: a sine dropping fast from `f * 2.2` to `f`.
pub fn boing(dur: f32, f: f32) -> Vec<f32> {
    let n = (dur * SR) as usize;
    let mut ph = 0.0f32;
    (0..n).map(|i| {
        let t = i as f32 / SR;
        let k = i as f32 / n as f32;
        ph += (f + f * 1.2 * (-t / 0.04).exp()) / SR;
        (2.0 * PI * ph).sin() * (t / 0.002).min(1.0) * (1.0 - k) * 0.75
    }).collect()
}

/// A two-step chirp, `fa` then `fb`, on a thin pulse.
pub fn chirp(dur: f32, fa: f32, fb: f32) -> Vec<f32> {
    let n = (dur * SR) as usize;
    let mut ph = 0.0f32;
    (0..n).map(|i| {
        let k = i as f32 / n as f32;
        ph += if k < 0.4 { fa } else { fb } / SR;
        pulse(ph, 0.25) * (1.0 - k) * 0.5
    }).collect()
}

/// Glassy pings climbing from `midi` by `interval` semitones, `steps` of them.
pub fn sparkle(dur: f32, midi: i32, steps: i32, interval: i32) -> Vec<f32> {
    let n = (dur * SR) as usize;
    let each = dur / steps as f32;
    let mut ph = 0.0f32;
    (0..n).map(|i| {
        let t = i as f32 / SR;
        let step = ((t / each) as i32).min(steps - 1);
        ph += midi_hz(midi + step * interval) / SR;
        (2.0 * PI * ph).sin() * (-(t % each) / 0.05).exp() * 0.5
    }).collect()
}

/// A bright snap, for a pop.
pub fn snap(dur: f32, f0: f32, seed: u32) -> Vec<f32> {
    let mut rng = Rng(seed | 1);
    let n = (dur * SR) as usize;
    let mut ph = 0.0f32;
    (0..n).map(|i| {
        let t = i as f32 / SR;
        let k = i as f32 / n as f32;
        ph += (f0 - f0 * 0.66 * k) / SR;
        let w = rng.next_f32() * 2.0 - 1.0;
        (2.0 * PI * ph).sin() * (-t / 0.025).exp() + w * (-t / 0.008).exp() * 0.5
    }).collect()
}

/// `b` into `a` at `at` seconds, scaled by `gain`, growing `a` as needed.
pub fn mix(mut a: Vec<f32>, b: &[f32], at: f32, gain: f32) -> Vec<f32> {
    let off = (at * SR) as usize;
    if a.len() < off + b.len() { a.resize(off + b.len(), 0.0); }
    for (i, v) in b.iter().enumerate() { a[off + i] += v * gain; }
    a
}

/// Soft-limit and encode.
pub fn finish(buf: &[f32], gain: f32) -> Vec<u8> {
    let scaled: Vec<f32> = buf.iter().map(|v| v * gain).collect();
    encode_pcm16_mono(&soft_limit_to_pcm16(&scaled, MIX_KNEE))
}

/// Faded out, low-passed at 3 kHz like the music, soft-limited and encoded
/// at half rate, as the music is: after the low-pass there is nothing up
/// there to lose, and an effect is half the bytes to download.
pub fn finish_warm(mut buf: Vec<f32>, gain: f32) -> Vec<u8> {
    warm(&mut buf);
    fade_out(&mut buf, 0.008); // however an effect was cut, it ends in silence
    let scaled: Vec<f32> = buf.iter().map(|v| v * gain).collect();
    encode_pcm16_music(&soft_limit_to_pcm16(&scaled, MIX_KNEE))
}

/// The usual level for a finished effect, warmed.
pub fn sfx(buf: &[f32]) -> Vec<u8> { finish_warm(buf.to_vec(), 20_000.0) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_warm_effect_is_half_rate_and_as_long_as_it_was() {
        let wav = sfx(&glide(0.5, 880.0, 440.0, Tone::Sine, 1.0, 0.0));
        let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
        let samples = (wav.len() - 44) / 2;
        assert_eq!(rate, SAMPLE_RATE / 2);
        assert!((samples as f32 / rate as f32 - 0.5).abs() < 0.01, "{samples} samples at {rate} Hz");
    }
}
