//! Serpent (Snake) assets.
//!
//! Direct port of `games/serpent/assets/generate_assets.c`.

use std::f32::consts::PI;

use crate::image::Image;
use crate::techno::{bass_note, clap, hat, kick, open_hat, sidechain_duck, supersaw, Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_mono, soft_limit_to_pcm16, SAMPLE_RATE};
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

/// Shared step-sequenced techno renderer for Serpent's three intensity
/// tiers. `bass_hit` marks which 16th steps in the bar trigger a bass note;
/// `bass_roots` (2 chords) alternate every 2 bars; `hook` is a 4-note riff
/// repeated identically every bar — the same unchanging riff throughout is
/// what makes each tier catchy rather than a new melody every few bars.
/// Hats/claps density scales with `energy`. The back half of the loop
/// (~every 30s) lifts with an octave-up hook harmony and a busier hat/clap
/// layer.
fn techno_loop(
    bpm: f32,
    bars: usize,
    bass_roots: &[f32; 2],
    bass_hit: &[bool; 16],
    hook: &[f32; 4],
    energy: f32,
    seed: u32,
) -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let steps_per_bar = 16;
    let lift_bar = bars / 2;
    let total_steps = bars * steps_per_bar;
    let step_ms = 60_000.0 / bpm / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * total_steps + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(seed);
    let mut kick_offsets = Vec::with_capacity(total_steps / 4);

    for step in 0..total_steps {
        let bar = step / steps_per_bar;
        let pos = step % steps_per_bar;
        let off = step * step_samples;
        let lifted = bar >= lift_bar;

        if pos % 4 == 0 {
            kick_offsets.push(off);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.4 * energy);
        }
        if lifted && pos == 8 {
            clap(&mut buf, off, &mut rng, 0.3 * energy);
        }
        if pos % 2 == 1 {
            hat(&mut buf, off, &mut rng, 0.20 * energy);
        }
        if (energy > 1.1 && (pos == 6 || pos == 14)) || (lifted && pos == 6) {
            open_hat(&mut buf, off, &mut rng, 0.16);
        }
        if bass_hit[pos] {
            let root = bass_roots[(bar / 2) % bass_roots.len()];
            bass_note(&mut buf, off, root, step_ms * 0.7, 0.58);
        }
        if pos % 4 == 0 {
            supersaw(&mut buf, off, hook[pos / 4], step_ms * 3.5, 0.18 * energy.min(1.3), 8.0, 0.008);
            if lifted {
                supersaw(
                    &mut buf,
                    off,
                    hook[pos / 4] * 2.0,
                    step_ms * 3.5,
                    0.09 * energy.min(1.3),
                    8.0,
                    0.008,
                );
            }
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.55, step_ms * 0.85);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.9);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// Base groove — a relaxed mid-tempo techno loop for the early game.
fn slither() -> Vec<u8> {
    const HIT: [bool; 16] = [
        true, false, true, false, false, true, false, true,
        true, false, false, true, false, true, false, false,
    ];
    // C - F vamp; hook: C5 E5 G5 E5.
    const HOOK: [f32; 4] = [523.25, 659.25, 783.99, 659.25];
    techno_loop(120.0, 16, &[130.81, 174.61], &HIT, &HOOK, 1.0, 0x5111_7000)
}

/// Faster, darker minor-key loop — kicks in as the snake grows.
fn stalk() -> Vec<u8> {
    const HIT: [bool; 16] = [
        true, false, true, true, false, true, false, true,
        true, false, true, false, true, true, false, true,
    ];
    // Fm - Bbm vamp; hook: F4 Ab4 C5 Ab4.
    const HOOK: [f32; 4] = [349.23, 415.30, 523.25, 415.30];
    techno_loop(132.0, 16, &[174.61, 233.08], &HIT, &HOOK, 1.25, 0x57A1_4000)
}

/// Hard, fast rave loop for high-level frenzy — dense hats, driving acid bass.
fn frenzy() -> Vec<u8> {
    const HIT: [bool; 16] = [
        true, true, false, true, true, false, true, true,
        false, true, true, false, true, true, false, true,
    ];
    // Am - Dm vamp; hook: A4 C5 E5 C5.
    const HOOK: [f32; 4] = [440.00, 523.25, 659.25, 523.25];
    techno_loop(150.0, 18, &[110.00, 146.83], &HIT, &HOOK, 1.6, 0xF6E2_9000)
}

fn eat_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 10;
    let mut rng = Rng(0x5EA7_0001);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let freq = 400.0 + 600.0 * i as f32 / n as f32;
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.3);
        let fund = (2.0 * PI * freq * t).sin();
        let third = (2.0 * PI * freq * 3.0 * t).sin() / 3.0;
        let sparkle = if i < sr as usize / 300 { (rng.next_f32() * 2.0 - 1.0) * 0.2 } else { 0.0 };
        let shaped = (fund * 0.8 + third * 0.3 + sparkle).tanh();
        s.push((e * 22000.0 * shaped) as i16);
    }
    encode_pcm16_mono(&s)
}

fn move_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 40;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let e = 1.0 - i as f32 / n as f32;
        let t = i as f32 / sr;
        let fund = (2.0 * PI * 200.0 * t).sin();
        let second = (2.0 * PI * 200.0 * 2.0 * t).sin() * 0.3;
        s.push((e * 5000.0 * (fund + second).tanh()) as i16);
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
            let e = (1.0 - j as f32 / seg as f32).powf(1.3);
            let fund = (2.0 * PI * f * t).sin();
            let third = (2.0 * PI * f * 3.0 * t).sin() / 3.0;
            let shaped = (fund * 0.8 + third * 0.3).tanh();
            buf[i * seg + j] = (e * 21000.0 * shaped) as i16;
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
        ("sounds/slither.wav", slither()),
        ("sounds/stalk.wav", stalk()),
        ("sounds/frenzy.wav", frenzy()),
    ]
}
