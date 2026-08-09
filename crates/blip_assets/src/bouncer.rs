//! Bouncer (Breakout) assets.
//!
//! Direct port of `games/bouncer/assets/generate_assets.c`.

use std::f32::consts::PI;

use crate::image::Image;
use crate::techno::{bass_note, clap, hat, kick, lead_stab, open_hat, Rng};
use crate::wav::{encode_pcm16_mono, env, mix_into, SAMPLE_RATE};
use crate::Asset;

fn paddle() -> Vec<u8> {
    let w: i32 = 120;
    let h: i32 = 20;
    let mut img = Image::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let mut in_corner = false;
            if (x < 2 || x >= w - 2) && (y < 2 || y >= h - 2) { in_corner = true; }
            if x < 1 || x >= w - 1 { in_corner = true; }
            if in_corner { continue; }
            let t = y as f32 / h as f32;
            let r = 50u8;
            let g = (100.0 + 100.0 * (1.0 - t)) as u8;
            let b = (200.0 + 55.0 * (1.0 - t)) as u8;
            img.set(x, y, r, g, b);
        }
    }
    for x in 4..w - 4 {
        img.set(x, 2, 150, 220, 255);
    }
    img.encode_png()
}

fn ball() -> Vec<u8> {
    let w: i32 = 16;
    let h: i32 = 16;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;
    let cy = h / 2;
    let r = w as f32 / 2.0 - 1.0;
    for y in 0..h {
        for x in 0..w {
            let dx = (x - cx) as f32 + 0.5;
            let dy = (y - cy) as f32 + 0.5;
            if dx * dx + dy * dy < r * r {
                let shade = (1.0 - (dx * 0.2 + dy * 0.2) / r).clamp(0.5, 1.0);
                let c = (200.0 + 55.0 * shade) as u8;
                img.set(x, y, c, c, c);
            }
        }
    }
    img.set(cx - 2, cy - 2, 255, 255, 255);
    img.set(cx - 1, cy - 2, 255, 255, 255);
    img.set(cx - 2, cy - 1, 255, 255, 255);
    img.encode_png()
}

fn brick(color: (u8, u8, u8)) -> Vec<u8> {
    let w: i32 = 72;
    let h: i32 = 22;
    let mut img = Image::new(w as u32, h as u32);
    let (br, bg, bb) = color;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let mut shade = 1.0_f32;
            if y < 3 { shade = 1.3; }
            if y > h - 4 { shade = 0.6; }
            if x < 2 { shade *= 1.2; }
            if x > w - 3 { shade *= 0.7; }
            let r = ((br as f32 * shade).min(255.0)) as u8;
            let g = ((bg as f32 * shade).min(255.0)) as u8;
            let b = ((bb as f32 * shade).min(255.0)) as u8;
            img.set(x, y, r, g, b);
        }
    }
    for x in 0..w {
        img.set(x, 0, 20, 20, 20);
        img.set(x, h - 1, 20, 20, 20);
    }
    for y in 0..h {
        img.set(0, y, 20, 20, 20);
        img.set(w - 1, y, 20, 20, 20);
    }
    img.encode_png()
}

/// Bouncy plucked lead voice — short, springy, and a little detuned for
/// character. Used for the melodic "bounce" hook over the tech-house groove.
fn pluck(buf: &mut [i16], off: usize, freq: f32, ms: f32, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    let att = (sr * 0.002) as usize;
    let rel = (n * 3 / 4).max(1);
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let e = env(i, n, att.max(1), rel);
        let w = (2.0 * PI * freq * t).sin()
            + 0.5 * (2.0 * PI * freq * 2.003 * t).sin()
            + 0.2 * (2.0 * PI * freq * 4.0 * t).sin();
        mix_into(buf, off + i, w * e * vol * 12000.0);
    }
}

const BPM: f32 = 126.0;
const STEPS_PER_BAR: usize = 16;
const BARS: usize = 8;
const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;

