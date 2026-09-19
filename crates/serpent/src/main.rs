//! Serpent (Snake), Rust port of `games/serpent/main.c` on macroquad.

use blip::input::{
    btn1_pressed, key_pressed, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_DOWN, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::texture::{FilterMode, Texture2D};
use blip::macroquad::prelude::ImageFormat;
use blip::{
    lerp, play_music, play_sfx, rand_int, web, window_conf, Blip, BlipColor, LifeResult, Session,
    Timer, BLIP_BLACK, BLIP_GRAY, BLIP_GREEN, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
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
const SPEED_START: f32 = 180.0;
const SPEED_MIN: f32 = 70.0;
const SPEED_STEP: f32 = 10.0;
const FOODS_PER_LVL: i32 = 5;

// ---- bonus fruit --------------------------------------------------------
// Ordinary food is not a decision: it sits there until eaten, so the only
// question is which way round the board to go, and a careful player never
// has a reason to hurry. The bonus is the opposite — it is worth five
// ordinary foods, it is somewhere else, and it is leaving. Whether to go
// for it is the first real choice the game asks, and the answer changes
// with how long the snake has got.
const BONUS_EVERY: i32 = 3;      // which food of the level brings one out
const BONUS_TTL: f32 = 6.0;      // seconds on the board
const BONUS_WARN: f32 = 2.0;     // when it starts flashing out
const BONUS_VALUE: i32 = 50;     // x level, against ordinary food's 10
// A hard floor on how long GAME OVER stays up before a key can dismiss it —
// a reflexive direction-key press right after dying would otherwise bounce
// straight back to the title screen before the score is even readable.
const GAME_OVER_MIN_WAIT: f32 = 2.0;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Dir { Up, Right, Down, Left }

impl Dir {
    fn opposite(self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
        }
    }
}

/// How many turns may be held ahead of the snake.
///
/// One is not enough, and that is the whole reason this exists. The
/// snake steps every 180ms at its slowest and 70ms at its fastest, but
/// a player rounding a corner presses *two* directions in one human
/// gesture — right, then down — far faster than that. With a single
/// slot the second press overwrote the first, the snake never turned
/// right, and it drove into the wall the player had just steered away
/// from. The press was not mistimed; it was thrown away.
///
/// Two is the number that matches the gesture. Three would let a
/// player queue a path the snake has not visibly committed to yet,
/// which is a different game.
const TURN_QUEUE: usize = 2;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct Cell { c: i32, r: i32 }

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Play, Dead, Over }

struct Game {
    snake: [Cell; MAX_LEN],
    snake_head: usize,
    snake_len: usize,
    cur_dir: Dir,
    /// Turns the player has asked for but the snake has not taken yet.
    /// See TURN_QUEUE.
    turns: [Dir; TURN_QUEUE],
    turns_len: usize,
    food: Cell,
    /// The timed bonus, when one is on the board. `bonus_ttl` counts down
    /// in seconds and is what makes it a decision rather than a pickup.
    bonus: Cell,
    bonus_ttl: f32,
    sess: Session,
    foods_eaten: i32,
    move_timer: f32,
    dead_timer: Timer,
    state: State,
    obstacles: [Cell; 16],
    obstacle_count: usize,
    active_music: i32,
    want_track: i32,
}

impl Game {
    fn new() -> Self {
        Self {
            snake: [Cell { c: 0, r: 0 }; MAX_LEN],
            snake_head: 0,
            snake_len: 0,
            cur_dir: Dir::Right,
            turns: [Dir::Right; TURN_QUEUE],
            turns_len: 0,
            food: Cell { c: 0, r: 0 },
            sess: Session::new(LIVES_START),
            bonus: Cell { c: 0, r: 0 },
            bonus_ttl: 0.0,
            foods_eaten: 0,
            move_timer: 0.0,
            dead_timer: Timer::default(),
            state: State::Title,
            obstacles: [Cell { c: 0, r: 0 }; 16],
            obstacle_count: 0,
            active_music: 0,
            want_track: 0,
        }
    }

