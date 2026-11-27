//! Vector-graphics asteroid shooter with monochrome line art.

use std::f32::consts::PI;

use blip::color::*;
use blip::input::{coin_start, key_active, key_pressed, play_again, Stick, BLIP_KEY_BUTTON2, BLIP_KEY_SPACE};
use blip::macroquad::texture::Texture2D;
use blip::{Shake,
    hsv,
    Jukebox, pool_iter, pool_iter_mut, pool_spawn, play_sfx, rand_int, rand_range_f32, web, window_conf, Blip, Fx,
    BlipColor, LifeResult, Session, Timer, segment_circle_overlap, wrap, BLIP_WHITE, BLIP_SEA,
    BLIP_LEAF, BLIP_HEART, BLIP_PLUM, BLIP_GOLD,
};

// The effects and music are made by build.rs; the tests check them here.
#[cfg(test)]
mod tunes;
mod bot;
mod levels;
mod draw;
mod rocks;
mod saucer;
use rocks::*;
use saucer::*;
use draw::*;

use levels::*;

// ---- layout -------------------------------------------------------------
const HUD_H:   i32 = 28;
const PLAY_W:  i32 = 680;
const PLAY_H:  i32 = 680;
const WIN_W:   i32 = PLAY_W;
const WIN_H:   i32 = PLAY_H + HUD_H;
const PLAY_Y0: f32 = HUD_H as f32;

// ---- ship -----------------------------------------------------------
const SHIP_TURN_RATE:   f32 = 3.6; // radians/sec
const SHIP_ACCEL:       f32 = 260.0;
const SHIP_MAX_SPEED:   f32 = 340.0;
const SHIP_RADIUS:      f32 = 11.0;
/// Accuracy pays: every STREAK_PER hits in a row without a wasted shot add one to the score multiple, up to
/// STREAK_MAX. Spraying shots breaks the run, so aiming beats firing at random.
const STREAK_PER: i32 = 5;
const STREAK_MAX: i32 = 4;
/// A saucer's hit radius: the big one is the easy target.
fn saucer_radius(big: bool) -> f32 { if big { 16.0 } else { 9.0 } }
const SHIP_INVULN:      f32 = 2.5;
const RESPAWN_DELAY:    f32 = 1.6;
const SAFE_RADIUS:      f32 = 110.0;
const LIVES_START:      i32 = 3;
const EXTRA_LIFE_SCORE: i32 = 10_000;

// ---- weapons --------------------------------------------------------
const MAX_BULLETS:   usize = 10;
const BULLET_SPEED:  f32 = 460.0;
const BULLET_TTL:    f32 = 0.85;
const FIRE_COOLDOWN: f32 = 0.22;
const HYPERSPACE_RISK: i32 = 20; // 1-in-N chance of self-destruct

// ---- asteroids --------------------------------------------------------
const MAX_ASTEROIDS: usize = 48;
const MAX_DEBRIS:    usize = 96;

// A broken big or medium rock leaves a crystal one time in three: points for flying through it before it fades. The
// bot sat still in the middle and turned (variety 0.05 in the fun report); a prize out in the field is a reason to fly.
const CRYSTAL_ODDS:   f32 = 0.33;
const CRYSTAL_TTL:    f32 = 7.0;
const CRYSTAL_WARN:   f32 = 1.5;
const CRYSTAL_R:      f32 = 9.0;
// At 150 a crystal paid less than the shooting the chase cost (the bot's score fell 13 %); 500 makes it a prize.
const CRYSTAL_POINTS: i32 = 500;
const MAX_CRYSTALS:   usize = 3;

/// A crystal drifting where a rock broke.
#[derive(Copy, Clone)]
struct Crystal { x: f32, y: f32, vx: f32, vy: f32, ttl: f32 }

// ---- saucer -----------------------------------------------------------
const SAUCER_SPEED:    f32 = 90.0;
const SAUCER_FIRE_CD:  f32 = 1.4;
const SAUCER_SPAWN_MIN: f32 = 9.0;
const SAUCER_SPAWN_MAX: f32 = 18.0;
/// How long before each shot the saucer's core lights up as a warning.
const SAUCER_TELL: f32 = 0.4;

