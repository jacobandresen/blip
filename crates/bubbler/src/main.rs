//! Bubbler — a tribute to Taito's Bubble Bobble (1986).
//!
//! One screen, two little dragons, a sky full of monsters. Blow a bubble
//! into a monster to trap it, then pop the bubble to finish it off; pop a
//! cluster at once and the points double down the chain. Hold jump to ride
//! bubbles like stepping stones. Take too long and the monsters get angry,
//! and then something worse comes looking for you.

use blip::input::{key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_D, BLIP_KEY_F,
    BLIP_KEY_G, BLIP_KEY_J, BLIP_KEY_K, BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_SPACE, BLIP_KEY_UP,
    BLIP_KEY_W};
use blip::macroquad::input::KeyCode;
use blip::macroquad::math::vec2;
use blip::macroquad::shapes::{draw_ellipse, draw_triangle};
use blip::{play_sfx, Jukebox, play_sfx_volume, web, window_conf, Blip, BlipColor};

// ---- layout -----------------------------------------------------------
const TILE: f32 = 24.0;
const COLS: usize = 26;
const ROWS: usize = 24;
const HUD: f32 = 24.0;
const WIN_W: i32 = 624;
const WIN_H: i32 = 600; // HUD + 24 rows
/// Screen text was laid out for the 456px screen; this centres it on this one.
const DY: f32 = (WIN_H as f32 - 456.0) / 2.0;
const GAP: (usize, usize) = (11, 14); // the hole in the floor and ceiling you fall through

// ---- feel ---------------------------------------------------------------
const GRAV: f32 = 950.0;
const MAX_FALL: f32 = 300.0; // a floaty fall: you steer your landings
const JUMP_V: f32 = 470.0;   // ~4.8 tiles, one platform up with room to spare
const RUN: f32 = 118.0;
const P_W: f32 = 20.0;
const P_H: f32 = 22.0;
const E_W: f32 = 20.0;
const E_H: f32 = 20.0;
const LIVES: i32 = 5;
/// Everyone is drawn this much bigger than their hit box: soft toys, easy
/// to see; the box stays small, so a near miss is a miss.
const TOY_SCALE: f32 = 1.4;
const RESPAWN_SAFE: f32 = 3.5;

// ---- bubbles --------------------------------------------------------------
const BUB_R: f32 = 12.0;
const SHOOT_V: f32 = 340.0;
const SHOOT_T: f32 = 0.4;    // the blown bubble traps only while it is still flying
const RISE: f32 = 44.0;
const RISE_TRAPPED: f32 = 62.0; // catches reach the ceiling cluster sooner, where chains happen
const CHAIN_REACH: f32 = 1.35;  // bubbles this many diameters apart pop together
const FREE_LIFE: f32 = 9.0;
const TRAP_LIFE: f32 = 10.0; // round 1; then the monster breaks out, angry
const BLOW_CD: f32 = 0.24;
const MAX_BUBBLES: usize = 36;
const TOP_Y: f32 = HUD + TILE + BUB_R + 4.0;

// ---- pressure -----------------------------------------------------------
const HURRY_AT: f32 = 70.0; // round 1: little players take their time
const SKULL_AT: f32 = 95.0;

// Each round is harder than the last: monsters move faster, break out of
// a bubble sooner, get angry and bring the skull earlier. From round 3
// walkers throw rocks along their platform; from round 4 angry ghosts
// spit sparks at you.
/// The ramp stops at round 5: the rounds after it are new places, not harder
/// ones (the autopilot lost all five lives on rounds 7 and 8 when it ran on).
const RAMP_TOP: usize = 4;
fn monster_pace(round: usize) -> f32 { 0.8 + 0.04 * round.min(RAMP_TOP) as f32 }
fn trap_life(round: usize) -> f32 { (TRAP_LIFE - 0.5 * round.min(RAMP_TOP) as f32).max(7.0) }
fn hurry_at(round: usize) -> f32 { HURRY_AT - 2.5 * round.min(RAMP_TOP) as f32 }
fn skull_at(round: usize) -> f32 { SKULL_AT - 3.5 * round.min(RAMP_TOP) as f32 }
const ROCKS_FROM: usize = 3;  // round index
const SPARKS_FROM: usize = 4;
const ROCK_V: f32 = 170.0;
const ROCK_WINDUP: f32 = 0.6;
/// How long a monster crouches before it jumps: long enough to see.
const JUMP_TELL: f32 = 0.22;
const EXTEND_MAX: i32 = 7; // lives a round clear can top a player up to
const SPARK_V: f32 = 95.0;

const ROUNDS: usize = 8;

// '#' block, A / B player spawns, w walker, h hopper, g ghost.
const LEVELS: [[&str; ROWS]; ROUNDS] = [
    [
        "###########....###########",
        "#........................#",
        "#........................#",
        "#........................#",
        "#........................#",
        "#........................#",
        "#........................#",
        "#........########........#",
        "#........................#",
        "#........................#",
        "#..w..................w..#",
        "#.#######........#######.#",
        "#........................#",
        "#........................#",
        "#........w......w........#",
        "#......############......#",
        "#........................#",
        "#........................#",
        "#...w................w...#",
        "#..#######......#######..#",
        "#........................#",
        "#........................#",
        "#A......................B#",
        "###########....###########",
    ],
    [
        "###########....###########",
        "#........................#",
        "#........................#",
        "#.....g............g.....#",
        "#........................#",
        "#........................#",
        "#........................#",
        "#######............#######",
        "#........................#",
        "#........................#",
        "#.........w....w.........#",
        "#......############......#",
        "#........................#",
        "#........................#",
        "#..h..................h..#",
        "#########........#########",
        "#........................#",
        "#........................#",
        "#.......w........w.......#",
        "#....################....#",
        "#........................#",
        "#........................#",
        "#A......................B#",
        "###########....###########",
    ],
    [
        "###########....###########",
        "#........................#",
        "#........................#",
        "#...........g............#",
        "#........................#",
        "#........................#",
        "#........w......w........#",
        "#.....##############.....#",
        "#........................#",
        "#........................#",
        "#..h..................h..#",
        "##########......##########",
        "#........................#",
        "#........................#",
        "#.....w............w.....#",
        "#...##################...#",
        "#........................#",
        "#........................#",
        "#.w.........h..........w.#",
        "######...########...######",
        "#........................#",
        "#........................#",
        "#A......................B#",
        "###########....###########",
    ],
    [
        "###########....###########",
        "#........................#",
        "#...........g............#",
        "#...g................g...#",
        "#........................#",
        "#........................#",
        "#..h.................h...#",
        "#########........#########",
        "#........................#",
        "#........................#",
        "#......w..........w......#",
        "#...##################...#",
        "#........................#",
        "#........................#",
        "#..w........h........w...#",
        "#######...######...#######",
        "#........................#",
        "#........................#",
        "#....w.............w.....#",
        "#..#########..#########..#",
        "#........................#",
        "#........................#",
        "#A......................B#",
        "###########....###########",
    ],
    [
        "###########....###########",
        "#........................#",
        "#...........g............#",
        "#...g................g...#",
        "#........................#",
        "#........................#",
        "#.......w........w.......#",
        "#....################....#",
        "#........................#",
        "#........................#",
        "#..w........h.........w..#",
        "#######...######...#######",
        "#........................#",
        "#........................#",
        "#....w...h..........w....#",
        "#...########..########...#",
        "#........................#",
        "#........................#",
        "#.w.......w....w.......w.#",
        "#####...##########...#####",
        "#........................#",
        "#........................#",
        "#A......................B#",
        "###########....###########",
    ],
    [
        "###########....###########",
        "#........................#",
        "#...........g............#",
        "#...g................g...#",
        "#........................#",
        "#........................#",
        "#....h......w.......h....#",
        "#..######.######.######..#",
        "#........................#",
        "#........................#",
        "#...w................w...#",
        "##########......##########",
        "#........................#",
        "#........................#",
        "#.......w........h.......#",
        "#....#######..#######....#",
        "#........................#",
        "#........................#",
        "#..w........w.........w..#",
        "#######...######...#######",
        "#........................#",
        "#........................#",
        "#A......................B#",
        "###########....###########",
    ],
    [
        "###########....###########",
        "#........................#",
        "#.....g............g.....#",
        "#........................#",
        "#........................#",
        "#........................#",
        "#...w.......h........w...#",
        "########..######..########",
        "#........................#",
        "#........................#",
        "#.......h........w.......#",
        "#....#######..#######....#",
        "#........................#",
        "#........................#",
        "#..w........w.........h..#",
        "######...########...######",
        "#........................#",
        "#........................#",
        "#.....w...........w......#",
        "#..########....########..#",
        "#........................#",
        "#........................#",
        "#A......................B#",
        "###########....###########",
    ],
    [
        "###########....###########",
        "#........................#",
        "#...........g............#",
        "#....g..............g....#",
        "#........................#",
        "#........................#",
        "#.........w....w.........#",
        "#.......##########.......#",
        "#........................#",
        "#........................#",
        "#...w................h...#",
        "#.#######........#######.#",
        "#........................#",
        "#........................#",
        "#........w......h........#",
        "#.....##############.....#",
        "#........................#",
        "#........................#",
        "#....w..............w....#",
        "#..#######......#######..#",
        "#........................#",
        "#........................#",
        "#A......................B#",
        "###########....###########",
    ],
];

/// Per round: sky top, sky bottom, block, block light, block dark, bokeh.
struct Palette { sky0: (u8, u8, u8), sky1: (u8, u8, u8), block: (u8, u8, u8), light: (u8, u8, u8), dark: (u8, u8, u8), glow: (u8, u8, u8) }
const PALETTES: [Palette; ROUNDS] = [
    Palette { sky0: (18, 12, 52), sky1: (48, 20, 84), block: (240, 120, 180), light: (255, 190, 225), dark: (160, 60, 120), glow: (255, 140, 220) },
    Palette { sky0: (6, 24, 50), sky1: (10, 60, 90), block: (80, 200, 230), light: (170, 240, 255), dark: (30, 110, 150), glow: (120, 230, 255) },
    Palette { sky0: (30, 16, 8), sky1: (70, 36, 16), block: (250, 180, 60), light: (255, 230, 150), dark: (170, 100, 20), glow: (255, 200, 90) },
    Palette { sky0: (8, 30, 18), sky1: (16, 70, 40), block: (110, 220, 110), light: (190, 255, 180), dark: (40, 130, 60), glow: (150, 255, 150) },
    Palette { sky0: (26, 6, 30), sky1: (60, 10, 50), block: (190, 110, 250), light: (230, 190, 255), dark: (110, 50, 170), glow: (220, 150, 255) },
    Palette { sky0: (36, 10, 18), sky1: (84, 26, 40), block: (255, 120, 120), light: (255, 196, 186), dark: (170, 56, 70), glow: (255, 150, 140) },
    Palette { sky0: (24, 22, 6), sky1: (60, 56, 16), block: (226, 222, 84), light: (252, 250, 176), dark: (136, 130, 34), glow: (244, 240, 124) },
    Palette { sky0: (6, 22, 26), sky1: (16, 54, 60), block: (150, 240, 210), light: (222, 255, 240), dark: (60, 150, 130), glow: (170, 255, 230) },
];

fn col(c: (u8, u8, u8), a: f32) -> BlipColor { BlipColor::new(c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0, a) }
fn rgba(r: f32, g: f32, b: f32, a: f32) -> BlipColor { BlipColor::new(r, g, b, a) }
fn now() -> f32 { blip::macroquad::time::get_time() as f32 }

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Intro, Play, Clear, Over, Won }

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Kind { Walker, Hopper, Ghost }

#[derive(Default, Clone, Copy)]
struct Input { left: bool, right: bool, jump: bool, jump_held: bool, blow: bool }

struct KeySet { left: &'static [KeyCode], right: &'static [KeyCode], jump: &'static [KeyCode], blow: &'static [KeyCode] }
// Alone, player one has every key; with a partner the arrows are theirs.
const SOLO: KeySet = KeySet {
    left: &[BLIP_KEY_A, BLIP_KEY_LEFT], right: &[BLIP_KEY_D, BLIP_KEY_RIGHT],
    jump: &[BLIP_KEY_W, BLIP_KEY_UP, BLIP_KEY_G, BLIP_KEY_BUTTON2], blow: &[BLIP_KEY_F, BLIP_KEY_SPACE],
};
const P1_KEYS: KeySet = KeySet {
    left: &[BLIP_KEY_A], right: &[BLIP_KEY_D], jump: &[BLIP_KEY_W, BLIP_KEY_G, BLIP_KEY_BUTTON2], blow: &[BLIP_KEY_F, BLIP_KEY_SPACE],
};
const P2_KEYS: KeySet = KeySet {
    left: &[BLIP_KEY_LEFT], right: &[BLIP_KEY_RIGHT], jump: &[BLIP_KEY_UP, BLIP_KEY_K], blow: &[BLIP_KEY_J],
};
fn read(k: &KeySet) -> Input {
    let any_held = |ks: &[KeyCode]| ks.iter().any(|c| key_held(*c) || key_pressed(*c));
    let any_pressed = |ks: &[KeyCode]| ks.iter().any(|c| key_pressed(*c));
    Input { left: any_held(k.left), right: any_held(k.right), jump: any_pressed(k.jump),
            jump_held: any_held(k.jump), blow: any_pressed(k.blow) }
}

struct Player {
    joined: bool,   // in this game
    alive: bool,    // on the board (not between lives, not out)
    x: f32, y: f32, vx: f32, vy: f32,
    face: f32,
    on_ground: bool,
    lives: i32,
    score: i32,
    blow_cd: f32,
    mouth: f32,     // > 0 while the cheeks puff out a bubble
    /// Eating a candy: seconds since it was picked up (negative when not),
    /// and the candy itself, flying in from where it lay.
    eat: f32,
    eaten: Fruit,
    safe: f32,      // respawn protection
    dead_t: f32,    // > 0 while the losing-a-life animation plays
    squash: f32,    // landing squash / jump stretch, eased back to 1
    walk: f32,
    blink: f32,
    spawn: (f32, f32),
}

impl Player {
    fn new(spawn: (f32, f32), face: f32) -> Self {
        Self { joined: false, alive: false, x: spawn.0, y: spawn.1, vx: 0.0, vy: 0.0, face, on_ground: false,
            lives: LIVES, score: 0, blow_cd: 0.0, mouth: 0.0, safe: 0.0, dead_t: 0.0, squash: 1.0, walk: 0.0,
            eat: -1.0, eaten: Fruit { x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, kind: 0, t: 0.0, on_ground: true, active: false },
            blink: blip::rand_range_f32(1.0, 4.0), spawn }
    }
    fn place(&mut self) {
        self.x = self.spawn.0; self.y = self.spawn.1;
        self.vx = 0.0; self.vy = 0.0; self.on_ground = false; self.alive = true; self.dead_t = 0.0;
        self.safe = RESPAWN_SAFE;
    }
    fn cx(&self) -> f32 { self.x + P_W / 2.0 }
    fn cy(&self) -> f32 { self.y + P_H / 2.0 }
}

