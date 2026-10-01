//! Bouncer (Breakout), Rust port of `games/bouncer/main.c` on macroquad.

use std::f32::consts::PI;

use blip::input::{
    btn1_pressed, key_active, key_pressed, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::rand::rand;
use blip::macroquad::texture::Texture2D;
use blip::{
    GAME_OVER_MIN_WAIT,
    load_png, load_png_smooth,
    clamp, Fx, Jukebox, play_sfx, play_sfx_volume, pool_iter, pool_iter_mut, pool_spawn, rects_overlap, web,
    window_conf, Blip, BlipColor, LifeResult, Pooled, Session, Timer, BLIP_BLACK, BLIP_CYAN,
    BLIP_GRAY, BLIP_GREEN, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
};

#[cfg(not(target_arch = "wasm32"))]
mod bot;

// ---- layout -----------------------------------------------------------
const WIN_W: i32 = 480;
const WIN_H: i32 = 540;
const HUD_H: i32 = 28;

// ---- brick grid -------------------------------------------------------
const BRICK_COLS: i32 = 10;
const BRICK_ROWS: i32 = 6;
const BRICK_W: i32 = 44;
const BRICK_H: i32 = 18;
const BRICK_GAP: i32 = 2;
const BRICK_OX: i32 = (WIN_W - BRICK_COLS * (BRICK_W + BRICK_GAP) + BRICK_GAP) / 2;
const BRICK_OY: i32 = HUD_H + 40;
const BRICK_TOTAL: usize = (BRICK_COLS * BRICK_ROWS) as usize;

// ---- paddle / ball ----------------------------------------------------
const PAD_W: i32 = 80;
const PAD_H: i32 = 12;
const PAD_Y: i32 = WIN_H - 48;
const PAD_SPEED: f32 = 280.0;
// Double-tap a direction to dash for a save: second press within the
// window, speed x(1 + BOOST) easing back to normal over DASH_SECS.
const DASH_WINDOW: f32 = 0.28;
const DASH_BOOST: f32 = 0.9;
const DASH_SECS: f32 = 0.35;
// A finger drags the paddle no faster than a full dash, so touch play gets
// no reach the keys can't match.
const PAD_SPEED_TOUCH: f32 = PAD_SPEED * (1.0 + DASH_BOOST);

const BALL_W: i32 = 18;
const BALL_H: i32 = 18;
const BALL_YAW_N: u32 = 12; // sheet layout: must match blip_assets::bouncer
const BALL_PITCH_N: u32 = 19;
const BALL_SPEED_0: f32 = 240.0;
const BALL_SPEED_MAX: f32 = 420.0;

// ---- loot drops -------------------------------------------------------
const MAX_DROPS: usize = 8;
const PADDLE_CAP: u32 = 12; // must match blip_assets::bouncer
const DROP_W: f32 = 32.0;
const DROP_H: f32 = 18.0;
const DROP_SPEED: f32 = 120.0;
const EFFECT_DURATION: f32 = 8.0;
const PAD_W_WIDE: f32 = 130.0;
const PAD_W_NARROW: f32 = 46.0;
const BALL_SLOW_FACTOR: f32 = 0.6;

// ---- tuning -----------------------------------------------------------
const LIVES_START: i32 = 3;
// Per brick. At 18 the ball hit its cap after 10 of 60 bricks and stayed
// there for the rest of the game; at 7 the cap arrives about halfway through.
const SPEED_INC: f32 = 7.0;
// Taken off the ramp when a life is lost, so the new ball is catchable.
const SPEED_LIFE_RELIEF: f32 = 60.0;

// ---- screwball spin -----------------------------------------------------
// Hitting the ball while the paddle is moving fast puts a spin on it — the
// faster the paddle, the sharper the curve. A stationary or slow-moving
// paddle produces a plain straight shot.
const SCREW_MIN_PAD_SPEED: f32 = 30.0; // below this, no spin at all
const SCREW_MAX_SPIN_RATE: f32 = 1.3;  // radians/sec of curve at full paddle speed
const SCREW_SPIN_DECAY: f32 = 0.9;     // spin bleeds off per second of flight
// However hard it's spinning, never let the ball curve more than this far off
// its post-hit direction — it should bend, not loop back on itself and start
// heading the "wrong" way.
const SCREW_MAX_CURVE: f32 = 0.75; // radians (~43 degrees)

// ---- pull ----------------------------------------------------------------
// The ball keeps pulling the way it is heading vertically, harder the
// flatter it flies, so a shot that would ping-pong wall to wall steepens
// within a second instead. A screwball also gathers speed along its path
// while it flies. The paddle and bricks set the speed back on the ramp.
const PULL: f32 = 70.0;          // px/s^2 along vy
const PULL_FLAT: f32 = 260.0;    // extra when |vy| is under 40% of the speed
const SCREW_GAIN: f32 = 0.18;    // fraction of speed gained per second, spinning
const PULL_CAP: f32 = 1.3;       // times BALL_SPEED_MAX, whatever the pull

// ---- seeking ---------------------------------------------------------------
// The last few bricks of a level could take minutes to hit (a playtest bot
// spent 190 s on the final three). After this long without breaking one, a
// rising ball glows and bends gently toward the nearest brick.
const SEEK_AFTER: f32 = 5.0;
const SEEK_RATE: f32 = 1.5; // radians/sec of bend at most

const BRICK_COLORS: [BlipColor; 8] = [
    BlipColor { r: 0.90, g: 0.25, b: 0.25, a: 1.0 },
    BlipColor { r: 0.95, g: 0.55, b: 0.20, a: 1.0 },
    BlipColor { r: 0.95, g: 0.85, b: 0.30, a: 1.0 },
    BlipColor { r: 0.35, g: 0.80, b: 0.35, a: 1.0 },
    BlipColor { r: 0.30, g: 0.50, b: 0.95, a: 1.0 },
    BlipColor { r: 0.65, g: 0.35, b: 0.90, a: 1.0 },
    BlipColor { r: 0.70, g: 0.72, b: 0.78, a: 1.0 },
    BlipColor { r: 0.70, g: 0.72, b: 0.78, a: 1.0 },
];

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Launch, Play, Dead, Win, Over }

