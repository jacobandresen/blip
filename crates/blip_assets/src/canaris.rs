//! Canaris – procedural assets (sprites + audio).

use std::f32::consts::PI;

use crate::image::Image;
use crate::wav::{encode_pcm16_mono, env, ms_to_samples, SAMPLE_RATE};
use crate::Asset;

// ── helpers ──────────────────────────────────────────────────────────────────

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345) & 0x7FFF_FFFF;
        self.0
    }
}

fn noise_burst(dur_ms: f32, amp: f32, decay: f32) -> Vec<i16> {
    let n = ms_to_samples(dur_ms);
    let fade = (SAMPLE_RATE as usize / 400).max(1);
    let mut rng = Lcg(42);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let mut e = 1.0_f32;
        if i < fade { e = i as f32 / fade as f32; }
        if i + fade > n { e = (n - i) as f32 / fade as f32; }
        let t = 1.0 - (i as f32 / n as f32).powf(decay);
        let r = rng.next() % 65536;
        let v = (r as f32 - 32768.0) / 32768.0;
        s.push((e * t * amp * 28000.0 * v) as i16);
    }
    s
}

fn sine_tone(freq: f32, dur_ms: f32, amp: f32) -> Vec<i16> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(dur_ms);
    let att = (n / 8).max(1);
    let rel = (n / 4).max(1);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = env(i, n, att, rel);
        s.push((e * amp * 28000.0 * (2.0 * PI * freq * t).sin()) as i16);
    }
    s
}

fn square_note(buf: &mut Vec<i16>, freq: f32, dur_ms: f32, amp: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(dur_ms);
    let att = (n / 8).max(1);
    let rel = (n / 3).max(1);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = env(i, n, att, rel);
        let sq = if (freq * t).fract() < 0.5 { 1.0_f32 } else { -1.0 };
        buf.push((e * amp * 18000.0 * sq) as i16);
    }
}

fn rest(buf: &mut Vec<i16>, dur_ms: f32) {
    let n = ms_to_samples(dur_ms);
    buf.extend(std::iter::repeat(0i16).take(n));
}

// ── images ────────────────────────────────────────────────────────────────────

/// Fill a horizontal band in an image.
fn hband(img: &mut Image, x0: i32, x1: i32, y0: i32, y1: i32, r: u8, g: u8, b: u8) {
    for y in y0..y1 {
        for x in x0..x1 {
            img.set(x, y, r, g, b);
        }
    }
}

/// Draw a ship sprite.  facing_right = player ship; !facing_right = enemy ship.
fn ship(facing_right: bool, sail_open: bool) -> Vec<u8> {
    let w: i32 = 48;
    let h: i32 = 32;
    let mut img = Image::new(w as u32, h as u32);

    // Colour palette
    let (hull_dark, hull_mid, hull_hi, deck_col, sail_col, flag_r, flag_b) =
        if facing_right {
            // player: cyan hull
            ((0u8, 110u8, 130u8), (0, 160, 185), (60, 220, 240), (20, 200, 220), (230, 230, 200), 200u8, 30u8)
        } else {
            // enemy: brown hull
            ((100u8, 60u8, 20u8), (160, 100, 40), (200, 140, 70), (140, 90, 40), (200, 190, 170), 180u8, 40u8)
        };

    // Lower hull (keel shape, tapered)
    for y in 22..30 {
        let taper = ((y - 22) as f32 / 8.0 * 6.0) as i32;
        let (lx, rx) = if facing_right {
            (4 + taper, 46 - taper / 2)
        } else {
            (4 + taper / 2, 46 - taper)
        };
        for x in lx..rx {
            img.set(x, y, hull_dark.0, hull_dark.1, hull_dark.2);
        }
    }

    // Upper hull body
    hband(&mut img, 4, 46, 16, 22, hull_mid.0, hull_mid.1, hull_mid.2);

    // Hull highlight along top edge
    for x in 4..46 { img.set(x, 16, hull_hi.0, hull_hi.1, hull_hi.2); }

    // Deck stripe
    hband(&mut img, 6, 44, 14, 16, deck_col.0, deck_col.1, deck_col.2);

    // Cannon ports (3 along hull)
    for cx in [11, 23, 35].iter() {
        img.set(*cx, 19, 15, 15, 25);
        img.set(*cx + 1, 19, 15, 15, 25);
    }

    // Mast
    let mx = if facing_right { 16 } else { 30 };
    for y in 2..14 { img.set(mx, y, 160, 120, 60); }
    img.set(mx, 14, 130, 95, 50);

    // Sail
    let sail_w: i32 = if sail_open { 13 } else { 9 };
    for y in 3..13 {
        let frac = (y - 3) as f32 / 10.0;
        // elliptical billow
        let sw = (sail_w as f32 * (1.0 - (frac * 2.0 - 1.0).powi(2)).sqrt()) as i32;
        let (sx, ex) = if facing_right {
            (mx - sw, mx)
        } else {
            (mx + 1, mx + 1 + sw)
        };
        for x in sx..ex {
            img.set(x, y, sail_col.0, sail_col.1, sail_col.2);
        }
        // sail edge
        if sw > 0 {
            let edge = if facing_right { sx } else { ex - 1 };
            img.set(edge, y, (sail_col.0 as i32 - 40).max(0) as u8,
                             (sail_col.1 as i32 - 40).max(0) as u8,
                             (sail_col.2 as i32 - 40).max(0) as u8);
        }
    }

    // Flag at mast top
    let (fx, fy) = if facing_right { (mx + 1, 2) } else { (mx - 2, 2) };
    img.set(fx,     fy,     flag_r, 50, flag_b);
    img.set(fx + 1, fy,     flag_r, 50, flag_b);
    img.set(fx,     fy + 1, flag_r / 2, 50, flag_b);

    img.encode_png()
}

