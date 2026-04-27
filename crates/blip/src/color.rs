//! Color constants matching the C macros.

use macroquad::color::Color;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: 1.0 }
}

pub const BLIP_BLACK:    Color = rgb(  0,   0,   0);
pub const BLIP_WHITE:    Color = rgb(255, 255, 255);
pub const BLIP_RED:      Color = rgb(220,  50,  50);
pub const BLIP_GREEN:    Color = rgb( 50, 200,  50);
pub const BLIP_BLUE:     Color = rgb( 50, 100, 220);
pub const BLIP_CYAN:     Color = rgb(  0, 200, 200);
pub const BLIP_MAGENTA:  Color = rgb(200,  50, 200);
pub const BLIP_YELLOW:   Color = rgb(230, 220,  50);
pub const BLIP_ORANGE:   Color = rgb(230, 130,  20);
pub const BLIP_GRAY:     Color = rgb(120, 120, 120);
pub const BLIP_DARKGRAY: Color = rgb( 50,  50,  50);