#[derive(Copy, Clone, PartialEq, Eq)]
enum DropKind { Wide, Narrow, Slow, Life }

#[derive(Copy, Clone)]
struct Drop { x: f32, y: f32, active: bool, kind: DropKind }

impl Pooled for Drop {
    fn is_active(&self) -> bool { self.active }
}

const DEAD_DROP: Drop = Drop { x: 0.0, y: 0.0, active: false, kind: DropKind::Wide };

// Steel bricks take two hits to break; `kind` for a steel brick switches from
// BRICK_STEEL to BRICK_STEEL_CRACKED once it's taken its first hit, so the
// color itself shows how much damage it's absorbed.
const BRICK_STEEL: usize = 6;
const BRICK_STEEL_CRACKED: usize = 7;
// Steel bricks only start showing up from this level on, so new players
// meet the plain single-hit grid first.
const STEEL_MIN_LEVEL: i32 = 4;

#[derive(Copy, Clone)]
struct Brick { kind: usize, hp: u8, alive: bool }

struct Game {
    bricks: [Brick; BRICK_TOTAL],
    drops: [Drop; MAX_DROPS],
    pad_x: f32,
    pad_w: f32,
    pad_vx: f32,
    /// Seconds since each direction was last pressed (left, right).
    tap_age: [f32; 2],
    /// Where a finger on the touch strip wants the paddle's centre.
    touch_x: Option<f32>,
    /// -1 / +1 while a dash is on, and how much of it is left.
    dash_dir: f32,
    dash_t: f32,
    pad_effect_timer: Timer,
    slow_timer: Timer,
    ball_x: f32, ball_y: f32,
    ball_vx: f32, ball_vy: f32,
    ball_spin: f32,
    ball_curve_used: f32,
    pad_kick: f32,   // recoil depth (px) from the last ball strike, on a damped spring
    pad_kick_v: f32,
    ball_rot: Mat3, // orientation of the ball's surface pattern in view space
    ball_speed: f32,
    sess: Session,
    dead_timer: Timer,
    state: State,
    /// Seconds since a brick last broke (see SEEK_AFTER).
    since_break: f32,
    fx: Fx,
}

impl Game {
    fn new() -> Self {
        Self {
            bricks: [Brick { kind: 0, hp: 1, alive: false }; BRICK_TOTAL],
            drops: [DEAD_DROP; MAX_DROPS],
            pad_x: 0.0,
            pad_w: PAD_W as f32,
            pad_vx: 0.0,
            tap_age: [f32::MAX; 2],
            touch_x: None,
            dash_dir: 0.0,
            dash_t: 0.0,
            pad_effect_timer: Timer::default(),
            slow_timer: Timer::default(),
            ball_x: 0.0, ball_y: 0.0, ball_vx: 0.0, ball_vy: 0.0,
            ball_spin: 0.0,
            ball_curve_used: 0.0,
            pad_kick: 0.0,
            pad_kick_v: 0.0,
            ball_rot: MAT3_ID,
            ball_speed: BALL_SPEED_0,
            sess: Session::new(LIVES_START),
            dead_timer: Timer::default(),
            state: State::Title,
            since_break: 0.0,
            fx: Fx::new(),
        }
    }

    fn reset_drops(&mut self) {
        self.drops = [DEAD_DROP; MAX_DROPS];
        self.pad_w = PAD_W as f32;
        self.pad_effect_timer = Timer::default();
        self.slow_timer = Timer::default();
    }

    fn bricks_alive(&self) -> i32 {
        self.bricks.iter().filter(|b| b.alive).count() as i32
    }

    /// Eight distinct brick layouts, cycling forever as `level` climbs so the
    /// game doesn't settle into repeating the same pattern from level 3 on.
    fn build_bricks(&mut self) {
        let pattern = (self.sess.level - 1).rem_euclid(8);
        let center_col = (BRICK_COLS - 1) as f32 / 2.0;
        for r in 0..BRICK_ROWS {
            for c in 0..BRICK_COLS {
                let i = (r * BRICK_COLS + c) as usize;
                let dist = (c as f32 - center_col).abs();
                let alive = match pattern {
                    0 => true,                                            // full grid
                    1 => (r + c) % 2 == 0,                                 // checkerboard
                    2 => dist <= 3.0,                                      // diamond
                    3 => dist <= (r + 1) as f32,                           // pyramid, narrow at top
                    4 => c < 2 || c >= BRICK_COLS - 2,                     // two side pillars
                    5 => r % 2 == 0,                                       // horizontal stripes
                    6 => r == 0 || r == BRICK_ROWS - 1 || c == 0 || c == BRICK_COLS - 1, // hollow border
                    _ => {                                                 // X shape
                        let step = (BRICK_COLS - 1) as f32 / (BRICK_ROWS - 1) as f32;
                        let target = r as f32 * step;
                        (c as f32 - target).abs() < 1.0 || (c as f32 - (BRICK_COLS - 1) as f32 + target).abs() < 1.0
                    }
                };
                // Odds a live brick is reinforced steel, ramping up with
                // level and capped so the grid never turns into a slog.
                let steel_chance = ((self.sess.level - STEEL_MIN_LEVEL + 1) as f32 * 0.05)
                    .clamp(0.0, 0.30);
                let is_steel = alive
                    && self.sess.level >= STEEL_MIN_LEVEL
                    && (rand() % 1000) < (steel_chance * 1000.0) as u32;
                let (kind, hp) = if is_steel { (BRICK_STEEL, 2) } else { (r as usize, 1) };
                self.bricks[i] = Brick { kind, hp, alive };
            }
        }
    }

    fn seeking(&self) -> bool { self.state == State::Play && self.since_break > SEEK_AFTER }

    /// Centre of the live brick nearest the ball.
    fn nearest_brick(&self) -> Option<(f32, f32)> {
        let (cx, cy, _) = ball_circle(self);
        (0..BRICK_TOTAL).filter(|&i| self.bricks[i].alive).map(brick_rect)
            .map(|(x, y)| (x + BRICK_W as f32 / 2.0, y + BRICK_H as f32 / 2.0))
            .min_by(|a, b| (a.0 - cx).hypot(a.1 - cy).total_cmp(&(b.0 - cx).hypot(b.1 - cy)))
    }

