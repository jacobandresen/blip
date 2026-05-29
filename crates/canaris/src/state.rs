//! Constants, entity types, and the Game struct shared by every scene.

use blip::{rand_int, web};

// ── layout ────────────────────────────────────────────────────────────────────

pub const WIN_W: i32 = 480;
pub const WIN_H: i32 = 540;
pub const HUD_H: i32 = 28;

pub const WORLD_W: f32 = WIN_W as f32 * 4.0;

// ── entity sizes ──────────────────────────────────────────────────────────────

pub const PLAYER_W: f32 = 48.0;
pub const PLAYER_H: f32 = 32.0;
pub const ENEMY_W:  f32 = 48.0;
pub const ENEMY_H:  f32 = 32.0;
pub const BALL_W:   f32 = 12.0;
pub const BALL_H:   f32 = 12.0;

// ── pools ─────────────────────────────────────────────────────────────────────

pub const MAX_ENEMIES:     usize = 4;
pub const MAX_CANNONBALLS: usize = 8;  // first 2 = player, rest = enemy
pub const MAX_EXPLOSIONS:  usize = 12;
pub const BOARDING_SLOTS:  usize = 6;

// ── tuning ────────────────────────────────────────────────────────────────────

pub const LIVES_START:       i32   = 3;
pub const PLAYER_HULL_MAX:   i32   = 20;
pub const PLAYER_CREW_START: i32   = 8;
pub const PLAYER_GOLD_START: i32   = 50;
pub const PLAYER_CANNONS:    i32   = 6;
pub const PLAYER_FOOD_START: i32   = 30;
pub const FOOD_MAX:          i32   = 30;

pub const PLAYER_SPEED:      f32   = 160.0;
pub const SEA_LANE_Y:        f32   = (WIN_H as f32 - HUD_H as f32) * 0.38 + HUD_H as f32;
pub const BOB_AMP:           f32   = 4.0;
pub const BOB_FREQ:          f32   = 1.8;

pub const ENGAGEMENT_DIST:   f32   = WIN_W as f32 * 0.6;
pub const PORT_ANCHOR_X:     f32   = WORLD_W * 0.75;
pub const PORT_DOCK_RADIUS:  f32   = 80.0;  // how close the player must be to press SPACE and dock
pub const PORT_SAFE_RADIUS:  f32   = 400.0; // enemies don't spawn or engage within this distance of port

pub const CANNON_SPEED:      f32   = 280.0;
pub const CANNON_GRAVITY:    f32   = 200.0;
pub const CANNON_ARC_VY:     f32   = 94.0;  // initial upward speed for parabolic arc

pub const MAX_SPLASHES:      usize = 4;
pub const SPLASH_TTL:        f32   = 0.55;
pub const PLAYER_RELOAD:     f32   = 0.8;
pub const ENEMY_RELOAD_BASE: f32   = 2.0;
pub const COMBAT_PLAYER_X:   f32   = 60.0;
pub const COMBAT_ENEMY_X:    f32   = WIN_W as f32 - 60.0 - ENEMY_W;
pub const COMBAT_BASE_Y:     f32   = WIN_H as f32 * 0.42;
pub const RETREAT_TIMER:     f32   = 20.0;
pub const DODGE_SPEED:       f32   = 140.0;
pub const COMBAT_Y_MIN:      f32   = HUD_H as f32 + 10.0;
pub const COMBAT_Y_MAX:      f32   = WIN_H as f32 - ENEMY_H - 52.0; // leaves room for bottom UI
pub const BOARD_Y_DIST:      f32   = 55.0;  // Y proximity required to trigger boarding
pub const COMBAT_UI_Y:       f32   = WIN_H as f32 - 46.0; // top of bottom UI strip

pub const BOARDING_TICK:     f32   = 1.2;
pub const BOARDING_TIMEOUT:  f32   = 30.0;

pub const EXPLOSION_TTL:     f32   = 0.5;
pub const DEAD_TTL:          f32   = 1.8;
pub const ANIM_FRAME_DUR:    f32   = 0.35;
pub const FOOD_DECAY_RATE:   f32   = 0.5; // points/sec
pub const FOOD_HULL_DMG_RATE:f32   = 1.5; // seconds between hull ticks when starving

pub const REPAIR_COST:  i32 = 30;
pub const CREW_COST:    i32 = 20;
pub const CANNON_COST:  i32 = 25;
pub const FOOD_COST:    i32 = 15;

pub const KEY_ENTER: blip::macroquad::input::KeyCode = blip::macroquad::input::KeyCode::Enter;