    #[inline]
    fn snake_at(&self, i: usize) -> Cell {
        self.snake[(self.snake_head + i) % MAX_LEN]
    }

    fn move_interval(&self) -> f32 {
        let ms = SPEED_START - (self.sess.level - 1) as f32 * SPEED_STEP;
        if ms < SPEED_MIN { SPEED_MIN } else { ms }
    }

    /// Is this cell clear of the snake and the obstacle field?
    ///
    /// From level 2 on the board has obstacle blocks — without this check
    /// food can land inside one and be permanently unreachable (it only
    /// ever relocates when eaten), softlocking the level.
    fn cell_is_free(&self, f: Cell) -> bool {
        for i in 0..self.snake_len {
            let b = self.snake_at(i);
            if b.c == f.c && b.r == f.r { return false; }
        }
        for i in 0..self.obstacle_count {
            if self.obstacles[i].c == f.c && self.obstacles[i].r == f.r { return false; }
        }
        true
    }

    fn spawn_food(&mut self) {
        loop {
            let f = Cell { c: rand_int(0, COLS - 1), r: rand_int(0, ROWS - 1) };
            if self.cell_is_free(f) && !(self.bonus_active() && f == self.bonus) {
                self.food = f;
                return;
            }
        }
    }

    fn bonus_active(&self) -> bool { self.bonus_ttl > 0.0 }

    /// Put a bonus somewhere the snake has to travel to reach.
    ///
    /// Deliberately not just "any free cell": a bonus that lands under the
    /// snake's nose is worth five foods for no decision at all, which is
    /// the one outcome that makes the rest of the board pointless. Tries
    /// for somewhere a third of the board away, and settles for anywhere
    /// free rather than spinning if the board is nearly full.
    fn spawn_bonus(&mut self) {
        let head = self.snake_at(0);
        let far_enough = (COLS + ROWS) / 3;
        // Two passes: somewhere worth travelling to, and then — only if
        // the board is too full to offer one — anywhere at all. The
        // second pass exists so a late, long-snake level still gets its
        // bonus instead of silently skipping it.
        for relaxed in [false, true] {
            for _ in 0..64 {
                let b = Cell { c: rand_int(0, COLS - 1), r: rand_int(0, ROWS - 1) };
                if !self.cell_is_free(b) || b == self.food { continue; }
                let dist = (b.c - head.c).abs() + (b.r - head.r).abs();
                if !relaxed && dist < far_enough { continue; }
                self.bonus = b;
                self.bonus_ttl = BONUS_TTL;
                return;
            }
        }
    }

    /// Set the obstacle field from a list of (dc, dr) offsets from the board centre.
    fn set_obstacles(&mut self, offsets: &[(i32, i32)]) {
        let ch = COLS / 2;
        let rh = ROWS / 2;
        self.obstacle_count = offsets.len();
        for (i, (dc, dr)) in offsets.iter().enumerate() {
            self.obstacles[i] = Cell { c: ch + dc, r: rh + dr };
        }
    }

    /// Level 1 is always obstacle-free; from level 2 on, five distinct
    /// obstacle layouts cycle forever so the field keeps changing shape.
    fn build_obstacles(&mut self) {
        self.obstacle_count = 0;
        if self.sess.level == 1 { return; }
        match (self.sess.level - 2).rem_euclid(5) {
            0 => self.set_obstacles(&[             // plus sign
                (-1, 0), (0, 0), (0, -1), (0, 1),
            ]),
            1 => self.set_obstacles(&[             // ring
                (-1, -1), (0, -1), (1, -1),
                (-1, 0),           (1, 0),
                (-1, 1),  (0, 1),  (1, 1),
            ]),
            2 => self.set_obstacles(&[             // two vertical walls with gaps
                (-4, -3), (-4, -1), (-4, 1), (-4, 3),
                (4, -3),  (4, -1),  (4, 1),  (4, 3),
            ]),
            3 => self.set_obstacles(&[             // four corner blocks
                (-7, -7), (-6, -7), (-7, -6), (-6, -6),
                (6, -7),  (7, -7),  (6, -6),  (7, -6),
                (-7, 6),  (-6, 6),  (-7, 7),  (-6, 7),
                (6, 6),   (7, 6),   (6, 7),   (7, 7),
            ]),
            _ => self.set_obstacles(&[             // scattered diagonal
                (-8, -8), (-6, -6), (-4, -4), (4, 4), (6, 6), (8, 8),
                (-8, 8), (-6, 6), (-4, 4), (4, -4), (6, -6), (8, -8),
            ]),
        }
    }

