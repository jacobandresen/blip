//! Serpent (Snake), Rust port of `games/serpent/main.c` on macroquad.

use blip::input::{coin_start, play_again, Stick};
use blip::macroquad::texture::Texture2D;
use blip::{Shake,
    GAME_OVER_MIN_WAIT,
    load_png,
    lerp, play_sfx, Jukebox, web, window_conf, Blip, BlipColor, Fx, LifeResult, Session,
    Timer, BLIP_BLACK, BLIP_GRAY, BLIP_CREAM, BLIP_GOLD, BLIP_GREEN, BLIP_INK, BLIP_LEAF, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
};

// The effects and music are made by build.rs; the tests check them here.
#[cfg(test)]
mod tunes;
mod bot;
mod levels;
mod draw;
use draw::*;

use levels::*;

// ---- layout -----------------------------------------------------------
mod field;
use field::{CELL, CHEST, COLS, FOREST, ROWS};
const HUD_H: i32 = 28;
/// The board's top left corner.
const BOARD_X: i32 = FOREST;
const BOARD_Y: i32 = HUD_H + FOREST;
const WIN_W: i32 = COLS * CELL + 2 * FOREST;
const WIN_H: i32 = ROWS * CELL + 2 * FOREST + HUD_H;
const MAX_LEN: usize = (COLS * ROWS) as usize;

// ---- tuning -----------------------------------------------------------
const LIVES_START: i32 = 3;
// A new level's obstacles arrive mid-run; this long to see them before they are solid.
const BANNER_SECS: f32 = 1.4;

// ---- shears ----------------------------------------------------------------
// After `SHEARS_FROM`, every second level's first food spawns shears; collecting them cuts `SHEARS_CUT` from the tail.
const SHEARS_BLUE: BlipColor = BlipColor { r: 0.45, g: 0.85, b: 1.0, a: 1.0 };

// ---- streaks ---------------------------------------------------------------
// Food within 22 steps adds a multiplier up to `STREAK_MAX`; 22 is one board width and a turn.
const STREAK_STEPS: i32 = 22;
const STREAK_MAX: i32 = 5;

const GOLD: BlipColor = BlipColor { r: 1.0, g: 0.84, b: 0.20, a: 1.0 };

/// The shadow things cast on the lawn (opaque, so the snake's overlapping strokes do not darken where they meet).
const LAWN_SHADOW: BlipColor = BlipColor { r: 0.38, g: 0.60, b: 0.22, a: 1.0 };

/// The head, a lighter emerald than the body behind it.
const HEAD_GREEN: BlipColor = BlipColor { r: 0.26, g: 0.78, b: 0.46, a: 1.0 };

/// The snake's scales, `k` of the way from head (0) to tail (1).
fn scale_green(k: f32) -> BlipColor {
    // An emerald darker than the sunny lawn, so it stands out on it.
    blip::blend(BlipColor { r: 0.16, g: 0.66, b: 0.38, a: 1.0 }, BlipColor { r: 0.06, g: 0.38, b: 0.24, a: 1.0 }, k)
}