// ---- smear --------------------------------------------------------------
/// Seconds the smear looks back at wave 0, per wave after, and at most.
const SMEAR_REACH:     f32 = 0.06;
const SMEAR_REACH_MAX: f32 = 0.84;
const SMEAR_ECHOES:    i32 = 10;
/// Pixels a line must have moved to leave an echo, and to leave a full one.
const SMEAR_STILL: f32 = 1.0;
const SMEAR_FULL:  f32 = 6.0;
const SMEAR_ALPHA: f32 = 0.6;

// ---- haze ---------------------------------------------------------------
const HAZES: usize = 8;
/// A patch's alpha at its fullest, on wave 1 and on the full wave.
const HAZE_ALPHA:     f32 = 0.05;
const HAZE_ALPHA_MAX: f32 = 0.42;
/// The same for the mushroom at its centre.
const SHROOM_ALPHA:     f32 = 0.04;
const SHROOM_ALPHA_MAX: f32 = 0.50;
const THREAD_ALPHA:     f32 = 0.10;
const THREAD_ALPHA_MAX: f32 = 0.25;
/// How far a mushroom reaches for a rock.
const THREAD_REACH: f32 = 240.0;

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Play, Dead, Over }

#[derive(Copy, Clone, PartialEq, Eq)]
enum ASize { Large, Medium, Small }

impl ASize {
    fn radius(self) -> f32 {
        match self { ASize::Large => 40.0, ASize::Medium => 22.0, ASize::Small => 11.0 }
    }
    fn speed_range(self) -> (f32, f32) {
        match self {
            ASize::Large  => (25.0, 55.0),
            ASize::Medium => (45.0, 95.0),
            ASize::Small  => (80.0, 150.0),
        }
    }
    fn points(self) -> i32 {
        match self { ASize::Large => 20, ASize::Medium => 50, ASize::Small => 100 }
    }
    fn child(self) -> Option<ASize> {
        match self {
            ASize::Large  => Some(ASize::Medium),
            ASize::Medium => Some(ASize::Small),
            ASize::Small  => None,
        }
    }
}

#[derive(Copy, Clone)]
struct Ship {
    x: f32, y: f32,
    vx: f32, vy: f32,
    angle: f32,
    thrusting: bool,
}

#[derive(Copy, Clone)]
struct Bullet {
    active: bool,
    x: f32, y: f32,
    vx: f32, vy: f32,
    ttl: f32,
    from_player: bool,
}
blip::pooled!(Bullet, Asteroid, Debris);
const BULLET_OFF: Bullet = Bullet { active: false, x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, ttl: 0.0, from_player: true };

#[derive(Copy, Clone)]
struct Asteroid {
    active: bool,
    x: f32, y: f32,
    vx: f32, vy: f32,
    size: ASize,
    rot: f32,
    spin: f32,
    jag: [f32; 10],
}
const ASTEROID_OFF: Asteroid = Asteroid {
    active: false, x: 0.0, y: 0.0, vx: 0.0, vy: 0.0,
    size: ASize::Large, rot: 0.0, spin: 0.0, jag: [1.0; 10],
};

/// A tumbling line shard thrown off by an explosion; fades out over `ttl0`.
#[derive(Copy, Clone)]
struct Debris {
    active: bool,
    x: f32, y: f32,
    vx: f32, vy: f32,
    rot: f32, spin: f32,
    len: f32,
    ttl: f32, ttl0: f32,
    color: BlipColor,
}
const DEBRIS_OFF: Debris = Debris {
    active: false, x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, rot: 0.0, spin: 0.0,
    len: 0.0, ttl: 0.0, ttl0: 1.0, color: BLIP_WHITE,
};

#[derive(Copy, Clone)]
struct Saucer {
    active: bool,
    x: f32, y: f32,
    vx: f32,
    wave_t: f32,
    fire_t: f32,
    big: bool,
}

