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
