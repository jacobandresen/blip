//! Raider — a 1942-style vertical dogfighting shoot-'em-up.

use blip::input::{
    any_key_pressed, key_held, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_DOWN, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::audio::{play_sound, set_sound_volume, stop_sound, PlaySoundParams};
use blip::macroquad::math::vec2;
use blip::macroquad::prelude::ImageFormat;
use blip::macroquad::rand::rand;
use blip::macroquad::texture::{draw_texture_ex, DrawTextureParams, FilterMode, Texture2D};
use blip::{
    clamp, play_music, play_sfx, pool_iter, pool_iter_mut, pool_spawn, rects_overlap, web,
    window_conf, Blip, BlipColor, LifeResult, Pooled, Session, Timer,
    BLIP_BLACK, BLIP_BLUE, BLIP_CYAN, BLIP_GRAY, BLIP_GREEN, BLIP_ORANGE, BLIP_RED, BLIP_WHITE,
    BLIP_YELLOW,
};

// ---- layout -------------------------------------------------------------
const WIN_W: i32 = 480;
const WIN_H: i32 = 540;
const HUD_H: i32 = 28;

// ---- player ---------------------------------------------------------------
const PLAYER_W: i32 = 36;
const PLAYER_H: i32 = 32;
const PLAYER_SPEED: f32 = 220.0;
const PLAYER_MIN_Y: f32 = (HUD_H + 150) as f32; // player stays out of the HUD band
const PLAYER_MAX_Y: f32 = (WIN_H - 40) as f32;

// ---- weapons ----------------------------------------------------------
// Five weapon tiers, one per power-up capsule caught: wider spreads, faster
// fire, and (from tier 3 up) bigger, glowing bullets. See weapon_spread(),
// weapon_cooldown(), and weapon_bullet_visual() below, which are what
// actually define each tier — these are just the shared building blocks.
const MAX_WEAPON_LEVEL: i32 = 5;
const BULLET_SPEED: f32 = 420.0;
const BULLET_W: f32 = 4.0;
const BULLET_H: f32 = 10.0;
const MAX_PLAYER_BULLETS: usize = 72; // headroom for tier 5's 7-way rapid spread

// ---- enemies ------------------------------------------------------------
const ENEMY_W: i32 = 26;
const ENEMY_H: i32 = 22;
const MAX_ENEMIES: usize = 22;
// Coordinated-turn flight model (see the Enemy doc comment / update_enemies()):
// roll toward a bank proportional to heading error, at a limited roll rate,
// then let the *bank* — not a direct heading clamp — drive the turn rate.
const ENEMY_ROLL_RATE: f32 = 3.4;  // rad/sec the bank angle itself can change by
const ENEMY_MAX_BANK: f32 = 0.95;  // radians (~54°) — steepest bank these fighters pull
const ENEMY_BANK_GAIN: f32 = 1.3;  // how sharply bank responds to heading error
// Rate damping on the bank command — same idea a real autopilot uses (react
// to how fast the heading is already turning, not just how far off it is) —
// so the plane settles onto its new heading instead of overshooting and
// rocking back and forth past it like a pendulum.
const ENEMY_BANK_DAMPING: f32 = 0.55;
const ENEMY_TURN_G: f32 = 120.0;   // tuned coordinated-turn constant: turn_rate = G*tan(bank)/speed
const ENEMY_EDGE_MARGIN: f32 = 18.0; // keeps flight paths clear of the corners — see update_enemies()
const ENEMY_BULLET_SPEED: f32 = 220.0;
const ENEMY_BULLET_W: f32 = 4.0;
const ENEMY_BULLET_H: f32 = 10.0;
const MAX_ENEMY_BULLETS: usize = 22;
// Waves are *much* longer than the original tuning — see wave_target_for()
// / spawn_interval_range() for the exact per-level curve.
const WAVE_KILL_BASE: i32 = 165; // 3x the previous tuning — levels run a lot longer
const SPAWN_MIN: f32 = 0.29;
const SPAWN_MAX: f32 = 0.66;

// ---- boss ---------------------------------------------------------------
// One boss per wave, levels 1-7 — see BOSS_SPECS for the full escalation
// (size, HP, fire pattern, speed). Sizes must match blip_assets' BOSS_SIZES.
const MAX_LEVEL: i32 = 7;
const BOSS_SIZES: [(i32, i32); 7] = [
    (72, 50), (84, 58), (98, 68), (114, 80), (132, 92), (152, 106), (176, 124),
];
const BOSS_INTRO_TIME: f32 = 1.3;

// ---- carrier launch -------------------------------------------------------
// Every level (and every new game) opens with the player's plane climbing
// away from a carrier at the bottom of the screen instead of just appearing.
// Size must match blip_assets' CARRIER_W / CARRIER_H.
const CARRIER_W: i32 = 108;
const CARRIER_H: i32 = 190;
const LAUNCH_TIME: f32 = 5.2;

// ---- power-up -----------------------------------------------------------
const MAX_POWERUPS: usize = 2;
const POW_W: f32 = 14.0;
const POW_H: f32 = 14.0;
const POW_SPEED: f32 = 90.0;

// ---- health -----------------------------------------------------------
// The plane now survives more than one stray shot: a 5-point health bar
// instead of an instant "any hit is a death". A life is only lost once
// health runs out, and a rare pickup (dropped by regular fighters, never
// the ace — it always drops a weapon tier instead) refills some of it
// back, falling the same way a weapon power-up does.
const PLAYER_HEALTH_MAX: i32 = 5;
const HEALTH_RESTORE: i32 = 2;
const HEALTH_DROP_CHANCE: f32 = 0.035;
const HEALTH_W: f32 = 14.0;
const HEALTH_H: f32 = 14.0;
const MAX_HEALTH_PICKUPS: usize = 1;
// A short flinch of invulnerability after a non-lethal hit, reusing the
// same respawn-grace timer (and its blink) that already gates hazard
// collisions — otherwise a single burst of overlapping bullets could burn
// through the whole health bar in one frame.
const HIT_GRACE: f32 = 0.9;

// ---- background: sea, sky, boats -------------------------------------------
// The world below the dogfight: an ocean scrolling past underneath (both the
// wave bands and the boats sit on it, so they share one scroll speed), a
// cloud layer floating between the sea and the planes, and enemy boats that
// sail across it.
const SEA_SCROLL_SPEED: f32 = 70.0;
const MAX_CLOUDS: usize = 8;
const BOAT_W: i32 = 34;
const BOAT_H: i32 = 16;
const MAX_BOATS: usize = 3;
// 2 1/2 D depth cue: planes fly *above* the sea, so they drop a soft dark
// silhouette (the same sprite, tinted and offset) onto the water beneath
// them — the classic shmup trick for reading altitude on a flat top-down
// scene. Offset toward lower-right, as if lit from the upper-left.
const PLANE_SHADOW_DX: f32 = 6.0;
const PLANE_SHADOW_DY: f32 = 10.0;

// ---- islands & turrets ----------------------------------------------------
// A rare hazard: a small island drifts down with the sea current, armed with
// a turret that tracks and fires an aimed shot at the player. Sizes/HP/score
// scale together (small/medium/large); at most one is ever on screen, and
// they show up only every 24-42s, so it stays a rare set-piece, not a wave
// enemy. Sizes must match blip_assets' ISLAND_SIZES.
const ISLAND_SIZES: [(i32, i32); 3] = [(64, 44), (98, 68), (140, 96)];
const ISLAND_HP: [i32; 3] = [26, 46, 74];
const ISLAND_SCORE: [i32; 3] = [150, 260, 420];
const MAX_ISLANDS: usize = 1;
const ISLAND_MIN_INTERVAL: f32 = 24.0;
const ISLAND_MAX_INTERVAL: f32 = 42.0;
const TURRET_FIRE_MIN: f32 = 1.3;
const TURRET_FIRE_MAX: f32 = 2.3;
const TURRET_BULLET_SPEED: f32 = 130.0;
const TURRET_BULLET_W: f32 = 6.0;
const TURRET_BULLET_H: f32 = 6.0;
const MAX_TURRET_BULLETS: usize = 8;

// ---- laser barrier ----------------------------------------------------
// A seldom, higher-level set-piece from level BARRIER_MIN_LEVEL on: a laser
// beam spans the full width of the screen — there's no dodging around it,
// only through it — powered by a "motor" the player can shoot down (a
// random 3-10 hits) to shut the beam off. Checked for on a long, rare timer,
// and never while a boss is already up.
const BARRIER_MIN_LEVEL: i32 = 3;
const BARRIER_MIN_INTERVAL: f32 = 75.0;
const BARRIER_MAX_INTERVAL: f32 = 140.0;
const BARRIER_Y: f32 = 230.0;      // fixed row the beam sits on
const BARRIER_BEAM_H: f32 = 10.0;  // collision + visual thickness of the beam
const BARRIER_WARMUP: f32 = 1.6;   // telegraph before the beam can actually hurt you
const BARRIER_HP_MIN: i32 = 3;
const BARRIER_HP_MAX: i32 = 10;    // inclusive — a random 3-10 hits to destroy
const MOTOR_W: f32 = 34.0;
const MOTOR_H: f32 = 26.0;
// How close (in px of vertical distance) the proximity hum starts fading
// in, and its loudest volume once the player is right on top of the beam.
const BARRIER_HUM_RANGE: f32 = 220.0;
const BARRIER_HUM_MAX_VOLUME: f32 = 0.5;

// ---- explosions -----------------------------------------------------------
const MAX_EXPLOSIONS: usize = MAX_ENEMIES + 16; // + headroom for power-up bursts
const EXPLOSION_TTL: f32 = 0.4;
const MAX_POWER_BANNER_TIME: f32 = 1.6;

// ---- tuning -------------------------------------------------------------
const LIVES_START: i32 = 3;
const DEAD_PAUSE: f32 = 1.6;
const RESPAWN_GRACE: f32 = 1.0;
const WIN_PAUSE: f32 = 2.2;
// A hard floor on how long GAME OVER stays up before a key can dismiss it —
// without this, a player still mashing fire from the fight that killed them
// bounces straight back to the title screen without the score ever
// registering.
const OVER_MIN_WAIT: f32 = 3.0;

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Launch, Play, Dead, Win, Won, Over }

#[derive(Copy, Clone, PartialEq, Eq)]
enum EnemyKind { Grunt, Weaver, Ace }

#[derive(Copy, Clone)]
struct Bullet { x: f32, y: f32, active: bool }
impl Pooled for Bullet {
    fn is_active(&self) -> bool { self.active }
}

/// Enemies fly on a small coordinated-turn model instead of a position
/// formula or a direct heading clamp — the way a real aircraft actually
/// turns: the pilot rolls into a bank, and it's the bank's lift component
/// that curves the flight path (turn_rate = G * tan(bank) / airspeed is the
/// standard relationship: https://skybrary.aero/articles/rate-turn). So each
/// plane tracks two angles, not one:
///   - `bank`   — current roll, chases a "desired bank" (steeper for a
///     bigger heading error, like correcting toward a target course) at a
///     limited roll rate — a plane can't snap-roll instantly either.
///   - `heading` — direction of travel, driven by the *current* bank via
///     that turn-rate relationship, not set directly.
/// See update_enemies() for the per-tick integration. The sprite is drawn
/// rotated to `heading`, so the nose always points where the plane is
/// actually going, and the curve into a turn comes from the bank lag, not
/// from clamping the heading change itself.
#[derive(Copy, Clone)]
struct Enemy {
    x: f32, y: f32,
    heading: f32,
    bank: f32,
    active: bool,
    kind: EnemyKind,
    t: f32,             // seconds alive, drives the weave phase and fire cadence
    flight_quirk: f32,  // Grunt: its fixed target heading; Weaver: sine phase offset; Ace: unused
    fire_timer: Timer,
    can_hide: bool,     // this plane ducks out of sight when it flies under a cloud — see draw_play()
}
impl Pooled for Enemy {
    fn is_active(&self) -> bool { self.active }
}

#[derive(Copy, Clone)]
struct Explosion { x: f32, y: f32, ttl: f32, max_ttl: f32, scale: f32, color: BlipColor, active: bool }
impl Pooled for Explosion {
    fn is_active(&self) -> bool { self.active }
}

/// Standard fireball orange, the default for combat explosions.
const EXPLOSION_ORANGE: BlipColor = BlipColor { r: 1.0, g: 0.6, b: 0.15, a: 1.0 };

#[derive(Copy, Clone)]
struct Powerup { x: f32, y: f32, active: bool }
impl Pooled for Powerup {
    fn is_active(&self) -> bool { self.active }
}

#[derive(Copy, Clone)]
struct HealthPickup { x: f32, y: f32, active: bool }
impl Pooled for HealthPickup {
    fn is_active(&self) -> bool { self.active }
}

