use crate::image::Image;
use crate::Asset;

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
    img.encode_png()
}

fn pit() -> Vec<u8> {
    let mut img = Image::new(480, 540);
    for y in 0..540 {
        for x in 0..480 {
            let [r, g, b] = if y >= 28 && (!(10..470).contains(&x) || y < 38 || y >= 530) {
                [102, 73, 38]
            } else {
                [21, 17, 12]
            };
            img.set(x, y, r, g, b);
        }
    }
    for i in 0..110u32 {
        let h = i.wrapping_mul(2654435761).wrapping_add(12345);
        let (x, y) = (10 + (h % 456) as i32, 38 + ((h >> 9) % 488) as i32);
        for dy in 0..2 {
            for dx in 0..2 {
                img.set(x + dx, y + dy, 36, 30, 21);
            }
        }
    }
    img.encode_png()
}

pub fn generate() -> Vec<Asset> {
    vec![("images/egg.png", egg()), ("images/pit.png", pit())]
}
