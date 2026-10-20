use crate::image::Image;
use crate::Asset;

fn glow_line(image: &mut Image, from: (f32, f32), to: (f32, f32), color: [u8; 3]) {
    for (width, alpha) in [(6.0, 41), (2.5, 89), (1.0, 255)] {
        image.line(from, to, width, [color[0], color[1], color[2], alpha]);
    }
}

fn title_rock(size: u32, radius: f32, jagged: [f32; 10]) -> Vec<u8> {
    let mut image = Image::new(size, size);
    let point = |i: usize| {
        let angle = i as f32 / 10.0 * std::f32::consts::TAU;
        (size as f32 / 2.0 + angle.cos() * radius * jagged[i % 10],
         size as f32 / 2.0 + angle.sin() * radius * jagged[i % 10])
    };
    for i in 0..10 { glow_line(&mut image, point(i), point(i + 1), [170, 60, 255]); }
    image.encode_png()
}

fn title_ship() -> Vec<u8> {
    let mut image = Image::new(48, 48);
    let hull = [(24.0, 10.0), (15.0, 34.0), (24.0, 28.0), (33.0, 34.0)];
    for i in 0..4 { glow_line(&mut image, hull[i], hull[(i + 1) % 4], [0, 255, 255]); }
    image.encode_png()
}

fn title_grid() -> Vec<u8> {
    let mut image = Image::new(680, 708);
    for i in 0..=7 {
        let t = i as f32 / 7.0;
        let y = 236.0 + 472.0 * t * t;
        let half = 340.0 * (0.15 + 0.85 * t);
        image.line((340.0 - half, y), (340.0 + half, y), 1.0, [170, 60, 255, ((0.1 + 0.35 * t) * 255.0) as u8]);
    }
    for verge in [-1.0, -0.5, 0.5, 1.0] {
        image.line((340.0 + 51.0 * verge, 236.0), (340.0 + 340.0 * verge, 708.0), 1.0, [170, 60, 255, 51]);
    }
    image.encode_png()
}

pub(crate) fn soft_glow() -> Vec<u8> {
    let mut image = Image::new(64, 64);
    for y in 0..64i32 {
        for x in 0..64i32 {
            let dx = (x as f32 + 0.5) / 32.0 - 1.0;
            let dy = (y as f32 + 0.5) / 32.0 - 1.0;
            let edge = (1.0 - (dx * dx + dy * dy).min(1.0)).powi(2);
            image.set_rgba(x, y, 255, 255, 255, (edge * 255.0) as u8);
        }
    }
    image.encode_png()
}

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/soft_glow.png", soft_glow()),
        ("images/title_grid.png", title_grid()),
        ("images/title_ship.png", title_ship()),
        ("images/title_rock_large.png", title_rock(96, 40.0, [0.78, 1.0, 0.88, 1.08, 0.75, 0.96, 1.12, 0.84, 1.0, 0.9])),
        ("images/title_rock_medium.png", title_rock(64, 22.0, [0.86, 1.12, 0.78, 0.96, 1.08, 0.82, 1.04, 0.9, 1.14, 0.8])),
    ]
}
