//! Bubbler — a tribute to Bubble Bobble. All of its sound is synthesised
//! on the device at load (like Brawler's), so the game ships as code:
//! an original bouncy chiptune theme, the round / clear / hurry / game-over
//! jingles, and the effects.

use std::f32::consts::PI;

use crate::techno::{Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_mono, encode_pcm16_music, soft_limit_to_pcm16, SAMPLE_RATE};

const SR: f32 = SAMPLE_RATE as f32;

fn midi_hz(n: i32) -> f32 { 440.0 * 2f32.powf((n - 69) as f32 / 12.0) }

/// Pulse wave with duty `d`, band-limited a little by blending in a sine
/// so it is bright without being harsh on a phone speaker.
fn pulse(phase: f32, d: f32) -> f32 {
    let p = phase.fract();
    let sq = if p < d { 1.0 } else { -1.0 };
    sq * 0.8 + (2.0 * PI * p).sin() * 0.2
}

fn tri(phase: f32) -> f32 {
    let p = phase.fract();
    4.0 * (p - 0.5).abs() - 1.0
}

/// A note into `buf` at `start` for `len` samples: a lead (pulse, vibrato,
/// quick attack, gentle decay), a bass (triangle with a pulse edge) or an
/// arpeggio blip.
#[derive(Clone, Copy)]
enum Voice { Lead, Bass, Arp, Bell }

fn note(buf: &mut [f32], start: usize, len: usize, midi: i32, vol: f32, v: Voice) {
    let f = midi_hz(midi);
    let mut ph = 0.0f32;
    let tail = match v { Voice::Lead => 0.06, Voice::Bass => 0.03, Voice::Arp => 0.01, Voice::Bell => 0.4 };
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
        };
        let vib = if let Voice::Lead = v { 1.0 + 0.004 * (2.0 * PI * 5.5 * t).sin() * (t / 0.15).min(1.0) } else { 1.0 };
        ph += f * vib / SR;
        let s = match v {
            Voice::Lead => pulse(ph, 0.25),
            Voice::Bass => tri(ph) * 0.8 + pulse(ph, 0.5) * 0.2,
            Voice::Arp => pulse(ph, 0.125),
            Voice::Bell => (2.0 * PI * ph).sin() + 0.35 * (2.0 * PI * ph * 2.0).sin() + 0.15 * (2.0 * PI * ph * 3.01).sin(),
        };
        buf[j] += s * env * vol;
    }
}

fn kick(buf: &mut [f32], start: usize, vol: f32) {
    let n = (0.16 * SR) as usize;
    let mut ph = 0.0f32;
    for i in 0..n {
        let j = start + i;
        if j >= buf.len() { break; }
        let t = i as f32 / SR;
        ph += (48.0 + 110.0 * (-t / 0.03).exp()) / SR;
        buf[j] += ((2.0 * PI * ph).sin() * 2.0).tanh() * (-t / 0.07).exp() * vol;
    }
}

fn snare(buf: &mut [f32], start: usize, rng: &mut Rng, vol: f32) {
    let n = (0.14 * SR) as usize;
    let mut lp = 0.0f32;
    for i in 0..n {
        let j = start + i;
        if j >= buf.len() { break; }
        let t = i as f32 / SR;
        let w = rng.next_f32() * 2.0 - 1.0;
        lp += (w - lp) * 0.5;
        let body = (2.0 * PI * 190.0 * t).sin() * (-t / 0.03).exp();
        buf[j] += (lp * (-t / 0.05).exp() * 0.9 + body * 0.5) * vol;
    }
}

fn hat(buf: &mut [f32], start: usize, rng: &mut Rng, vol: f32) {
    let n = (0.035 * SR) as usize;
    let mut prev = 0.0f32;
    for i in 0..n {
        let j = start + i;
        if j >= buf.len() { break; }
        let t = i as f32 / SR;
        let w = rng.next_f32() * 2.0 - 1.0;
        let hp = w - prev;
        prev = w;
        buf[j] += hp * (-t / 0.012).exp() * vol;
    }
}

const H: i32 = -2; // hold the previous note
const RR: i32 = -1; // rest

/// The theme: 152 bpm, sixteen bars of eighths, C major. Bouncy bass on
/// root-octave-fifth, a quiet chord arpeggio in sixteenths, a lead that
/// leaps and skips — written to be hummed after one round.
pub fn theme_wav() -> Vec<u8> { theme(152.0) }

/// The same theme, faster: HURRY UP plays it, the way the original's
/// music speeds up when the round has gone on too long.
pub fn hurry_wav() -> Vec<u8> { theme(188.0) }

