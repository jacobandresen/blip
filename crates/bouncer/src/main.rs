//! Breakout-inspired brick breaker.

use std::f32::consts::PI;

use blip::color::*;
use blip::input::{coin_start, key_pressed, Stick, BLIP_KEY_SPACE};
use blip::macroquad::rand::rand;
use blip::macroquad::texture::Texture2D;
use blip::{
    load_png, load_png_smooth,
    clamp, Fx, Jukebox, play_sfx, play_sfx_volume, pool_iter, pool_iter_mut, pool_spawn, rects_overlap, web,
    window_conf, Blip, BlipColor, LifeResult, Pooled, Session, Timer, rand_range_f32, BLIP_BLACK, BLIP_CYAN,
    BLIP_GREEN, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
};

// The effects and music are made by build.rs; the tests check them here.
#[cfg(test)]
mod tunes;
mod bot;
mod levels;
mod draw;
mod ball;
mod bricks;
mod drops;
use drops::*;
use ball::*;
use bricks::*;
use draw::*;

use levels::*;

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
const BALL_YAW_N: u32 = 12; // sheet layout: must match gfx.rs
const BALL_PITCH_N: u32 = 19;
const BALL_SPEED_0: f32 = 240.0;
const BALL_SPEED_MAX: f32 = 420.0;

// ---- loot drops -------------------------------------------------------
const MAX_DROPS: usize = 8;
const PADDLE_CAP: u32 = 12; // must match gfx.rs
const DROP_W: f32 = 32.0;
const DROP_H: f32 = 18.0;
const DROP_SPEED: f32 = 120.0;
const EFFECT_DURATION: f32 = 8.0;
const EFFECT_WARN: f32 = 2.0;   // the indicator blinks for this long before an effect ends
const PAD_W_WIDE: f32 = 130.0;
const PAD_W_NARROW: f32 = 46.0;
const BALL_SLOW_FACTOR: f32 = 0.6;
// MULTI splits the ball into this many, fanned this far apart (radians).
// A ball lost while another is in play costs nothing; the last one a life.
const MULTI_BALLS: usize = 3;
const MULTI_FAN: f32 = 0.45;

// ---- tuning -----------------------------------------------------------
const LIVES_START: i32 = 3;
/// How long a continue is offered at game over.
const CONTINUE_SECS: f32 = 9.0;
// Per brick. At 18 the ball hit its cap after 10 of 60 bricks and stayed
// there for the rest of the game; at 7 the cap arrives about halfway through.
const SPEED_INC: f32 = 7.0;
// Taken off the ramp when a life is lost, so the new ball is catchable.
const SPEED_LIFE_RELIEF: f32 = 60.0;

// Paddle velocity adds spin; faster movement creates a sharper curve.
const SCREW_MIN_PAD_SPEED: f32 = 30.0; // below this, no spin at all
const SCREW_MAX_SPIN_RATE: f32 = 1.3;  // radians/sec of curve at full paddle speed
const SCREW_SPIN_DECAY: f32 = 0.9;     // spin bleeds off per second of flight
// However hard it's spinning, never let the ball curve more than this far off
// its post-hit direction — it should bend, not loop back on itself and start
// heading the "wrong" way.
const SCREW_MAX_CURVE: f32 = 0.75; // radians (~43 degrees)

// Vertical pull steepens flat shots; screwballs gain speed until a paddle or brick resets it.
const PULL: f32 = 70.0;          // px/s^2 along vy
const PULL_FLAT: f32 = 260.0;    // extra when |vy| is under 40% of the speed
const SCREW_GAIN: f32 = 0.18;    // fraction of speed gained per second, spinning
const PULL_CAP: f32 = 1.3;       // times BALL_SPEED_MAX, whatever the pull

// A playtest took 190s to hit the last three bricks; after a delay, rising balls seek_brick the nearest brick.
const SEEK_AFTER: f32 = 5.0;
const SEEK_RATE: f32 = 1.5; // radians/sec of bend at most

