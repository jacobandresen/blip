//! Adder (Viper), a pit snake: walls and your own body kill, the adder grows
//! one segment per egg it swallows, and every fourth egg it sheds — the tail
//! it leaves behind stays on the floor as wall, so the pit slowly fills with
//! its own history. Holding fire, the strike, is the original's player-set
//! speed turned into a risk: a strike step is barely half a glide step and an
//! egg taken on one scores double, at the cost of half the turn buffer.

use blip::input::{
    btn1_pressed, key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_DOWN, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::texture::Texture2D;
use blip::{
    load_png, play_sfx, rand_int, web, window_conf, Blip, BlipColor, Fx, Jukebox, lerp, LifeResult,
    Session, Timer, GAME_OVER_MIN_WAIT, BLIP_BLACK, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
};

#[cfg(not(target_arch = "wasm32"))]
mod bot;

// ---- layout -----------------------------------------------------------
// 32px cells, not Serpent's 24: the shed skin and the adder's bands have to
// read at a glance on a phone that shows the 480px pit at half size.
const COLS: i32 = 15;
const ROWS: i32 = 16;
const CELL: i32 = 32;
const HUD_H: i32 = 28;
const WIN_W: i32 = COLS * CELL;
const WIN_H: i32 = ROWS * CELL + HUD_H;
const CELLS: usize = (COLS * ROWS) as usize;
const MAX_LEN: usize = CELLS;

// ---- tuning -----------------------------------------------------------
const LIVES_START: i32 = 3;
const STEP_MS_START: f32 = 200.0; // a glide step in pit 1
const STEP_MS_PIT: f32 = 12.0;    // every pit is this much quicker
const STEP_MS_MIN: f32 = 110.0;
const STRIKE_RATE: f32 = 0.55;    // a strike step, as a slice of the glide step
const EGGS_PER_PIT: i32 = 6;
const SHED_EVERY: i32 = 4;        // eggs between sheds
const SHED_SEGS: usize = 3;
/// A shed may not leave the adder less than this much of the pit to move in.
/// Skin is never removed, so without the rule a lucky run would wall itself in.
const PIT_OPEN: f32 = 0.35;
const SCORE_EGG: i32 = 10;
const AHEAD: i32 = 3;             // how far ahead of the head skin may not fall
const MAX_SKIN: usize = 96;       // 40% of the pit, past which sheds stop
const FAST_PIT: i32 = 4;          // the hunt tune from here on
const DEAD_SECS: f32 = 1.5;
const BANNER_SECS: f32 = 1.4;
const TURN_QUEUE: usize = 2;

// ---- palette ----------------------------------------------------------
const EARTH: BlipColor = BlipColor { r: 0.085, g: 0.07, b: 0.05, a: 1.0 };
const EARTH_LIT: BlipColor = BlipColor { r: 0.115, g: 0.095, b: 0.068, a: 1.0 };
const WALL: BlipColor = BlipColor { r: 0.40, g: 0.29, b: 0.15, a: 1.0 };
const SKIN: BlipColor = BlipColor { r: 0.86, g: 0.81, b: 0.65, a: 1.0 };
const SKIN_EDGE: BlipColor = BlipColor { r: 0.56, g: 0.51, b: 0.39, a: 1.0 };
const AMBER: BlipColor = BlipColor { r: 0.95, g: 0.66, b: 0.22, a: 1.0 };
const GOLD: BlipColor = BlipColor { r: 1.0, g: 0.86, b: 0.34, a: 1.0 };
const HEAD: BlipColor = BlipColor { r: 0.62, g: 0.73, b: 0.28, a: 1.0 };
const HIDE: BlipColor = BlipColor { r: 0.26, g: 0.32, b: 0.13, a: 1.0 };
const SAND_TEXT: BlipColor = BlipColor { r: 0.52, g: 0.45, b: 0.31, a: 1.0 };

/// The centre of a pit cell, in canvas pixels.
fn centre(c: Cell) -> (f32, f32) {
    ((c.c * CELL + CELL / 2) as f32, (HUD_H + c.r * CELL + CELL / 2) as f32)
}

fn inside(c: Cell) -> bool { c.c >= 0 && c.c < COLS && c.r >= 0 && c.r < ROWS }

fn at(c: Cell) -> usize { (c.r * COLS + c.c) as usize }

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
    fn delta(self) -> (i32, i32) {
        match self { Dir::Up => (0, -1), Dir::Right => (1, 0), Dir::Down => (0, 1), Dir::Left => (-1, 0) }
    }
}