    /// Remember a turn the player asked for, if it is one the snake can
    /// actually take.
    ///
    /// Validated against the *last direction queued* rather than the one
    /// the snake is travelling in right now. Those differ exactly when a
    /// turn is already waiting, and checking the wrong one is how a
    /// queue lets a player double back into their own neck: travelling
    /// right, queue up, then queue left — "left" is not the reverse of
    /// "right"'s successor, it is the reverse of nothing, and both turns
    /// are legal in sequence.
    fn queue_turn(&mut self, dir: Dir) {
        let last = if self.turns_len == 0 { self.cur_dir } else { self.turns[self.turns_len - 1] };
        if dir == last || dir == last.opposite() { return; }
        if self.turns_len == TURN_QUEUE { return; }
        self.turns[self.turns_len] = dir;
        self.turns_len += 1;
    }

    /// The direction for this step: the oldest turn still waiting, or
    /// carry straight on.
    fn take_turn(&mut self) -> Dir {
        if self.turns_len == 0 { return self.cur_dir; }
        let next = self.turns[0];
        for i in 1..self.turns_len { self.turns[i - 1] = self.turns[i]; }
        self.turns_len -= 1;
        next
    }

    fn reset_snake(&mut self) {
        self.build_obstacles();
        self.snake_head = 0;
        self.snake_len = 4;
        self.cur_dir = Dir::Right;
        self.turns_len = 0;
        // A bonus left over from the life just lost would be counting down
        // against a snake that was not on the board when it appeared.
        self.bonus_ttl = 0.0;
        for i in 0..self.snake_len {
            self.snake[i] = Cell { c: COLS / 2 - i as i32, r: ROWS / 2 };
        }
        self.spawn_food();
        self.move_timer = 0.0;
    }

    fn start_game(&mut self) {
        self.sess.reset(LIVES_START);
        self.foods_eaten = 0;
        self.reset_snake();
        self.want_track = rand_int(1, 3);
        self.state = State::Play;
    }
}

struct Sounds {
    eat: blip::BlipSound,
    game_over: blip::BlipSound,
}

