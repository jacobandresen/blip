//! Galactic Defender (Space Invaders), Rust port of
//! `games/galactic_defender/main.c` on macroquad.

use blip::input::{
    any_key_pressed, key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::prelude::ImageFormat;
use blip::macroquad::rand::rand;
use blip::macroquad::texture::{FilterMode, Texture2D};
use blip::{
    clamp, lerp, play_music, play_sfx, rand_int, rects_overlap, web, window_conf, Blip,
    BlipColor, LifeResult, Session, Timer,
    BLIP_BLACK, BLIP_CYAN, BLIP_GREEN, BLIP_MAGENTA, BLIP_ORANGE, BLIP_RED,
    BLIP_WHITE, BLIP_YELLOW,
};

// ---- layout -----------------------------------------------------------
const WIN_W: i32 = 480;
const WIN_H: i32 = 540;
const HUD_H: i32 = 28;
const PLAY_Y: i32 = HUD_H;
const GROUND_Y: i32 = WIN_H - 32;

// ---- alien grid -------------------------------------------------------
const ALIEN_COLS: i32 = 11; // max across all themes (theme 2 uses 11)
const ALIEN_ROWS: i32 = 6;  // max across all themes (theme 1 uses 6)
const ALIEN_W: i32 = 36;
const ALIEN_H: i32 = 28;
const ALIEN_XGAP: i32 = 5;
const ALIEN_YGAP: i32 = 8;
const ALIEN_TOTAL: usize = (ALIEN_COLS * ALIEN_ROWS) as usize;

// ---- tuning -----------------------------------------------------------
const PLAYER_SPEED: f32 = 200.0;
const BULLET_SPEED: f32 = 350.0;
const MARCH_START: i32 = 520;
const MARCH_MIN: i32 = 65;
const MARCH_DROP: f32 = 16.0;
const MAX_BOMBS: usize = 4;
const MAX_PLAYER_BULLETS: usize = 1;
const SHIELD_COLS: usize = 4;
const SHIELD_ROWS: usize = 3;
const SHIELD_BLOCK: i32 = 12;
const SHIELDS: usize = 4;
const EXPLOSION_TTL: f32 = 0.45;
const LIVES_START: i32 = 3;
const UFO_W: f32 = 36.0;
const UFO_H: f32 = 20.0;
const UFO_N_LIGHTS: usize = 8;
const UFO_SPIN_STEP_MS: f32 = 70.0; // ms between chase-light frame advances
const UFO_SPAWN_MIN: i32 = 6;       // seconds between UFO passes
const UFO_SPAWN_MAX: i32 = 12;

// ---- UFO death-laser attack --------------------------------------------
const UFO_LASER_CHANCE: f32 = 0.35;  // odds a given UFO pass becomes the laser attack
const UFO_TRACK_MIN_SECS: f32 = 1.8; // always stalks at least this long...
const UFO_TRACK_MAX_SECS: f32 = 4.0; // ...but never longer than this, win or lose
const UFO_TRACK_SPEED: f32 = 230.0;  // px/sec chasing the player's x — faster than the
                                      // player (PLAYER_SPEED) so it actually catches up
                                      // and locks on instead of just trailing behind
const UFO_LOCK_DIST: f32 = 14.0;     // "close enough" to the player to commit to charging
const UFO_STILL_SECS: f32 = 0.4;     // player must hold still this long before it commits
const UFO_CHARGE_SECS: f32 = 1.9;    // "gnarly" charge-up before it fires
const UFO_FIRE_SECS: f32 = 0.4;      // how long the beam itself is on screen
const LASER_HIT_W: f32 = UFO_W;      // width of the beam's kill zone

const MAX_UFO_BOMBS: usize = 1;
const UFO_BOMB_IDX: usize = MAX_PLAYER_BULLETS + MAX_BOMBS;
const N_BULLETS: usize = MAX_PLAYER_BULLETS + MAX_BOMBS + MAX_UFO_BOMBS;
const N_EXPLOSIONS: usize = ALIEN_TOTAL + 4;

// ---- player death / respawn --------------------------------------------
const DEAD_PAUSE: f32 = 3.6;         // total time in State::Dead before play resumes
const DEATH_EXPLOSION_PHASE: f32 = 0.7; // seconds of that pause spent on the giant explosion
                                         // before the ship starts fading back in — the
                                         // remainder (DEAD_PAUSE - this) is the mist fade-in
const RESPAWN_GRACE_SECS: f32 = 1.2; // once play resumes, firing stays locked out this long
                                      // so the respawn reads as a real vulnerable moment

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Play, Dead, Win, Over }

#[derive(Copy, Clone, PartialEq, Eq)]
enum UfoMode { Flying, Tracking, Charging, Firing }

#[derive(Copy, Clone)]
struct Alien {
    x: f32, y: f32,
    alive: bool,
    kind: usize, // 0=squid 1=crab 2=octopus
    anim: u8,    // 0/1
}

#[derive(Copy, Clone)]
struct Bullet {
    x: f32, y: f32,
    active: bool,
    player: bool,
}

#[derive(Copy, Clone)]
struct Explosion { x: f32, y: f32, ttl: f32, max_ttl: f32, scale: f32, active: bool }

#[derive(Copy, Clone)]
struct Shield {
    x: f32, y: f32,
    alive: [[bool; SHIELD_COLS]; SHIELD_ROWS],
}

struct Game {
    aliens: [Alien; ALIEN_TOTAL],
    bullets: [Bullet; N_BULLETS],
    explosions: [Explosion; N_EXPLOSIONS],
    shields: [Shield; SHIELDS],
    player_x: f32,
    sess: Session,
    march_timer: f32,
    march_dir: i32,
    march_drop_next: bool,
    bomb_timer: Timer,
    dead_timer: Timer,
    state: State,
    active_cols: i32,
    active_rows: i32,
    bomb_speed: f32,
    bomb_interval_range: (f32, f32),
    ufo_x: f32,
    ufo_active: bool,
    ufo_dir: i32,
    ufo_timer: Timer,
    ufo_bomb_timer: Timer,
    ufo_score: i32,
    ufo_score_timer: Timer,
    ufo_frame: usize,
    ufo_spin_timer: f32,
    march_step: usize,
    ufo_mode: UfoMode,
    laser_used_this_level: bool,
    player_still_secs: f32,
    ufo_track_timer: Timer,
    ufo_laser_x: f32,
    ufo_charge_timer: Timer,
    ufo_fire_timer: Timer,
    respawn_grace: Timer,
    dead_pause_total: f32,
}