/// A bouncy tech-house banger: four-on-the-floor kick, backbeat claps,
/// off-beat hats, a springy staccato bassline, and a playful pluck hook.
fn music() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0i16; total];
    let mut rng = Rng(0xB0DE_1234);

    // C-major-ish bouncy bass roots, one per 2-bar section: C3 - E3 - F3 - G3.
    let bass_roots = [130.81_f32, 164.81, 174.61, 196.00];
    const BASS_HIT: [bool; STEPS_PER_BAR] = [
        true, false, false, true, false, true, false, false,
        true, false, false, true, false, true, false, true,
    ];
    // Pluck hook cycling every 2 bars — a bright, bouncy C-major arpeggio fragment.
    let hook_notes: [&[f32]; 4] = [
        &[523.25, 659.25, 783.99],
        &[659.25, 783.99, 880.00],
        &[698.46, 880.00, 1046.50],
        &[783.99, 659.25, 523.25],
    ];

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;

        if pos % 4 == 0 {
            kick(&mut buf, off, 0.92);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.55);
        }
        if pos % 2 == 1 {
            hat(&mut buf, off, &mut rng, 0.24);
        }
        if bar % 2 == 1 && pos == 14 {
            open_hat(&mut buf, off, &mut rng, 0.20);
        }
        if BASS_HIT[pos] {
            let root = bass_roots[(bar / 2) % bass_roots.len()];
            bass_note(&mut buf, off, root, step_ms * 0.7, 0.44);
        }
        // Pluck hook: three quick notes starting on the "and" of beat 3 every other bar.
        if bar % 2 == 0 && (pos == 10 || pos == 11 || pos == 12) {
            let notes = hook_notes[(bar / 2) % hook_notes.len()];
            let idx = (pos - 10) as usize;
            pluck(&mut buf, off, notes[idx], step_ms * 1.4, 0.22);
        }
        // Tension stab on the downbeat of every 4th bar, building energy.
        if bar % 4 == 3 && pos == 0 {
            lead_stab(&mut buf, off, 392.00, step_ms * 6.0, 0.18);
        }
    }

    encode_pcm16_mono(&buf)
}

fn paddle_hit() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 15;
    let mut rng = Rng(0xBA11_0001);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.4);
        let fund = (2.0 * PI * 180.0 * t).sin();
        let third = (2.0 * PI * 180.0 * 3.0 * t).sin() / 3.0;
        let tick = if i < sr as usize / 400 { (rng.next_f32() * 2.0 - 1.0) * 0.25 } else { 0.0 };
        let shaped = (fund * 0.8 + third * 0.3 + tick).tanh();
        s.push((e * 19000.0 * shaped) as i16);
    }
    encode_pcm16_mono(&s)
}

fn brick_hit() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 20;
    let mut rng = Rng(0xBA11_0002);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.6);
        let fund = (2.0 * PI * 600.0 * t).sin();
        let fifth = (2.0 * PI * 600.0 * 5.0 * t).sin() / 5.0;
        let tick = if i < sr as usize / 500 { (rng.next_f32() * 2.0 - 1.0) * 0.3 } else { 0.0 };
        let shaped = (fund * 0.75 + fifth * 0.3 + tick).tanh();
        s.push((e * 17000.0 * shaped) as i16);
    }
    encode_pcm16_mono(&s)
}

fn brick_break() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 10;
    let mut rng = Rng(0xBA11_0003);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.3);
        let freq = 900.0 - 400.0 * i as f32 / n as f32;
        let tone = (2.0 * PI * freq * t).sin();
        let crackle = (rng.next_f32() * 2.0 - 1.0) * 0.35;
        let shaped = (tone * 0.85 + crackle).tanh();
        s.push((e * 15000.0 * shaped) as i16);
    }
    encode_pcm16_mono(&s)
}

fn life_lost() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 2;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let freq = 440.0 * (1.0 - 0.5 * i as f32 / n as f32);
        let e = (1.0 - i as f32 / n as f32).powf(1.2);
        let fund = (2.0 * PI * freq * t).sin();
        let detune = (2.0 * PI * freq * 1.008 * t).sin();
        let shaped = (fund * 0.7 + detune * 0.5).tanh();
        s.push((e * 18000.0 * shaped) as i16);
    }
    encode_pcm16_mono(&s)
}

fn win() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let freqs = [440.0_f32, 494.0, 523.0, 587.0, 659.0, 698.0, 784.0, 880.0];
    let seg = SAMPLE_RATE as usize / 6;
    let total = seg * 8;
    let mut buf = vec![0i16; total];
    for (i, f) in freqs.iter().enumerate() {
        for j in 0..seg {
            let t = j as f32 / sr;
            let quarter = seg / 4;
            let e = if j < quarter {
                j as f32 / quarter as f32
            } else {
                (seg - j) as f32 / seg as f32
            };
            buf[i * seg + j] = (e * 20000.0 * (2.0 * PI * f * t).sin()) as i16;
        }
    }
    encode_pcm16_mono(&buf)
}

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/paddle.png", paddle()),
        ("images/ball.png", ball()),
        ("images/brick_red.png",    brick((220, 60, 60))),
        ("images/brick_orange.png", brick((220, 140, 40))),
        ("images/brick_yellow.png", brick((200, 200, 50))),
        ("images/brick_green.png",  brick((50,  200, 80))),
        ("images/brick_blue.png",   brick((50,  100, 220))),
        ("images/brick_purple.png", brick((160, 50,  220))),
        ("sounds/paddle_hit.wav", paddle_hit()),
        ("sounds/brick_hit.wav",  brick_hit()),
        ("sounds/brick_break.wav", brick_break()),
        ("sounds/life_lost.wav",  life_lost()),
        ("sounds/win.wav",        win()),
        ("sounds/music.wav",      music()),
    ]
}