fn theme(bpm: f32) -> Vec<u8> {
    let lead: [[i32; 8]; 16] = [
        [76, H, 79, H, 84, H, 79, 76],
        [81, H, 76, H, 72, H, 76, 81],
        [77, H, 81, 84, H, 81, 77, H],
        [79, H, H, 83, 86, H, 83, H],
        [76, 79, 84, H, 88, H, 86, 84],
        [81, H, 84, H, 88, H, 86, 84],
        [81, H, 77, H, 79, H, 83, 86],
        [84, H, H, H, RR, 79, 81, 83],
        [84, H, 81, H, 77, H, 81, 84],
        [86, H, 83, H, 79, H, 83, 86],
        [88, H, 83, H, 79, 83, 88, H],
        [84, H, 81, H, 76, H, 81, H],
        [77, 81, 84, 89, H, 88, 86, 84],
        [86, H, 83, 79, H, 81, 83, 86],
        [88, H, 86, H, 84, H, 79, H],
        [84, H, H, H, RR, H, RR, H],
    ];
    // (root, chord intervals) per half bar
    let c = (48, [0, 4, 7]);
    let am = (45, [0, 3, 7]);
    let f = (41, [0, 4, 7]);
    let gch = (43, [0, 4, 7]);
    let em = (40, [0, 3, 7]);
    let chords = [
        [c, c], [am, am], [f, f], [gch, gch], [c, c], [am, am], [f, gch], [c, c],
        [f, f], [gch, gch], [em, em], [am, am], [f, f], [gch, gch], [c, gch], [c, c],
    ];
    let eighth = (60.0 / bpm / 2.0 * SR) as usize;
    let bar = eighth * 8;
    let total = bar * 16;
    let mut buf = vec![0.0f32; total + (0.5 * SR) as usize];
    let mut rng = Rng(0xB0BB_1E55);
    for b in 0..16 {
        let b0 = b * bar;
        // lead
        let mut k = 0;
        while k < 8 {
            let n = lead[b][k];
            if n >= 0 {
                let mut len = 1;
                while k + len < 8 && lead[b][k + len] == H { len += 1; }
                note(&mut buf, b0 + k * eighth, len * eighth - eighth / 6, n, 0.30, Voice::Lead);
                // a harmony a third below in the second half, bell-soft
                if b >= 8 { note(&mut buf, b0 + k * eighth, len * eighth, n - 4, 0.10, Voice::Bell); }
                k += len;
            } else { k += 1; }
        }
        for half in 0..2 {
            let (root, iv) = chords[b][half];
            let h0 = b0 + half * 4 * eighth;
            // bass: root, octave, fifth, octave
            for (s, off) in [0, 12, 7, 12].iter().enumerate() {
                note(&mut buf, h0 + s * eighth, eighth - eighth / 4, root + off, 0.34, Voice::Bass);
            }
            // arpeggio in sixteenths, an octave above middle
            for s in 0..8 {
                let n = root + 24 + iv[s % 3] + if s % 6 >= 3 { 12 } else { 0 };
                note(&mut buf, h0 + s * eighth / 2, eighth / 2, n, 0.07, Voice::Arp);
            }
        }
        // drums
        for beat in 0..4 {
            let t0 = b0 + beat * 2 * eighth;
            if beat % 2 == 0 { kick(&mut buf, t0, 0.55); } else { snare(&mut buf, t0, &mut rng, 0.32); }
            hat(&mut buf, t0 + eighth, &mut rng, 0.12);
            hat(&mut buf, t0, &mut rng, 0.07);
        }
        if b % 4 == 3 {
            // a snare fill into the next phrase
            for s in 0..4 { snare(&mut buf, b0 + 6 * eighth + s * eighth / 2, &mut rng, 0.14 + 0.05 * s as f32); }
            kick(&mut buf, b0 + 7 * eighth, 0.4);
        }
    }
    // A dotted-eighth echo on everything, quiet: the room the chiptune plays in.
    let d = eighth * 3 / 2;
    for i in (d..buf.len()).rev() { buf[i] += buf[i - d] * 0.22; }
    // Fold the tail back onto the start so the loop has no seam.
    let tail = buf.split_off(total);
    for (i, v) in tail.iter().enumerate() { buf[i] += v; }
    let scaled: Vec<f32> = buf.iter().map(|v| v * 21_000.0).collect();
    encode_pcm16_music(&soft_limit_to_pcm16(&scaled, MIX_KNEE))
}