impl Game {
    fn new() -> Self {
        let alien_default = Alien { x: 0.0, y: 0.0, alive: false, kind: 0, anim: 0 };
        let bullet_default = Bullet { x: 0.0, y: 0.0, active: false, player: false };
        let expl_default = Explosion { x: 0.0, y: 0.0, ttl: 0.0, max_ttl: EXPLOSION_TTL, scale: 1.0, active: false };
        let shield_default = Shield {
            x: 0.0, y: 0.0,
            alive: [[false; SHIELD_COLS]; SHIELD_ROWS],
        };
        Self {
            aliens: [alien_default; ALIEN_TOTAL],
            bullets: [bullet_default; N_BULLETS],
            explosions: [expl_default; N_EXPLOSIONS],
            shields: [shield_default; SHIELDS],
            player_x: 0.0,
            sess: Session::new(LIVES_START),
            march_timer: 0.0,
            march_dir: 1,
            march_drop_next: false,
            bomb_timer: Timer::default(),
            dead_timer: Timer::default(),
            state: State::Title,
            active_cols: 11,
            active_rows: 5,
            bomb_speed: 140.0,
            bomb_interval_range: (0.8, 2.5),
            ufo_x: 0.0,
            ufo_active: false,
            ufo_dir: 1,
            ufo_timer: Timer::default(),
            ufo_bomb_timer: Timer::default(),
            ufo_score: 0,
            ufo_score_timer: Timer::default(),
            ufo_frame: 0,
            ufo_spin_timer: 0.0,
            march_step: 0,
            ufo_mode: UfoMode::Flying,
            laser_used_this_level: false,
            player_still_secs: 0.0,
            ufo_track_timer: Timer::default(),
            ufo_laser_x: 0.0,
            ufo_charge_timer: Timer::default(),
            ufo_fire_timer: Timer::default(),
            respawn_grace: Timer::default(),
            dead_pause_total: DEAD_PAUSE,
        }
    }

    fn aliens_alive(&self) -> i32 {
        self.aliens.iter().filter(|a| a.alive).count() as i32
    }

    fn march_interval(&self) -> f32 {
        let alive = self.aliens_alive();
        if alive <= 0 { return MARCH_MIN as f32; }
        let theme_total = (self.active_cols * self.active_rows).max(1);
        // Each level the march is 7% faster, capping at a 50% speedup by level 8.
        let level_scale = (1.0 - (self.sess.level - 1) as f32 * 0.07).max(0.5);
        let ms = (MARCH_START as f32 * alive as f32 / theme_total as f32 * level_scale) as i32;
        ms.max(MARCH_MIN) as f32
    }

    fn spawn_explosion(&mut self, x: f32, y: f32) {
        self.spawn_explosion_ex(x, y, EXPLOSION_TTL, 1.0);
    }

    fn spawn_explosion_ex(&mut self, x: f32, y: f32, ttl: f32, scale: f32) {
        for e in self.explosions.iter_mut() {
            if !e.active {
                *e = Explosion { x, y, ttl, max_ttl: ttl, scale, active: true };
                return;
            }
        }
    }

    /// A dramatic, multi-burst explosion for the player's own ship going
    /// down — several overlapping fireballs of varying size and lifetime
    /// around the ship's `ALIEN_W`x`ALIEN_W` box at top-left `(x, y)`,
    /// instead of the single small alien-kill puff.
    fn spawn_player_death(&mut self, x: f32, y: f32) {
        const BURSTS: [(f32, f32, f32, f32); 6] = [
            (0.0,   0.0,  2.6, 2.4),
            (-16.0, -8.0, 1.7, 2.0),
            (16.0,  -6.0, 1.7, 2.1),
            (-10.0, 10.0, 1.5, 1.8),
            (12.0,  9.0,  1.5, 1.9),
            (0.0,   -14.0, 1.9, 2.2),
        ];
        for (dx, dy, scale, ttl_mul) in BURSTS {
            self.spawn_explosion_ex(x + dx, y + dy, EXPLOSION_TTL * ttl_mul, scale);
        }
    }

    fn free_bullet(&mut self, player: bool) -> Option<usize> {
        let (start, end) = if player {
            (0, MAX_PLAYER_BULLETS)
        } else {
            (MAX_PLAYER_BULLETS, MAX_PLAYER_BULLETS + MAX_BOMBS)
        };
        for i in start..end {
            if !self.bullets[i].active { return Some(i); }
        }
        None
    }

    fn build_shields(&mut self) {
        let total_w = SHIELDS as i32 * SHIELD_COLS as i32 * SHIELD_BLOCK
            + (SHIELDS as i32 - 1) * 40;
        let sx = (WIN_W - total_w) / 2;
        for s in 0..SHIELDS {
            self.shields[s].x = (sx + s as i32 * (SHIELD_COLS as i32 * SHIELD_BLOCK + 40)) as f32;
            self.shields[s].y = (GROUND_Y - 80) as f32;
            for r in 0..SHIELD_ROWS {
                for c in 0..SHIELD_COLS {
                    self.shields[s].alive[r][c] = true;
                }
            }
        }
    }