    fn launch_ball(&mut self) {
        self.since_break = 0.0;
        self.ball_x = self.pad_x + (self.pad_w / 2.0 - BALL_W as f32 / 2.0);
        self.ball_y = (PAD_Y - BALL_H - 2) as f32;
        let r01 = (rand() as f32) / (u32::MAX as f32);
        // 36-43 degrees off vertical, either side, at exactly the ramp's
        // speed (the old (sin, 1) vector served a third too fast).
        let side = if rand() % 2 == 0 { 1.0 } else { -1.0 };
        let (s, c) = (0.62 + r01 * 0.12).sin_cos();
        self.ball_vx = side * self.ball_speed * s;
        self.ball_vy = -self.ball_speed * c;
        self.ball_spin = 0.0;
        self.ball_rot = mat_mul(&rot_x(0.5), &rot_y(rand_f() * 6.28));
        self.ball_curve_used = 0.0;
    }

    fn start_game(&mut self) {
        self.sess.reset(LIVES_START);
        self.ball_speed = BALL_SPEED_0;
        self.reset_drops();
        self.pad_x = ((WIN_W - PAD_W) / 2) as f32;
        self.build_bricks();
        self.launch_ball();
        self.state = State::Launch;
    }

    fn next_level(&mut self) {
        self.sess.next_level();
        self.fx.clear();
        self.ball_speed = BALL_SPEED_0;
        self.reset_drops();
        self.build_bricks();
        self.pad_x = ((WIN_W - PAD_W) / 2) as f32;
        self.launch_ball();
        self.state = State::Launch;
    }
}

struct Sounds {
    paddle_hit: [blip::BlipSound; 3],
    brick_hit: [blip::BlipSound; 3],
    brick_break: [blip::BlipSound; 3],
    wall_hit: blip::BlipSound,
    life_lost: blip::BlipSound,
    win: blip::BlipSound,
    pickup_good: blip::BlipSound,
    pickup_bad: blip::BlipSound,
    pickup_life: blip::BlipSound,
}

/// A random take of an impact sound, louder for a harder hit.
fn play_variant(takes: &[blip::BlipSound; 3], speed: f32) {
    let vol = 0.65 + 0.35 * (speed / BALL_SPEED_MAX).min(1.0);
    play_sfx_volume(&takes[rand() as usize % takes.len()], vol);
}

fn update_title(g: &mut Game) {
    if btn1_pressed() {
        web::spend_coin();
        g.start_game();
    }
}

fn paddle_input(g: &mut Game, dt: f32) {
    let prev_x = g.pad_x;
    for (i, pressed) in [
        key_pressed(BLIP_KEY_LEFT) || key_pressed(BLIP_KEY_A),
        key_pressed(BLIP_KEY_RIGHT) || key_pressed(BLIP_KEY_D),
    ].into_iter().enumerate() {
        g.tap_age[i] += dt;
        if !pressed { continue; }
        if g.tap_age[i] < DASH_WINDOW {
            g.dash_dir = if i == 0 { -1.0 } else { 1.0 };
            g.dash_t = DASH_SECS;
            g.tap_age[i] = f32::MAX; // a third tap starts a new pair
        } else {
            g.tap_age[i] = 0.0;
        }
    }
    g.dash_t = (g.dash_t - dt).max(0.0);
    let dash = 1.0 + DASH_BOOST * g.dash_t / DASH_SECS;
    let left = key_active(BLIP_KEY_LEFT) || key_active(BLIP_KEY_A);
    let right = key_active(BLIP_KEY_RIGHT) || key_active(BLIP_KEY_D);
    if let Some(tx) = g.touch_x {
        let step = PAD_SPEED_TOUCH * dt;
        g.pad_x += clamp(tx - g.pad_w / 2.0 - g.pad_x, -step, step);
    } else {
        if left  { g.pad_x -= PAD_SPEED * dt * if g.dash_dir < 0.0 { dash } else { 1.0 }; }
        if right { g.pad_x += PAD_SPEED * dt * if g.dash_dir > 0.0 { dash } else { 1.0 }; }
    }
    g.pad_x = clamp(g.pad_x, 0.0, WIN_W as f32 - g.pad_w);
    // Actual on-screen speed this frame — reads as zero if held against a wall,
    // even with a direction key down, since the paddle isn't really moving.
    g.pad_vx = if dt > 0.0 { (g.pad_x - prev_x) / dt } else { 0.0 };
}