// ── world map ─────────────────────────────────────────────────────────────────

pub struct MapZone {
    pub name:     &'static str,
    pub desc:     &'static str,
    pub level_eq: i32,
    pub ships:    usize,
    pub map_x:    f32,
    pub map_y:    f32,
    pub stars:    u8,
}

pub const ZONES: [MapZone; 4] = [
    MapZone { name: "DANISH COAST",      desc: "Soft targets on shallow routes", level_eq: 1,  ships: 2, map_x: 190.0, map_y: 420.0, stars: 1 },
    MapZone { name: "KATTEGAT NARROWS",  desc: "Armed merchant convoys",         level_eq: 4,  ships: 3, map_x: 200.0, map_y: 290.0, stars: 2 },
    MapZone { name: "SKAGERRAK PASSAGE", desc: "Royal Navy patrol routes",       level_eq: 7,  ships: 4, map_x: 210.0, map_y: 165.0, stars: 3 },
    MapZone { name: "OPEN NORTH SEA",    desc: "Men-of-war and armed galleons",  level_eq: 11, ships: 4, map_x: 220.0, map_y:  60.0, stars: 4 },
];

// ── state machine ─────────────────────────────────────────────────────────────

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum State { Title, Sea, Combat, Boarding, Port, Map, Dead, GameOver }

// ── entities ──────────────────────────────────────────────────────────────────

#[derive(Copy, Clone)]
pub struct PlayerShip {
    pub world_x: f32,
    pub y:       f32,
    pub vx:      f32,
    pub hull:    i32,
    pub crew:    i32,
    pub gold:    i32,
    pub cannons: i32,
    pub food:    f32,
    pub reload_t: f32,
    pub anim_frame: u8,
    pub anim_t:  f32,
    pub hit_flash_t: f32,
    pub starve_t: f32,  // timer between starvation hull ticks
}

#[derive(Copy, Clone)]
pub struct EnemyShip {
    pub active:   bool,
    pub world_x:  f32,
    pub y:        f32,
    pub hull:     i32,
    pub hull_max: i32,
    pub crew:     i32,
    pub gold_loot:i32,
    pub reload_t: f32,
    pub anim_frame: u8,
    pub anim_t:  f32,
    pub hit_flash_t: f32,
    // combat-screen position (screen space)
    pub combat_y: f32,
    pub combat_vy: f32,
}

#[derive(Copy, Clone)]
pub struct Cannonball {
    pub active: bool,
    pub x: f32, pub y: f32,
    pub vx: f32, pub vy: f32,
    pub player: bool,
}

#[derive(Copy, Clone)]
pub struct Explosion {
    pub active: bool,
    pub x: f32, pub y: f32,
    pub ttl: f32,
}

