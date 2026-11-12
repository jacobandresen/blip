// Compiled by build.rs, and by the game for its tests: each uses part of this file.
#![allow(dead_code)]

//! WARLOCK's score and effects, synthesised here: bowed cellos and strings,
//! a choir pad, a plucked harp, war drums, bells, and a stone-cellar reverb
//! over all of it. Dark fantasy in D minor; the melodies are our own.

use blip::cosy::{midi_hz, SR};
use blip::wav::{encode_pcm16_half, encode_pcm16_music, low_pass, mix_into_f32, soft_limit_to_pcm16, Rng, MIX_KNEE};
use blip::Asset;
use std::f32::consts::TAU;

fn secs(s: f32) -> usize {
    (s * SR) as usize
}

// ---- instruments ---------------------------------------------------------------

/// A bowed string: two detuned saws through a soft low-pass, bow-swelled,
/// with vibrato that arrives once the note has settled.
fn bowed(buf: &mut [f32], at: usize, dur: f32, midi: i32, vol: f32, bright: f32) {
    let f = midi_hz(midi);
    let n = secs(dur);
    let rel = secs(0.35);
    let att = secs(0.06 + 0.1 * (1.0 - bright));
    let (mut p1, mut p2, mut lp, mut lp2) = (0.0f32, 0.37f32, 0.0f32, 0.0f32);
    let cut = (2.0 * TAU * (300.0 + 2600.0 * bright) / SR).min(0.9);
    for i in 0..n + rel {
        let t = i as f32 / SR;
        let vib = 1.0 + 0.004 * (TAU * 5.3 * t).sin() * ((t - 0.25) / 0.3).clamp(0.0, 1.0);
        p1 = (p1 + f * vib / SR).fract();
        p2 = (p2 + f * vib * 1.004 / SR).fract();
        let raw = (2.0 * p1 - 1.0) + (2.0 * p2 - 1.0) * 0.8;
        lp += cut * (raw - lp);
        lp2 += cut * (lp - lp2);
        let e = if i < att {
            i as f32 / att as f32
        } else if i < n {
            0.85 + 0.15 * (-(t - att as f32 / SR) * 3.0).exp()
        } else {
            0.85 * (1.0 - (i - n) as f32 / rel as f32)
        };
        mix_into_f32(buf, at + i, lp2 * e * vol);
    }
}

/// A short, biting string stroke for driving ostinatos (spiccato).
fn stroke(buf: &mut [f32], at: usize, midi: i32, vol: f32) {
    let f = midi_hz(midi);
    let n = secs(0.16);
    let (mut p, mut lp) = (0.0f32, 0.0f32);
    let cut = 2.0 * TAU * 1400.0 / SR;
    for i in 0..n {
        let t = i as f32 / SR;
        p = (p + f / SR).fract();
        lp += cut * ((2.0 * p - 1.0) - lp);
        let e = (t / 0.006).min(1.0) * (-t * 16.0).exp();
        mix_into_f32(buf, at + i, lp * e * vol);
    }
}

/// A wordless choir: stacked soft voices, each a little detuned, slow to swell.
fn choir(buf: &mut [f32], at: usize, dur: f32, midi: i32, vol: f32) {
    let f = midi_hz(midi);
    let n = secs(dur);
    let rel = secs(0.8);
    let att = secs(0.45);
    let detune = [0.997f32, 1.0, 1.0035];
    let mut ph = [0.0f32, 0.33, 0.66];
    for i in 0..n + rel {
        let t = i as f32 / SR;
        let mut s = 0.0;
        for (k, d) in detune.iter().enumerate() {
            let vib = 1.0 + 0.005 * (TAU * (4.6 + k as f32 * 0.4) * t + k as f32).sin();
            ph[k] = (ph[k] + f * d * vib / SR).fract();
            let x = TAU * ph[k];
            // an "ah": the fundamental with its second and third partials
            s += x.sin() + 0.45 * (2.0 * x).sin() + 0.25 * (3.0 * x).sin() + 0.08 * (5.0 * x).sin();
        }
        let e = if i < att {
            (i as f32 / att as f32).powf(1.5)
        } else if i < n {
            1.0
        } else {
            1.0 - (i - n) as f32 / rel as f32
        };
        mix_into_f32(buf, at + i, s * e * vol / 3.0);
    }
}

/// A harp string (Karplus-Strong): bright attack, warm decay.
fn harp(buf: &mut [f32], at: usize, midi: i32, vol: f32, rng: &mut Rng) {
    let f = midi_hz(midi);
    let period = (SR / f).max(2.0) as usize;
    let mut line: Vec<f32> = (0..period).map(|_| rng.next_f32() * 2.0 - 1.0).collect();
    for i in 1..period {
        line[i] = line[i] * 0.5 + line[i - 1] * 0.5;
    }
    let n = secs(2.2);
    for i in 0..n {
        let idx = i % period;
        let nxt = (i + 1) % period;
        let out = line[idx];
        line[idx] = (line[idx] + line[nxt]) * 0.5 * 0.998;
        mix_into_f32(buf, at + i, out * vol * (1.0 - i as f32 / n as f32));
    }
}