fn update_launch(g: &mut Game, dt: f32) {
    paddle_input(g, dt);
    g.fx.update(dt);
    g.ball_x = g.pad_x + (g.pad_w / 2.0 - BALL_W as f32 / 2.0);
    g.ball_y = (PAD_Y - BALL_H - 2) as f32;
    if key_pressed(BLIP_KEY_SPACE) || key_pressed(BLIP_KEY_UP) || key_pressed(BLIP_KEY_W) {
        g.state = State::Play;
    }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    paddle_input(g, dt);

    // Recoil spring, ~5 Hz and well damped: one dip, no bounce.
    g.pad_kick_v += (-1000.0 * g.pad_kick - 28.0 * g.pad_kick_v) * dt;
    g.pad_kick += g.pad_kick_v * dt;

    if g.pad_effect_timer.tick(dt) { g.pad_w = PAD_W as f32; }
    g.slow_timer.tick(dt);
    update_drops(g, dt, sfx);
    g.fx.update(dt);
    g.since_break += dt;
    seek(g, dt);

    let active_speed = if g.slow_timer.active() {
        g.ball_speed * BALL_SLOW_FACTOR
    } else {
        g.ball_speed
    };

    // ---- screwball curve: rotate the velocity vector (speed preserved) by
    //      the current spin rate, capped at SCREW_MAX_CURVE, spin bleeding
    //      off over time so the curve eases out instead of looping ----
    if g.ball_spin.abs() > 0.001 {
        let remaining = (SCREW_MAX_CURVE - g.ball_curve_used).max(0.0);
        let mut ang = g.ball_spin * dt;
        if ang.abs() > remaining {
            ang = ang.signum() * remaining;
            g.ball_spin = 0.0;
        }
        g.ball_curve_used += ang.abs();
        let (s, c) = ang.sin_cos();
        let (vx, vy) = (g.ball_vx, g.ball_vy);
        g.ball_vx = vx * c - vy * s;
        g.ball_vy = vx * s + vy * c;
        g.ball_spin *= (1.0 - SCREW_SPIN_DECAY * dt).max(0.0);
    }

    // ---- pull: steepen and quicken the flight (see PULL) ----
    {
        let sp = g.ball_vx.hypot(g.ball_vy).max(1.0);
        let flat = (1.0 - (g.ball_vy.abs() / sp) / 0.4).clamp(0.0, 1.0);
        let dir = if g.ball_vy < 0.0 { -1.0 } else { 1.0 };
        g.ball_vy += dir * (PULL + PULL_FLAT * flat) * dt;
        if g.ball_spin.abs() > 0.001 || g.ball_curve_used > 0.0 {
            let k = 1.0 + SCREW_GAIN * dt;
            g.ball_vx *= k;
            g.ball_vy *= k;
        }
        let sp = g.ball_vx.hypot(g.ball_vy);
        let cap = BALL_SPEED_MAX * PULL_CAP;
        if sp > cap { g.ball_vx *= cap / sp; g.ball_vy *= cap / sp; }
    }

    // Rolling-mark spin (visual only): angular rate = speed / radius, driven
    // by the horizontal component so a curving shot visibly spins faster.
    roll_ball(g, dt);

    // ---- integrate in substeps so a fast ball can't tunnel through a brick,
    //      the seam between two bricks, or the paddle in a single frame ----
    let speed = g.ball_vx.hypot(g.ball_vy).max(1.0);
    let substeps = ((speed * dt / (BALL_W as f32 * 0.4)).ceil() as i32).clamp(1, 8);
    let sdt = dt / substeps as f32;

    for _ in 0..substeps {
        g.ball_x += g.ball_vx * sdt;
        g.ball_y += g.ball_vy * sdt;

        ball_walls(g, sfx);
        ball_paddle(g, active_speed, sfx);
        ball_bricks(g, active_speed, sfx);

        if g.ball_y > WIN_H as f32 {
            g.fx.burst(g.ball_x + BALL_W as f32 / 2.0, WIN_H as f32 - 4.0, 14, 160.0, BLIP_RED);
            blip::bot::add("lives_lost", 1.0);
            blip::bot::add(&format!("lost_at_speed{}", (g.ball_vx.hypot(g.ball_vy) / 50.0) as i32 * 50), 1.0);
            play_sfx(&sfx.life_lost);
            g.reset_drops();
            match g.sess.lose_life() {
                LifeResult::StillAlive => { g.dead_timer.start(1.2); g.state = State::Dead; }
                LifeResult::GameOver => {
                    g.dead_timer.start(GAME_OVER_MIN_WAIT);
                    g.state = State::Over;
                    web::report_score(g.sess.score);
                }
            }
            return;
        }
    }

    if g.bricks_alive() == 0 {
        blip::bot::set("cleared", g.sess.level as f64);
        blip::bot::set(&format!("t_clear{}", g.sess.level), blip::bot::clock() as f64);
        play_sfx(&sfx.win);
        g.dead_timer.start(1.5);
        g.state = State::Win;
    }
}

/// The top-left corner of brick `i`.
fn brick_rect(i: usize) -> (f32, f32) {
    let (row, col) = (i as i32 / BRICK_COLS, i as i32 % BRICK_COLS);
    ((BRICK_OX + col * (BRICK_W + BRICK_GAP)) as f32, (BRICK_OY + row * (BRICK_H + BRICK_GAP)) as f32)
}

/// While seeking, bend a rising ball toward the nearest brick.
fn seek(g: &mut Game, dt: f32) {
    if !g.seeking() || g.ball_vy >= 0.0 { return; }
    let Some((tx, ty)) = g.nearest_brick() else { return };
    let (cx, cy, _) = ball_circle(g);
    let want = (ty - cy).atan2(tx - cx);
    let have = g.ball_vy.atan2(g.ball_vx);
    let diff = (want - have + PI).rem_euclid(2.0 * PI) - PI;
    let turn = diff.clamp(-SEEK_RATE * dt, SEEK_RATE * dt);
    let (s, c) = turn.sin_cos();
    let (vx, vy) = (g.ball_vx, g.ball_vy);
    g.ball_vx = vx * c - vy * s;
    g.ball_vy = vx * s + vy * c;
}

type Mat3 = [[f32; 3]; 3];
const MAT3_ID: Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

fn mat_mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut o = [[0.0; 3]; 3];
    for i in 0..3 { for j in 0..3 { for k in 0..3 { o[i][j] += a[i][k] * b[k][j]; } } }
    o
}
fn rot_x(t: f32) -> Mat3 { let (s, c) = t.sin_cos(); [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]] }
fn rot_y(t: f32) -> Mat3 { let (s, c) = t.sin_cos(); [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]] }
fn rand_f() -> f32 { (rand() % 10_000) as f32 / 10_000.0 }

/// Rolls the ball without slipping: its surface drifts along the velocity
/// (axis = view-normal x v), plus the screwball spin about the view axis.
fn roll_ball(g: &mut Game, dt: f32) {
    let r = BALL_W as f32 * 0.5;
    let w = [-g.ball_vy / r, g.ball_vx / r, g.ball_spin * 2.0];
    let ang = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt() * dt;
    if ang < 1e-6 { return; }
    let m = ang / dt;
    let (n0, n1, n2) = (w[0] / m, w[1] / m, w[2] / m);
    let (s, c) = ang.sin_cos();
    let t = 1.0 - c;
    let d: Mat3 = [
        [c + n0 * n0 * t,      n0 * n1 * t - n2 * s, n0 * n2 * t + n1 * s],
        [n1 * n0 * t + n2 * s, c + n1 * n1 * t,      n1 * n2 * t - n0 * s],
        [n2 * n0 * t - n1 * s, n2 * n1 * t + n0 * s, c + n2 * n2 * t],
    ];
    g.ball_rot = mat_mul(&d, &g.ball_rot);
    // Re-orthonormalise the rows so float drift never skews the sphere.
    let m = &mut g.ball_rot;
    for i in 0..3 {
        for j in 0..i {
            let dot: f32 = (0..3).map(|k| m[i][k] * m[j][k]).sum();
            for k in 0..3 { m[i][k] -= dot * m[j][k]; }
        }
        let len = m[i].iter().map(|v| v * v).sum::<f32>().sqrt();
        for k in 0..3 { m[i][k] /= len; }
    }
}

