//! WARLOCK — a dungeon-crawl maze shooter in the spirit of Wizard of Wor.
//! A young wizard descends into a rival sorcerer's tower: move through the
//! corridors, cast bolts down them, and use reveal to expose what the dark
//! hides. Cleansed rune stones keep the wraiths at bay; every chamber holds
//! its own lesson, and the Warlock himself waits at the bottom.

#[cfg(test)]
mod tunes;
mod bot;
#[cfg(not(target_arch = "wasm32"))]
mod scene;
mod levels;
mod art;
mod magic;
mod runes;

use levels::*;
use magic::Magic;

use blip::input::{
    key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_D,
    BLIP_KEY_DOWN, BLIP_KEY_F, BLIP_KEY_G, BLIP_KEY_J, BLIP_KEY_K, BLIP_KEY_LEFT, BLIP_KEY_RIGHT,
    BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::{
    lerp, rand_int, web, window_conf, Blip, BlipColor, Fx, GAME_OVER_MIN_WAIT, Jukebox,
    Session, Timer, BLIP_BLACK, BLIP_GRAY, BLIP_WHITE, BLIP_YELLOW,
};

// ---- layout ---------------------------------------------------------------
/// A chamber's title card: on screen this long, holding play for the first part.
const INTRO_SECS: f32 = 2.0;
const INTRO_HOLD: f32 = 1.1;
const CELL: i32 = 32;
pub const HUD_H: i32 = 28;
/// The carved panel under the cellar: chamber name, scrying glass, lives.
const PANEL_H: i32 = 96;
const WIN_W: i32 = COLS as i32 * CELL;
const WIN_H: i32 = ROWS as i32 * CELL + HUD_H + PANEL_H;

// ---- dungeon palette ------------------------------------------------------
const TORCH: BlipColor = BlipColor { r: 1.0, g: 0.62, b: 0.22, a: 1.0 };
const WIZ_ROBE: BlipColor = BlipColor { r: 0.20, g: 0.90, b: 0.78, a: 1.0 };
const WIZ_ROBE2: BlipColor = BlipColor { r: 0.98, g: 0.55, b: 0.25, a: 1.0 };
const REVEAL_C: BlipColor = BlipColor { r: 0.62, g: 0.90, b: 1.0, a: 1.0 };
const IMP: BlipColor = BlipColor { r: 1.0, g: 0.32, b: 0.22, a: 1.0 };
const WRAITH: BlipColor = BlipColor { r: 0.66, g: 0.78, b: 1.0, a: 1.0 };
const HOUND: BlipColor = BlipColor { r: 1.0, g: 0.70, b: 0.30, a: 1.0 };
const GARGOYLE: BlipColor = BlipColor { r: 0.42, g: 0.82, b: 0.44, a: 1.0 };
const WARLOCK_C: BlipColor = BlipColor { r: 0.82, g: 0.34, b: 0.94, a: 1.0 };
const RUNE_CLEAN: BlipColor = BlipColor { r: 1.0, g: 0.85, b: 0.35, a: 1.0 };
const GATE_C: BlipColor = BlipColor { r: 0.40, g: 0.85, b: 1.0, a: 1.0 };
const EXIT_C: BlipColor = BlipColor { r: 0.5, g: 1.0, b: 0.6, a: 1.0 };
const FIREBALL: BlipColor = BlipColor { r: 1.0, g: 0.45, b: 0.15, a: 1.0 };
const LANCE_C: BlipColor = BlipColor { r: 1.0, g: 0.85, b: 0.55, a: 1.0 };
const POTION_HASTE: BlipColor = BlipColor { r: 0.35, g: 0.85, b: 1.0, a: 1.0 };
const POTION_GOLD: BlipColor = BlipColor { r: 1.0, g: 0.82, b: 0.3, a: 1.0 };
const POTION_LIFE: BlipColor = BlipColor { r: 1.0, g: 0.3, b: 0.45, a: 1.0 };
const BOLT_TRAIL: BlipColor = BlipColor { r: 0.55, g: 0.8, b: 1.0, a: 1.0 };

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
    /// bolt cast straight after reveal: the two spells combine and it pierces
    Lance,
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
    /// Last horizontal heading, so a side-view sprite keeps facing it.
    face_left: bool,
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

/// A potion left by a slain monster: walk over it to drink.
#[derive(Clone, Copy, PartialEq, Debug)]
enum PotionKind {
    /// quick feet for a while
    Haste,
    /// worth points
    Gold,
    /// a spare hat: one more life
    Life,
}

#[derive(Clone, Copy)]
struct Potion {
    tile: Cell,
    kind: PotionKind,
    t: f32,
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
    /// Counts down after a bolt: the staff is thrust out and flares.
    cast_t: f32,
    /// Seconds of haste left from a draught.
    haste: f32,
    /// Just after a reveal: a bolt cast now combines with it into a lance.
    combo_t: f32,
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
            cast_t: 0.0,
            haste: 0.0,
            combo_t: 0.0,
        }
    }
}

/// A dragon whelp: it bolts through the corridors once a chamber's
/// monsters are gone and flies off when its time runs out. Strike it and
/// the next chamber pays double.
struct Whelp {
    tile: Cell,
    from: Cell,
    lerp: f32,
    step_t: f32,
    dir: Dir,
    left: f32,
    face_left: bool,
}

