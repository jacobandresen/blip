//! Spell effects: sparks, embers, smoke, shock rings, light pillars and
//! passing lights. The game spawns them from its rules; `art.rs` draws them
//! (most additively, so overlapping magic burns brighter) and feeds their
//! lights into the cellar's light map.

use blip::BlipColor;
use macroquad::rand::gen_range;
use std::f32::consts::TAU;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Kind {
    /// A hot point that streaks along its velocity.
    Spark,
    /// A soft glowing mote that drifts and twinkles.
    Mote,
    /// A grey puff that grows and fades (drawn normally, not added).
    Smoke,
    /// A tumbling shard of light.
    Shard,
}

#[derive(Copy, Clone)]
pub struct Particle {
    pub kind: Kind,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub t: f32,
    pub life: f32,
    pub size: f32,
    pub c: BlipColor,
    pub drag: f32,
    pub lift: f32,
    pub spin: f32,
}

/// A widening ring of light.
pub struct Wave {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub life: f32,
    pub r: f32,
    pub width: f32,
    pub c: BlipColor,
    /// Runes drawn around the ring as it widens.
    pub glyphs: bool,
}

/// A column of light rising from the floor.
pub struct Pillar {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub life: f32,
    pub w: f32,
    pub c: BlipColor,
}

/// A point light that fades out, for flashes the light map should see.
pub struct Flash {
    pub x: f32,
    pub y: f32,
    pub t: f32,
    pub life: f32,
    pub r: f32,
    pub c: [f32; 3],
    /// Shines through walls (a reveal sees everything).
    pub through: bool,
}

/// A struck figure: its sprite flashes, then unravels upward.
pub struct Corpse {
    pub x: f32,
    pub y: f32,
    pub col: u32,
    pub row: u32,
    pub flip: bool,
    pub t: f32,
    pub life: f32,
    pub c: BlipColor,
}

#[derive(Default)]
pub struct Magic {
    pub corpses: Vec<Corpse>,
    pub parts: Vec<Particle>,
    pub waves: Vec<Wave>,
    pub pillars: Vec<Pillar>,
    pub flashes: Vec<Flash>,
    /// Screen shake, in pixels; decays quickly.
    pub shake: f32,
    /// A whole-screen tint that fades (white for a ward break, red for a death).
    pub tint: Option<(BlipColor, f32, f32)>,
}

pub fn rgb3(c: BlipColor) -> [f32; 3] {
    [c.r, c.g, c.b]
}