/// A war drum: a deep skin that drops in pitch, a thud of felt on top.
fn drum(buf: &mut [f32], at: usize, f0: f32, vol: f32, decay: f32, rng: &mut Rng) {
    let n = secs(decay * 2.0);
    let mut ph = 0.0f32;
    let mut lp = 0.0f32;
    for i in 0..n {
        let t = i as f32 / SR;
        let f = f0 * (1.0 + 1.4 * (-t * 22.0).exp());
        ph = (ph + f / SR).fract();
        let body = (TAU * ph).sin() * (-t / decay).exp();
        lp += 0.08 * ((rng.next_f32() * 2.0 - 1.0) - lp);
        let thud = lp * (-t * 40.0).exp() * 2.5;
        mix_into_f32(buf, at + i, (body + thud) * vol);
    }
}

/// A frame drum or rim: a bright, short slap for the faster pieces.
fn slap(buf: &mut [f32], at: usize, vol: f32, rng: &mut Rng) {
    let n = secs(0.18);
    let (mut lp, mut hp) = (0.0f32, 0.0f32);
    for i in 0..n {
        let t = i as f32 / SR;
        let w = rng.next_f32() * 2.0 - 1.0;
        lp += 0.35 * (w - lp);
        hp = lp - hp * 0.2;
        let tone = (TAU * 190.0 * t).sin() * (-t * 30.0).exp();
        mix_into_f32(buf, at + i, (hp * 0.8 + tone) * (-t * 22.0).exp() * vol);
    }
}

/// A tolling bell: inharmonic partials, each fading at its own pace.
fn bell(buf: &mut [f32], at: usize, midi: i32, vol: f32, ring: f32) {
    let f = midi_hz(midi);
    let partials = [(1.0f32, 1.0f32, 1.0f32), (2.0, 0.6, 0.7), (2.76, 0.5, 0.45), (5.4, 0.3, 0.25), (8.93, 0.18, 0.15)];
    let n = secs(ring);
    for i in 0..n {
        let t = i as f32 / SR;
        let mut s = 0.0;
        for &(m, a, d) in &partials {
            s += (TAU * f * m * t).sin() * a * (-t / (ring * d * 0.5)).exp();
        }
        mix_into_f32(buf, at + i, s * vol * (t / 0.002).min(1.0));
    }
}

/// A rising or falling wash of breath-noise: wind through a cellar, a whoosh.
fn wind(buf: &mut [f32], at: usize, dur: f32, f0: f32, f1: f32, vol: f32, rng: &mut Rng) {
    let n = secs(dur);
    let (mut a, mut b) = (0.0f32, 0.0f32);
    for i in 0..n {
        let k = i as f32 / n as f32;
        let fc = f0 + (f1 - f0) * k;
        let g = (TAU * fc / SR).min(0.95);
        let w = rng.next_f32() * 2.0 - 1.0;
        a += g * (w - a);
        b += g * (a - b);
        let e = (k * 6.0).min(1.0) * (1.0 - k).powf(1.4);
        mix_into_f32(buf, at + i, (a - b) * 3.0 * e * vol);
    }
}

// ---- the room --------------------------------------------------------------------

/// A stone cellar: four damped combs into two all-passes (Schroeder), mixed
/// back under the dry signal. `size` scales the delays, `wet` the level.
fn cellar(buf: &mut Vec<f32>, size: f32, wet: f32, tail: f32) {
    buf.extend(std::iter::repeat_n(0.0, secs(tail)));
    let combs = [1557usize, 1617, 1491, 1422].map(|d| (d as f32 * size) as usize);
    let mut out = vec![0.0f32; buf.len()];
    for &d in &combs {
        let mut line = vec![0.0f32; d];
        let mut lp = 0.0f32;
        for i in 0..buf.len() {
            let y = line[i % d];
            lp = y * 0.65 + lp * 0.35;
            line[i % d] = buf[i] + lp * 0.84;
            out[i] += y;
        }
    }
    for &d in &[225usize, 556] {
        let d = (d as f32 * size) as usize;
        let mut line = vec![0.0f32; d];
        for (i, v) in out.iter_mut().enumerate() {
            let y = line[i % d];
            let nv = *v + y * 0.5;
            line[i % d] = nv;
            *v = y - nv * 0.5;
        }
    }
    for (s, r) in buf.iter_mut().zip(out.iter()) {
        *s += r * wet * 0.25;
    }
}

/// Fold the tail of a loop back over its head so it repeats without a seam.
fn wrap(mut buf: Vec<f32>, body: usize) -> Vec<f32> {
    let tail: Vec<f32> = buf.drain(body..).collect();
    for (i, v) in tail.into_iter().enumerate() {
        if i < buf.len() {
            buf[i] += v;
        }
    }
    buf
}

fn finish_music(mut buf: Vec<f32>, gain: f32) -> Vec<u8> {
    low_pass(&mut buf, 6000.0);
    let scaled: Vec<f32> = buf.iter().map(|s| s * gain).collect();
    encode_pcm16_music(&soft_limit_to_pcm16(&scaled, MIX_KNEE))
}

fn finish_sfx(mut buf: Vec<f32>, gain: f32) -> Vec<u8> {
    low_pass(&mut buf, 9000.0);
    let scaled: Vec<f32> = buf.iter().map(|s| s * gain).collect();
    encode_pcm16_half(&soft_limit_to_pcm16(&scaled, MIX_KNEE))
}

// ---- the score ----------------------------------------------------------------------

