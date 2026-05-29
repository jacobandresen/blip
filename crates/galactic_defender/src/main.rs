//! Galactic Defender (Space Invaders), Rust port of
//! `games/galactic_defender/main.c` on macroquad.

use blip::input::{
    any_key_pressed, key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::rand::rand;
use blip::macroquad::texture::Texture2D;
use blip::{
    blip_image, blip_sound, clamp, lerp, load_png, play_sfx, rand_int, rects_overlap, web,
    window_conf, Blip, BlipColor, LifeResult, MusicTracks, Session, Timer,
    BLIP_BLACK, BLIP_CYAN, BLIP_GREEN, BLIP_MAGENTA, BLIP_ORANGE, BLIP_RED,
    BLIP_WHITE, BLIP_YELLOW,
};

// ---- layout -----------------------------------------------------------
const WIN_W: i32 = 480;
const WIN_H: i32 = 540;
const HUD_H: i32 = 28;
const PLAY_Y: i32 = HUD_H;
const GROUND_Y: i32 = WIN_H - 32;

// ---- alien grid -------------------------------------------------------
const ALIEN_COLS: i32 = 13; // max across all themes (theme 2 uses 13)
const ALIEN_ROWS: i32 = 6;  // max across all themes (theme 1 uses 6)
const ALIEN_W: i32 = 32;
const ALIEN_H: i32 = 24;
const ALIEN_XGAP: i32 = 4;
const ALIEN_YGAP: i32 = 8;
const ALIEN_TOTAL: usize = (ALIEN_COLS * ALIEN_ROWS) as usize;

// ---- tuning -----------------------------------------------------------
const PLAYER_SPEED: f32 = 200.0;
const BULLET_SPEED: f32 = 350.0;
const MARCH_START: i32 = 520;
const MARCH_MIN: i32 = 65;
const MARCH_DROP: f32 = 16.0;
const MAX_BOMBS: usize = 4;
const MAX_PLAYER_BULLETS: usize = 1;
const SHIELD_COLS: usize = 4;
const SHIELD_ROWS: usize = 3;
const SHIELD_BLOCK: i32 = 12;
const SHIELDS: usize = 4;
const EXPLOSION_TTL: f32 = 0.45;
const LIVES_START: i32 = 3;

const MAX_UFO_BOMBS: usize = 1;
const UFO_BOMB_IDX: usize = MAX_PLAYER_BULLETS + MAX_BOMBS;
const N_BULLETS: usize = MAX_PLAYER_BULLETS + MAX_BOMBS + MAX_UFO_BOMBS;
const N_EXPLOSIONS: usize = ALIEN_TOTAL + 4;

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Play, Dead, Win, Over }

#[derive(Copy, Clone)]
struct Alien {
    x: f32, y: f32,
    alive: bool,
    kind: usize, // 0=squid 1=crab 2=octopus
    anim: u8,    // 0/1
}

#[derive(Copy, Clone)]
struct Bullet {
    x: f32, y: f32,
    active: bool,
    player: bool,
}

#[derive(Copy, Clone)]
struct Explosion { x: f32, y: f32, ttl: f32, active: bool }

#[derive(Copy, Clone)]
struct Shield {
    x: f32, y: f32,
    alive: [[bool; SHIELD_COLS]; SHIELD_ROWS],
}

struct Game {
    aliens: [Alien; ALIEN_TOTAL],
    bullets: [Bullet; N_BULLETS],
    explosions: [Explosion; N_EXPLOSIONS],
    shields: [Shield; SHIELDS],
    player_x: f32,
    sess: Session,
    march_timer: f32,
    march_dir: i32,
    march_drop_next: bool,
    bomb_timer: Timer,
    dead_timer: Timer,
    state: State,
    active_cols: i32,
    active_rows: i32,
    bomb_speed: f32,
    bomb_interval_range: (f32, f32),
    ufo_x: f32,
    ufo_active: bool,
    ufo_dir: i32,
    ufo_timer: Timer,
    ufo_bomb_timer: Timer,
    ufo_score: i32,
    ufo_score_timer: Timer,
}

