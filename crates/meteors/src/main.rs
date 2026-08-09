//! Meteors, a tribute to the vector-graphics rock-shooter arcade classic, on macroquad.
//! Pure line-art rendering (no sprite assets) — true to the original's monochrome
//! vector display. Sound effects are synthesized beeps; the background music is a
//! procedurally generated techno loop (see `blip_assets::meteors`).

use std::f32::consts::PI;

use blip::input::{
    any_key_pressed, key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_D,
    BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::rand::gen_range;
use blip::{
    play_music, pool_iter, pool_iter_mut, pool_spawn, play_sfx, rand_int, web, window_conf, Blip,
    BlipColor, LifeResult, Pooled, Session, Timer, BLIP_BLACK, BLIP_GRAY, BLIP_WHITE, NEON_CYAN,
    NEON_ORANGE, NEON_PINK, NEON_PURPLE, NEON_YELLOW,
};

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
const WAVE_BASE:     i32 = 3;
const WAVE_MAX:      i32 = 14;

// ---- saucer -----------------------------------------------------------
const SAUCER_SPEED:    f32 = 90.0;
const SAUCER_FIRE_CD:  f32 = 1.4;
const SAUCER_SPAWN_MIN: f32 = 9.0;
const SAUCER_SPAWN_MAX: f32 = 18.0;

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
impl Pooled for Bullet { fn is_active(&self) -> bool { self.active } }
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
impl Pooled for Asteroid { fn is_active(&self) -> bool { self.active } }
const ASTEROID_OFF: Asteroid = Asteroid {
    active: false, x: 0.0, y: 0.0, vx: 0.0, vy: 0.0,
    size: ASize::Large, rot: 0.0, spin: 0.0, jag: [1.0; 10],
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
    saucer: Saucer,
    saucer_cd: f32,
    sess: Session,
    next_life_score: i32,
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
            saucer: Saucer { active: false, x: 0.0, y: 0.0, vx: 0.0, wave_t: 0.0, fire_t: 0.0, big: true },
            saucer_cd: 12.0,
            sess: Session::new(LIVES_START),
            next_life_score: EXTRA_LIFE_SCORE,
        }
    }

    fn spawn_wave(&mut self) {
        for a in self.asteroids.iter_mut() { a.active = false; }
        let count = (WAVE_BASE + self.sess.level - 1).min(WAVE_MAX);
        for _ in 0..count {
            let (x, y) = spawn_pos_away_from_center();
            spawn_asteroid(self, x, y, ASize::Large);
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
        self.bullets = [BULLET_OFF; MAX_BULLETS];
        self.saucer.active = false;
        let (lo, hi) = saucer_spawn_range(self.sess.level);
        self.saucer_cd = rand_range(lo, hi);
        self.spawn_wave();
        self.respawn_ship();
        self.state = State::Play;
    }
}

struct Sounds {
    fire:        blip::BlipSound,
    thrust:      blip::BlipSound,
    bang_large:  blip::BlipSound,
    bang_medium: blip::BlipSound,
    bang_small:  blip::BlipSound,
    saucer_big:  blip::BlipSound,
    saucer_small: blip::BlipSound,
    ship_boom:   blip::BlipSound,
    hyperspace:  blip::BlipSound,
    extra_life:  blip::BlipSound,
}

fn rand_range(lo: f32, hi: f32) -> f32 {
    if hi <= lo { return lo; }
    gen_range(lo, hi)
}

fn spawn_pos_away_from_center() -> (f32, f32) {
    let cx = PLAY_W as f32 / 2.0;
    let cy = PLAY_Y0 + PLAY_H as f32 / 2.0;
    loop {
        let x = rand_range(0.0, PLAY_W as f32);
        let y = rand_range(PLAY_Y0, PLAY_Y0 + PLAY_H as f32);
        let dx = x - cx;
        let dy = y - cy;
        if dx * dx + dy * dy > SAFE_RADIUS * SAFE_RADIUS {
            return (x, y);
        }
    }
}