struct Game {
    state: State,
    ship: Ship,
    ship_alive: bool,
    fire_cd: f32,
    invuln_t: f32,
    respawn_t: Timer,
    bullets: [Bullet; MAX_BULLETS],
    asteroids: [Asteroid; MAX_ASTEROIDS],
    debris: [Debris; MAX_DEBRIS],
    saucer: Saucer,
    saucer_cd: f32,
    sess: Session,
    next_life_score: i32,
    /// Hits in a row without a shot going to waste (see STREAK_PER).
    streak: blip::Streak,
    /// Seconds since the current wave arrived, for its banner.
    wave_t: f32,
    fx: Fx,
    crystals: Vec<Crystal>,
    /// The screen shakes as the ship goes (see SHIP_SHAKE).
    shake: Shake,
}

impl Game {
    fn new() -> Self {
        Self {
            state: State::Title,
            ship: Ship { x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, angle: 0.0, thrusting: false },
            ship_alive: false,
            fire_cd: 0.0,
            invuln_t: 0.0,
            respawn_t: Timer::default(),
            bullets: [BULLET_OFF; MAX_BULLETS],
            asteroids: [ASTEROID_OFF; MAX_ASTEROIDS],
            debris: [DEBRIS_OFF; MAX_DEBRIS],
            saucer: Saucer { active: false, x: 0.0, y: 0.0, vx: 0.0, wave_t: 0.0, fire_t: 0.0, big: true },
            saucer_cd: 12.0,
            sess: Session::new(LIVES_START),
            next_life_score: EXTRA_LIFE_SCORE,
            streak: blip::Streak::new(STREAK_PER, STREAK_MAX),
            wave_t: 0.0,
            fx: Fx::new(),
            crystals: Vec::new(),
            shake: SHIP_SHAKE,
        }
    }

    fn start_wave(&mut self) {
        for a in self.asteroids.iter_mut() { a.active = false; }
        self.wave_t = 0.0;
        let count = (WAVE_BASE + self.sess.level - 1).min(WAVE_MAX);
        // Clear of the ship, wherever it is: a wave arrives mid-flight the
        // instant the last rock dies, and the centre may be nowhere near it.
        let (sx, sy) = if self.ship_alive { (self.ship.x, self.ship.y) } else { field_centre() };
        for _ in 0..count {
            let (x, y) = find_clear_spot(sx, sy);
            let (vx, vy) = random_velocity(ASize::Large);
            spawn_rock(self, x, y, ASize::Large, vx, vy);
        }
    }

    fn respawn_ship(&mut self) {
        self.ship = Ship {
            x: PLAY_W as f32 / 2.0,
            y: PLAY_Y0 + PLAY_H as f32 / 2.0,
            vx: 0.0, vy: 0.0,
            angle: 0.0,
            thrusting: false,
        };
        self.invuln_t = SHIP_INVULN;
        self.ship_alive = true;
    }

    fn start_game(&mut self) {
        self.sess.reset(LIVES_START);
        self.next_life_score = EXTRA_LIFE_SCORE;
        self.streak.miss();
        self.bullets = [BULLET_OFF; MAX_BULLETS];
        self.debris = [DEBRIS_OFF; MAX_DEBRIS];
        self.fx.clear();
        self.crystals.clear();
        self.saucer.active = false;
        let (lo, hi) = saucer_spawn_range(self.sess.level);
        self.saucer_cd = rand_range_f32(lo, hi);
        self.respawn_ship();
        self.start_wave();
        self.state = State::Play;
    }
}

blip::sounds!(Sounds { fire, thrust, crystal, bang_large, bang_medium, bang_small, saucer_big, saucer_small, ship_boom = "ship_explosion", hyperspace, extra_life,
    streak_up: [blip::BlipSound; 3] = [blip::sound!("streak2"), blip::sound!("streak3"), blip::sound!("streak4")] });

fn field_centre() -> (f32, f32) {
    (PLAY_W as f32 / 2.0, PLAY_Y0 + PLAY_H as f32 / 2.0)
}

