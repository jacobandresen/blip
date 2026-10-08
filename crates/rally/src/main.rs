//! Pong against the CPU.


use blip::input::{
    any_key_pressed, btn1_pressed, key_active, key_pressed, BLIP_KEY_DOWN, BLIP_KEY_S, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::input::KeyCode;
use blip::macroquad::math::Vec2;
use blip::macroquad::rand::rand;
use blip::{
    GAME_OVER_MIN_WAIT, segment_rect_overlap,
    play_sfx, Jukebox, web, window_conf, Blip, BlipColor, Timer, BLIP_BLACK,
    BLIP_GRAY, BLIP_WHITE, BLIP_YELLOW,
};

#[cfg(not(target_arch = "wasm32"))]
mod bot;

// ---- layout -----------------------------------------------------------
const WIN_W: i32 = 480;
const WIN_H: i32 = 540;
const HUD_H: i32 = 28;
const PLAY_T: f32 = HUD_H as f32;
const PLAY_B: f32 = WIN_H as f32;
const PLAY_H: f32 = PLAY_B - PLAY_T;

// ---- objects ----------------------------------------------------------
const PAD_W: f32 = 12.0;
const PAD_H: f32 = 72.0;
const BALL_SZ: f32 = 12.0;
const PAD_OFF: f32 = 28.0;

// ---- tuning -----------------------------------------------------------
const SCORE_WIN: i32 = 7;
const POINT_PAUSE: f32 = 1.2;
const PAD_SPEED: f32 = 300.0;
const BALL_SPD0: f32 = 275.0;
const BALL_INC: f32 = 15.0;
const BALL_MAX: f32 = 450.0;
const AI_SPD: f32 = 145.0;
// A finger drags its bat relatively, geared up so a thumb's sweep of about
// two thirds of the picture covers the whole field.
const TOUCH_GAIN: f32 = 1.6;

// ---- derived ----------------------------------------------------------
const LPAD_X: f32 = PAD_OFF;
const RPAD_X: f32 = WIN_W as f32 - PAD_OFF - PAD_W;
const PAD_YMIN: f32 = PLAY_T + 2.0;
const PAD_YMAX: f32 = PLAY_B - PAD_H - 2.0;

const C_PAD: BlipColor = BlipColor { r: 210.0/255.0, g: 210.0/255.0, b: 210.0/255.0, a: 1.0 };
const C_NET: BlipColor = BlipColor { r:  42.0/255.0, g:  42.0/255.0, b:  42.0/255.0, a: 1.0 };
const C_HUD_LINE: BlipColor = BlipColor { r: 28.0/255.0, g: 28.0/255.0, b: 28.0/255.0, a: 1.0 };

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Serve, Play, Point, Over }

#[derive(Copy, Clone, PartialEq, Eq)]
enum Mode { OnePlayer, TwoPlayer }

struct Game {
    lpad_y: f32, rpad_y: f32,
    ball_x: f32, ball_y: f32, ball_vx: f32, ball_vy: f32, ball_spd: f32,
    score_l: i32, score_r: i32,
    point_t: Timer,
    state: State,
    mode: Mode,
    ai_err: f32, // where on its face the CPU meets the ball, re-rolled each volley
    serve_dir: f32, // +1 serves to the right paddle; the loser of a point receives
    touch: [Option<Vec2>; 2],      // each player's finger this frame (slot 1 = P2)
    touch_prev: [Option<Vec2>; 2], // ... and last frame
    pressed: [bool; 2],            // ... pressed (a PC's pointer can hover)
    pressed_prev: [bool; 2],
}

impl Game {
    fn new() -> Self {
        Self {
            lpad_y: 0.0, rpad_y: 0.0,
            ball_x: 0.0, ball_y: 0.0, ball_vx: 0.0, ball_vy: 0.0, ball_spd: BALL_SPD0,
            score_l: 0, score_r: 0,
            point_t: Timer::default(),
            state: State::Title,
            mode: Mode::OnePlayer,
            ai_err: 0.0,
            serve_dir: 1.0,
            touch: [None; 2],
            touch_prev: [None; 2],
            pressed: [false; 2],
            pressed_prev: [false; 2],
        }
    }

    fn clamp_pads(&mut self) {
        self.lpad_y = self.lpad_y.clamp(PAD_YMIN, PAD_YMAX);
        self.rpad_y = self.rpad_y.clamp(PAD_YMIN, PAD_YMAX);
    }

