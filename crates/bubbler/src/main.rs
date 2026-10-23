//! Bubble Bobble-inspired platform game with one- or two-player co-op.

use blip::input::{
    key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_D, BLIP_KEY_F, BLIP_KEY_G, BLIP_KEY_J, BLIP_KEY_K,
    BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::input::KeyCode;
use blip::macroquad::math::vec2;
use blip::macroquad::shapes::{draw_ellipse, draw_triangle};
use blip::{
    hsv, play_sfx, play_sfx_volume, segment_circle_overlap, segment_rect_overlap, web, window_conf, Blip, BlipColor,
    Jukebox,
};
// The effects and music are made by build.rs; the tests check them here.
#[cfg(test)]
mod tunes;
#[cfg(test)]
mod gfx;
mod art;
mod levels;
use levels::{LEVELS, PALETTES, ROCKS_FROM, ROUNDS, ROWS, SPARKS_FROM};

// ---- layout -----------------------------------------------------------
const TILE: f32 = 24.0;
const COLS: usize = 26;
const HUD: f32 = 24.0;
const WIN_W: i32 = 624;
const WIN_H: i32 = 600; // HUD + 24 rows
/// Screen text was laid out for the 456px screen; this centres it on this one.
const DY: f32 = (WIN_H as f32 - 456.0) / 2.0;
const GAP: (usize, usize) = (11, 14); // the hole in the floor and ceiling you fall through

// ---- feel ---------------------------------------------------------------
const GRAV: f32 = 950.0;
const MAX_FALL: f32 = 300.0; // a floaty fall: you steer your landings
const JUMP_V: f32 = 470.0; // ~4.8 tiles, one platform up with room to spare
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
/// A bubble with a toy in it swells to hold the whole toy.
const TRAP_R: f32 = 19.0;
const SHOOT_V: f32 = 340.0;
const SHOOT_T: f32 = 0.4; // the blown bubble traps only while it is still flying
const RISE: f32 = 44.0;
const RISE_TRAPPED: f32 = 62.0; // catches reach the ceiling cluster sooner, where chains happen
const CHAIN_REACH: f32 = 1.35; // bubbles this many diameters apart pop together
const FREE_LIFE: f32 = 9.0;
const TRAP_LIFE: f32 = 10.0; // round 1; then the monster breaks out, angry
const BLOW_CD: f32 = 0.24;
const MAX_BUBBLES: usize = 36;

// ---- pressure -----------------------------------------------------------
const HURRY_AT: f32 = 70.0; // round 1: little players take their time
const SKULL_AT: f32 = 95.0;

// Rounds cap at 5: the autopilot lost all lives on rounds 7 and 8 at higher difficulty.
const RAMP_TOP: usize = 4;
fn monster_pace(round: usize) -> f32 {
    0.8 + 0.04 * round.min(RAMP_TOP) as f32
}
fn trap_life(round: usize) -> f32 {
    (TRAP_LIFE - 0.5 * round.min(RAMP_TOP) as f32).max(7.0)
}
fn hurry_at(round: usize) -> f32 {
    HURRY_AT - 2.5 * round.min(RAMP_TOP) as f32
}
fn skull_at(round: usize) -> f32 {
    SKULL_AT - 3.5 * round.min(RAMP_TOP) as f32
}
const ROCK_V: f32 = 170.0;
const ROCK_WINDUP: f32 = 0.6;
/// How long a monster crouches before it jumps: long enough to see.
const JUMP_TELL: f32 = 0.22;
const EXTEND_MAX: i32 = 7; // lives a round clear can top a player up to
const SPARK_V: f32 = 95.0;

fn col(c: (u8, u8, u8), a: f32) -> BlipColor {
    BlipColor::new(c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0, a)
}
fn rgba(r: f32, g: f32, b: f32, a: f32) -> BlipColor {
    BlipColor::new(r, g, b, a)
}
fn now() -> f32 {
    blip::macroquad::time::get_time() as f32
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum State {
    Title,
    Intro,
    Play,
    Clear,
    Over,
    Won,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Kind {
    Walker,
    Hopper,
    Ghost,
}

#[derive(Default, Clone, Copy)]
struct Input {
    left: bool,
    right: bool,
    jump: bool,
    jump_held: bool,
    blow: bool,
}

struct KeySet {
    left: &'static [KeyCode],
    right: &'static [KeyCode],
    jump: &'static [KeyCode],
    blow: &'static [KeyCode],
}
// Alone, player one has every key; with a partner the arrows are theirs.
const SOLO: KeySet = KeySet {
    left: &[BLIP_KEY_A, BLIP_KEY_LEFT],
    right: &[BLIP_KEY_D, BLIP_KEY_RIGHT],
    jump: &[BLIP_KEY_W, BLIP_KEY_UP, BLIP_KEY_G, BLIP_KEY_BUTTON2],
    blow: &[BLIP_KEY_F, BLIP_KEY_SPACE],
};
const P1_KEYS: KeySet = KeySet {
    left: &[BLIP_KEY_A],
    right: &[BLIP_KEY_D],
    jump: &[BLIP_KEY_W, BLIP_KEY_G, BLIP_KEY_BUTTON2],
    blow: &[BLIP_KEY_F, BLIP_KEY_SPACE],
};
const P2_KEYS: KeySet =
    KeySet { left: &[BLIP_KEY_LEFT], right: &[BLIP_KEY_RIGHT], jump: &[BLIP_KEY_UP, BLIP_KEY_K], blow: &[BLIP_KEY_J] };
fn read(k: &KeySet) -> Input {
    let any_held = |ks: &[KeyCode]| ks.iter().any(|c| key_held(*c) || key_pressed(*c));
    let any_pressed = |ks: &[KeyCode]| ks.iter().any(|c| key_pressed(*c));
    Input {
        left: any_held(k.left),
        right: any_held(k.right),
        jump: any_pressed(k.jump),
        jump_held: any_held(k.jump),
        blow: any_pressed(k.blow),
    }
}

struct Player {
    joined: bool, // in this game
    alive: bool,  // on the board (not between lives, not out)
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    face: f32,
    on_ground: bool,
    lives: i32,
    score: i32,
    blow_cd: f32,
    mouth: f32, // > 0 while the cheeks puff out a bubble
    /// Eating a candy: seconds since it was picked up (negative when not),
    /// and the candy itself, flying in from where it lay.
    eat: f32,
    eaten: Fruit,
    safe: f32,   // respawn protection
    dead_t: f32, // > 0 while the losing-a-life animation plays
    squash: f32, // landing squash / jump stretch, eased back to 1
    walk: f32,
    blink: f32,
    spawn: (f32, f32),
}

impl Player {
    fn new(spawn: (f32, f32), face: f32) -> Self {
        Self {
            joined: false,
            alive: false,
            x: spawn.0,
            y: spawn.1,
            vx: 0.0,
            vy: 0.0,
            face,
            on_ground: false,
            lives: LIVES,
            score: 0,
            blow_cd: 0.0,
            mouth: 0.0,
            safe: 0.0,
            dead_t: 0.0,
            squash: 1.0,
            walk: 0.0,
            eat: -1.0,
            eaten: Fruit { x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, kind: 0, t: 0.0, on_ground: true, active: false },
            blink: blip::rand_range_f32(1.0, 4.0),
            spawn,
        }
    }
    fn place(&mut self) {
        self.x = self.spawn.0;
        self.y = self.spawn.1;
        self.vx = 0.0;
        self.vy = 0.0;
        self.on_ground = false;
        self.alive = true;
        self.dead_t = 0.0;
        self.safe = RESPAWN_SAFE;
    }
    fn cx(&self) -> f32 {
        self.x + P_W / 2.0
    }
    fn cy(&self) -> f32 {
        self.y + P_H / 2.0
    }
}

#[derive(Clone, Copy)]
struct Enemy {
    kind: Kind,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    dir: f32,
    on_ground: bool,
    angry: bool,
    t: f32,
    jump_cd: f32,
    edge_cd: f32,
    active: bool,
    pop_in: f32,
    shot_cd: f32,
    windup: f32,
    /// Crouching before a jump (seconds left), and the leap (vx, vy) it springs into.
    crouch: f32,
    leap: (f32, f32),
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Shoot,
    Float,
    Top,
}

#[derive(Clone, Copy)]
struct Bubble {
    x: f32,
    y: f32,
    vx: f32,
    age: f32,
    phase: Phase,
    owner: usize,
    trapped: Option<(Kind, bool)>,
    active: bool,
    wob: f32,
    squish: f32,
}

impl Bubble {
    fn radius(&self) -> f32 {
        if self.trapped.is_some() {
            TRAP_R
        } else {
            BUB_R
        }
    }
    /// Where it rests against the ceiling.
    fn top_y(&self) -> f32 {
        HUD + TILE + self.radius() + 4.0
    }
}

#[derive(Clone, Copy)]
struct Fruit {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    kind: usize,
    t: f32,
    on_ground: bool,
    active: bool,
}

#[derive(Clone, Copy)]
struct ParticleStyle {
    c: BlipColor,
    size: f32,
    star: bool,
    ring: bool,
}

#[derive(Clone, Copy)]
struct Popup {
    x: f32,
    y: f32,
    t: f32,
    value: i32,
    c: BlipColor,
}

struct Skull {
    active: bool,
    x: f32,
    y: f32,
    speed: f32,
}

/// A monster's missile: a rock rolled along a platform, or a ghost's spark.
#[derive(Clone, Copy)]
struct Shot {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    t: f32,
    spark: bool,
    active: bool,
}

/// Tallies for a playtest run (printed by the native autopilot).
#[derive(Default, Clone)]
struct Stats {
    traps: u32,
    escapes: u32,
    pops: u32,
    kills: u32,
    chains: [u32; 6],
    fruit: u32,
    deaths_enemy: u32,
    deaths_skull: u32,
    deaths_by: [u32; 3],
    deaths_round: [u32; ROUNDS],
    hurries: u32,
    skulls: u32,
    bounces: u32,
}

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
    if !g.two_up {
        web::report_score(g.p[0].score);
    }
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
            stats: Stats::default(),
            chain: (0, 0.0),
            state: State::Title,
            round: 0,
            tiles: [[false; COLS]; ROWS],
            p: [Player::new(a, 1.0), Player::new(b, -1.0)],
            enemies: Vec::new(),
            bubbles: Vec::new(),
            fruits: Vec::new(),
            parts: blip::EffectParticles::new(60.0, 2.0),
            pops: Vec::new(),
            skull: Skull { active: false, x: 0.0, y: 0.0, speed: 0.0 },
            shots: Vec::new(),
            round_t: 0.0,
            state_t: 0.0,
            hurry: false,
            shake: 0.0,
            two_up: false,
            extend: false,
        };
        g.load_round(0);
        g
    }

    fn load_round(&mut self, round: usize) {
        self.round = round;
        for (r, row) in LEVELS[round].iter().enumerate() {
            for (c, ch) in row.chars().enumerate() {
                self.tiles[r][c] = ch == '#';
            }
        }
        let (a, b, es) = level_spawns(round);
        self.p[0].spawn = a;
        self.p[1].spawn = b;
        self.enemies = es
            .into_iter()
            .map(|(kind, x, y)| Enemy {
                kind,
                x,
                y,
                vx: 0.0,
                vy: 0.0,
                dir: if x < WIN_W as f32 / 2.0 { 1.0 } else { -1.0 },
                on_ground: false,
                angry: false,
                t: blip::rand_range_f32(0.0, 3.0),
                jump_cd: blip::rand_range_f32(0.8, 2.0),
                edge_cd: 0.0,
                active: true,
                pop_in: 0.0,
                shot_cd: blip::rand_range_f32(2.5, 4.5),
                windup: 0.0,
                crouch: 0.0,
                leap: (0.0, 0.0),
            })
            .collect();
        for e in self.enemies.iter_mut() {
            if e.kind == Kind::Ghost {
                e.vx = e.dir * 62.0;
                e.vy = 62.0;
            }
        }
        self.bubbles.clear();
        self.fruits.clear();
        self.shots.clear();
        self.skull.active = false;
        self.round_t = 0.0;
        self.hurry = false;
        for i in 0..2 {
            self.p[i].face = if i == 0 { 1.0 } else { -1.0 };
            if self.p[i].joined && self.p[i].lives > 0 {
                self.p[i].place();
            }
        }
    }

    fn start_game(&mut self, two: bool) {
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
        let first = std::env::var("BUBBLER_ROUND")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .map_or(0, |r| r.clamp(1, ROUNDS) - 1);
        #[cfg(target_arch = "wasm32")]
        let first = 0;
        self.load_round(first);
        self.state = State::Intro;
        self.state_t = 0.0;
    }

    fn join_player_two(&mut self) {
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
        if c < 0 || c >= COLS as i32 || r < 1 || r >= ROWS as i32 {
            return false;
        }
        self.tiles[r as usize][c as usize]
    }

    /// Anything under the span x0..x1 at the row whose top edge is `foot`?
    fn floor_at(&self, x0: f32, x1: f32, foot: f32) -> bool {
        let r = ((foot - HUD) / TILE).round() as i32;
        if ((HUD + r as f32 * TILE) - foot).abs() > 0.5 {
            return false;
        }
        let (c0, c1) = ((x0 / TILE).floor() as i32, ((x1 - 0.01) / TILE).floor() as i32);
        (c0..=c1).any(|c| self.platform(c, r))
    }

    /// Integrate a body with one-way platforms: it passes up through
    /// blocks and lands on their tops. Returns (on_ground, hit_wall).
    fn move_body(&self, x: &mut f32, y: &mut f32, vx: &mut f32, vy: &mut f32, w: f32, h: f32, dt: f32) -> (bool, bool) {
        *vy = (*vy + GRAV * dt).min(MAX_FALL);
        *x += *vx * dt;
        let mut wall = false;
        let (lo, hi) = (TILE, WIN_W as f32 - TILE - w);
        if *x < lo {
            *x = lo;
            wall = true;
        }
        if *x > hi {
            *x = hi;
            wall = true;
        }
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
        if !in_gap && *y < HUD + TILE {
            *y = HUD + TILE;
            if *vy < 0.0 {
                *vy = 0.0;
            }
        }
        // down the hole in the floor, back in through the one in the ceiling
        if *y > WIN_H as f32 {
            *y = HUD - h;
        }
        (ground, wall)
    }

    /// An expanding shockwave ring.
    fn ring(&mut self, x: f32, y: f32, c: BlipColor, size: f32) {
        self.parts.push(blip::EffectParticle {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            life: 0.35,
            max_life: 0.35,
            data: ParticleStyle { c, size, star: false, ring: true },
        });
    }

    fn burst(&mut self, x: f32, y: f32, n: usize, c: BlipColor, speed: f32, star: bool) {
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU + blip::rand_range_f32(-0.3, 0.3);
            let s = speed * blip::rand_range_f32(0.5, 1.0);
            let life = blip::rand_range_f32(0.35, 0.7);
            self.parts.push(blip::EffectParticle {
                x,
                y,
                vx: a.cos() * s,
                vy: a.sin() * s,
                life,
                max_life: life,
                data: ParticleStyle {
                    c,
                    size: if star { blip::rand_range_f32(2.5, 4.0) } else { blip::rand_range_f32(1.2, 2.6) },
                    star,
                    ring: false,
                },
            });
        }
    }
}