    fn init_aliens(&mut self) {
        let theme = (self.sess.level - 1).rem_euclid(5);
        // Level 1 gets a smaller formation than a later trip through theme 0,
        // so a first-time player isn't faced with a full 5x11 wall immediately.
        let (rows, cols) = match theme {
            0 => (if self.sess.level == 1 { 4 } else { 5 }, 10_i32),
            1 => (6,     10),
            2 => (3,     11),
            3 => (4,     10),
            _ => (6,     8),
        };
        self.bomb_speed = match theme { 0 => 160.0, 1 => 200.0, 2 => 250.0, 3 => 220.0, _ => 280.0 };
        // Bomb intervals tighten each level (capped at level 6 equivalent).
        let lv = ((self.sess.level - 1) as f32).min(5.0);
        self.bomb_interval_range = match theme {
            0 => ((0.6 - lv * 0.04).max(0.3),  (2.0 - lv * 0.15).max(0.9)),
            1 => ((0.5 - lv * 0.03).max(0.25), (1.5 - lv * 0.12).max(0.75)),
            2 => ((0.35 - lv * 0.02).max(0.2), (1.0 - lv * 0.08).max(0.55)),
            3 => ((0.45 - lv * 0.03).max(0.22), (1.2 - lv * 0.10).max(0.6)),
            _ => ((0.3 - lv * 0.02).max(0.18),  (0.85 - lv * 0.07).max(0.5)),
        };
        self.active_cols = cols;
        self.active_rows = rows;

        for a in self.aliens.iter_mut() { a.alive = false; }

        let grid_w = cols * (ALIEN_W + ALIEN_XGAP) - ALIEN_XGAP;
        let ox = (WIN_W - grid_w) / 2;
        let oy = PLAY_Y + 40;
        for r in 0..rows {
            let kind: usize = match theme {
                0 => [0, 1, 1, 2, 2][r as usize],
                1 => [0, 1, 1, 2, 2, 2][r as usize],
                2 => [0, 1, 2][r as usize],
                3 => [0, 0, 1, 2][r as usize],
                _ => [0, 0, 1, 1, 2, 2][r as usize],
            };
            for c in 0..cols {
                let i = (r * cols + c) as usize;
                self.aliens[i] = Alien {
                    x: (ox + c * (ALIEN_W + ALIEN_XGAP)) as f32,
                    y: (oy + r * (ALIEN_H + ALIEN_YGAP)) as f32,
                    alive: true,
                    kind,
                    anim: 0,
                };
            }
        }
        self.march_dir = 1;
        self.march_drop_next = false;
        self.march_timer = 0.0;
    }

    fn start_round_common(&mut self) {
        self.player_x = ((WIN_W - ALIEN_W) / 2) as f32;
        self.bullets.iter_mut().for_each(|b| b.active = false);
        self.explosions.iter_mut().for_each(|e| e.active = false);
        self.init_aliens();
        let theme = (self.sess.level - 1).rem_euclid(5);
        if theme != 2 && theme != 4 {
            self.build_shields();
        } else {
            for s in &mut self.shields {
                for row in &mut s.alive { row.fill(false); }
            }
        }
        // Give level 1 a longer bomb-free grace period to get oriented.
        self.bomb_timer.start(if self.sess.level == 1 { 3.5 } else { 2.0 });
        self.ufo_active = false;
        self.ufo_mode = UfoMode::Flying;
        self.laser_used_this_level = false;
        self.ufo_timer.start(rand_int(UFO_SPAWN_MIN, UFO_SPAWN_MAX) as f32);
        self.ufo_score_timer = Timer::default();
        self.ufo_bomb_timer = Timer::default();
        self.bullets[UFO_BOMB_IDX].active = false;
        self.state = State::Play;
    }

    fn start_game(&mut self) {
        self.sess.reset(LIVES_START);
        self.start_round_common();
    }
}

struct Sounds {
    shoot: blip::BlipSound,
    explosion: blip::BlipSound,
    level_clear: blip::BlipSound,
    ufo_siren: blip::BlipSound,
    march: [blip::BlipSound; 4],
    laser_charge: blip::BlipSound,
    laser_blast: blip::BlipSound,
}

const UFO_Y: f32 = (PLAY_Y + 8) as f32;

/// Detonate the laser: kill every alien, shield block, and (if it's standing
/// in the way) the player under the beam's column, with a cascade of
/// explosions down its length. Returns `true` if the player was killed.
fn fire_laser(g: &mut Game) -> bool {
    let bx0 = g.ufo_laser_x + UFO_W / 2.0 - LASER_HIT_W / 2.0;
    let bx1 = bx0 + LASER_HIT_W;

    for a in g.aliens.iter_mut() {
        if !a.alive { continue; }
        if a.x < bx1 && a.x + ALIEN_W as f32 > bx0 {
            a.alive = false;
            let pts = match a.kind { 0 => 30, 1 => 20, _ => 10 };
            g.sess.add_score(pts * g.sess.level);
        }
    }
    // A dense cascade of explosions straight down the beam's path, for a
    // "very dramatic" wipe (covers the vaporized aliens too).
    let mut y = UFO_Y;
    while y < GROUND_Y as f32 {
        g.spawn_explosion(bx0 + LASER_HIT_W / 2.0 - ALIEN_W as f32 / 2.0, y);
        y += 26.0;
    }

    for s in g.shields.iter_mut() {
        for r in 0..SHIELD_ROWS {
            for c in 0..SHIELD_COLS {
                if !s.alive[r][c] { continue; }
                let sx = s.x + (c as i32 * SHIELD_BLOCK) as f32;
                if sx < bx1 && sx + SHIELD_BLOCK as f32 > bx0 {
                    s.alive[r][c] = false;
                }
            }
        }
    }

    let player_hit = g.player_x < bx1 && g.player_x + ALIEN_W as f32 > bx0;
    if player_hit {
        let px = g.player_x;
        g.spawn_player_death(px, (GROUND_Y - 28) as f32);
        match g.sess.lose_life() {
            LifeResult::StillAlive => {
                g.bullets.iter_mut().for_each(|b| b.active = false);
                g.dead_timer.start(DEAD_PAUSE);
                g.dead_pause_total = DEAD_PAUSE;
                g.state = State::Dead;
            }
            LifeResult::GameOver => {
                g.state = State::Over;
            }
        }
    }
    player_hit
}

