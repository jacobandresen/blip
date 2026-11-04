//! WARLOCK — a dungeon-crawl maze shooter in the spirit of Wizard of Wor.
//! A young wizard descends into a rival sorcerer's tower: move through the
//! corridors, cast bolts down them, and use reveal to expose what the dark
//! hides. Cleansed rune stones keep the wraiths at bay; every chamber holds
//! its own lesson, and the Warlock himself waits at the bottom.

#[cfg(test)]
mod tunes;
#[cfg(not(target_arch = "wasm32"))]
mod bot;
mod levels;

use levels::*;

use blip::input::{
    coin_start, key_held, key_pressed, play_again, BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_D,
    BLIP_KEY_DOWN, BLIP_KEY_F, BLIP_KEY_G, BLIP_KEY_J, BLIP_KEY_K, BLIP_KEY_LEFT, BLIP_KEY_RIGHT,
    BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::{
    lerp, rand_int, web, window_conf, Blip, BlipColor, Fx, GAME_OVER_MIN_WAIT, Jukebox,
    Session, Timer, BLIP_BLACK, BLIP_GRAY, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
};

// ---- layout ---------------------------------------------------------------
const CELL: i32 = 24;
pub const HUD_H: i32 = 28;
const WIN_W: i32 = COLS as i32 * CELL;
const WIN_H: i32 = ROWS as i32 * CELL + HUD_H;

// ---- dungeon palette ------------------------------------------------------
const WALL: BlipColor = BlipColor { r: 0.18, g: 0.11, b: 0.28, a: 1.0 };
const WALL_HI: BlipColor = BlipColor { r: 0.27, g: 0.18, b: 0.40, a: 1.0 };
const FLOOR: BlipColor = BlipColor { r: 0.06, g: 0.04, b: 0.11, a: 1.0 };
const FLOOR_GRID: BlipColor = BlipColor { r: 0.11, g: 0.08, b: 0.18, a: 1.0 };
const TORCH: BlipColor = BlipColor { r: 1.0, g: 0.62, b: 0.22, a: 1.0 };
const WIZ_ROBE: BlipColor = BlipColor { r: 0.20, g: 0.90, b: 0.78, a: 1.0 };
const WIZ_HAT: BlipColor = BlipColor { r: 0.10, g: 0.34, b: 0.42, a: 1.0 };
const WIZ_ROBE2: BlipColor = BlipColor { r: 0.98, g: 0.55, b: 0.25, a: 1.0 };
const WIZ_HAT2: BlipColor = BlipColor { r: 0.42, g: 0.22, b: 0.12, a: 1.0 };
const BOLT: BlipColor = BlipColor { r: 1.0, g: 0.90, b: 0.45, a: 1.0 };
const REVEAL_C: BlipColor = BlipColor { r: 0.62, g: 0.90, b: 1.0, a: 1.0 };
const IMP: BlipColor = BlipColor { r: 1.0, g: 0.32, b: 0.22, a: 1.0 };
const WRAITH: BlipColor = BlipColor { r: 0.66, g: 0.78, b: 1.0, a: 1.0 };
const HOUND: BlipColor = BlipColor { r: 1.0, g: 0.70, b: 0.30, a: 1.0 };
const GARGOYLE: BlipColor = BlipColor { r: 0.42, g: 0.82, b: 0.44, a: 1.0 };
const WARLOCK_C: BlipColor = BlipColor { r: 0.82, g: 0.34, b: 0.94, a: 1.0 };
const RUNE_CURSED: BlipColor = BlipColor { r: 0.55, g: 0.20, b: 0.65, a: 1.0 };
const RUNE_CLEAN: BlipColor = BlipColor { r: 1.0, g: 0.85, b: 0.35, a: 1.0 };
const GATE_C: BlipColor = BlipColor { r: 0.40, g: 0.85, b: 1.0, a: 1.0 };
const EXIT_C: BlipColor = BlipColor { r: 0.5, g: 1.0, b: 0.6, a: 1.0 };
const FIREBALL: BlipColor = BlipColor { r: 1.0, g: 0.45, b: 0.15, a: 1.0 };
const PROTECT: BlipColor = BlipColor { r: 0.9, g: 0.95, b: 1.0, a: 1.0 };

// ---- directions -----------------------------------------------------------
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}
impl Dir {
    fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }
}
const DIRS: [Dir; 4] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];

/// A board cell: `c` column, `r` row, both 0-based.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Cell {
    pub c: i32,
    pub r: i32,
}

/// The solid/floor bitmap, indexed `r * COLS + c`.
#[derive(Clone)]
pub struct Walls {
    bits: [bool; COLS * ROWS],
}
impl Default for Walls {
    fn default() -> Self {
        Self { bits: [false; COLS * ROWS] }
    }
}
impl Walls {
    #[inline]
    pub fn in_bounds(c: i32, r: i32) -> bool {
        c >= 0 && r >= 0 && c < COLS as i32 && r < ROWS as i32
    }
    #[inline]
    pub fn set(&mut self, c: i32, r: i32, wall: bool) {
        if Self::in_bounds(c, r) {
            self.bits[(r * COLS as i32 + c) as usize] = wall;
        }
    }
    #[inline]
    pub fn get(&self, c: i32, r: i32) -> bool {
        Self::in_bounds(c, r) && self.bits[(r * COLS as i32 + c) as usize]
    }
    #[inline]
    pub fn floor(&self, c: i32, r: i32) -> bool {
        Self::in_bounds(c, r) && !self.bits[(r * COLS as i32 + c) as usize]
    }
}

/// The pixel centre of a cell.
fn center(c: Cell) -> (f32, f32) {
    ((c.c * CELL + CELL / 2) as f32, (HUD_H + c.r * CELL + CELL / 2) as f32)
}

// ---- entities ---------------------------------------------------------------
#[derive(Copy, Clone, PartialEq, Eq)]
enum EnemyKind {
    Imp,
    Wraith,
    Spellhound,
    Gargoyle,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Proj {
    Bolt,
    Fireball,
    Warlock,
}

#[derive(Copy, Clone)]
struct Enemy {
    kind: EnemyKind,
    tile: Cell,
    from: Cell,
    lerp: f32,
    step_t: f32,
    dir: Dir,
    visible: bool,
    revealed: f32,
    flash: f32,
    hunting: bool,
    target: Cell,
    lunging: bool,
    windup: f32,
    charge: f32,
    shot_cd: f32,
    gone: bool,
}

struct Projectile {
    proj: Proj,
    tile: Cell,
    from: Cell,
    lerp: f32,
    dir: Dir,
    gone: bool,
}

struct Rune {
    tile: Cell,
    cleansed: bool,
}

struct Sighting {
    tile: Cell,
    t: f32,
}

#[derive(Copy, Clone)]
struct Gate {
    a: Cell,
    b: Cell,
    cooldown: f32,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum WState {
    Materializing,
    Aiming,
    Casting,
    Pausing,
}

struct Warlock {
    circles: Vec<Cell>,
    sealed: Vec<bool>,
    circle: usize,
    wards: i32,
    state: WState,
    timer: f32,
    aim_dir: Dir,
    aim_from: Cell,
}

struct Wizard {
    tile: Cell,
    from: Cell,
    lerp: f32,
    facing: Dir,
    moving: bool,
    step_t: f32,
    protect: f32,
    bolt_cd: f32,
    reveal_cd: f32,
    reveal_t: f32,
}

impl Wizard {
    fn at(tile: Cell) -> Self {
        Wizard {
            tile,
            from: tile,
            lerp: 1.0,
            facing: Dir::Right,
            moving: false,
            step_t: 0.0,
            protect: 0.0,
            bolt_cd: 0.0,
            reveal_cd: 0.0,
            reveal_t: 0.0,
        }
    }
}

struct Bonus {
    stones: Vec<Cell>,
    seq: Vec<usize>,
    expect: usize,
    showing: bool,
    show_i: usize,
    show_t: f32,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum State {
    Title,
    Play,
    Dead,
    Over,
    Bonus,
}

enum Sfx {
    Bolt,
    Reveal,
    Cleanse,
    Kill,
    Death,
    Over,
    Chamber,
    Ward,
    Warlock,
}

struct Sounds {
    bolt: blip::BlipSound,
    reveal: blip::BlipSound,
    cleanse: blip::BlipSound,
    kill: blip::BlipSound,
    death: blip::BlipSound,
    over: blip::BlipSound,
    chamber: blip::BlipSound,
    ward: blip::BlipSound,
    warlock: blip::BlipSound,
}

struct Game {
    state: State,
    sess: Session,
    walls: Walls,
    player_spawn: Cell,
    exit: Cell,
    exit_open: bool,
    runes: Vec<Rune>,
    enemies: Vec<Enemy>,
    bolts: Vec<Projectile>,
    fires: Vec<Projectile>,
    sightings: Vec<Sighting>,
    gates: Vec<Gate>,
    warlock: Option<Warlock>,
    torches: Vec<Cell>,
    taunt: Option<(String, f32)>,
    banner: Option<(String, f32)>,
    wizards: [Wizard; 2],
    active: [bool; 2],
    lives: [i32; 2],
    two_player: bool,
    dead_index: Option<usize>,
    bonus: Option<Bonus>,
    fx: Fx,
    dead_timer: Timer,
    died_this_room: bool,
    cleared_no_death: i32,
    sfx: Option<Sounds>,
}

impl Game {
    fn new() -> Self {
        Self {
            state: State::Title,
            sess: Session::new(LIVES_START),
            walls: Walls::default(),
            player_spawn: Cell { c: 1, r: 1 },
            exit: Cell { c: 1, r: 1 },
            exit_open: false,
            runes: Vec::new(),
            enemies: Vec::new(),
            bolts: Vec::new(),
            fires: Vec::new(),
            sightings: Vec::new(),
            gates: Vec::new(),
            warlock: None,
            torches: Vec::new(),
            taunt: None,
            banner: None,
            wizards: [Wizard::at(Cell { c: 1, r: 1 }), Wizard::at(Cell { c: 1, r: 1 })],
            active: [true, false],
            lives: [LIVES_START, LIVES_START],
            two_player: false,
            dead_index: None,
            bonus: None,
            fx: Fx::new(),
            dead_timer: Timer::default(),
            died_this_room: false,
            cleared_no_death: 0,
            sfx: None,
        }
    }

