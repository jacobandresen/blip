//! `Blip` context: window config, frame loop, HUD, post-process overlay.

use macroquad::camera::{set_camera, set_default_camera, Camera2D};
use macroquad::color::Color;
use macroquad::math::Rect;
use macroquad::texture::Texture2D;
use macroquad::time::get_frame_time;
use macroquad::window::{clear_background, next_frame, screen_height, screen_width, Conf};

use crate::color::*;
use crate::draw;
use crate::font;

/// Build a `macroquad::Conf` matching the C `blip_init(title, w, h)` flow.
pub fn window_conf(title: &'static str, width: i32, height: i32) -> Conf {
    Conf {
        window_title: title.to_string(),
        window_width: width,
        window_height: height,
        window_resizable: true,
        ..Default::default()
    }
}

/// Per-game runtime state. Created once at startup, then `next_frame`'d
/// once per tick.
pub struct Blip {
    pub width: i32,
    pub height: i32,
    pub delta_time: f32,
}

impl Blip {
    pub fn new(width: i32, height: i32) -> Self {
        let b = Self { width, height, delta_time: 1.0 / 60.0 };
        b.apply_camera();
        b
    }

    /// Install a Camera2D that maps `(0,0)-(width,height)` virtual coordinates
    /// onto a centred letterboxed rectangle of the actual screen, so games
    /// keep using fixed pixel coordinates regardless of the true canvas size.
    fn apply_camera(&self) {
        let sw = screen_width();
        let sh = screen_height();
        let lw = self.width as f32;
        let lh = self.height as f32;
        if sw <= 0.0 || sh <= 0.0 { return; }
        let scale = (sw / lw).min(sh / lh);
        let vw = (lw * scale).round();
        let vh = (lh * scale).round();
        let vx = ((sw - vw) * 0.5).round();
        let vy = ((sh - vh) * 0.5).round();
        let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, lw, lh));
        // macroquad's display rect uses bottom-up Y; flip so (0,0) is top-left.
        cam.zoom.y = -cam.zoom.y;
        cam.viewport = Some((vx as i32, vy as i32, vw as i32, vh as i32));
        set_camera(&cam);    }

    /// End-of-frame: paint scanline overlay, present, refresh `delta_time`.
    /// `target_fps` is accepted for API parity but ignored — macroquad
    /// already paces to vsync.
    pub async fn next_frame(&mut self, _target_fps: i32) {
        self.draw_scanlines();
        next_frame().await;
        // New frame: paint the full screen black so the letterbox bars are
        // clean, then re-install the viewport camera (handles canvas
        // resizes between frames).
        set_default_camera();
        clear_background(macroquad::color::BLACK);
        self.apply_camera();
        let dt = get_frame_time();
        self.delta_time = if dt > 0.1 { 0.1 } else { dt };
    }

    fn draw_scanlines(&self) {
        // Match C: 1px line every 2px, 60/255 alpha black.
        let overlay = Color { r: 0.0, g: 0.0, b: 0.0, a: 60.0 / 255.0 };
        let w = self.width as f32;
        let mut y = 1;
        while y < self.height {
            draw::draw_line(0.0, y as f32, w - 1.0, y as f32, overlay);
            y += 2;
        }
    }

    // ----- drawing pass-throughs (kept as methods to mirror C `Blip *b`) -----

    #[inline]
    pub fn clear(&self, c: Color) { draw::clear(c); }
    #[inline]
    pub fn fill_rect(&self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        draw::fill_rect(x, y, w, h, c);
    }
    #[inline]
    pub fn draw_rect(&self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        draw::draw_rect(x, y, w, h, c);
    }
    #[inline]
    pub fn draw_line(&self, x1: f32, y1: f32, x2: f32, y2: f32, c: Color) {
        draw::draw_line(x1, y1, x2, y2, c);
    }
    #[inline]
    pub fn fill_circle(&self, cx: f32, cy: f32, r: f32, c: Color) {
        draw::fill_circle(cx, cy, r, c);
    }
    #[inline]
    pub fn draw_texture(&self, tex: &Texture2D, x: f32, y: f32, w: f32, h: f32) {
        draw::draw_texture(tex, x, y, w, h);
    }
    #[inline]
    pub fn draw_texture_tinted(
        &self, tex: &Texture2D, x: f32, y: f32, w: f32, h: f32, tint: Color,
    ) {
        draw::draw_texture_tinted(tex, x, y, w, h, tint);
    }

    // ----- font helpers -----
    #[inline]
    pub fn draw_char(&self, c: char, x: f32, y: f32, sz: f32, color: Color) {
        font::draw_char(c, x, y, sz, color);
    }
    #[inline]
    pub fn draw_text(&self, text: &str, x: f32, y: f32, sz: f32, color: Color) {
        font::draw_text(text, x, y, sz, color);
    }
    #[inline]
    pub fn draw_number(&self, n: i32, x: f32, y: f32, sz: f32, color: Color) {
        font::draw_number(n, x, y, sz, color);
    }
    #[inline]
    pub fn text_cx(&self, text: &str, sz: i32) -> i32 {
        font::text_cx(self.width, text, sz)
    }
    #[inline]
    pub fn draw_centered(&self, text: &str, y: f32, sz: f32, color: Color) {
        font::draw_centered(self.width, text, y, sz, color);
    }

    /// Standard 3-field HUD: SCORE / HI / LIVES.
    pub fn draw_hud(&self, score: i32, hi_score: i32, lives: i32) {
        let hud_h = 28.0;
        self.fill_rect(0.0, 0.0, self.width as f32, hud_h, BLIP_BLACK);
        self.draw_line(
            0.0, hud_h - 1.0, self.width as f32, hud_h - 1.0, BLIP_DARKGRAY,
        );
        self.draw_text("SCORE", 4.0, 5.0, 2.0, BLIP_YELLOW);
        self.draw_number(score, 68.0, 5.0, 2.0, BLIP_WHITE);
        self.draw_text("HI", (self.width / 2 - 22) as f32, 5.0, 2.0, BLIP_CYAN);
        self.draw_number(hi_score, (self.width / 2 + 8) as f32, 5.0, 2.0, BLIP_WHITE);
        self.draw_text("LIVES", (self.width - 90) as f32, 5.0, 2.0, BLIP_ORANGE);
        self.draw_number(lives, (self.width - 18) as f32, 5.0, 2.0, BLIP_WHITE);
    }
}