/// Returns `true` if the laser just killed the player this frame — the
/// caller should stop processing the round immediately when that happens.
fn update_ufo(g: &mut Game, dt: f32, sfx: &Sounds) -> bool {
    g.ufo_score_timer.tick(dt);

    if !g.ufo_active {
        if g.ufo_timer.tick(dt) && g.aliens_alive() > 0 {
            g.ufo_dir = if (rand() & 1) == 0 { 1 } else { -1 };
            g.ufo_x = if g.ufo_dir == 1 { -UFO_W } else { WIN_W as f32 };
            g.ufo_active = true;
            g.ufo_frame = 0;
            g.ufo_spin_timer = 0.0;
            g.bullets[UFO_BOMB_IDX].active = false;
            blip::play_alert(&sfx.ufo_siren);

            let roll = (rand() as f32) / (u32::MAX as f32);
            if !g.laser_used_this_level && roll < UFO_LASER_CHANCE {
                g.laser_used_this_level = true;
                g.ufo_mode = UfoMode::Tracking;
                g.ufo_track_timer.start(UFO_TRACK_MAX_SECS);
            } else {
                g.ufo_mode = UfoMode::Flying;
                g.ufo_bomb_timer.start(3.0);
            }
        }
        return false;
    }

    match g.ufo_mode {
        UfoMode::Flying => {
            g.ufo_x += 80.0 * g.ufo_dir as f32 * dt;
            g.ufo_spin_timer += dt * 1000.0;
            if g.ufo_spin_timer >= UFO_SPIN_STEP_MS {
                g.ufo_spin_timer = 0.0;
                g.ufo_frame = (g.ufo_frame + 1) % UFO_N_LIGHTS;
            }

            if g.ufo_x > WIN_W as f32 || g.ufo_x + UFO_W < 0.0 {
                g.ufo_active = false;
                g.bullets[UFO_BOMB_IDX].active = false;
                g.ufo_timer.start(rand_int(UFO_SPAWN_MIN, UFO_SPAWN_MAX) as f32);
                blip::stop_alert();
                return false;
            }

            // Only one bomb per pass — timer stays inactive after first fire.
            if g.ufo_bomb_timer.tick(dt) && !g.bullets[UFO_BOMB_IDX].active {
                g.bullets[UFO_BOMB_IDX] = Bullet {
                    x: g.ufo_x + UFO_W / 2.0 - 2.0,
                    y: UFO_Y + UFO_H * 0.6,
                    active: true,
                    player: false,
                };
            }
        }
        UfoMode::Tracking => {
            // Stalk the player's x, mimicking their movement, before
            // stopping to charge — a "being hunted" beat that gives the
            // player extra warning before the charge-up sound even starts.
            // It's faster than the player, so it actually catches up, but
            // it won't commit while the player keeps moving: it waits for
            // them to hold still (and be locked on) before switching to
            // Charging — or, at worst, once its time simply runs out.
            let target = (g.player_x + ALIEN_W as f32 / 2.0 - UFO_W / 2.0)
                .clamp(0.0, WIN_W as f32 - UFO_W);
            let step = UFO_TRACK_SPEED * dt;
            g.ufo_x += (target - g.ufo_x).clamp(-step, step);

            g.ufo_spin_timer += dt * 1000.0;
            if g.ufo_spin_timer >= UFO_SPIN_STEP_MS {
                g.ufo_spin_timer = 0.0;
                g.ufo_frame = (g.ufo_frame + 1) % UFO_N_LIGHTS;
            }

            let expired = g.ufo_track_timer.tick(dt);
            let elapsed = UFO_TRACK_MAX_SECS - g.ufo_track_timer.remaining();
            let locked_on = (g.ufo_x - target).abs() < UFO_LOCK_DIST;
            let player_still = g.player_still_secs >= UFO_STILL_SECS;
            if expired || (elapsed >= UFO_TRACK_MIN_SECS && locked_on && player_still) {
                g.ufo_mode = UfoMode::Charging;
                g.ufo_charge_timer.start(UFO_CHARGE_SECS);
                blip::stop_alert();
                play_sfx(&sfx.laser_charge);
            }
        }
        UfoMode::Charging => {
            // Frantic chase-light flicker while it powers up.
            g.ufo_spin_timer += dt * 1000.0;
            if g.ufo_spin_timer >= UFO_SPIN_STEP_MS * 0.25 {
                g.ufo_spin_timer = 0.0;
                g.ufo_frame = (g.ufo_frame + 1) % UFO_N_LIGHTS;
            }
            if g.ufo_charge_timer.tick(dt) {
                g.ufo_mode = UfoMode::Firing;
                g.ufo_fire_timer.start(UFO_FIRE_SECS);
                g.ufo_laser_x = g.ufo_x;
                play_sfx(&sfx.laser_blast);
                if fire_laser(g) {
                    return true;
                }
            }
        }
        UfoMode::Firing => {
            if g.ufo_fire_timer.tick(dt) {
                // The beam burns the UFO itself out.
                g.spawn_explosion(g.ufo_x, UFO_Y);
                play_sfx(&sfx.explosion);
                g.ufo_active = false;
                g.ufo_mode = UfoMode::Flying;
                g.ufo_timer.start(rand_int(UFO_SPAWN_MIN, UFO_SPAWN_MAX) as f32);
            }
        }
    }

    // Player bullet vs UFO — not while it's mid-beam (too late by then).
    if g.ufo_mode != UfoMode::Firing {
        for bi in 0..MAX_PLAYER_BULLETS {
            if !g.bullets[bi].active { continue; }
            if rects_overlap(g.bullets[bi].x, g.bullets[bi].y, 8.0, 16.0,
                             g.ufo_x, UFO_Y, UFO_W, UFO_H) {
                play_sfx(&sfx.explosion);
                let kill_x = g.ufo_x;
                g.spawn_explosion(kill_x, UFO_Y);
                g.bullets[bi].active = false;
                g.bullets[UFO_BOMB_IDX].active = false;
                let bonus = rand_int(1, 6) * 50;
                g.sess.add_score(bonus);
                g.ufo_score = bonus;
                g.ufo_score_timer.start(1.5);
                g.ufo_active = false;
                g.ufo_mode = UfoMode::Flying;
                g.ufo_timer.start(rand_int(UFO_SPAWN_MIN, UFO_SPAWN_MAX) as f32);
                blip::stop_alert();
                return false;
            }
        }
    }

    false
}