// D minor: D E F G A Bb C (and C# for the dominant).
const D2: i32 = 38;

/// Chord roots of the cellar's eight-bar round, two bars each.
const ROUND: [(i32, bool); 8] = [(38, true), (38, true), (34, false), (34, false), (31, true), (33, false), (38, true), (36, false)];

/// The descent: a slow 3/4 dirge. Cellos turn an ostinato, the choir holds
/// the chords, drums mark each bar, a lone string sings over the second
/// half, and a bell tolls the start of every round.
pub fn dungeon() -> Vec<u8> {
    let bpm = 80.0;
    let beat = 60.0 / bpm;
    let bar = beat * 3.0;
    let bars = 16;
    let body = secs(bar * bars as f32);
    let mut buf = vec![0.0f32; body + secs(4.0)];
    let mut rng = Rng(0x051a_b1e5);
    // the ostinato: six eighths a bar, root-fifth-octave and a turn
    let fig = [0, 7, 12, 3, 7, 0];
    // the solo string sings the title's horn call, turned into the dirge's 3/4
    let tune: [(i32, f32); 22] = [
        (62, 1.5), (69, 1.5), (69, 2.0), (67, 1.0), (65, 3.0), (70, 3.0),
        (69, 2.0), (67, 1.0), (65, 1.5), (64, 1.5), (62, 6.0),
        (62, 1.5), (69, 1.5), (74, 2.0), (72, 1.0), (70, 3.0), (74, 3.0),
        (72, 1.5), (70, 1.5), (69, 3.0), (67, 3.0), (69, 0.0),
    ];
    for b in 0..bars {
        let (root, minor) = ROUND[(b / 2) % 8];
        let t0 = b as f32 * bar;
        for (k, iv) in fig.iter().enumerate() {
            let at = secs(t0 + k as f32 * beat / 2.0);
            let accent = if k == 0 { 1.0 } else { 0.7 };
            bowed(&mut buf, at, beat * 0.45, root + iv, 1400.0 * accent, 0.35);
        }
        // the choir and low strings hold the chord through every two bars
        if b % 2 == 0 {
            let third = if minor { 3 } else { 4 };
            for iv in [12, 12 + third, 19] {
                choir(&mut buf, secs(t0), bar * 2.0 - 0.1, root + iv, 1100.0);
            }
            bowed(&mut buf, secs(t0), bar * 2.0 - 0.2, root - 12 + 12, 900.0, 0.2);
        }
        // drums: a deep stroke on one, a softer pair leading into the next bar
        drum(&mut buf, secs(t0), 52.0, 9000.0, 0.45, &mut rng);
        if b % 4 == 3 {
            drum(&mut buf, secs(t0 + beat * 2.0), 60.0, 5500.0, 0.3, &mut rng);
            drum(&mut buf, secs(t0 + beat * 2.5), 64.0, 6000.0, 0.3, &mut rng);
        } else if b % 2 == 1 {
            drum(&mut buf, secs(t0 + beat * 2.0), 58.0, 4000.0, 0.3, &mut rng);
        }
        // harp sparks on the off-beats of the first half
        if b < 8 && b % 2 == 1 {
            for (k, iv) in [12, 19, 24, 27].iter().enumerate() {
                harp(&mut buf, secs(t0 + beat * (0.5 + k as f32 * 0.5)), root + 24 + iv - 12, 2600.0, &mut rng);
            }
        }
    }
    bell(&mut buf, 0, 50, 2600.0, 5.0);
    bell(&mut buf, secs(bar * 8.0), 50, 2600.0, 5.0);
    // the solo string sings over the second round
    let mut t = bar * 8.0;
    for &(m, beats) in &tune {
        if beats > 0.0 {
            bowed(&mut buf, secs(t), beats * beat - 0.04, m, 1600.0, 0.6);
        }
        t += beats * beat;
    }
    let _ = D2;
    cellar(&mut buf, 1.3, 0.9, 0.0);
    finish_music(wrap(buf, body), 2.2)
}