fn update_title(g: &mut Game) {
    if btn1_pressed() { g.start_game(); }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    if key_pressed(BLIP_KEY_UP)    || key_pressed(BLIP_KEY_W) { g.queue_turn(Dir::Up); }
    if key_pressed(BLIP_KEY_DOWN)  || key_pressed(BLIP_KEY_S) { g.queue_turn(Dir::Down); }
    if key_pressed(BLIP_KEY_LEFT)  || key_pressed(BLIP_KEY_A) { g.queue_turn(Dir::Left); }
    if key_pressed(BLIP_KEY_RIGHT) || key_pressed(BLIP_KEY_D) { g.queue_turn(Dir::Right); }

    // Counted down in real time rather than in steps. The snake speeds up
    // as the level climbs, so a bonus measured in steps would quietly get
    // *longer* to reach exactly as the game got harder — the deadline has
    // to be the same six seconds whatever speed the snake is doing.
    if g.bonus_active() {
        g.bonus_ttl -= dt;
        if g.bonus_ttl < 0.0 { g.bonus_ttl = 0.0; }
    }

    g.move_timer += dt * 1000.0;
    if g.move_timer < g.move_interval() { return; }
    g.move_timer -= g.move_interval();
    g.cur_dir = g.take_turn();

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
    if !dead {
        for i in 0..g.obstacle_count {
            if g.obstacles[i].c == h.c && g.obstacles[i].r == h.r { dead = true; break; }
        }
    }
    if dead {
        play_sfx(&sfx.game_over);
        match g.sess.lose_life() {
            LifeResult::StillAlive => { g.dead_timer.start(1.5); g.state = State::Dead; }
            LifeResult::GameOver   => {
                g.dead_timer.start(GAME_OVER_MIN_WAIT);
                g.state = State::Over;
                web::report_score(g.sess.score);
            }
        }
        return;
    }

    // The bonus is taken before the level check below, so a bonus grabbed
    // on the same step that finishes a level still scores at the level it
    // was offered on rather than the next one.
    if g.bonus_active() && h == g.bonus {
        play_sfx(&sfx.eat);
        g.sess.add_score(BONUS_VALUE * g.sess.level);
        g.bonus_ttl = 0.0;
    }

    let ate = h.c == g.food.c && h.r == g.food.r;
    if ate {
        play_sfx(&sfx.eat);
        g.sess.add_score(10 * g.sess.level);
        g.foods_eaten += 1;
        // `foods_eaten` restarts at every level (five foods to a level),
        // so this is one bonus per level, on the third food — far enough
        // in that the snake has some length to manage, far enough from
        // the level change that the two events do not land together.
        if g.foods_eaten % BONUS_EVERY == 0 && !g.bonus_active() { g.spawn_bonus(); }
        if g.foods_eaten >= FOODS_PER_LVL {
            g.sess.next_level();
            g.foods_eaten = 0;
            let next = rand_int(1, 3);
            g.want_track = if next == g.active_music { next % 3 + 1 } else { next };
        }
        g.spawn_food();
    }

    g.snake_head = (g.snake_head + MAX_LEN - 1) % MAX_LEN;
    g.snake[g.snake_head] = h;
    if ate && g.snake_len < MAX_LEN { g.snake_len += 1; }
}

fn update_dead(g: &mut Game, dt: f32) {
    if g.dead_timer.tick(dt) {
        g.reset_snake();
        g.state = State::Play;
    }
}

fn update_over(g: &mut Game, dt: f32) {
    g.dead_timer.tick(dt);
    if g.dead_timer.active() || !btn1_pressed() { return; }
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
    // Visible wall border — shows the snake's movement boundary
    let wall = BlipColor { r: 0.20, g: 0.42, b: 0.20, a: 1.0 };
    let x0 = 0.0_f32;
    let y0 = HUD_H as f32;
    let w  = WIN_W as f32;
    let h  = (WIN_H - HUD_H) as f32;
    blip.fill_rect(x0,         y0,         w,   2.0, wall); // top
    blip.fill_rect(x0,         y0 + h - 2.0, w,   2.0, wall); // bottom
    blip.fill_rect(x0,         y0,         2.0, h,   wall); // left
    blip.fill_rect(x0 + w - 2.0, y0,         2.0, h,   wall); // right
}