#[derive(Clone, Copy)]
struct Enemy { kind: Kind, x: f32, y: f32, vx: f32, vy: f32, dir: f32, on_ground: bool, angry: bool,
    t: f32, jump_cd: f32, edge_cd: f32, active: bool, pop_in: f32, shot_cd: f32, windup: f32,
    /// Crouching before a jump (seconds left), and the leap (vx, vy) it springs into.
    crouch: f32, leap: (f32, f32) }

#[derive(Clone, Copy, PartialEq)]
enum Phase { Shoot, Float, Top }

#[derive(Clone, Copy)]
struct Bubble { x: f32, y: f32, vx: f32, age: f32, phase: Phase, owner: usize,
    trapped: Option<(Kind, bool)>, active: bool, wob: f32, squish: f32 }

#[derive(Clone, Copy)]
struct Fruit { x: f32, y: f32, vx: f32, vy: f32, kind: usize, t: f32, on_ground: bool, active: bool }

#[derive(Clone, Copy)]
struct ParticleStyle { c: BlipColor, size: f32, star: bool, ring: bool }

#[derive(Clone, Copy)]
struct Popup { x: f32, y: f32, t: f32, value: i32, c: BlipColor }

struct Skull { active: bool, x: f32, y: f32, speed: f32 }

/// A monster's missile: a rock rolled along a platform, or a ghost's spark.
#[derive(Clone, Copy)]
struct Shot { x: f32, y: f32, vx: f32, vy: f32, t: f32, spark: bool, active: bool }

/// Tallies for a playtest run (printed by the native autopilot).
#[derive(Default, Clone)]
struct Stats { traps: u32, escapes: u32, pops: u32, kills: u32, chains: [u32; 6], fruit: u32,
    deaths_enemy: u32, deaths_skull: u32, deaths_by: [u32; 3], deaths_round: [u32; ROUNDS], hurries: u32, skulls: u32, bounces: u32 }

struct Game {
    stats: Stats,
    chain: (i32, f32), // the last multi-catch and how long its banner has left
    state: State,
    round: usize,
    tiles: [[bool; COLS]; ROWS],
    p: [Player; 2],
    enemies: Vec<Enemy>,
    bubbles: Vec<Bubble>,
    fruits: Vec<Fruit>,
    parts: blip::EffectParticles<ParticleStyle>,
    pops: Vec<Popup>,
    skull: Skull,
    shots: Vec<Shot>,
    round_t: f32,
    state_t: f32,
    hurry: bool,
    shake: f32,
    two_up: bool, // player two is at the cabinet (joined this game)
    extend: bool, // the last clear gave a life back
}

/// A one-player game's score goes to the high-score board; a game player
/// two joined sets none.
fn report_score(g: &Game) {
    if !g.two_up { web::report_score(g.p[0].score); }
}

fn level_spawns(round: usize) -> ((f32, f32), (f32, f32), Vec<(Kind, f32, f32)>) {
    let mut a = (TILE + 2.0, HUD + 16.0 * TILE + TILE - P_H);
    let mut b = a;
    let mut es = Vec::new();
    for (r, row) in LEVELS[round].iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            let x = c as f32 * TILE + 2.0;
            let foot = HUD + (r as f32 + 1.0) * TILE;
            match ch {
                'A' => a = (x, foot - P_H),
                'B' => b = (x, foot - P_H),
                'w' => es.push((Kind::Walker, x, foot - E_H)),
                'h' => es.push((Kind::Hopper, x, foot - E_H)),
                'g' => es.push((Kind::Ghost, x, foot - E_H)),
                _ => {}
            }
        }
    }
    (a, b, es)
}

impl Game {
    fn new() -> Self {
        let (a, b, _) = level_spawns(0);
        let mut g = Self {
            stats: Stats::default(), chain: (0, 0.0), state: State::Title, round: 0, tiles: [[false; COLS]; ROWS],
            p: [Player::new(a, 1.0), Player::new(b, -1.0)],
            enemies: Vec::new(), bubbles: Vec::new(), fruits: Vec::new(), parts: blip::EffectParticles::new(60.0, 2.0), pops: Vec::new(),
            skull: Skull { active: false, x: 0.0, y: 0.0, speed: 0.0 },
            shots: Vec::new(),
            round_t: 0.0, state_t: 0.0, hurry: false, shake: 0.0, two_up: false, extend: false,
        };
        g.load_round(0);
        g
    }

    fn load_round(&mut self, round: usize) {
        self.round = round;
        for (r, row) in LEVELS[round].iter().enumerate() {
            for (c, ch) in row.chars().enumerate() { self.tiles[r][c] = ch == '#'; }
        }
        let (a, b, es) = level_spawns(round);
        self.p[0].spawn = a;
        self.p[1].spawn = b;
        self.enemies = es.into_iter().map(|(kind, x, y)| Enemy {
            kind, x, y, vx: 0.0, vy: 0.0, dir: if x < WIN_W as f32 / 2.0 { 1.0 } else { -1.0 },
            on_ground: false, angry: false, t: blip::rand_range_f32(0.0, 3.0), jump_cd: blip::rand_range_f32(0.8, 2.0), edge_cd: 0.0,
            active: true, pop_in: 0.0, shot_cd: blip::rand_range_f32(2.5, 4.5), windup: 0.0, crouch: 0.0, leap: (0.0, 0.0),
        }).collect();
        for e in self.enemies.iter_mut() {
            if e.kind == Kind::Ghost { e.vx = e.dir * 62.0; e.vy = 62.0; }
        }
        self.bubbles.clear();
        self.fruits.clear();
        self.shots.clear();
        self.skull.active = false;
        self.round_t = 0.0;
        self.hurry = false;
        for i in 0..2 {
            self.p[i].face = if i == 0 { 1.0 } else { -1.0 };
            if self.p[i].joined && self.p[i].lives > 0 { self.p[i].place(); }
        }
    }

    fn start(&mut self, two: bool) {
        for i in 0..2 {
            self.p[i].joined = i == 0 || two;
            self.p[i].lives = LIVES;
            self.p[i].score = 0;
            self.p[i].alive = false;
        }
        self.two_up = two;
        web::set_players(if two { 1 } else { 2 });
        // Playtest, native only: BUBBLER_ROUND=1..8 starts at that round.
        #[cfg(not(target_arch = "wasm32"))]
        let first = std::env::var("BUBBLER_ROUND").ok().and_then(|v| v.parse::<usize>().ok())
            .map_or(0, |r| r.clamp(1, ROUNDS) - 1);
        #[cfg(target_arch = "wasm32")]
        let first = 0;
        self.load_round(first);
        self.state = State::Intro;
        self.state_t = 0.0;
    }

    fn join_p2(&mut self) {
        let p = &mut self.p[1];
        p.joined = true;
        p.lives = LIVES;
        p.score = 0;
        p.place();
        self.two_up = true;
        web::set_players(1);
        web::spend_coin(); // player two's coin: two players, two coins
    }

    fn platform(&self, c: i32, r: i32) -> bool {
        if c < 0 || c >= COLS as i32 || r < 1 || r >= ROWS as i32 { return false; }
        self.tiles[r as usize][c as usize]
    }

    /// Anything under the span x0..x1 at the row whose top edge is `foot`?
    fn floor_at(&self, x0: f32, x1: f32, foot: f32) -> bool {
        let r = ((foot - HUD) / TILE).round() as i32;
        if ((HUD + r as f32 * TILE) - foot).abs() > 0.5 { return false; }
        let (c0, c1) = ((x0 / TILE).floor() as i32, ((x1 - 0.01) / TILE).floor() as i32);
        (c0..=c1).any(|c| self.platform(c, r))
    }

    /// Integrate a body with one-way platforms: it passes up through
    /// blocks and lands on their tops. Returns (on_ground, hit_wall).
    fn step_body(&self, x: &mut f32, y: &mut f32, vx: &mut f32, vy: &mut f32, w: f32, h: f32, dt: f32) -> (bool, bool) {
        *vy = (*vy + GRAV * dt).min(MAX_FALL);
        *x += *vx * dt;
        let mut wall = false;
        let (lo, hi) = (TILE, WIN_W as f32 - TILE - w);
        if *x < lo { *x = lo; wall = true; }
        if *x > hi { *x = hi; wall = true; }
        let old_foot = *y + h;
        *y += *vy * dt;
        let mut ground = false;
        if *vy >= 0.0 {
            let foot = *y + h;
            let r0 = ((old_foot - HUD) / TILE).ceil() as i32;
            let r1 = ((foot - HUD) / TILE).floor() as i32;
            for r in r0..=r1 {
                let top = HUD + r as f32 * TILE;
                let (c0, c1) = (((*x + 3.0) / TILE).floor() as i32, ((*x + w - 3.0) / TILE).floor() as i32);
                if (c0..=c1).any(|c| self.platform(c, r)) {
                    *y = top - h;
                    *vy = 0.0;
                    ground = true;
                    break;
                }
            }
        }
        // the ceiling (except the hole in it)
        let in_gap = (*x + w / 2.0) >= GAP.0 as f32 * TILE && (*x + w / 2.0) < (GAP.1 + 1) as f32 * TILE;
        if !in_gap && *y < HUD + TILE { *y = HUD + TILE; if *vy < 0.0 { *vy = 0.0; } }
        // down the hole in the floor, back in through the one in the ceiling
        if *y > WIN_H as f32 { *y = HUD - h; }
        (ground, wall)
    }

    /// An expanding shockwave ring.
    fn ring(&mut self, x: f32, y: f32, c: BlipColor, size: f32) {
        self.parts.push(blip::EffectParticle { x, y, vx: 0.0, vy: 0.0, life: 0.35, max_life: 0.35,
            data: ParticleStyle { c, size, star: false, ring: true } });
    }

    fn burst(&mut self, x: f32, y: f32, n: usize, c: BlipColor, speed: f32, star: bool) {
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU + blip::rand_range_f32(-0.3, 0.3);
            let s = speed * blip::rand_range_f32(0.5, 1.0);
            let life = blip::rand_range_f32(0.35, 0.7);
            self.parts.push(blip::EffectParticle { x, y, vx: a.cos() * s, vy: a.sin() * s, life, max_life: life,
                data: ParticleStyle { c,
                    size: if star { blip::rand_range_f32(2.5, 4.0) } else { blip::rand_range_f32(1.2, 2.6) },
                    star, ring: false } });
        }
    }
}

struct Sounds { blow: blip::BlipSound, pop: blip::BlipSound, trap: blip::BlipSound, kill: blip::BlipSound,
    fruit: blip::BlipSound, jump: blip::BlipSound, lose: blip::BlipSound, bounce: blip::BlipSound,
    sparkle: blip::BlipSound, round: blip::BlipSound, clear: blip::BlipSound, hurry: blip::BlipSound,
    over: blip::BlipSound, won: blip::BlipSound }

// ---- update ------------------------------------------------------------

fn update_players(g: &mut Game, inp: [Input; 2], dt: f32, sfx: &Sounds) {
    for i in 0..2 {
        if !g.p[i].joined { continue; }
        if g.p[i].dead_t > 0.0 {
            let p = &mut g.p[i];
            p.dead_t -= dt;
            p.vy += GRAV * 0.5 * dt;
            p.y += p.vy * dt;
            if p.dead_t <= 0.0 {
                p.lives -= 1;
                if p.lives > 0 { p.place(); } else { p.alive = false; }
            }
            continue;
        }
        if !g.p[i].alive { continue; }
        let k = inp[i];
        let (mut x, mut y, mut vx, mut vy) = (g.p[i].x, g.p[i].y, g.p[i].vx, g.p[i].vy);
        let dir = k.right as i32 as f32 - k.left as i32 as f32;
        vx += (dir * RUN - vx) * (18.0 * dt).min(1.0);
        if dir != 0.0 { g.p[i].face = dir; }
        if k.jump && g.p[i].on_ground {
            vy = -JUMP_V;
            g.p[i].squash = 1.35;
            let (fx, fy) = (x + P_W / 2.0, y + P_H);
            for s in [-1.0, 1.0] {
                g.parts.push(blip::EffectParticle { x: fx, y: fy, vx: s * blip::rand_range_f32(40.0, 70.0), vy: -blip::rand_range_f32(5.0, 20.0),
                    life: 0.25, max_life: 0.25, data: ParticleStyle { c: rgba(1.0, 1.0, 1.0, 0.45), size: 1.8, star: false, ring: false } });
            }
            play_sfx_volume(&sfx.jump, 0.5);
        }
        let was_air = !g.p[i].on_ground;
        let fall_speed = vy;
        let (ground, _) = g.step_body(&mut x, &mut y, &mut vx, &mut vy, P_W, P_H, dt);
        if ground && was_air && fall_speed > 150.0 {
            g.p[i].squash = 0.7;
            for s in [-1.0, 1.0] {
                g.parts.push(blip::EffectParticle { x: x + P_W / 2.0, y: y + P_H, vx: s * blip::rand_range_f32(30.0, 60.0), vy: -blip::rand_range_f32(10.0, 30.0),
                    life: 0.3, max_life: 0.3, data: ParticleStyle { c: rgba(1.0, 1.0, 1.0, 0.5), size: 2.0, star: false, ring: false } });
            }
        }
        let p = &mut g.p[i];
        p.x = x; p.y = y; p.vx = vx; p.vy = vy; p.on_ground = ground;
        p.squash += (1.0 - p.squash) * (12.0 * dt).min(1.0);
        p.walk += vx.abs() * dt * 0.09;
        p.blow_cd -= dt;
        p.mouth = (p.mouth - dt).max(0.0);
        if p.eat >= 0.0 {
            p.eat += dt;
            if p.eat > EAT_SECS { p.eat = -1.0; }
        }
        p.safe = (p.safe - dt).max(0.0);
        p.blink -= dt;
        if p.blink < -0.12 { p.blink = blip::rand_range_f32(2.0, 4.5); }
        if k.blow && p.blow_cd <= 0.0 && g.bubbles.len() < MAX_BUBBLES {
            p.blow_cd = BLOW_CD;
            p.mouth = 0.18;
            let (bx, by, f) = (p.cx() + p.face * 14.0, p.cy() - 1.0, p.face);
            g.bubbles.push(Bubble { x: bx, y: by, vx: f * SHOOT_V, age: 0.0, phase: Phase::Shoot, owner: i,
                trapped: None, active: true, wob: blip::rand_range_f32(0.0, 6.0), squish: 0.0 });
            play_sfx_volume(&sfx.blow, 0.6);
        }
    }
}