impl Game {
    fn new() -> Self {
        let alien_default = Alien { x: 0.0, y: 0.0, alive: false, kind: 0, anim: 0 };
        let bullet_default = Bullet { x: 0.0, y: 0.0, active: false, player: false };
        let expl_default = Explosion { x: 0.0, y: 0.0, ttl: 0.0, active: false };
        let shield_default = Shield {
            x: 0.0, y: 0.0,
            alive: [[false; SHIELD_COLS]; SHIELD_ROWS],
        };
        Self {
            aliens: [alien_default; ALIEN_TOTAL],
            bullets: [bullet_default; N_BULLETS],
            explosions: [expl_default; N_EXPLOSIONS],
            shields: [shield_default; SHIELDS],
            player_x: 0.0,
            sess: Session::new(web::GAME_GALACTIC_DEFENDER, LIVES_START),
            march_timer: 0.0,
            march_dir: 1,
            march_drop_next: false,
            bomb_timer: Timer::default(),
            dead_timer: Timer::default(),
            state: State::Title,
            active_cols: 11,
            active_rows: 5,
            bomb_speed: 140.0,
            bomb_interval_range: (0.8, 2.5),
            ufo_x: 0.0,
            ufo_active: false,
            ufo_dir: 1,
            ufo_timer: Timer::default(),
            ufo_bomb_timer: Timer::default(),
            ufo_score: 0,
            ufo_score_timer: Timer::default(),
        }
    }

    fn aliens_alive(&self) -> i32 {
        self.aliens.iter().filter(|a| a.alive).count() as i32
    }

    fn march_interval(&self) -> f32 {
        let alive = self.aliens_alive();
        if alive <= 0 { return MARCH_MIN as f32; }
        let theme_total = (self.active_cols * self.active_rows).max(1);
        // Each level the march is 7% faster, capping at a 50% speedup by level 8.
        let level_scale = (1.0 - (self.sess.level - 1) as f32 * 0.07).max(0.5);
        let ms = (MARCH_START as f32 * alive as f32 / theme_total as f32 * level_scale) as i32;
        ms.max(MARCH_MIN) as f32
    }

    fn spawn_explosion(&mut self, x: f32, y: f32) {
        for e in self.explosions.iter_mut() {
            if !e.active {
                *e = Explosion { x, y, ttl: EXPLOSION_TTL, active: true };
                return;
            }
        }
    }

    fn free_bullet(&mut self, player: bool) -> Option<usize> {
        let (start, end) = if player {
            (0, MAX_PLAYER_BULLETS)
        } else {
            (MAX_PLAYER_BULLETS, MAX_PLAYER_BULLETS + MAX_BOMBS)
        };
        for i in start..end {
            if !self.bullets[i].active { return Some(i); }
        }
        None
    }

    fn build_shields(&mut self) {
        let total_w = SHIELDS as i32 * SHIELD_COLS as i32 * SHIELD_BLOCK
            + (SHIELDS as i32 - 1) * 40;
        let sx = (WIN_W - total_w) / 2;
        for s in 0..SHIELDS {
            self.shields[s].x = (sx + s as i32 * (SHIELD_COLS as i32 * SHIELD_BLOCK + 40)) as f32;
            self.shields[s].y = (GROUND_Y - 80) as f32;
            for r in 0..SHIELD_ROWS {
                for c in 0..SHIELD_COLS {
                    self.shields[s].alive[r][c] = true;
                }
            }
        }
    }

