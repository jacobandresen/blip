//! Drawing primitives — thin wrappers around macroquad's shape and texture API.
//! All coordinates are in virtual-canvas pixels. The canvas origin (0, 0) is the top-left corner.
//! Games should call these through the `Blip` methods rather than importing this module directly.

use macroquad::color::Color;
use macroquad::shapes::{
    draw_circle, draw_line as mq_draw_line, draw_rectangle, draw_rectangle_lines,
};
use macroquad::texture::{draw_texture_ex, DrawTextureParams, Texture2D};
use macroquad::math::vec2;
use macroquad::window::clear_background;

/// Fill the entire canvas with a solid colour.
#[inline]
pub fn clear(c: Color) {
    clear_background(c);
}

/// Draw a solid filled rectangle.
#[inline]
pub fn fill_rect(x: f32, y: f32, w: f32, h: f32, c: Color) {
    draw_rectangle(x, y, w, h, c);
}

/// Draw a 1-pixel outline rectangle (no fill).
#[inline]
pub fn draw_rect(x: f32, y: f32, w: f32, h: f32, c: Color) {
    draw_rectangle_lines(x, y, w, h, 1.0, c);
}

/// Draw a 1-pixel line between two points.
#[inline]
pub fn draw_line(x1: f32, y1: f32, x2: f32, y2: f32, c: Color) {
    mq_draw_line(x1, y1, x2, y2, 1.0, c);
}

/// Draw a line of arbitrary thickness between two points.
#[inline]
pub fn draw_line_ex(x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, c: Color) {
    mq_draw_line(x1, y1, x2, y2, thickness, c);
}

/// Draw a neon-style glowing line: a soft wide halo under a bright hairline core.
/// Use for vector silhouettes that should read as "lit up" against a black background.
pub fn draw_glow_line(x1: f32, y1: f32, x2: f32, y2: f32, c: Color) {
    let halo = Color { a: c.a * 0.16, ..c };
    mq_draw_line(x1, y1, x2, y2, 6.0, halo);
    let mid = Color { a: c.a * 0.35, ..c };
    mq_draw_line(x1, y1, x2, y2, 2.5, mid);
    mq_draw_line(x1, y1, x2, y2, 1.0, c);
}

/// Draw a solid filled circle. (`cx`, `cy`) is the centre.
#[inline]
pub fn fill_circle(cx: f32, cy: f32, r: f32, c: Color) {
    draw_circle(cx, cy, r, c);
}

/// Draw a soft glowing circle: a faint wide halo under a bright core. Good for thruster
/// flames, muzzle flashes, and other small neon-lit particles.
pub fn fill_glow_circle(cx: f32, cy: f32, r: f32, c: Color) {
    let halo = Color { a: c.a * 0.15, ..c };
    draw_circle(cx, cy, r * 2.6, halo);
    let mid = Color { a: c.a * 0.35, ..c };
    draw_circle(cx, cy, r * 1.5, mid);
    draw_circle(cx, cy, r, c);
}

/// Draw a texture stretched to fill the given rectangle.
pub fn draw_texture(tex: &Texture2D, x: f32, y: f32, w: f32, h: f32) {
    draw_texture_ex(
        tex,
        x,
        y,
        macroquad::color::WHITE,
        DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() },
    );
}

/// Draw a texture stretched to fill the given rectangle, multiplied by a tint colour.
/// Use this for hit flashes, transparency, or palette swaps.
pub fn draw_texture_tinted(tex: &Texture2D, x: f32, y: f32, w: f32, h: f32, tint: Color) {
    draw_texture_ex(
        tex,
        x,
        y,
        tint,
        DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() },
    );
}
