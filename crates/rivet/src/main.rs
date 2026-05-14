//! RIVET — construction-site platformer (Donkey Kong homage)

use blip::input::{
    any_key_pressed, key_held, key_pressed,
    BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_DOWN, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::color::Color;
use blip::macroquad::rand::rand;
use blip::{
    play_music, play_sfx, pool_iter, pool_spawn, rects_overlap, web, window_conf, Blip,
    LifeResult, Pooled, Session, Timer,
    BLIP_BLACK, BLIP_DARKGRAY, BLIP_GREEN, BLIP_GRAY,
    BLIP_ORANGE, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
};

// ── Window ───────────────────────────────────────────────────────────
const WIN_W: i32 = 320;
const WIN_H: i32 = 480;

const GAME_ID: i32 = blip::web::GAME_RIVET;

// ── Colors ────────────────────────────────────────────────────────────
const BROWN:    Color = Color { r: 0.55, g: 0.27, b: 0.07, a: 1.0 };
const DK_DARK:  Color = Color { r: 0.22, g: 0.10, b: 0.02, a: 1.0 };
const STEEL:    Color = Color { r: 0.18, g: 0.42, b: 0.80, a: 1.0 };
const STEEL_LT: Color = Color { r: 0.44, g: 0.68, b: 1.00, a: 1.0 };
const SKIN:     Color = Color { r: 1.00, g: 0.82, b: 0.60, a: 1.0 };
const DK_BLUE:  Color = Color { r: 0.10, g: 0.20, b: 0.82, a: 1.0 };
const PINK:     Color = Color { r: 1.00, g: 0.41, b: 0.71, a: 1.0 };

// ── Platforms: (x1, y_top, x2) ───────────────────────────────────────
// Player feet snap to y_top when landing.
const PLAT_H: f32 = 8.0;
const PLAT: [(f32, f32, f32); 5] = [
    (0.0,  440.0, 320.0),   // 0  ground
    (20.0, 360.0, 300.0),   // 1
    (20.0, 280.0, 300.0),   // 2
    (20.0, 200.0, 300.0),   // 3
    (20.0, 120.0, 300.0),   // 4  top / gorilla
];

// ── Ladders: (x_left, y_top, y_bot) ──────────────────────────────────
// y_top = upper platform y, y_bot = lower platform y.
const LAD_W: f32 = 14.0;
const LADS: [(f32, f32, f32); 4] = [
    (245.0, 360.0, 440.0),  // 0  ground → 1  (right side)
    (60.0,  280.0, 360.0),  // 1  1 → 2       (left side)
    (245.0, 200.0, 280.0),  // 2  2 → 3       (right side)
    (60.0,  120.0, 200.0),  // 3  3 → 4       (left side)
];

// ── Player ────────────────────────────────────────────────────────────
const PL_W: f32 = 14.0;
const PL_H: f32 = 22.0;
const PL_SPD: f32 = 88.0;
const PL_CLM: f32 = 62.0;
const JUMP_V: f32 = -262.0;
const GRAV:   f32 = 580.0;
const MAX_FALL: f32 = 420.0;

// ── Barrel ────────────────────────────────────────────────────────────
const BR_W: f32 = 14.0;
const BR_H: f32 = 14.0;
const BR_SPD: f32 = 78.0;
const BR_CLM: f32 = 52.0;
const MAX_B: usize = 8;

// ── Gorilla ───────────────────────────────────────────────────────────
const GOR_X: f32 = 22.0;
const GOR_Y: f32 = 78.0; // visual top (PLAT[4].1 - 42)

const LIVES_START: i32 = 3;

// ── State ─────────────────────────────────────────────────────────────
#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Play, Dead, Win, Over }

#[derive(Copy, Clone, PartialEq, Eq)]
enum PlMode { Ground(usize), Air, Climb(usize) }

struct Player {
    x: f32, y: f32,
    vx: f32, vy: f32,
    mode: PlMode,
    facing: i32,
    anim: f32,
    scored: u8,  // bitmask: which barrel indices gave score this jump
}

impl Player {
    fn spawn() -> Self {
        Self {
            x: (WIN_W as f32 - PL_W) * 0.5,
            y: PLAT[0].1 - PL_H,
            vx: 0.0, vy: 0.0,
            mode: PlMode::Ground(0),
            facing: 1,
            anim: 0.0,
            scored: 0,
        }
    }
    fn feet(&self) -> f32 { self.y + PL_H }
    fn cx(&self)   -> f32 { self.x + PL_W * 0.5 }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum BmMode { Roll(usize), Fall, Descend(usize) }

#[derive(Copy, Clone)]
struct Barrel {
    x: f32, y: f32,
    vx: f32, vy: f32,
    mode: BmMode,
    active: bool,
    anim: f32,
    lad_zone: bool, // in a ladder x-zone right now (prevents re-rolling the dice every frame)
}

impl Pooled for Barrel {
    fn is_active(&self) -> bool { self.active }
}

impl Barrel {
    fn inactive() -> Self {
        Self {
            x: 0.0, y: 0.0, vx: 0.0, vy: 0.0,
            mode: BmMode::Roll(0), active: false, anim: 0.0, lad_zone: false,
        }
    }
    fn feet(&self) -> f32 { self.y + BR_H }
    fn cx(&self)   -> f32 { self.x + BR_W * 0.5 }
}

struct Sounds {
    jump:  blip::BlipSound,
    die:   blip::BlipSound,
    score: blip::BlipSound,
    win:   blip::BlipSound,
}

struct Game {
    pl:      Player,
    barrels: [Barrel; MAX_B],
    sess:    Session,
    state:   State,
    dead_t:    Timer,
    win_t:     Timer,
    throw_cd:  Timer,
    throw_int: f32,
    gor_t:  Timer, // active = gorilla throw animation playing
    flash:  Timer,
}

impl Game {
    fn new() -> Self {
        Self {
            pl:      Player::spawn(),
            barrels: [Barrel::inactive(); MAX_B],
            sess:    Session::new(GAME_ID, LIVES_START),
            state:   State::Title,
            dead_t: Timer::default(), win_t: Timer::default(),
            throw_cd: Timer::default(), throw_int: 2.5,
            gor_t: Timer::default(), flash: Timer::default(),
        }
    }

    fn start(&mut self) {
        self.sess.reset(GAME_ID, LIVES_START);
        self.begin_level();
        self.state = State::Play;
    }

    fn begin_level(&mut self) {
        self.throw_int = (3.2 - (self.sess.level - 1) as f32 * 0.28).max(1.2);
        self.throw_cd.start(self.throw_int * 0.6); // first barrel arrives sooner
        for b in &mut self.barrels { b.active = false; }
        self.gor_t = Timer::default();
        self.pl = Player::spawn();
    }

    fn spawn_barrel(&mut self) {
        pool_spawn(&mut self.barrels, Barrel {
            x: GOR_X + 30.0,
            y: PLAT[4].1 - BR_H,
            vx: BR_SPD,
            vy: 0.0,
            mode: BmMode::Roll(4),
            active: true,
            anim: 0.0,
            lad_zone: false,
        });
    }

    fn active_barrels(&self) -> usize {
        pool_iter(&self.barrels).count()
    }
}

// ── Player update ──────────────────────────────────────────────────────
fn update_player(pl: &mut Player, dt: f32) -> bool {
    let mut jumped = false;
    match pl.mode {
        PlMode::Ground(pi) => {
            let (px1, _py, px2) = PLAT[pi];
            let mut vx = 0.0_f32;
            if key_held(BLIP_KEY_LEFT) || key_held(BLIP_KEY_A)  { vx = -PL_SPD; pl.facing = -1; }
            if key_held(BLIP_KEY_RIGHT) || key_held(BLIP_KEY_D) { vx =  PL_SPD; pl.facing =  1; }
            pl.x = (pl.x + vx * dt).clamp(px1, px2 - PL_W);
            pl.anim = if vx != 0.0 { (pl.anim + dt * 8.0) % 2.0 } else { 0.0 };

            // Ladder grab is checked before jump so Up-to-climb takes priority.
            let cx  = pl.cx();
            let py  = PLAT[pi].1;
            let dn  = key_held(BLIP_KEY_DOWN) || key_held(BLIP_KEY_S);
            let up  = key_held(BLIP_KEY_UP)   || key_held(BLIP_KEY_W);
            let mut grabbed = false;
            for (i, &(lx, ly_top, ly_bot)) in LADS.iter().enumerate() {
                if cx >= lx && cx <= lx + LAD_W {
                    // Ladder top sits on our platform → press down to enter
                    if dn && (ly_top - py).abs() < 3.0 {
                        pl.x    = lx + LAD_W * 0.5 - PL_W * 0.5;
                        pl.y    = ly_top - PL_H + 4.0; // nudge inside so top-exit check doesn't fire instantly
                        pl.mode = PlMode::Climb(i);
                        pl.anim = 0.0;
                        grabbed = true;
                        break;
                    }
                    // Ladder bottom sits on our platform → press up to enter
                    if up && (ly_bot - py).abs() < 3.0 {
                        pl.x    = lx + LAD_W * 0.5 - PL_W * 0.5;
                        pl.y    = ly_bot - PL_H - 4.0; // nudge inside so bottom-exit check doesn't fire instantly
                        pl.mode = PlMode::Climb(i);
                        pl.anim = 0.0;
                        grabbed = true;
                        break;
                    }
                }
            }
            if !grabbed {
                let jump_pressed = key_pressed(BLIP_KEY_SPACE)
                    || key_pressed(BLIP_KEY_UP)
                    || key_pressed(BLIP_KEY_W);
                if jump_pressed {
                    pl.vy   = JUMP_V;
                    pl.vx   = vx;
                    pl.mode = PlMode::Air;
                    pl.scored = 0;
                    jumped  = true;
                }
            }
        }

        PlMode::Air => {
            pl.vy = (pl.vy + GRAV * dt).min(MAX_FALL);
            let old_y = pl.y;
            pl.y += pl.vy * dt;
            pl.x = (pl.x + pl.vx * dt).clamp(0.0, WIN_W as f32 - PL_W);

            if pl.vy > 0.0 {
                let prev_feet = old_y + PL_H;
                let curr_feet = pl.y + PL_H;
                for (i, &(px1, py, px2)) in PLAT.iter().enumerate() {
                    if pl.x + PL_W > px1 && pl.x < px2
                        && prev_feet <= py && curr_feet >= py
                    {
                        pl.y  = py - PL_H;
                        pl.vy = 0.0;
                        pl.vx = 0.0;
                        let (p1, _, p2) = PLAT[i];
                        pl.x = pl.x.clamp(p1, p2 - PL_W);
                        pl.mode = PlMode::Ground(i);
                        break;
                    }
                }
            }
        }

        PlMode::Climb(li) => {
            let (lx, ly_top, ly_bot) = LADS[li];
            let mut vy = 0.0_f32;
            if key_held(BLIP_KEY_UP)   || key_held(BLIP_KEY_W) { vy = -PL_CLM; }
            if key_held(BLIP_KEY_DOWN) || key_held(BLIP_KEY_S) { vy =  PL_CLM; }
            pl.y += vy * dt;
            if vy != 0.0 { pl.anim = (pl.anim + dt * 6.0) % 2.0; }

            // Reached top → land on upper platform
            if pl.feet() <= ly_top + 2.0 {
                pl.y = ly_top - PL_H;
                for (i, &(px1, py, px2)) in PLAT.iter().enumerate() {
                    if (py - ly_top).abs() < 4.0 {
                        pl.mode = PlMode::Ground(i);
                        pl.x = pl.x.clamp(px1, px2 - PL_W);
                        break;
                    }
                }
            }
            // Reached bottom → land on lower platform
            if pl.feet() >= ly_bot - 2.0 {
                pl.y = ly_bot - PL_H;
                for (i, &(px1, py, px2)) in PLAT.iter().enumerate() {
                    if (py - ly_bot).abs() < 4.0 {
                        pl.mode = PlMode::Ground(i);
                        pl.x = pl.x.clamp(px1, px2 - PL_W);
                        break;
                    }
                }
            }
            // Snap x to ladder center while climbing
            pl.x = lx + LAD_W * 0.5 - PL_W * 0.5;
        }
    }
    jumped
}

// ── Barrel update ──────────────────────────────────────────────────────
fn update_barrel(b: &mut Barrel, dt: f32) {
    match b.mode {
        BmMode::Roll(pi) => {
            let (px1, py, px2) = PLAT[pi];
            b.anim = (b.anim + dt * 5.5) % 4.0;
            b.x   += b.vx * dt;

            // Check ladder descent opportunity (once per entry into zone)
            let cx = b.cx();
            let mut in_zone = false;
            for (i, &(lx, ly_top, _ly_bot)) in LADS.iter().enumerate() {
                if (ly_top - py).abs() < 3.0 && cx >= lx && cx <= lx + LAD_W {
                    in_zone = true;
                    if !b.lad_zone {
                        b.lad_zone = true;
                        if rand() % 100 < 40 {
                            b.x    = lx + LAD_W * 0.5 - BR_W * 0.5;
                            b.vy   = BR_CLM;
                            b.vx   = 0.0;
                            b.mode = BmMode::Descend(i);
                            return;
                        }
                    }
                    break;
                }
            }
            if !in_zone { b.lad_zone = false; }

            // Fell off platform edge → enter fall.
            // Nudge y down so prev_feet > py next frame and the barrel doesn't
            // immediately snap back to the same platform via the landing check.
            if b.x + BR_W > px2 {
                b.x    = px2 - BR_W;
                b.y   += 3.0;
                b.mode = BmMode::Fall;
                b.vy   = 3.0;
                b.vx   = 18.0;
            } else if b.x < px1 {
                b.x    = px1;
                b.y   += 3.0;
                b.mode = BmMode::Fall;
                b.vy   = 3.0;
                b.vx   = -18.0;
            }
        }

        BmMode::Fall => {
            let old_y = b.y;
            b.vy = (b.vy + GRAV * dt).min(MAX_FALL);
            b.y += b.vy * dt;
            b.x += b.vx * dt;

            if b.vy > 0.0 {
                let prev_feet = old_y + BR_H;
                let curr_feet = b.y + BR_H;
                for (i, &(px1, py, px2)) in PLAT.iter().enumerate() {
                    if b.x + BR_W > px1 && b.x < px2
                        && prev_feet <= py && curr_feet >= py
                    {
                        b.y  = py - BR_H;
                        b.x  = b.x.clamp(px1, px2 - BR_W);
                        b.vy = 0.0;
                        // Even platforms roll right, odd roll left — natural zigzag.
                        b.vx = if i % 2 == 0 { BR_SPD } else { -BR_SPD };
                        b.mode = BmMode::Roll(i);
                        b.lad_zone = false;
                        break;
                    }
                }
            }

            if b.y > WIN_H as f32 + 20.0 { b.active = false; }
        }

        BmMode::Descend(li) => {
            let (_lx, _ly_top, ly_bot) = LADS[li];
            b.y   += b.vy * dt;
            b.anim = (b.anim + dt * 3.5) % 4.0;

            if b.feet() >= ly_bot {
                b.y = ly_bot - BR_H;
                for (i, &(px1, py, px2)) in PLAT.iter().enumerate() {
                    if (py - ly_bot).abs() < 4.0 {
                        b.x  = b.x.clamp(px1, px2 - BR_W);
                        b.vx = if i % 2 == 0 { BR_SPD } else { -BR_SPD };
                        b.vy = 0.0;
                        b.mode = BmMode::Roll(i);
                        b.lad_zone = false;
                        break;
                    }
                }
            }
        }
    }
}

// ── State update functions ─────────────────────────────────────────────
fn update_title(g: &mut Game) {
    g.sess.refresh_hi(GAME_ID);
    if any_key_pressed() { g.start(); }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.sess.refresh_hi(GAME_ID);

    // Gorilla throw timer
    if g.throw_cd.tick(dt) {
        let max_active = (2 + g.sess.level).min(MAX_B as i32) as usize;
        if g.active_barrels() < max_active {
            g.gor_t.start(0.45);
            g.throw_cd.start(g.throw_int);
            g.spawn_barrel();
        } else {
            g.throw_cd.start(0.8); // retry soon
        }
    }
    g.gor_t.tick(dt);

    // Player
    let jumped = update_player(&mut g.pl, dt);
    if jumped { play_sfx(&sfx.jump); }

    // Barrels
    for i in 0..MAX_B {
        if g.barrels[i].active { update_barrel(&mut g.barrels[i], dt); }
    }

    // Barrel ↔ player collisions
    let px = g.pl.x; let py = g.pl.y;
    for i in 0..MAX_B {
        let b = &mut g.barrels[i];
        if !b.active { continue; }

        // Death collision
        if rects_overlap(px, py, PL_W, PL_H, b.x, b.y, BR_W, BR_H) {
            play_sfx(&sfx.die);
            g.flash.start(1.2);
            match g.sess.lose_life() {
                LifeResult::StillAlive => {
                    g.dead_t.start(1.2);
                    g.state = State::Dead;
                }
                LifeResult::GameOver => {
                    web::game_over(GAME_ID, g.sess.score);
                    g.state = State::Over;
                }
            }
            return;
        }

        // Jump-over scoring: barrel passes under player during jump
        if matches!(g.pl.mode, PlMode::Air) && g.pl.vy > 0.0 {
            let bit = 1u8 << (i & 7);
            if g.pl.scored & bit == 0 {
                let bcx = b.cx();
                let bcy = b.y + BR_H * 0.5;
                if (bcx - g.pl.cx()).abs() < 26.0
                    && (bcy - g.pl.feet()).abs() < 22.0
                {
                    g.pl.scored |= bit;
                    g.sess.add_score(GAME_ID, 100);
                    play_sfx(&sfx.score);
                }
            }
        }
    }

    // Win: player reaches top platform
    if let PlMode::Ground(pi) = g.pl.mode {
        if pi == 4 {
            play_sfx(&sfx.win);
            g.sess.add_score(GAME_ID, 500 + g.sess.level * 100);
            g.win_t.start(2.2);
            g.state = State::Win;
        }
    }

    // Player fell off bottom
    if g.pl.y > WIN_H as f32 + 10.0 {
        play_sfx(&sfx.die);
        g.flash.start(1.2);
        match g.sess.lose_life() {
            LifeResult::StillAlive => {
                g.dead_t.start(1.2);
                g.state = State::Dead;
            }
            LifeResult::GameOver => {
                web::game_over(GAME_ID, g.sess.score);
                g.state = State::Over;
            }
        }
    }
}

fn update_dead(g: &mut Game, dt: f32) {
    g.flash.tick(dt);
    if g.dead_t.tick(dt) {
        g.pl    = Player::spawn();
        g.state = State::Play;
    }
}

fn update_win(g: &mut Game, dt: f32) {
    if g.win_t.tick(dt) {
        g.sess.next_level();
        g.begin_level();
        g.state = State::Play;
    }
}

fn update_over(g: &mut Game) {
    g.sess.refresh_hi(GAME_ID);
    if !any_key_pressed() { return; }
    web::spend_coin();
    g.start();
}

// ── Drawing ────────────────────────────────────────────────────────────

fn draw_platforms(blip: &Blip) {
    for &(x1, y, x2) in &PLAT {
        blip.fill_rect(x1, y, x2 - x1, PLAT_H, STEEL);
        blip.fill_rect(x1, y, x2 - x1, 2.0, STEEL_LT);
        blip.fill_rect(x1, y + PLAT_H, x2 - x1, 2.0, BLIP_DARKGRAY);
        // Rivet bolts
        let mut rx = x1 + 8.0;
        while rx < x2 - 6.0 {
            blip.fill_rect(rx, y + 2.0, 4.0, 4.0, STEEL_LT);
            rx += 20.0;
        }
    }
}

fn draw_ladders(blip: &Blip) {
    for &(lx, ly_top, ly_bot) in &LADS {
        let h = ly_bot - ly_top;
        // Rails
        blip.fill_rect(lx, ly_top, 2.0, h, BLIP_YELLOW);
        blip.fill_rect(lx + LAD_W - 2.0, ly_top, 2.0, h, BLIP_YELLOW);
        // Rungs
        let mut ry = ly_top + 4.0;
        while ry < ly_bot - 2.0 {
            blip.fill_rect(lx + 2.0, ry, LAD_W - 4.0, 2.0, BLIP_YELLOW);
            ry += 9.0;
        }
    }
}

fn draw_gorilla(blip: &Blip, throwing: bool) {
    let x = GOR_X;
    let y = GOR_Y;
    // Head
    blip.fill_rect(x + 3.0, y, 22.0, 14.0, BROWN);
    // Ears
    blip.fill_rect(x,        y + 3.0, 4.0, 6.0, BROWN);
    blip.fill_rect(x + 24.0, y + 3.0, 4.0, 6.0, BROWN);
    // Dark face
    blip.fill_rect(x + 5.0, y + 4.0, 18.0, 8.0, DK_DARK);
    // Eyes
    blip.fill_rect(x + 6.0,  y + 5.0, 4.0, 4.0, BLIP_WHITE);
    blip.fill_rect(x + 18.0, y + 5.0, 4.0, 4.0, BLIP_WHITE);
    blip.fill_rect(x + 7.0,  y + 6.0, 2.0, 2.0, BLIP_DARKGRAY);
    blip.fill_rect(x + 19.0, y + 6.0, 2.0, 2.0, BLIP_DARKGRAY);
    // Body
    blip.fill_rect(x, y + 14.0, 28.0, 20.0, BROWN);
    // Belly
    blip.fill_rect(x + 6.0, y + 16.0, 16.0, 14.0, DK_DARK);
    // Arms
    if throwing {
        blip.fill_rect(x - 8.0, y + 10.0, 9.0, 10.0, BROWN); // left arm mid
        blip.fill_rect(x + 27.0, y,        9.0, 16.0, BROWN); // right arm raised
    } else {
        blip.fill_rect(x - 8.0, y + 14.0, 9.0, 12.0, BROWN); // left arm down
        blip.fill_rect(x + 27.0, y + 14.0, 9.0, 12.0, BROWN); // right arm down
    }
    // Legs
    blip.fill_rect(x + 3.0,  y + 34.0, 9.0, 8.0, DK_DARK);
    blip.fill_rect(x + 16.0, y + 34.0, 9.0, 8.0, DK_DARK);
}

fn draw_princess(blip: &Blip) {
    // Stands on platform 4, to the right of the gorilla.
    let x = 190.0_f32;
    let y = PLAT[4].1 - 30.0; // feet at platform top
    // Hair (blonde, piled up)
    blip.fill_rect(x + 2.0, y,        12.0, 4.0, BLIP_YELLOW);
    blip.fill_rect(x + 4.0, y - 4.0,  8.0, 4.0, BLIP_YELLOW);
    // Tiara
    blip.fill_rect(x + 5.0, y - 7.0,  2.0, 3.0, BLIP_YELLOW); // center spike
    blip.fill_rect(x + 9.0, y - 6.0,  2.0, 2.0, BLIP_YELLOW); // right spike
    blip.fill_rect(x + 2.0, y - 6.0,  2.0, 2.0, BLIP_YELLOW); // left spike
    // Face
    blip.fill_rect(x + 3.0, y + 4.0,  10.0, 8.0, SKIN);
    // Eyes
    blip.fill_rect(x + 5.0, y + 6.0,  2.0, 2.0, BLIP_DARKGRAY);
    blip.fill_rect(x + 9.0, y + 6.0,  2.0, 2.0, BLIP_DARKGRAY);
    // Mouth (small smile)
    blip.fill_rect(x + 6.0, y + 10.0, 4.0, 1.0, BLIP_DARKGRAY);
    // Arms
    blip.fill_rect(x,        y + 12.0, 3.0, 8.0, SKIN);
    blip.fill_rect(x + 13.0, y + 12.0, 3.0, 8.0, SKIN);
    // Dress bodice
    blip.fill_rect(x + 3.0, y + 12.0, 10.0, 9.0, PINK);
    // Skirt (wider at bottom)
    blip.fill_rect(x + 1.0, y + 21.0, 14.0, 9.0, PINK);
    // "HELP!" heart bubble above her
    blip.fill_rect(x + 4.0, y - 14.0, 3.0, 3.0, BLIP_RED);
    blip.fill_rect(x + 9.0, y - 14.0, 3.0, 3.0, BLIP_RED);
    blip.fill_rect(x + 3.0, y - 12.0, 10.0, 4.0, BLIP_RED);
    blip.fill_rect(x + 5.0, y - 9.0,  6.0, 3.0, BLIP_RED);
    blip.fill_rect(x + 7.0, y - 7.0,  2.0, 2.0, BLIP_RED);
}

fn draw_player(blip: &Blip, pl: &Player, flash: Timer) {
    if flash.active() && (flash.remaining() * 8.0) as i32 % 2 == 0 { return; }

    let x = pl.x;
    let y = pl.y;
    let f = pl.facing;

    // Hat
    blip.fill_rect(x + 1.0, y, PL_W - 2.0, 5.0, BLIP_RED);
    blip.fill_rect(x, y + 3.0, PL_W, 2.0, BLIP_RED);
    // Face
    blip.fill_rect(x + 2.0, y + 5.0, PL_W - 4.0, 7.0, SKIN);
    // Eye (side that faces direction)
    let eye_x = if f > 0 { x + PL_W - 5.0 } else { x + 3.0 };
    blip.fill_rect(eye_x, y + 6.5, 2.0, 2.0, BLIP_DARKGRAY);
    // Mustache
    let mus_x = if f > 0 { x + PL_W - 7.0 } else { x + 2.0 };
    blip.fill_rect(mus_x, y + 9.5, 5.0, 2.0, BROWN);
    // Body
    blip.fill_rect(x + 1.0, y + 12.0, PL_W - 2.0, 6.0, BLIP_RED);
    // Overalls shoulder straps
    blip.fill_rect(x + 3.0, y + 12.0, 2.0, 3.0, DK_BLUE);
    blip.fill_rect(x + 9.0, y + 12.0, 2.0, 3.0, DK_BLUE);
    // Legs (animated walk cycle)
    let frame = pl.anim as i32 % 2;
    if matches!(pl.mode, PlMode::Climb(_)) {
        // Climbing: legs stationary, arms at rungs
        blip.fill_rect(x + 1.0, y + 18.0, 5.0, 4.0, DK_BLUE);
        blip.fill_rect(x + 8.0, y + 18.0, 5.0, 4.0, DK_BLUE);
    } else if frame == 0 {
        blip.fill_rect(x + 1.0, y + 18.0, 5.0, 4.0, DK_BLUE);
        blip.fill_rect(x + 8.0, y + 18.0, 5.0, 4.0, DK_BLUE);
    } else {
        blip.fill_rect(x + 1.0, y + 16.0, 5.0, 6.0, DK_BLUE); // left leg up
        blip.fill_rect(x + 8.0, y + 20.0, 5.0, 2.0, DK_BLUE); // right leg down
    }
}

fn draw_barrel(blip: &Blip, b: &Barrel) {
    let x = b.x; let y = b.y;
    // Body (rounded-ish via 3 rects)
    blip.fill_rect(x + 1.0, y,       BR_W - 2.0, BR_H,       BROWN);
    blip.fill_rect(x,        y + 2.0, BR_W,        BR_H - 4.0, BROWN);
    // Rolling bands (2 vertical stripes that shift with anim)
    let rot = (b.anim * 3.5) as i32;
    let b1 = ((rot      ) as f32).rem_euclid(BR_W) as i32;
    let b2 = ((rot + (BR_W as i32 / 2)) as f32).rem_euclid(BR_W) as i32;
    for &bx in &[b1, b2] {
        // Clip the stripe to barrel interior
        let sx = (x + 1.0 + bx as f32).min(x + BR_W - 3.0);
        blip.fill_rect(sx, y + 1.0, 2.0, BR_H - 2.0, DK_DARK);
    }
}

fn draw_scene(blip: &Blip, g: &Game) {
    blip.clear(BLIP_BLACK);

    draw_platforms(blip);
    draw_ladders(blip);
    draw_gorilla(blip, g.gor_t.active());
    draw_princess(blip);

    for b in pool_iter(&g.barrels) {
        draw_barrel(blip, b);
    }

    draw_player(blip, &g.pl, g.flash);

    blip.draw_hud(g.sess.score, g.sess.hi, g.sess.lives);
}

fn draw_title(blip: &Blip, hi: i32) {
    blip.clear(BLIP_BLACK);
    // Decorative girder lines
    for &y in &[160.0_f32, 240.0, 320.0, 400.0] {
        blip.fill_rect(0.0, y, WIN_W as f32, PLAT_H, STEEL);
        blip.fill_rect(0.0, y, WIN_W as f32, 2.0, STEEL_LT);
    }
    draw_gorilla(blip, false);
    blip.draw_centered("RIVET",        140.0, 7.0, BLIP_YELLOW);
    blip.draw_centered("PRESS ANY KEY", 220.0, 3.0, BLIP_WHITE);
    blip.draw_centered("CLIMB TO THE TOP", 260.0, 2.0, BLIP_GRAY);
    blip.draw_centered("ARROWS / WASD",   290.0, 2.0, BLIP_GRAY);
    blip.draw_centered("[1] SPACE TO JUMP", 310.0, 2.0, BLIP_GRAY);
    if hi > 0 {
        let s = format!("HI {}", hi);
        blip.draw_centered(&s, 350.0, 2.0, BLIP_ORANGE);
    }
}

fn draw_win(blip: &Blip, level: i32) {
    blip.clear(BLIP_BLACK);
    blip.draw_centered("STAGE CLEAR!", (WIN_H / 3) as f32,     5.0, BLIP_GREEN);
    let s = format!("NEXT LEVEL  {}", level + 1);
    blip.draw_centered(&s,            (WIN_H / 2) as f32,     3.0, BLIP_YELLOW);
}

fn draw_over(blip: &Blip, score: i32) {
    blip.clear(BLIP_BLACK);
    blip.draw_centered("GAME OVER",     (WIN_H / 4) as f32,     5.0, BLIP_RED);
    let s = format!("SCORE  {}", score);
    blip.draw_centered(&s,              (WIN_H / 2) as f32,     3.0, BLIP_WHITE);
    blip.draw_centered("PRESS ANY KEY", (WIN_H * 2 / 3) as f32, 3.0, BLIP_YELLOW);
}

// ── Asset bytes ────────────────────────────────────────────────────────
const JUMP_WAV:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/jump.wav"));
const DIE_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/die.wav"));
const SCORE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/score.wav"));
const WIN_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/win.wav"));
const MUSIC_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music.wav"));

fn conf() -> blip::macroquad::window::Conf {
    window_conf("RIVET", WIN_W, WIN_H)
}

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g    = Game::new();

    let sfx = Sounds {
        jump:  blip::audio::load_sound(JUMP_WAV).await,
        die:   blip::audio::load_sound(DIE_WAV).await,
        score: blip::audio::load_sound(SCORE_WAV).await,
        win:   blip::audio::load_sound(WIN_WAV).await,
    };
    let music = blip::audio::load_sound(MUSIC_WAV).await;
    play_music(&music);

    loop {
        let dt = blip.delta_time;
        match g.state {
            State::Title => update_title(&mut g),
            State::Play  => update_play(&mut g, dt, &sfx),
            State::Dead  => update_dead(&mut g, dt),
            State::Win   => update_win(&mut g, dt),
            State::Over  => update_over(&mut g),
        }

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title => draw_title(&blip, g.sess.hi),
            State::Win   => { draw_scene(&blip, &g); draw_win(&blip, g.sess.level); }
            State::Over  => { draw_scene(&blip, &g); draw_over(&blip, g.sess.score); }
            State::Play | State::Dead => draw_scene(&blip, &g),
        }

        blip.next_frame(60).await;
    }
}