fn player_ship_a() -> Vec<u8> { ship(true, true) }
fn player_ship_b() -> Vec<u8> { ship(true, false) }
fn enemy_ship_a()  -> Vec<u8> { ship(false, true) }
fn enemy_ship_b()  -> Vec<u8> { ship(false, false) }

fn cannonball() -> Vec<u8> {
    let w: i32 = 8;
    let h: i32 = 8;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;
    let cy = h / 2;
    for dy in -2i32..=2 {
        for dx in -2i32..=2 {
            if dx * dx + dy * dy <= 5 {
                img.set(cx + dx, cy + dy, 50, 50, 60);
            }
        }
    }
    // highlight pixel
    img.set(cx - 1, cy - 1, 120, 120, 140);
    img.encode_png()
}

fn explosion() -> Vec<u8> {
    let w: i32 = 32;
    let h: i32 = 32;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;
    let cy = h / 2;
    let spokes: [f32; 12] = [
        0.0, 0.524, 1.047, 1.571, 2.094, 2.618,
        3.142, 3.665, 4.189, 4.712, 5.236, 5.760,
    ];
    for angle in spokes {
        for r in 0..(w / 2 - 1) {
            let x = cx + (r as f32 * angle.cos()) as i32;
            let y = cy + (r as f32 * angle.sin()) as i32;
            let t = r as f32 / (w as f32 / 2.0);
            let red   = (255.0 * (1.0 - t * 0.6)) as u8;
            let green = (160.0 * (1.0 - t)) as u8;
            img.set(x, y, red, green, 0);
        }
    }
    // bright core
    for dy in -2i32..=2 {
        for dx in -2i32..=2 {
            img.set(cx + dx, cy + dy, 255, 240, 180);
        }
    }
    img.encode_png()
}

