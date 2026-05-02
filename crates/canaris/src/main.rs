//! Canaris – privateer arcade game inspired by Kaptajn Kaper i Kattegat (1985).

use blip::input::{any_key_pressed, key_held, key_pressed, BLIP_KEY_DOWN, BLIP_KEY_LEFT,
                   BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
                   BLIP_KEY_A, BLIP_KEY_D};
use blip::macroquad::prelude::{Color, ImageFormat};
use blip::macroquad::texture::{FilterMode, Texture2D};
use blip::macroquad::input::KeyCode;
use blip::{
    clamp, play_ambient, play_music, play_sfx, rand_int, rects_overlap, web, window_conf, Blip,
    BLIP_BLACK, BLIP_CYAN, BLIP_DARKGRAY, BLIP_WHITE, BLIP_YELLOW, BLIP_RED, BLIP_GREEN,
    BLIP_GRAY, BLIP_ORANGE, BlipColor,
};

// ── layout ────────────────────────────────────────────────────────────────────

const WIN_W: i32 = 480;
const WIN_H: i32 = 540;
const HUD_H: i32 = 28;

const WORLD_W: f32 = WIN_W as f32 * 4.0;

// ── entity sizes ──────────────────────────────────────────────────────────────

const PLAYER_W: f32 = 48.0;
const PLAYER_H: f32 = 32.0;
const ENEMY_W:  f32 = 48.0;
const ENEMY_H:  f32 = 32.0;
const BALL_W:   f32 = 8.0;
const BALL_H:   f32 = 8.0;

// ── pools ─────────────────────────────────────────────────────────────────────

const MAX_ENEMIES:     usize = 4;
const MAX_CANNONBALLS: usize = 8;  // first 2 = player, rest = enemy
const MAX_EXPLOSIONS:  usize = 12;
const BOARDING_SLOTS:  usize = 6;

// ── tuning ────────────────────────────────────────────────────────────────────

const LIVES_START:       i32   = 3;
const PLAYER_HULL_MAX:   i32   = 20;
const PLAYER_CREW_START: i32   = 8;
const PLAYER_GOLD_START: i32   = 50;
const PLAYER_CANNONS:    i32   = 6;
const PLAYER_FOOD_START: i32   = 30;
const FOOD_MAX:          i32   = 30;

const PLAYER_SPEED:      f32   = 160.0;
const SEA_LANE_Y:        f32   = (WIN_H as f32 - HUD_H as f32) * 0.38 + HUD_H as f32;
const BOB_AMP:           f32   = 4.0;
const BOB_FREQ:          f32   = 1.8;

const ENGAGEMENT_DIST:   f32   = WIN_W as f32 * 0.6;
const PORT_ANCHOR_X:     f32   = WORLD_W * 0.75;
const PORT_DOCK_RADIUS:  f32   = 80.0;  // how close the player must be to press SPACE and dock
const PORT_SAFE_RADIUS:  f32   = 400.0; // enemies don't spawn or engage within this distance of port

const CANNON_SPEED:      f32   = 280.0;
const CANNON_GRAVITY:    f32   = 60.0;
const PLAYER_RELOAD:     f32   = 0.8;
const ENEMY_RELOAD_BASE: f32   = 2.0;
const COMBAT_PLAYER_X:   f32   = 60.0;
const COMBAT_ENEMY_X:    f32   = WIN_W as f32 - 60.0 - ENEMY_W;
const COMBAT_BASE_Y:     f32   = WIN_H as f32 * 0.42;
const RETREAT_TIMER:     f32   = 20.0;
const DODGE_SPEED:       f32   = 140.0;
const COMBAT_Y_MIN:      f32   = HUD_H as f32 + 10.0;
const COMBAT_Y_MAX:      f32   = WIN_H as f32 - ENEMY_H - 52.0; // leaves room for bottom UI
const BOARD_Y_DIST:      f32   = 55.0;  // Y proximity required to trigger boarding
const COMBAT_UI_Y:       f32   = WIN_H as f32 - 46.0; // top of bottom UI strip

const BOARDING_TICK:     f32   = 1.2;
const BOARDING_TIMEOUT:  f32   = 30.0;

const EXPLOSION_TTL:     f32   = 0.5;
const DEAD_TTL:          f32   = 1.8;
const ANIM_FRAME_DUR:    f32   = 0.35;
const FOOD_DECAY_RATE:   f32   = 0.5; // points/sec
const FOOD_HULL_DMG_RATE:f32   = 1.5; // seconds between hull ticks when starving

const REPAIR_COST:  i32 = 30;
const CREW_COST:    i32 = 20;
const CANNON_COST:  i32 = 25;
const FOOD_COST:    i32 = 15;

const KEY_BOARD: KeyCode = KeyCode::E;
const KEY_ENTER: KeyCode = KeyCode::Enter;

// ── state machine ─────────────────────────────────────────────────────────────

struct MapZone {
    name:     &'static str,
    desc:     &'static str,
    level_eq: i32,
    ships:    usize,
    map_x:    f32,
    map_y:    f32,
    stars:    u8,
}

const ZONES: [MapZone; 4] = [
    MapZone { name: "DANISH COAST",      desc: "Soft targets on shallow routes", level_eq: 1,  ships: 2, map_x: 190.0, map_y: 420.0, stars: 1 },
    MapZone { name: "KATTEGAT NARROWS",  desc: "Armed merchant convoys",         level_eq: 4,  ships: 3, map_x: 200.0, map_y: 290.0, stars: 2 },
    MapZone { name: "SKAGERRAK PASSAGE", desc: "Royal Navy patrol routes",       level_eq: 7,  ships: 4, map_x: 210.0, map_y: 165.0, stars: 3 },
    MapZone { name: "OPEN NORTH SEA",    desc: "Men-of-war and armed galleons",  level_eq: 11, ships: 4, map_x: 220.0, map_y:  60.0, stars: 4 },
];

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Sea, Combat, Boarding, Port, Map, Dead, GameOver }

// ── entities ──────────────────────────────────────────────────────────────────

#[derive(Copy, Clone)]
struct PlayerShip {
    world_x: f32,
    y:       f32,
    vx:      f32,
    hull:    i32,
    crew:    i32,
    gold:    i32,
    cannons: i32,
    food:    f32,
    reload_t: f32,
    anim_frame: u8,
    anim_t:  f32,
    hit_flash_t: f32,
    starve_t: f32,  // timer between starvation hull ticks
}

#[derive(Copy, Clone)]
struct EnemyShip {
    active:   bool,
    world_x:  f32,
    y:        f32,
    hull:     i32,
    hull_max: i32,
    crew:     i32,
    gold_loot:i32,
    reload_t: f32,
    anim_frame: u8,
    anim_t:  f32,
    hit_flash_t: f32,
    // combat-screen position (screen space)
    combat_y: f32,
    combat_vy: f32,
}

#[derive(Copy, Clone)]
struct Cannonball {
    active: bool,
    x: f32, y: f32,
    vx: f32, vy: f32,
    player: bool,
}

#[derive(Copy, Clone)]
struct Explosion {
    active: bool,
    x: f32, y: f32,
    ttl: f32,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum SlotOwner { Player, Enemy, Empty }

#[derive(Copy, Clone)]
struct BoardingSlot {
    owner: SlotOwner,
    hp:    i32,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum PortItem { Sail, Map, Repair, Crew, Cannons, Food }

impl PortItem {
    // Order: Sail → Map → Repair → Crew → Cannons → Food → (wrap) Sail
    fn next(self) -> Self {
        match self {
            PortItem::Sail    => PortItem::Map,
            PortItem::Map     => PortItem::Repair,
            PortItem::Repair  => PortItem::Crew,
            PortItem::Crew    => PortItem::Cannons,
            PortItem::Cannons => PortItem::Food,
            PortItem::Food    => PortItem::Sail,
        }
    }
    fn prev(self) -> Self {
        match self {
            PortItem::Sail    => PortItem::Food,
            PortItem::Map     => PortItem::Sail,
            PortItem::Repair  => PortItem::Map,
            PortItem::Crew    => PortItem::Repair,
            PortItem::Cannons => PortItem::Crew,
            PortItem::Food    => PortItem::Cannons,
        }
    }
    fn label(self) -> &'static str {
        match self {
            PortItem::Sail    => "SET SAIL",
            PortItem::Map     => "WORLD MAP",
            PortItem::Repair  => "REPAIR HULL",
            PortItem::Crew    => "HIRE CREW",
            PortItem::Cannons => "BUY CANNONS",
            PortItem::Food    => "BUY PROVISIONS",
        }
    }
    fn cost(self) -> i32 {
        match self {
            PortItem::Sail    => 0,
            PortItem::Map     => 0,
            PortItem::Repair  => REPAIR_COST,
            PortItem::Crew    => CREW_COST,
            PortItem::Cannons => CANNON_COST,
            PortItem::Food    => FOOD_COST,
        }
    }
}

// ── game ──────────────────────────────────────────────────────────────────────

#[allow(dead_code)]
struct Game {
    state:   State,
    time:    f32,