    fn init_aliens(&mut self) {
        let theme = (self.sess.level - 1).rem_euclid(3);
        let (rows, cols) = match theme {
            0 => (5_i32, 11_i32),
            1 => (6,     11),
            _ => (3,     13),
        };
        self.bomb_speed = match theme { 0 => 160.0, 1 => 200.0, _ => 250.0 };
        // Bomb intervals tighten each level (capped at level 6 equivalent).
        let lv = ((self.sess.level - 1) as f32).min(5.0);
        self.bomb_interval_range = match theme {
            0 => ((0.6 - lv * 0.04).max(0.3),  (2.0 - lv * 0.15).max(0.9)),
            1 => ((0.5 - lv * 0.03).max(0.25), (1.5 - lv * 0.12).max(0.75)),
            _ => ((0.35 - lv * 0.02).max(0.2), (1.0 - lv * 0.08).max(0.55)),
        };
        self.active_cols = cols;
        self.active_rows = rows;

        for a in self.aliens.iter_mut() { a.alive = false; }

        let grid_w = cols * (ALIEN_W + ALIEN_XGAP) - ALIEN_XGAP;
        let ox = (WIN_W - grid_w) / 2;
        let oy = PLAY_Y + 40;
        for r in 0..rows {
            let kind: usize = match theme {
                0 => [0, 1, 1, 2, 2][r as usize],
                1 => [0, 1, 1, 2, 2, 2][r as usize],
                _ => [0, 1, 2][r as usize],
            };
            for c in 0..cols {
                let i = (r * cols + c) as usize;
                self.aliens[i] = Alien {
                    x: (ox + c * (ALIEN_W + ALIEN_XGAP)) as f32,
                    y: (oy + r * (ALIEN_H + ALIEN_YGAP)) as f32,
                    alive: true,
                    kind,
                    anim: 0,
                };
            }
        }
        self.march_dir = 1;
        self.march_drop_next = false;
        self.march_timer = 0.0;
    }

    fn start_round_common(&mut self) {
        self.player_x = ((WIN_W - ALIEN_W) / 2) as f32;
        self.bullets.iter_mut().for_each(|b| b.active = false);
        self.explosions.iter_mut().for_each(|e| e.active = false);
        self.init_aliens();
        let theme = (self.sess.level - 1).rem_euclid(3);
        if theme != 2 {
            self.build_shields();
        } else {
            for s in &mut self.shields {
                for row in &mut s.alive { row.fill(false); }
            }
        }
        self.bomb_timer.start(2.0);
        self.ufo_active = false;
        self.ufo_timer.start(rand_int(15, 25) as f32);
        self.ufo_score_timer = Timer::default();
        self.ufo_bomb_timer = Timer::default();
        self.bullets[UFO_BOMB_IDX].active = false;
        self.state = State::Play;
    }

    fn start_game(&mut self) {
        self.sess.reset(LIVES_START);
        self.start_round_common();
    }
}

struct Sounds {
    shoot: blip::BlipSound,
    explosion: blip::BlipSound,
    level_clear: blip::BlipSound,
}

fn update_ufo(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.ufo_score_timer.tick(dt);

    if !g.ufo_active {
        if g.ufo_timer.tick(dt) && g.aliens_alive() > 0 {
            g.ufo_dir = if (rand() & 1) == 0 { 1 } else { -1 };
            g.ufo_x = if g.ufo_dir == 1 { -32.0 } else { WIN_W as f32 };
            g.ufo_active = true;
            g.ufo_bomb_timer.start(3.0);
            g.bullets[UFO_BOMB_IDX].active = false;
        }
        return;
    }

    const UFO_Y: f32 = (PLAY_Y + 8) as f32;
    g.ufo_x += 80.0 * g.ufo_dir as f32 * dt;

    if g.ufo_x > WIN_W as f32 || g.ufo_x + 32.0 < 0.0 {
        g.ufo_active = false;
        g.bullets[UFO_BOMB_IDX].active = false;
        g.ufo_timer.start(rand_int(15, 25) as f32);
        return;
    }

    // Only one bomb per pass — timer stays inactive after first fire.
    if g.ufo_bomb_timer.tick(dt) && !g.bullets[UFO_BOMB_IDX].active {
        g.bullets[UFO_BOMB_IDX] = Bullet {
            x: g.ufo_x + 12.0,
            y: UFO_Y + 12.0,
            active: true,
            player: false,
        };
    }

    for bi in 0..MAX_PLAYER_BULLETS {
        if !g.bullets[bi].active { continue; }
        if rects_overlap(g.bullets[bi].x, g.bullets[bi].y, 8.0, 16.0,
                         g.ufo_x, UFO_Y, 32.0, 12.0) {
            play_sfx(&sfx.explosion);
            let kill_x = g.ufo_x;
            g.spawn_explosion(kill_x, UFO_Y);
            g.bullets[bi].active = false;
            g.bullets[UFO_BOMB_IDX].active = false;
            let bonus = rand_int(1, 6) * 50;
            g.sess.add_score(bonus);
            g.ufo_score = bonus;
            g.ufo_score_timer.start(1.5);
            g.ufo_active = false;
            g.ufo_timer.start(rand_int(15, 25) as f32);
            return;
        }
    }
}