fn nearest_player(g: &Game, x: f32, y: f32) -> Option<(f32, f32)> {
    g.p.iter().filter(|p| p.joined && p.alive && p.dead_t <= 0.0)
        .map(|p| (p.cx(), p.cy()))
        .min_by(|a, b| ((a.0 - x).hypot(a.1 - y)).total_cmp(&(b.0 - x).hypot(b.1 - y)))
}

fn update_enemies(g: &mut Game, dt: f32) {
    for i in 0..g.enemies.len() {
        let mut e = g.enemies[i];
        if !e.active { continue; }
        e.t += dt;
        e.pop_in = (e.pop_in + dt * 3.0).min(1.0);
        let fast = if e.angry { 1.35 } else { 1.0 } * monster_pace(g.round);
        let target = nearest_player(g, e.x + E_W / 2.0, e.y + E_H / 2.0);
        // A jump is told before it happens: the monster crouches, still,
        // for JUMP_TELL, then springs.
        if e.crouch > 0.0 {
            e.crouch -= dt;
            e.vx = 0.0;
            if e.crouch <= 0.0 { (e.vx, e.vy) = e.leap; e.on_ground = false; }
        }
        match e.kind {
            Kind::Walker => {
                if e.on_ground && e.crouch <= 0.0 {
                    e.vx = if e.windup > 0.0 { 0.0 } else { e.dir * 58.0 * fast };
                    e.edge_cd -= dt;
                    let ahead = if e.dir > 0.0 { e.x + E_W + 2.0 } else { e.x - 2.0 };
                    if e.edge_cd <= 0.0 && !g.floor_at(ahead, ahead + 1.0, e.y + E_H) {
                        e.edge_cd = 0.6;
                        if blip::rand_range_f32(0.0, 1.0) < 0.55 { e.dir = -e.dir; e.vx = -e.vx; }
                    }
                    e.jump_cd -= dt;
                    if e.jump_cd <= 0.0 {
                        e.jump_cd = blip::rand_range_f32(1.0, 2.4) / fast;
                        if let Some((px, py)) = target {
                            if py < e.y - 30.0 && (px - e.x).abs() < 150.0 && blip::rand_range_f32(0.0, 1.0) < 0.7 {
                                e.crouch = JUMP_TELL;
                                e.leap = (e.dir * 58.0 * fast, -JUMP_V);
                            }
                            else if blip::rand_range_f32(0.0, 1.0) < 0.15 { e.dir = if px > e.x { 1.0 } else { -1.0 }; }
                        }
                    }
                }
                let (mut x, mut y, mut vx, mut vy) = (e.x, e.y, e.vx, e.vy);
                let (ground, wall) = g.step_body(&mut x, &mut y, &mut vx, &mut vy, E_W, E_H, dt);
                if wall { e.dir = -e.dir; }
                e.x = x; e.y = y; e.vx = vx; e.vy = vy; e.on_ground = ground;
            }
            Kind::Hopper => {
                if e.on_ground && e.crouch <= 0.0 {
                    e.vx *= 0.8;
                    e.jump_cd -= dt;
                    if e.jump_cd <= 0.0 {
                        e.jump_cd = blip::rand_range_f32(0.6, 1.3) / fast;
                        let toward = target.map(|(px, _)| if px > e.x { 1.0 } else { -1.0 }).unwrap_or(e.dir);
                        e.dir = if blip::rand_range_f32(0.0, 1.0) < 0.75 { toward } else { -toward };
                        let high = target.map(|(_, py)| py < e.y - 20.0).unwrap_or(false) || blip::rand_range_f32(0.0, 1.0) < 0.3;
                        e.crouch = JUMP_TELL;
                        e.leap = (e.dir * 85.0 * fast, if high { -JUMP_V * 0.98 } else { -300.0 });
                    }
                }
                let (mut x, mut y, mut vx, mut vy) = (e.x, e.y, e.vx, e.vy);
                let (ground, wall) = g.step_body(&mut x, &mut y, &mut vx, &mut vy, E_W, E_H, dt);
                if wall { e.dir = -e.dir; vx = -vx; }
                e.x = x; e.y = y; e.vx = vx; e.vy = vy; e.on_ground = ground;
            }
            Kind::Ghost => {
                let sp = 62.0 * fast;
                e.vx = e.vx.signum() * sp;
                e.vy = e.vy.signum() * sp * 0.8;
                e.x += e.vx * dt;
                e.y += e.vy * dt + (e.t * 3.0).sin() * 12.0 * dt;
                if e.x < TILE { e.x = TILE; e.vx = e.vx.abs(); }
                if e.x > WIN_W as f32 - TILE - E_W { e.x = WIN_W as f32 - TILE - E_W; e.vx = -e.vx.abs(); }
                if e.y < HUD + TILE { e.y = HUD + TILE; e.vy = e.vy.abs(); }
                if e.y > WIN_H as f32 - TILE - E_H { e.y = WIN_H as f32 - TILE - E_H; e.vy = -e.vy.abs(); }
            }
        }
        // Throwing, from ROCKS_FROM / SPARKS_FROM on: a walker rolls a rock
        // along its platform at a player level with it and in front; an
        // angry ghost spits a slow spark straight at the nearest player.
        // A walker stops and shakes for ROCK_WINDUP before the rock leaves,
        // so a player can see it coming.
        e.shot_cd -= dt;
        if e.windup > 0.0 {
            e.windup -= dt;
            if e.windup <= 0.0 {
                let (ex, ey) = (e.x + E_W / 2.0, e.y + E_H / 2.0);
                g.shots.push(Shot { x: ex + e.dir * 12.0, y: ey + 2.0, vx: e.dir * ROCK_V, vy: 0.0,
                    t: 0.0, spark: false, active: true });
            }
        } else if e.shot_cd <= 0.0 && e.pop_in >= 1.0 {
            if let Some((px, py)) = target {
                let (ex, ey) = (e.x + E_W / 2.0, e.y + E_H / 2.0);
                if e.kind == Kind::Walker && g.round >= ROCKS_FROM && e.on_ground
                    && (py - ey).abs() < 14.0 && (px - ex).abs() < 300.0 {
                    e.dir = if px > ex { 1.0 } else { -1.0 };
                    e.windup = ROCK_WINDUP;
                    e.shot_cd = blip::rand_range_f32(2.2, 4.0) / fast;
                } else if e.kind == Kind::Ghost && g.round >= SPARKS_FROM && e.angry {
                    let (dx, dy) = (px - ex, py - ey);
                    let d = dx.hypot(dy).max(1.0);
                    g.shots.push(Shot { x: ex, y: ey, vx: dx / d * SPARK_V, vy: dy / d * SPARK_V,
                        t: 0.0, spark: true, active: true });
                    e.shot_cd = blip::rand_range_f32(3.0, 5.0) / fast;
                } else {
                    e.shot_cd = 0.4;
                }
            }
        }
        g.enemies[i] = e;
    }
}

/// Rocks roll until they hit a block or a player's bubble; sparks drift
/// through walls and fade after a few seconds.
fn update_shots(g: &mut Game, dt: f32) {
    for i in 0..g.shots.len() {
        let mut s = g.shots[i];
        s.t += dt;
        s.x += s.vx * dt;
        s.y += s.vy * dt;
        if s.spark {
            if s.t > 4.0 { s.active = false; }
        } else {
            let (c, r) = (((s.x + s.vx.signum() * 5.0) / TILE).floor() as i32, ((s.y - HUD) / TILE).floor() as i32);
            if g.platform(c, r) || c <= 0 || c >= COLS as i32 - 1 { s.active = false; }
        }
        if s.x < -10.0 || s.x > WIN_W as f32 + 10.0 || s.y < HUD || s.y > WIN_H as f32 { s.active = false; }
        // a bubble you blow knocks it out of the air
        if s.active && g.bubbles.iter().any(|b| b.active && b.phase == Phase::Shoot && b.trapped.is_none()
            && (b.x - s.x).hypot(b.y - s.y) < BUB_R + 5.0) {
            s.active = false;
        }
        if !s.active { let (x, y) = (s.x, s.y); g.burst(x, y, 5, rgba(1.0, 0.85, 0.6, 0.8), 70.0, s.spark); }
        g.shots[i] = s;
    }
    g.shots.retain(|s| s.active);
}

/// Pop `start` and everything touching it, and everything touching those.
/// Trapped monsters in the chain are finished, each worth double the last.
fn pop_chain(g: &mut Game, start: usize, by: usize, sfx: &Sounds) {
    let mut chain = vec![start];
    let mut k = 0;
    while k < chain.len() {
        let a = g.bubbles[chain[k]];
        for j in 0..g.bubbles.len() {
            if chain.contains(&j) || !g.bubbles[j].active { continue; }
            let b = g.bubbles[j];
            if (a.x - b.x).hypot(a.y - b.y) < BUB_R * 2.0 * CHAIN_REACH { chain.push(j); }
        }
        k += 1;
    }
    let mut kills = 0;
    // One popup for the whole chain: catches sit 24px apart and a popup is
    // wider than that, so one each piled up into an unreadable smear.
    let (mut chain_total, mut chain_at) = (0, (0.0, 0.0));
    for &j in &chain {
        let b = g.bubbles[j];
        g.bubbles[j].active = false;
        let ring = if b.owner == 0 { rgba(0.6, 1.0, 0.6, 0.9) } else { rgba(0.6, 0.85, 1.0, 0.9) };
        g.burst(b.x, b.y, 8, ring, 90.0, false);
        g.ring(b.x, b.y, ring, BUB_R);
        if b.trapped.is_some() {
            let value = 1000 << kills.min(4);
            g.p[by].score += value;
            chain_total += value;
            if kills == 0 { chain_at = (b.x, b.y - 6.0); }
            g.burst(b.x, b.y, 10, rgba(1.0, 0.85, 0.3, 1.0), 150.0, true);
            g.fruits.push(Fruit { x: b.x - 8.0, y: b.y - 8.0, vx: blip::rand_range_f32(-110.0, 110.0), vy: -300.0,
                kind: kills.min(3), t: 0.0, on_ground: false, active: true });
            kills += 1;
        } else {
            g.p[by].score += 10;
        }
    }
    if chain_total > 0 {
        g.pops.push(Popup { x: chain_at.0, y: chain_at.1, t: 0.0, value: chain_total, c: rgba(1.0, 0.95, 0.4, 1.0) });
    }
    play_sfx(&sfx.pop);
    g.stats.pops += 1;
    g.stats.kills += kills as u32;
    if kills > 0 { g.stats.chains[(kills as usize).min(5)] += 1; }
    if kills > 0 { play_sfx(&sfx.kill); }
    if kills >= 2 {
        g.chain = (kills as i32, 1.2);
        g.shake = 0.25 + 0.08 * kills as f32;
        play_sfx(&sfx.sparkle);
    }
}

fn update_bubbles(g: &mut Game, dt: f32, inp: [Input; 2], sfx: &Sounds) {
    let gather = WIN_W as f32 / 2.0;
    for i in 0..g.bubbles.len() {
        let mut b = g.bubbles[i];
        if !b.active { continue; }
        b.age += dt;
        b.wob += dt;
        b.squish = (b.squish - dt * 4.0).max(0.0);
        let was_free = b.trapped.is_none();
        match b.phase {
            Phase::Shoot => {
                b.x += b.vx * dt;
                b.vx *= 1.0 - 2.5 * dt;
                if b.x < TILE + BUB_R { b.x = TILE + BUB_R; b.phase = Phase::Float; }
                if b.x > WIN_W as f32 - TILE - BUB_R { b.x = WIN_W as f32 - TILE - BUB_R; b.phase = Phase::Float; }
                if b.age > SHOOT_T { b.phase = Phase::Float; }
                // trap a monster it meets while still flying
                if b.trapped.is_none() {
                    for e in g.enemies.iter_mut() {
                        if !e.active || e.pop_in < 1.0 { continue; }
                        let (ex, ey) = (e.x + E_W / 2.0, e.y + E_H / 2.0);
                        if (ex - b.x).abs() < BUB_R + E_W / 2.0 - 2.0 && (ey - b.y).abs() < BUB_R + E_H / 2.0 - 2.0 {
                            e.active = false;
                            b.trapped = Some((e.kind, e.angry));
                            b.age = 0.0;
                            b.phase = Phase::Float;
                            b.x = ex; b.y = ey;
                            play_sfx(&sfx.trap);
                            break;
                        }
                    }
                }
            }
            Phase::Float => {
                b.y -= if b.trapped.is_some() { RISE_TRAPPED } else { RISE } * dt;
                b.x += (b.wob * 2.2).sin() * 10.0 * dt;
                if b.y <= TOP_Y { b.y = TOP_Y; b.phase = Phase::Top; }
            }
            Phase::Top => {
                let d = gather - b.x;
                b.x += d.signum() * d.abs().min(38.0) * dt + (b.wob * 1.7).sin() * 6.0 * dt;
                b.y = TOP_Y + (b.wob * 2.0).sin() * 1.5;
            }
        }
        if was_free && b.trapped.is_some() { g.stats.traps += 1; }
        let life = if b.trapped.is_some() { trap_life(g.round) } else { FREE_LIFE };
        if b.age > life && b.phase != Phase::Shoot {
            if b.trapped.is_some() { g.stats.escapes += 1; }
            b.active = false;
            if let Some((kind, _)) = b.trapped {
                // out it comes, and it is not happy about it
                g.enemies.push(Enemy { kind, x: b.x - E_W / 2.0, y: b.y - E_H / 2.0, vx: 0.0, vy: 0.0,
                    dir: if blip::rand_range_f32(0.0, 1.0) < 0.5 { -1.0 } else { 1.0 }, on_ground: false, angry: true, t: 0.0,
                    jump_cd: 0.5, edge_cd: 0.0, active: true, pop_in: 1.0, shot_cd: blip::rand_range_f32(1.5, 3.0), windup: 0.0,
                        crouch: 0.0, leap: (0.0, 0.0) });
                let last = g.enemies.len() - 1;
                if kind == Kind::Ghost { g.enemies[last].vx = 62.0; g.enemies[last].vy = 62.0; }
            }
            g.burst(b.x, b.y, 6, rgba(1.0, 1.0, 1.0, 0.6), 60.0, false);
        }
        g.bubbles[i] = b;
    }
    // bubbles along the ceiling jostle into a cluster instead of stacking up
    for i in 0..g.bubbles.len() {
        for j in (i + 1)..g.bubbles.len() {
            let (a, b) = (g.bubbles[i], g.bubbles[j]);
            if !a.active || !b.active || a.phase == Phase::Shoot || b.phase == Phase::Shoot { continue; }
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let d = dx.hypot(dy).max(0.1);
            let min = BUB_R * 2.0 - 1.0;
            if d < min {
                let push = (min - d) * 0.5;
                g.bubbles[i].x -= dx / d * push;
                g.bubbles[j].x += dx / d * push;
                if a.phase == Phase::Top && b.phase == Phase::Top {
                    g.bubbles[j].y += 0.0;
                } else {
                    g.bubbles[i].y -= dy / d * push * 0.5;
                    g.bubbles[j].y += dy / d * push * 0.5;
                }
            }
        }
    }
    // players against bubbles: hold jump to ride one, otherwise it pops
    for pi in 0..2 {
        let p = &g.p[pi];
        if !p.joined || !p.alive || p.dead_t > 0.0 { continue; }
        let (px, py, vy) = (p.cx(), p.cy(), p.vy);
        for i in 0..g.bubbles.len() {
            let b = g.bubbles[i];
            if !b.active { continue; }
            let d = (b.x - px).hypot(b.y - py);
            if d > BUB_R + 11.0 { continue; }
            if vy > 40.0 && py < b.y - 6.0 && inp[pi].jump_held {
                g.p[pi].vy = -JUMP_V * 0.82;
                g.p[pi].squash = 1.3;
                g.bubbles[i].squish = 1.0;
                g.stats.bounces += 1;
                play_sfx_volume(&sfx.bounce, 0.7);
                break;
            }
            if b.owner == pi && b.age < 0.25 && b.trapped.is_none() { continue; } // your own, just blown
            pop_chain(g, i, pi, sfx);
            break;
        }
    }
    g.bubbles.retain(|b| b.active);
}

