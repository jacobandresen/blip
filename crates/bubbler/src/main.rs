//! Bubble Bobble-inspired platform game with one- or two-player co-op.

use blip::input::{
    any_pressed, key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_D, BLIP_KEY_F, BLIP_KEY_G, BLIP_KEY_J,
    BLIP_KEY_K, BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
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
mod bot;
mod toys;
mod draw;
mod players;
mod bubbles;
mod candies;
mod hazards;
mod scenes;
use hazards::*;
use scenes::*;
use players::*;
use bubbles::*;
use candies::*;
use toys::*;
use draw::*;
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
/// Bodies touch only when their sprites overlap past this inset (px, a side and top and bottom), so a brush is not
/// a catch; a thrown shot reaches this far (px) beyond the dragon's sprite. 5 x 6 (was 3.5 x 5) for small players:
/// at a child's 400 ms reaction all four test runs lasted 90 s into round 2, where two had ended in round 1.
const CONTACT_INSET: (f32, f32) = (5.0, 6.0);
const SHOT_REACH: (f32, f32) = (3.0, 2.0);
const LIVES: i32 = 5;
/// How long a continue is offered when every dragon is out of hearts.
const CONTINUE_SECS: f32 = 9.0;
/// Everyone is drawn this much bigger than their hit box: soft toys, easy
/// to see; the box stays small, so a near miss is a miss.
const TOY_SCALE: f32 = 1.4;
/// The plush monsters are drawn bigger still (kawaii: big soft toys), over the same small hit box; trapped in a
/// bubble they keep the bubble's size.
const PLUSH_SCALE: f32 = 1.55; // ~1.1x the size squashed into a bubble, so a catch looks like it fits
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
use blip::now;

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
    /// Sets this toy's own rhythm of blinks and waves (from where it started, so no dice are thrown).
    seed: f32,
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

/// A little heart floating up from a dragon that has just eaten a candy.
struct Heart {
    x: f32,
    y: f32,
    t: f32,
    sway: f32,
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
    /// The continue offered when every dragon is out of hearts, and the best score player one reached before one
    /// was taken (a continue starts the score afresh; the best of the run is what is reported).
    cont: blip::Continue,
    run_best: i32,
    enemies: Vec<Enemy>,
    bubbles: Vec<Bubble>,
    fruits: Vec<Fruit>,
    parts: blip::EffectParticles<ParticleStyle>,
    pops: Vec<Popup>,
    hearts: Vec<Heart>,
    skull: Skull,
    shots: Vec<Shot>,
    round_t: f32,
    state_t: f32,
    hurry: bool,
    shake: f32,
    two_up: bool, // player two is at the cabinet (joined this game)
    hop_cd: f32,  // until a toy's hop may squeak again
    extend: bool, // the last clear gave a life back
}

/// The game has ended (Over or Won). A one-player game.s score goes to the
/// high-score board; a game player two joined sets none. A bot run ends here.
fn report_score(g: &Game) {
    blip::bot::finish(if g.state == State::Won { "won" } else { "over" });
    if !g.two_up {
        web::report_score(g.p[0].score.max(g.run_best));
    }
}

/// Where a round starts: each player's spot, and each enemy's kind and spot.
type Spawns = ((f32, f32), (f32, f32), Vec<(Kind, f32, f32)>);

fn level_spawns(round: usize) -> Spawns {
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
            hearts: Vec::new(),
            skull: Skull { active: false, x: 0.0, y: 0.0, speed: 0.0 },
            shots: Vec::new(),
            round_t: 0.0,
            state_t: 0.0,
            cont: blip::Continue::default(),
            run_best: 0,
            hurry: false,
            shake: 0.0,
            two_up: false,
            hop_cd: 0.0,
            extend: false,
        };
        g.load_round(0);
        g
    }

    fn load_round(&mut self, round: usize) {
        self.round = round;
        blip::bot::levels(ROUNDS);
        blip::bot::reached_level(round as i32 + 1);
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
                seed: x * 0.37 + y * 0.11,
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
        self.run_best = 0;
        self.two_up = two;
        web::set_players(if two { 1 } else { 2 });
        self.load_round(blip::bot::level(ROUNDS));
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
    #[allow(clippy::too_many_arguments)]
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

blip::sounds!(Sounds {
    blow, pop, trap, kill = "defeated", fruit, jump, lose = "lost", bounce, sparkle, huff, hop,
    round = "jingle_round", clear = "jingle_clear", hurry = "jingle_hurry", over = "jingle_over", won = "jingle_won"
});

// ---- update ------------------------------------------------------------

fn update_fx(g: &mut Game, dt: f32) {
    g.parts.update(dt);
    for p in g.pops.iter_mut() {
        p.t += dt;
        p.y = (p.y - 30.0 * dt).max(HUD + 6.0);
    }
    g.pops.retain(|p| p.t < 1.4);
    for h in g.hearts.iter_mut() {
        h.t += dt;
        if h.t > 0.0 {
            h.y -= 34.0 * dt;
            h.x += (h.t * 6.0).cos() * h.sway * 14.0 * dt;
        }
    }
    g.hearts.retain(|h| h.t < HEART_SECS);
    g.shake = (g.shake - dt).max(0.0);
    g.chain.1 = (g.chain.1 - dt).max(0.0);
}

fn players_left(g: &Game) -> bool {
    g.p.iter().any(|p| p.joined && (p.lives > 0 || p.dead_t > 0.0))
}

fn update_play(g: &mut Game, dt: f32, inp: [Input; 2], sfx: &Sounds) {
    g.round_t += dt;
    if !g.hurry && g.round_t >= hurry_at(g.round) { hurry_up(g, sfx); }
    let player_starts = std::array::from_fn(|i| (g.p[i].cx(), g.p[i].cy()));
    let enemy_starts: Vec<(f32, f32)> = g.enemies.iter().map(|e| (e.x + E_W / 2.0, e.y + E_H / 2.0)).collect();
    move_players(g, inp, dt, sfx);
    // one squeak at a time, however many toys hop together
    g.hop_cd -= dt;
    if move_toys(g, dt) && g.hop_cd <= 0.0 {
        g.hop_cd = 0.3;
        play_sfx_volume(&sfx.hop, 0.35);
    }
    move_bubbles(g, dt, inp, sfx, &enemy_starts, player_starts);
    move_candies(g, dt, sfx, player_starts);
    let skull_motion = update_skull(g, dt);
    update_chewing(g, dt);
    let shot_hits = move_shots(g, dt, player_starts);
    hurt_players(g, sfx, skull_motion, shot_hits, player_starts, &enemy_starts);
    g.enemies.retain(|e| e.active);
    if !players_left(g) {
        // Out of hearts: a continue is offered first, so a child can play on from the round reached.
        g.state = State::Over;
        g.state_t = 0.0;
        g.cont.start(CONTINUE_SECS);
        // A new best ends on the winning jingle: the child hears they did their best ever.
        play_sfx(if web::beats_best(g.p[0].score.max(g.run_best)) { &sfx.won } else { &sfx.over });
        return;
    }
    let trapped = g.bubbles.iter().any(|b| b.trapped.is_some());
    let death_pending = g.p.iter().any(|p| p.joined && p.dead_t > 0.0);
    if g.enemies.is_empty() && !trapped && !death_pending { room_cleared(g, sfx); }
}

/// The round has run long: every monster turns angry.
fn hurry_up(g: &mut Game, sfx: &Sounds) {
    g.hurry = true;
    g.stats.hurries += 1;
    for e in g.enemies.iter_mut() {
        e.angry = true;
    }
    play_sfx(&sfx.hurry);
}

/// Every monster is caught and popped: the skull and the shots go, and every player still in gets a life back (EXTEND),
/// up to EXTEND_MAX.
fn room_cleared(g: &mut Game, sfx: &Sounds) {
    g.state = State::Clear;
    g.state_t = 0.0;
    g.skull.active = false;
    g.shots.clear();
    g.extend = false;
    for p in g.p.iter_mut() {
        if p.joined && p.lives > 0 && p.lives < EXTEND_MAX {
            p.lives += 1;
            g.extend = true;
        }
    }
    play_sfx(&sfx.clear);
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("BUBBLER", WIN_W, WIN_H)
}

#[blip::macroquad::main(conf)]
async fn main() {
    blip::bot::playthroughs(bot::PLAYTHROUGHS);
    let mut blip = Blip::new(WIN_W, WIN_H);
    let art = art::Art::load();
    // Bright filled blocks twitter under the full interlace (see Brawler).
    blip.set_interlace(0.5);
    let mut g = Game::new();
    web::set_players(2);

    let sfx = Sounds::load().await;
    // The theme, and the same tune faster for HURRY UP.
    let mut music = Jukebox::new("bubbler", &["THEME", "HURRY"]);
    let mut shot_frame = 0u32;
    #[cfg(not(target_arch = "wasm32"))]
    let mut scene_set = false;

    loop {
        let dt = blip.delta_time;
        g.state_t += dt;
        let hi = web::high_score();

        if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 {
                stage_cabinet_card(&mut g);
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        let frozen = if let Ok(scene) = std::env::var("BUBBLER_SCENE") {
            if !scene_set {
                scene_set = true;
                stage_scene(&mut g, &scene);
            }
            true
        } else {
            false
        };
        #[cfg(target_arch = "wasm32")]
        let frozen = false;
        if blip::bot::active() {
            bot::drive(&g);
            bot::report(&g);
        }
        let solo = !g.two_up;
        let inp = [read(if solo { &SOLO } else { &P1_KEYS }), read(&P2_KEYS)];
        let p2_start = key_pressed(BLIP_KEY_J) || key_pressed(BLIP_KEY_K);

        if !frozen {
            update_screen(&mut g, dt, inp, p2_start, &sfx, !blip.screenshot_mode || shot_frame > 400);
            update_fx(&mut g, dt);
        }

        let want_music = matches!(g.state, State::Play | State::Intro | State::Clear | State::Title);
        if want_music {
            music.play(if g.state == State::Play && g.hurry { 1 } else { 0 });
        } else {
            music.stop();
        }

        draw(&blip, &art, &g, hi);
        blip.next_frame().await;
    }
}

/// The screen's rules this frame: start a game, the round's intro, play, the clear, the end screens.
/// `play` is false while a screenshot holds the board still.
fn update_screen(g: &mut Game, dt: f32, inp: [Input; 2], p2_start: bool, sfx: &Sounds, play: bool) {
    let pressed = inp[0].blow || inp[0].jump || key_pressed(BLIP_KEY_SPACE);
    match g.state {
        State::Title => {
            // A game is paid for as it starts: one coin, two for two players.
            if p2_start {
                g.start_game(true);
                web::spend_coin();
                web::spend_coin();
            } else if pressed {
                g.start_game(false);
                web::spend_coin();
            }
        }
        State::Intro => update_intro(g, dt, sfx),
        State::Play => {
            if !g.p[1].joined && (key_pressed(BLIP_KEY_J) || key_pressed(BLIP_KEY_K)) { g.join_player_two(); }
            if play { update_play(g, dt, inp, sfx); }
        }
        State::Clear => update_clear(g, dt, inp, sfx),
        State::Over => match g.cont.update(dt, pressed || p2_start) {
            // Play on: full hearts again, in the same round, the score started afresh.
            Some(true) => {
                blip::bot::add("continues", 1.0);
                g.run_best = g.run_best.max(g.p[0].score);
                for p in g.p.iter_mut().filter(|p| p.joined) {
                    p.lives = LIVES;
                    p.score = 0;
                    p.place();
                }
                g.state = State::Play;
                g.state_t = 0.0;
            }
            Some(false) => {
                report_score(g);
                back_to_title(g);
            }
            None => if g.cont.ticked() { play_sfx_volume(&sfx.bounce, 0.6); },
        },
        State::Won => if g.state_t > 2.5 && (pressed || p2_start) { back_to_title(g); },
    }
}

/// The title again, its second seat open.
fn back_to_title(g: &mut Game) {
    g.state = State::Title;
    g.two_up = false;
    web::set_players(2);
}

/// A round's opening: the gang pops into place, then play.
fn update_intro(g: &mut Game, dt: f32, sfx: &Sounds) {
    if g.state_t < dt * 1.5 { play_sfx(&sfx.round); }
    for e in g.enemies.iter_mut() { e.pop_in = (g.state_t / 1.2).min(0.999); }
    if g.state_t > 1.8 {
        g.state = State::Play;
        g.state_t = 0.0;
        for e in g.enemies.iter_mut() { e.pop_in = 1.0; }
    }
}

/// A room cleared: the players still run and collect, leftover bubbles pop one by one for a little applause, then
/// the next room (or, after the last, the win).
fn update_clear(g: &mut Game, dt: f32, inp: [Input; 2], sfx: &Sounds) {
    let player_starts = std::array::from_fn(|i| (g.p[i].cx(), g.p[i].cy()));
    move_players(g, inp, dt, sfx);
    move_bubbles(g, dt, inp, sfx, &[], player_starts);
    move_candies(g, dt, sfx, player_starts);
    if g.state_t > 1.0 {
        let tick = (g.state_t * 8.0) as i32 != ((g.state_t - dt) * 8.0) as i32;
        if let Some(i) = g.bubbles.iter().position(|b| b.active).filter(|_| tick) {
            let b = g.bubbles[i];
            g.bubbles[i].active = false;
            g.burst(b.x, b.y, 8, rgba(1.0, 1.0, 1.0, 0.8), 80.0, false);
            play_sfx_volume(&sfx.pop, 0.4);
        }
        g.bubbles.retain(|b| b.active);
    }
    if g.state_t > 3.4 {
        if g.round + 1 >= ROUNDS {
            g.state = State::Won;
            g.state_t = 0.0;
            report_score(g);
            play_sfx(&sfx.won);
        } else {
            g.load_round(g.round + 1);
            g.state = State::Intro;
            g.state_t = 0.0;
        }
    }
}

#[cfg(test)]
mod tests;
