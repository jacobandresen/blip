use blip::image::Image;
use blip::Asset;

/// A filled polygon in the blip look: the body `fill`, lit toward the top left, ringed in ink.
fn shape(size: u32, pts: &[(f32, f32)], fill: [u8; 3]) -> Image {
    let mut image = Image::new(size, size);
    let inside = |x: f32, y: f32| {
        let mut odd = false;
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            if (a.1 > y) != (b.1 > y) && x < a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0) { odd = !odd; }
        }
        odd
    };
    let c = size as f32 / 2.0;
    for y in 0..size as i32 {
        for x in 0..size as i32 {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            if !inside(fx, fy) { continue; }
            let lit = (fx - c * 0.8).hypot(fy - c * 0.8) < c * 0.35;
            let k = if lit { 1.25 } else { 1.0 };
            image.set(x, y, (fill[0] as f32 * k).min(255.0) as u8, (fill[1] as f32 * k).min(255.0) as u8, (fill[2] as f32 * k).min(255.0) as u8);
        }
    }
    image.outline(blip::look::INK);
    image.outline(blip::look::INK);
    image
}

fn title_rock(size: u32, radius: f32, jagged: [f32; 10]) -> Vec<u8> {
    let pts: Vec<(f32, f32)> = (0..10).map(|i| {
        let angle = i as f32 / 10.0 * std::f32::consts::TAU;
        (size as f32 / 2.0 + angle.cos() * radius * jagged[i], size as f32 / 2.0 + angle.sin() * radius * jagged[i])
    }).collect();
    shape(size, &pts, blip::look::PLUM).encode_png()
}

fn title_ship() -> Vec<u8> {
    shape(48, &[(24.0, 10.0), (15.0, 34.0), (24.0, 28.0), (33.0, 34.0)], blip::look::CREAM).encode_png()
}

/// A round white blot thinning to nothing at the rim, with no edge to see.
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
        ("images/title_ship.png", title_ship()),
        ("images/title_rock_large.png", title_rock(96, 40.0, [0.78, 1.0, 0.88, 1.08, 0.75, 0.96, 1.12, 0.84, 1.0, 0.9])),
        ("images/title_rock_medium.png", title_rock(64, 22.0, [0.86, 1.12, 0.78, 0.96, 1.08, 0.82, 1.04, 0.9, 1.14, 0.8])),
    ]
}