/// Short tunes: `which` 0 round start, 1 round clear, 2 hurry, 3 game over,
/// 4 all rounds won.
pub fn jingle_wav(which: usize) -> Vec<u8> {
    let (notes, step, voice): (&[i32], f32, Voice) = match which {
        0 => (&[72, 76, 79, 84, H, 79, 84, H], 0.09, Voice::Lead),
        1 => (&[79, 84, 88, 91, H, 88, 91, 96, H, H], 0.085, Voice::Lead),
        2 => (&[84, 83, 84, 83, 84, 83, 84, 83, 91, H], 0.06, Voice::Lead),
        3 => (&[72, H, 71, H, 69, H, 67, H, 65, H, 64, H, 60, H, H, H], 0.13, Voice::Lead),
        _ => (&[72, 76, 79, 84, 79, 84, 88, 91, H, 88, 91, 96, H, H, H, H], 0.1, Voice::Lead),
    };
    let st = (step * SR) as usize;
    let mut buf = vec![0.0f32; st * (notes.len() + 6)];
    let mut k = 0;
    while k < notes.len() {
        let n = notes[k];
        if n >= 0 {
            let mut len = 1;
            while k + len < notes.len() && notes[k + len] == H { len += 1; }
            note(&mut buf, k * st, len * st, n, 0.34, voice);
            note(&mut buf, k * st, len * st, n - 12, 0.12, Voice::Bell);
            k += len;
        } else { k += 1; }
    }
    let scaled: Vec<f32> = buf.iter().map(|v| v * 21_000.0).collect();
    encode_pcm16_mono(&soft_limit_to_pcm16(&scaled, MIX_KNEE))
}

/// Effects. `which`: 0 blow a bubble, 1 pop, 2 trap an enemy, 3 an enemy
/// defeated (a bright chirp), 4 fruit, 5 jump, 6 player lost, 7 bounce on
/// a bubble, 8 extra points sparkle.
pub fn sfx_wav(which: usize) -> Vec<u8> {
    let mut rng = Rng(0x5EED_0001 + which as u32 * 977);
    let dur = match which { 0 => 0.16, 1 => 0.12, 2 => 0.3, 3 => 0.35, 4 => 0.22, 5 => 0.18, 6 => 1.1, 7 => 0.16, _ => 0.4 };
    let n = (dur * SR) as usize;
    let mut buf = vec![0.0f32; n];
    let mut ph = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let k = t / dur;
        let w = rng.next_f32() * 2.0 - 1.0;
        *out = match which {
            // blow: a round wet "bloop" rising, with a little air
            0 => {
                ph += (260.0 + 520.0 * k * k) / SR;
                (2.0 * PI * ph).sin() * (1.0 - k).powf(1.5) * 0.8 + w * 0.08 * (1.0 - k)
            }
            // pop: a bright snap
            1 => {
                ph += (1800.0 - 1200.0 * k) / SR;
                (2.0 * PI * ph).sin() * (-t / 0.025).exp() + w * (-t / 0.008).exp() * 0.6
            }
            // trap: a wobbly rising "bwoing"
            2 => {
                ph += (330.0 + 330.0 * k + 40.0 * (2.0 * PI * 18.0 * t).sin()) / SR;
                pulse(ph, 0.3) * (1.0 - k) * 0.55
            }
            // defeated: a quick two-step chirp up
            3 => {
                let f = if k < 0.4 { 880.0 } else { 1320.0 };
                ph += f / SR;
                pulse(ph, 0.25) * (1.0 - k) * 0.5
            }
            // fruit: a bell ding
            4 => {
                ph += 1568.0 / SR;
                ((2.0 * PI * ph).sin() + 0.4 * (2.0 * PI * ph * 2.0).sin()) * (-t / 0.09).exp() * 0.6
            }
            // jump: a springy upward sweep
            5 => {
                ph += (300.0 + 600.0 * k) / SR;
                pulse(ph, 0.5) * (1.0 - k) * 0.35
            }
            // lost a life: a sad falling wobble
            6 => {
                ph += (620.0 * (1.0 - 0.7 * k) + 30.0 * (2.0 * PI * 7.0 * t).sin()) / SR;
                pulse(ph, 0.4) * (1.0 - k).powf(0.8) * 0.45
            }
            // bounce on a bubble: a soft rubbery boing
            7 => {
                ph += (220.0 + 260.0 * (-t / 0.04).exp()) / SR;
                (2.0 * PI * ph).sin() * (1.0 - k) * 0.7
            }
            // sparkle: rising glassy pings
            _ => {
                let step = (k * 6.0) as i32;
                ph += midi_hz(84 + step * 3) / SR;
                (2.0 * PI * ph).sin() * (-(t % (dur / 6.0)) / 0.05).exp() * 0.5
            }
        };
    }
    let scaled: Vec<f32> = buf.iter().map(|v| v * 20_000.0).collect();
    encode_pcm16_mono(&soft_limit_to_pcm16(&scaled, MIX_KNEE))
}
