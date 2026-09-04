//! The `Blip` context — the central object every game creates once and holds for its lifetime.
//!
//! `Blip` owns the virtual canvas (a fixed-size render target), drives the frame loop,
//! and applies the CRT post-process effect (scanlines, glitch tears, chromatic aberration,
//! and a curved-glass shader pass) when blitting to the real window. Games interact with it
//! through its drawing methods and `blip.delta_time`.

use macroquad::camera::{set_camera, Camera2D};
use macroquad::color::{Color, WHITE};
use macroquad::material::{
    gl_use_default_material, gl_use_material, load_material, Material, MaterialParams,
};
use macroquad::math::{vec2, Rect};
use macroquad::miniquad::{ShaderSource, UniformDesc, UniformType};
use macroquad::shapes::draw_rectangle;
use macroquad::texture::{
    draw_texture_ex, get_screen_data, render_target_ex, DrawTextureParams, FilterMode,
    RenderTarget, RenderTargetParams,
};
use macroquad::time::get_frame_time;
use macroquad::window::{clear_background, next_frame, screen_height, screen_width, Conf};

use crate::color::*;
use crate::draw;
use crate::font;

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

// ---------------------------------------------------------------------------- //
// Curved-glass shader                                                           //
// ---------------------------------------------------------------------------- //
//
// The flat composite (game frame + glitch + scanlines) is rendered to an
// offscreen target, then this material blits it to the window: the image is
// bowed into a slightly convex tube, the bright phosphor blooms into its
// neighbours, the corners fall off into the black bezel, and a faint diagonal
// glare sits on the "glass". GLSL ES 1.00 so it runs on WebGL 1.

const CRT_VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
"#;

const CRT_FRAGMENT: &str = r#"#version 100
precision mediump float;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform sampler2D Texture;
uniform vec4 _Time;
uniform vec2 ScreenSize;

// Bow flat 0..1 UVs outward into a convex tube. Divisors set how strong the
// barrel is per axis — larger is flatter. The 1.09 factor is underscan: it
// shrinks the picture inside the curved glass so that even after the barrel
// pushes the corners outward, the whole game screen — every edge, including the
// top HUD row — stays visible, framed by a black rim like a real tube that
// never quite filled its own face.
vec2 curve(vec2 p) {
    p = p * 2.0 - 1.0;
    vec2 off = abs(p.yx) / vec2(13.0, 10.0);
    p = p + p * off * off;
    p *= 1.055;
    return p * 0.5 + 0.5;
}

// Cheap ordered dither — breaks up 8-bit banding in the dark gradients.
float dither(vec2 p) {
    return fract(sin(dot(floor(p), vec2(12.9898, 78.233))) * 43758.5453);
}

void main() {
    // The offscreen composite is stored y-flipped relative to the screen; undo
    // that here (a clean full flip, no half-texel offset from DrawTextureParams).
    vec2 fuv = vec2(uv.x, 1.0 - uv.y);

    vec2 cuv = curve(fuv);

    // Thin soft edge where the tube meets the bezel — centred on the boundary so
    // the outermost row/column of the game canvas is still drawn, not eaten.
    vec2 e = smoothstep(vec2(-0.0025), vec2(0.0025), cuv)
           * smoothstep(vec2(-0.0025), vec2(0.0025), vec2(1.0) - cuv);
    float mask = e.x * e.y;

    vec3 col = texture2D(Texture, cuv).rgb;

    // ---- phosphor bloom: blur a ring of taps, keep the bright part, add back ----
    vec2 px = 1.0 / ScreenSize;
    vec3 b = vec3(0.0);
    b += texture2D(Texture, cuv + vec2( 1.5,  0.5) * px * 2.0).rgb;
    b += texture2D(Texture, cuv + vec2(-1.5, -0.5) * px * 2.0).rgb;
    b += texture2D(Texture, cuv + vec2( 0.5, -1.5) * px * 2.0).rgb;
    b += texture2D(Texture, cuv + vec2(-0.5,  1.5) * px * 2.0).rgb;
    b += texture2D(Texture, cuv + vec2( 2.5,  1.5) * px * 4.0).rgb;
    b += texture2D(Texture, cuv + vec2(-2.5, -1.5) * px * 4.0).rgb;
    b += texture2D(Texture, cuv + vec2( 1.5, -2.5) * px * 4.0).rgb;
    b += texture2D(Texture, cuv + vec2(-1.5,  2.5) * px * 4.0).rgb;
    b *= 0.125;
    col += max(b - 0.25, 0.0) * 1.4;

    // ---- tube vignette (light — the corners must stay clearly readable) ----
    float vig = cuv.x * cuv.y * (1.0 - cuv.x) * (1.0 - cuv.y);
    vig = clamp(pow(vig * 18.0, 0.15), 0.0, 1.0);
    col *= mix(1.0, vig, 0.45);

    // ---- glare on the glass (flat fuv, so it doesn't move with the curve) ----
    // Scaled by the pixel's own brightness so it only shows over lit phosphor —
    // over the black background it stays exactly black and can't band.
    float glare = smoothstep(0.7, 0.05,
        distance(fuv * vec2(1.0, 1.3), vec2(0.32, 0.30 * 1.3)));
    col += glare * 0.18 * dot(col, vec3(0.333));

    // gentle mains-hum brightness flicker + a touch of gain
    col *= 1.08 + 0.008 * sin(_Time.x * 10.7);

    // dither the near-black gradients so the vignette doesn't step
    col += (dither(gl_FragCoord.xy) - 0.5) * (1.0 / 255.0);

    gl_FragColor = vec4(col * color.rgb * mask, 1.0);
}
"#;