/// The Warlock's sanctum: driving 4/4, spiccato cellos in sixteenths, war
/// drums and choir stabs, a high string line that will not settle.
pub fn tower() -> Vec<u8> {
    let bpm = 132.0;
    let beat = 60.0 / bpm;
    let bar = beat * 4.0;
    let bars = 16;
    let body = secs(bar * bars as f32);
    let mut buf = vec![0.0f32; body + secs(3.0)];
    let mut rng = Rng(0x70_e2);
    let roots = [38, 38, 34, 36, 38, 38, 41, 40, 38, 38, 34, 36, 31, 33, 38, 37];
    let pat = [0, 0, 12, 0, 7, 0, 12, 0, 0, 0, 12, 0, 3, 0, 7, 12];
    for b in 0..bars {
        let root = roots[b];
        let t0 = b as f32 * bar;
        for (k, iv) in pat.iter().enumerate() {
            let acc = if k % 4 == 0 { 1.0 } else { 0.6 };
            stroke(&mut buf, secs(t0 + k as f32 * beat / 4.0), root + iv, 3200.0 * acc);
        }
        drum(&mut buf, secs(t0), 50.0, 9000.0, 0.35, &mut rng);
        drum(&mut buf, secs(t0 + beat * 1.5), 56.0, 6000.0, 0.25, &mut rng);
        drum(&mut buf, secs(t0 + beat * 2.0), 50.0, 8000.0, 0.35, &mut rng);
        slap(&mut buf, secs(t0 + beat), 3000.0, &mut rng);
        slap(&mut buf, secs(t0 + beat * 3.0), 3000.0, &mut rng);
        if b % 4 == 3 {
            for k in 0..4 {
                drum(&mut buf, secs(t0 + beat * (3.0 + k as f32 * 0.25)), 62.0 + k as f32 * 4.0, 5000.0, 0.2, &mut rng);
            }
        }
        // choir stabs on one and the and-of-two
        for (k, at) in [0.0f32, 2.5].iter().enumerate() {
            let dur = if k == 0 { beat * 1.5 } else { beat * 1.2 };
            for iv in [12, 15, 19] {
                choir(&mut buf, secs(t0 + at * beat), dur, root + iv, 900.0);
            }
        }
    }
    // the high line: long notes climbing and falling back
    let line: [(i32, f32); 18] = [
        (74, 4.0), (73, 2.0), (74, 2.0), (77, 4.0), (76, 4.0),
        (74, 2.0), (72, 2.0), (70, 4.0), (69, 8.0),
        (74, 4.0), (77, 2.0), (79, 2.0), (81, 4.0), (82, 4.0),
        (81, 2.0), (77, 2.0), (76, 4.0), (74, 8.0),
    ];
    let mut t = 0.0;
    for &(m, beats) in &line {
        bowed(&mut buf, secs(t), beats * beat - 0.03, m, 1500.0, 0.7);
        t += beats * beat;
    }
    // in the second half the horns take up the wizard's call, as if in answer
    let call: [(i32, f32); 12] = [
        (62, 2.0), (69, 2.0), (69, 3.0), (67, 1.0), (65, 4.0), (70, 4.0),
        (69, 3.0), (67, 1.0), (65, 2.0), (64, 2.0), (69, 4.0), (73, 4.0),
    ];
    let mut t = bar * 8.0;
    for &(m, beats) in &call {
        horn(&mut buf, secs(t), beats * beat - 0.04, m - 12, 1500.0, &mut rng);
        horn(&mut buf, secs(t), beats * beat - 0.04, m - 24, 900.0, &mut rng);
        t += beats * beat;
    }
    cellar(&mut buf, 1.0, 0.7, 0.0);
    finish_music(wrap(buf, body), 2.2)
}

/// A horn: two saws whose filter opens with the breath, then settles warm.
fn horn(buf: &mut [f32], at: usize, dur: f32, midi: i32, vol: f32, rng: &mut Rng) {
    let f = midi_hz(midi);
    let n = secs(dur);
    let rel = secs(0.25);
    let (mut p1, mut p2, mut p3, mut lp, mut lp2) = (0.0f32, 0.5f32, 0.25f32, 0.0f32, 0.0f32);
    for i in 0..n + rel {
        let t = i as f32 / SR;
        let open = 500.0 + 2200.0 * (t / 0.09).min(1.0) * (0.65 + 0.35 * (-t * 2.5).exp());
        let cut = (TAU * open / SR).min(0.9);
        // brass speaks a little under the note and lips up to it
        let scoop = 1.0 - 0.018 * (-t * 28.0).exp();
        let vib = scoop * (1.0 + 0.005 * (TAU * 4.8 * t).sin() * ((t - 0.35) / 0.4).clamp(0.0, 1.0));
        p1 = (p1 + f * vib / SR).fract();
        p2 = (p2 + f * vib * 1.006 / SR).fract();
        p3 = (p3 + f * 0.5 * vib / SR).fract();
        let raw = (2.0 * p1 - 1.0) + (2.0 * p2 - 1.0) + 0.45 * (2.0 * p3 - 1.0);
        lp += cut * (raw - lp);
        lp2 += cut * (lp - lp2);
        // a breath of air at the lip as the note speaks
        let breath = (rng.next_f32() * 2.0 - 1.0) * (-t * 30.0).exp() * 0.35;
        let e = if i < n { (t / 0.05).min(1.0) * (0.8 + 0.2 * (-t * 4.0).exp()) } else { 0.8 * (1.0 - (i - n) as f32 / rel as f32) };
        mix_into_f32(buf, at + i, (lp2 + breath) * e * vol);
    }
}