fn spawn_asteroid(g: &mut Game, x: f32, y: f32, size: ASize) {
    let (smin, smax) = size.speed_range();
    let speed = rand_range(smin, smax);
    let dir = rand_range(0.0, PI * 2.0);
    let mut jag = [1.0f32; 10];
    for j in jag.iter_mut() { *j = rand_range(0.75, 1.25); }
    pool_spawn(&mut g.asteroids, Asteroid {
        active: true, x, y,
        vx: dir.cos() * speed, vy: dir.sin() * speed,
        size, rot: rand_range(0.0, PI * 2.0),
        spin: rand_range(-1.5, 1.5),
        jag,
    });
}

fn split_asteroid(g: &mut Game, x: f32, y: f32, size: ASize) {
    let Some(child) = size.child() else { return };
    for _ in 0..2 {
        let ox = rand_range(-6.0, 6.0);
        let oy = rand_range(-6.0, 6.0);
        spawn_asteroid(g, x + ox, y + oy, child);
    }
}

fn wrap(v: f32, lo: f32, hi: f32) -> f32 {
    let span = hi - lo;
    if v < lo { v + span } else if v >= hi { v - span } else { v }
}

fn spawn_bullet(bullets: &mut [Bullet; MAX_BULLETS], x: f32, y: f32, vx: f32, vy: f32, from_player: bool) {
    pool_spawn(bullets, Bullet { active: true, x, y, vx, vy, ttl: BULLET_TTL, from_player });
}

/// Saucers spawn more often, shoot more often, and skew toward the harder
/// "small" variant as the level climbs, so late levels keep escalating even
/// after the asteroid count itself caps out.
fn saucer_spawn_range(level: i32) -> (f32, f32) {
    let lv = (level - 1).min(6) as f32;
    ((SAUCER_SPAWN_MIN - lv * 0.6).max(4.0), (SAUCER_SPAWN_MAX - lv * 1.2).max(8.0))
}

fn saucer_fire_cooldown(level: i32) -> f32 {
    let lv = (level - 1).min(6) as f32;
    (SAUCER_FIRE_CD - lv * 0.12).max(0.7)
}

fn spawn_saucer(g: &mut Game) {
    let small_prob = (0.33 + (g.sess.level - 1) as f32 * 0.05).min(0.75);
    let big = g.sess.score < 5000 || rand_range(0.0, 1.0) > small_prob;
    let from_left = rand_int(0, 1) == 0;
    let x = if from_left { -20.0 } else { PLAY_W as f32 + 20.0 };
    let y = rand_range(PLAY_Y0 + 40.0, PLAY_Y0 + PLAY_H as f32 - 40.0);
    let vx = if from_left { SAUCER_SPEED } else { -SAUCER_SPEED };
    let fire_cd = saucer_fire_cooldown(g.sess.level);
    g.saucer = Saucer { active: true, x, y, vx, wave_t: 0.0, fire_t: fire_cd * 0.5, big };
}

fn hyperspace(g: &mut Game, sfx: &Sounds) {
    play_sfx(&sfx.hyperspace);
    g.ship.x = rand_range(0.0, PLAY_W as f32);
    g.ship.y = rand_range(PLAY_Y0, PLAY_Y0 + PLAY_H as f32);
    g.ship.vx = 0.0;
    g.ship.vy = 0.0;
    if rand_int(1, HYPERSPACE_RISK) == 1 {
        kill_ship(g, sfx);
    }
}

fn kill_ship(g: &mut Game, sfx: &Sounds) {
    if !g.ship_alive { return; }
    play_sfx(&sfx.ship_boom);
    g.ship_alive = false;
    match g.sess.lose_life() {
        LifeResult::StillAlive => { g.respawn_t.start(RESPAWN_DELAY); g.state = State::Dead; }
        LifeResult::GameOver   => { g.state = State::Over; }
    }
}

fn award(g: &mut Game, sfx: &Sounds, pts: i32) {
    g.sess.add_score(pts);
    if g.sess.score >= g.next_life_score {
        g.sess.lives += 1;
        g.next_life_score += EXTRA_LIFE_SCORE;
        play_sfx(&sfx.extra_life);
    }
}

fn update_title(g: &mut Game) {
    if any_key_pressed() { g.start_game(); }
}