/// `n` shards, up to `len` long, thrown from (`x`, `y`) at up to `speed`, carrying half the velocity of what broke.
#[allow(clippy::too_many_arguments)]
fn burst(g: &mut Game, x: f32, y: f32, vx: f32, vy: f32, n: usize, len: f32, speed: f32, color: BlipColor) {
    for _ in 0..n {
        let dir = rand_range_f32(0.0, PI * 2.0);
        let sp = rand_range_f32(0.25, 1.0) * speed;
        let ttl = rand_range_f32(0.5, 1.1);
        pool_spawn(&mut g.debris, Debris {
            active: true, x, y,
            vx: vx * 0.5 + dir.cos() * sp, vy: vy * 0.5 + dir.sin() * sp,
            rot: rand_range_f32(0.0, PI * 2.0), spin: rand_range_f32(-6.0, 6.0),
            len: len * rand_range_f32(0.5, 1.0), ttl, ttl0: ttl, color,
        });
    }
}

fn spawn_bullet(bullets: &mut [Bullet; MAX_BULLETS], x: f32, y: f32, vx: f32, vy: f32, from_player: bool) {
    pool_spawn(bullets, Bullet { active: true, x, y, vx, vy, ttl: BULLET_TTL, from_player });
}

/// Whether segment `from -> to` passes within `radius` of `centre`; this prevents fast bullets tunnelling through rocks.
fn wrapped_segment_circle_overlap(
    from: (f32, f32),
    to: (f32, f32),
    centre: (f32, f32),
    radius: f32,
) -> bool {
    for dx in [-PLAY_W as f32, 0.0, PLAY_W as f32] {
        for dy in [-PLAY_H as f32, 0.0, PLAY_H as f32] {
            if segment_circle_overlap(
                from.0, from.1, to.0, to.1,
                centre.0 + dx, centre.1 + dy, radius,
            ) {
                return true;
            }
        }
    }
    false
}

fn wrapped_motion_overlap(
    a: (f32, f32),
    b: (f32, f32),
    a_delta: (f32, f32),
    b_delta: (f32, f32),
    radius: f32,
) -> bool {
    let (w, h) = (PLAY_W as f32, PLAY_H as f32);
    let dx = (a.0 - b.0 + w * 0.5).rem_euclid(w) - w * 0.5;
    let dy = (a.1 - b.1 + h * 0.5).rem_euclid(h) - h * 0.5;
    let (vx, vy) = (a_delta.0 - b_delta.0, a_delta.1 - b_delta.1);
    let from = (dx - vx, dy - vy);
    for ox in [-w, 0.0, w] {
        for oy in [-h, 0.0, h] {
            if segment_circle_overlap(from.0, from.1, dx, dy, ox, oy, radius) {
                return true;
            }
        }
    }
    false
}

/// Where a jump lands: clear space, so the gamble is the 1-in-N blow-up, not
/// a rock, the saucer or a saucer's shot the player could not see.
fn landing_spot(g: &Game) -> (f32, f32) {
    let clear = |x: f32, y: f32| g.asteroids.iter().filter(|a| a.active).all(|a| {
        let (dx, dy) = (x - a.x, y - a.y);
        dx * dx + dy * dy > (a.size.radius() + SHIP_RADIUS + 40.0).powi(2)
    }) && (!g.saucer.active || (x - g.saucer.x).hypot(y - g.saucer.y) > saucer_radius(g.saucer.big) + SHIP_RADIUS + 60.0)
        // nor in the next half second of a saucer's shot
        && g.bullets.iter().filter(|b| b.active && !b.from_player)
            .all(|b| !wrapped_segment_circle_overlap((b.x, b.y), (b.x + b.vx * 0.5, b.y + b.vy * 0.5), (x, y), SHIP_RADIUS + 12.0));
    let mut spot = (0.0, 0.0);
    for _ in 0..16 {
        spot = (rand_range_f32(0.0, PLAY_W as f32), rand_range_f32(PLAY_Y0, PLAY_Y0 + PLAY_H as f32));
        if clear(spot.0, spot.1) { break; }
    }
    spot
}

fn hyperspace(g: &mut Game, sfx: &Sounds) {
    blip::bot::add("hyperspace", 1.0);
    play_sfx(&sfx.hyperspace);
    (g.ship.x, g.ship.y) = landing_spot(g);
    g.ship.vx = 0.0;
    g.ship.vy = 0.0;
    if rand_int(1, HYPERSPACE_RISK) == 1 {
        blip::bot::add("death_hyperspace", 1.0);
        kill_ship(g, sfx);
    }
}