const DIRS: [Dir; 4] = [Dir::Up, Dir::Right, Dir::Down, Dir::Left];

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct Cell { c: i32, r: i32 }

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Play, Dead, Over }

struct Game {
    /// Ring buffer like Serpent's: the head moves backwards, so `seg(i)` counts
    /// out from the head and the tail falls off the end on its own.
    snake: [Cell; MAX_LEN],
    head: usize,
    len: usize,
    dir: Dir,
    /// Turns asked for but not yet taken. See `turn_cap`.
    turns: [Dir; TURN_QUEUE],
    turns_len: usize,
    egg: Cell,
    /// Shed skin: wall for the rest of the run, and the reason a pit gets
    /// harder. Never cleared by a life, only by a new game.
    skin: [Cell; MAX_SKIN],
    skin_len: usize,
    sess: Session,
    /// Eggs swallowed in this pit.
    eggs: i32,
    step_ms: f32,
    /// True while the throttle is down, kept for the drawing and the whip.
    striking: bool,
    dead_timer: Timer,
    state: State,
    fx: Fx,
    banner_t: f32,
}

impl Game {
    fn new() -> Self {
        Self {
            snake: [Cell { c: 0, r: 0 }; MAX_LEN],
            head: 0,
            len: 0,
            dir: Dir::Right,
            turns: [Dir::Right; TURN_QUEUE],
            turns_len: 0,
            egg: Cell { c: 0, r: 0 },
            skin: [Cell { c: 0, r: 0 }; MAX_SKIN],
            skin_len: 0,
            sess: Session::new(LIVES_START),
            eggs: 0,
            step_ms: 0.0,
            striking: false,
            dead_timer: Timer::default(),
            state: State::Title,
            fx: Fx::new(),
            banner_t: BANNER_SECS,
        }
    }

    #[inline]
    fn seg(&self, i: usize) -> Cell { self.snake[(self.head + i) % MAX_LEN] }

    fn is_skin(&self, c: Cell) -> bool { self.skin[..self.skin_len].contains(&c) }

    fn body_at(&self, c: Cell, len: usize) -> bool { (0..len).any(|i| self.seg(i) == c) }

    /// A glide step quickens every pit down to a floor; the strike takes a
    /// further slice off whichever base the pit is on.
    fn step_interval(&self, strike: bool) -> f32 {
        let base = (STEP_MS_START - (self.sess.level - 1) as f32 * STEP_MS_PIT).max(STEP_MS_MIN);
        if strike { base * STRIKE_RATE } else { base }
    }

    /// What an egg is worth: the pit it was found in, doubled on the step a
    /// strike took it.
    fn egg_points(&self, strike: bool) -> i32 {
        SCORE_EGG * self.sess.level * if strike { 2 } else { 1 }
    }

    /// Two turns fit a corner gesture at glide speed. At strike speed a second
    /// buffered turn is one the adder never visibly takes, so the neck is
    /// stiff and the queue holds one.
    fn turn_cap(strike: bool) -> usize { if strike { 1 } else { TURN_QUEUE } }

    /// Queue a turn if the adder can take it, checked against the last
    /// direction queued rather than the one it is travelling: up then left from
    /// travelling right is otherwise a U-turn into its own neck.
    fn queue_turn(&mut self, dir: Dir, strike: bool) {
        let last = if self.turns_len == 0 { self.dir } else { self.turns[self.turns_len - 1] };
        if dir == last || dir == last.opposite() { return; }
        if self.turns_len >= Self::turn_cap(strike) { return; }
        self.turns[self.turns_len] = dir;
        self.turns_len += 1;
    }

    fn take_turn(&mut self) -> Dir {
        if self.turns_len == 0 { return self.dir; }
        let next = self.turns[0];
        for i in 1..self.turns_len { self.turns[i - 1] = self.turns[i]; }
        self.turns_len -= 1;
        next
    }

