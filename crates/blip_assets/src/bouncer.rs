//! Bouncer (Breakout) assets — neon rave techno theme.

use std::f32::consts::PI;

use crate::image::Image;
use crate::wav::{encode_pcm16_mono, env, mix_into, ms_to_samples, SAMPLE_RATE};
use crate::Asset;

// ---- visual assets -------------------------------------------------------

fn paddle() -> Vec<u8> {
    let w: i32 = 120;
    let h: i32 = 20;
    let mut img = Image::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let in_corner = ((x < 2 || x >= w - 2) && (y < 2 || y >= h - 2))
                || x < 1
                || x >= w - 1;
            if in_corner {
                continue;
            }
            let t = y as f32 / h as f32;
            // Electric blue → deep cyan gradient
            let r = (15.0 + 20.0 * t) as u8;
            let g = (180.0 + 60.0 * (1.0 - t)) as u8;
            let b = 255u8;
            img.set(x, y, r, g, b);
        }
    }
    // Hot-pink neon highlight stripe at top
    for x in 4..w - 4 {
        img.set(x, 2, 255, 20, 200);
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
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < r {
                // Warm neon yellow-white glow
                let shade = 1.0 - dist / r;
                let rv = 255u8;
                let gv = (200.0 + 55.0 * shade) as u8;
                let bv = (30.0 * shade) as u8;
                img.set(x, y, rv, gv, bv);
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
            let shade: f32 = if y < 3 {
                1.5          // bright neon highlight at top
            } else if y > h - 4 {
                0.45         // deep shadow at bottom
            } else if x < 2 {
                1.3
            } else if x > w - 3 {
                0.6
            } else {
                1.0
            };
            let r = ((br as f32 * shade).min(255.0)) as u8;
            let g = ((bg as f32 * shade).min(255.0)) as u8;
            let b = ((bb as f32 * shade).min(255.0)) as u8;
            img.set(x, y, r, g, b);
        }
    }
    // Near-black border for crisp neon separation
    for x in 0..w {
        img.set(x, 0, 8, 4, 20);
        img.set(x, h - 1, 8, 4, 20);
    }
    for y in 0..h {
        img.set(0, y, 8, 4, 20);
        img.set(w - 1, y, 8, 4, 20);
    }
    img.encode_png()
}

// ---- SFX -----------------------------------------------------------------

fn paddle_hit() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(45.0);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = env(i, n, 2, n / 2);
        // Sharp electronic blip with click transient
        let wave = (2.0 * PI * 280.0 * t).sin()
            + 0.3 * (2.0 * PI * 1400.0 * t).sin() * (1.0 - i as f32 / n as f32);
        s.push((e * 18000.0 * wave / 1.3) as i16);
    }
    encode_pcm16_mono(&s)
}

fn brick_hit() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(35.0);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = env(i, n, 1, n * 2 / 3);
        // Synth square-ish tok
        let wave = (2.0 * PI * 750.0 * t).sin()
            + 0.33 * (2.0 * PI * 2250.0 * t).sin()
            + 0.2 * (2.0 * PI * 3750.0 * t).sin();
        s.push((e * 14000.0 * wave / 1.53) as i16);
    }
    encode_pcm16_mono(&s)
}

fn brick_break() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(120.0);
    let mut rng = Lcg(0xBEEF_1234);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = env(i, n, 2, n / 3);
        let progress = i as f32 / n as f32;
        // Sweep down + noise burst = electronic crunch
        let freq = 1200.0 - 900.0 * progress;
        let tone = (2.0 * PI * freq * t).sin();
        let noise = (rng.next() % 65536) as f32 / 32768.0 - 1.0;
        let wave = 0.6 * tone + 0.4 * noise * (1.0 - progress);
        s.push((e * 16000.0 * wave) as i16);
    }
    encode_pcm16_mono(&s)
}

fn life_lost() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(600.0);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = env(i, n, 40, n / 3);
        let progress = i as f32 / n as f32;
        // Deep techno whomp: low sine sweep down with detuned layer
        let freq = 380.0 * (1.0 - 0.8 * progress);
        let wave = (2.0 * PI * freq * t).sin()
            + 0.4 * (2.0 * PI * freq * 1.5 * t).sin();
        s.push((e * 18000.0 * wave / 1.4) as i16);
    }
    encode_pcm16_mono(&s)
}

