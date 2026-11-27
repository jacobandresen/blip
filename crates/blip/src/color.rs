//! The blip colour palette: the original eleven named constants, and the blip look's (`crate::look`), which every
//! game draws its outlines, HUD and boxes in. Define a game's own colours with `Color { r, g, b, a }` (0.0–1.0).

use macroquad::color::Color;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: 1.0 }
}

pub const BLIP_BLACK: Color = rgb(0, 0, 0); // background / clear colour
pub const BLIP_WHITE: Color = rgb(255, 255, 255); // bullets, ball, text
pub const BLIP_RED: Color = rgb(220, 50, 50); // danger, lives, player hats
pub const BLIP_GREEN: Color = rgb(50, 200, 50); // pickups, health, go signals
pub const BLIP_BLUE: Color = rgb(50, 100, 220); // water, shields, cold things
pub const BLIP_CYAN: Color = rgb(0, 200, 200); // HUD "HI" label
pub const BLIP_MAGENTA: Color = rgb(200, 50, 200); // portals, special items
pub const BLIP_YELLOW: Color = rgb(230, 220, 50); // score, coins, highlights
pub const BLIP_ORANGE: Color = rgb(230, 130, 20); // HUD "LIVES" label, fire
pub const BLIP_GRAY: Color = rgb(120, 120, 120); // inactive / dim text
pub const BLIP_DARKGRAY: Color = rgb(50, 50, 50); // separator lines, shadows

// ---- the blip look (`crate::look`): the palette every game draws its outlines, HUD and boxes in.
const fn pal(c: [u8; 3]) -> Color { rgb(c[0], c[1], c[2]) }
pub const BLIP_INK: Color = pal(crate::look::INK);
pub const BLIP_CREAM: Color = pal(crate::look::CREAM);
pub const BLIP_HEART: Color = pal(crate::look::HEART);
pub const BLIP_GOLD: Color = pal(crate::look::GOLD);
pub const BLIP_LEAF: Color = pal(crate::look::LEAF);
pub const BLIP_SEA: Color = pal(crate::look::SEA);
pub const BLIP_PLUM: Color = pal(crate::look::PLUM);
pub const BLIP_NIGHT: Color = pal(crate::look::NIGHT);
pub const BLIP_DUSK: Color = pal(crate::look::DUSK);
/// A soft round shadow under things that stand on the ground.
pub const BLIP_SHADOW: Color = Color { a: 0.32, ..BLIP_INK };

/// A colour from hue (0..1, wrapping round the rainbow), saturation and
/// value (0..1), and alpha.
pub fn hsv(h: f32, s: f32, v: f32, a: f32) -> Color {
    let h = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    Color { r: r + m, g: g + m, b: b + m, a }
}

/// `c` darkened (`k` < 1) or brightened (`k` > 1), alpha untouched: the shadow
/// side of a flat-coloured shape.
pub fn shade(c: Color, k: f32) -> Color {
    Color { r: c.r * k, g: c.g * k, b: c.b * k, a: c.a }
}

/// A colour `k` of the way from `a` to `b`, alpha included.
pub fn blend(a: Color, b: Color, k: f32) -> Color {
    Color { r: a.r + (b.r - a.r) * k, g: a.g + (b.g - a.g) * k, b: a.b + (b.b - a.b) * k, a: a.a + (b.a - a.a) * k }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_runs_from_one_colour_to_the_other_and_shade_keeps_alpha() {
        let (a, b) = (BLIP_RED, Color { a: 0.5, ..BLIP_BLUE });
        let near = |x: Color, y: Color| [x.r - y.r, x.g - y.g, x.b - y.b, x.a - y.a].iter().all(|d| d.abs() < 1e-6);
        assert!(near(blend(a, b, 0.0), a));
        assert!(near(blend(a, b, 1.0), b));
        let mid = blend(a, b, 0.5);
        assert!((mid.a - 0.75).abs() < 1e-6 && (mid.r - (a.r + b.r) / 2.0).abs() < 1e-6);
        let dark = shade(b, 0.5);
        assert_eq!(dark.a, b.a);
        assert!((dark.b - b.b * 0.5).abs() < 1e-6);
    }
}