    fn sfx(&self, s: Sfx) {
        let Some(sounds) = &self.sfx else { return };
        use Sfx::*;
        let target = match s {
            Bolt => &sounds.bolt,
            Reveal => &sounds.reveal,
            Cleanse => &sounds.cleanse,
            Kill => &sounds.kill,
            Death => &sounds.death,
            Over => &sounds.over,
            Chamber => &sounds.chamber,
            Ward => &sounds.ward,
            Warlock => &sounds.warlock,
        };
        blip::play_sfx(target);
    }

    fn track(&self) -> usize {
        if matches!(self.state, State::Play | State::Dead) && self.warlock.is_some() {
            1
        } else {
            0
        }
    }

    // ---- room loading ----------------------------------------------------

    fn load_room(&mut self) {
        let room = room_for(self.sess.level);
        self.walls = walls_for(room);
        self.player_spawn = Cell { c: room.player.0, r: room.player.1 };
        self.exit = Cell { c: room.exit.0, r: room.exit.1 };
        self.exit_open = false;
        self.runes = room
            .runes
            .iter()
            .map(|&(c, r)| Rune { tile: Cell { c, r }, cleansed: false })
            .collect();

        let mk = |c: i32, r: i32, kind: EnemyKind, dir: Dir| Enemy {
            kind,
            tile: Cell { c, r },
            from: Cell { c, r },
            lerp: 1.0,
            step_t: rand_int(0, 50) as f32 / 100.0,
            dir,
            visible: kind != EnemyKind::Wraith,
            revealed: 0.0,
            flash: rand_int(0, 60) as f32 / 60.0,
            hunting: false,
            target: Cell { c, r },
            lunging: false,
            windup: 0.0,
            charge: 0.0,
            shot_cd: if kind == EnemyKind::Gargoyle { rand_int(0, 150) as f32 / 100.0 } else { 0.0 },
            gone: false,
        };

        let mut enemies = Vec::new();
        for &(c, r) in room.imps {
            enemies.push(mk(c, r, EnemyKind::Imp, Dir::Right));
        }
        for &(c, r) in room.wraiths {
            enemies.push(mk(c, r, EnemyKind::Wraith, Dir::Left));
        }
        for &(c, r) in room.hounds {
            enemies.push(mk(c, r, EnemyKind::Spellhound, Dir::Up));
        }
        for &(c, r) in room.gargoyles {
            enemies.push(mk(c, r, EnemyKind::Gargoyle, Dir::Down));
        }
        // Later wings get extra imps to keep the corridors busy.
        let extra = (self.sess.level - 6).max(0) as usize;
        for _ in 0..extra {
            if let Some((c, r)) = self.random_open_cell() {
                enemies.push(mk(c, r, EnemyKind::Imp, Dir::Right));
            }
        }
        self.enemies = enemies;

        self.gates = room
            .gates
            .iter()
            .map(|&((ac, ar), (bc, br))| Gate {
                a: Cell { c: ac, r: ar },
                b: Cell { c: bc, r: br },
                cooldown: 0.0,
            })
            .collect();

        self.warlock = if room.warlock {
            let circles: Vec<Cell> = room.warlock_circles.iter().map(|&(c, r)| Cell { c, r }).collect();
            Some(Warlock {
                sealed: vec![false; circles.len()],
                circle: 0,
                wards: room.warlock_wards,
                circles,
                state: WState::Materializing,
                timer: 0.7,
                aim_dir: Dir::Right,
                aim_from: Cell { c: 0, r: 0 },
            })
        } else {
            None
        };

        self.bolts.clear();
        self.fires.clear();
        self.sightings.clear();
        self.taunt = Some((room_taunt(self.sess.level), 3.0));
        self.died_this_room = false;
        self.place_wizard(0, self.player_spawn);
        if self.active[1] {
            self.place_wizard(1, self.second_spawn());
        }
        self.torches = self.find_torches();
        self.bonus = None;
        blip::bot::set("level", self.sess.level as f64);
    }

    fn random_open_cell(&self) -> Option<(i32, i32)> {
        random_open_cell(&self.walls, self.player_spawn, self.exit)
    }

    fn find_torches(&self) -> Vec<Cell> {
        let mut out = Vec::new();
        for r in 1..ROWS as i32 - 1 {
            for c in 1..COLS as i32 - 1 {
                if self.walls.get(c, r) {
                    let n = [(-1, 0), (1, 0), (0, -1), (0, 1)]
                        .iter()
                        .filter(|&&(dc, dr)| self.walls.floor(c + dc, r + dr))
                        .count();
                    if n >= 2 && (c * 7 + r * 13) % 4 == 0 {
                        out.push(Cell { c, r });
                    }
                }
            }
        }
        out
    }

    fn place_wizard(&mut self, idx: usize, tile: Cell) {
        let w = &mut self.wizards[idx];
        w.tile = tile;
        w.from = tile;
        w.lerp = 1.0;
        w.moving = false;
        w.step_t = 0.0;
        w.protect = 0.0;
        w.bolt_cd = 0.0;
        w.reveal_cd = 0.0;
        w.reveal_t = 0.0;
    }

    /// A free floor tile near the entrance for the second wizard.
    fn second_spawn(&self) -> Cell {
        const OFF: [(i32, i32); 8] = [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)];
        for &(dc, dr) in &OFF {
            let c = Cell { c: self.player_spawn.c + dc, r: self.player_spawn.r + dr };
            if self.walls.floor(c.c, c.r) && !self.enemy_occupied(c) {
                return c;
            }
        }
        self.player_spawn
    }

    fn start_game(&mut self) {
        self.sess.reset(LIVES_START);
        self.lives = [LIVES_START, LIVES_START];
        self.active = [true, false];
        self.two_player = false;
        self.dead_index = None;
        self.cleared_no_death = 0;
        self.load_room();
        self.state = State::Play;
        self.fx.clear();
    }

    /// J (the second station's button) joins a second wizard in co-op, lights
    /// the kiosk's second station and charges the extra credit.
    fn try_join(&mut self) {
        if self.two_player || !key_pressed(BLIP_KEY_J) {
            return;
        }
        self.two_player = true;
        self.active[1] = true;
        self.lives[1] = LIVES_START;
        self.place_wizard(1, self.second_spawn());
        blip::bot::add("join", 1.0);
        web::spend_coin();
        web::set_mode(true);
    }

    // ---- queries (all read-only, no `&mut self`) --------------------------

    fn wizard_px(&self, idx: usize) -> (f32, f32) {
        let w = &self.wizards[idx];
        let a = center(w.from);
        let b = center(w.tile);
        let t = w.lerp;
        (lerp(a.0, b.0, t), lerp(a.1, b.1, t))
    }

    /// Is an active wizard standing on `tile` (other than the one moving)?
    fn wizard_occupies(&self, tile: Cell, exclude: usize) -> bool {
        (0..2).any(|i| i != exclude && self.active[i] && self.wizards[i].tile == tile)
    }

    fn enemy_px(&self, e: &Enemy) -> (f32, f32) {
        let a = center(e.from);
        let b = center(e.tile);
        let t = e.lerp;
        (lerp(a.0, b.0, t), lerp(a.1, b.1, t))
    }

    fn enemy_occupied(&self, c: Cell) -> bool {
        self.enemies.iter().any(|e| !e.gone && e.tile == c)
    }

    fn gate_index(&self, c: Cell) -> Option<usize> {
        self.gates.iter().position(|g| (g.a == c || g.b == c) && g.cooldown <= 0.0)
    }

    fn step_blocked(&self, next: Cell, idx: usize) -> bool {
        !self.walls.floor(next.c, next.r)
            || self.gates.iter().any(|g| g.cooldown > 0.0 && (g.a == next || g.b == next))
            || self.enemy_occupied(next)
            || self.wizard_occupies(next, idx)
            || self.warlock.as_ref().map_or(false, |wl| {
                wl.wards > 0 && wl.state != WState::Materializing && wl.circles[wl.circle] == next
            })
    }

    // ---- input ------------------------------------------------------------

    fn held_dir(&self, idx: usize) -> Option<Dir> {
        // Player one uses WASD; alone, the arrows are theirs too. Player two
        // uses the arrows (the deck's second station).
        let (w_up, w_down, w_left, w_right) = if idx == 0 {
            let arrows = !self.two_player;
            (
                key_held(BLIP_KEY_W) || (arrows && key_held(BLIP_KEY_UP)),
                key_held(BLIP_KEY_S) || (arrows && key_held(BLIP_KEY_DOWN)),
                key_held(BLIP_KEY_A) || (arrows && key_held(BLIP_KEY_LEFT)),
                key_held(BLIP_KEY_D) || (arrows && key_held(BLIP_KEY_RIGHT)),
            )
        } else {
            (
                key_held(BLIP_KEY_UP),
                key_held(BLIP_KEY_DOWN),
                key_held(BLIP_KEY_LEFT),
                key_held(BLIP_KEY_RIGHT),
            )
        };
        if w_up {
            Some(Dir::Up)
        } else if w_down {
            Some(Dir::Down)
        } else if w_left {
            Some(Dir::Left)
        } else if w_right {
            Some(Dir::Right)
        } else {
            None
        }
    }

