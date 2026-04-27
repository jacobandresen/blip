//! Serpent (Snake), Rust port of `games/serpent/main.c` on macroquad.

use blip::input::{
    any_key_pressed, key_pressed, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_DOWN, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::texture::{FilterMode, Texture2D};
use blip::macroquad::prelude::ImageFormat;
use blip::{
    play_music, play_sfx, rand_int, web, window_conf, Blip, BlipColor, BLIP_BLACK, BLIP_GRAY,
    BLIP_GREEN, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
};

// ---- layout -----------------------------------------------------------
const COLS: i32 = 20;
const ROWS: i32 = 20;
const CELL: i32 = 24;
const HUD_H: i32 = 28;
const WIN_W: i32 = COLS * CELL;
const WIN_H: i32 = ROWS * CELL + HUD_H;
const MAX_LEN: usize = (COLS * ROWS) as usize;

// ---- tuning -----------------------------------------------------------
const LIVES_START: i32 = 3;
const SPEED_START: f32 = 160.0;
const SPEED_MIN: f32 = 70.0;
const SPEED_STEP: f32 = 10.0;
const FOODS_PER_LVL: i32 = 5;

#[derive(Copy, Clone, PartialEq, Eq)]
enum Dir { Up, Right, Down, Left }

#[derive(Copy, Clone, PartialEq, Eq)]
struct Cell { c: i32, r: i32 }

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Play, Dead, Over }

struct Game {
    snake: [Cell; MAX_LEN],
    snake_head: usize,
    snake_len: usize,
    cur_dir: Dir,
    want_dir: Dir,
    food: Cell,
    score: i32,
    hi_score: i32,
    lives: i32,
    level: i32,
    foods_eaten: i32,
    move_timer: f32,
    dead_timer: f32,
    state: State,
}

impl Game {
    fn new() -> Self {
        Self {
            snake: [Cell { c: 0, r: 0 }; MAX_LEN],
            snake_head: 0,
            snake_len: 0,
            cur_dir: Dir::Right,
            want_dir: Dir::Right,
            food: Cell { c: 0, r: 0 },
            score: 0,
            hi_score: 0,
            lives: 0,
            level: 1,
            foods_eaten: 0,
            move_timer: 0.0,
            dead_timer: 0.0,
            state: State::Title,
        }
    }

    #[inline]
    fn snake_at(&self, i: usize) -> Cell {
        self.snake[(self.snake_head + i) % MAX_LEN]
    }

    fn move_interval(&self) -> f32 {
        let ms = SPEED_START - (self.level - 1) as f32 * SPEED_STEP;
        if ms < SPEED_MIN { SPEED_MIN } else { ms }
    }

    fn spawn_food(&mut self) {
        loop {
            let f = Cell {
                c: rand_int(0, COLS - 1),
                r: rand_int(0, ROWS - 1),
            };
            let mut ok = true;
            for i in 0..self.snake_len {
                let b = self.snake_at(i);
                if b.c == f.c && b.r == f.r { ok = false; break; }
            }
            if ok { self.food = f; return; }
        }
    }

    fn reset_snake(&mut self) {
        self.snake_head = 0;
        self.snake_len = 4;
        self.cur_dir = Dir::Right;
        self.want_dir = Dir::Right;
        for i in 0..self.snake_len {
            self.snake[i] = Cell { c: COLS / 2 - i as i32, r: ROWS / 2 };
        }
        self.spawn_food();
        self.move_timer = 0.0;
    }

    fn start_game(&mut self) {
        self.score = 0;
        self.level = 1;
        self.foods_eaten = 0;
        self.lives = LIVES_START;
        self.reset_snake();
        self.state = State::Play;
    }
}

struct Sounds {
    eat: blip::BlipSound,
    game_over: blip::BlipSound,
}

fn update_title(g: &mut Game) {
    if any_key_pressed() { g.start_game(); }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    if (key_pressed(BLIP_KEY_UP)    || key_pressed(BLIP_KEY_W)) && g.cur_dir != Dir::Down  { g.want_dir = Dir::Up; }
    if (key_pressed(BLIP_KEY_DOWN)  || key_pressed(BLIP_KEY_S)) && g.cur_dir != Dir::Up    { g.want_dir = Dir::Down; }
    if (key_pressed(BLIP_KEY_LEFT)  || key_pressed(BLIP_KEY_A)) && g.cur_dir != Dir::Right { g.want_dir = Dir::Left; }
    if (key_pressed(BLIP_KEY_RIGHT) || key_pressed(BLIP_KEY_D)) && g.cur_dir != Dir::Left  { g.want_dir = Dir::Right; }

    g.move_timer += dt * 1000.0;
    if g.move_timer < g.move_interval() { return; }
    g.move_timer -= g.move_interval();
    g.cur_dir = g.want_dir;

    let mut h = g.snake_at(0);
    match g.cur_dir {
        Dir::Up => h.r -= 1,
        Dir::Down => h.r += 1,
        Dir::Left => h.c -= 1,
        Dir::Right => h.c += 1,
    }

    let mut dead = h.c < 0 || h.c >= COLS || h.r < 0 || h.r >= ROWS;
    if !dead {
        for i in 0..g.snake_len.saturating_sub(1) {
            let b = g.snake_at(i);
            if b.c == h.c && b.r == h.r { dead = true; break; }
        }
    }
    if dead {
        play_sfx(&sfx.game_over);
        g.lives -= 1;
        if g.lives > 0 {
            g.dead_timer = 1.5;
            g.state = State::Dead;
        } else {
            g.state = State::Over;
        }
        return;
    }

    let ate = h.c == g.food.c && h.r == g.food.r;
    if ate {
        play_sfx(&sfx.eat);
        g.score += 10 * g.level;
        if g.score > g.hi_score { g.hi_score = g.score; }
        g.foods_eaten += 1;
        if g.foods_eaten >= FOODS_PER_LVL {
            g.level += 1;
            g.foods_eaten = 0;
        }
        g.spawn_food();
    }

    g.snake_head = (g.snake_head + MAX_LEN - 1) % MAX_LEN;
    g.snake[g.snake_head] = h;
    if ate && g.snake_len < MAX_LEN { g.snake_len += 1; }
}