/// Split the orientation into sheet cell (yaw, pitch) and a roll angle:
/// R = Rz(roll) * Rx(pitch) * Ry(yaw).
fn ball_pose(m: &Mat3) -> (u32, u32, f32) {
    let pitch = m[2][1].clamp(-1.0, 1.0).asin();
    let yaw = (-m[2][0]).atan2(m[2][2]);
    let roll = m[1][0].atan2(m[0][0]) - (pitch.sin() * yaw.sin()).atan2(yaw.cos());
    let period = 2.0 * PI / 3.0;
    let col = ((yaw.rem_euclid(period) / period * BALL_YAW_N as f32).round() as u32) % BALL_YAW_N;
    let row = (((pitch + PI / 2.0) / PI * (BALL_PITCH_N - 1) as f32).round() as u32).min(BALL_PITCH_N - 1);
    (col, row, roll)
}

/// The ball as a circle: centre and radius.
#[inline]
fn ball_circle(g: &Game) -> (f32, f32, f32) {
    let r = BALL_W as f32 / 2.0;
    (g.ball_x + r, g.ball_y + r, r)
}

/// Left / right / top walls. The overshoot past the wall is reflected back
/// rather than clamped flat, so no speed is lost to the ball "sticking".
fn ball_walls(g: &mut Game, sfx: &Sounds) {
    if g.ball_x < 0.0 {
        g.ball_x = -g.ball_x;
        g.ball_vx = g.ball_vx.abs();
        play_sfx_volume(&sfx.wall_hit, 0.55);
    }
    let right = (WIN_W - BALL_W) as f32;
    if g.ball_x > right {
        g.ball_x = 2.0 * right - g.ball_x;
        g.ball_vx = -g.ball_vx.abs();
        play_sfx_volume(&sfx.wall_hit, 0.55);
    }
    let top = HUD_H as f32;
    if g.ball_y < top {
        g.ball_y = 2.0 * top - g.ball_y;
        g.ball_vy = g.ball_vy.abs();
        play_sfx_volume(&sfx.wall_hit, 0.55);
    }
}

fn ball_paddle(g: &mut Game, speed: f32, sfx: &Sounds) {
    if g.ball_vy <= 0.0 {
        return;
    }
    if !rects_overlap(
        g.ball_x, g.ball_y, BALL_W as f32, BALL_H as f32,
        g.pad_x, PAD_Y as f32, g.pad_w, PAD_H as f32,
    ) {
        return;
    }
    play_variant(&sfx.paddle_hit, speed);
    web::haptic();
    g.fx.ring(g.ball_x + BALL_W as f32 / 2.0, PAD_Y as f32, 22.0, 0.25, BlipColor { r: 0.6, g: 0.85, b: 1.0, a: 0.8 });
    g.pad_kick_v = 40.0 + 70.0 * (speed / BALL_SPEED_MAX).min(1.0);
    let incoming_vx = g.ball_vx;

    // Where it landed across the face, -1 (left tip) .. +1 (right tip). The
    // classic "aim by where you hit" english, plus a slice of the paddle's
    // own sideways speed carried into the ball the way a moving bat drags it.
    let rel = ((g.ball_x + BALL_W as f32 / 2.0 - g.pad_x) / g.pad_w - 0.5) * 2.0;
    let steer = rel.clamp(-1.0, 1.0) * 1.15; // radians
    let nvx = speed * steer.sin() + g.pad_vx * 0.18;
    let mut nvy = -(speed * steer.cos()).abs();
    if nvy > -speed * 0.32 {
        nvy = -speed * 0.32; // keep it from grazing along just over the paddle
    }
    // Renormalise: the english + paddle drag changed |v|; the ball's speed
    // stays exactly on the ramp.
    let m = (nvx * nvx + nvy * nvy).sqrt().max(1.0);
    g.ball_vx = nvx / m * speed;
    g.ball_vy = nvy / m * speed;
    g.ball_y = (PAD_Y - BALL_H) as f32 - 0.5;

    // Screwball spin from the slip between the paddle's surface and the
    // ball's — their *relative* horizontal velocity, not the paddle's alone.
    let rel_vx = g.pad_vx - incoming_vx;
    g.ball_spin = if rel_vx.abs() > SCREW_MIN_PAD_SPEED {
        let t = (rel_vx.abs() / (PAD_SPEED + BALL_SPEED_MAX)).min(1.0);
        rel_vx.signum() * t * SCREW_MAX_SPIN_RATE
    } else {
        0.0
    };
    g.ball_curve_used = 0.0;
}