/// The wizard's theme, for the title: a rolling triplet ostinato under a
/// horn call that climbs out of D minor, choir and timpani behind it, a
/// great bell on the downbeat of each half, and a dark wind swelling at the turn.
pub fn theme() -> Vec<u8> {
    let bpm = 96.0;
    let beat = 60.0 / bpm;
    let bar = beat * 4.0;
    let bars = 24;
    let body = secs(bar * bars as f32);
    let mut buf = vec![0.0f32; body + secs(4.0)];
    let mut rng = Rng(0x7e3e);
    // (root, third) of each bar: Dm Bb C A | Dm Bb Gm A | Bb F Gm A | Dm Bb A Dm
    let chords: [(i32, i32); 16] = [
        (38, 3), (34, 4), (36, 4), (33, 4), (38, 3), (34, 4), (31, 3), (33, 4),
        (34, 4), (29, 4), (31, 3), (33, 4), (38, 3), (34, 4), (33, 4), (38, 3),
    ];
    for (b, &(root, third)) in chords.iter().enumerate() {
        let t0 = b as f32 * bar;
        // the ostinato: eighth-note triplets, root-fifth-octave-tenth and back
        let fig = [0, 7, 12, 12 + third, 12, 7];
        for k in 0..12 {
            let at = secs(t0 + k as f32 * beat / 3.0);
            let iv = fig[k % 6];
            let acc = if k % 3 == 0 { 1.0 } else { 0.65 };
            if b < 8 {
                harp(&mut buf, at, root + 12 + iv, 2000.0 * acc, &mut rng);
            } else {
                harp(&mut buf, at, root + 12 + iv, 1500.0 * acc, &mut rng);
                stroke(&mut buf, at, root + iv, 2400.0 * acc);
            }
        }
        // low strings and choir hold the chord
        bowed(&mut buf, secs(t0), bar - 0.1, root, 1000.0, 0.25);
        let lift = if b >= 8 { 1.3 } else { 1.0 };
        for iv in [12, 12 + third, 19, 24] {
            choir(&mut buf, secs(t0), bar - 0.05, root + iv, 800.0 * lift);
        }
        // timpani on one and three, a roll into every fourth bar
        drum(&mut buf, secs(t0), 49.0, 9000.0, 0.5, &mut rng);
        drum(&mut buf, secs(t0 + beat * 2.0), 55.0, 6000.0, 0.4, &mut rng);
        if b % 4 == 3 {
            for k in 0..6 {
                let q = k as f32 / 6.0;
                drum(&mut buf, secs(t0 + beat * (3.0 + q)), 55.0, 2500.0 + 5000.0 * q, 0.2, &mut rng);
            }
        }
    }
    // the great bell, and the dark wind that rises at the turn
    bell(&mut buf, 0, 38, 3800.0, 6.0);
    bell(&mut buf, secs(bar * 8.0), 38, 3800.0, 6.0);
    wind(&mut buf, secs(bar * 6.0), bar * 2.2, 200.0, 1400.0, 2600.0, &mut rng);
    wind(&mut buf, secs(bar * 14.0), bar * 2.0, 1600.0, 300.0, 2200.0, &mut rng);
    // the horn call; in the second half the strings double it an octave up
    let call: [(i32, f32); 46] = [
        (62, 1.0), (69, 1.0), (69, 1.5), (67, 0.5), (65, 2.0), (70, 2.0),
        (69, 1.5), (67, 0.5), (65, 1.0), (64, 1.0), (69, 4.0),
        (62, 1.0), (69, 1.0), (74, 1.5), (72, 0.5), (70, 2.0), (74, 2.0),
        (72, 1.0), (70, 1.0), (69, 1.0), (67, 1.0), (69, 2.0), (73, 2.0),
        (74, 2.0), (77, 2.0), (76, 1.5), (74, 0.5), (72, 2.0),
        (70, 1.0), (72, 1.0), (74, 1.0), (79, 1.0), (77, 2.0), (76, 2.0),
        (74, 1.0), (81, 1.0), (81, 1.5), (79, 0.5), (77, 2.0), (82, 2.0),
        (81, 1.5), (79, 0.5), (77, 1.0), (76, 1.0), (74, 4.0), (0, 0.0),
    ];
    let mut t = 0.0;
    for &(m, beats) in &call {
        if beats == 0.0 {
            continue;
        }
        let dur = beats * beat - 0.05;
        let second = t >= bar * 8.0;
        horn(&mut buf, secs(t), dur, m - 12, if second { 1400.0 } else { 1800.0 }, &mut rng);
        if second {
            bowed(&mut buf, secs(t), dur, m, 1300.0, 0.75);
        }
        t += beats * beat;
    }
    // in the second half: a cello answering the call, frame drums on two and
    // four, and a low choir on the open fifth beneath it all
    let answer: [(i32, f32); 14] = [
        (58, 2.0), (62, 2.0), (57, 2.0), (60, 2.0), (58, 2.0), (62, 2.0), (61, 4.0),
        (57, 2.0), (62, 2.0), (58, 2.0), (65, 2.0), (64, 2.0), (61, 2.0), (62, 4.0),
    ];
    let mut t = bar * 8.0;
    for &(m, beats) in &answer {
        bowed(&mut buf, secs(t), beats * beat - 0.05, m - 12, 1100.0, 0.5);
        t += beats * beat;
    }
    for b in 8..16 {
        let t0 = b as f32 * bar;
        slap(&mut buf, secs(t0 + beat), 2200.0, &mut rng);
        slap(&mut buf, secs(t0 + beat * 3.0), 2200.0, &mut rng);
        if b % 2 == 0 {
            for m in [26, 33] {
                choir(&mut buf, secs(t0), bar * 2.0 - 0.1, m + 12, 700.0);
            }
        }
    }
    // a harp sweeps up into each half
    let scale = [0, 2, 3, 5, 7, 8, 10];
    for end in [bar * 8.0, bar * 24.0] {
        for k in 0..14 {
            let m = 50 + 12 * (k / 7) as i32 + scale[k % 7];
            harp(&mut buf, secs(end - beat * 2.0 + k as f32 * beat * 2.0 / 14.0), m + 12, 1700.0, &mut rng);
        }
    }
    // the third part: the dark arts. The light goes out of the harmony; a low
    // choir chants on D with the Phrygian half step above it, a great bell
    // tolls, the strings hold a drone, and the timpani gather into the return.
    let b0 = 16.0 * bar;
    for k in 0..8 {
        let t0 = b0 + k as f32 * bar;
        if k % 2 == 0 {
            bowed(&mut buf, secs(t0), bar * 2.0 - 0.1, 26, 1300.0, 0.2);
            bowed(&mut buf, secs(t0), bar * 2.0 - 0.1, 33, 900.0, 0.2);
            bell(&mut buf, secs(t0), 38, 2600.0, 5.0);
        }
        drum(&mut buf, secs(t0), 46.0, 6000.0, 0.6, &mut rng);
        // a slow pulse of plucked fifths, falling back by a half step
        for (q, m) in [(0.0f32, 50), (1.0, 57), (2.0, 51), (3.0, 57)] {
            harp(&mut buf, secs(t0 + q * beat), m, 1500.0, &mut rng);
        }
    }
    let chant: [(i32, f32); 14] = [
        (62, 4.0), (63, 4.0), (62, 4.0), (60, 4.0),
        (62, 2.0), (63, 2.0), (65, 4.0), (63, 4.0), (62, 4.0),
        (63, 2.0), (65, 2.0), (66, 2.0), (65, 2.0), (63, 4.0),
    ];
    let mut t = b0;
    for &(m, beats) in &chant {
        choir(&mut buf, secs(t), beats * beat - 0.1, m - 12, 1300.0);
        choir(&mut buf, secs(t), beats * beat - 0.1, m - 24, 900.0);
        t += beats * beat;
    }
    wind(&mut buf, secs(b0 + bar * 4.0), bar * 4.0, 150.0, 900.0, 2400.0, &mut rng);
    for k in 0..16 {
        let q = k as f32 / 16.0;
        drum(&mut buf, secs(b0 + bar * 7.0 + q * bar), 52.0, 2000.0 + 7000.0 * q, 0.22, &mut rng);
    }
    cellar(&mut buf, 1.4, 0.95, 0.0);
    finish_music(wrap(buf, body), 1.6)
}

