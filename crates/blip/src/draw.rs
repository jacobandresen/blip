//! Drawing primitives wrapping macroquad's shape/texture API.

use macroquad::color::Color;
use macroquad::shapes::{
    draw_circle, draw_line as mq_draw_line, draw_rectangle, draw_rectangle_lines,
};
use macroquad::texture::{draw_texture_ex, DrawTextureParams, Texture2D};
use macroquad::math::vec2;
use macroquad::window::clear_background;

#[inline]
pub fn clear(c: Color) {
    clear_background(c);
}

#[inline]
pub fn fill_rect(x: f32, y: f32, w: f32, h: f32, c: Color) {
    draw_rectangle(x, y, w, h, c);
}

#[inline]
pub fn draw_rect(x: f32, y: f32, w: f32, h: f32, c: Color) {
    draw_rectangle_lines(x, y, w, h, 1.0, c);
}

#[inline]
pub fn draw_line(x1: f32, y1: f32, x2: f32, y2: f32, c: Color) {
    mq_draw_line(x1, y1, x2, y2, 1.0, c);
}

#[inline]
pub fn fill_circle(cx: f32, cy: f32, r: f32, c: Color) {
    draw_circle(cx, cy, r, c);
}

pub fn draw_texture(tex: &Texture2D, x: f32, y: f32, w: f32, h: f32) {
    draw_texture_ex(
        tex,
        x,
        y,
        macroquad::color::WHITE,
        DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() },
    );
}

pub fn draw_texture_tinted(tex: &Texture2D, x: f32, y: f32, w: f32, h: f32, tint: Color) {
    draw_texture_ex(
        tex,
        x,
        y,
        tint,
        DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() },
    );
}