    fn bolt_pressed(&self, idx: usize) -> bool {
        if idx == 0 {
            key_pressed(BLIP_KEY_F) || key_pressed(BLIP_KEY_SPACE)
        } else {
            key_pressed(BLIP_KEY_J)
        }
    }
    fn bolt_held(&self, idx: usize) -> bool {
        if idx == 0 {
            key_held(BLIP_KEY_F) || key_held(BLIP_KEY_SPACE)
        } else {
            key_held(BLIP_KEY_J)
        }
    }
    fn reveal_pressed(&self, idx: usize) -> bool {
        if idx == 0 {
            key_pressed(BLIP_KEY_G) || key_pressed(BLIP_KEY_BUTTON2)
        } else {
            key_pressed(BLIP_KEY_K)
        }
    }

    // ---- play update ------------------------------------------------------

    fn update_play(&mut self, dt: f32) {
        self.fx.update(dt);
        for i in 0..2 {
            if self.active[i] {
                self.update_wizard(i, dt);
            }
        }
        self.update_enemies(dt);
        self.check_contact();
        self.update_projectiles(dt);
        self.update_warlock(dt);
        if self.warlock.as_ref().is_some_and(|w| w.wards <= 0) {
            self.warlock = None;
        }
        for s in &mut self.sightings {
            s.t += dt;
        }
        self.sightings.retain(|s| s.t < SIGHTING_FADE);
        for g in &mut self.gates {
            g.cooldown = (g.cooldown - dt).max(0.0);
        }
        if let Some((_, t)) = &mut self.taunt {
            *t -= dt;
        }
        if let Some((_, t)) = &mut self.banner {
            *t -= dt;
        }
        for i in 0..2 {
            let w = &mut self.wizards[i];
            w.protect = (w.protect - dt).max(0.0);
            w.bolt_cd = (w.bolt_cd - dt).max(0.0);
            w.reveal_cd = (w.reveal_cd - dt).max(0.0);
            if w.reveal_t > 0.0 {
                w.reveal_t = (w.reveal_t - dt).max(0.0);
            }
        }
    }

    fn update_wizard(&mut self, idx: usize, dt: f32) {
        let held = self.held_dir(idx);
        let exit_open = self.exit_open;
        let exit = self.exit;

        let (cur, moving, step_t, lerp, facing, from) = {
            let w = &self.wizards[idx];
            (w.tile, w.moving, w.step_t, w.lerp, w.facing, w.from)
        };

        let mut reach_exit = false;
        let mut reach_gate = None;

        let next = if moving {
            let ns = step_t - dt;
            if ns <= 0.0 {
                if exit_open && cur == exit {
                    reach_exit = true;
                } else if let Some(gi) = self.gate_index(cur) {
                    reach_gate = Some(gi);
                }
                Some((cur, cur, 0.0, 1.0, false, facing))
            } else {
                Some((cur, from, ns, (lerp + dt / MOVE_SECS).min(1.0), true, facing))
            }
        } else if let Some(dir) = held {
            let (dc, dr) = dir.delta();
            let n = Cell { c: cur.c + dc, r: cur.r + dr };
            if self.walls.floor(n.c, n.r) && !self.step_blocked(n, idx) {
                Some((n, cur, MOVE_SECS, 0.0, true, dir))
            } else {
                Some((cur, cur, 0.0, 1.0, false, dir))
            }
        } else {
            Some((cur, cur, 0.0, 1.0, false, facing))
        };

        if let Some((tile, from, step_t, lerp, moving, facing)) = next {
            let w = &mut self.wizards[idx];
            w.tile = tile;
            w.from = from;
            w.step_t = step_t;
            w.lerp = lerp;
            w.moving = moving;
            w.facing = facing;
        }

        if reach_exit {
            self.finish_chamber();
        } else if let Some(gi) = reach_gate {
            self.warp_through_gate(idx, gi);
        }
    }

    fn warp_through_gate(&mut self, idx: usize, gi: usize) {
        let gate = self.gates[gi];
        let dest = if self.wizards[idx].tile == gate.a { gate.b } else { gate.a };
        if self.enemy_occupied(dest)
            || self.wizard_occupies(dest, idx)
            || self.warlock.as_ref().map_or(false, |wl| wl.wards > 0 && wl.circles[wl.circle] == dest)
        {
            return;
        }
        let (x, y) = self.wizard_px(idx);
        self.fx.ring(x, y, CELL as f32 * 0.9, 0.35, GATE_C);
        self.gates[gi].cooldown = GATE_CD;
        self.place_wizard(idx, dest);
        let (x, y) = center(dest);
        self.fx.ring(x, y, CELL as f32 * 0.9, 0.35, GATE_C);
        blip::bot::add("gate", 1.0);
    }

    fn reveal(&mut self, idx: usize) {
        if self.wizards[idx].reveal_cd > 0.0 {
            return;
        }
        self.wizards[idx].reveal_cd = REVEAL_CD;
        self.wizards[idx].reveal_t = 0.55;
        blip::bot::add("reveal", 1.0);
        let (x, y) = self.wizard_px(idx);
        self.fx.ring(x, y, CELL as f32 * 6.0, 0.5, REVEAL_C);
        // expose wraiths, leave a last-seen outline
        let mut newly = Vec::new();
        for e in &mut self.enemies {
            if e.kind == EnemyKind::Wraith && !e.gone {
                e.revealed = REVEAL_SECS;
                newly.push(e.tile);
            }
        }
        for tile in newly {
            self.sightings.push(Sighting { tile, t: 0.0 });
        }
        // spellhounds race to where the spell was cast
        let cast_point = self.wizards[idx].tile;
        for e in &mut self.enemies {
            if e.kind == EnemyKind::Spellhound && !e.gone {
                e.hunting = true;
                e.target = cast_point;
            }
        }
        // cleanse a stone beside this wizard
        let mut cleansed = false;
        for r in &mut self.runes {
            if !r.cleansed && distance_lt(self.wizards[idx].tile, r.tile, 1.5) {
                r.cleansed = true;
                cleansed = true;
                self.sess.add_score(SCORE_RUNE);
                let (rx, ry) = center(r.tile);
                self.fx.ring(rx, ry, CELL as f32 * 1.4, 0.4, RUNE_CLEAN);
                self.fx.popup(rx, ry - 6.0, &format!("+{SCORE_RUNE}"), RUNE_CLEAN);
                blip::bot::add("cleanse", 1.0);
            }
        }
        if cleansed {
            self.sfx(Sfx::Cleanse);
        } else {
            self.sfx(Sfx::Reveal);
        }
        self.check_exit();
    }

    fn cast_bolt(&mut self, idx: usize) {
        let live = self.bolts.iter().filter(|b| !b.gone).count();
        if live >= BOLT_MAX {
            return;
        }
        self.wizards[idx].bolt_cd = BOLT_RATE;
        let tile = self.wizards[idx].tile;
        self.bolts.push(Projectile {
            proj: Proj::Bolt,
            tile,
            from: tile,
            lerp: 0.0,
            dir: self.wizards[idx].facing,
            gone: false,
        });
        blip::bot::add("bolt", 1.0);
        self.sfx(Sfx::Bolt);
    }

    fn update_enemies(&mut self, dt: f32) {
        // visibility bookkeeping
        for i in 0..self.enemies.len() {
            let e = &mut self.enemies[i];
            e.revealed = (e.revealed - dt).max(0.0);
            if e.kind == EnemyKind::Wraith {
                e.flash += dt;
                let period = WRAITH_FLASH_ON + WRAITH_FLASH_OFF;
                if e.flash > period {
                    e.flash -= period;
                }
                let near = (0..2).any(|k| self.active[k] && distance_lt(e.tile, self.wizards[k].tile, WRAITH_LUNGE_RANGE));
                // A fresh lunge first materializes: visible, but still, for a
                // beat — so the reveal is a warning, not an instant pounce.
                if near && !e.lunging {
                    e.charge = WRAITH_CHARGE;
                }
                e.lunging = near;
                e.charge = (e.charge - dt).max(0.0);
                e.visible = e.revealed > 0.0
                    || near
                    || e.flash < WRAITH_FLASH_ON
                    || self.runes.iter().any(|r| r.cleansed && distance_lt(e.tile, r.tile, RUNE_LIGHT));
            }
        }
        // movement, decided against a snapshot of occupied tiles
        let occupied: Vec<Cell> = self.enemies.iter().filter(|e| !e.gone).map(|e| e.tile).collect();
        for i in 0..self.enemies.len() {
            let e = self.enemies[i];
            if e.gone || e.kind == EnemyKind::Gargoyle {
                continue;
            }
            if e.kind == EnemyKind::Wraith && e.charge > 0.0 {
                continue; // materializing: visible, but not yet moving
            }
            self.enemies[i].step_t -= dt;
            if self.enemies[i].step_t > 0.0 {
                continue;
            }
            let interval = enemy_step(&e);
            let decision = if e.lunging {
                let target = (0..2)
                    .filter(|&k| self.active[k])
                    .map(|k| self.wizards[k].tile)
                    .min_by_key(|t| (t.c - e.tile.c).abs() + (t.r - e.tile.r).abs())
                    .unwrap_or(e.tile);
                bfs_next(&self.walls, &occupied, e.tile, target)
            } else if e.hunting {
                bfs_next(&self.walls, &occupied, e.tile, e.target)
            } else {
                decide_patrol(&self.walls, &occupied, &e)
            };
            if let Some((next, dir)) = decision {
                self.enemies[i].from = e.tile;
                self.enemies[i].tile = next;
                self.enemies[i].dir = dir;
                self.enemies[i].lerp = 0.0;
                self.enemies[i].step_t = interval;
            } else {
                self.enemies[i].step_t = interval;
            }
        }
        // gargoyles: raise wings, then cast
        for i in 0..self.enemies.len() {
            let e = self.enemies[i];
            if e.gone || e.kind != EnemyKind::Gargoyle {
                continue;
            }
            if e.windup > 0.0 {
                self.enemies[i].windup -= dt;
                if self.enemies[i].windup <= 0.0 {
                    let dir = pick_open_dir(&self.walls, e.tile);
                    self.fires.push(Projectile {
                        proj: Proj::Fireball,
                        tile: e.tile,
                        from: e.tile,
                        lerp: 0.0,
                        dir,
                        gone: false,
                    });
                    self.enemies[i].shot_cd = GARGOYLE_CD;
                    let (x, y) = center(e.tile);
                    self.fx.burst(x, y, 6, 60.0, FIREBALL);
                }
            } else {
                self.enemies[i].shot_cd -= dt;
                if self.enemies[i].shot_cd <= 0.0 {
                    self.enemies[i].windup = GARGOYLE_TELL;
                }
            }
        }
        // advance render lerp
        for e in &mut self.enemies {
            e.lerp = (e.lerp + dt).min(1.0);
        }
    }