fn win() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    // Rave stab fanfare: quick ascending chord hits
    let stabs: &[(f32, f32)] = &[
        (261.63, 0.0),   // C4
        (329.63, 80.0),  // E4
        (392.00, 160.0), // G4
        (523.25, 240.0), // C5
        (659.25, 380.0), // E5
        (783.99, 500.0), // G5
    ];
    let total = ms_to_samples(900.0);
    let mut buf = vec![0i16; total];
    for &(freq, offset_ms) in stabs {
        let off = ms_to_samples(offset_ms);
        let dur = ms_to_samples(350.0);
        for j in 0..dur {
            if off + j >= total { break; }
            let t = (off + j) as f32 / sr;
            let e = env(j, dur, 8, dur / 3);
            // Square-ish stab
            let wave = (2.0 * PI * freq * t).sin()
                + 0.333 * (6.0 * PI * freq * t).sin()
                + 0.2 * (10.0 * PI * freq * t).sin();
            let v = buf[off + j] as i32 + (e * 14000.0 * wave / 1.53) as i32;
            buf[off + j] = v.clamp(-32_767, 32_767) as i16;
        }
    }
    encode_pcm16_mono(&buf)
}

// ---- audio synthesis helpers ---------------------------------------------

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12345) & 0x7FFF_FFFF;
        self.0
    }
}

/// Four-on-the-floor kick: punchy sine sweep 150→45 Hz.
fn mix_kick(buf: &mut [i16], off: usize, sr: f32, amp: f32) {
    let n = ms_to_samples(100.0);
    let sweep = ms_to_samples(55.0);
    for i in 0..n {
        let e = env(i, n, 2, n / 3);
        let t = (off + i) as f32 / sr;
        let p = (i as f32 / sweep as f32).min(1.0);
        let freq = 150.0 - 105.0 * p;
        let main = (2.0 * PI * freq * t).sin();
        let sub = 0.3 * (2.0 * PI * freq * 0.5 * t).sin();
        mix_into(buf, off + i, e * amp * (main + sub) / 1.3);
    }
}

/// Hi-hat: short or long burst of high-frequency noise.
fn mix_hihat(buf: &mut [i16], off: usize, amp: f32, open: bool, seed: u32) {
    let dur_ms = if open { 55.0 } else { 11.0 };
    let n = ms_to_samples(dur_ms);
    let release = (n * 2 / 3).max(1);
    let mut rng = Lcg(seed ^ 0xCAFE_F00D);
    for i in 0..n {
        let e = env(i, n, 1, release);
        let noise = (rng.next() % 65536) as f32 / 32768.0 - 1.0;
        mix_into(buf, off + i, e * amp * noise);
    }
}

/// Acid bass: sawtooth approximation with resonant filter sweep (303-style).
fn mix_acid(buf: &mut [i16], off: usize, sr: f32, freq: f32, dur_ms: f32, amp: f32) {
    let n = ms_to_samples(dur_ms * 0.82);
    for i in 0..n {
        let e = env(i, n, 5, n / 4);
        let t = (off + i) as f32 / sr;
        // Filter opens fast then sweeps closed = classic acid squelch
        let filt = (1.5 - 1.2 * i as f32 / n as f32).clamp(0.05, 1.5);
        let wave = (2.0 * PI * freq * t).sin()
            - filt * 0.5 * (4.0 * PI * freq * t).sin()
            + filt * 0.33 * (6.0 * PI * freq * t).sin()
            - filt * 0.15 * (8.0 * PI * freq * t).sin();
        mix_into(buf, off + i, e * amp * wave / 1.98);
    }
}

/// Synth lead: square-ish wave with slight detune for rave anthem feel.
fn mix_lead(buf: &mut [i16], off: usize, sr: f32, freq: f32, dur_ms: f32, amp: f32) {
    let n = ms_to_samples(dur_ms * 0.88);
    for i in 0..n {
        let e = env(i, n, 18, n / 5);
        let t = (off + i) as f32 / sr;
        // Odd harmonics (square) + subtle detune layer
        let wave = (2.0 * PI * freq * t).sin()
            + 0.333 * (6.0 * PI * freq * t).sin()
            + 0.2 * (10.0 * PI * freq * t).sin()
            + 0.143 * (14.0 * PI * freq * t).sin()
            + 0.12 * (2.0 * PI * freq * 1.008 * t).sin();
        mix_into(buf, off + i, e * amp * wave / 2.0);
    }
}

/// Stab bass: punchy, staccato industrial hit.
fn mix_stab(buf: &mut [i16], off: usize, sr: f32, freq: f32, dur_ms: f32, amp: f32) {
    let n = ms_to_samples(dur_ms * 0.45);
    for i in 0..n {
        let e = env(i, n, 6, n / 3);
        let t = (off + i) as f32 / sr;
        let wave = (2.0 * PI * freq * t).sin()
            + 0.5 * (4.0 * PI * freq * t).sin()
            + 0.25 * (6.0 * PI * freq * t).sin();
        mix_into(buf, off + i, e * amp * wave / 1.75);
    }
}

