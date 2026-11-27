use blip::image::Image;
use blip::Asset;

const SPRITE: u32 = 32;

/// A cream egg: shell shading toward the rim, a highlight and a few speckles.
fn egg() -> Vec<u8> {
    let mut img = Image::new(SPRITE, SPRITE);
    let (w, h) = (SPRITE as i32, SPRITE as i32);
    let (cx, cy) = (w / 2, h / 2);
    let (rx, ry) = (w / 2 - 4, h / 2 - 3);
    for y in 0..h {
        for x in 0..w {
            let dx = (x - cx) as f32 / rx as f32;
            let mut dy = (y - cy) as f32 / ry as f32;
            if dy < 0.0 {
                dy *= 1.22;
            } // the narrow end
            let d = dx * dx + dy * dy;
            if d > 1.0 {
                continue;
            }
            if d > 0.80 {
                img.set(x, y, 168, 150, 110); // the rim
            } else {
                let lit = ((1.0 - d) * 26.0) as u8;
                img.set(x, y, 236 + lit / 4, 226 + lit / 4, 186);
            }
        }
    }
    img.set(cx - 3, cy - 5, 255, 250, 226);
    img.set(cx - 2, cy - 5, 255, 250, 226);
    img.set(cx - 3, cy - 4, 255, 250, 226);
    for (x, y) in [
        (cx - 5, cy + 2),
        (cx + 2, cy - 4),
        (cx + 5, cy + 3),
        (cx - 1, cy + 7),
        (cx + 4, cy - 7),
    ] {
        img.set(x, y, 198, 178, 130);
    }
    img.outline(blip::look::INK);
    img.encode_png()
}

fn hash(n: u32) -> u32 {
    let x = n.wrapping_mul(2654435761) ^ 0x5bd1_e995;
    (x ^ (x >> 13)).wrapping_mul(0x27d4_eb2d)
}

/// The pit: dim flagstones (the blip floor, as in Bouncer) with a grain of noise and lit pebbles, inside a plank rim
/// with a bevel and an ink edge where it meets the floor.
fn pit() -> Vec<u8> {
    let mut img = Image::new(480, 540);
    let floor = |x: i32, y: i32| (10..470).contains(&x) && (38..530).contains(&y);
    for y in 0..540 {
        for x in 0..480 {
            if y < 28 { img.set(x, y, 21, 17, 12); continue; }
            if floor(x, y) {
                let n = (hash((x as u32) << 16 | y as u32) % 5) as i32 - 2;
                // Darker toward the rim: the pit is a hollow.
                let edge = (x - 10).min(469 - x).min(y - 38).min(529 - y).min(24);
                let k = 24 - edge;
                // 40px flagstones: a lit top row, a dark joint, alternate stones a shade apart.
                let (tx, ty) = ((x - 10) % 40, (y - 38) % 40);
                let joint = tx == 0 || ty == 0;
                let tone = if joint { -8 } else if ty == 1 { 5 } else if ((x - 10) / 40 + (y - 38) / 40) % 2 == 0 { 2 } else { 0 };
                let v = |c: i32| (c + n + tone - k / 4).clamp(0, 255) as u8;
                img.set(x, y, v(30), v(32), v(52));
            } else {
                // Planks run along each side; a grain line every few pixels and a seam every 60.
                let along = if (10..470).contains(&x) { x } else { y };
                let across = if (10..470).contains(&x) { y } else { x };
                let grain = (hash(across as u32 * 7 + along as u32 / 23) % 9) as i32 - 4;
                let seam = along % 60 == 0;
                let (r, g, b) = if seam { (70, 48, 24) } else { (102 + grain, 73 + grain, 38 + grain / 2) };
                img.set(x, y, r.clamp(0, 255) as u8, g.clamp(0, 255) as u8, b.clamp(0, 255) as u8);
            }
        }
    }
    // Bevel: light on the rim's outer top and left, a dark lip where the rim drops to the floor.
    for x in 0..480 { img.set(x, 28, 150, 112, 64); img.set(x, 29, 132, 98, 56); }
    for y in 28..540 { img.set(0, y, 140, 104, 60); }
    let ink = blip::look::INK;
    for x in 9..471 { img.set(x, 37, ink[0], ink[1], ink[2]); img.set(x, 530, ink[0], ink[1], ink[2]); }
    for y in 37..531 { img.set(9, y, ink[0], ink[1], ink[2]); img.set(470, y, ink[0], ink[1], ink[2]); }
    for (d, v) in [(0, 14u8), (1, 18)] {
        for x in 10 + d..470 { img.set(x, 38 + d, v, v, v * 2); }
        for y in 38 + d..530 { img.set(10 + d, y, v, v, v * 2); }
    }
    for i in 0..140u32 {
        let h = hash(i + 7);
        let (x, y) = (14 + (h % 452) as i32, 42 + ((h >> 9) % 484) as i32);
        let big = h >> 20 & 3 == 0;
        let s = if big { 3 } else { 2 };
        for dy in 0..s {
            for dx in 0..s {
                let lit = dx == 0 && dy == 0;
                let (r, g, b) = if lit { (64, 66, 92) } else { (40, 42, 64) };
                img.set(x + dx, y + dy, r, g, b);
            }
        }
    }
    img.encode_png()
}
pub fn generate() -> Vec<Asset> {
    vec![("images/egg.png", egg()), ("images/pit.png", pit())]
}