#[derive(Copy, Clone)]
pub struct Splash {
    pub active: bool,
    pub x: f32, pub y: f32,
    pub ttl: f32,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SlotOwner { Player, Enemy, Empty }

#[derive(Copy, Clone)]
pub struct BoardingSlot {
    pub owner: SlotOwner,
    pub hp:    i32,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PortItem { Sail, Map, Repair, Crew, Cannons, Food }

impl PortItem {
    // Order: Sail → Map → Repair → Crew → Cannons → Food → (wrap) Sail
    pub fn next(self) -> Self {
        match self {
            PortItem::Sail    => PortItem::Map,
            PortItem::Map     => PortItem::Repair,
            PortItem::Repair  => PortItem::Crew,
            PortItem::Crew    => PortItem::Cannons,
            PortItem::Cannons => PortItem::Food,
            PortItem::Food    => PortItem::Sail,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            PortItem::Sail    => PortItem::Food,
            PortItem::Map     => PortItem::Sail,
            PortItem::Repair  => PortItem::Map,
            PortItem::Crew    => PortItem::Repair,
            PortItem::Cannons => PortItem::Crew,
            PortItem::Food    => PortItem::Cannons,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            PortItem::Sail    => "SET SAIL",
            PortItem::Map     => "WORLD MAP",
            PortItem::Repair  => "REPAIR HULL",
            PortItem::Crew    => "HIRE CREW",
            PortItem::Cannons => "BUY CANNONS",
            PortItem::Food    => "BUY PROVISIONS",
        }
    }
    pub fn cost(self) -> i32 {
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
pub struct Game {
    pub state:   State,
    pub time:    f32,

    pub player:  PlayerShip,
    pub cam_x:   f32,

    pub enemies: [EnemyShip; MAX_ENEMIES],
    pub cannonballs: [Cannonball; MAX_CANNONBALLS],
    pub explosions:  [Explosion; MAX_EXPLOSIONS],

    pub combat_enemy_idx: usize,
    pub retreat_t: f32,
    pub splashes: [Splash; MAX_SPLASHES],

    pub slots:           [BoardingSlot; BOARDING_SLOTS],
    pub boarding_t:      f32,
    pub boarding_total_t:f32,
    pub boarding_hit_slot: usize,  // index of last attacked slot (99 = none)
    pub boarding_hit_t:  f32,      // flash timer for that slot

    pub port_cursor:   PortItem,
    pub port_msg:      &'static str,
    pub port_msg_t:    f32,
    pub port_msg_ok:   bool,
    pub map_cursor:    usize,

    pub score:    i32,
    pub hi_score: i32,
    pub lives:    i32,
    pub level:    i32,
    pub level_t:  f32,
    pub dead_t:   f32,
}

impl Game {
    pub fn new() -> Self {
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
            splashes: [Splash { active: false, x: 0.0, y: 0.0, ttl: 0.0 }; MAX_SPLASHES],
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
            score: 0, hi_score: web::load_hi_score(web::GAME_CANARIS),
            lives: LIVES_START,
            level: 1, level_t: 60.0,
            dead_t: 0.0,
        }
    }

    pub fn start_game(&mut self) {
        let hi = self.hi_score.max(web::load_hi_score(web::GAME_CANARIS));
        *self = Game::new();
        self.hi_score = hi;
        self.state = State::Sea;
        self.spawn_enemies();
    }

    pub fn spawn_enemies(&mut self) {
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

    pub fn spawn_enemies_n(&mut self, n: usize) {
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

    pub fn spawn_explosion(&mut self, x: f32, y: f32) {
        for e in self.explosions.iter_mut() {
            if !e.active {
                *e = Explosion { active: true, x, y, ttl: EXPLOSION_TTL };
                return;
            }
        }
    }

    pub fn spawn_splash(&mut self, x: f32, y: f32) {
        for s in self.splashes.iter_mut() {
            if !s.active {
                *s = Splash { active: true, x, y, ttl: SPLASH_TTL };
                return;
            }
        }
    }

    pub fn fire_ball(&mut self, from_player: bool, src_x: f32, src_y: f32) {
        let (start, end) = if from_player { (0, 2) } else { (2, MAX_CANNONBALLS) };
        for i in start..end {
            if !self.cannonballs[i].active {
                let vx = if from_player { CANNON_SPEED } else { -CANNON_SPEED };
                // Upward kick for parabolic arc; slight per-shot jitter for variety
                let vy = -CANNON_ARC_VY + rand_int(-6, 6) as f32;
                self.cannonballs[i] = Cannonball {
                    active: true, x: src_x, y: src_y, vx, vy, player: from_player,
                };
                return;
            }
        }
    }

    pub fn respawn_at_sea(&mut self) {
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

    pub fn enter_combat(&mut self, idx: usize) {
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

    pub fn enter_boarding(&mut self) {
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

    pub fn enter_port(&mut self) {
        self.port_cursor = PortItem::Sail;
        self.port_msg    = "WELCOME TO PORT";
        self.port_msg_t  = 2.0;
        self.state       = State::Port;
    }
}

// ── asset bundles ─────────────────────────────────────────────────────────────

#[allow(dead_code)]
pub struct Sounds {
    pub cannon_fire:    blip::audio::BlipSound,
    pub explosion:      blip::audio::BlipSound,
    pub splash:         blip::audio::BlipSound,
    pub hull_hit:       blip::audio::BlipSound,
    pub boarding_clash: blip::audio::BlipSound,
    pub coin_jingle:    blip::audio::BlipSound,
    pub life_lost:      blip::audio::BlipSound,
    pub sea_music:      blip::audio::BlipSound,
    pub combat_music:   blip::audio::BlipSound,
    pub port_music:     blip::audio::BlipSound,
    pub ocean_ambience: blip::audio::BlipSound,
}

pub struct Textures {
    pub player_a: blip::BlipTex,
    pub player_b: blip::BlipTex,
    pub enemy_a:  blip::BlipTex,
    pub enemy_b:  blip::BlipTex,
    pub ball:     blip::BlipTex,
    pub explosion:blip::BlipTex,
    pub port_bg:  blip::BlipTex,
    pub sea_wave:   blip::BlipTex,
    pub sea_wave_b: blip::BlipTex,
    pub crew:       blip::BlipTex,
    pub map_bg:     blip::BlipTex,
}
