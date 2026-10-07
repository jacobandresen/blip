//! 5×7 bitmap font for digits, case-insensitive A–Z, and `! : - . ( )`.
//! `sz` scales each pixel; at 2.0 each glyph is 10×14.

use macroquad::color::Color;

use crate::draw::fill_rect;

/// 43 glyphs × 7 rows. Bits 4-0 of each row = columns left→right.
/// Index 0-9 = digits, 10-35 = A-Z, 36 = ' ', 37 = '!', 38 = ':',
/// 39 = '-', 40 = '.', 41 = '(', 42 = ')'.
pub const FONT: [[u8; 7]; 43] = [
    /* 0 */ [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
    /* 1 */ [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
    /* 2 */ [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
    /* 3 */ [0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E],
    /* 4 */ [0x11, 0x11, 0x11, 0x1F, 0x01, 0x01, 0x01],
    /* 5 */ [0x1F, 0x10, 0x10, 0x1E, 0x01, 0x01, 0x1E],
    /* 6 */ [0x0E, 0x10, 0x10, 0x1E, 0x11, 0x11, 0x0E],
    /* 7 */ [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
    /* 8 */ [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
    /* 9 */ [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x11, 0x0E],
    /* A */ [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
    /* B */ [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
    /* C */ [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
    /* D */ [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C],
    /* E */ [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
    /* F */ [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
    /* G */ [0x0E, 0x10, 0x10, 0x17, 0x11, 0x11, 0x0E],
    /* H */ [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
    /* I */ [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
    /* J */ [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C],
    /* K */ [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
    /* L */ [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
    /* M */ [0x11, 0x1B, 0x15, 0x11, 0x11, 0x11, 0x11],
    /* N */ [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
    /* O */ [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
    /* P */ [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
    /* Q */ [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
    /* R */ [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
    /* S */ [0x0E, 0x11, 0x10, 0x0E, 0x01, 0x11, 0x0E],
    /* T */ [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
    /* U */ [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
    /* V */ [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
    /* W */ [0x11, 0x11, 0x11, 0x11, 0x15, 0x1B, 0x11],
    /* X */ [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
    /* Y */ [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
    /* Z */ [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
    /* ' '*/ [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    /* ! */ [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
    /* : */ [0x00, 0x04, 0x04, 0x00, 0x04, 0x04, 0x00],
    /* - */ [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
    /* . */ [0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x04],
    /* ( */ [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
    /* ) */ [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
];

const OUTLINE_OFFSETS: [(f32, f32); 8] = [
    (-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0),
    (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0),
];

fn char_to_glyph(c: char) -> Option<usize> {
    match c {
        '0'..='9' => Some((c as u8 - b'0') as usize),
        'A'..='Z' => Some(10 + (c as u8 - b'A') as usize),
        'a'..='z' => Some(10 + (c as u8 - b'a') as usize),
        ' ' => Some(36),
        '!' => Some(37),
        ':' => Some(38),
        '-' => Some(39),
        '.' => Some(40),
        '(' => Some(41),
        ')' => Some(42),
        _ => None,
    }
}

/// Draw a single character at pixel position (`x`, `y`). Unknown characters are silently skipped.
pub fn draw_char(c: char, x: f32, y: f32, sz: f32, color: Color) {
    let Some(idx) = char_to_glyph(c) else { return };
    for (row, bits) in FONT[idx].iter().enumerate() {
        for col in 0..5 {
            if bits & (1 << (4 - col)) != 0 {
                fill_rect(x + col as f32 * sz, y + row as f32 * sz, sz, sz, color);
            }
        }
    }
}

/// Draw a left-aligned string. Each character is 6×sz pixels wide (5 pixels + 1 gap).
pub fn draw_text(text: &str, x: f32, y: f32, sz: f32, color: Color) {
    let mut cx = x;
    for c in text.chars() {
        draw_char(c, cx, y, sz, color);
        cx += 6.0 * sz;
    }
}

/// Draw a string with an outline of `ink` all round it, one font pixel thick
/// at small sizes: lettering that has to be read over a picture.
pub fn draw_text_outlined(text: &str, x: f32, y: f32, sz: f32, color: Color, ink: Color) {
    let d = (sz * 0.5).max(1.0);
    for (dx, dy) in OUTLINE_OFFSETS {
        draw_text(text, x + dx * d, y + dy * d, sz, ink);
    }
    draw_text(text, x, y, sz, color);
}

/// Width of a string in pixels at size `sz` (a fixed 6-pixel cell).
pub fn text_width(text: &str, sz: f32) -> f32 {
    text.chars().count() as f32 * 6.0 * sz
}

/// Draw a glowing string with a one-pixel halo and near-white core.
/// About 10× the cost of [`draw_text`]; use for short labels.
pub fn draw_text_glow(text: &str, x: f32, y: f32, sz: f32, c: Color) {
    let halo = Color { a: c.a * 0.5, ..c };
    for (dx, dy) in OUTLINE_OFFSETS {
        draw_text(text, x + dx, y + dy, sz, halo);
    }
    // core: mostly white with a wash of the neon colour so the tint reads
    let core = Color { r: (c.r + 2.0) / 3.0, g: (c.g + 2.0) / 3.0, b: (c.b + 2.0) / 3.0, a: 1.0 };
    draw_text(text, x, y, sz, core);
}

/// Convenience wrapper to draw an integer without allocating a String at the call site.
pub fn draw_number(n: i32, x: f32, y: f32, sz: f32, color: Color) {
    draw_text(&n.to_string(), x, y, sz, color);
}

/// Return the x coordinate that would centre `text` within a canvas of `width` pixels.
pub fn text_cx(width: i32, text: &str, sz: i32) -> i32 {
    (width - text.chars().count() as i32 * 6 * sz) / 2
}

/// Draw a horizontally centred string within a canvas of `width` pixels.
pub fn draw_centered(width: i32, text: &str, y: f32, sz: f32, color: Color) {
    draw_text(text, text_cx(width, text, sz as i32) as f32, y, sz, color);
}

#[cfg(test)]
mod tests {
    use super::text_width;

    #[test]
    fn a_character_is_six_pixels_wide_at_size_one() {
        assert_eq!(text_width("", 2.0), 0.0);
        assert_eq!(text_width("FIGHT", 1.0), 30.0);
        assert_eq!(text_width("K.O.", 5.0), 120.0);
    }
}
