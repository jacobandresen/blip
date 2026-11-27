//! Space Invaders-inspired shooter.

use blip::color::*;
use blip::input::{coin_start, key_active, play_again, Stick, BLIP_KEY_SPACE};
use blip::macroquad::rand::rand;
use blip::macroquad::texture::Texture2D;
use blip::{
    GAME_OVER_MIN_WAIT,
    load_png,
    clamp, lerp, play_sfx, Jukebox, rand_int, rects_overlap, web, window_conf, Blip, Fx,
    BlipColor, LifeResult, Session, Timer, segment_rect_overlap,
    BLIP_BLACK, BLIP_GREEN, BLIP_ORANGE, BLIP_RED,
    BLIP_WHITE, BLIP_YELLOW,
};

// The effects and music are made by build.rs; the tests check them here.
#[cfg(test)]
mod tunes;
mod bot;
mod levels;
mod draw;
mod ufo;
mod boss;
mod invaders;
mod shields;
use boss::*;
use invaders::*;
use shields::*;
use ufo::*;
use draw::*;

use levels::*;

// ---- layout -----------------------------------------------------------
const WIN_W: i32 = 480;
const WIN_H: i32 = 540;
const HUD_H: i32 = 28;
const PLAY_Y: i32 = HUD_H;
const GROUND_Y: i32 = WIN_H - 32;

// ---- alien grid -------------------------------------------------------
const ALIEN_COLS: i32 = 10; // max across all themes
const ALIEN_ROWS: i32 = 6;  // max across all themes (theme 1 uses 6)
const ALIEN_W: i32 = 36;
const ALIEN_H: i32 = 28;
const ALIEN_XGAP: i32 = 5;
const ALIEN_YGAP: i32 = 8;
const ALIEN_TOTAL: usize = (ALIEN_COLS * ALIEN_ROWS) as usize;

// ---- tuning -----------------------------------------------------------
const PLAYER_SPEED: f32 = 200.0;
const PLAYER_ACCEL: f32 = 2200.0;
const PLAYER_BRAKE: f32 = 3200.0;
const TOUCH_CLOSE_SECS: f32 = 0.08;
const BULLET_SPEED: f32 = 420.0;
// The formation's step down at each edge: gentle on level 1 (a steady
// playtest bot was landed on at 16 px), full height from level 4.
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

const BOSS_SCALE: f32 = 2.6;          // times the ordinary saucer
const BOSS_W: f32 = UFO_W * BOSS_SCALE;
const BOSS_H: f32 = UFO_H * BOSS_SCALE;
const BOSS_Y: f32 = (PLAY_Y + 46) as f32;
const BOSS_SWAY: f32 = 26.0;          // how far it rises and falls as it crosses
const BOSS_FLASH: f32 = 0.08;         // how long it glows white after a hit

const MAX_UFO_BOMBS: usize = 1;
const UFO_BOMB_IDX: usize = MAX_PLAYER_BULLETS + MAX_BOMBS;
const N_BULLETS: usize = MAX_PLAYER_BULLETS + MAX_BOMBS + MAX_UFO_BOMBS;
const N_EXPLOSIONS: usize = ALIEN_TOTAL + 4;

// ---- player death / respawn --------------------------------------------
// Total time in State::Dead before play resumes. 2.4 (was 3.6): at a person's 250 ms reaction a lost cannon came
// every 20-30 s, and the pauses were a quarter of the run.
const DEAD_PAUSE: f32 = 2.4;
const DEATH_EXPLOSION_PHASE: f32 = 0.7; // seconds of that pause spent on the giant explosion
                                         // before the ship starts fading back in — the
                                         // remainder (DEAD_PAUSE - this) is the mist fade-in
const RESPAWN_GRACE_SECS: f32 = 1.2; // once play resumes, firing stays locked out this long
const DIVE_EVERY: (i32, i32) = (7, 12); // seconds between dives
const DIVE_TELL: f32 = 0.8;
const DIVE_SPEED: f32 = 150.0;
const DIVE_STEER: f32 = 60.0;
const DIVE_MIN_ALIENS: i32 = 6;         // the last few stay in formation

// ---- sharpshooting ---------------------------------------------------------
// Every third alien hit in a row without a shot going over the top is worth
// one multiple more, up to this. A miss starts the count again.
const STREAK_PER: i32 = 3;
// At 5 the bot scored five times what random fire did (skill 0.36 in the fun report): a steady eye kept x5 for most of
// a wave while random fire seldom passed x2, so a higher cap pays the aim that earns it.
const STREAK_MAX: i32 = 8;
/// Invaders this close above the ground line turn it into a warning.
const LANDING_WARN: i32 = 110;

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

