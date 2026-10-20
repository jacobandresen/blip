use crate::image::Image;
use crate::Asset;

fn impact_glint() -> Vec<u8> {
    let mut image = Image::new(32, 32);
    for y in 0..32i32 {
        for x in 0..32i32 {
            let dx = (x - 16) as f32;
            let dy = (y - 16) as f32;
            let d = (dx * dx + dy * dy).sqrt();
            if d < 13.0 && ((x + y) % 5 != 0 || d < 8.0) {
                let edge = (1.0 - d / 13.0).clamp(0.0, 1.0);
                image.set_rgba(
                    x,
                    y,
                    255,
                    (210.0 + 45.0 * edge) as u8,
                    90,
                    (edge * 210.0) as u8,
                );
            }
        }
    }
    image.encode_png()
}

pub fn generate() -> Vec<Asset> {
    vec![("images/impact_glint.png", impact_glint())]
}