    fn update_projectiles(&mut self, dt: f32) {
        let mut kills = Vec::new();
        let mut warlock_hit = false;
        for i in 0..self.bolts.len() {
            let b = &mut self.bolts[i];
            if b.gone {
                continue;
            }
            b.lerp += dt / BOLT_INTERVAL;
            if b.lerp < 1.0 {
                continue;
            }
            b.from = b.tile;
            let (dc, dr) = b.dir.delta();
            let next = Cell { c: b.tile.c + dc, r: b.tile.r + dr };
            if self.walls.get(next.c, next.r) {
                b.gone = true;
                continue;
            }
            b.tile = next;
            b.lerp = 0.0;
            if let Some(idx) = self.enemies.iter().position(|e| !e.gone && e.tile == b.tile) {
                kills.push(idx);
                b.gone = true;
                continue;
            }
            if let Some(wl) = &self.warlock {
                if wl.wards > 0 && wl.state == WState::Pausing && wl.circles[wl.circle] == b.tile {
                    warlock_hit = true;
                    b.gone = true;
                }
            }
        }
        self.bolts.retain(|b| !b.gone);
        for idx in kills {
            self.kill_enemy(idx);
        }
        if warlock_hit {
            self.hit_warlock();
        }

        let mut hit_idx = None;
        for i in 0..self.fires.len() {
            let f = &mut self.fires[i];
            if f.gone {
                continue;
            }
            let interval = GARGOYLE_SHOT_STEP;
            f.lerp += dt / interval;
            if f.lerp < 1.0 {
                continue;
            }
            f.from = f.tile;
            let (dc, dr) = f.dir.delta();
            let next = Cell { c: f.tile.c + dc, r: f.tile.r + dr };
            if self.walls.get(next.c, next.r) {
                f.gone = true;
                continue;
            }
            f.tile = next;
            f.lerp = 0.0;
            for k in 0..2 {
                if self.active[k] && f.tile == self.wizards[k].tile {
                    hit_idx = Some(k);
                    f.gone = true;
                    break;
                }
            }
        }
        self.fires.retain(|p| !p.gone);
        if let Some(k) = hit_idx {
            self.hit_wizard(k);
        }
    }

    fn kill_enemy(&mut self, idx: usize) {
        let e = self.enemies[idx];
        if e.gone {
            return;
        }
        self.enemies[idx].gone = true;
        let pts = match e.kind {
            EnemyKind::Imp => SCORE_IMP,
            EnemyKind::Wraith => SCORE_WRAITH,
            EnemyKind::Spellhound => SCORE_HOUND,
            EnemyKind::Gargoyle => SCORE_GARGOYLE,
        };
        self.sess.add_score(pts);
        let (x, y) = center(e.tile);
        self.fx.burst(x, y, 14, 130.0, WRAITH);
        self.fx.popup(x, y - 6.0, &format!("+{pts}"), BLIP_WHITE);
        blip::bot::add("kill", 1.0);
        self.sfx(Sfx::Kill);
        self.check_exit();
    }

    fn update_warlock(&mut self, dt: f32) {
        let mut fire: Option<(Cell, Dir)> = None;
        let mut advance = false;
        {
            let Some(wl) = &mut self.warlock else { return };
            if wl.wards <= 0 {
                return;
            }
            wl.timer -= dt;
            match wl.state {
                WState::Materializing => {
                    if wl.timer <= 0.0 {
                        wl.state = WState::Aiming;
                        wl.timer = 0.95;
                        wl.aim_dir = pick_open_dir(&self.walls, wl.circles[wl.circle]);
                        wl.aim_from = wl.circles[wl.circle];
                    }
                }
                WState::Aiming => {
                    if wl.timer <= 0.0 {
                        wl.state = WState::Casting;
                        wl.timer = 0.12;
                    }
                }
                WState::Casting => {
                    if wl.timer <= 0.0 {
                        fire = Some((wl.aim_from, wl.aim_dir));
                        wl.state = WState::Pausing;
                        wl.timer = 1.1;
                    }
                }
                WState::Pausing => {
                    if wl.timer <= 0.0 {
                        advance = true;
                    }
                }
            }
        }
        if let Some((from, dir)) = fire {
            self.fires.push(Projectile {
                proj: Proj::Warlock,
                tile: from,
                from,
                lerp: 0.0,
                dir,
                gone: false,
            });
            self.sfx(Sfx::Warlock);
        }
        if advance {
            self.warlock_advance();
        }
    }

    fn warlock_advance(&mut self) {
        let Some(wl) = &mut self.warlock else { return };
        for _ in 0..wl.circles.len() {
            wl.circle = (wl.circle + 1) % wl.circles.len();
            if !wl.sealed[wl.circle] {
                break;
            }
        }
        wl.state = WState::Materializing;
        wl.timer = 0.7;
    }

    fn hit_warlock(&mut self) {
        let Some(wl) = &mut self.warlock else { return };
        if wl.wards <= 0 {
            return;
        }
        wl.wards -= 1;
        wl.sealed[wl.circle] = true;
        let tile = wl.circles[wl.circle];
        let wards_left = wl.wards;
        // `wl` is no longer used past here, so the borrow ends and the rest
        // of the method is free to touch other parts of `self`.
        self.sess.add_score(SCORE_WARD);
        let (x, y) = center(tile);
        self.fx.burst(x, y, 18, 160.0, WARLOCK_C);
        self.fx.popup(x, y - 8.0, &format!("+{SCORE_WARD}"), WARLOCK_C);
        blip::bot::add("ward", 1.0);
        self.sfx(Sfx::Ward);
        if wards_left > 0 {
            self.warlock_advance();
        } else {
            self.check_exit();
        }
    }

    /// A monster only hurts a wizard once its body has actually reached her:
    /// tile bookkeeping alone would let a ghost kill from a whole cell away,
    /// so this measures the real, interpolated positions of both. Runs every
    /// frame, after wizards and enemies have moved.
    fn check_contact(&mut self) {
        let r = CELL as f32 * 0.45;
        let r2 = r * r;
        let mut victim = None;
        for e in self.enemies.iter().filter(|e| !e.gone) {
            let (ex, ey) = self.enemy_px(e);
            for k in 0..2 {
                if !self.active[k] {
                    continue;
                }
                let (wx, wy) = self.wizard_px(k);
                let dx = ex - wx;
                let dy = ey - wy;
                if dx * dx + dy * dy <= r2 {
                    victim = Some(k);
                    break;
                }
            }
            if victim.is_some() {
                break;
            }
        }
        if let Some(k) = victim {
            self.hit_wizard(k);
        }
    }

    fn hit_wizard(&mut self, idx: usize) {
        if !self.active[idx] || self.wizards[idx].protect > 0.0 || self.state != State::Play {
            return;
        }
        let (x, y) = self.wizard_px(idx);
        self.fx.burst(x, y, 16, 150.0, BLIP_RED);
        blip::bot::add("death", 1.0);
        self.sfx(Sfx::Death);
        self.died_this_room = true;
        self.lives[idx] -= 1;
        if self.lives[idx] > 0 {
            self.dead_index = Some(idx);
            self.dead_timer.start(RESPAWN_PROTECT);
            self.state = State::Dead;
        } else {
            self.active[idx] = false; // downed: out until a partner clears the chamber
            if self.active.iter().all(|&a| !a) {
                self.dead_timer.start(GAME_OVER_MIN_WAIT);
                self.state = State::Over;
                self.sfx(Sfx::Over);
                if !self.two_player {
                    web::report_score(self.sess.score);
                }
            }
        }
    }

    fn respawn(&mut self) {
        let idx = self.dead_index.take().unwrap_or(0);
        let spawn = if idx == 0 { self.player_spawn } else { self.second_spawn() };
        self.place_wizard(idx, spawn);
        self.active[idx] = true;
        self.wizards[idx].protect = RESPAWN_PROTECT;
        self.state = State::Play;
        let (exit, walls) = (self.exit, &self.walls);
        for e in &mut self.enemies {
            if e.gone {
                continue;
            }
            if distance_lt(e.tile, spawn, 2.5) {
                if let Some((c, r)) = random_open_cell(walls, spawn, exit) {
                    e.tile = Cell { c, r };
                    e.from = e.tile;
                    e.lerp = 1.0;
                    e.step_t = rand_int(20, 60) as f32 / 100.0;
                }
            }
        }
    }

    fn check_exit(&mut self) {
        let monsters_alive = self.enemies.iter().any(|e| !e.gone);
        let warlock_alive = self.warlock.as_ref().map_or(false, |wl| wl.wards > 0);
        let runes_left = self.runes.iter().any(|r| !r.cleansed);
        self.exit_open = !monsters_alive && !warlock_alive && !runes_left;
    }