fn port_bg() -> Vec<u8> {
    let w: i32 = 480;
    let h: i32 = 512;
    let mut img = Image::new(w as u32, h as u32);

    // Sky gradient (dark blue → lighter at horizon)
    for y in 0..200 {
        let t = y as f32 / 200.0;
        let r = (10.0 + t * 20.0) as u8;
        let g = (20.0 + t * 35.0) as u8;
        let b = (50.0 + t * 60.0) as u8;
        hband(&mut img, 0, w, y, y + 1, r, g, b);
    }

    // Horizon line
    hband(&mut img, 0, w, 199, 201, 60, 90, 120);

    // Water (dark teal, lower portion)
    for y in 200..h {
        let t = (y - 200) as f32 / (h - 200) as f32;
        let r = (10.0 + t * 5.0) as u8;
        let g = (60.0 - t * 20.0) as u8;
        let b = (90.0 - t * 30.0) as u8;
        hband(&mut img, 0, w, y, y + 1, r, g, b);
    }

    // Dock planks (horizontal wooden slats)
    let dock_y = 340;
    hband(&mut img, 60, 420, dock_y, dock_y + 80, 120, 80, 40);
    for y in (dock_y..dock_y + 80).step_by(8) {
        hband(&mut img, 60, 420, y, y + 1, 90, 60, 30);
    }
    // dock vertical planks
    for x in (60..420).step_by(16) {
        for y in dock_y..dock_y + 80 {
            if (x + y / 4) % 2 == 0 {
                img.set(x, y, 100, 65, 35);
            }
        }
    }

    // Buildings in background
    // Warehouse left
    hband(&mut img, 40, 140, 130, 200, 70, 65, 55);
    hband(&mut img, 40, 140, 125, 130, 90, 85, 75);  // roof edge
    // Warehouse right
    hband(&mut img, 300, 420, 140, 200, 65, 60, 50);
    hband(&mut img, 300, 420, 135, 140, 85, 80, 70);
    // Harbour master's house (centre)
    hband(&mut img, 180, 280, 150, 200, 80, 70, 55);
    // Roof (triangle via staircase)
    for i in 0..25 {
        let rx = 180 + i;
        let ry = 150 - i / 2;
        hband(&mut img, rx, w - rx, ry, ry + 1, 100, 60, 40);
    }

    // Windows on buildings
    for wx in [60, 90, 110].iter() {
        hband(&mut img, *wx, *wx + 12, 150, 165, 200, 190, 120);
    }
    for wx in [320, 355, 385].iter() {
        hband(&mut img, *wx, *wx + 12, 160, 175, 200, 190, 120);
    }
    hband(&mut img, 205, 220, 168, 183, 200, 190, 120);
    hband(&mut img, 240, 255, 168, 183, 200, 190, 120);

    // Dock post pilings
    for px in [80, 140, 220, 300, 380].iter() {
        hband(&mut img, *px - 3, *px + 3, dock_y, dock_y + 110, 80, 55, 30);
        // cap
        hband(&mut img, *px - 5, *px + 5, dock_y - 4, dock_y, 100, 70, 40);
    }

    // Wave shimmer on water surface
    for x in (0..w).step_by(24) {
        for i in 0..3 {
            let wx = x + i * 8;
            let wy = 210 + (wx * 3 % 15) as i32;
            hband(&mut img, wx, wx + 6, wy, wy + 1, 40, 100, 120);
        }
    }

    img.encode_png()
}

fn sea_wave() -> Vec<u8> {
    let w: i32 = 120;
    let h: i32 = 40;
    let mut img = Image::new(w as u32, h as u32);

    // Base water colour
    hband(&mut img, 0, w, 0, h, 10, 60, 90);

    // Wave crests using sine
    for x in 0..w {
        let t = x as f32 / w as f32;
        let wave1 = (t * PI * 2.0).sin();
        let wave2 = (t * PI * 4.0 + 1.0).sin() * 0.4;
        let crest_y = (h / 2) as f32 + (wave1 + wave2) * 5.0;

        // Crest highlight
        let cy = crest_y as i32;
        img.set(x, cy.clamp(0, h - 1), 80, 160, 200);
        img.set(x, (cy + 1).clamp(0, h - 1), 40, 110, 150);

        // Foam
        if ((x as f32 * 0.3 + t * PI).sin()) > 0.7 {
            img.set(x, cy.clamp(0, h - 1), 200, 230, 240);
        }
    }

    // Second wave layer (darker, further back)
    for x in 0..w {
        let t = x as f32 / w as f32;
        let wave = (t * PI * 2.0 + 1.0).sin();
        let cy = (h as f32 * 0.3 + wave * 3.0) as i32;
        img.set(x, cy.clamp(0, h - 1), 20, 80, 110);
        img.set(x, (cy + 1).clamp(0, h - 1), 15, 65, 95);
    }

    // Deep water bottom
    hband(&mut img, 0, w, h - 8, h, 5, 40, 65);

    img.encode_png()
}

