use crate::image::Image;
use crate::Asset;

const ALIEN_W: i32 = 36;
const ALIEN_H: i32 = 28;
const UFO_W: i32 = 36;
const UFO_H: i32 = 20;
const UFO_N_LIGHTS: usize = 8;

// Dimensions match the Rust game's sprite constants.
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

/// An alien glyph with two frames for the march animation.
fn alien(kind: usize, frame: usize) -> Vec<u8> {
    let w: i32 = ALIEN_W;
    let h: i32 = ALIEN_H;
    let mut img = Image::new(w as u32, h as u32);
    let (r, g, b) = match kind {
        0 => (255u8, 80, 255),
        1 => (0, 230, 230),
        _ => (110, 255, 110),
    };
    // [frame A, frame B] — rows 0..4 (head/eyes/mandibles) stay put, rows
    // 5..7 (legs / claws / tentacles) swap between frames.
    let patterns: [[u16; 8]; 2] = match kind {
        0 => [
            // Squid: antenna nubs, jagged toothy mandible, legs together.
            [
                row("001000100"),
                row("001111100"),
                row("011111110"),
                row("111111111"),
                row("110111011"),
                row("011111110"),
                row("101010101"),
                row("000101000"),
            ],
            // ...antenna twitch inward, legs thrown wide apart.
            [
                row("000101000"),
                row("001111100"),
                row("011111110"),
                row("111111111"),
                row("110111011"),
                row("011111110"),
                row("100000001"),
                row("010000010"),
            ],
        ],
        1 => [
            // Crab: flat head, ridged shell, pincers tucked in close.
            [
                row("000000000"),
                row("000111000"),
                row("001111100"),
                row("011111110"),
                row("111111111"),
                row("110111011"),
                row("010000010"),
                row("000101000"),
            ],
            // ...pincers thrown wide open, gnarly.
            [
                row("000000000"),
                row("000111000"),
                row("001111100"),
                row("011111110"),
                row("111111111"),
                row("110111011"),
                row("100000001"),
                row("001000100"),
            ],
        ],
        _ => [
            // Octopus: round head, tentacle skirt waving one way...
            [
                row("000000000"),
                row("000111000"),
                row("001111100"),
                row("011111110"),
                row("111111111"),
                row("011111110"),
                row("101010101"),
                row("010101010"),
            ],
            // ...and the other, for a wavy crawl.
            [
                row("000000000"),
                row("000111000"),
                row("001111100"),
                row("011111110"),
                row("111111111"),
                row("011111110"),
                row("010101010"),
                row("101010101"),
            ],
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
    // Eyes on the wide head row: a bright square in a dark socket with a dark
    // pupil, darting with the frame.
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

/// Saucer frame with its light ring shifted for animation.
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
    img.set(
        cx as i32 - 1,
        (dome_cy - dome_r * 0.4) as i32,
        220,
        250,
        255,
    );

    // Rim lights, evenly spaced and alternating colour, offset by `frame`
    // so the ring appears to rotate as frames advance.
    for i in 0..UFO_N_LIGHTS {
        let idx = i + frame;
        let a = (idx as f32 / UFO_N_LIGHTS as f32) * std::f32::consts::PI * 2.0;
        let lx = (cx + a.cos() * disc_rx * 0.88).round() as i32;
        let ly = (cy + a.sin() * disc_ry * 0.88).round() as i32;
        let (r, g, b) = if idx.is_multiple_of(2) {
            (255u8, 230, 60)
        } else {
            (255, 255, 255)
        };
        img.set(lx, ly, r, g, b);
        img.set(lx, ly - 1, r, g, b);
    }
    img.encode_png()
}

fn bullet() -> Vec<u8> {
    let w: i32 = 4;
    let h: i32 = 12;
    let mut img = Image::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w { img.set(x, y, 255, 255, 255); }
    }
    img.encode_png()
}

fn explosion() -> Vec<u8> {
    let w: i32 = 32;
    let h: i32 = 32;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;
    let cy = h / 2;
    // Twelve rays, one every 30 degrees.
    for angle in (0..12).map(|k| k as f32 * std::f32::consts::PI / 6.0) {
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

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/player_ship.png", player_ship()),
        ("images/alien_squid_a.png", alien(0, 0)),
        ("images/alien_squid_b.png", alien(0, 1)),
        ("images/alien_crab_a.png", alien(1, 0)),
        ("images/alien_crab_b.png", alien(1, 1)),
        ("images/alien_octopus_a.png", alien(2, 0)),
        ("images/alien_octopus_b.png", alien(2, 1)),
        ("images/bullet.png", bullet()),
        ("images/explosion.png", explosion()),
        ("images/shield_block.png", shield_block()),
        ("images/ufo_saucer_0.png", ufo_saucer(0)),
        ("images/ufo_saucer_1.png", ufo_saucer(1)),
        ("images/ufo_saucer_2.png", ufo_saucer(2)),
        ("images/ufo_saucer_3.png", ufo_saucer(3)),
        ("images/ufo_saucer_4.png", ufo_saucer(4)),
        ("images/ufo_saucer_5.png", ufo_saucer(5)),
        ("images/ufo_saucer_6.png", ufo_saucer(6)),
        ("images/ufo_saucer_7.png", ufo_saucer(7)),
    ]
}
