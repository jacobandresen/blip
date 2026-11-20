use blip::image::Image;
use blip::Asset;

const SPRITE: u32 = 24;

/// A shaded apple: red body lit from the top left, a dimple, a brown stem and a green leaf.
fn food() -> Vec<u8> {
    let mut img = Image::new(SPRITE, SPRITE);
    let (cx, cy) = (11.5f32, 13.5f32);
    for y in 0..SPRITE as i32 {
        for x in 0..SPRITE as i32 {
            // Two overlapping lobes make the apple's shoulders.
            let (fx, fy) = (x as f32, y as f32);
            let lobe = |ox: f32| ((fx - (cx + ox)).powi(2) + ((fy - cy) * 1.08).powi(2)).sqrt();
            let d = lobe(-2.2).min(lobe(2.2));
            if d > 8.6 { continue; }
            if fy < 7.0 && (fx - cx).abs() < 1.5 { continue; } // the dimple
            let light = (((fx - 8.0).powi(2) + (fy - 9.0).powi(2)).sqrt() / 13.0).min(1.0);
            let k = 1.0 - light;
            let edge = d > 7.4;
            let (r, g, b) = if edge { (120.0, 20.0, 30.0) } else { (170.0 + 85.0 * k, 30.0 + 40.0 * k, 40.0 + 30.0 * k) };
            img.set(x, y, r as u8, g as u8, b as u8);
        }
    }
    for (x, y) in [(9, 9), (8, 10), (9, 10), (8, 11)] { img.set(x, y, 255, 220, 220); }
    for (x, y) in [(12, 2), (12, 3), (12, 4), (11, 5), (11, 6)] { img.set(x, y, 100, 60, 25); }
    for (x, y) in [(13, 3), (14, 3), (15, 2), (14, 4), (15, 3), (16, 2), (17, 2), (16, 3)] { img.set(x, y, 60, 180, 60); }
    img.encode_png()
}

pub fn generate() -> Vec<Asset> {
    vec![("images/food.png", food())]
}