    fn finish_chamber(&mut self) {
        self.cleared_no_death += 1;
        if !self.died_this_room {
            let pts = SCORE_CLEAR_NODEATH;
            self.sess.add_score(pts);
            let (x, y) = center(self.wizards[0].tile);
            self.fx.popup(x, y - 8.0, &format!("+{pts}"), BLIP_YELLOW);
        }
        // A downed partner returns with one life now the chamber is clear.
        if self.two_player {
            for i in 0..2 {
                if !self.active[i] && self.lives[i] <= 0 {
                    self.lives[i] = 1;
                    self.active[i] = true;
                }
            }
        }
        self.sfx(Sfx::Chamber);
        let was_warlock = room_for(self.sess.level).warlock;
        if was_warlock {
            self.state = State::Bonus;
            self.start_bonus();
        } else {
            self.sess.next_level();
            self.load_room();
        }
    }

    // ---- bonus ------------------------------------------------------------

    fn start_bonus(&mut self) {
        self.walls = walls_for(&ROOMS[0]);
        self.enemies.clear();
        self.fires.clear();
        self.bolts.clear();
        self.warlock = None;
        self.sightings.clear();
        self.gates.clear();
        self.runes.clear();
        self.exit_open = false;
        self.torches = self.find_torches();
        let stones: Vec<Cell> = [(3, 1), (11, 1), (3, 11), (11, 11)]
            .iter()
            .map(|&(c, r)| Cell { c, r })
            .collect();
        let len = 5;
        let mut seq = Vec::new();
        for _ in 0..len {
            seq.push(rand_int(0, 3) as usize);
        }
        self.place_wizard(0, self.player_spawn);
        if self.active[1] {
            self.place_wizard(1, self.second_spawn());
        }
        self.bonus = Some(Bonus {
            stones,
            seq,
            expect: 0,
            showing: true,
            show_i: 0,
            show_t: 0.0,
        });
        self.banner = Some(("SPELL CIRCLE".to_string(), 2.0));
        blip::bot::set("wing", self.cleared_no_death as f64);
    }

    fn update_bonus(&mut self, dt: f32) {
        self.fx.update(dt);
        self.update_wizard(0, dt);
        self.wizards[0].reveal_cd = (self.wizards[0].reveal_cd - dt).max(0.0);
        if self.wizards[0].reveal_t > 0.0 {
            self.wizards[0].reveal_t = (self.wizards[0].reveal_t - dt).max(0.0);
        }
        if let Some((_, t)) = &mut self.banner {
            *t -= dt;
        }

        // The showing phase just advances the reveal clock.
        let repeat_state = {
            let Some(b) = &mut self.bonus else { return };
            if b.showing {
                b.show_t += dt;
                if b.show_t > 0.7 {
                    b.show_t = 0.0;
                    b.show_i += 1;
                    if b.show_i >= b.seq.len() {
                        b.showing = false;
                        b.expect = 0;
                    }
                }
                return;
            }
            (b.stones.clone(), b.seq.clone(), b.expect)
        };
        let (stones, seq, expect) = repeat_state;

        if !(self.reveal_pressed(0) && self.wizards[0].reveal_cd <= 0.0) {
            return;
        }
        self.wizards[0].reveal_cd = REVEAL_CD;
        self.wizards[0].reveal_t = 0.45;
        let (x, y) = self.wizard_px(0);
        self.fx.ring(x, y, CELL as f32 * 2.0, 0.4, REVEAL_C);
        let hit = stones.iter().position(|s| distance_lt(self.wizards[0].tile, *s, 1.5));
        match hit {
            Some(k) if k == seq[expect] => {
                let pts = 100 * (expect as i32 + 1);
                self.sess.add_score(pts);
                let (sx, sy) = center(stones[k]);
                self.fx.ring(sx, sy, CELL as f32 * 1.3, 0.4, RUNE_CLEAN);
                self.fx.popup(sx, sy - 6.0, &format!("+{pts}"), RUNE_CLEAN);
                if let Some(b) = &mut self.bonus {
                    b.expect = expect + 1;
                }
                if expect + 1 >= seq.len() {
                    self.leave_bonus();
                    return;
                }
                self.sfx(Sfx::Cleanse);
            }
            _ => {
                self.sfx(Sfx::Death);
                self.leave_bonus();
            }
        }
    }

    fn leave_bonus(&mut self) {
        self.bonus = None;
        self.sess.next_level();
        self.load_room();
    }

    // ---- transitions ------------------------------------------------------

    fn update_dead(&mut self, dt: f32) {
        self.fx.update(dt);
        if self.dead_timer.tick(dt) {
            self.respawn();
        }
    }