struct Sounds {
    blow: blip::BlipSound,
    pop: blip::BlipSound,
    trap: blip::BlipSound,
    kill: blip::BlipSound,
    fruit: blip::BlipSound,
    jump: blip::BlipSound,
    lose: blip::BlipSound,
    bounce: blip::BlipSound,
    sparkle: blip::BlipSound,
    round: blip::BlipSound,
    clear: blip::BlipSound,
    hurry: blip::BlipSound,
    over: blip::BlipSound,
    won: blip::BlipSound,
}

// ---- update ------------------------------------------------------------

fn player_dust(g: &mut Game, x: f32, y: f32, speed: (f32, f32), lift: (f32, f32), life: f32, alpha: f32, size: f32) {
    for side in [-1.0, 1.0] {
        g.parts.push(blip::EffectParticle {
            x,
            y,
            vx: side * blip::rand_range_f32(speed.0, speed.1),
            vy: -blip::rand_range_f32(lift.0, lift.1),
            life,
            max_life: life,
            data: ParticleStyle { c: rgba(1.0, 1.0, 1.0, alpha), size, star: false, ring: false },
        });
    }
}

fn move_players(g: &mut Game, inp: [Input; 2], dt: f32, sfx: &Sounds) {
    for i in 0..2 {
        if !g.p[i].joined {
            continue;
        }
        if g.p[i].dead_t > 0.0 {
            let p = &mut g.p[i];
            p.dead_t -= dt;
            p.vy += GRAV * 0.5 * dt;
            p.y += p.vy * dt;
            if p.dead_t <= 0.0 {
                p.lives -= 1;
                if p.lives > 0 {
                    p.place();
                } else {
                    p.alive = false;
                }
            }
            continue;
        }
        if !g.p[i].alive {
            continue;
        }
        let k = inp[i];
        let (mut x, mut y, mut vx, mut vy) = (g.p[i].x, g.p[i].y, g.p[i].vx, g.p[i].vy);
        let dir = k.right as i32 as f32 - k.left as i32 as f32;
        vx += (dir * RUN - vx) * (18.0 * dt).min(1.0);
        if dir != 0.0 {
            g.p[i].face = dir;
        }
        if k.jump && g.p[i].on_ground {
            vy = -JUMP_V;
            g.p[i].squash = 1.35;
            let (fx, fy) = (x + P_W / 2.0, y + P_H);
            player_dust(g, fx, fy, (40.0, 70.0), (5.0, 20.0), 0.25, 0.45, 1.8);
            play_sfx_volume(&sfx.jump, 0.5);
        }
        let was_air = !g.p[i].on_ground;
        let fall_speed = vy;
        let (ground, _) = g.move_body(&mut x, &mut y, &mut vx, &mut vy, P_W, P_H, dt);
        if ground && was_air && fall_speed > 150.0 {
            g.p[i].squash = 0.7;
            player_dust(g, x + P_W / 2.0, y + P_H, (30.0, 60.0), (10.0, 30.0), 0.3, 0.5, 2.0);
        }
        let p = &mut g.p[i];
        p.x = x;
        p.y = y;
        p.vx = vx;
        p.vy = vy;
        p.on_ground = ground;
        p.squash += (1.0 - p.squash) * (12.0 * dt).min(1.0);
        p.walk += vx.abs() * dt * 0.09;
        p.blow_cd -= dt;
        p.mouth = (p.mouth - dt).max(0.0);
        if p.eat >= 0.0 {
            p.eat += dt;
            if p.eat > EAT_SECS {
                p.eat = -1.0;
            }
        }
        p.safe = (p.safe - dt).max(0.0);
        p.blink -= dt;
        if p.blink < -0.12 {
            p.blink = blip::rand_range_f32(2.0, 4.5);
        }
        if k.blow && p.blow_cd <= 0.0 && g.bubbles.len() < MAX_BUBBLES {
            p.blow_cd = BLOW_CD;
            p.mouth = 0.18;
            let (bx, by, f) = (p.cx() + p.face * 14.0, p.cy() - 1.0, p.face);
            g.bubbles.push(Bubble {
                x: bx,
                y: by,
                vx: f * SHOOT_V,
                age: 0.0,
                phase: Phase::Shoot,
                owner: i,
                trapped: None,
                active: true,
                wob: blip::rand_range_f32(0.0, 6.0),
                squish: 0.0,
            });
            play_sfx_volume(&sfx.blow, 0.6);
        }
    }
}