#[derive(Copy, Clone)]
struct Cloud { x: f32, y: f32, r: f32, speed: f32, variant: u8 }

/// An enemy boat sailing across the sea far below the dogfight — scrolls
/// down with the water (SEA_SCROLL_SPEED) plus its own slow lateral cruise.
/// Shootable for a small bonus; doesn't fire back. `bob_phase` offsets each
/// boat's rocking-with-the-swell cycle so a flotilla doesn't bob in unison.
#[derive(Copy, Clone)]
struct Boat { x: f32, y: f32, active: bool, vx: f32, bob_phase: f32 }
impl Pooled for Boat {
    fn is_active(&self) -> bool { self.active }
}

/// A rare landmass drifting down through the sea with the current, armed
/// with a turret — shootable for a bonus like the boats, but unlike them it
/// shoots back: the turret tracks and fires an aimed shot at the player on
/// its own timer. `size` indexes ISLAND_SIZES/HP/SCORE (small/medium/large).
#[derive(Copy, Clone)]
struct Island { x: f32, y: f32, size: u8, active: bool, hp: i32, fire_timer: Timer }
impl Pooled for Island {
    fn is_active(&self) -> bool { self.active }
}

/// A turret shell. Unlike the enemy planes' straight-down bullets, this one
/// carries its own velocity, set once at the moment it's fired (aimed at the
/// player then, not homing afterward — a real shell doesn't course-correct).
#[derive(Copy, Clone)]
struct TurretBullet { x: f32, y: f32, vx: f32, vy: f32, active: bool }
impl Pooled for TurretBullet {
    fn is_active(&self) -> bool { self.active }
}

/// A laser barrier gating the full width of the screen at `BARRIER_Y`, and
/// the "motor" powering it — the only part of it that's shootable. Only one
/// is ever up at a time, so it's a plain struct rather than a pool.
#[derive(Copy, Clone)]
struct Barrier {
    active: bool,
    motor_x: f32,
    hp: i32,
    max_hp: i32,
    warmup: Timer, // telegraph: on screen but harmless and can't be shot yet
    t: f32,        // seconds since spawn, drives the beam's flicker
}

#[derive(Copy, Clone)]
struct Boss {
    x: f32, y: f32,
    active: bool,
    entered: bool, // finished its entrance descent, patrolling now
    hp: i32, max_hp: i32,
    dir: f32,
    tier: usize, // 0..=6, indexes BOSS_SPECS / BOSS_SIZES (level - 1)
    t: f32,      // seconds since spawn, drives the dip wiggle and the sweep pattern
    volley: u32, // volleys fired so far — cycles spec.patterns and seeds Curtain's gap
    fire_timer: Timer,
    escort_timer: Timer,
}

/// A volley shape `boss_fire()` can pick — layered in as bosses get tougher
/// so a plain evenly-spaced fan, easy to read and dodge once you've seen
/// it, isn't the whole fight anymore.
#[derive(Copy, Clone, PartialEq)]
enum BossPattern {
    /// `spec.bullets` shots spread evenly across `spec.fan_w`, centred
    /// under the boss — the original, simplest volley.
    Fan,
    /// Same spread, but centred on the player's current x instead of the
    /// boss's — the gap moves with the player, so parking in one spot
    /// under the boss stops being safe.
    Aimed,
    /// Same spread again, but its centre sweeps side to side over
    /// successive volleys instead of tracking anything — a searchlight
    /// pass across the whole width rather than a fixed gap.
    Sweep,
    /// A dense band across most of the play width with a single gap that
    /// moves each volley — find the gap and thread it, rather than
    /// dodging discrete shots.
    Curtain,
}

/// One row per boss (levels 1-7): bigger, tougher, and meaner than the last.
/// `bullets`/`fan_w` describe the fan of shots fired each volley (`patterns`
/// decides how that fan is aimed/moved — cycled round-robin, one per
/// volley); `dips` makes the boss periodically sink toward the player
/// instead of holding a flat patrol line; `escorts` has it call in a
/// fighter every few seconds.
struct BossSpec {
    hp: i32,
    speed: f32,
    fire_min: f32,
    fire_max: f32,
    bullets: i32,
    fan_w: f32,
    dips: bool,
    escorts: bool,
    patterns: &'static [BossPattern],
    name: &'static str,
}

const BOSS_SPECS: [BossSpec; 7] = [
    BossSpec { hp:  80, speed:  76.0, fire_min: 0.48, fire_max: 1.00, bullets:  3, fan_w:  80.0, dips: false, escorts: false, patterns: &[BossPattern::Fan],                                                name: "SCOUT BOMBER"   },
    BossSpec { hp: 130, speed:  84.0, fire_min: 0.44, fire_max: 0.92, bullets:  5, fan_w: 100.0, dips: false, escorts: false, patterns: &[BossPattern::Fan],                                                name: "INTERCEPTOR"    },
    BossSpec { hp: 190, speed:  92.0, fire_min: 0.38, fire_max: 0.82, bullets:  5, fan_w: 120.0, dips: true,  escorts: false, patterns: &[BossPattern::Fan, BossPattern::Aimed],                            name: "GUNSHIP"        },
    BossSpec { hp: 260, speed: 100.0, fire_min: 0.34, fire_max: 0.72, bullets:  7, fan_w: 150.0, dips: true,  escorts: false, patterns: &[BossPattern::Fan, BossPattern::Aimed, BossPattern::Sweep],       name: "DREADNOUGHT"    },
    BossSpec { hp: 340, speed: 109.0, fire_min: 0.30, fire_max: 0.64, bullets:  7, fan_w: 170.0, dips: true,  escorts: true,  patterns: &[BossPattern::Aimed, BossPattern::Sweep],                         name: "BATTLE CRUISER" },
    BossSpec { hp: 430, speed: 118.0, fire_min: 0.27, fire_max: 0.58, bullets:  9, fan_w: 200.0, dips: true,  escorts: true,  patterns: &[BossPattern::Sweep, BossPattern::Aimed, BossPattern::Curtain],   name: "DOOM CARRIER"   },
    BossSpec { hp: 560, speed: 132.0, fire_min: 0.22, fire_max: 0.47, bullets: 11, fan_w: 240.0, dips: true,  escorts: true,  patterns: &[BossPattern::Fan, BossPattern::Aimed, BossPattern::Sweep, BossPattern::Curtain], name: "APEX DESTROYER" },
];

fn boss_size(tier: usize) -> (f32, f32) {
    let (w, h) = BOSS_SIZES[tier];
    (w as f32, h as f32)
}

struct Game {
    player_x: f32,
    player_y: f32,
    player_bank: f32,   // cosmetic roll while strafing — eased toward a target, not instant
    ship_y: f32,        // carrier position during the launch sequence
    launch_timer: Timer,
    weapon_level: i32,
    health: i32,
    bullets: [Bullet; MAX_PLAYER_BULLETS],
    enemy_bullets: [Bullet; MAX_ENEMY_BULLETS],
    enemies: [Enemy; MAX_ENEMIES],
    explosions: [Explosion; MAX_EXPLOSIONS],
    powerups: [Powerup; MAX_POWERUPS],
    health_pickups: [HealthPickup; MAX_HEALTH_PICKUPS],
    clouds: [Cloud; MAX_CLOUDS],
    boats: [Boat; MAX_BOATS],
    boat_timer: Timer,
    islands: [Island; MAX_ISLANDS],
    island_timer: Timer,
    turret_bullets: [TurretBullet; MAX_TURRET_BULLETS],
    sea_scroll: f32,
    barrier: Barrier,
    barrier_timer: Timer,
    boss: Boss,
    sess: Session,
    state: State,
    fire_cd: Timer,
    spawn_timer: Timer,
    wave_kills: i32,
    wave_target: i32,
    dead_timer: Timer,
    win_timer: Timer,
    over_timer: Timer,
    respawn_grace: Timer,
    max_power_banner: Timer,
    boss_intro: Timer,
}

fn wave_target_for(level: i32) -> i32 {
    (WAVE_KILL_BASE + (level - 1) * 21).min(330)
}

fn spawn_interval_range(level: i32) -> (f32, f32) {
    let l = (level - 1).min(6) as f32;
    ((SPAWN_MIN - l * 0.025).max(0.18), (SPAWN_MAX - l * 0.055).max(0.42))
}

fn rand01() -> f32 {
    (rand() as f32) / (u32::MAX as f32)
}

// ---- weapon tiers ---------------------------------------------------------
// One place per property, all indexed by weapon_level (1..=MAX_WEAPON_LEVEL),
// so the escalation from "single popgun" to "seven-way glowing barrage" stays
// easy to tune as one coherent ladder.

/// Bullet spawn offsets from the player's centreline, one shot's worth.
fn weapon_spread(level: i32) -> &'static [f32] {
    match level {
        1 => &[0.0],
        2 => &[-12.0, 0.0, 12.0],
        3 => &[-22.0, -10.0, 0.0, 10.0, 22.0],
        4 => &[-24.0, -12.0, 0.0, 12.0, 24.0],
        _ => &[-30.0, -20.0, -10.0, 0.0, 10.0, 20.0, 30.0], // 5: full barrage
    }
}

fn weapon_cooldown(level: i32) -> f32 {
    match level {
        1 => 0.16,
        2 => 0.16,
        3 => 0.15,
        4 => 0.13,
        _ => 0.11,
    }
}

/// The colour a given tier reads as everywhere: the bullets it fires, the
/// pickup flash on catching it, and (implicitly, one tier ahead) the tint on
/// a falling capsule hinting at what it's about to grant.
fn weapon_tier_color(level: i32) -> BlipColor {
    match level {
        1 | 2 => BLIP_YELLOW,
        3 => BLIP_CYAN,
        4 => BLIP_ORANGE,
        _ => BlipColor::new(1.0, 0.85, 0.25, 1.0), // 5: gold
    }
}

/// (visual width scale, use a glowing circle instead of a plain rect).
/// Higher tiers get chunkier, glowing bolts — a plain rect stops reading as
/// "more powerful" past a certain size, a glow keeps escalating.
fn weapon_bullet_visual(level: i32) -> (f32, bool) {
    match level {
        1 | 2 => (1.0, false),
        3 => (1.2, false),
        4 => (1.4, true),
        _ => (1.8, true), // 5
    }
}

impl Game {
    fn new() -> Self {
        let dead_bullet = Bullet { x: 0.0, y: 0.0, active: false };
        let dead_enemy = Enemy {
            x: 0.0, y: 0.0, heading: 0.0, bank: 0.0, active: false, kind: EnemyKind::Grunt,
            t: 0.0, flight_quirk: 0.0, fire_timer: Timer::default(), can_hide: false,
        };
        let dead_explosion = Explosion { x: 0.0, y: 0.0, ttl: 0.0, max_ttl: EXPLOSION_TTL, scale: 1.0, color: EXPLOSION_ORANGE, active: false };
        let dead_powerup = Powerup { x: 0.0, y: 0.0, active: false };
        let dead_health_pickup = HealthPickup { x: 0.0, y: 0.0, active: false };

        // Two parallax layers of clouds, seeded at deterministic (not random —
        // rand() needs macroquad running) spread-out positions; they wrap and
        // look randomized within a second or two of play.
        let mut clouds = [Cloud { x: 0.0, y: 0.0, r: 0.0, speed: 0.0, variant: 0 }; MAX_CLOUDS];
        for (i, c) in clouds.iter_mut().enumerate() {
            let far = i % 3 != 0;
            c.x = (i as f32 * 71.0) % WIN_W as f32;
            c.y = (i as f32 * 53.0) % WIN_H as f32;
            c.r = if far { 22.0 } else { 36.0 };
            c.speed = if far { 24.0 } else { 48.0 };
            c.variant = (i % 3) as u8;
        }

        Self {
            player_x: ((WIN_W - PLAYER_W) / 2) as f32,
            player_y: PLAYER_MAX_Y,
            player_bank: 0.0,
            ship_y: (WIN_H + CARRIER_H) as f32,
            launch_timer: Timer::default(),
            weapon_level: 1,
            health: PLAYER_HEALTH_MAX,
            bullets: [dead_bullet; MAX_PLAYER_BULLETS],
            enemy_bullets: [dead_bullet; MAX_ENEMY_BULLETS],
            enemies: [dead_enemy; MAX_ENEMIES],
            explosions: [dead_explosion; MAX_EXPLOSIONS],
            powerups: [dead_powerup; MAX_POWERUPS],
            health_pickups: [dead_health_pickup; MAX_HEALTH_PICKUPS],
            clouds,
            boats: [Boat { x: 0.0, y: 0.0, active: false, vx: 0.0, bob_phase: 0.0 }; MAX_BOATS],
            boat_timer: { let mut t = Timer::default(); t.start(3.0); t },
            islands: [Island { x: 0.0, y: 0.0, size: 0, active: false, hp: 0, fire_timer: Timer::default() }; MAX_ISLANDS],
            island_timer: { let mut t = Timer::default(); t.start(14.0); t }, // first one shows up a bit into the level, not instantly
            turret_bullets: [TurretBullet { x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, active: false }; MAX_TURRET_BULLETS],
            sea_scroll: 0.0,
            barrier: Barrier {
                active: false, motor_x: 0.0, hp: 0, max_hp: 0,
                warmup: Timer::default(), t: 0.0,
            },
            barrier_timer: { let mut t = Timer::default(); t.start(BARRIER_MIN_INTERVAL); t },
            boss: Boss {
                x: 0.0, y: 0.0, active: false, entered: false,
                hp: 0, max_hp: 0, dir: 1.0, tier: 0, t: 0.0, volley: 0,
                fire_timer: Timer::default(), escort_timer: Timer::default(),
            },
            sess: Session::new(LIVES_START),
            state: State::Title,
            fire_cd: Timer::default(),
            spawn_timer: Timer::default(),
            wave_kills: 0,
            wave_target: wave_target_for(1),
            dead_timer: Timer::default(),
            win_timer: Timer::default(),
            over_timer: Timer::default(),
            respawn_grace: Timer::default(),
            max_power_banner: Timer::default(),
            boss_intro: Timer::default(),
        }
    }

