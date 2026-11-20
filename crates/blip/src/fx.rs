//! Shared score popups, sparks and rings.
//! Call `update(dt)` each frame and `draw(&blip)` after the playfield.

use macroquad::color::Color;

use crate::ctx::Blip;
use crate::font;

/// A moving, expiring effect with game-specific rendering data.
pub struct EffectParticle<T> {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
    pub data: T,
}

/// Shared motion and lifetime handling for effects rendered by a game.
pub struct EffectParticles<T> {
    items: Vec<EffectParticle<T>>,
    pub gravity: f32,
    pub drag: f32,
}

impl<T> EffectParticles<T> {
    pub fn new(gravity: f32, drag: f32) -> Self {
        Self { items: Vec::new(), gravity, drag }
    }

    pub fn push(&mut self, particle: EffectParticle<T>) { self.items.push(particle); }

    pub fn iter(&self) -> impl Iterator<Item = &EffectParticle<T>> { self.items.iter() }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.items {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vx *= 1.0 - self.drag * dt;
            p.vy = p.vy * (1.0 - self.drag * dt) + self.gravity * dt;
            p.life -= dt;
        }
        self.items.retain(|p| p.life > 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{shake, EffectParticle, EffectParticles};

    #[test]
    fn a_shake_stays_within_its_amplitude_and_ends_still() {
        for k in 0..100 {
            let (dx, dy) = shake(k as f32 * 0.003, 0.3, 5.0);
            assert!(dx.abs() <= 5.0 && dy.abs() <= 5.0);
        }
        assert_eq!(shake(0.0, 0.3, 5.0), (0.0, 0.0));
        assert_eq!(shake(-0.1, 0.3, 5.0), (0.0, 0.0));
    }

    #[test]
    fn effect_particles_share_motion_and_lifetime_updates() {
        let mut particles = EffectParticles::new(10.0, 1.0);
        particles.push(EffectParticle {
            x: 1.0, y: 2.0, vx: 4.0, vy: 0.0, life: 0.5, max_life: 0.5, data: (),
        });
        particles.update(0.25);
        let p = particles.iter().next().unwrap();
        assert!((p.x - 2.0).abs() < 1e-6);
        assert!((p.y - 2.0).abs() < 1e-6);
        assert!((p.vx - 3.0).abs() < 1e-6);
        assert!((p.vy - 2.5).abs() < 1e-6);
        assert!((p.life - 0.25).abs() < 1e-6);
        particles.update(0.25);
        assert_eq!(particles.iter().count(), 0);
    }
}

/// How long a popup stays up, and how far it rises (px).
const POPUP_SECS: f32 = 0.8;
const POPUP_RISE: f32 = 28.0;

struct Popup { x: f32, y: f32, t: f32, text: String, c: Color }
struct Spark { x: f32, y: f32, vx: f32, vy: f32, t: f32, life: f32, c: Color, size: f32 }
struct Ring { x: f32, y: f32, t: f32, life: f32, r: f32, c: Color }

/// A screen shake's offset now, for `Blip::set_offset`: `left` of `secs` to go, at most `amp` canvas pixels, easing
/// out. Driven by the time left, so it draws none of the game's random numbers.
pub fn shake(left: f32, secs: f32, amp: f32) -> (f32, f32) {
    if left <= 0.0 || secs <= 0.0 { return (0.0, 0.0); }
    let k = (left / secs).min(1.0);
    let a = amp * k * k;
    ((left * 97.0).sin() * a, (left * 71.0).cos() * a)
}

/// Fireworks for a level cleared, `t` seconds in: a few bursts going off one after another inside `area`
/// (x, y, w, h), each a ring of streaks that opens, droops and fades, in turn through `colours`. Placed by the clock and
/// the burst's number, so it draws none of the game's random numbers.
pub fn draw_fireworks(blip: &Blip, t: f32, area: (f32, f32, f32, f32), colours: &[Color]) {
    const BURSTS: u32 = 5;
    const EVERY: f32 = 0.28;
    const LIFE: f32 = 0.9;
    const STREAKS: u32 = 18;
    for k in 0..BURSTS {
        let age = (t - k as f32 * EVERY) / LIFE;
        if !(0.0..1.0).contains(&age) { continue; }
        let h = (k + 1).wrapping_mul(2_654_435_761).rotate_left(9);
        let cx = area.0 + (h % 1000) as f32 / 1000.0 * area.2;
        let cy = area.1 + ((h >> 10) % 1000) as f32 / 1000.0 * area.3;
        let c = colours[k as usize % colours.len().max(1)];
        let reach = 50.0 + (h >> 20) as f32 % 30.0;
        let r = reach * (1.0 - (1.0 - age) * (1.0 - age));
        let droop = 26.0 * age * age;
        let fade = Color { a: c.a * (1.0 - age), ..c };
        for s in 0..STREAKS {
            let (sn, cs) = (s as f32 / STREAKS as f32 * std::f32::consts::TAU).sin_cos();
            let (x0, y0) = (cx + cs * r * 0.7, cy + sn * r * 0.7 + droop);
            let (x1, y1) = (cx + cs * r, cy + sn * r + droop);
            blip.draw_line_ex(x0, y0, x1, y1, 2.0, fade);
        }
        if age < 0.15 { blip.fill_glow_circle(cx, cy, 10.0 * (1.0 - age / 0.15), Color { a: 0.9, ..c }); }
    }
}

/// The effects layer: popups, sparks and rings together.
#[derive(Default)]
pub struct Fx {
    popups: Vec<Popup>,
    sparks: Vec<Spark>,
    rings: Vec<Ring>,
    /// Pulls sparks down, px/s². 0 lets them drift (space, a top-down board).
    pub gravity: f32,
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

    pub fn clear(&mut self) {
        self.popups.clear();
        self.sparks.clear();
        self.rings.clear();
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