fn draw_ufo(blip: &Blip, g: &Game, saucer: &[Texture2D; UFO_N_LIGHTS]) {
    if g.ufo_active {
        match g.ufo_mode {
            UfoMode::Flying => {
                blip.draw_texture(&saucer[g.ufo_frame], g.ufo_x, UFO_Y, UFO_W, UFO_H);
            }
            UfoMode::Tracking => {
                blip.draw_texture(&saucer[g.ufo_frame], g.ufo_x, UFO_Y, UFO_W, UFO_H);
                // A faint lock-on reticle down toward the player — a quiet
                // "it's noticed you" tell, well before the loud charge-up.
                let cx = g.ufo_x + UFO_W / 2.0;
                let cy = UFO_Y + UFO_H;
                let px = g.player_x + ALIEN_W as f32 / 2.0;
                let py = (GROUND_Y - 28) as f32;
                blip.draw_line(cx, cy, px, py, BlipColor { r: 1.0, g: 0.3, b: 0.3, a: 0.18 });
            }
            UfoMode::Charging => {
                // The whole ship shakes as it powers up.
                let jx = (rand() % 5) as f32 - 2.0;
                let jy = (rand() % 3) as f32 - 1.0;
                blip.draw_texture(&saucer[g.ufo_frame], g.ufo_x + jx, UFO_Y + jy, UFO_W, UFO_H);

                let progress = 1.0 - (g.ufo_charge_timer.remaining() / UFO_CHARGE_SECS).max(0.0);
                let cx = g.ufo_x + UFO_W / 2.0;
                let cy = UFO_Y + UFO_H;

                // Growing charge glow beneath it, brightening as it nears firing.
                let glow_r = 8.0 + progress * 16.0;
                let glow_c = BlipColor {
                    r: 1.0, g: 0.25 + progress * 0.5, b: 0.15, a: 0.6 + progress * 0.4,
                };
                blip.fill_glow_circle(cx, cy, glow_r, glow_c);

                // Warning line straight down, telegraphing exactly where the
                // beam will land — always visible (never fully hidden, so it
                // can't be missed), pulsing faster and brighter as it nears
                // firing so the urgency escalates without ever going dark.
                let pulse_rate = 3.0 + progress * 10.0;
                let pulse = 0.5 + 0.5 * (progress * pulse_rate * std::f32::consts::TAU).sin();
                let warn_c = BlipColor {
                    r: 1.0, g: 0.2, b: 0.2, a: 0.5 + progress * 0.4 + pulse * 0.15,
                };
                blip.draw_glow_line(cx, cy, cx, GROUND_Y as f32, warn_c);
            }
            UfoMode::Firing => {
                let cx = g.ufo_laser_x + UFO_W / 2.0;
                let fade = (g.ufo_fire_timer.remaining() / UFO_FIRE_SECS).clamp(0.0, 1.0);
                // Full-width flash for the first instant — the dramatic hit.
                if fade > 0.7 {
                    blip.fill_rect(
                        0.0, PLAY_Y as f32, WIN_W as f32, (GROUND_Y - PLAY_Y) as f32,
                        BlipColor { r: 1.0, g: 1.0, b: 1.0, a: (fade - 0.7) * 0.6 },
                    );
                }
                // The beam: a wide hot core inside a wider red-hot glow.
                let beam_c = BlipColor { r: 1.0, g: 0.3 + fade * 0.4, b: 0.3, a: 1.0 };
                for off in [-10.0_f32, -5.0, 0.0, 5.0, 10.0] {
                    blip.draw_glow_line(cx + off, UFO_Y, cx + off, GROUND_Y as f32, beam_c);
                }
                blip.draw_line_ex(cx - 3.0, UFO_Y, cx - 3.0, GROUND_Y as f32, 6.0, BLIP_WHITE);
                blip.draw_line_ex(cx + 3.0, UFO_Y, cx + 3.0, GROUND_Y as f32, 6.0, BLIP_WHITE);
            }
        }
    }
    if g.ufo_score_timer.active() {
        let text = format!("{} PTS", g.ufo_score);
        blip.draw_centered(&text, UFO_Y, 2.0, BLIP_YELLOW);
    }
}