/// The deeper wings, from the seventh chamber: Phrygian and restless. A
/// cello pulses on D against the half step above it, frame drums turn in
/// threes, the choir moans, and low in the strings the wizard's call is
/// heard again, slower, as if from far below.
pub fn depths() -> Vec<u8> {
    let bpm = 104.0;
    let beat = 60.0 / bpm;
    let bar = beat * 4.0;
    let bars = 16;
    let body = secs(bar * bars as f32);
    let mut buf = vec![0.0f32; body + secs(4.0)];
    let mut rng = Rng(0xde_e9);
    // two bars on D, one on Eb, one back on D; the turn falls to C and Bb
    let roots = [38, 38, 39, 38, 38, 38, 36, 34, 38, 38, 39, 38, 41, 39, 37, 38];
    for b in 0..bars {
        let root = roots[b];
        let t0 = b as f32 * bar;
        // the pulse: triplet eighths, root and the half step, accent every third
        for k in 0..12 {
            let iv = [0, 0, 1, 0, 0, 7, 0, 1, 0, 12, 7, 1][k];
            let acc = if k % 3 == 0 { 1.0 } else { 0.55 };
            stroke(&mut buf, secs(t0 + k as f32 * beat / 3.0), root + iv, 2600.0 * acc);
        }
        drum(&mut buf, secs(t0), 48.0, 8000.0, 0.45, &mut rng);
        drum(&mut buf, secs(t0 + beat * 2.0 + beat / 3.0), 54.0, 5000.0, 0.3, &mut rng);
        slap(&mut buf, secs(t0 + beat), 2200.0, &mut rng);
        slap(&mut buf, secs(t0 + beat * 3.0), 2200.0, &mut rng);
        slap(&mut buf, secs(t0 + beat * 3.0 + beat * 2.0 / 3.0), 1400.0, &mut rng);
        if b % 2 == 0 {
            for iv in [12, 15, 19] {
                choir(&mut buf, secs(t0), bar * 2.0 - 0.1, root + iv, 850.0);
            }
        }
    }
    bell(&mut buf, 0, 38, 3000.0, 6.0);
    bell(&mut buf, secs(bar * 8.0), 39, 3000.0, 6.0);
    wind(&mut buf, secs(bar * 7.0), bar, 300.0, 1500.0, 2000.0, &mut rng);
    wind(&mut buf, secs(bar * 15.0), bar, 300.0, 1500.0, 2000.0, &mut rng);
    // the call, slow and low, the second time with its last notes bent to Eb
    let call: [(i32, f32); 12] = [
        (50, 4.0), (57, 4.0), (57, 6.0), (55, 2.0), (53, 8.0), (58, 8.0),
        (57, 6.0), (55, 2.0), (53, 4.0), (52, 4.0), (51, 8.0), (50, 8.0),
    ];
    let mut t = 0.0;
    for &(m, beats) in &call {
        bowed(&mut buf, secs(t), beats * beat - 0.05, m, 1500.0, 0.45);
        t += beats * beat;
    }
    cellar(&mut buf, 1.3, 0.9, 0.0);
    finish_music(wrap(buf, body), 1.8)
}

// ---- effects ---------------------------------------------------------------------------

fn fx(len: f32) -> Vec<f32> {
    vec![0.0; secs(len)]
}