struct Bonus {
    stones: Vec<Cell>,
    seq: Vec<usize>,
    expect: usize,
    showing: bool,
    show_i: usize,
    show_t: f32,
    /// Seconds left to repeat the sequence; the round ends when they run out.
    left: f32,
}

/// Time to repeat the spell circle's sequence, so an idle round never stalls the game.
const BONUS_TIME: f32 = 20.0;

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
    Fire,
    Pulse,
    Whelp,
    Potion,
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
    fire: blip::BlipSound,
    pulse: blip::BlipSound,
    whelp: blip::BlipSound,
    potion: blip::BlipSound,
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
    magic: Magic,
    /// Bumped whenever the walls change, so baked torchlight is redone.
    room_stamp: u32,
    whelp: Option<Whelp>,
    potions: Vec<Potion>,
    /// The lance has been named once this game.
    lance_named: bool,
    /// The whelp has had its turn in this chamber.
    whelp_done: bool,
    /// This chamber's points count twice (the whelp was struck last chamber).
    double: bool,
    double_next: bool,
    /// Monsters the chamber started with, and the heartbeat's clock.
    start_monsters: usize,
    pulse_t: f32,
    /// The chamber's title card: counts down from INTRO_SECS as it fades.
    intro: f32,
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
            magic: Magic::default(),
            room_stamp: 0,
            whelp: None,
            potions: Vec::new(),
            lance_named: false,
            whelp_done: false,
            double: false,
            double_next: false,
            start_monsters: 0,
            pulse_t: 0.0,
            intro: 0.0,
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
            Fire => &sounds.fire,
            Pulse => &sounds.pulse,
            Whelp => &sounds.whelp,
            Potion => &sounds.potion,
        };
        blip::play_sfx(target);
    }

    fn track(&self) -> usize {
        if self.state == State::Title {
            return 2;
        }
        if matches!(self.state, State::Play | State::Dead) && self.warlock.is_some() {
            1
        } else if self.sess.level > ROOMS.len() as i32 {
            3
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
            face_left: dir == Dir::Left,
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
        let extra = (self.sess.level - 6).clamp(0, 5) as usize;
        for _ in 0..extra {
            if let Some((c, r)) = self.random_open_cell() {
                enemies.push(mk(c, r, EnemyKind::Imp, Dir::Right));
            }
        }
        self.enemies = enemies;
        self.start_monsters = self.enemies.len().max(1);
        self.intro = INTRO_SECS;
        self.double = self.double_next;
        self.double_next = false;
        self.whelp = None;
        self.whelp_done = false;
        self.potions.clear();
        self.pulse_t = 0.5;

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
        self.room_stamp = self.room_stamp.wrapping_add(1);
        self.magic.clear();
        self.bonus = None;
    }

    fn random_open_cell(&self) -> Option<(i32, i32)> {
        random_open_cell(&self.walls, self.player_spawn, self.exit)
    }

    /// Sconces hang on wall faces that look down onto floor, spaced apart.
    fn find_torches(&self) -> Vec<Cell> {
        let mut out: Vec<Cell> = Vec::new();
        for r in 0..ROWS as i32 - 1 {
            for c in 1..COLS as i32 - 1 {
                if !self.walls.get(c, r) || !self.walls.floor(c, r + 1) {
                    continue;
                }
                let cell = Cell { c, r };
                let spaced = out.iter().all(|o| (o.c - c).abs() + (o.r - r).abs() >= 5);
                if spaced && (c * 7 + r * 13) % 3 == 0 {
                    out.push(cell);
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
        blip::bot::levels(levels::ROOMS.len());
        self.lives = [LIVES_START, LIVES_START];
        self.active = [true, false];
        self.two_player = false;
        self.dead_index = None;
        self.cleared_no_death = 0;
        self.double_next = false;
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
        self.join();
    }

    /// J on the title: a two-player start, both wizards in from the first chamber.
    fn start_two(&mut self) {
        web::spend_coin();
        self.start_game();
        self.join();
    }

    fn join(&mut self) {
        self.two_player = true;
        self.active[1] = true;
        self.lives[1] = LIVES_START;
        self.place_wizard(1, self.second_spawn());
        self.banner = Some(("FRIENDLY FIRE IS OFF. YOU'RE WELCOME".to_string(), 2.0));
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
            || self.warlock.as_ref().is_some_and(|wl| {
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

    /// While the title card holds the screen the cellar waits.
    fn in_intro(&self) -> bool {
        self.intro > INTRO_SECS - INTRO_HOLD
    }

    fn update_play(&mut self, dt: f32) {
        self.fx.update(dt);
        self.update_ambience(dt);
        if self.intro > 0.0 {
            self.intro = (self.intro - dt).max(0.0);
            if self.in_intro() {
                return;
            }
        }
        for i in 0..2 {
            if self.active[i] {
                self.update_wizard(i, dt);
            }
        }
        self.update_potions(dt);
        self.update_enemies(dt);
        self.update_whelp(dt);
        self.update_pulse(dt);
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
            w.cast_t = (w.cast_t - dt).max(0.0);
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
        let step_secs = if self.wizards[idx].haste > 0.0 { MOVE_SECS * HASTE_MOVE } else { MOVE_SECS };

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
                Some((cur, from, ns, (lerp + dt / step_secs).min(1.0), true, facing))
            }
        } else if let Some(dir) = held {
            let (dc, dr) = dir.delta();
            let n = Cell { c: cur.c + dc, r: cur.r + dr };
            if self.walls.floor(n.c, n.r) && !self.step_blocked(n, idx) {
                Some((n, cur, step_secs, 0.0, true, dir))
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
            || self.warlock.as_ref().is_some_and(|wl| wl.wards > 0 && wl.circles[wl.circle] == dest)
        {
            return;
        }
        let (x, y) = self.wizard_px(idx);
        self.magic.warp(x, y, GATE_C);
        self.gates[gi].cooldown = GATE_CD;
        self.place_wizard(idx, dest);
        let (x, y) = center(dest);
        self.magic.warp(x, y, GATE_C);
        blip::bot::add("gate", 1.0);
    }

    fn reveal(&mut self, idx: usize) {
        if self.wizards[idx].reveal_cd > 0.0 {
            return;
        }
        self.wizards[idx].reveal_cd = REVEAL_CD;
        self.wizards[idx].reveal_t = 0.55;
        self.wizards[idx].combo_t = LANCE_WINDOW;
        blip::bot::add("reveal", 1.0);
        let (x, y) = self.wizard_px(idx);
        self.magic.reveal(x, y, REVEAL_C);
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
        for i in 0..self.runes.len() {
            let r = self.runes[i].tile;
            if !self.runes[i].cleansed && distance_lt(self.wizards[idx].tile, r, 1.5) {
                self.runes[i].cleansed = true;
                cleansed = true;
                self.add_points(SCORE_RUNE);
                let (rx, ry) = center(r);
                self.magic.cleanse(rx, ry, RUNE_CLEAN);
                self.fx.popup(rx, ry - 22.0, &format!("+{SCORE_RUNE}"), RUNE_CLEAN);
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
        self.wizards[idx].cast_t = 0.16;
        let (x, y) = self.wizard_px(idx);
        let (tx, ty) = art::staff_tip(&self.wizards[idx]);
        let (dc, dr) = self.wizards[idx].facing.delta();
        let col = if idx == 0 { REVEAL_C } else { WIZ_ROBE2 };
        self.magic.cast(x + tx, y + ty, dc as f32, dr as f32, col);
        let tile = self.wizards[idx].tile;
        // reveal, then bolt: the spells combine into a lance that pierces
        let lance = self.wizards[idx].combo_t > 0.0;
        self.wizards[idx].combo_t = 0.0;
        if lance {
            self.magic.splash(x + tx, y + ty, LANCE_C);
            self.magic.splash(x + tx, y + ty, col);
            self.magic.shake = self.magic.shake.max(0.25);
            if !self.lance_named {
                self.lance_named = true;
                self.banner = Some(("REVEAL + BOLT: ARCANE LANCE!".to_string(), 1.6));
            }
            blip::bot::add("lance", 1.0);
        }
        self.bolts.push(Projectile {
            proj: if lance { Proj::Lance } else { Proj::Bolt },
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
            // the last few monsters hurry, as the heartbeat does
            let alive = occupied.len() as f32;
            let pace = 0.75 + 0.25 * (alive / self.start_monsters.max(1) as f32).min(1.0);
            let interval = enemy_step(&e) * pace;
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
                match dir {
                    Dir::Left => self.enemies[i].face_left = true,
                    Dir::Right => self.enemies[i].face_left = false,
                    _ => {}
                }
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
                    self.magic.burst(x, y - 2.0, 12, 90.0, FIREBALL, magic::Kind::Spark);
                    self.magic.flash(x, y, 80.0, 0.3, FIREBALL);
                    self.sfx(Sfx::Fire);
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
        let mut whelp_hit = false;
        for i in 0..self.bolts.len() {
            let b = &mut self.bolts[i];
            if b.gone {
                continue;
            }
            b.lerp += dt / if b.proj == Proj::Lance { BOLT_INTERVAL * 0.7 } else { BOLT_INTERVAL };
            let (px, py) = art::proj_px(b);
            self.magic.trail(px, py, BOLT_TRAIL);
            if b.lerp < 1.0 {
                continue;
            }
            b.from = b.tile;
            let (dc, dr) = b.dir.delta();
            let next = Cell { c: b.tile.c + dc, r: b.tile.r + dr };
            if self.walls.get(next.c, next.r) || !Walls::in_bounds(next.c, next.r) {
                b.gone = true;
                let (x, y) = center(b.tile);
                self.magic.splash(x + dc as f32 * 14.0, y + dr as f32 * 14.0, BOLT_TRAIL);
                continue;
            }
            b.tile = next;
            b.lerp = 0.0;
            if let Some(idx) = self.enemies.iter().position(|e| !e.gone && e.tile == b.tile) {
                if !kills.contains(&idx) {
                    kills.push(idx);
                }
                if b.proj != Proj::Lance {
                    b.gone = true;
                    continue;
                }
            }
            if self.whelp.as_ref().is_some_and(|w| w.tile == b.tile || w.from == b.tile) {
                whelp_hit = true;
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
        if whelp_hit {
            self.strike_whelp();
        }

        let mut hit_idx = None;
        for i in 0..self.fires.len() {
            let f = &mut self.fires[i];
            if f.gone {
                continue;
            }
            let interval = GARGOYLE_SHOT_STEP;
            f.lerp += dt / interval;
            let (px, py) = art::proj_px(f);
            let ember = if f.proj == Proj::Warlock { WARLOCK_C } else { FIREBALL };
            self.magic.cinders(px, py, ember);
            if f.lerp < 1.0 {
                continue;
            }
            f.from = f.tile;
            let (dc, dr) = f.dir.delta();
            let next = Cell { c: f.tile.c + dc, r: f.tile.r + dr };
            if self.walls.get(next.c, next.r) || !Walls::in_bounds(next.c, next.r) {
                f.gone = true;
                let (x, y) = center(f.tile);
                self.magic.splash(x + dc as f32 * 14.0, y + dr as f32 * 14.0, ember);
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

    fn drop_potion(&mut self, tile: Cell, kind: PotionKind) {
        if self.potions.iter().any(|p| p.tile == tile) {
            return;
        }
        self.potions.push(Potion { tile, kind, t: 0.0 });
    }

    /// Potions age and seep away; a wizard standing on one drinks it.
    fn update_potions(&mut self, dt: f32) {
        for w in &mut self.wizards {
            w.haste = (w.haste - dt).max(0.0);
            w.combo_t = (w.combo_t - dt).max(0.0);
        }
        for p in &mut self.potions {
            p.t += dt;
        }
        self.potions.retain(|p| p.t < POTION_SECS);
        let mut i = 0;
        while i < self.potions.len() {
            let p = self.potions[i];
            if let Some(w) = (0..2).find(|&w| self.active[w] && self.wizards[w].tile == p.tile) {
                self.potions.remove(i);
                self.drink(w, p);
            } else {
                i += 1;
            }
        }
    }

    fn drink(&mut self, w: usize, p: Potion) {
        let (x, y) = center(p.tile);
        let (text, col) = match p.kind {
            PotionKind::Haste => {
                self.wizards[w].haste = HASTE_SECS;
                ("HASTE!".to_string(), POTION_HASTE)
            }
            PotionKind::Gold => {
                self.add_points(SCORE_GOLD);
                (format!("+{}", if self.double { SCORE_GOLD * 2 } else { SCORE_GOLD }), POTION_GOLD)
            }
            PotionKind::Life => {
                self.lives[w] = (self.lives[w] + 1).min(LIVES_MAX);
                ("A SPARE HAT!".to_string(), POTION_LIFE)
            }
        };
        self.fx.popup(x, y - 22.0, &text, col);
        self.magic.splash(x, y, col);
        blip::bot::add("potion", 1.0);
        self.sfx(Sfx::Potion);
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
        self.add_points(pts);
        let (x, y) = self.enemy_px(&e);
        let col = match e.kind {
            EnemyKind::Imp => IMP,
            EnemyKind::Wraith => WRAITH,
            EnemyKind::Spellhound => HOUND,
            EnemyKind::Gargoyle => FIREBALL,
        };
        self.magic.dissolve(x, y, col);
        let (sc, sr) = match e.kind {
            EnemyKind::Imp => (0, 2),
            EnemyKind::Wraith => (2, 2),
            EnemyKind::Spellhound => (4, 2),
            EnemyKind::Gargoyle => (0, 3),
        };
        self.magic.corpse(x, y, sc, sr, e.face_left, 0.8, col);
        self.fx.popup(x, y - 22.0, &format!("+{pts}"), BLIP_WHITE);
        if rand_int(0, 99) < POTION_CHANCE {
            let roll = rand_int(0, 9);
            self.drop_potion(e.tile, if roll == 0 { PotionKind::Life } else if roll <= 4 { PotionKind::Haste } else { PotionKind::Gold });
        }
        blip::bot::add("kill", 1.0);
        self.sfx(Sfx::Kill);
        self.maybe_whelp();
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
        let (x, y) = center(wl.circles[wl.circle]);
        self.magic.summon(x, y, WARLOCK_C);
        const LINES: [&str; 7] = [
            "You will not leave.",
            "Burn, little wizard.",
            "I am NOT a vampire.",
            "My tower, my rules.",
            "Is that your best spell?",
            "Mind your robes.",
            "Again!",
        ];
        self.taunt = Some((LINES[rand_int(0, LINES.len() as i32 - 1) as usize].to_string(), 1.8));
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
        self.add_points(SCORE_WARD);
        let (x, y) = center(tile);
        self.magic.shatter(x, y, WARLOCK_C);
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
        self.magic.fall(x, y, if idx == 0 { REVEAL_C } else { WIZ_ROBE2 });
        self.magic.corpse(x, y, 0, idx as u32, false, 1.1, if idx == 0 { REVEAL_C } else { WIZ_ROBE2 });
        blip::bot::add("deaths", 1.0);
        self.sfx(Sfx::Death);
        self.died_this_room = true;
        self.lives[idx] -= 1;
        const QUIPS: [&str; 5] = ["THAT WAS NOT A HEAL SPELL", "WIZARDS ARE NOT FIREPROOF", "REVIVE! REVIVE!", "OOPS", "MAGICK HAS ITS PRICE"];
        self.banner = Some((QUIPS[rand_int(0, QUIPS.len() as i32 - 1) as usize].to_string(), 1.6));
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

    fn add_points(&mut self, pts: i32) {
        self.sess.add_score(if self.double { pts * 2 } else { pts });
    }

    /// The last monster down: a whelp may burst in for its few seconds.
    fn maybe_whelp(&mut self) {
        if self.whelp_done || self.warlock.is_some() || self.sess.level < 2 || self.enemies.iter().any(|e| !e.gone) {
            return;
        }
        self.whelp_done = true;
        let far = (0..40)
            .filter_map(|_| self.random_open_cell())
            .map(|(c, r)| Cell { c, r })
            .max_by_key(|cl| (cl.c - self.wizards[0].tile.c).abs() + (cl.r - self.wizards[0].tile.r).abs());
        let Some(tile) = far else { return };
        self.whelp = Some(Whelp { tile, from: tile, lerp: 1.0, step_t: 0.4, dir: Dir::Left, left: WHELP_SECS, face_left: true });
        let (x, y) = center(tile);
        self.magic.summon(x, y, FIREBALL);
        self.magic.burst(x, y, 20, 140.0, FIREBALL, magic::Kind::Spark);
        self.banner = Some(("A DRAGON WHELP!".to_string(), 1.8));
        self.sfx(Sfx::Whelp);
        blip::bot::add("whelp", 1.0);
    }

    fn update_whelp(&mut self, dt: f32) {
        let Some(w) = &mut self.whelp else { return };
        w.left -= dt;
        w.lerp = (w.lerp + dt / WHELP_STEP).min(1.0);
        if w.left <= 0.0 {
            let (x, y) = center(w.tile);
            self.whelp = None;
            self.magic.burst(x, y, 16, 100.0, FIREBALL, magic::Kind::Mote);
            self.magic.wave(x, y, 30.0, 0.4, 2.0, FIREBALL, false);
            self.banner = Some(("IT FLEW AWAY".to_string(), 1.2));
            self.check_exit();
            return;
        }
        w.step_t -= dt;
        if w.step_t > 0.0 {
            return;
        }
        w.step_t = WHELP_STEP;
        // prefer straight on, never straight back unless cornered
        let back = match w.dir { Dir::Up => Dir::Down, Dir::Down => Dir::Up, Dir::Left => Dir::Right, Dir::Right => Dir::Left };
        let mut opts: Vec<Dir> = DIRS.iter().copied().filter(|&d| {
            let (dc, dr) = d.delta();
            d != back && self.walls.floor(w.tile.c + dc, w.tile.r + dr)
        }).collect();
        if opts.is_empty() {
            opts.push(back);
        }
        let d = if opts.contains(&w.dir) && rand_int(0, 3) > 0 { w.dir } else { opts[rand_int(0, opts.len() as i32 - 1) as usize] };
        let (dc, dr) = d.delta();
        w.from = w.tile;
        w.tile = Cell { c: w.tile.c + dc, r: w.tile.r + dr };
        w.lerp = 0.0;
        w.dir = d;
        match d {
            Dir::Left => w.face_left = true,
            Dir::Right => w.face_left = false,
            _ => {}
        }
        if rand_int(0, 4) == 0 {
            let (x, y) = center(w.from);
            self.magic.cinders(x, y, FIREBALL);
        }
    }

    fn whelp_px(&self) -> Option<(f32, f32)> {
        let w = self.whelp.as_ref()?;
        let a = center(w.from);
        let b = center(w.tile);
        Some((lerp(a.0, b.0, w.lerp), lerp(a.1, b.1, w.lerp)))
    }

    fn strike_whelp(&mut self) {
        let Some((x, y)) = self.whelp_px() else { return };
        self.whelp = None;
        self.add_points(SCORE_WHELP);
        self.double_next = true;
        self.magic.dissolve(x, y, FIREBALL);
        self.magic.corpse(x, y, 0, 4, false, 0.8, FIREBALL);
        self.magic.wave(x, y, 80.0, 0.7, 3.0, RUNE_CLEAN, true);
        self.fx.popup(x, y - 22.0, &format!("+{SCORE_WHELP}"), RUNE_CLEAN);
        self.banner = Some(("NEXT CHAMBER PAYS DOUBLE".to_string(), 2.0));
        self.sfx(Sfx::Chamber);
        blip::bot::add("whelp_hit", 1.0);
        self.check_exit();
    }

    /// The cellar's heartbeat quickens as the chamber empties.
    fn update_pulse(&mut self, dt: f32) {
        if self.warlock.is_some() {
            return;
        }
        let alive = self.enemies.iter().filter(|e| !e.gone).count();
        if alive == 0 && self.whelp.is_none() {
            return;
        }
        self.pulse_t -= dt;
        if self.pulse_t <= 0.0 {
            let k = alive as f32 / self.start_monsters as f32;
            self.pulse_t = if self.whelp.is_some() { 0.38 } else { 0.55 + 0.75 * k };
            self.sfx(Sfx::Pulse);
        }
    }

    fn check_exit(&mut self) {
        let monsters_alive = self.enemies.iter().any(|e| !e.gone) || self.whelp.is_some();
        let warlock_alive = self.warlock.as_ref().is_some_and(|wl| wl.wards > 0);
        let runes_left = self.runes.iter().any(|r| !r.cleansed);
        let was_open = self.exit_open;
        self.exit_open = !monsters_alive && !warlock_alive && !runes_left;
        if self.exit_open && !was_open {
            let (x, y) = center(self.exit);
            self.magic.open(x, y, RUNE_CLEAN);
            self.banner = Some(("THE WAY DOWN IS OPEN".to_string(), 2.2));
        }
    }

    fn finish_chamber(&mut self) {
        self.cleared_no_death += 1;
        if !self.died_this_room {
            let pts = SCORE_CLEAR_NODEATH;
            self.add_points(pts);
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
        self.room_stamp = self.room_stamp.wrapping_add(1);
        self.magic.clear();
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
            left: BONUS_TIME,
        });
        self.banner = Some(("SPELL CIRCLE".to_string(), 2.0));
        self.taunt = None;
        blip::bot::set("wing", self.cleared_no_death as f64);
    }

    fn update_bonus(&mut self, dt: f32) {
        self.fx.update(dt);
        self.update_ambience(dt);
        // Either wizard may stand on the stones.
        for i in 0..2 {
            if i == 1 && !self.active[1] { continue; }
            self.update_wizard(i, dt);
            self.wizards[i].reveal_cd = (self.wizards[i].reveal_cd - dt).max(0.0);
            if self.wizards[i].reveal_t > 0.0 {
                self.wizards[i].reveal_t = (self.wizards[i].reveal_t - dt).max(0.0);
            }
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
            b.left -= dt;
            (b.stones.clone(), b.seq.clone(), b.expect, b.left)
        };
        let (stones, seq, expect, left) = repeat_state;
        if left <= 0.0 {
            self.banner = Some(("THE CIRCLE FADES".to_string(), 1.6));
            self.leave_bonus();
            return;
        }

        let Some(who) = (0..2).find(|&i| (i == 0 || self.active[1]) && self.reveal_pressed(i) && self.wizards[i].reveal_cd <= 0.0) else {
            return;
        };
        self.wizards[who].reveal_cd = REVEAL_CD;
        self.wizards[who].reveal_t = 0.45;
        let (x, y) = self.wizard_px(who);
        self.fx.ring(x, y, CELL as f32 * 2.0, 0.4, REVEAL_C);
        let hit = stones.iter().position(|s| distance_lt(self.wizards[who].tile, *s, 1.5));
        match hit {
            Some(k) if k == seq[expect] => {
                let pts = 100 * (expect as i32 + 1);
                self.add_points(pts);
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
        // Back to play, or the next chamber is drawn but never updated.
        self.state = State::Play;
        self.sess.next_level();
        self.load_room();
    }

    // ---- transitions ------------------------------------------------------

    /// Embers rising from the torches and the spells' own update.
    fn update_ambience(&mut self, dt: f32) {
        self.magic.update(dt);
        // dust and ash hanging in the air, catching the light
        if rand_int(0, 1000) < (dt * 9000.0) as i32 {
            let x = rand_int(0, WIN_W) as f32;
            let y = (HUD_H + rand_int(0, ROWS as i32 * CELL)) as f32;
            let c = BlipColor { r: 1.0, g: 0.8, b: 0.55, a: 0.3 };
            self.magic.spawn(magic::Kind::Mote, x, y, rand_int(-8, 8) as f32, rand_int(-4, 6) as f32, 3.0, 0.7, c);
        }
        for &tc in &self.torches {
            if rand_int(0, 1000) < (dt * 2500.0) as i32 {
                let (x, y) = art::torch_flame(tc);
                let vx = rand_int(-20, 20) as f32;
                self.magic.spawn(magic::Kind::Mote, x, y - 4.0, vx, -30.0, 1.0 + rand_int(0, 80) as f32 / 100.0, 0.9, TORCH);
            }
        }
    }

    fn update_dead(&mut self, dt: f32) {
        self.fx.update(dt);
        self.update_ambience(dt);
        if self.dead_timer.tick(dt) {
            self.respawn();
        }
    }

    fn check_over_again(&mut self, dt: f32) {
        self.dead_timer.tick(dt);
        if !self.dead_timer.active() && self.fire_start() {
            self.start_game();
        }
    }

    /// Player one's bolt (F, Space, or the deck's first button) starts a game
    /// from the title or the game-over screen, for the coin it costs.
    fn fire_start(&self) -> bool {
        if !self.bolt_pressed(0) {
            return false;
        }
        web::spend_coin();
        true
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

/// Each chamber's name, shown on the panel.
fn room_name(level: i32) -> &'static str {
    let n = (level - 1).rem_euclid(6);
    let deep = (level - 1) / 6;
    if deep > 0 {
        return ["THE DEEP UNDERCROFT", "THE DROWNED CRYPT", "THE ASH KENNELS", "THE SHIFTING HALL", "THE BURNING GALLERY", "THE LOWER SANCTUM"][n as usize];
    }
    ["THE UNDERCROFT", "THE BONE CRYPT", "THE KENNELS", "THE HALL OF GATES", "THE GARGOYLE GALLERY", "THE WARLOCK'S SANCTUM"][n as usize]
}

/// A short original line the Warlock drops between chambers; once the
/// tower has been cleared he gets chattier (and stranger).
fn room_taunt(level: i32) -> String {
    let n = (level - 1).rem_euclid(6);
    if level > 6 {
        return [
            "Back again? Bring friends.",
            "Mind the friendly fire.",
            "I am NOT a vampire.",
            "Haste will not save you.",
            "Combine your elements, child.",
            "Again? Very well.",
        ][n as usize]
            .to_string();
    }
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

// ---- drawing (the rest lives in art.rs) ----------------------------------

/// The spell circle says what it wants: watch, then stand on each stone in turn and reveal.
fn draw_bonus_hint(blip: &Blip, g: &Game) {
    let Some(b) = &g.bonus else { return };
    let y = (HUD_H + ROWS as i32 * CELL - 46) as f32;
    let cx = WIN_W as f32 / 2.0;
    if b.showing {
        blip.draw_centered_outlined("WATCH THE STONES", cx, y, 2.0, RUNE_CLEAN, BLIP_BLACK);
        return;
    }
    blip.draw_centered_outlined("STEP ON THE STONES IN ORDER", cx, y, 2.0, RUNE_CLEAN, BLIP_BLACK);
    let press = web::controls().pick("PRESS Z ON EACH", "PRESS REVEAL ON EACH", "PRESS REVEAL ON EACH");
    let count = format!("{press}   {}/{}   {}", b.expect, b.seq.len(), b.left.ceil().max(0.0) as i32);
    blip.draw_centered_outlined(&count, cx, y + 18.0, 2.0, BLIP_WHITE, BLIP_BLACK);
}

fn draw_taunt(blip: &Blip, g: &Game) {
    draw_bonus_hint(blip, g);
    if let Some((text, t)) = &g.taunt {
        if *t > 0.0 {
            blip.draw_centered_outlined(text, WIN_W as f32 / 2.0, HUD_H as f32 + 10.0, 2.0, WARLOCK_C, BLIP_BLACK);
        }
    }
    if let Some((text, t)) = &g.banner {
        if *t > 0.0 {
            let k = (*t * 3.0).min(1.0);
            let sz = if text.chars().count() * 18 <= WIN_W as usize - 16 { 3.0 } else { 2.0 };
            blip.draw_centered_outlined(text, WIN_W as f32 / 2.0, (HUD_H + ROWS as i32 * CELL / 2 - 10) as f32, sz, BlipColor { a: k, ..RUNE_CLEAN }, BlipColor { a: k, ..BLIP_BLACK });
        }
    }
}

fn hud_lives(g: &Game) -> i32 {
    if g.two_player {
        g.lives[0] + g.lives[1]
    } else {
        g.lives[0]
    }
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
    blip.draw_game_over_note(&format!("WINGS CLEARED {}", g.cleared_no_death), BLIP_GRAY);
}

// ---- entry ----------------------------------------------------------------

fn conf() -> blip::macroquad::window::Conf {
    window_conf("WARLOCK", WIN_W, WIN_H)
}

#[blip::macroquad::main(conf)]
async fn main() {
    blip::bot::playthroughs(bot::PLAYTHROUGHS);
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
        fire: blip::audio::load_sound(blip::wav!("fire")).await,
        pulse: blip::audio::load_sound(blip::wav!("pulse")).await,
        whelp: blip::audio::load_sound(blip::wav!("whelp")).await,
        potion: blip::audio::load_sound(blip::wav!("potion")).await,
    });
    let mut music = Jukebox::new("WARLOCK", &["DUNGEON", "TOWER", "THEME", "DEPTHS"]);
    let mut art = art::Art::new();
    // the title shows the first cellar, torchlit and empty
    g.load_room();

    if blip.screenshot_mode {
        g.start_game();
        #[cfg(not(target_arch = "wasm32"))]
        scene::setup(&mut g);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let mut frame: u32 = 0;

    loop {
        #[cfg(not(target_arch = "wasm32"))]
        if blip.screenshot_mode {
            frame += 1;
            scene::tick(&mut g, frame);
        }
        #[allow(unused_mut)]
        let mut dt = blip.delta_time;
        #[cfg(not(target_arch = "wasm32"))]
        if blip.screenshot_mode && std::env::var_os("WARLOCK_SCENE").is_some() {
            dt = 1.0 / 60.0;
        }
        #[allow(unused_mut)]
        let mut t = blip::macroquad::time::get_time() as f32;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(at) = std::env::var("WARLOCK_T").ok().and_then(|v| v.parse::<f32>().ok()) {
            t = at + frame as f32 / 60.0;
        }

        if blip::bot::active() {
            bot::drive(&g);
        }

        let was_playing = g.state == State::Play;
        match g.state {
            State::Title => {
                g.update_ambience(dt);
                if key_pressed(BLIP_KEY_J) {
                    g.start_two();
                } else if g.fire_start() {
                    g.start_game();
                }
            }
            State::Play => {
                g.try_join();
                for i in 0..2 {
                    if !g.active[i] || g.in_intro() {
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
        art.draw_frame(&blip, &g, t);

        blip.next_frame(60).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaving_the_spell_circle_returns_to_play() {
        let mut g = Game::new();
        g.start_game();
        let level = g.sess.level;
        g.state = State::Bonus;
        g.start_bonus();
        g.leave_bonus();
        assert!(g.state == State::Play, "the spell circle never handed back to play");
        assert!(g.bonus.is_none());
        assert_eq!(g.sess.level, level + 1);
    }

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
    fn reveal_then_bolt_casts_a_lance_that_pierces() {
        let mut g = Game::new();
        g.start_game();
        g.intro = 0.0;
        // clear a row and line up three imps along it
        let w = g.wizards[0].tile;
        for c in w.c + 1..w.c + 5 {
            g.walls.set(c, w.r, false);
        }
        g.enemies.truncate(3);
        for (k, e) in g.enemies.iter_mut().enumerate() {
            e.kind = EnemyKind::Imp;
            e.tile = Cell { c: w.c + 2 + k as i32, r: w.r };
            e.from = e.tile;
            e.step_t = 1000.0;
        }
        g.wizards[0].facing = Dir::Right;
        g.reveal(0);
        g.cast_bolt(0);
        assert!(g.bolts[0].proj == Proj::Lance);
        for _ in 0..40 {
            g.update_projectiles(0.02);
        }
        assert!(g.enemies.iter().all(|e| e.gone), "the lance goes through all three");
        // a plain bolt stops at the first
        let mut g = Game::new();
        g.start_game();
        g.wizards[0].reveal_cd = 0.0;
        g.cast_bolt(0);
        assert!(g.bolts[0].proj == Proj::Bolt);
    }

    #[test]
    fn potions_haste_gild_and_add_a_hat() {
        let mut g = Game::new();
        g.start_game();
        g.intro = 0.0;
        let here = g.wizards[0].tile;
        g.drop_potion(here, PotionKind::Haste);
        g.update_potions(0.01);
        assert!(g.potions.is_empty(), "standing on it drinks it");
        assert!(g.wizards[0].haste > 0.0);
        let before = g.sess.score;
        g.drop_potion(here, PotionKind::Gold);
        g.update_potions(0.01);
        assert_eq!(g.sess.score, before + SCORE_GOLD);
        g.lives[0] = LIVES_MAX;
        g.drop_potion(here, PotionKind::Life);
        g.update_potions(0.01);
        assert_eq!(g.lives[0], LIVES_MAX, "hats are capped");
        // one left lying seeps away
        g.drop_potion(Cell { c: here.c + 50, r: here.r }, PotionKind::Gold);
        g.update_potions(POTION_SECS + 0.1);
        assert!(g.potions.is_empty());
    }

    #[test]
    fn j_on_the_title_starts_a_two_player_game() {
        let mut g = Game::new();
        g.start_two();
        assert!(g.state == State::Play);
        assert!(g.two_player);
        assert_eq!(g.active, [true, true]);
        assert_eq!(g.lives, [LIVES_START, LIVES_START]);
        assert_ne!(g.wizards[1].tile, g.wizards[0].tile);
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
            face_left: true,
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

    #[test]
    fn the_last_monster_brings_a_whelp_that_holds_the_stairs() {
        let mut g = Game::new();
        g.sess.level = 2;
        g.load_room();
        for r in &mut g.runes {
            r.cleansed = true;
        }
        let n = g.enemies.len();
        for i in 0..n {
            g.kill_enemy(i);
        }
        assert!(g.whelp.is_some(), "clearing a chamber from room 2 on should call a whelp");
        assert!(!g.exit_open, "the stairs wait while the whelp is loose");
        // left alone, it flies off and the stairs open
        for _ in 0..((WHELP_SECS + 1.0) * 60.0) as i32 {
            g.update_whelp(1.0 / 60.0);
        }
        assert!(g.whelp.is_none(), "the whelp should leave when its time runs out");
        assert!(g.exit_open, "the stairs open once the whelp has gone");
        assert!(!g.double_next, "an escaped whelp pays nothing");
    }

    #[test]
    fn striking_the_whelp_doubles_the_next_chamber() {
        let mut g = Game::new();
        g.sess.level = 2;
        g.load_room();
        for r in &mut g.runes {
            r.cleansed = true;
        }
        let n = g.enemies.len();
        for i in 0..n {
            g.kill_enemy(i);
        }
        g.strike_whelp();
        assert!(g.whelp.is_none());
        assert!(g.exit_open);
        g.finish_chamber();
        assert!(g.double, "the chamber after a struck whelp pays double");
        let before = g.sess.score;
        g.add_points(100);
        assert_eq!(g.sess.score - before, 200);
        g.finish_chamber();
        assert!(!g.double, "the double lasts one chamber");
    }

    #[test]
    fn no_whelp_in_the_first_chamber_or_the_sanctum() {
        for level in [1, 6] {
            let mut g = Game::new();
            g.sess.level = level;
            g.load_room();
            let n = g.enemies.len();
            for i in 0..n {
                g.kill_enemy(i);
            }
            assert!(g.whelp.is_none(), "level {level} should not call a whelp");
        }
    }
}