fn update_play(g: &mut Game, dt: f32, sfx: &mut Sounds, thrust_snd_t: &mut f32) {
    if g.ship_alive {
        if key_held(BLIP_KEY_LEFT)  || key_held(BLIP_KEY_A) { g.ship.angle -= SHIP_TURN_RATE * dt; }
        if key_held(BLIP_KEY_RIGHT) || key_held(BLIP_KEY_D) { g.ship.angle += SHIP_TURN_RATE * dt; }

        g.ship.thrusting = key_held(BLIP_KEY_UP) || key_held(BLIP_KEY_W);
        if g.ship.thrusting {
            g.ship.vx += g.ship.angle.sin() * SHIP_ACCEL * dt;
            g.ship.vy -= g.ship.angle.cos() * SHIP_ACCEL * dt;
            *thrust_snd_t -= dt;
            if *thrust_snd_t <= 0.0 {
                play_sfx(&sfx.thrust);
                *thrust_snd_t = 0.18;
            }
        }
        let speed = (g.ship.vx * g.ship.vx + g.ship.vy * g.ship.vy).sqrt();
        if speed > SHIP_MAX_SPEED {
            let s = SHIP_MAX_SPEED / speed;
            g.ship.vx *= s;
            g.ship.vy *= s;
        }
        g.ship.x = wrap(g.ship.x + g.ship.vx * dt, 0.0, PLAY_W as f32);
        g.ship.y = wrap(g.ship.y + g.ship.vy * dt, PLAY_Y0, PLAY_Y0 + PLAY_H as f32);

        g.fire_cd -= dt;
        if key_held(BLIP_KEY_SPACE) && g.fire_cd <= 0.0 {
            let vx = g.ship.angle.sin() * BULLET_SPEED + g.ship.vx * 0.3;
            let vy = -g.ship.angle.cos() * BULLET_SPEED + g.ship.vy * 0.3;
            spawn_bullet(&mut g.bullets, g.ship.x, g.ship.y, vx, vy, true);
            g.fire_cd = FIRE_COOLDOWN;
            play_sfx(&sfx.fire);
        }
        if key_pressed(BLIP_KEY_BUTTON2) {
            hyperspace(g, sfx);
        }
        if g.invuln_t > 0.0 { g.invuln_t -= dt; }
    }

    update_world(g, dt, sfx);

    // ---- enemy bullets / asteroids / saucer vs ship ----
    if g.ship_alive && g.invuln_t <= 0.0 {
        for bi in 0..MAX_BULLETS {
            if !g.bullets[bi].active || g.bullets[bi].from_player { continue; }
            let dx = g.bullets[bi].x - g.ship.x;
            let dy = g.bullets[bi].y - g.ship.y;
            if dx * dx + dy * dy <= SHIP_RADIUS * SHIP_RADIUS {
                g.bullets[bi].active = false;
                kill_ship(g, sfx);
                break;
            }
        }
    }
    if g.ship_alive && g.invuln_t <= 0.0 {
        for ai in 0..MAX_ASTEROIDS {
            if !g.asteroids[ai].active { continue; }
            let r = g.asteroids[ai].size.radius();
            let dx = g.ship.x - g.asteroids[ai].x;
            let dy = g.ship.y - g.asteroids[ai].y;
            if dx * dx + dy * dy <= (r + SHIP_RADIUS) * (r + SHIP_RADIUS) {
                let (ax, ay, size) = (g.asteroids[ai].x, g.asteroids[ai].y, g.asteroids[ai].size);
                g.asteroids[ai].active = false;
                split_asteroid(g, ax, ay, size);
                kill_ship(g, sfx);
                break;
            }
        }
    }
    if g.ship_alive && g.invuln_t <= 0.0 && g.saucer.active {
        let r = if g.saucer.big { 16.0 } else { 9.0 };
        let dx = g.ship.x - g.saucer.x;
        let dy = g.ship.y - g.saucer.y;
        if dx * dx + dy * dy <= (r + SHIP_RADIUS) * (r + SHIP_RADIUS) {
            g.saucer.active = false;
            kill_ship(g, sfx);
        }
    }
}