impl Magic {
    pub fn clear(&mut self) {
        self.corpses.clear();
        self.parts.clear();
        self.waves.clear();
        self.pillars.clear();
        self.flashes.clear();
        self.shake = 0.0;
        self.tint = None;
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.parts {
            p.t += dt;
            p.vy -= p.lift * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            let k = 1.0 - p.drag * dt;
            p.vx *= k;
            p.vy *= k;
        }
        self.parts.retain(|p| p.t < p.life);
        for c in &mut self.corpses {
            c.t += dt;
        }
        self.corpses.retain(|c| c.t < c.life);
        for w in &mut self.waves {
            w.t += dt;
        }
        self.waves.retain(|w| w.t < w.life);
        for p in &mut self.pillars {
            p.t += dt;
        }
        self.pillars.retain(|p| p.t < p.life);
        for f in &mut self.flashes {
            f.t += dt;
        }
        self.flashes.retain(|f| f.t < f.life);
        self.shake = (self.shake - dt * 30.0).max(0.0);
        if let Some((_, t, life)) = &mut self.tint {
            *t += dt;
            if *t >= *life {
                self.tint = None;
            }
        }
        // A runaway spell storm can't swamp a phone.
        if self.parts.len() > 900 {
            let extra = self.parts.len() - 900;
            self.parts.drain(0..extra);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn(&mut self, kind: Kind, x: f32, y: f32, vx: f32, vy: f32, life: f32, size: f32, c: BlipColor) {
        self.parts.push(Particle {
            kind,
            x,
            y,
            vx,
            vy,
            t: 0.0,
            life,
            size,
            c,
            drag: match kind {
                Kind::Spark => 3.0,
                Kind::Mote => 1.5,
                Kind::Smoke => 1.2,
                Kind::Shard => 2.0,
            },
            lift: match kind {
                Kind::Mote => 14.0,
                Kind::Smoke => 18.0,
                _ => 0.0,
            },
            spin: gen_range(-8.0, 8.0),
        });
    }

    /// Sparks thrown round a point.
    pub fn burst(&mut self, x: f32, y: f32, n: usize, speed: f32, c: BlipColor, kind: Kind) {
        for i in 0..n {
            let a = (i as f32 + gen_range(0.0, 0.8)) / n as f32 * TAU;
            let v = speed * gen_range(0.35, 1.0);
            let life = gen_range(0.35, 0.8);
            let size = gen_range(1.2, 2.6);
            self.spawn(kind, x, y, a.cos() * v, a.sin() * v, life, size, c);
        }
    }

    pub fn wave(&mut self, x: f32, y: f32, r: f32, life: f32, width: f32, c: BlipColor, glyphs: bool) {
        self.waves.push(Wave { x, y, t: 0.0, life, r, width, c, glyphs });
    }

    pub fn flash(&mut self, x: f32, y: f32, r: f32, life: f32, c: BlipColor) {
        self.flashes.push(Flash { x, y, t: 0.0, life, r, c: rgb3(c), through: false });
    }

    pub fn flash_through(&mut self, x: f32, y: f32, r: f32, life: f32, c: BlipColor) {
        self.flashes.push(Flash { x, y, t: 0.0, life, r, c: rgb3(c), through: true });
    }

    pub fn pillar(&mut self, x: f32, y: f32, w: f32, life: f32, c: BlipColor) {
        self.pillars.push(Pillar { x, y, t: 0.0, life, w, c });
    }

    pub fn tint(&mut self, c: BlipColor, life: f32) {
        self.tint = Some((c, 0.0, life));
    }

    // ---- the spells and their consequences -----------------------------------

    /// The staff's crystal flares as a bolt leaves it.
    pub fn cast(&mut self, x: f32, y: f32, dx: f32, dy: f32, c: BlipColor) {
        self.flash(x, y, 70.0, 0.18, c);
        for _ in 0..10 {
            let a = dy.atan2(dx) + gen_range(-0.7, 0.7);
            let v = gen_range(60.0, 180.0);
            self.spawn(Kind::Spark, x, y, a.cos() * v, a.sin() * v, gen_range(0.15, 0.35), gen_range(1.0, 2.0), c);
        }
        self.wave(x, y, 14.0, 0.22, 1.5, c, false);
    }

    /// Glitter shed behind a flying bolt.
    pub fn trail(&mut self, x: f32, y: f32, c: BlipColor) {
        let jx = gen_range(-3.0, 3.0);
        let jy = gen_range(-3.0, 3.0);
        self.spawn(Kind::Mote, x + jx, y + jy, jx * 6.0, jy * 6.0, gen_range(0.25, 0.5), gen_range(0.8, 1.8), c);
    }

    /// A fireball's smoke and cinders.
    pub fn cinders(&mut self, x: f32, y: f32, c: BlipColor) {
        self.spawn(Kind::Smoke, x + gen_range(-2.0, 2.0), y + gen_range(-2.0, 2.0), 0.0, 0.0, gen_range(0.4, 0.7), gen_range(3.0, 5.0), BlipColor { r: 0.18, g: 0.16, b: 0.16, a: 0.25 });
        self.spawn(Kind::Spark, x, y, gen_range(-30.0, 30.0), gen_range(-30.0, 30.0), gen_range(0.2, 0.4), 1.3, c);
    }

    /// A bolt dashes itself on stone.
    pub fn splash(&mut self, x: f32, y: f32, c: BlipColor) {
        self.burst(x, y, 12, 140.0, c, Kind::Spark);
        self.flash(x, y, 60.0, 0.25, c);
        self.wave(x, y, 16.0, 0.25, 1.5, c, false);
    }

    /// Reveal: a wave of runes sweeps the cellar and lights everything it passes.
    pub fn reveal(&mut self, x: f32, y: f32, c: BlipColor) {
        self.wave(x, y, 260.0, 1.0, 3.0, c, true);
        self.wave(x, y, 150.0, 0.65, 1.5, BlipColor { r: 1.0, g: 1.0, b: 1.0, a: 0.8 }, false);
        self.flash_through(x, y, 300.0, 1.1, c);
        self.flash(x, y, 120.0, 0.4, BlipColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 });
        self.tint(BlipColor { r: 0.5, g: 0.85, b: 1.0, a: 0.12 }, 0.4);
        for _ in 0..40 {
            let a = gen_range(0.0, TAU);
            let v = gen_range(40.0, 220.0);
            self.spawn(Kind::Mote, x, y, a.cos() * v, a.sin() * v, gen_range(0.5, 1.1), gen_range(1.0, 2.4), c);
        }
    }

    /// Cleanse: a pillar of gold light, spiralling motes, the stone ringing.
    pub fn cleanse(&mut self, x: f32, y: f32, c: BlipColor) {
        self.pillar(x, y, 26.0, 1.5, c);
        self.flash(x, y, 200.0, 1.3, c);
        self.wave(x, y, 70.0, 0.8, 2.5, c, true);
        self.wave(x, y, 36.0, 0.5, 4.0, BlipColor { r: 1.0, g: 1.0, b: 0.9, a: 1.0 }, false);
        self.tint(BlipColor { r: 1.0, g: 0.85, b: 0.4, a: 0.12 }, 0.5);
        for _ in 0..30 {
            let h = gen_range(0.0, 200.0);
            self.spawn(Kind::Mote, x + gen_range(-9.0, 9.0), y - h * 0.3, gen_range(-6.0, 6.0), -gen_range(80.0, 220.0), gen_range(0.6, 1.4), gen_range(0.8, 1.8), c);
        }
        for i in 0..36 {
            let a = i as f32 / 36.0 * TAU;
            let r = gen_range(6.0, 14.0);
            let px = x + a.cos() * r;
            let py = y + a.sin() * r * 0.5;
            self.spawn(Kind::Mote, px, py, -a.sin() * 40.0, -gen_range(40.0, 120.0), gen_range(0.8, 1.5), gen_range(1.0, 2.2), c);
        }
        self.shake = self.shake.max(2.0);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn corpse(&mut self, x: f32, y: f32, col: u32, row: u32, flip: bool, life: f32, c: BlipColor) {
        self.corpses.push(Corpse { x, y, col, row, flip, t: 0.0, life, c });
    }

    /// A monster unravels into embers and a rising wisp.
    pub fn dissolve(&mut self, x: f32, y: f32, c: BlipColor) {
        self.burst(x, y, 34, 220.0, c, Kind::Spark);
        self.burst(x, y, 10, 90.0, BlipColor { r: 1.0, g: 1.0, b: 0.9, a: 1.0 }, Kind::Spark);
        for _ in 0..24 {
            self.spawn(Kind::Mote, x + gen_range(-8.0, 8.0), y + gen_range(-8.0, 8.0), gen_range(-20.0, 20.0), gen_range(-60.0, -10.0), gen_range(0.6, 1.2), gen_range(1.2, 2.6), c);
        }
        for _ in 0..5 {
            self.spawn(Kind::Smoke, x + gen_range(-6.0, 6.0), y + gen_range(-6.0, 6.0), gen_range(-10.0, 10.0), gen_range(-20.0, 0.0), gen_range(0.5, 0.9), gen_range(4.0, 7.0), BlipColor { r: 0.12, g: 0.12, b: 0.15, a: 0.22 });
        }
        self.wave(x, y, 44.0, 0.45, 3.0, c, false);
        self.flash(x, y, 150.0, 0.5, c);
        self.flash(x, y, 60.0, 0.15, BlipColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 });
        self.shake = self.shake.max(3.0);
    }

    /// A ward crystal shatters: shards, a white-violet blast, the room shakes.
    pub fn shatter(&mut self, x: f32, y: f32, c: BlipColor) {
        for _ in 0..26 {
            let a = gen_range(0.0, TAU);
            let v = gen_range(80.0, 260.0);
            self.spawn(Kind::Shard, x, y, a.cos() * v, a.sin() * v, gen_range(0.5, 1.0), gen_range(2.0, 4.0), c);
        }
        self.burst(x, y, 30, 240.0, BlipColor { r: 1.0, g: 0.9, b: 1.0, a: 1.0 }, Kind::Spark);
        self.wave(x, y, 140.0, 0.7, 4.0, c, true);
        self.wave(x, y, 70.0, 0.45, 2.0, BlipColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, false);
        self.flash(x, y, 300.0, 0.8, c);
        self.tint(BlipColor { r: 0.9, g: 0.75, b: 1.0, a: 0.35 }, 0.35);
        self.shake = self.shake.max(7.0);
    }

    /// A wizard falls: blue flame, scattered sparks, a red pulse.
    pub fn fall(&mut self, x: f32, y: f32, c: BlipColor) {
        self.burst(x, y, 30, 200.0, c, Kind::Spark);
        for _ in 0..18 {
            self.spawn(Kind::Mote, x + gen_range(-6.0, 6.0), y + gen_range(-10.0, 6.0), gen_range(-30.0, 30.0), gen_range(-90.0, -20.0), gen_range(0.6, 1.4), gen_range(1.4, 3.0), c);
        }
        self.wave(x, y, 60.0, 0.6, 3.0, c, false);
        self.flash(x, y, 140.0, 0.6, c);
        self.tint(BlipColor { r: 0.6, g: 0.0, b: 0.0, a: 0.35 }, 0.5);
        self.shake = self.shake.max(6.0);
    }

    /// A gate carries a wizard across: a swirl at each end.
    pub fn warp(&mut self, x: f32, y: f32, c: BlipColor) {
        for i in 0..24 {
            let a = i as f32 / 24.0 * TAU;
            let r = 16.0;
            self.spawn(Kind::Mote, x + a.cos() * r, y + a.sin() * r, -a.sin() * 90.0 - a.cos() * 40.0, a.cos() * 90.0 - a.sin() * 40.0, 0.45, 1.6, c);
        }
        self.wave(x, y, 26.0, 0.35, 2.0, c, false);
        self.flash(x, y, 90.0, 0.4, c);
    }

    /// The way down opens: a pillar and a ring of gold over the stairs.
    pub fn open(&mut self, x: f32, y: f32, c: BlipColor) {
        self.pillar(x, y, 28.0, 1.6, c);
        self.wave(x, y, 90.0, 0.9, 3.0, c, true);
        self.flash(x, y, 200.0, 1.2, c);
        self.burst(x, y, 30, 150.0, c, Kind::Mote);
    }

    /// The Warlock tears through from elsewhere.
    pub fn summon(&mut self, x: f32, y: f32, c: BlipColor) {
        for i in 0..28 {
            let a = i as f32 / 28.0 * TAU;
            let r = gen_range(18.0, 30.0);
            self.spawn(Kind::Mote, x + a.cos() * r, y + a.sin() * r, -a.cos() * r * 2.5 - a.sin() * 50.0, -a.sin() * r * 2.5 + a.cos() * 50.0, 0.5, 1.8, c);
        }
        self.flash(x, y, 120.0, 0.6, c);
    }
}
