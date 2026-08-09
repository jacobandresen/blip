//! Galactic Defender assets.
//!
//! Direct port of `games/galactic_defender/assets/generate_assets.c`.

use std::f32::consts::PI;

use crate::image::Image;
use crate::techno::{bass_note, clap, hat, kick, lead_stab, open_hat, Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_mono, ms_to_samples, soft_limit_to_pcm16, SAMPLE_RATE};
use crate::Asset;

// Must match crates/galactic_defender/src/main.rs's ALIEN_W / ALIEN_H.
const ALIEN_W: i32 = 36;
const ALIEN_H: i32 = 28;
// Must match crates/galactic_defender/src/main.rs's UFO_W / UFO_H.
const UFO_W: i32 = 36;
const UFO_H: i32 = 20;

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
        let fund = (2.0 * PI * freq * t).sin();
        let third = (2.0 * PI * freq * 3.0 * t).sin() / 3.0;
        let shaped = (fund * 0.8 + third * 0.3).tanh();
        s.push((e * amp * 27000.0 * shaped) as i16);
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

/// Turn a 9-character '0'/'1' string into a 9-bit row mask (MSB = leftmost).
fn row(s: &str) -> u16 {
    let mut v = 0u16;
    for (i, c) in s.bytes().enumerate() {
        if c == b'1' {
            v |= 1 << (8 - i);
        }
    }
    v
}

/// A gnarly alien glyph — a 9x8 bitmap (finer than the old 5x5/7x7 versions,
/// so there's room for real detail at a smaller footprint), with a second
/// animation frame per kind (mandibles / claws / tentacles shift) for the
/// classic two-frame march-cycle look. `frame` is 0 or 1.
fn alien(kind: usize, frame: usize) -> Vec<u8> {
    let w: i32 = ALIEN_W;
    let h: i32 = ALIEN_H;
    let mut img = Image::new(w as u32, h as u32);
    let (r, g, b) = match kind {
        0 => (255u8, 80, 255),
        1 => (0,    230, 230),
        _ => (110,  255, 110),
    };
    // [frame A, frame B] — rows 0..4 (head/eyes/mandibles) stay put, rows
    // 5..7 (legs / claws / tentacles) swap between frames.
    let patterns: [[u16; 8]; 2] = match kind {
        0 => [
            // Squid: antenna nubs, jagged toothy mandible, legs together.
            [row("001000100"), row("001111100"), row("011111110"),
             row("111111111"), row("110111011"), row("011111110"),
             row("101010101"), row("000101000")],
            // ...antenna twitch inward, legs thrown wide apart.
            [row("000101000"), row("001111100"), row("011111110"),
             row("111111111"), row("110111011"), row("011111110"),
             row("100000001"), row("010000010")],
        ],
        1 => [
            // Crab: flat head, ridged shell, pincers tucked in close.
            [row("000000000"), row("000111000"), row("001111100"),
             row("011111110"), row("111111111"), row("110111011"),
             row("010000010"), row("000101000")],
            // ...pincers thrown wide open, gnarly.
            [row("000000000"), row("000111000"), row("001111100"),
             row("011111110"), row("111111111"), row("110111011"),
             row("100000001"), row("001000100")],
        ],
        _ => [
            // Octopus: round head, tentacle skirt waving one way...
            [row("000000000"), row("000111000"), row("001111100"),
             row("011111110"), row("111111111"), row("011111110"),
             row("101010101"), row("010101010")],
            // ...and the other, for a wavy crawl.
            [row("000000000"), row("000111000"), row("001111100"),
             row("011111110"), row("111111111"), row("011111110"),
             row("010101010"), row("101010101")],
        ],
    };
    let pattern = patterns[frame];
    let cell = 3;
    let cols = 9;
    let rows = 8;
    let ox = (w - cols * cell) / 2;
    let oy = (h - rows * cell) / 2;
    for ry in 0..rows {
        for cx in 0..cols {
            if pattern[ry as usize] & (1 << (cols - 1 - cx)) != 0 {
                let px_x = ox + cx * cell;
                let px_y = oy + ry * cell;
                for dy in 0..cell {
                    for dx in 0..cell {
                        img.set(px_x + dx, px_y + dy, r, g, b);
                    }
                }
            }
        }
    }
    // Eyes on the wide head row: a bright square "sclera" ringed by a dark
    // socket (so they read clearly against every body colour) with a dark
    // pupil in the middle — a plain white square doesn't actually look like
    // an eye. The pupil nudges toward the socket's own offset each frame,
    // like the eyes are darting, for a bit of menace.
    let eye_y = oy + 3 * cell;
    let eye_dx = if frame == 1 { 1 } else { 0 };
    let eye_size = cell + 1;
    for ex in [ox + 3 * cell - eye_dx, ox + 5 * cell + eye_dx] {
        for dy in -1..=eye_size {
            for dx in -1..=eye_size {
                img.set(ex + dx, eye_y + dy, 10, 10, 10);
            }
        }
        for dy in 0..eye_size {
            for dx in 0..eye_size {
                img.set(ex + dx, eye_y + dy, 255, 255, 255);
            }
        }
        let pupil_ox = 1 + eye_dx;
        for dy in 1..=2 {
            for dx in 0..=1 {
                img.set(ex + pupil_ox + dx, eye_y + dy, 15, 15, 20);
            }
        }
    }
    img.encode_png()
}