    fn spawn_explosion(&mut self, x: f32, y: f32, scale: f32, color: BlipColor) {
        pool_spawn(&mut self.explosions, Explosion { x, y, ttl: EXPLOSION_TTL, max_ttl: EXPLOSION_TTL, scale, color, active: true });
    }

    /// A small multi-burst for the player's own plane going down — reads
    /// bigger and more dramatic than a single puff.
    fn spawn_player_death(&mut self, x: f32, y: f32) {
        const BURSTS: [(f32, f32, f32); 4] = [(0.0, 0.0, 2.2), (-10.0, -6.0, 1.5), (10.0, -4.0, 1.5), (0.0, 8.0, 1.6)];
        for (dx, dy, scale) in BURSTS {
            self.spawn_explosion(x + dx, y + dy, scale, EXPLOSION_ORANGE);
        }
    }

    /// A ring of colour-tinted sparks plus a bright central flash around a
    /// power-up catch — more particles and a bigger flash at higher weapon
    /// tiers, so maxing out the gun actually looks like an event.
    fn spawn_powerup_burst(&mut self, x: f32, y: f32, level: i32, color: BlipColor) {
        let n = (2 + level).min(8);
        let dist = 10.0 + level as f32 * 2.5;
        for i in 0..n {
            let ang = (i as f32 / n as f32) * std::f32::consts::PI * 2.0;
            let (dx, dy) = (ang.cos() * dist, ang.sin() * dist);
            self.spawn_explosion(x + dx, y + dy, 0.8 + level as f32 * 0.12, color);
        }
        self.spawn_explosion(x, y, 1.3 + level as f32 * 0.22, color);
    }

    fn reset_player(&mut self) {
        self.player_x = ((WIN_W - PLAYER_W) / 2) as f32;
        self.player_y = PLAYER_MAX_Y;
    }

    /// Fresh wave: clears all entities and (re)arms the spawner. Used both for
    /// a brand-new game and for the stage transition after a boss kill — the
    /// caller decides whether `sess` gets reset first.
    fn start_round(&mut self) {
        self.weapon_level = 1;
        self.health = PLAYER_HEALTH_MAX;
        for b in self.bullets.iter_mut() { b.active = false; }
        for b in self.enemy_bullets.iter_mut() { b.active = false; }
        for e in self.enemies.iter_mut() { e.active = false; }
        for p in self.powerups.iter_mut() { p.active = false; }
        for h in self.health_pickups.iter_mut() { h.active = false; }
        for e in self.explosions.iter_mut() { e.active = false; }
        for b in self.turret_bullets.iter_mut() { b.active = false; }
        self.boss.active = false;
        self.barrier.active = false;
        self.wave_kills = 0;
        self.wave_target = wave_target_for(self.sess.level);
        self.spawn_timer.start(1.0);
        self.max_power_banner = Timer::default();
        self.boss_intro = Timer::default();
        if !self.boat_timer.active() { self.boat_timer.start(3.0); }
        if !self.island_timer.active() { self.island_timer.start(ISLAND_MIN_INTERVAL); }
        if !self.barrier_timer.active() { self.barrier_timer.start(BARRIER_MIN_INTERVAL); }

        // Launch sequence: parked on the carrier deck, climbing away from it
        // into the fight. update_launch() drives player_x/ship_y from here.
        self.player_x = ((WIN_W - PLAYER_W) / 2) as f32;
        self.player_y = (WIN_H - 60) as f32;
        self.ship_y = (WIN_H - CARRIER_H / 2) as f32;
        self.launch_timer.start(LAUNCH_TIME);
        self.state = State::Launch;
    }

    fn start_game(&mut self) {
        self.sess.reset(LIVES_START);
        self.start_round();
    }

    /// Just after a death: put the player back without touching the wave in
    /// progress (enemies and the boss, if any, keep going).
    fn respawn(&mut self) {
        self.reset_player();
        self.health = PLAYER_HEALTH_MAX;
        self.respawn_grace.start(RESPAWN_GRACE);
        self.state = State::Play;
    }
}

struct Sounds {
    shoot: blip::BlipSound,
    enemy_explode: blip::BlipSound,
    player_explode: blip::BlipSound,
    player_hit: blip::BlipSound,
    boss_explode: blip::BlipSound,
    boss_warning: blip::BlipSound,
    // Escalating weapon-tier pickup chimes: index 0 = reaching tier 2, ...,
    // index 3 = reaching tier 5 (the big fanfare). Index 0 is also reused for
    // the "already maxed, bonus points" catch.
    powerup_up: [blip::BlipSound; 4],
    health_pickup: blip::BlipSound,
    stage_clear: blip::BlipSound,
    victory: blip::BlipSound,
    game_over: blip::BlipSound,
    turret_fire: blip::BlipSound,
    // Looped and volume-ridden live by update_barrier() — not a one-shot.
    barrier_hum: blip::BlipSound,
}

fn spawn_boat(g: &mut Game) {
    let x = 20.0 + rand01() * (WIN_W as f32 - BOAT_W as f32 - 40.0);
    let vx = (rand01() - 0.5) * 30.0; // slow lateral cruise, +/- 15 px/s
    let bob_phase = rand01() * std::f32::consts::TAU;
    pool_spawn(&mut g.boats, Boat { x, y: -(BOAT_H as f32), active: true, vx, bob_phase });
}

fn spawn_island(g: &mut Game) {
    let size = (rand() % ISLAND_SIZES.len() as u32) as u8;
    let (w, h) = ISLAND_SIZES[size as usize];
    let x = 10.0 + rand01() * (WIN_W as f32 - w as f32 - 20.0);
    let mut fire_timer = Timer::default();
    fire_timer.start(1.6 + rand01()); // a moment to scroll fully into view before it opens up
    pool_spawn(&mut g.islands, Island { x, y: -(h as f32), size, active: true, hp: ISLAND_HP[size as usize], fire_timer });
}

/// Arm a fresh laser barrier: a random motor position and a random 3-10 HP,
/// then re-arm `barrier_timer` for the next (seldom) one.
fn spawn_barrier(g: &mut Game, sfx: &Sounds) {
    let span = (BARRIER_HP_MAX - BARRIER_HP_MIN + 1) as f32;
    let hp = (BARRIER_HP_MIN as f32 + rand01() * span) as i32;
    let hp = hp.clamp(BARRIER_HP_MIN, BARRIER_HP_MAX);
    let motor_x = MOTOR_W / 2.0 + 20.0 + rand01() * (WIN_W as f32 - MOTOR_W - 40.0);
    let mut warmup = Timer::default();
    warmup.start(BARRIER_WARMUP);
    g.barrier = Barrier { active: true, motor_x, hp, max_hp: hp, warmup, t: 0.0 };
    g.barrier_timer.start(BARRIER_MIN_INTERVAL + rand01() * (BARRIER_MAX_INTERVAL - BARRIER_MIN_INTERVAL));
    // Starts silent; update_barrier() fades it in/out by proximity every frame.
    play_sound(&sfx.barrier_hum, PlaySoundParams { looped: true, volume: 0.0 });
}

/// Laser barrier upkeep: ticks its warmup/flicker clock, and fades a
/// proximity hum in and out by how close the player currently is to the
/// beam — the "block the entire screen" hazard is heard coming before it's
/// close enough to hurt, and gets more insistent the nearer the player
/// flies to it.
fn update_barrier(g: &mut Game, dt: f32, sfx: &Sounds) {
    if !g.barrier.active {
        // Guards against a lingering hum if the barrier was ever cleared out
        // from under it (e.g. a level transition), not just destroyed normally.
        stop_sound(&sfx.barrier_hum);
        return;
    }
    g.barrier.t += dt;
    g.barrier.warmup.tick(dt);

    let player_cy = g.player_y + PLAYER_H as f32 / 2.0;
    let dist = (player_cy - BARRIER_Y).abs();
    let k = (1.0 - dist / BARRIER_HUM_RANGE).clamp(0.0, 1.0);
    set_sound_volume(&sfx.barrier_hum, k * BARRIER_HUM_MAX_VOLUME);
}

/// The world under the dogfight: sea scroll, cloud drift, and boat/island
/// spawns — shared by update_launch() and update_play() so the background
/// keeps moving through the carrier takeoff too, not just once the fight
/// starts. Turret *firing* is play-only (see update_islands()) — islands
/// still drift past harmlessly during launch, same as the boats.
fn update_background(g: &mut Game, dt: f32) {
    g.sea_scroll += SEA_SCROLL_SPEED * dt;

    for c in g.clouds.iter_mut() {
        c.y += c.speed * dt;
        if c.y - c.r > WIN_H as f32 {
            c.y = -c.r;
            c.x = rand01() * WIN_W as f32;
        }
    }

    for b in pool_iter_mut(&mut g.boats) {
        b.y += SEA_SCROLL_SPEED * dt;
        b.x += b.vx * dt;
        if b.y - BOAT_H as f32 > WIN_H as f32
            || b.x < -(BOAT_W as f32) - 20.0
            || b.x > WIN_W as f32 + 20.0
        {
            b.active = false;
        }
    }
    if g.boat_timer.tick(dt) {
        spawn_boat(g);
        g.boat_timer.start(4.0 + rand01() * 4.0);
    }

    for isl in pool_iter_mut(&mut g.islands) {
        isl.y += SEA_SCROLL_SPEED * dt;
        if isl.y - ISLAND_SIZES[isl.size as usize].1 as f32 > WIN_H as f32 {
            isl.active = false;
        }
    }
    if g.island_timer.tick(dt) {
        spawn_island(g);
        g.island_timer.start(ISLAND_MIN_INTERVAL + rand01() * (ISLAND_MAX_INTERVAL - ISLAND_MIN_INTERVAL));
    }
}

/// Turret AI + shell flight — play-only (see update_background() for why).
/// Each island's turret tracks the player's current position and fires an
/// aimed shell at it on its own timer once it's scrolled fully into view.
fn update_islands(g: &mut Game, dt: f32, sfx: &Sounds) {
    let (px, py) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0);
    for isl in pool_iter_mut(&mut g.islands) {
        if isl.y < 0.0 { continue; } // still scrolling in from off the top
        let (w, h) = ISLAND_SIZES[isl.size as usize];
        if isl.fire_timer.tick(dt) {
            let tx = isl.x + w as f32 * 0.5;
            let ty = isl.y + h as f32 * 0.42; // the turret's baked position — see island_sprite()
            let (dx, dy) = (px - tx, py - ty);
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            pool_spawn(&mut g.turret_bullets, TurretBullet {
                x: tx, y: ty, vx: dx / len * TURRET_BULLET_SPEED, vy: dy / len * TURRET_BULLET_SPEED, active: true,
            });
            play_sfx(&sfx.turret_fire);
            isl.fire_timer.start(TURRET_FIRE_MIN + rand01() * (TURRET_FIRE_MAX - TURRET_FIRE_MIN));
        }
    }
    for tb in pool_iter_mut(&mut g.turret_bullets) {
        tb.x += tb.vx * dt;
        tb.y += tb.vy * dt;
        if tb.y < -20.0 || tb.y > WIN_H as f32 + 20.0 || tb.x < -20.0 || tb.x > WIN_W as f32 + 20.0 {
            tb.active = false;
        }
    }
}

