//! Galactic Defender assets.
//!
//! Direct port of `games/galactic_defender/assets/generate_assets.c`.

use std::f32::consts::PI;

use crate::image::Image;
use crate::techno::{bass_note, clap, hat, kick, lead_stab, open_hat, Rng};
use crate::wav::{encode_pcm16_mono, ms_to_samples, SAMPLE_RATE};
use crate::Asset;

fn gen_tone(freq: f32, dur_ms: f32, amp: f32) -> Vec<i16> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(dur_ms);
    let fade = SAMPLE_RATE as usize / 200;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let mut e = 1.0_f32;
        if i < fade { e = i as f32 / fade as f32; }
        if i + fade > n { e = (n - i) as f32 / fade as f32; }
        s.push((e * amp * 32000.0 * (2.0 * PI * freq * t).sin()) as i16);
    }
    s
}

/// LCG for deterministic noise (matches C `rand()` behavior loosely; fine for parity).
struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12345) & 0x7FFF_FFFF;
        self.0
    }
}

fn gen_noise(dur_ms: f32, amp: f32) -> Vec<i16> {
    let n = ms_to_samples(dur_ms);
    let fade = SAMPLE_RATE as usize / 200;
    let mut rng = Lcg(1);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let mut e = 1.0_f32;
        if i < fade { e = i as f32 / fade as f32; }
        if i + fade > n { e = (n - i) as f32 / fade as f32; }
        let decay = 1.0 - i as f32 / n as f32;
        let r = rng.next() % 65536;
        let noise = (r as f32 - 32768.0) / 32768.0;
        s.push((e * amp * decay * 32000.0 * noise) as i16);
    }
    s
}

fn player_ship() -> Vec<u8> {
    let w: i32 = 32;
    let h: i32 = 28;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;
    for y in 0..h {
        for x in 0..w {
            let top_half = y as f32 / h as f32;
            let half_w = (1.0 + top_half * (w as f32 / 2.0 - 1.0)) as i32;
            if (x - cx).abs() <= half_w && y > h / 4 {
                img.set(x, y, 0, 200, 200);
            }
            if (x - cx).abs() <= 2 && y <= h / 4 + 2 {
                img.set(x, y, 0, 220, 255);
            }
            if y == h - 1 && (x - cx).abs() <= 4 && (x - cx).abs() >= 2 {
                img.set(x, y, 255, 100, 0);
            }
        }
    }
    img.set(cx, h / 2 - 2, 180, 230, 255);
    img.set(cx - 1, h / 2 - 1, 100, 180, 255);
    img.set(cx + 1, h / 2 - 1, 100, 180, 255);
    img.encode_png()
}

fn alien(kind: usize) -> Vec<u8> {
    let w: i32 = 32;
    let h: i32 = 24;
    let mut img = Image::new(w as u32, h as u32);
    let (r, g, b) = match kind {
        0 => (255u8, 100, 255),
        1 => (0,    220, 220),
        _ => (100,  255, 100),
    };
    let patterns: [[u8; 5]; 3] = [
        [0x0E, 0x1F, 0x15, 0x1F, 0x0A],
        [0x0E, 0x1F, 0x1F, 0x0E, 0x11],
        [0x15, 0x1F, 0x0E, 0x1F, 0x15],
    ];
    let ox = w / 2 - 3;
    let oy = h / 2 - 3;
    for row in 0..5 {
        for col in 0..5 {
            if patterns[kind][row] & (1 << (4 - col)) != 0 {
                let px_x = ox + col * 2;
                let px_y = oy + row as i32 * 2;
                img.set(px_x,     px_y,     r, g, b);
                img.set(px_x + 1, px_y,     r, g, b);
                img.set(px_x,     px_y + 1, r, g, b);
                img.set(px_x + 1, px_y + 1, r, g, b);
            }
        }
    }
    img.set(ox,     oy - 1, r, g, b);
    img.set(ox + 8, oy - 1, r, g, b);
    img.set(ox + 2, oy + 2, 255, 255, 255);
    img.set(ox + 6, oy + 2, 255, 255, 255);
    img.encode_png()
}

fn bullet() -> Vec<u8> {
    let w: i32 = 8;
    let h: i32 = 16;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;
    for y in 0..h {
        img.set(cx,     y, 255, 255, 255);
        img.set(cx - 1, y, 200, 200, 200);
        img.set(cx + 1, y, 200, 200, 200);
    }
    img.encode_png()
}

fn explosion() -> Vec<u8> {
    let w: i32 = 32;
    let h: i32 = 32;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;
    let cy = h / 2;
    let angles = [
        0.0_f32, 0.523, 1.047, 1.571, 2.094, 2.618,
        3.142, 3.665, 4.189, 4.712, 5.236, 5.760,
    ];
    for angle in angles {
        for r in 0..(w / 2 - 1) {
            let x = cx + (r as f32 * angle.cos()) as i32;
            let y = cy + (r as f32 * angle.sin()) as i32;
            let t = r as f32 / (w as f32 / 2.0);
            let red = (255.0 * (1.0 - t)) as u8;
            let green = (150.0 * (1.0 - t)) as u8;
            img.set(x, y, red, green, 0);
        }
    }
    for dy in -2..=2 {
        for dx in -2..=2 {
            img.set(cx + dx, cy + dy, 255, 255, 200);
        }
    }
    img.encode_png()
}

fn shield_block() -> Vec<u8> {
    let w: i32 = 12;
    let h: i32 = 12;
    let mut img = Image::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            img.set(x, y, 0, 180, 0);
        }
    }
    for x in 0..w {
        img.set(x, 0, 100, 255, 100);
    }
    img.encode_png()
}