fn draw_snake(blip: &Blip, g: &Game, head: &Texture2D, body: &Texture2D) {
    // Slide each segment from the cell it left toward the cell it's entering,
    // by how far through the current step-tick we are, so the snake glides
    // instead of jumping a whole cell at a time. Pure rendering — the game
    // logic stays on its clean integer grid. Only while actually moving,
    // though: the fatal tick returns before ever touching move_timer again,
    // so once dead it sits stalled at whatever tiny fraction it was on — and
    // rendering that fraction would show the snake still gliding in *toward*
    // its last cell, a step short of the wall or body it hit. Dead freezes
    // it fully arrived instead, right on the edge it died against.
    let f = if g.state == State::Play {
        (g.move_timer / g.move_interval()).clamp(0.0, 1.0)
    } else {
        1.0
    };
    // Death flash: blink the whole snake red/white for as long as
    // dead_timer is counting down — the classic arcade "you got hit" tell.
    let tint = if g.state == State::Dead && (g.dead_timer.remaining() / 0.12) as i32 % 2 == 0 {
        BLIP_RED
    } else {
        BLIP_WHITE
    };
    let seg_px = |i: usize| -> (f32, f32) {
        let cur = g.snake_at(i);
        let prev = if i + 1 < g.snake_len {
            g.snake_at(i + 1)
        } else {
            // the tail: extrapolate the cell one step back along the body
            let ahead = g.snake_at(i - 1);
            Cell { c: 2 * cur.c - ahead.c, r: 2 * cur.r - ahead.r }
        };
        let x = lerp(prev.c as f32, cur.c as f32, f) * CELL as f32;
        let y = HUD_H as f32 + lerp(prev.r as f32, cur.r as f32, f) * CELL as f32;
        (x, y)
    };
    for i in (1..g.snake_len).rev() {
        let (x, y) = seg_px(i);
        blip.draw_texture_tinted(body, x, y, CELL as f32, CELL as f32, tint);
    }
    let (x, y) = seg_px(0);
    blip.draw_texture_tinted(head, x, y, CELL as f32, CELL as f32, tint);
}

fn draw_play(blip: &Blip, g: &Game, head: &Texture2D, body: &Texture2D, food: &Texture2D) {
    draw_board(blip);
    for i in 0..g.obstacle_count {
        let obs = g.obstacles[i];
        blip.fill_rect(
            (obs.c * CELL) as f32,
            (HUD_H + obs.r * CELL) as f32,
            CELL as f32, CELL as f32,
            BLIP_GRAY,
        );
    }
    blip.draw_texture(food,
        (g.food.c * CELL) as f32,
        (HUD_H + g.food.r * CELL) as f32,
        CELL as f32, CELL as f32);
    draw_bonus(blip, g);
    draw_snake(blip, g, head, body);
    blip.draw_hud(g.sess.score, g.sess.lives);
}

/// The timed bonus: a gold star that shrinks as its clock runs out, and
/// blinks for the last two seconds.
///
/// The countdown is drawn into the piece itself rather than put in the
/// HUD, because the decision it drives — go for it, or keep working the
/// board — is made while looking at the snake, not at the score. A number
/// at the top of the screen would be a number nobody reads in time.
fn draw_bonus(blip: &Blip, g: &Game) {
    if !g.bonus_active() { return; }
    // Blink out at the end. Off for the shorter part of each cycle, so the
    // thing spends most of its last seconds visible and still findable —
    // a 50/50 blink reads as gone half the time.
    let expiring = g.bonus_ttl <= BONUS_WARN;
    if expiring && (g.bonus_ttl * 6.0) as i32 % 3 == 0 { return; }

    let cx = (g.bonus.c * CELL) as f32 + CELL as f32 / 2.0;
    let cy = (HUD_H + g.bonus.r * CELL) as f32 + CELL as f32 / 2.0;
    // Shrinks with the clock, but never below half — the target still has
    // to be worth aiming at on the step you finally reach it.
    let life = g.bonus_ttl / BONUS_TTL;
    let r = CELL as f32 * (0.26 + 0.20 * life);
    let gold = BlipColor { r: 1.0, g: 0.84, b: 0.20, a: 1.0 };
    let hot = BlipColor { r: 1.0, g: 0.98, b: 0.72, a: 1.0 };
    blip.fill_circle(cx, cy, r, gold);
    blip.fill_circle(cx, cy, r * 0.45, hot);
}