/// An alien on its way down at the ship (see DIVE_LEVEL).
#[derive(Copy, Clone)]
struct Diver { x: f32, y: f32, kind: usize, t: f32 }

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

/// A square piece of a shot alien.
struct Chip { x: f32, y: f32, vx: f32, vy: f32, turn: f32, spin: f32, ttl: f32, c: BlipColor }
const CHIP_SECS: f32 = 0.8;

struct Game {
    aliens: [Alien; ALIEN_TOTAL],
    bullets: [Bullet; N_BULLETS],
    explosions: [Explosion; N_EXPLOSIONS],
    shields: [Shield; SHIELDS],
    player_x: f32,
    player_vx: f32,
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
    /// Where a finger on the touch strip wants the cannon's centre.
    touch_x: Option<f32>,
    ufo_track_timer: Timer,
    ufo_laser_x: f32,
    ufo_charge_timer: Timer,
    ufo_fire_timer: Timer,
    respawn_grace: Timer,
    dead_pause_total: f32,
    boss_active: bool,
    /// Set once this level's mothership has been dealt with, so clearing
    /// the formation does not summon a second one.
    boss_done: bool,
    boss_hp: i32,
    boss_hp_max: i32,
    boss_x: f32,
    boss_dir: i32,
    boss_phase: f32,
    boss_fire: Timer,
    boss_flash: f32,
    fx: Fx,
    /// Pieces of shot aliens, falling and fading (see break_alien).
    chips: Vec<Chip>,
    /// Alien hits in a row without a miss: one more multiple every STREAK_PER, up to STREAK_MAX.
    streak: blip::Streak,
    /// A run that just broke, and seconds it stays shown (in red, under the ground).
    lost_run: (i32, f32),
    /// Seconds to the next dive; the alien shaking before it goes and how
    /// long it has; the one in the air.
    dive_cd: f32,
    dive_pick: Option<(usize, f32)>,
    diver: Option<Diver>,
}

