//! Serpent (Snake), Rust port of `games/serpent/main.c` on macroquad.

use blip::input::{coin_start, play_again, Stick};
use blip::macroquad::texture::Texture2D;
use blip::{
    GAME_OVER_MIN_WAIT,
    load_png,
    lerp, play_sfx, Jukebox, rand_int, web, window_conf, Blip, BlipColor, Fx, LifeResult, Session,
    Timer, BLIP_BLACK, BLIP_GRAY, BLIP_GREEN, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
};

// The effects and music are made by build.rs; the tests check them here.
#[cfg(test)]
mod tunes;
mod bot;
mod levels;

use levels::*;

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

// ---- bonus fruit --------------------------------------------------------
// Worth five foods, somewhere else, and leaving: whether to go for it is the
// game's first real choice.
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

/// The snake's scales, `k` of the way from head (0) to tail (1).
fn scale_green(k: f32) -> BlipColor {
    blip::blend(BlipColor { r: 0.36, g: 0.86, b: 0.36, a: 1.0 }, BlipColor { r: 0.16, g: 0.50, b: 0.20, a: 1.0 }, k)
}

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
    /// Seconds of screen shake left (see CRASH_SHAKE).
    shake: f32,
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
            shake: 0.0,
            fx: Fx::new(),
            banner_t: 9.0,
        }
    }

    /// The music for where the game is: the calm tune, then the fast one.
    fn track(&self) -> usize {
        if self.state == State::Title { 0 }
        else if self.sess.level >= FRENZY_LEVEL { 2 }
        else if self.sess.level >= PROWL_LEVEL { 1 }
        else { 0 }
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

    fn spawn_food(&mut self) -> bool {
        let mut free = Vec::new();
        let mut item_cells = Vec::new();
        for r in 0..ROWS {
            for c in 0..COLS {
                let f = Cell { c, r };
                if !self.cell_is_free(f) { continue; }
                item_cells.push(f);
                if !(self.bonus_active() && f == self.bonus)
                    && !(self.shears_active() && f == self.shears) {
                    free.push(f);
                }
            }
        }
        let choices = if free.is_empty() { &item_cells } else { &free };
        if choices.is_empty() { return false; }
        self.food = choices[rand_int(0, choices.len() as i32 - 1) as usize];
        if free.is_empty() {
            if self.bonus_active() && self.food == self.bonus { self.bonus_ttl = 0.0; }
            if self.shears_active() && self.food == self.shears { self.shears_ttl = 0.0; }
        }
        true
    }

    fn bonus_active(&self) -> bool { self.bonus_ttl > 0.0 }
    fn shears_active(&self) -> bool { self.shears_ttl > 0.0 }

    fn spawn_shears(&mut self) {
        let mut free = Vec::new();
        for r in 0..ROWS {
            for c in 0..COLS {
                let cell = Cell { c, r };
                if self.cell_is_free(cell) && cell != self.food
                    && !(self.bonus_active() && cell == self.bonus)
                {
                    free.push(cell);
                }
            }
        }
        if free.is_empty() { return; }
        self.shears = free[rand_int(0, free.len() as i32 - 1) as usize];
        self.shears_ttl = SHEARS_TTL;
    }

    /// Put a bonus somewhere the snake must travel to: aim for a third of the
    /// board away, and settle for any free cell only if the board is nearly
    /// full.
    fn spawn_bonus(&mut self) {
        self.mouse_t = 0.0;
        let head = self.snake_at(0);
        let far_enough = (COLS + ROWS) / 3;
        let mut free = Vec::new();
        let mut distant = Vec::new();
        for r in 0..ROWS {
            for c in 0..COLS {
                let cell = Cell { c, r };
                if !self.cell_is_free(cell) || cell == self.food
                    || (self.shears_active() && cell == self.shears)
                {
                    continue;
                }
                free.push(cell);
                if (cell.c - head.c).abs() + (cell.r - head.r).abs() >= far_enough {
                    distant.push(cell);
                }
            }
        }
        let choices = if distant.is_empty() { &free } else { &distant };
        if choices.is_empty() { return; }
        self.bonus = choices[rand_int(0, choices.len() as i32 - 1) as usize];
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
    g.shake = CRASH_SHAKE;
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
/// A crash shakes the board this long, up to 5 px.
const CRASH_SHAKE: f32 = 0.35;
const POP_AFTER: f32 = 0.25;

fn update_over(g: &mut Game, dt: f32) {
    if play_again(&mut g.dead_timer, dt) { g.start_game(); }
}

/// A dark lawn of two-tone squares, so the eye can count cells without grid lines.
fn draw_board(blip: &Blip) {
    let tiles = [BlipColor { r: 0.035, g: 0.075, b: 0.045, a: 1.0 }, BlipColor { r: 0.05, g: 0.10, b: 0.06, a: 1.0 }];
    for r in 0..ROWS {
        for c in 0..COLS {
            blip.fill_rect((c * CELL) as f32, (HUD_H + r * CELL) as f32, CELL as f32, CELL as f32, tiles[((c + r) & 1) as usize]);
        }
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
        blip::clamp(g.move_timer / g.move_interval(), 0.0, 1.0)
    } else {
        1.0
    };
    let dead = g.state == State::Dead;
    let flash = dead && DEAD_SECS - g.dead_timer.remaining() < POP_AFTER && (g.dead_timer.remaining() / 0.08) as i32 % 2 == 0;
    let first = if dead { g.popped.min(g.snake_len) } else { 0 };
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
        scale_green(k)
    };
    // Body radius tapers toward the tail; a swallowed food bulges it as it travels down.
    let base_r = CELL as f32 / 2.0 - 3.0;
    let radius = |i: usize| {
        let taper = if i + 4 >= n { 0.55 + 0.45 * (n - 1 - i) as f32 / 4.0 } else { 1.0 };
        let bulge = g.gulps.iter().map(|&k| (1.0 - (k - i as f32).abs()).max(0.0)).fold(0.0, f32::max);
        base_r * taper.min(1.0) + bulge * 3.5
    };
    let ink = BlipColor { r: 0.02, g: 0.12, b: 0.04, a: 1.0 };
    let pts: Vec<(f32, f32)> = (0..n).map(seg_px).collect();
    // Outline first, then the body, then the belly stripe and a scale mark on every other segment.
    for i in (first + 1..n).rev() {
        let ((x0, y0), (x1, y1)) = (pts[i], pts[i - 1]);
        blip.fill_capsule(x0, y0, x1, y1, radius(i) + 1.5, radius(i - 1) + 1.5, ink);
    }
    for i in (first + 1..n).rev() {
        let ((x0, y0), (x1, y1)) = (pts[i], pts[i - 1]);
        blip.fill_capsule(x0, y0, x1, y1, radius(i), radius(i - 1), shade(i));
    }
    if !flash {
        for i in (first + 1..n).rev() {
            let ((x0, y0), (x1, y1)) = (pts[i], pts[i - 1]);
            let c = shade(i);
            let hi = BlipColor { r: (c.r + 0.25).min(1.0), g: (c.g + 0.12).min(1.0), b: (c.b + 0.2).min(1.0), a: 1.0 };
            blip.fill_capsule(x0, y0, x1, y1, radius(i) * 0.32, radius(i - 1) * 0.32, hi);
            if i % 2 == 0 && i + 2 < n {
                blip.fill_circle(x0, y0, radius(i) * 0.5, blip::shade(c, 0.6));
            }
        }
    }
    // A new level: a sparkle runs down the body from head to tail.
    if g.state == State::Play && g.banner_t < 0.9 && g.sess.level > 1 {
        let at = g.banner_t / 0.9 * n as f32;
        for k in 0..3 {
            let i = (at as usize).saturating_sub(k * 2).min(n - 1);
            let (sx, sy) = pts[i];
            let a = 1.0 - k as f32 * 0.3;
            blip.fill_glow_circle(sx, sy, 7.0 - k as f32 * 1.5, BlipColor { r: 1.0, g: 0.95, b: 0.6, a });
        }
    }
    if first > 0 { return; }
    let (hx, hy) = pts[0];
    let (fx, fy) = match g.cur_dir { Dir::Up => (0.0, -1.0), Dir::Down => (0.0, 1.0), Dir::Left => (-1.0, 0.0), Dir::Right => (1.0, 0.0) };
    let hr = CELL as f32 / 2.0 - 1.0;
    let head = if flash { BLIP_RED } else { BlipColor { r: 0.45, g: 0.95, b: 0.45, a: 1.0 } };
    // A round head, longer than wide: a capsule from the neck to the snout.
    let (nx, ny) = (hx - fx * 3.0, hy - fy * 3.0);
    let (sx, sy) = (hx + fx * 3.0, hy + fy * 3.0);
    blip.fill_capsule(nx, ny, sx, sy, hr + 1.5, hr * 0.8 + 1.5, ink);
    blip.fill_capsule(nx, ny, sx, sy, hr, hr * 0.8, head);
    // A forked tongue flicks out now and then, in the direction of travel.
    if g.state == State::Play && blip::now().rem_euclid(1.6) < 0.18 {
        let (bx, by) = (hx + fx * (hr + 2.0), hy + fy * (hr + 2.0));
        let (tx, ty) = (hx + fx * (hr + 8.0), hy + fy * (hr + 8.0));
        blip.draw_line_ex(bx, by, tx, ty, 2.0, BLIP_RED);
        for side in [-1.0f32, 1.0] {
            blip.draw_line_ex(tx, ty, tx + fx * 3.0 - fy * side * 3.0, ty + fy * 3.0 + fx * side * 3.0, 1.5, BLIP_RED);
        }
    }
    // The mouth opens as food comes within a cell ahead.
    let ahead = Cell { c: g.snake_at(0).c + fx as i32, r: g.snake_at(0).r + fy as i32 };
    if g.state == State::Play && (ahead == g.food || (g.bonus_active() && ahead == g.bonus)) {
        let open = 2.0 + 4.0 * f;
        let (mx, my) = (hx + fx * (hr + 1.0), hy + fy * (hr + 1.0));
        blip.fill_capsule(mx - fy * open, my + fx * open, mx + fy * open, my - fx * open, 2.5, 2.5, BlipColor { r: 0.35, g: 0.0, b: 0.05, a: 1.0 });
    }
    for side in [-1.0f32, 1.0] {
        let (ex, ey) = (hx + fx * 3.0 - fy * side * 5.5, hy + fy * 3.0 + fx * side * 5.5);
        blip.fill_circle(ex, ey, 3.6, BLIP_WHITE);
        blip.fill_circle(ex + fx * 1.4, ey + fy * 1.4, 1.8, BLIP_BLACK);
    }
}

fn draw_play(blip: &Blip, g: &Game, food: &Texture2D) {
    draw_board(blip);
    draw_pickups(blip, g, food);
    draw_obstacles(blip, g);
    draw_snake(blip, g);
    g.fx.draw(blip);
    if g.banner_t < BANNER_SECS && g.state == State::Play {
        let a = (1.0 - g.banner_t / BANNER_SECS).min(1.0) * 1.5;
        // Row 5 is clear in every obstacle layout.
        blip.draw_centered(&format!("LEVEL {}", g.sess.level), (HUD_H + 5 * CELL + 2) as f32, 4.0,
            BlipColor { a: a.min(1.0), ..BLIP_YELLOW });
    }
    blip.draw_hud(g.sess.score, g.sess.lives);
    blip.draw_hud_multiple(g.sess.score, g.streak.multiple());
}

/// A new layout blinks in, see-through, until it is solid.
fn draw_obstacles(blip: &Blip, g: &Game) {
    let solid = g.obs_fade >= 1.0;
    let a = if solid { 1.0 } else if (g.obs_fade * 8.0) as i32 % 2 == 0 { 0.55 } else { 0.25 };
    for i in 0..g.obstacle_count {
        let obs = g.obstacles[i];
        let (x, y) = ((obs.c * CELL) as f32, (HUD_H + obs.r * CELL) as f32);
        // A bevelled stone: lit top-left, shadowed bottom-right, a crack for texture. A new one rises out of the lawn
        // as it blinks in.
        let full = CELL as f32 - 2.0;
        let s = if solid { full } else { full * (0.4 + 0.6 * g.obs_fade) };
        let (x, y) = (x + (full - s) / 2.0, y + (full - s) / 2.0);
        blip.fill_rect(x + 1.0, y + 1.0, s, s, BlipColor { r: 0.30, g: 0.32, b: 0.36, a });
        blip.fill_rect(x + 4.0, y + 4.0, s - 6.0, s - 6.0, BlipColor { r: 0.46, g: 0.48, b: 0.52, a });
        blip.fill_rect(x + 1.0, y + 1.0, s, 3.0, BlipColor { r: 0.72, g: 0.74, b: 0.78, a });
        blip.fill_rect(x + 1.0, y + 1.0, 3.0, s, BlipColor { r: 0.62, g: 0.64, b: 0.68, a });
        let k = (obs.c * 7 + obs.r * 3) % 4;
        if k < 2 && solid {
            let cx = x + 8.0 + k as f32 * 6.0;
            blip.draw_line_ex(cx, y + 6.0, cx + 3.0, y + 12.0, 1.5, BlipColor { r: 0.25, g: 0.26, b: 0.30, a });
            blip.draw_line_ex(cx + 3.0, y + 12.0, cx + 1.0, y + 17.0, 1.5, BlipColor { r: 0.25, g: 0.26, b: 0.30, a });
        }
    }
}

/// The food, breathing a little so the eye finds it; the bonus and the shears.
fn draw_pickups(blip: &Blip, g: &Game, food: &Texture2D) {
    let breathe = 1.0 + 0.08 * (blip::now() * 5.0).sin();
    let fs = CELL as f32 * breathe;
    let (fcx, fcy) = ((g.food.c * CELL) as f32 + CELL as f32 / 2.0, (HUD_H + g.food.r * CELL) as f32 + CELL as f32 / 2.0);
    blip.draw_texture(food, fcx - fs / 2.0, fcy - fs / 2.0, fs, fs);
    draw_bonus(blip, g);
    draw_shears(blip, g);
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
    // A golden mouse facing the way it last ran: body, head, two round ears, a wiggling tail.
    let (fx, fy) = (g.mouse_dir.0 as f32, g.mouse_dir.1 as f32);
    let t = blip::now();
    let (tx, ty) = (cx - fx * r * 1.0, cy - fy * r * 1.0);
    let wig = (t * 14.0).sin() * r * 0.5;
    blip.draw_line_ex(tx, ty, tx - fx * r * 0.9 - fy * wig, ty - fy * r * 0.9 + fx * wig, 1.5, gold);
    blip.fill_capsule(cx - fx * r * 0.5, cy - fy * r * 0.5, cx + fx * r * 0.3, cy + fy * r * 0.3, r * 0.75, r * 0.6, gold);
    let (hx, hy) = (cx + fx * r * 0.75, cy + fy * r * 0.75);
    for side in [-1.0f32, 1.0] {
        blip.fill_circle(hx - fx * r * 0.2 - fy * side * r * 0.45, hy - fy * r * 0.2 + fx * side * r * 0.45, r * 0.3, hot);
    }
    blip.fill_circle(hx, hy, r * 0.45, gold);
    blip.fill_circle(hx + fx * r * 0.45, hy + fy * r * 0.45, 1.2, BLIP_RED);
    blip.fill_circle(hx + fx * r * 0.1 - fy * r * 0.2, hy + fy * r * 0.1 + fx * r * 0.2, 1.0, BLIP_BLACK);
}

/// The shears: a pale blue X of two blades, flashing as they run out.
fn draw_shears(blip: &Blip, g: &Game) {
    if !g.shears_active() { return; }
    if g.shears_ttl <= BONUS_WARN && (g.shears_ttl * 6.0) as i32 % 3 == 0 { return; }
    let (cx, cy) = cell_centre(g.shears);
    let r = CELL as f32 * 0.34;
    blip.draw_line_ex(cx - r, cy - r, cx + r, cy + r, 3.0, SHEARS_BLUE);
    blip.draw_line_ex(cx - r, cy + r, cx + r, cy - r, 3.0, SHEARS_BLUE);
    // the two finger rings
    blip.fill_circle(cx - r, cy + r, 3.0, SHEARS_BLUE);
    blip.fill_circle(cx + r, cy + r, 3.0, SHEARS_BLUE);
    blip.fill_circle(cx, cy, 2.0, BLIP_WHITE);
}

fn draw_title(blip: &Blip, hi: i32) {
    draw_board(blip);
    // A dark band behind the words, so they read over the lawn.
    blip.fill_rect(0.0, (WIN_H / 4 - 24) as f32, WIN_W as f32, 140.0, BlipColor { r: 0.0, g: 0.0, b: 0.0, a: 0.55 });
    blip.fill_rect(0.0, (WIN_H * 2 / 3 - 10) as f32, WIN_W as f32, 36.0, BlipColor { r: 0.0, g: 0.0, b: 0.0, a: 0.55 });
    blip.draw_centered_outlined("SERPENT", (WIN_W / 2) as f32, (WIN_H / 4) as f32, 6.0, BLIP_GREEN,
        BlipColor { r: 0.02, g: 0.12, b: 0.04, a: 1.0 });
    // SERPENT is sz=6 (42px tall) — clear its bottom by a real margin.
    blip.draw_hi(hi, (WIN_H / 4 + 50) as f32, BLIP_YELLOW);
    let by = web::controls();
    blip.draw_start_prompt((WIN_H / 2) as f32);
    blip.draw_centered(by.pick("ARROW KEYS OR WASD", "STEER WITH THE PAD", "SWIPE TO STEER"),
        (WIN_H * 2 / 3) as f32, 2.0, BLIP_GRAY);
    draw_title_snake(blip, blip::now());
}

/// A snake winding across the title screen after a golden mouse, wrapping round: drawn as in play, an outlined body
/// tapering to the tail with a belly stripe and scale marks.
fn draw_title_snake(blip: &Blip, t: f32) {
    const SEGS: usize = 22;
    let span = WIN_W as f32 + 260.0;
    let at = |d: f32| {
        let x = (t * 90.0 - d).rem_euclid(span) - 160.0;
        (x, WIN_H as f32 * 0.83 + ((t * 90.0 - d) * 0.035).sin() * 22.0)
    };
    let ink = BlipColor { r: 0.02, g: 0.12, b: 0.04, a: 1.0 };
    let shade = |i: usize| {
        let k = i as f32 / SEGS as f32;
        scale_green(k)
    };
    let r = |i: usize| 11.0 * if i + 5 >= SEGS { 0.5 + 0.5 * (SEGS - 1 - i) as f32 / 5.0 } else { 1.0 };
    let pts: Vec<(f32, f32)> = (0..SEGS).map(|i| at(i as f32 * 10.0)).collect();
    // Skip a joint that wraps across the screen edge.
    let joined = |i: usize| (pts[i].0 - pts[i - 1].0).abs() < 40.0;
    for i in (1..SEGS).rev().filter(|&i| joined(i)) {
        blip.fill_capsule(pts[i].0, pts[i].1, pts[i - 1].0, pts[i - 1].1, r(i) + 1.5, r(i - 1) + 1.5, ink);
    }
    for i in (1..SEGS).rev().filter(|&i| joined(i)) {
        blip.fill_capsule(pts[i].0, pts[i].1, pts[i - 1].0, pts[i - 1].1, r(i), r(i - 1), shade(i));
        let c = shade(i);
        let hi = BlipColor { r: (c.r + 0.25).min(1.0), g: (c.g + 0.12).min(1.0), b: (c.b + 0.2).min(1.0), a: 1.0 };
        blip.fill_capsule(pts[i].0, pts[i].1, pts[i - 1].0, pts[i - 1].1, r(i) * 0.32, r(i - 1) * 0.32, hi);
        if i % 2 == 0 && i + 3 < SEGS { blip.fill_circle(pts[i].0, pts[i].1, r(i) * 0.5, blip::shade(c, 0.6)); }
    }
    let (hx, hy) = pts[0];
    blip.fill_circle(hx + 2.0, hy, 13.5, ink);
    blip.fill_circle(hx + 2.0, hy, 12.0, BlipColor { r: 0.45, g: 0.95, b: 0.45, a: 1.0 });
    for side in [-1.0f32, 1.0] {
        blip.fill_circle(hx + 5.0, hy + side * 5.5, 3.6, BLIP_WHITE);
        blip.fill_circle(hx + 6.4, hy + side * 5.5, 1.8, BLIP_BLACK);
    }
    if t.rem_euclid(1.2) < 0.2 {
        blip.draw_line_ex(hx + 14.0, hy, hx + 22.0, hy, 2.0, BLIP_RED);
        for side in [-1.0f32, 1.0] { blip.draw_line_ex(hx + 22.0, hy, hx + 25.0, hy + side * 3.0, 1.5, BLIP_RED); }
    }
    // The golden mouse, always a little out of reach.
    let mx = (t * 90.0 + 70.0).rem_euclid(span) - 160.0;
    let my = WIN_H as f32 * 0.83 + ((t * 90.0 + 70.0) * 0.035).sin() * 22.0 + (t * 18.0).sin().abs() * -3.0;
    let wig = (t * 14.0).sin() * 4.0;
    blip.draw_line_ex(mx - 9.0, my, mx - 17.0, my + wig, 1.5, GOLD);
    blip.fill_capsule(mx - 5.0, my, mx + 3.0, my, 8.0, 6.5, GOLD);
    blip.fill_circle(mx + 2.0, my - 6.0, 3.5, BlipColor { r: 1.0, g: 0.98, b: 0.72, a: 1.0 });
    blip.fill_circle(mx + 8.0, my, 5.0, GOLD);
    blip.fill_circle(mx + 12.5, my, 1.3, BLIP_RED);
    blip.fill_circle(mx + 9.0, my - 2.0, 1.1, BLIP_BLACK);
}

fn draw_over(blip: &Blip, g: &Game, hi: i32) {
    // The field as it ended, dimmed under the score.
    draw_board(blip);
    draw_obstacles(blip, g);
    draw_snake(blip, g);
    blip.fill_rect(0.0, HUD_H as f32, WIN_W as f32, (WIN_H - HUD_H) as f32, BlipColor { r: 0.0, g: 0.0, b: 0.0, a: 0.68 });
    let (title, colour) = if g.snake_len + g.obstacle_count == MAX_LEN { ("FIELD CLEARED", BLIP_GREEN) } else { ("GAME OVER", BLIP_RED) };
    blip.draw_end_screen(title, g.sess.score, hi, colour, BLIP_GREEN, BLIP_YELLOW, !g.dead_timer.active());
    blip.draw_game_over_note(&format!("LEVEL {}   LENGTH {}", g.sess.level, g.snake_len), BLIP_GRAY);
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("SERPENT", WIN_W, WIN_H)
}

#[blip::macroquad::main(conf)]
async fn main() {
    blip::bot::playthroughs(bot::PLAYTHROUGHS);
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let food = load_png(blip::png!("food"));

    let sfx = Sounds::load().await;
    let mut music = Jukebox::new("serpent", &["SLITHER", "PROWL", "FRENZY"]);

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
            State::Title => draw_title(&blip, web::high_score()),
            State::Over  => draw_over(&blip, &g, web::high_score()),
            State::Play | State::Dead => blip.shaken(g.shake, CRASH_SHAKE, 5.0, || draw_play(&blip, &g, &food)),
        }
        g.shake = (g.shake - dt).max(0.0);

        blip.next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_golden_mouse_runs_from_a_head_that_comes_near() {
        let mut g = running();
        let head = g.snake_at(0);
        g.bonus = Cell { c: head.c + 3, r: head.r };
        g.bonus_ttl = BONUS_TTL;
        let dist = |g: &Game| (g.bonus.c - head.c).abs() + (g.bonus.r - head.r).abs();
        assert!(!scurry(&mut g, MOUSE_HOP * 0.5), "not before its hop is due");
        assert!(scurry(&mut g, MOUSE_HOP));
        assert_eq!(dist(&g), 4, "one cell farther away");
        g.bonus = Cell { c: head.c + MOUSE_SEES + 2, r: head.r };
        let before = g.bonus;
        assert!(!scurry(&mut g, MOUSE_HOP));
        assert_eq!(g.bonus, before, "a far head is not seen");
    }

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