/// The ship's loss shakes the screen 0.4 s, up to 6 px.
const SHIP_SHAKE: Shake = Shake::new(0.4, 6.0);

fn kill_ship(g: &mut Game, sfx: &Sounds) {
    g.shake.start();
    if !g.ship_alive { return; }
    play_sfx(&sfx.ship_boom);
    g.ship_alive = false;
    g.streak.miss();
    let (sx, sy, svx, svy) = (g.ship.x, g.ship.y, g.ship.vx, g.ship.vy);
    burst(g, sx, sy, svx, svy, 7, 12.0, 110.0, BLIP_SEA);
    g.state = match g.sess.lose_life_after(&mut g.respawn_t, RESPAWN_DELAY) {
        LifeResult::StillAlive => State::Dead,
        LifeResult::GameOver => {
            // A new best gets the extra-life jingle after the blast.
            if g.sess.beats_best() { play_sfx(&sfx.extra_life); }
            State::Over
        }
    };
}

fn award(g: &mut Game, sfx: &Sounds, pts: i32) {
    g.sess.add_score(pts);
    if g.sess.score >= g.next_life_score {
        g.sess.lives += 1;
        g.next_life_score += EXTRA_LIFE_SCORE;
        play_sfx(&sfx.extra_life);
        let (x, y) = if g.ship_alive { (g.ship.x, g.ship.y - 24.0) } else { field_centre() };
        g.fx.popup(x, y, "EXTRA LIFE", BLIP_SEA);
        g.fx.ring(x, y + 24.0, 60.0, 0.6, BLIP_SEA);
    }
}

fn update_title(g: &mut Game) {
    if coin_start() { g.start_game(); }
}

/// Turn, thrust, a little drag and the speed limit; the field wraps. Returns
/// how far the ship moved, across the seam if it wrapped.
fn fly_ship(g: &mut Game, dt: f32, sfx: &Sounds, thrust_snd_t: &mut f32) -> (f32, f32) {
    let before = (g.ship.x, g.ship.y);
    if Stick::Left.active() { g.ship.angle -= SHIP_TURN_RATE * dt; }
    if Stick::Right.active() { g.ship.angle += SHIP_TURN_RATE * dt; }

    g.ship.thrusting = Stick::Up.active();
    if g.ship.thrusting {
        g.ship.vx += g.ship.angle.sin() * SHIP_ACCEL * dt;
        g.ship.vy -= g.ship.angle.cos() * SHIP_ACCEL * dt;
        *thrust_snd_t -= dt;
        if *thrust_snd_t <= 0.0 {
            play_sfx(&sfx.thrust);
            *thrust_snd_t = 0.18;
        }
    }
    // Light drag; a frictionless hull is unforgiving to steer.
    let drag = (-0.35 * dt).exp();
    g.ship.vx *= drag;
    g.ship.vy *= drag;
    let speed = (g.ship.vx * g.ship.vx + g.ship.vy * g.ship.vy).sqrt();
    if speed > SHIP_MAX_SPEED {
        let s = SHIP_MAX_SPEED / speed;
        g.ship.vx *= s;
        g.ship.vy *= s;
    }
    g.ship.x = wrap(g.ship.x + g.ship.vx * dt, 0.0, PLAY_W as f32);
    g.ship.y = wrap(g.ship.y + g.ship.vy * dt, PLAY_Y0, PLAY_Y0 + PLAY_H as f32);
    (
        (g.ship.x - before.0 + PLAY_W as f32 * 0.5).rem_euclid(PLAY_W as f32) - PLAY_W as f32 * 0.5,
        (g.ship.y - before.1 + PLAY_H as f32 * 0.5).rem_euclid(PLAY_H as f32) - PLAY_H as f32 * 0.5,
    )
}

fn fire_shot(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.fire_cd -= dt;
    if key_active(BLIP_KEY_SPACE) && g.fire_cd <= 0.0 {
        let vx = g.ship.angle.sin() * BULLET_SPEED + g.ship.vx * 0.3;
        let vy = -g.ship.angle.cos() * BULLET_SPEED + g.ship.vy * 0.3;
        spawn_bullet(&mut g.bullets, g.ship.x, g.ship.y, vx, vy, true);
        g.fire_cd = FIRE_COOLDOWN;
        play_sfx(&sfx.fire);
    }
}