fn update_fruit(g: &mut Game, dt: f32, sfx: &Sounds) {
    const VALUE: [i32; 4] = [500, 1000, 2000, 4000];
    for i in 0..g.fruits.len() {
        let mut f = g.fruits[i];
        if !f.active { continue; }
        f.t += dt;
        if !f.on_ground {
            let (mut x, mut y, mut vx, mut vy) = (f.x, f.y, f.vx, f.vy);
            let (ground, _) = g.step_body(&mut x, &mut y, &mut vx, &mut vy, 16.0, 16.0, dt);
            f.x = x; f.y = y; f.vx = vx * (1.0 - 0.8 * dt); f.vy = vy;
            if ground { f.on_ground = true; f.vx = 0.0; }
        } else if !g.floor_at(f.x, f.x + 16.0, f.y + 16.0) {
            f.on_ground = false;
        }
        if f.t > 9.0 { f.active = false; }
        for pi in 0..2 {
            let p = &g.p[pi];
            if !p.joined || !p.alive || p.dead_t > 0.0 || f.t < 0.35 { continue; }
            if (p.cx() - (f.x + 8.0)).abs() < 16.0 && (p.cy() - (f.y + 8.0)).abs() < 18.0 {
                f.active = false;
                let v = VALUE[f.kind];
                g.p[pi].score += v;
                g.p[pi].eat = 0.0;
                g.p[pi].eaten = f;
                g.stats.fruit += 1;
                g.pops.push(Popup { x: f.x + 8.0, y: f.y, t: 0.0, value: v, c: rgba(1.0, 1.0, 1.0, 1.0) });
                g.burst(f.x + 8.0, f.y + 8.0, 5, rgba(1.0, 1.0, 0.6, 1.0), 60.0, true);
                play_sfx(&sfx.fruit);
                break;
            }
        }
        g.fruits[i] = f;
    }
    g.fruits.retain(|f| f.active);
}

/// Crumbs fly from a chewing dragon's mouth on each chomp.
fn update_chewing(g: &mut Game, dt: f32) {
    for pi in 0..2 {
        let p = &g.p[pi];
        if p.eat < 0.0 { continue; }
        if !CHOMPS.iter().any(|&c| p.eat - dt < c && p.eat >= c) { continue; }
        let (mx, my) = mouth_pos(p);
        let c = col(CANDY[candy_pick(&p.eaten)], 1.0);
        g.burst(mx, my, 4, c, 45.0, false);
    }
}

fn update_skull(g: &mut Game, dt: f32) {
    if !g.skull.active {
        if g.round_t >= skull_at(g.round) {
            g.skull = Skull { active: true, x: WIN_W as f32 / 2.0, y: HUD + 10.0, speed: 45.0 };
            g.stats.skulls += 1;
        }
        return;
    }
    g.skull.speed += (4.0 + 1.5 * g.round as f32) * dt;
    if let Some((px, py)) = nearest_player(g, g.skull.x, g.skull.y) {
        let (dx, dy) = (px - g.skull.x, py - g.skull.y);
        let d = dx.hypot(dy).max(1.0);
        g.skull.x += dx / d * g.skull.speed * dt;
        g.skull.y += dy / d * g.skull.speed * dt;
    }
}

fn hurt_players(g: &mut Game, sfx: &Sounds) {
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("BUBBLER_GOD").is_some() {
        if g.skull.active && g.p.iter().any(|p| p.alive && (g.skull.x - p.cx()).hypot(g.skull.y - p.cy()) < 18.0) {
            g.stats.deaths_skull += 1; // would have been caught
        }
        return;
    }
    for pi in 0..2 {
        let p = &g.p[pi];
        if !p.joined || !p.alive || p.dead_t > 0.0 || p.safe > 0.0 { continue; }
        let (px, py) = (p.cx(), p.cy());
        let killer = g.enemies.iter().find(|e| e.active && e.pop_in >= 1.0
            && (e.x + E_W / 2.0 - px).abs() < (E_W + P_W) / 2.0 - 7.0
            && (e.y + E_H / 2.0 - py).abs() < (E_H + P_H) / 2.0 - 10.0).map(|e| e.kind);
        let hit_enemy = killer.is_some();
        let hit_skull = g.skull.active && (g.skull.x - px).hypot(g.skull.y - py) < 18.0;
        let hit_shot = g.shots.iter().any(|s| (s.x - px).abs() < P_W / 2.0 + 3.0 && (s.y - py).abs() < P_H / 2.0 + 2.0);
        if hit_shot { g.shots.retain(|s| !((s.x - px).abs() < P_W / 2.0 + 3.0 && (s.y - py).abs() < P_H / 2.0 + 2.0)); }
        let hit_enemy = hit_enemy || hit_shot;
        if hit_enemy || hit_skull {
            if hit_skull { g.stats.deaths_skull += 1; } else { g.stats.deaths_enemy += 1; }
            if let Some(k) = killer { g.stats.deaths_by[k as usize] += 1; }
            #[cfg(not(target_arch = "wasm32"))]
            if std::env::var_os("BUBBLER_BOT").is_some() {
                let p = &g.p[pi];
                let e = g.enemies.iter().filter(|e| e.active)
                    .min_by(|a, b| (a.x - p.x).hypot(a.y - p.y).total_cmp(&(b.x - p.x).hypot(b.y - p.y)));
                println!("DEATH round={} t={:.1} p{} at ({:.0},{:.0}) v=({:.0},{:.0}) ground={} face={} | {:?}",
                    g.round + 1, g.round_t, pi + 1, p.x, p.y, p.vx, p.vy, p.on_ground, p.face,
                    e.map(|e| (e.kind, e.x as i32, e.y as i32, e.vx as i32, e.vy as i32, e.angry)));
            }
            g.stats.deaths_round[g.round] += 1;
            let p = &mut g.p[pi];
            p.dead_t = 1.4;
            p.vy = -260.0;
            play_sfx(&sfx.lose);
            let (x, y) = (p.cx(), p.cy());
            g.burst(x, y, 14, rgba(1.0, 0.5, 0.6, 1.0), 120.0, true);
        }
    }
}

fn update_fx(g: &mut Game, dt: f32) {
    g.parts.update(dt);
    for p in g.pops.iter_mut() { p.t += dt; p.y = (p.y - 30.0 * dt).max(HUD + 6.0); }
    g.pops.retain(|p| p.t < 1.4);
    g.shake = (g.shake - dt).max(0.0);
    g.chain.1 = (g.chain.1 - dt).max(0.0);
}

fn players_left(g: &Game) -> bool {
    g.p.iter().any(|p| p.joined && (p.lives > 0 || p.dead_t > 0.0))
}

fn update_play(g: &mut Game, dt: f32, inp: [Input; 2], sfx: &Sounds) {
    g.round_t += dt;
    if !g.hurry && g.round_t >= hurry_at(g.round) {
        g.hurry = true;
        g.stats.hurries += 1;
        for e in g.enemies.iter_mut() { e.angry = true; }
        play_sfx(&sfx.hurry);
    }
    update_players(g, inp, dt, sfx);
    update_enemies(g, dt);
    update_bubbles(g, dt, inp, sfx);
    update_fruit(g, dt, sfx);
    update_skull(g, dt);
    update_chewing(g, dt);
    update_shots(g, dt);
    hurt_players(g, sfx);
    g.enemies.retain(|e| e.active);
    if !players_left(g) {
        g.state = State::Over;
        g.state_t = 0.0;
        report_score(g);
        play_sfx(&sfx.over);
        return;
    }
    let trapped = g.bubbles.iter().any(|b| b.trapped.is_some());
    if g.enemies.is_empty() && !trapped {
        g.state = State::Clear;
        g.state_t = 0.0;
        g.skull.active = false;
        g.shots.clear();
        // EXTEND: every player still in gets a life back, up to EXTEND_MAX
        g.extend = false;
        for p in g.p.iter_mut() {
            if p.joined && p.lives > 0 && p.lives < EXTEND_MAX { p.lives += 1; g.extend = true; }
        }
        play_sfx(&sfx.clear);
    }
}

// ---- drawing -------------------------------------------------------------

fn tint(c: (u8, u8, u8), k: f32) -> (u8, u8, u8) {
    (((c.0 as f32) * k).min(255.0) as u8, ((c.1 as f32) * k).min(255.0) as u8, ((c.2 as f32) * k).min(255.0) as u8)
}

fn draw_background(blip: &Blip, g: &Game) {
    let pal = &PALETTES[g.round];
    let bands = 18;
    for i in 0..bands {
        let t = i as f32 / (bands - 1) as f32;
        let c = (
            (pal.sky0.0 as f32 + (pal.sky1.0 as f32 - pal.sky0.0 as f32) * t) as u8,
            (pal.sky0.1 as f32 + (pal.sky1.1 as f32 - pal.sky0.1 as f32) * t) as u8,
            (pal.sky0.2 as f32 + (pal.sky1.2 as f32 - pal.sky0.2 as f32) * t) as u8,
        );
        let h = WIN_H as f32 / bands as f32;
        blip.fill_rect(0.0, i as f32 * h, WIN_W as f32, h + 1.0, col(c, 1.0));
    }
    // soft bokeh drifting up behind the play field
    let t = now();
    for i in 0..22 {
        let s = i as f32 * 37.7;
        let x = (s * 13.1) % WIN_W as f32;
        let y = WIN_H as f32 - ((s * 7.3 + t * (6.0 + (i % 5) as f32 * 3.0)) % (WIN_H as f32 + 40.0)) + 20.0;
        let r = 3.0 + (i % 4) as f32 * 3.0;
        let a = 0.05 + 0.04 * ((t * 0.8 + s).sin() * 0.5 + 0.5);
        blip.fill_circle(x, y, r, col(pal.glow, a));
    }
    // twinkles
    for i in 0..30 {
        let s = i as f32 * 91.3;
        let x = (s * 5.7) % WIN_W as f32;
        let y = HUD + (s * 3.1) % (WIN_H as f32 - HUD);
        let a = ((t * 2.0 + s).sin() * 0.5 + 0.5).powi(3) * 0.8;
        blip.fill_rect(x, y, 1.5, 1.5, rgba(1.0, 1.0, 1.0, a));
    }
}

fn draw_tiles(blip: &Blip, g: &Game, ox: f32, oy: f32) {
    let pal = &PALETTES[g.round];
    for r in 0..ROWS {
        for c in 0..COLS {
            if !g.tiles[r][c] && !(c == 0 || c == COLS - 1) { continue; }
            let border = c == 0 || c == COLS - 1 || r == 0 || r == ROWS - 1;
            if (r == 0 || r == ROWS - 1) && !g.tiles[r][c] { continue; }
            let (x, y) = (c as f32 * TILE + ox, HUD + r as f32 * TILE + oy);
            let base = if border { tint(pal.block, 0.62) } else { pal.block };
            blip.fill_rect(x + 1.0, y + 1.0, TILE - 2.0, TILE - 2.0, col(base, 1.0));
            blip.fill_rect(x + 1.0, y + 1.0, TILE - 2.0, 3.0, col(if border { tint(pal.light, 0.7) } else { pal.light }, 1.0));
            blip.fill_rect(x + 1.0, y + TILE - 4.0, TILE - 2.0, 3.0, col(pal.dark, 1.0));
            blip.fill_rect(x + TILE - 4.0, y + 1.0, 3.0, TILE - 2.0, col(pal.dark, 0.7));
            // a little pressed pattern so the blocks read as tiles, not paint
            let dot = col(pal.light, if border { 0.25 } else { 0.45 });
            blip.fill_circle(x + TILE / 2.0 - 1.0, y + TILE / 2.0, 3.0, dot);
            blip.fill_circle(x + TILE / 2.0 - 2.0, y + TILE / 2.0 - 1.0, 1.0, rgba(1.0, 1.0, 1.0, 0.45));
        }
    }
}

/// A cute little dragon: round body, cream belly, big shiny eyes, rosy
/// cheek, spikes down its back, stubby feet; squash and stretch with
/// every jump and landing, blinking, cheeks puffing to blow.
// ---- plush toys ------------------------------------------------------------
// Everyone in Bubbler is a soft toy: a felt body with a fuzzy edge and a
// stitched seam, shiny button eyes, rosy cheeks and a stitched smile. Angry
// monsters only blush redder and frown; nobody looks scary.

/// A felt body: a soft shadow, a fuzzy rim of tufts, the fabric, a soft
/// highlight up and to the left.
fn plush_body(cx: f32, cy: f32, rx: f32, ry: f32, c: (u8, u8, u8), alpha: f32) {
    use blip::macroquad::shapes::draw_circle;
    draw_ellipse(cx + 1.5, cy + 2.0, rx, ry, 0.0, rgba(0.1, 0.0, 0.15, 0.25 * alpha));
    let fuzz = col(tint(c, 0.82), alpha);
    let n = 18;
    for k in 0..n {
        let a = k as f32 / n as f32 * std::f32::consts::TAU;
        draw_circle(cx + a.cos() * rx * 0.94, cy + a.sin() * ry * 0.94, rx.min(ry) * 0.2, fuzz);
    }
    draw_ellipse(cx, cy, rx, ry, 0.0, col(c, alpha));
    draw_ellipse(cx - rx * 0.28, cy - ry * 0.32, rx * 0.5, ry * 0.36, -20.0, col(tint(c, 1.18), alpha * 0.55));
}