/// `formation_angle`, when given, overrides the plane's random flight_quirk
/// with a shared value instead — the mechanism that makes a group of planes
/// spawned together actually hold formation (see spawn_wave_tick()): since
/// desired_heading is derived purely from flight_quirk and the shared clock
/// `t`, planes that start together with the *same* flight_quirk fly the
/// exact same heading trajectory every frame after, so the lateral spacing
/// they spawned with never drifts. A lone plane instead gets its own random
/// value, so solo spawns still look varied.
fn spawn_enemy(g: &mut Game, kind: EnemyKind, x: f32, formation_angle: Option<f32>) {
    let mut fire_timer = Timer::default();
    if kind != EnemyKind::Weaver {
        fire_timer.start(0.6 + rand01() * 1.2);
    }
    let flight_quirk = match kind {
        // Grunt: target heading — up to ~±31 degrees off straight down.
        EnemyKind::Grunt  => formation_angle.unwrap_or_else(|| (rand01() - 0.5) * 1.1),
        // Weaver: sine phase.
        EnemyKind::Weaver => formation_angle.unwrap_or_else(|| rand01() * std::f32::consts::TAU),
        EnemyKind::Ace    => 0.0,
    };
    // Aces are mid-dogfight with the player and stay in view; Grunts and
    // Weavers randomly get to duck through a cloud on the way past.
    let can_hide = kind != EnemyKind::Ace && rand01() < 0.4;
    pool_spawn(&mut g.enemies, Enemy {
        x, y: -(ENEMY_H as f32), heading: 0.0, bank: 0.0, active: true, kind, t: 0.0, flight_quirk, fire_timer, can_hide,
    });
}

/// One spawner tick: usually a lone plane, occasionally a coordinated
/// formation — a V of weavers sharing one weave phase, or a line-abreast
/// squadron of grunts sharing one target heading — so the group actually
/// holds its shape as it flies instead of each plane going its own way.
fn spawn_wave_tick(g: &mut Game) {
    let lo = ENEMY_EDGE_MARGIN;
    let hi = (WIN_W - ENEMY_W) as f32 - ENEMY_EDGE_MARGIN;
    match rand() % 12 {
        0 => {
            let cx = 40.0 + rand01() * (WIN_W as f32 - 80.0 - ENEMY_W as f32);
            let phase = rand01() * std::f32::consts::TAU;
            for dx in [-46.0_f32, 0.0, 46.0] {
                spawn_enemy(g, EnemyKind::Weaver, (cx + dx).clamp(lo, hi), Some(phase));
            }
        }
        1 => {
            let cx = 60.0 + rand01() * (WIN_W as f32 - 120.0 - ENEMY_W as f32);
            let angle = (rand01() - 0.5) * 0.9;
            for dx in [-38.0_f32, 0.0, 38.0] {
                spawn_enemy(g, EnemyKind::Grunt, (cx + dx).clamp(lo, hi), Some(angle));
            }
        }
        _ => {
            let x = lo + rand01() * (hi - lo);
            let kind = match rand() % 10 {
                0 => EnemyKind::Ace,
                1..=3 => EnemyKind::Weaver,
                _ => EnemyKind::Grunt,
            };
            spawn_enemy(g, kind, x, None);
        }
    }
}

fn update_enemies(g: &mut Game, dt: f32) {
    let player_x = g.player_x;
    let player_y = g.player_y;
    for i in 0..MAX_ENEMIES {
        if !g.enemies[i].active { continue; }
        g.enemies[i].t += dt;

        let speed = match g.enemies[i].kind {
            EnemyKind::Grunt  => 100.0,
            EnemyKind::Weaver => 118.0,
            EnemyKind::Ace    => 92.0,
        };

        // The "autopilot target" — the heading this plane wants to be
        // flying right now. It doesn't turn onto this directly; it banks
        // toward it below, and the bank is what actually turns the plane.
        let desired_heading = match g.enemies[i].kind {
            EnemyKind::Grunt => g.enemies[i].flight_quirk,
            EnemyKind::Weaver => {
                (g.enemies[i].t * 2.6 + g.enemies[i].flight_quirk).sin() * 0.85
            }
            EnemyKind::Ace => {
                let dx = player_x - g.enemies[i].x;
                let dy = (player_y - g.enemies[i].y).max(24.0);
                dx.atan2(dy).clamp(-1.15, 1.15)
            }
        };

        // Roll toward a bank angle proportional to the heading error (a big
        // error commands a steep bank, like correcting hard onto course;
        // as the error closes the plane rolls back toward level) — clamped
        // to how fast it can actually roll and how far it can bank. A pure
        // proportional command overshoots and rings (the plane corrects onto
        // heading, sails past it, corrects back, past it again — a visible
        // pendulum swing), so the command is damped by the turn rate the
        // *current* bank is already producing, the way a real autopilot's
        // rate damping anticipates and kills that overshoot before it builds.
        let mut err = desired_heading - g.enemies[i].heading;
        err = err.rem_euclid(std::f32::consts::TAU);
        if err > std::f32::consts::PI { err -= std::f32::consts::TAU; }
        let current_turn_rate = ENEMY_TURN_G * g.enemies[i].bank.tan() / speed;
        let desired_bank = (err * ENEMY_BANK_GAIN - current_turn_rate * ENEMY_BANK_DAMPING)
            .clamp(-ENEMY_MAX_BANK, ENEMY_MAX_BANK);
        let max_roll = ENEMY_ROLL_RATE * dt;
        g.enemies[i].bank += (desired_bank - g.enemies[i].bank).clamp(-max_roll, max_roll);

        // The bank is what turns the plane — the coordinated-turn relation
        // turn_rate = g*tan(bank)/airspeed (ENEMY_TURN_G stands in for real
        // gravity, tuned for this game's scale rather than 9.8 m/s^2).
        let turn_rate = ENEMY_TURN_G * g.enemies[i].bank.tan() / speed;
        g.enemies[i].heading += turn_rate * dt;

        // Fly forward along the current heading (0 = straight down, +x/-x
        // as it banks left/right) — position falls out of the physics
        // instead of being written directly.
        let h = g.enemies[i].heading;
        g.enemies[i].x += h.sin() * speed * dt;
        g.enemies[i].y += h.cos() * speed * dt;
        // Keep clear of the extreme corners — nothing else bounds x once a
        // plane is airborne, and a hard bank near the top edge can otherwise
        // carry it right into the corner, where the CRT glass's barrel
        // distortion curves hardest and visibly crops it.
        g.enemies[i].x = g.enemies[i].x.clamp(ENEMY_EDGE_MARGIN, (WIN_W - ENEMY_W) as f32 - ENEMY_EDGE_MARGIN);

        if g.enemies[i].y > WIN_H as f32 {
            g.enemies[i].active = false;
            continue;
        }
        if g.enemies[i].kind != EnemyKind::Weaver && g.enemies[i].fire_timer.tick(dt) {
            let (ex, ey, kind) = (g.enemies[i].x, g.enemies[i].y, g.enemies[i].kind);
            pool_spawn(&mut g.enemy_bullets, Bullet {
                x: ex + ENEMY_W as f32 / 2.0 - ENEMY_BULLET_W / 2.0,
                y: ey + ENEMY_H as f32,
                active: true,
            });
            let (mn, mx) = if kind == EnemyKind::Ace { (0.9, 1.6) } else { (1.1, 2.0) };
            g.enemies[i].fire_timer.start(mn + rand01() * (mx - mn));
        }
    }
}

fn spawn_boss(g: &mut Game, sfx: &Sounds) {
    let tier = ((g.sess.level - 1) as usize).min(BOSS_SPECS.len() - 1);
    let spec = &BOSS_SPECS[tier];
    let (bw, bh) = boss_size(tier);
    let mut fire_timer = Timer::default();
    fire_timer.start(1.0);
    let mut escort_timer = Timer::default();
    if spec.escorts { escort_timer.start(2.5); }
    g.boss = Boss {
        x: (WIN_W as f32 - bw) / 2.0,
        y: -bh,
        active: true,
        entered: false,
        hp: spec.hp, max_hp: spec.hp,
        dir: 1.0,
        tier,
        t: 0.0,
        volley: 0,
        fire_timer,
        escort_timer,
    };
    g.boss_intro.start(BOSS_INTRO_TIME);
    play_sfx(&sfx.boss_warning);
}

/// One volley, `n` shots spread evenly across `width` and centred on
/// `center_x` — the shared building block behind Fan/Aimed/Sweep; they
/// differ only in what centre and width they pass in each time they fire.
fn fire_fan(g: &mut Game, center_x: f32, width: f32, n: i32, y: f32) {
    let half = width / 2.0;
    for i in 0..n {
        let t = if n <= 1 { 0.5 } else { i as f32 / (n - 1) as f32 };
        let dx = -half + t * width;
        pool_spawn(&mut g.enemy_bullets, Bullet {
            x: center_x + dx - ENEMY_BULLET_W / 2.0,
            y,
            active: true,
        });
    }
}

/// A dense band across most of the play width with a single gap centred on
/// `gap_center` — the player has to find and thread the gap rather than
/// dodge discrete shots.
fn fire_curtain(g: &mut Game, gap_center: f32, n: i32, y: f32) {
    let width = WIN_W as f32 - 40.0;
    let half = width / 2.0;
    let cx = WIN_W as f32 / 2.0;
    let gap_w = (width / n as f32) * 1.6;
    for i in 0..n {
        let t = if n <= 1 { 0.5 } else { i as f32 / (n - 1) as f32 };
        let x = cx - half + t * width;
        if (x - gap_center).abs() < gap_w / 2.0 { continue; }
        pool_spawn(&mut g.enemy_bullets, Bullet { x: x - ENEMY_BULLET_W / 2.0, y, active: true });
    }
}

/// Fire one volley: `spec.patterns[volley % len]` picks the shape (Fan,
/// Aimed, Sweep, or Curtain — see `BossPattern`), cycling round-robin so a
/// multi-pattern boss doesn't repeat the same one twice in a row.
fn boss_fire(g: &mut Game, spec: &BossSpec) {
    let (bw, bh) = boss_size(g.boss.tier);
    let (bx, by) = (g.boss.x, g.boss.y);
    let n = spec.bullets.max(1);
    let y = by + bh - 6.0;
    let boss_cx = bx + bw / 2.0;
    let pattern = spec.patterns[g.boss.volley as usize % spec.patterns.len()];
    g.boss.volley = g.boss.volley.wrapping_add(1);

    match pattern {
        BossPattern::Fan => fire_fan(g, boss_cx, spec.fan_w, n, y),
        BossPattern::Aimed => {
            // A narrower spread than Fan — it's already aimed, so it
            // doesn't need as much width to threaten a moving target.
            let player_cx = g.player_x + PLAYER_W as f32 / 2.0;
            fire_fan(g, player_cx, spec.fan_w * 0.55, n, y);
        }
        BossPattern::Sweep => {
            let cx = WIN_W as f32 / 2.0;
            let range = ((WIN_W as f32 - spec.fan_w) * 0.5 - 10.0).max(20.0);
            let center = (cx + (g.boss.t * 0.8).sin() * range)
                .clamp(spec.fan_w / 2.0 + 10.0, WIN_W as f32 - spec.fan_w / 2.0 - 10.0);
            fire_fan(g, center, spec.fan_w, n, y);
        }
        BossPattern::Curtain => {
            let gap_center = 30.0 + rand01() * (WIN_W as f32 - 60.0);
            fire_curtain(g, gap_center, n + n / 2 + 1, y);
        }
    }
}

fn update_boss(g: &mut Game, dt: f32) {
    if !g.boss.active { return; }
    // Stay off-screen (parked at its spawn position, fully above the top
    // edge) until the WARNING banner has cleared — the ship and the banner
    // should never be on screen at the same time.
    if g.boss_intro.active() { return; }

    let spec = &BOSS_SPECS[g.boss.tier];
    let (bw, _bh) = boss_size(g.boss.tier);
    g.boss.t += dt;

    let target_y = (HUD_H + 50) as f32;
    if !g.boss.entered {
        g.boss.y = (g.boss.y + 70.0 * dt).min(target_y);
        if g.boss.y >= target_y { g.boss.entered = true; }
        return;
    }

    g.boss.x += g.boss.dir * spec.speed * dt;
    if g.boss.x < 10.0 { g.boss.x = 10.0; g.boss.dir = 1.0; }
    let max_x = WIN_W as f32 - bw - 10.0;
    if g.boss.x > max_x { g.boss.x = max_x; g.boss.dir = -1.0; }

    // From tier 3 up, the boss periodically sinks toward the player instead
    // of holding a flat patrol line — makes the fight feel a lot less static.
    let dip = if spec.dips { (g.boss.t * 1.1).sin().max(0.0) * 26.0 } else { 0.0 };
    g.boss.y = target_y + dip;

    // Tier 7 enrages below half health: much faster fire, no other changes
    // needed since the bullet/HP-bar colour already reads the tier.
    let hp_frac = g.boss.hp as f32 / g.boss.max_hp as f32;
    let enraged = g.boss.tier == BOSS_SPECS.len() - 1 && hp_frac <= 0.5;
    let (fmin, fmax) = if enraged {
        (spec.fire_min * 0.5, spec.fire_max * 0.5)
    } else {
        (spec.fire_min, spec.fire_max)
    };

    if g.boss.fire_timer.tick(dt) {
        boss_fire(g, spec);
        g.boss.fire_timer.start(fmin + rand01() * (fmax - fmin));
    }

    if spec.escorts && g.boss.escort_timer.tick(dt) {
        let x = (g.boss.x + bw / 2.0 - ENEMY_W as f32 / 2.0).clamp(ENEMY_EDGE_MARGIN, (WIN_W - ENEMY_W) as f32 - ENEMY_EDGE_MARGIN);
        spawn_enemy(g, EnemyKind::Grunt, x, None);
        g.boss.escort_timer.start(2.2 + rand01() * 1.6);
    }
}