/// Circle-vs-rectangle against the brick grid — one contact per call. The
/// bounce normal is the direction from the closest point on the brick to the
/// ball's centre, so a glancing hit on a brick's corner deflects along the
/// real diagonal instead of snapping to a pure horizontal / vertical bounce.
fn ball_bricks(g: &mut Game, speed: f32, sfx: &Sounds) {
    let (cx, cy, rad) = ball_circle(g);

    for i in 0..BRICK_TOTAL {
        if !g.bricks[i].alive {
            continue;
        }
        let row = i as i32 / BRICK_COLS;
        let col = i as i32 % BRICK_COLS;
        let bx = (BRICK_OX + col * (BRICK_W + BRICK_GAP)) as f32;
        let by = (BRICK_OY + row * (BRICK_H + BRICK_GAP)) as f32;

        let px = cx.clamp(bx, bx + BRICK_W as f32);
        let py = cy.clamp(by, by + BRICK_H as f32);
        let (dx, dy) = (cx - px, cy - py);
        let d2 = dx * dx + dy * dy;
        if d2 >= rad * rad {
            continue;
        }

        let d = d2.sqrt();
        let (nx, ny) = if d > 0.001 {
            (dx / d, dy / d)
        } else {
            // centre buried in the brick — eject along the shallowest axis
            let ox = (cx - bx).min(bx + BRICK_W as f32 - cx);
            let oy = (cy - by).min(by + BRICK_H as f32 - cy);
            if ox < oy {
                (if cx < bx + BRICK_W as f32 / 2.0 { -1.0 } else { 1.0 }, 0.0)
            } else {
                (0.0, if cy < by + BRICK_H as f32 / 2.0 { -1.0 } else { 1.0 })
            }
        };

        // reflect about the contact normal, then lift the ball clear of the
        // brick and put its speed back exactly on the ramp
        let vn = g.ball_vx * nx + g.ball_vy * ny;
        if vn < 0.0 {
            g.ball_vx -= 2.0 * vn * nx;
            g.ball_vy -= 2.0 * vn * ny;
        }
        let pen = rad - d + 0.5;
        g.ball_x += nx * pen;
        g.ball_y += ny * pen;
        let m = g.ball_vx.hypot(g.ball_vy).max(1.0);
        g.ball_vx = g.ball_vx / m * speed;
        g.ball_vy = g.ball_vy / m * speed;

        g.bricks[i].hp -= 1;
        let (mx, my) = (bx + BRICK_W as f32 / 2.0, by + BRICK_H as f32 / 2.0);
        if g.bricks[i].hp == 0 {
            g.bricks[i].alive = false;
            g.since_break = 0.0;
            let points = (BRICK_ROWS - row) * 10 * g.sess.level;
            g.sess.add_score(points);
            g.fx.burst(mx, my, 10, 120.0, BRICK_COLORS[g.bricks[i].kind]);
            g.fx.popup(mx, my, &format!("{points}"), BLIP_WHITE);
            // Same ramp on every level, so the ball plays identically no
            // matter how far the player has gotten.
            g.ball_speed = clamp(g.ball_speed + SPEED_INC, 0.0, BALL_SPEED_MAX);

            // 30% chance to spawn a loot drop; one in ten of those a life
            // (at one in five a steady player ended level 1 with six).
            if rand() % 10 < 3 {
                let drop_x = bx + BRICK_W as f32 / 2.0 - DROP_W / 2.0;
                let drop_kind = match rand() % 10 {
                    0..=2 => DropKind::Wide,
                    3..=5 => DropKind::Slow,
                    6..=8 => DropKind::Narrow,
                    _ => DropKind::Life,
                };
                pool_spawn(&mut g.drops, Drop { x: drop_x, y: by, active: true, kind: drop_kind });
            }
            play_variant(&sfx.brick_break, speed);
        } else {
            // Steel brick survived — the cracked texture warns the next hit
            // finishes it.
            g.bricks[i].kind = BRICK_STEEL_CRACKED;
            g.fx.burst(mx, my, 5, 70.0, BRICK_COLORS[BRICK_STEEL]);
            play_variant(&sfx.brick_hit, speed);
        }
        return;
    }
}

fn update_drops(g: &mut Game, dt: f32, sfx: &Sounds) {
    for d in pool_iter_mut(&mut g.drops) {
        d.y += DROP_SPEED * dt;
        if d.y > WIN_H as f32 { d.active = false; continue; }
        if rects_overlap(d.x, d.y, DROP_W, DROP_H,
                         g.pad_x, PAD_Y as f32, g.pad_w, PAD_H as f32) {
            d.active = false;
            blip::bot::add(["got_wide", "got_narrow", "got_slow", "got_life"][d.kind as usize], 1.0);
            let (label, c) = match d.kind {
                DropKind::Wide => ("WIDE", BLIP_GREEN),
                DropKind::Narrow => ("NARROW", BLIP_RED),
                DropKind::Slow => ("SLOW", BLIP_CYAN),
                DropKind::Life => ("+1 LIFE", BLIP_YELLOW),
            };
            g.fx.popup(d.x + DROP_W / 2.0, PAD_Y as f32 - 14.0, label, c);
            play_sfx(match d.kind {
                DropKind::Wide | DropKind::Slow => &sfx.pickup_good,
                DropKind::Narrow => &sfx.pickup_bad,
                DropKind::Life => &sfx.pickup_life,
            });
            match d.kind {
                DropKind::Wide => {
                    g.pad_w = PAD_W_WIDE;
                    g.pad_effect_timer.start(EFFECT_DURATION);
                }
                DropKind::Narrow => {
                    g.pad_w = PAD_W_NARROW;
                    g.pad_effect_timer.start(EFFECT_DURATION);
                }
                DropKind::Slow => {
                    g.slow_timer.start(EFFECT_DURATION);
                }
                DropKind::Life => {
                    g.sess.lives += 1;
                }
            }
        }
    }
}

fn update_dead(g: &mut Game, dt: f32) {
    g.fx.update(dt);
    if g.dead_timer.tick(dt) {
        g.ball_speed = (g.ball_speed - SPEED_LIFE_RELIEF).max(BALL_SPEED_0);
        g.pad_x = ((WIN_W - PAD_W) / 2) as f32;
        g.launch_ball();
        g.state = State::Launch;
    }
}

fn update_win(g: &mut Game, dt: f32) {
    if g.dead_timer.tick(dt) { g.next_level(); }
}

fn update_over(g: &mut Game, dt: f32) {
    g.dead_timer.tick(dt);
    if g.dead_timer.active() || !btn1_pressed() { return; }
    web::spend_coin();
    g.start_game();
}