    fn reset_for_serve(&mut self) {
        self.lpad_y = PLAY_T + PLAY_H * 0.5 - PAD_H * 0.5;
        self.rpad_y = self.lpad_y;
        self.ball_x = WIN_W as f32 * 0.5 - BALL_SZ * 0.5;
        self.ball_y = PLAY_T + PLAY_H * 0.5 - BALL_SZ * 0.5;
        self.ball_vx = 0.0;
        self.ball_vy = 0.0;
        self.ball_spd = BALL_SPD0;
    }

    fn launch(&mut self) {
        let r = (rand() % 10000) as f32 / 10000.0 - 0.5;
        let a = r * 0.77;
        self.ball_vx = self.serve_dir * self.ball_spd * a.cos();
        self.ball_vy = self.ball_spd * a.sin();
    }

    fn start_game(&mut self) {
        self.score_l = 0;
        self.score_r = 0;
        self.serve_dir = if rand().is_multiple_of(2) { 1.0 } else { -1.0 };
        self.reset_for_serve();
        self.state = State::Serve;
    }
}

struct Beeps {
    wall: blip::BlipSound,    // 587 Hz / 35 ms
    hit_l: blip::BlipSound,   // 240 Hz / 35 ms
    hit_r: blip::BlipSound,   // 300 Hz / 35 ms
    score_l: blip::BlipSound, // 660 Hz / 120 ms (player scored)
    score_r: blip::BlipSound, // 110 Hz / 200 ms (CPU scored)
}

/// A spin of a paddle dial this frame — the touch dials inject these same
/// up/down keys (P1: arrows, P2: I/K), and on the keyboard the paddle keys
/// are the dial. This is what "ROTATE DIAL TO START" listens for.
fn p1_dial_spun() -> bool {
    key_pressed(BLIP_KEY_UP) || key_pressed(BLIP_KEY_DOWN)
        || key_pressed(BLIP_KEY_W) || key_pressed(BLIP_KEY_S)
}
fn p2_dial_spun() -> bool {
    key_pressed(KeyCode::I) || key_pressed(KeyCode::K)
}

/// A finger came down (or the mouse was clicked) on player `slot`'s side
/// this frame.
fn touch_began(g: &Game, slot: usize) -> bool {
    g.pressed[slot] && !g.pressed_prev[slot]
}

/// How far player `slot`'s finger has dragged their bat this frame.
fn touch_drag(g: &Game, slot: usize) -> f32 {
    match (g.touch[slot], g.touch_prev[slot]) {
        (Some(now), Some(before)) => (now.y - before.y) * TOUCH_GAIN,
        _ => 0.0,
    }
}

/// Keys and fingers on the bats, for the serve and the rally.
fn move_pads(g: &mut Game, dt: f32) {
    if key_active(BLIP_KEY_UP)   || key_active(BLIP_KEY_W) { g.lpad_y -= PAD_SPEED * dt; }
    if key_active(BLIP_KEY_DOWN) || key_active(BLIP_KEY_S) { g.lpad_y += PAD_SPEED * dt; }
    g.lpad_y += touch_drag(g, 0);
    if g.mode == Mode::TwoPlayer {
        if key_active(KeyCode::I) { g.rpad_y -= PAD_SPEED * dt; }
        if key_active(KeyCode::K) { g.rpad_y += PAD_SPEED * dt; }
        g.rpad_y += touch_drag(g, 1);
    }
}

fn update_title(g: &mut Game) {
    // Spin the P2 dial (or press "2") for a two-player game; spin your own
    // dial (or press Space) for one player against the CPU.
    if key_pressed(KeyCode::Key2) || p2_dial_spun() || touch_began(g, 1) {
        g.mode = Mode::TwoPlayer;
        web::set_mode(true);
        // Two players, two coins.
        web::spend_coin();
        web::spend_coin();
        g.start_game();
    } else if p1_dial_spun() || touch_began(g, 0) || btn1_pressed() {
        g.mode = Mode::OnePlayer;
        web::set_mode(false);
        web::spend_coin();
        g.start_game();
    }
}


