//! Rally (Pong vs CPU), Rust port of `games/rally/main.c` on macroquad.

use blip::input::{
    any_key_pressed, key_held, BLIP_KEY_DOWN, BLIP_KEY_S, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::rand::rand;
use blip::{
    play_music, play_sfx, rects_overlap, web, window_conf, Blip, BlipColor, BLIP_BLACK,
    BLIP_GRAY, BLIP_WHITE, BLIP_YELLOW,
};

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
const PAD_SPEED: f32 = 300.0;
const BALL_SPD0: f32 = 275.0;
const BALL_INC: f32 = 15.0;
const BALL_MAX: f32 = 450.0;
const AI_SPD: f32 = 145.0;

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

struct Game {
    lpad_y: f32, rpad_y: f32,
    ball_x: f32, ball_y: f32, ball_vx: f32, ball_vy: f32, ball_spd: f32,
    score_l: i32, score_r: i32,
    point_t: f32,
    state: State,
}

impl Game {
    fn new() -> Self {
        Self {
            lpad_y: 0.0, rpad_y: 0.0,
            ball_x: 0.0, ball_y: 0.0, ball_vx: 0.0, ball_vy: 0.0, ball_spd: BALL_SPD0,
            score_l: 0, score_r: 0,
            point_t: 0.0,
            state: State::Title,
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
        self.ball_vx = self.ball_spd * a.cos();
        self.ball_vy = self.ball_spd * a.sin();
    }

    fn start_game(&mut self) {
        self.score_l = 0;
        self.score_r = 0;
        self.reset_for_serve();
        self.state = State::Serve;
    }
}

struct Beeps {
    wall: blip::BlipSound,    // 490 Hz / 22 ms
    hit_l: blip::BlipSound,   // 240 Hz / 35 ms
    hit_r: blip::BlipSound,   // 300 Hz / 35 ms
    score_l: blip::BlipSound, // 660 Hz / 120 ms (player scored)
    score_r: blip::BlipSound, // 110 Hz / 200 ms (CPU scored)
}

fn update_title(g: &mut Game) {
    if any_key_pressed() { g.start_game(); }
}

fn update_serve(g: &mut Game, dt: f32) {
    if key_held(BLIP_KEY_UP)   || key_held(BLIP_KEY_W) { g.lpad_y -= PAD_SPEED * dt; }
    if key_held(BLIP_KEY_DOWN) || key_held(BLIP_KEY_S) { g.lpad_y += PAD_SPEED * dt; }
    g.clamp_pads();
    if any_key_pressed() {
        g.launch();
        g.state = State::Play;
    }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Beeps) {
    if key_held(BLIP_KEY_UP)   || key_held(BLIP_KEY_W) { g.lpad_y -= PAD_SPEED * dt; }
    if key_held(BLIP_KEY_DOWN) || key_held(BLIP_KEY_S) { g.lpad_y += PAD_SPEED * dt; }

    let target = g.ball_y + BALL_SZ * 0.5 - PAD_H * 0.5;
    let diff = target - g.rpad_y;
    let mv = AI_SPD * dt;
    g.rpad_y += if diff > mv { mv } else if diff < -mv { -mv } else { diff };

    g.clamp_pads();

    g.ball_x += g.ball_vx * dt;
    g.ball_y += g.ball_vy * dt;

    if g.ball_y <= PLAY_T {
        g.ball_y = PLAY_T;
        g.ball_vy = g.ball_vy.abs();
        play_sfx(&sfx.wall);
    }
    if g.ball_y + BALL_SZ >= PLAY_B {
        g.ball_y = PLAY_B - BALL_SZ;
        g.ball_vy = -g.ball_vy.abs();
        play_sfx(&sfx.wall);
    }

    if g.ball_vx < 0.0
        && rects_overlap(g.ball_x, g.ball_y, BALL_SZ, BALL_SZ,
                         LPAD_X, g.lpad_y, PAD_W, PAD_H)
    {
        g.ball_x = LPAD_X + PAD_W;
        let rel = (g.ball_y + BALL_SZ * 0.5 - g.lpad_y) / PAD_H - 0.5;
        g.ball_spd = (g.ball_spd + BALL_INC).min(BALL_MAX);
        g.ball_vx =  g.ball_spd * (rel * 1.1).cos();
        g.ball_vy =  g.ball_spd * (rel * 1.1).sin();
        play_sfx(&sfx.hit_l);
    }

    if g.ball_vx > 0.0
        && rects_overlap(g.ball_x, g.ball_y, BALL_SZ, BALL_SZ,
                         RPAD_X, g.rpad_y, PAD_W, PAD_H)
    {
        g.ball_x = RPAD_X - BALL_SZ;
        let rel = (g.ball_y + BALL_SZ * 0.5 - g.rpad_y) / PAD_H - 0.5;
        g.ball_spd = (g.ball_spd + BALL_INC).min(BALL_MAX);
        g.ball_vx = -g.ball_spd * (rel * 1.1).cos();
        g.ball_vy =  g.ball_spd * (rel * 1.1).sin();
        play_sfx(&sfx.hit_r);
    }

    if g.ball_x + BALL_SZ < 0.0 {
        g.score_r += 1;
        play_sfx(&sfx.score_r);
        if g.score_r >= SCORE_WIN { g.state = State::Over; }
        else { g.reset_for_serve(); g.point_t = 1.2; g.state = State::Point; }
    }
    if g.ball_x > WIN_W as f32 {
        g.score_l += 1;
        play_sfx(&sfx.score_l);
        if g.score_l >= SCORE_WIN { g.state = State::Over; }
        else { g.reset_for_serve(); g.point_t = 1.2; g.state = State::Point; }
    }
}

fn update_point(g: &mut Game, dt: f32) {
    g.point_t -= dt;
    if g.point_t <= 0.0 { g.state = State::Serve; }
}

fn update_over(g: &mut Game) {
    if !any_key_pressed() { return; }
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
    blip.draw_centered("RALLY",         cy - 28.0, 5.0, BLIP_YELLOW);
    blip.draw_centered("PRESS ANY KEY", cy + 30.0, 2.0, BLIP_GRAY);
}

fn draw_serve(blip: &Blip, g: &Game) {
    blip.clear(BLIP_BLACK);
    draw_field(blip, g);
    blip.fill_rect(g.ball_x, g.ball_y, BALL_SZ, BALL_SZ, BLIP_WHITE);
    blip.draw_centered("PRESS FIRE", PLAY_T + PLAY_H * 0.5 + 54.0, 2.0, BLIP_GRAY);
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
    if (g.point_t * 6.0) as i32 % 2 == 0 {
        blip.draw_centered("POINT!", PLAY_T + PLAY_H * 0.5, 3.0, BLIP_YELLOW);
    }
}

fn draw_over(blip: &Blip, g: &Game) {
    blip.clear(BLIP_BLACK);
    draw_net(blip);
    draw_hud(blip, g.score_l, g.score_r);
    let cy = PLAY_T + PLAY_H * 0.5;
    let msg = if g.score_l >= SCORE_WIN { "YOU WIN!" } else { "GAME OVER" };
    blip.draw_centered(msg,            cy - 20.0, 3.0, BLIP_YELLOW);
    blip.draw_centered("PRESS ANY KEY", cy + 24.0, 2.0, BLIP_GRAY);
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("RALLY", WIN_W, WIN_H)
}

const MUSIC_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music.wav"));

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let sfx = Beeps {
        wall:    blip::audio::beep(490.0,  22.0).await,
        hit_l:   blip::audio::beep(240.0,  35.0).await,
        hit_r:   blip::audio::beep(300.0,  35.0).await,
        score_l: blip::audio::beep(660.0, 120.0).await,
        score_r: blip::audio::beep(110.0, 200.0).await,
    };
    let music = blip::audio::load_sound(MUSIC_WAV).await;
    play_music(&music);

    loop {
        let dt = blip.delta_time;
        match g.state {
            State::Title => update_title(&mut g),
            State::Serve => update_serve(&mut g, dt),
            State::Play  => update_play(&mut g, dt, &sfx),
            State::Point => update_point(&mut g, dt),
            State::Over  => update_over(&mut g),
        }
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