fn update_title(g: &mut Game) {
    if any_key_pressed() { g.start_game(); }
}

/// Carrier launch: the plane climbs from the deck up to its normal starting
/// height while the ship falls away behind/below it. No input, no hazards —
/// a short cinematic beat at the top of every level.
fn update_launch(g: &mut Game, dt: f32) {
    update_background(g, dt);

    let done = g.launch_timer.tick(dt);
    let k = (1.0 - g.launch_timer.remaining() / LAUNCH_TIME).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - k) * (1.0 - k); // ease-out: quick start, gentle settle

    let start_y = (WIN_H - 60) as f32;
    g.player_y = start_y + (PLAYER_MAX_Y - start_y) * eased;
    g.ship_y = (WIN_H - CARRIER_H / 2) as f32 + eased * (CARRIER_H as f32 * 1.4);

    if done {
        g.player_y = PLAYER_MAX_Y;
        g.respawn_grace.start(0.4); // a beat of invulnerability as the fight starts
        g.state = State::Play;
    }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.respawn_grace.tick(dt);
    g.boss_intro.tick(dt);
    g.max_power_banner.tick(dt);

    // ---- movement ----
    let left  = key_held(BLIP_KEY_LEFT)  || key_held(BLIP_KEY_A);
    let right = key_held(BLIP_KEY_RIGHT) || key_held(BLIP_KEY_D);
    let up    = key_held(BLIP_KEY_UP)    || key_held(BLIP_KEY_W);
    let down  = key_held(BLIP_KEY_DOWN)  || key_held(BLIP_KEY_S);
    if left  { g.player_x -= PLAYER_SPEED * dt; }
    if right { g.player_x += PLAYER_SPEED * dt; }
    if up    { g.player_y -= PLAYER_SPEED * dt; }
    if down  { g.player_y += PLAYER_SPEED * dt; }
    g.player_x = clamp(g.player_x, 0.0, (WIN_W - PLAYER_W) as f32);
    g.player_y = clamp(g.player_y, PLAYER_MIN_Y, PLAYER_MAX_Y);

    // Roll into a strafe, the way the original 1942's plane does — eased
    // toward the target tilt rather than snapping, so it reads as banking.
    let target_bank = if left && !right { -0.30 } else if right && !left { 0.30 } else { 0.0 };
    g.player_bank += (target_bank - g.player_bank) * (dt * 9.0).min(1.0);

    // ---- firing ----
    g.fire_cd.tick(dt);
    if key_held(BLIP_KEY_SPACE) && !g.fire_cd.active() && !g.respawn_grace.active() {
        g.fire_cd.start(weapon_cooldown(g.weapon_level));
        let bx = g.player_x + PLAYER_W as f32 / 2.0 - BULLET_W / 2.0;
        for &dx in weapon_spread(g.weapon_level) {
            pool_spawn(&mut g.bullets, Bullet { x: bx + dx, y: g.player_y, active: true });
        }
        play_sfx(&sfx.shoot);
    }

    // ---- simple movement (no cross-entity reads) ----
    for b in pool_iter_mut(&mut g.bullets) {
        b.y -= BULLET_SPEED * dt;
        if b.y < -BULLET_H { b.active = false; }
    }
    for b in pool_iter_mut(&mut g.enemy_bullets) {
        b.y += ENEMY_BULLET_SPEED * dt;
        if b.y > WIN_H as f32 { b.active = false; }
    }
    for p in pool_iter_mut(&mut g.powerups) {
        p.y += POW_SPEED * dt;
        if p.y > WIN_H as f32 { p.active = false; }
    }
    for h in pool_iter_mut(&mut g.health_pickups) {
        h.y += POW_SPEED * dt;
        if h.y > WIN_H as f32 { h.active = false; }
    }
    for e in pool_iter_mut(&mut g.explosions) {
        e.ttl -= dt;
        if e.ttl <= 0.0 { e.active = false; }
    }
    update_background(g, dt);

    update_enemies(g, dt);
    update_boss(g, dt);
    update_islands(g, dt, sfx);
    update_barrier(g, dt, sfx);

    if !g.boss.active {
        if g.spawn_timer.tick(dt) {
            spawn_wave_tick(g);
            let (mn, mx) = spawn_interval_range(g.sess.level);
            g.spawn_timer.start(mn + rand01() * (mx - mn));
        }
        if g.wave_kills >= g.wave_target {
            spawn_boss(g, sfx);
        }
        // A seldom, higher-level set-piece: checked only when nothing else
        // is already claiming the screen (no boss, no barrier already up).
        if !g.barrier.active && g.sess.level >= BARRIER_MIN_LEVEL && g.barrier_timer.tick(dt) {
            spawn_barrier(g, sfx);
        }
    }

    // ---- player bullets vs enemies / boss ----
    for bi in 0..MAX_PLAYER_BULLETS {
        if !g.bullets[bi].active { continue; }
        let (bx, by) = (g.bullets[bi].x, g.bullets[bi].y);
        let mut consumed = false;
        for ei in 0..MAX_ENEMIES {
            if !g.enemies[ei].active { continue; }
            let (ex, ey) = (g.enemies[ei].x, g.enemies[ei].y);
            if rects_overlap(bx, by, BULLET_W, BULLET_H, ex, ey, ENEMY_W as f32, ENEMY_H as f32) {
                let kind = g.enemies[ei].kind;
                g.enemies[ei].active = false;
                g.spawn_explosion(ex + ENEMY_W as f32 / 2.0, ey + ENEMY_H as f32 / 2.0, 1.0, EXPLOSION_ORANGE);
                let pts = match kind { EnemyKind::Grunt => 20, EnemyKind::Weaver => 30, EnemyKind::Ace => 50 };
                g.sess.add_score(pts * g.sess.level);
                g.wave_kills += 1;
                play_sfx(&sfx.enemy_explode);
                if kind == EnemyKind::Ace {
                    pool_spawn(&mut g.powerups, Powerup { x: ex, y: ey, active: true });
                } else if rand01() < HEALTH_DROP_CHANCE {
                    pool_spawn(&mut g.health_pickups, HealthPickup { x: ex, y: ey, active: true });
                }
                consumed = true;
                break;
            }
        }
        if consumed {
            g.bullets[bi].active = false;
            continue;
        }
        let (bossw, bossh) = boss_size(g.boss.tier);
        if g.boss.active && rects_overlap(bx, by, BULLET_W, BULLET_H, g.boss.x, g.boss.y, bossw, bossh) {
            g.bullets[bi].active = false;
            g.boss.hp -= 1;
            g.spawn_explosion(bx, by, 0.6, EXPLOSION_ORANGE);
            if g.boss.hp <= 0 {
                let (cx, cy) = (g.boss.x + bossw / 2.0, g.boss.y + bossh / 2.0);
                g.boss.active = false;
                // A bigger, longer death for a bigger boss.
                let burst = 1.6 + g.boss.tier as f32 * 0.3;
                g.spawn_explosion(cx, cy, burst, EXPLOSION_ORANGE);
                g.spawn_explosion(cx - 14.0, cy - 8.0, burst * 0.6, EXPLOSION_ORANGE);
                g.spawn_explosion(cx + 14.0, cy + 6.0, burst * 0.6, EXPLOSION_ORANGE);
                g.sess.add_score(500 * g.sess.level);
                play_sfx(&sfx.boss_explode);
                if g.sess.level >= MAX_LEVEL {
                    // The final boss is down — the game is won.
                    g.sess.add_score(2000);
                    play_sfx(&sfx.victory);
                    g.state = State::Won;
                } else {
                    g.sess.next_level();
                    g.wave_kills = 0;
                    g.wave_target = wave_target_for(g.sess.level);
                    g.win_timer.start(WIN_PAUSE);
                    g.state = State::Win;
                }
            }
        }

        // Boats don't fight back — a quick bonus target for a stray shot.
        if g.bullets[bi].active {
            for boi in 0..MAX_BOATS {
                if !g.boats[boi].active { continue; }
                let (boat_x, boat_y) = (g.boats[boi].x, g.boats[boi].y);
                if rects_overlap(bx, by, BULLET_W, BULLET_H, boat_x, boat_y, BOAT_W as f32, BOAT_H as f32) {
                    g.bullets[bi].active = false;
                    g.boats[boi].active = false;
                    g.spawn_explosion(boat_x + BOAT_W as f32 / 2.0, boat_y + BOAT_H as f32 / 2.0, 1.3, EXPLOSION_ORANGE);
                    g.sess.add_score(40 * g.sess.level);
                    play_sfx(&sfx.enemy_explode);
                    break;
                }
            }
        }

        // Islands take several hits before the turret goes down — unlike the
        // boats, this one shoots back, so knocking it out is worth a lot more.
        if g.bullets[bi].active {
            for isi in 0..MAX_ISLANDS {
                if !g.islands[isi].active { continue; }
                let (iw, ih) = ISLAND_SIZES[g.islands[isi].size as usize];
                let (ix, iy) = (g.islands[isi].x, g.islands[isi].y);
                if rects_overlap(bx, by, BULLET_W, BULLET_H, ix, iy, iw as f32, ih as f32) {
                    g.bullets[bi].active = false;
                    g.islands[isi].hp -= 1;
                    g.spawn_explosion(bx, by, 0.5, EXPLOSION_ORANGE);
                    if g.islands[isi].hp <= 0 {
                        g.islands[isi].active = false;
                        let (cx, cy) = (ix + iw as f32 / 2.0, iy + ih as f32 / 2.0);
                        g.spawn_explosion(cx, cy, 1.9, EXPLOSION_ORANGE);
                        g.sess.add_score(ISLAND_SCORE[g.islands[isi].size as usize]);
                        play_sfx(&sfx.boss_explode);
                    } else {
                        play_sfx(&sfx.enemy_explode);
                    }
                    break;
                }
            }
        }

        // The laser barrier's motor — the only part of it that's shootable,
        // and not shootable at all while it's still warming up (matches the
        // beam itself not being able to hurt the player yet either).
        if g.bullets[bi].active && g.barrier.active && !g.barrier.warmup.active() {
            let (mx, my) = (g.barrier.motor_x - MOTOR_W / 2.0, BARRIER_Y - MOTOR_H / 2.0);
            if rects_overlap(bx, by, BULLET_W, BULLET_H, mx, my, MOTOR_W, MOTOR_H) {
                g.bullets[bi].active = false;
                g.barrier.hp -= 1;
                g.spawn_explosion(bx, by, 0.6, EXPLOSION_ORANGE);
                if g.barrier.hp <= 0 {
                    g.barrier.active = false;
                    stop_sound(&sfx.barrier_hum);
                    g.spawn_explosion(g.barrier.motor_x, BARRIER_Y, 2.0, EXPLOSION_ORANGE);
                    g.sess.add_score(300 * g.sess.level);
                    play_sfx(&sfx.boss_explode);
                } else {
                    play_sfx(&sfx.enemy_explode);
                }
            }
        }
    }

    // ---- power-up catch ----
    for i in 0..MAX_POWERUPS {
        if !g.powerups[i].active { continue; }
        if rects_overlap(g.player_x, g.player_y, PLAYER_W as f32, PLAYER_H as f32, g.powerups[i].x, g.powerups[i].y, POW_W, POW_H) {
            g.powerups[i].active = false;
            let (cx, cy) = (g.powerups[i].x + POW_W / 2.0, g.powerups[i].y + POW_H / 2.0);
            if g.weapon_level < MAX_WEAPON_LEVEL {
                g.weapon_level += 1;
                g.spawn_powerup_burst(cx, cy, g.weapon_level, weapon_tier_color(g.weapon_level));
                play_sfx(&sfx.powerup_up[(g.weapon_level - 2) as usize]);
                g.sess.add_score(30 * g.weapon_level);
                if g.weapon_level == MAX_WEAPON_LEVEL {
                    g.max_power_banner.start(MAX_POWER_BANNER_TIME);
                }
            } else {
                // Already at max: a small sparkle and a score bonus instead.
                g.spawn_explosion(cx, cy, 1.1, weapon_tier_color(MAX_WEAPON_LEVEL));
                play_sfx(&sfx.powerup_up[0]);
                g.sess.add_score(150);
            }
        }
    }

    // ---- health pickup catch ----
    for i in 0..MAX_HEALTH_PICKUPS {
        if !g.health_pickups[i].active { continue; }
        if rects_overlap(g.player_x, g.player_y, PLAYER_W as f32, PLAYER_H as f32, g.health_pickups[i].x, g.health_pickups[i].y, HEALTH_W, HEALTH_H) {
            g.health_pickups[i].active = false;
            let (cx, cy) = (g.health_pickups[i].x + HEALTH_W / 2.0, g.health_pickups[i].y + HEALTH_H / 2.0);
            g.health = (g.health + HEALTH_RESTORE).min(PLAYER_HEALTH_MAX);
            g.spawn_explosion(cx, cy, 1.0, BLIP_GREEN);
            play_sfx(&sfx.health_pickup);
        }
    }

    // ---- player vs hazards ----
    if g.state == State::Play && !g.respawn_grace.active() {
        let (px, py) = (g.player_x, g.player_y);
        let mut hit = false;
        for i in 0..MAX_ENEMY_BULLETS {
            if !g.enemy_bullets[i].active { continue; }
            if rects_overlap(px, py, PLAYER_W as f32, PLAYER_H as f32, g.enemy_bullets[i].x, g.enemy_bullets[i].y, ENEMY_BULLET_W, ENEMY_BULLET_H) {
                g.enemy_bullets[i].active = false;
                hit = true;
            }
        }
        for i in 0..MAX_TURRET_BULLETS {
            if !g.turret_bullets[i].active { continue; }
            if rects_overlap(px, py, PLAYER_W as f32, PLAYER_H as f32, g.turret_bullets[i].x, g.turret_bullets[i].y, TURRET_BULLET_W, TURRET_BULLET_H) {
                g.turret_bullets[i].active = false;
                hit = true;
            }
        }
        for i in 0..MAX_ENEMIES {
            if !g.enemies[i].active { continue; }
            if rects_overlap(px, py, PLAYER_W as f32, PLAYER_H as f32, g.enemies[i].x, g.enemies[i].y, ENEMY_W as f32, ENEMY_H as f32) {
                g.spawn_explosion(g.enemies[i].x, g.enemies[i].y, 1.0, EXPLOSION_ORANGE);
                g.enemies[i].active = false;
                hit = true;
            }
        }
        let (bossw, bossh) = boss_size(g.boss.tier);
        if g.boss.active && rects_overlap(px, py, PLAYER_W as f32, PLAYER_H as f32, g.boss.x, g.boss.y, bossw, bossh) {
            hit = true;
        }
        // The laser barrier itself — full width, so there's no dodging
        // sideways around it, only staying clear of its row vertically
        // (or shooting the motor down before it reaches you).
        if g.barrier.active && !g.barrier.warmup.active()
            && rects_overlap(px, py, PLAYER_W as f32, PLAYER_H as f32,
                0.0, BARRIER_Y - BARRIER_BEAM_H / 2.0, WIN_W as f32, BARRIER_BEAM_H)
        {
            hit = true;
        }
        if hit {
            g.health -= 1;
            // Every hit costs a weapon tier too, not just health — getting
            // clipped stings twice, and it gives a reason to stay cautious
            // even at max power instead of just tanking hits with it.
            if g.weapon_level > 1 {
                g.weapon_level -= 1;
                g.spawn_explosion(px + PLAYER_W as f32 / 2.0, py + PLAYER_H as f32 / 2.0, 0.9, BLIP_GRAY);
            }
            if g.health > 0 {
                // Still flying: a flinch of invulnerability (with the usual
                // respawn blink) instead of going down outright.
                play_sfx(&sfx.player_hit);
                g.respawn_grace.start(HIT_GRACE);
            } else {
                g.spawn_player_death(px + PLAYER_W as f32 / 2.0, py + PLAYER_H as f32 / 2.0);
                play_sfx(&sfx.player_explode);
                match g.sess.lose_life() {
                    LifeResult::StillAlive => { g.dead_timer.start(DEAD_PAUSE); g.state = State::Dead; }
                    LifeResult::GameOver   => { g.over_timer.start(OVER_MIN_WAIT); g.state = State::Over; }
                }
            }
        }
    }
}