fn crew_figure() -> Vec<u8> {
    let w: i32 = 12;
    let h: i32 = 20;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;

    // Head
    img.set(cx, 1, 220, 185, 140);
    img.set(cx - 1, 1, 220, 185, 140);
    img.set(cx, 2, 220, 185, 140);
    img.set(cx - 1, 2, 220, 185, 140);

    // Hat
    img.set(cx - 2, 0, 60, 50, 40);
    img.set(cx - 1, 0, 60, 50, 40);
    img.set(cx,     0, 60, 50, 40);
    img.set(cx + 1, 0, 60, 50, 40);

    // Body
    for y in 3..9 {
        img.set(cx - 1, y, 80, 90, 110);
        img.set(cx,     y, 80, 90, 110);
    }
    // Belt
    img.set(cx - 1, 7, 60, 50, 35);
    img.set(cx,     7, 60, 50, 35);

    // Arms
    img.set(cx - 3, 4, 220, 185, 140);
    img.set(cx - 2, 4, 220, 185, 140);
    img.set(cx + 1, 4, 220, 185, 140);
    img.set(cx + 2, 4, 220, 185, 140);
    // Sword in right hand
    img.set(cx + 2, 5, 200, 200, 220);
    img.set(cx + 2, 6, 200, 200, 220);
    img.set(cx + 2, 7, 200, 200, 220);

    // Legs
    for y in 9..17 {
        img.set(cx - 2, y, 70, 55, 40);
        img.set(cx + 1, y, 70, 55, 40);
    }

    // Boots
    for y in 17..20 {
        img.set(cx - 3, y, 40, 30, 20);
        img.set(cx - 2, y, 40, 30, 20);
        img.set(cx + 1, y, 40, 30, 20);
        img.set(cx + 2, y, 40, 30, 20);
    }

    img.encode_png()
}

// ── sounds ────────────────────────────────────────────────────────────────────

fn cannon_fire() -> Vec<u8> {
    // Low-frequency noise burst with sharp attack
    let mut buf = noise_burst(120.0, 1.0, 0.6);
    // Add a deep thump underneath
    let thump = sine_tone(80.0, 100.0, 0.5);
    for (i, s) in thump.iter().enumerate() {
        if i < buf.len() {
            let v = buf[i] as i32 + *s as i32;
            buf[i] = v.clamp(-32767, 32767) as i16;
        }
    }
    encode_pcm16_mono(&buf)
}

fn explosion_sfx() -> Vec<u8> {
    let buf = noise_burst(300.0, 0.9, 0.4);
    encode_pcm16_mono(&buf)
}

fn splash() -> Vec<u8> {
    // Lighter, higher miss sound
    let buf = noise_burst(80.0, 0.5, 0.8);
    encode_pcm16_mono(&buf)
}

fn hull_hit() -> Vec<u8> {
    // Low thud ~150 Hz square wave
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(60.0);
    let mut buf = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = env(i, n, 20, n / 3);
        let sq = if (150.0_f32 * t).fract() < 0.5 { 1.0_f32 } else { -1.0 };
        buf.push((e * 22000.0 * sq) as i16);
    }
    encode_pcm16_mono(&buf)
}

fn boarding_clash() -> Vec<u8> {
    // Metallic ping ~800 Hz, short
    encode_pcm16_mono(&sine_tone(800.0, 40.0, 0.7))
}

fn coin_jingle() -> Vec<u8> {
    // D5 → F#5 → A5 ascending arpeggio
    let mut buf = Vec::new();
    square_note(&mut buf, 587.33, 80.0, 0.6);  // D5
    square_note(&mut buf, 739.99, 80.0, 0.6);  // F#5
    square_note(&mut buf, 880.00, 80.0, 0.6);  // A5
    encode_pcm16_mono(&buf)
}

fn life_lost() -> Vec<u8> {
    // Descending glissando 440 → 110 Hz
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(400.0);
    let mut buf = Vec::with_capacity(n);
    let mut phase = 0.0_f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let freq = 440.0 * (1.0 - t * 0.75); // 440 → 110
        phase += 2.0 * PI * freq / sr;
        let e = 1.0 - t;
        buf.push((e * 22000.0 * phase.sin()) as i16);
    }
    encode_pcm16_mono(&buf)
}