fn draw_play(blip: &Blip, g: &Game, paddle: &Texture2D, ball: &Texture2D, shade: &Texture2D, brick: &[Texture2D; 8], drops: &[Texture2D; 4]) {
    for i in 0..BRICK_TOTAL {
        if !g.bricks[i].alive { continue; }
        let r = i as i32 / BRICK_COLS;
        let c = i as i32 % BRICK_COLS;
        let bx = (BRICK_OX + c * (BRICK_W + BRICK_GAP)) as f32;
        let by = (BRICK_OY + r * (BRICK_H + BRICK_GAP)) as f32;
        blip.draw_texture(&brick[g.bricks[i].kind], bx, by, BRICK_W as f32, BRICK_H as f32);
    }

    // Pickups sway as they fall.
    for d in pool_iter(&g.drops) {
        let phase = d.y * 0.045 + d.x;
        blip.draw_texture_cell(&drops[d.kind as usize], 0, 1, 0, 1, d.x, d.y, DROP_W, DROP_H, phase.sin() * 0.16);
    }

    // Draw active effect indicators
    let indicator_y = (PAD_Y - 20) as f32;
    if g.slow_timer.active() {
        blip.draw_centered("SLOW", indicator_y, 2.0, BLIP_CYAN);
    } else if g.pad_effect_timer.active() {
        if g.pad_w > PAD_W as f32 {
            blip.draw_centered("WIDE",   indicator_y, 2.0, BLIP_GREEN);
        } else {
            blip.draw_centered("NARROW", indicator_y, 2.0, BLIP_RED);
        }
    }

    // Left cap, stretched middle, right cap: the ends keep their shape at any width.
    let (cap, py) = (PADDLE_CAP as f32, PAD_Y as f32 + g.pad_kick);
    let (sw, cap_w) = (paddle.width(), PADDLE_CAP as f32 * 0.5);
    let ph = PAD_H as f32;
    // Dash afterimages, trailing behind and fading with the boost.
    if g.dash_t > 0.0 {
        let k = g.dash_t / DASH_SECS;
        for i in 1..=3 {
            let x = g.pad_x - g.dash_dir * i as f32 * 9.0;
            let c = BlipColor { r: 0.35, g: 0.75, b: 1.0, a: 0.28 * k / i as f32 };
            blip.fill_rect(x + 2.0, py + 2.0, g.pad_w - 4.0, ph - 4.0, c);
        }
    }
    blip.draw_texture_region(paddle, 0.0, 0.0, cap, 24.0, g.pad_x, py, cap_w, ph);
    blip.draw_texture_region(paddle, cap, 0.0, sw - 2.0 * cap, 24.0, g.pad_x + cap_w, py, g.pad_w - 2.0 * cap_w, ph);
    blip.draw_texture_region(paddle, sw - cap, 0.0, cap, 24.0, g.pad_x + g.pad_w - cap_w, py, cap_w, ph);

    // Soft drop shadow, offset toward the direction of travel, so the ball
    // reads as rolling across the play field rather than floating over it.
    let ball_cx = g.ball_x + BALL_W as f32 / 2.0;
    let ball_cy = g.ball_y + BALL_H as f32 / 2.0;
    let speed = (g.ball_vx * g.ball_vx + g.ball_vy * g.ball_vy).sqrt().max(1.0);
    let shadow_ox = (g.ball_vx / speed) * 4.0;
    let shadow_oy = (g.ball_vy / speed) * 4.0 + 3.0;
    blip.fill_circle(
        ball_cx + shadow_ox, ball_cy + shadow_oy, BALL_W as f32 * 0.42,
        BlipColor { r: 0.0, g: 0.0, b: 0.0, a: 0.35 },
    );

    // Seeking: the ball glows and the brick it is bending toward pulses.
    if g.seeking() {
        let pulse = 0.5 + 0.5 * (blip::macroquad::time::get_time() as f32 * 8.0).sin();
        blip.fill_glow_circle(ball_cx, ball_cy, BALL_W as f32 * 0.55, BlipColor { r: 1.0, g: 0.9, b: 0.5, a: 0.35 });
        if let Some((tx, ty)) = g.nearest_brick() {
            let (x, y) = (tx - BRICK_W as f32 / 2.0 - 2.0, ty - BRICK_H as f32 / 2.0 - 2.0);
            blip.draw_rect(x, y, BRICK_W as f32 + 4.0, BRICK_H as f32 + 4.0, BlipColor { r: 1.0, g: 0.95, b: 0.6, a: pulse });
        }
    }

    let (col, row, roll) = ball_pose(&g.ball_rot);
    let (bx, by, bs) = (g.ball_x - 1.0, g.ball_y - 1.0, BALL_W as f32 + 2.0);
    blip.draw_texture_cell(ball, col, BALL_YAW_N, row, BALL_PITCH_N, bx, by, bs, bs, roll);
    blip.draw_texture(shade, bx, by, bs, bs);

    g.fx.draw(blip);
    blip.draw_hud(g.sess.score, g.sess.lives);
}

fn draw_title(blip: &Blip, hi: &web::HighScore) {
    blip.clear(BLIP_BLACK);
    blip.draw_centered("BOUNCER",                 (WIN_H / 4) as f32,         6.0, BLIP_CYAN);
    // BOUNCER is sz=6 (42px tall) — clear its bottom by a real margin.
    blip.draw_hi(hi, (WIN_H / 4 + 50) as f32, BLIP_YELLOW);
    blip.draw_centered("PRESS FIRE",              (WIN_H / 2) as f32,         3.0, BLIP_WHITE);
    blip.draw_centered("LEFT RIGHT ARROW OR AD",  (WIN_H * 2 / 3) as f32,     2.0, BLIP_GRAY);
    blip.draw_centered("SPACE TO LAUNCH",         (WIN_H * 2 / 3 + 20) as f32, 2.0, BLIP_GRAY);
    blip.draw_centered("DOUBLE TAP TO DASH",      (WIN_H * 2 / 3 + 40) as f32, 2.0, BLIP_GRAY);
}

fn draw_win(blip: &Blip, level: i32) {
    let buf = format!("LEVEL {}", level + 1);  // level just incremented in next_level
    blip.clear(BLIP_BLACK);
    blip.draw_centered("CLEARED", (WIN_H / 3) as f32, 5.0, BLIP_GREEN);
    blip.draw_centered(&buf,      (WIN_H / 2) as f32, 3.0, BLIP_YELLOW);
}

fn draw_over(blip: &Blip, score: i32, hi: &web::HighScore, waiting: bool) {
    blip.clear(BLIP_BLACK);
    blip.draw_game_over(score, hi, BLIP_RED, BLIP_GREEN, BLIP_YELLOW, !waiting);
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("BOUNCER", WIN_W, WIN_H)
}

