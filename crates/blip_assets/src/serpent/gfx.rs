use crate::image::Image;
use crate::Asset;

const SPRITE: u32 = 24;

fn food() -> Vec<u8> {
    let mut img = Image::new(SPRITE, SPRITE);
    let (w, h) = (SPRITE as i32, SPRITE as i32);
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

pub fn generate() -> Vec<Asset> {
    vec![("images/food.png", food())]
}