fn sea_music() -> Vec<u8> {
    // E minor, 110 bpm – square-wave bass + melody over 8 bars
    // 110 bpm → quarter = 545ms, eighth = 273ms, half = 1091ms
    let q = 545.0_f32;
    let e = 273.0_f32;
    let h = 1091.0_f32;
    let d = 0.7_f32; // amplitude

    let mut buf = Vec::new();

    // Bass line (root movement)
    let bass: &[(f32, f32)] = &[
        (82.41, h), (73.42, h),   // E2, D2
        (82.41, h), (55.00, h),   // E2, A1
        (82.41, q), (82.41, q), (73.42, h), // E2 E2 D2
        (65.41, h), (55.00, h),   // C2, A1
    ];
    let total_bass: usize = bass.iter().map(|(_, ms)| ms_to_samples(*ms)).sum();
    let mut bass_buf = Vec::with_capacity(total_bass);
    for &(f, ms) in bass {
        square_note(&mut bass_buf, f, ms, d);
    }

    // Melody (upper voice)
    let melody: &[(f32, f32)] = &[
        (329.63, q), (293.66, e), (329.63, e), (349.23, q), (293.66, q), // E4 D4 E4 F4 D4
        (261.63, h), (220.00, h),                                           // C4 A3
        (329.63, q), (349.23, e), (392.00, e), (440.00, q), (392.00, q), // E4 F4 G4 A4 G4
        (349.23, h), (293.66, h),                                           // F4 D4
        (329.63, q), (293.66, q), (261.63, q), (220.00, q),               // E4 D4 C4 A3
        (246.94, h), (220.00, h),                                           // B3 A3
        (261.63, q), (293.66, q), (329.63, q), (293.66, q),               // C4 D4 E4 D4
        (220.00, h), (0.0, h),                                              // A3 rest
    ];
    let total_mel: usize = melody.iter().map(|(_, ms)| ms_to_samples(*ms)).sum();
    let mut mel_buf = Vec::with_capacity(total_mel);
    for &(f, ms) in melody {
        if f > 0.0 {
            square_note(&mut mel_buf, f, ms, d * 0.55);
        } else {
            rest(&mut mel_buf, ms);
        }
    }

    // Mix melody over bass (pad to same length)
    let len = bass_buf.len().max(mel_buf.len());
    bass_buf.resize(len, 0);
    mel_buf.resize(len, 0);
    buf.resize(len, 0i16);
    for i in 0..len {
        let v = bass_buf[i] as i32 + mel_buf[i] as i32;
        buf[i] = v.clamp(-32767, 32767) as i16;
    }

    encode_pcm16_mono(&buf)
}

fn combat_music() -> Vec<u8> {
    // Tense / dissonant, 140 bpm, 4 bars
    // 140 bpm → quarter = 429ms, eighth = 214ms
    let q = 429.0_f32;
    let e = 214.0_f32;
    let d = 0.7_f32;

    // Driving bass ostinato
    let bass: &[(f32, f32)] = &[
        (110.00, e), (110.00, e), (116.54, e), (110.00, e), // A2 A2 Bb2 A2
        (103.83, e), (110.00, e), (103.83, e), (98.00, e),  // Ab2 A2 Ab2 G2
        (110.00, e), (110.00, e), (123.47, e), (110.00, e), // A2 A2 B2 A2
        (116.54, q), (103.83, q),                            // Bb2 Ab2
    ];
    let mut bass_buf = Vec::new();
    for _ in 0..2 {
        for &(f, ms) in bass {
            square_note(&mut bass_buf, f, ms, d);
        }
    }

    // Melody fragments (short, agitated)
    let melody: &[(f32, f32)] = &[
        (440.00, e), (466.16, e), (440.00, q),  // A4 Bb4 A4
        (415.30, e), (392.00, e), (0.0, q),      // Ab4 G4 rest
        (440.00, e), (493.88, e), (466.16, q),  // A4 B4 Bb4
        (440.00, q), (0.0, q),                   // A4 rest
        (392.00, e), (415.30, e), (440.00, e), (466.16, e), // G4 Ab4 A4 Bb4
        (493.88, q), (440.00, q),                 // B4 A4
        (415.30, e), (440.00, e), (415.30, q),  // Ab4 A4 Ab4
        (392.00, q), (0.0, q),                   // G4 rest
    ];
    let mut mel_buf = Vec::new();
    for _ in 0..2 {
        for &(f, ms) in melody {
            if f > 0.0 {
                square_note(&mut mel_buf, f, ms, d * 0.5);
            } else {
                rest(&mut mel_buf, ms);
            }
        }
    }

    let len = bass_buf.len().max(mel_buf.len());
    bass_buf.resize(len, 0);
    mel_buf.resize(len, 0);
    let mut mixed = vec![0i16; len];
    for i in 0..len {
        let v = bass_buf[i] as i32 + mel_buf[i] as i32;
        mixed[i] = v.clamp(-32767, 32767) as i16;
    }

    encode_pcm16_mono(&mixed)
}