    player:  PlayerShip,
    cam_x:   f32,

    enemies: [EnemyShip; MAX_ENEMIES],
    cannonballs: [Cannonball; MAX_CANNONBALLS],
    explosions:  [Explosion; MAX_EXPLOSIONS],

    combat_enemy_idx: usize,
    retreat_t: f32,

    slots:           [BoardingSlot; BOARDING_SLOTS],
    boarding_t:      f32,
    boarding_total_t:f32,
    boarding_hit_slot: usize,  // index of last attacked slot (99 = none)
    boarding_hit_t:  f32,      // flash timer for that slot

    port_cursor:   PortItem,
    port_msg:      &'static str,
    port_msg_t:    f32,
    port_msg_ok:   bool,
    map_cursor:    usize,

    score:    i32,
    hi_score: i32,
    lives:    i32,
    level:    i32,
    level_t:  f32,
    dead_t:   f32,
}

impl Game {
    fn new() -> Self {
        Game {
            state:   State::Title,
            time:    0.0,
            player:  PlayerShip {
                world_x: 40.0, y: SEA_LANE_Y,
                vx: 0.0, hull: PLAYER_HULL_MAX, crew: PLAYER_CREW_START,
                gold: PLAYER_GOLD_START, cannons: PLAYER_CANNONS,
                food: PLAYER_FOOD_START as f32,
                reload_t: 0.0, anim_frame: 0, anim_t: 0.0,
                hit_flash_t: 0.0, starve_t: 0.0,
            },
            cam_x:   0.0,
            enemies: [EnemyShip {
                active: false, world_x: 0.0, y: SEA_LANE_Y,
                hull: 0, hull_max: 0, crew: 0, gold_loot: 0,
                reload_t: 0.0, anim_frame: 0, anim_t: 0.0, hit_flash_t: 0.0,
                combat_y: COMBAT_BASE_Y, combat_vy: 0.0,
            }; MAX_ENEMIES],
            cannonballs: [Cannonball { active: false, x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, player: false }; MAX_CANNONBALLS],
            explosions:  [Explosion { active: false, x: 0.0, y: 0.0, ttl: 0.0 }; MAX_EXPLOSIONS],
            combat_enemy_idx: 0,
            retreat_t: 0.0,
            slots: [BoardingSlot { owner: SlotOwner::Empty, hp: 0 }; BOARDING_SLOTS],
            boarding_t: 0.0,
            boarding_total_t: 0.0,
            boarding_hit_slot: 99,
            boarding_hit_t: 0.0,
            port_cursor: PortItem::Sail,
            port_msg: "",
            port_msg_t: 0.0,
            port_msg_ok: true,
            map_cursor: 0,
            score: 0, hi_score: 0,
            lives: LIVES_START,
            level: 1, level_t: 60.0,
            dead_t: 0.0,
        }
    }

    fn start_game(&mut self) {
        let hi = self.hi_score;
        *self = Game::new();
        self.hi_score = hi;
        self.state = State::Sea;
        self.spawn_enemies();
    }

    fn spawn_enemies(&mut self) {
        let hull_base = 6 + self.level * 2;
        for i in 0..MAX_ENEMIES {
            // Reject positions inside the port safe zone so enemies never block the harbour.
            let wx = loop {
                let x = WORLD_W * 0.3 + rand_int(0, (WORLD_W * 0.5) as i32) as f32;
                if (x - PORT_ANCHOR_X).abs() > PORT_SAFE_RADIUS { break x; }
            };
            self.enemies[i] = EnemyShip {
                active:   true,
                world_x:  wx,
                y:        SEA_LANE_Y + rand_int(-20, 20) as f32,
                hull:     hull_base,
                hull_max: hull_base,
                crew:     2 + self.level,
                gold_loot:20 + rand_int(0, 30) as i32,
                reload_t: rand_int(10, 30) as f32 * 0.1,
                anim_frame: 0, anim_t: 0.0, hit_flash_t: 0.0,
                combat_y: COMBAT_BASE_Y,
                combat_vy: 0.0,
            };
        }
    }

    fn spawn_enemies_n(&mut self, n: usize) {
        let hull_base = 6 + self.level * 2;
        for i in 0..MAX_ENEMIES {
            if i < n {
                let wx = loop {
                    let x = WORLD_W * 0.3 + rand_int(0, (WORLD_W * 0.5) as i32) as f32;
                    if (x - PORT_ANCHOR_X).abs() > PORT_SAFE_RADIUS { break x; }
                };
                self.enemies[i] = EnemyShip {
                    active: true, world_x: wx,
                    y: SEA_LANE_Y + rand_int(-10, 10) as f32,
                    hull: hull_base, hull_max: hull_base,
                    crew: 2 + self.level,
                    gold_loot: 20 + rand_int(0, 30) as i32 * self.level,
                    reload_t: rand_int(10, 30) as f32 * 0.1,
                    anim_frame: 0, anim_t: 0.0, hit_flash_t: 0.0,
                    combat_y: COMBAT_BASE_Y, combat_vy: 0.0,
                };
            } else {
                self.enemies[i].active = false;
            }
        }
    }

    fn spawn_explosion(&mut self, x: f32, y: f32) {
        for e in self.explosions.iter_mut() {
            if !e.active {
                *e = Explosion { active: true, x, y, ttl: EXPLOSION_TTL };
                return;
            }
        }
    }

    fn fire_ball(&mut self, from_player: bool, src_x: f32, src_y: f32) {
        let (start, end) = if from_player { (0, 2) } else { (2, MAX_CANNONBALLS) };
        for i in start..end {
            if !self.cannonballs[i].active {
                let vx = if from_player { CANNON_SPEED } else { -CANNON_SPEED };
                let vy = rand_int(-20, 20) as f32;
                self.cannonballs[i] = Cannonball {
                    active: true, x: src_x, y: src_y, vx, vy, player: from_player,
                };
                return;
            }
        }
    }

    fn respawn_at_sea(&mut self) {
        self.player.hull    = PLAYER_HULL_MAX / 2;
        self.player.world_x = 40.0;
        self.player.y       = SEA_LANE_Y;
        self.player.vx      = 0.0;
        self.cam_x          = 0.0;
        // clear projectiles
        for b in self.cannonballs.iter_mut() { b.active = false; }
        for e in self.explosions.iter_mut()  { e.active = false; }
        self.state = State::Sea;
    }

    fn enter_combat(&mut self, idx: usize) {
        self.combat_enemy_idx = idx;
        self.retreat_t = RETREAT_TIMER;
        self.player.y          = COMBAT_BASE_Y;
        self.player.reload_t   = 0.0;
        self.enemies[idx].combat_y  = COMBAT_BASE_Y;
        self.enemies[idx].combat_vy = 0.0;
        // clear stale projectiles
        for b in self.cannonballs.iter_mut() { b.active = false; }
        self.state = State::Combat;
    }

    fn enter_boarding(&mut self) {
        let idx = self.combat_enemy_idx;
        // Slots 0-2 = player side (left), slots 3-5 = enemy side (right).
        // HP scales with crew count so a well-crewed ship fights longer.
        let p_hp = (self.player.crew / 3).clamp(1, 3) as i32;
        let e_hp = (self.enemies[idx].crew / 2).clamp(1, 3) as i32;
        for i in 0..3 {
            self.slots[i] = BoardingSlot { owner: SlotOwner::Player, hp: p_hp };
        }
        for i in 3..BOARDING_SLOTS {
            self.slots[i] = BoardingSlot { owner: SlotOwner::Enemy, hp: e_hp };
        }
        self.boarding_t        = BOARDING_TICK;
        self.boarding_total_t  = BOARDING_TIMEOUT;
        self.boarding_hit_slot = 99;
        self.boarding_hit_t    = 0.0;
        self.state             = State::Boarding;
    }