    fn check_over_again(&mut self, dt: f32) {
        if play_again(&mut self.dead_timer, dt) {
            self.start_game();
        }
    }
}

// ---- free helpers ----------------------------------------------------------

/// A random open floor cell away from the entrance and the exit.
fn random_open_cell(walls: &Walls, avoid_a: Cell, avoid_b: Cell) -> Option<(i32, i32)> {
    for _ in 0..300 {
        let c = rand_int(1, COLS as i32 - 2);
        let r = rand_int(1, ROWS as i32 - 2);
        if walls.floor(c, r)
            && (c, r) != (avoid_a.c, avoid_a.r)
            && (c, r) != (avoid_b.c, avoid_b.r)
        {
            return Some((c, r));
        }
    }
    None
}

fn distance_lt(a: Cell, b: Cell, tiles: f32) -> bool {
    let dc = (a.c - b.c) as f32;
    let dr = (a.r - b.r) as f32;
    dc * dc + dr * dr < tiles * tiles
}

fn enemy_step(e: &Enemy) -> f32 {
    match e.kind {
        EnemyKind::Imp => IMP_STEP,
        EnemyKind::Wraith => {
            if e.lunging {
                WRAITH_LUNGE_STEP
            } else {
                WRAITH_STEP
            }
        }
        EnemyKind::Spellhound => {
            if e.hunting {
                HOUND_HUNT_STEP
            } else {
                HOUND_WANDER_STEP
            }
        }
        EnemyKind::Gargoyle => f32::INFINITY,
    }
}

fn pick_open_dir(walls: &Walls, tile: Cell) -> Dir {
    for d in DIRS {
        let (dc, dr) = d.delta();
        if walls.floor(tile.c + dc, tile.r + dr) {
            return d;
        }
    }
    Dir::Down
}

fn is_taken(blocked: &[Cell], c: Cell) -> bool {
    blocked.contains(&c)
}

fn bfs_next(walls: &Walls, blocked: &[Cell], from: Cell, to: Cell) -> Option<(Cell, Dir)> {
    use std::collections::VecDeque;
    if from == to {
        return None;
    }
    let mut seen = [[false; ROWS]; COLS];
    let mut q = VecDeque::new();
    for d in DIRS {
        let (dc, dr) = d.delta();
        let n = Cell { c: from.c + dc, r: from.r + dr };
        if walls.floor(n.c, n.r) && !is_taken(blocked, n) {
            seen[n.c as usize][n.r as usize] = true;
            q.push_back((n, d));
        }
    }
    while let Some((cell, first)) = q.pop_front() {
        if cell == to {
            return Some((cell, first));
        }
        for d in DIRS {
            let (dc, dr) = d.delta();
            let n = Cell { c: cell.c + dc, r: cell.r + dr };
            if walls.floor(n.c, n.r) && !seen[n.c as usize][n.r as usize] && !is_taken(blocked, n) {
                seen[n.c as usize][n.r as usize] = true;
                q.push_back((n, first));
            }
        }
    }
    None
}

/// Pick the next step for a patrolling enemy: prefer straight, else a random
/// open corridor not already occupied.
fn decide_patrol(walls: &Walls, blocked: &[Cell], e: &Enemy) -> Option<(Cell, Dir)> {
    let mut candidates: Vec<(Cell, Dir)> = Vec::new();
    for d in DIRS {
        let (dc, dr) = d.delta();
        let n = Cell { c: e.tile.c + dc, r: e.tile.r + dr };
        if walls.floor(n.c, n.r) && !is_taken(blocked, n) {
            candidates.push((n, d));
        }
    }
    if candidates.is_empty() {
        return None;
    }
    if let Some(s) = candidates.iter().find(|(.., d)| *d == e.dir) {
        return Some(*s);
    }
    Some(candidates[rand_int(0, candidates.len() as i32 - 1) as usize])
}

/// A short original line the Warlock drops between chambers.
fn room_taunt(level: i32) -> String {
    let n = (level - 1).rem_euclid(6);
    [
        "Who wanders my tower?",
        "Mind the ghosts.",
        "Who let slip the hounds?",
        "My runes are not for you.",
        "The stone will not save you.",
        "At last, a rival.",
    ][n as usize]
        .to_string()
}

// ---- drawing --------------------------------------------------------------

fn draw_maze(blip: &Blip, g: &Game, t: f32) {
    for r in 0..ROWS as i32 {
        for c in 0..COLS as i32 {
            let x = (c * CELL) as f32;
            let y = (HUD_H + r * CELL) as f32;
            if g.walls.get(c, r) {
                blip.fill_rect(x, y, CELL as f32, CELL as f32, WALL);
                blip.fill_rect(x, y, CELL as f32, 2.0, WALL_HI);
            } else {
                blip.fill_rect(x, y, CELL as f32, CELL as f32, FLOOR);
                blip.draw_rect(x, y, CELL as f32, CELL as f32, FLOOR_GRID);
            }
        }
    }
    for &cell in &g.torches {
        let (tx, ty) = center(cell);
        let flick = 0.7 + 0.3 * (t * 9.0 + (cell.c * 3 + cell.r * 5) as f32).sin();
        blip.fill_glow_circle(tx, ty, 4.0 * flick, TORCH);
        blip.fill_circle(tx, ty, 2.0 * flick, BlipColor { r: 1.0, g: 0.9, b: 0.6, a: 1.0 });
    }
}

fn draw_wizard(blip: &Blip, g: &Game, idx: usize) {
    if !g.active[idx] {
        return;
    }
    let w = &g.wizards[idx];
    if g.state == State::Dead && g.dead_index == Some(idx) && (g.dead_timer.remaining() / 0.12) as i32 % 2 == 0 {
        return;
    }
    let (x, y) = g.wizard_px(idx);
    let s = CELL as f32 * 0.36;
    let robe = if idx == 0 { WIZ_ROBE } else { WIZ_ROBE2 };
    let hat = if idx == 0 { WIZ_HAT } else { WIZ_HAT2 };
    if w.protect > 0.0 && (w.protect * 8.0) as i32 % 2 == 0 {
        blip.fill_circle(x, y, s + 4.0, PROTECT);
    }
    blip.fill_circle(x, y + 2.0, s, robe);
    blip.fill_capsule(x, y - s, x, y - s - s * 1.1, s * 0.55, s * 0.30, hat);
    blip.fill_circle(x, y - s * 0.2, s * 0.62, hat);
    let (dx, dy) = w.facing.delta();
    let (sx, sy) = (x + dx as f32 * s * 1.6, y + dy as f32 * s * 1.6);
    blip.draw_line_ex(x, y, sx, sy, 2.0, hat);
    blip.fill_glow_circle(sx, sy, 3.4, REVEAL_C);
    blip.fill_circle(sx, sy, 2.0, BlipColor { r: 1.0, g: 1.0, b: 0.9, a: 1.0 });
    let (ex, ey) = match w.facing {
        Dir::Left => (x - s * 0.3, y - s * 0.1),
        Dir::Right => (x + s * 0.3, y - s * 0.1),
        Dir::Up => (x, y - s * 0.4),
        Dir::Down => (x, y + s * 0.1),
    };
    blip.fill_rect(ex - 1.2, ey - 1.2, 2.4, 2.4, BLIP_BLACK);
}

fn draw_imp(blip: &Blip, g: &Game, e: &Enemy, t: f32) {
    let (x, y) = g.enemy_px(e);
    let s = CELL as f32 * 0.3;
    let bob = (t * 6.0).sin() * 1.5;
    let (dx, _) = e.dir.delta();
    blip.fill_capsule(x - dx as f32 * 4.0, y - s, x - dx as f32 * 8.0, y - s * 2.0, 1.5, 1.0, IMP);
    blip.fill_capsule(x + dx as f32 * 4.0, y - s, x + dx as f32 * 8.0, y - s * 2.0, 1.5, 1.0, IMP);
    blip.fill_circle(x, y + bob * 0.3, s, IMP);
    blip.fill_rect(x - 3.0, y - 2.0, 2.4, 2.4, BLIP_BLACK);
    blip.fill_rect(x + 1.0, y - 2.0, 2.4, 2.4, BLIP_BLACK);
}

fn draw_wraith(blip: &Blip, g: &Game, e: &Enemy, t: f32) {
    if !e.visible {
        return;
    }
    let (x, y) = g.enemy_px(e);
    let s = CELL as f32 * 0.34;
    let a = if e.lunging {
        if e.charge > 0.0 {
            // materializing: fade in across the charge beat
            0.35 + 0.65 * (1.0 - e.charge / WRAITH_CHARGE)
        } else {
            1.0
        }
    } else {
        0.75 + 0.25 * (t * 8.0).sin()
    };
    let c = BlipColor { a, ..WRAITH };
    blip.fill_circle(x, y, s, c);
    for k in 0..4 {
        let hx = x - s + (k as f32 + 0.5) * s * 2.0 / 4.0;
        let hy = y + s * 0.7 + if k % 2 == 0 { 2.0 } else { -2.0 };
        blip.fill_circle(hx, hy, 2.5, c);
    }
    blip.fill_rect(x - 3.0, y - 3.0, 2.4, 2.4, BLIP_BLACK);
    blip.fill_rect(x + 0.6, y - 3.0, 2.4, 2.4, BLIP_BLACK);
}

fn draw_hound(blip: &Blip, g: &Game, e: &Enemy) {
    let (x, y) = g.enemy_px(e);
    let s = CELL as f32 * 0.3;
    let ears = if e.hunting { s * 1.6 } else { s * 0.9 };
    blip.fill_capsule(x - s, y, x + s, y, s, s, HOUND);
    blip.fill_capsule(x - s * 0.5, y - s, x - s * 0.5, y - ears, 2.0, 1.5, HOUND);
    blip.fill_capsule(x + s * 0.5, y - s, x + s * 0.5, y - ears, 2.0, 1.5, HOUND);
    blip.fill_circle(x + s, y, s * 0.6, HOUND);
    blip.fill_rect(x + s, y - 2.0, 2.4, 2.4, BLIP_BLACK);
}

fn draw_gargoyle(blip: &Blip, g: &Game, e: &Enemy) {
    let (x, y) = g.enemy_px(e);
    let s = CELL as f32 * 0.34;
    let wing = if e.windup > 0.0 {
        let k = 1.0 - e.windup / GARGOYLE_TELL;
        s * (0.4 + 1.2 * k)
    } else {
        s * 0.4
    };
    blip.fill_capsule(x - s, y - wing, x - s * 1.8, y - wing * 1.4, 2.0, 1.0, GARGOYLE);
    blip.fill_capsule(x + s, y - wing, x + s * 1.8, y - wing * 1.4, 2.0, 1.0, GARGOYLE);
    blip.fill_circle(x, y, s, GARGOYLE);
    blip.fill_rect(x - 3.0, y - 3.0, 2.4, 2.4, BLIP_BLACK);
    blip.fill_rect(x + 0.6, y - 3.0, 2.4, 2.4, BLIP_BLACK);
}

fn draw_warlock(blip: &Blip, g: &Game, t: f32) {
    let Some(wl) = &g.warlock else { return };
    if wl.wards <= 0 {
        return;
    }
    const CIRC: BlipColor = BlipColor { r: 0.4, g: 0.2, b: 0.5, a: 1.0 };
    for (i, c) in wl.circles.iter().enumerate() {
        let (x, y) = center(*c);
        if wl.sealed[i] {
            blip.draw_circle(x, y, CELL as f32 * 0.3, 1.0, BLIP_GRAY);
        } else if i == wl.circle {
            let pulse = 1.0 + 0.3 * (t * 6.0).sin();
            blip.draw_circle(x, y, CELL as f32 * 0.32 * pulse, 2.0, WARLOCK_C);
        } else {
            blip.draw_circle(x, y, CELL as f32 * 0.3, 1.0, CIRC);
        }
    }
    if wl.state == WState::Aiming || wl.state == WState::Casting {
        let mut p = wl.aim_from;
        let (dc, dr) = wl.aim_dir.delta();
        for _ in 0..GARGOYLE_RANGE {
            p = Cell { c: p.c + dc, r: p.r + dr };
            if g.walls.get(p.c, p.r) {
                break;
            }
            let (lx, ly) = center(p);
            blip.fill_rect(lx - 6.0, ly - 1.0, 12.0, 2.0, BlipColor { a: 0.5, ..WARLOCK_C });
        }
    }
    let (x, y) = center(wl.circles[wl.circle]);
    let fade = match wl.state {
        WState::Materializing => (1.0 - wl.timer / 0.7).clamp(0.0, 1.0),
        _ => 1.0,
    };
    let c = BlipColor { a: fade, ..WARLOCK_C };
    let s = CELL as f32 * 0.36;
    blip.fill_circle(x, y + 2.0, s, c);
    blip.fill_capsule(x, y - s, x, y - s * 2.0, s * 0.5, s * 0.25, c);
    let (dx, dy) = wl.aim_dir.delta();
    blip.fill_glow_circle(x + dx as f32 * s * 1.6, y + dy as f32 * s * 1.6, 3.0, WARLOCK_C);
    blip.fill_rect(x - 3.0, y - 3.0, 2.4, 2.4, BLIP_BLACK);
    blip.fill_rect(x + 0.6, y - 3.0, 2.4, 2.4, BLIP_BLACK);
}

fn draw_projectile(blip: &Blip, p: &Projectile) {
    let a = center(p.from);
    let b = center(p.tile);
    let t = p.lerp;
    let (x, y) = (lerp(a.0, b.0, t), lerp(a.1, b.1, t));
    match p.proj {
        Proj::Bolt => {
            blip.fill_glow_circle(x, y, 3.0, BOLT);
            let (dx, dy) = p.dir.delta();
            blip.draw_line_ex(x - dx as f32 * 8.0, y - dy as f32 * 8.0, x, y, 2.0, BOLT);
        }
        _ => {
            blip.fill_glow_circle(x, y, 4.0, FIREBALL);
            blip.fill_circle(x, y, 2.5, BlipColor { r: 1.0, g: 0.9, b: 0.5, a: 1.0 });
        }
    }
}

fn draw_runes(blip: &Blip, g: &Game, t: f32) {
    for r in &g.runes {
        let (x, y) = center(r.tile);
        if r.cleansed {
            let pulse = 1.0 + 0.2 * (t * 4.0).sin();
            blip.draw_circle(x, y, CELL as f32 * 0.32 * pulse, 2.0, RUNE_CLEAN);
            blip.fill_glow_circle(x, y, 3.0, RUNE_CLEAN);
        } else {
            blip.draw_circle(x, y, CELL as f32 * 0.3, 1.5, RUNE_CURSED);
            blip.fill_circle(x, y, 2.0, RUNE_CURSED);
        }
    }
}

fn draw_gates(blip: &Blip, g: &Game) {
    for gate in &g.gates {
        let closed = gate.cooldown > 0.0;
        for c in [gate.a, gate.b] {
            let (x, y) = center(c);
            let color = if closed { BLIP_GRAY } else { GATE_C };
            blip.draw_circle(x, y, CELL as f32 * 0.36, 2.0, color);
            blip.draw_circle(x, y, CELL as f32 * 0.18, 1.5, color);
        }
    }
}

fn draw_exit(blip: &Blip, g: &Game, t: f32) {
    let (x, y) = center(g.exit);
    if g.exit_open {
        let pulse = 1.0 + 0.25 * (t * 5.0).sin();
        blip.draw_circle(x, y, CELL as f32 * 0.4 * pulse, 3.0, EXIT_C);
        blip.fill_glow_circle(x, y, 4.0, EXIT_C);
    } else {
        blip.draw_circle(x, y, CELL as f32 * 0.34, 1.5, BlipColor { a: 0.5, ..BLIP_GRAY });
    }
}

fn draw_sightings(blip: &Blip, g: &Game) {
    for s in &g.sightings {
        let k = s.t / SIGHTING_FADE;
        let c = BlipColor { a: (1.0 - k) * 0.8, ..WRAITH };
        let (x, y) = center(s.tile);
        blip.draw_rect(x + 3.0, y + 3.0, (CELL - 6) as f32, (CELL - 6) as f32, c);
    }
}

fn draw_reveal(blip: &Blip, g: &Game) {
    for idx in 0..2 {
        if !g.active[idx] || g.wizards[idx].reveal_t <= 0.0 {
            continue;
        }
        let k = 1.0 - g.wizards[idx].reveal_t / 0.55;
        let (x, y) = g.wizard_px(idx);
        let r = CELL as f32 * (1.0 + k * 7.0);
        blip.draw_circle(x, y, r, 2.0, BlipColor { a: (1.0 - k) * 0.9, ..REVEAL_C });
    }
}

fn draw_counters(blip: &Blip, g: &Game) {
    let monsters = g.enemies.iter().filter(|e| !e.gone).count();
    let warlock = g.warlock.as_ref().map_or(false, |wl| wl.wards > 0);
    let all = (monsters + if warlock { 1 } else { 0 }) as i32;
    let runes_left = g.runes.iter().filter(|r| !r.cleansed).count() as i32;
    let cy = HUD_H as f32 + 10.0;
    let ty = HUD_H as f32 + 4.0;
    // Counters draw as glyphs echoing the sprites, so the numbers read at a
    // glance: a horned monster, then the rune it is safe to cleanse.
    let mut x = 10.0;
    blip.fill_circle(x, cy, 4.0, IMP);
    blip.fill_rect(x - 1.8, cy - 2.6, 1.8, 1.8, BLIP_BLACK);
    blip.fill_rect(x + 0.6, cy - 2.6, 1.8, 1.8, BLIP_BLACK);
    x += 12.0;
    blip.draw_number(all, x, ty, 2.0, BLIP_WHITE);
    x += all.to_string().len().max(1) as f32 * 12.0 + 16.0;
    blip.draw_circle(x, cy, 4.0, 1.5, RUNE_CURSED);
    blip.fill_circle(x, cy, 1.6, RUNE_CURSED);
    x += 12.0;
    blip.draw_number(runes_left, x, ty, 2.0, BLIP_WHITE);
    // reveal cooldown readout: a ring that refills beside the counts
    let cx = x + runes_left.to_string().len().max(1) as f32 * 12.0 + 16.0;
    blip.draw_circle(cx, cy, 7.0, 2.0, BLIP_GRAY);
    let cd = g.wizards[0].reveal_cd;
    if cd <= 0.0 {
        blip.fill_circle(cx, cy, 3.5, REVEAL_C);
    } else {
        let frac = 1.0 - cd / REVEAL_CD;
        blip.fill_circle(cx, cy - 7.0 + 14.0 * frac, 2.0, REVEAL_C);
    }
}

fn draw_taunt(blip: &Blip, g: &Game) {
    if let Some((text, t)) = &g.taunt {
        if *t > 0.0 {
            blip.draw_centered(text, HUD_H as f32 + 22.0, 2.0, WARLOCK_C);
        }
    }
    if let Some((text, t)) = &g.banner {
        if *t > 0.0 {
            blip.draw_centered(text, WIN_H as f32 / 2.0, 4.0, BLIP_YELLOW);
        }
    }
}

fn draw_world(blip: &Blip, g: &Game, t: f32) {
    draw_maze(blip, g, t);
    draw_gates(blip, g);
    draw_runes(blip, g, t);
    draw_exit(blip, g, t);
    draw_sightings(blip, g);
    draw_warlock(blip, g, t);
    for e in &g.enemies {
        if e.gone {
            continue;
        }
        match e.kind {
            EnemyKind::Imp => draw_imp(blip, g, e, t),
            EnemyKind::Wraith => draw_wraith(blip, g, e, t),
            EnemyKind::Spellhound => draw_hound(blip, g, e),
            EnemyKind::Gargoyle => draw_gargoyle(blip, g, e),
        }
    }
    for b in &g.bolts {
        draw_projectile(blip, b);
    }
    for f in &g.fires {
        draw_projectile(blip, f);
    }
    draw_reveal(blip, g);
    draw_wizard(blip, g, 0);
    draw_wizard(blip, g, 1);
    g.fx.draw(blip);
}

fn hud_lives(g: &Game) -> i32 {
    if g.two_player {
        g.lives[0] + g.lives[1]
    } else {
        g.lives[0]
    }
}

fn draw_play(blip: &Blip, g: &Game, t: f32) {
    draw_world(blip, g, t);
    draw_counters(blip, g);
    draw_taunt(blip, g);
    blip.draw_hud(g.sess.score, hud_lives(g));
    if g.two_player && (g.state == State::Play || g.state == State::Dead) {
        blip.draw_text(&format!("P1 {}  P2 {}", g.lives[0], g.lives[1]), WIN_W as f32 - 96.0, HUD_H as f32 + 4.0, 2.0, BLIP_WHITE);
    }
}

fn draw_title(blip: &Blip, t: f32) {
    blip.draw_centered("WARLOCK", WIN_H as f32 * 0.20, 6.0, WARLOCK_C);
    let hi = web::high_score();
    blip.draw_hi(&hi, WIN_H as f32 * 0.20 + 46.0, BLIP_YELLOW);
    let x = (t * 40.0).rem_euclid(WIN_W as f32 + 60.0) - 30.0;
    let y = WIN_H as f32 * 0.62;
    blip.fill_circle(x, y, 10.0, WRAITH);
    let by = web::controls();
    blip.draw_centered(
        by.pick("PRESS FIRE", "PRESS FIRE", "TAP TO START"),
        WIN_H as f32 * 0.50,
        3.0,
        BLIP_WHITE,
    );
    blip.draw_centered(
        by.pick(
            "ARROWS/WASD MOVE  SPACE BOLT  Z REVEAL",
            "STICK MOVE  A BOLT  B REVEAL",
            "STICK MOVE  TAP BOLT",
        ),
        WIN_H as f32 * 0.60,
        2.0,
        BLIP_GRAY,
    );
    blip.draw_centered(
        by.pick("J TO JOIN A SECOND WIZARD", "2ND STATION JOINS WITH A", "2ND PLAYER JOINS ON THE DECK"),
        WIN_H as f32 * 0.68,
        2.0,
        WIZ_ROBE2,
    );
}

fn draw_over(blip: &Blip, g: &Game) {
    let hi = web::high_score();
    blip.draw_game_over(
        g.sess.score,
        &hi,
        WARLOCK_C,
        BLIP_YELLOW,
        BLIP_WHITE,
        !g.dead_timer.active(),
    );
    blip.draw_centered(
        &format!("WINGS CLEARED {}", g.cleared_no_death),
        WIN_H as f32 * 0.66,
        2.0,
        BLIP_GRAY,
    );
}

fn draw_bonus(blip: &Blip, g: &Game, t: f32) {
    draw_maze(blip, g, t);
    let Some(b) = &g.bonus else { return };
    for (i, s) in b.stones.iter().enumerate() {
        let (x, y) = center(*s);
        let lit = b.showing && i == b.seq[b.show_i];
        if lit {
            blip.draw_circle(x, y, CELL as f32 * 0.42, 3.0, RUNE_CLEAN);
            blip.fill_glow_circle(x, y, 5.0, RUNE_CLEAN);
        } else {
            blip.draw_circle(x, y, CELL as f32 * 0.32, 1.5, RUNE_CURSED);
            blip.fill_circle(x, y, 2.0, RUNE_CURSED);
        }
    }
    draw_wizard(blip, g, 0);
    draw_wizard(blip, g, 1);
    g.fx.draw(blip);
    draw_taunt(blip, g);
    blip.draw_hud(g.sess.score, hud_lives(g));
    if !b.showing {
        blip.draw_centered("REPEAT THE LIGHTS", (HUD_H + 16) as f32, 2.0, BLIP_WHITE);
    }
}

// ---- entry ----------------------------------------------------------------

fn conf() -> blip::macroquad::window::Conf {
    window_conf("WARLOCK", WIN_W, WIN_H)
}

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();
    // Leave the second station available for a mid-game co-op join.
    web::set_players(2);
    g.sfx = Some(Sounds {
        bolt: blip::audio::load_sound(blip::wav!("bolt")).await,
        reveal: blip::audio::load_sound(blip::wav!("reveal")).await,
        cleanse: blip::audio::load_sound(blip::wav!("cleanse")).await,
        kill: blip::audio::load_sound(blip::wav!("kill")).await,
        death: blip::audio::load_sound(blip::wav!("death")).await,
        over: blip::audio::load_sound(blip::wav!("over")).await,
        chamber: blip::audio::load_sound(blip::wav!("chamber")).await,
        ward: blip::audio::load_sound(blip::wav!("ward")).await,
        warlock: blip::audio::load_sound(blip::wav!("warlock")).await,
    });
    let mut music = Jukebox::new("WARLOCK", &["DUNGEON", "TOWER"]);

