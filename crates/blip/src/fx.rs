//! Small feedback effects every game can use to show what just happened:
//! a score popup that rises and fades, a burst of sparks, a ring that
//! widens from a point, and a screen shake.
//!
//! Each is a plain struct kept in the game's state. Call `update(dt)` once a
//! frame and `draw(&blip)` after the playfield:
//!
//! ```ignore
//! g.fx.popup(x, y, "+50", BLIP_YELLOW);
//! g.fx.burst(x, y, 12, 90.0, BLIP_RED);
//! g.fx.update(dt);
//! g.fx.draw(&blip);
//! ```

use macroquad::color::Color;

use crate::ctx::Blip;
use crate::font;

/// How long a popup stays up, and how far it rises (px).
const POPUP_SECS: f32 = 0.8;
const POPUP_RISE: f32 = 28.0;

struct Popup { x: f32, y: f32, t: f32, text: String, c: Color }
struct Spark { x: f32, y: f32, vx: f32, vy: f32, t: f32, life: f32, c: Color, size: f32 }
struct Ring { x: f32, y: f32, t: f32, life: f32, r: f32, c: Color }

/// The effects layer: popups, sparks, rings and shake together.
#[derive(Default)]
pub struct Fx {
    popups: Vec<Popup>,
    sparks: Vec<Spark>,
    rings: Vec<Ring>,
    /// Pulls sparks down, px/s². 0 lets them drift (space, a top-down board).
    pub gravity: f32,
    shake: f32,
}

impl Fx {
    pub fn new() -> Self { Self::default() }

    /// Text (usually points) that rises from `x`, `y` and fades out.
    pub fn popup(&mut self, x: f32, y: f32, text: &str, c: Color) {
        self.popups.push(Popup { x, y, t: 0.0, text: text.to_string(), c });
    }

    /// `n` sparks thrown out from a point at up to `speed` px/s.
    pub fn burst(&mut self, x: f32, y: f32, n: usize, speed: f32, c: Color) {
        use macroquad::rand::gen_range;
        for i in 0..n {
            // Evenly round the circle with a little jitter, so a burst reads as round.
            let a = (i as f32 + gen_range(0.0, 0.6)) / n as f32 * std::f32::consts::TAU;
            let v = speed * gen_range(0.45, 1.0);
            let life = gen_range(0.3, 0.55);
            self.sparks.push(Spark { x, y, vx: a.cos() * v, vy: a.sin() * v, t: 0.0, life, c, size: gen_range(1.5, 3.0) });
        }
    }

    /// A thin ring that widens to `radius` over `life` seconds.
    pub fn ring(&mut self, x: f32, y: f32, radius: f32, life: f32, c: Color) {
        self.rings.push(Ring { x, y, t: 0.0, life, r: radius, c });
    }

    /// Shake the screen; strength is in pixels and decays over about a third of a second.
    pub fn shake(&mut self, px: f32) { self.shake = self.shake.max(px); }

    /// The current shake offset to add to the playfield's drawing.
    pub fn shake_offset(&self) -> (f32, f32) {
        if self.shake <= 0.0 { return (0.0, 0.0); }
        use macroquad::rand::gen_range;
        (gen_range(-self.shake, self.shake), gen_range(-self.shake, self.shake))
    }

    pub fn clear(&mut self) {
        self.popups.clear();
        self.sparks.clear();
        self.rings.clear();
        self.shake = 0.0;
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.popups { p.t += dt; }
        self.popups.retain(|p| p.t < POPUP_SECS);
        for s in &mut self.sparks {
            s.t += dt;
            s.vy += self.gravity * dt;
            s.x += s.vx * dt;
            s.y += s.vy * dt;
            s.vx *= 1.0 - 2.5 * dt; // air drag, so a burst slows and hangs
            s.vy *= 1.0 - 2.5 * dt;
        }
        self.sparks.retain(|s| s.t < s.life);
        for r in &mut self.rings { r.t += dt; }
        self.rings.retain(|r| r.t < r.life);
        self.shake = (self.shake - dt * self.shake.max(3.0) * 3.0).max(0.0);
    }

    pub fn draw(&self, blip: &Blip) {
        for r in &self.rings {
            let k = r.t / r.life;
            let rad = r.r * (0.3 + 0.7 * k);
            let c = Color { a: r.c.a * (1.0 - k), ..r.c };
            let dots = (rad * 0.8).clamp(10.0, 32.0) as i32;
            for j in 0..dots {
                let a = j as f32 / dots as f32 * std::f32::consts::TAU;
                blip.fill_circle(r.x + a.cos() * rad, r.y + a.sin() * rad, 1.6, c);
            }
        }
        for s in &self.sparks {
            let k = s.t / s.life;
            blip.fill_rect(s.x - s.size / 2.0, s.y - s.size / 2.0, s.size, s.size, Color { a: s.c.a * (1.0 - k * k), ..s.c });
        }
        for p in &self.popups {
            let k = p.t / POPUP_SECS;
            // Rises quickly then settles; holds full colour for the first half.
            let y = p.y - POPUP_RISE * (1.0 - (1.0 - k) * (1.0 - k));
            let a = if k < 0.5 { 1.0 } else { 1.0 - (k - 0.5) * 2.0 };
            let w = p.text.chars().count() as f32 * 6.0 * 2.0;
            let x = (p.x - w / 2.0).clamp(2.0, blip.width as f32 - w - 2.0);
            font::draw_text(&p.text, x + 1.0, y + 1.0, 2.0, Color { r: 0.0, g: 0.0, b: 0.0, a: a * 0.7 });
            font::draw_text(&p.text, x, y, 2.0, Color { a, ..p.c });
        }
    }
}
