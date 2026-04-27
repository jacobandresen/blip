//! Serpent (Snake) assets.
//!
//! Direct port of `games/serpent/assets/generate_assets.c`.

use std::f32::consts::PI;

use crate::image::Image;
use crate::wav::{encode_pcm16_mono, env, ms_to_samples, SAMPLE_RATE};
use crate::Asset;

const W: u32 = 24;
const H: u32 = 24;

fn head() -> Vec<u8> {
    let mut img = Image::new(W, H);
    let (w, h) = (W as i32, H as i32);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            img.set(x, y, 80, 220, 80);
        }
    }
    for x in 1..w - 1 {
        img.set(x, 1, 150, 255, 150);
        img.set(x, h - 2, 40, 120, 40);
    }
    img.set(w / 2 - 3, h / 2 - 2, 10, 10, 10);
    img.set(w / 2 + 3, h / 2 - 2, 10, 10, 10);
    img.set(w / 2 - 3, h / 2 - 1, 10, 10, 10);
    img.set(w / 2 + 3, h / 2 - 1, 10, 10, 10);
    img.set(w / 2, h - 3, 230, 40, 40);
    img.set(w / 2 - 1, h - 2, 230, 40, 40);
    img.set(w / 2 + 1, h - 2, 230, 40, 40);
    img.encode_png()
}

fn body() -> Vec<u8> {
    let mut img = Image::new(W, H);
    let (w, h) = (W as i32, H as i32);
    for y in 2..h - 2 {
        for x in 2..w - 2 {
            img.set(x, y, 50, 170, 50);
        }
    }
    for x in 4..w - 4 {
        img.set(x, h / 2, 30, 120, 30);
    }
    for x in 2..w - 2 {
        img.set(x, 2, 80, 200, 80);
        img.set(x, h - 3, 30, 110, 30);
    }
    img.encode_png()
}

fn food() -> Vec<u8> {
    let mut img = Image::new(W, H);
    let (w, h) = (W as i32, H as i32);
    let cx = w / 2;
    let cy = h / 2;
    let r = w / 2 - 3;
    for y in 0..h {
        for x in 0..w {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= r * r {
                img.set(x, y, 220, 50, 50);
            }
        }
    }
    img.set(cx - 2, cy - 2, 255, 150, 150);
    img.set(cx - 1, cy - 2, 255, 200, 200);
    img.set(cx, 0, 80, 50, 20);
    img.set(cx + 1, 1, 80, 50, 20);
    img.set(cx + 2, 0, 40, 160, 40);
    img.set(cx + 3, 1, 40, 160, 40);
    img.encode_png()
}

fn music_note(buf: &mut Vec<i16>, freq: f32, ms: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(ms);
    let att = 110;
    let rel = (n / 8).max(1);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = env(i, n, att, rel);
        let w = if freq > 0.0 {
            (((2.0 * PI * freq * t).sin())
                - (1.0 / 9.0) * (6.0 * PI * freq * t).sin()
                + (1.0 / 25.0) * (10.0 * PI * freq * t).sin())
                / 1.08
        } else {
            0.0
        };
        buf.push((w * e * 22000.0) as i16);
    }
}

fn music() -> Vec<u8> {
    let e = 250.0;
    let seq: &[(f32, f32)] = &[
        (261.63, e), (329.63, e), (392.00, e), (440.00, e),
        (392.00, e), (329.63, e), (261.63, e), (329.63, e),
        (392.00, e), (440.00, e), (392.00, e), (329.63, e),
        (392.00, e), (329.63, e), (261.63, e), (293.66, e),
        (329.63, e), (392.00, e), (440.00, e), (392.00, e),
        (329.63, e), (261.63, e), (293.66, e), (329.63, e),
        (392.00, e), (440.00, e), (392.00, e), (329.63, e),
        (293.66, e), (261.63, e), (196.00, e), (261.63, e),
    ];
    let total: usize = seq.iter().map(|(_, ms)| ms_to_samples(*ms)).sum();
    let mut buf: Vec<i16> = Vec::with_capacity(total);
    for (f, ms) in seq {
        music_note(&mut buf, *f, *ms);
    }
    encode_pcm16_mono(&buf)
}

fn eat_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 10;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let freq = 400.0 + 600.0 * i as f32 / n as f32;
        let t = i as f32 / sr;
        let e = 1.0 - i as f32 / n as f32;
        s.push((e * 22000.0 * (2.0 * PI * freq * t).sin()) as i16);
    }
    encode_pcm16_mono(&s)
}

fn move_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 40;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let e = 1.0 - i as f32 / n as f32;
        s.push((e * 5000.0 * (2.0 * PI * 200.0 * i as f32 / sr).sin()) as i16);
    }
    encode_pcm16_mono(&s)
}

fn game_over_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let freqs = [440.0_f32, 349.0, 261.0, 196.0];
    let seg = SAMPLE_RATE as usize / 3;
    let total = seg * 4;
    let mut buf = vec![0i16; total];
    for (i, f) in freqs.iter().enumerate() {
        for j in 0..seg {
            let t = j as f32 / sr;
            let e = 1.0 - j as f32 / seg as f32;
            buf[i * seg + j] = (e * 20000.0 * (2.0 * PI * f * t).sin()) as i16;
        }
    }
    encode_pcm16_mono(&buf)
}

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/head.png", head()),
        ("images/body.png", body()),
        ("images/food.png", food()),
        ("sounds/eat.wav", eat_sfx()),
        ("sounds/move.wav", move_sfx()),
        ("sounds/game_over.wav", game_over_sfx()),
        ("sounds/music.wav", music()),
    ]
}