fn update_world(g: &mut Game, dt: f32, sfx: &Sounds) {
    for b in pool_iter_mut(&mut g.bullets) {
        b.x += b.vx * dt;
        b.y += b.vy * dt;
        b.ttl -= dt;
        if b.ttl <= 0.0 || b.x < 0.0 || b.x > PLAY_W as f32 || b.y < PLAY_Y0 || b.y > PLAY_Y0 + PLAY_H as f32 {
            b.active = false;
        }
    }

    for a in pool_iter_mut(&mut g.asteroids) {
        a.x = wrap(a.x + a.vx * dt, 0.0, PLAY_W as f32);
        a.y = wrap(a.y + a.vy * dt, PLAY_Y0, PLAY_Y0 + PLAY_H as f32);
        a.rot += a.spin * dt;
    }

    if g.saucer.active {
        g.saucer.x += g.saucer.vx * dt;
        g.saucer.wave_t += dt;
        g.saucer.y += (g.saucer.wave_t * 2.0).sin() * 24.0 * dt;
        g.saucer.fire_t -= dt;
        if g.saucer.fire_t <= 0.0 && g.ship_alive {
            let dx = g.ship.x - g.saucer.x;
            let dy = g.ship.y - g.saucer.y;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let spread = if g.saucer.big { 0.35 } else { 0.04 };
            let jitter = rand_range(-spread, spread);
            let base = dy.atan2(dx) + jitter;
            let speed = 260.0;
            spawn_bullet(&mut g.bullets, g.saucer.x, g.saucer.y, base.cos() * speed, base.sin() * speed, false);
            let _ = dist;
            g.saucer.fire_t = saucer_fire_cooldown(g.sess.level);
        }
        if g.saucer.x < -40.0 || g.saucer.x > PLAY_W as f32 + 40.0 {
            g.saucer.active = false;
        }
    } else {
        g.saucer_cd -= dt;
        if g.saucer_cd <= 0.0 {
            spawn_saucer(g);
            let (lo, hi) = saucer_spawn_range(g.sess.level);
            g.saucer_cd = rand_range(lo, hi);
        }
    }

    // ---- collisions: player bullets vs asteroids ----
    for bi in 0..MAX_BULLETS {
        if !g.bullets[bi].active || !g.bullets[bi].from_player { continue; }
        let (bx, by) = (g.bullets[bi].x, g.bullets[bi].y);
        for ai in 0..MAX_ASTEROIDS {
            if !g.asteroids[ai].active { continue; }
            let r = g.asteroids[ai].size.radius();
            let dx = bx - g.asteroids[ai].x;
            let dy = by - g.asteroids[ai].y;
            if dx * dx + dy * dy <= r * r {
                g.bullets[bi].active = false;
                let (ax, ay, size) = (g.asteroids[ai].x, g.asteroids[ai].y, g.asteroids[ai].size);
                g.asteroids[ai].active = false;
                award(g, sfx, size.points());
                match size {
                    ASize::Large  => play_sfx(&sfx.bang_large),
                    ASize::Medium => play_sfx(&sfx.bang_medium),
                    ASize::Small  => play_sfx(&sfx.bang_small),
                }
                split_asteroid(g, ax, ay, size);
                break;
            }
        }
    }

    // ---- player bullets vs saucer ----
    if g.saucer.active {
        for bi in 0..MAX_BULLETS {
            if !g.bullets[bi].active || !g.bullets[bi].from_player { continue; }
            let dx = g.bullets[bi].x - g.saucer.x;
            let dy = g.bullets[bi].y - g.saucer.y;
            let r = if g.saucer.big { 16.0 } else { 9.0 };
            if dx * dx + dy * dy <= r * r {
                g.bullets[bi].active = false;
                g.saucer.active = false;
                award(g, sfx, if g.saucer.big { 200 } else { 1000 });
                if g.saucer.big { play_sfx(&sfx.saucer_big); } else { play_sfx(&sfx.saucer_small); }
                break;
            }
        }
    }

    if pool_iter(&g.asteroids).count() == 0 {
        g.sess.next_level();
        g.spawn_wave();
    }
}