fn update_serve(g: &mut Game, dt: f32) {
    move_pads(g, dt);
    g.clamp_pads();
    if any_key_pressed() || touch_began(g, 0) || touch_began(g, 1) {
        g.launch();
        g.state = State::Play;
    }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Beeps) {
    let dt = dt.max(1e-4);
    let (lpy0, rpy0) = (g.lpad_y, g.rpad_y);

    move_pads(g, dt);
    if g.mode == Mode::OnePlayer {
        // Tracks the ball only while it is coming; otherwise drifts to centre.
        let target = if g.ball_vx > 0.0 {
            g.ball_y + BALL_SZ * 0.5 - PAD_H * 0.5 + g.ai_err
        } else {
            PLAY_T + PLAY_H * 0.5 - PAD_H * 0.5
        };
        let diff = target - g.rpad_y;
        let mv = AI_SPD * dt * if g.ball_vx > 0.0 { 1.0 } else { 0.5 };
        g.rpad_y += if diff > mv { mv } else if diff < -mv { -mv } else { diff };
    }

    g.clamp_pads();

    // Paddle velocity this frame; a moving paddle throws the ball.
    let lpad_vy = (g.lpad_y - lpy0) / dt;
    let rpad_vy = (g.rpad_y - rpy0) / dt;

    // Integrate in substeps so a fast ball can't skip past a paddle or clip
    // through a wall inside one frame (worst case: a frame-rate dip).
    let speed = g.ball_vx.hypot(g.ball_vy).max(1.0);
    let substeps = ((speed * dt / (BALL_SZ * 0.5)).ceil() as i32).clamp(1, 8);
    let sdt = dt / substeps as f32;

    for _ in 0..substeps {
        let (prev_x, prev_y) = (g.ball_x + BALL_SZ * 0.5, g.ball_y + BALL_SZ * 0.5);
        g.ball_x += g.ball_vx * sdt;
        g.ball_y += g.ball_vy * sdt;
        let (next_x, next_y) = (g.ball_x + BALL_SZ * 0.5, g.ball_y + BALL_SZ * 0.5);

        // Reflect the overshoot rather than clamp, so no speed is shaved off.
        if g.ball_y < PLAY_T {
            g.ball_y = 2.0 * PLAY_T - g.ball_y;
            g.ball_vy = g.ball_vy.abs();
            play_sfx(&sfx.wall);
        }
        if g.ball_y + BALL_SZ > PLAY_B {
            g.ball_y = 2.0 * (PLAY_B - BALL_SZ) - g.ball_y;
            g.ball_vy = -g.ball_vy.abs();
            play_sfx(&sfx.wall);
        }

        if g.ball_vx < 0.0
            && segment_rect_overlap(prev_x, prev_y, next_x, next_y,
                LPAD_X - BALL_SZ * 0.5, g.lpad_y - BALL_SZ * 0.5,
                PAD_W + BALL_SZ, PAD_H + BALL_SZ)
        {
            g.ball_x = LPAD_X + PAD_W;
            let py = g.lpad_y;
            bounce_paddle(g, py, lpad_vy, 1.0);
            play_sfx(&sfx.hit_l);
            web::haptic();
        }

        if g.ball_vx > 0.0
            && segment_rect_overlap(prev_x, prev_y, next_x, next_y,
                RPAD_X - BALL_SZ * 0.5, g.rpad_y - BALL_SZ * 0.5,
                PAD_W + BALL_SZ, PAD_H + BALL_SZ)
        {
            g.ball_x = RPAD_X - BALL_SZ;
            let py = g.rpad_y;
            bounce_paddle(g, py, rpad_vy, -1.0);
            play_sfx(&sfx.hit_r);
            if g.mode == Mode::TwoPlayer { web::haptic(); }
        }

        if g.ball_x + BALL_SZ < 0.0 {
            award_point(g, false, &sfx.score_r);
            return;
        }
        if g.ball_x > WIN_W as f32 {
            award_point(g, true, &sfx.score_l);
            return;
        }
    }
}

fn award_point(g: &mut Game, left_scored: bool, sound: &blip::BlipSound) {
    if left_scored {
        g.score_l += 1;
        g.serve_dir = 1.0;
    } else {
        g.score_r += 1;
        g.serve_dir = -1.0;
    }
    play_sfx(sound);

    let score = if left_scored { g.score_l } else { g.score_r };
    if score >= SCORE_WIN {
        g.point_t.start(GAME_OVER_MIN_WAIT);
        g.state = State::Over;
    } else {
        g.reset_for_serve();
        g.point_t.start(POINT_PAUSE);
        g.state = State::Point;
    }
}