    fn enter_port(&mut self) {
        self.port_cursor = PortItem::Sail;
        self.port_msg    = "WELCOME TO PORT";
        self.port_msg_t  = 2.0;
        self.state       = State::Port;
    }
}

// ── assets ────────────────────────────────────────────────────────────────────

#[allow(dead_code)]
struct Sounds {
    cannon_fire:    blip::audio::BlipSound,
    explosion:      blip::audio::BlipSound,
    splash:         blip::audio::BlipSound,
    hull_hit:       blip::audio::BlipSound,
    boarding_clash: blip::audio::BlipSound,
    coin_jingle:    blip::audio::BlipSound,
    life_lost:      blip::audio::BlipSound,
    sea_music:      blip::audio::BlipSound,
    combat_music:   blip::audio::BlipSound,
    port_music:     blip::audio::BlipSound,
    ocean_ambience: blip::audio::BlipSound,
}

struct Textures {
    player_a: Texture2D,
    player_b: Texture2D,
    enemy_a:  Texture2D,
    enemy_b:  Texture2D,
    ball:     Texture2D,
    explosion:Texture2D,
    port_bg:  Texture2D,
    sea_wave:   Texture2D,
    sea_wave_b: Texture2D,
    crew:       Texture2D,
    map_bg:     Texture2D,
}

// ── asset includes ────────────────────────────────────────────────────────────

const PLAYER_A_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/player_ship_a.png"));
const PLAYER_B_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/player_ship_b.png"));
const ENEMY_A_PNG:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/enemy_ship_a.png"));
const ENEMY_B_PNG:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/enemy_ship_b.png"));
const BALL_PNG:     &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/cannonball.png"));
const EXPLODE_PNG:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/explosion.png"));
const PORT_BG_PNG:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/port_bg.png"));
const SEA_WAVE_PNG:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/sea_wave.png"));
const SEA_WAVE_B_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/sea_wave_b.png"));
const CREW_PNG:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/crew_figure.png"));
const MAP_BG_PNG:     &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/kattegat_map.png"));

const CANNON_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/cannon_fire.wav"));
const EXPLODE_WAV:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/explosion.wav"));
const SPLASH_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/splash.wav"));
const HULL_HIT_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/hull_hit.wav"));
const CLASH_WAV:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/boarding_clash.wav"));
const COINS_WAV:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/coin_jingle.wav"));
const LIFE_WAV:     &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/life_lost.wav"));
const SEA_MUS_WAV:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/sea_music.wav"));
const COMBAT_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/combat_music.wav"));
const PORT_MUS_WAV:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/port_music.wav"));
const AMBIENT_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/ocean_ambience.wav"));

fn load_png(bytes: &'static [u8]) -> Texture2D {
    let tex = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
    tex.set_filter(FilterMode::Nearest);
    tex
}

// ── update ────────────────────────────────────────────────────────────────────

fn update_title(g: &mut Game, dt: f32) {
    g.time += dt;
    if any_key_pressed() {
        g.start_game();
    }
}

fn update_sea(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.time += dt;

    // Player movement
    if key_held(BLIP_KEY_RIGHT) || key_held(BLIP_KEY_D) {
        g.player.vx = clamp(g.player.vx + 300.0 * dt, -PLAYER_SPEED, PLAYER_SPEED);
    } else if key_held(BLIP_KEY_LEFT) || key_held(BLIP_KEY_A) {
        g.player.vx = clamp(g.player.vx - 300.0 * dt, -PLAYER_SPEED, PLAYER_SPEED);
    } else {
        g.player.vx *= 1.0 - 5.0 * dt;
    }
    g.player.world_x = (g.player.world_x + g.player.vx * dt).rem_euclid(WORLD_W);

    // Vertical bob
    g.player.y = SEA_LANE_Y + (g.time * BOB_FREQ).sin() * BOB_AMP;

    // Camera
    let target_cam = g.player.world_x - WIN_W as f32 * 0.35;
    g.cam_x = clamp(target_cam, 0.0, WORLD_W - WIN_W as f32);

    // Sprite animation
    g.player.anim_t += dt;
    if g.player.anim_t >= ANIM_FRAME_DUR {
        g.player.anim_t = 0.0;
        g.player.anim_frame ^= 1;
    }

    // Food decay
    g.player.food -= FOOD_DECAY_RATE * dt;
    if g.player.food < 0.0 {
        g.player.food = 0.0;
        g.player.starve_t -= dt;
        if g.player.starve_t <= 0.0 {
            g.player.starve_t = FOOD_HULL_DMG_RATE;
            g.player.hull -= 1;
        }
    } else {
        g.player.starve_t = FOOD_HULL_DMG_RATE;
    }

    // Hull flash
    if g.player.hit_flash_t > 0.0 { g.player.hit_flash_t -= dt; }

    // Level timer
    g.level_t -= dt;
    if g.level_t <= 0.0 {
        g.level += 1;
        g.level_t = 60.0 + g.level as f32 * 10.0;
        g.score += 500 * g.level;
        if g.score > g.hi_score { g.hi_score = g.score; }
        g.spawn_enemies();
    }

    // Enemy movement + engagement check
    let enemy_speed = 30.0 + g.level as f32 * 5.0;
    for i in 0..MAX_ENEMIES {
        if !g.enemies[i].active { continue; }
        g.enemies[i].world_x -= enemy_speed * dt;
        // Bob enemies gently
        g.enemies[i].y = SEA_LANE_Y - 8.0 + (g.time * BOB_FREQ * 0.9 + i as f32).sin() * BOB_AMP;
        if g.enemies[i].world_x < -ENEMY_W {
            g.enemies[i].world_x = WORLD_W + rand_int(200, 800) as f32;
        }

        // Check engagement — suppress near port so the player can always dock
        let near_port = (g.player.world_x - PORT_ANCHOR_X).abs() < PORT_SAFE_RADIUS;
        let dist = (g.enemies[i].world_x - g.player.world_x).abs();
        if !near_port && dist < ENGAGEMENT_DIST {
            g.enter_combat(i);
            play_music(&sfx.combat_music);
            return;
        }
    }

    // Port docking — player must sail close and press SPACE
    if (g.player.world_x - PORT_ANCHOR_X).abs() < PORT_DOCK_RADIUS
        && key_pressed(BLIP_KEY_SPACE)
    {
        g.enter_port();
        play_music(&sfx.port_music);
        return;
    }

    // Death check
    if g.player.hull <= 0 {
        play_sfx(&sfx.life_lost);
        g.lives -= 1;
        g.dead_t = DEAD_TTL;
        g.state  = State::Dead;
    }
}

fn update_combat(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.time += dt;
    g.retreat_t -= dt;

    let pidx = g.combat_enemy_idx;

    // Player dodge (UP/DOWN)
    if key_held(BLIP_KEY_UP) || key_held(BLIP_KEY_W) {
        g.player.y = clamp(g.player.y - DODGE_SPEED * dt, COMBAT_Y_MIN, COMBAT_Y_MAX);
    }
    if key_held(BLIP_KEY_DOWN) || key_held(BLIP_KEY_S) {
        g.player.y = clamp(g.player.y + DODGE_SPEED * dt, COMBAT_Y_MIN, COMBAT_Y_MAX);
    }

    // Player fire (SPACE)
    g.player.reload_t -= dt;
    if key_pressed(BLIP_KEY_SPACE) && g.player.reload_t <= 0.0 && g.player.cannons > 0 {
        g.fire_ball(true, COMBAT_PLAYER_X + PLAYER_W, g.player.y + PLAYER_H / 2.0);
        g.player.reload_t = PLAYER_RELOAD;
        g.player.cannons -= 1;
        play_sfx(&sfx.cannon_fire);
    }

    // Boarding (E) — only when ships are close in Y
    let y_gap = (g.player.y - g.enemies[pidx].combat_y).abs();
    if key_pressed(KEY_BOARD) && y_gap < BOARD_Y_DIST {
        g.enter_boarding();
        return;
    }

    // Enemy AI — oscillating dodge pattern, not player-tracking
    let osc = (g.time * 0.7 + pidx as f32 * 1.3).sin();
    let target_y = COMBAT_BASE_Y + osc * (COMBAT_Y_MAX - COMBAT_Y_MIN) * 0.38;
    let ey = &mut g.enemies[pidx].combat_y;
    let ev = &mut g.enemies[pidx].combat_vy;
    *ev += (target_y - *ey) * 2.0 * dt;
    *ev *= 1.0 - 4.0 * dt;
    *ey = clamp(*ey + *ev * dt, COMBAT_Y_MIN, COMBAT_Y_MAX);

    g.enemies[pidx].reload_t -= dt;
    if g.enemies[pidx].reload_t <= 0.0 {
        let reload = (ENEMY_RELOAD_BASE - g.level as f32 * 0.15).max(0.6);
        g.enemies[pidx].reload_t = reload;
        // Aim slightly toward player Y
        let aim_vy = (g.player.y - g.enemies[pidx].combat_y).clamp(-40.0, 40.0);
        let ex = COMBAT_ENEMY_X;
        let ey = g.enemies[pidx].combat_y + ENEMY_H / 2.0;
        // Manually spawn to pass custom vy
        for i in 2..MAX_CANNONBALLS {
            if !g.cannonballs[i].active {
                g.cannonballs[i] = Cannonball {
                    active: true, x: ex, y: ey,
                    vx: -CANNON_SPEED, vy: aim_vy * 0.4, player: false,
                };
                break;
            }
        }
        play_sfx(&sfx.cannon_fire);
    }

    // Sprite animation
    g.player.anim_t += dt;
    if g.player.anim_t >= ANIM_FRAME_DUR {
        g.player.anim_t = 0.0;
        g.player.anim_frame ^= 1;
    }
    g.enemies[pidx].anim_t += dt;
    if g.enemies[pidx].anim_t >= ANIM_FRAME_DUR {
        g.enemies[pidx].anim_t = 0.0;
        g.enemies[pidx].anim_frame ^= 1;
    }

    // Hit flash timers
    if g.player.hit_flash_t > 0.0        { g.player.hit_flash_t -= dt; }
    if g.enemies[pidx].hit_flash_t > 0.0 { g.enemies[pidx].hit_flash_t -= dt; }

    // Move cannonballs (index loop avoids borrow conflict with spawn_explosion)
    for i in 0..MAX_CANNONBALLS {
        if !g.cannonballs[i].active { continue; }
        g.cannonballs[i].vy += CANNON_GRAVITY * dt;
        g.cannonballs[i].x  += g.cannonballs[i].vx * dt;
        g.cannonballs[i].y  += g.cannonballs[i].vy * dt;
        let bx = g.cannonballs[i].x;
        let by = g.cannonballs[i].y;
        let is_player = g.cannonballs[i].player;

        // Off screen → splash
        if bx < -BALL_W || bx > WIN_W as f32 + BALL_W
           || by < HUD_H as f32 || by > WIN_H as f32 {
            play_sfx(&sfx.splash);
            g.cannonballs[i].active = false;
            continue;
        }

        // Hit player ship
        if !is_player && rects_overlap(bx, by, BALL_W, BALL_H,
            COMBAT_PLAYER_X, g.player.y, PLAYER_W, PLAYER_H)
        {
            g.cannonballs[i].active = false;
            g.player.hull -= 2;
            g.player.hit_flash_t = 0.18;
            let ex = COMBAT_PLAYER_X + PLAYER_W / 2.0;
            let ey_p = g.player.y + PLAYER_H / 2.0;
            g.spawn_explosion(ex, ey_p);
            play_sfx(&sfx.hull_hit);
        }

        // Hit enemy ship
        if is_player && rects_overlap(bx, by, BALL_W, BALL_H,
            COMBAT_ENEMY_X, g.enemies[pidx].combat_y, ENEMY_W, ENEMY_H)
        {
            g.cannonballs[i].active = false;
            g.enemies[pidx].hull -= 2;
            g.enemies[pidx].hit_flash_t = 0.18;
            let ex = COMBAT_ENEMY_X + ENEMY_W / 2.0;
            let ey_e = g.enemies[pidx].combat_y + ENEMY_H / 2.0;
            g.spawn_explosion(ex, ey_e);
            play_sfx(&sfx.hull_hit);
        }
    }

    // Explosions
    for e in g.explosions.iter_mut() {
        if e.active { e.ttl -= dt; if e.ttl <= 0.0 { e.active = false; } }
    }

    // Win: enemy sunk
    if g.enemies[pidx].hull <= 0 {
        g.enemies[pidx].active = false;
        let loot = g.enemies[pidx].gold_loot;
        g.player.gold += loot;
        g.score += 200 * g.level;
        if g.score > g.hi_score { g.hi_score = g.score; }
        play_sfx(&sfx.coin_jingle);
        g.spawn_explosion(COMBAT_ENEMY_X + ENEMY_W / 2.0, g.enemies[pidx].combat_y + ENEMY_H / 2.0);
        for b in g.cannonballs.iter_mut() { b.active = false; }
        g.state = State::Sea;
        play_music(&sfx.sea_music);
        return;
    }

    // Player death
    if g.player.hull <= 0 {
        play_sfx(&sfx.life_lost);
        g.lives -= 1;
        g.dead_t = DEAD_TTL;
        for b in g.cannonballs.iter_mut() { b.active = false; }
        g.state = State::Dead;
        return;
    }

    // Retreat: push enemy far away so it can't immediately re-engage
    if g.retreat_t <= 0.0 {
        g.enemies[pidx].world_x = g.player.world_x + WORLD_W * 0.2
            + rand_int(200, 600) as f32;
        for b in g.cannonballs.iter_mut() { b.active = false; }
        g.state = State::Sea;
        play_music(&sfx.sea_music);
    }
}

fn update_boarding(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.boarding_t        -= dt;
    g.boarding_total_t  -= dt;
    g.time              += dt;
    if g.boarding_hit_t > 0.0 { g.boarding_hit_t -= dt; }

    // ── Player action (SPACE): attack the frontmost enemy slot ──
    if key_pressed(BLIP_KEY_SPACE) {
        // leftmost remaining enemy slot = the one closest to player territory
        for i in 0..BOARDING_SLOTS {
            if g.slots[i].owner == SlotOwner::Enemy {
                g.slots[i].hp -= 1;
                g.boarding_hit_slot = i;
                g.boarding_hit_t    = 0.28;
                play_sfx(&sfx.boarding_clash);
                if g.slots[i].hp <= 0 {
                    g.slots[i].owner = SlotOwner::Empty;
                    g.slots[i].hp    = 0;
                }
                break;
            }
        }
    }

    // ── Enemy auto-tick: attack rightmost remaining player slot ──
    if g.boarding_t <= 0.0 {
        g.boarding_t = BOARDING_TICK;
        // rightmost player slot = the one closest to the enemy side
        let mut hit: Option<usize> = None;
        for i in (0..BOARDING_SLOTS).rev() {
            if g.slots[i].owner == SlotOwner::Player {
                hit = Some(i);
                break;
            }
        }
        if let Some(i) = hit {
            g.slots[i].hp -= 1;
            g.boarding_hit_slot = i;
            g.boarding_hit_t    = 0.28;
            play_sfx(&sfx.boarding_clash);
            if g.slots[i].hp <= 0 {
                // Enemy crew captures this slot
                g.slots[i].owner = SlotOwner::Enemy;
                g.slots[i].hp    = 2;
            }
        }
    }

    // ── Outcome checks ──
    let enemy_alive  = g.slots.iter().any(|s| s.owner == SlotOwner::Enemy);
    let player_alive = g.slots.iter().any(|s| s.owner == SlotOwner::Player);

    if !enemy_alive {
        let idx  = g.combat_enemy_idx;
        let loot = g.enemies[idx].gold_loot * 2;
        g.player.gold += loot;
        g.score       += 150 * g.level + loot;
        if g.score > g.hi_score { g.hi_score = g.score; }
        g.enemies[idx].active = false;
        play_sfx(&sfx.coin_jingle);
        g.state = State::Sea;
        play_music(&sfx.sea_music);
        return;
    }

    if !player_alive {
        g.player.hull = 0;
        g.lives      -= 1;
        g.dead_t      = DEAD_TTL;
        play_sfx(&sfx.life_lost);
        g.state = State::Dead;
        play_music(&sfx.sea_music);
        return;
    }

    if g.boarding_total_t <= 0.0 {
        // Timeout — both sides withdraw, no loot
        g.state = State::Sea;
        play_music(&sfx.sea_music);
    }
}

fn update_port(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.port_msg_t -= dt;

    if key_pressed(BLIP_KEY_UP) || key_pressed(BLIP_KEY_W) {
        g.port_cursor = g.port_cursor.prev();
    }
    if key_pressed(BLIP_KEY_DOWN) || key_pressed(BLIP_KEY_S) {
        g.port_cursor = g.port_cursor.next();
    }

    let confirm = key_pressed(BLIP_KEY_SPACE) || key_pressed(KEY_ENTER);
    if confirm {
        match g.port_cursor {
            PortItem::Sail => {
                g.spawn_enemies();
                g.state = State::Sea;
                play_music(&sfx.sea_music);
            }
            PortItem::Map => {
                g.state = State::Map;
            }
            item => {
                let cost = item.cost();
                if g.player.gold >= cost {
                    g.player.gold -= cost;
                    match item {
                        PortItem::Repair  => { g.player.hull = PLAYER_HULL_MAX; g.port_msg = "HULL REPAIRED"; }
                        PortItem::Crew    => { g.player.crew += 2; g.port_msg = "CREW HIRED"; }
                        PortItem::Cannons => { g.player.cannons += 4; g.port_msg = "CANNONS LOADED"; }
                        PortItem::Food    => { g.player.food = FOOD_MAX as f32; g.port_msg = "PROVISIONS STOCKED"; }
                        PortItem::Sail    => {}
                        PortItem::Map     => {}
                    }
                    g.port_msg_t  = 1.5;
                    g.port_msg_ok = true;
                    play_sfx(&sfx.coin_jingle);
                } else {
                    g.port_msg    = "NOT ENOUGH GOLD";
                    g.port_msg_t  = 1.5;
                    g.port_msg_ok = false;
                }
            }
        }
    }
}

fn update_dead(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.dead_t -= dt;
    if g.dead_t <= 0.0 {
        if g.lives > 0 {
            g.respawn_at_sea();
            play_music(&sfx.sea_music);
        } else {
            g.state = State::GameOver;
        }
    }
}

fn update_gameover(g: &mut Game) {
    if any_key_pressed() {
        web::spend_coin();
        g.state = State::Title;
    }
}

// ── draw ──────────────────────────────────────────────────────────────────────

fn draw_sea_bg(blip: &Blip, tex_a: &Texture2D, tex_b: &Texture2D, cam_x: f32, time: f32) {
    let play_y    = HUD_H as f32;
    let play_h    = (WIN_H - HUD_H) as f32;
    let horizon_y = play_y + play_h * 0.32;

    // Sky
    blip.fill_rect(0.0, play_y, WIN_W as f32, horizon_y - play_y,
                   Color::new(0.05, 0.10, 0.22, 1.0));
    // Horizon glow strip
    blip.fill_rect(0.0, horizon_y - 2.0, WIN_W as f32, 4.0,
                   Color::new(0.10, 0.25, 0.40, 1.0));
    // Deep water
    blip.fill_rect(0.0, horizon_y, WIN_W as f32, (WIN_H as f32) - horizon_y,
                   Color::new(0.04, 0.18, 0.30, 1.0));

    let tile_w = 120.0_f32;
    // Pick A or B frame at 2.5 Hz; phase offset desynchronises the two layers
    let wave_tex = |phase: f32| -> &Texture2D {
        if ((time + phase) * 2.5) as u32 % 2 == 0 { tex_a } else { tex_b }
    };

    // Layer 1 — horizon waves, full colour, fastest scroll
    {
        let tex    = wave_tex(0.0);
        let offset = (cam_x + time * 18.0).rem_euclid(tile_w);
        let mut sx = -offset;
        while sx < WIN_W as f32 {
            blip.draw_texture(tex, sx, horizon_y - 10.0, tile_w, 40.0);
            sx += tile_w;
        }
    }

    // Layer 2 — mid-sea waves, slightly desaturated, medium scroll
    {
        let tex    = wave_tex(0.4);
        let offset = (cam_x * 1.05 + time * 11.0).rem_euclid(tile_w);
        let tint   = Color::new(0.8, 0.9, 1.0, 1.0);
        let mut sx = -offset;
        while sx < WIN_W as f32 {
            blip.draw_texture_tinted(tex, sx, horizon_y + 90.0, tile_w, 40.0, tint);
            sx += tile_w;
        }
    }
}

fn draw_sea_foreground(blip: &Blip, tex_a: &Texture2D, tex_b: &Texture2D, cam_x: f32, time: f32) {
    let tile_w = 120.0_f32;
    let tex    = if ((time * 1.8) as u32) % 2 == 0 { tex_a } else { tex_b };
    let offset = (cam_x * 1.15 + time * 5.0).rem_euclid(tile_w);
    let tint   = Color::new(0.7, 0.85, 1.0, 0.85);
    let fg_y   = SEA_LANE_Y + 18.0;
    let mut sx = -offset;
    while sx < WIN_W as f32 {
        blip.draw_texture_tinted(tex, sx, fg_y, tile_w, 48.0, tint);
        sx += tile_w;
    }
}

fn draw_ship_fire(blip: &Blip, sx: f32, sy: f32, hull: i32, hull_max: i32, time: f32) {
    if hull * 2 >= hull_max { return; }

    let intensity = 1.0 - (hull as f32 / (hull_max as f32 * 0.5)).clamp(0.0, 1.0);
    let n_flames: usize = if hull * 4 < hull_max { 4 } else { 2 };

    const X_OFF: [f32; 4] = [8.0, 18.0, 30.0, 14.0];
    const FREQS: [f32; 4] = [7.3,  9.1, 11.7,  6.8];

    for i in 0..n_flames {
        let flicker = (time * FREQS[i]).sin() * 0.35 + 0.65;
        let h = (6.0 + intensity * 10.0) * flicker;
        let w = 4.0_f32;
        let fx = sx + X_OFF[i] + (time * 3.1 + i as f32 * 1.7).sin() * 1.5;
        // Base on deck (sy+16); flames grow upward — never touches hull/waterline at sy+22+
        let fy = sy + 16.0 - h;
        let seg = h / 3.0;
        blip.fill_rect(fx,       fy + seg * 2.0, w,       seg, Color::new(0.75, 0.15, 0.0, 0.90));
        blip.fill_rect(fx,       fy + seg,        w,       seg, Color::new(1.0,  0.45, 0.0, 0.90));
        blip.fill_rect(fx + 0.5, fy,              w - 1.0, seg, Color::new(1.0,  0.90, 0.1, 0.85));
    }
}

fn world_to_screen(wx: f32, cam_x: f32) -> f32 { wx - cam_x }

fn draw_title(blip: &Blip, g: &Game, tex: &Textures) {
    draw_sea_bg(blip, &tex.sea_wave, &tex.sea_wave_b, g.time * 20.0, g.time);

    // Ship bobbing on the horizon
    let bob = (g.time * BOB_FREQ).sin() * BOB_AMP;
    let ship_w = PLAYER_W * 2.0;
    let ship_h = PLAYER_H * 2.0;
    let ship_x = WIN_W as f32 / 2.0 - ship_w / 2.0;
    let ship_y = WIN_H as f32 * 0.40 + bob;
    let ship_t = if (g.time * (1.0 / ANIM_FRAME_DUR)) as u32 % 2 == 0 { &tex.player_a } else { &tex.player_b };
    blip.draw_texture(ship_t, ship_x, ship_y, ship_w, ship_h);

    blip.draw_centered("CANARIS",               WIN_H as f32 * 0.10, 6.0, BLIP_CYAN);
    blip.draw_centered("PRIVATEER OF KATTEGAT", WIN_H as f32 * 0.22, 2.0, BLIP_YELLOW);

    if g.hi_score > 0 {
        let hi_str = format!("BEST {}", g.hi_score);
        blip.draw_centered(&hi_str, WIN_H as f32 * 0.63, 2.0, BLIP_YELLOW);
    }
    // Blink "PRESS ANY KEY" at 1Hz
    if (g.time * 2.0) as u32 % 2 == 0 {
        blip.draw_centered("PRESS ANY KEY",     WIN_H as f32 * 0.72, 3.0, BLIP_WHITE);
    }
    blip.draw_centered("SAIL. RAID. PLUNDER.",  WIN_H as f32 * 0.84, 2.0, BLIP_GRAY);
}

fn draw_hud_canaris(blip: &Blip, g: &Game) {
    blip.draw_hud(g.score, g.hi_score, g.lives);
    // Secondary stat row at y=20 (within the 28px HUD black band, below the score row)
    let y = 20.0_f32;
    let hull_col = if g.player.hull < 6 { BLIP_RED } else { BLIP_GREEN };
    let food_col = if g.player.food < 5.0 { BLIP_RED } else { BLIP_WHITE };
    blip.draw_text(&format!("H:{}", g.player.hull),    4.0,  y, 1.0, hull_col);
    blip.draw_text(&format!("G:{}", g.player.gold),  100.0,  y, 1.0, BLIP_YELLOW);
    blip.draw_text(&format!("F:{}", g.player.food as i32), 210.0, y, 1.0, food_col);
    blip.draw_text(&format!("C:{}", g.player.cannons), 320.0, y, 1.0, BLIP_ORANGE);
    blip.draw_text(&format!("LV{}", g.level),         410.0,  y, 1.0, BLIP_CYAN);
}

fn tint_white() -> BlipColor { BLIP_WHITE }

fn draw_sea(blip: &Blip, g: &Game, tex: &Textures) {
    draw_sea_bg(blip, &tex.sea_wave, &tex.sea_wave_b, g.cam_x, g.time);

    // Enemies
    for e in g.enemies.iter() {
        if !e.active { continue; }
        let sx = world_to_screen(e.world_x, g.cam_x);
        if sx < -ENEMY_W || sx > WIN_W as f32 { continue; }
        let et = if e.anim_frame == 0 { &tex.enemy_a } else { &tex.enemy_b };
        if e.hit_flash_t > 0.0 {
            blip.draw_texture_tinted(et, sx, e.y, ENEMY_W, ENEMY_H, tint_white());
        } else {
            blip.draw_texture(et, sx, e.y, ENEMY_W, ENEMY_H);
        }
        draw_ship_fire(blip, sx, e.y, e.hull, e.hull_max, g.time);
    }

    // Port marker
    let port_sx = world_to_screen(PORT_ANCHOR_X, g.cam_x);
    if port_sx > -20.0 && port_sx < WIN_W as f32 + 20.0 {
        blip.fill_rect(port_sx, WIN_H as f32 - 40.0, 8.0, 30.0, BLIP_YELLOW);
        blip.draw_text("PORT", port_sx - 10.0, WIN_H as f32 - 52.0, 1.0, BLIP_YELLOW);
        let near = (g.player.world_x - PORT_ANCHOR_X).abs() < PORT_DOCK_RADIUS;
        if near && (g.time * 2.0) as u32 % 2 == 0 {
            blip.draw_centered("SPACE: DOCK", WIN_H as f32 - 65.0, 1.0, BLIP_YELLOW);
        }
    }

    // Player
    let psx = world_to_screen(g.player.world_x, g.cam_x);
    let pt  = if g.player.anim_frame == 0 { &tex.player_a } else { &tex.player_b };
    if g.player.hit_flash_t > 0.0 {
        blip.draw_texture_tinted(pt, psx, g.player.y, PLAYER_W, PLAYER_H, tint_white());
    } else {
        blip.draw_texture(pt, psx, g.player.y, PLAYER_W, PLAYER_H);
    }
    draw_ship_fire(blip, psx, g.player.y, g.player.hull, PLAYER_HULL_MAX, g.time);

    // Wake trail when moving fast enough
    if g.player.vx.abs() > 5.0 {
        let wake_col = Color::new(0.5, 0.85, 1.0, 0.5);
        let wake_y   = g.player.y + PLAYER_H - 4.0;
        let offsets: [f32; 4]        = [6.0, 14.0, 24.0, 36.0];
        let sizes:   [(f32, f32); 4] = [(6.0, 4.0), (5.0, 3.0), (4.0, 2.0), (3.0, 2.0)];
        for (i, &off) in offsets.iter().enumerate() {
            let wx = if g.player.vx > 0.0 {
                psx - off
            } else {
                psx + PLAYER_W + off - sizes[i].0
            };
            blip.fill_rect(wx, wake_y, sizes[i].0, sizes[i].1, wake_col);
        }
    }

    // Foreground wave layer drawn over hull bottoms — ships appear at the waterline
    draw_sea_foreground(blip, &tex.sea_wave, &tex.sea_wave_b, g.cam_x, g.time);

    draw_hud_canaris(blip, g);
}

fn draw_combat(blip: &Blip, g: &Game, tex: &Textures) {
    let pidx = g.combat_enemy_idx;

    // Scrolling sea background
    draw_sea_bg(blip, &tex.sea_wave, &tex.sea_wave_b, g.time * 15.0, g.time);

    // Player ship — with muzzle flash: player just fired if reload_t is nearly full
    let pt = if g.player.anim_frame == 0 { &tex.player_a } else { &tex.player_b };
    if g.player.hit_flash_t > 0.0 {
        blip.draw_texture_tinted(pt, COMBAT_PLAYER_X, g.player.y, PLAYER_W, PLAYER_H, tint_white());
    } else {
        blip.draw_texture(pt, COMBAT_PLAYER_X, g.player.y, PLAYER_W, PLAYER_H);
    }
    // Muzzle flash: bright spark at cannon mouth for first 0.08s of reload
    if g.player.reload_t > PLAYER_RELOAD - 0.08 && g.player.reload_t > 0.0 {
        let fx = COMBAT_PLAYER_X + PLAYER_W + 2.0;
        let fy = g.player.y + PLAYER_H * 0.55 - 4.0;
        blip.fill_rect(fx, fy, 10.0, 8.0, BLIP_YELLOW);
        blip.fill_rect(fx + 2.0, fy + 1.0, 6.0, 6.0, BLIP_WHITE);
    }
    draw_ship_fire(blip, COMBAT_PLAYER_X, g.player.y, g.player.hull, PLAYER_HULL_MAX, g.time);

    // Enemy ship
    let et = if g.enemies[pidx].anim_frame == 0 { &tex.enemy_a } else { &tex.enemy_b };
    let enemy_cy = g.enemies[pidx].combat_y;
    if g.enemies[pidx].hit_flash_t > 0.0 {
        blip.draw_texture_tinted(et, COMBAT_ENEMY_X, enemy_cy, ENEMY_W, ENEMY_H, tint_white());
    } else {
        blip.draw_texture(et, COMBAT_ENEMY_X, enemy_cy, ENEMY_W, ENEMY_H);
    }
    // Enemy muzzle flash (left side, firing toward player)
    if g.enemies[pidx].reload_t > (ENEMY_RELOAD_BASE - g.level as f32 * 0.15).max(0.6) - 0.08 {
        let fx = COMBAT_ENEMY_X - 12.0;
        let fy = enemy_cy + ENEMY_H * 0.55 - 4.0;
        blip.fill_rect(fx, fy, 10.0, 8.0, BLIP_ORANGE);
        blip.fill_rect(fx + 2.0, fy + 1.0, 6.0, 6.0, BLIP_YELLOW);
    }
    draw_ship_fire(blip, COMBAT_ENEMY_X, enemy_cy, g.enemies[pidx].hull, g.enemies[pidx].hull_max, g.time);

    // Cannonballs
    for b in g.cannonballs.iter() {
        if !b.active { continue; }
        blip.draw_texture(&tex.ball, b.x, b.y, BALL_W, BALL_H);
    }

    // Explosions (scale up over lifetime)
    for e in g.explosions.iter() {
        if !e.active { continue; }
        let t    = e.ttl / EXPLOSION_TTL;
        let size = 32.0 * (1.0 + (1.0 - t) * 0.7);
        blip.draw_texture(&tex.explosion, e.x - size / 2.0, e.y - size / 2.0, size, size);
    }

    // ── Bottom UI strip ──────────────────────────────────────────────────────
    // Dark panel behind UI
    blip.fill_rect(0.0, COMBAT_UI_Y - 4.0, WIN_W as f32, WIN_H as f32 - COMBAT_UI_Y + 4.0,
                   Color::new(0.0, 0.0, 0.0, 0.55));

    // Row 1 (COMBAT_UI_Y): hull bars
    let bar_w = 90.0_f32;
    let bar_h = 8.0_f32;
    let bar_y = COMBAT_UI_Y;

    // Player hull bar (left side, centered on player ship)
    let player_bar_x = COMBAT_PLAYER_X + PLAYER_W / 2.0 - bar_w / 2.0;
    let phull_frac = (g.player.hull as f32 / PLAYER_HULL_MAX as f32).clamp(0.0, 1.0);
    let p_col = if phull_frac < 0.3 { BLIP_RED } else if phull_frac < 0.6 { BLIP_YELLOW } else { BLIP_GREEN };
    blip.fill_rect(player_bar_x, bar_y, bar_w, bar_h, BLIP_DARKGRAY);
    blip.fill_rect(player_bar_x, bar_y, bar_w * phull_frac, bar_h, p_col);
    blip.draw_text("YOU",  player_bar_x, bar_y - 8.0, 1.0, BLIP_WHITE);

    // Enemy hull bar (right side, centered on enemy ship)
    let enemy_bar_x = COMBAT_ENEMY_X + ENEMY_W / 2.0 - bar_w / 2.0;
    let ehull_frac = (g.enemies[pidx].hull as f32 / g.enemies[pidx].hull_max as f32).clamp(0.0, 1.0);
    let e_col = if ehull_frac < 0.3 { BLIP_GREEN } else if ehull_frac < 0.6 { BLIP_YELLOW } else { BLIP_RED };
    blip.fill_rect(enemy_bar_x, bar_y, bar_w, bar_h, BLIP_DARKGRAY);
    blip.fill_rect(enemy_bar_x, bar_y, bar_w * ehull_frac, bar_h, e_col);
    blip.draw_text("ENEMY", enemy_bar_x, bar_y - 8.0, 1.0, BLIP_WHITE);

    // Row 2: retreat timer bar (centered)
    let rt_y = COMBAT_UI_Y + bar_h + 5.0;
    let rt = (g.retreat_t / RETREAT_TIMER).clamp(0.0, 1.0);
    let rt_w = WIN_W as f32 * 0.5;
    blip.fill_rect(WIN_W as f32 / 2.0 - rt_w / 2.0, rt_y, rt_w, 4.0, BLIP_DARKGRAY);
    blip.fill_rect(WIN_W as f32 / 2.0 - rt_w / 2.0, rt_y, rt_w * rt, 4.0, BLIP_ORANGE);
    blip.draw_text("TIME", WIN_W as f32 / 2.0 - 10.0, rt_y - 7.0, 1.0, BLIP_ORANGE);

    // Row 3: boarding hint / no-ammo warning
    let hint_y = rt_y + 8.0;
    let y_gap = (g.player.y - g.enemies[pidx].combat_y).abs();
    if g.player.cannons == 0 {
        if (g.time * 3.0) as u32 % 2 == 0 {
            blip.draw_centered("NO AMMO - [E] BOARD OR RETREAT", hint_y, 1.0, BLIP_RED);
        }
    } else if y_gap < BOARD_Y_DIST {
        blip.draw_centered("[E] BOARD  [SPACE] FIRE", hint_y, 1.0, BLIP_CYAN);
    } else {
        blip.draw_centered("[SPACE] FIRE  [UP/DN] DODGE", hint_y, 1.0, BLIP_GRAY);
    }

    draw_hud_canaris(blip, g);
}

fn draw_boarding(blip: &Blip, g: &Game, tex: &Textures) {
    // Night sea backdrop
    blip.fill_rect(0.0, HUD_H as f32, WIN_W as f32, (WIN_H - HUD_H) as f32,
                   Color::new(0.04, 0.07, 0.16, 1.0));

    // Title
    blip.draw_centered("BOARDING ACTION", HUD_H as f32 + 10.0, 3.0, BLIP_RED);

    // Side labels, centered in each half (each char ~7px at size 2 = 14px)
    blip.draw_text("YOUR CREW", 57.0, HUD_H as f32 + 40.0, 2.0, BLIP_CYAN);
    blip.draw_text("ENEMY CREW", 290.0, HUD_H as f32 + 40.0, 2.0, BLIP_RED);

    // Ship deck planks
    let deck_y = 100.0_f32;
    let deck_h = 200.0_f32;
    blip.fill_rect(0.0, deck_y, WIN_W as f32, deck_h,
                   Color::new(0.29, 0.18, 0.10, 1.0));
    for row in 1..5 {
        let py = deck_y + row as f32 * (deck_h / 5.0);
        blip.fill_rect(0.0, py, WIN_W as f32, 2.0, Color::new(0.17, 0.10, 0.05, 1.0));
    }
    // Deck rail
    blip.fill_rect(0.0, deck_y - 8.0, WIN_W as f32, 9.0, Color::new(0.45, 0.28, 0.14, 1.0));
    blip.fill_rect(0.0, deck_y - 8.0, WIN_W as f32, 2.0, Color::new(0.65, 0.42, 0.22, 1.0));

    // Centre battle-line divider
    blip.fill_rect(237.0, deck_y - 8.0, 6.0, deck_h + 8.0, BLIP_RED);
    blip.fill_rect(239.5, deck_y - 8.0, 2.0, deck_h + 8.0, Color::new(1.0, 0.55, 0.55, 1.0));

    // Crew slots — 3 player (left) + 3 enemy (right), 80px columns
    let fig_w   = 24.0_f32;
    let fig_h   = 40.0_f32;
    let fig_y   = deck_y + (deck_h - fig_h) / 2.0 - 8.0;
    let hp_y    = fig_y + fig_h + 5.0;
    let pip_w   = 18.0_f32;
    let pip_gap = 3.0_f32;
    let pips_w  = 3.0 * pip_w + 2.0 * pip_gap;

    for i in 0..BOARDING_SLOTS {
        let slot  = &g.slots[i];
        let col_x = i as f32 * 80.0;
        let fig_x = col_x + (80.0 - fig_w) / 2.0;

        // Hit flash
        if g.boarding_hit_slot == i && g.boarding_hit_t > 0.0 {
            let a = (g.boarding_hit_t / 0.28).min(1.0) * 0.55;
            blip.fill_rect(col_x + 1.0, deck_y + 2.0, 78.0, deck_h - 4.0,
                           Color::new(1.0, 1.0, 0.3, a));
        }

        match slot.owner {
            SlotOwner::Empty => {
                blip.fill_rect(fig_x + 4.0, fig_y + 8.0, fig_w - 8.0, fig_h - 8.0,
                               Color::new(0.15, 0.09, 0.05, 1.0));
                blip.draw_text("X", col_x + 30.0, fig_y + 12.0, 2.0, BLIP_DARKGRAY);
            }
            SlotOwner::Player => {
                blip.draw_texture_tinted(&tex.crew, fig_x, fig_y, fig_w, fig_h, BLIP_CYAN);
            }
            SlotOwner::Enemy => {
                blip.draw_texture_tinted(&tex.crew, fig_x, fig_y, fig_w, fig_h, BLIP_RED);
            }
        }

        // HP pips (3 max)
        if slot.owner != SlotOwner::Empty {
            let px0 = col_x + (80.0 - pips_w) / 2.0;
            for pip in 0..3_i32 {
                let px  = px0 + pip as f32 * (pip_w + pip_gap);
                let col = if pip < slot.hp {
                    match slot.owner {
                        SlotOwner::Player => BLIP_CYAN,
                        SlotOwner::Enemy  => BLIP_RED,
                        _                 => BLIP_GRAY,
                    }
                } else {
                    BLIP_DARKGRAY
                };
                blip.fill_rect(px, hp_y, pip_w, 6.0, col);
            }
        }
    }

    // Timeout bar
    let t_frac = (g.boarding_total_t / BOARDING_TIMEOUT).clamp(0.0, 1.0);
    let tb_y   = deck_y + deck_h + 16.0;
    let tb_x   = 38.0_f32;
    let tb_w   = WIN_W as f32 - 76.0;
    blip.draw_text("TIME", 4.0, tb_y + 1.0, 1.0, BLIP_GRAY);
    blip.fill_rect(tb_x - 1.0, tb_y - 1.0, tb_w + 2.0, 10.0, BLIP_DARKGRAY);
    blip.fill_rect(tb_x, tb_y, tb_w * t_frac, 8.0, BLIP_ORANGE);
    blip.draw_text(&format!("{:.0}s", g.boarding_total_t.max(0.0)),
                   tb_x + tb_w + 4.0, tb_y + 1.0, 1.0, BLIP_GRAY);

    // Crew-count totals
    let p_alive = g.slots.iter().filter(|s| s.owner == SlotOwner::Player).count();
    let e_alive = g.slots.iter().filter(|s| s.owner == SlotOwner::Enemy).count();
    blip.draw_text(&format!("{}/3", p_alive), 8.0, tb_y + 18.0, 2.0, BLIP_CYAN);
    blip.draw_text(&format!("{}/3", e_alive), WIN_W as f32 - 44.0, tb_y + 18.0, 2.0, BLIP_RED);

    // Controls hint
    blip.draw_centered("[SPACE] ATTACK", tb_y + 38.0, 2.0, BLIP_WHITE);

    draw_hud_canaris(blip, g);
}

fn draw_port(blip: &Blip, g: &Game, tex: &Textures) {
    blip.draw_texture(&tex.port_bg, 0.0, HUD_H as f32, WIN_W as f32, (WIN_H - HUD_H) as f32);

    let panel_x = WIN_W as f32 * 0.12;
    let panel_w = WIN_W as f32 * 0.76;

    // ── Player stats strip ────────────────────────────────────────────────────
    let stats_y = 218.0_f32;
    let stats_h = 52.0_f32;
    blip.fill_rect(panel_x - 2.0, stats_y - 2.0, panel_w + 4.0, stats_h + 4.0, BLIP_DARKGRAY);
    blip.fill_rect(panel_x, stats_y, panel_w, stats_h, Color::new(0.04, 0.07, 0.12, 1.0));

    let hull_frac = (g.player.hull as f32 / PLAYER_HULL_MAX as f32).clamp(0.0, 1.0);
    let hull_col  = if hull_frac > 0.5 { BLIP_GREEN } else if hull_frac > 0.25 { BLIP_YELLOW } else { BLIP_RED };
    blip.draw_text("HULL", panel_x + 4.0, stats_y + 4.0, 1.0, BLIP_GRAY);
    blip.draw_text(&format!("{}/{}", g.player.hull, PLAYER_HULL_MAX),
                   panel_x + panel_w - 52.0, stats_y + 4.0, 1.0, hull_col);
    blip.fill_rect(panel_x + 4.0, stats_y + 14.0, panel_w - 8.0, 8.0, BLIP_DARKGRAY);
    blip.fill_rect(panel_x + 4.0, stats_y + 14.0, (panel_w - 8.0) * hull_frac, 8.0, hull_col);

    let col_w = (panel_w - 8.0) / 3.0;
    blip.draw_text(&format!("CREW {}", g.player.crew),
                   panel_x + 4.0, stats_y + 30.0, 2.0, BLIP_CYAN);
    blip.draw_text(&format!("FOOD {}", g.player.food as i32),
                   panel_x + 4.0 + col_w, stats_y + 30.0, 2.0, BLIP_GREEN);
    blip.draw_text(&format!("GUNS {}", g.player.cannons),
                   panel_x + 4.0 + col_w * 2.0, stats_y + 30.0, 2.0, BLIP_ORANGE);

    // ── Status message (between strips) ──────────────────────────────────────
    if g.port_msg_t > 0.0 {
        let msg_col = if g.port_msg_ok { BLIP_GREEN } else { BLIP_RED };
        blip.draw_centered(g.port_msg, stats_y + stats_h + 8.0, 2.0, msg_col);
    }

    // ── Menu panel ────────────────────────────────────────────────────────────
    let panel_y = stats_y + stats_h + 28.0;
    let panel_h = 182.0_f32;
    blip.fill_rect(panel_x - 2.0, panel_y - 2.0, panel_w + 4.0, panel_h + 4.0, BLIP_CYAN);
    blip.fill_rect(panel_x, panel_y, panel_w, panel_h, BLIP_BLACK);

    blip.draw_text(&format!("GOLD: {}", g.player.gold),
                   panel_x + 8.0, panel_y + 6.0, 2.0, BLIP_YELLOW);

    let items = [PortItem::Sail, PortItem::Map, PortItem::Repair, PortItem::Crew, PortItem::Cannons, PortItem::Food];
    for (i, &item) in items.iter().enumerate() {
        let iy         = panel_y + 30.0 + i as f32 * 24.0;
        let no_cost    = item == PortItem::Sail || item == PortItem::Map;
        let can_afford = no_cost || g.player.gold >= item.cost();
        let selected   = item == g.port_cursor;
        let col = if selected { BLIP_YELLOW } else if can_afford { BLIP_WHITE } else { BLIP_DARKGRAY };
        let prefix = if selected { ">" } else { " " };
        blip.draw_text(&format!("{} {}", prefix, item.label()), panel_x + 8.0, iy, 2.0, col);
        if !no_cost {
            let cost_col = if can_afford { BLIP_YELLOW } else { BLIP_DARKGRAY };
            blip.draw_text(&format!("{}G", item.cost()),
                           panel_x + panel_w - 44.0, iy, 2.0, cost_col);
        }
    }

    draw_hud_canaris(blip, g);
}

fn update_map(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.time += dt;

    if key_pressed(BLIP_KEY_UP) || key_pressed(BLIP_KEY_W) {
        if g.map_cursor + 1 < ZONES.len() { g.map_cursor += 1; }
    }
    if key_pressed(BLIP_KEY_DOWN) || key_pressed(BLIP_KEY_S) {
        if g.map_cursor > 0 { g.map_cursor -= 1; }
    }

    if key_pressed(BLIP_KEY_SPACE) {
        let z = &ZONES[g.map_cursor];
        g.level   = z.level_eq;
        g.level_t = 90.0;
        g.spawn_enemies_n(z.ships);
        g.state = State::Sea;
        play_music(&sfx.sea_music);
    }

    if key_pressed(KeyCode::Escape) || key_pressed(KEY_BOARD) {
        g.state = State::Port;
    }
}

fn draw_map(blip: &Blip, g: &Game, tex: &Textures) {
    let play_y = HUD_H as f32;

    blip.draw_texture(&tex.map_bg, 0.0, play_y, WIN_W as f32, (WIN_H - HUD_H) as f32);

    blip.draw_centered("KATTEGAT", play_y + 20.0, 2.0, BLIP_CYAN);

    // Route line connecting adjacent zone dots
    for i in 0..ZONES.len() - 1 {
        let a = &ZONES[i];
        let b = &ZONES[i + 1];
        let lx = (a.map_x + b.map_x) * 0.5;
        let ly = play_y + b.map_y;
        let lh = a.map_y - b.map_y;
        blip.fill_rect(lx, ly, 2.0, lh, Color::new(0.3, 0.5, 0.7, 0.6));
    }

    // Zone crosshair dots
    for (i, z) in ZONES.iter().enumerate() {
        let selected = i == g.map_cursor;
        let blink_on = !selected || (g.time * 3.0) as u32 % 2 == 0;
        let col = if selected { BLIP_YELLOW } else { Color::new(0.3, 0.6, 0.9, 1.0) };
        if blink_on {
            blip.fill_rect(z.map_x - 5.0, play_y + z.map_y - 5.0, 10.0, 10.0, col);
            blip.fill_rect(z.map_x - 8.0, play_y + z.map_y - 1.0, 16.0, 2.0, col);
            blip.fill_rect(z.map_x - 1.0, play_y + z.map_y - 8.0, 2.0, 16.0, col);
        }
    }

    // Info panel
    let panel_y = play_y + (WIN_H - HUD_H) as f32 * 0.60;
    blip.fill_rect(8.0, panel_y, 300.0, 110.0, Color::new(0.0, 0.05, 0.15, 0.82));
    let z = &ZONES[g.map_cursor];
    blip.draw_text(z.name, 16.0, panel_y + 10.0, 2.0, BLIP_YELLOW);
    blip.draw_text(z.desc, 16.0, panel_y + 34.0, 1.0, BLIP_WHITE);
    let stars_str = match z.stars {
        1 => "DANGER  *",
        2 => "DANGER  **",
        3 => "DANGER  ***",
        _ => "DANGER  ****",
    };
    blip.draw_text(stars_str, 16.0, panel_y + 52.0, 1.0, BLIP_RED);
    let ships_str = format!("SHIPS   {}", z.ships);
    blip.draw_text(&ships_str, 16.0, panel_y + 68.0, 1.0, BLIP_CYAN);
    blip.draw_text("W/S SELECT    SPACE SAIL    E BACK",
                   16.0, panel_y + 90.0, 1.0, BLIP_GRAY);

    draw_hud_canaris(blip, g);
}

fn draw_dead(blip: &Blip, g: &Game, tex: &Textures) {
    let t = g.dead_t / DEAD_TTL;
    if t > 0.7 {
        // Flash red
        blip.clear(BLIP_RED);
    } else {
        draw_sea(blip, g, tex);
        // Fade-out overlay
        let alpha = (1.0 - t / 0.7).min(1.0);
        let dim = Color::new(0.0, 0.0, 0.0, alpha * 0.6);
        blip.fill_rect(0.0, 0.0, WIN_W as f32, WIN_H as f32, dim);
    }
    blip.draw_centered("SHIP SUNK!", (WIN_H / 2) as f32, 4.0, BLIP_RED);
    let lives_str = format!("LIVES {}", g.lives);
    blip.draw_centered(&lives_str, (WIN_H / 2 + 40) as f32, 3.0, BLIP_WHITE);
}

fn draw_gameover(blip: &Blip, g: &Game) {
    blip.clear(BLIP_BLACK);
    blip.draw_centered("GAME OVER",     (WIN_H / 4) as f32,     5.0, BLIP_RED);
    let score_str = format!("SCORE {}", g.score);
    blip.draw_centered(&score_str,      (WIN_H / 2) as f32,     3.0, BLIP_WHITE);
    let hi_str = format!("BEST  {}", g.hi_score);
    blip.draw_centered(&hi_str,         (WIN_H / 2 + 30) as f32, 2.0, BLIP_YELLOW);
    blip.draw_centered("PRESS ANY KEY", (WIN_H * 2 / 3) as f32, 3.0, BLIP_CYAN);
}

// ── entry ─────────────────────────────────────────────────────────────────────

fn conf() -> blip::macroquad::window::Conf {
    window_conf("CANARIS", WIN_W, WIN_H)
}

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g    = Game::new();