    /// The free cells the head can reach, and whether the egg is one of them.
    /// Every egg is placed out of this list and every shed is judged by it: an
    /// egg behind a wall the game built itself is a life the player cannot play.
    fn reachable(&self, len: usize, skin_len: usize) -> (Vec<Cell>, bool) {
        let mut blocked = vec![false; CELLS];
        // From index 1: the head's own cell is where the walk starts.
        for i in 1..len { blocked[at(self.seg(i))] = true; }
        for i in 0..skin_len { blocked[at(self.skin[i])] = true; }
        let start = self.seg(0);
        let mut seen = vec![false; CELLS];
        seen[at(start)] = true;
        let mut out = Vec::new();
        let mut stack = vec![start];
        let mut egg_ok = false;
        while let Some(c) = stack.pop() {
            if c == self.egg { egg_ok = true; }
            for d in DIRS {
                let (dc, dr) = d.delta();
                let n = Cell { c: c.c + dc, r: c.r + dr };
                if !inside(n) || seen[at(n)] || blocked[at(n)] { continue; }
                seen[at(n)] = true;
                out.push(n);
                stack.push(n);
            }
        }
        (out, egg_ok)
    }

    fn spawn_egg(&mut self) {
        let (open, _) = self.reachable(self.len, self.skin_len);
        if open.is_empty() { return; } // the pit is full: the old egg stands
        self.egg = open[rand_int(0, open.len() as i32 - 1) as usize];
    }

    /// Freeze the tail as skin. Cells in the head's next few steps are left
    /// out of it, and the shed is skipped outright if the pit stops being open
    /// enough or the egg is walled off — the meter stays full and it is tried
    /// again after the next egg.
    fn shed(&mut self) -> bool {
        if self.len <= SHED_SEGS + 1 || self.skin_len + SHED_SEGS > MAX_SKIN { return false; }
        let (dc, dr) = self.dir.delta();
        let head = self.seg(0);
        let ahead: Vec<Cell> = (1..=AHEAD).map(|k| Cell { c: head.c + dc * k, r: head.r + dr * k }).collect();
        // Contiguous from the very tip, or the skin would land inside the body.
        let mut take = 0;
        while take < SHED_SEGS {
            let c = self.seg(self.len - 1 - take);
            if ahead.contains(&c) { break; }
            take += 1;
        }
        if take == 0 { return false; }

        let len = self.len - take;
        let skin = self.skin_len + take;
        for k in 0..take { self.skin[self.skin_len + k] = self.seg(self.len - 1 - k); }
        let (open, egg_ok) = self.reachable(len, skin);
        let free = CELLS - len - skin;
        if !egg_ok || (open.len() as f32) < PIT_OPEN * free as f32 {
            return false;
        }
        self.len = len;
        self.skin_len = skin;
        true
    }

    /// One step of the adder. `strike` is the throttle as the step lands, which
    /// is what the egg it takes is scored at.
    fn step(&mut self, strike: bool, sfx: &Sounds) {
        self.dir = self.take_turn();
        let (dc, dr) = self.dir.delta();
        let head = self.seg(0);
        let next = Cell { c: head.c + dc, r: head.r + dr };

        // Named deaths, because the bot's playtests are read by cause.
        let why = if !inside(next) { "wall" }
            else if self.is_skin(next) { "skin" }
            else if self.body_at(next, self.len.saturating_sub(1)) { "self" }
            else { "" };
        if !why.is_empty() { self.die(why, sfx); return; }

        self.head = (self.head + MAX_LEN - 1) % MAX_LEN;
        self.snake[self.head] = next;
        if next != self.egg { return; }

        let points = self.egg_points(strike);
        self.sess.add_score(points);
        play_sfx(&sfx.eat);
        blip::bot::add("eggs", 1.0);
        if strike { blip::bot::add("strike_eggs", 1.0); }
        let (x, y) = centre(next);
        self.fx.ring(x, y, CELL as f32 * 1.2, 0.28, GOLD);
        self.fx.popup(x, y - 12.0, &format!("+{points}"), if strike { AMBER } else { BLIP_WHITE });
        self.eggs += 1;
        if self.len < MAX_LEN { self.len += 1; }

        if self.eggs % SHED_EVERY == 0 && self.shed() {
            play_sfx(&sfx.shed);
            blip::bot::add("sheds", 1.0);
            let (sx, sy) = centre(self.seg(self.len - 1));
            self.fx.burst(sx, sy, 12, 90.0, SKIN);
            self.fx.popup(sx, sy - 12.0, "SHED", SKIN);
        }
        if self.eggs >= EGGS_PER_PIT {
            self.advance_pit(sfx);
        } else {
            self.spawn_egg();
        }
    }