/// The pixel centre of a board cell.
fn cell_centre(c: Cell) -> (f32, f32) {
    ((BOARD_X + c.c * CELL + CELL / 2) as f32, (BOARD_Y + c.r * CELL + CELL / 2) as f32)
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

/// Two turns fit a corner gesture; more would queue movement the snake has not visibly committed to.
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
    /// The bonus is a golden mouse: seconds since its last hop, and the way it last ran.
    mouse_t: f32,
    mouse_dir: (i32, i32),
    /// The tail-cutting pickup (see SHEARS_FROM), timed like the bonus.
    shears: Cell,
    shears_ttl: f32,
    sess: Session,
    foods_eaten: i32,
    /// Quick meals in a row (see STREAK_STEPS): one more multiple each, up to STREAK_MAX.
    streak: blip::Streak,
    steps_since_food: i32,
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
    /// Segments popped so far after a crash, head first.
    popped: usize,
    /// The board shakes at a crash (see CRASH_SHAKE).
    shake: Shake,
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
            mouse_t: 0.0,
            mouse_dir: (1, 0),
            shears: Cell { c: 0, r: 0 },
            shears_ttl: 0.0,
            foods_eaten: 0,
            streak: blip::Streak::new(1, STREAK_MAX),
            steps_since_food: 0,
            move_timer: 0.0,
            dead_timer: Timer::default(),
            state: State::Title,
            obstacles: [Cell { c: 0, r: 0 }; 16],
            obstacle_count: 0,
            obs_fade: 1.0,
            gulps: Vec::new(),
            popped: 0,
            shake: CRASH_SHAKE,
            fx: Fx::new(),
            banner_t: 9.0,
        }
    }

    /// The tune for where the game is: the title's, then one for each stretch of levels (TUNE_FROM).
    fn track(&self) -> usize {
        if self.state == State::Title { return 0; }
        TUNE_FROM.iter().filter(|&&from| self.sess.level >= from).count()
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

    /// Every cell the snake and the obstacles leave free, row by row.
    fn free_cells(&self) -> Vec<Cell> {
        (0..ROWS).flat_map(|r| (0..COLS).map(move |c| Cell { c, r })).filter(|&f| self.cell_is_free(f)).collect()
    }

    fn spawn_food(&mut self) -> bool {
        let item_cells = self.free_cells();
        let free: Vec<Cell> = item_cells.iter().copied()
            .filter(|&f| !(self.bonus_active() && f == self.bonus || self.shears_active() && f == self.shears))
            .collect();
        let Some(food) = blip::rand_item(if free.is_empty() { &item_cells } else { &free }) else { return false };
        self.food = food;
        if free.is_empty() {
            if self.bonus_active() && self.food == self.bonus { self.bonus_ttl = 0.0; }
            if self.shears_active() && self.food == self.shears { self.shears_ttl = 0.0; }
        }
        true
    }

    fn bonus_active(&self) -> bool { self.bonus_ttl > 0.0 }
    fn shears_active(&self) -> bool { self.shears_ttl > 0.0 }

    fn spawn_shears(&mut self) {
        let mut free = self.free_cells();
        free.retain(|&cell| cell != self.food && !(self.bonus_active() && cell == self.bonus));
        let Some(shears) = blip::rand_item(&free) else { return };
        self.shears = shears;
        self.shears_ttl = SHEARS_TTL;
    }

    /// Put a bonus somewhere the snake must travel to: aim for a third of the
    /// board away, and settle for any free cell only if the board is nearly
    /// full.
    fn spawn_bonus(&mut self) {
        self.mouse_t = 0.0;
        let head = self.snake_at(0);
        let far_enough = (COLS + ROWS) / 3;
        let mut free = self.free_cells();
        free.retain(|&cell| cell != self.food && !(self.shears_active() && cell == self.shears));
        let distant: Vec<Cell> = free.iter().copied()
            .filter(|cell| (cell.c - head.c).abs() + (cell.r - head.r).abs() >= far_enough)
            .collect();
        let Some(bonus) = blip::rand_item(if distant.is_empty() { &free } else { &distant }) else { return };
        self.bonus = bonus;
        self.bonus_ttl = BONUS_TTL;
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

    /// The next layout leaves the snake, its items and the next three steps clear.
    fn raise_obstacles(&mut self) {
        self.build_obstacles();
        let head = self.snake_at(0);
        let (dc, dr) = match self.cur_dir { Dir::Up => (0, -1), Dir::Down => (0, 1), Dir::Left => (-1, 0), Dir::Right => (1, 0) };
        let ahead: Vec<Cell> = (1..=3).map(|k| Cell { c: head.c + dc * k, r: head.r + dr * k }).collect();
        let mut kept = 0;
        for i in 0..self.obstacle_count {
            let o = self.obstacles[i];
            let on_snake = (0..self.snake_len).any(|j| self.snake_at(j) == o);
            let on_item = o == self.food
                || (self.bonus_active() && o == self.bonus)
                || (self.shears_active() && o == self.shears);
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
        self.shears_ttl = 0.0;
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
        self.streak.miss();
        self.steps_since_food = 0;
    }

    fn start_game(&mut self) {
        self.sess.reset(LIVES_START);
        self.foods_eaten = 0;
        self.reset_snake();
        self.state = State::Play;
    }
}

blip::sounds!(Sounds { eat, game_over, bonus, bonus_eat, level, squeak });

fn update_title(g: &mut Game) {
    if coin_start() { g.start_game(); }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    if Stick::Up.pressed() { g.queue_turn(Dir::Up); }
    if Stick::Down.pressed() { g.queue_turn(Dir::Down); }
    if Stick::Left.pressed() { g.queue_turn(Dir::Left); }
    if Stick::Right.pressed() { g.queue_turn(Dir::Right); }

    // Real time, not steps: the deadline must not stretch as the snake speeds up.
    if g.bonus_active() {
        g.bonus_ttl -= dt;
        if g.bonus_ttl < 0.0 { g.bonus_ttl = 0.0; }
        if scurry(g, dt) { blip::audio::play_sfx_volume(&sfx.squeak, 0.35); }
    }
    g.shears_ttl = (g.shears_ttl - dt).max(0.0);

    g.obs_fade = (g.obs_fade + dt / OBS_FADE_SECS).min(1.0);
    g.fx.update(dt);
    g.banner_t += dt;

    g.move_timer += dt * 1000.0;
    if g.move_timer < g.move_interval() { return; }
    g.move_timer -= g.move_interval();
    step_snake(g, sfx);
}

// The golden mouse runs from a head within MOUSE_SEES cells, a cell every MOUSE_HOP seconds (the snake steps every
// 70-180 ms, so it is always caught in the end): the last few cells become a chase with turns in it. Serpent's busy
// measure was 0.71, long straight runs with the controls untouched.
const MOUSE_SEES: i32 = 5;
const MOUSE_HOP: f32 = 0.55;

/// The mouse hops to the free neighbouring cell farthest from the head, when that is farther than where it is; true
/// when it hopped.
fn scurry(g: &mut Game, dt: f32) -> bool {
    g.mouse_t += dt;
    let head = g.snake_at(0);
    let dist = |c: Cell| (c.c - head.c).abs() + (c.r - head.r).abs();
    if g.mouse_t < MOUSE_HOP || dist(g.bonus) > MOUSE_SEES { return false; }
    g.mouse_t = 0.0;
    let mut best = (g.bonus, dist(g.bonus), (0, 0));
    for (dc, dr) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let to = Cell { c: g.bonus.c + dc, r: g.bonus.r + dr };
        let inside = (0..COLS).contains(&to.c) && (0..ROWS).contains(&to.r);
        if !inside || !g.cell_is_free(to) || to == g.food || (g.shears_active() && to == g.shears) { continue; }
        if dist(to) > best.1 { best = (to, dist(to), (dc, dr)); }
    }
    if best.0 == g.bonus { return false; }
    g.bonus = best.0;
    g.mouse_dir = best.2;
    true
}

/// One step: the next turn, then the new head crashes, collects and eats.
fn step_snake(g: &mut Game, sfx: &Sounds) {
    g.cur_dir = g.take_turn();
    let mut h = g.snake_at(0);
    match g.cur_dir {
        Dir::Up => h.r -= 1,
        Dir::Down => h.r += 1,
        Dir::Left => h.c -= 1,
        Dir::Right => h.c += 1,
    }
    if let Some(why) = check_crash(g, h) {
        lose_life(g, sfx, why);
        return;
    }
    // Before the level check so the bonus scores at the level it was offered on.
    collect_pickups(g, h, sfx);
    let ate = h == g.food;
    let offers = if ate { eat_food(g, h, sfx) } else { (false, false) };
    g.steps_since_food += 1;
    // The run breaks the step the window closes (not at the next meal), so the HUD's multiple is never one already lost.
    if g.steps_since_food > STREAK_STEPS { g.streak.miss(); }
    for k in g.gulps.iter_mut() { *k += 1.0; }
    let len = g.snake_len as f32;
    g.gulps.retain(|&k| k < len);

    // The body follows the head; eating leaves the tail where it was.
    g.snake_head = (g.snake_head + MAX_LEN - 1) % MAX_LEN;
    g.snake[g.snake_head] = h;
    if ate && g.snake_len < MAX_LEN { g.snake_len += 1; }
    if ate { spawn_after_meal(g, offers, sfx); }
}

/// What the head at `h` runs into, if anything: a wall, the body, or a
/// settled obstacle.
fn check_crash(g: &Game, h: Cell) -> Option<&'static str> {
    if h.c < 0 || h.c >= COLS || h.r < 0 || h.r >= ROWS { return Some("wall"); }
    if (0..g.snake_len.saturating_sub(1)).any(|i| g.snake_at(i) == h) { return Some("self"); }
    if g.obs_fade >= 1.0 && g.obstacles[..g.obstacle_count].contains(&h) { return Some("obstacle"); }
    None
}