    if blip.screenshot_mode {
        g.start_game();
    }

    loop {
        let dt = blip.delta_time;
        let t = blip::macroquad::time::get_time() as f32;

        #[cfg(not(target_arch = "wasm32"))]
        if blip::bot::active() {
            bot::drive(&g);
        }

        let was_playing = g.state == State::Play;
        match g.state {
            State::Title => {
                g.try_join();
                if coin_start() {
                    g.start_game();
                }
            }
            State::Play => {
                g.try_join();
                for i in 0..2 {
                    if !g.active[i] {
                        continue;
                    }
                    if g.bolt_pressed(i) || (g.bolt_held(i) && g.wizards[i].bolt_cd <= 0.0) {
                        g.cast_bolt(i);
                    }
                    if g.reveal_pressed(i) {
                        g.reveal(i);
                    }
                }
                g.update_play(dt);
            }
            State::Dead => g.update_dead(dt),
            State::Over => g.check_over_again(dt),
            State::Bonus => g.update_bonus(dt),
        }
        if was_playing && g.state != State::Play {
            web::haptic();
        }

        music.play(g.track());

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title => draw_title(&blip, t),
            State::Over => draw_over(&blip, &g),
            State::Play | State::Dead => draw_play(&blip, &g, t),
            State::Bonus => draw_bonus(&blip, &g, t),
        }