/// A stitched seam: short dashes along an arc of the ellipse (`a0`..`a1`, radians).
fn stitches(cx: f32, cy: f32, rx: f32, ry: f32, a0: f32, a1: f32, n: usize, c: BlipColor) {
    for k in 0..n {
        let t0 = a0 + (a1 - a0) * (k as f32 + 0.15) / n as f32;
        let t1 = a0 + (a1 - a0) * (k as f32 + 0.6) / n as f32;
        blip::macroquad::shapes::draw_line(cx + t0.cos() * rx, cy + t0.sin() * ry, cx + t1.cos() * rx, cy + t1.sin() * ry, 1.0, c);
    }
}

/// A shiny button eye; `blink` closes it to a stitched line.
fn button_eye(x: f32, y: f32, r: f32, blink: bool, alpha: f32) {
    use blip::macroquad::shapes::{draw_circle, draw_line};
    if blink {
        draw_line(x - r, y, x + r, y, 1.4, col((40, 25, 45), alpha));
        return;
    }
    draw_circle(x, y, r, col((35, 22, 40), alpha));
    draw_circle(x, y, r * 0.72, col((60, 40, 70), alpha));
    draw_circle(x - r * 0.35, y - r * 0.38, r * 0.32, col((255, 255, 255), alpha));
}

/// A little stitched smile (or, `frown`, a worried wiggle) centred on (`x`, `y`).
fn stitched_mouth(x: f32, y: f32, w: f32, frown: bool, alpha: f32) {
    let c = col((90, 40, 60), alpha);
    let dir = if frown { -1.0 } else { 1.0 };
    let pts: Vec<(f32, f32)> = (0..=4).map(|k| {
        let u = k as f32 / 4.0 * 2.0 - 1.0;
        (x + u * w, y + dir * (1.0 - u * u) * w * 0.55)
    }).collect();
    for p in pts.windows(2) {
        blip::macroquad::shapes::draw_line(p[0].0, p[0].1, p[1].0, p[1].1, 1.3, c);
    }
}

fn cheeks(x: f32, y: f32, apart: f32, r: f32, angry: bool, alpha: f32) {
    let c = if angry { rgba(1.0, 0.25, 0.3, 0.7 * alpha) } else { rgba(1.0, 0.5, 0.6, 0.55 * alpha) };
    for s in [-1.0f32, 1.0] { blip::macroquad::shapes::draw_circle(x + s * apart, y, r, c); }
}

/// Eating a candy: it flies into the mouth for EAT_FLY, then two chomps.
const EAT_SECS: f32 = 0.6;
const EAT_FLY: f32 = 0.15;
const CHOMPS: [f32; 2] = [0.28, 0.45];

/// Where a player's mouth is on screen (see draw_dragon).
fn mouth_pos(p: &Player) -> (f32, f32) {
    (p.cx() + p.face * 10.5 * TOY_SCALE, p.y + P_H - 9.0 * TOY_SCALE)
}

/// The player: a plush dragon standing on `foot`, facing `face`, drawn `k`
/// times life size. `eat` is seconds into eating a candy, negative if not.
fn draw_dragon(x: f32, foot: f32, face: f32, body: (u8, u8, u8), squash: f32, walk: f32, blink: bool,
               mouth: bool, alpha: f32, spin: f32, k: f32, eat: f32) {
    use blip::macroquad::shapes::draw_circle;
    let breathe = 1.0 + (now() * 3.0 + x * 0.05).sin() * 0.03;
    let squash = squash * breathe;
    let (sy, sx) = (squash * k, k / squash.sqrt());
    let cy = foot - 11.0 * sy;
    let dark = tint(body, 0.7);
    let belly = (255, 240, 205);
    // stubby felt feet, stepping
    let step = (walk * std::f32::consts::TAU).sin() * 1.8;
    for (i, s) in [-1.0f32, 1.0].iter().enumerate() {
        let lift = if i == 0 { step.max(0.0) } else { (-step).max(0.0) };
        draw_ellipse(x + s * 5.5 * sx, foot - (2.2 + lift) * k, 4.6 * k, 3.0 * k, 0.0, col(dark, alpha));
    }
    // soft rounded spikes down the back, in the belly's cream
    for sp in 0..3 {
        let a = std::f32::consts::PI * (0.55 + 0.17 * sp as f32) + spin;
        let (bx, by) = (x - face * a.sin() * 11.0 * sx, cy - a.cos() * 10.0 * sy);
        draw_circle(bx, by, 3.0 * k, col(tint(belly, 0.9), alpha));
        draw_circle(bx, by, 2.2 * k, col(belly, alpha));
    }
    plush_body(x, cy, 11.5 * sx, 10.5 * sy, body, alpha);
    // the round cream belly, with its seam
    draw_ellipse(x + face * 2.5 * sx, cy + 3.5 * sy, 6.8 * sx, 5.8 * sy, 0.0, col(belly, alpha));
    stitches(x + face * 2.5 * sx, cy + 3.5 * sy, 6.8 * sx, 5.8 * sy, 0.0, std::f32::consts::TAU, 12, col(tint(belly, 0.7), alpha));
    // a little felt tuft on top
    draw_circle(x - face * 1.0 * k, cy - 10.5 * sy, 2.8 * k, col(tint(body, 1.1), alpha));
    let chewing = eat >= EAT_FLY;
    // eyes: buttons, or squeezed shut in two happy arcs while it chews
    for (i, off) in [2.0f32, 7.5].iter().enumerate() {
        let (ex, ey) = (x + face * off * sx, cy - 3.8 * sy);
        if chewing {
            let (c, r) = (col((40, 25, 45), alpha), 2.4 * k);
            blip::macroquad::shapes::draw_line(ex - r, ey + r * 0.4, ex, ey - r * 0.5, 1.5, c);
            blip::macroquad::shapes::draw_line(ex, ey - r * 0.5, ex + r, ey + r * 0.4, 1.5, c);
        } else {
            let r = if i == 0 { 2.7 } else { 2.5 } * k;
            button_eye(ex, ey, r, blink, alpha);
        }
    }
    cheeks(x + face * 4.8 * sx, cy + 1.2 * sy, 4.2 * sx, 1.8 * k, false, alpha);
    if eat >= 0.0 && !chewing {
        // wide open for the candy coming in
        draw_ellipse(x + face * 10.5 * sx, cy + 2.0 * sy, 3.6 * k, 3.4 * k, 0.0, col((120, 40, 60), alpha));
        draw_ellipse(x + face * 10.5 * sx, cy + 3.0 * sy, 2.2 * k, 1.4 * k, 0.0, col((240, 120, 140), alpha));
    } else if chewing {
        // a full cheek bulging, the mouth working open and shut
        let chew = ((eat - EAT_FLY) * 22.0).sin();
        draw_circle(x + face * 9.0 * sx, cy + 2.2 * sy, 3.6 * k * (1.0 + 0.12 * chew.abs()), col(body, alpha));
        draw_circle(x + face * 9.0 * sx, cy + 2.6 * sy, 1.9 * k, rgba(1.0, 0.5, 0.6, 0.55 * alpha));
        if chew > 0.0 {
            draw_ellipse(x + face * 11.0 * sx, cy + 3.4 * sy, 1.8 * k, 1.2 * k, 0.0, col((120, 40, 60), alpha));
        } else {
            stitched_mouth(x + face * 10.5 * sx, cy + 3.2 * sy, 1.6 * k, false, alpha);
        }
    } else if mouth {
        // blowing: a round "o"
        draw_ellipse(x + face * 10.5 * sx, cy + 2.0 * sy, 2.4 * k, 2.2 * k, 0.0, col((120, 40, 60), alpha));
    } else {
        stitched_mouth(x + face * 5.2 * sx, cy + 1.6 * sy, 2.0 * k, false, alpha);
    }
}

/// A monster as a plush toy, its feet on `y + E_H`, drawn `scale` times life size.
fn draw_enemy(kind: Kind, x: f32, y: f32, angry: bool, t: f32, scale: f32, alpha: f32) {
    use blip::macroquad::shapes::{draw_circle, draw_line};
    let s = scale;
    let cx = x + E_W / 2.0;
    // grow upward from the feet, so a bigger toy still stands on its platform
    let cy = y + E_H - E_H / 2.0 * s;
    match kind {
        Kind::Walker => {
            // a plush wind-up robot, its felt key turning
            let body = if angry { (255, 150, 140) } else { (200, 190, 245) };
            let bob = (t * 10.0).sin().abs() * 1.2 * s;
            let cy = cy - bob;
            let key = (t * 6.0).sin();
            draw_line(cx, cy - 9.0 * s, cx, cy - 12.5 * s, 2.0 * s, col((230, 180, 70), alpha));
            draw_ellipse(cx - 2.6 * s * key.abs().max(0.3), cy - 13.0 * s, 2.6 * s * key.abs().max(0.3), 1.8 * s, 0.0, col((255, 205, 90), alpha));
            draw_ellipse(cx + 2.6 * s * key.abs().max(0.3), cy - 13.0 * s, 2.6 * s * key.abs().max(0.3), 1.8 * s, 0.0, col((255, 205, 90), alpha));
            for sgn in [-1.0f32, 1.0] {
                draw_ellipse(cx + sgn * 5.0 * s, cy + 9.0 * s + bob, 3.4 * s, 2.2 * s, 0.0, col(tint(body, 0.7), alpha));
            }
            plush_body(cx, cy, 10.5 * s, 9.5 * s, body, alpha);
            // a felt face panel with its seam
            draw_ellipse(cx, cy + 0.5 * s, 7.2 * s, 5.0 * s, 0.0, col((250, 245, 255), alpha));
            stitches(cx, cy + 0.5 * s, 7.2 * s, 5.0 * s, 0.0, std::f32::consts::TAU, 12, col((150, 140, 190), alpha));
            for sgn in [-1.0f32, 1.0] { button_eye(cx + sgn * 3.0 * s, cy - 0.5 * s, 1.8 * s, false, alpha); }
            if angry {
                for sgn in [-1.0f32, 1.0] {
                    draw_line(cx + sgn * 5.0 * s, cy - 4.2 * s, cx + sgn * 1.4 * s, cy - 3.0 * s, 1.2, col((90, 40, 60), alpha));
                }
            }
            stitched_mouth(cx, cy + 3.0 * s, 1.8 * s, angry, alpha);
            cheeks(cx, cy + 2.4 * s, 5.4 * s, 1.4 * s, angry, alpha);
        }
        Kind::Hopper => {
            // a plush bunny that hops, ears flopping with each bounce
            let body = if angry { (255, 140, 160) } else { (255, 195, 220) };
            let sq = 1.0 + (t * 8.0).sin() * 0.08;
            let flop = (t * 8.0).sin() * 0.25;
            for sgn in [-1.0f32, 1.0] {
                let (ex, ey) = (cx + sgn * 4.0 * s, cy - 11.0 * s);
                draw_ellipse(ex + sgn * flop * 4.0 * s, ey, 2.8 * s, 6.5 * s, sgn * (12.0 + flop * 40.0), col(tint(body, 0.85), alpha));
                draw_ellipse(ex + sgn * flop * 4.0 * s, ey + 0.5 * s, 1.4 * s, 4.5 * s, sgn * (12.0 + flop * 40.0), col((255, 160, 190), alpha));
            }
            plush_body(cx, cy + 1.0 * s, 10.0 * s / sq, 9.0 * s * sq, body, alpha);
            draw_circle(cx + 9.0 * s, cy + 5.0 * s, 2.6 * s, col((255, 250, 250), alpha)); // a cotton tail
            for sgn in [-1.0f32, 1.0] { button_eye(cx + sgn * 3.6 * s, cy - 0.5 * s, 1.9 * s, false, alpha); }
            draw_circle(cx, cy + 2.2 * s, 1.1 * s, col((230, 90, 130), alpha)); // a little nose
            if angry {
                for sgn in [-1.0f32, 1.0] {
                    draw_line(cx + sgn * 5.8 * s, cy - 4.0 * s, cx + sgn * 1.8 * s, cy - 2.8 * s, 1.2, col((90, 40, 60), alpha));
                }
            }
            stitched_mouth(cx, cy + 4.0 * s, 1.6 * s, angry, alpha);
            cheeks(cx, cy + 2.6 * s, 6.0 * s, 1.5 * s, angry, alpha);
        }
        Kind::Ghost => {
            // a pillow ghost, its soft hem waving
            let body = if angry { (255, 190, 235) } else { (215, 245, 255) };
            let a = alpha * 0.95;
            for k in 0..4 {
                let wx = cx - 7.5 * s + k as f32 * 5.0 * s;
                let wy = cy + 7.5 * s + (t * 6.0 + k as f32).sin() * 1.2;
                draw_circle(wx, wy, 3.0 * s, col(tint(body, 0.85), a));
            }
            plush_body(cx, cy - 1.0 * s, 10.0 * s, 9.5 * s, body, a);
            for sgn in [-1.0f32, 1.0] { button_eye(cx + sgn * 3.5 * s, cy - 2.0 * s, 2.0 * s, false, alpha); }
            if angry {
                for sgn in [-1.0f32, 1.0] {
                    draw_line(cx + sgn * 5.6 * s, cy - 5.6 * s, cx + sgn * 1.8 * s, cy - 4.4 * s, 1.2, col((90, 40, 60), alpha));
                }
            }
            stitched_mouth(cx, cy + 2.2 * s, 1.8 * s, angry, alpha);
            cheeks(cx, cy + 1.0 * s, 6.0 * s, 1.5 * s, angry, alpha);
        }
    }
}

fn hsv(h: f32, s: f32, v: f32, a: f32) -> BlipColor {
    let h = (h.rem_euclid(1.0)) * 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as i32 { 0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x), 3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x) };
    let m = v - c;
    rgba(r + m, g + m, b + m, a)
}