    let tex = Textures {
        player_a: load_png(PLAYER_A_PNG),
        player_b: load_png(PLAYER_B_PNG),
        enemy_a:  load_png(ENEMY_A_PNG),
        enemy_b:  load_png(ENEMY_B_PNG),
        ball:     load_png(BALL_PNG),
        explosion:load_png(EXPLODE_PNG),
        port_bg:  load_png(PORT_BG_PNG),
        sea_wave:   load_png(SEA_WAVE_PNG),
        sea_wave_b: load_png(SEA_WAVE_B_PNG),
        crew:       load_png(CREW_PNG),
        map_bg:     load_png(MAP_BG_PNG),
    };

    let sfx = Sounds {
        cannon_fire:    blip::audio::load_sound(CANNON_WAV).await,
        explosion:      blip::audio::load_sound(EXPLODE_WAV).await,
        splash:         blip::audio::load_sound(SPLASH_WAV).await,
        hull_hit:       blip::audio::load_sound(HULL_HIT_WAV).await,
        boarding_clash: blip::audio::load_sound(CLASH_WAV).await,
        coin_jingle:    blip::audio::load_sound(COINS_WAV).await,
        life_lost:      blip::audio::load_sound(LIFE_WAV).await,
        sea_music:      blip::audio::load_sound(SEA_MUS_WAV).await,
        combat_music:   blip::audio::load_sound(COMBAT_WAV).await,
        port_music:     blip::audio::load_sound(PORT_MUS_WAV).await,
        ocean_ambience: blip::audio::load_sound(AMBIENT_WAV).await,
    };