fn update_title(g: &mut Game) {
    if any_key_pressed() { g.start_game(); }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.respawn_grace.tick(dt);
    let shoot = !g.respawn_grace.active()
        && (key_pressed(BLIP_KEY_SPACE)
            || key_pressed(BLIP_KEY_UP)
            || key_pressed(BLIP_KEY_W));

    let moving_left = key_held(BLIP_KEY_LEFT) || key_held(BLIP_KEY_A);
    let moving_right = key_held(BLIP_KEY_RIGHT) || key_held(BLIP_KEY_D);
    let ps = PLAYER_SPEED * dt;
    if moving_left { g.player_x -= ps; }
    if moving_right { g.player_x += ps; }
    g.player_x = clamp(g.player_x, 0.0, (WIN_W - ALIEN_W) as f32);
    if moving_left || moving_right {
        g.player_still_secs = 0.0;
    } else {
        g.player_still_secs += dt;
    }

    if shoot {
        if let Some(i) = g.free_bullet(true) {
            play_sfx(&sfx.shoot);
            g.bullets[i] = Bullet {
                x: g.player_x + (ALIEN_W / 2 - 4) as f32,
                y: (GROUND_Y - 28) as f32,
                active: true,
                player: true,
            };
        }
    }

    for b in g.bullets.iter_mut() {
        if !b.active { continue; }
        b.y += if b.player { -BULLET_SPEED } else { g.bomb_speed } * dt;
        if b.y < PLAY_Y as f32 || b.y > WIN_H as f32 { b.active = false; }
    }

    g.march_timer += dt * 1000.0;
    if g.march_timer >= g.march_interval() {
        g.march_timer = 0.0;
        if g.aliens_alive() > 0 {
            play_sfx(&sfx.march[g.march_step]);
            g.march_step = (g.march_step + 1) % 4;
        }
        if g.march_drop_next {
            for a in g.aliens.iter_mut() {
                if a.alive { a.y += MARCH_DROP; }
            }
            g.march_dir = -g.march_dir;
            g.march_drop_next = false;
        } else {
            let step = (ALIEN_W / 3) as f32;
            let mut hit_edge = false;
            for a in g.aliens.iter_mut() {
                if !a.alive { continue; }
                a.x += step * g.march_dir as f32;
                a.anim ^= 1;
                if a.x < 2.0 || a.x + ALIEN_W as f32 > (WIN_W - 2) as f32 {
                    hit_edge = true;
                }
            }
            if hit_edge { g.march_drop_next = true; }
        }
    }

    if g.bomb_timer.tick(dt) {
        let r01 = (rand() as f32) / (u32::MAX as f32);
        let (lo, hi) = g.bomb_interval_range;
        g.bomb_timer.start(lerp(lo, hi, r01));
        let mut candidates = [0usize; ALIEN_COLS as usize];
        let mut nc = 0usize;
        for c in 0..g.active_cols {
            for r in (0..g.active_rows).rev() {
                let idx = (r * g.active_cols + c) as usize;
                if g.aliens[idx].alive {
                    candidates[nc] = idx;
                    nc += 1;
                    break;
                }
            }
        }
        if nc > 0 {
            if let Some(bi) = g.free_bullet(false) {
                let idx = candidates[rand_int(0, nc as i32 - 1) as usize];
                let a = g.aliens[idx];
                g.bullets[bi] = Bullet {
                    x: a.x + (ALIEN_W / 2 - 4) as f32,
                    y: a.y + ALIEN_H as f32,
                    active: true,
                    player: false,
                };
            }
        }
    }

    // Player bullet vs aliens
    for bi in 0..MAX_PLAYER_BULLETS {
        if !g.bullets[bi].active { continue; }
        for ai in 0..ALIEN_TOTAL {
            if !g.aliens[ai].alive { continue; }
            if rects_overlap(
                g.bullets[bi].x, g.bullets[bi].y, 8.0, 16.0,
                g.aliens[ai].x, g.aliens[ai].y, ALIEN_W as f32, ALIEN_H as f32,
            ) {
                play_sfx(&sfx.explosion);
                let (ax, ay, kind) = (g.aliens[ai].x, g.aliens[ai].y, g.aliens[ai].kind);
                g.spawn_explosion(ax, ay);
                g.aliens[ai].alive = false;
                g.bullets[bi].active = false;
                let pts = match kind { 0 => 30, 1 => 20, _ => 10 };
                g.sess.add_score(pts * g.sess.level);
                break;
            }
        }
    }

    // Bullets vs shields
    for bi in 0..N_BULLETS {
        if !g.bullets[bi].active { continue; }
        'outer: for s in 0..SHIELDS {
            for r in 0..SHIELD_ROWS {
                for c in 0..SHIELD_COLS {
                    if !g.shields[s].alive[r][c] { continue; }
                    let bx = g.shields[s].x + (c as i32 * SHIELD_BLOCK) as f32;
                    let by = g.shields[s].y + (r as i32 * SHIELD_BLOCK) as f32;
                    if rects_overlap(
                        g.bullets[bi].x, g.bullets[bi].y, 8.0, 16.0,
                        bx, by, SHIELD_BLOCK as f32, SHIELD_BLOCK as f32,
                    ) {
                        g.shields[s].alive[r][c] = false;
                        g.bullets[bi].active = false;
                        break 'outer;
                    }
                }
            }
        }
    }

    // Aliens vs shields — as the formation marches down it plows straight
    // through any barrier blocks in its path, same as the classic game.
    for a in g.aliens.iter() {
        if !a.alive { continue; }
        for s in 0..SHIELDS {
            for r in 0..SHIELD_ROWS {
                for c in 0..SHIELD_COLS {
                    if !g.shields[s].alive[r][c] { continue; }
                    let bx = g.shields[s].x + (c as i32 * SHIELD_BLOCK) as f32;
                    let by = g.shields[s].y + (r as i32 * SHIELD_BLOCK) as f32;
                    if rects_overlap(
                        a.x, a.y, ALIEN_W as f32, ALIEN_H as f32,
                        bx, by, SHIELD_BLOCK as f32, SHIELD_BLOCK as f32,
                    ) {
                        g.shields[s].alive[r][c] = false;
                    }
                }
            }
        }
    }

    // Bombs vs player
    for bi in MAX_PLAYER_BULLETS..N_BULLETS {
        if !g.bullets[bi].active { continue; }
        if rects_overlap(
            g.bullets[bi].x, g.bullets[bi].y, 8.0, 16.0,
            g.player_x, (GROUND_Y - 28) as f32, ALIEN_W as f32, 28.0,
        ) {
            g.bullets[bi].active = false;
            let px = g.player_x;
            g.spawn_player_death(px, (GROUND_Y - 28) as f32);
            play_sfx(&sfx.explosion);
            match g.sess.lose_life() {
                LifeResult::StillAlive => {
                    for k in MAX_PLAYER_BULLETS..N_BULLETS {
                        g.bullets[k].active = false;
                    }
                    g.dead_timer.start(DEAD_PAUSE);
                    g.dead_pause_total = DEAD_PAUSE;
                    g.state = State::Dead;
                }
                LifeResult::GameOver => {
                    g.state = State::Over;
                }
            }
            // The UFO stops updating once we leave State::Play, so its siren
            // loop would otherwise keep wailing through the Dead/Over screen.
            if g.ufo_active { g.ufo_active = false; blip::stop_alert(); }
            return;
        }
    }

    for a in g.aliens.iter() {
        if a.alive && a.y + ALIEN_H as f32 >= GROUND_Y as f32 {
            // The invasion reaching the ground ends the game outright, same
            // as the classic rule — but it still plays out as a real "kill"
            // (the giant death explosion) instead of an abrupt cut to the
            // game-over screen. Skip straight past the mist respawn phase
            // since there's no coming back from this one.
            let px = g.player_x;
            g.spawn_player_death(px, (GROUND_Y - 28) as f32);
            play_sfx(&sfx.explosion);
            g.sess.lives = 0;
            g.bullets.iter_mut().for_each(|b| b.active = false);
            g.dead_timer.start(DEATH_EXPLOSION_PHASE);
            g.dead_pause_total = DEATH_EXPLOSION_PHASE;
            g.state = State::Dead;
            if g.ufo_active { g.ufo_active = false; blip::stop_alert(); }
            return;
        }
    }

    if update_ufo(g, dt, sfx) {
        return; // the laser just killed the player this frame
    }

    if g.aliens_alive() == 0 {
        play_sfx(&sfx.level_clear);
        g.sess.next_level();
        g.dead_timer.start(1.5);
        g.state = State::Win;
        if g.ufo_active { g.ufo_active = false; blip::stop_alert(); }
    }

    for e in g.explosions.iter_mut() {
        if e.active {
            e.ttl -= dt;
            if e.ttl <= 0.0 { e.active = false; }
        }
    }
}

fn update_dead(g: &mut Game, dt: f32) {
    // Keep the death explosion's burst of fireballs animating through the
    // pause instead of freezing at full brightness until play resumes.
    for e in g.explosions.iter_mut() {
        if e.active {
            e.ttl -= dt;
            if e.ttl <= 0.0 { e.active = false; }
        }
    }
    if g.dead_timer.tick(dt) {
        g.bullets.iter_mut().for_each(|b| b.active = false);
        if g.sess.lives <= 0 {
            g.state = State::Over;
        } else {
            g.respawn_grace.start(RESPAWN_GRACE_SECS);
            g.state = State::Play;
        }
    }
}