fn update_dead(g: &mut Game, dt: f32, sfx: &Sounds) {
    update_world(g, dt, sfx);
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

fn update_over(g: &mut Game) {
    if !any_key_pressed() { return; }
    web::spend_coin();
    g.start_game();
}

fn draw_ship(blip: &Blip, ship: &Ship, invuln_t: f32, color: BlipColor) {
    if invuln_t > 0.0 && (invuln_t * 10.0) as i32 % 2 == 0 { return; }
    let a = ship.angle;
    let fwd = (a.sin(), -a.cos());
    let side = (a.cos(), a.sin());
    let nose = (ship.x + fwd.0 * 14.0, ship.y + fwd.1 * 14.0);
    let left = (ship.x - fwd.0 * 10.0 - side.0 * 9.0, ship.y - fwd.1 * 10.0 - side.1 * 9.0);
    let right = (ship.x - fwd.0 * 10.0 + side.0 * 9.0, ship.y - fwd.1 * 10.0 + side.1 * 9.0);
    let tail = (ship.x - fwd.0 * 4.0, ship.y - fwd.1 * 4.0);
    blip.draw_glow_line(nose.0, nose.1, left.0, left.1, color);
    blip.draw_glow_line(left.0, left.1, tail.0, tail.1, color);
    blip.draw_glow_line(tail.0, tail.1, right.0, right.1, color);
    blip.draw_glow_line(right.0, right.1, nose.0, nose.1, color);

    if ship.thrusting {
        let flick = rand_range(0.5, 1.0);
        let flame = (ship.x - fwd.0 * (10.0 + 10.0 * flick), ship.y - fwd.1 * (10.0 + 10.0 * flick));
        blip.draw_glow_line(left.0, left.1, flame.0, flame.1, NEON_ORANGE);
        blip.draw_glow_line(right.0, right.1, flame.0, flame.1, NEON_ORANGE);
    }
}

fn draw_asteroid(blip: &Blip, a: &Asteroid) {
    let n = a.jag.len();
    let r = a.size.radius();
    let mut prev: Option<(f32, f32)> = None;
    let first_pt = {
        let ang = a.rot;
        let rr = r * a.jag[0];
        (a.x + ang.cos() * rr, a.y + ang.sin() * rr)
    };
    for i in 0..=n {
        let idx = i % n;
        let ang = a.rot + (idx as f32 / n as f32) * PI * 2.0;
        let rr = r * a.jag[idx];
        let pt = (a.x + ang.cos() * rr, a.y + ang.sin() * rr);
        if let Some(p) = prev {
            blip.draw_glow_line(p.0, p.1, pt.0, pt.1, NEON_PURPLE);
        }
        prev = Some(pt);
    }
    let _ = first_pt;
}

fn draw_saucer(blip: &Blip, s: &Saucer) {
    let r = if s.big { 16.0 } else { 9.0 };
    let w = r * 2.6;
    let dw = r * 1.1;
    let h = r * 0.5;
    let (x, y) = (s.x, s.y);
    let pts = [
        (x - w / 2.0, y),
        (x - dw / 2.0, y - h),
        (x + dw / 2.0, y - h),
        (x + w / 2.0, y),
        (x + dw / 2.0, y + h),
        (x - dw / 2.0, y + h),
    ];
    for i in 0..pts.len() {
        let p0 = pts[i];
        let p1 = pts[(i + 1) % pts.len()];
        blip.draw_glow_line(p0.0, p0.1, p1.0, p1.1, NEON_PINK);
    }
}

/// A receding synthwave horizon grid — used only on the title/game-over menu
/// screens so the vector gameplay itself stays clean and readable.
fn draw_horizon_grid(blip: &Blip, y0: f32, y1: f32) {
    let cx = WIN_W as f32 / 2.0;
    let rows = 7;
    for i in 0..=rows {
        let t = i as f32 / rows as f32;
        let y = y0 + (y1 - y0) * t * t; // ease toward the horizon
        let half_w = cx * (0.15 + 0.85 * t);
        let a = 0.10 + 0.35 * t;
        let c = BlipColor { a, ..NEON_PURPLE };
        blip.draw_line(cx - half_w, y, cx + half_w, y, c);
    }
    let verges = [-1.0_f32, -0.5, 0.5, 1.0];
    for v in verges {
        let top = (cx + cx * 0.15 * v, y0);
        let bot = (cx + cx * v, y1);
        blip.draw_line(top.0, top.1, bot.0, bot.1, BlipColor { a: 0.20, ..NEON_PURPLE });
    }
}

fn draw_play(blip: &Blip, g: &Game) {
    blip.clear(BLIP_BLACK);
    for a in pool_iter(&g.asteroids) { draw_asteroid(blip, a); }
    if g.saucer.active { draw_saucer(blip, &g.saucer); }
    for b in pool_iter(&g.bullets) {
        let c = if b.from_player { NEON_YELLOW } else { NEON_PINK };
        blip.fill_glow_circle(b.x, b.y, 2.0, c);
    }
    if g.ship_alive {
        draw_ship(blip, &g.ship, g.invuln_t, NEON_CYAN);
    }
    blip.draw_hud(g.sess.score, g.sess.lives);
    let lvl = format!("LEVEL {}", g.sess.level);
    blip.draw_text(&lvl, 4.0, WIN_H as f32 - 18.0, 1.5, BLIP_GRAY);
}

fn draw_title(blip: &Blip) {
    blip.clear(BLIP_BLACK);
    draw_horizon_grid(blip, (WIN_H / 3) as f32, WIN_H as f32);
    blip.draw_centered("METEORS", (WIN_H / 4) as f32, 6.0, NEON_CYAN);
    blip.draw_centered("PRESS ANY KEY", (WIN_H / 2) as f32, 3.0, NEON_YELLOW);
    blip.draw_centered("ARROWS/WASD ROTATE+THRUST", (WIN_H * 2 / 3) as f32, 2.0, BLIP_GRAY);
    blip.draw_centered("SPACE FIRE  ·  Z HYPERSPACE", (WIN_H * 2 / 3) as f32 + 24.0, 2.0, BLIP_GRAY);
}

fn draw_over(blip: &Blip, score: i32) {
    let buf = format!("SCORE {score}");
    blip.clear(BLIP_BLACK);
    draw_horizon_grid(blip, (WIN_H / 3) as f32, WIN_H as f32);
    blip.draw_centered("GAME OVER", (WIN_H / 4) as f32, 5.0, NEON_PINK);
    blip.draw_centered(&buf, (WIN_H / 2) as f32, 3.0, BLIP_WHITE);
    blip.draw_centered("PRESS ANY KEY", (WIN_H * 2 / 3) as f32, 3.0, NEON_YELLOW);
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("METEORS", WIN_W, WIN_H)
}

const TECHNO_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/techno.wav"));
const FIRE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/fire.wav"));
const SHIP_EXPLOSION_WAV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/ship_explosion.wav"));

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let techno = blip::audio::load_sound(TECHNO_WAV).await;
    play_music(&techno);

    let mut sfx = Sounds {
        fire:         blip::audio::load_sound(FIRE_WAV).await,
        thrust:       blip::audio::beep(90.0, 90.0).await,
        bang_large:   blip::audio::beep(120.0, 190.0).await,
        bang_medium:  blip::audio::beep(220.0, 150.0).await,
        bang_small:   blip::audio::beep(380.0, 110.0).await,
        saucer_big:   blip::audio::beep(200.0, 300.0).await,
        saucer_small: blip::audio::beep(520.0, 220.0).await,
        ship_boom:    blip::audio::load_sound(SHIP_EXPLOSION_WAV).await,
        hyperspace:   blip::audio::beep(1300.0, 80.0).await,
        extra_life:   blip::audio::beep(880.0, 220.0).await,
    };
    let mut thrust_snd_t = 0.0f32;

    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;

        if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 {
                g.start_game();
                g.ship.angle = -0.4;
                g.ship.thrusting = true;
                g.invuln_t = 0.0;
                spawn_bullet(&mut g.bullets, g.ship.x, g.ship.y - 40.0, 0.0, -BULLET_SPEED, true);
            }
        }

        match g.state {
            State::Title => update_title(&mut g),
            State::Play  => update_play(&mut g, dt, &mut sfx, &mut thrust_snd_t),
            State::Dead  => update_dead(&mut g, dt, &sfx),
            State::Over  => update_over(&mut g),
        }

        match g.state {
            State::Title => draw_title(&blip),
            State::Over  => draw_over(&blip, g.sess.score),
            State::Play | State::Dead => draw_play(&blip, &g),
        }

        blip.next_frame(60).await;
    }
}