fn update_play(g: &mut Game, dt: f32, sfx: &mut Sounds, thrust_snd_t: &mut f32) {
    // At 0.6 volume: the chime comes often, and its jingle peaks as loud as a level clear.
    if let Some(m) = g.streak.risen() { blip::audio::play_sfx_volume(&sfx.streak_up[(m as usize - 2).min(2)], 0.6); }
    let mut ship = Sweep { from: (g.ship.x, g.ship.y), delta: (0.0, 0.0), jumped: false };
    if g.ship_alive {
        ship.delta = fly_ship(g, dt, sfx, thrust_snd_t);
        fire_shot(g, dt, sfx);
        if key_pressed(BLIP_KEY_BUTTON2) {
            hyperspace(g, sfx);
            ship.delta = (0.0, 0.0);
            ship.jumped = true;
        }
        if g.invuln_t > 0.0 { g.invuln_t -= dt; }
    }

    let rocks_before: [bool; MAX_ASTEROIDS] = std::array::from_fn(|i| g.asteroids[i].active);
    let saucer_before = g.saucer.active.then_some((g.saucer.x, g.saucer.y));
    let shot_starts: [Option<(f32, f32)>; MAX_BULLETS] =
        std::array::from_fn(|i| g.bullets[i].active.then_some((g.bullets[i].x, g.bullets[i].y)));
    update_world(g, dt, sfx);

    let open = |g: &Game| g.ship_alive && g.invuln_t <= 0.0;
    if open(g) { shot_down(g, dt, sfx, &ship, &shot_starts); }
    expire_enemy_bullets(g);
    if open(g) { rammed_by_rock(g, dt, sfx, &ship, &rocks_before); }
    if open(g) && g.saucer.active { rammed_by_saucer(g, sfx, &ship, saucer_before); }
}

/// How the ship moved this frame: from where, by how much, and whether it jumped through hyperspace.
struct Sweep { from: (f32, f32), delta: (f32, f32), jumped: bool }

/// A saucer's shot that meets the ship, swept with it through the frame (one fired this frame, or a ship just out of
/// hyperspace, from where they are now).
fn shot_down(g: &mut Game, dt: f32, sfx: &Sounds, ship: &Sweep, starts: &[Option<(f32, f32)>; MAX_BULLETS]) {
    for (bi, start) in starts.iter().enumerate() {
        let b = g.bullets[bi];
        if !b.active || b.from_player { continue; }
        let hit = match *start {
            Some(start) if !ship.jumped => {
                let from = (start.0 - ship.from.0, start.1 - ship.from.1);
                let to = (from.0 + b.vx * dt - ship.delta.0, from.1 + b.vy * dt - ship.delta.1);
                segment_circle_overlap(from.0, from.1, to.0, to.1, 0.0, 0.0, SHIP_RADIUS)
            }
            _ => segment_circle_overlap(b.x, b.y, b.x, b.y, g.ship.x, g.ship.y, SHIP_RADIUS),
        };
        if hit {
            g.bullets[bi].active = false;
            blip::bot::add("death_saucer_shot", 1.0);
            kill_ship(g, sfx);
            return;
        }
    }
}

/// A rock (one there when the frame began) that the ship flies into: it breaks up as the ship does.
fn rammed_by_rock(g: &mut Game, dt: f32, sfx: &Sounds, ship: &Sweep, before: &[bool; MAX_ASTEROIDS]) {
    for (ai, was) in before.iter().enumerate() {
        let a = g.asteroids[ai];
        if !*was || !a.active { continue; }
        if !wrapped_motion_overlap((g.ship.x, g.ship.y), (a.x, a.y), ship.delta, (a.vx * dt, a.vy * dt), a.size.radius() + SHIP_RADIUS) { continue; }
        g.asteroids[ai].active = false;
        let c = rock_color(&a, g.sess.level, blip::now());
        burst(g, a.x, a.y, a.vx, a.vy, 6, a.size.radius() * 0.5, 70.0, c);
        split_rock(g, a.x, a.y, a.size, a.vx, a.vy);
        blip::bot::add("death_rock", 1.0);
        kill_ship(g, sfx);
        return;
    }
}