impl Game {
    /// The saucer's lights step round, one every UFO_SPIN_STEP_MS.
    fn spin_lights(&mut self, dt: f32) {
        self.ufo_spin_timer += dt * 1000.0;
        if self.ufo_spin_timer >= UFO_SPIN_STEP_MS {
            self.ufo_spin_timer = 0.0;
            self.ufo_frame = (self.ufo_frame + 1) % UFO_N_LIGHTS;
        }
    }

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
            player_vx: 0.0,
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
            touch_x: None,
            ufo_track_timer: Timer::default(),
            ufo_laser_x: 0.0,
            ufo_charge_timer: Timer::default(),
            ufo_fire_timer: Timer::default(),
            respawn_grace: Timer::default(),
            dead_pause_total: DEAD_PAUSE,
            boss_active: false,
            boss_done: false,
            boss_hp: 0,
            boss_hp_max: 0,
            boss_x: 0.0,
            boss_dir: 1,
            boss_phase: 0.0,
            boss_fire: Timer::default(),
            boss_flash: 0.0,
            fx: Fx::new(),
            chips: Vec::new(),
            streak: blip::Streak::new(STREAK_PER, STREAK_MAX),
            lost_run: (0, 0.0),
            dive_cd: 0.0,
            dive_pick: None,
            diver: None,
        }
    }

    fn aliens_alive(&self) -> i32 {
        self.aliens.iter().filter(|a| a.alive).count() as i32
    }

    fn march_interval(&self) -> f32 {
        let alive = self.aliens_alive();
        if alive <= 0 { return MARCH_MIN as f32; }
        let theme_total = (self.active_cols * self.active_rows).max(1);
        // Each level the march is 7% faster, up to level 5. It went on to twice as fast by level 8, and the full
        // formations of levels 6, 7, 11 and 12 then landed in 7 of 12 bot runs: one shot at a time cannot clear
        // 50-60 invaders in the 25-30 s they took to land.
        let level_scale = (1.0 - (self.sess.level - 1) as f32 * 0.07).max(0.72);
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

    /// The player ship's death: several overlapping fireballs round its
    /// `ALIEN_W`x`ALIEN_W` box at `(x, y)`.
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
        (start..end).find(|&i| !self.bullets[i].active)
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

    fn build_formation(&mut self) {
        let theme = (self.sess.level - 1).rem_euclid(5);
        // Level 1 gets a smaller formation than a later trip through theme 0,
        // so a first-time player isn't faced with a full 5x11 wall immediately.
        let (rows, cols) = match theme {
            0 => (if self.sess.level == 1 { 4 } else { 5 }, 10_i32),
            1 => (if self.sess.level == 2 { 5 } else { 6 }, 10),
            // 10 wide, not 11: at 446 of 480 px it met an edge every third step and dropped so often that it landed
            // in 21 s on wave 8 (seed 3).
            2 => (3,     10),
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

    fn start_wave(&mut self) {
        self.player_x = ((WIN_W - ALIEN_W) / 2) as f32;
        self.player_vx = 0.0;
        self.bullets.iter_mut().for_each(|b| b.active = false);
        self.explosions.iter_mut().for_each(|e| e.active = false);
        self.fx.clear();
        self.chips.clear();
        self.streak.miss();
        self.dive_cd = rand_int(DIVE_EVERY.0, DIVE_EVERY.1) as f32;
        self.dive_pick = None;
        self.diver = None;
        self.build_formation();
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
        // A mothership belongs to the level that summoned it. Losing a
        // life mid-fight re-runs this, so the fight restarts with the
        // formation already dead — see update_play()'s win check.
        self.boss_active = false;
        self.boss_done = false;
        self.boss_flash = 0.0;
        self.state = State::Play;
    }

    fn start_game(&mut self) {
        self.sess.reset(LIVES_START);
        self.start_wave();
    }
}

blip::sounds!(Sounds {
    shoot, explosion, dive, level_clear, ufo_siren,
    march: [blip::BlipSound; 4] = [blip::sound!("march1"), blip::sound!("march2"), blip::sound!("march3"), blip::sound!("march4")],
    laser_charge, laser_blast, game_over,
    streak_up: [blip::BlipSound; 4] = [blip::sound!("streak2"), blip::sound!("streak3"), blip::sound!("streak4"), blip::sound!("streak5")],
});

const UFO_Y: f32 = (PLAY_Y + 8) as f32;

fn update_title(g: &mut Game) {
    if coin_start() { g.start_game(); }
}

/// An alien kind's colour, and so its score: magenta 30, cyan 20, green 10,
/// the same as the title screen's legend.
fn alien_color(kind: usize) -> BlipColor {
    match kind {
        0 => BlipColor { r: 0.93, g: 0.42, b: 0.86, a: 1.0 }, // squid   — 30 points
        1 => BLIP_SEA,                                       // crab    — 20 points
        _ => BLIP_LEAF,                                      // octopus — 10 points
    }
}

/// A bomb (drawn 4x12) against the ship's wedge, not its 36x28 box, whose top
/// corners are empty sky.
fn bomb_hits_ship(bx: f32, by: f32, px: f32) -> bool {
    let top = (GROUND_Y - 28) as f32;
    let tip = by + 12.0;
    if tip < top || by > GROUND_Y as f32 { return false; }
    // The wedge's half-width at the bomb's lowest row inside the ship
    // (sprite: 1px at the top to 16 of 32 at the base, drawn 36 wide).
    let row = (tip.min(GROUND_Y as f32) - top) / 28.0;
    let half = (1.0 + row * 15.0) * ALIEN_W as f32 / 32.0;
    let dx = (bx + 2.0 - (px + ALIEN_W as f32 / 2.0)).abs();
    dx <= half.max(3.0) + 2.0
}

fn bomb_hits_ship_swept(
    from: (f32, f32),
    to: (f32, f32),
    player_from: f32,
    player_to: f32,
) -> bool {
    let span = (to.0 - from.0).abs().max((to.1 - from.1).abs())
        .max((player_to - player_from).abs());
    let steps = (span / 4.0).ceil().max(1.0) as usize;
    (0..=steps).any(|i| {
        let t = i as f32 / steps as f32;
        bomb_hits_ship(
            lerp(from.0, to.0, t),
            lerp(from.1, to.1, t),
            lerp(player_from, player_to, t),
        )
    })
}

fn bullet_hits_rect(from: (f32, f32), to: (f32, f32), x: f32, y: f32, w: f32, h: f32) -> bool {
    segment_rect_overlap(from.0 + 4.0, from.1 + 8.0, to.0 + 4.0, to.1 + 8.0,
        x - 4.0, y - 8.0, w + 8.0, h + 16.0)
}

fn bullet_hits_moving_rect(
    from: (f32, f32),
    to: (f32, f32),
    rect_from: (f32, f32),
    rect_to: (f32, f32),
    w: f32,
    h: f32,
) -> bool {
    blip::sweep_hits((from.0 + 4.0, from.1 + 8.0), (to.0 + 4.0, to.1 + 8.0), rect_from, rect_to, (-4.0, -8.0, w + 8.0, h + 16.0))
}

fn expire_player_shots(g: &mut Game, went_over_top: &[bool; MAX_PLAYER_BULLETS]) {
    for (i, &expired) in went_over_top.iter().enumerate() {
        if expired && g.bullets[i].active {
            g.bullets[i].active = false;
            g.streak.miss();
        }
    }
}

fn expire_ground_bombs(g: &mut Game, reached_ground: &[bool; N_BULLETS]) {
    for (i, &reached) in reached_ground.iter().enumerate() {
        if reached && g.bullets[i].active {
            g.bullets[i].active = false;
            g.fx.burst(g.bullets[i].x + 2.0, GROUND_Y as f32 - 2.0, 6, 60.0, BLIP_ORANGE);
        }
    }
}

/// A cannon lost: the pause before the next, or the game-over wait. True while cannons are left.
fn lose_cannon(g: &mut Game) -> bool {
    match g.sess.lose_life() {
        LifeResult::StillAlive => {
            g.dead_timer.start(DEAD_PAUSE);
            g.dead_pause_total = DEAD_PAUSE;
            g.state = State::Dead;
            true
        }
        LifeResult::GameOver => {
            g.dead_timer.start(GAME_OVER_MIN_WAIT);
            g.state = State::Over;
            false
        }
    }
}

/// The ship is hit: the explosion, a life gone, and the pause or the end.
fn lose_ship(g: &mut Game, sfx: &Sounds, why: &str) {
    let px = g.player_x;
    g.spawn_player_death(px, (GROUND_Y - 28) as f32);
    play_sfx(&sfx.explosion);
    blip::bot::add(why, 1.0);
    // Losing the cannon breaks the run of hits too: the multiple is for shooting well and staying alive.
    g.streak.miss();
    g.dive_pick = None;
    g.diver = None;
    if lose_cannon(g) { g.bullets[MAX_PLAYER_BULLETS..].iter_mut().for_each(|b| b.active = false); }
    // The UFO stops updating once we leave State::Play, so its siren
    // loop would otherwise keep wailing through the Dead/Over screen.
    stop_ufo(g);
}

fn update_explosions(g: &mut Game, dt: f32) {
    for e in &mut g.explosions {
        if e.active {
            e.ttl -= dt;
            if e.ttl <= 0.0 { e.active = false; }
        }
    }
}

/// The cannon has mass: it spools up to full speed and brakes over ~0.08 s.
fn move_ship(g: &mut Game, dt: f32) {
    let mut moving_left = Stick::Left.active();
    let mut moving_right = Stick::Right.active();
    let dir = moving_right as i32 - moving_left as i32;
    let mut target = dir as f32 * PLAYER_SPEED;
    // A finger asks for the speed that closes the gap in TOUCH_CLOSE_SECS,
    // capped at the keys' top speed, so the cannon settles under it.
    if let Some(tx) = g.touch_x {
        let gap = tx - ALIEN_W as f32 / 2.0 - g.player_x;
        target = clamp(gap / TOUCH_CLOSE_SECS, -PLAYER_SPEED, PLAYER_SPEED);
        moving_left = gap < -1.0;
        moving_right = gap > 1.0;
    }
    g.player_vx = blip::spool(g.player_vx, target, PLAYER_ACCEL, PLAYER_BRAKE, dt);
    g.player_x += g.player_vx * dt;
    let max_x = (WIN_W - ALIEN_W) as f32;
    if g.player_x < 0.0 || g.player_x > max_x { g.player_vx = 0.0; }
    g.player_x = clamp(g.player_x, 0.0, max_x);
    if moving_left || moving_right {
        g.player_still_secs = 0.0;
    } else {
        g.player_still_secs += dt;
    }
}

/// Hold to fire, like the cabinet: a new shot goes as soon as the last clears;
/// MAX_PLAYER_BULLETS = 1 is the rate limit.
fn fire(g: &mut Game, sfx: &Sounds) {
    let shoot = !g.respawn_grace.active()
        && (key_active(BLIP_KEY_SPACE)
            || Stick::Up.active());
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
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    let player_x_before = g.player_x;
    let ufo_start_x = g.ufo_x;
    let ufo_was_active = g.ufo_active;
    // At 0.6 volume: the chime comes often, and its jingle peaks as loud as a level clear.
    if let Some(m) = g.streak.risen() { blip::audio::play_sfx_volume(&sfx.streak_up[(m as usize - 2).min(3)], 0.6); }
    g.respawn_grace.tick(dt);
    move_ship(g, dt);
    fire(g, sfx);

    let bullet_starts: [(f32, f32); N_BULLETS] = std::array::from_fn(|i| (g.bullets[i].x, g.bullets[i].y));
    let bullet_was_active: [bool; N_BULLETS] = std::array::from_fn(|i| g.bullets[i].active);
    let (over_top, landed) = move_shots(g, dt);
    g.fx.update(dt);
    march(g, dt, sfx);
    drop_bombs(g, dt);
    shoot_aliens(g, &bullet_starts, sfx);
    shoot_diver(g, &bullet_starts, sfx);
    hit_shields(g, &bullet_starts, &bullet_was_active);
    plow_shields(g);

    if let Some(death) = bombed(g, &bullet_starts, &bullet_was_active, player_x_before) {
        lose_ship(g, sfx, death);
        expire_player_shots(g, &over_top);
        expire_ground_bombs(g, &landed);
        return;
    }
    expire_ground_bombs(g, &landed);

    if update_diver(g, dt, sfx) {
        lose_ship(g, sfx, "death_diver");
        expire_player_shots(g, &over_top);
        return;
    }
    if g.aliens.iter().any(|a| a.alive && a.y + ALIEN_H as f32 >= GROUND_Y as f32) {
        invaded(g, sfx);
        return;
    }
    if update_ufo(g, dt, sfx, &bullet_starts, ufo_start_x, ufo_was_active) {
        expire_player_shots(g, &over_top);
        return; // the laser just killed the player this frame
    }

    update_boss(g, dt, sfx);
    hit_boss(g, sfx, &bullet_starts);
    expire_player_shots(g, &over_top);
    if g.aliens_alive() == 0 && !g.boss_active && g.diver.is_none() { formation_cleared(g, sfx); }
    update_explosions(g, dt);
    update_chips(g, dt);
}

/// The ship's shots climb and the bombs fall; which shots went over the top and which bombs reached the ground (both
/// stay live until the hit tests have had them; a bomb over the top just goes).
fn move_shots(g: &mut Game, dt: f32) -> ([bool; MAX_PLAYER_BULLETS], [bool; N_BULLETS]) {
    let mut over_top = [false; MAX_PLAYER_BULLETS];
    let mut landed = [false; N_BULLETS];
    for (i, b) in g.bullets.iter_mut().enumerate() {
        if !b.active { continue; }
        b.y += if b.player { -BULLET_SPEED } else { g.bomb_speed } * dt;
        if b.y < PLAY_Y as f32 {
            if b.player { over_top[i] = true; } else { b.active = false; }
        }
        // A bomb that misses bursts on the ground line.
        landed[i] = !b.player && b.y + 12.0 > GROUND_Y as f32;
    }
    (over_top, landed)
}

/// The first bomb to strike the ship this frame, swept with it from where both were (a bomb dropped this frame from
/// where it is), spent; and the death it counts as.
fn bombed(g: &mut Game, starts: &[(f32, f32); N_BULLETS], was_active: &[bool; N_BULLETS], ship_before: f32) -> Option<&'static str> {
    for bi in MAX_PLAYER_BULLETS..N_BULLETS {
        if !g.bullets[bi].active { continue; }
        let now = (g.bullets[bi].x, g.bullets[bi].y);
        let (from, ship_from) = if was_active[bi] { (starts[bi], ship_before) } else { (now, g.player_x) };
        if bomb_hits_ship_swept(from, now, ship_from, g.player_x) {
            g.bullets[bi].active = false;
            return Some(if bi == UFO_BOMB_IDX { "death_ufo_bomb" } else { "death_bomb" });
        }
    }
    None
}

/// The invasion landing ends the game outright, but as a real death explosion, and skips the respawn.
fn invaded(g: &mut Game, sfx: &Sounds) {
    blip::bot::add("death_landing", 1.0);
    let px = g.player_x;
    g.spawn_player_death(px, (GROUND_Y - 28) as f32);
    play_sfx(&sfx.explosion);
    g.sess.lives = 0;
    g.bullets.iter_mut().for_each(|b| b.active = false);
    g.dead_timer.start(DEATH_EXPLOSION_PHASE);
    g.dead_pause_total = DEATH_EXPLOSION_PHASE;
    g.state = State::Dead;
    stop_ufo(g);
}

/// The formation is gone. Whichever way this goes the ordinary UFO's pass is over: the level has ended, or a
/// mothership is taking the sky, and two sirens and two saucers at once is a fight nobody can read.
fn formation_cleared(g: &mut Game, sfx: &Sounds) {
    stop_ufo(g);
    // On a boss level the formation is the doorway, not the level.
    if g.is_boss_level() && !g.boss_done {
        g.bullets[UFO_BOMB_IDX].active = false;
        g.start_boss();
        blip::play_alert(&sfx.ufo_siren);
    } else {
        play_sfx(&sfx.level_clear);
        g.sess.next_level();
        g.dead_timer.start(1.5);
        g.state = State::Win;
    }
}

fn update_dead(g: &mut Game, dt: f32) {
    g.fx.update(dt);
    update_chips(g, dt);
    // Keep the death explosion's burst of fireballs animating through the
    // pause instead of freezing at full brightness until play resumes.
    update_explosions(g, dt);
    if g.dead_timer.tick(dt) {
        g.bullets.iter_mut().for_each(|b| b.active = false);
        if g.sess.lives <= 0 {
            g.dead_timer.start(GAME_OVER_MIN_WAIT);
            g.state = State::Over;
        } else {
            g.respawn_grace.start(RESPAWN_GRACE_SECS);
            g.state = State::Play;
        }
    }
}

fn update_clear(g: &mut Game, dt: f32) {
    if g.dead_timer.tick(dt) { g.start_wave(); }
}

fn update_over(g: &mut Game, dt: f32) {
    if play_again(&mut g.dead_timer, dt) { g.start_game(); }
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("DEFENDER", WIN_W, WIN_H)
}

#[blip::macroquad::main(conf)]
async fn main() {
    blip::bot::playthroughs(bot::PLAYTHROUGHS);
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let art = Art::load();

    let sfx = Sounds::load().await;
    // The invasion tune, and the mothership's on boss levels.
    let mut music = Jukebox::new("defender", &["INVASION", "MOTHERSHIP", "OCCUPATION"]);

    if blip.screenshot_mode {
        g.start_game();
    }

    loop {
        let dt = blip.delta_time;

        // The mothership has its own tune; the other waves take the two marches in turn.
        music.play(if g.state == State::Title { 0 } else if g.is_boss_level() { 1 } else if g.sess.level % 2 == 0 { 2 } else { 0 });
        g.touch_x = blip.touch(0).map(|p| p.x);
        if blip::bot::active() { bot::drive(&g); }
        let prev_state = g.state;
        match g.state {
            State::Title => update_title(&mut g),
            State::Play  => update_play(&mut g, dt, &sfx),
            State::Dead  => update_dead(&mut g, dt),
            State::Win   => update_clear(&mut g, dt),
            State::Over  => update_over(&mut g, dt),
        }
        if prev_state == State::Play && matches!(g.state, State::Dead | State::Over) { web::haptic(); }
        // A broken run of 2 or more shows a moment in red where the streak was, so the rule that broke it is seen.
        if let Some(n) = g.streak.broken().filter(|&n| n >= 2) { g.lost_run = (n, LOST_RUN_SECS); }
        g.lost_run.1 -= dt;
        if prev_state != State::Over && g.state == State::Over {
            // A new best ends on the level-clear jingle, not the game-over one.
            play_sfx(if g.sess.beats_best() { &sfx.level_clear } else { &sfx.game_over });
            g.sess.report();
        }

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title => draw_title(&blip, &art.alien),
            State::Win   => draw_clear(&blip, g.sess.level, 1.5 - g.dead_timer.remaining()),
            State::Over  => {
                draw_backdrop(&blip, blip::now());
                blip.draw_game_over(g.sess.score, web::high_score(), BLIP_RED, BLIP_GREEN, BLIP_YELLOW, !g.dead_timer.active());
            }
            State::Play | State::Dead => draw_play(&blip, &g, &art),
        }

        blip.next_frame().await;
    }
}

#[cfg(test)]
mod tests;
