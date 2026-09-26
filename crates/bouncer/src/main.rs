//! Bouncer (Breakout), Rust port of `games/bouncer/main.c` on macroquad.

use std::f32::consts::PI;

use blip::input::{
    btn1_pressed, key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::prelude::ImageFormat;
use blip::macroquad::rand::rand;
use blip::macroquad::texture::{FilterMode, Texture2D};
use blip::{
    clamp, play_music, play_sfx, pool_iter, pool_iter_mut, pool_spawn, rects_overlap, web,
    window_conf, Blip, BlipColor, LifeResult, Pooled, Session, Timer, BLIP_BLACK, BLIP_CYAN,
    BLIP_GRAY, BLIP_GREEN, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
};

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

const BALL_W: i32 = 18;
const BALL_H: i32 = 18;
const BALL_YAW_N: u32 = 12; // sheet layout: must match blip_assets::bouncer
const BALL_PITCH_N: u32 = 19;
const BALL_SPEED_0: f32 = 240.0;
const BALL_SPEED_MAX: f32 = 420.0;

// ---- loot drops -------------------------------------------------------
const MAX_DROPS: usize = 8;
const DROP_SIZE: f32 = 14.0;
const DROP_SPEED: f32 = 120.0;
const EFFECT_DURATION: f32 = 8.0;
const PAD_W_WIDE: f32 = 130.0;
const PAD_W_NARROW: f32 = 46.0;
const BALL_SLOW_FACTOR: f32 = 0.6;

// ---- tuning -----------------------------------------------------------
const LIVES_START: i32 = 3;
const SPEED_INC: f32 = 18.0;
// A hard floor on how long GAME OVER stays up before a key can dismiss it —
// without this, a direction/launch key still held from the rally that
// killed you bounces straight back to the title screen unread.
const GAME_OVER_MIN_WAIT: f32 = 2.0;

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
    pad_effect_timer: Timer,
    slow_timer: Timer,
    ball_x: f32, ball_y: f32,
    ball_vx: f32, ball_vy: f32,
    ball_spin: f32,
    ball_curve_used: f32,
    ball_rot: Mat3, // orientation of the ball's surface pattern in view space
    ball_speed: f32,
    sess: Session,
    dead_timer: Timer,
    state: State,
}