fn update_dead(g: &mut Game, dt: f32) {
    if g.dead_timer.tick(dt) { g.respawn(); }
}

fn update_win(g: &mut Game, dt: f32) {
    if g.win_timer.tick(dt) { g.start_round(); }
}

fn update_won(g: &mut Game) {
    if !any_key_pressed() { return; }
    web::spend_coin();
    g.start_game();
}

fn update_over(g: &mut Game, dt: f32) {
    g.over_timer.tick(dt);
    // OVER_MIN_WAIT has to actually elapse before any key can dismiss this —
    // otherwise the fire button still held down from the fight that killed
    // you bounces straight back to the title screen.
    if g.over_timer.active() || !any_key_pressed() { return; }
    web::spend_coin();
    g.start_game();
}

/// The ocean, scrolling past underneath everything else: a base fill, a
/// series of wavy crest-lines that scroll with `sea_scroll`, and a scatter
/// of sunlight glints. Wobble depends on both x and y so the rows don't all
/// read as one repeating wallpaper strip.
/// The ocean: a vertical colour gradient (top of screen reads as farther
/// away, so it's lighter and hazier — cheap atmospheric perspective), two
/// octaves of wave lines (a long lazy swell plus a shorter chop riding on
/// top of it, instead of one uniform frequency), sun-glitter concentrated in
/// a diagonal glint band the way real light reflects off water toward a
/// fixed sun direction rather than scattering evenly, and — the 2 1/2 D
/// touch that ties the sky layer to the sea — each cloud casts a soft, drifting
/// shadow onto the water below it.
fn draw_sea(blip: &Blip, g: &Game) {
    let bands = 14;
    let band_h = WIN_H as f32 / bands as f32;
    let far  = (0.10, 0.24, 0.46); // hazy, sun-bleached blue at the horizon (top)
    let near = (0.03, 0.11, 0.30); // deep, saturated blue close up (bottom)
    for i in 0..bands {
        let t = i as f32 / (bands - 1) as f32;
        let r = far.0 + (near.0 - far.0) * t;
        let gcol = far.1 + (near.1 - far.1) * t;
        let b = far.2 + (near.2 - far.2) * t;
        blip.fill_rect(0.0, i as f32 * band_h, WIN_W as f32, band_h + 1.0, BlipColor::new(r, gcol, b, 1.0));
    }

    let spacing = 30.0;
    let rows = (WIN_H as f32 / spacing) as i32 + 2;
    let phase = g.sea_scroll % spacing;
    let segs = 12;
    let seg_w = WIN_W as f32 / segs as f32;
    for i in -1..rows {
        let y = i as f32 * spacing + phase;
        let depth = (y / WIN_H as f32).clamp(0.0, 1.0);
        // Long swell + short chop layered together (two frequencies, not
        // one) — a single sine reads as a uniform ripple; two makes it look
        // like real, irregular open water.
        let alpha = 0.22 + depth * 0.30;
        let color = BlipColor::new(0.55 + depth * 0.15, 0.72 + depth * 0.12, 0.92, alpha);
        for s in 0..segs {
            let x0 = s as f32 * seg_w;
            let x1 = x0 + seg_w;
            let w0 = (x0 * 0.045 + y * 0.05).sin() * 3.2 + (x0 * 0.11 - y * 0.14).sin() * 1.1;
            let w1 = (x1 * 0.045 + y * 0.05).sin() * 3.2 + (x1 * 0.11 - y * 0.14).sin() * 1.1;
            blip.draw_line(x0, y + w0, x1, y + w1, color);
        }
    }

    // Sun-glitter: a diagonal band of dense, bright glints scrolling with the
    // water (real specular sun-glint off wavelets), plus a light scatter of
    // dim ones everywhere else so the whole sea doesn't look dead outside it.
    for i in 0..26 {
        let seed = i as f32 * 53.7;
        let x = (seed * 13.0) % WIN_W as f32;
        let y = (seed * 29.0 + g.sea_scroll) % (WIN_H as f32 + 40.0) - 20.0;
        let band_center = x * 0.62 + 40.0;
        let dist = (y - band_center).abs();
        let in_band = dist < 60.0;
        let twinkle = (g.sea_scroll * 0.05 + seed).sin() * 0.5 + 0.5;
        let (r, a) = if in_band {
            (1.9 - dist / 60.0, (0.55 + twinkle * 0.35) * (1.0 - dist / 60.0))
        } else {
            (1.1, 0.20 + twinkle * 0.10)
        };
        if r > 0.2 {
            blip.fill_circle(x, y, r, BlipColor::new(0.85, 0.94, 1.0, a));
        }
    }

    // Cloud shadows on the water — a soft dark smear beneath each cloud,
    // offset the same direction as the plane shadows so the whole scene
    // reads as lit from one consistent direction.
    for c in g.clouds.iter() {
        let sx = c.x + PLANE_SHADOW_DX * 1.5;
        let sy = c.y + PLANE_SHADOW_DY * 1.5;
        blip.fill_circle(sx, sy, c.r * 0.75, BlipColor::new(0.0, 0.0, 0.0, 0.10));
    }
}

/// Drop the same sprite behind an entity, tinted dark and translucent and
/// offset toward the sea — see PLANE_SHADOW_DX/DY. Draw this before the
/// entity itself, after the sea/boats/clouds layers so it reads as resting
/// on the water rather than floating.
fn draw_shadow(tex: &Texture2D, x: f32, y: f32, w: f32, h: f32, rotation: f32) {
    draw_texture_ex(tex, x + PLANE_SHADOW_DX, y + PLANE_SHADOW_DY, BlipColor::new(0.0, 0.0, 0.0, 0.30), DrawTextureParams {
        dest_size: Some(vec2(w, h)),
        rotation,
        ..Default::default()
    });
}

fn draw_boats(blip: &Blip, g: &Game, boat_tex: &Texture2D) {
    for b in pool_iter(&g.boats) {
        // Rock with the swell: a slow bob (vertical rise/fall) plus a small
        // roll (side-to-side tilt) derived from it, like a hull actually
        // riding waves instead of sliding across flat glass.
        let bob = (g.sea_scroll * 0.05 + b.bob_phase).sin();
        let chop = (g.sea_scroll * 0.13 + b.bob_phase * 1.7).sin();
        let y_off = bob * 1.8 + chop * 0.6;
        let roll = bob * 0.11 + b.vx.signum() * 0.05;

        // V-shaped wake fanning out from the stern, fading with distance —
        // drawn fresh from the boat's current drift each frame rather than
        // baked into the sprite, so it always matches its heading.
        let stern_x = b.x + BOAT_W as f32 / 2.0;
        let stern_y = b.y + BOAT_H as f32 * 0.8 + y_off;
        let drift = (-b.vx * 0.06).clamp(-1.5, 1.5);
        for k in 1..=6 {
            let t = k as f32;
            let back = t * 4.2;
            let spread = t * 2.4;
            let alpha = (0.28 - t * 0.042).max(0.0);
            if alpha <= 0.0 { continue; }
            let wy = stern_y - back;
            let lx = stern_x - spread + drift * back * 0.25;
            let rx = stern_x + spread + drift * back * 0.25;
            blip.fill_circle(lx, wy, 1.1, BlipColor::new(0.85, 0.93, 1.0, alpha));
            blip.fill_circle(rx, wy, 1.1, BlipColor::new(0.85, 0.93, 1.0, alpha));
        }

        draw_texture_ex(boat_tex, b.x, b.y + y_off, BLIP_WHITE, DrawTextureParams {
            dest_size: Some(vec2(BOAT_W as f32, BOAT_H as f32)),
            flip_x: b.vx < 0.0,
            rotation: roll,
            ..Default::default()
        });
    }
}

/// The sky layer, between the sea and the planes: fluffy three-lobe cloud
/// puffs, opaque enough to read clearly against the ocean below.
/// The sky layer, between the sea and the planes: noise-generated cumulus
/// puffs (see cloud_sprite() in blip_assets — fractal value noise, not flat
/// circles) cycled across 3 baked variants for shape variety.
fn draw_clouds(blip: &Blip, g: &Game, cloud_tex: &[Texture2D; 3]) {
    for c in g.clouds.iter() {
        let tex = &cloud_tex[c.variant as usize % cloud_tex.len()];
        // Stretched noticeably wider than tall — long cloud banks rather than
        // round puffs, big enough for a plane to actually vanish into one.
        let w = c.r * 3.6;
        let h = c.r * 1.7;
        blip.draw_texture(tex, c.x - w / 2.0, c.y - h / 2.0, w, h);
    }
}