fn lose_life(g: &mut Game, sfx: &Sounds, why: &str) {
    blip::bot::add(&format!("death_{why}"), 1.0);
    blip::bot::add(&format!("death_len{}", g.snake_len / 10 * 10), 1.0);
    let (x, y) = cell_centre(g.snake_at(0));
    g.fx.burst(x, y, 16, 140.0, BLIP_RED);
    play_sfx(&sfx.game_over);
    // A lost life breaks the run of quick meals, as it breaks every streak in the arcade.
    g.streak.miss();
    g.popped = 0;
    g.shake.start();
    g.state = match g.sess.lose_life_after(&mut g.dead_timer, DEAD_SECS) {
        LifeResult::StillAlive => State::Dead,
        LifeResult::GameOver => {
            // A new best gets the level-up chime after the crash.
            if g.sess.beats_best() { play_sfx(&sfx.level); }
            State::Over
        }
    };
}

/// The bonus scores; the shears cut the tail, a puff per segment.
fn collect_pickups(g: &mut Game, h: Cell, sfx: &Sounds) {
    if g.bonus_active() && h == g.bonus {
        play_sfx(&sfx.bonus_eat);
        blip::bot::add("bonus_eaten", 1.0);
        g.sess.add_score(BONUS_VALUE * g.sess.level);
        let (x, y) = cell_centre(h);
        g.fx.burst(x, y, 12, 110.0, GOLD);
        g.fx.popup(x, y - 10.0, &format!("+{}", BONUS_VALUE * g.sess.level), GOLD);
        g.bonus_ttl = 0.0;
    }
    if g.shears_active() && h == g.shears {
        play_sfx(&sfx.bonus_eat);
        blip::bot::add("shears", 1.0);
        g.shears_ttl = 0.0;
        let cut = SHEARS_CUT.min(g.snake_len.saturating_sub(3));
        for k in 0..cut {
            let (x, y) = cell_centre(g.snake_at(g.snake_len - 1 - k));
            g.fx.burst(x, y, 4, 70.0, SHEARS_BLUE);
        }
        g.snake_len -= cut;
        let (x, y) = cell_centre(h);
        g.fx.popup(x, y - 10.0, "SNIP", SHEARS_BLUE);
    }
}