fn draw_ufo(blip: &Blip, g: &Game) {
    const UFO_Y: f32 = (PLAY_Y + 8) as f32;
    if g.ufo_active {
        blip.fill_rect(g.ufo_x, UFO_Y, 32.0, 12.0, BLIP_RED);
        blip.fill_rect(g.ufo_x + 8.0, UFO_Y - 8.0, 16.0, 8.0, BLIP_CYAN);
    }
    if g.ufo_score_timer.active() {
        let text = format!("{} PTS", g.ufo_score);
        blip.draw_centered(&text, UFO_Y, 2.0, BLIP_YELLOW);
    }
}

fn update_title(g: &mut Game) {
    g.sess.refresh_hi();
    if any_key_pressed() { g.start_game(); }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.sess.refresh_hi();
    let shoot = key_pressed(BLIP_KEY_SPACE)
        || key_pressed(BLIP_KEY_UP)
        || key_pressed(BLIP_KEY_W);

    let ps = PLAYER_SPEED * dt;
    if key_held(BLIP_KEY_LEFT)  || key_held(BLIP_KEY_A) { g.player_x -= ps; }
    if key_held(BLIP_KEY_RIGHT) || key_held(BLIP_KEY_D) { g.player_x += ps; }
    g.player_x = clamp(g.player_x, 0.0, (WIN_W - ALIEN_W) as f32);

    if shoot {
        if let Some(i) = g.free_bullet(true) {
            play_sfx(&sfx.shoot);
            g.bullets[i] = Bullet {
                x: g.player_x + (ALIEN_W / 2 - 4) as f32,
                y: (GROUND_Y - 28) as f32,
                active: true,
                player: true,
            };
        }
    }

    for b in g.bullets.iter_mut() {
        if !b.active { continue; }
        b.y += if b.player { -BULLET_SPEED } else { g.bomb_speed } * dt;
        if b.y < PLAY_Y as f32 || b.y > WIN_H as f32 { b.active = false; }
    }

    g.march_timer += dt * 1000.0;
    if g.march_timer >= g.march_interval() {
        g.march_timer = 0.0;
        if g.march_drop_next {
            for a in g.aliens.iter_mut() {
                if a.alive { a.y += MARCH_DROP; }
            }
            g.march_dir = -g.march_dir;
            g.march_drop_next = false;
        } else {
            let step = (ALIEN_W / 3) as f32;
            let mut hit_edge = false;
            for a in g.aliens.iter_mut() {
                if !a.alive { continue; }
                a.x += step * g.march_dir as f32;
                a.anim ^= 1;
                if a.x < 2.0 || a.x + ALIEN_W as f32 > (WIN_W - 2) as f32 {
                    hit_edge = true;
                }
            }
            if hit_edge { g.march_drop_next = true; }
        }
    }

    if g.bomb_timer.tick(dt) {
        let r01 = (rand() as f32) / (u32::MAX as f32);
        let (lo, hi) = g.bomb_interval_range;
        g.bomb_timer.start(lerp(lo, hi, r01));
        let mut candidates = [0usize; ALIEN_COLS as usize];
        let mut nc = 0usize;
        for c in 0..g.active_cols {
            for r in (0..g.active_rows).rev() {
                let idx = (r * g.active_cols + c) as usize;
                if g.aliens[idx].alive {
                    candidates[nc] = idx;
                    nc += 1;
                    break;
                }
            }
        }
        if nc > 0 {
            if let Some(bi) = g.free_bullet(false) {
                let idx = candidates[rand_int(0, nc as i32 - 1) as usize];
                let a = g.aliens[idx];
                g.bullets[bi] = Bullet {
                    x: a.x + (ALIEN_W / 2 - 4) as f32,
                    y: a.y + ALIEN_H as f32,
                    active: true,
                    player: false,
                };
            }
        }
    }

    // Player bullet vs aliens
    for bi in 0..MAX_PLAYER_BULLETS {
        if !g.bullets[bi].active { continue; }
        for ai in 0..ALIEN_TOTAL {
            if !g.aliens[ai].alive { continue; }
            if rects_overlap(
                g.bullets[bi].x, g.bullets[bi].y, 8.0, 16.0,
                g.aliens[ai].x, g.aliens[ai].y, ALIEN_W as f32, ALIEN_H as f32,
            ) {
                play_sfx(&sfx.explosion);
                let (ax, ay, kind) = (g.aliens[ai].x, g.aliens[ai].y, g.aliens[ai].kind);
                g.spawn_explosion(ax, ay);
                g.aliens[ai].alive = false;
                g.bullets[bi].active = false;
                let pts = match kind { 0 => 30, 1 => 20, _ => 10 };
                g.sess.add_score(pts * g.sess.level);
                break;
            }
        }
    }

    // Bullets vs shields
    for bi in 0..N_BULLETS {
        if !g.bullets[bi].active { continue; }
        'outer: for s in 0..SHIELDS {
            for r in 0..SHIELD_ROWS {
                for c in 0..SHIELD_COLS {
                    if !g.shields[s].alive[r][c] { continue; }
                    let bx = g.shields[s].x + (c as i32 * SHIELD_BLOCK) as f32;
                    let by = g.shields[s].y + (r as i32 * SHIELD_BLOCK) as f32;
                    if rects_overlap(
                        g.bullets[bi].x, g.bullets[bi].y, 8.0, 16.0,
                        bx, by, SHIELD_BLOCK as f32, SHIELD_BLOCK as f32,
                    ) {
                        g.shields[s].alive[r][c] = false;
                        g.bullets[bi].active = false;
                        break 'outer;
                    }
                }
            }
        }
    }

    // Bombs vs player
    for bi in MAX_PLAYER_BULLETS..N_BULLETS {
        if !g.bullets[bi].active { continue; }
        if rects_overlap(
            g.bullets[bi].x, g.bullets[bi].y, 8.0, 16.0,
            g.player_x, (GROUND_Y - 28) as f32, ALIEN_W as f32, 28.0,
        ) {
            g.bullets[bi].active = false;
            let px = g.player_x;
            g.spawn_explosion(px, (GROUND_Y - 28) as f32);
            play_sfx(&sfx.explosion);
            match g.sess.lose_life() {
                LifeResult::StillAlive => {
                    for k in MAX_PLAYER_BULLETS..N_BULLETS {
                        g.bullets[k].active = false;
                    }
                    g.dead_timer.start(1.5);
                    g.state = State::Dead;
                }
                LifeResult::GameOver => {
                    g.state = State::Over;
                }
            }
            return;
        }
    }

    for a in g.aliens.iter() {
        if a.alive && a.y + ALIEN_H as f32 >= GROUND_Y as f32 {
            g.state = State::Over;
            return;
        }
    }

    update_ufo(g, dt, sfx);

    if g.aliens_alive() == 0 {
        play_sfx(&sfx.level_clear);
        g.sess.next_level();
        g.dead_timer.start(1.5);
        g.state = State::Win;
    }

    for e in g.explosions.iter_mut() {
        if e.active {
            e.ttl -= dt;
            if e.ttl <= 0.0 { e.active = false; }
        }
    }
}

