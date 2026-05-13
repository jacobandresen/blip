//! Bouncer (Breakout), Rust port of `games/bouncer/main.c` on macroquad.

use std::f32::consts::PI;

use blip::input::{
    any_key_pressed, key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::prelude::ImageFormat;
use blip::macroquad::rand::rand;
use blip::macroquad::texture::{FilterMode, Texture2D};
use blip::{
    clamp, play_music, play_sfx, rects_overlap, web, window_conf, Blip, BLIP_BLACK, BLIP_CYAN,
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

const BALL_W: i32 = 14;
const BALL_H: i32 = 14;
const BALL_SPEED_0: f32 = 240.0;
const BALL_SPEED_MAX: f32 = 380.0;

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

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Launch, Play, Dead, Win, Over }

#[derive(Copy, Clone, PartialEq, Eq)]
enum DropKind { Wide, Narrow, Slow, Life }

#[derive(Copy, Clone)]
struct Drop { x: f32, y: f32, active: bool, kind: DropKind }

const DEAD_DROP: Drop = Drop { x: 0.0, y: 0.0, active: false, kind: DropKind::Wide };

#[derive(Copy, Clone)]
struct Brick { kind: usize, alive: bool }

struct Game {
    bricks: [Brick; BRICK_TOTAL],
    drops: [Drop; MAX_DROPS],
    pad_x: f32,
    pad_w: f32,
    pad_effect_timer: f32,
    slow_timer: f32,
    ball_x: f32, ball_y: f32,
    ball_vx: f32, ball_vy: f32,
    ball_speed: f32,
    score: i32, hi_score: i32, lives: i32, level: i32,
    dead_timer: f32,
    state: State,
}

impl Game {
    fn new() -> Self {
        Self {
            bricks: [Brick { kind: 0, alive: false }; BRICK_TOTAL],
            drops: [DEAD_DROP; MAX_DROPS],
            pad_x: 0.0,
            pad_w: PAD_W as f32,
            pad_effect_timer: 0.0,
            slow_timer: 0.0,
            ball_x: 0.0, ball_y: 0.0, ball_vx: 0.0, ball_vy: 0.0,
            ball_speed: BALL_SPEED_0,
            score: 0, hi_score: web::load_hi_score(web::GAME_BOUNCER), lives: 0, level: 1,
            dead_timer: 0.0,
            state: State::Title,
        }
    }

    fn reset_drops(&mut self) {
        self.drops = [DEAD_DROP; MAX_DROPS];
        self.pad_w = PAD_W as f32;
        self.pad_effect_timer = 0.0;
        self.slow_timer = 0.0;
    }

    fn bricks_alive(&self) -> i32 {
        self.bricks.iter().filter(|b| b.alive).count() as i32
    }

    fn build_bricks(&mut self) {
        for r in 0..BRICK_ROWS {
            for c in 0..BRICK_COLS {
                let i = (r * BRICK_COLS + c) as usize;
                let alive = match self.level {
                    1 => true,
                    2 => (r + c) % 2 == 0,
                    _ => {
                        let center_col = (BRICK_COLS - 1) / 2;
                        let distance = (c as isize - center_col as isize).abs();
                        distance <= 3
                    }
                };
                self.bricks[i] = Brick { kind: r as usize, alive };
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
    }

    fn start_game(&mut self) {
        self.hi_score = self.hi_score.max(web::load_hi_score(web::GAME_BOUNCER));
        self.score = 0;
        self.lives = LIVES_START;
        self.level = 1;
        self.ball_speed = BALL_SPEED_0;
        self.reset_drops();
        self.pad_x = ((WIN_W - PAD_W) / 2) as f32;
        self.build_bricks();
        self.launch_ball();
        self.state = State::Launch;
    }

    fn next_level(&mut self) {
        self.level += 1;
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
    g.hi_score = g.hi_score.max(web::load_hi_score(web::GAME_BOUNCER));
    if any_key_pressed() { g.start_game(); }
}

fn paddle_input(g: &mut Game, dt: f32) {
    let ps = PAD_SPEED * dt;
    if key_held(BLIP_KEY_LEFT)  || key_held(BLIP_KEY_A) { g.pad_x -= ps; }
    if key_held(BLIP_KEY_RIGHT) || key_held(BLIP_KEY_D) { g.pad_x += ps; }
    g.pad_x = clamp(g.pad_x, 0.0, WIN_W as f32 - g.pad_w);
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
    g.hi_score = g.hi_score.max(web::load_hi_score(web::GAME_BOUNCER));
    paddle_input(g, dt);

    g.ball_x += g.ball_vx * dt;
    g.ball_y += g.ball_vy * dt;

    if g.ball_x < 0.0 { g.ball_x = 0.0; g.ball_vx = g.ball_vx.abs(); }
    if g.ball_x + BALL_W as f32 > WIN_W as f32 {
        g.ball_x = (WIN_W - BALL_W) as f32;
        g.ball_vx = -g.ball_vx.abs();
    }
    if g.ball_y < HUD_H as f32 { g.ball_y = HUD_H as f32; g.ball_vy = g.ball_vy.abs(); }

    // Tick drop effects
    if g.pad_effect_timer > 0.0 {
        g.pad_effect_timer -= dt;
        if g.pad_effect_timer <= 0.0 {
            g.pad_effect_timer = 0.0;
            g.pad_w = PAD_W as f32;
        }
    }
    if g.slow_timer > 0.0 {
        g.slow_timer -= dt;
        if g.slow_timer < 0.0 { g.slow_timer = 0.0; }
    }

    update_drops(g, dt);

    if g.ball_y > WIN_H as f32 {
        play_sfx(&sfx.life_lost);
        g.reset_drops();
        g.lives -= 1;
        if g.lives > 0 {
            g.dead_timer = 1.2;
            g.state = State::Dead;
        } else {
            g.state = State::Over;
        }
        return;
    }

    let active_speed = if g.slow_timer > 0.0 { g.ball_speed * BALL_SLOW_FACTOR } else { g.ball_speed };

    if g.ball_vy > 0.0
        && rects_overlap(
            g.ball_x, g.ball_y, BALL_W as f32, BALL_H as f32,
            g.pad_x, PAD_Y as f32, g.pad_w, PAD_H as f32,
        )
    {
        play_sfx(&sfx.paddle_hit);
        let rel = (g.ball_x + BALL_W as f32 / 2.0 - g.pad_x) / g.pad_w;
        let angle = (rel - 0.5) * 2.0 * 1.2;
        g.ball_vx = active_speed * angle.sin();
        g.ball_vy = -active_speed * angle.cos();
        if g.ball_vy.abs() < active_speed * 0.3 {
            g.ball_vy = -active_speed * 0.3;
        }
        g.ball_y = (PAD_Y - BALL_H - 1) as f32;
    }

    for i in 0..BRICK_TOTAL {
        if !g.bricks[i].alive { continue; }
        let r = i as i32 / BRICK_COLS;
        let c = i as i32 % BRICK_COLS;
        let bx = (BRICK_OX + c * (BRICK_W + BRICK_GAP)) as f32;
        let by = (BRICK_OY + r * (BRICK_H + BRICK_GAP)) as f32;
        if !rects_overlap(g.ball_x, g.ball_y, BALL_W as f32, BALL_H as f32,
                          bx, by, BRICK_W as f32, BRICK_H as f32) { continue; }

        let kind = g.bricks[i].kind;
        g.bricks[i].alive = false;
        g.score += (BRICK_ROWS - r) * 10 * g.level;
        if g.score > g.hi_score { g.hi_score = g.score; web::save_hi_score(web::GAME_BOUNCER, g.hi_score); }
        g.ball_speed = clamp(g.ball_speed + SPEED_INC, 0.0, BALL_SPEED_MAX);

        // 30% chance to spawn a loot drop
        if rand() % 10 < 3 {
            let drop_x = bx + BRICK_W as f32 / 2.0 - DROP_SIZE / 2.0;
            let drop_y = by;
            let kind_idx = rand() % 10;
            let drop_kind = match kind_idx {
                0..=2 => DropKind::Wide,
                3..=5 => DropKind::Slow,
                6..=7 => DropKind::Narrow,
                _     => DropKind::Life,
            };
            for j in 0..MAX_DROPS {
                if !g.drops[j].active {
                    g.drops[j] = Drop { x: drop_x, y: drop_y, active: true, kind: drop_kind };
                    break;
                }
            }
        }

        let over_x = if g.ball_vx > 0.0 { bx - (g.ball_x + BALL_W as f32) }
                     else { (bx + BRICK_W as f32) - g.ball_x };
        let over_y = if g.ball_vy > 0.0 { by - (g.ball_y + BALL_H as f32) }
                     else { (by + BRICK_H as f32) - g.ball_y };
        if over_x.abs() < over_y.abs() { g.ball_vx = -g.ball_vx; }
        else                            { g.ball_vy = -g.ball_vy; }

        let spd = (g.ball_vx * g.ball_vx + g.ball_vy * g.ball_vy).sqrt();
        if spd > 0.0 {
            g.ball_vx = g.ball_vx / spd * active_speed;
            g.ball_vy = g.ball_vy / spd * active_speed;
        }

        if kind <= 1 { play_sfx(&sfx.brick_break); }
        else         { play_sfx(&sfx.brick_hit); }
        break;
    }

    if g.bricks_alive() == 0 {
        play_sfx(&sfx.win);
        g.dead_timer = 1.5;
        g.state = State::Win;
    }
}

fn update_drops(g: &mut Game, dt: f32) {
    for i in 0..MAX_DROPS {
        if !g.drops[i].active { continue; }
        g.drops[i].y += DROP_SPEED * dt;
        if g.drops[i].y > WIN_H as f32 { g.drops[i].active = false; continue; }
        if rects_overlap(g.drops[i].x, g.drops[i].y, DROP_SIZE, DROP_SIZE,
                         g.pad_x, PAD_Y as f32, g.pad_w, PAD_H as f32) {
            g.drops[i].active = false;
            let kind = g.drops[i].kind;
            match kind {
                DropKind::Wide => {
                    g.pad_w = PAD_W_WIDE;
                    g.pad_effect_timer = EFFECT_DURATION;
                }
                DropKind::Narrow => {
                    g.pad_w = PAD_W_NARROW;
                    g.pad_effect_timer = EFFECT_DURATION;
                }
                DropKind::Slow => {
                    g.slow_timer = EFFECT_DURATION;
                }
                DropKind::Life => {
                    g.lives += 1;
                }
            }
        }
    }
}

fn update_dead(g: &mut Game, dt: f32) {
    g.dead_timer -= dt;
    if g.dead_timer <= 0.0 {
        g.pad_x = ((WIN_W - PAD_W) / 2) as f32;
        g.launch_ball();
        g.state = State::Launch;
    }
}

fn update_win(g: &mut Game, dt: f32) {
    g.dead_timer -= dt;
    if g.dead_timer <= 0.0 { g.next_level(); }
}

fn update_over(g: &mut Game) {
    g.hi_score = g.hi_score.max(web::load_hi_score(web::GAME_BOUNCER));
    web::game_over(web::GAME_BOUNCER, g.score);
    if !any_key_pressed() { return; }
    web::spend_coin();
    g.start_game();
}

fn draw_play(blip: &Blip, g: &Game, paddle: &Texture2D, ball: &Texture2D, brick: &[Texture2D; 6]) {
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
    for i in 0..MAX_DROPS {
        if !g.drops[i].active { continue; }
        let x = g.drops[i].x;
        let y = g.drops[i].y;
        match g.drops[i].kind {
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
    if g.slow_timer > 0.0 {
        blip.draw_centered("SLOW", indicator_y, 2.0, BLIP_CYAN);
    } else if g.pad_effect_timer > 0.0 {
        if g.pad_w > PAD_W as f32 {
            blip.draw_centered("WIDE",   indicator_y, 2.0, BLIP_GREEN);
        } else {
            blip.draw_centered("NARROW", indicator_y, 2.0, BLIP_RED);
        }
    }

    blip.draw_texture(paddle, g.pad_x, PAD_Y as f32, g.pad_w, PAD_H as f32);
    blip.draw_texture(ball, g.ball_x, g.ball_y, BALL_W as f32, BALL_H as f32);
    blip.draw_hud(g.score, g.hi_score, g.lives);
}

fn draw_title(blip: &Blip) {
    blip.clear(BLIP_BLACK);
    blip.draw_centered("BOUNCER",                 (WIN_H / 4) as f32,         6.0, BLIP_CYAN);
    blip.draw_centered("PRESS ANY KEY",           (WIN_H / 2) as f32,         3.0, BLIP_WHITE);
    blip.draw_centered("LEFT RIGHT ARROW OR AD",  (WIN_H * 2 / 3) as f32,     2.0, BLIP_GRAY);
    blip.draw_centered("SPACE TO LAUNCH",         (WIN_H * 2 / 3 + 20) as f32, 2.0, BLIP_GRAY);
}

fn draw_win(blip: &Blip, level: i32) {
    let buf = format!("LEVEL {}", level + 1);
    blip.clear(BLIP_BLACK);
    blip.draw_centered("CLEARED", (WIN_H / 3) as f32, 5.0, BLIP_GREEN);
    blip.draw_centered(&buf,      (WIN_H / 2) as f32, 3.0, BLIP_YELLOW);
}

fn draw_over(blip: &Blip, score: i32) {
    let buf = format!("SCORE {}", score);
    blip.clear(BLIP_BLACK);
    blip.draw_centered("GAME OVER",     (WIN_H / 4) as f32,     5.0, BLIP_RED);
    blip.draw_centered(&buf,            (WIN_H / 2) as f32,     3.0, BLIP_WHITE);
    blip.draw_centered("PRESS ANY KEY", (WIN_H * 2 / 3) as f32, 3.0, BLIP_YELLOW);
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("BOUNCER", WIN_W, WIN_H)
}

const PADDLE_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/paddle.png"));
const BALL_PNG:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/ball.png"));
const BRICK_RED_PNG:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_red.png"));
const BRICK_ORANGE_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_orange.png"));
const BRICK_YELLOW_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_yellow.png"));
const BRICK_GREEN_PNG:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_green.png"));
const BRICK_BLUE_PNG:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_blue.png"));
const BRICK_PURPLE_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/brick_purple.png"));
const PADDLE_HIT_WAV:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/paddle_hit.wav"));
const BRICK_HIT_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/brick_hit.wav"));
const BRICK_BREAK_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/brick_break.wav"));
const LIFE_LOST_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/life_lost.wav"));
const WIN_WAV:         &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/win.wav"));
const MUSIC_WAV:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music.wav"));

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
    let brick = [
        load_png(BRICK_RED_PNG),
        load_png(BRICK_ORANGE_PNG),
        load_png(BRICK_YELLOW_PNG),
        load_png(BRICK_GREEN_PNG),
        load_png(BRICK_BLUE_PNG),
        load_png(BRICK_PURPLE_PNG),
    ];

    let sfx = Sounds {
        paddle_hit:  blip::audio::load_sound(PADDLE_HIT_WAV).await,
        brick_hit:   blip::audio::load_sound(BRICK_HIT_WAV).await,
        brick_break: blip::audio::load_sound(BRICK_BREAK_WAV).await,
        life_lost:   blip::audio::load_sound(LIFE_LOST_WAV).await,
        win:         blip::audio::load_sound(WIN_WAV).await,
    };
    let music = blip::audio::load_sound(MUSIC_WAV).await;
    play_music(&music);

    loop {
        let dt = blip.delta_time;
        match g.state {
            State::Title  => update_title(&mut g),
            State::Launch => update_launch(&mut g, dt),
            State::Play   => update_play(&mut g, dt, &sfx),
            State::Dead   => update_dead(&mut g, dt),
            State::Win    => update_win(&mut g, dt),
            State::Over   => update_over(&mut g),
        }

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title => draw_title(&blip),
            State::Win   => draw_win(&blip, g.level),
            State::Over  => draw_over(&blip, g.score),
            State::Launch | State::Play | State::Dead => {
                draw_play(&blip, &g, &paddle, &ball, &brick);
            }
        }

        blip.next_frame(60).await;
    }
}
