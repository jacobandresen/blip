//! The blip colour palette — eleven named constants, matching the original C macros.
//! Use these for a consistent retro look. You can always define your own colours
//! with `macroquad::color::Color { r, g, b, a }` (all values 0.0–1.0).

use macroquad::color::Color;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: 1.0 }
}

pub const BLIP_BLACK:    Color = rgb(  0,   0,   0); // background / clear colour
pub const BLIP_WHITE:    Color = rgb(255, 255, 255); // bullets, ball, text
pub const BLIP_RED:      Color = rgb(220,  50,  50); // danger, lives, player hats
pub const BLIP_GREEN:    Color = rgb( 50, 200,  50); // pickups, health, go signals
pub const BLIP_BLUE:     Color = rgb( 50, 100, 220); // water, shields, cold things
pub const BLIP_CYAN:     Color = rgb(  0, 200, 200); // HUD "HI" label
pub const BLIP_MAGENTA:  Color = rgb(200,  50, 200); // portals, special items
pub const BLIP_YELLOW:   Color = rgb(230, 220,  50); // score, coins, highlights
pub const BLIP_ORANGE:   Color = rgb(230, 130,  20); // HUD "LIVES" label, fire
pub const BLIP_GRAY:     Color = rgb(120, 120, 120); // inactive / dim text
pub const BLIP_DARKGRAY: Color = rgb( 50,  50,  50); // separator lines, shadows

// ---- neon accents — saturated, high-voltage versions for glow effects and modern accents.
// Use sparingly against BLIP_BLACK: a neon-outlined player ship, a synthwave horizon line,
// a title-screen highlight. Pair with `Blip::draw_glow_line` / `fill_glow_circle` for the halo.
pub const NEON_CYAN:    Color = rgb(  0, 255, 255); // player craft, glow trails
pub const NEON_MAGENTA: Color = rgb(255,   0, 220); // hostiles, danger accents
pub const NEON_PINK:    Color = rgb(255,  20, 147); // hit flashes, UI highlights
pub const NEON_PURPLE:  Color = rgb(170,  60, 255); // horizon grids, background accents
pub const NEON_GREEN:   Color = rgb(140, 255,  60); // pickups, go signals
pub const NEON_ORANGE:  Color = rgb(255, 120,   0); // thrust, fire, explosions
pub const NEON_YELLOW:  Color = rgb(255, 230,   0); // score pops, bullets

/// A colour from hue (0..1, wrapping round the rainbow), saturation and
/// value (0..1), and alpha.
pub fn hsv(h: f32, s: f32, v: f32, a: f32) -> Color {
    let h = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as i32 { 0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x), 3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x) };
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
    Color {
        r: a.r + (b.r - a.r) * k,
        g: a.g + (b.g - a.g) * k,
        b: a.b + (b.b - a.b) * k,
        a: a.a + (b.a - a.a) * k,
    }
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