// ---- chains -----------------------------------------------------------------
// Each brick the ball breaks before it comes back to the paddle is worth one
// multiple more, up to this: a shot that gets in behind the wall pays.
const CHAIN_MAX: i32 = 5;

const BRICK_COLORS: [BlipColor; 9] = [
    BLIP_HEART,
    BlipColor { r: 0.94, g: 0.54, b: 0.22, a: 1.0 },
    BLIP_GOLD,
    BLIP_LEAF,
    BLIP_SEA,
    BLIP_PLUM,
    BlipColor { r: 0.70, g: 0.72, b: 0.78, a: 1.0 },
    BlipColor { r: 0.70, g: 0.72, b: 0.78, a: 1.0 },
    BlipColor { r: 1.00, g: 0.60, b: 0.15, a: 1.0 },
];

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Launch, Play, Dead, Win, Over }

#[derive(Copy, Clone, PartialEq, Eq)]
enum DropKind { Wide, Narrow, Slow, Life, Multi }

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
// A bomb brick takes the eight round it with it when it breaks (and sets off
// any bomb among them). From level 2, a few to a level.
const BRICK_BOMB: usize = 8;
const BOMB_CHANCE: u32 = 5; // percent of plain bricks
const BOMB_MAX: usize = 4;

#[derive(Copy, Clone)]
struct Brick { kind: usize, hp: u8, alive: bool, wobble: f32 }

/// A shard of a broken brick, tumbling off the bottom of the screen: its outline around its centre.
struct Shard { x: f32, y: f32, vx: f32, vy: f32, turn: f32, spin: f32, pts: Vec<(f32, f32)>, color: BlipColor }

/// The shards' own random numbers, so breaking a brick never moves the game's.
use blip::wav::Rng as ShardRng;

/// One ball's own state. The game's `ball_*` fields are the ball in hand;
/// the others wait in `extra` and are swapped in for their turn.
#[derive(Copy, Clone)]
struct Ball { x: f32, y: f32, vx: f32, vy: f32, spin: f32, curve_used: f32, rot: Mat3 }

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
    /// The other balls in play after a MULTI.
    extra: Vec<Ball>,
    ball_speed: f32,
    sess: Session,
    dead_timer: Timer,
    /// The continue offered at game over, and the best score of the run before one was taken (a continue starts the
    /// score afresh; the run's best is what is reported).
    cont: blip::Continue,
    state: State,
    /// Seconds since a brick last broke (see SEEK_AFTER).
    since_break: f32,
    /// Bricks broken since the ball last left the paddle (see CHAIN_MAX).
    chain: i32,
    fx: Fx,
    shards: Vec<Shard>,
}