/// Points, more for a streak of quick meals; returns whether the meal earns
/// the shears and the bonus.
fn eat_food(g: &mut Game, h: Cell, sfx: &Sounds) -> (bool, bool) {
    play_sfx(&sfx.eat);
    blip::bot::add("food", 1.0);
    g.gulps.push(0.0);
    let times = g.streak.hit();
    g.steps_since_food = 0;
    let points = 10 * g.sess.level * times;
    g.sess.add_score(points);
    let (x, y) = cell_centre(h);
    g.fx.ring(x, y, CELL as f32 * 1.3, 0.3, BlipColor { r: 1.0, g: 0.85, b: 0.4, a: 1.0 });
    if times > 1 {
        g.fx.popup(x, y - 10.0, &format!("+{points} X{times}"), GOLD);
    } else {
        g.fx.popup(x, y - 10.0, &format!("+{points}"), BLIP_WHITE);
    }
    g.foods_eaten += 1;
    // One shears per level, mid-level, away from the level change.
    let shears = g.foods_eaten == 1 && g.sess.level % 2 == 0 && g.snake_len >= SHEARS_FROM && !g.shears_active();
    let bonus = g.foods_eaten % BONUS_EVERY == 0 && !g.bonus_active();
    if g.foods_eaten >= FOODS_PER_LVL { next_level(g, sfx); }
    (shears, bonus)
}