/// Aim by impact position, increase speed, and add a quarter of paddle velocity.
/// `dir` is +1 for the left paddle and -1 for the right.
fn bounce_paddle(g: &mut Game, pad_y: f32, pad_vy: f32, dir: f32) {
    let rel = ((g.ball_y + BALL_SZ * 0.5 - pad_y) / PAD_H - 0.5).clamp(-0.5, 0.5);
    g.ball_spd = (g.ball_spd + BALL_INC).min(BALL_MAX);
    let a = rel * 1.1;
    g.ball_vx = dir * g.ball_spd * a.cos();
    g.ball_vy = g.ball_spd * a.sin() + pad_vy * 0.25;
    // Renormalise so |v| stays exactly on the speed ramp after the english.
    let m = g.ball_vx.hypot(g.ball_vy).max(1.0);
    g.ball_vx = g.ball_vx / m * g.ball_spd;
    g.ball_vy = g.ball_vy / m * g.ball_spd;
    if dir > 0.0 {
        g.ai_err = (rand() % 1000) as f32 / 1000.0 * PAD_H * 0.7 - PAD_H * 0.35;
    }
}

fn update_point(g: &mut Game, dt: f32) {
    if g.point_t.tick(dt) { g.state = State::Serve; }
}

fn update_over(g: &mut Game, dt: f32) {
    g.point_t.tick(dt);
    let again = p1_dial_spun() || p2_dial_spun() || touch_began(g, 0) || touch_began(g, 1);
    if g.point_t.active() || !again { return; }
    web::spend_coin();
    g.start_game();
}

fn draw_net(blip: &Blip) {
    let mut y = PLAY_T as i32;
    while y < PLAY_B as i32 {
        blip.fill_rect((WIN_W / 2 - 1) as f32, y as f32, 3.0, 13.0, C_NET);
        y += 22;
    }
}

fn draw_hud(blip: &Blip, score_l: i32, score_r: i32) {
    blip.fill_rect(0.0, (HUD_H - 1) as f32, WIN_W as f32, 1.0, C_HUD_LINE);
    let buf_l = format!("{}:{}", score_l, SCORE_WIN);
    blip.draw_text(&buf_l, (WIN_W / 2 - 72) as f32, 5.0, 2.0, BLIP_YELLOW);
    let buf_r = format!("{}:{}", score_r, SCORE_WIN);
    blip.draw_text(&buf_r, (WIN_W / 2 + 20) as f32, 5.0, 2.0, BLIP_YELLOW);
}

fn draw_field(blip: &Blip, g: &Game) {
    draw_net(blip);
    draw_hud(blip, g.score_l, g.score_r);
    blip.fill_rect(LPAD_X, g.lpad_y, PAD_W, PAD_H, C_PAD);
    blip.fill_rect(RPAD_X, g.rpad_y, PAD_W, PAD_H, C_PAD);
}

fn draw_title(blip: &Blip) {
    blip.clear(BLIP_BLACK);
    draw_net(blip);
    draw_hud(blip, 0, 0);
    let py = PLAY_T + PLAY_H * 0.5 - PAD_H * 0.5;
    blip.fill_rect(LPAD_X, py, PAD_W, PAD_H, C_PAD);
    blip.fill_rect(RPAD_X, py, PAD_W, PAD_H, C_PAD);
    let cy = PLAY_T + PLAY_H * 0.5;
    let t = blip::macroquad::time::get_time() as f32;
    let travel = (t * 150.0).rem_euclid(WIN_W as f32 * 2.0);
    let ball_x = if travel <= WIN_W as f32 { travel } else { WIN_W as f32 * 2.0 - travel };
    let ball_y = cy + 126.0 + (t * 2.1).sin() * 10.0;
    let trail_dir = if travel < WIN_W as f32 { -1.0 } else { 1.0 };
    for (i, alpha) in [0.12, 0.24, 0.42].into_iter().enumerate() {
        let offset = (i as f32 + 1.0) * 9.0 * trail_dir;
        blip.fill_rect(ball_x + offset, ball_y, BALL_SZ, BALL_SZ,
            BlipColor { r: 0.72, g: 0.92, b: 1.0, a: alpha });
    }
    blip.fill_rect(ball_x, ball_y, BALL_SZ, BALL_SZ, BLIP_WHITE);
    blip.draw_centered("RALLY", cy - 34.0, 5.0, BLIP_YELLOW);
    // Moving a paddle (dial or keys) picks the mode; there is no menu.
    blip.draw_centered("MOVE LEFT PADDLE: 1 PLAYER",   cy + 14.0, 2.0, BLIP_WHITE);
    blip.draw_centered("MOVE RIGHT PADDLE: 2 PLAYERS", cy + 36.0, 2.0, BLIP_WHITE);
}

