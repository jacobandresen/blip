//! Resolve palette entries, Rust color names, and hexadecimal RGBA values.

use super::Value;
use blip::{
    BLIP_BLACK, BLIP_BLUE, BLIP_CYAN, BLIP_DARKGRAY, BLIP_GRAY, BLIP_GREEN, BLIP_MAGENTA,
    BLIP_ORANGE, BLIP_RED, BLIP_WHITE, BLIP_YELLOW, NEON_CYAN,
};
use macroquad::color::Color;

pub(super) fn value_color(value: &Value) -> Color {
    match value {
        Value::Color(color) => *color,
        Value::Num(number) => paper_color(*number as i32),
        Value::Str(text) => {
            let name = text.trim().to_ascii_uppercase();
            let hex = name.strip_prefix('#');
            if let Some(hex) = hex {
                if hex.len() == 6 || hex.len() == 8 {
                    if let Ok(bits) = u32::from_str_radix(hex, 16) {
                        let (red, green, blue, alpha) = if hex.len() == 8 {
                            (
                                (bits >> 24) & 255,
                                (bits >> 16) & 255,
                                (bits >> 8) & 255,
                                bits & 255,
                            )
                        } else {
                            (bits >> 16, (bits >> 8) & 255, bits & 255, 255)
                        };
                        return Color::new(
                            red as f32 / 255.0,
                            green as f32 / 255.0,
                            blue as f32 / 255.0,
                            alpha as f32 / 255.0,
                        );
                    }
                }
            }
            match name.as_str() {
                "BLIP_BLACK" => BLIP_BLACK,
                "BLIP_WHITE" => BLIP_WHITE,
                "BLIP_RED" => BLIP_RED,
                "BLIP_GREEN" => BLIP_GREEN,
                "BLIP_BLUE" => BLIP_BLUE,
                "BLIP_CYAN" => BLIP_CYAN,
                "BLIP_MAGENTA" => BLIP_MAGENTA,
                "BLIP_YELLOW" => BLIP_YELLOW,
                "BLIP_ORANGE" => BLIP_ORANGE,
                "BLIP_GRAY" => BLIP_GRAY,
                "BLIP_DARKGRAY" => BLIP_DARKGRAY,
                "NEON_CYAN" => NEON_CYAN,
                "NEON_MAGENTA" => blip::NEON_MAGENTA,
                "NEON_PINK" => blip::NEON_PINK,
                "NEON_PURPLE" => blip::NEON_PURPLE,
                "NEON_GREEN" => blip::NEON_GREEN,
                "NEON_ORANGE" => blip::NEON_ORANGE,
                "NEON_YELLOW" => blip::NEON_YELLOW,
                _ => BLIP_WHITE,
            }
        }
    }
}
pub(super) fn paper_color(index: i32) -> Color {
    [
        Color::new(0.06, 0.11, 0.64, 1.),
        Color::new(1., 0.96, 0.64, 1.),
        BLIP_CYAN,
        Color::new(1., 0.34, 0.59, 1.),
        Color::new(0.65, 0.92, 0.42, 1.),
        BLIP_ORANGE,
        BLIP_GRAY,
        Color::new(0.07, 0.07, 0.07, 1.),
    ][index.rem_euclid(8) as usize]
}
