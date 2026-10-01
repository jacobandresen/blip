//! Serpent (Snake), Rust port of `games/serpent/main.c` on macroquad.

use blip::input::{
    btn1_pressed, key_pressed, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_DOWN, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::texture::Texture2D;
use blip::{
    GAME_OVER_MIN_WAIT,
    load_png,
    lerp, play_sfx, Jukebox, rand_int, web, window_conf, Blip, BlipColor, Fx, LifeResult, Session,
    Timer, BLIP_BLACK, BLIP_GRAY, BLIP_GREEN, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
};

#[cfg(not(target_arch = "wasm32"))]
mod bot;

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
const FRENZY_LEVEL: i32 = 5;     // the fast tune from here on

// ---- bonus fruit --------------------------------------------------------
// Worth five foods, somewhere else, and leaving: whether to go for it is the
// game's first real choice.
const BONUS_EVERY: i32 = 3;      // which food of the level brings one out
const BONUS_TTL: f32 = 6.0;      // seconds on the board
const BONUS_WARN: f32 = 2.0;     // when it starts flashing out
const BONUS_VALUE: i32 = 50;     // x level, against ordinary food's 10
// A new level's obstacles arrive mid-run; this long to see them before they are solid.
const OBS_FADE_SECS: f32 = 1.2;
const BANNER_SECS: f32 = 1.4;

const GOLD: BlipColor = BlipColor { r: 1.0, g: 0.84, b: 0.20, a: 1.0 };

/// The pixel centre of a board cell.
fn cell_centre(c: Cell) -> (f32, f32) {
    ((c.c * CELL + CELL / 2) as f32, (HUD_H + c.r * CELL + CELL / 2) as f32)
}

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

/// How many turns may be held ahead of the snake. Rounding a corner is two
/// presses in one gesture (right, then down), faster than a 180ms step; with
/// one slot the second overwrote the first. Three would let a player queue a
/// path the snake has not visibly committed to.
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
    /// 0..1 as a new layout fades in; obstacles are solid only at 1.
    obs_fade: f32,
    /// Swallowed food, as a segment index travelling down the body.
    gulps: Vec<f32>,
    /// Score popups, the ring where food was eaten, the burst on a crash.
    fx: Fx,
    banner_t: f32,
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
            obs_fade: 1.0,
            gulps: Vec::new(),
            fx: Fx::new(),
            banner_t: 9.0,
        }
    }

    /// The music for where the game is: the calm tune, then the fast one.
    fn track(&self) -> usize {
        if self.state != State::Title && self.sess.level >= FRENZY_LEVEL { 1 } else { 0 }
    }

    #[inline]
    fn snake_at(&self, i: usize) -> Cell {
        self.snake[(self.snake_head + i) % MAX_LEN]
    }

    fn move_interval(&self) -> f32 {
        let ms = SPEED_START - (self.sess.level - 1) as f32 * SPEED_STEP;
        if ms < SPEED_MIN { SPEED_MIN } else { ms }
    }

    /// Clear of the snake and the obstacles (from level 2), or food could
    /// land in a block and be unreachable.
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

    /// Put a bonus somewhere the snake must travel to: aim for a third of the
    /// board away, and settle for any free cell only if the board is nearly
    /// full.
    fn spawn_bonus(&mut self) {
        let head = self.snake_at(0);
        let far_enough = (COLS + ROWS) / 3;
        // Two passes: worth travelling to, then anywhere, so a long-snake
        // level still gets its bonus.
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
                (-1, 0), (0, 0), (1, 0), (0, -1), (0, 1),
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

    /// The next level's layout, arriving under a moving snake: cells on the
    /// snake, the food, the bonus or the head's next three steps are left
    /// out, and the rest fade in harmless (see OBS_FADE_SECS).
    fn raise_obstacles(&mut self) {
        self.build_obstacles();
        let head = self.snake_at(0);
        let (dc, dr) = match self.cur_dir { Dir::Up => (0, -1), Dir::Down => (0, 1), Dir::Left => (-1, 0), Dir::Right => (1, 0) };
        let ahead: Vec<Cell> = (1..=3).map(|k| Cell { c: head.c + dc * k, r: head.r + dr * k }).collect();
        let mut kept = 0;
        for i in 0..self.obstacle_count {
            let o = self.obstacles[i];
            let on_snake = (0..self.snake_len).any(|j| self.snake_at(j) == o);
            let on_item = o == self.food || (self.bonus_active() && o == self.bonus);
            if on_snake || on_item || ahead.contains(&o) { continue; }
            self.obstacles[kept] = o;
            kept += 1;
        }
        self.obstacle_count = kept;
        self.obs_fade = 0.0;
    }

    /// Queue a turn if the snake can take it, checked against the last
    /// direction queued, not the current one: otherwise up then left from
    /// travelling right is a U-turn into its own neck.
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
        self.obs_fade = 1.0;
        self.gulps.clear();
        // The middle row runs through the plus and the ring, and the ring
        // closes round the head: pick the nearest row with the snake and
        // its first six steps clear of obstacles.
        let row = [0, 2, -2, 4, -4, 6, -6].iter().map(|d| ROWS / 2 + d)
            .find(|&r| (COLS / 2 - 3..=COLS / 2 + 6)
                .all(|c| !self.obstacles[..self.obstacle_count].contains(&Cell { c, r })))
            .unwrap_or(ROWS / 2);
        for i in 0..self.snake_len {
            self.snake[i] = Cell { c: COLS / 2 - i as i32, r: row };
        }
        self.spawn_food();
        self.move_timer = 0.0;
    }

    fn start_game(&mut self) {
        self.sess.reset(LIVES_START);
        self.foods_eaten = 0;
        self.reset_snake();
        self.state = State::Play;
    }
}

