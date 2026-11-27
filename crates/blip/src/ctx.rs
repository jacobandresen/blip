//! Central game context: fixed-size canvas, frame loop and CRT post-processing.
//! Games draw through `Blip` and read `delta_time` each frame.

use macroquad::camera::{set_camera, Camera2D};
use macroquad::color::{Color, WHITE};
use macroquad::material::{
    gl_use_default_material, gl_use_material, Material,
};
use macroquad::math::{vec2, Rect};
use macroquad::shapes::draw_rectangle;
use macroquad::texture::{
    draw_texture_ex, render_target_ex, DrawTextureParams, FilterMode,
    RenderTarget, RenderTargetParams,
};
use macroquad::time::get_frame_time;
use macroquad::window::{clear_background, get_internal_gl, next_frame, screen_height, screen_width, Conf};

use crate::crt;

/// Create the macroquad window configuration. Pass the result to `#[macroquad::main(conf)]`.
/// `width` and `height` set the *virtual* canvas size — the window is resizable and will
/// letterbox the canvas to fit.
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

/// The main blip runtime. Create one at the start of `main` with `Blip::new(w, h)`,
/// then call `blip.next_frame().await` at the end of every game loop iteration.
/// Read `blip.delta_time` each frame to get the elapsed seconds since the last tick.
pub struct Blip {
    pub width:      i32,
    pub height:     i32,
    pub delta_time: f32,
    rt:             RenderTarget,
    rng:            Lcg,
    crt_look:       bool, // a CRT's picture, or a clean one (crate::web::crt)
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
    rt_blended: bool, // the frame is being shown shrunk, so sampled linear
    interlace_field: u8, // 0 or 1, flips every frame
    // How bright the fading field's rows stay (the other field's are 0.76).
    field_dim: f32,
    // ---- curved-glass shader pass ----
    crt:         Option<Material>, // None if the shader failed to compile
    // How much of the phosphor bloom is added back: 1.0 suits lit lines on
    // black; a game with bright filled scenes turns it down or it smears.
    bloom:       f32,
    // Scanline dimming shader (crt::SCANLINE_FRAGMENT); None falls back to a
    // rectangle per row, like `crt` above.
    scanline:    Option<Material>,
    seal:        Option<Material>, // crt::SEAL_FRAGMENT: sets the frame's alpha to 1
    screen_rt:   Option<RenderTarget>, // offscreen composite target, window-sized
    screen_rt_w: i32,
    screen_rt_h: i32,
    // ---- adaptive render quality ----
    // Sustained slow frames step quality down: 0 full, 1 no CRT curve, 2 no scanlines or noise.
    fx_level:      u8,
    fx_settle:     u8,  // frames to ignore after startup / a level change
    fx_slow_accum: f32, // seconds run slow at the current level
    fx_frame_ema:  f32, // smoothed frame time, seconds
    // ---- the HUD's score multiple: the last one drawn, and seconds since it last rose or broke ----
    pub(crate) multiple: std::cell::Cell<(i32, i32, f32)>, // (shown, before the change, seconds since)
    // ---- screenshot capture (native only) ----
    pub screenshot_mode:   bool,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    screenshot_frame:      u32,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    screenshot_frame_target: u32,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    screenshot_path:       Option<String>,
}

/// Free a render target once. Dropped, macroquad 0.4 frees its texture twice:
/// the render pass deletes it at once, then the texture's handle deletes the
/// same id at frame end, after a new target may have taken it (a resized
/// canvas went black under the curved glass).
fn retire(rt: RenderTarget) {
    let pass = rt.render_pass.raw_miniquad_id();
    std::mem::forget(rt);
    let mut gl = unsafe { get_internal_gl() };
    gl.flush();
    gl.quad_context.delete_render_pass(pass);
}