fn port_music() -> Vec<u8> {
    // G major, warm, 90 bpm, 6 bars
    // 90 bpm → quarter = 667ms, eighth = 333ms, half = 1333ms
    let q = 667.0_f32;
    let e = 333.0_f32;
    let h = 1333.0_f32;
    let d = 0.6_f32;

    // Bass (G major I-IV-V-I)
    let bass: &[(f32, f32)] = &[
        (98.00, h), (98.00, h),   // G2 G2
        (130.81, h), (130.81, h), // C3 C3
        (146.83, h), (146.83, h), // D3 D3
        (98.00, h), (98.00, h),   // G2 G2
        (110.00, h), (110.00, h), // A2 A2
        (146.83, h), (98.00, h),  // D3 G2
    ];
    let mut bass_buf = Vec::new();
    for &(f, ms) in bass {
        square_note(&mut bass_buf, f, ms, d);
    }

    // Melody
    let melody: &[(f32, f32)] = &[
        (392.00, q), (440.00, q), (392.00, e), (349.23, e), (392.00, q),  // G4 A4 G4 F4 G4
        (329.63, q), (293.66, h), (0.0, q),                                 // E4 D4 rest
        (392.00, e), (440.00, e), (493.88, q), (440.00, q), (392.00, q),  // G4 A4 B4 A4 G4
        (329.63, q), (261.63, h), (0.0, q),                                 // E4 C4 rest
        (440.00, q), (392.00, q), (349.23, q), (329.63, q),               // A4 G4 F4 E4
        (392.00, q), (440.00, q), (293.66, h),                              // G4 A4 D4
        (392.00, q), (329.63, q), (261.63, q), (293.66, q),               // G4 E4 C4 D4
        (392.00, h), (0.0, h),                                              // G4 rest
        (349.23, q), (392.00, q), (440.00, q), (392.00, q),               // F4 G4 A4 G4
        (329.63, h), (293.66, h),                                           // E4 D4
        (261.63, q), (293.66, q), (329.63, q), (392.00, e), (440.00, e),  // C4 D4 E4 G4 A4
        (392.00, h), (0.0, h),                                              // G4 rest
    ];
    let mut mel_buf = Vec::new();
    for &(f, ms) in melody {
        if f > 0.0 {
            square_note(&mut mel_buf, f, ms, d * 0.5);
        } else {
            rest(&mut mel_buf, ms);
        }
    }

    let len = bass_buf.len().max(mel_buf.len());
    bass_buf.resize(len, 0);
    mel_buf.resize(len, 0);
    let mut mixed = vec![0i16; len];
    for i in 0..len {
        let v = bass_buf[i] as i32 + mel_buf[i] as i32;
        mixed[i] = v.clamp(-32767, 32767) as i16;
    }

    encode_pcm16_mono(&mixed)
}

// ── generate ──────────────────────────────────────────────────────────────────

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/player_ship_a.png", player_ship_a()),
        ("images/player_ship_b.png", player_ship_b()),
        ("images/enemy_ship_a.png",  enemy_ship_a()),
        ("images/enemy_ship_b.png",  enemy_ship_b()),
        ("images/cannonball.png",    cannonball()),
        ("images/explosion.png",     explosion()),
        ("images/port_bg.png",       port_bg()),
        ("images/sea_wave.png",      sea_wave()),
        ("images/crew_figure.png",   crew_figure()),
        ("sounds/cannon_fire.wav",   cannon_fire()),
        ("sounds/explosion.wav",     explosion_sfx()),
        ("sounds/splash.wav",        splash()),
        ("sounds/hull_hit.wav",      hull_hit()),
        ("sounds/boarding_clash.wav",boarding_clash()),
        ("sounds/coin_jingle.wav",   coin_jingle()),
        ("sounds/life_lost.wav",     life_lost()),
        ("sounds/sea_music.wav",     sea_music()),
        ("sounds/combat_music.wav",  combat_music()),
        ("sounds/port_music.wav",    port_music()),
    ]
}