/// A cast bolt: a crack of air and a bright arcing zap that sings upward.
fn bolt_sfx() -> Vec<u8> {
    let mut b = fx(0.5);
    let mut rng = Rng(0xb017);
    wind(&mut b, 0, 0.22, 2500.0, 6000.0, 9000.0, &mut rng);
    let n = secs(0.25);
    let mut ph = 0.0f32;
    for i in 0..n {
        let t = i as f32 / SR;
        let f = 600.0 + 2200.0 * (t / 0.25);
        ph = (ph + f / SR).fract();
        let crackle = if rng.next_f32() < 0.08 { rng.next_f32() * 2.0 - 1.0 } else { 0.0 };
        let s = ((TAU * ph).sin() + 0.4 * (TAU * ph * 2.01).sin()) * (-t * 14.0).exp() + crackle * 0.7 * (-t * 10.0).exp();
        mix_into_f32(&mut b, i, s * 7000.0);
    }
    cellar(&mut b, 0.6, 0.6, 0.2);
    finish_sfx(b, 2.0)
}

/// Reveal: a deep breath of air, then a shimmer of voices rising through it.
fn reveal_sfx() -> Vec<u8> {
    let mut b = fx(1.4);
    let mut rng = Rng(0x5ee);
    wind(&mut b, 0, 0.9, 300.0, 2400.0, 10000.0, &mut rng);
    for (k, m) in [74, 81, 86].iter().enumerate() {
        choir(&mut b, secs(k as f32 * 0.07), 0.5, *m, 2600.0);
    }
    for (k, m) in [86, 89, 93, 98].iter().enumerate() {
        bell(&mut b, secs(0.05 + k as f32 * 0.06), *m, 1300.0, 1.0);
    }
    cellar(&mut b, 0.9, 0.8, 0.4);
    finish_sfx(b, 2.0)
}

/// A cleansed stone: a choir rises into a major chord over a warm bell.
fn cleanse_sfx() -> Vec<u8> {
    let mut b = fx(2.0);
    bell(&mut b, 0, 62, 3500.0, 2.0);
    for (k, m) in [62, 66, 69, 74].iter().enumerate() {
        choir(&mut b, secs(k as f32 * 0.06), 0.9, *m, 2200.0);
    }
    bell(&mut b, secs(0.25), 81, 1400.0, 1.4);
    cellar(&mut b, 1.0, 0.8, 0.5);
    finish_sfx(b, 2.0)
}

/// A monster unmade: a burst of flame and a thud that drops away.
fn kill_sfx() -> Vec<u8> {
    let mut b = fx(0.7);
    let mut rng = Rng(0x4111);
    wind(&mut b, 0, 0.35, 4000.0, 500.0, 12000.0, &mut rng);
    drum(&mut b, 0, 70.0, 9000.0, 0.15, &mut rng);
    cellar(&mut b, 0.7, 0.5, 0.2);
    finish_sfx(b, 2.0)
}

/// A wizard falls: a low string sighs downward under a struck gong.
fn death_sfx() -> Vec<u8> {
    let mut b = fx(2.4);
    let mut rng = Rng(0xdea7);
    bell(&mut b, 0, 38, 4500.0, 2.4);
    bowed(&mut b, 0, 0.5, 57, 3500.0, 0.5);
    bowed(&mut b, secs(0.45), 0.5, 53, 3500.0, 0.45);
    bowed(&mut b, secs(0.9), 0.9, 50, 3800.0, 0.4);
    drum(&mut b, 0, 45.0, 8000.0, 0.5, &mut rng);
    cellar(&mut b, 1.2, 0.8, 0.5);
    finish_sfx(b, 2.0)
}

/// Game over: a slow choir dirge settling on D minor, a last bell.
fn over_sfx() -> Vec<u8> {
    let mut b = fx(4.5);
    let steps = [(70, 62, 65), (69, 61, 64), (65, 62, 57)];
    for (k, &(a, c, d)) in steps.iter().enumerate() {
        let t = secs(k as f32 * 1.1);
        for m in [a, c, d] {
            choir(&mut b, t, 1.05, m, 1800.0);
        }
        bowed(&mut b, t, 1.0, [46, 45, 38][k], 2200.0, 0.3);
    }
    bell(&mut b, secs(2.2), 50, 3000.0, 2.2);
    cellar(&mut b, 1.3, 0.9, 0.3);
    finish_sfx(b, 2.0)
}

/// A chamber cleared: a short fanfare of strings and choir, ending in D major.
fn chamber_sfx() -> Vec<u8> {
    let mut b = fx(2.0);
    let mut rng = Rng(0xc1ea);
    for (k, &(m, d)) in [(62, 0.14), (69, 0.14), (74, 0.6)].iter().enumerate() {
        bowed(&mut b, secs(k as f32 * 0.15), d, m, 3500.0, 0.8);
    }
    for m in [62, 66, 69, 74] {
        choir(&mut b, secs(0.3), 0.8, m, 1800.0);
    }
    drum(&mut b, secs(0.3), 55.0, 8000.0, 0.4, &mut rng);
    cellar(&mut b, 1.0, 0.7, 0.4);
    finish_sfx(b, 2.0)
}