fn update_win(g: &mut Game, dt: f32) {
    if g.dead_timer.tick(dt) { g.start_round_common(); }
}

fn update_over(g: &mut Game) {
    if !any_key_pressed() { return; }
    web::spend_coin();
    g.start_game();
}

fn draw_play(blip: &Blip, g: &Game,
             player: &Texture2D, alien: &[[Texture2D; 2]; 3],
             explosion: &Texture2D, shield: &Texture2D, saucer: &[Texture2D; UFO_N_LIGHTS]) {
    blip.draw_line(0.0, GROUND_Y as f32, WIN_W as f32, GROUND_Y as f32, BLIP_GREEN);

    for s in 0..SHIELDS {
        for r in 0..SHIELD_ROWS {
            for c in 0..SHIELD_COLS {
                if !g.shields[s].alive[r][c] { continue; }
                blip.draw_texture(
                    shield,
                    g.shields[s].x + (c as i32 * SHIELD_BLOCK) as f32,
                    g.shields[s].y + (r as i32 * SHIELD_BLOCK) as f32,
                    SHIELD_BLOCK as f32, SHIELD_BLOCK as f32,
                );
            }
        }
    }

    for a in g.aliens.iter() {
        if !a.alive { continue; }
        blip.draw_texture_tinted(
            &alien[a.kind][a.anim as usize],
            a.x, a.y, ALIEN_W as f32, ALIEN_H as f32, BLIP_WHITE,
        );
    }

    // While dead: hide the ship through the explosion, then have it fade
    // back in out of a dissipating mist once the respawn phase starts.
    let (ship_alpha, mist) = if g.state == State::Dead {
        // Use the pause that was actually armed, not the DEAD_PAUSE constant —
        // the "invasion reached the ground" death only runs the short
        // explosion phase before cutting to game over, with no respawn fade.
        let total = g.dead_pause_total;
        let elapsed = (total - g.dead_timer.remaining()).clamp(0.0, total);
        if elapsed <= DEATH_EXPLOSION_PHASE || total <= DEATH_EXPLOSION_PHASE {
            (0.0, 0.0)
        } else {
            let t = ((elapsed - DEATH_EXPLOSION_PHASE) / (total - DEATH_EXPLOSION_PHASE))
                .clamp(0.0, 1.0);
            (t, 1.0 - t)
        }
    } else {
        (1.0, 0.0)
    };

    if mist > 0.0 {
        let cx = g.player_x + ALIEN_W as f32 / 2.0;
        let cy = (GROUND_Y - 28) as f32 + 14.0;
        blip.fill_glow_circle(
            cx, cy, 14.0 + 22.0 * mist,
            BlipColor { r: 0.6, g: 0.9, b: 1.0, a: mist * 0.6 },
        );
    }

    if ship_alpha > 0.0 {
        blip.draw_texture_tinted(
            player, g.player_x, (GROUND_Y - 28) as f32, ALIEN_W as f32, 28.0,
            BlipColor { r: 1.0, g: 1.0, b: 1.0, a: ship_alpha },
        );
    }

    for b in g.bullets.iter() {
        if !b.active { continue; }
        let c = if b.player { BLIP_WHITE } else { BLIP_ORANGE };
        blip.fill_rect(b.x, b.y, 4.0, 12.0, c);
    }

    for e in g.explosions.iter() {
        if !e.active { continue; }
        let alpha = (e.ttl / e.max_ttl).clamp(0.0, 1.0);
        let tc = BlipColor { r: 1.0, g: 1.0, b: 1.0, a: alpha };
        // (e.x, e.y) is the top-left of a plain ALIEN_W box at scale 1; grow
        // outward from that box's centre for bigger bursts so callers don't
        // need to know about scale.
        let sz = ALIEN_W as f32 * e.scale;
        let grow = (sz - ALIEN_W as f32) / 2.0;
        blip.draw_texture_tinted(
            explosion,
            e.x - grow, e.y - grow, sz, sz, tc,
        );
    }

    draw_ufo(blip, g, saucer);
    blip.draw_hud(g.sess.score, g.sess.lives);
}

fn draw_title(blip: &Blip, alien: &[[Texture2D; 2]; 3]) {
    blip.clear(BLIP_BLACK);
    blip.draw_centered("GALACTIC", (WIN_H / 5) as f32,        5.0, BLIP_CYAN);
    blip.draw_centered("DEFENDER", (WIN_H / 5 + 50) as f32,   5.0, BLIP_MAGENTA);

    let dw = (ALIEN_W / 2) as f32;
    let dh = (ALIEN_H / 2) as f32;
    let ax = (blip.text_cx("30 PTS", 2) - ALIEN_W / 2 - 8) as f32;
    let voff = ((7 * 2 - ALIEN_H / 2) / 2) as f32;

    let row0 = (WIN_H / 2 - 40) as f32;
    let row1 = (WIN_H / 2 - 20) as f32;
    let row2 = (WIN_H / 2) as f32;

    blip.draw_texture_tinted(&alien[0][0], ax, row0 + voff, dw, dh, BLIP_MAGENTA);
    blip.draw_texture_tinted(&alien[1][0], ax, row1 + voff, dw, dh, BLIP_CYAN);
    blip.draw_texture_tinted(&alien[2][0], ax, row2 + voff, dw, dh, BLIP_GREEN);

    blip.draw_centered("30 PTS",        row0,                 2.0, BLIP_MAGENTA);
    blip.draw_centered("20 PTS",        row1,                 2.0, BLIP_CYAN);
    blip.draw_centered("10 PTS",        row2,                 2.0, BLIP_GREEN);
    blip.draw_centered("PRESS ANY KEY", (WIN_H * 2 / 3) as f32, 3.0, BLIP_WHITE);
}

fn draw_win(blip: &Blip, level: i32) {
    let buf = format!("LEVEL {}", level);
    blip.clear(BLIP_BLACK);
    blip.draw_centered("WAVE CLEAR", (WIN_H / 3) as f32, 4.0, BLIP_CYAN);
    blip.draw_centered(&buf,         (WIN_H / 2) as f32, 3.0, BLIP_YELLOW);
}

