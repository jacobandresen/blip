//! The pieces of the blip look drawn at runtime (`crate::look`): hearts, the score coin, the message box and soft
//! shadows. Every game's HUD, banners and end screens are made of these, so each one reads as a blip game.

use macroquad::color::Color;
use macroquad::shapes::{draw_ellipse, draw_rectangle};

use crate::color::{BLIP_CREAM, BLIP_GOLD, BLIP_HEART, BLIP_INK, BLIP_SHADOW};
use crate::font::text_width;
use crate::Blip;

/// A 7x6 heart, rows top to bottom (bit 6 is the left column).
const HEART: [u8; 6] = [0b0110110, 0b1111111, 0b1111111, 0b0111110, 0b0011100, 0b0001000];
/// A 7x7 coin.
const COIN: [u8; 7] = [0b0011100, 0b0111110, 0b1111111, 0b1111111, 0b1111111, 0b0111110, 0b0011100];

impl Blip {
    /// Rows of 7-bit pixel art at scale `s`, filled with `fill` and ringed one pixel round in ink.
    fn draw_bits(&self, rows: &[u8], x: f32, y: f32, s: f32, fill: Color) {
        let on = |c: i32, r: i32| (0..7).contains(&c) && (0..rows.len() as i32).contains(&r) && rows[r as usize] & (64 >> c) != 0;
        for r in -1..=rows.len() as i32 {
            for c in -1..=7 {
                let ring = !on(c, r) && [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dx, dy)| on(c + dx, r + dy));
                if on(c, r) || ring {
                    draw_rectangle(x + (c + 1) as f32 * s, y + (r + 1) as f32 * s, s, s, if ring { BLIP_INK } else { fill });
                }
            }
        }
    }

    /// A heart, 9x8 pixels at scale `s` with its ink ring; an empty one (a life lost) is dark inside.
    pub fn draw_heart(&self, x: f32, y: f32, s: f32, full: bool) {
        let fill = if full { BLIP_HEART } else { Color::new(0.30, 0.20, 0.30, 1.0) };
        self.draw_bits(&HEART, x, y, s, fill);
        if full {
            draw_rectangle(x + 2.0 * s, y + 2.0 * s, s, s, BLIP_CREAM);
        }
    }

    /// The score's coin, 9x9 pixels at scale `s`.
    pub fn draw_coin(&self, x: f32, y: f32, s: f32) {
        self.draw_bits(&COIN, x, y, s, BLIP_GOLD);
        draw_rectangle(x + 5.0 * s, y + 3.0 * s, s, 3.0 * s, Color::new(0.80, 0.52, 0.12, 1.0));
        draw_rectangle(x + 3.0 * s, y + 3.0 * s, s, s, BLIP_CREAM);
    }

    /// The blip message box, after the adventure games' dialogue boxes: ink with rounded corners and a cream rim
    /// two pixels in. `alpha` fades the whole box.
    pub fn draw_panel(&self, x: f32, y: f32, w: f32, h: f32, alpha: f32) {
        let (x, y, w, h) = (x.round(), y.round(), w.round(), h.round());
        let ink = Color { a: 0.94 * alpha, ..BLIP_INK };
        let rim = Color { a: alpha, ..BLIP_CREAM };
        draw_rectangle(x + 2.0, y, w - 4.0, h, ink);
        draw_rectangle(x, y + 2.0, 2.0, h - 4.0, ink);
        draw_rectangle(x + w - 2.0, y + 2.0, 2.0, h - 4.0, ink);
        draw_rectangle(x + 1.0, y + 1.0, 1.0, 1.0, ink);
        draw_rectangle(x + w - 2.0, y + 1.0, 1.0, 1.0, ink);
        draw_rectangle(x + 1.0, y + h - 2.0, 1.0, 1.0, ink);
        draw_rectangle(x + w - 2.0, y + h - 2.0, 1.0, 1.0, ink);
        // The rim, its corners cut by one pixel.
        draw_rectangle(x + 4.0, y + 2.0, w - 8.0, 2.0, rim);
        draw_rectangle(x + 4.0, y + h - 4.0, w - 8.0, 2.0, rim);
        draw_rectangle(x + 2.0, y + 4.0, 2.0, h - 8.0, rim);
        draw_rectangle(x + w - 4.0, y + 4.0, 2.0, h - 8.0, rim);
        draw_rectangle(x + 3.0, y + 3.0, 1.0, 1.0, rim);
        draw_rectangle(x + w - 4.0, y + 3.0, 1.0, 1.0, rim);
        draw_rectangle(x + 3.0, y + h - 4.0, 1.0, 1.0, rim);
        draw_rectangle(x + w - 4.0, y + h - 4.0, 1.0, 1.0, rim);
    }

    /// One line in a message box centred on (`cx`, `cy`): banners, stage names, a call to the player.
    pub fn draw_message(&self, text: &str, cx: f32, cy: f32, sz: f32, color: Color, alpha: f32) {
        let tw = text_width(text, sz) - sz;
        let (pad_x, pad_y) = (8.0 + sz * 3.0, 6.0 + sz * 2.0);
        let (w, h) = (tw + pad_x * 2.0, 7.0 * sz + pad_y * 2.0);
        self.draw_panel(cx - w / 2.0, cy - h / 2.0, w, h, alpha);
        self.draw_text(text, (cx - tw / 2.0).round(), (cy - 3.5 * sz).round(), sz, Color { a: color.a * alpha, ..color });
    }

    /// A soft round shadow on the ground under (`cx`, `cy`), `w` across: what makes a figure stand on the floor
    /// in the blip look. Flat, so it reads as pixel art.
    pub fn draw_shadow(&self, cx: f32, cy: f32, w: f32) {
        draw_ellipse(cx, cy, w * 0.5, (w * 0.2).max(2.0), 0.0, BLIP_SHADOW);
    }
}