/// A ward crystal shatters: a burst of glassy rings over a heavy boom.
fn ward_sfx() -> Vec<u8> {
    let mut b = fx(1.6);
    let mut rng = Rng(0x3a7d);
    drum(&mut b, 0, 40.0, 11000.0, 0.6, &mut rng);
    wind(&mut b, 0, 0.3, 7000.0, 3000.0, 9000.0, &mut rng);
    for k in 0..7 {
        let m = 88 + (rng.next_f32() * 12.0) as i32;
        bell(&mut b, secs(k as f32 * 0.025), m, 900.0, 0.7);
    }
    cellar(&mut b, 1.1, 0.8, 0.4);
    finish_sfx(b, 2.0)
}

/// The Warlock's curse: a low growling voice that slides down a fifth.
fn warlock_sfx() -> Vec<u8> {
    let mut b = fx(1.1);
    let n = secs(0.7);
    let (mut ph, mut lp) = (0.0f32, 0.0f32);
    for i in 0..n {
        let t = i as f32 / SR;
        let f = 110.0 * (1.0 - 0.33 * (t / 0.7));
        ph = (ph + f / SR).fract();
        let raw = 2.0 * ph - 1.0;
        // a vowel that closes from "ah" to "oo"
        let cut = 2.0 * TAU * (900.0 - 600.0 * t / 0.7) / SR;
        lp += cut * (raw - lp);
        let e = (t / 0.05).min(1.0) * (1.0 - t / 0.7);
        mix_into_f32(&mut b, i, lp * e * 9000.0 * (1.0 + 0.3 * (TAU * 7.0 * t).sin()));
    }
    choir(&mut b, 0, 0.6, 50, 1500.0);
    cellar(&mut b, 1.2, 0.8, 0.3);
    finish_sfx(b, 2.0)
}

/// A gargoyle's fireball: a roaring whoosh.
fn fire_sfx() -> Vec<u8> {
    let mut b = fx(0.8);
    let mut rng = Rng(0xf12e);
    wind(&mut b, 0, 0.6, 400.0, 1600.0, 16000.0, &mut rng);
    drum(&mut b, 0, 60.0, 5000.0, 0.15, &mut rng);
    cellar(&mut b, 0.7, 0.5, 0.2);
    finish_sfx(b, 2.0)
}

/// The cellar's heartbeat: a soft lub-dub that quickens as the chamber empties.
fn pulse_sfx() -> Vec<u8> {
    let mut b = fx(0.7);
    let mut rng = Rng(0x91);
    drum(&mut b, 0, 46.0, 7000.0, 0.16, &mut rng);
    drum(&mut b, secs(0.17), 42.0, 5000.0, 0.16, &mut rng);
    finish_sfx(b, 2.0)
}

/// A dragon whelp: a rising, rasping screech.
fn whelp_sfx() -> Vec<u8> {
    let mut b = fx(0.9);
    let mut rng = Rng(0xd2a6);
    let n = secs(0.55);
    let (mut ph, mut lp) = (0.0f32, 0.0f32);
    for i in 0..n {
        let t = i as f32 / SR;
        let f = 500.0 + 900.0 * (t / 0.2).min(1.0) - 300.0 * (t / 0.55);
        ph = (ph + f * (1.0 + 0.03 * (TAU * 33.0 * t).sin()) / SR).fract();
        let raw = (2.0 * ph - 1.0) + (rng.next_f32() - 0.5) * 0.6;
        lp += 0.3 * (raw - lp);
        let e = (t / 0.03).min(1.0) * (1.0 - t / 0.55);
        mix_into_f32(&mut b, i, lp * e * 9000.0);
    }
    cellar(&mut b, 0.9, 0.7, 0.3);
    finish_sfx(b, 2.0)
}

/// A potion drunk: a glassy rising arpeggio over a soft shimmer.
fn potion_sfx() -> Vec<u8> {
    let mut b = fx(0.9);
    let mut rng = Rng(0x9071);
    for (k, m) in [74, 78, 81, 86, 90].iter().enumerate() {
        bell(&mut b, secs(k as f32 * 0.045), *m, 2600.0 - k as f32 * 200.0, 0.9);
    }
    wind(&mut b, 0, 0.5, 3000.0, 7000.0, 900.0, &mut rng);
    cellar(&mut b, 0.8, 0.6, 0.3);
    finish_sfx(b, 2.0)
}

/// The effects, as `sounds/NAME.wav`.
pub fn sounds() -> Vec<Asset> {
    vec![
        ("sounds/bolt.wav", bolt_sfx()),
        ("sounds/reveal.wav", reveal_sfx()),
        ("sounds/cleanse.wav", cleanse_sfx()),
        ("sounds/kill.wav", kill_sfx()),
        ("sounds/death.wav", death_sfx()),
        ("sounds/over.wav", over_sfx()),
        ("sounds/chamber.wav", chamber_sfx()),
        ("sounds/ward.wav", ward_sfx()),
        ("sounds/warlock.wav", warlock_sfx()),
        ("sounds/fire.wav", fire_sfx()),
        ("sounds/pulse.wav", pulse_sfx()),
        ("sounds/whelp.wav", whelp_sfx()),
        ("sounds/potion.wav", potion_sfx()),
    ]
}

/// The music: each tune as a WAV to play.
pub fn music() -> Vec<(String, Vec<u8>)> {
    blip::wav::tunes(&[("DUNGEON", &dungeon), ("TOWER", &tower), ("THEME", &theme), ("DEPTHS", &depths)])
}
