//! RIVET asset generation — sounds and placeholder screenshot.

use std::f32::consts::PI;

use crate::image::Image;
use crate::wav::{encode_pcm16_mono, env, mix_into, ms_to_samples, SAMPLE_RATE};
use crate::Asset;

fn jump() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(110.0);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = env(i, n, 8, 35);
        let freq = 200.0 + 900.0 * (i as f32 / n as f32);
        s.push((e * 20000.0 * (2.0 * PI * freq * t).sin()) as i16);
    }
    encode_pcm16_mono(&s)
}

fn die() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(700.0);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = env(i, n, 60, 200);
        let freq = 440.0 * (1.0 - 0.78 * i as f32 / n as f32);
        let wave = (2.0 * PI * freq * t).sin()
            + 0.4 * (2.0 * PI * freq * 1.5 * t).sin();
        s.push((e * 14000.0 * wave / 1.4) as i16);
    }
    encode_pcm16_mono(&s)
}

fn score_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(160.0);
    let half = n / 2;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let seg = i % half;
        let e = env(seg, half, 5, 30);
        let freq = if i < half { 660.0_f32 } else { 880.0 };
        s.push((e * 18000.0 * (2.0 * PI * freq * t).sin()) as i16);
    }
    encode_pcm16_mono(&s)
}

fn win() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let notes: &[f32] = &[261.63, 329.63, 392.0, 523.25, 659.25, 783.99];
    let n_each = ms_to_samples(140.0);
    let total = n_each * notes.len();
    let mut buf = Vec::with_capacity(total);
    for (ni, &freq) in notes.iter().enumerate() {
        for j in 0..n_each {
            let t = (ni * n_each + j) as f32 / sr;
            let e = env(j, n_each, 20, 55);
            let wave = (2.0 * PI * freq * t).sin()
                + 0.45 * (4.0 * PI * freq * t).sin();
            buf.push((e * 18000.0 * wave / 1.45) as i16);
        }
    }
    encode_pcm16_mono(&buf)
}

fn music() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let beat = ms_to_samples(240.0);
    let freqs: &[f32] = &[
        65.41, 65.41, 98.0, 65.41, 87.31, 87.31, 110.0, 87.31,
        65.41, 65.41, 98.0, 65.41, 87.31, 87.31, 116.54, 87.31,
    ];
    let total = beat * freqs.len();
    let mut buf = vec![0i16; total];
    for (ni, &freq) in freqs.iter().enumerate() {
        let off = ni * beat;
        let att = 70;
        let rel = beat / 3;
        for j in 0..beat {
            let t = (off + j) as f32 / sr;
            let e = env(j, beat, att, rel);
            let wave = (2.0 * PI * freq * t).sin()
                + 0.3 * (4.0 * PI * freq * t).sin();
            mix_into(&mut buf, off + j, e * 14000.0 * wave / 1.3);
        }
    }
    encode_pcm16_mono(&buf)
}

pub fn screenshot() -> Vec<u8> {
    let w = 320u32;
    let h = 180u32;
    let mut img = Image::new(w, h);

    // Black background
    // Platforms (steel blue)
    let plat_color = (46, 107, 204);
    let plat_hl    = (115, 174, 255);
    let lad_color  = (200, 160, 0);
    let plats: &[(i32, i32, i32)] = &[
        (0, 160, 320),
        (10, 128, 150),
        (10, 96, 150),
        (10, 64, 150),
        (10, 32, 150),
    ];
    for &(x1, y, x2) in plats {
        for x in x1..x2 {
            for dy in 0..4i32 {
                let (r, g, b) = if dy < 1 { plat_hl } else { plat_color };
                img.set(x, y + dy, r, g, b);
            }
        }
    }
    // Ladders
    let lads: &[(i32, i32, i32)] = &[
        (122, 32, 160),
        (30, 64, 128),
        (122, 96, 128),
        (30, 32, 64),
    ];
    for &(x, y1, y2) in lads {
        for y in y1..y2 {
            img.set(x, y, lad_color.0, lad_color.1, lad_color.2);
            img.set(x + 7, y, lad_color.0, lad_color.1, lad_color.2);
        }
        let mut y = y1;
        while y < y2 {
            for dx in 0..7i32 {
                img.set(x + dx, y, lad_color.0, lad_color.1, lad_color.2);
            }
            y += 5;
        }
    }
    // Gorilla (brown blob)
    for y in 20..32i32 {
        for x in 14..26i32 {
            img.set(x, y, 140, 69, 18);
        }
    }
    // Title text placeholder (just colored dots to suggest "RIVET")
    // White dots for player
    for dy in 0..4i32 {
        for dx in 0..3i32 {
            img.set(74 + dx, 140 + dy, 220, 80, 80);
        }
    }

    img.encode_png()
}

pub fn generate() -> Vec<Asset> {
    vec![
        ("sounds/jump.wav",  jump()),
        ("sounds/die.wav",   die()),
        ("sounds/score.wav", score_sfx()),
        ("sounds/win.wav",   win()),
        ("sounds/music.wav", music()),
    ]
}