fn update_dead(g: &mut Game, dt: f32) {
    if g.dead_timer.tick(dt) {
        g.bullets.iter_mut().for_each(|b| b.active = false);
        g.state = State::Play;
    }
}

fn update_win(g: &mut Game, dt: f32) {
    if g.dead_timer.tick(dt) { g.start_round_common(); }
}

fn update_over(g: &mut Game) {
    g.sess.refresh_hi();
    g.sess.notify_game_over();
    if !any_key_pressed() { return; }
    web::spend_coin();
    g.start_game();
}

fn draw_play(blip: &Blip, g: &Game,
             player: &Texture2D, alien: &[Texture2D; 3],
             explosion: &Texture2D, shield: &Texture2D) {
    blip.draw_line(0.0, GROUND_Y as f32, WIN_W as f32, GROUND_Y as f32, BLIP_GREEN);

    for s in 0..SHIELDS {
        for r in 0..SHIELD_ROWS {
            for c in 0..SHIELD_COLS {
                if !g.shields[s].alive[r][c] { continue; }
                blip.draw_texture(
                    shield,
                    g.shields[s].x + (c as i32 * SHIELD_BLOCK) as f32,
                    g.shields[s].y + (r as i32 * SHIELD_BLOCK) as f32,
                    SHIELD_BLOCK as f32, SHIELD_BLOCK as f32,
                );
            }
        }
    }

    let dim = BlipColor { r: 180.0/255.0, g: 180.0/255.0, b: 180.0/255.0, a: 1.0 };
    for a in g.aliens.iter() {
        if !a.alive { continue; }
        let tint = if a.anim != 0 { dim } else { BLIP_WHITE };
        blip.draw_texture_tinted(
            &alien[a.kind],
            a.x, a.y, ALIEN_W as f32, ALIEN_H as f32, tint,
        );
    }

    blip.draw_texture(player, g.player_x, (GROUND_Y - 28) as f32,
                      ALIEN_W as f32, 28.0);

    for b in g.bullets.iter() {
        if !b.active { continue; }
        let c = if b.player { BLIP_WHITE } else { BLIP_ORANGE };
        blip.fill_rect(b.x, b.y, 4.0, 12.0, c);
    }

    for e in g.explosions.iter() {
        if !e.active { continue; }
        let alpha = e.ttl / EXPLOSION_TTL;
        let tc = BlipColor { r: 1.0, g: 1.0, b: 1.0, a: alpha };
        blip.draw_texture_tinted(
            explosion,
            e.x, e.y, ALIEN_W as f32, ALIEN_W as f32, tc,
        );
    }

    draw_ufo(blip, g);
    blip.draw_hud(g.sess.score, g.sess.hi, g.sess.lives);
}