/// True if (x, y) — an entity's centre — falls inside the visible body of
/// any cloud currently on screen (an ellipse test against each cloud's drawn
/// footprint, shrunk a bit so a plane has to be substantially under the
/// cloud, not just clipping its edge, before it counts as covered).
fn point_in_any_cloud(g: &Game, x: f32, y: f32) -> bool {
    g.clouds.iter().any(|c| {
        let (rw, rh) = (c.r * 3.6 * 0.5 * 0.8, c.r * 1.7 * 0.5 * 0.8);
        let (dx, dy) = ((x - c.x) / rw, (y - c.y) / rh);
        dx * dx + dy * dy <= 1.0
    })
}

/// Islands, part of the sea/sky scenery layer (drawn after the clouds, before
/// the aerial combat) — plus a slim HP bar, same idiom as the boss's, once
/// one has taken a hit.
fn draw_islands(blip: &Blip, g: &Game, island_tex: &[Texture2D; 3]) {
    for isl in pool_iter(&g.islands) {
        let (w, h) = ISLAND_SIZES[isl.size as usize];
        blip.draw_texture(&island_tex[isl.size as usize], isl.x, isl.y, w as f32, h as f32);
        let max_hp = ISLAND_HP[isl.size as usize];
        if isl.hp < max_hp {
            let frac = (isl.hp as f32 / max_hp as f32).clamp(0.0, 1.0);
            blip.draw_rect(isl.x, isl.y - 6.0, w as f32, 3.0, BLIP_GRAY);
            blip.fill_rect(isl.x, isl.y - 6.0, w as f32 * frac, 3.0, BLIP_RED);
        }
    }
}

fn draw_launch(
    blip: &Blip, g: &Game,
    player_tex: &Texture2D, carrier_tex: &Texture2D, boat_tex: &Texture2D, cloud_tex: &[Texture2D; 3],
    island_tex: &[Texture2D; 3],
) {
    draw_sea(blip, g);
    draw_boats(blip, g, boat_tex);
    draw_islands(blip, g, island_tex);
    draw_clouds(blip, g, cloud_tex);
    let carrier_x = ((WIN_W - CARRIER_W) / 2) as f32;
    let carrier_y = g.ship_y - CARRIER_H as f32 / 2.0;
    draw_shadow(carrier_tex, carrier_x, carrier_y, CARRIER_W as f32, CARRIER_H as f32, 0.0);
    blip.draw_texture(carrier_tex, carrier_x, carrier_y, CARRIER_W as f32, CARRIER_H as f32);
    draw_shadow(player_tex, g.player_x, g.player_y, PLAYER_W as f32, PLAYER_H as f32, 0.0);
    blip.draw_texture(player_tex, g.player_x, g.player_y, PLAYER_W as f32, PLAYER_H as f32);
    blip.draw_hud(g.sess.score, g.sess.lives);
}

fn draw_play(
    blip: &Blip, g: &Game,
    player_tex: &Texture2D, enemy_tex: &[Texture2D; 3], boss_tex: &[Texture2D; 7], pow_tex: &Texture2D,
    health_tex: &Texture2D,
    boss_name_ja_tex: &[Texture2D; 7], boat_tex: &Texture2D, cloud_tex: &[Texture2D; 3],
    island_tex: &[Texture2D; 3],
) {
    draw_sea(blip, g);
    draw_boats(blip, g, boat_tex);
    draw_islands(blip, g, island_tex);
    draw_clouds(blip, g, cloud_tex);

    // Tinted by the tier it's about to grant, so the colour is a readable
    // preview of what catching it does — and it brightens as you approach
    // the max tier.
    let next_tier_color = weapon_tier_color((g.weapon_level + 1).min(MAX_WEAPON_LEVEL));
    for p in pool_iter(&g.powerups) {
        blip.draw_texture_tinted(pow_tex, p.x, p.y, POW_W, POW_H, next_tier_color);
    }
    for h in pool_iter(&g.health_pickups) {
        blip.draw_texture(health_tex, h.x, h.y, HEALTH_W, HEALTH_H);
    }

    // Shadows first (a separate pass, not interleaved with the sprites) so a
    // low enemy's shadow never draws over another plane sitting behind it.
    for e in pool_iter(&g.enemies) {
        let (ecx, ecy) = (e.x + ENEMY_W as f32 / 2.0, e.y + ENEMY_H as f32 / 2.0);
        if e.can_hide && point_in_any_cloud(g, ecx, ecy) { continue; }
        let tex = match e.kind {
            EnemyKind::Grunt  => &enemy_tex[0],
            EnemyKind::Weaver => &enemy_tex[1],
            EnemyKind::Ace    => &enemy_tex[2],
        };
        draw_shadow(tex, e.x, e.y, ENEMY_W as f32, ENEMY_H as f32, e.heading);
    }
    if g.boss.active {
        let (bw, bh) = boss_size(g.boss.tier);
        draw_shadow(&boss_tex[g.boss.tier], g.boss.x, g.boss.y, bw, bh, 0.0);
    }
    if g.state == State::Play && !g.respawn_grace.active() {
        draw_shadow(player_tex, g.player_x, g.player_y, PLAYER_W as f32, PLAYER_H as f32, g.player_bank);
    }

    for e in pool_iter(&g.enemies) {
        // Some planes duck out of sight under a cloud they happen to be
        // flying through — purely a draw-order trick (nothing else about
        // them changes: they keep flying, firing, and can still be hit) —
        // reappearing once they've flown clear of it.
        let (ecx, ecy) = (e.x + ENEMY_W as f32 / 2.0, e.y + ENEMY_H as f32 / 2.0);
        if e.can_hide && point_in_any_cloud(g, ecx, ecy) { continue; }
        let tex = match e.kind {
            EnemyKind::Grunt  => &enemy_tex[0],
            EnemyKind::Weaver => &enemy_tex[1],
            EnemyKind::Ace    => &enemy_tex[2],
        };
        // Nose points where the plane is actually flying — see
        // update_enemies() for the heading/turn-rate model `heading` comes from.
        draw_texture_ex(tex, e.x, e.y, BLIP_WHITE, DrawTextureParams {
            dest_size: Some(vec2(ENEMY_W as f32, ENEMY_H as f32)),
            rotation: e.heading,
            ..Default::default()
        });
    }

    if g.boss.active {
        let (bw, bh) = boss_size(g.boss.tier);
        let frac = (g.boss.hp as f32 / g.boss.max_hp as f32).clamp(0.0, 1.0);
        // The final boss flashes white-hot once enraged (below half health) —
        // a plain colour tint on the same sprite, no extra art needed.
        let enraged = g.boss.tier == BOSS_SPECS.len() - 1 && frac <= 0.5;
        let tint = if enraged && ((g.boss.t * 8.0) as i32 % 2 == 0) {
            BlipColor::new(1.6, 1.4, 1.2, 1.0)
        } else {
            BlipColor::new(1.0, 1.0, 1.0, 1.0)
        };
        blip.draw_texture_tinted(&boss_tex[g.boss.tier], g.boss.x, g.boss.y, bw, bh, tint);
        blip.draw_rect(g.boss.x, g.boss.y - 8.0, bw, 4.0, BLIP_GRAY);
        blip.fill_rect(g.boss.x, g.boss.y - 8.0, bw * frac, 4.0, BLIP_RED);
    }

    if g.barrier.active {
        let warming = g.barrier.warmup.active();
        let flicker = ((g.barrier.t * 10.0) as i32 % 2) == 0;
        // Dim and flickering while it warms up (harmless, telegraphing),
        // solid and white-hot down the middle once it's actually live.
        let beam_color = if warming {
            if flicker { BlipColor::new(1.0, 0.2, 0.2, 0.5) } else { BlipColor::new(1.0, 0.2, 0.2, 0.15) }
        } else {
            BlipColor::new(1.0, 0.15, 0.15, 0.9)
        };
        blip.fill_rect(0.0, BARRIER_Y - BARRIER_BEAM_H / 2.0, WIN_W as f32, BARRIER_BEAM_H, beam_color);
        if !warming {
            blip.fill_rect(0.0, BARRIER_Y - 1.5, WIN_W as f32, 3.0, BLIP_WHITE);
        } else {
            let color = if flicker { BLIP_RED } else { BLIP_WHITE };
            blip.draw_centered("LASER BARRIER", BARRIER_Y - 30.0, 2.4, color);
        }

        // The motor: a dark housing with a glowing core and its own health
        // meter above it — the only part of the barrier that's shootable.
        let (mx, my) = (g.barrier.motor_x - MOTOR_W / 2.0, BARRIER_Y - MOTOR_H / 2.0);
        blip.fill_rect(mx, my, MOTOR_W, MOTOR_H, BLIP_GRAY);
        blip.draw_rect(mx, my, MOTOR_W, MOTOR_H, BLIP_BLACK);
        blip.fill_glow_circle(g.barrier.motor_x, BARRIER_Y, 7.0, BLIP_RED);
        let hp_frac = (g.barrier.hp as f32 / g.barrier.max_hp as f32).clamp(0.0, 1.0);
        blip.draw_rect(mx, my - 8.0, MOTOR_W, 4.0, BLIP_GRAY);
        blip.fill_rect(mx, my - 8.0, MOTOR_W * hp_frac, 4.0, BLIP_RED);
    }

    for b in pool_iter(&g.enemy_bullets) {
        blip.fill_glow_circle(b.x + ENEMY_BULLET_W / 2.0, b.y + ENEMY_BULLET_H / 2.0, 4.0, BLIP_RED);
    }
    // Turret shells in orange, not the planes' red, so an island's fire
    // reads as a different kind of threat at a glance.
    for b in pool_iter(&g.turret_bullets) {
        blip.fill_glow_circle(b.x + TURRET_BULLET_W / 2.0, b.y + TURRET_BULLET_H / 2.0, 4.5, BLIP_ORANGE);
    }
    // Bullet look escalates with the current weapon tier: wider, brighter,
    // and glowing instead of a flat rect from tier 4 up.
    let bcolor = weapon_tier_color(g.weapon_level);
    let (bscale, bglow) = weapon_bullet_visual(g.weapon_level);
    let bw = BULLET_W * bscale;
    for b in pool_iter(&g.bullets) {
        let (cx, cy) = (b.x + BULLET_W / 2.0, b.y + BULLET_H / 2.0);
        if bglow {
            blip.fill_glow_circle(cx, cy, bw * 1.1, bcolor);
        } else {
            blip.fill_rect(cx - bw / 2.0, b.y, bw, BULLET_H, bcolor);
        }
    }

    if g.state == State::Play {
        // Blink the plane while the post-respawn grace period is active.
        let blink = g.respawn_grace.active() && ((g.respawn_grace.remaining() * 12.0) as i32 % 2 == 0);
        if !blink {
            draw_texture_ex(player_tex, g.player_x, g.player_y, BLIP_WHITE, DrawTextureParams {
                dest_size: Some(vec2(PLAYER_W as f32, PLAYER_H as f32)),
                rotation: g.player_bank,
                ..Default::default()
            });
        }
    }

    for e in pool_iter(&g.explosions) {
        let k = (e.ttl / e.max_ttl).clamp(0.0, 1.0);
        let r = 14.0 * e.scale * (0.6 + k * 0.6);
        blip.fill_glow_circle(e.x, e.y, r, BlipColor::new(e.color.r, e.color.g, e.color.b, k));
    }

    blip.draw_hud(g.sess.score, g.sess.lives);
    draw_bottom_hud(blip, g, player_tex);

    if g.max_power_banner.active() {
        let flash = ((g.max_power_banner.remaining() * 14.0) as i32 % 2) == 0;
        let color = if flash { weapon_tier_color(MAX_WEAPON_LEVEL) } else { BLIP_WHITE };
        blip.draw_centered("MAXIMUM POWER", (WIN_H / 2 - 10) as f32, 3.5, color);
    }

    if g.boss_intro.active() {
        let flash = ((g.boss_intro.remaining() * 10.0) as i32 % 2) == 0;
        let color = if flash { BLIP_RED } else { BLIP_WHITE };
        let name = BOSS_SPECS[g.boss.tier].name;
        blip.draw_centered("WARNING", (WIN_H / 2 - 40) as f32, 3.0, color);
        blip.draw_centered(name,      (WIN_H / 2 - 16) as f32, 3.0, color);
        // The Japanese name below it — a texture, not the bitmap font (which
        // only covers A-Z/0-9), tinted the same flashing colour.
        let ja = &boss_name_ja_tex[g.boss.tier];
        let jx = (WIN_W as f32 - ja.width()) / 2.0;
        blip.draw_texture_tinted(ja, jx, (WIN_H / 2 + 8) as f32, ja.width(), ja.height(), color);
    }
}

