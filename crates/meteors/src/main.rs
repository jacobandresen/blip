//! Meteors, a tribute to the vector-graphics rock-shooter arcade classic, on macroquad.
//! Pure line-art rendering (no sprite assets), true to the original's monochrome
//! vector display. Music and effects ring on glass (see `blip_assets::meteors`).

use std::f32::consts::PI;

use blip::input::{
    btn1_pressed, key_active, key_pressed, BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_D,
    BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::rand::gen_range;
use blip::macroquad::texture::{FilterMode, Texture2D};
use blip::{
    GAME_OVER_MIN_WAIT, hsv,
    Jukebox, pool_iter, pool_iter_mut, pool_spawn, play_sfx, rand_int, web, window_conf, Blip, Fx,
    BlipColor, LifeResult, Pooled, Session, Timer, BLIP_BLACK, BLIP_GRAY, BLIP_WHITE, NEON_CYAN,
    NEON_GREEN, NEON_ORANGE, NEON_PINK, NEON_PURPLE, NEON_YELLOW,
};

#[cfg(not(target_arch = "wasm32"))]
mod bot;

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
const MAX_DEBRIS:    usize = 96;
const WAVE_BASE:     i32 = 3;
const WAVE_MAX:      i32 = 14;

// ---- saucer -----------------------------------------------------------
const SAUCER_SPEED:    f32 = 90.0;
const SAUCER_FIRE_CD:  f32 = 1.4;
const SAUCER_SPAWN_MIN: f32 = 9.0;
const SAUCER_SPAWN_MAX: f32 = 18.0;
/// How long before each shot the saucer's core lights up as a warning.
const SAUCER_TELL: f32 = 0.4;
const WAVE_BANNER_SECS: f32 = 1.6;
const STORM_WAVE: i32 = 5;

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
/// The wave by which the haze is at its fullest, the black nearly gone.
const HAZE_FULL_WAVE: i32 = 12;
/// A patch's alpha at its fullest, on wave 1 and on the full wave.
const HAZE_ALPHA:     f32 = 0.05;
const HAZE_ALPHA_MAX: f32 = 0.42;
/// The same for the mushroom at its centre.
const SHROOM_ALPHA:     f32 = 0.04;
const SHROOM_ALPHA_MAX: f32 = 0.50;
/// Threads join the mushrooms from this wave, and reach for the rocks from
/// the second; their alpha at first and `THREAD_WAVES` waves on.
const THREAD_WAVE:      i32 = 3;
const THREAD_ROCK_WAVE: i32 = 6;
const THREAD_WAVES:     i32 = 8;
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
impl Pooled for Debris { fn is_active(&self) -> bool { self.active } }
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
    /// Seconds since the current wave arrived, for its banner.
    wave_t: f32,
    fx: Fx,
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
            wave_t: 0.0,
            fx: Fx::new(),
        }
    }

    fn spawn_wave(&mut self) {
        for a in self.asteroids.iter_mut() { a.active = false; }
        self.wave_t = 0.0;
        let count = (WAVE_BASE + self.sess.level - 1).min(WAVE_MAX);
        // Clear of the ship, wherever it is: a wave arrives mid-flight the
        // instant the last rock dies, and the centre may be nowhere near it.
        let (sx, sy) = if self.ship_alive { (self.ship.x, self.ship.y) } else { field_centre() };
        for _ in 0..count {
            let (x, y) = spawn_pos_away_from(sx, sy);
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
        self.debris = [DEBRIS_OFF; MAX_DEBRIS];
        self.fx.clear();
        self.saucer.active = false;
        let (lo, hi) = saucer_spawn_range(self.sess.level);
        self.saucer_cd = rand_range(lo, hi);
        self.respawn_ship();
        self.spawn_wave();
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

fn field_centre() -> (f32, f32) {
    (PLAY_W as f32 / 2.0, PLAY_Y0 + PLAY_H as f32 / 2.0)
}

/// A point at least SAFE_RADIUS from (cx, cy), measured across the wrap.
fn spawn_pos_away_from(cx: f32, cy: f32) -> (f32, f32) {
    let (w, h) = (PLAY_W as f32, PLAY_H as f32);
    loop {
        let x = rand_range(0.0, w);
        let y = rand_range(PLAY_Y0, PLAY_Y0 + h);
        let dx = (x - cx).abs().min(w - (x - cx).abs());
        let dy = (y - cy).abs().min(h - (y - cy).abs());
        if dx * dx + dy * dy > SAFE_RADIUS * SAFE_RADIUS {
            return (x, y);
        }
    }
}

fn spawn_asteroid(g: &mut Game, x: f32, y: f32, size: ASize) {
    let (smin, smax) = size.speed_range();
    let speed = rand_range(smin, smax);
    let dir = rand_range(0.0, PI * 2.0);
    spawn_asteroid_v(g, x, y, size, dir.cos() * speed, dir.sin() * speed);
}

fn spawn_asteroid_v(g: &mut Game, x: f32, y: f32, size: ASize, vx: f32, vy: f32) {
    let mut jag = [1.0f32; 10];
    for j in jag.iter_mut() { *j = rand_range(0.75, 1.25); }
    pool_spawn(&mut g.asteroids, Asteroid {
        active: true, x, y,
        vx, vy,
        size, rot: rand_range(0.0, PI * 2.0),
        spin: rand_range(-1.5, 1.5),
        jag,
    });
}

/// Fragments leave along opposite sides of a random axis, on top of the
/// parent's velocity (plus a share of the impactor's), so momentum carries through.
fn split_asteroid(g: &mut Game, x: f32, y: f32, size: ASize, pvx: f32, pvy: f32) {
    let Some(child) = size.child() else { return };
    let axis = rand_range(0.0, PI * 2.0);
    let (ax, ay) = (axis.cos(), axis.sin());
    let (smin, smax) = child.speed_range();
    for sign in [1.0_f32, -1.0] {
        let kick = rand_range(smin, smax) * 0.6;
        spawn_asteroid_v(g, x + ax * sign * 6.0, y + ay * sign * 6.0, child,
            pvx + ax * sign * kick, pvy + ay * sign * kick);
    }
}

fn burst(g: &mut Game, x: f32, y: f32, vx: f32, vy: f32, n: usize, len: f32, speed: f32, color: BlipColor) {
    for _ in 0..n {
        let dir = rand_range(0.0, PI * 2.0);
        let sp = rand_range(0.25, 1.0) * speed;
        let ttl = rand_range(0.5, 1.1);
        pool_spawn(&mut g.debris, Debris {
            active: true, x, y,
            vx: vx * 0.5 + dir.cos() * sp, vy: vy * 0.5 + dir.sin() * sp,
            rot: rand_range(0.0, PI * 2.0), spin: rand_range(-6.0, 6.0),
            len: len * rand_range(0.5, 1.0), ttl, ttl0: ttl, color,
        });
    }
}

fn wrap(v: f32, lo: f32, hi: f32) -> f32 {
    let span = hi - lo;
    if v < lo { v + span } else if v >= hi { v - span } else { v }
}

/// Did the segment `p0 -> p1` pass within `r` of `(cx, cy)` at any point? Used
/// for bullet hits: a fast bullet covers several of its own widths per frame,
/// so a point test at the frame's end lets it punch straight through a small
/// rock. Testing the whole swept path closes that gap.
fn seg_hits_circle(x0: f32, y0: f32, x1: f32, y1: f32, cx: f32, cy: f32, r: f32) -> bool {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 1e-6 {
        (((cx - x0) * dx + (cy - y0) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (ex, ey) = (cx - (x0 + t * dx), cy - (y0 + t * dy));
    ex * ex + ey * ey <= r * r
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
    let (sx, sy, svx, svy) = (g.ship.x, g.ship.y, g.ship.vx, g.ship.vy);
    burst(g, sx, sy, svx, svy, 7, 12.0, 110.0, NEON_CYAN);
    match g.sess.lose_life() {
        LifeResult::StillAlive => { g.respawn_t.start(RESPAWN_DELAY); g.state = State::Dead; }
        LifeResult::GameOver   => {
            g.respawn_t.start(GAME_OVER_MIN_WAIT);
            g.state = State::Over;
            web::report_score(g.sess.score);
        }
    }
}

fn award(g: &mut Game, sfx: &Sounds, pts: i32) {
    g.sess.add_score(pts);
    if g.sess.score >= g.next_life_score {
        g.sess.lives += 1;
        g.next_life_score += EXTRA_LIFE_SCORE;
        play_sfx(&sfx.extra_life);
        let (x, y) = if g.ship_alive { (g.ship.x, g.ship.y - 24.0) } else { field_centre() };
        g.fx.popup(x, y, "EXTRA LIFE", NEON_CYAN);
        g.fx.ring(x, y + 24.0, 60.0, 0.6, NEON_CYAN);
    }
}

fn update_title(g: &mut Game) {
    if btn1_pressed() {
        web::spend_coin();
        g.start_game();
    }
}

fn update_play(g: &mut Game, dt: f32, sfx: &mut Sounds, thrust_snd_t: &mut f32) {
    if g.ship_alive {
        if key_active(BLIP_KEY_LEFT)  || key_active(BLIP_KEY_A) { g.ship.angle -= SHIP_TURN_RATE * dt; }
        if key_active(BLIP_KEY_RIGHT) || key_active(BLIP_KEY_D) { g.ship.angle += SHIP_TURN_RATE * dt; }

        g.ship.thrusting = key_active(BLIP_KEY_UP) || key_active(BLIP_KEY_W);
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

        g.fire_cd -= dt;
        if key_active(BLIP_KEY_SPACE) && g.fire_cd <= 0.0 {
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
            let (bx, by) = (g.bullets[bi].x, g.bullets[bi].y);
            let (px, py) = (bx - g.bullets[bi].vx * dt, by - g.bullets[bi].vy * dt);
            if seg_hits_circle(px, py, bx, by, g.ship.x, g.ship.y, SHIP_RADIUS) {
                g.bullets[bi].active = false;
                blip::bot::add("death_saucer_shot", 1.0);
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
                let a = g.asteroids[ai];
                g.asteroids[ai].active = false;
                let c = rock_color(&a, g.sess.level, blip::macroquad::time::get_time() as f32);
                burst(g, a.x, a.y, a.vx, a.vy, 6, a.size.radius() * 0.5, 70.0, c);
                split_asteroid(g, a.x, a.y, a.size, a.vx, a.vy);
                blip::bot::add("death_rock", 1.0);
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
            let (sx, sy) = (g.saucer.x, g.saucer.y);
            burst(g, sx, sy, g.saucer.vx, 0.0, 6, r, 90.0, NEON_PINK);
            blip::bot::add("death_saucer_ram", 1.0);
            kill_ship(g, sfx);
        }
    }
}

fn update_world(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.fx.update(dt);
    g.wave_t += dt;
    for b in pool_iter_mut(&mut g.bullets) {
        b.x += b.vx * dt;
        b.y += b.vy * dt;
        b.ttl -= dt;
        if b.ttl <= 0.0 || b.x < 0.0 || b.x > PLAY_W as f32 || b.y < PLAY_Y0 || b.y > PLAY_Y0 + PLAY_H as f32 {
            b.active = false;
        }
    }

    for d in pool_iter_mut(&mut g.debris) {
        d.x += d.vx * dt;
        d.y += d.vy * dt;
        d.rot += d.spin * dt;
        d.ttl -= dt;
        if d.ttl <= 0.0 { d.active = false; }
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
            let spread = if g.saucer.big { 0.35 } else { 0.04 };
            let jitter = rand_range(-spread, spread);
            let base = dy.atan2(dx) + jitter;
            let speed = 260.0;
            spawn_bullet(&mut g.bullets, g.saucer.x, g.saucer.y, base.cos() * speed, base.sin() * speed, false);
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
        let (px, py) = (bx - g.bullets[bi].vx * dt, by - g.bullets[bi].vy * dt);
        for ai in 0..MAX_ASTEROIDS {
            if !g.asteroids[ai].active { continue; }
            let r = g.asteroids[ai].size.radius();
            if seg_hits_circle(px, py, bx, by, g.asteroids[ai].x, g.asteroids[ai].y, r) {
                g.bullets[bi].active = false;
                let a = g.asteroids[ai];
                let size = a.size;
                // A share of the bullet's momentum goes into the fragments.
                let (pvx, pvy) = (a.vx + g.bullets[bi].vx * 0.04, a.vy + g.bullets[bi].vy * 0.04);
                g.asteroids[ai].active = false;
                let c = rock_color(&a, g.sess.level, blip::macroquad::time::get_time() as f32);
                burst(g, a.x, a.y, a.vx, a.vy, 5, size.radius() * 0.5, 60.0, c);
                award(g, sfx, size.points());
                match size {
                    ASize::Large  => play_sfx(&sfx.bang_large),
                    ASize::Medium => play_sfx(&sfx.bang_medium),
                    ASize::Small  => play_sfx(&sfx.bang_small),
                }
                split_asteroid(g, a.x, a.y, size, pvx, pvy);
                break;
            }
        }
    }

    // ---- player bullets vs saucer ----
    if g.saucer.active {
        for bi in 0..MAX_BULLETS {
            if !g.bullets[bi].active || !g.bullets[bi].from_player { continue; }
            let (bx, by) = (g.bullets[bi].x, g.bullets[bi].y);
            let (px, py) = (bx - g.bullets[bi].vx * dt, by - g.bullets[bi].vy * dt);
            let r = if g.saucer.big { 16.0 } else { 9.0 };
            if seg_hits_circle(px, py, bx, by, g.saucer.x, g.saucer.y, r) {
                g.bullets[bi].active = false;
                g.saucer.active = false;
                let (sx, sy, svx) = (g.saucer.x, g.saucer.y, g.saucer.vx);
                burst(g, sx, sy, svx, 0.0, 6, r, 90.0, NEON_PINK);
                let pts = if g.saucer.big { 200 } else { 1000 };
                g.fx.popup(sx, sy - 14.0, &format!("{pts}"), NEON_YELLOW);
                award(g, sfx, pts);
                if g.saucer.big { play_sfx(&sfx.saucer_big); } else { play_sfx(&sfx.saucer_small); }
                break;
            }
        }
    }

    if pool_iter(&g.asteroids).count() == 0 {
        blip::bot::set(&format!("t_wave{}", g.sess.level), blip::bot::clock() as f64);
        g.sess.next_level();
        blip::bot::set("level", g.sess.level as f64);
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

fn update_over(g: &mut Game, dt: f32) {
    g.respawn_t.tick(dt);
    if g.respawn_t.active() || !btn1_pressed() { return; }
    web::spend_coin();
    g.start_game();
}

fn draw_ship(blip: &Blip, ship: &Ship, invuln_t: f32, level: i32, color: BlipColor) {
    if invuln_t > 0.0 && (invuln_t * 10.0) as i32 % 2 == 0 { return; }
    let a = ship.angle;
    let fwd = (a.sin(), -a.cos());
    let side = (a.cos(), a.sin());
    let nose = (ship.x + fwd.0 * 14.0, ship.y + fwd.1 * 14.0);
    let left = (ship.x - fwd.0 * 10.0 - side.0 * 9.0, ship.y - fwd.1 * 10.0 - side.1 * 9.0);
    let right = (ship.x - fwd.0 * 10.0 + side.0 * 9.0, ship.y - fwd.1 * 10.0 + side.1 * 9.0);
    let tail = (ship.x - fwd.0 * 4.0, ship.y - fwd.1 * 4.0);
    let hull = [(nose, left), (left, tail), (tail, right), (right, nose)];
    for (back, age) in smear_echoes(level) {
        let (dx, dy) = (ship.vx * back, ship.vy * back);
        for (p, q) in hull {
            smear_line(blip, (p, q), ((p.0 - dx, p.1 - dy), (q.0 - dx, q.1 - dy)), age, color);
        }
    }
    for (p, q) in hull { blip.draw_glow_line(p.0, p.1, q.0, q.1, color); }

    if ship.thrusting {
        let flick = rand_range(0.5, 1.0);
        let flame = (ship.x - fwd.0 * (10.0 + 10.0 * flick), ship.y - fwd.1 * (10.0 + 10.0 * flick));
        blip.draw_glow_line(left.0, left.1, flame.0, flame.1, NEON_ORANGE);
        blip.draw_glow_line(right.0, right.1, flame.0, flame.1, NEON_ORANGE);
    }
}

// ---- the trip -------------------------------------------------------------
// Each wave gets a little more psychedelic: moving lines smear further, the
// rocks take on more colours, and from wave 3 more and more of them are
// flowers.

type Seg = ((f32, f32), (f32, f32));

/// The echoes a moving line leaves, oldest first: how many seconds back each
/// one stands, and how bright it is for its age. Both grow with the wave.
fn smear_echoes(level: i32) -> impl Iterator<Item = (f32, f32)> {
    let n = (1 + level).min(SMEAR_ECHOES);
    let reach = (SMEAR_REACH * (1 + level) as f32).min(SMEAR_REACH_MAX);
    (1..=n).rev().map(move |e| (reach * e as f32 / n as f32, 1.0 - (e - 1) as f32 / n as f32))
}

/// One echo of a line: where it was `then`, lit only by how far it has come
/// since. What moves fast streaks; what stands still stays a sharp line.
fn smear_line(blip: &Blip, now: Seg, then: Seg, age: f32, c: BlipColor) {
    let dist = |a: (f32, f32), b: (f32, f32)| ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    let moved = (dist(now.0, then.0) + dist(now.1, then.1)) * 0.5;
    if moved < SMEAR_STILL { return; }
    let lit = ((moved - SMEAR_STILL) / SMEAR_FULL).min(1.0) * age;
    blip.draw_line_ex(then.0.0, then.0.1, then.1.0, then.1.1, 1.0, BlipColor { a: SMEAR_ALPHA * lit, ..c });
}

/// A rock's own fixed random number, 0..1, from its shape.
fn rock_seed(a: &Asteroid, k: usize) -> f32 { (a.jag[k] * 7.13).fract() }

/// A rock's colour: wave 1's purple, fanning out round the rainbow wave by
/// wave, and drifting from wave 4.
fn rock_color(a: &Asteroid, level: i32, t: f32) -> BlipColor {
    if level <= 1 { return NEON_PURPLE; }
    let spread = (0.18 * (level - 1) as f32).min(1.0);
    let drift = if level >= 4 { t * 0.04 * (level - 3) as f32 } else { 0.0 };
    hsv(0.76 + (rock_seed(a, 3) - 0.5) * spread + drift, 0.75, 1.0, 1.0)
}

/// From wave 3 a growing share of the rocks are flowers: a quarter, half,
/// three quarters, then nine in ten.
fn is_flower(a: &Asteroid, level: i32) -> bool {
    rock_seed(a, 5) < ((level - 2) as f32 * 0.25).clamp(0.0, 0.9)
}

/// A rock's outline round its own centre, as it is turned now: a flower's
/// petals (the same size as the rock it stands for) or the jagged ring.
fn outline(a: &Asteroid, flower: bool) -> Vec<Seg> {
    let r = a.size.radius();
    if flower {
        let petals = 5 + (rock_seed(a, 1) * 3.0) as usize;
        (0..petals * 10).map(|i| {
            let (uy, ux) = (a.rot + (i / 10) as f32 / petals as f32 * PI * 2.0).sin_cos();
            let pt = |k: usize| {
                let (ty, tx) = (k as f32 / 10.0 * PI * 2.0).sin_cos();
                let (ex, ey) = (r * (0.55 + 0.45 * tx), r * 0.22 * ty);
                (ux * ex - uy * ey, uy * ex + ux * ey)
            };
            (pt(i % 10), pt(i % 10 + 1))
        }).collect()
    } else {
        let n = a.jag.len();
        let pt = |k: usize| {
            let (s, c) = (a.rot + (k % n) as f32 / n as f32 * PI * 2.0).sin_cos();
            (c * r * a.jag[k % n], s * r * a.jag[k % n])
        };
        (0..n).map(|i| (pt(i), pt(i + 1))).collect()
    }
}

/// A round white blot thinning to nothing at the rim, with no edge to see.
fn haze_texture() -> Texture2D {
    const N: usize = 64;
    let mut px = Vec::with_capacity(N * N * 4);
    for i in 0..N * N {
        let (x, y) = ((i % N) as f32 + 0.5, (i / N) as f32 + 0.5);
        let d2 = (x / N as f32 * 2.0 - 1.0).powi(2) + (y / N as f32 * 2.0 - 1.0).powi(2);
        px.extend([255, 255, 255, ((1.0 - d2.min(1.0)).powi(2) * 255.0) as u8]);
    }
    let tex = Texture2D::from_rgba8(N as u16, N as u16, &px);
    tex.set_filter(FilterMode::Linear);
    tex
}

/// A mushroom in thin lines, its cap `w` to each side, leaning by `lean`.
fn draw_mushroom(blip: &Blip, x: f32, y: f32, w: f32, lean: f32, c: BlipColor) {
    let (s, k) = lean.sin_cos();
    let at = |px: f32, py: f32| (x + (px * k - py * s) * w, y + (px * s + py * k) * w);
    let line = |p: (f32, f32), q: (f32, f32)| blip.draw_line_ex(p.0, p.1, q.0, q.1, 1.5, c);
    // part of an ellipse, from angle `a0` to `a1`, in `n` lines
    let arc = |cx: f32, cy: f32, rx: f32, ry: f32, a0: f32, a1: f32, n: usize| {
        let pt = |j: usize| {
            let (sn, cs) = (a0 + (a1 - a0) * j as f32 / n as f32).sin_cos();
            at(cx + cs * rx, cy + sn * ry)
        };
        for j in 0..n { line(pt(j), pt(j + 1)); }
    };
    arc(0.0, -0.2, 1.0, 0.85, PI, PI * 2.0, 12); // the cap
    arc(0.0, -0.2, 1.0, 0.18, 0.0, PI, 8);       // its underside
    line(at(-0.26, -0.03), at(-0.36, 0.9));      // the stalk
    line(at(0.26, -0.03), at(0.36, 0.9));
    arc(0.0, 0.9, 0.36, 0.1, 0.0, PI, 5);
    for (sx, sy, sr) in [(-0.45, -0.58, 0.16), (0.1, -0.75, 0.2), (0.55, -0.45, 0.12)] {
        arc(sx, sy, sr, sr * 0.8, 0.0, PI * 2.0, 8);
    }
}

/// A thread from `p` to `q`, shading from `c0` to `c1`: a faint bowed line
/// with a brighter pulse `pulse` of the way along it.
fn draw_thread(blip: &Blip, p: (f32, f32), q: (f32, f32), bow: f32, pulse: f32, c0: BlipColor, c1: BlipColor, a: f32) {
    const N: usize = 14;
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let at = |u: f32| {
        let b = 4.0 * u * (1.0 - u) * bow;
        (p.0 + dx * u - dy * b, p.1 + dy * u + dx * b)
    };
    for j in 0..N {
        let u = j as f32 / N as f32;
        let (m, n) = (at(u), at(u + 1.0 / N as f32));
        let lit = 1.0 + 2.0 * (1.0 - ((u - pulse).abs() * 6.0).min(1.0));
        let mix = |x: f32, y: f32| x + (y - x) * u;
        blip.draw_line(m.0, m.1, n.0, n.1,
            BlipColor { r: mix(c0.r, c1.r), g: mix(c0.g, c1.g), b: mix(c0.b, c1.b), a: a * lit });
    }
}

/// Green and purple patches behind the field, a mushroom at the heart of
/// each, wandering and coming and going on clocks of their own. Wave by wave
/// they fill out, swell and stay, until little of the black is left. From
/// wave 3 threads run between the mushrooms, and later out to the rocks.
fn draw_haze(blip: &Blip, blot: &Texture2D, rocks: &[Asteroid], level: i32, t: f32) {
    let full = ((level - 1) as f32 / (HAZE_FULL_WAVE - 1) as f32).clamp(0.0, 1.0);
    let patch = |i: usize| {
        let k = i as f32;
        let x = PLAY_W as f32 * (0.5 + 0.45 * (t * (0.05 + 0.011 * k) + k * 2.4).sin());
        let y = PLAY_Y0 + PLAY_H as f32 * (0.5 + 0.45 * (t * (0.04 + 0.013 * k) + k * 1.7).cos());
        let r = 200.0 + 70.0 * (k * 1.3).sin();
        // squared, so on the early waves each is gone a while between showings
        let fade = (0.5 + 0.5 * (t * (0.21 + 0.03 * k) + k * 1.9).sin()).powi(2);
        (x, y, r, 0.7 * full + (1.0 - 0.7 * full) * fade, if i % 2 == 0 { NEON_GREEN } else { NEON_PURPLE })
    };
    for i in 0..HAZES {
        let (x, y, r, fade, c) = patch(i);
        let r = r * (1.0 + 0.6 * full);
        // purple is the darker of the two to the eye, so it is laid on thicker
        let a = (HAZE_ALPHA + (HAZE_ALPHA_MAX - HAZE_ALPHA) * full) * if i % 2 == 0 { 1.0 } else { 1.5 };
        blip.draw_texture_tinted(blot, x - r, y - r, r * 2.0, r * 2.0, BlipColor { a: a * fade, ..c });
    }
    if level >= THREAD_WAVE {
        let grown = ((level - THREAD_WAVE) as f32 / THREAD_WAVES as f32).min(1.0);
        let a = THREAD_ALPHA + (THREAD_ALPHA_MAX - THREAD_ALPHA) * grown;
        // from the foot of the stalk
        let foot = |i: usize| { let (x, y, r, fade, c) = patch(i); ((x, y + r * 0.13), fade, c) };
        for i in 0..HAZES {
            let k = i as f32;
            let (p, fade, c) = foot(i);
            // each to the next, and two waves on to one across the ring as well
            for (n, step) in [1, 3].into_iter().enumerate().take(if level >= THREAD_WAVE + 2 { 2 } else { 1 }) {
                let (q, fade2, c2) = foot((i + step) % HAZES);
                let bow = 0.12 * (t * 0.3 + k + n as f32 * 2.0).sin();
                draw_thread(blip, p, q, bow, (t * 0.15 + k * 0.37 + n as f32 * 0.5).fract(), c, c2, a * fade.min(fade2));
            }
            if level < THREAD_ROCK_WAVE { continue; }
            let near = pool_iter(rocks)
                .map(|rock| (rock, (rock.x - p.0).hypot(rock.y - p.1)))
                .filter(|&(_, d)| d < THREAD_REACH)
                .min_by(|m, n| m.1.total_cmp(&n.1));
            if let Some((rock, d)) = near {
                // thinning out as the rock gets away
                let hold = (1.0 - d / THREAD_REACH) * 0.6;
                draw_thread(blip, p, (rock.x, rock.y), 0.1 * (t * 0.4 + k).sin(), (t * 0.3 + k * 0.61).fract(),
                    c, rock_color(rock, level, t), a * fade * hold);
            }
        }
    }
    for i in 0..HAZES {
        let (x, y, r, fade, c) = patch(i);
        let a = SHROOM_ALPHA + (SHROOM_ALPHA_MAX - SHROOM_ALPHA) * full;
        let lean = 0.12 * (t * 0.5 + i as f32).sin();
        draw_mushroom(blip, x, y, r * 0.14, lean, BlipColor { a: a * fade, ..c });
    }
}

fn draw_asteroid(blip: &Blip, a: &Asteroid, level: i32, t: f32) {
    let c = rock_color(a, level, t);
    let flower = is_flower(a, level);
    let lines = outline(a, flower);
    let at = |p: (f32, f32)| (a.x + p.0, a.y + p.1);
    for (back, age) in smear_echoes(level) {
        // Where the rock was: drifted back, and turned back about its centre.
        let (ox, oy) = (a.x - a.vx * back, a.y - a.vy * back);
        let (s, k) = (-a.spin * back).sin_cos();
        let was = |p: (f32, f32)| (ox + p.0 * k - p.1 * s, oy + p.0 * s + p.1 * k);
        for &(p, q) in &lines { smear_line(blip, (at(p), at(q)), (was(p), was(q)), age, c); }
    }
    for &(p, q) in &lines {
        let (p, q) = (at(p), at(q));
        blip.draw_glow_line(p.0, p.1, q.0, q.1, c);
    }
    if flower {
        let heart = hsv(0.14 + rock_seed(a, 2) * 0.1, 0.8, 1.0, 1.0);
        blip.fill_glow_circle(a.x, a.y, a.size.radius() * 0.2, heart);
    }
}

fn draw_saucer(blip: &Blip, s: &Saucer, level: i32) {
    let r = if s.big { 16.0 } else { 9.0 };
    // A shot is coming: the core lights up and swells just before it fires.
    if s.fire_t < SAUCER_TELL {
        let k = 1.0 - s.fire_t / SAUCER_TELL;
        blip.fill_glow_circle(s.x, s.y, 2.0 + 4.0 * k, NEON_PINK);
    }
    let w = r * 2.6;
    let dw = r * 1.1;
    let h = r * 0.5;
    let hull = |x: f32, y: f32| [
        (x - w / 2.0, y),
        (x - dw / 2.0, y - h),
        (x + dw / 2.0, y - h),
        (x + w / 2.0, y),
        (x + dw / 2.0, y + h),
        (x - dw / 2.0, y + h),
    ];
    let pts = hull(s.x, s.y);
    for (back, age) in smear_echoes(level) {
        let was = hull(s.x - s.vx * back, s.y - (s.wave_t * 2.0).sin() * 24.0 * back);
        for i in 0..pts.len() {
            let j = (i + 1) % pts.len();
            smear_line(blip, (pts[i], pts[j]), (was[i], was[j]), age, NEON_PINK);
        }
    }
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

fn draw_play(blip: &Blip, g: &Game, blot: &Texture2D) {
    let t = blip::macroquad::time::get_time() as f32;
    blip.clear(BLIP_BLACK);
    draw_haze(blip, blot, &g.asteroids, g.sess.level, t);
    for a in pool_iter(&g.asteroids) { draw_asteroid(blip, a, g.sess.level, t); }
    for d in pool_iter(&g.debris) {
        let h = d.len * 0.5;
        let col = BlipColor { a: d.ttl / d.ttl0, ..d.color };
        let shard = |back: f32| {
            let (x, y) = (d.x - d.vx * back, d.y - d.vy * back);
            let (s, c) = (d.rot - d.spin * back).sin_cos();
            ((x - c * h, y - s * h), (x + c * h, y + s * h))
        };
        let (p, q) = shard(0.0);
        // no echo from before the burst
        for (back, age) in smear_echoes(g.sess.level).filter(|e| e.0 < d.ttl0 - d.ttl) {
            smear_line(blip, (p, q), shard(back), age, col);
        }
        blip.draw_line(p.0, p.1, q.0, q.1, col);
    }
    if g.saucer.active { draw_saucer(blip, &g.saucer, g.sess.level); }
    g.fx.draw(blip);
    if g.wave_t < WAVE_BANNER_SECS && g.state == State::Play {
        let a = (1.0 - g.wave_t / WAVE_BANNER_SECS).min(0.5) * 2.0;
        blip.draw_centered(&format!("WAVE {}", g.sess.level), PLAY_Y0 + 120.0, 4.0, BlipColor { a, ..NEON_CYAN });
    }
    for b in pool_iter(&g.bullets) {
        // from wave 3 the ship's shots run through the rainbow
        let c = if !b.from_player { NEON_PINK }
            else if g.sess.level >= 3 { hsv(t * 0.8 + b.x * 0.004, 0.6, 1.0, 1.0) } else { NEON_YELLOW };
        // a shot's echoes are dots, back to the muzzle and no further
        for (back, age) in smear_echoes(g.sess.level).filter(|e| e.0 < BULLET_TTL - b.ttl) {
            blip.fill_circle(b.x - b.vx * back, b.y - b.vy * back, 1.0, BlipColor { a: SMEAR_ALPHA * age, ..c });
        }
        blip.fill_glow_circle(b.x, b.y, 2.0, c);
    }
    if g.ship_alive {
        draw_ship(blip, &g.ship, g.invuln_t, g.sess.level, NEON_CYAN);
    }
    blip.draw_hud(g.sess.score, g.sess.lives);
    // x=12 clears the shell's 3px canvas clip, which eats more of this wider canvas's units.
    let lvl = format!("LEVEL {}", g.sess.level);
    blip.draw_text(&lvl, 12.0, WIN_H as f32 - 18.0, 1.5, BLIP_GRAY);
}

fn draw_title(blip: &Blip, hi: &web::HighScore) {
    blip.clear(BLIP_BLACK);
    draw_horizon_grid(blip, (WIN_H / 3) as f32, WIN_H as f32);
    blip.draw_centered("METEORS", (WIN_H / 4) as f32, 6.0, NEON_CYAN);
    // METEORS is sz=6 (42px tall) — clear its bottom by a real margin.
    blip.draw_hi(hi, (WIN_H / 4) as f32 + 50.0, NEON_YELLOW);
    blip.draw_centered("PRESS FIRE", (WIN_H / 2) as f32, 3.0, NEON_YELLOW);
    blip.draw_centered("ARROWS/WASD ROTATE+THRUST", (WIN_H * 2 / 3) as f32, 2.0, BLIP_GRAY);
    blip.draw_centered("SPACE FIRE  ·  Z HYPERSPACE", (WIN_H * 2 / 3) as f32 + 24.0, 2.0, BLIP_GRAY);
}

fn draw_over(blip: &Blip, score: i32, hi: &web::HighScore, waiting: bool) {
    blip.clear(BLIP_BLACK);
    draw_horizon_grid(blip, (WIN_H / 3) as f32, WIN_H as f32);
    blip.draw_game_over(score, hi, NEON_PINK, NEON_CYAN, NEON_YELLOW, !waiting);
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("METEORS", WIN_W, WIN_H)
}


const FIRE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/fire.wav"));
const THRUST_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/thrust.wav"));
const BANG_LARGE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/bang_large.wav"));
const BANG_MEDIUM_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/bang_medium.wav"));
const BANG_SMALL_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/bang_small.wav"));
const SAUCER_BIG_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/saucer_big.wav"));
const SAUCER_SMALL_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/saucer_small.wav"));
const HYPERSPACE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/hyperspace.wav"));
const EXTRA_LIFE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/extra_life.wav"));
const SHIP_EXPLOSION_WAV: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/ship_explosion.wav"));

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();
    let blot = haze_texture();

    // DRIFT, then STORM from wave STORM_WAVE (synthesised here, see blip_assets::meteors).
    let mut music = Jukebox::new(&[blip_assets::meteors::drift_wav, blip_assets::meteors::storm_wav]);
    music.start(0).await;

    let mut sfx = Sounds {
        fire:         blip::audio::load_sound(FIRE_WAV).await,
        thrust:       blip::audio::load_sound(THRUST_WAV).await,
        bang_large:   blip::audio::load_sound(BANG_LARGE_WAV).await,
        bang_medium:  blip::audio::load_sound(BANG_MEDIUM_WAV).await,
        bang_small:   blip::audio::load_sound(BANG_SMALL_WAV).await,
        saucer_big:   blip::audio::load_sound(SAUCER_BIG_WAV).await,
        saucer_small: blip::audio::load_sound(SAUCER_SMALL_WAV).await,
        ship_boom:    blip::audio::load_sound(SHIP_EXPLOSION_WAV).await,
        hyperspace:   blip::audio::load_sound(HYPERSPACE_WAV).await,
        extra_life:   blip::audio::load_sound(EXTRA_LIFE_WAV).await,
    };
    let mut thrust_snd_t = 0.0f32;

    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;

        music.play(if g.state != State::Title && g.sess.level >= STORM_WAVE { 1 } else { 0 });
        if g.state != State::Play { music.warm_up().await; }

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

        #[cfg(not(target_arch = "wasm32"))]
        if blip::bot::active() { bot::drive(&g, blip::bot::clock()); }
        match g.state {
            State::Title => update_title(&mut g),
            State::Play  => update_play(&mut g, dt, &mut sfx, &mut thrust_snd_t),
            State::Dead  => update_dead(&mut g, dt, &sfx),
            State::Over  => update_over(&mut g, dt),
        }

        match g.state {
            State::Title => draw_title(&blip, &web::high_score()),
            State::Over  => draw_over(&blip, g.sess.score, &web::high_score(), g.respawn_t.active()),
            State::Play | State::Dead => draw_play(&blip, &g, &blot),
        }

        blip.next_frame(60).await;
    }
}
