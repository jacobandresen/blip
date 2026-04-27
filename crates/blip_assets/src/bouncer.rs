//! Bouncer (Breakout) assets.
//!
//! Direct port of `games/bouncer/assets/generate_assets.c`.

use std::f32::consts::PI;

use crate::image::Image;
use crate::wav::{encode_pcm16_mono, env, ms_to_samples, SAMPLE_RATE};
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
                let mut shade = 1.0 - (dx * 0.2 + dy * 0.2) / r;
                if shade < 0.5 { shade = 0.5; }
                if shade > 1.0 { shade = 1.0; }
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

fn music_note(buf: &mut Vec<i16>, freq: f32, ms: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(ms);
    let att = 220;
    let rel = (n / 6).max(1);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = env(i, n, att, rel);
        let w = if freq > 0.0 {
            ((2.0 * PI * freq * t).sin()
                + 0.5 * (4.0 * PI * freq * t).sin()
                + 0.25 * (8.0 * PI * freq * t).sin())
                / 1.75
        } else { 0.0 };
        buf.push((w * e * 22000.0) as i16);
    }
}

fn music() -> Vec<u8> {
    let e = 187.5_f32;
    let seq: &[(f32, f32)] = &[
        (523.25, e), (659.25, e), (783.99, e), (659.25, e),
        (880.00, e), (783.99, e), (659.25, e), (587.33, e),
        (659.25, e), (783.99, e), (880.00, e), (783.99, e),
        (659.25, e), (587.33, e), (523.25, e), (659.25, e),
        (783.99, e), (880.00, e), (783.99, e), (659.25, e),
        (783.99, e), (659.25, e), (587.33, e), (523.25, e),
        (587.33, e), (659.25, e), (783.99, e), (659.25, e),
        (523.25, e), (392.00, e), (440.00, e), (523.25, e),
    ];
    let total: usize = seq.iter().map(|(_, ms)| ms_to_samples(*ms)).sum();
    let mut buf: Vec<i16> = Vec::with_capacity(total);
    for (f, ms) in seq {
        music_note(&mut buf, *f, *ms);
    }
    encode_pcm16_mono(&buf)
}

fn paddle_hit() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 15;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = 1.0 - i as f32 / n as f32;
        s.push((e * 18000.0 * (2.0 * PI * 180.0 * t).sin()) as i16);
    }
    encode_pcm16_mono(&s)
}

fn brick_hit() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 20;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = 1.0 - i as f32 / n as f32;
        s.push((e * 16000.0 * (2.0 * PI * 600.0 * t).sin()) as i16);
    }
    encode_pcm16_mono(&s)
}

fn brick_break() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 10;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = 1.0 - i as f32 / n as f32;
        let freq = 900.0 - 400.0 * i as f32 / n as f32;
        s.push((e * 14000.0 * (2.0 * PI * freq * t).sin()) as i16);
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
        let e = 1.0 - i as f32 / n as f32;
        s.push((e * 18000.0 * (2.0 * PI * freq * t).sin()) as i16);
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