/// The ship and the saucer collide, swept together through the frame (a saucer new this frame, where it is).
fn rammed_by_saucer(g: &mut Game, sfx: &Sounds, ship: &Sweep, before: Option<(f32, f32)>) {
    let r = saucer_radius(g.saucer.big) + SHIP_RADIUS;
    let (sx, sy) = (g.saucer.x, g.saucer.y);
    let hit = match before {
        Some(s0) => wrapped_motion_overlap((g.ship.x, g.ship.y), (sx, sy), ship.delta, (sx - s0.0, sy - s0.1), r),
        None => (g.ship.x - sx).powi(2) + (g.ship.y - sy).powi(2) <= r * r,
    };
    if !hit { return; }
    g.saucer.active = false;
    burst(g, sx, sy, g.saucer.vx, 0.0, 6, saucer_radius(g.saucer.big), 90.0, BLIP_HEART);
    blip::bot::add("death_saucer_ram", 1.0);
    kill_ship(g, sfx);
}

fn update_world(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.fx.update(dt);
    g.wave_t += dt;
    let rocks_at_start: [bool; MAX_ASTEROIDS] = std::array::from_fn(|i| g.asteroids[i].active);
    let saucer_start = g.saucer.active.then_some((g.saucer.x, g.saucer.y));
    let bullet_starts: [(f32, f32); MAX_BULLETS] = std::array::from_fn(|i| (g.bullets[i].x, g.bullets[i].y));
    let spent = move_bullets(g, dt);
    drift_field(g, dt);
    fly_saucer(g, dt);
    collect_crystals(g, dt, sfx);
    shoot_rocks(g, dt, sfx, &rocks_at_start);
    if g.saucer.active { shoot_saucer(g, dt, sfx, &bullet_starts, saucer_start); }

    for (i, &spent) in spent.iter().enumerate() {
        // A shot that runs out without hitting anything breaks the run.
        if spent && g.bullets[i].active { g.bullets[i].active = false; g.streak.miss(); }
    }

    if pool_iter(&g.asteroids).count() == 0 {
        g.sess.next_level();
        g.start_wave();
        // A wave cleared: a ring pulses out from the ship.
        if g.ship_alive {
            g.fx.ring(g.ship.x, g.ship.y, 160.0, 0.7, BLIP_SEA);
            g.fx.ring(g.ship.x, g.ship.y, 90.0, 0.5, BLIP_WHITE);
        }
    }
}

/// Every shot flies on; which of the ship's ran out (they may still hit something this frame).
fn move_bullets(g: &mut Game, dt: f32) -> [bool; MAX_BULLETS] {
    let mut spent = [false; MAX_BULLETS];
    for (i, b) in g.bullets.iter_mut().enumerate() {
        if !b.active { continue; }
        b.x += b.vx * dt;
        b.y += b.vy * dt;
        b.ttl -= dt;
        // The ship's shots wrap like everything else in the field, so a rock
        // across the edge can be hit; the saucer's leave, or they would come
        // back from behind with no warning (they go in expire_enemy_bullets).
        if b.from_player {
            b.x = wrap(b.x, 0.0, PLAY_W as f32);
            b.y = wrap(b.y, PLAY_Y0, PLAY_Y0 + PLAY_H as f32);
            spent[i] = b.ttl <= 0.0;
        }
    }
    spent
}