fn draw_bubble(b: &Bubble, ox: f32, oy: f32, t: f32, round: usize) {
    let (x, y) = (b.x + ox, b.y + oy);
    let life = if b.trapped.is_some() { trap_life(round) } else { FREE_LIFE };
    let escaping = b.trapped.is_some() && b.age > life - 1.6;
    let shake = if escaping { (t * 50.0).sin() * 1.5 } else { 0.0 };
    let grow = if b.phase == Phase::Shoot && b.trapped.is_none() { 0.35 + 0.65 * blip::ease_out_cubic(b.age / 0.12) } else { 1.0 };
    let r = (BUB_R + (b.wob * 3.0).sin() * 0.6) * grow;
    let (rx, ry) = (r * (1.0 + b.squish * 0.25), r * (1.0 - b.squish * 0.25));
    if let Some((kind, angry)) = b.trapped {
        // centred in its bubble (draw_enemy stands a toy on its feet)
        draw_enemy(kind, x - E_W / 2.0 + shake, y - E_H + E_H / 2.0 * 0.78 + (b.wob * 5.0).sin(), angry, b.wob, 0.78, 1.0);
    }
    let tint_c = if b.owner == 0 { (120, 255, 140) } else { (120, 200, 255) };
    draw_ellipse(x + shake, y, rx, ry, 0.0, col(tint_c, if b.trapped.is_some() { 0.16 } else { 0.22 }));
    let rim = if escaping && (t * 10.0) as i32 % 2 == 0 { rgba(1.0, 0.3, 0.3, 0.95) }
              else { hsv(b.wob * 0.15 + x * 0.002, 0.45, 1.0, 0.85) };
    blip::macroquad::shapes::draw_ellipse_lines(x + shake, y, rx, ry, 0.0, 1.8, rim);
    draw_ellipse(x - r * 0.38 + shake, y - r * 0.42, r * 0.3, r * 0.17, -35.0, rgba(1.0, 1.0, 1.0, 0.85));
    blip::macroquad::shapes::draw_circle(x + r * 0.45 + shake, y + r * 0.35, 1.2, rgba(1.0, 1.0, 1.0, 0.6));
}

/// Candy colours, picked per drop so a shower of them is a pick-and-mix.
const CANDY: [(u8, u8, u8); 6] = [
    (255, 120, 160), (120, 200, 255), (255, 200, 80), (150, 230, 140), (200, 150, 255), (255, 150, 90),
];

/// A dropped treat, sweeter the bigger the chain that made it (see VALUE):
/// a wrapped bonbon, a swirled lollipop, a sprinkled donut, a cupcake with
/// a cherry on top. Each shines, and twinkles now and then.
fn draw_fruit(f: &Fruit, ox: f32, oy: f32) {
    if f.t > 7.0 && (f.t * 10.0) as i32 % 2 == 0 { return; }
    let bob = if f.on_ground { (f.t * 4.0).sin() * 1.0 } else { 0.0 };
    draw_candy(f.kind, candy_pick(f), f.t, f.x + 8.0 + ox, f.y + 8.0 + oy + bob, true);
}

/// Which colour a candy is: fixed by where it dropped.
fn candy_pick(f: &Fruit) -> usize {
    ((f.x * 7.0 + f.kind as f32 * 13.0) as i32).rem_euclid(CANDY.len() as i32) as usize
}

/// A candy of `kind` in colour `pick`, centred on (`x`, `y`); `t` drives
/// its shine, `halo` the glow it has lying on the floor.
fn draw_candy(kind: usize, pick: usize, t: f32, x: f32, y: f32, halo: bool) {
    use blip::macroquad::shapes::{draw_circle, draw_line};
    let c = CANDY[pick];
    let c2 = CANDY[(pick + 2) % CANDY.len()];
    if halo {
        // a warm halo behind, so a treat is found at a glance
        let glow = 0.07 + 0.03 * (t * 5.0).sin();
        draw_circle(x, y + 1.0, 13.0, rgba(1.0, 0.8, 0.6, glow));
        draw_circle(x, y + 1.0, 10.0, rgba(1.0, 0.85, 0.65, glow));
    }
    let shine = rgba(1.0, 1.0, 1.0, 0.85);
    match kind {
        0 => { // a wrapped bonbon: a round sweet, twisted cellophane ends
            for s in [-1.0f32, 1.0] {
                draw_triangle(vec2(x + s * 6.0, y), vec2(x + s * 12.0, y - 5.0), vec2(x + s * 12.0, y + 5.0), col(tint(c, 1.15), 1.0));
                draw_line(x + s * 9.0, y - 3.0, x + s * 9.0, y + 3.0, 1.0, col(tint(c, 0.75), 1.0));
            }
            draw_circle(x, y, 7.5, col(tint(c, 0.8), 1.0));
            draw_circle(x, y, 6.6, col(c, 1.0));
            for k in 0..3 { // candy stripes
                let dx = -4.0 + k as f32 * 4.0;
                draw_line(x + dx - 1.5, y - 5.0, x + dx + 1.5, y + 5.0, 1.6, col((255, 255, 255), 0.55));
            }
            draw_ellipse(x - 2.6, y - 3.0, 2.4, 1.4, -30.0, shine);
        }
        1 => { // a lollipop: a spiral of two colours on a stick
            draw_line(x, y + 4.0, x + 1.0, y + 14.0, 2.2, col((250, 245, 235), 1.0));
            draw_circle(x, y - 1.0, 8.0, col(tint(c, 0.8), 1.0));
            draw_circle(x, y - 1.0, 7.2, col(c, 1.0));
            let turn = t * 1.5;
            for k in 0..28 {
                let u = k as f32 / 28.0;
                let a = u * 12.0 + turn;
                let r = 1.0 + u * 5.6;
                draw_circle(x + a.cos() * r, y - 1.0 + a.sin() * r, 1.2, col(c2, 1.0));
            }
            draw_ellipse(x - 3.0, y - 4.4, 2.2, 1.3, -30.0, shine);
        }
        2 => { // a donut: soft dough, glossy icing, sprinkles
            draw_ellipse(x, y + 1.0, 10.0, 7.5, 0.0, col((215, 155, 95), 1.0));
            draw_ellipse(x, y, 9.0, 6.4, 0.0, col(c, 1.0));
            draw_ellipse(x, y + 0.4, 3.2, 2.2, 0.0, col((90, 50, 40), 1.0));
            for k in 0..9 {
                let a = k as f32 * 0.7 + 0.3;
                let r = 5.4 + (k % 2) as f32 * 1.4;
                let (sx, sy) = (x + a.cos() * r, y + a.sin() * r * 0.7);
                let sc = CANDY[(pick + 1 + k) % CANDY.len()];
                draw_line(sx - 1.0, sy - 0.6, sx + 1.0, sy + 0.6, 1.4, col(sc, 1.0));
            }
            draw_ellipse(x - 4.0, y - 3.0, 2.6, 1.2, -15.0, shine);
        }
        _ => { // a cupcake: a pleated case, swirled frosting, a cherry
            draw_triangle(vec2(x - 8.0, y + 1.0), vec2(x + 8.0, y + 1.0), vec2(x + 6.0, y + 10.0), col(c2, 1.0));
            draw_triangle(vec2(x - 8.0, y + 1.0), vec2(x - 6.0, y + 10.0), vec2(x + 6.0, y + 10.0), col(c2, 1.0));
            for k in 0..5 {
                let px = x - 6.0 + k as f32 * 3.0;
                draw_line(px, y + 2.0, px + (k as f32 - 2.0) * 0.4, y + 9.5, 1.0, col(tint(c2, 0.75), 1.0));
            }
            for (dy, r) in [(-1.0f32, 8.0f32), (-4.5, 6.0), (-7.5, 3.8)] {
                draw_ellipse(x, y + dy, r, r * 0.6, 0.0, col(tint(c, 0.9), 1.0));
                draw_ellipse(x, y + dy - 0.6, r * 0.9, r * 0.5, 0.0, col(c, 1.0));
            }
            draw_circle(x + 0.5, y - 11.0, 2.8, col((220, 30, 60), 1.0));
            draw_line(x + 0.5, y - 13.5, x + 2.5, y - 16.0, 1.0, col((80, 140, 50), 1.0));
            draw_circle(x - 0.4, y - 11.8, 0.8, shine);
            draw_ellipse(x - 3.0, y - 4.5, 2.0, 1.0, -15.0, shine);
        }
    }
    // a sugar twinkle now and then
    let tw = (t * 2.0 + pick as f32).fract();
    if tw < 0.12 {
        let a = 1.0 - tw / 0.12;
        let (sx, sy) = (x + 6.0, y - 7.0);
        draw_line(sx - 3.0, sy, sx + 3.0, sy, 1.0, rgba(1.0, 1.0, 1.0, a));
        draw_line(sx, sy - 3.0, sx, sy + 3.0, 1.0, rgba(1.0, 1.0, 1.0, a));
    }
}

fn draw_skull(s: &Skull, ox: f32, oy: f32, t: f32) {
    let (x, y) = (s.x + ox, s.y + oy + (t * 4.0).sin() * 2.0);
    use blip::macroquad::shapes::draw_circle;
    for k in 0..3 { draw_circle(x, y, 16.0 + k as f32 * 5.0, rgba(0.6, 0.2, 0.9, 0.12)); }
    draw_ellipse(x, y, 12.0, 11.0, 0.0, rgba(0.95, 0.95, 1.0, 1.0));
    draw_ellipse(x, y + 8.0, 7.0, 4.0, 0.0, rgba(0.95, 0.95, 1.0, 1.0));
    for sgn in [-1.0f32, 1.0] {
        draw_ellipse(x + sgn * 4.5, y - 1.0, 3.5, 4.2, 0.0, rgba(0.15, 0.0, 0.25, 1.0));
        draw_circle(x + sgn * 4.5, y - 1.0, 1.4, hsv(t * 0.5, 0.7, 1.0, 1.0));
    }
    for k in -1..=1 { blip::macroquad::shapes::draw_rectangle(x + k as f32 * 3.0 - 1.0, y + 7.0, 2.0, 4.0, rgba(0.2, 0.1, 0.3, 1.0)); }
}

fn draw_world(blip: &Blip, g: &Game) {
    let t = now();
    let (ox, oy) = if g.shake > 0.0 { ((t * 90.0).sin() * g.shake * 8.0, (t * 77.0).cos() * g.shake * 8.0) } else { (0.0, 0.0) };
    draw_background(blip, g);
    draw_tiles(blip, g, ox, oy);
    for f in &g.fruits { draw_fruit(f, ox, oy); }
    for e in &g.enemies {
        if !e.active { continue; }
        let s = blip::ease_out_cubic(e.pop_in);
        let shake = if e.windup > 0.0 || e.crouch > 0.0 { (e.t * 70.0).sin() * 1.4 } else { 0.0 };
        // crouching to jump: drawn smaller, feet still on the platform
        let k = if e.crouch > 0.0 { 0.82 } else { 1.0 };
        draw_enemy(e.kind, e.x + ox + shake, e.y + oy, e.angry, e.t, s.max(0.05) * TOY_SCALE * k, s);
    }
    for (i, p) in g.p.iter().enumerate() {
        if !p.joined { continue; }
        if !p.alive && p.dead_t <= 0.0 { continue; }
        if p.safe > 0.0 && (p.safe * 12.0) as i32 % 2 == 0 && p.dead_t <= 0.0 { continue; }
        let body = if i == 0 { (90, 210, 110) } else { (90, 170, 255) };
        let spin = if p.dead_t > 0.0 { (1.4 - p.dead_t) * 9.0 } else { 0.0 };
        draw_dragon(p.cx() + ox, p.y + P_H + oy, p.face, body, p.squash, p.walk, p.blink < 0.0, p.mouth > 0.0,
            1.0, spin, TOY_SCALE, p.eat);
        // the candy flying into the open mouth
        if p.eat >= 0.0 && p.eat < EAT_FLY {
            let (mx, my) = mouth_pos(p);
            let k = blip::ease_out_cubic(p.eat / EAT_FLY);
            let (fx, fy) = (p.eaten.x + 8.0, p.eaten.y + 8.0);
            draw_candy(p.eaten.kind, candy_pick(&p.eaten), 1.0, fx + (mx - fx) * k + ox, fy + (my - fy) * k + oy, false);
        }
    }
    for b in &g.bubbles { draw_bubble(b, ox, oy, t, g.round); }
    if g.skull.active { draw_skull(&g.skull, ox, oy, t); }
    for s in &g.shots {
        let (x, y) = (s.x + ox, s.y + oy);
        if s.spark {
            let a = (4.0 - s.t).clamp(0.0, 1.0);
            blip::macroquad::shapes::draw_circle(x, y, 7.0, rgba(1.0, 0.45, 0.8, 0.25 * a));
            blip::macroquad::shapes::draw_poly(x, y, 4, 5.0, t * 400.0, rgba(1.0, 0.7, 0.95, a));
            blip::macroquad::shapes::draw_circle(x, y, 2.0, rgba(1.0, 1.0, 1.0, a));
        } else {
            // a rolling rock: the highlight turns as it goes
            blip::macroquad::shapes::draw_circle(x + 1.0, y + 2.0, 6.0, rgba(0.1, 0.05, 0.15, 0.4));
            blip::macroquad::shapes::draw_circle(x, y, 6.0, col((150, 120, 110), 1.0));
            let a = s.x / 6.0;
            blip::macroquad::shapes::draw_circle(x + a.cos() * 2.5, y + a.sin() * 2.5, 1.6, col((110, 85, 80), 1.0));
            blip::macroquad::shapes::draw_circle(x - 2.0, y - 2.2, 1.5, rgba(1.0, 0.95, 0.9, 0.7));
        }
    }
    for p in g.parts.iter() {
        let a = (p.life / p.max_life).clamp(0.0, 1.0);
        let c = BlipColor { a: p.data.c.a * a, ..p.data.c };
        if p.data.ring {
            let r = p.data.size + (1.0 - a) * 16.0;
            blip::macroquad::shapes::draw_circle_lines(p.x + ox, p.y + oy, r, 2.0 * a + 0.5, c);
        } else if p.data.star {
            let r = p.data.size * (0.6 + 0.4 * a);
            blip::macroquad::shapes::draw_poly(p.x + ox, p.y + oy, 4, r, t * 200.0, c);
        } else {
            blip::macroquad::shapes::draw_circle(p.x + ox, p.y + oy, p.data.size, c);
        }
    }
    for p in &g.pops {
        let a = (1.4 - p.t).clamp(0.0, 1.0);
        let s = format!("{}", p.value);
        let sz = 2.0 * (1.0 + 0.5 * (1.0 - blip::ease_out_cubic(p.t / 0.18)));
        let w = s.len() as f32 * 6.0 * sz;
        let x = (p.x - w / 2.0).clamp(TILE + 2.0, WIN_W as f32 - TILE - 2.0 - w);
        blip.draw_text(&s, x + 1.0, p.y + 1.0, sz, col(PLUM, a * 0.8));
        blip.draw_text(&s, x, p.y, sz, BlipColor { a, ..p.c });
    }
}

