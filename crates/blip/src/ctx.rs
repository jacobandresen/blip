//! `Blip` context: window config, frame loop, HUD, post-process overlay.

use macroquad::camera::{set_camera, Camera2D};
use macroquad::color::{Color, WHITE};
use macroquad::math::{vec2, Rect};
use macroquad::shapes::draw_rectangle;
use macroquad::texture::{
    draw_texture_ex, render_target_ex, DrawTextureParams, FilterMode, RenderTarget,
    RenderTargetParams,
};
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

/// Lightweight LCG — no stdlib dependency, deterministic per-frame noise.
struct Lcg(u32);
impl Lcg {
    #[inline]
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 16) as f32 / 65_535.0
    }
}

/// Per-game runtime state. Created once at startup, then `next_frame`'d
/// once per tick.
pub struct Blip {
    pub width:      i32,
    pub height:     i32,
    pub delta_time: f32,
    rt:             RenderTarget,
    rng:            Lcg,
    // ---- glitch: horizontal tear ----
    tear_cd:  f32, // cooldown to next tear event (seconds)
    tear_t:   f32, // remaining duration of active tear (0 = inactive)
    tear_y:   f32, // split point as fraction of virtual height (0..1)
    tear_dx:  f32, // horizontal shift in virtual pixels
    // ---- glitch: vertical roll ----
    roll_cd:  f32,
    roll_t:   f32,
    roll_dy:  f32, // current vertical offset in virtual pixels
    roll_spd: f32, // pixels per second
    // ---- glitch: chromatic aberration ----
    chroma_cd: f32,
    chroma_t:  f32,
    chroma_dx: f32, // horizontal shift in virtual pixels
    // ---- interlaced field ----
    interlace_field: u8, // 0 or 1, flips every frame
}

impl Blip {
    pub fn new(width: i32, height: i32) -> Self {
        // sample_count=0 avoids the MSAA resolve path in miniquad, which calls
        // glCheckFramebufferStatus — a WebGL function missing from the JS bundle.
        let rt = render_target_ex(width as u32, height as u32, RenderTargetParams {
            sample_count: 0,
            ..Default::default()
        });
        rt.texture.set_filter(FilterMode::Nearest);

        let mut rng = Lcg(0xdead_beef);
        // Stagger initial cooldowns so effects don't all fire at once.
        let tear_cd   =  5.0 + rng.next() * 10.0;
        let roll_cd   = 15.0 + rng.next() * 20.0;
        let chroma_cd =  2.0 + rng.next() *  4.0;

        let b = Self {
            width, height, delta_time: 1.0 / 60.0,
            rt, rng,
            tear_cd,  tear_t: 0.0, tear_y: 0.5, tear_dx: 0.0,
            roll_cd,  roll_t: 0.0, roll_dy: 0.0, roll_spd: 0.0,
            chroma_cd, chroma_t: 0.0, chroma_dx: 0.0,
            interlace_field: 0,
        };
        b.apply_camera();
        b
    }

    /// Point the camera at the render target so subsequent game draws land there.
    fn apply_camera(&self) {
        // No zoom.y flip: macroquad's Camera2D already handles RT vs screen
        // inversion differences via its internal `invert_y` logic.
        let mut cam = Camera2D::from_display_rect(
            Rect::new(0.0, 0.0, self.width as f32, self.height as f32),
        );
        cam.render_target = Some(self.rt.clone());
        set_camera(&cam);
    }

    /// Letterboxed screen rect `(x, y, w, h)` for the current window size.
    fn viewport(&self) -> (f32, f32, f32, f32) {
        let sw = screen_width();
        let sh = screen_height();
        let lw = self.width  as f32;
        let lh = self.height as f32;
        let scale = (sw / lw).min(sh / lh);
        let vw = (lw * scale).round();
        let vh = (lh * scale).round();
        let vx = ((sw - vw) * 0.5).round();
        let vy = ((sh - vh) * 0.5).round();
        (vx, vy, vw, vh)
    }