/// Crystals drift and fade; the ship flying through one takes its points.
fn collect_crystals(g: &mut Game, dt: f32, sfx: &Sounds) {
    for c in g.crystals.iter_mut() {
        c.x = wrap(c.x + c.vx * dt, 0.0, PLAY_W as f32);
        c.y = wrap(c.y + c.vy * dt, PLAY_Y0, PLAY_Y0 + PLAY_H as f32);
        c.ttl -= dt;
    }
    g.crystals.retain(|c| c.ttl > 0.0);
    if !g.ship_alive { return; }
    let (sx, sy) = (g.ship.x, g.ship.y);
    let mut taken = Vec::new();
    g.crystals.retain(|c| {
        let near = wrapped_segment_circle_overlap((sx, sy), (sx, sy), (c.x, c.y), SHIP_RADIUS + CRYSTAL_R);
        if near { taken.push((c.x, c.y)); }
        !near
    });
    for (x, y) in taken {
        play_sfx(&sfx.crystal);
        blip::bot::add("crystals", 1.0);
        g.fx.popup(x, y - 12.0, &format!("{CRYSTAL_POINTS}"), BLIP_SEA);
        g.fx.ring(x, y, 28.0, 0.35, BLIP_SEA);
        award(g, sfx, CRYSTAL_POINTS);
    }
}

fn expire_enemy_bullets(g: &mut Game) {
    for b in &mut g.bullets {
        if !b.from_player
            && (b.ttl <= 0.0 || b.x < 0.0 || b.x > PLAY_W as f32
                || b.y < PLAY_Y0 || b.y > PLAY_Y0 + PLAY_H as f32)
        {
            b.active = false;
        }
    }
}

fn update_dead(g: &mut Game, dt: f32, sfx: &Sounds) {
    update_world(g, dt, sfx);
    expire_enemy_bullets(g);
    if g.respawn_t.tick(dt) {
        let cx = PLAY_W as f32 / 2.0;
        let cy = PLAY_Y0 + PLAY_H as f32 / 2.0;
        let clear = pool_iter(&g.asteroids).all(|a| {
            let dx = a.x - cx;
            let dy = a.y - cy;
            dx * dx + dy * dy > SAFE_RADIUS * SAFE_RADIUS
        });
        if clear {
            g.respawn_ship();
            g.state = State::Play;
        } else {
            g.respawn_t.start(0.4);
        }
    }
}

fn update_over(g: &mut Game, dt: f32) {
    if play_again(&mut g.respawn_t, dt) { g.start_game(); }
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("METEORS", WIN_W, WIN_H)
}

#[blip::macroquad::main(conf)]
async fn main() {
    blip::bot::playthroughs(bot::PLAYTHROUGHS);
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();
    let blot = blip::load_png_smooth(blip::png!("soft_glow"));

    let title_art = [
        blip::png!("title_rock_large"),
        blip::png!("title_rock_medium"),
        blip::png!("title_ship"),
    ].map(blip::load_png);

    // DRIFT, BLOOM from wave BLOOM_WAVE (the flowers), then STORM from wave STORM_WAVE.
    let mut music = Jukebox::new("meteors", &["DRIFT", "BLOOM", "STORM"]);

    let mut sfx = Sounds::load().await;
    let mut thrust_snd_t = 0.0f32;

    if blip.screenshot_mode {
        g.start_game();
        // The card's picture: a late wave, flowers and haze and all.
        g.sess.level = 8;
        g.start_wave();
        g.wave_t = WAVE_BANNER_SECS;
        g.ship.angle = -0.4;
        g.ship.thrusting = true;
        g.invuln_t = 0.0;
        spawn_bullet(&mut g.bullets, g.ship.x, g.ship.y - 40.0, 0.0, -BULLET_SPEED, true);
    }

    loop {
        let dt = blip.delta_time;

        music.play(if g.state == State::Title { 0 }
            else if g.sess.level >= STORM_WAVE { 2 }
            else if g.sess.level >= BLOOM_WAVE { 1 }
            else { 0 });

        if blip::bot::active() { bot::drive(&g); }
        match g.state {
            State::Title => update_title(&mut g),
            State::Play  => update_play(&mut g, dt, &mut sfx, &mut thrust_snd_t),
            State::Dead  => update_dead(&mut g, dt, &sfx),
            State::Over  => update_over(&mut g, dt),
        }

        match g.state {
            State::Title => draw_title(&blip, &title_art),
            State::Over  => draw_over(&blip, g.sess.score, web::high_score(), g.respawn_t.active()),
            State::Play | State::Dead => blip.shaken(&g.shake, || draw_play(&blip, &g, &blot)),
        }
        g.shake.tick(dt);

        blip.next_frame().await;
    }
}

#[cfg(test)]
mod tests;