fn draw_title(blip: &Blip, alien: &[Texture2D; 3]) {
    blip.clear(BLIP_BLACK);
    blip.draw_centered("GALACTIC", (WIN_H / 5) as f32,        5.0, BLIP_CYAN);
    blip.draw_centered("DEFENDER", (WIN_H / 5 + 50) as f32,   5.0, BLIP_MAGENTA);

    let dw = (ALIEN_W / 2) as f32;
    let dh = (ALIEN_H / 2) as f32;
    let ax = (blip.text_cx("30 PTS", 2) - ALIEN_W / 2 - 8) as f32;
    let voff = ((7 * 2 - ALIEN_H / 2) / 2) as f32;

    let row0 = (WIN_H / 2 - 40) as f32;
    let row1 = (WIN_H / 2 - 20) as f32;
    let row2 = (WIN_H / 2) as f32;

    blip.draw_texture_tinted(&alien[0], ax, row0 + voff, dw, dh, BLIP_MAGENTA);
    blip.draw_texture_tinted(&alien[1], ax, row1 + voff, dw, dh, BLIP_CYAN);
    blip.draw_texture_tinted(&alien[2], ax, row2 + voff, dw, dh, BLIP_GREEN);

    blip.draw_centered("30 PTS",        row0,                 2.0, BLIP_MAGENTA);
    blip.draw_centered("20 PTS",        row1,                 2.0, BLIP_CYAN);
    blip.draw_centered("10 PTS",        row2,                 2.0, BLIP_GREEN);
    blip.draw_centered("PRESS ANY KEY", (WIN_H * 2 / 3) as f32, 3.0, BLIP_WHITE);
}

fn draw_win(blip: &Blip, level: i32) {
    let buf = format!("LEVEL {}", level);
    blip.clear(BLIP_BLACK);
    blip.draw_centered("WAVE CLEAR", (WIN_H / 3) as f32, 4.0, BLIP_CYAN);
    blip.draw_centered(&buf,         (WIN_H / 2) as f32, 3.0, BLIP_YELLOW);
}