    play_music(&sfx.sea_music);
    play_ambient(&sfx.ocean_ambience);

    loop {
        let dt = blip.delta_time;

        match g.state {
            State::Title    => update_title(&mut g, dt),
            State::Sea      => update_sea(&mut g, dt, &sfx),
            State::Combat   => update_combat(&mut g, dt, &sfx),
            State::Boarding => update_boarding(&mut g, dt, &sfx),
            State::Port     => update_port(&mut g, dt, &sfx),
            State::Map      => update_map(&mut g, dt, &sfx),
            State::Dead     => update_dead(&mut g, dt, &sfx),
            State::GameOver => update_gameover(&mut g),
        }

        blip.clear(BLIP_BLACK);

        match g.state {
            State::Title    => draw_title(&blip, &g, &tex),
            State::Sea      => draw_sea(&blip, &g, &tex),
            State::Combat   => draw_combat(&blip, &g, &tex),
            State::Boarding => draw_boarding(&blip, &g, &tex),
            State::Port     => draw_port(&blip, &g, &tex),
            State::Map      => draw_map(&blip, &g, &tex),
            State::Dead     => draw_dead(&blip, &g, &tex),
            State::GameOver => draw_gameover(&blip, &g),
        }

        blip.next_frame(60).await;
    }
}