/// A flying-saucer UFO: a wide metallic disc, a glass dome, and a ring of
/// rim lights. The disc and dome are always drawn upright (rotating the
/// whole bitmap looks jagged with nearest-neighbour pixel-art filtering) —
/// instead, `frame` (0..N_LIGHTS) advances the light ring by one position
/// each call, so cycling through all `N_LIGHTS` frames in order gives a
/// smooth, seamlessly-looping "spinning lights" illusion, classic UFO-toy
/// style, while staying pixel-crisp.
const UFO_N_LIGHTS: usize = 8;

fn ufo_saucer(frame: usize) -> Vec<u8> {
    let w: i32 = UFO_W;
    let h: i32 = UFO_H;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w as f32 / 2.0;
    let cy = h as f32 * 0.62;
    let disc_rx = w as f32 / 2.0 - 1.0;
    let disc_ry = h as f32 * 0.30;

    // Disc body — a squashed metallic-red ellipse with a darker underside band.
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 + 0.5 - cx) / disc_rx;
            let dy = (y as f32 + 0.5 - cy) / disc_ry;
            let d2 = dx * dx + dy * dy;
            if d2 <= 1.0 {
                if dy > 0.25 {
                    img.set(x, y, 150, 30, 40); // shadowed underside
                } else {
                    img.set(x, y, 220, 60, 70);
                }
            }
        }
    }

    // Dome on top.
    let dome_cy = cy - disc_ry * 0.9;
    let dome_r = w as f32 * 0.22;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - dome_cy;
            if dx * dx + dy * dy <= dome_r * dome_r && (y as f32) < cy - disc_ry * 0.35 {
                img.set(x, y, 120, 230, 255);
            }
        }
    }
    img.set(cx as i32 - 1, (dome_cy - dome_r * 0.4) as i32, 220, 250, 255);

    // Rim lights, evenly spaced and alternating colour, offset by `frame`
    // so the ring appears to rotate as frames advance.
    for i in 0..UFO_N_LIGHTS {
        let idx = i + frame;
        let a = (idx as f32 / UFO_N_LIGHTS as f32) * std::f32::consts::PI * 2.0;
        let lx = (cx + a.cos() * disc_rx * 0.88).round() as i32;
        let ly = (cy + a.sin() * disc_ry * 0.88).round() as i32;
        let (r, g, b) = if idx % 2 == 0 { (255u8, 230, 60) } else { (255, 255, 255) };
        img.set(lx, ly, r, g, b);
        img.set(lx, ly - 1, r, g, b);
    }
    img.encode_png()
}

/// A wailing two-tone siren — like a European ambulance's "hi-lo" — so the
/// UFO boss is unmistakable by ear before it's even on screen. One glide
/// cycle (low -> high -> low) that loops seamlessly.
fn ufo_siren() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(1200.0);
    let mut s = Vec::with_capacity(n);
    let f_lo = 650.0_f32;
    let f_hi = 950.0_f32;
    let mut phase = 0.0_f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let tri = if t < 0.5 { t * 2.0 } else { 2.0 - t * 2.0 };
        let freq = f_lo + (f_hi - f_lo) * tri;
        phase += freq / sr;
        let fund = (2.0 * PI * phase).sin();
        let third = (2.0 * PI * phase * 3.0).sin() / 4.0;
        let shaped = (fund * 0.85 + third * 0.25).tanh();
        s.push((shaped * 20000.0) as i16);
    }
    encode_pcm16_mono(&s)
}