fn draw_serve(blip: &Blip, g: &Game) {
    blip.clear(BLIP_BLACK);
    draw_field(blip, g);
    blip.fill_rect(g.ball_x, g.ball_y, BALL_SZ, BALL_SZ, BLIP_WHITE);
    let prompt = web::controls().pick("PRESS FIRE", "PRESS FIRE", "TAP TO SERVE");
    blip.draw_centered(prompt, PLAY_T + PLAY_H * 0.5 + 54.0, 2.0, BLIP_GRAY);
}

fn draw_play(blip: &Blip, g: &Game) {
    blip.clear(BLIP_BLACK);
    draw_field(blip, g);
    blip.fill_rect(g.ball_x, g.ball_y, BALL_SZ, BALL_SZ, BLIP_WHITE);
}

fn draw_point(blip: &Blip, g: &Game) {
    blip.clear(BLIP_BLACK);
    draw_net(blip);
    draw_hud(blip, g.score_l, g.score_r);
    if (g.point_t.remaining() * 6.0) as i32 % 2 == 0 {
        blip.draw_centered("POINT!", PLAY_T + PLAY_H * 0.5, 3.0, BLIP_YELLOW);
    }
}

fn draw_over(blip: &Blip, g: &Game) {
    blip.clear(BLIP_BLACK);
    draw_net(blip);
    draw_hud(blip, g.score_l, g.score_r);
    let cy = PLAY_T + PLAY_H * 0.5;
    let msg = if g.mode == Mode::TwoPlayer {
        if g.score_l >= SCORE_WIN { "P1 WINS!" } else { "P2 WINS!" }
    } else {
        if g.score_l >= SCORE_WIN { "YOU WIN!" } else { "GAME OVER" }
    };
    blip.draw_centered(msg, cy - 20.0, 3.0, BLIP_YELLOW);
    if !g.point_t.active() {
        blip.draw_centered("MOVE A PADDLE TO PLAY AGAIN", cy + 24.0, 2.0, BLIP_GRAY);
    }
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("RALLY", WIN_W, WIN_H)
}

const WALL_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/wall.wav"));
const HIT_L_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/hit_l.wav"));
const HIT_R_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/hit_r.wav"));
const SCORE_L_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/score_l.wav"));
const SCORE_R_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/score_r.wav"));

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let sfx = Beeps {
        wall:    blip::audio::load_sound(WALL_WAV).await,
        hit_l:   blip::audio::load_sound(HIT_L_WAV).await,
        hit_r:   blip::audio::load_sound(HIT_R_WAV).await,
        score_l: blip::audio::load_sound(SCORE_L_WAV).await,
        score_r: blip::audio::load_sound(SCORE_R_WAV).await,
    };
    // Five loops in rotation, synthesised here (see blip_assets::rally).
    use blip_assets::rally::{music, music2, music3, music4, music5};
    let mut jukebox = Jukebox::new(&[music, music2, music3, music4, music5]);
    jukebox.start(0).await;
    #[cfg(not(target_arch = "wasm32"))]
    let mut autopilot = bot::Bot { err: 0.0, last_vx: 0.0, hits: 0 };

    loop {
        let dt = blip.delta_time;

        jukebox.rotate(dt);
        if g.state != State::Play { jukebox.warm_up().await; }

        g.touch = [blip.touch(0), blip.touch(1)];
        g.pressed = [blip.touch_pressed(0), blip.touch_pressed(1)];
        #[cfg(not(target_arch = "wasm32"))]
        if blip::bot::active() { bot::drive(&g, &mut autopilot, blip::bot::clock()); }
        match g.state {
            State::Title => update_title(&mut g),
            State::Serve => update_serve(&mut g, dt),
            State::Play  => update_play(&mut g, dt, &sfx),
            State::Point => update_point(&mut g, dt),
            State::Over  => update_over(&mut g, dt),
        }
        g.touch_prev = g.touch;
        g.pressed_prev = g.pressed;

        // Feed the two paddle positions to the shell so it can spin the
        // on-screen dials to match — the human paddle(s) and, in 1-player
        // mode, the CPU's.
        let span = PAD_YMAX - PAD_YMIN;
        web::paddles(
            ((g.lpad_y - PAD_YMIN) / span).clamp(0.0, 1.0),
            ((g.rpad_y - PAD_YMIN) / span).clamp(0.0, 1.0),
        );

        match g.state {
            State::Title => draw_title(&blip),
            State::Serve => draw_serve(&blip, &g),
            State::Play  => draw_play(&blip, &g),
            State::Point => draw_point(&blip, &g),
            State::Over  => draw_over(&blip, &g),
        }
        blip.next_frame(60).await;
    }
}