// ---- cosy text -------------------------------------------------------------
// Warm pastels on soft rounded panels, a plum shadow instead of black: the
// text sits on something, never straight over a platform or a monster.
const CREAM: (u8, u8, u8) = (255, 244, 222);
const PEACH: (u8, u8, u8) = (255, 196, 150);
const PINK: (u8, u8, u8) = (255, 170, 200);
const MINT: (u8, u8, u8) = (170, 240, 190);
const SKY: (u8, u8, u8) = (165, 215, 255);
const PLUM: (u8, u8, u8) = (60, 20, 70);

fn text_w(text: &str, sz: f32) -> f32 { text.chars().count() as f32 * 6.0 * sz - sz }

/// A rounded panel: the pill every banner sits on.
fn pill(cx: f32, y: f32, w: f32, h: f32, edge: BlipColor, a: f32) {
    use blip::macroquad::shapes::{draw_circle, draw_rectangle};
    let r = (h / 2.0).min(14.0);
    let x = cx - w / 2.0;
    let fill = col((34, 14, 44), 0.82 * a);
    let rim = BlipColor { a: edge.a * a * 0.9, ..edge };
    // border: the same shape one pixel-and-a-half bigger, underneath
    for (grow, c) in [(1.6f32, rim), (0.0, fill)] {
        let (xx, yy, ww, hh, rr) = (x - grow, y - grow, w + grow * 2.0, h + grow * 2.0, r + grow);
        draw_rectangle(xx + rr, yy, ww - rr * 2.0, hh, c);
        draw_rectangle(xx, yy + rr, ww, hh - rr * 2.0, c);
        for (px, py) in [(xx + rr, yy + rr), (xx + ww - rr, yy + rr), (xx + rr, yy + hh - rr), (xx + ww - rr, yy + hh - rr)] {
            draw_circle(px, py, rr, c);
        }
    }
    draw_rectangle(x + r, y + 2.0, w - r * 2.0, 1.5, rgba(1.0, 1.0, 1.0, 0.08 * a));
}

/// Text on its own pill, centred on the screen, popping in with `pop`.
fn cosy(blip: &Blip, text: &str, y: f32, sz: f32, c: (u8, u8, u8), edge: (u8, u8, u8), pop: f32, a: f32) {
    if a <= 0.01 { return; }
    let s = sz * (0.7 + 0.3 * blip::ease_out_cubic(pop));
    let (w, h) = (text_w(text, s), 7.0 * s);
    let (px, py) = (8.0 + s * 2.5, 6.0 + s * 1.5);
    let cx = WIN_W as f32 / 2.0;
    let top = y + (7.0 * sz - h) / 2.0;
    pill(cx, top - py, w + px * 2.0, h + py * 2.0, col(edge, 1.0), a);
    let x = cx - w / 2.0;
    blip.draw_text(text, x + s * 0.5, top + s * 0.6, s, col(PLUM, 0.9 * a));
    blip.draw_text(text, x, top, s, col(c, a));
}

/// Plain text with the soft plum shadow, left-aligned (HUD, small print).
fn soft(blip: &Blip, text: &str, x: f32, y: f32, sz: f32, c: (u8, u8, u8), a: f32) {
    blip.draw_text(text, x + sz * 0.5, y + sz * 0.5, sz, col(PLUM, 0.9 * a));
    blip.draw_text(text, x, y, sz, col(c, a));
}

fn draw_hud(blip: &Blip, g: &Game, _hi: &web::HighScore) {
    // one row: P1 on the left, the round in the middle, P2 on the right
    blip.fill_rect(0.0, 0.0, WIN_W as f32, HUD, col((26, 10, 34), 0.92));
    blip.fill_rect(0.0, HUD - 2.0, WIN_W as f32, 2.0, col(PINK, 0.25));
    // A whole-pixel size: at 1.5 the font's rows were uneven, and a phone
    // (the picture at 0.57) showed 6px text that could not be read.
    let sz = 2.0;
    let y = (HUD - 7.0 * sz) / 2.0 - 1.0;
    use blip::macroquad::shapes::draw_circle;
    for (i, colr, life) in [(0usize, MINT, (110, 220, 130)), (1, SKY, (110, 180, 255))] {
        let p = &g.p[i];
        if !p.joined {
            // player two's seat, waiting: a gentle pulse, not a blink
            let a = 0.55 + 0.45 * (now() * 3.0).sin().abs();
            let t = "2P JOIN";
            soft(blip, t, WIN_W as f32 - 10.0 - text_w(t, sz), y, sz, SKY, a);
            continue;
        }
        let score = format!("{}", p.score);
        let (sw, lives) = (text_w(&score, sz), p.lives.max(0));
        let lw = lives as f32 * 11.0;
        let x0 = if i == 0 { 10.0 } else { WIN_W as f32 - 10.0 - sw - 8.0 - lw };
        soft(blip, &score, x0, y, sz, colr, 1.0);
        for k in 0..lives {
            // a little heart per life, in the player's colour
            let (hx, hy) = (x0 + sw + 11.0 + k as f32 * 11.0, HUD / 2.0 - 2.0);
            let c = col(life, 1.0);
            draw_circle(hx - 2.0, hy, 2.6, c);
            draw_circle(hx + 2.0, hy, 2.6, c);
            draw_triangle(vec2(hx - 4.5, hy + 0.8), vec2(hx + 4.5, hy + 0.8), vec2(hx, hy + 5.5), c);
            draw_circle(hx - 2.6, hy - 0.8, 0.9, rgba(1.0, 1.0, 1.0, 0.7));
        }
    }
    let round = format!("ROUND {}", g.round + 1);
    soft(blip, &round, (WIN_W as f32 - text_w(&round, sz)) / 2.0, y, sz, PEACH, 1.0);
}

fn draw_title(blip: &Blip, g: &Game, hi: &web::HighScore) {
    let t = now();
    draw_background(blip, g);
    // bubbles drifting up behind the logo
    for i in 0..14 {
        let s = i as f32 * 53.1;
        let b = Bubble { x: (s * 11.3) % WIN_W as f32, y: WIN_H as f32 - ((s * 9.1 + t * 30.0) % (WIN_H as f32 + 40.0)) + 20.0,
            vx: 0.0, age: 0.0, phase: Phase::Float, owner: i % 2, trapped: None, active: true, wob: t + s, squish: 0.0 };
        draw_bubble(&b, 0.0, 0.0, t, 0);
    }
    // the logo: each letter a bubble-lettered bob
    let word = "BUBBLER";
    let sz = 6.0;
    let w = word.len() as f32 * 6.0 * sz;
    let x0 = (WIN_W as f32 - w) / 2.0;
    for (k, ch) in word.chars().enumerate() {
        let bob = (t * 3.0 + k as f32 * 0.6).sin() * 5.0;
        let x = x0 + k as f32 * 6.0 * sz;
        let s = ch.to_string();
        let (bx, by) = (x + 2.5 * sz, 88.0 + DY + bob + 3.5 * sz);
        let hue = hsv(0.9 + k as f32 * 0.06, 0.45, 1.0, 1.0);
        blip::macroquad::shapes::draw_circle(bx, by, 4.6 * sz, BlipColor { a: 0.16, ..hue });
        blip::macroquad::shapes::draw_circle_lines(bx, by, 4.6 * sz, 2.0, BlipColor { a: 0.7, ..hue });
        draw_ellipse(bx - 1.8 * sz, by - 2.2 * sz, 1.3 * sz, 0.7 * sz, -35.0, rgba(1.0, 1.0, 1.0, 0.7));
        for (dx, dy) in [(-2.0f32, 0.0f32), (2.0, 0.0), (0.0, -2.0), (0.0, 2.0)] {
            blip.draw_text(&s, x + dx, 88.0 + DY + bob + dy, sz, rgba(0.15, 0.02, 0.25, 0.9));
        }
        blip.draw_text(&s, x, 88.0 + DY + bob, sz, hue);
    }
    // the two of them, bouncing
    for (i, x) in [WIN_W as f32 / 2.0 - 70.0, WIN_W as f32 / 2.0 + 70.0].iter().enumerate() {
        let hop = ((t * 3.2 + i as f32 * 1.6).sin()).max(0.0);
        let foot = 278.0 + DY - hop * 34.0;
        let sq = if hop < 0.08 { 0.8 } else { 1.0 + hop * 0.12 };
        let body = if i == 0 { (90, 210, 110) } else { (90, 170, 255) };
        draw_dragon(*x, foot, if i == 0 { 1.0 } else { -1.0 }, body, sq, t * 2.0, (t * 0.7 + i as f32) % 3.0 < 0.1,
            (t * 1.3 + i as f32 * 0.5) % 2.0 < 0.2, 1.0, 0.0, 2.4, -1.0);
    }
    let glow = 0.65 + 0.35 * (t * 2.5).sin().abs();
    let by = web::controls();
    cosy(blip, by.pick("P1 PRESS BUBBLE", "P1 PRESS BUBBLE", "P1 TOUCH TO START"), 300.0 + DY, 2.0, MINT, MINT, 1.0, glow);
    let two = by.pick("P2 PRESS J TO JOIN", "P2 TOUCH TO JOIN", "P2 TOUCH TO JOIN");
    // Whole-pixel sizes from here down: at 1.2 to 1.5 the font's rows are uneven.
    soft(blip, two, (WIN_W as f32 - text_w(two, 2.0)) / 2.0, 334.0 + DY, 2.0, SKY, 1.0);
    // how to play, on its own panel
    let l1 = by.pick("MOVE A D   JUMP W   BUBBLE F", "PAD MOVES   BUTTONS BUBBLE AND JUMP", "TOUCH TO BUBBLE   SLIDE TO RUN");
    let l2 = by.pick("HOLD JUMP TO RIDE BUBBLES", "HOLD JUMP TO RIDE BUBBLES", "SWIPE UP TO JUMP");
    pill(WIN_W as f32 / 2.0, 358.0 + DY, text_w(l1, 2.0) + 36.0, 54.0, col(PEACH, 0.8), 0.9);
    soft(blip, l1, (WIN_W as f32 - text_w(l1, 2.0)) / 2.0, 368.0 + DY, 2.0, CREAM, 0.95);
    soft(blip, l2, (WIN_W as f32 - text_w(l2, 2.0)) / 2.0, 388.0 + DY, 2.0, CREAM, 0.95);
    if hi.score > 0 {
        let h = hi.label("HI");
        soft(blip, &h, (WIN_W as f32 - text_w(&h, 2.0)) / 2.0, 428.0 + DY, 2.0, PEACH, 1.0);
    }
    let _ = g;
}

/// Playtest autopilot, native only (BUBBLER_BOT=1 or 2 players). Hunts the
/// nearest monster, blows when it is level and in range, goes up to pop
/// trapped bubbles, hops over trouble. Deliberately no better than a
/// decent player: it reacts late and misjudges now and then.
#[cfg(not(target_arch = "wasm32"))]
fn bot_input(g: &Game, pi: usize, mem: &mut [f32; 4]) -> Input {
    let mut k = Input::default();
    let passive = std::env::var_os("BUBBLER_PASSIVE").is_some();
    let p = &g.p[pi];
    if !p.joined || !p.alive || p.dead_t > 0.0 { return k; }
    let (px, py) = (p.cx(), p.cy());
    mem[0] -= 1.0 / 60.0;
    // danger: a monster about to walk into us
    let danger = g.enemies.iter().filter(|e| e.active)
        .map(|e| (e.x + E_W / 2.0 - px, e.y + E_H / 2.0 - py))
        .find(|(dx, dy)| dx.abs() < 34.0 && dy.abs() < 22.0);
    // trapped bubbles are the priority: go and pop them
    let trapped = g.bubbles.iter().filter(|b| b.trapped.is_some())
        .min_by(|a, b| (a.x - px).abs().total_cmp(&(b.x - px).abs()));
    let fruit = g.fruits.iter().filter(|f| f.on_ground && (f.y + 8.0 - py).abs() < 30.0)
        .min_by(|a, b| (a.x - px).abs().total_cmp(&(b.x - px).abs()));
    let target = if let Some(b) = trapped { Some((b.x, b.y, true)) }
    else if let Some(f) = fruit.filter(|f| (f.x - px).abs() < 160.0) { Some((f.x + 8.0, py, true)) } else {
        g.enemies.iter().filter(|e| e.active && e.pop_in >= 1.0)
            .min_by(|a, b| ((a.x - px).hypot((a.y - py) * 1.6)).total_cmp(&(b.x - px).hypot((b.y - py) * 1.6)))
            .map(|e| (e.x + E_W / 2.0, e.y + E_H / 2.0, false))
    };
    // in the air over a monster: steer clear of it
    if !p.on_ground {
        if let Some(e) = g.enemies.iter().filter(|e| e.active)
            .find(|e| (e.x + E_W / 2.0 - px).abs() < 30.0 && e.y > py && e.y - py < 90.0) {
            let away = if e.x + E_W / 2.0 > px { -1.0 } else { 1.0 };
            k.left = away < 0.0; k.right = away > 0.0;
            return k;
        }
    }
    if let Some((dx, _)) = danger {
        // hop away / over
        k.left = dx > 0.0; k.right = dx < 0.0;
        if p.on_ground && blip::rand_range_f32(0.0, 1.0) < 0.25 { k.jump = true; }
        // and bubble it if we face it
        if (dx > 0.0) == (p.face > 0.0) && !passive { k.blow = true; }
        k.jump_held = true;
        return k;
    }
    if let Some((tx, ty, is_bubble)) = target {
        let dx = tx - px;
        let dy = ty - py;
        if is_bubble {
            if dx.abs() > 10.0 { k.left = dx < 0.0; k.right = dx > 0.0; }
            if p.on_ground && (dy < -20.0 || dx.abs() < 40.0) && mem[0] <= 0.0 { k.jump = true; mem[0] = 0.3; }
            k.jump_held = false; // bump into it, do not ride it
        } else {
            let level = dy.abs() < 18.0;
            if level && dx.abs() < 150.0 {
                // face it and blow
                if (dx > 0.0) != (p.face > 0.0) { k.left = dx < 0.0; k.right = dx > 0.0; }
                else if blip::rand_range_f32(0.0, 1.0) < 0.5 && !passive { k.blow = true; }
                if dx.abs() > 70.0 { k.left = dx < 0.0; k.right = dx > 0.0; }
            } else {
                if dx.abs() > 30.0 || dy > 0.0 {
                    // head for it; to go down walk off an edge, to go up jump
                    let wander = if mem[1] > 0.0 { mem[2] } else { 0.0 };
                    let dir = if wander != 0.0 { wander } else { dx.signum() };
                    k.left = dir < 0.0; k.right = dir > 0.0;
                }
                if dy < -30.0 && p.on_ground && mem[0] <= 0.0 && blip::rand_range_f32(0.0, 1.0) < 0.12 { k.jump = true; mem[0] = 0.4; }
            }
        }
        // stuck against a wall or under a ledge: wander a bit
        mem[1] -= 1.0 / 60.0;
        if (p.vx.abs() < 5.0 && (k.left || k.right)) && mem[1] <= 0.0 {
            mem[1] = 0.8;
            mem[2] = if blip::rand_range_f32(0.0, 1.0) < 0.5 { -1.0 } else { 1.0 };
        }
    }
    k.jump_held = k.jump_held || k.jump;
    k
}