/// One ominous thumping march step — a low pitch-swept thud with a driven
/// sub-octave layer for extra weight. Four of these at descending base
/// frequencies (classic Space Invaders "duh-duh-duh-duh") cycle as the
/// aliens advance.
fn march_thump(base_freq: f32) -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(150.0);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.5);
        let freq = base_freq + base_freq * 2.5 * (-t / 0.05).exp();
        let fund = (2.0 * PI * freq * t).sin();
        let sub = (2.0 * PI * freq * 0.5 * t).sin();
        let shaped = (fund * 0.75 + sub * 0.8).tanh();
        s.push((e * 26000.0 * shaped) as i16);
    }
    encode_pcm16_mono(&s)
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
    let mut buf = vec![0f32; total];
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
            bass_note(&mut buf, off, root, step_ms * 0.7, 0.60);
        }
        if bar % 2 == 1 && pos == 0 {
            lead_stab(&mut buf, off, stab_note, step_ms * 5.0, 0.18 * energy.min(1.4));
        }
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
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
    let mut buf = vec![0f32; total];
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
            bass_note(&mut buf, off, root, step_ms * 8.0, 0.54);
        }
        if bar % 4 == 2 && pos == 8 {
            lead_stab(&mut buf, off, 220.0, step_ms * 6.0, 0.14);
        }
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
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
            let e = (1.0 - j as f32 / seg as f32).powf(1.3);
            let fund = (2.0 * PI * f * t).sin();
            let third = (2.0 * PI * f * 3.0 * t).sin() / 3.0;
            let shaped = (fund * 0.8 + third * 0.3).tanh();
            buf[pos] = (e * 19000.0 * shaped) as i16;
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
            let e = (1.0 - j as f32 / seg as f32).powf(1.3);
            let fund = (2.0 * PI * f * t).sin();
            let third = (2.0 * PI * f * 3.0 * t).sin() / 3.0;
            let shaped = (fund * 0.8 + third * 0.3).tanh();
            buf[i * seg + j] = (e * 19000.0 * shaped) as i16;
        }
    }
    encode_pcm16_mono(&buf)
}

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/player_ship.png",     player_ship()),
        ("images/alien_squid_a.png",   alien(0, 0)),
        ("images/alien_squid_b.png",   alien(0, 1)),
        ("images/alien_crab_a.png",    alien(1, 0)),
        ("images/alien_crab_b.png",    alien(1, 1)),
        ("images/alien_octopus_a.png", alien(2, 0)),
        ("images/alien_octopus_b.png", alien(2, 1)),
        ("images/bullet.png",        bullet()),
        ("images/explosion.png",     explosion()),
        ("images/shield_block.png",  shield_block()),
        ("images/ufo_saucer_0.png",  ufo_saucer(0)),
        ("images/ufo_saucer_1.png",  ufo_saucer(1)),
        ("images/ufo_saucer_2.png",  ufo_saucer(2)),
        ("images/ufo_saucer_3.png",  ufo_saucer(3)),
        ("images/ufo_saucer_4.png",  ufo_saucer(4)),
        ("images/ufo_saucer_5.png",  ufo_saucer(5)),
        ("images/ufo_saucer_6.png",  ufo_saucer(6)),
        ("images/ufo_saucer_7.png",  ufo_saucer(7)),
        ("sounds/shoot.wav",       encode_pcm16_mono(&gen_tone(880.0, 80.0, 0.6))),
        ("sounds/explosion.wav",   encode_pcm16_mono(&gen_noise(300.0, 0.8))),
        ("sounds/game_over.wav",   game_over_sfx()),
        ("sounds/march1.wav",      march_thump(98.0)),
        ("sounds/march2.wav",      march_thump(87.0)),
        ("sounds/march3.wav",      march_thump(78.0)),
        ("sounds/march4.wav",      march_thump(70.0)),
        ("sounds/level_clear.wav", level_clear_sfx()),
        ("sounds/ufo_siren.wav",   ufo_siren()),
        ("sounds/music.wav",       music()),
        ("sounds/music2.wav",      music2()),
        ("sounds/music3.wav",      music3()),
    ]
}