fn draw_title(blip: &Blip, hi: &web::HighScore) {
    blip.clear(BLIP_BLACK);
    blip.draw_centered("SERPENT",            (WIN_H / 4) as f32,       6.0, BLIP_GREEN);
    // SERPENT is sz=6 (42px tall) — clear its bottom by a real margin.
    blip.draw_hi(hi, (WIN_H / 4 + 50) as f32, BLIP_YELLOW);
    blip.draw_centered("PRESS FIRE",         (WIN_H / 2) as f32,       3.0, BLIP_WHITE);
    blip.draw_centered("ARROW KEYS OR WASD", (WIN_H * 2 / 3) as f32,   2.0, BLIP_GRAY);
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
    window_conf("SERPENT", WIN_W, WIN_H)
}

const HEAD_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/head.png"));
const BODY_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/body.png"));
const FOOD_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/food.png"));
const EAT_WAV: &[u8]  = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/eat.wav"));
const GAME_OVER_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/game_over.wav"));
const SLITHER_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/slither.wav"));
const STALK_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/stalk.wav"));
const FRENZY_WAV:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/frenzy.wav"));

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
    let slither = blip::audio::load_sound(SLITHER_WAV).await;
    let stalk   = blip::audio::load_sound(STALK_WAV).await;
    let frenzy  = blip::audio::load_sound(FRENZY_WAV).await;
    let tracks = [&slither, &stalk, &frenzy];
    play_music(&slither);
    g.active_music = 1;

    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;

        if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 {
                g.start_game();
            }
        }

        match g.state {
            State::Title => update_title(&mut g),
            State::Play  => update_play(&mut g, dt, &sfx),
            State::Dead  => update_dead(&mut g, dt),
            State::Over  => update_over(&mut g, dt),
        }

        if g.want_track != 0 && g.want_track != g.active_music {
            play_music(tracks[(g.want_track - 1) as usize]);
            g.active_music = g.want_track;
            g.want_track = 0;
        }

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title => draw_title(&blip, &web::high_score()),
            State::Over  => draw_over(&blip, g.sess.score, &web::high_score(), g.dead_timer.active()),
            State::Play | State::Dead => draw_play(&blip, &g, &head, &body, &food),
        }

        blip.next_frame(60).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A snake set up mid-board, travelling right, with nothing queued.
    fn running() -> Game {
        let mut g = Game::new();
        g.reset_snake();
        g
    }

    /// One move step, exactly as update_play() takes it: the turn comes
    /// off the queue and becomes the direction travelled from now on.
    /// Calling take_turn() alone would not commit it, and a test that
    /// did so would be asserting against a snake that never turned.
    fn step(g: &mut Game) -> Dir {
        g.cur_dir = g.take_turn();
        g.cur_dir
    }

    #[test]
    fn a_corner_taken_faster_than_one_step_keeps_both_turns() {
        // The defect this queue exists for. Travelling right, the player
        // rounds a corner with one gesture — down, then left — well
        // inside a single 180ms step. With a single slot the "down" was
        // overwritten and never happened: the snake carried straight on
        // into whatever the player was steering around.
        let mut g = running();
        g.queue_turn(Dir::Down);
        g.queue_turn(Dir::Left);

        assert_eq!(step(&mut g), Dir::Down, "the first turn of the gesture was dropped");
        assert_eq!(step(&mut g), Dir::Left, "the second turn of the gesture was dropped");
        assert_eq!(step(&mut g), Dir::Left, "with nothing queued the snake carries on");
    }

    #[test]
    fn a_queued_turn_cannot_double_back_into_the_neck() {
        // The bug a queue introduces if it validates against the
        // direction the snake is travelling in *now* rather than the
        // last one queued. Travelling right: "up" is legal, and then
        // "left" is legal against `cur_dir` (right) while being an
        // immediate reversal of the "up" that will have happened by
        // then. Taken in sequence that is a U-turn into its own neck.
        let mut g = running();
        g.queue_turn(Dir::Up);
        g.queue_turn(Dir::Down);

        assert_eq!(step(&mut g), Dir::Up);
        assert_eq!(step(&mut g), Dir::Up, "a reversal of the queued turn was accepted");
    }

    #[test]
    fn a_reversal_is_still_refused_with_nothing_queued() {
        let mut g = running(); // travelling right
        g.queue_turn(Dir::Left);
        assert_eq!(step(&mut g), Dir::Right, "the snake reversed into itself");
    }

    #[test]
    fn holding_a_direction_does_not_fill_the_queue() {
        // Repeats of the direction already being travelled are not
        // turns, and must not push a real turn out of the queue.
        let mut g = running();
        g.queue_turn(Dir::Right);
        g.queue_turn(Dir::Right);
        g.queue_turn(Dir::Down);
        assert_eq!(step(&mut g), Dir::Down, "a real turn was crowded out by repeats");
    }

    #[test]
    fn the_queue_is_bounded() {
        // Three turns inside one step is no longer a gesture, it is a
        // path the snake has not committed to. The third is dropped
        // rather than remembered.
        let mut g = running();
        g.queue_turn(Dir::Down);
        g.queue_turn(Dir::Left);
        g.queue_turn(Dir::Up);
        assert_eq!(g.turns_len, TURN_QUEUE);
        assert_eq!(step(&mut g), Dir::Down);
        assert_eq!(step(&mut g), Dir::Left);
        assert_eq!(step(&mut g), Dir::Left, "a third turn inside one step was remembered anyway");
    }

    #[test]
    fn a_bonus_never_lands_on_the_snake_the_food_or_an_obstacle() {
        // Any of those is a bonus that cannot be taken: inside the snake
        // it is unreachable, on the food it is taken by accident, in an
        // obstacle it is a reward for dying.
        let mut g = running();
        g.sess.level = 3;          // levels past 1 have an obstacle field
        g.build_obstacles();
        for _ in 0..200 {
            g.bonus_ttl = 0.0;
            g.spawn_bonus();
            assert!(g.bonus_active(), "no bonus was placed on an almost-empty board");
            assert!(g.cell_is_free(g.bonus), "the bonus landed on the snake or an obstacle");
            assert_ne!(g.bonus, g.food, "the bonus landed on the ordinary food");
        }
    }

    #[test]
    fn a_bonus_is_placed_away_from_the_snakes_head() {
        // A bonus that appears under the nose is five foods for no
        // decision, which is the one outcome that makes the rest of the
        // board pointless.
        let mut g = running();
        let head = g.snake_at(0);
        let far_enough = (COLS + ROWS) / 3;
        let mut close = 0;
        for _ in 0..200 {
            g.bonus_ttl = 0.0;
            g.spawn_bonus();
            if (g.bonus.c - head.c).abs() + (g.bonus.r - head.r).abs() < far_enough { close += 1; }
        }
        assert_eq!(close, 0, "{close}/200 bonuses appeared within reach of the head");
    }

    #[test]
    fn ordinary_food_never_replaces_a_bonus_that_is_still_up() {
        // spawn_food() runs on the same step a bonus can be live, and two
        // pieces on one cell is one of them silently eating the other.
        let mut g = running();
        g.spawn_bonus();
        let bonus = g.bonus;
        for _ in 0..200 {
            g.spawn_food();
            assert_ne!(g.food, bonus, "food was placed on top of the live bonus");
        }
    }

    #[test]
    fn a_bonus_does_not_survive_the_life_that_earned_it() {
        let mut g = running();
        g.spawn_bonus();
        assert!(g.bonus_active());
        g.reset_snake();
        assert!(!g.bonus_active(), "the bonus carried over into the next life");
    }

    #[test]
    fn a_new_snake_starts_with_an_empty_queue() {
        // Turns left over from the life just lost would steer the new
        // snake on its first step, which reads as the game moving on its
        // own before the player has touched anything.
        let mut g = running();
        g.queue_turn(Dir::Down);
        g.reset_snake();
        assert_eq!(g.turns_len, 0);
        assert_eq!(step(&mut g), Dir::Right);
    }
}