    /// End-of-frame: blit the render target to screen with CRT post-process,
    /// then reset for the next game frame.
    pub async fn next_frame(&mut self, _target_fps: i32) {
        // Switch to a screen-space camera.  set_default_camera() is deliberately
        // NOT used here: it flushes the RT draws but leaves camera_matrix pointing
        // at the RT projection.  Blit vertices are in screen pixels, so using the
        // RT matrix clips everything when the window is larger than the game canvas.
        {
            let cam = Camera2D::from_display_rect(
                Rect::new(0.0, 0.0, screen_width(), screen_height()),
            );
            set_camera(&cam);
        }
        clear_background(macroquad::color::BLACK);
        self.draw_post_process();

        next_frame().await;

        // Prepare render target for the next game frame.
        self.apply_camera();
        clear_background(macroquad::color::BLACK);

        let raw = get_frame_time();
        self.delta_time = if raw > 0.1 { 0.1 } else { raw };
        self.update_glitch(self.delta_time);
        self.interlace_field ^= 1;
    }

    // ------------------------------------------------------------------ //
    // Glitch state machine                                                 //
    // ------------------------------------------------------------------ //

    fn update_glitch(&mut self, dt: f32) {
        let lh = self.height as f32;

        // Tear
        if self.tear_t > 0.0 {
            self.tear_t -= dt;
        } else {
            self.tear_cd -= dt;
            if self.tear_cd <= 0.0 {
                self.tear_t  = 0.08 + self.rng.next() * 0.20;
                self.tear_y  = 0.15 + self.rng.next() * 0.70;
                self.tear_dx = (self.rng.next() - 0.5) * 60.0;
                self.tear_cd =  5.0 + self.rng.next() * 15.0;
            }
        }

        // Roll
        if self.roll_t > 0.0 {
            self.roll_t  -= dt;
            self.roll_dy  = (self.roll_dy + self.roll_spd * dt) % lh;
        } else {
            self.roll_cd -= dt;
            if self.roll_cd <= 0.0 {
                self.roll_t   = 0.5 + self.rng.next() * 1.3;
                self.roll_spd = 180.0 + self.rng.next() * 320.0;
                self.roll_dy  = 0.0;
                self.roll_cd  = 15.0 + self.rng.next() * 25.0;
            }
        }

        // Chromatic aberration
        if self.chroma_t > 0.0 {
            self.chroma_t -= dt;
        } else {
            self.chroma_cd -= dt;
            if self.chroma_cd <= 0.0 {
                self.chroma_t  = 0.06 + self.rng.next() * 0.18;
                self.chroma_dx = 4.0  + self.rng.next() * 8.0;
                self.chroma_cd = 2.0  + self.rng.next() *  6.0;
            }
        }
    }

    // ------------------------------------------------------------------ //
    // Post-process rendering                                               //
    // ------------------------------------------------------------------ //

