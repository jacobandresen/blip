//! 5×7 bitmap font for digits, case-insensitive A–Z, and `! : - . ( )`, drawn through `Blip`.
//! `sz` scales each pixel; at 2.0 each glyph is 10×14.

use macroquad::color::Color;
use macroquad::shapes::draw_rectangle;

use crate::glyphs::{glyph_index, FONT};
use crate::Blip;

const OUTLINE_OFFSETS: [(f32, f32); 8] = [
    (-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0),
    (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0),
];

/// Width of a string in pixels at size `sz` (a fixed 6-pixel cell).
pub fn text_width(text: &str, sz: f32) -> f32 {
    text.chars().count() as f32 * 6.0 * sz
}

impl Blip {
    /// Draw a single character at pixel position (`x`, `y`). Unknown characters are silently skipped.
    fn draw_char(&self, c: char, x: f32, y: f32, sz: f32, color: Color) {
        let Some(idx) = glyph_index(c) else { return };
        // A rectangle per run of lit pixels in a row: the same pixels as one a pixel, in fewer draws.
        for (row, bits) in FONT[idx].iter().enumerate() {
            let mut col = 0;
            while col < 5 {
                if bits & (1 << (4 - col)) == 0 { col += 1; continue; }
                let start = col;
                while col < 5 && bits & (1 << (4 - col)) != 0 { col += 1; }
                draw_rectangle(x + start as f32 * sz, y + row as f32 * sz, (col - start) as f32 * sz, sz, color);
            }
        }
    }

    /// Draw a left-aligned string. Each character is 6×sz pixels wide (5 pixels + 1 gap).
    pub fn draw_text(&self, text: &str, x: f32, y: f32, sz: f32, color: Color) {
        let mut cx = x;
        for c in text.chars() {
            self.draw_char(c, cx, y, sz, color);
            cx += 6.0 * sz;
        }
    }

    /// Draw a string with an outline of `ink` all round it, one font pixel thick
    /// at small sizes: lettering that has to be read over a picture.
    pub fn draw_text_outlined(&self, text: &str, x: f32, y: f32, sz: f32, color: Color, ink: Color) {
        // The outline is never more solid than the text: fading text left a dark smudge of outline behind. Each pass
        // gets the alpha that its OUTLINE_OFFSETS.len() overlapping copies stack up to (solid stays solid).
        let target = ink.a.min(color.a);
        let a = if target >= 1.0 { 1.0 } else { 1.0 - (1.0 - target).powf(1.0 / OUTLINE_OFFSETS.len() as f32) };
        let ink = Color { a, ..ink };
        let d = (sz * 0.5).max(1.0);
        for (dx, dy) in OUTLINE_OFFSETS {
            self.draw_text(text, x + dx * d, y + dy * d, sz, ink);
        }
        self.draw_text(text, x, y, sz, color);
    }

    /// Draw an outlined string centred on `cx`.
    pub fn draw_centered_outlined(&self, text: &str, cx: f32, y: f32, sz: f32, color: Color, ink: Color) {
        self.draw_text_outlined(text, cx - text_width(text, sz) / 2.0, y, sz, color, ink);
    }

    /// A game's name on its title screen, the same lettering in every game: bold, `color` lit toward the top and
    /// shaded toward the foot like a toy's plastic, ringed in ink and standing on an ink shadow.
    pub fn draw_logo(&self, text: &str, cx: f32, y: f32, sz: f32, color: Color) {
        let ink = crate::color::BLIP_INK;
        // Bold: every glyph pixel is half a pixel wider, so the strokes stay strong inside the ring.
        let bold = (sz * 0.5).round();
        let x = (cx - (text_width(text, sz) - sz + bold) / 2.0).round();
        let y = y.round();
        let ring = (sz * 0.5).round().max(1.0);
        let lit = crate::color::blend(color, crate::color::BLIP_CREAM, 0.5);
        let dark = crate::color::shade(color, 0.7);
        let glyphs = |pass: &dyn Fn(f32, f32, usize)| {
            let mut gx = x;
            for c in text.chars() {
                if let Some(idx) = glyph_index(c) {
                    for (row, bits) in FONT[idx].iter().enumerate() {
                        for col in 0..5 {
                            if bits & (1 << (4 - col)) != 0 { pass(gx + col as f32 * sz, y + row as f32 * sz, row); }
                        }
                    }
                }
                gx += 6.0 * sz;
            }
        };
        let (w, h) = (sz + bold, sz);
        glyphs(&|px, py, _| draw_rectangle(px - ring, py - ring + sz * 0.75, w + ring * 2.0, h + ring * 2.0, ink));
        glyphs(&|px, py, _| draw_rectangle(px - ring, py - ring, w + ring * 2.0, h + ring * 2.0, ink));
        glyphs(&|px, py, row| {
            let c = match row { 0 | 1 => lit, 5 | 6 => dark, _ => color };
            draw_rectangle(px, py, w, h, c);
        });
    }

    /// Draw an integer as text.
    pub fn draw_number(&self, n: i32, x: f32, y: f32, sz: f32, color: Color) {
        self.draw_text(&n.to_string(), x, y, sz, color);
    }

    /// The x coordinate that centres `text` on this canvas.
    pub fn text_cx(&self, text: &str, sz: i32) -> i32 {
        (self.width - text.chars().count() as i32 * 6 * sz) / 2
    }

    /// Draw a string horizontally centred on this canvas.
    pub fn draw_centered(&self, text: &str, y: f32, sz: f32, color: Color) {
        self.draw_text(text, self.text_cx(text, sz as i32) as f32, y, sz, color);
    }
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