    fn die(&mut self, why: &str, sfx: &Sounds) {
        blip::bot::add(&format!("death_{why}"), 1.0);
        blip::bot::set("length", self.len as f64);
        let (x, y) = centre(self.seg(0));
        self.fx.burst(x, y, 16, 140.0, BLIP_RED);
        play_sfx(&sfx.game_over);
        match self.sess.lose_life() {
            LifeResult::StillAlive => { self.dead_timer.start(DEAD_SECS); self.state = State::Dead; }
            LifeResult::GameOver => {
                self.dead_timer.start(GAME_OVER_MIN_WAIT);
                self.state = State::Over;
                web::report_score(self.sess.score);
            }
        }
    }

    /// A pit is eaten out: the floor keeps every skin it gathered, the adder
    /// comes back short in the middle, and the next pit is quicker.
    fn advance_pit(&mut self, sfx: &Sounds) {
        self.sess.next_level();
        self.eggs = 0;
        blip::bot::set("pit", self.sess.level as f64);
        play_sfx(&sfx.pit);
        self.banner_t = 0.0;
        self.reset_snake();
    }

    /// Three segments in a middle row that has room for them and their first
    /// steps. Rows are tried out from the middle, because a later pit starts
    /// among the skin shed in the pits before it.
    fn reset_snake(&mut self) {
        self.head = 0;
        self.len = 3;
        self.dir = Dir::Right;
        self.turns_len = 0;
        self.step_ms = 0.0;
        let mut rows: Vec<i32> = Vec::new();
        for k in 0..ROWS { rows.push(ROWS / 2 + if k % 2 == 0 { k / 2 } else { -(k / 2 + 1) }); }
        let row = rows.into_iter()
            .find(|&r| (COLS / 2 - 3..=COLS / 2 + 4).all(|c| !self.is_skin(Cell { c, r })))
            .unwrap_or(ROWS / 2);
        for i in 0..self.len { self.snake[i] = Cell { c: COLS / 2 - i as i32, r: row }; }
        self.spawn_egg();
    }

    fn start_game(&mut self) {
        self.sess.reset(LIVES_START);
        self.skin_len = 0;
        self.eggs = 0;
        self.fx = Fx::new();
        self.reset_snake();
        self.state = State::Play;
    }

    /// The music for where the game is: the slow coil, then the hunt.
    fn track(&self) -> usize {
        if self.state != State::Title && self.sess.level >= FAST_PIT { 1 } else { 0 }
    }
}

struct Sounds {
    eat: blip::BlipSound,
    shed: blip::BlipSound,
    strike: blip::BlipSound,
    pit: blip::BlipSound,
    game_over: blip::BlipSound,
}

/// Held fire is the throttle. Every other cabinet game fires on the press;
/// the adder is the one where the button is a state, not an event.
fn strike_held() -> bool { key_held(BLIP_KEY_SPACE) }