// ---- music — three techno rave variations --------------------------------
//
//  Variation 1 — Acid Techno (bars 1-4):
//    Classic TB-303 acid bass riff in A minor over four-on-the-floor kick.
//    Closed hi-hats on 8th notes, open hat on off-beats.
//
//  Variation 2 — Rave Anthem / Detroit Techno (bars 5-8):
//    Soaring square-wave lead melody in D minor with 16th-note hi-hat drive.
//    "Hands in the air" anthem feel.
//
//  Variation 3 — Hard Techno / Peak Time (bars 9-12):
//    F minor stab bass, minimal and industrial. Heavy kick, relentless 8th hats.
//
fn music() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;

    const BPM: f32 = 138.0;
    let beat_ms = 60_000.0 / BPM;  // ≈ 434.78 ms
    let e8 = beat_ms / 2.0;         // 8th note
    let e16 = beat_ms / 4.0;        // 16th note

    let section_ms = beat_ms * 16.0; // 4 bars × 4 beats
    let total = ms_to_samples(section_ms * 3.0);
    let mut buf = vec![0i16; total];

    let soff = |ms: f32| -> usize { ms_to_samples(ms) };

    // ---- Variation 1: Acid Techno ----------------------------------------

    let s1 = 0.0_f32;

    // Acid riff in A minor — 8th-note pattern, 2-bar phrase repeated twice
    //  bar A: A2  A2  —  C3  A2  —  G2  F2
    //  bar B: E2  —   E2  G2  A2  A2  —   C3
    let acid_a: &[(f32, bool)] = &[
        (110.00, true),  // A2
        (110.00, true),  // A2
        (0.0,    false), // rest
        (130.81, true),  // C3
        (110.00, true),  // A2
        (0.0,    false), // rest
        (98.00,  true),  // G2
        (87.31,  true),  // F2
    ];
    let acid_b: &[(f32, bool)] = &[
        (82.41,  true),  // E2
        (0.0,    false), // rest
        (82.41,  true),  // E2
        (98.00,  true),  // G2
        (110.00, true),  // A2
        (110.00, true),  // A2
        (0.0,    false), // rest
        (130.81, true),  // C3
    ];

    for bar in 0..4usize {
        let bar_start = s1 + bar as f32 * beat_ms * 4.0;

        // Four-on-the-floor kick
        for b in 0..4usize {
            mix_kick(&mut buf, soff(bar_start + b as f32 * beat_ms), sr, 20000.0);
        }

        // Closed hi-hat every 8th, open hi-hat on 3rd and 7th 8th notes
        for hi in 0..8usize {
            let t = bar_start + hi as f32 * e8;
            let open = hi == 2 || hi == 6;
            let seed = (bar * 100 + hi) as u32;
            mix_hihat(&mut buf, soff(t), 6500.0, open, seed);
        }

        // Acid riff: alternate A/B patterns each bar
        let riff = if bar % 2 == 0 { acid_a } else { acid_b };
        for (idx, &(freq, play)) in riff.iter().enumerate() {
            if play {
                mix_acid(&mut buf, soff(bar_start + idx as f32 * e8), sr, freq, e8, 11000.0);
            }
        }
    }

    // ---- Variation 2: Rave Anthem / Detroit Techno -----------------------

    let s2 = section_ms;

    // Soaring D minor lead melody — 8 notes per bar (8th notes)
    let lead_bars: &[&[(f32, bool)]] = &[
        // bar 1 — descending from D4
        &[
            (293.66, true),  // D4
            (261.63, true),  // C4
            (233.08, true),  // Bb3
            (220.00, true),  // A3
            (196.00, true),  // G3
            (174.61, true),  // F3
            (196.00, true),  // G3
            (220.00, true),  // A3
        ],
        // bar 2 — continue descent then rise
        &[
            (233.08, true),  // Bb3
            (220.00, true),  // A3
            (196.00, true),  // G3
            (174.61, true),  // F3
            (164.81, true),  // E3
            (146.83, true),  // D3
            (164.81, true),  // E3
            (174.61, true),  // F3
        ],
        // bar 3 — lower range, building tension
        &[
            (196.00, true),  // G3
            (174.61, true),  // F3
            (164.81, true),  // E3
            (146.83, true),  // D3
            (130.81, true),  // C3
            (146.83, true),  // D3
            (164.81, true),  // E3
            (174.61, true),  // F3
        ],
        // bar 4 — climb back up, anthem resolve
        &[
            (196.00, true),  // G3
            (220.00, true),  // A3
            (233.08, true),  // Bb3
            (261.63, true),  // C4
            (293.66, true),  // D4
            (261.63, true),  // C4
            (233.08, true),  // Bb3
            (220.00, true),  // A3
        ],
    ];

    for bar in 0..4usize {
        let bar_start = s2 + bar as f32 * beat_ms * 4.0;

        mix_kick(&mut buf, soff(bar_start), sr, 20000.0);
        mix_kick(&mut buf, soff(bar_start + beat_ms), sr, 20000.0);
        mix_kick(&mut buf, soff(bar_start + beat_ms * 2.0), sr, 20000.0);
        mix_kick(&mut buf, soff(bar_start + beat_ms * 3.0), sr, 20000.0);

        // 16th-note hi-hat for relentless drive
        for hi in 0..16usize {
            let t = bar_start + hi as f32 * e16;
            let open = hi % 8 == 4; // open on every 3rd beat off-beat
            let seed = (100 + bar * 100 + hi) as u32;
            mix_hihat(&mut buf, soff(t), 5500.0, open, seed);
        }

        for (idx, &(freq, play)) in lead_bars[bar].iter().enumerate() {
            if play {
                mix_lead(&mut buf, soff(bar_start + idx as f32 * e8), sr, freq, e8, 9500.0);
            }
        }
    }

    // ---- Variation 3: Hard Techno / Peak Time ----------------------------

    let s3 = section_ms * 2.0;

    // F minor stab bass — minimal industrial power riff (8th notes)
    let stab_bars: &[&[(f32, bool)]] = &[
        // bar 1 — tonic stabs
        &[
            (87.31,  true),  // F2
            (0.0,    false),
            (87.31,  true),  // F2
            (87.31,  true),  // F2
            (103.83, true),  // Ab2
            (0.0,    false),
            (77.78,  true),  // Eb2
            (0.0,    false),
        ],
        // bar 2 — up to fifth
        &[
            (87.31,  true),  // F2
            (0.0,    false),
            (87.31,  true),  // F2
            (130.81, true),  // C3  (fifth)
            (87.31,  true),  // F2
            (0.0,    false),
            (103.83, true),  // Ab2
            (0.0,    false),
        ],
        // bar 3 — flat-seven tension
        &[
            (87.31,  true),  // F2
            (0.0,    false),
            (87.31,  true),  // F2
            (87.31,  true),  // F2
            (116.54, true),  // Bb2
            (0.0,    false),
            (103.83, true),  // Ab2
            (77.78,  true),  // Eb2
        ],
        // bar 4 — resolve with power drive
        &[
            (87.31,  true),  // F2
            (77.78,  true),  // Eb2
            (87.31,  true),  // F2
            (103.83, true),  // Ab2
            (116.54, true),  // Bb2
            (0.0,    false),
            (87.31,  true),  // F2
            (0.0,    false),
        ],
    ];

    for bar in 0..4usize {
        let bar_start = s3 + bar as f32 * beat_ms * 4.0;

        // Extra-heavy kick for peak-time energy
        for b in 0..4usize {
            mix_kick(&mut buf, soff(bar_start + b as f32 * beat_ms), sr, 23000.0);
        }

        // Driving 8th-note hi-hat, open hat on beat 3 downbeat
        for hi in 0..8usize {
            let t = bar_start + hi as f32 * e8;
            let open = hi == 4;
            let seed = (200 + bar * 100 + hi) as u32;
            mix_hihat(&mut buf, soff(t), 7500.0, open, seed);
        }

        // Stab riff
        for (idx, &(freq, play)) in stab_bars[bar].iter().enumerate() {
            if play {
                mix_stab(&mut buf, soff(bar_start + idx as f32 * e8), sr, freq, e8, 14000.0);
            }
        }
    }

    encode_pcm16_mono(&buf)
}

// ---- generate ------------------------------------------------------------

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/paddle.png",       paddle()),
        ("images/ball.png",         ball()),
        // Neon rave colour palette (replaces classic rainbow)
        ("images/brick_red.png",    brick((255, 0, 200))),   // hot magenta
        ("images/brick_orange.png", brick((255, 110, 0))),   // neon orange
        ("images/brick_yellow.png", brick((240, 240, 0))),   // electric yellow
        ("images/brick_green.png",  brick((0, 255, 90))),    // neon green
        ("images/brick_blue.png",   brick((0, 80, 255))),    // electric blue
        ("images/brick_purple.png", brick((200, 0, 255))),   // neon violet
        ("sounds/paddle_hit.wav",  paddle_hit()),
        ("sounds/brick_hit.wav",   brick_hit()),
        ("sounds/brick_break.wav", brick_break()),
        ("sounds/life_lost.wav",   life_lost()),
        ("sounds/win.wav",         win()),
        ("sounds/music.wav",       music()),
    ]
}