const PADDLE_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/paddle.png"));
const DROP_PNGS: [&[u8]; 4] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/drop_wide.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/drop_narrow.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/drop_slow.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/drop_life.png")),
];
const BALL_SHADE_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/ball_shade.png"));
const BALL_PNG:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/ball.png"));
const BRICK_RED_PNG:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_red.png"));
const BRICK_ORANGE_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_orange.png"));
const BRICK_YELLOW_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_yellow.png"));
const BRICK_GREEN_PNG:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_green.png"));
const BRICK_BLUE_PNG:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_blue.png"));
const BRICK_PURPLE_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_purple.png"));
const BRICK_STEEL_PNG:         &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_steel.png"));
const BRICK_STEEL_CRACKED_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_steel_cracked.png"));
const PADDLE_HIT_WAV: [&[u8]; 3] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/paddle_hit_0.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/paddle_hit_1.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/paddle_hit_2.wav")),
];
const BRICK_HIT_WAV: [&[u8]; 3] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/brick_hit_0.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/brick_hit_1.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/brick_hit_2.wav")),
];
const BRICK_BREAK_WAV: [&[u8]; 3] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/brick_break_0.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/brick_break_1.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/brick_break_2.wav")),
];
const WALL_HIT_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/wall_hit.wav"));
const LIFE_LOST_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/life_lost.wav"));
const WIN_WAV:         &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/win.wav"));
const PICKUP_GOOD_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/pickup_good.wav"));
const PICKUP_BAD_WAV:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/pickup_bad.wav"));
const PICKUP_LIFE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/pickup_life.wav"));


async fn load_takes(wavs: &[&'static [u8]; 3]) -> [blip::BlipSound; 3] {
    [
        blip::audio::load_sound(wavs[0]).await,
        blip::audio::load_sound(wavs[1]).await,
        blip::audio::load_sound(wavs[2]).await,
    ]
}


#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let paddle = load_png_smooth(PADDLE_PNG);
    let ball = load_png_smooth(BALL_PNG);
    let drops = DROP_PNGS.map(load_png_smooth);
    let ball_shade = load_png_smooth(BALL_SHADE_PNG);
    let brick = [
        load_png(BRICK_RED_PNG),
        load_png(BRICK_ORANGE_PNG),
        load_png(BRICK_YELLOW_PNG),
        load_png(BRICK_GREEN_PNG),
        load_png(BRICK_BLUE_PNG),
        load_png(BRICK_PURPLE_PNG),
        load_png(BRICK_STEEL_PNG),
        load_png(BRICK_STEEL_CRACKED_PNG),
    ];

    let sfx = Sounds {
        paddle_hit:  load_takes(&PADDLE_HIT_WAV).await,
        brick_hit:   load_takes(&BRICK_HIT_WAV).await,
        brick_break: load_takes(&BRICK_BREAK_WAV).await,
        wall_hit:    blip::audio::load_sound(WALL_HIT_WAV).await,
        life_lost:   blip::audio::load_sound(LIFE_LOST_WAV).await,
        win:         blip::audio::load_sound(WIN_WAV).await,
        pickup_good: blip::audio::load_sound(PICKUP_GOOD_WAV).await,
        pickup_bad:  blip::audio::load_sound(PICKUP_BAD_WAV).await,
        pickup_life: blip::audio::load_sound(PICKUP_LIFE_WAV).await,
    };
    // Two tunes, alternating by level (synthesised here, see blip_assets::bouncer).
    let mut music = Jukebox::new(&[blip_assets::bouncer::bounce_wav, blip_assets::bouncer::rebound_wav]);
    music.start(0).await;

    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;

        music.play(if g.state == State::Title { 0 } else { (g.sess.level as usize + 1) % 2 });
        if g.state != State::Play { music.warm_up().await; }

        if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 {
                g.start_game();
                for (i, kind) in [DropKind::Wide, DropKind::Narrow, DropKind::Slow, DropKind::Life].into_iter().enumerate() {
                    pool_spawn(&mut g.drops, Drop { x: 90.0 + i as f32 * 90.0, y: 250.0 + i as f32 * 25.0, active: true, kind });
                }
            }
        }

        g.touch_x = blip.touch(0).map(|p| p.x);
        #[cfg(not(target_arch = "wasm32"))]
        if blip::bot::active() { bot::drive(&g, blip::bot::clock()); }
        match g.state {
            State::Title  => update_title(&mut g),
            State::Launch => update_launch(&mut g, dt),
            State::Play   => update_play(&mut g, dt, &sfx),
            State::Dead   => update_dead(&mut g, dt),
            State::Win    => update_win(&mut g, dt),
            State::Over   => update_over(&mut g, dt),
        }

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title => draw_title(&blip, &web::high_score()),
            State::Win   => draw_win(&blip, g.sess.level),
            State::Over  => draw_over(&blip, g.sess.score, &web::high_score(), g.dead_timer.active()),
            State::Launch | State::Play | State::Dead => {
                draw_play(&blip, &g, &paddle, &ball, &ball_shade, &brick, &drops);
                // The ball waits on the paddle after every lost life; say so.
                if g.state == State::Launch && (blip::macroquad::time::get_time() * 2.0) as i64 % 2 == 0 {
                    blip.draw_centered("PRESS FIRE", (PAD_Y - 90) as f32, 3.0, BLIP_WHITE);
                }
            }
        }

        blip.next_frame(60).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rot_z(t: f32) -> Mat3 { let (s, c) = t.sin_cos(); [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]] }

    #[test]
    fn pose_decomposition_reconstructs_orientation() {
        let r = mat_mul(&rot_z(0.7), &mat_mul(&rot_x(0.4), &rot_y(1.1)));
        let pitch = r[2][1].asin();
        let yaw = (-r[2][0]).atan2(r[2][2]);
        let roll = r[1][0].atan2(r[0][0]) - (pitch.sin() * yaw.sin()).atan2(yaw.cos());
        assert!((pitch - 0.4).abs() < 1e-4 && (yaw - 1.1).abs() < 1e-4 && (roll - 0.7).abs() < 1e-4);
    }

    #[test]
    fn rolling_right_drifts_surface_right() {
        let mut g = Game::new();
        g.ball_vx = 100.0;
        roll_ball(&mut g, 0.05);
        // The view-facing point (0,0,1) should move toward +x.
        assert!(g.ball_rot[0][2] > 0.0 && g.ball_rot[1][2].abs() < 1e-4);
    }
}