fn update_title(g: &mut Game) {
    if btn1_pressed() {
        web::spend_coin();
        g.start_game();
    }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    let strike = strike_held();
    if key_pressed(BLIP_KEY_UP)    || key_pressed(BLIP_KEY_W) { g.queue_turn(Dir::Up, strike); }
    if key_pressed(BLIP_KEY_DOWN)  || key_pressed(BLIP_KEY_S) { g.queue_turn(Dir::Down, strike); }
    if key_pressed(BLIP_KEY_LEFT)  || key_pressed(BLIP_KEY_A) { g.queue_turn(Dir::Left, strike); }
    if key_pressed(BLIP_KEY_RIGHT) || key_pressed(BLIP_KEY_D) { g.queue_turn(Dir::Right, strike); }

    // The whip plays on the press, not on every step of the strike.
    if strike && !g.striking { play_sfx(&sfx.strike); }
    g.striking = strike;

    g.fx.update(dt);
    g.banner_t += dt;
    g.step_ms += dt * 1000.0;
    let interval = g.step_interval(strike);
    if g.step_ms < interval { return; }
    g.step_ms -= interval;
    g.step(strike, sfx);
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

// ---- drawing -------------------------------------------------------------

/// The pit floor: alternating earth tones, so the cells the adder moves
/// between are legible at a glance, inside a wall of packed earth.
fn draw_pit(blip: &Blip) {
    let top = HUD_H as f32;
    blip.fill_rect(0.0, top, WIN_W as f32, WIN_H as f32 - top, EARTH);
    for r in 0..ROWS {
        for c in (0..COLS).step_by(1) {
            if (c + r) % 2 == 0 { continue; }
            blip.fill_rect((c * CELL) as f32, (HUD_H + r * CELL) as f32, CELL as f32, CELL as f32, EARTH_LIT);
        }
    }
    let w = 3.0;
    blip.fill_rect(0.0, top, WIN_W as f32, w, WALL);
    blip.fill_rect(0.0, WIN_H as f32 - w, WIN_W as f32, w, WALL);
    blip.fill_rect(0.0, top, w, WIN_H as f32 - top, WALL);
    blip.fill_rect(WIN_W as f32 - w, top, w, WIN_H as f32 - top, WALL);
}

/// One flake of shed skin: pale, split down the middle, and never the same
/// shape as the floor it lies on.
fn draw_skin(blip: &Blip, c: Cell) {
    let (x, y) = ((c.c * CELL) as f32, (HUD_H + c.r * CELL) as f32);
    let s = CELL as f32;
    blip.fill_rect(x + 3.0, y + 4.0, s - 6.0, s - 9.0, SKIN_EDGE);
    blip.fill_rect(x + 4.0, y + 5.0, s - 8.0, s - 11.0, SKIN);
    blip.fill_rect(x + s / 2.0 - 1.0, y + 5.0, 2.0, s - 11.0, SKIN_EDGE);
    blip.fill_rect(x + 3.0, y + 3.0, 5.0, 3.0, SKIN);
    blip.fill_rect(x + s - 9.0, y + s - 8.0, 6.0, 3.0, SKIN);
}

/// The adder as one body: thick strokes between segment centres, an amber band
/// every third segment, brighter toward the head, fangs out on a strike.
fn draw_adder(blip: &Blip, g: &Game) {
    let f = if g.state == State::Play {
        (g.step_ms / g.step_interval(strike_held())).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let flash = g.state == State::Dead && (g.dead_timer.remaining() / 0.12) as i32 % 2 == 0;
    let half = CELL as f32 / 2.0;
    let seg_px = |i: usize| -> (f32, f32) {
        let cur = g.seg(i);
        let prev = if i + 1 < g.len {
            g.seg(i + 1)
        } else {
            let ahead = g.seg(i - 1);
            Cell { c: 2 * cur.c - ahead.c, r: 2 * cur.r - ahead.r }
        };
        (lerp(prev.c as f32, cur.c as f32, f) * CELL as f32 + half,
         HUD_H as f32 + lerp(prev.r as f32, cur.r as f32, f) * CELL as f32 + half)
    };
    let thick = CELL as f32 - 8.0;
    let n = g.len.max(1);
    let shade = |i: usize| -> BlipColor {
        if flash { return BLIP_RED; }
        let k = i as f32 / n as f32;
        let (a, b) = if i % 3 == 1 { (AMBER, BlipColor { r: 0.55, g: 0.36, b: 0.12, a: 1.0 }) } else { (HEAD, HIDE) };
        BlipColor { r: lerp(a.r, b.r, k), g: lerp(a.g, b.g, k), b: lerp(a.b, b.b, k), a: 1.0 }
    };
    for i in (1..n).rev() {
        let (x0, y0) = seg_px(i);
        let (x1, y1) = seg_px(i - 1);
        let c = shade(i);
        blip.draw_line_ex(x0, y0, x1, y1, thick, c);
        blip.fill_rect(x0 - thick / 2.0, y0 - thick / 2.0, thick, thick, c);
    }
    let (hx, hy) = seg_px(0);
    let hs = CELL as f32 - 2.0;
    let skin = if flash { BLIP_RED } else { BlipColor { r: 0.72, g: 0.84, b: 0.34, a: 1.0 } };
    blip.fill_rect(hx - hs / 2.0, hy - hs / 2.0, hs, hs, skin);
    let (fx, fy) = match g.dir {
        Dir::Up => (0.0, -1.0), Dir::Down => (0.0, 1.0), Dir::Left => (-1.0, 0.0), Dir::Right => (1.0, 0.0),
    };
    let t = blip::macroquad::time::get_time();
    if g.striking && g.state == State::Play {
        // The fangs, out while the throttle is down.
        for side in [-1.0f32, 1.0] {
            let (bx, by) = (hx + fx * hs * 0.30 - fy * side * 4.5, hy + fy * hs * 0.30 + fx * side * 4.5);
            blip.fill_rect(bx - 2.0, by - 2.0, 4.0, 4.0, BLIP_WHITE);
        }
    } else if g.state == State::Play && t.rem_euclid(1.8) < 0.2 {
        let tip = hs / 2.0 + 7.0;
        blip.draw_line_ex(hx + fx * hs / 2.0, hy + fy * hs / 2.0, hx + fx * tip, hy + fy * tip, 2.0, BLIP_RED);
    }
    for side in [-1.0f32, 1.0] {
        let (ex, ey) = (hx + fx * 5.0 - fy * side * 6.5, hy + fy * 5.0 + fx * side * 6.5);
        blip.fill_circle(ex, ey, 3.4, BLIP_WHITE);
        blip.fill_circle(ex + fx * 1.4, ey + fy * 1.4, 1.7, BLIP_BLACK);
    }
}

/// The pit number and the throttle, in the pit's own corners: the top bar
/// belongs to the engine (score, lives, hi) and is full.
fn draw_readouts(blip: &Blip, g: &Game) {
    let y = (WIN_H - 26) as f32;
    blip.draw_text_outlined(&format!("PIT {}", g.sess.level), 8.0, y, 2.0, SAND_TEXT, BLIP_BLACK);
    // Five pips that fill in turn while the throttle is down: the only readout
    // the game needs for it, since the whip already sounds.
    let lit = if g.striking { (blip::macroquad::time::get_time() * 12.0) as i32 % 6 } else { 0 };
    let dim = BlipColor { r: 0.30, g: 0.26, b: 0.18, a: 1.0 };
    for k in 0..5 {
        let x = WIN_W as f32 - 8.0 - (5 - k) as f32 * 14.0;
        blip.fill_rect(x, y + 2.0, 10.0, 12.0, if k < lit { AMBER } else { dim });
    }
}

fn draw_play(blip: &Blip, g: &Game, egg: &Texture2D) {
    draw_pit(blip);
    for i in 0..g.skin_len { draw_skin(blip, g.skin[i]); }
    let breathe = 1.0 + 0.06 * (blip::macroquad::time::get_time() as f32 * 5.0).sin();
    let s = CELL as f32 * breathe;
    let (ex, ey) = centre(g.egg);
    blip.draw_texture(egg, ex - s / 2.0, ey - s / 2.0, s, s);
    draw_adder(blip, g);
    g.fx.draw(blip);
    draw_readouts(blip, g);
    if g.banner_t < BANNER_SECS {
        let a = (1.0 - g.banner_t / BANNER_SECS).min(1.0);
        blip.draw_centered(&format!("PIT {}", g.sess.level), (HUD_H + 6 * CELL) as f32, 4.0,
            BlipColor { a, ..BLIP_YELLOW });
    }
    blip.draw_hud(g.sess.score, g.sess.lives);
}

fn draw_title(blip: &Blip, hi: &web::HighScore) {
    blip.clear(BLIP_BLACK);
    blip.draw_centered("ADDER", (WIN_H / 4) as f32, 6.0, HEAD);
    // ADDER is sz=6 (42px tall) — clear its bottom by a real margin.
    blip.draw_hi(hi, (WIN_H / 4 + 50) as f32, BLIP_YELLOW);
    let by = web::controls();
    blip.draw_centered(by.pick("PRESS FIRE", "PRESS FIRE", "TAP TO START"), (WIN_H / 2) as f32, 3.0, BLIP_WHITE);
    blip.draw_centered(by.pick("ARROWS OR WASD TO STEER", "STEER WITH THE PAD", "SWIPE TO STEER"),
        (WIN_H * 2 / 3) as f32, 2.0, SAND_TEXT);
    blip.draw_centered("HOLD FIRE TO STRIKE", (WIN_H * 2 / 3 + 24) as f32, 2.0, AMBER);
    // The floor a run starts on: other snakes' skin, already there.
    let flakes = [(2, 12), (5, 13), (9, 12), (12, 14), (1, 15), (7, 15), (13, 11)];
    for (c, r) in flakes { draw_skin(blip, Cell { c, r }); }
    draw_title_coil(blip, blip::macroquad::time::get_time() as f32);
}

/// A viper coiling across the title screen, head first, tail last.
fn draw_title_coil(blip: &Blip, t: f32) {
    const SEGS: usize = 18;
    let span = WIN_W as f32 + 140.0;
    let at = |d: f32| {
        let x = (t * 30.0 + d).rem_euclid(span) - 70.0;
        (x, WIN_H as f32 * 0.88 + (x * 0.05 + t * 0.7).sin() * 22.0)
    };
    for i in (0..SEGS).rev() {
        let (x, y) = at(i as f32 * 9.0);
        let k = i as f32 / SEGS as f32;
        let c = if i % 3 == 1 {
            BlipColor { r: lerp(0.95, 0.5, k), g: lerp(0.66, 0.32, k), b: lerp(0.22, 0.10, k), a: 1.0 }
        } else {
            BlipColor { r: lerp(HEAD.r, HIDE.r, k), g: lerp(HEAD.g, HIDE.g, k), b: lerp(HEAD.b, HIDE.b, k), a: 1.0 }
        };
        blip.fill_circle(x, y, 10.0 - k * 5.0, c);
    }
    let (hx, hy) = at(0.0);
    blip.fill_circle(hx + 4.0, hy - 3.0, 2.6, BLIP_WHITE);
    blip.fill_circle(hx + 5.0, hy - 3.0, 1.3, BLIP_BLACK);
}

fn draw_over(blip: &Blip, g: &Game, hi: &web::HighScore) {
    blip.clear(BLIP_BLACK);
    blip.draw_game_over(g.sess.score, hi, BLIP_RED, HEAD, BLIP_YELLOW, !g.dead_timer.active());
    blip.draw_centered(&format!("PIT {}   LENGTH {}", g.sess.level, g.len),
        (WIN_H / 2 + 54) as f32, 2.0, SAND_TEXT);
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("ADDER", WIN_W, WIN_H)
}

const EGG_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/egg.png"));
const EAT_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/eat.wav"));
const SHED_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/shed.wav"));
const STRIKE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/strike.wav"));
const PIT_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/pit.wav"));
const GAME_OVER_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/game_over.wav"));

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let egg = load_png(EGG_PNG);

    let sfx = Sounds {
        eat: blip::audio::load_sound(EAT_WAV).await,
        shed: blip::audio::load_sound(SHED_WAV).await,
        strike: blip::audio::load_sound(STRIKE_WAV).await,
        pit: blip::audio::load_sound(PIT_WAV).await,
        game_over: blip::audio::load_sound(GAME_OVER_WAV).await,
    };
    let mut music = Jukebox::new(&[blip_assets::adder::coil_wav, blip_assets::adder::hunt_wav]);
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
            State::Play | State::Dead => draw_play(&blip, &g, &egg),
        }

        blip.next_frame(60).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An adder mid-pit, travelling right, with nothing queued.
    fn running() -> Game {
        let mut g = Game::new();
        g.reset_snake();
        g
    }

    /// One step as update_play() takes it: the turn comes off the queue and
    /// becomes the direction.
    fn step_dir(g: &mut Game) -> Dir {
        g.dir = g.take_turn();
        g.dir
    }

    /// An adder that has eaten its way across the pit: full rows, head first,
    /// so on every other row the tail comes back to lie beside the head.
    fn big_snake(g: &mut Game, len: usize) {
        g.head = 0;
        g.len = len;
        g.dir = Dir::Right;
        for i in 0..len {
            let row = (i / COLS as usize) as i32;
            let k = i % COLS as usize;
            let c = if row % 2 == 0 { k as i32 } else { COLS - 1 - k as i32 };
            g.snake[i] = Cell { c, r: row };
        }
    }

    #[test]
    fn a_shed_leaves_the_tail_it_took_on_the_floor() {
        let mut g = running();
        big_snake(&mut g, 24);
        let tail: Vec<Cell> = (0..SHED_SEGS).map(|k| g.seg(g.len - 1 - k)).collect();
        let len = g.len;
        assert!(g.shed(), "a shed in the open was refused");
        assert_eq!(g.len, len - SHED_SEGS, "the adder did not lose the tail it shed");
        for c in &tail { assert!(g.is_skin(*c), "the shed tail is not on the floor"); }
        assert!((0..g.len).all(|i| !tail.contains(&g.seg(i))), "the skin is still part of the body");
    }

    #[test]
    fn a_shed_never_walls_the_egg_off_or_the_adder_in() {
        // The shed is the only thing that can close the pit, and it is never
        // taken back: a run that walls itself in is unplayable, not hard.
        for len in [8usize, 20, 40] {
            let mut g = running();
            big_snake(&mut g, len);
            g.egg = Cell { c: COLS - 2, r: ROWS - 2 };
            for round in 0..3 {
                let (had_len, had_skin) = (g.len, g.skin_len);
                if !g.shed() { continue; }
                assert!(g.len < had_len && g.skin_len > had_skin, "round {round}: the shed changed nothing");
                let (open, egg_ok) = g.reachable(g.len, g.skin_len);
                let free = CELLS - g.len - g.skin_len;
                assert!(egg_ok, "round {round}: the shed buried the egg");
                assert!(open.len() as f32 >= PIT_OPEN * free as f32,
                    "round {round}: the shed left {} of {free} free cells", open.len());
            }
        }
    }

    #[test]
    fn a_shed_is_skipped_when_the_tail_lies_ahead_of_the_head() {
        // A tight coil can put the tail two cells in front of the head; skin
        // there is a death the player never had the chance to see coming.
        let mut g = running();
        g.head = 0;
        g.dir = Dir::Up;
        let path = [(4, 4), (3, 4), (2, 4), (2, 3), (3, 3), (4, 3)];
        g.len = path.len();
        for (i, (c, r)) in path.iter().enumerate() { g.snake[i] = Cell { c: *c, r: *r }; }
        assert!(!g.shed(), "the shed put skin in the head's next steps");
        assert_eq!(g.skin_len, 0);
    }

    #[test]
    fn the_strike_halves_the_step_and_doubles_the_egg() {
        let mut g = running();
        g.sess.level = 1;
        let (glide, strike) = (g.step_interval(false), g.step_interval(true));
        assert!(strike < glide * 0.6, "a strike is barely quicker: {strike} of {glide}");
        assert_eq!(g.egg_points(true), g.egg_points(false) * 2, "a struck egg is not worth double");
        g.sess.level = 3;
        assert_eq!(g.egg_points(false), SCORE_EGG * 3, "an egg is not worth the pit it was found in");
        assert_eq!(g.egg_points(true), SCORE_EGG * 3 * 2);
    }

    #[test]
    fn a_glide_step_quickens_a_pit_at_a_time_and_stops_at_the_floor() {
        let mut g = running();
        g.sess.level = 1;
        let first = g.step_interval(false);
        g.sess.level = 2;
        assert!(g.step_interval(false) < first, "the second pit is no quicker than the first");
        g.sess.level = 50;
        assert_eq!(g.step_interval(false), STEP_MS_MIN, "the speed floor was passed");
        assert_eq!(g.step_interval(true), STEP_MS_MIN * STRIKE_RATE);
    }

    #[test]
    fn a_reversal_is_refused_with_nothing_queued() {
        let mut g = running(); // travelling right
        g.queue_turn(Dir::Left, false);
        assert_eq!(step_dir(&mut g), Dir::Right, "the adder reversed into itself");
    }

    #[test]
    fn the_strike_holds_one_queued_turn_and_the_glide_two() {
        let mut g = running();
        g.queue_turn(Dir::Down, false);
        g.queue_turn(Dir::Left, false);
        assert_eq!(g.turns_len, 2, "a corner gesture at glide speed lost a turn");
        assert_eq!(step_dir(&mut g), Dir::Down);
        assert_eq!(step_dir(&mut g), Dir::Left);

        let mut g = running();
        g.queue_turn(Dir::Down, true);
        g.queue_turn(Dir::Left, true);
        assert_eq!(g.turns_len, 1, "a strike queued a turn it cannot take in time");
        assert_eq!(step_dir(&mut g), Dir::Down);
        assert_eq!(step_dir(&mut g), Dir::Down, "a strike carried out a turn it should have dropped");
    }

    #[test]
    fn a_strike_never_queues_past_a_queue_it_already_has() {
        // Lifting and pressing the throttle between two turns inside one step
        // leaves a glide queue, already two long, standing under the strike's
        // cap of one.
        let mut g = running();
        g.queue_turn(Dir::Down, false);
        g.queue_turn(Dir::Left, false);
        g.queue_turn(Dir::Up, true);
        assert_eq!(g.turns_len, TURN_QUEUE, "a turn was queued past the queue");
        assert_eq!(step_dir(&mut g), Dir::Down);
    }

    #[test]
    fn a_new_life_keeps_the_pit_and_its_skin_and_starts_with_a_clear_queue() {
        let mut g = running();
        big_snake(&mut g, 12);
        assert!(g.shed());
        let skin = g.skin_len;
        g.sess.level = 4;
        g.queue_turn(Dir::Down, false);

        g.reset_snake(); // the life just lost
        assert_eq!(g.turns_len, 0, "a turn from the dead life steers the new one");
        assert_eq!(g.skin_len, skin, "the pit lost the skin it had already gathered");
        assert_eq!(g.sess.level, 4, "a lost life took the pit number with it");

        g.start_game();
        assert_eq!(g.skin_len, 0, "a new game started on a used floor");
        assert_eq!(g.sess.level, 1);
    }
}