fn nearest_player(g: &Game, x: f32, y: f32) -> Option<(f32, f32)> {
    g.p.iter()
        .filter(|p| p.joined && p.alive && p.dead_t <= 0.0)
        .map(|p| (p.cx(), p.cy()))
        .min_by(|a, b| ((a.0 - x).hypot(a.1 - y)).total_cmp(&(b.0 - x).hypot(b.1 - y)))
}

fn move_toys(g: &mut Game, dt: f32) {
    for i in 0..g.enemies.len() {
        let mut e = g.enemies[i];
        if !e.active {
            continue;
        }
        e.t += dt;
        e.pop_in = (e.pop_in + dt * 3.0).min(1.0);
        let fast = if e.angry { 1.35 } else { 1.0 } * monster_pace(g.round);
        let target = nearest_player(g, e.x + E_W / 2.0, e.y + E_H / 2.0);
        // A jump is told before it happens: the monster crouches, still,
        // for JUMP_TELL, then springs.
        if e.crouch > 0.0 {
            e.crouch -= dt;
            e.vx = 0.0;
            if e.crouch <= 0.0 {
                (e.vx, e.vy) = e.leap;
                e.on_ground = false;
            }
        }
        match e.kind {
            Kind::Walker => {
                if e.on_ground && e.crouch <= 0.0 {
                    e.vx = if e.windup > 0.0 { 0.0 } else { e.dir * 58.0 * fast };
                    e.edge_cd -= dt;
                    let ahead = if e.dir > 0.0 { e.x + E_W + 2.0 } else { e.x - 2.0 };
                    if e.edge_cd <= 0.0 && !g.floor_at(ahead, ahead + 1.0, e.y + E_H) {
                        e.edge_cd = 0.6;
                        if blip::rand_range_f32(0.0, 1.0) < 0.55 {
                            e.dir = -e.dir;
                            e.vx = -e.vx;
                        }
                    }
                    e.jump_cd -= dt;
                    if e.jump_cd <= 0.0 {
                        e.jump_cd = blip::rand_range_f32(1.0, 2.4) / fast;
                        if let Some((px, py)) = target {
                            if py < e.y - 30.0 && (px - e.x).abs() < 150.0 && blip::rand_range_f32(0.0, 1.0) < 0.7 {
                                e.crouch = JUMP_TELL;
                                e.leap = (e.dir * 58.0 * fast, -JUMP_V);
                            } else if blip::rand_range_f32(0.0, 1.0) < 0.15 {
                                e.dir = if px > e.x { 1.0 } else { -1.0 };
                            }
                        }
                    }
                }
                let (mut x, mut y, mut vx, mut vy) = (e.x, e.y, e.vx, e.vy);
                let (ground, wall) = g.move_body(&mut x, &mut y, &mut vx, &mut vy, E_W, E_H, dt);
                if wall {
                    e.dir = -e.dir;
                }
                e.x = x;
                e.y = y;
                e.vx = vx;
                e.vy = vy;
                e.on_ground = ground;
            }
            Kind::Hopper => {
                if e.on_ground && e.crouch <= 0.0 {
                    e.vx *= 0.8;
                    e.jump_cd -= dt;
                    if e.jump_cd <= 0.0 {
                        e.jump_cd = blip::rand_range_f32(0.6, 1.3) / fast;
                        let toward = target.map(|(px, _)| if px > e.x { 1.0 } else { -1.0 }).unwrap_or(e.dir);
                        e.dir = if blip::rand_range_f32(0.0, 1.0) < 0.75 { toward } else { -toward };
                        let high = target.map(|(_, py)| py < e.y - 20.0).unwrap_or(false)
                            || blip::rand_range_f32(0.0, 1.0) < 0.3;
                        e.crouch = JUMP_TELL;
                        e.leap = (e.dir * 85.0 * fast, if high { -JUMP_V * 0.98 } else { -300.0 });
                    }
                }
                let (mut x, mut y, mut vx, mut vy) = (e.x, e.y, e.vx, e.vy);
                let (ground, wall) = g.move_body(&mut x, &mut y, &mut vx, &mut vy, E_W, E_H, dt);
                if wall {
                    e.dir = -e.dir;
                    vx = -vx;
                }
                e.x = x;
                e.y = y;
                e.vx = vx;
                e.vy = vy;
                e.on_ground = ground;
            }
            Kind::Ghost => {
                let sp = 62.0 * fast;
                e.vx = e.vx.signum() * sp;
                e.vy = e.vy.signum() * sp * 0.8;
                e.x += e.vx * dt;
                e.y += e.vy * dt + (e.t * 3.0).sin() * 12.0 * dt;
                if e.x < TILE {
                    e.x = TILE;
                    e.vx = e.vx.abs();
                }
                if e.x > WIN_W as f32 - TILE - E_W {
                    e.x = WIN_W as f32 - TILE - E_W;
                    e.vx = -e.vx.abs();
                }
                if e.y < HUD + TILE {
                    e.y = HUD + TILE;
                    e.vy = e.vy.abs();
                }
                if e.y > WIN_H as f32 - TILE - E_H {
                    e.y = WIN_H as f32 - TILE - E_H;
                    e.vy = -e.vy.abs();
                }
            }
        }
        // Walkers wind up before rolling rocks at aligned players; angry ghosts aim slow sparks at the nearest player.
        e.shot_cd -= dt;
        if e.windup > 0.0 {
            e.windup -= dt;
            if e.windup <= 0.0 {
                let (ex, ey) = (e.x + E_W / 2.0, e.y + E_H / 2.0);
                g.shots.push(Shot {
                    x: ex + e.dir * 12.0,
                    y: ey + 2.0,
                    vx: e.dir * ROCK_V,
                    vy: 0.0,
                    t: 0.0,
                    spark: false,
                    active: true,
                });
            }
        } else if e.shot_cd <= 0.0 && e.pop_in >= 1.0 {
            if let Some((px, py)) = target {
                let (ex, ey) = (e.x + E_W / 2.0, e.y + E_H / 2.0);
                if e.kind == Kind::Walker
                    && g.round >= ROCKS_FROM
                    && e.on_ground
                    && (py - ey).abs() < 14.0
                    && (px - ex).abs() < 300.0
                {
                    e.dir = if px > ex { 1.0 } else { -1.0 };
                    e.windup = ROCK_WINDUP;
                    e.shot_cd = blip::rand_range_f32(2.2, 4.0) / fast;
                } else if e.kind == Kind::Ghost && g.round >= SPARKS_FROM && e.angry {
                    let (dx, dy) = (px - ex, py - ey);
                    let d = dx.hypot(dy).max(1.0);
                    g.shots.push(Shot {
                        x: ex,
                        y: ey,
                        vx: dx / d * SPARK_V,
                        vy: dy / d * SPARK_V,
                        t: 0.0,
                        spark: true,
                        active: true,
                    });
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
fn move_shots(g: &mut Game, dt: f32, player_starts: [(f32, f32); 2]) -> [bool; 2] {
    let mut player_hits = [false; 2];
    for i in 0..g.shots.len() {
        let mut s = g.shots[i];
        let from = (s.x, s.y);
        s.t += dt;
        s.x += s.vx * dt;
        s.y += s.vy * dt;
        let expired =
            (s.spark && s.t > 4.0) || s.x < -10.0 || s.x > WIN_W as f32 + 10.0 || s.y < HUD || s.y > WIN_H as f32;
        if !s.spark {
            let (c, r) = (((s.x + s.vx.signum() * 5.0) / TILE).floor() as i32, ((s.y - HUD) / TILE).floor() as i32);
            if g.platform(c, r) || c <= 0 || c >= COLS as i32 - 1 {
                s.active = false;
            }
        }
        // a bubble you blow knocks it out of the air
        if s.active
            && g.bubbles.iter().any(|b| {
                b.active
                    && b.phase == Phase::Shoot
                    && b.trapped.is_none()
                    && (b.x - s.x).hypot(b.y - s.y) < b.radius() + 5.0
            })
        {
            s.active = false;
        }
        if s.active {
            for pi in 0..2 {
                let p = &g.p[pi];
                if !p.joined || !p.alive || p.dead_t > 0.0 || p.safe > 0.0 {
                    continue;
                }
                if segment_rect_overlap(
                    from.0 - player_starts[pi].0,
                    from.1 - player_starts[pi].1,
                    s.x - p.cx(),
                    s.y - p.cy(),
                    -P_W / 2.0 - 3.0,
                    -P_H / 2.0 - 2.0,
                    P_W + 6.0,
                    P_H + 4.0,
                ) {
                    player_hits[pi] = true;
                    s.active = false;
                    break;
                }
            }
        }
        if expired {
            s.active = false;
        }
        if !s.active {
            let (x, y) = (s.x, s.y);
            g.burst(x, y, 5, rgba(1.0, 0.85, 0.6, 0.8), 70.0, s.spark);
        }
        g.shots[i] = s;
    }
    g.shots.retain(|s| s.active);
    player_hits
}

/// Pop `start` and everything touching it, and everything touching those.
/// Trapped monsters in the chain are finished, each worth double the last.
fn pop_chain(g: &mut Game, start: usize, by: usize, sfx: &Sounds) {
    let mut chain = vec![start];
    let mut k = 0;
    while k < chain.len() {
        let a = g.bubbles[chain[k]];
        for j in 0..g.bubbles.len() {
            if chain.contains(&j) || !g.bubbles[j].active {
                continue;
            }
            let b = g.bubbles[j];
            if blip::distance(a.x, a.y, b.x, b.y) < (a.radius() + b.radius()) * CHAIN_REACH {
                chain.push(j);
            }
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
        g.ring(b.x, b.y, ring, b.radius());
        if b.trapped.is_some() {
            let value = 1000 << kills.min(4);
            g.p[by].score += value;
            chain_total += value;
            if kills == 0 {
                chain_at = (b.x, b.y - 6.0);
            }
            g.burst(b.x, b.y, 10, rgba(1.0, 0.85, 0.3, 1.0), 150.0, true);
            g.fruits.push(Fruit {
                x: b.x - 8.0,
                y: b.y - 8.0,
                vx: blip::rand_range_f32(-110.0, 110.0),
                vy: -300.0,
                kind: kills.min(3),
                t: 0.0,
                on_ground: false,
                active: true,
            });
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
    if kills > 0 {
        g.stats.chains[kills.min(5)] += 1;
    }
    if kills > 0 {
        play_sfx(&sfx.kill);
    }
    if kills >= 2 {
        g.chain = (kills as i32, 1.2);
        g.shake = 0.25 + 0.08 * kills as f32;
        play_sfx(&sfx.sparkle);
    }
}

fn move_bubbles(
    g: &mut Game,
    dt: f32,
    inp: [Input; 2],
    sfx: &Sounds,
    enemy_starts: &[(f32, f32)],
    player_starts: [(f32, f32); 2],
) {
    let gather = WIN_W as f32 / 2.0;
    let bubble_starts: Vec<(f32, f32)> = g.bubbles.iter().map(|b| (b.x, b.y)).collect();
    for i in 0..g.bubbles.len() {
        let mut b = g.bubbles[i];
        if !b.active {
            continue;
        }
        b.age += dt;
        b.wob += dt;
        b.squish = (b.squish - dt * 4.0).max(0.0);
        let was_free = b.trapped.is_none();
        match b.phase {
            Phase::Shoot => {
                let from = (b.x, b.y);
                b.x += b.vx * dt;
                b.vx *= 1.0 - 2.5 * dt;
                if b.x < TILE + BUB_R {
                    b.x = TILE + BUB_R;
                    b.phase = Phase::Float;
                }
                if b.x > WIN_W as f32 - TILE - BUB_R {
                    b.x = WIN_W as f32 - TILE - BUB_R;
                    b.phase = Phase::Float;
                }
                if b.age > SHOOT_T {
                    b.phase = Phase::Float;
                }
                // trap a monster it meets while still flying
                if b.trapped.is_none() {
                    for (ei, e) in g.enemies.iter_mut().enumerate() {
                        if !e.active || e.pop_in < 1.0 {
                            continue;
                        }
                        let (ex, ey) = (e.x + E_W / 2.0, e.y + E_H / 2.0);
                        let enemy_from = enemy_starts.get(ei).copied().unwrap_or((ex, ey));
                        let (rx, ry) = (BUB_R + E_W / 2.0 - 2.0, BUB_R + E_H / 2.0 - 2.0);
                        if segment_rect_overlap(
                            from.0 - enemy_from.0,
                            from.1 - enemy_from.1,
                            b.x - ex,
                            b.y - ey,
                            -rx,
                            -ry,
                            rx * 2.0,
                            ry * 2.0,
                        ) {
                            e.active = false;
                            b.trapped = Some((e.kind, e.angry));
                            b.age = 0.0;
                            b.phase = Phase::Float;
                            b.x = ex;
                            b.y = ey;
                            play_sfx(&sfx.trap);
                            break;
                        }
                    }
                }
            }
            Phase::Float => {
                b.y -= if b.trapped.is_some() { RISE_TRAPPED } else { RISE } * dt;
                b.x += (b.wob * 2.2).sin() * 10.0 * dt;
                if b.y <= b.top_y() {
                    b.y = b.top_y();
                    b.phase = Phase::Top;
                }
            }
            Phase::Top => {
                let d = gather - b.x;
                b.x += d.signum() * d.abs().min(38.0) * dt + (b.wob * 1.7).sin() * 6.0 * dt;
                b.y = b.top_y() + (b.wob * 2.0).sin() * 1.5;
            }
        }
        // inside the walls, at whatever size it now is
        let r = b.radius();
        b.x = b.x.clamp(TILE + r, WIN_W as f32 - TILE - r);
        if was_free && b.trapped.is_some() {
            g.stats.traps += 1;
        }
        let life = if b.trapped.is_some() { trap_life(g.round) } else { FREE_LIFE };
        if b.age > life && b.phase != Phase::Shoot {
            if b.trapped.is_some() {
                g.stats.escapes += 1;
            }
            b.active = false;
            if let Some((kind, _)) = b.trapped {
                // out it comes, and it is not happy about it
                g.enemies.push(Enemy {
                    kind,
                    x: b.x - E_W / 2.0,
                    y: b.y - E_H / 2.0,
                    vx: 0.0,
                    vy: 0.0,
                    dir: if blip::rand_range_f32(0.0, 1.0) < 0.5 { -1.0 } else { 1.0 },
                    on_ground: false,
                    angry: true,
                    t: 0.0,
                    jump_cd: 0.5,
                    edge_cd: 0.0,
                    active: true,
                    pop_in: 1.0,
                    shot_cd: blip::rand_range_f32(1.5, 3.0),
                    windup: 0.0,
                    crouch: 0.0,
                    leap: (0.0, 0.0),
                });
                let last = g.enemies.len() - 1;
                if kind == Kind::Ghost {
                    g.enemies[last].vx = 62.0;
                    g.enemies[last].vy = 62.0;
                }
            }
            g.burst(b.x, b.y, 6, rgba(1.0, 1.0, 1.0, 0.6), 60.0, false);
        }
        g.bubbles[i] = b;
    }
    // bubbles along the ceiling jostle into a cluster instead of stacking up
    for i in 0..g.bubbles.len() {
        for j in (i + 1)..g.bubbles.len() {
            let (a, b) = (g.bubbles[i], g.bubbles[j]);
            if !a.active || !b.active || a.phase == Phase::Shoot || b.phase == Phase::Shoot {
                continue;
            }
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let d = dx.hypot(dy).max(0.1);
            let min = a.radius() + b.radius() - 1.0;
            if d < min {
                let push = (min - d) * 0.5;
                g.bubbles[i].x -= dx / d * push;
                g.bubbles[j].x += dx / d * push;
                if a.phase != Phase::Top || b.phase != Phase::Top {
                    g.bubbles[i].y -= dy / d * push * 0.5;
                    g.bubbles[j].y += dy / d * push * 0.5;
                }
            }
        }
    }
    // players against bubbles: hold jump to ride one, otherwise it pops
    for pi in 0..2 {
        let p = &g.p[pi];
        if !p.joined || !p.alive || p.dead_t > 0.0 {
            continue;
        }
        let (px, py, vy) = (p.cx(), p.cy(), p.vy);
        for i in 0..g.bubbles.len() {
            let b = g.bubbles[i];
            if !b.active {
                continue;
            }
            let bubble_from = bubble_starts.get(i).copied().unwrap_or((b.x, b.y));
            if !segment_circle_overlap(
                player_starts[pi].0 - bubble_from.0,
                player_starts[pi].1 - bubble_from.1,
                px - b.x,
                py - b.y,
                0.0,
                0.0,
                b.radius() + 11.0,
            ) {
                continue;
            }
            if vy > 40.0 && player_starts[pi].1 < bubble_from.1 - 6.0 && inp[pi].jump_held {
                g.p[pi].vy = -JUMP_V * 0.82;
                g.p[pi].squash = 1.3;
                g.bubbles[i].squish = 1.0;
                g.stats.bounces += 1;
                play_sfx_volume(&sfx.bounce, 0.7);
                break;
            }
            if b.owner == pi && b.age < 0.25 && b.trapped.is_none() {
                continue;
            } // your own, just blown
            pop_chain(g, i, pi, sfx);
            break;
        }
    }
    g.bubbles.retain(|b| b.active);
}

fn move_candies(g: &mut Game, dt: f32, sfx: &Sounds, player_starts: [(f32, f32); 2]) {
    const VALUE: [i32; 4] = [500, 1000, 2000, 4000];
    for i in 0..g.fruits.len() {
        let mut f = g.fruits[i];
        if !f.active {
            continue;
        }
        let fruit_from = (f.x + 8.0, f.y + 8.0);
        f.t += dt;
        if !f.on_ground {
            let (mut x, mut y, mut vx, mut vy) = (f.x, f.y, f.vx, f.vy);
            let (ground, _) = g.move_body(&mut x, &mut y, &mut vx, &mut vy, 16.0, 16.0, dt);
            f.x = x;
            f.y = y;
            f.vx = vx * (1.0 - 0.8 * dt);
            f.vy = vy;
            if ground {
                f.on_ground = true;
                f.vx = 0.0;
            }
        } else if !g.floor_at(f.x, f.x + 16.0, f.y + 16.0) {
            f.on_ground = false;
        }
        for pi in 0..2 {
            let p = &g.p[pi];
            if !p.joined || !p.alive || p.dead_t > 0.0 || f.t < 0.35 {
                continue;
            }
            if segment_rect_overlap(
                player_starts[pi].0 - fruit_from.0,
                player_starts[pi].1 - fruit_from.1,
                p.cx() - (f.x + 8.0),
                p.cy() - (f.y + 8.0),
                -16.0,
                -18.0,
                32.0,
                36.0,
            ) {
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
        if f.t > 9.0 {
            f.active = false;
        }
        g.fruits[i] = f;
    }
    g.fruits.retain(|f| f.active);
}

/// Crumbs fly from a chewing dragon's mouth on each chomp.
fn update_chewing(g: &mut Game, dt: f32) {
    for pi in 0..2 {
        let p = &g.p[pi];
        if p.eat < 0.0 {
            continue;
        }
        if !CHOMPS.iter().any(|&c| p.eat - dt < c && p.eat >= c) {
            continue;
        }
        let (mx, my) = mouth_pos(p);
        let c = col(CANDY[candy_pick(&p.eaten)], 1.0);
        g.burst(mx, my, 4, c, 45.0, false);
    }
}

fn update_skull(g: &mut Game, dt: f32) -> Option<((f32, f32), (f32, f32))> {
    if !g.skull.active {
        if g.round_t >= skull_at(g.round) {
            g.skull = Skull { active: true, x: WIN_W as f32 / 2.0, y: HUD + 10.0, speed: 45.0 };
            g.stats.skulls += 1;
        }
        return None;
    }
    let from = (g.skull.x, g.skull.y);
    g.skull.speed += (4.0 + 1.5 * g.round as f32) * dt;
    if let Some((px, py)) = nearest_player(g, g.skull.x, g.skull.y) {
        let (dx, dy) = (px - g.skull.x, py - g.skull.y);
        let d = dx.hypot(dy).max(1.0);
        g.skull.x += dx / d * g.skull.speed * dt;
        g.skull.y += dy / d * g.skull.speed * dt;
        return Some((from, (g.skull.x, g.skull.y)));
    }
    None
}

fn shot_hits_player(s: &Shot, x: f32, y: f32) -> bool {
    (s.x - x).abs() < P_W / 2.0 + 3.0 && (s.y - y).abs() < P_H / 2.0 + 2.0
}

fn hurt_players(
    g: &mut Game,
    sfx: &Sounds,
    skull_motion: Option<((f32, f32), (f32, f32))>,
    shot_hits: [bool; 2],
    player_starts: [(f32, f32); 2],
    enemy_starts: &[(f32, f32)],
) {
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("BUBBLER_GOD").is_some() {
        if g.skull.active && g.p.iter().any(|p| p.alive && (g.skull.x - p.cx()).hypot(g.skull.y - p.cy()) < 18.0) {
            g.stats.deaths_skull += 1; // would have been caught
        }
        return;
    }
    for pi in 0..2 {
        let p = &g.p[pi];
        if !p.joined || !p.alive || p.dead_t > 0.0 || p.safe > 0.0 {
            continue;
        }
        let (px, py) = (p.cx(), p.cy());
        let killer = g
            .enemies
            .iter()
            .enumerate()
            .find(|(ei, e)| {
                if !e.active || e.pop_in < 1.0 {
                    return false;
                }
                let end = (e.x + E_W / 2.0, e.y + E_H / 2.0);
                let start = enemy_starts.get(*ei).copied().unwrap_or(end);
                segment_rect_overlap(
                    player_starts[pi].0 - start.0,
                    player_starts[pi].1 - start.1,
                    px - end.0,
                    py - end.1,
                    -((E_W + P_W) / 2.0 - 7.0),
                    -((E_H + P_H) / 2.0 - 10.0),
                    2.0 * ((E_W + P_W) / 2.0 - 7.0),
                    2.0 * ((E_H + P_H) / 2.0 - 10.0),
                )
            })
            .map(|(_, e)| e.kind);
        let hit_skull = g.skull.active
            && skull_motion.map_or_else(
                || (g.skull.x - px).hypot(g.skull.y - py) < 18.0,
                |(from, to)| {
                    segment_circle_overlap(
                        from.0 - player_starts[pi].0,
                        from.1 - player_starts[pi].1,
                        to.0 - px,
                        to.1 - py,
                        0.0,
                        0.0,
                        18.0,
                    )
                },
            );
        let hit_shot = shot_hits[pi] || g.shots.iter().any(|s| shot_hits_player(s, px, py));
        if hit_shot {
            g.shots.retain(|s| !shot_hits_player(s, px, py));
        }
        let hit_enemy = killer.is_some() || hit_shot;
        if hit_enemy || hit_skull {
            if hit_skull {
                g.stats.deaths_skull += 1;
            } else {
                g.stats.deaths_enemy += 1;
            }
            if let Some(k) = killer {
                g.stats.deaths_by[k as usize] += 1;
            }
            #[cfg(not(target_arch = "wasm32"))]
            if std::env::var_os("BUBBLER_BOT").is_some() {
                let p = &g.p[pi];
                let e = g
                    .enemies
                    .iter()
                    .filter(|e| e.active)
                    .min_by(|a, b| (a.x - p.x).hypot(a.y - p.y).total_cmp(&(b.x - p.x).hypot(b.y - p.y)));
                println!(
                    "DEATH round={} t={:.1} p{} at ({:.0},{:.0}) v=({:.0},{:.0}) ground={} face={} | {:?}",
                    g.round + 1,
                    g.round_t,
                    pi + 1,
                    p.x,
                    p.y,
                    p.vx,
                    p.vy,
                    p.on_ground,
                    p.face,
                    e.map(|e| (e.kind, e.x as i32, e.y as i32, e.vx as i32, e.vy as i32, e.angry))
                );
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
    for p in g.pops.iter_mut() {
        p.t += dt;
        p.y = (p.y - 30.0 * dt).max(HUD + 6.0);
    }
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
        for e in g.enemies.iter_mut() {
            e.angry = true;
        }
        play_sfx(&sfx.hurry);
    }
    let player_starts = std::array::from_fn(|i| (g.p[i].cx(), g.p[i].cy()));
    let enemy_starts: Vec<(f32, f32)> = g.enemies.iter().map(|e| (e.x + E_W / 2.0, e.y + E_H / 2.0)).collect();
    move_players(g, inp, dt, sfx);
    move_toys(g, dt);
    move_bubbles(g, dt, inp, sfx, &enemy_starts, player_starts);
    move_candies(g, dt, sfx, player_starts);
    let skull_motion = update_skull(g, dt);
    update_chewing(g, dt);
    let shot_hits = move_shots(g, dt, player_starts);
    hurt_players(g, sfx, skull_motion, shot_hits, player_starts, &enemy_starts);
    g.enemies.retain(|e| e.active);
    if !players_left(g) {
        g.state = State::Over;
        g.state_t = 0.0;
        report_score(g);
        play_sfx(&sfx.over);
        return;
    }
    let trapped = g.bubbles.iter().any(|b| b.trapped.is_some());
    let death_pending = g.p.iter().any(|p| p.joined && p.dead_t > 0.0);
    if g.enemies.is_empty() && !trapped && !death_pending {
        g.state = State::Clear;
        g.state_t = 0.0;
        g.skull.active = false;
        g.shots.clear();
        // EXTEND: every player still in gets a life back, up to EXTEND_MAX
        g.extend = false;
        for p in g.p.iter_mut() {
            if p.joined && p.lives > 0 && p.lives < EXTEND_MAX {
                p.lives += 1;
                g.extend = true;
            }
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
    draw_bokeh(blip, pal);
}

fn draw_bokeh(blip: &Blip, pal: &levels::Palette) {
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

fn draw_tiles(blip: &Blip, art: &art::Art, g: &Game, ox: f32, oy: f32) {
    for r in 0..ROWS {
        for c in 0..COLS {
            if !(g.tiles[r][c] || c == 0 || c == COLS - 1) {
                continue;
            }
            let border = c == 0 || c == COLS - 1 || r == 0 || r == ROWS - 1;
            if (r == 0 || r == ROWS - 1) && !g.tiles[r][c] {
                continue;
            }
            let (x, y) = (c as f32 * TILE + ox, HUD + r as f32 * TILE + oy);
            art.tile(blip, g.round, border, x, y, TILE);
        }
    }
}

// ---- plush toys ------------------------------------------------------------

const EAT_SECS: f32 = 0.6;
const EAT_FLY: f32 = 0.15;
const CHOMPS: [f32; 2] = [0.28, 0.45];

fn mouth_pos(p: &Player) -> (f32, f32) {
    (p.cx() + p.face * 10.5 * TOY_SCALE, p.y + P_H - 9.0 * TOY_SCALE)
}

fn draw_bubble(blip: &Blip, art: &art::Art, b: &Bubble, ox: f32, oy: f32, t: f32, round: usize) {
    let (x, y) = (b.x + ox, b.y + oy);
    let life = if b.trapped.is_some() { trap_life(round) } else { FREE_LIFE };
    let escaping = b.trapped.is_some() && b.age > life - 1.6;
    let shake = if escaping { (t * 50.0).sin() * 1.5 } else { 0.0 };
    let grow = if b.phase == Phase::Shoot && b.trapped.is_none() {
        0.35 + 0.65 * blip::ease_out_cubic(b.age / 0.12)
    } else {
        1.0
    };
    let r = (b.radius() + (b.wob * 3.0).sin() * 0.6) * grow;
    let (rx, ry) = (r * (1.0 + b.squish * 0.25), r * (1.0 - b.squish * 0.25));
    if let Some((kind, angry)) = b.trapped {
        // the whole toy inside: it reaches 18 px from its middle at full size
        art.toy_in_bubble(blip, kind, angry, x + shake, y + (b.wob * 5.0).sin() * 0.6, (TRAP_R - 2.0) / 18.0);
    }
    let rim = if escaping && (t * 10.0) as i32 % 2 == 0 {
        rgba(1.0, 0.3, 0.3, 0.95)
    } else {
        hsv(b.wob * 0.15 + x * 0.002, 0.45, 1.0, 0.85)
    };
    let body = if b.trapped.is_some() { 0.16 } else { 0.22 };
    draw_glass(x + shake, y, rx, ry, b.owner, body, rim);
}

/// A bubble's glass: tinted with its owner's colour, a rim, and the window's shine.
fn draw_glass(x: f32, y: f32, rx: f32, ry: f32, owner: usize, body: f32, rim: BlipColor) {
    let tint = if owner == 0 { (120, 255, 140) } else { (120, 200, 255) };
    draw_ellipse(x, y, rx, ry, 0.0, col(tint, body));
    blip::macroquad::shapes::draw_ellipse_lines(x, y, rx, ry, 0.0, 1.8, rim);
    let r = (rx + ry) / 2.0;
    draw_ellipse(x - r * 0.38, y - r * 0.42, r * 0.3, r * 0.17, -35.0, rgba(1.0, 1.0, 1.0, 0.85));
    blip::macroquad::shapes::draw_circle(x + r * 0.45, y + r * 0.35, 1.2, rgba(1.0, 1.0, 1.0, 0.6));
}

/// Candy colours, picked per drop so a shower of them is a pick-and-mix.
const CANDY: [(u8, u8, u8); 6] =
    [(255, 120, 160), (120, 200, 255), (255, 200, 80), (150, 230, 140), (200, 150, 255), (255, 150, 90)];

/// A dropped treat, sweeter the bigger the chain that made it (see VALUE):
/// a wrapped bonbon, a swirled lollipop, a sprinkled donut, a cupcake with
/// a cherry on top. Each shines, and twinkles now and then.
fn draw_fruit(f: &Fruit, ox: f32, oy: f32) {
    if f.t > 7.0 && (f.t * 10.0) as i32 % 2 == 0 {
        return;
    }
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
        0 => {
            // a wrapped bonbon: a round sweet, twisted cellophane ends
            for s in [-1.0f32, 1.0] {
                draw_triangle(
                    vec2(x + s * 6.0, y),
                    vec2(x + s * 12.0, y - 5.0),
                    vec2(x + s * 12.0, y + 5.0),
                    col(tint(c, 1.15), 1.0),
                );
                draw_line(x + s * 9.0, y - 3.0, x + s * 9.0, y + 3.0, 1.0, col(tint(c, 0.75), 1.0));
            }
            draw_circle(x, y, 7.5, col(tint(c, 0.8), 1.0));
            draw_circle(x, y, 6.6, col(c, 1.0));
            for k in 0..3 {
                // candy stripes
                let dx = -4.0 + k as f32 * 4.0;
                draw_line(x + dx - 1.5, y - 5.0, x + dx + 1.5, y + 5.0, 1.6, col((255, 255, 255), 0.55));
            }
            draw_ellipse(x - 2.6, y - 3.0, 2.4, 1.4, -30.0, shine);
        }
        1 => {
            // a lollipop: a spiral of two colours on a stick
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
        2 => {
            // a donut: soft dough, glossy icing, sprinkles
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
        _ => {
            // a cupcake: a pleated case, swirled frosting, a cherry
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
    for k in 0..3 {
        draw_circle(x, y, 16.0 + k as f32 * 5.0, rgba(0.6, 0.2, 0.9, 0.12));
    }
    draw_ellipse(x, y, 12.0, 11.0, 0.0, rgba(0.95, 0.95, 1.0, 1.0));
    draw_ellipse(x, y + 8.0, 7.0, 4.0, 0.0, rgba(0.95, 0.95, 1.0, 1.0));
    for sgn in [-1.0f32, 1.0] {
        draw_ellipse(x + sgn * 4.5, y - 1.0, 3.5, 4.2, 0.0, rgba(0.15, 0.0, 0.25, 1.0));
        draw_circle(x + sgn * 4.5, y - 1.0, 1.4, hsv(t * 0.5, 0.7, 1.0, 1.0));
    }
    for k in -1..=1 {
        blip::macroquad::shapes::draw_rectangle(x + k as f32 * 3.0 - 1.0, y + 7.0, 2.0, 4.0, rgba(0.2, 0.1, 0.3, 1.0));
    }
}

fn draw_world(blip: &Blip, art: &art::Art, g: &Game) {
    let t = now();
    let (ox, oy) =
        if g.shake > 0.0 { ((t * 90.0).sin() * g.shake * 8.0, (t * 77.0).cos() * g.shake * 8.0) } else { (0.0, 0.0) };
    draw_background(blip, g);
    draw_tiles(blip, art, g, ox, oy);
    for f in &g.fruits {
        draw_fruit(f, ox, oy);
    }
    for e in &g.enemies {
        if !e.active {
            continue;
        }
        let s = blip::ease_out_cubic(e.pop_in);
        let shake = if e.windup > 0.0 || e.crouch > 0.0 { (e.t * 70.0).sin() * 1.4 } else { 0.0 };
        // crouching to jump: drawn smaller, feet still on the platform
        let k = if e.crouch > 0.0 { 0.82 } else { 1.0 };
        art.enemy(
            blip,
            e.kind,
            e.x + ox + shake,
            e.y + oy,
            s.max(0.05) * TOY_SCALE * k,
            s,
            e.angry,
            // the way it is going; a ghost drifting nearly still keeps its heading
            if e.vx.abs() > 1.0 { e.vx.signum() } else { e.dir },
        );
    }
    for (i, p) in g.p.iter().enumerate() {
        if !p.joined {
            continue;
        }
        if !p.alive && p.dead_t <= 0.0 {
            continue;
        }
        if p.safe > 0.0 && (p.safe * 12.0) as i32 % 2 == 0 && p.dead_t <= 0.0 {
            continue;
        }
        art.player(blip, p.cx() + ox, p.y + P_H + oy, i, TOY_SCALE, p.face);
        // the candy flying into the open mouth
        if p.eat >= 0.0 && p.eat < EAT_FLY {
            let (mx, my) = mouth_pos(p);
            let k = blip::ease_out_cubic(p.eat / EAT_FLY);
            let (fx, fy) = (p.eaten.x + 8.0, p.eaten.y + 8.0);
            draw_candy(
                p.eaten.kind,
                candy_pick(&p.eaten),
                1.0,
                fx + (mx - fx) * k + ox,
                fy + (my - fy) * k + oy,
                false,
            );
        }
    }
    for b in &g.bubbles {
        draw_bubble(blip, art, b, ox, oy, t, g.round);
    }
    if g.skull.active {
        draw_skull(&g.skull, ox, oy, t);
    }
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

fn text_w(text: &str, sz: f32) -> f32 {
    text.chars().count() as f32 * 6.0 * sz - sz
}

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
        for (px, py) in
            [(xx + rr, yy + rr), (xx + ww - rr, yy + rr), (xx + rr, yy + hh - rr), (xx + ww - rr, yy + hh - rr)]
        {
            draw_circle(px, py, rr, c);
        }
    }
    draw_rectangle(x + r, y + 2.0, w - r * 2.0, 1.5, rgba(1.0, 1.0, 1.0, 0.08 * a));
}

/// Text on its own pill, centred on the screen, popping in with `pop`.
fn cosy(blip: &Blip, text: &str, y: f32, sz: f32, c: (u8, u8, u8), edge: (u8, u8, u8), pop: f32, a: f32) {
    if a <= 0.01 {
        return;
    }
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

fn draw_scores(blip: &Blip, g: &Game, _hi: &web::HighScore) {
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

fn draw_title(blip: &Blip, art: &art::Art, g: &Game, hi: &web::HighScore) {
    let t = now();
    art.title_background(blip);
    draw_bokeh(blip, &PALETTES[0]);
    // bubbles drifting up behind the logo
    for i in 0..14 {
        let s = i as f32 * 53.1;
        let x = (s * 11.3) % WIN_W as f32;
        let y = WIN_H as f32 - ((s * 9.1 + t * 30.0) % (WIN_H as f32 + 40.0)) + 20.0;
        // the sheen drifts round the colours as each bubble wobbles
        let (wob, r) = (t + s, 12.0 + ((t + s) * 3.0).sin() * 0.6);
        draw_glass(x, y, r, r, i % 2, 0.22, hsv(wob * 0.15 + x * 0.002, 0.45, 1.0, 0.85));
    }
    for letter in 0..7 {
        let bob = (t * 3.0 + letter as f32 * 0.6).sin() * 5.0;
        art.title_letter(blip, letter, 186.0 + letter as f32 * 36.0, 88.0 + DY + bob);
    }
    // the two of them, bouncing
    for (i, x) in [WIN_W as f32 / 2.0 - 70.0, WIN_W as f32 / 2.0 + 70.0].iter().enumerate() {
        let hop = ((t * 3.2 + i as f32 * 1.6).sin()).max(0.0);
        let foot = 278.0 + DY - hop * 34.0;
        // the two of them face each other
        art.player(blip, *x, foot, i, 2.4, if i == 0 { 1.0 } else { -1.0 });
    }
    let glow = 0.65 + 0.35 * (t * 2.5).sin().abs();
    art.title_panels(blip, web::controls(), glow);
    if hi.score > 0 {
        let h = hi.label("HI");
        soft(blip, &h, (WIN_W as f32 - text_w(&h, 2.0)) / 2.0, 428.0 + DY, 2.0, PEACH, 1.0);
    }
    let _ = g;
}

/// Native playtest bot (`BUBBLER_BOT=1` or `2`); targets nearby enemies and avoids hazards.
#[cfg(not(target_arch = "wasm32"))]
fn bot_input(g: &Game, pi: usize, mem: &mut [f32; 4]) -> Input {
    let mut k = Input::default();
    let passive = std::env::var_os("BUBBLER_PASSIVE").is_some();
    let p = &g.p[pi];
    if !p.joined || !p.alive || p.dead_t > 0.0 {
        return k;
    }
    let (px, py) = (p.cx(), p.cy());
    mem[0] -= 1.0 / 60.0;
    // danger: a monster about to walk into us
    let danger = g
        .enemies
        .iter()
        .filter(|e| e.active)
        .map(|e| (e.x + E_W / 2.0 - px, e.y + E_H / 2.0 - py))
        .find(|(dx, dy)| dx.abs() < 34.0 && dy.abs() < 22.0);
    // trapped bubbles are the priority: go and pop them
    let trapped =
        g.bubbles.iter().filter(|b| b.trapped.is_some()).min_by(|a, b| (a.x - px).abs().total_cmp(&(b.x - px).abs()));
    let fruit = g
        .fruits
        .iter()
        .filter(|f| f.on_ground && (f.y + 8.0 - py).abs() < 30.0)
        .min_by(|a, b| (a.x - px).abs().total_cmp(&(b.x - px).abs()));
    let target = if let Some(b) = trapped {
        Some((b.x, b.y, true))
    } else if let Some(f) = fruit.filter(|f| (f.x - px).abs() < 160.0) {
        Some((f.x + 8.0, py, true))
    } else {
        g.enemies
            .iter()
            .filter(|e| e.active && e.pop_in >= 1.0)
            .min_by(|a, b| ((a.x - px).hypot((a.y - py) * 1.6)).total_cmp(&(b.x - px).hypot((b.y - py) * 1.6)))
            .map(|e| (e.x + E_W / 2.0, e.y + E_H / 2.0, false))
    };
    // in the air over a monster: steer clear of it
    if !p.on_ground {
        if let Some(e) = g
            .enemies
            .iter()
            .filter(|e| e.active)
            .find(|e| (e.x + E_W / 2.0 - px).abs() < 30.0 && e.y > py && e.y - py < 90.0)
        {
            let away = if e.x + E_W / 2.0 > px { -1.0 } else { 1.0 };
            k.left = away < 0.0;
            k.right = away > 0.0;
            return k;
        }
    }
    if let Some((dx, _)) = danger {
        // hop away / over
        k.left = dx > 0.0;
        k.right = dx < 0.0;
        if p.on_ground && blip::rand_range_f32(0.0, 1.0) < 0.25 {
            k.jump = true;
        }
        // and bubble it if we face it
        if (dx > 0.0) == (p.face > 0.0) && !passive {
            k.blow = true;
        }
        k.jump_held = true;
        return k;
    }
    if let Some((tx, ty, is_bubble)) = target {
        let dx = tx - px;
        let dy = ty - py;
        if is_bubble {
            if dx.abs() > 10.0 {
                k.left = dx < 0.0;
                k.right = dx > 0.0;
            }
            if p.on_ground && (dy < -20.0 || dx.abs() < 40.0) && mem[0] <= 0.0 {
                k.jump = true;
                mem[0] = 0.3;
            }
            k.jump_held = false; // bump into it, do not ride it
        } else {
            let level = dy.abs() < 18.0;
            if level && dx.abs() < 150.0 {
                // face it and blow
                if (dx > 0.0) != (p.face > 0.0) {
                    k.left = dx < 0.0;
                    k.right = dx > 0.0;
                } else if blip::rand_range_f32(0.0, 1.0) < 0.5 && !passive {
                    k.blow = true;
                }
                if dx.abs() > 70.0 {
                    k.left = dx < 0.0;
                    k.right = dx > 0.0;
                }
            } else {
                if dx.abs() > 30.0 || dy > 0.0 {
                    // head for it; to go down walk off an edge, to go up jump
                    let wander = if mem[1] > 0.0 { mem[2] } else { 0.0 };
                    let dir = if wander != 0.0 { wander } else { dx.signum() };
                    k.left = dir < 0.0;
                    k.right = dir > 0.0;
                }
                if dy < -30.0 && p.on_ground && mem[0] <= 0.0 && blip::rand_range_f32(0.0, 1.0) < 0.12 {
                    k.jump = true;
                    mem[0] = 0.4;
                }
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

fn conf() -> blip::macroquad::window::Conf {
    window_conf("BUBBLER", WIN_W, WIN_H)
}

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let art = art::Art::load();
    // Bright filled blocks twitter under the full interlace (see Brawler).
    blip.set_interlace(0.5);
    let mut g = Game::new();
    web::set_players(2);

    let load = |b: &'static [u8]| async move { blip::audio::load_sound(b).await };
    let sfx = Sounds {
        blow: load(blip::wav!("blow")).await,
        pop: load(blip::wav!("pop")).await,
        trap: load(blip::wav!("trap")).await,
        kill: load(blip::wav!("defeated")).await,
        fruit: load(blip::wav!("fruit")).await,
        jump: load(blip::wav!("jump")).await,
        lose: load(blip::wav!("lost")).await,
        bounce: load(blip::wav!("bounce")).await,
        sparkle: load(blip::wav!("sparkle")).await,
        round: load(blip::wav!("jingle_round")).await,
        clear: load(blip::wav!("jingle_clear")).await,
        hurry: load(blip::wav!("jingle_hurry")).await,
        over: load(blip::wav!("jingle_over")).await,
        won: load(blip::wav!("jingle_won")).await,
    };
    // The theme, and the same tune faster for HURRY UP.
    let mut music = Jukebox::new("BUBBLER", &["THEME", "HURRY"]);
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
        {
            bot_clock += blip.delta_time;
        }
        let dt = blip.delta_time;
        g.state_t += dt;
        let hi = web::high_score();

        // The cabinet card: two dragons mid-round, bubbles up, a chain ready.
        if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 {
                g.start_game(true);
                g.state = State::Play;
                g.p[0].x = 150.0;
                g.p[0].y = HUD + 19.0 * TILE - P_H;
                g.p[0].mouth = 10.0;
                g.p[0].safe = 0.0;
                g.p[1].x = 360.0;
                g.p[1].y = HUD + 15.0 * TILE - P_H;
                g.p[1].face = -1.0;
                g.p[1].safe = 0.0;
                for (k, e) in g.enemies.iter_mut().enumerate() {
                    if k < 3 {
                        e.active = false;
                    }
                    e.pop_in = 1.0;
                }
                for (k, (x, y)) in
                    [(282.0, 70.0), (306.0, 72.0), (330.0, 70.0), (200.0, 260.0), (230.0, 390.0)].iter().enumerate()
                {
                    g.bubbles.push(Bubble {
                        x: *x,
                        y: *y,
                        vx: 0.0,
                        age: 1.0,
                        phase: if k < 3 { Phase::Top } else { Phase::Float },
                        owner: k % 2,
                        trapped: if k < 3 { Some((Kind::Walker, false)) } else { None },
                        active: true,
                        wob: k as f32,
                        squish: 0.0,
                    });
                }
                // one of each treat on the floor
                for (kind, x) in [(0usize, 150.0f32), (1, 196.0), (2, 410.0), (3, 456.0)] {
                    g.fruits.push(Fruit {
                        x,
                        y: HUD + 23.0 * TILE - 16.0,
                        vx: 0.0,
                        vy: 0.0,
                        kind,
                        t: 1.0,
                        on_ground: true,
                        active: true,
                    });
                }
            }
        }

        // Native-only: BUBBLER_SCENE=intro|play|hurry|chain|clear|over|won
        // jumps to that screen and freezes it, for checking the text.
        #[cfg(not(target_arch = "wasm32"))]
        let frozen = if let Ok(scene) = std::env::var("BUBBLER_SCENE") {
            if !scene_set {
                scene_set = true;
                if scene != "title" {
                    g.start_game(true);
                    g.state = State::Play;
                }
                for e in g.enemies.iter_mut() {
                    e.pop_in = 1.0;
                }
                for p in g.p.iter_mut() {
                    p.safe = 0.0;
                    p.score = 128_450;
                }
                g.p[1].score = 97_320;
                match scene.as_str() {
                    "intro" => {
                        g.state = State::Intro;
                        g.state_t = 1.2;
                    }
                    "hurry" => {
                        g.round_t = hurry_at(g.round) + 0.1;
                        g.hurry = true;
                    }
                    "chain" => {
                        g.chain = (3, 1.0);
                    }
                    "solo" => {
                        g.p[1].joined = false;
                        g.two_up = false;
                    }
                    "clear" => {
                        g.state = State::Clear;
                        g.state_t = 0.8;
                    }
                    "over" => {
                        g.state = State::Over;
                        g.state_t = 3.0;
                    }
                    "won" => {
                        g.state = State::Won;
                        g.state_t = 3.0;
                    }
                    _ => {}
                }
            }
            true
        } else {
            false
        };
        #[cfg(target_arch = "wasm32")]
        let frozen = false;
        let solo = !g.two_up;
        #[allow(unused_mut)]
        let mut inp = [read(if solo { &SOLO } else { &P1_KEYS }), read(&P2_KEYS)];
        #[allow(unused_mut)]
        let mut p2_start = key_pressed(BLIP_KEY_J) || key_pressed(BLIP_KEY_K);
        #[cfg(not(target_arch = "wasm32"))]
        if bot > 0
            && std::env::var("BUBBLER_MAXT").ok().and_then(|v| v.parse::<f32>().ok()).is_some_and(|m| bot_clock > m)
        {
            g.state = State::Over;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if bot > 0 {
            match g.state {
                State::Title => {
                    if bot == 2 {
                        p2_start = true;
                    } else {
                        inp[0].blow = true;
                    }
                }
                State::Over | State::Won => {
                    let st = &g.stats;
                    println!("RESULT state={} round={} time={:.0}s score={}+{} traps={} escapes={} pops={} kills={} chains={:?} fruit={} deaths_enemy={} deaths_skull={} hurries={} skulls={} bounces={} by_kind(w,h,g)={:?} by_round={:?} rounds={:?}",
                            if g.state == State::Won { "won" } else { "over" }, g.round + 1, bot_clock, g.p[0].score, g.p[1].score,
                            st.traps, st.escapes, st.pops, st.kills, st.chains, st.fruit, st.deaths_enemy, st.deaths_skull,
                            st.hurries, st.skulls, st.bounces, st.deaths_by, st.deaths_round, bot_rounds);
                    std::process::exit(0);
                }
                _ => {
                    inp = [bot_input(&g, 0, &mut bot_mem[0]), bot_input(&g, 1, &mut bot_mem[1])];
                }
            }
            if g.state == State::Clear && g.state_t < dt * 1.5 {
                bot_rounds.push(g.round_t as u32);
            }
        }

        if !frozen {
            match g.state {
                State::Title => {
                    // A game is paid for as it starts: one coin, two for two players.
                    if p2_start {
                        g.start_game(true);
                        web::spend_coin();
                        web::spend_coin();
                    } else if inp[0].blow || inp[0].jump || key_pressed(BLIP_KEY_SPACE) {
                        g.start_game(false);
                        web::spend_coin();
                    }
                }
                State::Intro => {
                    if g.state_t < dt * 1.5 {
                        play_sfx(&sfx.round);
                    }
                    for e in g.enemies.iter_mut() {
                        e.pop_in = (g.state_t / 1.2).min(0.999);
                    }
                    if g.state_t > 1.8 {
                        g.state = State::Play;
                        g.state_t = 0.0;
                        for e in g.enemies.iter_mut() {
                            e.pop_in = 1.0;
                        }
                    }
                }
                State::Play => {
                    if !g.p[1].joined && (key_pressed(BLIP_KEY_J) || key_pressed(BLIP_KEY_K)) {
                        g.join_player_two();
                    }
                    if !blip.screenshot_mode || shot_frame > 400 {
                        update_play(&mut g, dt, inp, &sfx);
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    for _ in 1..speed {
                        if g.state != State::Play {
                            break;
                        }
                        let extra = [bot_input(&g, 0, &mut bot_mem[0]), bot_input(&g, 1, &mut bot_mem[1])];
                        update_play(&mut g, dt, extra, &sfx);
                        update_fx(&mut g, dt);
                        bot_clock += dt;
                    }
                }
                State::Clear => {
                    let player_starts = std::array::from_fn(|i| (g.p[i].cx(), g.p[i].cy()));
                    move_players(&mut g, inp, dt, &sfx);
                    move_bubbles(&mut g, dt, inp, &sfx, &[], player_starts);
                    move_candies(&mut g, dt, &sfx, player_starts);
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
        if !frozen {
            update_fx(&mut g, dt);
        }

        let want_music = matches!(g.state, State::Play | State::Intro | State::Clear | State::Title);
        if want_music {
            music.play(if g.state == State::Play && g.hurry { 1 } else { 0 });
        } else {
            music.stop();
        }

        blip.clear(BlipColor::new(0.0, 0.0, 0.0, 1.0));
        match g.state {
            State::Title => draw_title(&blip, &art, &g, &hi),
            _ => {
                draw_world(&blip, &art, &g);
                draw_scores(&blip, &g, &hi);
                let st = g.state_t;
                // the end screens dim the world so their words sit on calm
                if matches!(g.state, State::Over | State::Won) {
                    blip.fill_rect(
                        0.0,
                        HUD,
                        WIN_W as f32,
                        WIN_H as f32 - HUD,
                        col((20, 8, 28), 0.55 * (st * 2.0).min(1.0)),
                    );
                }
                match g.state {
                    State::Intro => {
                        cosy(
                            &blip,
                            &format!("ROUND {}", g.round + 1),
                            170.0 + DY,
                            3.5,
                            PEACH,
                            PEACH,
                            st * 3.0,
                            (st * 4.0).min(1.0),
                        );
                        if st > 0.7 {
                            cosy(
                                &blip,
                                "READY!",
                                236.0 + DY,
                                2.2,
                                CREAM,
                                MINT,
                                (st - 0.7) * 3.0,
                                ((st - 0.7) * 4.0).min(1.0),
                            );
                        }
                    }
                    State::Play if g.hurry && g.round_t < hurry_at(g.round) + 2.4 => {
                        let a = 0.6 + 0.4 * (g.round_t * 8.0).sin().abs();
                        cosy(&blip, "HURRY UP!", 196.0 + DY, 3.2, (255, 150, 150), (255, 110, 120), 1.0, a);
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
                            cosy(
                                &blip,
                                "EXTEND  +1 LIFE",
                                246.0 + DY,
                                1.8,
                                PEACH,
                                PINK,
                                (st - 0.6) * 3.0,
                                ((st - 0.6) * 4.0).min(1.0),
                            );
                        }
                    }
                    State::Over => {
                        cosy(&blip, "GAME OVER", 160.0 + DY, 3.8, PINK, PINK, st * 2.0, (st * 3.0).min(1.0));
                        if hi.score > 0 {
                            cosy(
                                &blip,
                                &hi.label("HI"),
                                222.0 + DY,
                                1.5,
                                PEACH,
                                PEACH,
                                1.0,
                                (st - 1.0).clamp(0.0, 1.0),
                            );
                        }
                        if st > 2.5 {
                            cosy(
                                &blip,
                                "PRESS BUBBLE",
                                276.0 + DY,
                                2.0,
                                CREAM,
                                MINT,
                                1.0,
                                0.65 + 0.35 * (st * 2.5).sin().abs(),
                            );
                        }
                    }
                    State::Won => {
                        cosy(&blip, "ALL ROUNDS CLEAR!", 120.0 + DY, 2.6, CREAM, MINT, st * 2.0, (st * 3.0).min(1.0));
                        cosy(
                            &blip,
                            "HAPPY END",
                            192.0 + DY,
                            3.8,
                            PEACH,
                            PINK,
                            (st - 0.5) * 2.0,
                            ((st - 0.5) * 3.0).clamp(0.0, 1.0),
                        );
                        if st > 2.5 {
                            cosy(
                                &blip,
                                "PRESS BUBBLE",
                                276.0 + DY,
                                2.0,
                                CREAM,
                                MINT,
                                1.0,
                                0.65 + 0.35 * (st * 2.5).sin().abs(),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
        blip.next_frame(60).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(x: f32, y: f32) -> (usize, usize) {
        (((y - HUD) / TILE) as usize, (x / TILE) as usize)
    }

    #[test]
    fn every_round_is_a_full_grid_with_both_players_and_something_to_trap() {
        for (round, rows) in LEVELS.iter().enumerate() {
            assert!(
                rows.iter().all(|row| row.chars().count() == COLS),
                "round {}: a row is not {COLS} wide",
                round + 1
            );
            let text = rows.concat();
            assert_eq!(text.matches('A').count(), 1, "round {}: player one's start", round + 1);
            assert_eq!(text.matches('B').count(), 1, "round {}: player two's start", round + 1);
            assert!(!level_spawns(round).2.is_empty(), "round {}: no monsters", round + 1);
        }
    }

    #[test]
    fn nobody_starts_inside_a_wall() {
        for (round, rows) in LEVELS.iter().enumerate() {
            let solid = |(r, c): (usize, usize)| rows[r].as_bytes()[c] == b'#';
            let (a, b, monsters) = level_spawns(round);
            for (x, y) in [a, b].into_iter().chain(monsters.iter().map(|&(_, x, y)| (x, y))) {
                assert!(!solid(cell(x + 1.0, y + 1.0)), "round {}: a start at ({x}, {y}) is in a wall", round + 1);
            }
        }
    }

    #[test]
    fn no_monster_starts_on_top_of_a_player() {
        for round in 0..ROUNDS {
            let (a, b, monsters) = level_spawns(round);
            for &(_, x, y) in &monsters {
                for p in [a, b] {
                    assert!(
                        (x - p.0).hypot(y - p.1) > 3.0 * TILE,
                        "round {}: a monster starts within three tiles of a player",
                        round + 1
                    );
                }
            }
        }
    }

    #[test]
    fn the_pressure_rises_then_holds_from_the_ramps_top() {
        for round in 0..RAMP_TOP {
            assert!(monster_pace(round + 1) >= monster_pace(round));
            assert!(trap_life(round + 1) <= trap_life(round));
        }
        for round in RAMP_TOP..ROUNDS {
            assert_eq!(monster_pace(round), monster_pace(RAMP_TOP));
            assert_eq!(trap_life(round), trap_life(RAMP_TOP));
        }
        assert!(trap_life(ROUNDS) >= 7.0, "a trapped monster stays caught long enough to pop");
    }

    #[test]
    fn hurry_up_always_comes_before_the_skull() {
        for round in 0..ROUNDS {
            assert!(hurry_at(round) < skull_at(round), "round {}", round + 1);
        }
    }

    #[test]
    fn the_game_goes_on_while_a_joined_player_has_lives_or_is_falling() {
        let mut g = Game::new();
        g.p[0].joined = true;
        g.p[1].joined = false;
        g.p[0].lives = 1;
        assert!(players_left(&g));
        g.p[0].lives = 0;
        g.p[0].dead_t = 0.5;
        assert!(players_left(&g), "the last life is still tumbling");
        g.p[0].dead_t = 0.0;
        assert!(!players_left(&g));
    }
}
