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
mod monsters;
mod bonus;
mod hud;
mod potions;
use hud::*;
mod wizards;
mod foes;
use monsters::*;

use levels::*;
use magic::Magic;

use blip::input::{
    key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_D,
    BLIP_KEY_DOWN, BLIP_KEY_F, BLIP_KEY_G, BLIP_KEY_J, BLIP_KEY_K, BLIP_KEY_LEFT, BLIP_KEY_RIGHT,
    BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::{
    lerp, rand_int, web, window_conf, Blip, BlipColor, Fx, Jukebox,
    Session, Timer, BLIP_BLACK, BLIP_GRAY, BLIP_WHITE, BLIP_YELLOW,
};

// ---- layout ---------------------------------------------------------------
/// A chamber's title card: on screen this long, holding play for the first part.
const INTRO_SECS: f32 = 2.0;
/// How long a continue is offered at game over.
const CONTINUE_SECS: f32 = 9.0;
const INTRO_HOLD: f32 = 1.1;
const CELL: i32 = 32;
/// A monster or fireball touches a wizard when their drawn centres are this close (px): the bodies, not the tiles.
const TOUCH_R: f32 = CELL as f32 * 0.45;
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
/// Where something stepping from `from` to `to` is drawn, `t` of the way.
fn lerp_px(from: Cell, to: Cell, t: f32) -> (f32, f32) {
    let (a, b) = (center(from), center(to));
    (lerp(a.0, b.0, t), lerp(a.1, b.1, t))
}

fn center(c: Cell) -> (f32, f32) {
    ((c.c * CELL + CELL / 2) as f32, (HUD_H + c.r * CELL + CELL / 2) as f32)
}

// ---- entities ---------------------------------------------------------------
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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
    /// The current step's length (s): the body is drawn sliding from `from` to `tile` over it.
    step: f32,
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

blip::sounds!(Sounds { bolt, reveal, cleanse, kill, death, over, chamber, ward, warlock, fire, pulse, whelp, potion });

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
    /// The continue offered at game over, and the best score of the run before one was taken (a continue starts
    /// the score afresh; the run's best is what is reported).
    cont: blip::Continue,
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
            cont: blip::Continue::default(),
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
        self.runes = room.runes.iter().map(|&(c, r)| Rune { tile: Cell { c, r }, cleansed: false }).collect();
        self.enemies = self.spawn_monsters(room);
        self.start_monsters = self.enemies.len().max(1);
        self.intro = INTRO_SECS;
        self.double = self.double_next;
        self.double_next = false;
        self.whelp = None;
        self.whelp_done = false;
        self.potions.clear();
        self.pulse_t = 0.5;
        self.gates = room.gates.iter()
            .map(|&((ac, ar), (bc, br))| Gate { a: Cell { c: ac, r: ar }, b: Cell { c: bc, r: br }, cooldown: 0.0 })
            .collect();
        self.warlock = room.warlock.then(|| warlock_for(room));
        self.bolts.clear();
        self.fires.clear();
        self.sightings.clear();
        self.taunt = Some((room_taunt(self.sess.level), 3.0));
        self.died_this_room = false;
        self.place_wizard(0, self.player_spawn);
        if self.active[1] { self.place_wizard(1, self.second_spawn()); }
        self.torches = self.find_torches();
        self.room_stamp = self.room_stamp.wrapping_add(1);
        self.magic.clear();
        self.bonus = None;
    }

    /// The chamber's monsters where its plan puts them, each a little out of step with the others; later wings get
    /// extra imps to keep the corridors busy.
    fn spawn_monsters(&self, room: &Room) -> Vec<Enemy> {
        let mk = |c: i32, r: i32, kind: EnemyKind, dir: Dir| Enemy {
            kind,
            tile: Cell { c, r },
            from: Cell { c, r },
            lerp: 1.0,
            step_t: rand_int(0, 50) as f32 / 100.0,
            step: 1.0,
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
        for &(c, r) in room.imps { enemies.push(mk(c, r, EnemyKind::Imp, Dir::Right)); }
        for &(c, r) in room.wraiths { enemies.push(mk(c, r, EnemyKind::Wraith, Dir::Left)); }
        for &(c, r) in room.hounds { enemies.push(mk(c, r, EnemyKind::Spellhound, Dir::Up)); }
        for &(c, r) in room.gargoyles { enemies.push(mk(c, r, EnemyKind::Gargoyle, Dir::Down)); }
        let extra = (self.sess.level - 6).clamp(0, 5) as usize;
        for _ in 0..extra {
            if let Some((c, r)) = self.random_open_cell() { enemies.push(mk(c, r, EnemyKind::Imp, Dir::Right)); }
        }
        enemies
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
        // A playtest can start with fewer lives (BLIP_BOT_LIVES), to reach game over soon.
        let lives = blip::bot::num("LIVES").map_or(LIVES_START, |n| n as i32);
        self.sess.reset(lives);
        blip::bot::levels(levels::ROOMS.len());
        self.lives = [lives, LIVES_START];
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
        lerp_px(w.from, w.tile, w.lerp)
    }

    /// Is an active wizard standing on `tile` (other than the one moving)?
    fn wizard_occupies(&self, tile: Cell, exclude: usize) -> bool {
        (0..2).any(|i| i != exclude && self.active[i] && self.wizards[i].tile == tile)
    }

    fn enemy_px(&self, e: &Enemy) -> (f32, f32) {
        lerp_px(e.from, e.tile, e.lerp)
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

    // ---- touch, hurt, respawn, the exit -----------------------------------

    /// A monster only hurts a wizard once its body has actually reached her:
    /// tile bookkeeping alone would let a ghost kill from a whole cell away,
    /// so this measures the real, interpolated positions of both. Runs every
    /// frame, after wizards and enemies have moved.
    fn check_contact(&mut self) {
        let r = TOUCH_R;
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
                    victim = Some((k, e.kind));
                    break;
                }
            }
            if victim.is_some() {
                break;
            }
        }
        if let Some((k, kind)) = victim {
            let lives = self.lives[k];
            self.hit_wizard(k);
            if self.lives[k] < lives { blip::bot::add(&format!("death_{kind:?}").to_lowercase(), 1.0); }
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
        self.banner = Some((blip::rand_item(&QUIPS).unwrap_or_default().to_string(), 1.6));
        if self.lives[idx] > 0 {
            self.dead_index = Some(idx);
            self.dead_timer.start(RESPAWN_PROTECT);
            self.state = State::Dead;
        } else {
            self.active[idx] = false; // downed: out until a partner clears the chamber
            if self.active.iter().all(|&a| !a) {
                self.cont.start(CONTINUE_SECS);
                self.state = State::Over;
                // A new best (over the run's continues) rings the chamber-cleared chord instead of the knell.
                self.sfx(if self.sess.beats_best() { Sfx::Chamber } else { Sfx::Over });
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

    // ---- ambience and the screens between --------------------------------

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

    /// Game over offers a continue: a bolt within CONTINUE_SECS tries the same chamber again with full lives (and
    /// the score afresh); otherwise the run's best is reported and the title returns.
    fn check_over_again(&mut self, dt: f32) {
        match self.cont.update(dt, self.bolt_pressed(0)) {
            Some(true) => {
                self.sess.play_on(LIVES_START);
                self.lives = [LIVES_START, LIVES_START];
                self.active = [true, self.two_player];
                self.dead_index = None;
                self.load_room();
                self.state = State::Play;
            }
            Some(false) => {
                if !self.two_player {
                    self.sess.report();
                }
                self.state = State::Title;
            }
            None => if self.cont.ticked() { self.sfx(Sfx::Pulse); },
        }
    }

    /// The title, over the first cellar's ambience: J starts a game for two, a bolt (or a coin) one for one.
    fn update_title(&mut self, dt: f32) {
        self.update_ambience(dt);
        if key_pressed(BLIP_KEY_J) {
            self.start_two();
        } else if self.fire_start() {
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

/// The Warlock in the sanctum, materializing in the first of his spell circles, warded.
fn warlock_for(room: &Room) -> Warlock {
    let circles: Vec<Cell> = room.warlock_circles.iter().map(|&(c, r)| Cell { c, r }).collect();
    Warlock {
        sealed: vec![false; circles.len()],
        circle: 0,
        wards: room.warlock_wards,
        circles,
        state: WState::Materializing,
        timer: 0.7,
        aim_dir: Dir::Right,
        aim_from: Cell { c: 0, r: 0 },
    }
}

fn distance_lt(a: Cell, b: Cell, tiles: f32) -> bool {
    let dc = (a.c - b.c) as f32;
    let dr = (a.r - b.r) as f32;
    dc * dc + dr * dr < tiles * tiles
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
    g.sfx = Some(Sounds::load().await);
    let mut music = Jukebox::new("warlock", &["DUNGEON", "TOWER", "THEME", "DEPTHS"]);
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
        let mut t = blip::now();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(at) = std::env::var("WARLOCK_T").ok().and_then(|v| v.parse::<f32>().ok()) {
            t = at + frame as f32 / 60.0;
        }

        if blip::bot::active() {
            bot::drive(&g);
        }

        let was_playing = g.state == State::Play;
        match g.state {
            State::Title => g.update_title(dt),
            State::Play => {
                g.try_join();
                g.cast_spells();
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

        blip.next_frame().await;
    }
}

#[cfg(test)]
mod tests;