/// A second HUD readout, down in the corners of the playfield instead of
/// up in the top bar with SCORE/LIVES: a mini plane icon and the lives
/// count bottom-left, and the new per-life health meter bottom-right —
/// close to the action, where a glance during a dogfight actually lands.
fn draw_bottom_hud(blip: &Blip, g: &Game, player_tex: &Texture2D) {
    let y = (WIN_H - 30) as f32;
    // The curved-glass CRT post-process (see blip::ctx's CRT_FRAGMENT)
    // clips a wedge in each corner of the canvas — both readouts sit in
    // a bottom corner, so they're kept this far in from the true left/
    // right edges instead of flush against them.
    let corner_margin = 24.0;

    // Lives, bottom-left.
    let mini_w = PLAYER_W as f32 * 0.6;
    let mini_h = PLAYER_H as f32 * 0.6;
    blip.draw_texture(player_tex, corner_margin, y, mini_w, mini_h);
    blip.draw_text("x", corner_margin + mini_w + 3.0, y + 3.0, 2.0, BLIP_WHITE);
    blip.draw_number(g.sess.lives, corner_margin + mini_w + 15.0, y + 3.0, 2.0, BLIP_WHITE);

    // Health, bottom-right: a bar that empties as the plane takes hits and
    // refills back to full on every respawn or health pickup — five hits
    // and it's down, same as a life, but each life now soaks up more than
    // one stray shot.
    let bar_w = 64.0;
    let bar_h = 9.0;
    let bar_x = (WIN_W as f32) - bar_w - corner_margin;
    let bar_y = y + (mini_h - bar_h) / 2.0;
    let hp_frac = (g.health as f32 / PLAYER_HEALTH_MAX as f32).clamp(0.0, 1.0);
    let hp_color = if hp_frac > 0.6 { BLIP_GREEN } else if hp_frac > 0.3 { BLIP_YELLOW } else { BLIP_RED };
    blip.draw_text("HP", bar_x - 26.0, y + 3.0, 2.0, BLIP_WHITE);
    blip.draw_rect(bar_x, bar_y, bar_w, bar_h, BLIP_GRAY);
    blip.fill_rect(bar_x, bar_y, bar_w * hp_frac, bar_h, hp_color);
}

fn draw_title(blip: &Blip, player_tex: &Texture2D) {
    blip.clear(BLIP_BLACK);
    blip.draw_centered("RAIDER", (WIN_H / 4) as f32, 5.0, BLIP_BLUE);
    let px = (WIN_W as f32 - PLAYER_W as f32 * 2.0) / 2.0;
    blip.draw_texture(player_tex, px, (WIN_H / 2 - 70) as f32, PLAYER_W as f32 * 2.0, PLAYER_H as f32 * 2.0);
    blip.draw_centered("PRESS ANY KEY",       (WIN_H * 2 / 3) as f32,      3.0, BLIP_WHITE);
    blip.draw_centered("ARROWS OR WASD MOVE", (WIN_H * 2 / 3 + 22) as f32, 2.0, BLIP_GRAY);
    blip.draw_centered("SPACE TO FIRE",       (WIN_H * 2 / 3 + 40) as f32, 2.0, BLIP_GRAY);
    blip.draw_centered("CATCH CAPSULE POWER UP", (WIN_H * 2 / 3 + 58) as f32, 2.0, BLIP_GRAY);
}

fn draw_win(blip: &Blip, level: i32) {
    let buf = format!("WAVE {level}");
    blip.clear(BLIP_BLACK);
    blip.draw_centered("STAGE CLEAR", (WIN_H / 3) as f32, 5.0, BLIP_GREEN);
    blip.draw_centered(&buf,          (WIN_H / 2) as f32, 3.0, BLIP_YELLOW);
}

fn draw_won(blip: &Blip, score: i32) {
    let buf = format!("SCORE {score}");
    blip.clear(BLIP_BLACK);
    blip.draw_centered("YOU WON!!",           (WIN_H / 4) as f32,      6.0, BLIP_YELLOW);
    blip.draw_centered("ALL 7 WAVES CLEARED", (WIN_H / 2 - 20) as f32, 2.5, BLIP_GREEN);
    blip.draw_centered(&buf,                  (WIN_H / 2 + 14) as f32, 3.0, BLIP_WHITE);
    blip.draw_centered("PRESS ANY KEY",       (WIN_H * 2 / 3) as f32,  3.0, BLIP_CYAN);
}

fn draw_over(blip: &Blip, score: i32, waiting: bool) {
    let buf = format!("SCORE {score}");
    blip.clear(BLIP_BLACK);
    blip.draw_centered("GAME OVER", (WIN_H / 4) as f32, 5.0, BLIP_RED);
    blip.draw_centered(&buf,        (WIN_H / 2) as f32, 3.0, BLIP_WHITE);
    // Only invite a key press once OVER_MIN_WAIT has actually elapsed — the
    // prompt would otherwise be a lie, since input is ignored until then.
    if !waiting {
        blip.draw_centered("PRESS ANY KEY", (WIN_H * 2 / 3) as f32, 3.0, BLIP_YELLOW);
    }
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("RAIDER", WIN_W, WIN_H)
}

const PLAYER_PNG:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/player_plane.png"));
const ENEMY_GRUNT_PNG:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/enemy_grunt.png"));
const ENEMY_WEAVER_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/enemy_weaver.png"));
const ENEMY_ACE_PNG:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/enemy_ace.png"));
const POWERUP_PNG:      &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/powerup.png"));
const HEALTH_PACK_PNG:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/health_pack.png"));
const CARRIER_PNG:      &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/carrier.png"));
const BOAT_PNG:         &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boat.png"));
const CLOUD_PNGS: [&[u8]; 3] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/cloud_1.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/cloud_2.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/cloud_3.png")),
];
// One sprite per island size (small/medium/large) — see ISLAND_SIZES above.
const ISLAND_PNGS: [&[u8]; 3] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/island_small.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/island_medium.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/island_large.png")),
];

// One sprite per boss tier (levels 1-7) — see BOSS_SPECS / BOSS_SIZES above.
const BOSS_PNGS: [&[u8]; 7] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_1.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_2.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_3.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_4.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_5.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_6.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_7.png")),
];

// The Japanese banner for each boss name — see draw_play()'s WARNING banner.
const BOSS_NAME_JA_PNGS: [&[u8]; 7] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_name_ja_1.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_name_ja_2.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_name_ja_3.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_name_ja_4.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_name_ja_5.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_name_ja_6.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/boss_name_ja_7.png")),
];

const SHOOT_WAV:          &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/shoot.wav"));
const ENEMY_EXPLODE_WAV:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/enemy_explode.wav"));
const PLAYER_EXPLODE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/player_explode.wav"));
const PLAYER_HIT_WAV:     &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/player_hit.wav"));
const BOSS_EXPLODE_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/boss_explode.wav"));
const BOSS_WARNING_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/boss_warning.wav"));
const POWERUP2_WAV:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/powerup2.wav"));
const POWERUP3_WAV:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/powerup3.wav"));
const POWERUP4_WAV:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/powerup4.wav"));
const MAX_POWER_WAV:      &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/max_power.wav"));
const HEALTH_PICKUP_WAV:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/health_pickup.wav"));
const STAGE_CLEAR_WAV:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/stage_clear.wav"));
const VICTORY_WAV:        &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/victory.wav"));
const GAME_OVER_WAV:      &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/game_over.wav"));
const TURRET_FIRE_WAV:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/turret_fire.wav"));
const BARRIER_HUM_WAV:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/barrier_hum.wav"));
const MUSIC_WAV:          &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music.wav"));
const MUSIC2_WAV:         &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music2.wav"));
// music: 41.71s (116 BPM march, A-B-A' form)  music2: 31.30s (124 BPM march, minor key)
const MUSIC_DURATIONS: [f32; 2] = [41.71, 31.30];

fn load_png(bytes: &'static [u8]) -> Texture2D {
    let tex = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
    tex.set_filter(FilterMode::Nearest);
    tex
}

// Clouds are soft noise-shaded alpha sprites, not crisp pixel art — linear
// filtering smooths their fBm edges instead of showing blocky alpha steps.
fn load_png_smooth(bytes: &'static [u8]) -> Texture2D {
    let tex = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
    tex.set_filter(FilterMode::Linear);
    tex
}

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let player_tex = load_png(PLAYER_PNG);
    let enemy_tex = [load_png(ENEMY_GRUNT_PNG), load_png(ENEMY_WEAVER_PNG), load_png(ENEMY_ACE_PNG)];
    let boss_tex = BOSS_PNGS.map(load_png);
    let boss_name_ja_tex = BOSS_NAME_JA_PNGS.map(load_png);
    let powerup_tex = load_png(POWERUP_PNG);
    let health_tex = load_png(HEALTH_PACK_PNG);
    let carrier_tex = load_png(CARRIER_PNG);
    let boat_tex = load_png(BOAT_PNG);
    let cloud_tex = CLOUD_PNGS.map(load_png_smooth);
    let island_tex = ISLAND_PNGS.map(load_png);

    let sfx = Sounds {
        shoot:          blip::audio::load_sound(SHOOT_WAV).await,
        enemy_explode:  blip::audio::load_sound(ENEMY_EXPLODE_WAV).await,
        player_explode: blip::audio::load_sound(PLAYER_EXPLODE_WAV).await,
        player_hit:     blip::audio::load_sound(PLAYER_HIT_WAV).await,
        boss_explode:   blip::audio::load_sound(BOSS_EXPLODE_WAV).await,
        boss_warning:   blip::audio::load_sound(BOSS_WARNING_WAV).await,
        powerup_up: [
            blip::audio::load_sound(POWERUP2_WAV).await,
            blip::audio::load_sound(POWERUP3_WAV).await,
            blip::audio::load_sound(POWERUP4_WAV).await,
            blip::audio::load_sound(MAX_POWER_WAV).await,
        ],
        health_pickup:  blip::audio::load_sound(HEALTH_PICKUP_WAV).await,
        stage_clear:    blip::audio::load_sound(STAGE_CLEAR_WAV).await,
        victory:        blip::audio::load_sound(VICTORY_WAV).await,
        game_over:      blip::audio::load_sound(GAME_OVER_WAV).await,
        turret_fire:    blip::audio::load_sound(TURRET_FIRE_WAV).await,
        barrier_hum:    blip::audio::load_sound(BARRIER_HUM_WAV).await,
    };
    // Two loops in rotation instead of one, so a long level doesn't just
    // hear the same ~40s of march on repeat — see MUSIC_DURATIONS.
    let music = [
        blip::audio::load_sound(MUSIC_WAV).await,
        blip::audio::load_sound(MUSIC2_WAV).await,
    ];
    let mut music_idx: usize = 0;
    let mut music_timer: f32 = MUSIC_DURATIONS[0];
    play_music(&music[0]);

    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;

        // Switch to the other loop at each loop boundary.
        music_timer -= dt;
        if music_timer <= 0.0 {
            music_idx = 1 - music_idx;
            music_timer = MUSIC_DURATIONS[music_idx];
            play_music(&music[music_idx]);
        }

        if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 {
                g.start_game();
            }
        }

        let prev_state = g.state;
        match g.state {
            State::Title  => update_title(&mut g),
            State::Launch => update_launch(&mut g, dt),
            State::Play   => update_play(&mut g, dt, &sfx),
            State::Dead   => update_dead(&mut g, dt),
            State::Win    => update_win(&mut g, dt),
            State::Won    => update_won(&mut g),
            State::Over   => update_over(&mut g, dt),
        }
        if prev_state != State::Win  && g.state == State::Win  { play_sfx(&sfx.stage_clear); }
        if prev_state != State::Over && g.state == State::Over { play_sfx(&sfx.game_over); }

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title  => draw_title(&blip, &player_tex),
            State::Launch => draw_launch(&blip, &g, &player_tex, &carrier_tex, &boat_tex, &cloud_tex, &island_tex),
            State::Win    => draw_win(&blip, g.sess.level),
            State::Won    => draw_won(&blip, g.sess.score),
            State::Over   => draw_over(&blip, g.sess.score, g.over_timer.active()),
            State::Play | State::Dead => {
                draw_play(&blip, &g, &player_tex, &enemy_tex, &boss_tex, &powerup_tex, &health_tex, &boss_name_ja_tex, &boat_tex, &cloud_tex, &island_tex);
            }
        }

        blip.next_frame(60).await;
    }
}