fn next_level(g: &mut Game, sfx: &Sounds) {
    g.sess.next_level();
    g.raise_obstacles();
    g.banner_t = 0.0;
    play_sfx(&sfx.level);
    g.foods_eaten = 0;
}

/// What the meal earned, then the next food; a full board ends the game.
fn spawn_after_meal(g: &mut Game, (shears, bonus): (bool, bool), sfx: &Sounds) {
    if shears { g.spawn_shears(); }
    if bonus {
        g.spawn_bonus();
        blip::bot::add("bonus_offered", 1.0);
        play_sfx(&sfx.bonus);
    }
    if !g.spawn_food() {
        g.state = State::Over;
        g.dead_timer.start(GAME_OVER_MIN_WAIT);
        g.sess.report();
    }
}

/// After a crash the snake pops apart from head to tail, a puff in its own colour per segment, done before the respawn.
fn update_dead(g: &mut Game, dt: f32) {
    g.fx.update(dt);
    let elapsed = DEAD_SECS - g.dead_timer.remaining();
    let every = (1.1 / g.snake_len.max(1) as f32).min(0.05);
    let due = (((elapsed - POP_AFTER) / every).max(0.0) as usize).min(g.snake_len);
    while g.popped < due {
        let k = g.popped as f32 / g.snake_len.max(2) as f32;
        let (x, y) = cell_centre(g.snake_at(g.popped));
        g.fx.burst(x, y, 4, 80.0, scale_green(k));
        g.popped += 1;
    }
    if g.dead_timer.tick(dt) {
        g.reset_snake();
        g.state = State::Play;
    }
}

/// The wait after a crash, and the red flash before the snake starts to pop.
const DEAD_SECS: f32 = 1.5;
/// A crash shakes the board 0.35 s, up to 5 px.
const CRASH_SHAKE: Shake = Shake::new(0.35, 5.0);
const POP_AFTER: f32 = 0.25;

fn update_over(g: &mut Game, dt: f32) {
    if play_again(&mut g.dead_timer, dt) { g.start_game(); }
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("SERPENT", WIN_W, WIN_H)
}

#[blip::macroquad::main(conf)]
async fn main() {
    blip::bot::playthroughs(bot::PLAYTHROUGHS);
    let mut blip = Blip::new(WIN_W, WIN_H);
    // A bright lawn: the bloom tuned for lines on black would wash it out.
    blip.set_bloom(0.2);
    // and the interlace's dark rows, tuned for lines on black, would halve it and twitter (as Bubbler and Brawler).
    blip.set_interlace(0.6);
    blip::bot::arena(BOARD_X as f32, BOARD_Y as f32, (COLS * CELL) as f32, (ROWS * CELL) as f32);
    let mut g = Game::new();

    let food = load_png(blip::png!("food"));
    let field = load_png(blip::png!("field"));
    let wood = load_png(blip::png!("wood"));

    let sfx = Sounds::load().await;
    let mut music = Jukebox::new("serpent", &["MEADOW", "STROLL", "BURROW", "PICNIC", "SCAMPER", "FINALE"]);

    if blip.screenshot_mode {
        g.start_game();
    }

    loop {
        let dt = blip.delta_time;

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

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title => draw_title(&blip, &field, &wood),
            State::Over  => draw_over(&blip, &g, web::high_score(), &field),
            State::Play | State::Dead => blip.shaken(&g.shake, || draw_play(&blip, &g, &food, &field)),
        }
        g.shake.tick(dt);

        blip.next_frame().await;
    }
}

#[cfg(test)]
mod tests;