impl Game {
    fn new() -> Self {
        Self {
            bricks: [Brick { kind: 0, hp: 1, alive: false, wobble: 0.0 }; BRICK_TOTAL],
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
            extra: Vec::new(),
            ball_speed: BALL_SPEED_0,
            sess: Session::new(LIVES_START),
            dead_timer: Timer::default(),
            cont: blip::Continue::default(),
            state: State::Title,
            since_break: 0.0,
            chain: 0,
            fx: Fx::new(),
            shards: Vec::new(),
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
        let mut bombs = 0;
        let pattern = (self.sess.level - 1).rem_euclid(LAYOUTS);
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
                    4 => !(2..BRICK_COLS - 2).contains(&c),                // two side pillars
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
                let is_bomb = alive && !is_steel && self.sess.level >= BOMB_MIN_LEVEL
                    && bombs < BOMB_MAX && rand() % 100 < BOMB_CHANCE;
                if is_bomb { bombs += 1; }
                let (kind, hp) = if is_steel { (BRICK_STEEL, 2) } else if is_bomb { (BRICK_BOMB, 1) } else { (r as usize, 1) };
                self.bricks[i] = Brick { kind, hp, alive, wobble: 0.0 };
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

    fn ball(&self) -> Ball {
        Ball { x: self.ball_x, y: self.ball_y, vx: self.ball_vx, vy: self.ball_vy,
               spin: self.ball_spin, curve_used: self.ball_curve_used, rot: self.ball_rot }
    }

    fn set_ball(&mut self, b: Ball) {
        (self.ball_x, self.ball_y, self.ball_vx, self.ball_vy) = (b.x, b.y, b.vx, b.vy);
        (self.ball_spin, self.ball_curve_used, self.ball_rot) = (b.spin, b.curve_used, b.rot);
    }

    /// Put extra ball `k` in hand and the one in hand in its place.
    fn swap_ball(&mut self, k: usize) {
        let mine = self.ball();
        let theirs = std::mem::replace(&mut self.extra[k], mine);
        self.set_ball(theirs);
    }

    /// Split the ball in hand until MULTI_BALLS are in play, each copy
    /// turned a fan's width from it.
    fn split_ball(&mut self) {
        let b = self.ball();
        let mut side = 1.0;
        while 1 + self.extra.len() < MULTI_BALLS {
            let (s, c) = (MULTI_FAN * side).sin_cos();
            let (vx, mut vy) = (b.vx * c - b.vy * s, b.vx * s + b.vy * c);
            // Never straight along the paddle line.
            if vy.abs() < b.vx.hypot(b.vy) * 0.3 { vy = -b.vx.hypot(b.vy) * 0.3; }
            self.extra.push(Ball { vx, vy, spin: 0.0, curve_used: 0.0, ..b });
            side = -side;
        }
    }

    fn new_ball(&mut self) {
        self.extra.clear();
        self.since_break = 0.0;
        self.ball_x = self.pad_x + (self.pad_w / 2.0 - BALL_W as f32 / 2.0);
        self.ball_y = (PAD_Y - BALL_H - 2) as f32;
        let r01 = rand_range_f32(0.0, 1.0);
        // 36-43 degrees off vertical, either side, at exactly the ramp's
        // speed (the old (sin, 1) vector served a third too fast).
        let side = if rand().is_multiple_of(2) { 1.0 } else { -1.0 };
        let (s, c) = (0.62 + r01 * 0.12).sin_cos();
        self.ball_vx = side * self.ball_speed * s;
        self.ball_vy = -self.ball_speed * c;
        self.ball_spin = 0.0;
        self.ball_rot = mat_mul(&rot_x(0.5), &rot_y(rand_range_f32(0.0, std::f32::consts::TAU)));
        self.ball_curve_used = 0.0;
    }

    fn start_game(&mut self) {
        self.sess.reset(LIVES_START);
        blip::bot::levels(LAYOUTS as usize);
        self.ball_speed = BALL_SPEED_0;
        self.reset_drops();
        self.pad_x = ((WIN_W - PAD_W) / 2) as f32;
        self.build_bricks();
        self.new_ball();
        self.state = State::Launch;
    }

    /// A continue taken: the same bricks, fresh lives, the score afresh, a new ball on the paddle.
    fn continue_play(&mut self) {
        self.sess.play_on(LIVES_START);
        self.ball_speed = BALL_SPEED_0;
        self.reset_drops();
        self.pad_x = ((WIN_W - PAD_W) / 2) as f32;
        self.new_ball();
        self.state = State::Launch;
    }

    fn next_level(&mut self) {
        self.sess.next_level();
        self.fx.clear();
        self.shards.clear();
        self.ball_speed = BALL_SPEED_0;
        self.reset_drops();
        self.build_bricks();
        self.pad_x = ((WIN_W - PAD_W) / 2) as f32;
        self.new_ball();
        self.state = State::Launch;
    }
}

blip::sounds!(Sounds {
    paddle_hit: [blip::BlipSound; 3] = [blip::sound!("paddle_hit_0"), blip::sound!("paddle_hit_1"), blip::sound!("paddle_hit_2")],
    brick_hit: [blip::BlipSound; 3] = [blip::sound!("brick_hit_0"), blip::sound!("brick_hit_1"), blip::sound!("brick_hit_2")],
    brick_break: [blip::BlipSound; 3] = [blip::sound!("brick_break_0"), blip::sound!("brick_break_1"), blip::sound!("brick_break_2")],
    wall_hit, bomb, life_lost, win, pickup_good, pickup_bad, pickup_life,
});

/// A random take of an impact sound, louder for a harder hit.
fn play_variant(takes: &[blip::BlipSound; 3], speed: f32) {
    let vol = 0.65 + 0.35 * (speed / BALL_SPEED_MAX).min(1.0);
    play_sfx_volume(&takes[rand() as usize % takes.len()], vol);
}

fn update_title(g: &mut Game) {
    if coin_start() { g.start_game(); }
}

fn move_paddle(g: &mut Game, dt: f32) {
    let prev_x = g.pad_x;
    for (i, pressed) in [
        Stick::Left.pressed(),
        Stick::Right.pressed(),
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
    let left = Stick::Left.active();
    let right = Stick::Right.active();
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
    g.chain = 0;
    move_paddle(g, dt);
    g.fx.update(dt);
    g.ball_x = g.pad_x + (g.pad_w / 2.0 - BALL_W as f32 / 2.0);
    g.ball_y = (PAD_Y - BALL_H - 2) as f32;
    if key_pressed(BLIP_KEY_SPACE) || Stick::Up.pressed() {
        g.state = State::Play;
    }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    move_paddle(g, dt);

    // Recoil spring, ~5 Hz and well damped: one dip, no bounce.
    g.pad_kick_v += (-1000.0 * g.pad_kick - 28.0 * g.pad_kick_v) * dt;
    g.pad_kick += g.pad_kick_v * dt;

    if g.pad_effect_timer.tick(dt) { g.pad_w = PAD_W as f32; }
    g.slow_timer.tick(dt);
    move_drops(g, dt, sfx);
    g.fx.update(dt);
    shake_and_tumble(g, dt);
    g.since_break += dt;

    let active_speed = if g.slow_timer.active() {
        g.ball_speed * BALL_SLOW_FACTOR
    } else {
        g.ball_speed
    };
    if fly_balls(g, dt, active_speed, sfx) {
        if let Some(b) = g.extra.pop() { g.set_ball(b); } else { return ball_lost(g, sfx); }
    }

    if g.bricks_alive() == 0 {
        play_sfx(&sfx.win);
        g.dead_timer.start(1.5);
        g.state = State::Win;
    }
}

/// Every ball flies by the same code: each extra is swapped into the ball fields for its turn, and an extra that is
/// lost is gone. True when the main ball is lost.
fn fly_balls(g: &mut Game, dt: f32, speed: f32, sfx: &Sounds) -> bool {
    let lost = fly_ball(g, dt, speed, sfx);
    let mut k = 0;
    while k < g.extra.len() {
        g.swap_ball(k);
        let gone = fly_ball(g, dt, speed, sfx);
        g.swap_ball(k);
        if gone { g.extra.remove(k); } else { k += 1; }
    }
    lost
}

/// The last ball is lost: a life goes, and the ball waits on the paddle again, or the game is over.
fn ball_lost(g: &mut Game, sfx: &Sounds) {
    blip::bot::add(&format!("lost_at_speed{}", (g.ball_vx.hypot(g.ball_vy) / 50.0) as i32 * 50), 1.0);
    play_sfx(&sfx.life_lost);
    g.reset_drops();
    g.state = match g.sess.lose_life() {
        LifeResult::StillAlive => { g.dead_timer.start(1.2); State::Dead }
        LifeResult::GameOver => {
            // A new best (over the run's continues) gets the win jingle after the lost ball.
            if g.sess.beats_best() { play_sfx(&sfx.win); }
            g.cont.start(CONTINUE_SECS);
            State::Over
        }
    };
}

/// Hit bricks settle; broken halves fall under gravity until they leave the screen.
fn shake_and_tumble(g: &mut Game, dt: f32) {
    for b in g.bricks.iter_mut() { b.wobble = (b.wobble - dt * 3.5).max(0.0); }
    for s in g.shards.iter_mut() {
        // Lighter than real gravity, so the pieces are seen to tumble before they leave.
        s.vy += 520.0 * dt;
        s.x += s.vx * dt;
        s.y += s.vy * dt;
        s.turn += s.spin * dt;
    }
    g.shards.retain(|s| s.y < WIN_H as f32 + 30.0);
}

fn update_dead(g: &mut Game, dt: f32) {
    g.fx.update(dt);
    if g.dead_timer.tick(dt) {
        g.ball_speed = (g.ball_speed - SPEED_LIFE_RELIEF).max(BALL_SPEED_0);
        g.pad_x = ((WIN_W - PAD_W) / 2) as f32;
        g.new_ball();
        g.state = State::Launch;
    }
}

fn update_clear(g: &mut Game, dt: f32) {
    if g.dead_timer.tick(dt) { g.next_level(); }
}

/// Game over offers a continue: fire within CONTINUE_SECS plays on against the bricks still standing, with fresh
/// lives (and the score afresh); otherwise the run's best is reported and the title returns.
fn update_over(g: &mut Game, dt: f32) {
    match g.cont.update(dt, blip::input::btn1_pressed()) {
        Some(true) => g.continue_play(),
        Some(false) => {
            g.sess.report();
            g.state = State::Title;
        }
        None => {}
    }
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("BOUNCER", WIN_W, WIN_H)
}

#[blip::macroquad::main(conf)]
async fn main() {
    blip::bot::playthroughs(bot::PLAYTHROUGHS);
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let art = Art::load();

    let sfx = Sounds::load().await;
    // Two tunes, alternating by level.
    let mut music = Jukebox::new("bouncer", &["BOUNCE", "REBOUND", "RICOCHET"]);

    if blip.screenshot_mode {
        g.start_game();
        for (i, kind) in [DropKind::Wide, DropKind::Narrow, DropKind::Slow, DropKind::Life, DropKind::Multi].into_iter().enumerate() {
            pool_spawn(&mut g.drops, Drop { x: 70.0 + i as f32 * 75.0, y: 250.0 + i as f32 * 22.0, active: true, kind });
        }
    }

    loop {
        let dt = blip.delta_time;

        music.play(if g.state == State::Title { 0 } else { (g.sess.level as usize + 2) % 3 });

        g.touch_x = blip.touch(0).map(|p| p.x);
        if blip::bot::active() { bot::drive(&g); }
        match g.state {
            State::Title  => update_title(&mut g),
            State::Launch => update_launch(&mut g, dt),
            State::Play   => update_play(&mut g, dt, &sfx),
            State::Dead   => update_dead(&mut g, dt),
            State::Win    => update_clear(&mut g, dt),
            State::Over   => update_over(&mut g, dt),
        }
        if g.state == State::Over && g.cont.ticked() { play_sfx(&sfx.wall_hit); }

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title => draw_title(&blip, &art),
            State::Win   => draw_clear(&blip, g.sess.level, 1.5 - g.dead_timer.remaining()),
            State::Over  => {
                draw_backdrop(&blip, g.sess.level);
                blip.draw_game_over(g.sess.score, web::high_score(), BLIP_RED, BLIP_GREEN, BLIP_YELLOW, false);
                blip.draw_continue(&g.cont, BLIP_YELLOW);
            }
            State::Launch | State::Play | State::Dead => {
                draw_play(&blip, &g, &art);
                // The ball waits on the paddle after every lost life; say so.
                if g.state == State::Launch && blip::blink(2.0) {
                    let prompt = web::controls().pick("PRESS SPACE", "PRESS FIRE", "TAP TO LAUNCH");
                    blip.draw_centered(prompt, (PAD_Y - 90) as f32, 3.0, BLIP_WHITE);
                }
            }
        }

        blip.next_frame().await;
    }
}

#[cfg(test)]
mod tests;