struct Sounds {
    eat: blip::BlipSound,
    game_over: blip::BlipSound,
    bonus: blip::BlipSound,
    bonus_eat: blip::BlipSound,
    level: blip::BlipSound,
}

fn update_title(g: &mut Game) {
    if btn1_pressed() {
        web::spend_coin();
        g.start_game();
    }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    if key_pressed(BLIP_KEY_UP)    || key_pressed(BLIP_KEY_W) { g.queue_turn(Dir::Up); }
    if key_pressed(BLIP_KEY_DOWN)  || key_pressed(BLIP_KEY_S) { g.queue_turn(Dir::Down); }
    if key_pressed(BLIP_KEY_LEFT)  || key_pressed(BLIP_KEY_A) { g.queue_turn(Dir::Left); }
    if key_pressed(BLIP_KEY_RIGHT) || key_pressed(BLIP_KEY_D) { g.queue_turn(Dir::Right); }

    // Real time, not steps: the deadline must not stretch as the snake speeds up.
    if g.bonus_active() {
        g.bonus_ttl -= dt;
        if g.bonus_ttl < 0.0 { g.bonus_ttl = 0.0; }
    }

    g.obs_fade = (g.obs_fade + dt / OBS_FADE_SECS).min(1.0);
    g.fx.update(dt);
    g.banner_t += dt;

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
            if g.obs_fade < 1.0 { break; }
            if g.obstacles[i].c == h.c && g.obstacles[i].r == h.r { dead = true; break; }
        }
    }
    if dead {
        let why = if h.c < 0 || h.c >= COLS || h.r < 0 || h.r >= ROWS { "wall" }
            else if g.obs_fade >= 1.0 && g.obstacles[..g.obstacle_count].contains(&h) { "obstacle" } else { "self" };
        blip::bot::add(&format!("death_{why}"), 1.0);
        blip::bot::add(&format!("death_len{}", g.snake_len / 10 * 10), 1.0);
        let (x, y) = cell_centre(g.snake_at(0));
        g.fx.burst(x, y, 16, 140.0, BLIP_RED);
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

    // Before the level check so it scores at the level it was offered on.
    if g.bonus_active() && h == g.bonus {
        play_sfx(&sfx.bonus_eat);
        blip::bot::add("bonus_eaten", 1.0);
        g.sess.add_score(BONUS_VALUE * g.sess.level);
        let (x, y) = cell_centre(h);
        g.fx.burst(x, y, 12, 110.0, GOLD);
        g.fx.popup(x, y - 10.0, &format!("+{}", BONUS_VALUE * g.sess.level), GOLD);
        g.bonus_ttl = 0.0;
    }

    let ate = h.c == g.food.c && h.r == g.food.r;
    if ate {
        play_sfx(&sfx.eat);
        g.gulps.push(0.0);
        g.sess.add_score(10 * g.sess.level);
        let (x, y) = cell_centre(h);
        g.fx.ring(x, y, CELL as f32 * 1.3, 0.3, BlipColor { r: 1.0, g: 0.85, b: 0.4, a: 1.0 });
        g.fx.popup(x, y - 10.0, &format!("+{}", 10 * g.sess.level), BLIP_WHITE);
        g.foods_eaten += 1;
        // One bonus per level, mid-level, away from the level change.
        if g.foods_eaten % BONUS_EVERY == 0 && !g.bonus_active() {
            g.spawn_bonus();
            blip::bot::add("bonus_offered", 1.0);
            play_sfx(&sfx.bonus);
        }
        if g.foods_eaten >= FOODS_PER_LVL {
            g.sess.next_level();
            g.raise_obstacles();
            g.banner_t = 0.0;
            blip::bot::set("level", g.sess.level as f64);
            blip::bot::set(&format!("t_level{}", g.sess.level), blip::bot::clock() as f64);
            play_sfx(&sfx.level);
            g.foods_eaten = 0;
        }
        g.spawn_food();
    }
    for k in g.gulps.iter_mut() { *k += 1.0; }
    let len = g.snake_len as f32;
    g.gulps.retain(|&k| k < len);

    g.snake_head = (g.snake_head + MAX_LEN - 1) % MAX_LEN;
    g.snake[g.snake_head] = h;
    if ate && g.snake_len < MAX_LEN { g.snake_len += 1; }
}