/// Shared step-sequenced techno renderer for the title and pursuit loops.
/// `bass_hit` marks which 16th steps in the bar trigger a bass note;
/// `bass_roots` cycle every 2 bars; hat/clap density scales with `energy`.
fn techno_loop(
    bpm: f32,
    bars: usize,
    bass_roots: &[f32],
    bass_hit: &[bool; 16],
    stab_note: f32,
    energy: f32,
    seed: u32,
) -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let steps_per_bar = 16;
    let total_steps = bars * steps_per_bar;
    let step_ms = 60_000.0 / bpm / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * total_steps + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0i16; total];
    let mut rng = Rng(seed);

    for step in 0..total_steps {
        let bar = step / steps_per_bar;
        let pos = step % steps_per_bar;
        let off = step * step_samples;

        if pos % 4 == 0 {
            kick(&mut buf, off, 0.9);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.4 * energy);
        }
        if pos % 2 == 1 {
            hat(&mut buf, off, &mut rng, 0.20 * energy);
        }
        if energy > 1.2 && (pos == 6 || pos == 14) {
            open_hat(&mut buf, off, &mut rng, 0.16);
        }
        if bass_hit[pos] {
            let root = bass_roots[(bar / 2) % bass_roots.len()];
            bass_note(&mut buf, off, root, step_ms * 0.7, 0.32);
        }
        if bar % 2 == 1 && pos == 0 {
            lead_stab(&mut buf, off, stab_note, step_ms * 5.0, 0.18 * energy.min(1.4));
        }
    }

    encode_pcm16_mono(&buf)
}

/// Title/default loop — driving mid-tempo techno (~15.7 s @ 124 BPM, 8 bars).
fn music() -> Vec<u8> {
    const HIT: [bool; 16] = [
        true, false, true, false, false, true, false, true,
        true, false, false, true, false, true, false, true,
    ];
    // A-minor roots: A2 - D3 - C3 - D3.
    techno_loop(124.0, 8, &[110.00, 146.83, 130.81, 146.83], &HIT, 440.00, 1.0, 0xD0D0_1000)
}

/// Fast pursuit loop — harder and denser (~13.8 s @ 142 BPM, 8 bars).
fn music2() -> Vec<u8> {
    const HIT: [bool; 16] = [
        true, true, false, true, true, false, true, true,
        false, true, true, false, true, true, false, true,
    ];
    // D-minor roots: D3 - F3 - Eb3 - F3.
    techno_loop(142.0, 8, &[146.83, 174.61, 155.56, 174.61], &HIT, 523.25, 1.5, 0xD0D0_2000)
}

/// Slow, dark dread loop — sparse kick, deep sub-bass drone, distant stabs
/// (~19.5 s @ 100 BPM, 8 bars).
fn music3() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let bpm = 100.0_f32;
    let bars = 8;
    let steps_per_bar = 16;
    let total_steps = bars * steps_per_bar;
    let step_ms = 60_000.0 / bpm / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * total_steps + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0i16; total];
    let mut rng = Rng(0xD0D0_3000);

    // A1 - Bb1 - G1 - Bb1 sub-bass drone, one long note per bar.
    let drone_roots = [55.00_f32, 58.27, 49.00, 58.27];
    const KICK_HIT: [bool; 16] = [
        true, false, false, false, false, false, true, false,
        false, false, true, false, false, false, false, false,
    ];

    for step in 0..total_steps {
        let bar = step / steps_per_bar;
        let pos = step % steps_per_bar;
        let off = step * step_samples;

        if KICK_HIT[pos] {
            kick(&mut buf, off, 0.8);
        }
        if pos == 8 {
            open_hat(&mut buf, off, &mut rng, 0.10);
        }
        if pos == 0 {
            let root = drone_roots[(bar / 2) % drone_roots.len()];
            bass_note(&mut buf, off, root, step_ms * 8.0, 0.30);
        }
        if bar % 4 == 2 && pos == 8 {
            lead_stab(&mut buf, off, 220.0, step_ms * 6.0, 0.14);
        }
    }

    encode_pcm16_mono(&buf)
}

fn game_over_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let freqs = [440.0_f32, 330.0, 220.0, 110.0];
    let total = SAMPLE_RATE as usize * 2;
    let seg = total / 4;
    let mut buf = vec![0i16; total];
    let mut pos = 0;
    for f in freqs {
        for j in 0..seg {
            if pos >= total { break; }
            let t = j as f32 / sr;
            let e = 1.0 - j as f32 / seg as f32;
            buf[pos] = (e * 20000.0 * (2.0 * PI * f * t).sin()) as i16;
            pos += 1;
        }
    }
    encode_pcm16_mono(&buf)
}

fn level_clear_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let freqs = [440.0_f32, 550.0, 660.0, 880.0];
    let seg = SAMPLE_RATE as usize / 4;
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
        ("images/player_ship.png",   player_ship()),
        ("images/alien_squid.png",   alien(0)),
        ("images/alien_crab.png",    alien(1)),
        ("images/alien_octopus.png", alien(2)),
        ("images/bullet.png",        bullet()),
        ("images/explosion.png",     explosion()),
        ("images/shield_block.png",  shield_block()),
        ("sounds/shoot.wav",       encode_pcm16_mono(&gen_tone(880.0, 80.0, 0.6))),
        ("sounds/explosion.wav",   encode_pcm16_mono(&gen_noise(300.0, 0.8))),
        ("sounds/game_over.wav",   game_over_sfx()),
        ("sounds/march.wav",       encode_pcm16_mono(&gen_tone(220.0, 60.0, 0.4))),
        ("sounds/level_clear.wav", level_clear_sfx()),
        ("sounds/music.wav",       music()),
        ("sounds/music2.wav",      music2()),
        ("sounds/music3.wav",      music3()),
    ]
}
