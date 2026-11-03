//! WARLOCK's still artwork. The game draws the wizard, monsters and spell
//! effects at runtime (they animate); here it makes the two still pieces it
//! embeds: the rune-stone glyph used in the HUD and the title emblem.

use blip::image::Image;
use blip::Asset;

const N: u32 = 24;

/// A cursed rune stone: a violet shard in a pale ring.
fn rune() -> Vec<u8> {
    let mut img = Image::new(N, N);
    let (w, h) = (N as i32, N as i32);
    let cx = (w / 2) as f32;
    let cy = (h / 2) as f32;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            // outer ring, soft pale
            if d >= w as f32 * 0.30 && d <= w as f32 * 0.42 {
                img.set(x, y, 200, 200, 230);
            }
            // inner shard: a tilted diamond of violet
            let a = dx.abs() + dy.abs();
            if a < w as f32 * 0.20 {
                img.set(x, y, 150, 60, 190);
            }
            if a < w as f32 * 0.13 {
                img.set(x, y, 220, 130, 250);
            }
        }
    }
    img.encode_png()
}

/// A crooked tower emblem for the title: a crenellated tower under a ring.
fn emblem() -> Vec<u8> {
    let (w, h) = (48, 48);
    let mut img = Image::new(w, h);
    let cx = 24;
    // moon ring
    for y in 0..48i32 {
        for x in 0..48i32 {
            let dx = (x - cx) as f32;
            let dy = (y - 30) as f32;
            let d = (dx * dx + dy * dy).sqrt();
            if d >= 14.0 && d <= 16.0 {
                img.set(x, y, 190, 200, 255);
            }
        }
    }
    // tower body (deep violet)
    for y in 24..48i32 {
        for x in 12..36i32 {
            img.set(x, y, 46, 28, 71);
        }
    }
    // crenellations
    for x in 12..36i32 {
        if (x - 12) % 6 < 4 {
            img.set(x, 22, 46, 28, 71);
            img.set(x, 23, 46, 28, 71);
        }
    }
    // a lit doorway
    for y in 38..48i32 {
        for x in 20..28i32 {
            img.set(x, y, 255, 180, 60);
        }
    }
    img.encode_png()
}

pub fn generate() -> Vec<Asset> {
    vec![("images/rune.png", rune()), ("images/emblem.png", emblem())]
}