fn conf() -> blip::macroquad::window::Conf { window_conf("BUBBLER", WIN_W, WIN_H) }

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    // Bright filled blocks twitter under the full interlace (see Brawler).
    blip.set_interlace(0.5);
    let mut g = Game::new();
    web::set_players(2);

    use blip_assets::bubbler::{jingle_wav, sfx_wav, theme_wav};
    let load = |b: Vec<u8>| async move { blip::audio::load_sound(&b).await };
    let sfx = Sounds {
        blow: load(sfx_wav(0)).await, pop: load(sfx_wav(1)).await, trap: load(sfx_wav(2)).await,
        kill: load(sfx_wav(3)).await, fruit: load(sfx_wav(4)).await, jump: load(sfx_wav(5)).await,
        lose: load(sfx_wav(6)).await, bounce: load(sfx_wav(7)).await, sparkle: load(sfx_wav(8)).await,
        round: load(jingle_wav(0)).await, clear: load(jingle_wav(1)).await, hurry: load(jingle_wav(2)).await,
        over: load(jingle_wav(3)).await, won: load(jingle_wav(4)).await,
    };
    // The theme, and the same tune faster for HURRY UP.
    let mut music = Jukebox::new(&[theme_wav, blip_assets::bubbler::hurry_wav]);
    music.start(0).await;
    let mut shot_frame = 0u32;
    #[cfg(not(target_arch = "wasm32"))]
    let mut scene_set = false;
    #[cfg(not(target_arch = "wasm32"))]
    let bot: u32 = std::env::var("BUBBLER_BOT").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    #[cfg(not(target_arch = "wasm32"))]
    let speed: u32 = std::env::var("BUBBLER_SPEED").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    #[cfg(not(target_arch = "wasm32"))]
    let (mut bot_mem, mut bot_clock, mut bot_rounds) = ([[0.0f32; 4]; 2], 0.0f32, Vec::<u32>::new());

    loop {
        #[cfg(not(target_arch = "wasm32"))]
        { bot_clock += blip.delta_time; }
        let dt = blip.delta_time;
        g.state_t += dt;
        let hi = web::high_score();

        // The cabinet card: two dragons mid-round, bubbles up, a chain ready.
        if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 {
                g.start(true);
                g.state = State::Play;
                g.p[0].x = 150.0; g.p[0].y = HUD + 19.0 * TILE - P_H; g.p[0].mouth = 10.0; g.p[0].safe = 0.0;
                g.p[1].x = 360.0; g.p[1].y = HUD + 15.0 * TILE - P_H; g.p[1].face = -1.0; g.p[1].safe = 0.0;
                for (k, e) in g.enemies.iter_mut().enumerate() {
                    if k < 3 { e.active = false; }
                    e.pop_in = 1.0;
                }
                for (k, (x, y)) in [(282.0, 70.0), (306.0, 72.0), (330.0, 70.0), (200.0, 260.0), (230.0, 390.0)].iter().enumerate() {
                    g.bubbles.push(Bubble { x: *x, y: *y, vx: 0.0, age: 1.0, phase: if k < 3 { Phase::Top } else { Phase::Float },
                        owner: k % 2, trapped: if k < 3 { Some((Kind::Walker, false)) } else { None }, active: true,
                        wob: k as f32, squish: 0.0 });
                }
                // one of each treat on the floor
                for (kind, x) in [(0usize, 150.0f32), (1, 196.0), (2, 410.0), (3, 456.0)] {
                    g.fruits.push(Fruit { x, y: HUD + 23.0 * TILE - 16.0, vx: 0.0, vy: 0.0, kind, t: 1.0, on_ground: true, active: true });
                }
            }
        }

        // Native-only: BUBBLER_SCENE=intro|play|hurry|chain|clear|over|won
        // jumps to that screen and freezes it, for checking the text.
        #[cfg(not(target_arch = "wasm32"))]
        let frozen = if let Ok(scene) = std::env::var("BUBBLER_SCENE") {
            if !scene_set {
                scene_set = true;
                if scene != "title" { g.start(true); g.state = State::Play; }
                for e in g.enemies.iter_mut() { e.pop_in = 1.0; }
                for p in g.p.iter_mut() { p.safe = 0.0; p.score = 128_450; }
                g.p[1].score = 97_320;
                match scene.as_str() {
                    "intro" => { g.state = State::Intro; g.state_t = 1.2; }
                    "hurry" => { g.round_t = hurry_at(g.round) + 0.1; g.hurry = true; }
                    "chain" => { g.chain = (3, 1.0); }
                    "solo" => { g.p[1].joined = false; g.two_up = false; }
                    "clear" => { g.state = State::Clear; g.state_t = 0.8; }
                    "over" => { g.state = State::Over; g.state_t = 3.0; }
                    "won" => { g.state = State::Won; g.state_t = 3.0; }
                    _ => {}
                }
            }
            true
        } else { false };
        #[cfg(target_arch = "wasm32")]
        let frozen = false;
        let solo = !g.two_up;
        #[allow(unused_mut)]
        let mut inp = [read(if solo { &SOLO } else { &P1_KEYS }), read(&P2_KEYS)];
        #[allow(unused_mut)]
        let mut p2_start = key_pressed(BLIP_KEY_J) || key_pressed(BLIP_KEY_K);
        #[cfg(not(target_arch = "wasm32"))]
        if bot > 0 && std::env::var("BUBBLER_MAXT").ok().and_then(|v| v.parse::<f32>().ok()).map_or(false, |m| bot_clock > m) {
            g.state = State::Over;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if bot > 0 {
            match g.state {
                State::Title => { if bot == 2 { p2_start = true; } else { inp[0].blow = true; } }
                State::Over | State::Won => {
                    {
                        let st = &g.stats;
                        println!("RESULT state={} round={} time={:.0}s score={}+{} traps={} escapes={} pops={} kills={} chains={:?} fruit={} deaths_enemy={} deaths_skull={} hurries={} skulls={} bounces={} by_kind(w,h,g)={:?} by_round={:?} rounds={:?}",
                            if g.state == State::Won { "won" } else { "over" }, g.round + 1, bot_clock, g.p[0].score, g.p[1].score,
                            st.traps, st.escapes, st.pops, st.kills, st.chains, st.fruit, st.deaths_enemy, st.deaths_skull,
                            st.hurries, st.skulls, st.bounces, st.deaths_by, st.deaths_round, bot_rounds);
                        std::process::exit(0);
                    }
                }
                _ => { inp = [bot_input(&g, 0, &mut bot_mem[0]), bot_input(&g, 1, &mut bot_mem[1])]; }
            }
            if g.state == State::Clear && g.state_t < dt * 1.5 { bot_rounds.push(g.round_t as u32); }
        }

        if !frozen { match g.state {
            State::Title => {
                // A game is paid for as it starts: one coin, two for two players.
                if p2_start { g.start(true); web::spend_coin(); web::spend_coin(); }
                else if inp[0].blow || inp[0].jump || key_pressed(BLIP_KEY_SPACE) { g.start(false); web::spend_coin(); }
            }
            State::Intro => {
                if g.state_t < dt * 1.5 { play_sfx(&sfx.round); }
                for e in g.enemies.iter_mut() { e.pop_in = (g.state_t / 1.2).min(0.999); }
                if g.state_t > 1.8 {
                    g.state = State::Play;
                    g.state_t = 0.0;
                    for e in g.enemies.iter_mut() { e.pop_in = 1.0; }
                }
            }
            State::Play => {
                if !g.p[1].joined && (key_pressed(BLIP_KEY_J) || key_pressed(BLIP_KEY_K)) { g.join_p2(); }
                if !blip.screenshot_mode || shot_frame > 400 { update_play(&mut g, dt, inp, &sfx); }
                #[cfg(not(target_arch = "wasm32"))]
                for _ in 1..speed {
                    if g.state != State::Play { break; }
                    let extra = [bot_input(&g, 0, &mut bot_mem[0]), bot_input(&g, 1, &mut bot_mem[1])];
                    update_play(&mut g, dt, extra, &sfx);
                    update_fx(&mut g, dt);
                    bot_clock += dt;
                }
            }
            State::Clear => {
                update_players(&mut g, inp, dt, &sfx);
                update_bubbles(&mut g, dt, inp, &sfx);
                update_fruit(&mut g, dt, &sfx);
                // leftover bubbles pop one by one, for a little applause
                if g.state_t > 1.0 {
                    if let Some(i) = g.bubbles.iter().position(|b| b.active) {
                        if (g.state_t * 8.0) as i32 != ((g.state_t - dt) * 8.0) as i32 {
                            let b = g.bubbles[i];
                            g.bubbles[i].active = false;
                            g.burst(b.x, b.y, 8, rgba(1.0, 1.0, 1.0, 0.8), 80.0, false);
                            play_sfx_volume(&sfx.pop, 0.4);
                        }
                    }
                    g.bubbles.retain(|b| b.active);
                }
                if g.state_t > 3.4 {
                    if g.round + 1 >= ROUNDS {
                        g.state = State::Won;
                        g.state_t = 0.0;
                        report_score(&g);
                        play_sfx(&sfx.won);
                    } else {
                        g.load_round(g.round + 1);
                        g.state = State::Intro;
                        g.state_t = 0.0;
                    }
                }
            }
            State::Over | State::Won => {
                if g.state_t > 2.5 && (inp[0].blow || inp[0].jump || key_pressed(BLIP_KEY_SPACE) || p2_start) {
                    g.state = State::Title;
                    g.two_up = false;
                    web::set_players(2);
                }
            }
        }
        }
        if !frozen { update_fx(&mut g, dt); }

        let want_music = matches!(g.state, State::Play | State::Intro | State::Clear | State::Title);
        if want_music {
            music.play(if g.state == State::Play && g.hurry { 1 } else { 0 });
        } else {
            music.stop();
        }
        if g.state == State::Title { music.warm_up().await; }

        blip.clear(BlipColor::new(0.0, 0.0, 0.0, 1.0));
        match g.state {
            State::Title => draw_title(&blip, &g, &hi),
            _ => {
                draw_world(&blip, &g);
                draw_hud(&blip, &g, &hi);
                let st = g.state_t;
                // the end screens dim the world so their words sit on calm
                if matches!(g.state, State::Over | State::Won) {
                    blip.fill_rect(0.0, HUD, WIN_W as f32, WIN_H as f32 - HUD, col((20, 8, 28), 0.55 * (st * 2.0).min(1.0)));
                }
                match g.state {
                    State::Intro => {
                        cosy(&blip, &format!("ROUND {}", g.round + 1), 170.0 + DY, 3.5, PEACH, PEACH, st * 3.0, (st * 4.0).min(1.0));
                        if st > 0.7 { cosy(&blip, "READY!", 236.0 + DY, 2.2, CREAM, MINT, (st - 0.7) * 3.0, ((st - 0.7) * 4.0).min(1.0)); }
                    }
                    State::Play => {
                        if g.hurry && g.round_t < hurry_at(g.round) + 2.4 {
                            let a = 0.6 + 0.4 * (g.round_t * 8.0).sin().abs();
                            cosy(&blip, "HURRY UP!", 196.0 + DY, 3.2, (255, 150, 150), (255, 110, 120), 1.0, a);
                        }
                    }
                    _ => {}
                }
                if g.chain.1 > 0.0 {
                    let text = format!("{} CHAIN!", g.chain.0);
                    cosy(&blip, &text, 262.0 + DY, 2.6, CREAM, PINK, (1.2 - g.chain.1) * 5.0, g.chain.1.min(1.0));
                }
                // bubble wipe: out at the end of a round, back in at the start of the next
                let wipe = match g.state {
                    State::Clear => ((g.state_t - 2.6) / 0.8).clamp(0.0, 1.0),
                    State::Intro => 1.0 - (g.state_t / 0.7).clamp(0.0, 1.0),
                    _ => 0.0,
                };
                if wipe > 0.0 {
                    let pal = &PALETTES[g.round];
                    for i in 0..12 {
                        for j in 0..12 {
                            let (cx, cy) = (i as f32 * 60.0, j as f32 * 57.0);
                            let r = wipe * 46.0 * (0.8 + 0.2 * ((i * 7 + j * 3) % 5) as f32 / 4.0);
                            blip::macroquad::shapes::draw_circle(cx, cy, r, col(pal.sky0, 1.0));
                            blip::macroquad::shapes::draw_circle_lines(cx, cy, r, 2.0, col(pal.glow, 0.5 * wipe));
                        }
                    }
                }
                match g.state {
                    State::Clear => {
                        cosy(&blip, "CLEAR!", 186.0 + DY, 4.0, CREAM, MINT, st * 3.0, (st * 4.0).min(1.0));
                        if g.extend && st > 0.6 {
                            cosy(&blip, "EXTEND  +1 LIFE", 246.0 + DY, 1.8, PEACH, PINK, (st - 0.6) * 3.0, ((st - 0.6) * 4.0).min(1.0));
                        }
                    }
                    State::Over => {
                        cosy(&blip, "GAME OVER", 160.0 + DY, 3.8, PINK, PINK, st * 2.0, (st * 3.0).min(1.0));
                        if hi.score > 0 {
                            cosy(&blip, &hi.label("HI"), 222.0 + DY, 1.5, PEACH, PEACH, 1.0, (st - 1.0).clamp(0.0, 1.0));
                        }
                        if st > 2.5 {
                            cosy(&blip, "PRESS BUBBLE", 276.0 + DY, 2.0, CREAM, MINT, 1.0, 0.65 + 0.35 * (st * 2.5).sin().abs());
                        }
                    }
                    State::Won => {
                        cosy(&blip, "ALL ROUNDS CLEAR!", 120.0 + DY, 2.6, CREAM, MINT, st * 2.0, (st * 3.0).min(1.0));
                        cosy(&blip, "HAPPY END", 192.0 + DY, 3.8, PEACH, PINK, (st - 0.5) * 2.0, ((st - 0.5) * 3.0).clamp(0.0, 1.0));
                        if st > 2.5 {
                            cosy(&blip, "PRESS BUBBLE", 276.0 + DY, 2.0, CREAM, MINT, 1.0, 0.65 + 0.35 * (st * 2.5).sin().abs());
                        }
                    }
                    _ => {}
                }
            }
        }
        blip.next_frame(60).await;
    }
}