        blip.next_frame(60).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bolt_stops_at_a_wall_and_not_before() {
        let mut g = Game::new();
        g.start_game();
        g.place_wizard(0, Cell { c: 1, r: 1 });
        g.wizards[0].facing = Dir::Right;
        g.cast_bolt(0);
        let mut last = Cell { c: 0, r: 0 };
        for _ in 0..40 {
            g.update_projectiles(BOLT_INTERVAL);
            if g.bolts.is_empty() {
                break;
            }
            last = g.bolts[0].tile;
        }
        assert!(last.c <= 14, "the bolt crossed the right wall");
    }

    #[test]
    fn reveal_exposes_wraiths() {
        let mut g = Game::new();
        g.sess.level = 2;
        g.load_room(); // room 2 introduces wraiths
        g.wizards[0].reveal_cd = 0.0;
        assert!(g.enemies.iter().any(|e| e.kind == EnemyKind::Wraith));
        g.reveal(0);
        for e in &g.enemies {
            if e.kind == EnemyKind::Wraith {
                assert!(e.revealed > 0.0, "wraith not exposed by reveal");
            }
        }
    }

    #[test]
    fn cleansing_a_rune_opens_the_exit() {
        let mut g = Game::new();
        g.start_game();
        for r in &mut g.runes {
            r.cleansed = true;
        }
        for e in &mut g.enemies {
            e.gone = true;
        }
        g.check_exit();
        assert!(g.exit_open, "exit should open once the chamber is clear");
    }

    #[test]
    fn reveal_cleanses_an_adjacent_rune() {
        let mut g = Game::new();
        g.start_game();
        g.wizards[0].reveal_cd = 0.0;
        let rune = g.runes[0].tile;
        // stand beside it
        g.place_wizard(0, Cell { c: rune.c + 1, r: rune.r });
        g.reveal(0);
        assert!(g.runes[0].cleansed, "an adjacent rune was not cleansed by reveal");
    }

    #[test]
    fn the_warlock_falls_after_four_ward_breaks() {
        let mut g = Game::new();
        g.sess.level = 6;
        g.load_room();
        assert!(g.warlock.is_some(), "room 6 has no warlock");
        assert_eq!(g.warlock.as_ref().unwrap().wards, 4);
        for _ in 0..4 {
            g.hit_warlock();
        }
        assert_eq!(
            g.warlock.as_ref().map(|w| w.wards),
            Some(0),
            "four ward breaks should defeat the warlock"
        );
        for r in &mut g.runes {
            r.cleansed = true;
        }
        for e in &mut g.enemies {
            e.gone = true;
        }
        g.check_exit();
        assert!(g.exit_open, "exit should open once the warlock and minions are cleared");
    }

    #[test]
    fn reveal_draws_spellhounds_to_the_cast_point() {
        let mut g = Game::new();
        g.sess.level = 3;
        g.load_room();
        g.wizards[0].reveal_cd = 0.0;
        g.wizards[0].tile = Cell { c: 4, r: 4 };
        g.reveal(0);
        for e in &g.enemies {
            if e.kind == EnemyKind::Spellhound {
                assert!(e.hunting, "reveal should set a spellhound hunting");
                assert_eq!(e.target, Cell { c: 4, r: 4 }, "spellhound should head for the cast point");
            }
        }
    }

    #[test]
    fn a_rune_gate_carries_the_wizard_across_the_room() {
        let mut g = Game::new();
        g.sess.level = 4;
        g.load_room();
        assert!(!g.gates.is_empty());
        let gate = g.gates[0];
        // step onto gate A
        g.place_wizard(0, gate.a);
        g.warp_through_gate(0, 0);
        assert_eq!(g.wizards[0].tile, gate.b, "the wizard should emerge at the paired sigil");
        assert!(g.gates[0].cooldown > 0.0, "the gate pair should close after use");
    }

    #[test]
    fn joining_adds_a_second_wizard_with_her_own_lives() {
        let mut g = Game::new();
        g.start_game();
        assert!(!g.two_player);
        assert!(!g.active[1]);
        // simulate the join key (J is pressed through the input layer; here we
        // drive the join directly, which try_join gates on key_pressed).
        g.two_player = true;
        g.active[1] = true;
        g.place_wizard(1, Cell { c: 2, r: 1 });
        assert_eq!(g.lives[1], LIVES_START);
        assert_eq!(hud_lives(&g), LIVES_START * 2, "co-op shows both wizards' lives");
    }

    #[test]
    fn a_downed_partner_returns_on_chamber_clear() {
        let mut g = Game::new();
        g.start_game();
        g.two_player = true;
        g.active[1] = true;
        // partner two runs out of lives
        g.active[1] = false;
        g.lives[1] = 0;
        // clearing the chamber revives them with one life
        g.finish_chamber();
        assert!(g.active[1], "the downed partner should return");
        assert_eq!(g.lives[1], 1, "the returned partner gets one life");
    }

    #[test]
    fn a_bolt_passes_through_the_other_wizard() {
        // Friendly spells pass through wizards: no wizard is a bolt target.
        let mut g = Game::new();
        g.start_game();
        g.two_player = true;
        g.active[1] = true;
        // place wizard 1 directly in front of wizard 0's facing
        g.wizards[0].facing = Dir::Right;
        g.place_wizard(1, Cell { c: g.wizards[0].tile.c + 2, r: g.wizards[0].tile.r });
        g.cast_bolt(0);
        // run the bolt a few tiles; wizard 1 must not take damage
        for _ in 0..6 {
            g.update_projectiles(BOLT_INTERVAL);
        }
        assert_eq!(g.lives[1], LIVES_START, "a friendly bolt must not harm the other wizard");
    }

    #[test]
    fn a_ghost_does_not_kill_until_its_body_reaches_the_wizard() {
        let mut g = Game::new();
        g.start_game();
        let w = g.wizards[0].tile;
        g.enemies.clear();
        g.enemies.push(Enemy {
            kind: EnemyKind::Wraith,
            tile: Cell { c: w.c + 1, r: w.r },
            from: Cell { c: w.c + 1, r: w.r },
            lerp: 0.0,
            step_t: 100.0,
            dir: Dir::Left,
            visible: true,
            revealed: 0.0,
            flash: 0.0,
            hunting: false,
            target: Cell { c: 0, r: 0 },
            lunging: true,
            windup: 0.0,
            charge: 0.0,
            shot_cd: 0.0,
            gone: false,
        });
        // the wraith starts a step onto the wizard's tile: its body is still
        // a whole cell away, so no contact yet
        g.enemies[0].from = g.enemies[0].tile;
        g.enemies[0].tile = w;
        g.enemies[0].lerp = 0.0;
        g.check_contact();
        assert_eq!(g.lives[0], LIVES_START, "a ghost killed from a whole cell away");
        // once it has visually arrived, the touch is real
        g.enemies[0].lerp = 1.0;
        g.check_contact();
        assert_eq!(g.lives[0], LIVES_START - 1, "an arrived ghost did not connect");
    }

    #[test]
    fn a_wraith_pauses_visibly_before_its_first_lunge() {
        let mut g = Game::new();
        g.sess.level = 2;
        g.load_room();
        let wraith_tile = {
            let e = g.enemies.iter_mut().find(|e| e.kind == EnemyKind::Wraith).unwrap();
            e.lunging = false;
            e.charge = 0.0;
            e.step_t = 100.0;
            e.tile
        };
        // put a wizard beside it, then run one visibility pass
        g.place_wizard(0, Cell { c: wraith_tile.c + 1, r: wraith_tile.r });
        g.update_enemies(0.0);
        let e = g.enemies.iter().find(|e| e.kind == EnemyKind::Wraith).unwrap();
        assert!(e.lunging, "the wraith should now be lunging");
        assert!(e.charge > 0.0, "a fresh lunge should telegraph before it moves");
    }

    #[test]
    fn a_cleansed_stone_makes_nearby_wraiths_visible() {
        let mut g = Game::new();
        g.sess.level = 2;
        g.load_room();
        // cleanse the first rune and put a wraith beside it
        g.runes[0].cleansed = true;
        let rune_tile = g.runes[0].tile;
        if let Some(w) = g.enemies.iter_mut().find(|e| e.kind == EnemyKind::Wraith) {
            w.tile = rune_tile;
            w.from = rune_tile;
            w.revealed = 0.0;
            w.flash = WRAITH_FLASH_OFF; // force the invisible part of the cycle
            w.step_t = 100.0; // hold still so update_enemies doesn't move it
        }
        g.update_enemies(0.0);
        let near = g.enemies.iter().find(|e| e.kind == EnemyKind::Wraith && e.tile == rune_tile);
        assert!(
            near.is_some_and(|w| w.visible),
            "a cleansed rune should force the wraith beside it visible"
        );
    }
}