fn update_dead(g: &mut Game, dt: f32) {
    g.fx.update(dt);
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

/// The snake drawn as one body: thick strokes between segment centres,
/// brighter toward the head, eyes on the side it is heading.
fn draw_snake(blip: &Blip, g: &Game) {
    // Glide between cells by step progress; frozen fully arrived once dead
    // (move_timer stalls mid-step on the fatal tick).
    let f = if g.state == State::Play {
        (g.move_timer / g.move_interval()).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let flash = g.state == State::Dead && (g.dead_timer.remaining() / 0.12) as i32 % 2 == 0;
    let half = CELL as f32 / 2.0;
    let seg_px = |i: usize| -> (f32, f32) {
        let cur = g.snake_at(i);
        let prev = if i + 1 < g.snake_len {
            g.snake_at(i + 1)
        } else {
            // the tail: extrapolate the cell one step back along the body
            let ahead = g.snake_at(i - 1);
            Cell { c: 2 * cur.c - ahead.c, r: 2 * cur.r - ahead.r }
        };
        let x = lerp(prev.c as f32, cur.c as f32, f) * CELL as f32 + half;
        let y = HUD_H as f32 + lerp(prev.r as f32, cur.r as f32, f) * CELL as f32 + half;
        (x, y)
    };
    let n = g.snake_len;
    let shade = |i: usize| -> BlipColor {
        if flash { return BLIP_RED; }
        let k = i as f32 / n.max(2) as f32;
        BlipColor { r: lerp(0.36, 0.16, k), g: lerp(0.86, 0.50, k), b: lerp(0.36, 0.20, k), a: 1.0 }
    };
    let thick = CELL as f32 - 6.0;
    for i in (1..n).rev() {
        let (x0, y0) = seg_px(i);
        let (x1, y1) = seg_px(i - 1);
        let c = shade(i);
        blip.draw_line_ex(x0, y0, x1, y1, thick, c);
        blip.fill_rect(x0 - thick / 2.0, y0 - thick / 2.0, thick, thick, c);
    }
    // A swallowed food, travelling down toward the tail.
    for &k in &g.gulps {
        let i = (k as usize).min(n - 1);
        let (x, y) = seg_px(i);
        let r = thick / 2.0 + 3.0;
        blip.fill_rect(x - r, y - r, r * 2.0, r * 2.0, shade(i));
    }
    let (hx, hy) = seg_px(0);
    let hs = CELL as f32 - 4.0;
    let head = if flash { BLIP_RED } else { BlipColor { r: 0.45, g: 0.95, b: 0.45, a: 1.0 } };
    blip.fill_rect(hx - hs / 2.0, hy - hs / 2.0, hs, hs, head);
    let (fx, fy) = match g.cur_dir { Dir::Up => (0.0, -1.0), Dir::Down => (0.0, 1.0), Dir::Left => (-1.0, 0.0), Dir::Right => (1.0, 0.0) };
    // A tongue flicks out now and then, in the direction of travel.
    if g.state == State::Play && blip::macroquad::time::get_time().rem_euclid(1.6) < 0.18 {
        let tip = hs / 2.0 + 6.0;
        blip.draw_line_ex(hx + fx * hs / 2.0, hy + fy * hs / 2.0, hx + fx * tip, hy + fy * tip, 2.0, BLIP_RED);
    }
    for side in [-1.0f32, 1.0] {
        let (ex, ey) = (hx + fx * 4.0 - fy * side * 5.0, hy + fy * 4.0 + fx * side * 5.0);
        blip.fill_circle(ex, ey, 3.2, BLIP_WHITE);
        blip.fill_circle(ex + fx * 1.2, ey + fy * 1.2, 1.6, BLIP_BLACK);
    }
}

fn draw_play(blip: &Blip, g: &Game, food: &Texture2D) {
    draw_board(blip);
    // A new layout blinks in, see-through, until it is solid.
    let solid = g.obs_fade >= 1.0;
    let a = if solid { 1.0 } else if (g.obs_fade * 8.0) as i32 % 2 == 0 { 0.55 } else { 0.25 };
    for i in 0..g.obstacle_count {
        let obs = g.obstacles[i];
        let (x, y) = ((obs.c * CELL) as f32, (HUD_H + obs.r * CELL) as f32);
        blip.fill_rect(x + 1.0, y + 1.0, CELL as f32 - 2.0, CELL as f32 - 2.0, BlipColor { a, ..BLIP_GRAY });
        blip.fill_rect(x + 1.0, y + 1.0, CELL as f32 - 2.0, 3.0, BlipColor { r: 0.75, g: 0.75, b: 0.75, a });
    }
    // Food breathes a little so the eye finds it.
    let breathe = 1.0 + 0.08 * (blip::macroquad::time::get_time() as f32 * 5.0).sin();
    let fs = CELL as f32 * breathe;
    let (fcx, fcy) = ((g.food.c * CELL) as f32 + CELL as f32 / 2.0, (HUD_H + g.food.r * CELL) as f32 + CELL as f32 / 2.0);
    blip.draw_texture(food, fcx - fs / 2.0, fcy - fs / 2.0, fs, fs);
    draw_bonus(blip, g);
    draw_snake(blip, g);
    g.fx.draw(blip);
    if g.banner_t < BANNER_SECS && g.state == State::Play {
        let a = (1.0 - g.banner_t / BANNER_SECS).min(1.0) * 1.5;
        // Row 5 is clear in every obstacle layout.
        blip.draw_centered(&format!("LEVEL {}", g.sess.level), (HUD_H + 5 * CELL + 2) as f32, 4.0,
            BlipColor { a: a.min(1.0), ..BLIP_YELLOW });
    }
    blip.draw_hud(g.sess.score, g.sess.lives);
}

/// The timed bonus: shrinks with its clock (drawn in-world, where the
/// go-for-it decision is made) and blinks for the last two seconds.
fn draw_bonus(blip: &Blip, g: &Game) {
    if !g.bonus_active() { return; }
    // Off for only a third of each cycle so it stays findable.
    let expiring = g.bonus_ttl <= BONUS_WARN;
    if expiring && (g.bonus_ttl * 6.0) as i32 % 3 == 0 { return; }

    let cx = (g.bonus.c * CELL) as f32 + CELL as f32 / 2.0;
    let cy = (HUD_H + g.bonus.r * CELL) as f32 + CELL as f32 / 2.0;
    let life = g.bonus_ttl / BONUS_TTL;
    let r = CELL as f32 * (0.26 + 0.20 * life);
    let gold = GOLD;
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
    draw_title_snake(blip, blip::macroquad::time::get_time() as f32);
}

/// A snake winding across the title screen, head first, wrapping round.
fn draw_title_snake(blip: &Blip, t: f32) {
    const SEGS: usize = 16;
    let span = WIN_W as f32 + 160.0;
    let at = |d: f32| {
        let x = (t * 90.0 - d).rem_euclid(span) - 80.0;
        (x, WIN_H as f32 * 0.83 + (x * 0.035).sin() * 18.0)
    };
    for i in (0..SEGS).rev() {
        let (x, y) = at(i as f32 * 11.0);
        let k = i as f32 / SEGS as f32;
        let c = BlipColor { r: lerp(0.36, 0.16, k), g: lerp(0.86, 0.50, k), b: lerp(0.36, 0.20, k), a: 1.0 };
        blip.fill_circle(x, y, 9.0 - k * 3.0, c);
    }
    let (hx, hy) = at(0.0);
    blip.fill_circle(hx + 3.0, hy - 4.0, 2.6, BLIP_WHITE);
    blip.fill_circle(hx + 4.0, hy - 4.0, 1.3, BLIP_BLACK);
}

fn draw_over(blip: &Blip, g: &Game, hi: &web::HighScore) {
    blip.clear(BLIP_BLACK);
    blip.draw_game_over(g.sess.score, hi, BLIP_RED, BLIP_GREEN, BLIP_YELLOW, !g.dead_timer.active());
    blip.draw_centered(&format!("LEVEL {}   LENGTH {}", g.sess.level, g.snake_len),
        (WIN_H / 2 + 54) as f32, 2.0, BLIP_GRAY);
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("SERPENT", WIN_W, WIN_H)
}

const FOOD_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/food.png"));
const EAT_WAV: &[u8]  = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/eat.wav"));
const GAME_OVER_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/game_over.wav"));
const BONUS_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/bonus.wav"));
const BONUS_EAT_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/bonus_eat.wav"));
const LEVEL_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/level.wav"));


#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let food = load_png(FOOD_PNG);

    let sfx = Sounds {
        eat:       blip::audio::load_sound(EAT_WAV).await,
        game_over: blip::audio::load_sound(GAME_OVER_WAV).await,
        bonus:     blip::audio::load_sound(BONUS_WAV).await,
        bonus_eat: blip::audio::load_sound(BONUS_EAT_WAV).await,
        level:     blip::audio::load_sound(LEVEL_WAV).await,
    };
    let mut music = Jukebox::new(&[blip_assets::serpent::slither_wav, blip_assets::serpent::frenzy_wav]);
    music.start(0).await;

    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;

        if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 {
                g.start_game();
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        if blip::bot::active() { bot::drive(&g); }
        let was_playing = g.state == State::Play;
        match g.state {
            State::Title => update_title(&mut g),
            State::Play  => update_play(&mut g, dt, &sfx),
            State::Dead  => update_dead(&mut g, dt),
            State::Over  => update_over(&mut g, dt),
        }
        if was_playing && g.state != State::Play { web::haptic(); }

        music.play(g.track());
        if g.state != State::Play { music.warm_up().await; }

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title => draw_title(&blip, &web::high_score()),
            State::Over  => draw_over(&blip, &g, &web::high_score()),
            State::Play | State::Dead => draw_play(&blip, &g, &food),
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

    /// One move step as update_play() takes it: the turn comes off the queue
    /// and becomes the direction.
    fn step(g: &mut Game) -> Dir {
        g.cur_dir = g.take_turn();
        g.cur_dir
    }

    #[test]
    fn a_corner_taken_faster_than_one_step_keeps_both_turns() {
        // Down then left inside one step must both happen.
        let mut g = running();
        g.queue_turn(Dir::Down);
        g.queue_turn(Dir::Left);

        assert_eq!(step(&mut g), Dir::Down, "the first turn of the gesture was dropped");
        assert_eq!(step(&mut g), Dir::Left, "the second turn of the gesture was dropped");
        assert_eq!(step(&mut g), Dir::Left, "with nothing queued the snake carries on");
    }

    #[test]
    fn a_queued_turn_cannot_double_back_into_the_neck() {
        // Travelling right, up then down: the second is a reversal of the
        // first and must be refused.
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
    fn a_new_levels_obstacles_never_land_on_the_snake_or_just_ahead_of_it() {
        // They arrive under a moving snake; one on its body or in its next
        // steps is a death the player never had a chance to see coming.
        for level in 2..=6 {
            let mut g = running();
            g.sess.level = level;
            g.raise_obstacles();
            let head = g.snake_at(0);
            for o in &g.obstacles[..g.obstacle_count] {
                assert!((0..g.snake_len).all(|i| g.snake_at(i) != *o), "level {level}: an obstacle on the snake");
                assert!(!(o.r == head.r && o.c > head.c && o.c <= head.c + 3), "level {level}: an obstacle just ahead");
            }
            assert!(g.obs_fade < 1.0, "the new layout is solid at once");
        }
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