    fn draw_post_process(&mut self) {
        let (vx, vy, vw, vh) = self.viewport();
        if vw <= 0.0 || vh <= 0.0 { return; }

        let lw = self.width  as f32;
        let lh = self.height as f32;
        let scale = vw / lw;

        let roll_on   = self.roll_t  > 0.0;
        let tear_on   = self.tear_t  > 0.0 && !roll_on; // don't combine tear + roll
        let chroma_on = self.chroma_t > 0.0;

        // Cloning the texture handle is cheap (it's just a GPU ID).
        let tex = self.rt.texture.clone();

        // ---- chromatic aberration: coloured ghost layers under main image ----
        if chroma_on {
            let dx = self.chroma_dx * scale;
            draw_texture_ex(&tex, vx - dx, vy,
                Color::new(1.0, 0.0, 0.0, 0.35),
                DrawTextureParams { dest_size: Some(vec2(vw, vh)), ..Default::default() });
            draw_texture_ex(&tex, vx + dx, vy,
                Color::new(0.0, 0.4, 1.0, 0.35),
                DrawTextureParams { dest_size: Some(vec2(vw, vh)), ..Default::default() });
        }

        // ---- main image (with roll or tear applied) ----
        //
        // Source-rect convention: the screen camera has y=0 at screen top,
        // matching macroquad's game coordinate system.  Source Rect(0, a, lw, b)
        // maps directly to game rows starting at y=a with height b.
        if roll_on {
            // Upper screen strip: game rows [roll_dy, lh)
            let top_src_h = lh - self.roll_dy;
            let top_dst_h = vh * top_src_h / lh;
            draw_texture_ex(&tex, vx, vy, WHITE, DrawTextureParams {
                dest_size: Some(vec2(vw, top_dst_h)),
                source:    Some(Rect::new(0.0, self.roll_dy, lw, top_src_h)),
                ..Default::default()
            });
            // Lower screen strip: game rows [0, roll_dy) (wrapped)
            if self.roll_dy >= 1.0 {
                let bot_dst_h = vh - top_dst_h;
                draw_texture_ex(&tex, vx, vy + top_dst_h, WHITE, DrawTextureParams {
                    dest_size: Some(vec2(vw, bot_dst_h)),
                    source:    Some(Rect::new(0.0, 0.0, lw, self.roll_dy)),
                    ..Default::default()
                });
            }
        } else if tear_on {
            let split_lh = self.tear_y * lh;
            let split_vh = self.tear_y * vh;
            let tdx      = self.tear_dx * scale;

            // Top strip: game rows [0, split_lh)
            draw_texture_ex(&tex, vx, vy, WHITE, DrawTextureParams {
                dest_size: Some(vec2(vw, split_vh)),
                source:    Some(Rect::new(0.0, 0.0, lw, split_lh)),
                ..Default::default()
            });
            // Bottom strip: game rows [split_lh, lh), shifted horizontally
            draw_texture_ex(&tex, vx + tdx, vy + split_vh, WHITE, DrawTextureParams {
                dest_size: Some(vec2(vw, vh - split_vh)),
                source:    Some(Rect::new(0.0, split_lh, lw, lh - split_lh)),
                ..Default::default()
            });
            // Bright glitch line at the split point
            let gw = vw * (0.4 + self.rng.next() * 0.6);
            let gh = 1.0 + (self.rng.next() * 2.0).floor();
            let ga = 0.5 + self.rng.next() * 0.5;
            draw_rectangle(vx, vy + split_vh - gh * 0.5, gw, gh,
                Color::new(1.0, 1.0, 1.0, ga));
        } else {
            draw_texture_ex(&tex, vx, vy, WHITE, DrawTextureParams {
                dest_size: Some(vec2(vw, vh)),
                ..Default::default()
            });
        }

        // ---- interlaced CRT scanlines ----
        // Active field rows get a subtle CRT shadow; inactive field rows are
        // heavily dimmed to simulate the phosphor of the opposite field fading.
        // The active field flips every frame, producing the interlaced flicker.
        let active   = Color { r: 0.0, g: 0.0, b: 0.0, a: 60.0 / 255.0 };
        let inactive = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.75 };
        let bottom   = vy + vh;
        let f0 = self.interlace_field as f32;
        let f1 = 1.0 - f0;
        let mut sy = vy + f0;
        while sy < bottom { draw_rectangle(vx, sy, vw, 1.0, active);   sy += 2.0; }
        let mut sy = vy + f1;
        while sy < bottom { draw_rectangle(vx, sy, vw, 1.0, inactive); sy += 2.0; }

        // ---- background noise ----
        let pixel = scale.max(1.0);
        for _ in 0..48 {
            let nx = vx + self.rng.next() * vw;
            let ny = vy + self.rng.next() * vh;
            let a  = 0.02 + self.rng.next() * 0.10;
            let v  = self.rng.next();
            draw_rectangle(nx, ny, pixel, pixel, Color::new(v, v, v, a));
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
    pub fn draw_texture(&self, tex: &macroquad::texture::Texture2D, x: f32, y: f32, w: f32, h: f32) {
        draw::draw_texture(tex, x, y, w, h);
    }
    #[inline]
    pub fn draw_texture_tinted(
        &self, tex: &macroquad::texture::Texture2D, x: f32, y: f32, w: f32, h: f32, tint: Color,
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