fn draw_over(blip: &Blip, score: i32) {
    let buf = format!("SCORE {}", score);
    blip.clear(BLIP_BLACK);
    blip.draw_centered("GAME OVER",     (WIN_H / 4) as f32,     5.0, BLIP_RED);
    blip.draw_centered(&buf,            (WIN_H / 2) as f32,     3.0, BLIP_WHITE);
    blip.draw_centered("PRESS ANY KEY", (WIN_H * 2 / 3) as f32, 3.0, BLIP_YELLOW);
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("GALACTIC DEFENDER", WIN_W, WIN_H)
}

const PLAYER_SHIP_PNG:  &[u8] = blip_image!("player_ship.png");
const ALIEN_SQUID_PNG:  &[u8] = blip_image!("alien_squid.png");
const ALIEN_CRAB_PNG:   &[u8] = blip_image!("alien_crab.png");
const ALIEN_OCTO_PNG:   &[u8] = blip_image!("alien_octopus.png");
const EXPLOSION_PNG:    &[u8] = blip_image!("explosion.png");
const SHIELD_PNG:       &[u8] = blip_image!("shield_block.png");
const SHOOT_WAV:        &[u8] = blip_sound!("shoot.wav");
const EXPLOSION_WAV:    &[u8] = blip_sound!("explosion.wav");
const LEVEL_CLEAR_WAV:  &[u8] = blip_sound!("level_clear.wav");
const MUSIC_WAV:        &[u8] = blip_sound!("music.wav");
const MUSIC2_WAV:       &[u8] = blip_sound!("music2.wav");
const MUSIC3_WAV:       &[u8] = blip_sound!("music3.wav");

// Loop durations in seconds — used to switch tracks at loop boundaries.
// music: 9.6s  music2: 5.6s  music3: 8.0s
const MUSIC_DURATIONS: [f32; 3] = [9.6, 5.6, 8.0];

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let player = load_png(PLAYER_SHIP_PNG);
    let alien = [
        load_png(ALIEN_SQUID_PNG),
        load_png(ALIEN_CRAB_PNG),
        load_png(ALIEN_OCTO_PNG),
    ];
    let explosion = load_png(EXPLOSION_PNG);
    let shield = load_png(SHIELD_PNG);

    let sfx = Sounds {
        shoot:       blip::audio::load_sound(SHOOT_WAV).await,
        explosion:   blip::audio::load_sound(EXPLOSION_WAV).await,
        level_clear: blip::audio::load_sound(LEVEL_CLEAR_WAV).await,
    };
    let tracks = [
        blip::audio::load_sound(MUSIC_WAV).await,
        blip::audio::load_sound(MUSIC2_WAV).await,
        blip::audio::load_sound(MUSIC3_WAV).await,
    ];
    let mut music = MusicTracks::start(&tracks);
    let mut music_timer: f32 = MUSIC_DURATIONS[0];

    loop {
        let dt = blip.delta_time;

        // Switch to a random different loop at each loop boundary.
        music_timer -= dt;
        if music_timer <= 0.0 {
            let cur = music.current();
            let candidate = rand_int(0, 1) as usize; // 0 or 1
            let next = if candidate < cur { candidate } else { candidate + 1 };
            music.switch_to(next);
            music_timer = MUSIC_DURATIONS[next];
        }
        match g.state {
            State::Title => update_title(&mut g),
            State::Play  => update_play(&mut g, dt, &sfx),
            State::Dead  => update_dead(&mut g, dt),
            State::Win   => update_win(&mut g, dt),
            State::Over  => update_over(&mut g),
        }

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title => draw_title(&blip, &alien),
            State::Win   => draw_win(&blip, g.sess.level),
            State::Over  => draw_over(&blip, g.sess.score),
            State::Play | State::Dead => {
                draw_play(&blip, &g, &player, &alien, &explosion, &shield);
            }
        }

        blip.next_frame(60).await;
    }
}