impl Blip {
    pub fn new(width: i32, height: i32) -> Self {
        crate::bot::canvas(width as f32, height as f32);
        // sample_count=0 avoids the MSAA resolve path in miniquad, which calls
        // glCheckFramebufferStatus — a WebGL function missing from the JS bundle.
        let rt = render_target_ex(width as u32, height as u32, RenderTargetParams {
            sample_count: 0,
            ..Default::default()
        });
        rt.texture.set_filter(FilterMode::Nearest);

        // Curved glass, the alpha seal and the scanlines; a shader that fails to compile (old WebGL, driver quirks)
        // is None, and the frame is composited straight to the screen without it.
        let (crt, seal, scanline) = (crt::glass(), crt::seal(), crt::scanlines());

        // Unseeded, every game deals the same food, rocks and drops each time.
        // A bot run draws the same numbers every time, so its result repeats.
        let seed = crate::bot::seed().unwrap_or((macroquad::miniquad::date::now() * 1000.0) as u64);
        macroquad::rand::srand(seed);
        let mut rng = Lcg(0xdead_beef);
        // Stagger initial cooldowns so effects don't all fire at once.
        let tear_cd   = 90.0 + rng.next() * 90.0;
        let roll_cd   = 240.0 + rng.next() * 240.0;
        let chroma_cd = 60.0 + rng.next() * 60.0;

        #[cfg(not(target_arch = "wasm32"))]
        let (screenshot_mode, screenshot_path, screenshot_frame_target) = {
            let path   = std::env::var("BLIP_SCREENSHOT_OUT").ok();
            let mode   = path.is_some();
            let target = std::env::var("BLIP_SCREENSHOT_FRAME")
                .ok().and_then(|s| s.parse().ok()).unwrap_or(25u32);
            (mode, path, target)
        };
        #[cfg(target_arch = "wasm32")]
        let (screenshot_mode, screenshot_path, screenshot_frame_target): (bool, Option<String>, u32) =
            (false, None, 25);

        let b = Self {
            width, height, delta_time: 1.0 / 60.0,
            rt, rng, crt_look: crate::web::crt(),
            tear_cd,  tear_t: 0.0, tear_y: 0.5, tear_dx: 0.0,
            roll_cd,  roll_t: 0.0, roll_dy: 0.0, roll_spd: 0.0,
            chroma_cd, chroma_t: 0.0, chroma_dx: 0.0,
            rt_blended: false,
            interlace_field: 0,
            field_dim: 0.25,
            crt,
            seal,
            bloom: 1.0,
            scanline,
            screen_rt: None,
            screen_rt_w: 0,
            screen_rt_h: 0,
            fx_level: 0,
            fx_settle: 45, // ~0.75s of warmup (asset decode, shader compile, JIT)
            multiple: std::cell::Cell::new((1, 1, f32::INFINITY)),
            fx_slow_accum: 0.0,
            fx_frame_ema: 1.0 / 60.0,
            screenshot_mode,
            screenshot_frame: 0,
            screenshot_frame_target,
            screenshot_path,
        };
        b.apply_camera();
        crate::web::picture(width, height);
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
        if crate::bot::unseen() { cam.viewport = Some((0, 0, 1, 1)); }
        set_camera(&cam);
    }

    /// Draw with `draw` shaken by `shake`, then the view restored.
    pub fn shaken(&self, shake: &crate::Shake, draw: impl FnOnce()) {
        let (dx, dy) = shake.offset();
        self.set_offset(dx, dy);
        draw();
        self.set_offset(0.0, 0.0);
    }

    /// Shift everything drawn after this by `(dx, dy)` canvas pixels, for a
    /// screen shake; `set_offset(0.0, 0.0)` restores the normal view.
    pub fn set_offset(&self, dx: f32, dy: f32) {
        let mut cam = Camera2D::from_display_rect(
            Rect::new(-dx, -dy, self.width as f32, self.height as f32),
        );
        cam.render_target = Some(self.rt.clone());
        if crate::bot::unseen() { cam.viewport = Some((0, 0, 1, 1)); }
        set_camera(&cam);
    }