fn draw_over(blip: &Blip, score: i32) {
    let buf = format!("SCORE {}", score);
    blip.clear(BLIP_BLACK);
    blip.draw_centered("GAME OVER",     (WIN_H / 4) as f32,     5.0, BLIP_RED);
    blip.draw_centered(&buf,            (WIN_H / 2) as f32,     3.0, BLIP_WHITE);
    blip.draw_centered("PRESS ANY KEY", (WIN_H * 2 / 3) as f32, 3.0, BLIP_YELLOW);
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("GALACTIC DEFENDER", WIN_W, WIN_H)
}

const PLAYER_SHIP_PNG:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/player_ship.png"));
const ALIEN_SQUID_A_PNG:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/alien_squid_a.png"));
const ALIEN_SQUID_B_PNG:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/alien_squid_b.png"));
const ALIEN_CRAB_A_PNG:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/alien_crab_a.png"));
const ALIEN_CRAB_B_PNG:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/alien_crab_b.png"));
const ALIEN_OCTO_A_PNG:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/alien_octopus_a.png"));
const ALIEN_OCTO_B_PNG:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/alien_octopus_b.png"));
const EXPLOSION_PNG:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/explosion.png"));
const SHIELD_PNG:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/shield_block.png"));
const UFO_SAUCER_0_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/ufo_saucer_0.png"));
const UFO_SAUCER_1_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/ufo_saucer_1.png"));
const UFO_SAUCER_2_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/ufo_saucer_2.png"));
const UFO_SAUCER_3_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/ufo_saucer_3.png"));
const UFO_SAUCER_4_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/ufo_saucer_4.png"));
const UFO_SAUCER_5_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/ufo_saucer_5.png"));
const UFO_SAUCER_6_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/ufo_saucer_6.png"));
const UFO_SAUCER_7_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/ufo_saucer_7.png"));
const SHOOT_WAV:        &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/shoot.wav"));
const EXPLOSION_WAV:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/explosion.wav"));
const LEVEL_CLEAR_WAV:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/level_clear.wav"));
const UFO_SIREN_WAV:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/ufo_siren.wav"));
const LASER_CHARGE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/laser_charge.wav"));
const LASER_BLAST_WAV:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/laser_blast.wav"));
const MARCH1_WAV:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/march1.wav"));
const MARCH2_WAV:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/march2.wav"));
const MARCH3_WAV:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/march3.wav"));
const MARCH4_WAV:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/march4.wav"));
const MUSIC_WAV:        &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music.wav"));
const MUSIC2_WAV:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music2.wav"));
const MUSIC3_WAV:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music3.wav"));

// Loop durations in seconds — used to switch tracks at loop boundaries.
// music: 15.73s (124 BPM techno)  music2: 13.77s (142 BPM techno)  music3: 19.45s (100 BPM dread)
const MUSIC_DURATIONS: [f32; 3] = [15.73, 13.77, 19.45];

fn load_png(bytes: &'static [u8]) -> Texture2D {
    let tex = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
    tex.set_filter(FilterMode::Nearest);
    tex
}

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let player = load_png(PLAYER_SHIP_PNG);
    let alien = [
        [load_png(ALIEN_SQUID_A_PNG), load_png(ALIEN_SQUID_B_PNG)],
        [load_png(ALIEN_CRAB_A_PNG),  load_png(ALIEN_CRAB_B_PNG)],
        [load_png(ALIEN_OCTO_A_PNG),  load_png(ALIEN_OCTO_B_PNG)],
    ];
    let explosion = load_png(EXPLOSION_PNG);
    let shield = load_png(SHIELD_PNG);
    let saucer = [
        load_png(UFO_SAUCER_0_PNG), load_png(UFO_SAUCER_1_PNG),
        load_png(UFO_SAUCER_2_PNG), load_png(UFO_SAUCER_3_PNG),
        load_png(UFO_SAUCER_4_PNG), load_png(UFO_SAUCER_5_PNG),
        load_png(UFO_SAUCER_6_PNG), load_png(UFO_SAUCER_7_PNG),
    ];

    let sfx = Sounds {
        shoot:       blip::audio::load_sound(SHOOT_WAV).await,
        explosion:   blip::audio::load_sound(EXPLOSION_WAV).await,
        level_clear: blip::audio::load_sound(LEVEL_CLEAR_WAV).await,
        ufo_siren:   blip::audio::load_sound(UFO_SIREN_WAV).await,
        march: [
            blip::audio::load_sound(MARCH1_WAV).await,
            blip::audio::load_sound(MARCH2_WAV).await,
            blip::audio::load_sound(MARCH3_WAV).await,
            blip::audio::load_sound(MARCH4_WAV).await,
        ],
        laser_charge: blip::audio::load_sound(LASER_CHARGE_WAV).await,
        laser_blast:  blip::audio::load_sound(LASER_BLAST_WAV).await,
    };
    let music = [
        blip::audio::load_sound(MUSIC_WAV).await,
        blip::audio::load_sound(MUSIC2_WAV).await,
        blip::audio::load_sound(MUSIC3_WAV).await,
    ];
    let mut music_idx: usize = 0;
    let mut music_timer: f32 = MUSIC_DURATIONS[0];
    play_music(&music[0]);

    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;

        if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 {
                g.start_game();
            }
        }

        // Switch to a random different loop at each loop boundary.
        music_timer -= dt;
        if music_timer <= 0.0 {
            let next = {
                let candidate = rand_int(0, 1) as usize; // 0 or 1
                if candidate < music_idx { candidate } else { candidate + 1 } // skip current
            };
            music_idx = next;
            music_timer = MUSIC_DURATIONS[next];
            play_music(&music[next]);
        }
        match g.state {
            State::Title => update_title(&mut g),
            State::Play  => update_play(&mut g, dt, &sfx),
            State::Dead  => update_dead(&mut g, dt),
            State::Win   => update_win(&mut g, dt),
            State::Over  => update_over(&mut g),
        }

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title => draw_title(&blip, &alien),
            State::Win   => draw_win(&blip, g.sess.level),
            State::Over  => draw_over(&blip, g.sess.score),
            State::Play | State::Dead => {
                draw_play(&blip, &g, &player, &alien, &explosion, &shield, &saucer);
            }
        }

        blip.next_frame(60).await;
    }
}