impl Game {
    fn new() -> Self {
        Self {
            bricks: [Brick { kind: 0, hp: 1, alive: false }; BRICK_TOTAL],
            drops: [DEAD_DROP; MAX_DROPS],
            pad_x: 0.0,
            pad_w: PAD_W as f32,
            pad_vx: 0.0,
            pad_effect_timer: Timer::default(),
            slow_timer: Timer::default(),
            ball_x: 0.0, ball_y: 0.0, ball_vx: 0.0, ball_vy: 0.0,
            ball_spin: 0.0,
            ball_curve_used: 0.0,
            ball_rot: MAT3_ID,
            ball_speed: BALL_SPEED_0,
            sess: Session::new(LIVES_START),
            dead_timer: Timer::default(),
            state: State::Title,
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

    fn launch_ball(&mut self) {
        self.ball_x = self.pad_x + (self.pad_w / 2.0 - BALL_W as f32 / 2.0);
        self.ball_y = (PAD_Y - BALL_H - 2) as f32;
        let r01 = (rand() as f32) / (u32::MAX as f32);
        let angle = -1.1 + r01 * 0.2;
        self.ball_vx = self.ball_speed * (angle + PI / 2.0).cos();
        self.ball_vy = -self.ball_speed;
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
        self.reset_drops();
        self.build_bricks();
        self.pad_x = ((WIN_W - PAD_W) / 2) as f32;
        self.launch_ball();
        self.state = State::Launch;
    }
}

struct Sounds {
    paddle_hit: blip::BlipSound,
    brick_hit: blip::BlipSound,
    brick_break: blip::BlipSound,
    life_lost: blip::BlipSound,
    win: blip::BlipSound,
}

fn update_title(g: &mut Game) {
    if btn1_pressed() { g.start_game(); }
}

fn paddle_input(g: &mut Game, dt: f32) {
    let prev_x = g.pad_x;
    let ps = PAD_SPEED * dt;
    if key_held(BLIP_KEY_LEFT)  || key_held(BLIP_KEY_A) { g.pad_x -= ps; }
    if key_held(BLIP_KEY_RIGHT) || key_held(BLIP_KEY_D) { g.pad_x += ps; }
    g.pad_x = clamp(g.pad_x, 0.0, WIN_W as f32 - g.pad_w);
    // Actual on-screen speed this frame — reads as zero if held against a wall,
    // even with a direction key down, since the paddle isn't really moving.
    g.pad_vx = if dt > 0.0 { (g.pad_x - prev_x) / dt } else { 0.0 };
}

fn update_launch(g: &mut Game, dt: f32) {
    paddle_input(g, dt);
    g.ball_x = g.pad_x + (g.pad_w / 2.0 - BALL_W as f32 / 2.0);
    g.ball_y = (PAD_Y - BALL_H - 2) as f32;
    if key_pressed(BLIP_KEY_SPACE) || key_pressed(BLIP_KEY_UP) || key_pressed(BLIP_KEY_W) {
        g.state = State::Play;
    }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    paddle_input(g, dt);

    if g.pad_effect_timer.tick(dt) { g.pad_w = PAD_W as f32; }
    g.slow_timer.tick(dt);
    update_drops(g, dt);

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

        ball_walls(g);
        ball_paddle(g, active_speed, sfx);
        ball_bricks(g, active_speed, sfx);

        if g.ball_y > WIN_H as f32 {
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
        play_sfx(&sfx.win);
        g.dead_timer.start(1.5);
        g.state = State::Win;
    }
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
fn ball_walls(g: &mut Game) {
    if g.ball_x < 0.0 {
        g.ball_x = -g.ball_x;
        g.ball_vx = g.ball_vx.abs();
    }
    let right = (WIN_W - BALL_W) as f32;
    if g.ball_x > right {
        g.ball_x = 2.0 * right - g.ball_x;
        g.ball_vx = -g.ball_vx.abs();
    }
    let top = HUD_H as f32;
    if g.ball_y < top {
        g.ball_y = 2.0 * top - g.ball_y;
        g.ball_vy = g.ball_vy.abs();
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
    play_sfx(&sfx.paddle_hit);
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
        if g.bricks[i].hp == 0 {
            g.bricks[i].alive = false;
            g.sess.add_score((BRICK_ROWS - row) * 10 * g.sess.level);
            // Same ramp on every level, so the ball plays identically no
            // matter how far the player has gotten.
            g.ball_speed = clamp(g.ball_speed + SPEED_INC, 0.0, BALL_SPEED_MAX);

            // 30% chance to spawn a loot drop
            if rand() % 10 < 3 {
                let drop_x = bx + BRICK_W as f32 / 2.0 - DROP_SIZE / 2.0;
                let drop_kind = match rand() % 10 {
                    0..=2 => DropKind::Wide,
                    3..=5 => DropKind::Slow,
                    6..=7 => DropKind::Narrow,
                    _ => DropKind::Life,
                };
                pool_spawn(&mut g.drops, Drop { x: drop_x, y: by, active: true, kind: drop_kind });
            }
            play_sfx(&sfx.brick_break);
        } else {
            // Steel brick survived — the cracked texture warns the next hit
            // finishes it.
            g.bricks[i].kind = BRICK_STEEL_CRACKED;
            play_sfx(&sfx.brick_hit);
        }
        return;
    }
}

fn update_drops(g: &mut Game, dt: f32) {
    for d in pool_iter_mut(&mut g.drops) {
        d.y += DROP_SPEED * dt;
        if d.y > WIN_H as f32 { d.active = false; continue; }
        if rects_overlap(d.x, d.y, DROP_SIZE, DROP_SIZE,
                         g.pad_x, PAD_Y as f32, g.pad_w, PAD_H as f32) {
            d.active = false;
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
    if g.dead_timer.tick(dt) {
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

fn draw_play(blip: &Blip, g: &Game, paddle: &Texture2D, ball: &Texture2D, shade: &Texture2D, brick: &[Texture2D; 8]) {
    for i in 0..BRICK_TOTAL {
        if !g.bricks[i].alive { continue; }
        let r = i as i32 / BRICK_COLS;
        let c = i as i32 % BRICK_COLS;
        let bx = (BRICK_OX + c * (BRICK_W + BRICK_GAP)) as f32;
        let by = (BRICK_OY + r * (BRICK_H + BRICK_GAP)) as f32;
        blip.draw_texture(&brick[g.bricks[i].kind], bx, by, BRICK_W as f32, BRICK_H as f32);
    }

    // Draw loot drops
    let s = DROP_SIZE;
    for d in pool_iter(&g.drops) {
        let (x, y) = (d.x, d.y);
        match d.kind {
            DropKind::Wide   => blip.draw_rect(x, y + s * 0.3, s, s * 0.4, BLIP_GREEN),
            DropKind::Narrow => blip.draw_rect(x + s * 0.3, y, s * 0.4, s, BLIP_RED),
            DropKind::Slow   => {
                blip.draw_rect(x, y + s * 0.3, s, s * 0.4, BLIP_CYAN);
                blip.draw_rect(x + s * 0.3, y, s * 0.4, s, BLIP_CYAN);
            }
            DropKind::Life   => {
                blip.draw_rect(x, y + s * 0.3, s, s * 0.4, BLIP_YELLOW);
                blip.draw_rect(x + s * 0.3, y, s * 0.4, s, BLIP_YELLOW);
            }
        }
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

    blip.draw_texture(paddle, g.pad_x, PAD_Y as f32, g.pad_w, PAD_H as f32);

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

    let (col, row, roll) = ball_pose(&g.ball_rot);
    let (bx, by, bs) = (g.ball_x - 1.0, g.ball_y - 1.0, BALL_W as f32 + 2.0);
    blip.draw_texture_cell(ball, col, BALL_YAW_N, row, BALL_PITCH_N, bx, by, bs, bs, roll);
    blip.draw_texture(shade, bx, by, bs, bs);

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
}

fn draw_win(blip: &Blip, level: i32) {
    let buf = format!("LEVEL {}", level + 1);  // level just incremented in next_level
    blip.clear(BLIP_BLACK);
    blip.draw_centered("CLEARED", (WIN_H / 3) as f32, 5.0, BLIP_GREEN);
    blip.draw_centered(&buf,      (WIN_H / 2) as f32, 3.0, BLIP_YELLOW);
}

fn draw_over(blip: &Blip, score: i32, hi: &web::HighScore, waiting: bool) {
    let buf = format!("SCORE {score}");
    blip.clear(BLIP_BLACK);
    blip.draw_centered("GAME OVER", (WIN_H / 4) as f32, 5.0, BLIP_RED);
    blip.draw_centered(&buf,        (WIN_H / 2) as f32, 3.0, BLIP_WHITE);
    blip.draw_best(score, hi, (WIN_H / 2 + 28) as f32, BLIP_GREEN);
    if !waiting {
        blip.draw_centered("PRESS FIRE", (WIN_H * 2 / 3) as f32, 3.0, BLIP_YELLOW);
    }
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("BOUNCER", WIN_W, WIN_H)
}

const PADDLE_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/paddle.png"));
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
const PADDLE_HIT_WAV:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/paddle_hit.wav"));
const BRICK_HIT_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/brick_hit.wav"));
const BRICK_BREAK_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/brick_break.wav"));
const LIFE_LOST_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/life_lost.wav"));
const WIN_WAV:         &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/win.wav"));
const MUSIC_WAV:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music.wav"));
const MUSIC2_WAV:      &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music2.wav"));
const MUSIC3_WAV:      &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music3.wav"));
const MUSIC4_WAV:      &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music4.wav"));
const MUSIC5_WAV:      &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music5.wav"));
const MUSIC6_WAV:      &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music6.wav"));
// Six loops in rotation instead of one, so a long session doesn't just hear
// the same ~30s on repeat — see blip_assets::bouncer's music()..music6() for
// what each one is (tech-house, acid, downtempo, trance, funky, dark).
const MUSIC_DURATIONS: [f32; 6] = [30.7262, 29.3388, 29.0500, 31.5512, 32.2471, 30.7922];

fn load_png(bytes: &'static [u8]) -> Texture2D {
    let tex = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
    tex.set_filter(FilterMode::Nearest);
    tex
}

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let paddle = load_png(PADDLE_PNG);
    let ball = load_png(BALL_PNG);
    ball.set_filter(FilterMode::Linear);
    let ball_shade = load_png(BALL_SHADE_PNG);
    ball_shade.set_filter(FilterMode::Linear);
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
        paddle_hit:  blip::audio::load_sound(PADDLE_HIT_WAV).await,
        brick_hit:   blip::audio::load_sound(BRICK_HIT_WAV).await,
        brick_break: blip::audio::load_sound(BRICK_BREAK_WAV).await,
        life_lost:   blip::audio::load_sound(LIFE_LOST_WAV).await,
        win:         blip::audio::load_sound(WIN_WAV).await,
    };
    let music = [
        blip::audio::load_sound(MUSIC_WAV).await,
        blip::audio::load_sound(MUSIC2_WAV).await,
        blip::audio::load_sound(MUSIC3_WAV).await,
        blip::audio::load_sound(MUSIC4_WAV).await,
        blip::audio::load_sound(MUSIC5_WAV).await,
        blip::audio::load_sound(MUSIC6_WAV).await,
    ];
    let mut music_idx: usize = 0;
    let mut music_timer: f32 = MUSIC_DURATIONS[0];
    play_music(&music[0]);

    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;

        // Advance to the next loop in rotation at each track's boundary.
        music_timer -= dt;
        if music_timer <= 0.0 {
            music_idx = (music_idx + 1) % music.len();
            music_timer = MUSIC_DURATIONS[music_idx];
            play_music(&music[music_idx]);
        }

        if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 {
                g.start_game();
            }
        }

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
                draw_play(&blip, &g, &paddle, &ball, &ball_shade, &brick);
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