    /// Draw the canvas region (`x`, `y`, `w`, `h`) over the whole canvas: a camera that pans and zooms.
    /// `set_offset(0.0, 0.0)` restores the normal view.
    pub fn set_view_rect(&self, x: f32, y: f32, w: f32, h: f32) {
        let mut cam = Camera2D::from_display_rect(Rect::new(x, y, w, h));
        cam.render_target = Some(self.rt.clone());
        if crate::bot::unseen() { cam.viewport = Some((0, 0, 1, 1)); }
        set_camera(&cam);
    }

    /// Turn everything drawn after this by `degrees` (clockwise) about the middle of the canvas, shifted by
    /// (`dx`, `dy`): a camera swinging round a corner. `set_offset(0.0, 0.0)` restores the normal view.
    pub fn set_view(&self, dx: f32, dy: f32, degrees: f32) {
        let mut cam = Camera2D::from_display_rect(
            Rect::new(-dx, -dy, self.width as f32, self.height as f32),
        );
        cam.rotation = degrees;
        cam.render_target = Some(self.rt.clone());
        if crate::bot::unseen() { cam.viewport = Some((0, 0, 1, 1)); }
        set_camera(&cam);
    }

    /// Draw through a camera that turns the picture by `radians` (clockwise on screen) about `pivot`, a point in
    /// canvas pixels, and puts the pivot at `to`: a view that turns with the player round a corner.
    /// `set_offset(0.0, 0.0)` restores the normal view.
    pub fn set_view_pivot(&self, pivot: (f32, f32), to: (f32, f32), radians: f32) {
        let (w, h) = (self.width as f32, self.height as f32);
        let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, w, h));
        cam.rotation = radians.to_degrees();
        // The camera turns about its target, which it puts in the middle of the canvas: pick the target so that
        // the pivot, turned with everything else, lands on `to`.
        let (s, c) = radians.sin_cos();
        let (vx, vy) = (to.0 - w / 2.0, to.1 - h / 2.0);
        let (rx, ry) = (vx * c + vy * s, -vx * s + vy * c);
        cam.target = macroquad::math::vec2(pivot.0 - rx, pivot.1 - ry);
        cam.render_target = Some(self.rt.clone());
        if crate::bot::unseen() { cam.viewport = Some((0, 0, 1, 1)); }
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

    /// Where the finger in touch `slot` points, in game pixels: through the
    /// letterbox and, while the curved glass is drawn, through the same bow the
    /// shader gives the picture, so a paddle lands under the fingertip.
    pub fn touch(&self, slot: usize) -> Option<macroquad::math::Vec2> {
        let (fx, fy, _) = crate::web::touch_fraction(slot)?;
        let (vx, vy, vw, vh) = self.viewport();
        if vw <= 0.0 || vh <= 0.0 { return None; }
        let mut u = vec2((fx * screen_width() - vx) / vw, (fy * screen_height() - vy) / vh);
        if self.crt.is_some() && self.fx() == 0 && !self.screenshot_mode {
            u = crt::curve(u);
        }
        Some(vec2(u.x * self.width as f32, u.y * self.height as f32))
    }

    /// Is touch `slot` pressed: a finger down, or a PC's mouse button held
    /// over the trackpad (a pointer merely hovering there is [`Self::touch`]
    /// without being pressed).
    pub fn touch_pressed(&self, slot: usize) -> bool {
        matches!(crate::web::touch_fraction(slot), Some((_, _, true)))
    }

    /// End the current frame: blit the virtual canvas to the screen (with CRT effects),
    /// then yield to macroquad and wait for the next frame.
    /// Call this exactly once at the bottom of your game loop.
    pub async fn next_frame(&mut self) {
        // A playtest's hit boxes go on the canvas, under the glass like the game.
        self.apply_camera();
        crate::bot::overlay();
        if let Some(seal) = &self.seal {
            gl_use_material(seal);
            draw_rectangle(0.0, 0.0, self.width as f32, self.height as f32, WHITE);
            gl_use_default_material();
        }
        // A screen-space camera, not set_default_camera(): that leaves
        // camera_matrix on the RT projection, which clips blits when the
        // window is larger than the canvas.
        {
            let cam = Camera2D::from_display_rect(
                Rect::new(0.0, 0.0, screen_width(), screen_height()),
            );
            set_camera(&cam);
        }
        clear_background(macroquad::color::BLACK);
        if !crate::bot::unseen() { self.draw_post_process(); }

        // Screenshot capture: save after frame 2 (title screen fully rendered) and exit.
        #[cfg(not(target_arch = "wasm32"))]
        if self.screenshot_mode {
            self.screenshot_frame += 1;
            if self.screenshot_frame >= self.screenshot_frame_target {
                if let Some(ref path) = self.screenshot_path {
                    let img = macroquad::texture::get_screen_data();
                    img.export_png(path);
                }
                std::process::exit(0);
            }
        }

        crate::bot::before_present();

        next_frame().await;

        // Prepare render target for the next game frame.
        self.apply_camera();
        clear_background(macroquad::color::BLACK);

        let raw = get_frame_time();
        self.delta_time = if raw > 0.1 { 0.1 } else { raw };
        if let Some(fixed) = crate::bot::after_present() { self.delta_time = fixed; }
        self.update_fx_level(raw);
        self.update_glitch(self.delta_time);
        self.interlace_field ^= 1;
    }

    /// Watch the frame time and drop a level of CRT effects if the device
    /// clearly can't keep up. Latches down — never steps back up — so it
    /// can't oscillate between quality levels mid-game.
    fn update_fx_level(&mut self, raw_ft: f32) {
        // Dev / testing override: BLIP_FX=0|1|2 pins the level (native only).
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(v) = std::env::var_os("BLIP_FX")
            .and_then(|s| s.to_string_lossy().trim().parse::<u8>().ok())
        {
            self.fx_level = v.min(2);
            return;
        }

        if self.fx_level >= 2 { return; }
        if self.fx_settle > 0 { self.fx_settle -= 1; return; }
        // Ignore a zero reading or a one-off hitch (tab switch, GC pause).
        let ft = raw_ft;
        if !(ft > 0.0005 && ft < 0.1) { return; }

        // Smoothed frame time, ~0.4s time constant at 60fps.
        self.fx_frame_ema += (ft - self.fx_frame_ema) * 0.08;

        // 16.7ms = 60fps. Trip once we're sustained below ~50fps; the worse
        // it's running, the less jank we sit through before acting.
        if self.fx_frame_ema > 0.020 {
            self.fx_slow_accum += ft;
            let trip = if self.fx_frame_ema > 0.030 { 0.8 } else { 1.8 };
            if self.fx_slow_accum > trip {
                self.fx_level += 1;
                self.fx_slow_accum = 0.0;
                self.fx_frame_ema = 1.0 / 60.0;
                self.fx_settle = 30;
                // Free the offscreen composite target — the shader pass that
                // needed it is off now.
                if self.fx_level >= 1 {
                    if let Some(old) = self.screen_rt.take() { retire(old); }
                    self.screen_rt_w = 0;
                    self.screen_rt_h = 0;
                }
            }
        } else {
            self.fx_slow_accum = (self.fx_slow_accum - ft).max(0.0);
        }
    }

    // ------------------------------------------------------------------ //
    // Glitch state machine                                                 //
    // ------------------------------------------------------------------ //

    /// The picture's faults: a line tear, a nudge and a colour fringe, each rare (minutes apart) and slight, a hint of
    /// an old set rather than something to watch for.
    fn update_glitch(&mut self, dt: f32) {
        if !self.crt_look || !crate::web::glitches() { return; }
        let lh = self.height as f32;

        // Tear
        if self.tear_t > 0.0 {
            self.tear_t -= dt;
        } else {
            self.tear_cd -= dt;
            if self.tear_cd <= 0.0 {
                self.tear_t  = 0.025 + self.rng.next() * 0.035;
                self.tear_y  = 0.15 + self.rng.next() * 0.70;
                self.tear_dx = (self.rng.next() - 0.5) * 3.0;
                self.tear_cd = 150.0 + self.rng.next() * 150.0;
            }
        }

        // A brief vertical nudge, rather than rolling the whole picture out of view.
        if self.roll_t > 0.0 {
            self.roll_t  -= dt;
            self.roll_dy  = (self.roll_dy + self.roll_spd * dt) % lh;
        } else {
            self.roll_cd -= dt;
            if self.roll_cd <= 0.0 {
                self.roll_t   = 0.08 + self.rng.next() * 0.06;
                self.roll_spd = 2.0 + self.rng.next() * 4.0;
                self.roll_dy  = 0.0;
                self.roll_cd  = 360.0 + self.rng.next() * 360.0;
            }
        }

        // Chromatic aberration
        if self.chroma_t > 0.0 {
            self.chroma_t -= dt;
        } else {
            self.chroma_cd -= dt;
            if self.chroma_cd <= 0.0 {
                self.chroma_t  = 0.025 + self.rng.next() * 0.05;
                self.chroma_dx = 0.4 + self.rng.next() * 0.8;
                self.chroma_cd = 120.0 + self.rng.next() * 120.0;
            }
        }
    }

    // ------------------------------------------------------------------ //
    // Post-process rendering                                               //
    // ------------------------------------------------------------------ //

    fn draw_post_process(&mut self) {
        let (vx, vy, vw, vh) = self.viewport();
        if vw <= 0.0 || vh <= 0.0 { return; }

        // Screenshot mode: clean 1:1 blit, all CRT effects suppressed — unless
        // BLIP_SCREENSHOT_FX is set, which keeps the full pipeline (handy for
        // eyeballing the curved-glass shader from a headless capture).
        if self.screenshot_mode && std::env::var_os("BLIP_SCREENSHOT_FX").is_none() {
            let tex = self.rt.texture.clone();
            draw_texture_ex(&tex, vx, vy, WHITE, DrawTextureParams {
                dest_size: Some(vec2(vw, vh)),
                ..Default::default()
            });
            return;
        }

        // No curved-glass material (old WebGL / driver quirk), or the
        // adaptive quality has switched it off on a slow device: composite
        // straight to the screen.
        let Some(crt) = self.crt.clone().filter(|_| self.fx() == 0) else {
            self.composite(vx, vy, vw, vh, false);
            return;
        };

        // 1. Composite the frame + glitch + scanlines flat into an offscreen
        //    target sized to the letterboxed viewport.
        let iw = (vw as i32).max(1);
        let ih = (vh as i32).max(1);
        if self.screen_rt.is_none() || self.screen_rt_w != iw || self.screen_rt_h != ih {
            let srt = render_target_ex(iw as u32, ih as u32, RenderTargetParams {
                sample_count: 0,
                ..Default::default()
            });
            srt.texture.set_filter(FilterMode::Linear);
            if let Some(old) = self.screen_rt.replace(srt) { retire(old); }
            self.screen_rt_w = iw;
            self.screen_rt_h = ih;
        }
        let srt = self.screen_rt.clone().unwrap();
        {
            let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, vw, vh));
            cam.render_target = Some(srt.clone());
            set_camera(&cam);
        }
        clear_background(macroquad::color::BLACK);
        self.composite(0.0, 0.0, vw, vh, true);

        // 2. Blit the offscreen target back to the screen through the shader,
        //    which bows the image into a curved tube, blooms the bright
        //    phosphor, darkens the corners and lays a soft glare on the glass.
        {
            let cam = Camera2D::from_display_rect(
                Rect::new(0.0, 0.0, screen_width(), screen_height()));
            set_camera(&cam);
        }
        clear_background(macroquad::color::BLACK);
        crt.set_uniform("ScreenSize", vec2(vw, vh));
        crt.set_uniform("Bloom", self.bloom);
        gl_use_material(&crt);
        // The composite was drawn under a render-target camera whose vertical
        // axis is inverted vs. the screen; the shader flips it back (`fuv`).
        draw_texture_ex(&srt.texture, vx, vy, WHITE, DrawTextureParams {
            dest_size: Some(vec2(vw, vh)),
            ..Default::default()
        });
        gl_use_default_material();
    }

    /// Composite glitch effects and scanlines into `(vx, vy, vw, vh)`.
    /// `offscreen_target` corrects scanline parity, which flips for FBOs.
    fn composite(&mut self, vx: f32, vy: f32, vw: f32, vh: f32, offscreen_target: bool) {
        let scale = vw / self.width as f32;
        // Shrunk (a phone shows Meteors at half size), nearest sampling
        // drops every other pixel and one-pixel lines break up: blend then.
        // Enlarged, pixels stay crisp.
        let tex = self.rt.texture.clone();
        if (scale < 1.0) != self.rt_blended {
            self.rt_blended = scale < 1.0;
            let filter = if self.rt_blended { FilterMode::Linear } else { FilterMode::Nearest };
            self.rt.texture.set_filter(filter);
        }

        // Chromatic aberration: coloured ghost layers under the picture.
        if self.chroma_t > 0.0 {
            let dx = self.chroma_dx * scale;
            let params = || DrawTextureParams { dest_size: Some(vec2(vw, vh)), ..Default::default() };
            draw_texture_ex(&tex, vx - dx, vy, Color::new(1.0, 0.0, 0.0, 0.12), params());
            draw_texture_ex(&tex, vx + dx, vy, Color::new(0.0, 0.4, 1.0, 0.12), params());
        }

        // The top-origin screen camera and `gl_FragCoord` keep scanlines aligned across roll/tear strips.
        let scanline_shader_active = self.fx() < 2 && self.scanline.is_some();
        if scanline_shader_active {
            let scanline = self.scanline.as_ref().unwrap();
            let field = if offscreen_target { self.interlace_field } else { 1 - self.interlace_field };
            scanline.set_uniform("InterlaceField", field as f32);
            scanline.set_uniform("FieldDim", self.field_dim);
            gl_use_material(scanline);
        }
        let roll_on = self.roll_t > 0.0;
        if roll_on {
            self.draw_rolled(&tex, vx, vy, vw, vh);
        } else if self.tear_t > 0.0 { // a tear never combines with a roll
            self.draw_torn(&tex, vx, vy, vw, vh);
        } else {
            draw_texture_ex(&tex, vx, vy, WHITE, DrawTextureParams { dest_size: Some(vec2(vw, vh)), ..Default::default() });
        }
        if scanline_shader_active { gl_use_default_material(); }

        // If shader compilation failed, draw scanlines as row rectangles. Quality level 2 skips them.
        if self.fx() < 2 && self.scanline.is_none() { self.draw_scanline_rows(vx, vy, vw, vh); }
        if self.fx() < 2 { self.draw_noise(vx, vy, vw, vh, scale); }
    }

    /// The vertical roll: the picture slid up by `roll_dy` game rows, the rows off the top wrapping round below.
    fn draw_rolled(&self, tex: &macroquad::texture::Texture2D, vx: f32, vy: f32, vw: f32, vh: f32) {
        let (lw, lh) = (self.width as f32, self.height as f32);
        // Upper screen strip: game rows [roll_dy, lh)
        let top_src_h = lh - self.roll_dy;
        let top_dst_h = vh * top_src_h / lh;
        draw_texture_ex(tex, vx, vy, WHITE, DrawTextureParams {
            dest_size: Some(vec2(vw, top_dst_h)),
            source:    Some(Rect::new(0.0, self.roll_dy, lw, top_src_h)),
            ..Default::default()
        });
        // Lower screen strip: game rows [0, roll_dy) (wrapped)
        if self.roll_dy >= 1.0 {
            draw_texture_ex(tex, vx, vy + top_dst_h, WHITE, DrawTextureParams {
                dest_size: Some(vec2(vw, vh - top_dst_h)),
                source:    Some(Rect::new(0.0, 0.0, lw, self.roll_dy)),
                ..Default::default()
            });
        }
    }

    /// The tear: the picture below `tear_y` shifted sideways, with a bright glitch line along the split.
    fn draw_torn(&mut self, tex: &macroquad::texture::Texture2D, vx: f32, vy: f32, vw: f32, vh: f32) {
        let (lw, lh) = (self.width as f32, self.height as f32);
        let scale = vw / lw;
        let split_lh = self.tear_y * lh;
        let split_vh = self.tear_y * vh;
        let tdx      = self.tear_dx * scale;
        // Top strip: game rows [0, split_lh)
        draw_texture_ex(tex, vx, vy, WHITE, DrawTextureParams {
            dest_size: Some(vec2(vw, split_vh)),
            source:    Some(Rect::new(0.0, 0.0, lw, split_lh)),
            ..Default::default()
        });
        // Bottom strip: game rows [split_lh, lh), shifted horizontally
        draw_texture_ex(tex, vx + tdx, vy + split_vh, WHITE, DrawTextureParams {
            dest_size: Some(vec2(vw, vh - split_vh)),
            source:    Some(Rect::new(0.0, split_lh, lw, lh - split_lh)),
            ..Default::default()
        });
        // The glitch line is plain colour, not the scanline-shaded texture, so reset first (its own alpha needs
        // to blend normally; crt::SCANLINE_FRAGMENT doesn't handle that).
        if self.fx() < 2 && self.scanline.is_some() { gl_use_default_material(); }
        let gw = vw * (0.2 + self.rng.next() * 0.3);
        let gh = 1.0;
        let ga = 0.1 + self.rng.next() * 0.1;
        draw_rectangle(vx, vy + split_vh - gh * 0.5, gw, gh, Color::new(1.0, 1.0, 1.0, ga));
    }

    /// Scanlines as row rectangles, for when the scanline shader did not compile.
    fn draw_scanline_rows(&self, vx: f32, vy: f32, vw: f32, vh: f32) {
        let active   = Color { r: 0.0, g: 0.0, b: 0.0, a: 60.0 / 255.0 };
        let inactive = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 - self.field_dim };
        let bottom   = vy + vh;
        let f0 = self.interlace_field as f32;
        let f1 = 1.0 - f0;
        let mut sy = vy + f0;
        while sy < bottom { draw_rectangle(vx, sy, vw, 1.0, active);   sy += 2.0; }
        let mut sy = vy + f1;
        while sy < bottom { draw_rectangle(vx, sy, vw, 1.0, inactive); sy += 2.0; }
    }

    /// Background noise: a few dozen faint specks, a game pixel each.
    fn draw_noise(&mut self, vx: f32, vy: f32, vw: f32, vh: f32, scale: f32) {
        let pixel = scale.max(1.0);
        for _ in 0..48 {
            let nx = vx + self.rng.next() * vw;
            let ny = vy + self.rng.next() * vh;
            let a  = 0.02 + self.rng.next() * 0.10;
            let v  = self.rng.next();
            draw_rectangle(nx, ny, pixel, pixel, Color::new(v, v, v, a));
        }
    }

    // ----- drawing helpers — see blip::draw and blip::font for full docs -----

    /// How bright the interlace's fading rows stay, 0.25 (the default: lines
    /// on black) to 0.76 (no interlace). Rows swap between this and 0.76
    /// every frame, which twitters on a game of bright filled scenes.
    pub fn set_interlace(&mut self, dim: f32) { self.field_dim = dim.clamp(0.25, 0.76); }

    /// The post-process level drawn: the adaptive one, or 2 (none of the
    /// CRT) for a clean picture.
    fn fx(&self) -> u8 { if self.crt_look { self.fx_level } else { 2 } }

    /// The device could not hold the frame rate and the post-process has
    /// stepped down: a game can spend less on its own effects too.
    pub fn low_power(&self) -> bool { self.fx_level > 0 }

    /// Set the strength of the screen's phosphor bloom, 0.0 (none) to 1.0
    /// (the default, tuned for bright lines on a black field).
    pub fn set_bloom(&mut self, k: f32) { self.bloom = k.clamp(0.0, 1.0); }
}