/// The main blip runtime. Create one at the start of `main` with `Blip::new(w, h)`,
/// then call `blip.next_frame(60).await` at the end of every game loop iteration.
/// Read `blip.delta_time` each frame to get the elapsed seconds since the last tick.
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
    // ---- curved-glass shader pass ----
    crt:         Option<Material>, // None if the shader failed to compile
    screen_rt:   Option<RenderTarget>, // offscreen composite target, window-sized
    screen_rt_w: i32,
    screen_rt_h: i32,
    // ---- screenshot capture ----
    pub screenshot_mode:   bool,
    screenshot_frame:      u32,
    screenshot_frame_target: u32,
    screenshot_path:       Option<String>,
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

        // Curved-glass post-process. If the shader fails to compile (old WebGL,
        // driver quirks) we fall back to compositing straight to the screen.
        let crt = load_material(
            ShaderSource::Glsl { vertex: CRT_VERTEX, fragment: CRT_FRAGMENT },
            MaterialParams {
                uniforms: vec![UniformDesc::new("ScreenSize", UniformType::Float2)],
                ..Default::default()
            },
        )
        .ok();

        let mut rng = Lcg(0xdead_beef);
        // Stagger initial cooldowns so effects don't all fire at once.
        let tear_cd   =  5.0 + rng.next() * 10.0;
        let roll_cd   = 15.0 + rng.next() * 20.0;
        let chroma_cd =  2.0 + rng.next() *  4.0;

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
            rt, rng,
            tear_cd,  tear_t: 0.0, tear_y: 0.5, tear_dx: 0.0,
            roll_cd,  roll_t: 0.0, roll_dy: 0.0, roll_spd: 0.0,
            chroma_cd, chroma_t: 0.0, chroma_dx: 0.0,
            interlace_field: 0,
            crt,
            screen_rt: None,
            screen_rt_w: 0,
            screen_rt_h: 0,
            screenshot_mode,
            screenshot_frame: 0,
            screenshot_frame_target,
            screenshot_path,
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

    /// End the current frame: blit the virtual canvas to the screen (with CRT effects),
    /// then yield to macroquad and wait for the next frame.
    /// Call this exactly once at the bottom of your game loop.
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

        // Screenshot capture: save after frame 2 (title screen fully rendered) and exit.
        #[cfg(not(target_arch = "wasm32"))]
        if self.screenshot_mode {
            self.screenshot_frame += 1;
            if self.screenshot_frame >= self.screenshot_frame_target {
                if let Some(ref path) = self.screenshot_path {
                    let img = get_screen_data();
                    img.export_png(path);
                }
                std::process::exit(0);
            }
        }

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
                self.tear_cd = 14.0 + self.rng.next() * 30.0;
            }
        }

        // Roll — kept rare enough that it reads as a genuine "something's
        // wrong" moment rather than a recurring tic.
        if self.roll_t > 0.0 {
            self.roll_t  -= dt;
            self.roll_dy  = (self.roll_dy + self.roll_spd * dt) % lh;
        } else {
            self.roll_cd -= dt;
            if self.roll_cd <= 0.0 {
                self.roll_t   = 0.5 + self.rng.next() * 1.3;
                self.roll_spd = 180.0 + self.rng.next() * 320.0;
                self.roll_dy  = 0.0;
                self.roll_cd  = 60.0 + self.rng.next() * 90.0;
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
                self.chroma_cd =  7.0 + self.rng.next() * 15.0;
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

        // No curved-glass material: composite straight to the screen, as before.
        let Some(crt) = self.crt.clone() else {
            self.composite(vx, vy, vw, vh);
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
            self.screen_rt   = Some(srt);
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
        self.composite(0.0, 0.0, vw, vh);

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
        gl_use_material(&crt);
        // The composite was drawn under a render-target camera whose vertical
        // axis is inverted vs. the screen; the shader flips it back (`fuv`).
        draw_texture_ex(&srt.texture, vx, vy, WHITE, DrawTextureParams {
            dest_size: Some(vec2(vw, vh)),
            ..Default::default()
        });
        gl_use_default_material();
    }

    /// Composite the current game frame with the glitch effects and interlaced
    /// scanlines, drawn into whatever target/camera is currently bound. `(vx, vy)`
    /// is the top-left corner and `(vw, vh)` the size in that target's pixels.
    fn composite(&mut self, vx: f32, vy: f32, vw: f32, vh: f32) {
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

        // Debug: outline the exact canvas edge so overscan can be eyeballed.
        if std::env::var_os("BLIP_DEBUG_EDGE").is_some() {
            let t = 2.0;
            draw_rectangle(vx, vy, vw, t, BLIP_MAGENTA);
            draw_rectangle(vx, vy + vh - t, vw, t, BLIP_MAGENTA);
            draw_rectangle(vx, vy, t, vh, BLIP_MAGENTA);
            draw_rectangle(vx + vw - t, vy, t, vh, BLIP_MAGENTA);
        }
    }

    // ----- drawing helpers — see blip::draw and blip::font for full docs -----

    /// Fill the canvas with a solid colour. Call this at the start of your draw pass.
    #[inline]
    pub fn clear(&self, c: Color) { draw::clear(c); }
    /// Draw a solid filled rectangle.
    #[inline]
    pub fn fill_rect(&self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        draw::fill_rect(x, y, w, h, c);
    }
    /// Draw a 1-pixel outline rectangle (no fill).
    #[inline]
    pub fn draw_rect(&self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        draw::draw_rect(x, y, w, h, c);
    }
    /// Draw a 1-pixel line between two points.
    #[inline]
    pub fn draw_line(&self, x1: f32, y1: f32, x2: f32, y2: f32, c: Color) {
        draw::draw_line(x1, y1, x2, y2, c);
    }
    /// Draw a line of arbitrary thickness between two points.
    #[inline]
    pub fn draw_line_ex(&self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, c: Color) {
        draw::draw_line_ex(x1, y1, x2, y2, thickness, c);
    }
    /// Draw a neon glowing line — a soft halo under a bright core. Use for vector
    /// silhouettes (ship outlines, asteroid edges) that should read as "lit up".
    #[inline]
    pub fn draw_glow_line(&self, x1: f32, y1: f32, x2: f32, y2: f32, c: Color) {
        draw::draw_glow_line(x1, y1, x2, y2, c);
    }
    /// Draw a solid filled circle. (`cx`, `cy`) is the centre, `r` is the radius.
    #[inline]
    pub fn fill_circle(&self, cx: f32, cy: f32, r: f32, c: Color) {
        draw::fill_circle(cx, cy, r, c);
    }
    /// Draw a soft glowing circle — halo under a bright core. Good for thruster flames,
    /// muzzle flashes, and explosion particles.
    #[inline]
    pub fn fill_glow_circle(&self, cx: f32, cy: f32, r: f32, c: Color) {
        draw::fill_glow_circle(cx, cy, r, c);
    }
    /// Draw a texture stretched to fill the given rectangle.
    #[inline]
    pub fn draw_texture(&self, tex: &macroquad::texture::Texture2D, x: f32, y: f32, w: f32, h: f32) {
        draw::draw_texture(tex, x, y, w, h);
    }
    /// Draw a texture stretched to fill the given rectangle, multiplied by a tint colour.
    #[inline]
    pub fn draw_texture_tinted(
        &self, tex: &macroquad::texture::Texture2D, x: f32, y: f32, w: f32, h: f32, tint: Color,
    ) {
        draw::draw_texture_tinted(tex, x, y, w, h, tint);
    }

    // ----- font helpers -----

    /// Draw a single character at pixel position (`x`, `y`). `sz` is the pixel scale.
    #[inline]
    pub fn draw_char(&self, c: char, x: f32, y: f32, sz: f32, color: Color) {
        font::draw_char(c, x, y, sz, color);
    }
    /// Draw a left-aligned string.
    #[inline]
    pub fn draw_text(&self, text: &str, x: f32, y: f32, sz: f32, color: Color) {
        font::draw_text(text, x, y, sz, color);
    }
    /// Draw an integer as text — avoids a `format!` call.
    #[inline]
    pub fn draw_number(&self, n: i32, x: f32, y: f32, sz: f32, color: Color) {
        font::draw_number(n, x, y, sz, color);
    }
    /// Return the x coordinate that would horizontally centre `text` on this canvas.
    #[inline]
    pub fn text_cx(&self, text: &str, sz: i32) -> i32 {
        font::text_cx(self.width, text, sz)
    }
    /// Draw a string horizontally centred on this canvas.
    #[inline]
    pub fn draw_centered(&self, text: &str, y: f32, sz: f32, color: Color) {
        font::draw_centered(self.width, text, y, sz, color);
    }

    /// Draw the standard two-field HUD bar (SCORE / LIVES) across the top of the canvas.
    pub fn draw_hud(&self, score: i32, lives: i32) {
        let hud_h = 28.0;
        self.fill_rect(0.0, 0.0, self.width as f32, hud_h, BLIP_BLACK);
        self.draw_line(
            0.0, hud_h - 1.0, self.width as f32, hud_h - 1.0, BLIP_DARKGRAY,
        );
        self.draw_text("SCORE", 4.0, 5.0, 2.0, BLIP_YELLOW);
        self.draw_number(score, 68.0, 5.0, 2.0, BLIP_WHITE);
        self.draw_text("LIVES", (self.width - 90) as f32, 5.0, 2.0, BLIP_ORANGE);
        self.draw_number(lives, (self.width - 18) as f32, 5.0, 2.0, BLIP_WHITE);
    }
}