fn update_dead(g: &mut Game, dt: f32) {
    g.dead_timer -= dt;
    if g.dead_timer <= 0.0 {
        g.reset_snake();
        g.state = State::Play;
    }
}

fn update_over(g: &mut Game) {
    if !any_key_pressed() { return; }
    web::spend_coin();
    g.start_game();
}

fn draw_board(blip: &Blip) {
    let grid = BlipColor { r: 18.0/255.0, g: 18.0/255.0, b: 18.0/255.0, a: 1.0 };
    for c in 0..=COLS {
        blip.draw_line((c * CELL) as f32, HUD_H as f32,
                       (c * CELL) as f32, WIN_H as f32, grid);
    }
    for r in 0..=ROWS {
        blip.draw_line(0.0, (HUD_H + r * CELL) as f32,
                       WIN_W as f32, (HUD_H + r * CELL) as f32, grid);
    }
}

fn draw_snake(blip: &Blip, g: &Game, head: &Texture2D, body: &Texture2D) {
    for i in (1..g.snake_len).rev() {
        let b = g.snake_at(i);
        blip.draw_texture_tinted(
            body,
            (b.c * CELL) as f32,
            (HUD_H + b.r * CELL) as f32,
            CELL as f32, CELL as f32, BLIP_WHITE,
        );
    }
    let h = g.snake_at(0);
    blip.draw_texture_tinted(
        head,
        (h.c * CELL) as f32,
        (HUD_H + h.r * CELL) as f32,
        CELL as f32, CELL as f32, BLIP_WHITE,
    );
}

fn draw_play(blip: &Blip, g: &Game, head: &Texture2D, body: &Texture2D, food: &Texture2D) {
    draw_board(blip);
    blip.draw_texture(food,
        (g.food.c * CELL) as f32,
        (HUD_H + g.food.r * CELL) as f32,
        CELL as f32, CELL as f32);
    draw_snake(blip, g, head, body);
    blip.draw_hud(g.score, g.hi_score, g.lives);
}

fn draw_title(blip: &Blip) {
    blip.clear(BLIP_BLACK);
    blip.draw_centered("SERPENT",            (WIN_H / 4) as f32,       6.0, BLIP_GREEN);
    blip.draw_centered("PRESS ANY KEY",      (WIN_H / 2) as f32,       3.0, BLIP_WHITE);
    blip.draw_centered("ARROW KEYS OR WASD", (WIN_H * 2 / 3) as f32,   2.0, BLIP_GRAY);
}

fn draw_over(blip: &Blip, score: i32) {
    let buf = format!("SCORE {}", score);
    blip.clear(BLIP_BLACK);
    blip.draw_centered("GAME OVER",     (WIN_H / 4) as f32,     5.0, BLIP_RED);
    blip.draw_centered(&buf,            (WIN_H / 2) as f32,     3.0, BLIP_WHITE);
    blip.draw_centered("PRESS ANY KEY", (WIN_H * 2 / 3) as f32, 3.0, BLIP_YELLOW);
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("SERPENT", WIN_W, WIN_H)
}

const HEAD_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/head.png"));
const BODY_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/body.png"));
const FOOD_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/food.png"));
const EAT_WAV: &[u8]  = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/eat.wav"));
const GAME_OVER_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/game_over.wav"));
const MUSIC_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music.wav"));

fn load_png(bytes: &'static [u8]) -> Texture2D {
    let tex = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
    tex.set_filter(FilterMode::Nearest);
    tex
}

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let head = load_png(HEAD_PNG);
    let body = load_png(BODY_PNG);
    let food = load_png(FOOD_PNG);

    let sfx = Sounds {
        eat:       blip::audio::load_sound(EAT_WAV).await,
        game_over: blip::audio::load_sound(GAME_OVER_WAV).await,
    };
    let music = blip::audio::load_sound(MUSIC_WAV).await;
    play_music(&music);

    loop {
        let dt = blip.delta_time;
        match g.state {
            State::Title => update_title(&mut g),
            State::Play  => update_play(&mut g, dt, &sfx),
            State::Dead  => update_dead(&mut g, dt),
            State::Over  => update_over(&mut g),
        }

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title => draw_title(&blip),
            State::Over  => draw_over(&blip, g.score),
            State::Play | State::Dead => draw_play(&blip, &g, &head, &body, &food),
        }

        blip.next_frame(60).await;
    }
}
