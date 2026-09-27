//! Raider — a 1942-style vertical dogfighting shoot-'em-up.

use blip::input::{
    btn1_pressed, key_active, key_held, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_DOWN, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::audio::{play_sound, set_sound_volume, stop_sound, PlaySoundParams};
use blip::macroquad::math::vec2;
use blip::macroquad::shapes::draw_triangle;
use blip::macroquad::prelude::ImageFormat;
use blip::macroquad::rand::rand;
use blip::macroquad::texture::{draw_texture_ex, DrawTextureParams, FilterMode, Texture2D};
use blip::{
    clamp, play_music, play_sfx, play_sfx_volume, pool_iter, pool_iter_mut, pool_spawn, rects_overlap, web,
    window_conf, Blip, BlipColor, LifeResult, Pooled, Session, Timer,
    BLIP_BLACK, BLIP_BLUE, BLIP_CYAN, BLIP_GRAY, BLIP_GREEN, BLIP_ORANGE, BLIP_RED, BLIP_WHITE,
    BLIP_YELLOW,
};

// ---- layout -------------------------------------------------------------
const WIN_W: i32 = 480;
const WIN_H: i32 = 540;
const HUD_H: i32 = 28;

// ---- player ---------------------------------------------------------------
const PLAYER_W: i32 = 54;
const PLAYER_H: i32 = 48;
const PLAYER_SPEED: f32 = 252.0; // a touch quicker — the d-pad is digital, so response has to carry it
const PLAYER_ACCEL: f32 = 2600.0;
const PLAYER_BRAKE: f32 = 3400.0;
const PLAYER_MIN_Y: f32 = (HUD_H + 150) as f32; // player stays out of the HUD band
// Kept well clear of the bottom edge so the on-screen control pad (which
// rises over the lowest ~15% of the playfield on a touch screen) can never
// hide the plane behind it.
const PLAYER_MAX_Y: f32 = (WIN_H - 88) as f32;

// ---- weapons ----------------------------------------------------------
// Five weapon tiers, one per power-up capsule caught: more guns, heavier
// rounds and faster fire. weapon_guns() and weapon_cooldown() define them.
const MAX_WEAPON_LEVEL: i32 = 5;
const BULLET_SPEED: f32 = 420.0;
const BULLET_H: f32 = 10.0;
const MAX_PLAYER_BULLETS: usize = 112; // tier 5: 8 rounds every 0.11s, ~1.3s in flight
const MAX_CASINGS: usize = 24;

// ---- enemies ------------------------------------------------------------
// Fewer planes, twice the size, flying in formation (see Flight).
const ENEMY_W: i32 = 52;
const ENEMY_H: i32 = 44;
const MAX_ENEMIES: usize = 30;
const ENEMY_BULLET_SPEED: f32 = 205.0;
const MAX_ENEMY_BULLETS: usize = 260;
const WAVE_KILL_BASE: i32 = 60;

// ---- flight model ---------------------------------------------------------
// Each enemy flight is one path through the air, flown the way an aeroplane
// flies: it banks to turn. In a level coordinated turn the lift tilts with
// the bank and its horizontal part pulls the plane round:
//   turn rate  w = g tan(bank) / v        radius  r = v^2 / (g tan(bank))
//   load       n = 1 / cos(bank)
// and the wing only holds the plane up above its stall speed, which rises
// with the load: v_stall(n) = v_stall sqrt(n). At speed v a pilot can bank
// at most acos((v_stall / v)^2) before the wing lets go, and the model never
// commands more. Turning costs speed (induced drag grows with n^2 - 1); the
// engine wins it back on the straights. Turns ease in and out at the roll
// rate, so the path is a smooth curve, never a corner.
const FLIGHT_G: f32 = 130.0;       // px/s^2: g at this game's scale and pace
const FLIGHT_STALL: f32 = 55.0;    // px/s, wings level
const FLIGHT_ROLL: f32 = 1.6;      // rad/s
const FLIGHT_MAX_BANK: f32 = 1.15; // ~66 degrees, a hard fighter turn
const FLIGHT_DRAG: f32 = 0.18;     // speed lost per second per unit of (n^2 - 1)
const FLIGHT_THRUST: f32 = 0.5;    // how fast the engine returns it to trim speed
const MAX_FLIGHTS: usize = 10;

// ---- boss ---------------------------------------------------------------
// One boss per wave, levels 1-7 — see BOSS_SPECS for the full escalation
// (size, HP, fire pattern, speed). Sizes must match blip_assets' BOSS_SIZES.
const MAX_LEVEL: i32 = 7;
const BOSS_SIZES: [(i32, i32); 7] = [
    (96, 76), (104, 84), (116, 92), (150, 106), (164, 122), (182, 134), (272, 196),
];
// Gun positions per boss (x: fraction of the half span, y: fraction of the
// length from the nose). Must match blip_assets' BOSS_TURRETS.
const BOSS_TURRETS: [&[(f32, f32)]; 7] = [
    &[(0.0, 0.06), (0.0, 0.45), (0.0, 0.97)],
    &[(0.0, 0.06), (0.0, 0.50), (0.0, 0.97)],
    &[(0.0, 0.07), (0.0, 0.36), (-0.09, 0.62), (0.09, 0.62), (0.0, 0.98)],
    &[(0.0, 0.06), (0.0, 0.30), (0.0, 0.60), (0.0, 0.97)],
    &[(0.0, 0.07), (0.0, 0.36), (-0.07, 0.60), (0.07, 0.60), (0.0, 0.97)],
    &[(0.0, 0.06), (0.0, 0.28), (0.0, 0.55), (0.0, 0.97)],
    &[(0.0, 0.05), (0.0, 0.22), (0.0, 0.42), (0.0, 0.62), (-0.05, 0.80), (0.05, 0.80), (0.0, 0.97)],
];
const BOSS_INTRO_TIME: f32 = 1.3;

// Wing gun positions per boss, as fractions of the half span (both sides),
// on the wing's centre chord; the fuselage guns are BOSS_TURRETS. More
// turrets each level: 3, 5, 7, 8, 9, 12, 15.
const BOSS_WING_GUNS: [&[f32]; 7] = [
    &[], &[0.62], &[0.62], &[0.36, 0.72], &[0.31, 0.62], &[0.32, 0.6, 0.8], &[0.22, 0.38, 0.6, 0.78],
];
// Wing chords per boss (root LE, root TE, tip LE, tip TE as fractions of
// the length from the nose). Must match blip_assets' boss_plane().
const BOSS_WINGS: [(f32, f32, f32, f32); 7] = [
    (0.30, 0.52, 0.37, 0.46), (0.30, 0.50, 0.37, 0.45), (0.32, 0.54, 0.40, 0.48), (0.30, 0.52, 0.38, 0.46),
    (0.30, 0.46, 0.34, 0.41), (0.30, 0.48, 0.36, 0.43), (0.30, 0.46, 0.36, 0.41),
];
const MAX_TURRETS: usize = 16;

// ---- carrier launch -------------------------------------------------------
// Every level opens on the deck of an Essex-class carrier: the engine
// coughs into life, the launch officer winds it up, the plane rolls the
// length of the deck and climbs away while the ship falls behind.
// Sizes must match blip_assets' CARRIER_W / CARRIER_H.
const CARRIER_W: i32 = 186;
const CARRIER_H: i32 = 760;
const CARRIER_DECK_X: f32 = 88.0;  // the deck centreline in the sprite
const CARRIER_BOW: f32 = 42.0;     // where the flight deck starts in the sprite
const CARRIER_DECK_LEN: f32 = 700.0;
const LAUNCH_TIME: f32 = 7.6;
const LAUNCH_START_END: f32 = 1.7;  // engine start
const LAUNCH_SIGNAL_END: f32 = 2.5; // the launch officer's signal
const LAUNCH_ROLL_END: f32 = 4.9;   // the deck run
const LAUNCH_DECK_POS: f32 = 0.62;  // where on the deck the plane is spotted (0 bow .. 1 stern)
const LAUNCH_WAVE_AT: f32 = 5.4;
const LAUNCH_END_Y: f32 = (WIN_H - 175) as f32;
const LAUNCH_DECK_Y: f32 = WIN_H as f32 * 0.4; // on deck the camera leads, so the ship has room to fall behind
const PROP_MAX_VOLUME: f32 = 0.9;

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
const HEALTH_DROP_CHANCE: f32 = 0.06;
const HEALTH_W: f32 = 14.0;
const HEALTH_H: f32 = 14.0;
const MAX_HEALTH_PICKUPS: usize = 1;
// A short flinch of invulnerability after a non-lethal hit, reusing the
// same respawn-grace timer (and its blink) that already gates hazard
// collisions — otherwise a single burst of overlapping bullets could burn
// through the whole health bar in one frame.
const HIT_GRACE: f32 = 1.15;

// ---- background: sea, sky, boats -------------------------------------------
// The world below the dogfight: an ocean scrolling past underneath (both the
// wave bands and the boats sit on it, so they share one scroll speed), a
// cloud layer floating between the sea and the planes, and enemy boats that
// sail across it.
const SEA_SCROLL_SPEED: f32 = 70.0;

// ---- airspeed ---------------------------------------------------------
// 1.0 is cruise. Forward opens the throttle and back closes it; the plane
// never flies backwards. Below cruise the scrolling world carries it down
// the screen, and slow flight slows the scroll itself.
const AIRSPEED_MAX: f32 = 1.4;
const AIRSPEED_RATE: f32 = 0.9;    // per second of throttle held
const AIRSPEED_RELAX: f32 = 0.3;   // back toward cruise with no input
const AIRSPEED_DRIFT: f32 = 150.0; // screen px/s per unit of airspeed off cruise
const STALL_WARN: f32 = 0.35;      // the engine labours and STALL flashes below this
const STALL_SPEED: f32 = 0.06;
const STALL_HOLD: f32 = 0.5;       // seconds at stall speed before it drops
const STALL_FALL: f32 = 1.5;       // spinning down to the sea
// The engine loops, slowest (coughing, the stall warning) to full throttle,
// and the airspeed each one belongs to.
const ENGINE_AT: [f32; 5] = [0.1, 0.45, 1.0, 1.2, 1.4];
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
// From level BARRIER_MIN_LEVEL: a full-width laser beam that creeps down
// the screen toward the player. There is no way round it and the plane
// cannot back away, so the motor (3-10 hits) must come down before it
// arrives. Rare, and never while a boss is up.
const BARRIER_MIN_LEVEL: i32 = 3;
const BARRIER_MIN_INTERVAL: f32 = 75.0;
const BARRIER_MAX_INTERVAL: f32 = 140.0;
const BARRIER_START_Y: f32 = 90.0; // where the beam appears
const BARRIER_SPEED: f32 = 16.0;    // px/s down the screen at level 3, +2 a level
const BARRIER_BEAM_H: f32 = 10.0;  // collision + visual thickness of the beam
const BARRIER_WARMUP: f32 = 1.6;   // telegraph before the beam can actually hurt you
const BARRIER_HP_MIN: i32 = 3;
const BARRIER_HP_MAX: i32 = 10;    // inclusive — a random 3-10 hits to destroy
const MOTOR_W: f32 = 34.0;
const MOTOR_H: f32 = 26.0;
// How close (in px of vertical distance) the proximity hum starts fading
// in, and its loudest volume once the player is right on top of the beam.
const BARRIER_HUM_MAX_VOLUME: f32 = 0.55;

// ---- explosions -----------------------------------------------------------
const MAX_EXPLOSIONS: usize = MAX_ENEMIES + 16; // + headroom for power-up bursts
const EXPLOSION_TTL: f32 = 0.4;
// Now and then a kill goes down in flames instead of blowing up.
const FLAMES_CHANCE: f32 = 0.3;
// Now and then a round glances off a plane instead of hitting home.
const RICOCHET_CHANCE: f32 = 0.08;
const WRECK_SECS: f32 = 1.7;
const MAX_WRECKS: usize = 6;
const MAX_PUFFS: usize = 120;
const MAX_POWER_BANNER_TIME: f32 = 1.6;

// ---- tuning -------------------------------------------------------------
const LIVES_START: i32 = 3;
const DEAD_PAUSE: f32 = 1.6;
const RESPAWN_GRACE: f32 = 1.5;
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
struct Bullet { x: f32, y: f32, vx: f32, vy: f32, active: bool }
impl Pooled for Bullet {
    fn is_active(&self) -> bool { self.active }
}

/// A player round: centred on x, moving at (vx, -vy) (up the screen, with
/// the spray sideways), with its calibre r (drawn radius; the hit box is
/// 2r + 2 wide). `bounced` once it has ricocheted: it can still hit, but
/// not glance off again.
#[derive(Copy, Clone)]
struct Round { x: f32, y: f32, vx: f32, vy: f32, r: f32, bounced: bool, active: bool }
impl Pooled for Round {
    fn is_active(&self) -> bool { self.active }
}

/// A spent brass casing kicked out of the guns, tumbling away and fading.
#[derive(Copy, Clone)]
struct Casing { x: f32, y: f32, vx: f32, vy: f32, rot: f32, ttl: f32, active: bool }
impl Pooled for Casing {
    fn is_active(&self) -> bool { self.active }
}

/// One enemy aircraft. The leader flies its flight's path (see Flight);
/// a wingman flies itself, on the same physics, steering for its `slot`
/// (px right and behind, in the leader's frame). It only ever moves along
/// its nose, so in a turn it swings wide or cuts inside like a real
/// wingman instead of sliding sideways.
#[derive(Copy, Clone)]
struct Enemy {
    x: f32, y: f32,
    heading: f32,
    bank: f32,
    speed: f32,
    active: bool,
    kind: EnemyKind,
    t: f32,
    flight: usize,
    slot: (f32, f32),
    fire_timer: Timer,
    burst: u8,          // rounds left in the burst being fired
    burst_t: f32,
    can_hide: bool,     // this plane ducks out of sight when it flies under a cloud — see draw_play()
}

/// A leg of a flight plan.
#[derive(Copy, Clone)]
enum Leg {
    Straight(f32),                  // seconds, wings level
    Turn { bank: f32, by: f32 },    // bank (signed, rad) until the heading has swung `by`
    Pursue(f32),                    // seconds chasing the player's lead point
}

/// A formation's path through the air (see the flight model constants).
#[derive(Copy, Clone)]
struct Flight {
    active: bool,
    x: f32, y: f32,
    heading: f32, bank: f32, speed: f32, trim: f32,
    max_bank: f32, // flat enough that the inside wingman stays above stall
    legs: [Leg; 4], n: usize, leg: usize, leg_t: f32, turned: f32,
    t: f32,
}
impl Pooled for Enemy {
    fn is_active(&self) -> bool { self.active }
}

#[derive(Copy, Clone)]
struct Explosion { x: f32, y: f32, ttl: f32, max_ttl: f32, scale: f32, color: BlipColor, active: bool }

/// A shot-down plane falling to the sea: it carries on the way it was going
/// (bent a little, never turning back), rolls, shrinks as it drops, trails
/// fire and smoke, then splashes. `heading` is only how the sprite is
/// turned; `course` is where it goes.
#[derive(Copy, Clone)]
struct Wreck {
    x: f32, y: f32, heading: f32, spin: f32, speed: f32, t: f32, kind: EnemyKind, puff_t: f32, active: bool,
    /// Each fall is its own: its course, how that bends, a roll wobble, how long it takes.
    course: f32, bend: f32, wobble: f32, dur: f32,
}
impl Pooled for Wreck {
    fn is_active(&self) -> bool { self.active }
}

/// Fire (short, bright, shrinking) or smoke (dark, growing, drifting with the sea).
#[derive(Copy, Clone)]
struct Puff { x: f32, y: f32, r: f32, grow: f32, ttl: f32, max_ttl: f32, fire: bool, top: bool, active: bool }  // top: drawn over the player (its own damage)
impl Pooled for Puff {
    fn is_active(&self) -> bool { self.active }
}
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

/// A laser barrier across the full width of the screen at row `y`, and
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
    y: f32,
}

/// A gun turret on a boss: it traverses toward its target at a real
/// turret's slew rate, fires bursts along its barrels, and can be shot out.
#[derive(Copy, Clone)]
struct Turret {
    fx: f32, fy: f32, // x: fraction of the half span, y: fraction of the length from the nose
    angle: f32,       // barrels' direction, 0 = straight down the screen
    hp: i32,
    burst: u8, burst_t: f32,
    smoke_t: f32,
}

#[derive(Copy, Clone)]
struct Boss {
    turrets: [Turret; MAX_TURRETS],
    n_turrets: usize,
    x: f32, y: f32,
    active: bool,
    entered: bool, // finished its entrance descent, patrolling now
    hp: i32, max_hp: i32,
    dir: f32,
    tier: usize, // 0..=6, indexes BOSS_SPECS / BOSS_SIZES (level - 1)
    t: f32,      // seconds since spawn, drives the dip wiggle and the sweep pattern
    volley: u32, // volleys fired so far — cycles spec.patterns
    fire_timer: Timer,
    escort_timer: Timer,
}

/// How a boss's gunners aim a volley; tougher bosses mix in more of them.
#[derive(Copy, Clone, PartialEq)]
enum BossPattern {
    /// A spread down the bomber's track.
    Fan,
    /// Bursts laid on the player.
    Aimed,
    /// Each turret traversing slowly across the sky.
    Sweep,
    /// A wide barrage from every turret.
    Curtain,
}

/// One row per boss (levels 1-7). `patterns` cycle one per volley; `dips` sinks the boss toward
/// the player now and then; `escorts` calls in a pair of fighters.
struct BossSpec {
    hp: i32,
    speed: f32,
    fire_min: f32,
    fire_max: f32,
    dips: bool,
    escorts: bool,
    patterns: &'static [BossPattern],
    name: &'static str,
}

const BOSS_SPECS: [BossSpec; 7] = [
    BossSpec { hp:  80, speed:  76.0, fire_min: 0.48, fire_max: 1.00, dips: false, escorts: false, patterns: &[BossPattern::Fan],                                                name: "KI-49 DONRYU"   },
    BossSpec { hp: 130, speed:  84.0, fire_min: 0.44, fire_max: 0.92, dips: false, escorts: false, patterns: &[BossPattern::Fan],                                                name: "KI-67 HIRYU"    },
    BossSpec { hp: 190, speed:  92.0, fire_min: 0.38, fire_max: 0.82, dips: true,  escorts: false, patterns: &[BossPattern::Fan, BossPattern::Aimed],                            name: "G4M BETTY"      },
    BossSpec { hp: 260, speed: 100.0, fire_min: 0.34, fire_max: 0.72, dips: true,  escorts: false, patterns: &[BossPattern::Fan, BossPattern::Aimed, BossPattern::Sweep],       name: "G8N RENZAN"     },
    BossSpec { hp: 340, speed: 109.0, fire_min: 0.30, fire_max: 0.64, dips: true,  escorts: true,  patterns: &[BossPattern::Aimed, BossPattern::Sweep],                         name: "H8K EMILY"      },
    BossSpec { hp: 430, speed: 118.0, fire_min: 0.27, fire_max: 0.58, dips: true,  escorts: true,  patterns: &[BossPattern::Sweep, BossPattern::Aimed, BossPattern::Curtain],   name: "G5N SHINZAN"    },
    BossSpec { hp: 560, speed: 132.0, fire_min: 0.22, fire_max: 0.47, dips: true,  escorts: true,  patterns: &[BossPattern::Fan, BossPattern::Aimed, BossPattern::Sweep, BossPattern::Curtain], name: "FUGAKU"         },
];

fn boss_size(tier: usize) -> (f32, f32) {
    let (w, h) = BOSS_SIZES[tier];
    (w as f32, h as f32)
}

struct Game {
    player_x: f32,
    player_y: f32,
    player_vx: f32,
    player_vy: f32,
    player_bank: f32,   // cosmetic roll while strafing — eased toward a target, not instant
    airspeed: f32,      // see AIRSPEED_*: 1.0 = cruise
    stall_t: f32,       // seconds spent at stall speed
    stall_fall: f32,    // > 0 while spinning down after a stall
    stall_spin: f32,
    scroll_k: f32,      // the world's scroll, relative to cruise
    ship_y: f32,        // carrier position during the launch sequence
    launch_climb: f32,  // 0..1 progress up the launch climb — drives the plane's grow-in scale
    launch_wave_up: bool, // opening wave already scrambled in for this launch
    weapon_level: i32,
    health: i32,
    bullets: [Round; MAX_PLAYER_BULLETS],
    casings: [Casing; MAX_CASINGS],
    /// Muzzle flash left on the guns, seconds.
    muzzle_t: f32,
    /// Alternates the casing ejection side, burst by burst.
    eject_left: bool,
    enemy_bullets: [Bullet; MAX_ENEMY_BULLETS],
    enemies: [Enemy; MAX_ENEMIES],
    flights: [Flight; MAX_FLIGHTS],
    gun_cd: f32,      // keeps the enemy gun sound from stacking
    smoke_t: f32,     // damage smoke cadence
    backfire_t: f32,  // near a stall, the engine backfires
    launch_t: f32,    // seconds into the carrier launch
    carrier_scale: f32,
    explosions: [Explosion; MAX_EXPLOSIONS],
    wrecks: [Wreck; MAX_WRECKS],
    puffs: [Puff; MAX_PUFFS],
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
    (WAVE_KILL_BASE + (level - 1) * 15).min(160)
}

/// Seconds between flights arriving.
fn spawn_interval_range(level: i32) -> (f32, f32) {
    let l = (level - 1).min(6) as f32;
    (1.5 - l * 0.08, 2.6 - l * 0.14)
}

fn rand01() -> f32 {
    (rand() as f32) / (u32::MAX as f32)
}

// ---- weapon tiers ---------------------------------------------------------
// One place per property, all indexed by weapon_level (1..=MAX_WEAPON_LEVEL),
// so the escalation from "single popgun" to "seven-way glowing barrage" stays
// easy to tune as one coherent ladder.

/// Bullet spawn offsets from the player's centreline, one shot's worth.
/// (rounds per burst, fan width in px, calibre radius) for a tier. Rounds
/// leave evenly across the fan and splay outward with their offset.
fn weapon_guns(level: i32) -> (usize, f32, f32) {
    match level {
        1 => (2, 14.0, 2.0),
        2 => (3, 24.0, 2.4),
        3 => (5, 40.0, 2.8),
        4 => (6, 52.0, 3.3),
        _ => (8, 64.0, 3.9),
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

impl Game {
    fn new() -> Self {
        let dead_bullet = Bullet { x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, active: false };
        let dead_enemy = Enemy {
            x: 0.0, y: 0.0, heading: 0.0, bank: 0.0, speed: 100.0, active: false, kind: EnemyKind::Grunt,
            t: 0.0, flight: 0, slot: (0.0, 0.0), fire_timer: Timer::default(), burst: 0, burst_t: 0.0,
            can_hide: false,
        };
        let dead_flight = Flight { active: false, x: 0.0, y: 0.0, heading: 0.0, bank: 0.0, speed: 100.0,
            trim: 100.0, max_bank: FLIGHT_MAX_BANK, legs: [Leg::Straight(0.0); 4], n: 0, leg: 0, leg_t: 0.0, turned: 0.0, t: 0.0 };
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
            player_vx: 0.0,
            player_vy: 0.0,
            player_bank: 0.0,
            airspeed: 1.0,
            stall_t: 0.0,
            stall_fall: 0.0,
            stall_spin: 1.0,
            scroll_k: 1.0,
            ship_y: (WIN_H + CARRIER_H) as f32,
            launch_climb: 0.0,
            launch_wave_up: false,
            weapon_level: 1,
            health: PLAYER_HEALTH_MAX,
            bullets: [Round { x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, r: 0.0, bounced: false, active: false }; MAX_PLAYER_BULLETS],
            casings: [Casing { x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, rot: 0.0, ttl: 0.0, active: false }; MAX_CASINGS],
            muzzle_t: 0.0,
            eject_left: false,
            enemy_bullets: [dead_bullet; MAX_ENEMY_BULLETS],
            enemies: [dead_enemy; MAX_ENEMIES],
            flights: [dead_flight; MAX_FLIGHTS],
            gun_cd: 0.0,
            smoke_t: 0.0,
            backfire_t: 0.0,
            launch_t: 0.0,
            carrier_scale: 1.0,
            explosions: [dead_explosion; MAX_EXPLOSIONS],
            wrecks: [Wreck { x: 0.0, y: 0.0, heading: 0.0, spin: 0.0, speed: 0.0, t: 0.0, kind: EnemyKind::Grunt,
                puff_t: 0.0, active: false, course: 0.0, bend: 0.0, wobble: 0.0, dur: 1.0 }; MAX_WRECKS],
            puffs: [Puff { x: 0.0, y: 0.0, r: 0.0, grow: 0.0, ttl: 0.0, max_ttl: 1.0, fire: false, top: false, active: false }; MAX_PUFFS],
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
                warmup: Timer::default(), t: 0.0, y: BARRIER_START_Y,
            },
            barrier_timer: { let mut t = Timer::default(); t.start(BARRIER_MIN_INTERVAL); t },
            boss: Boss {
                turrets: [Turret { fx: 0.0, fy: 0.0, angle: 0.0, hp: 0, burst: 0, burst_t: 0.0, smoke_t: 0.0 }; MAX_TURRETS],
                n_turrets: 0,
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
        for c in self.casings.iter_mut() { c.active = false; }
        for b in self.enemy_bullets.iter_mut() { b.active = false; }
        for e in self.enemies.iter_mut() { e.active = false; }
        for p in self.powerups.iter_mut() { p.active = false; }
        for h in self.health_pickups.iter_mut() { h.active = false; }
        for e in self.explosions.iter_mut() { e.active = false; }
        for w in self.wrecks.iter_mut() { w.active = false; }
        for p in self.puffs.iter_mut() { p.active = false; }
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

        // Launch sequence: down on the carrier deck at the bottom edge,
        // climbing away from it up into the fight. update_launch() drives
        // player_y / ship_y / launch_climb from here.
        self.player_x = ((WIN_W - PLAYER_W) / 2) as f32;
        self.player_y = LAUNCH_DECK_Y;
        self.player_bank = 0.0;
        self.player_vx = 0.0;
        self.player_vy = 0.0;
        self.airspeed = 1.0;
        self.stall_t = 0.0;
        self.stall_fall = 0.0;
        self.scroll_k = 1.0;
        self.ship_y = (WIN_H - CARRIER_H / 2) as f32;
        self.launch_climb = 0.0;
        self.launch_wave_up = false;
        self.launch_t = 0.0;
        self.carrier_scale = 1.0;
        self.ship_y = 0.0;
        for f in self.flights.iter_mut() { f.active = false; }
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
    /// Per weapon tier, two takes of that many guns firing together.
    shoot: Vec<[blip::BlipSound; 2]>,
    enemy_explode: blip::BlipSound,
    player_explode: blip::BlipSound,
    player_hit: blip::BlipSound,
    ricochet: [blip::BlipSound; 2],
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
    barrier_hum2: blip::BlipSound,
    // Carrier launch: a one-shot engine crank/catch, plus a seamless
    // propeller loop update_launch() fades in and out around it.
    engine_start: blip::BlipSound,
    enemy_gun: blip::BlipSound,
    backfire: blip::BlipSound,
    /// In-flight engine loops, slowest first (see ENGINE_AT, engine_mix()).
    engine: [blip::BlipSound; 5],
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
    g.barrier = Barrier { active: true, motor_x, hp, max_hp: hp, warmup, t: 0.0, y: BARRIER_START_Y };
    g.barrier_timer.start(BARRIER_MIN_INTERVAL + rand01() * (BARRIER_MAX_INTERVAL - BARRIER_MIN_INTERVAL));
    // Silent to start; update_barrier() rides both loops by distance.
    play_sound(&sfx.barrier_hum, PlaySoundParams { looped: true, volume: 0.0 });
    play_sound(&sfx.barrier_hum2, PlaySoundParams { looped: true, volume: 0.0 });
}

/// Laser barrier upkeep: after its warm-up the beam creeps down the screen;
/// the hum builds as it closes on the player (a low drone from the start,
/// a harsher buzz crossfading in over the second half) and fades once it
/// has gone past.
fn update_barrier(g: &mut Game, dt: f32, sfx: &Sounds) {
    if !g.barrier.active {
        stop_sound(&sfx.barrier_hum);
        stop_sound(&sfx.barrier_hum2);
        return;
    }
    g.barrier.t += dt;
    g.barrier.warmup.tick(dt);
    if !g.barrier.warmup.active() {
        g.barrier.y += (BARRIER_SPEED + 2.0 * (g.sess.level - BARRIER_MIN_LEVEL).max(0) as f32) * dt;
    }
    if g.barrier.y > WIN_H as f32 + 20.0 {
        g.barrier.active = false;
        stop_sound(&sfx.barrier_hum);
        stop_sound(&sfx.barrier_hum2);
        return;
    }
    let player_cy = g.player_y + PLAYER_H as f32 / 2.0;
    let ahead = player_cy - g.barrier.y; // > 0 while it is still coming
    let k = if ahead >= 0.0 {
        1.0 - (ahead / (player_cy - BARRIER_START_Y).max(1.0)).clamp(0.0, 1.0)
    } else {
        (1.0 + ahead / 120.0).max(0.0)
    };
    set_sound_volume(&sfx.barrier_hum, (0.12 + 0.88 * k) * BARRIER_HUM_MAX_VOLUME);
    set_sound_volume(&sfx.barrier_hum2, ((k - 0.5) * 2.0).max(0.0).powf(1.5) * BARRIER_HUM_MAX_VOLUME);
}

/// The world under the dogfight: sea scroll, cloud drift, and boat/island
/// spawns — shared by update_launch() and update_play() so the background
/// keeps moving through the carrier takeoff too, not just once the fight
/// starts. Turret *firing* is play-only (see update_islands()) — islands
/// still drift past harmlessly during launch, same as the boats.
fn update_background(g: &mut Game, dt: f32) {
    let sc = SEA_SCROLL_SPEED * g.scroll_k;
    g.sea_scroll += sc * dt;

    for c in g.clouds.iter_mut() {
        c.y += c.speed * g.scroll_k * dt;
        if c.y - c.r > WIN_H as f32 {
            c.y = -c.r;
            c.x = rand01() * WIN_W as f32;
        }
    }

    for b in pool_iter_mut(&mut g.boats) {
        b.y += sc * dt;
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
        isl.y += sc * dt;
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

fn trim_speed(kind: EnemyKind) -> f32 {
    match kind {
        EnemyKind::Grunt  => 115.0, // A6M Zero
        EnemyKind::Weaver => 125.0, // Ki-43
        EnemyKind::Ace    => 110.0, // Ki-84, flown by someone who knows how
    }
}

#[derive(Copy, Clone)]
enum Formation { Vic, Pair, Echelon, Abreast, FingerFour, Solo }

/// Slots (right, behind) in the leader's frame, px.
fn slots(f: Formation) -> &'static [(f32, f32)] {
    match f {
        Formation::Vic     => &[(0.0, 0.0), (-52.0, 42.0), (52.0, 42.0)],
        Formation::Pair    => &[(0.0, 0.0), (56.0, 38.0)],
        Formation::Echelon => &[(0.0, 0.0), (52.0, 40.0), (104.0, 80.0)],
        Formation::Abreast => &[(-62.0, 0.0), (0.0, 0.0), (62.0, 0.0)],
        Formation::FingerFour => &[(0.0, 0.0), (-52.0, 40.0), (56.0, 34.0), (108.0, 72.0)],
        Formation::Solo    => &[(0.0, 0.0)],
    }
}

/// Where a slot sits in the world for a flight point and heading.
fn slot_pos(fx: f32, fy: f32, heading: f32, slot: (f32, f32)) -> (f32, f32) {
    let (fwd_x, fwd_y) = (heading.sin(), heading.cos());
    let (right_x, right_y) = (-heading.cos(), heading.sin());
    (fx + right_x * slot.0 - fwd_x * slot.1, fy + right_y * slot.0 - fwd_y * slot.1)
}

fn active_enemies(g: &Game) -> usize { pool_iter(&g.enemies).count() }

/// Put a formation in the air at (x, y) flying `heading` along `legs`.
fn spawn_flight(g: &mut Game, kind: EnemyKind, form: Formation, x: f32, y: f32, heading: f32, legs: &[Leg]) {
    let members = slots(form);
    if active_enemies(g) + members.len() > MAX_ENEMIES { return; }
    let Some(fi) = (0..MAX_FLIGHTS).find(|&i| !g.flights[i].active && !g.enemies.iter().any(|e| e.active && e.flight == i)) else { return };
    let mut plan = [Leg::Straight(6.0); 4];
    for (i, l) in legs.iter().take(4).enumerate() { plan[i] = *l; }
    let trim = trim_speed(kind);
    // In a level turn the whole formation turns at one rate, so a wingman
    // `lat` inside flies at v(1 - lat/r). Keeping that above stall needs
    // r >= lat / (1 - v_min/v), i.e. bank <= atan(v^2 / (g r)).
    let lat = members.iter().map(|s| s.0.abs()).fold(0.0, f32::max);
    let max_bank = if lat == 0.0 { FLIGHT_MAX_BANK } else {
        let r = lat / (1.0 - FLIGHT_STALL * 1.27 / trim);
        (trim * trim / (FLIGHT_G * r)).atan().min(FLIGHT_MAX_BANK)
    };
    g.flights[fi] = Flight { active: true, x, y, heading, bank: 0.0, speed: trim, trim, max_bank,
        legs: plan, n: legs.len().min(4), leg: 0, leg_t: 0.0, turned: 0.0, t: 0.0 };
    let can_hide = kind != EnemyKind::Ace && rand01() < 0.4;
    for &slot in members {
        let (sx, sy) = slot_pos(x, y, heading, slot);
        let mut fire_timer = Timer::default();
        fire_timer.start(0.8 + rand01() * 1.4);
        pool_spawn(&mut g.enemies, Enemy {
            x: sx - ENEMY_W as f32 / 2.0, y: sy - ENEMY_H as f32 / 2.0, heading, bank: 0.0, speed: trim, active: true,
            kind, t: 0.0, flight: fi, slot, fire_timer, burst: 0, burst_t: 0.0, can_hide,
        });
    }
}

/// A diving pass: in from the top toward where the player is, then a hard
/// banked break-away to one side and out.
fn spawn_dive_pass(g: &mut Game, kind: EnemyKind, form: Formation) {
    let x = 90.0 + rand01() * (WIN_W as f32 - 180.0);
    let px = g.player_x + PLAYER_W as f32 / 2.0;
    let heading = (px - x).atan2(420.0).clamp(-0.35, 0.35);
    let side = if x < WIN_W as f32 / 2.0 { 1.0 } else { -1.0 };
    spawn_flight(g, kind, form, x, -70.0, heading, &[
        Leg::Straight(1.4 + rand01() * 0.6),
        Leg::Turn { bank: side * 0.95, by: 1.8 + rand01() * 0.6 },
        Leg::Straight(6.0),
    ]);
}

/// A crossing pass: in from a top corner on a long diagonal, easing into a
/// gentle turn downward and out the bottom (`high`: straight across the
/// top of the screen instead).
fn spawn_crossing(g: &mut Game, kind: EnemyKind, form: Formation, high: bool) {
    let from_left = rand01() < 0.5;
    let s = if from_left { 1.0 } else { -1.0 };
    let x = if from_left { -70.0 } else { WIN_W as f32 + 70.0 };
    let y = 40.0 + rand01() * 110.0;
    if high {
        spawn_flight(g, kind, form, x, y, s * 1.35, &[Leg::Straight(9.0)]);
        return;
    }
    spawn_flight(g, kind, form, x, y, s * (0.95 + rand01() * 0.25), &[
        Leg::Straight(1.8 + rand01() * 0.6),
        Leg::Turn { bank: -s * 0.5, by: 0.7 },
        Leg::Straight(6.0),
    ]);
}

/// Down into the fight, a 180-degree turn, and away up the screen.
fn spawn_turn_back(g: &mut Game, kind: EnemyKind, form: Formation) {
    let x = 110.0 + rand01() * (WIN_W as f32 - 220.0);
    let side = if x < WIN_W as f32 / 2.0 { 1.0 } else { -1.0 };
    spawn_flight(g, kind, form, x, -70.0, 0.0, &[
        Leg::Straight(1.2 + rand01() * 0.5),
        Leg::Turn { bank: side * 1.0, by: std::f32::consts::PI },
        Leg::Straight(6.0),
    ]);
}

/// A lone ace: chases the player's lead point, then breaks away.
fn spawn_ace(g: &mut Game) {
    let x = 80.0 + rand01() * (WIN_W as f32 - 160.0);
    let side = if x < WIN_W as f32 / 2.0 { 1.0 } else { -1.0 };
    spawn_flight(g, EnemyKind::Ace, Formation::Solo, x, -70.0, 0.0, &[
        Leg::Pursue(3.4),
        Leg::Turn { bank: side * 1.1, by: 2.2 },
        Leg::Straight(6.0),
    ]);
}

/// The opening wave, in the air as the player climbs away from the carrier.
fn spawn_opening_wave(g: &mut Game) {
    spawn_dive_pass(g, EnemyKind::Grunt, Formation::Vic);
    spawn_crossing(g, EnemyKind::Weaver, Formation::Pair, true);
    if g.sess.level >= 2 { spawn_dive_pass(g, EnemyKind::Grunt, Formation::Pair); }
}

/// One spawner tick: a flight of Zeros diving on the player (a finger-four
/// from level 2), Ki-43s crossing in echelon or turning back in a vic, or a
/// lone Ki-84; from level 2 more and more often a second flight with it.
fn spawn_wave_tick(g: &mut Game) {
    let level = g.sess.level;
    let flights = if rand01() < (0.15 * (level - 1) as f32).min(0.6) { 2 } else { 1 };
    for _ in 0..flights {
        let r = rand01();
        if r < 0.32 {
            let form = if level >= 2 && rand01() < 0.5 { Formation::FingerFour } else { Formation::Vic };
            spawn_dive_pass(g, EnemyKind::Grunt, form);
        }
        else if r < 0.52 { spawn_dive_pass(g, EnemyKind::Grunt, Formation::Pair); }
        else if r < 0.68 { spawn_crossing(g, EnemyKind::Weaver, Formation::Echelon, false); }
        else if r < 0.78 { spawn_crossing(g, EnemyKind::Weaver, Formation::Abreast, false); }
        else if r < 0.90 { spawn_turn_back(g, EnemyKind::Weaver, Formation::Vic); }
        else { spawn_ace(g); }
    }
}

/// Fly every flight one step (see the flight model constants).
fn update_flight(f: &mut Flight, dt: f32, player: (f32, f32, f32)) {
    f.t += dt;
    f.leg_t += dt;
    let leg = if f.leg < f.n { f.legs[f.leg] } else { Leg::Straight(99.0) };
    let w_now = FLIGHT_G * f.bank.tan() / f.speed;
    let mut cmd = match leg {
        Leg::Straight(_) => 0.0,
        Leg::Turn { bank, .. } => bank,
        Leg::Pursue(_) => {
            // pure pursuit of where the player will be, damped by the turn
            // already under way so it converges instead of hunting
            let (px, py, pvx) = player;
            let (lx, ly) = (px + pvx * 0.5, py);
            let want = (lx - f.x).atan2((ly - f.y).max(40.0));
            let mut err = want - f.heading;
            err = (err + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            err * 1.4 - w_now * 0.5
        }
    };
    let lim = ((FLIGHT_STALL / f.speed).powi(2)).min(1.0).acos().min(f.max_bank);
    cmd = cmd.clamp(-lim, lim);
    f.bank += (cmd - f.bank).clamp(-FLIGHT_ROLL * dt, FLIGHT_ROLL * dt);
    let w = FLIGHT_G * f.bank.tan() / f.speed;
    f.heading += w * dt;
    f.turned += (w * dt).abs();
    let n = 1.0 / f.bank.cos();
    f.speed += (FLIGHT_THRUST * (f.trim - f.speed) - FLIGHT_DRAG * (n * n - 1.0) * f.speed) * dt;
    f.speed = f.speed.max(FLIGHT_STALL * 1.15);
    f.x += f.heading.sin() * f.speed * dt;
    f.y += f.heading.cos() * f.speed * dt;
    let done = match leg {
        Leg::Straight(secs) | Leg::Pursue(secs) => f.leg_t >= secs,
        Leg::Turn { by, .. } => f.turned >= by,
    };
    if done && f.leg < f.n { f.leg += 1; f.leg_t = 0.0; f.turned = 0.0; }
}

/// A wingman's step, flown as a real one would: bank to match the
/// leader's turn rate (tan(bank) = w v / g at its own speed), correct
/// gently toward a point well ahead of its slot, and use the throttle to
/// hold its place along the track. Returns its new centre.
fn fly_wing(e: &mut Enemy, f: &Flight, dt: f32) -> (f32, f32) {
    let (cx, cy) = (e.x + ENEMY_W as f32 / 2.0, e.y + ENEMY_H as f32 / 2.0);
    let (sx, sy) = slot_pos(f.x, f.y, f.heading, e.slot);
    let (ax, ay) = (sx + f.heading.sin() * 110.0, sy + f.heading.cos() * 110.0);
    let want = (ax - cx).atan2(ay - cy);
    let err = (want - e.heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    let lim = ((FLIGHT_STALL / e.speed).powi(2)).min(1.0).acos().min(FLIGHT_MAX_BANK);
    let w_lead = FLIGHT_G * f.bank.tan() / f.speed;
    let cmd = ((w_lead * e.speed / FLIGHT_G).atan() + err * 1.6).clamp(-lim, lim);
    e.bank += (cmd - e.bank).clamp(-FLIGHT_ROLL * 1.3 * dt, FLIGHT_ROLL * 1.3 * dt);
    e.heading += FLIGHT_G * e.bank.tan() / e.speed * dt;
    let along = (sx - cx) * f.heading.sin() + (sy - cy) * f.heading.cos();
    let target = (f.speed + along * 1.2).min(f.speed * 1.35).max(FLIGHT_STALL * 1.15);
    e.speed += (target - e.speed) * (2.0 * dt).min(1.0);
    (cx + e.heading.sin() * e.speed * dt, cy + e.heading.cos() * e.speed * dt)
}

fn update_enemies(g: &mut Game, dt: f32, allow_fire: bool, sfx: &Sounds) {
    let player = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0, g.player_vx);
    for f in g.flights.iter_mut() {
        if f.active { update_flight(f, dt, player); }
    }
    g.gun_cd = (g.gun_cd - dt).max(0.0);
    let lvl = (g.sess.level - 1) as f32; // every level fires longer bursts, more often
    for i in 0..MAX_ENEMIES {
        if !g.enemies[i].active { continue; }
        let e = &mut g.enemies[i];
        e.t += dt;
        let f = g.flights[e.flight];
        let (cx, cy) = if e.slot == (0.0, 0.0) {
            e.heading = f.heading;
            e.bank = f.bank;
            e.speed = f.speed;
            (f.x, f.y)
        } else {
            fly_wing(e, &f, dt)
        };
        e.x = cx - ENEMY_W as f32 / 2.0;
        e.y = cy - ENEMY_H as f32 / 2.0;
        if e.t > 2.0 && (cx < -80.0 || cx > WIN_W as f32 + 80.0 || cy > WIN_H as f32 + 80.0 || cy < -140.0) {
            e.active = false;
            continue;
        }
        if !allow_fire { continue; }
        // Guns fire along the nose: only when it points down the screen
        // and the player is near the line of fire, in short bursts.
        let (fx, fy) = (e.heading.sin(), e.heading.cos());
        if e.burst == 0 && e.fire_timer.tick(dt) {
            let (dx, dy) = (player.0 - cx, player.1 - cy);
            let d = dx.hypot(dy).max(1.0);
            let on_line = (dx * fx + dy * fy) / d;
            if fy > 0.3 && on_line > 0.9 - 0.015 * lvl && d < 420.0 + 20.0 * lvl {
                e.burst = (3.0 + lvl * 0.5).min(7.0) as u8;
                e.burst_t = 0.0;
                if g.gun_cd <= 0.0 {
                    play_sfx_volume(&sfx.enemy_gun, 0.4);
                    g.gun_cd = 0.18;
                }
            }
            let (mn, mx) = if e.kind == EnemyKind::Ace { (0.8, 1.5) } else { (1.2, 2.4) };
            e.fire_timer.start((mn + rand01() * (mx - mn)) / (1.0 + 0.12 * lvl));
        }
        if e.burst > 0 {
            e.burst_t -= dt;
            if e.burst_t <= 0.0 {
                e.burst -= 1;
                e.burst_t = 0.08;
                let (rx, ry) = (-e.heading.cos(), e.heading.sin());
                for side in [-1.0f32, 1.0] {
                    let (gx, gy) = (cx + rx * side * 15.0 + fx * 16.0, cy + ry * side * 15.0 + fy * 16.0);
                    pool_spawn(&mut g.enemy_bullets, Bullet {
                        x: gx, y: gy, vx: fx * ENEMY_BULLET_SPEED, vy: fy * ENEMY_BULLET_SPEED, active: true,
                    });
                }
            }
        }
    }
    for fi in 0..MAX_FLIGHTS {
        if g.flights[fi].active && g.flights[fi].t > 3.0 && !g.enemies.iter().any(|e| e.active && e.flight == fi) {
            g.flights[fi].active = false;
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
    let mut turrets = [Turret { fx: 0.0, fy: 0.0, angle: 0.0, hp: 0, burst: 0, burst_t: 0.0, smoke_t: 0.0 }; MAX_TURRETS];
    let mut n = 0;
    let (rle, rte, tle, tte) = BOSS_WINGS[tier];
    let mounts = BOSS_TURRETS[tier].iter().copied().chain(BOSS_WING_GUNS[tier].iter().flat_map(|&t| {
        let mid = (rle + (tle - rle) * t + rte + (tte - rte) * t) / 2.0;
        [(-t, mid), (t, mid)]
    }));
    for (fx, fy) in mounts.take(MAX_TURRETS) {
        turrets[n] = Turret { fx, fy, angle: 0.0, hp: 6 + tier as i32 * 2, burst: 0, burst_t: rand01() * 0.3, smoke_t: 0.0 };
        n += 1;
    }
    g.boss = Boss {
        turrets,
        n_turrets: n,
        x: (WIN_W as f32 - bw) / 2.0, // centred: its patrol swings about the middle
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

/// The boss's bank in the draw (radians): it leans into its patrol.
fn boss_bank(g: &Game) -> f32 { -g.boss.dir * 0.05 }

/// A turret's centre on screen: the sprite flies nose-down and is drawn
/// rotated by boss_bank() about its centre.
fn turret_pos(g: &Game, k: usize) -> (f32, f32) {
    let (bw, bh) = boss_size(g.boss.tier);
    let t = &g.boss.turrets[k];
    let (cx, cy) = (g.boss.x + bw / 2.0, g.boss.y + bh / 2.0);
    let (dx, dy) = (t.fx * bw / 2.0, (0.5 - t.fy) * bh);
    let (s, c) = boss_bank(g).sin_cos();
    (cx + dx * c - dy * s, cy + dx * s + dy * c)
}

fn turret_radius(tier: usize) -> f32 { 4.2 + tier as f32 * 0.3 }

/// One round from (x, y) at `angle` off straight down.
fn boss_round(g: &mut Game, x: f32, y: f32, angle: f32) {
    let (s, c) = angle.sin_cos();
    pool_spawn(&mut g.enemy_bullets, Bullet {
        x, y, vx: s * ENEMY_BULLET_SPEED * 0.95, vy: c * ENEMY_BULLET_SPEED * 0.95, active: true,
    });
}

/// Rounds per turret burst: more every other level.
fn boss_burst_len(tier: usize) -> u8 { 2 + tier as u8 / 2 }

/// A volley: every live turret starts a burst, staggered a little so the
/// gunners do not fire as one. `spec.patterns[volley % len]` picks how they
/// aim: a spread down the track, laid on the player, a sweep, or a barrage.
fn boss_fire(g: &mut Game) {
    g.boss.volley = g.boss.volley.wrapping_add(1);
    let len = boss_burst_len(g.boss.tier);
    for k in 0..g.boss.n_turrets {
        let t = &mut g.boss.turrets[k];
        if t.hp <= 0 { continue; }
        t.burst = len;
        t.burst_t = rand01() * 0.25;
    }
}

/// Traverse and fire every turret. Traverse is rate-limited (about 100
/// degrees a second), so a fast-moving player can outrun the gunners.
fn update_turrets(g: &mut Game, dt: f32) {
    let spec = &BOSS_SPECS[g.boss.tier];
    let pattern = spec.patterns[g.boss.volley as usize % spec.patterns.len()];
    let (px, py) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0);
    let r = turret_radius(g.boss.tier);
    for k in 0..g.boss.n_turrets {
        let (x, y) = turret_pos(g, k);
        let bt = g.boss.t;
        let t = &mut g.boss.turrets[k];
        if t.hp <= 0 {
            t.smoke_t -= dt;
            if t.smoke_t <= 0.0 {
                t.smoke_t = 0.12;
                pool_spawn(&mut g.puffs, Puff { x, y, r: 2.5, grow: 12.0, ttl: 0.8, max_ttl: 0.8, fire: false, top: false, active: true });
            }
            continue;
        }
        let aim = (px - x).atan2(py - y);
        let want = match pattern {
            BossPattern::Aimed => aim,
            BossPattern::Fan => aim * 0.5 + t.fx * 0.6,
            BossPattern::Sweep => (bt * 0.8 + k as f32 * 0.7).sin() * 0.9,
            BossPattern::Curtain => t.fx * 1.1 + (bt * 1.3).sin() * 0.3,
        };
        let err = (want - t.angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        t.angle += err.clamp(-1.75 * dt, 1.75 * dt);
        if t.burst > 0 {
            t.burst_t -= dt;
            if t.burst_t <= 0.0 {
                t.burst -= 1;
                t.burst_t = 0.075;
                let a = t.angle + (rand01() - 0.5) * 0.06;
                let (s, c) = a.sin_cos();
                // twin barrels, firing alternately
                let side = if t.burst % 2 == 0 { 1.0 } else { -1.0 };
                let (ox, oy) = (c * 1.8 * side, -s * 1.8 * side);
                boss_round(g, x + s * r * 1.8 + ox, y + c * r * 1.8 + oy, a);
            }
        }
    }
}

fn update_boss(g: &mut Game, dt: f32) {
    if !g.boss.active { return; }
    // Stay off-screen until the WARNING banner has cleared.
    if g.boss_intro.active() { return; }

    let spec = &BOSS_SPECS[g.boss.tier];
    let (bw, _bh) = boss_size(g.boss.tier);
    g.boss.t += dt;

    let target_y = (HUD_H + 40) as f32;
    if !g.boss.entered {
        g.boss.y = (g.boss.y + 60.0 * dt).min(target_y);
        if g.boss.y >= target_y { g.boss.entered = true; g.boss.t = 0.0; }
        return;
    }

    // A big aeroplane does not zig-zag: it drifts across the sky in long,
    // gentle curves (the bank shows in the draw), a little faster per tier.
    let amp = ((WIN_W as f32 - bw) / 2.0 - 8.0).max(0.0);
    let w = 0.25 + spec.speed / 900.0;
    g.boss.x = (WIN_W as f32 - bw) / 2.0 + amp * (g.boss.t * w).sin();
    g.boss.dir = (g.boss.t * w).cos();
    let dip = if spec.dips { (g.boss.t * 0.5).sin().max(0.0) * 16.0 } else { 0.0 };
    g.boss.y = target_y + dip;

    // The largest boss enrages below half health: its gunners fire twice as often.
    let hp_frac = g.boss.hp as f32 / g.boss.max_hp as f32;
    let enraged = g.boss.tier == BOSS_SPECS.len() - 1 && hp_frac <= 0.5;
    let (fmin, fmax) = if enraged { (spec.fire_min * 0.5, spec.fire_max * 0.5) } else { (spec.fire_min, spec.fire_max) };
    if g.boss.fire_timer.tick(dt) {
        boss_fire(g);
        let guns = 1.0 + g.boss.n_turrets as f32 * 0.09;
        g.boss.fire_timer.start((fmin + rand01() * (fmax - fmin)) * guns);
    }
    update_turrets(g, dt);
    if spec.escorts && g.boss.escort_timer.tick(dt) {
        spawn_dive_pass(g, EnemyKind::Grunt, Formation::Pair);
        g.boss.escort_timer.start(5.0 + rand01() * 2.5);
    }
}

fn update_title(g: &mut Game) {
    if btn1_pressed() { g.start_game(); }
}

/// Carrier launch. The camera rides with the plane, so the carrier moves:
///  - engine start: the plane sits spotted aft; the engine coughs and
///    catches (the engine bank runs from the sputter loop up), smoke from
///    the exhausts;
///  - the launch officer winds it up to full throttle;
///  - the deck run: the deck slides back under the plane, accelerating,
///    until it runs off the bow;
///  - the climb: the carrier falls away below and shrinks with height, the
///    plane's shadow falls behind it, and the player takes the stick.
/// The opening wave arrives during the climb, holding its fire.
fn update_launch(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.launch_t += dt;
    let t = g.launch_t;
    let roll_len = LAUNCH_ROLL_END - LAUNCH_SIGNAL_END;
    let lift_v = 2.0 * LAUNCH_DECK_POS * CARRIER_DECK_LEN / roll_len; // deck px/s at the bow
    // where on the deck the plane is, 0 at the bow, negative once airborne
    let (d, climb) = if t < LAUNCH_SIGNAL_END {
        (LAUNCH_DECK_POS, 0.0)
    } else if t < LAUNCH_ROLL_END {
        let u = (t - LAUNCH_SIGNAL_END) / roll_len;
        (LAUNCH_DECK_POS * (1.0 - u * u), 0.0)
    } else {
        // with height the sea below appears to slide by more slowly
        let u = t - LAUNCH_ROLL_END;
        let c = smoothstep01(u / (LAUNCH_TIME - LAUNCH_ROLL_END));
        (-(lift_v * u * (1.0 - 0.45 * c)) / CARRIER_DECK_LEN, c)
    };
    g.launch_climb = climb;
    g.carrier_scale = 1.0 - 0.3 * climb;
    g.player_y = LAUNCH_DECK_Y + (LAUNCH_END_Y - LAUNCH_DECK_Y) * climb;
    let plane_cy = g.player_y + PLAYER_H as f32 / 2.0;
    g.ship_y = plane_cy - (CARRIER_BOW + d * CARRIER_DECK_LEN) * g.carrier_scale;

    // the sea: the ship's own way at first, the plane's once it flies
    g.scroll_k = if t < LAUNCH_SIGNAL_END { 0.35 } else if t < LAUNCH_ROLL_END { 0.35 + 0.4 * (t - LAUNCH_SIGNAL_END) / roll_len } else { 0.75 + 0.25 * climb };
    update_background(g, dt);
    update_wrecks(g, dt, sfx);

    // the engine note: coughing into life, then full throttle, then cruise
    g.airspeed = if t < 0.9 { 0.1 } else if t < LAUNCH_START_END { 0.1 + 0.4 * (t - 0.9) / (LAUNCH_START_END - 0.9) }
        else if t < LAUNCH_SIGNAL_END { 0.5 + 0.9 * (t - LAUNCH_START_END) / (LAUNCH_SIGNAL_END - LAUNCH_START_END) }
        else if t < LAUNCH_ROLL_END { AIRSPEED_MAX } else { AIRSPEED_MAX - (AIRSPEED_MAX - 1.0) * climb };
    // start-up smoke from the exhausts
    if t > 0.3 && t < LAUNCH_START_END + 0.3 {
        g.smoke_t -= dt;
        if g.smoke_t <= 0.0 {
            g.smoke_t = 0.07;
            // the exhaust stacks either side of the cowling, at deck scale (0.62)
            let (cx, ny) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 * (0.5 - 0.3 * 0.62));
            for side in [-1.0f32, 1.0] {
                pool_spawn(&mut g.puffs, Puff { x: cx + side * 4.5, y: ny, r: 1.3, grow: 8.0,
                    ttl: 0.8, max_ttl: 0.8, fire: false, top: false, active: true });
            }
        }
    }

    // on the deck the plane runs the centreline; airborne it is yours
    if t < LAUNCH_ROLL_END + 0.4 {
        g.player_x = ((WIN_W - PLAYER_W) / 2) as f32;
    } else {
        let left  = key_held(BLIP_KEY_LEFT)  || key_held(BLIP_KEY_A);
        let right = key_held(BLIP_KEY_RIGHT) || key_held(BLIP_KEY_D);
        if left  { g.player_x -= PLAYER_SPEED * dt; }
        if right { g.player_x += PLAYER_SPEED * dt; }
        g.player_x = clamp(g.player_x, 0.0, (WIN_W - PLAYER_W) as f32);
        let target_bank = if left && !right { -0.30 } else if right && !left { 0.30 } else { 0.0 };
        g.player_bank += (target_bank - g.player_bank) * (dt * 9.0).min(1.0);
    }

    if !g.launch_wave_up && t >= LAUNCH_WAVE_AT {
        spawn_opening_wave(g);
        g.launch_wave_up = true;
    }
    if g.launch_wave_up {
        update_enemies(g, dt, false, sfx);
    }

    if t >= LAUNCH_TIME {
        g.airspeed = 1.0;
        g.scroll_k = 1.0;
        g.launch_climb = 1.0;
        g.respawn_grace.start(0.4);
        g.spawn_timer.start(1.5);
        g.state = State::Play;
        // Playtest, native only: RAIDER_BOSS=1..7 goes straight to that boss,
        // RAIDER_HP=1..5 starts damaged, RAIDER_BARRIER=1 brings a barrier in.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<i32>().ok());
            if let Some(n) = env("RAIDER_BOSS") {
                g.sess.level = n.clamp(1, BOSS_SPECS.len() as i32);
                g.wave_kills = g.wave_target;
            }
            if let Some(hp) = env("RAIDER_HP") { g.health = hp.clamp(1, PLAYER_HEALTH_MAX); }
            if env("RAIDER_BARRIER").is_some() {
                g.sess.level = g.sess.level.max(BARRIER_MIN_LEVEL);
                g.barrier_timer.start(1.0);
            }
        }
    }
}

/// Hermite smoothstep clamped to 0..1.
fn smoothstep01(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.respawn_grace.tick(dt);
    g.boss_intro.tick(dt);
    g.max_power_banner.tick(dt);

    // ---- movement ----
    if g.stall_fall > 0.0 {
        update_stall(g, dt, sfx);
    } else {
        let left  = key_active(BLIP_KEY_LEFT)  || key_active(BLIP_KEY_A);
        let right = key_active(BLIP_KEY_RIGHT) || key_active(BLIP_KEY_D);
        let up    = key_active(BLIP_KEY_UP)    || key_active(BLIP_KEY_W);
        let down  = key_active(BLIP_KEY_DOWN)  || key_active(BLIP_KEY_S);
        let throttle = up as i32 as f32 - down as i32 as f32;
        if throttle != 0.0 {
            g.airspeed += throttle * AIRSPEED_RATE * dt;
        } else {
            g.airspeed += (1.0 - g.airspeed).clamp(-AIRSPEED_RELAX * dt, AIRSPEED_RELAX * dt);
        }
        g.airspeed = g.airspeed.clamp(0.0, AIRSPEED_MAX);
        // Sideways: spool up and brake over ~0.1s so the plane has weight.
        let dx = right as i32 as f32 - left as i32 as f32;
        let target = dx * PLAYER_SPEED;
        let rate = if dx == 0.0 || target * g.player_vx < 0.0 { PLAYER_BRAKE } else { PLAYER_ACCEL } * dt;
        g.player_vx += clamp(target - g.player_vx, -rate, rate);
        // Up and down the screen is only gaining on or falling behind the
        // scrolling world.
        let vy_target = (1.0 - g.airspeed) * AIRSPEED_DRIFT;
        g.player_vy += (vy_target - g.player_vy) * (6.0 * dt).min(1.0);
        g.player_x += g.player_vx * dt;
        g.player_y += g.player_vy * dt;
        let (max_x, max_y) = ((WIN_W - PLAYER_W) as f32, PLAYER_MAX_Y);
        if g.player_x < 0.0 || g.player_x > max_x { g.player_vx = 0.0; }
        g.player_x = clamp(g.player_x, 0.0, max_x);
        g.player_y = clamp(g.player_y, PLAYER_MIN_Y, max_y);

        // Bank follows lateral velocity.
        let target_bank = g.player_vx / PLAYER_SPEED * 0.30;
        g.player_bank += (target_bank - g.player_bank) * (dt * 9.0).min(1.0);

        if g.airspeed <= STALL_SPEED { g.stall_t += dt; } else { g.stall_t = 0.0; }
        if g.stall_t >= STALL_HOLD {
            g.stall_fall = 1e-4;
            g.stall_spin = if rand01() < 0.5 { -1.0 } else { 1.0 };
        }
    }
    g.scroll_k = if g.airspeed < 1.0 { 0.25 + 0.75 * g.airspeed } else { 1.0 + 0.5 * (g.airspeed - 1.0) };

    // ---- firing ----
    g.fire_cd.tick(dt);
    if g.stall_fall == 0.0 && key_active(BLIP_KEY_SPACE) && !g.fire_cd.active() && !g.respawn_grace.active() {
        g.fire_cd.start(weapon_cooldown(g.weapon_level));
        let (n, fan, r) = weapon_guns(g.weapon_level);
        let cx = g.player_x + PLAYER_W as f32 / 2.0;
        for i in 0..n {
            let dx = if n > 1 { (i as f32 / (n - 1) as f32 - 0.5) * fan } else { 0.0 };
            // Guns are not lasers: each round leaves a little off true.
            pool_spawn(&mut g.bullets, Round {
                x: cx + dx + (rand01() - 0.5) * 3.0,
                y: g.player_y + rand01() * 6.0,
                vx: dx * 2.2 + (rand01() - 0.5) * 28.0 + g.player_vx * 0.15,
                vy: BULLET_SPEED,
                r,
                bounced: false,
                active: true,
            });
        }
        g.muzzle_t = 0.05;
        g.eject_left = !g.eject_left;
        let side = if g.eject_left { -1.0 } else { 1.0 };
        pool_spawn(&mut g.casings, Casing {
            x: cx + side * 6.0, y: g.player_y + PLAYER_H as f32 * 0.45,
            vx: side * (70.0 + rand01() * 50.0) + g.player_vx * 0.5,
            vy: 20.0 + rand01() * 40.0,
            rot: rand01() * 3.0, ttl: 0.45, active: true,
        });
        let takes = &sfx.shoot[(g.weapon_level.clamp(1, MAX_WEAPON_LEVEL) - 1) as usize];
        play_sfx_volume(&takes[(rand() % takes.len() as u32) as usize], 0.8 + 0.04 * g.weapon_level as f32);
    }
    g.muzzle_t = (g.muzzle_t - dt).max(0.0);
    for c in pool_iter_mut(&mut g.casings) {
        c.x += c.vx * dt;
        c.y += c.vy * dt;
        c.vy += SEA_SCROLL_SPEED * 3.0 * dt;
        c.vx *= 1.0 - 2.5 * dt;
        c.rot += 14.0 * dt;
        c.ttl -= dt;
        if c.ttl <= 0.0 { c.active = false; }
    }

    // ---- simple movement (no cross-entity reads) ----
    for b in pool_iter_mut(&mut g.bullets) {
        b.y -= b.vy * dt;
        b.x += b.vx * dt;
        if b.y < -BULLET_H || b.y > WIN_H as f32 || b.x < -10.0 || b.x > WIN_W as f32 + 10.0 { b.active = false; }
    }
    for b in pool_iter_mut(&mut g.enemy_bullets) {
        b.y += b.vy * dt;
        b.x += b.vx * dt;
        if b.y > WIN_H as f32 + 10.0 || b.y < -20.0 || b.x < -10.0 || b.x > WIN_W as f32 + 10.0 { b.active = false; }
    }

    // Damaged: smoke from the engine at 3 health, fire with it at 2, more
    // of both at 1.
    if g.health <= 3 && g.stall_fall == 0.0 {
        g.smoke_t -= dt;
        if g.smoke_t <= 0.0 {
            let hurt = (PLAYER_HEALTH_MAX - g.health) as f32; // 2..4
            g.smoke_t = 0.13 - 0.025 * hurt;
            let (cx, ny) = (g.player_x + PLAYER_W as f32 / 2.0 + (rand01() - 0.5) * 6.0, g.player_y + PLAYER_H as f32 * 0.3);
            let dark = if g.health <= 2 { 1.0 } else { 0.6 };
            pool_spawn(&mut g.puffs, Puff { x: cx, y: ny, r: 2.5 * dark + 1.0, grow: 12.0 + 8.0 * dark,
                ttl: 0.9, max_ttl: 0.9, fire: false, top: true, active: true });
            if g.health <= 2 {
                // flames licking back from both sides of the cowling
                for side in [-1.0f32, 1.0] {
                    let fx = g.player_x + PLAYER_W as f32 / 2.0 + side * (3.0 + rand01() * 3.0);
                    pool_spawn(&mut g.puffs, Puff { x: fx, y: g.player_y + PLAYER_H as f32 * 0.2, r: 2.5 + hurt * 0.9 + rand01() * 2.0,
                        grow: -9.0, ttl: 0.28, max_ttl: 0.28, fire: true, top: true, active: true });
                }
            }
        }
    }
    // Near a stall the engine starts to backfire.
    if g.airspeed < STALL_WARN && g.stall_fall == 0.0 {
        g.backfire_t -= dt;
        if g.backfire_t <= 0.0 {
            g.backfire_t = 0.18 + rand01() * 0.35;
            play_sfx_volume(&sfx.backfire, 0.75);
            let (cx, ny) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 * 0.2);
            pool_spawn(&mut g.puffs, Puff { x: cx, y: ny, r: 3.0, grow: -8.0, ttl: 0.12, max_ttl: 0.12, fire: true, top: true, active: true });
            pool_spawn(&mut g.puffs, Puff { x: cx, y: ny, r: 2.5, grow: 14.0, ttl: 0.7, max_ttl: 0.7, fire: false, top: true, active: true });
        }
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
    update_wrecks(g, dt, sfx);
    update_background(g, dt);

    update_enemies(g, dt, true, sfx);
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
        let bw = g.bullets[bi].r * 2.0 + 2.0;
        let (bx, by) = (g.bullets[bi].x - bw / 2.0, g.bullets[bi].y);
        let mut consumed = false;
        for ei in 0..MAX_ENEMIES {
            if !g.enemies[ei].active { continue; }
            let (ex, ey) = (g.enemies[ei].x, g.enemies[ei].y);
            if rects_overlap(bx, by, bw, BULLET_H, ex, ey, ENEMY_W as f32, ENEMY_H as f32) {
                if !g.bullets[bi].bounced && rand01() < RICOCHET_CHANCE {
                    // Glances off: a spark, and away at 35-80 degrees to
                    // one side, a little slower, the plane unharmed.
                    let b = &mut g.bullets[bi];
                    let side = if rand01() < 0.5 { -1.0 } else { 1.0 };
                    let a = 0.6 + rand01() * 0.8;
                    let v = BULLET_SPEED * 0.8;
                    b.vx = side * v * a.sin();
                    b.vy = v * a.cos();
                    b.bounced = true;
                    let (sx, sy) = (b.x, b.y);
                    g.spawn_explosion(sx, sy, 0.18, BlipColor::new(1.0, 0.95, 0.7, 1.0));
                    let take = &sfx.ricochet[(rand() % sfx.ricochet.len() as u32) as usize];
                    play_sfx_volume(take, 0.7);
                    break;
                }
                shoot_down(g, ei, sfx, rand01() < FLAMES_CHANCE);
                consumed = true;
                break;
            }
        }
        if consumed {
            g.bullets[bi].active = false;
            continue;
        }
        let (bossw, bossh) = boss_size(g.boss.tier);
        if g.boss.active && rects_overlap(bx, by, bw, BULLET_H, g.boss.x, g.boss.y, bossw, bossh) {
            g.bullets[bi].active = false;
            g.boss.hp -= 1;
            g.spawn_explosion(bx, by, 0.6, EXPLOSION_ORANGE);
            // a round on a turret damages the turret too; enough and it is out
            let r = turret_radius(g.boss.tier) + 3.0;
            for k in 0..g.boss.n_turrets {
                if g.boss.turrets[k].hp <= 0 { continue; }
                let (tx, ty) = turret_pos(g, k);
                if (tx - bx).hypot(ty - by) < r {
                    g.boss.turrets[k].hp -= 1;
                    if g.boss.turrets[k].hp <= 0 {
                        g.spawn_explosion(tx, ty, 1.0, EXPLOSION_ORANGE);
                        g.sess.add_score(50 * g.sess.level);
                        play_sfx_volume(&sfx.enemy_explode, 0.7);
                    }
                    break;
                }
            }
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
                if rects_overlap(bx, by, bw, BULLET_H, boat_x, boat_y, BOAT_W as f32, BOAT_H as f32) {
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
                if rects_overlap(bx, by, bw, BULLET_H, ix, iy, iw as f32, ih as f32) {
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
            let (mx, my) = (g.barrier.motor_x - MOTOR_W / 2.0, g.barrier.y - MOTOR_H / 2.0);
            if rects_overlap(bx, by, bw, BULLET_H, mx, my, MOTOR_W, MOTOR_H) {
                g.bullets[bi].active = false;
                g.barrier.hp -= 1;
                g.spawn_explosion(bx, by, 0.6, EXPLOSION_ORANGE);
                if g.barrier.hp <= 0 {
                    g.barrier.active = false;
                    stop_sound(&sfx.barrier_hum);
                    stop_sound(&sfx.barrier_hum2);
                    g.spawn_explosion(g.barrier.motor_x, g.barrier.y, 2.0, EXPLOSION_ORANGE);
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
    // The aircraft, not its box: the middle 70% (fuselage and inner wings).
    if g.state == State::Play && !g.respawn_grace.active() && g.stall_fall == 0.0 {
        let (px, py) = (g.player_x, g.player_y);
        let (hx, hy, hw, hh) = (px + PLAYER_W as f32 * 0.15, py + PLAYER_H as f32 * 0.12, PLAYER_W as f32 * 0.7, PLAYER_H as f32 * 0.7);
        let mut hit = false;
        for i in 0..MAX_ENEMY_BULLETS {
            if !g.enemy_bullets[i].active { continue; }
            if rects_overlap(hx, hy, hw, hh, g.enemy_bullets[i].x - 2.0, g.enemy_bullets[i].y - 2.0, 4.0, 4.0) {
                g.enemy_bullets[i].active = false;
                hit = true;
            }
        }
        for i in 0..MAX_TURRET_BULLETS {
            if !g.turret_bullets[i].active { continue; }
            if rects_overlap(hx, hy, hw, hh, g.turret_bullets[i].x, g.turret_bullets[i].y, TURRET_BULLET_W, TURRET_BULLET_H) {
                g.turret_bullets[i].active = false;
                hit = true;
            }
        }
        for i in 0..MAX_ENEMIES {
            if !g.enemies[i].active { continue; }
            let (ew, eh) = (ENEMY_W as f32 * 0.6, ENEMY_H as f32 * 0.6);
            if rects_overlap(hx, hy, hw, hh, g.enemies[i].x + ENEMY_W as f32 * 0.2, g.enemies[i].y + ENEMY_H as f32 * 0.2, ew, eh) {
                g.spawn_explosion(g.enemies[i].x, g.enemies[i].y, 1.0, EXPLOSION_ORANGE);
                g.enemies[i].active = false;
                hit = true;
            }
        }
        let (bossw, bossh) = boss_size(g.boss.tier);
        if g.boss.active && rects_overlap(hx, hy, hw, hh, g.boss.x + bossw * 0.2, g.boss.y + bossh * 0.15, bossw * 0.6, bossh * 0.7) {
            hit = true;
        }
        // The beam: full width, so only shooting the motor stops it.
        if g.barrier.active && !g.barrier.warmup.active()
            && rects_overlap(hx, hy, hw, hh,
                0.0, g.barrier.y - BARRIER_BEAM_H / 2.0, WIN_W as f32, BARRIER_BEAM_H)
        {
            hit = true;
        }
        if hit {
            g.health -= 1;
            // A non-fatal clip knocks the gun down one tier — never below 2,
            // so a good run isn't gutted by one mistake and the lost tier is
            // easy to catch straight back. A fatal hit leaves the weapon
            // alone (losing the life is punishment enough).
            if g.health > 0 && g.weapon_level > 2 {
                g.weapon_level -= 1;
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

/// An enemy is down: points, the kill count, its drop, and either a fireball
/// or (`flames`) a wreck that burns all the way to the sea.
fn shoot_down(g: &mut Game, ei: usize, sfx: &Sounds, flames: bool) {
    let e = g.enemies[ei];
    g.enemies[ei].active = false;
    let (ecx, ecy) = (e.x + ENEMY_W as f32 / 2.0, e.y + ENEMY_H as f32 / 2.0);
    if flames && pool_iter(&g.wrecks).count() < MAX_WRECKS {
        g.spawn_explosion(ecx, ecy, 0.45, EXPLOSION_ORANGE);
        // A random fall along its own course: anything from a lazy burning
        // glide to a tight spinning roll, bending up to ~25 degrees off.
        let spin = (if rand01() < 0.5 { -1.0 } else { 1.0 }) * (0.3 + rand01() * 4.2);
        pool_spawn(&mut g.wrecks, Wreck { x: e.x, y: e.y, heading: e.heading, spin,
            speed: 85.0 + rand01() * 45.0, t: 0.0, kind: e.kind, puff_t: 0.0, active: true,
            course: e.heading + (rand01() - 0.5) * 0.3, bend: (rand01() - 0.5) * 0.3,
            wobble: rand01() * 3.0, dur: WRECK_SECS * (0.75 + rand01() * 0.55) });
        play_sfx_volume(&sfx.enemy_explode, 0.55);
    } else {
        g.spawn_explosion(ecx, ecy, 1.0, EXPLOSION_ORANGE);
        play_sfx(&sfx.enemy_explode);
    }
    let pts = match e.kind { EnemyKind::Grunt => 20, EnemyKind::Weaver => 30, EnemyKind::Ace => 50 };
    g.sess.add_score(pts * g.sess.level);
    g.wave_kills += 1;
    if e.kind == EnemyKind::Ace {
        pool_spawn(&mut g.powerups, Powerup { x: e.x, y: e.y, active: true });
    } else if rand01() < HEALTH_DROP_CHANCE {
        pool_spawn(&mut g.health_pickups, HealthPickup { x: e.x, y: e.y, active: true });
    }
}

fn update_wrecks(g: &mut Game, dt: f32, sfx: &Sounds) {
    for i in 0..MAX_WRECKS {
        if !g.wrecks[i].active { continue; }
        let w = &mut g.wrecks[i];
        w.t += dt;
        w.spin *= 1.0 + 0.8 * dt; // the roll tightens as it falls
        w.heading += (w.spin + (w.t * 5.0).sin() * w.wobble) * dt;
        w.course += w.bend * dt;
        w.speed *= 1.0 - 0.25 * dt; // it keeps its momentum
        w.x += w.course.sin() * w.speed * dt;
        w.y += w.course.cos() * w.speed * dt + SEA_SCROLL_SPEED * g.scroll_k * 0.5 * dt;
        w.puff_t -= dt;
        let (cx, cy, k) = (w.x + ENEMY_W as f32 / 2.0, w.y + ENEMY_H as f32 / 2.0, w.t / w.dur);
        let (emit, done) = (w.puff_t <= 0.0, w.t >= w.dur);
        if emit { w.puff_t = 0.035; }
        if done { w.active = false; }
        let s = 1.0 - 0.55 * k;
        if emit && !done {
            let j = || (rand01() - 0.5) * 4.0;
            pool_spawn(&mut g.puffs, Puff { x: cx + j(), y: cy + j(), r: 2.5 * s, grow: 16.0 * s,
                ttl: 0.9, max_ttl: 0.9, fire: false, top: false, active: true });
            pool_spawn(&mut g.puffs, Puff { x: cx + j(), y: cy + j(), r: 3.4 * s, grow: -5.0,
                ttl: 0.22, max_ttl: 0.22, fire: true, top: false, active: true });
        }
        if done {
            // the splash where it goes in
            g.spawn_explosion(cx, cy, 0.8, BlipColor::new(0.85, 0.93, 1.0, 1.0));
        }
    }
    // A burning plane still at altitude brings down any plane it crosses,
    // and that one burns too. Low in its fall it is under them.
    for wi in 0..MAX_WRECKS {
        let w = g.wrecks[wi];
        if !w.active || w.t / w.dur > 0.6 { continue; }
        let s = 1.0 - 0.55 * w.t / w.dur;
        let (ww, wh) = (ENEMY_W as f32 * s * 0.7, ENEMY_H as f32 * s * 0.7);
        let (wx, wy) = (w.x + (ENEMY_W as f32 - ww) / 2.0, w.y + (ENEMY_H as f32 - wh) / 2.0);
        for ei in 0..MAX_ENEMIES {
            let e = g.enemies[ei];
            if e.active && rects_overlap(wx, wy, ww, wh, e.x + 4.0, e.y + 4.0,
                ENEMY_W as f32 - 8.0, ENEMY_H as f32 - 8.0) {
                shoot_down(g, ei, sfx, true);
            }
        }
    }
    let sc = SEA_SCROLL_SPEED * g.scroll_k;
    for p in pool_iter_mut(&mut g.puffs) {
        p.y += sc * dt;
        p.r = (p.r + p.grow * dt).max(0.5);
        p.ttl -= dt;
        if p.ttl <= 0.0 { p.active = false; }
    }
}

/// Stalled: the plane falls off into a spin, trailing smoke, and goes into
/// the sea. It costs a life like being shot down.
fn update_stall(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.stall_fall += dt;
    g.player_bank += g.stall_spin * (2.0 + g.stall_fall * 5.0) * dt;
    g.player_y += 30.0 * dt;
    let k = g.stall_fall / STALL_FALL;
    let (cx, cy) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0);
    g.stall_t -= dt;
    if g.stall_t <= 0.0 {
        g.stall_t = 0.05;
        pool_spawn(&mut g.puffs, Puff { x: cx, y: cy, r: 3.0 * (1.0 - 0.5 * k), grow: 14.0,
            ttl: 0.9, max_ttl: 0.9, fire: false, top: false, active: true });
    }
    if g.stall_fall >= STALL_FALL {
        g.stall_fall = 0.0;
        g.spawn_explosion(cx, cy, 1.4, BlipColor::new(0.85, 0.93, 1.0, 1.0));
        play_sfx(&sfx.player_explode);
        g.health = 0;
        match g.sess.lose_life() {
            LifeResult::StillAlive => { g.dead_timer.start(DEAD_PAUSE); g.state = State::Dead; }
            LifeResult::GameOver   => { g.over_timer.start(OVER_MIN_WAIT); g.state = State::Over; }
        }
    }
}

/// Engine loop volumes for the current airspeed: a crossfade between the
/// two loops either side of it, louder the faster it turns, the coughing
/// low loop pushed up near a stall, and the engine dying through the fall.
fn engine_mix(g: &Game) -> [f32; 5] {
    let a = g.airspeed.clamp(ENGINE_AT[0], AIRSPEED_MAX);
    let mut w = [0.0f32; 5];
    for i in 0..4 {
        if a >= ENGINE_AT[i] && a <= ENGINE_AT[i + 1] {
            let t = (a - ENGINE_AT[i]) / (ENGINE_AT[i + 1] - ENGINE_AT[i]);
            w[i] = 1.0 - t;
            w[i + 1] = t;
        }
    }
    if g.airspeed < STALL_WARN {
        let s = 1.0 - g.airspeed / STALL_WARN;
        w[0] = w[0].max(s);
    }
    let loud = PROP_MAX_VOLUME * (0.55 + 0.45 * g.airspeed / AIRSPEED_MAX);
    let dying = if g.stall_fall > 0.0 { (1.0 - g.stall_fall / STALL_FALL).max(0.0).powi(2) } else { 1.0 };
    w.map(|v| v * loud * dying)
}

fn update_dead(g: &mut Game, dt: f32) {
    if g.dead_timer.tick(dt) { g.respawn(); }
}

fn update_win(g: &mut Game, dt: f32) {
    if g.win_timer.tick(dt) { g.start_round(); }
}

fn update_won(g: &mut Game) {
    if !btn1_pressed() { return; }
    web::spend_coin();
    g.start_game();
}

fn update_over(g: &mut Game, dt: f32) {
    g.over_timer.tick(dt);
    // OVER_MIN_WAIT has to actually elapse before any key can dismiss this —
    // otherwise the fire button still held down from the fight that killed
    // you bounces straight back to the title screen.
    if g.over_timer.active() || !btn1_pressed() { return; }
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

/// A round in flight: a slug laid along its velocity, a dark jacket with a
/// lighter tip, and a faint streak of its motion. No tracers.
fn draw_slug(blip: &Blip, x: f32, y: f32, vx: f32, vy: f32, len: f32, wid: f32, body: BlipColor) {
    let v = vx.hypot(vy).max(1.0);
    let (dx, dy) = (vx / v, vy / v);
    let (tx, ty) = (x - dx * len, y - dy * len);
    blip.draw_line_ex(tx - dx * len, ty - dy * len, tx, ty, wid * 0.6, BlipColor::new(body.r, body.g, body.b, 0.18));
    blip.draw_line_ex(tx, ty, x, y, wid + 1.2, BlipColor::new(0.05, 0.05, 0.06, 0.8));
    blip.draw_line_ex(tx, ty, x, y, wid, body);
    blip.fill_circle(x, y, wid * 0.45, BlipColor::new(1.0, 0.92, 0.7, 1.0));
}

/// An aircraft sprite, nose along `heading`, drawn narrower as it banks:
/// seen from above, a banked wing is foreshortened.
/// Flight moves along (sin h, cos h); rotating the nose-down sprite by r
/// points its nose at (-sin r, cos r), hence the minus.
fn draw_plane(tex: &Texture2D, x: f32, y: f32, w: f32, h: f32, heading: f32, bank: f32, tint: BlipColor) {
    let ww = w * (0.55 + 0.45 * bank.cos());
    draw_texture_ex(tex, x + (w - ww) / 2.0, y, tint, DrawTextureParams {
        dest_size: Some(vec2(ww, h)),
        rotation: -heading,
        ..Default::default()
    });
}

fn draw_launch(
    blip: &Blip, g: &Game,
    player_tex: &Texture2D, enemy_tex: &[Texture2D; 3], carrier_tex: &Texture2D, boat_tex: &Texture2D,
    cloud_tex: &[Texture2D; 3], island_tex: &[Texture2D; 3],
) {
    let _ = island_tex;
    draw_sea(blip, g);
    draw_boats(blip, g, boat_tex);
    let t = g.launch_t;
    let cs = g.carrier_scale;
    let (cw, ch) = (CARRIER_W as f32 * cs, CARRIER_H as f32 * cs);
    let cx0 = WIN_W as f32 / 2.0 - CARRIER_DECK_X * cs;
    let top = g.ship_y;
    // wake: the bow wave peeling off both sides, a churned band astern
    let bow = (WIN_W as f32 / 2.0, top + 2.0 * cs);
    for side in [-1.0f32, 1.0] {
        for k in 0..14 {
            let f = k as f32 / 13.0;
            let (x, y) = (bow.0 + side * (8.0 + f * 70.0) * cs, bow.1 + f * 150.0 * cs);
            let r = (2.0 + f * 6.0) * cs;
            blip.fill_circle(x, y, r, BlipColor::new(0.9, 0.95, 1.0, 0.35 * (1.0 - f)));
        }
    }
    let stern = top + ch - 10.0 * cs;
    for k in 0..26 {
        let f = k as f32 / 25.0;
        let jitter = ((k as f32 * 12.9898 + g.sea_scroll * 0.07).sin() * 43758.5).fract() - 0.5;
        let (x, y) = (WIN_W as f32 / 2.0 + jitter * (40.0 + f * 90.0) * cs, stern + f * 220.0 * cs);
        blip.fill_circle(x, y, (7.0 + f * 12.0) * cs, BlipColor::new(0.88, 0.94, 1.0, 0.3 * (1.0 - f)));
    }
    blip.draw_texture(carrier_tex, cx0, top, cw, ch);

    // deck crew: the launch officer beside the plane, handlers aft
    let deck = |dx: f32, dy: f32| (cx0 + (CARRIER_DECK_X + dx) * cs, top + dy * cs);
    let crew = |x: f32, y: f32, shirt: BlipColor, low: bool| {
        let s = 1.5 * cs * if low { 0.8 } else { 1.0 };
        blip.fill_circle(x + 1.5 * s, y + 2.0 * s, 3.2 * s, BlipColor::new(0.0, 0.0, 0.0, 0.3));
        blip.fill_circle(x, y, 3.0 * s, shirt);
        blip.fill_circle(x, y - 0.6 * s, 1.6 * s, BlipColor::new(0.86, 0.68, 0.52, 1.0));
    };
    let spot_y = CARRIER_BOW + LAUNCH_DECK_POS * CARRIER_DECK_LEN;
    let yellow = BlipColor::new(0.95, 0.8, 0.2, 1.0);
    let blue = BlipColor::new(0.2, 0.35, 0.75, 1.0);
    let red = BlipColor::new(0.8, 0.2, 0.2, 1.0);
    let (ox, oy) = deck(-44.0, spot_y - 58.0);
    crew(ox, oy, yellow, t > LAUNCH_SIGNAL_END);
    // the flag: wound round overhead to rev up, held high, then down the deck
    let (fa, flen) = if t < LAUNCH_START_END { (t * 9.0, 9.0) } else if t < LAUNCH_SIGNAL_END { (-1.57, 11.0) } else { (-0.4, 10.0) };
    let (fx, fy) = (ox + fa.cos() * flen * cs, oy + fa.sin() * flen * cs);
    blip.draw_line_ex(ox, oy, fx, fy, 1.2 * cs, BlipColor::new(0.3, 0.25, 0.2, 1.0));
    blip.fill_rect(fx - 2.5 * cs, fy - 2.0 * cs, 5.0 * cs, 4.0 * cs, BlipColor::new(0.95, 0.2, 0.2, 1.0));
    for (dx, dy, c) in [(-60.0, spot_y + 70.0, blue), (58.0, spot_y + 52.0, blue), (-30.0, spot_y + 150.0, blue),
                        (40.0, spot_y + 160.0, blue), (70.0, 230.0, red), (72.0, 380.0, red)] {
        let (x, y) = deck(dx, dy);
        crew(x, y, c, false);
    }

    // clouds only once there is height under the plane
    if g.launch_climb > 0.05 { draw_clouds(blip, g, cloud_tex); }

    // the opening wave, streaming in from the top while the player climbs
    let enemy_tex_of = |e: &Enemy| match e.kind {
        EnemyKind::Grunt => &enemy_tex[0],
        EnemyKind::Weaver => &enemy_tex[1],
        EnemyKind::Ace => &enemy_tex[2],
    };
    for e in pool_iter(&g.enemies) {
        draw_shadow(enemy_tex_of(e), e.x, e.y, ENEMY_W as f32, ENEMY_H as f32, -e.heading);
    }
    for e in pool_iter(&g.enemies) {
        draw_plane(enemy_tex_of(e), e.x, e.y, ENEMY_W as f32, ENEMY_H as f32, e.heading, e.bank, BLIP_WHITE);
    }

    // the plane: its shadow under it on the deck, falling behind as it climbs
    let climb = g.launch_climb;
    // seen from above, the plane grows as it climbs toward the camera
    let scale = 0.62 + 0.38 * climb;
    let pw = PLAYER_W as f32 * scale;
    let ph = PLAYER_H as f32 * scale;
    let px = g.player_x + (PLAYER_W as f32 - pw) / 2.0;
    let py = g.player_y + (PLAYER_H as f32 - ph) / 2.0;
    let (sdx, sdy) = (1.5 + (PLANE_SHADOW_DX - 1.5) * climb, 2.0 + (PLANE_SHADOW_DY - 2.0) * climb);
    draw_texture_ex(player_tex, px + sdx, py + sdy, BlipColor::new(0.0, 0.0, 0.0, 0.35), DrawTextureParams {
        dest_size: Some(vec2(pw, ph)), rotation: g.player_bank, ..Default::default()
    });
    draw_texture_ex(player_tex, px, py, BLIP_WHITE, DrawTextureParams {
        dest_size: Some(vec2(pw, ph)),
        rotation: g.player_bank,
        ..Default::default()
    });
    for p in pool_iter(&g.puffs) {
        let a = p.ttl / p.max_ttl;
        blip.fill_circle(p.x, p.y, p.r, BlipColor::new(0.78, 0.8, 0.84, 0.55 * a));
    }
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
        draw_shadow(tex, e.x, e.y, ENEMY_W as f32, ENEMY_H as f32, -e.heading);
    }
    if g.boss.active {
        let (bw, bh) = boss_size(g.boss.tier);
        draw_shadow(&boss_tex[g.boss.tier], g.boss.x, g.boss.y, bw, bh, 0.0);
    }
    if g.state == State::Play && !g.respawn_grace.active() && g.stall_fall == 0.0 {
        draw_shadow(player_tex, g.player_x, g.player_y, PLAYER_W as f32, PLAYER_H as f32, g.player_bank);
    }

    // Speed: streaks of wind rushing past, more and longer the faster the
    // plane flies (they ride the sea scroll, which follows the airspeed).
    {
        let k = g.scroll_k;
        let n = (6.0 + 22.0 * (k - 0.6).max(0.0)) as i32;
        let len = 14.0 + 60.0 * (k - 0.6).max(0.0);
        let a = (0.05 + 0.14 * (k - 0.8).max(0.0)).min(0.22);
        for i in 0..n {
            let seed = i as f32 * 71.3;
            let x = (seed * 17.7) % WIN_W as f32;
            let y = (seed * 31.1 + g.sea_scroll * 7.0) % (WIN_H as f32 + len) - len;
            blip.draw_line_ex(x, y, x, y + len, 1.0, BlipColor::new(0.9, 0.95, 1.0, a));
        }
    }

    // Planes going down: smoke under them, the burning plane shrinking
    // toward the sea with its shadow closing in, fire on top.
    for p in pool_iter(&g.puffs) {
        if p.fire || p.top { continue; }
        let a = p.ttl / p.max_ttl;
        blip.fill_circle(p.x, p.y, p.r, BlipColor::new(0.12, 0.11, 0.10, 0.5 * a));
    }
    for w in pool_iter(&g.wrecks) {
        let k = w.t / w.dur;
        let s = 1.0 - 0.55 * k;
        let (ww, wh) = (ENEMY_W as f32 * s, ENEMY_H as f32 * s);
        let (x, y) = (w.x + (ENEMY_W as f32 - ww) / 2.0, w.y + (ENEMY_H as f32 - wh) / 2.0);
        let tex = match w.kind {
            EnemyKind::Grunt  => &enemy_tex[0],
            EnemyKind::Weaver => &enemy_tex[1],
            EnemyKind::Ace    => &enemy_tex[2],
        };
        draw_texture_ex(tex, x + PLANE_SHADOW_DX * (1.0 - k), y + PLANE_SHADOW_DY * (1.0 - k),
            BlipColor::new(0.0, 0.0, 0.0, 0.3), DrawTextureParams {
                dest_size: Some(vec2(ww, wh)), rotation: -w.heading, ..Default::default()
            });
        let burn = 1.0 - 0.6 * k;
        draw_texture_ex(tex, x, y, BlipColor::new(burn, burn * 0.85, burn * 0.75, 1.0), DrawTextureParams {
            dest_size: Some(vec2(ww, wh)), rotation: -w.heading, ..Default::default()
        });
    }
    for p in pool_iter(&g.puffs) {
        if !p.fire || p.top { continue; }
        let a = p.ttl / p.max_ttl;
        blip.fill_circle(p.x, p.y, p.r, BlipColor::new(1.0, 0.45 + 0.4 * a, 0.1, 0.9 * a));
        blip.fill_circle(p.x, p.y, p.r * 0.5, BlipColor::new(1.0, 0.95, 0.6, a));
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
        draw_plane(tex, e.x, e.y, ENEMY_W as f32, ENEMY_H as f32, e.heading, e.bank, BLIP_WHITE);
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
        draw_texture_ex(&boss_tex[g.boss.tier], g.boss.x, g.boss.y, tint, DrawTextureParams {
            dest_size: Some(vec2(bw, bh)),
            rotation: boss_bank(g),
            ..Default::default()
        });
        // Turrets: a steel ring, a glazed dome, twin barrels on the aim.
        let r = turret_radius(g.boss.tier);
        for k in 0..g.boss.n_turrets {
            let t = &g.boss.turrets[k];
            let (x, y) = turret_pos(g, k);
            blip.fill_circle(x + 1.0, y + 1.5, r + 0.8, BlipColor::new(0.0, 0.0, 0.0, 0.3));
            blip.fill_circle(x, y, r + 0.8, BlipColor::new(0.18, 0.2, 0.18, 1.0));
            if t.hp <= 0 {
                blip.fill_circle(x, y, r * 0.8, BlipColor::new(0.08, 0.07, 0.06, 1.0));
                continue;
            }
            let (s, c) = t.angle.sin_cos();
            let (ox, oy) = (c * 1.8, -s * 1.8);
            let len = r * 2.0;
            for side in [-1.0f32, 1.0] {
                let (bx, by) = (x + ox * side, y + oy * side);
                blip.draw_line_ex(bx, by, bx + s * len, by + c * len, 1.4, BlipColor::new(0.1, 0.1, 0.11, 1.0));
            }
            blip.fill_circle(x, y, r * 0.78, BlipColor::new(0.36, 0.42, 0.34, 1.0));
            blip.fill_circle(x - s * 1.5, y - c * 1.5, r * 0.5, BlipColor::new(0.55, 0.68, 0.72, 0.9));
            blip.fill_circle(x - r * 0.25, y - r * 0.3, r * 0.18, BlipColor::new(0.95, 1.0, 1.0, 0.8));
        }
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
        let by = g.barrier.y;
        blip.fill_rect(0.0, by - BARRIER_BEAM_H / 2.0, WIN_W as f32, BARRIER_BEAM_H, beam_color);
        if !warming {
            blip.fill_rect(0.0, by - 1.5, WIN_W as f32, 3.0, BLIP_WHITE);
        } else {
            let color = if flicker { BLIP_RED } else { BLIP_WHITE };
            blip.draw_centered("LASER BARRIER", by + 22.0, 2.4, color);
        }

        // The motor: a dark housing with a glowing core and its own health
        // meter above it — the only part of the barrier that's shootable.
        let (mx, my) = (g.barrier.motor_x - MOTOR_W / 2.0, g.barrier.y - MOTOR_H / 2.0);
        blip.fill_rect(mx, my, MOTOR_W, MOTOR_H, BLIP_GRAY);
        blip.draw_rect(mx, my, MOTOR_W, MOTOR_H, BLIP_BLACK);
        blip.fill_glow_circle(g.barrier.motor_x, by, 7.0, BLIP_RED);
        let hp_frac = (g.barrier.hp as f32 / g.barrier.max_hp as f32).clamp(0.0, 1.0);
        blip.draw_rect(mx, my - 8.0, MOTOR_W, 4.0, BLIP_GRAY);
        blip.fill_rect(mx, my - 8.0, MOTOR_W * hp_frac, 4.0, BLIP_RED);
    }

    // Everyone's rounds are slugs: steel for the fighters' and gunners',
    // heavier flak shells from the islands, brass from the player's guns.
    for b in pool_iter(&g.enemy_bullets) {
        draw_slug(blip, b.x, b.y, b.vx, b.vy, 6.0, 2.2, BlipColor::new(0.78, 0.74, 0.66, 1.0));
    }
    for b in pool_iter(&g.turret_bullets) {
        draw_slug(blip, b.x + TURRET_BULLET_W / 2.0, b.y + TURRET_BULLET_H / 2.0, b.vx, b.vy, 8.0, 3.2,
            BlipColor::new(0.62, 0.58, 0.5, 1.0));
    }
    let brass = BlipColor::new(0.82, 0.64, 0.28, 1.0);
    let copper = BlipColor::new(0.72, 0.38, 0.20, 1.0);
    let glint = BlipColor::new(1.0, 0.93, 0.72, 0.9);
    for b in pool_iter(&g.bullets) {
        let v = b.vx.hypot(b.vy).max(1.0);
        let (dx, dy) = (b.vx / v, -b.vy / v); // unit, nose-ward
        let (px, py) = (-dy, dx);
        let (len, wid) = (b.r * 3.4, b.r * 1.15);
        let (nx, ny) = (b.x, b.y);
        let (tx, ty) = (nx - dx * len, ny - dy * len);
        let (sx, sy) = (nx - dx * len * 0.35, ny - dy * len * 0.35); // where the ogive starts
        blip.draw_line_ex(tx, ty, sx, sy, wid, brass);
        draw_triangle(vec2(sx + px * wid / 2.0, sy + py * wid / 2.0),
            vec2(sx - px * wid / 2.0, sy - py * wid / 2.0), vec2(nx, ny), copper);
        blip.draw_line_ex(tx + px * wid * 0.22, ty + py * wid * 0.22,
            sx + px * wid * 0.22, sy + py * wid * 0.22, (wid * 0.28).max(0.6), glint);
    }
    // Spent brass, tumbling off the wings.
    for c in pool_iter(&g.casings) {
        let a = (c.ttl / 0.45).clamp(0.0, 1.0);
        let (s, co) = c.rot.sin_cos();
        blip.draw_line_ex(c.x - co * 2.2, c.y - s * 2.2, c.x + co * 2.2, c.y + s * 2.2, 1.8,
            BlipColor::new(0.86, 0.66, 0.26, a));
    }
    // Muzzle flash at each gun for a frame or three after a burst.
    if g.muzzle_t > 0.0 && g.state == State::Play {
        let (n, fan, r) = weapon_guns(g.weapon_level);
        let cx = g.player_x + PLAYER_W as f32 / 2.0;
        let a = g.muzzle_t / 0.05;
        for i in 0..n {
            let dx = if n > 1 { (i as f32 / (n - 1) as f32 - 0.5) * fan } else { 0.0 };
            blip.fill_glow_circle(cx + dx, g.player_y + 2.0, r * 1.6 * a + 1.0,
                BlipColor::new(1.0, 0.85, 0.45, a));
        }
    }

    if g.state == State::Play && g.stall_fall > 0.0 {
        // Spinning down to the sea, its shadow closing in.
        let k = g.stall_fall / STALL_FALL;
        let s = 1.0 - 0.5 * k;
        let (w, h) = (PLAYER_W as f32 * s, PLAYER_H as f32 * s);
        let (x, y) = (g.player_x + (PLAYER_W as f32 - w) / 2.0, g.player_y + (PLAYER_H as f32 - h) / 2.0);
        draw_texture_ex(player_tex, x + PLANE_SHADOW_DX * (1.0 - k), y + PLANE_SHADOW_DY * (1.0 - k),
            BlipColor::new(0.0, 0.0, 0.0, 0.3), DrawTextureParams {
                dest_size: Some(vec2(w, h)), rotation: g.player_bank, ..Default::default()
            });
        let d = 1.0 - 0.5 * k;
        draw_texture_ex(player_tex, x, y, BlipColor::new(d, d, d, 1.0), DrawTextureParams {
            dest_size: Some(vec2(w, h)), rotation: g.player_bank, ..Default::default()
        });
    } else if g.state == State::Play {
        // Blink the plane while the post-respawn grace period is active.
        let blink = g.respawn_grace.active() && ((g.respawn_grace.remaining() * 12.0) as i32 % 2 == 0);
        if g.airspeed < STALL_WARN && (blip::macroquad::time::get_time() * 5.0) as i64 % 2 == 0 {
            blip.draw_text("STALL", g.player_x + PLAYER_W as f32 / 2.0 - 15.0, g.player_y - 14.0, 2.0, BLIP_RED);
        }
        if !blink {
            // near a stall the airframe buffets
            let shake = if g.airspeed < STALL_WARN { 2.5 * (1.0 - g.airspeed / STALL_WARN) } else { 0.0 };
            let tt = blip::macroquad::time::get_time() as f32;
            let (bx, by) = ((tt * 57.0).sin() * shake, (tt * 43.0).cos() * shake);
            // at full throttle, vapour streams off the wingtips
            if g.airspeed > 1.15 {
                let a = ((g.airspeed - 1.15) / (AIRSPEED_MAX - 1.15)).min(1.0) * 0.35;
                let wy = g.player_y + PLAYER_H as f32 * 0.42;
                for side in [-1.0f32, 1.0] {
                    let wx = g.player_x + PLAYER_W as f32 / 2.0 + side * PLAYER_W as f32 * 0.47;
                    blip.draw_line_ex(wx, wy, wx, wy + 40.0, 1.4, BlipColor::new(1.0, 1.0, 1.0, a));
                }
            }
            draw_texture_ex(player_tex, g.player_x + bx, g.player_y + by, BLIP_WHITE, DrawTextureParams {
                dest_size: Some(vec2(PLAYER_W as f32, PLAYER_H as f32)),
                rotation: g.player_bank,
                ..Default::default()
            });
        }
        // the player's own smoke and flames, over the airframe
        for p in pool_iter(&g.puffs) {
            if blink || !p.top || p.fire { continue; }
            let a = p.ttl / p.max_ttl;
            blip.fill_circle(p.x, p.y, p.r, BlipColor::new(0.14, 0.13, 0.12, 0.42 * a));
        }
        for p in pool_iter(&g.puffs) {
            if blink || !p.top || !p.fire { continue; }
            let a = p.ttl / p.max_ttl;
            blip.fill_circle(p.x, p.y, p.r, BlipColor::new(1.0, 0.4 + 0.4 * a, 0.08, 0.95 * a));
            blip.fill_circle(p.x, p.y, p.r * 0.5, BlipColor::new(1.0, 0.95, 0.6, a));
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

fn draw_title(blip: &Blip, player_tex: &Texture2D, hi: &web::HighScore) {
    blip.clear(BLIP_BLACK);
    blip.draw_centered("RAIDER", (WIN_H / 4) as f32, 5.0, BLIP_BLUE);
    // RAIDER is sz=5 (35px tall) — clear its bottom by a real margin.
    blip.draw_hi(hi, (WIN_H / 4 + 43) as f32, BLIP_YELLOW);
    let px = (WIN_W as f32 - PLAYER_W as f32 * 2.0) / 2.0;
    blip.draw_texture(player_tex, px, (WIN_H / 2 - 70) as f32, PLAYER_W as f32 * 2.0, PLAYER_H as f32 * 2.0);
    blip.draw_centered("PRESS FIRE TO START", (WIN_H * 2 / 3) as f32, 3.0, BLIP_WHITE);
    blip.draw_centered("UP THROTTLE  DOWN SLOW", (WIN_H * 2 / 3 + 30) as f32, 2.0, BLIP_GRAY);
    blip.draw_centered("TOO SLOW AND YOU STALL", (WIN_H * 2 / 3 + 48) as f32, 2.0, BLIP_GRAY);
}

fn draw_win(blip: &Blip, level: i32) {
    let buf = format!("WAVE {level}");
    blip.clear(BLIP_BLACK);
    blip.draw_centered("STAGE CLEAR", (WIN_H / 3) as f32, 5.0, BLIP_GREEN);
    blip.draw_centered(&buf,          (WIN_H / 2) as f32, 3.0, BLIP_YELLOW);
}

fn draw_won(blip: &Blip, score: i32, hi: &web::HighScore) {
    let buf = format!("SCORE {score}");
    blip.clear(BLIP_BLACK);
    blip.draw_centered("YOU WON!!",           (WIN_H / 4) as f32,      6.0, BLIP_YELLOW);
    blip.draw_centered("ALL 7 WAVES CLEARED", (WIN_H / 2 - 20) as f32, 2.5, BLIP_GREEN);
    blip.draw_centered(&buf,                  (WIN_H / 2 + 14) as f32, 3.0, BLIP_WHITE);
    blip.draw_best(score, hi, (WIN_H / 2 + 40) as f32, BLIP_GREEN);
    blip.draw_centered("PRESS FIRE",          (WIN_H * 2 / 3) as f32,  3.0, BLIP_CYAN);
}

fn draw_over(blip: &Blip, score: i32, hi: &web::HighScore, waiting: bool) {
    let buf = format!("SCORE {score}");
    blip.clear(BLIP_BLACK);
    blip.draw_centered("GAME OVER", (WIN_H / 4) as f32, 5.0, BLIP_RED);
    blip.draw_centered(&buf,        (WIN_H / 2) as f32, 3.0, BLIP_WHITE);
    blip.draw_best(score, hi, (WIN_H / 2 + 28) as f32, BLIP_GREEN);
    // Only invite a key press once OVER_MIN_WAIT has actually elapsed — the
    // prompt would otherwise be a lie, since input is ignored until then.
    if !waiting {
        blip.draw_centered("PRESS FIRE", (WIN_H * 2 / 3) as f32, 3.0, BLIP_YELLOW);
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

macro_rules! burst_wav {
    ($t:literal, $k:literal) => {
        include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/burst", $t, "_", $k, ".wav"))
    };
}
const SHOOT_WAV: [[&[u8]; 2]; 5] = [
    [burst_wav!("1", "1"), burst_wav!("1", "2")],
    [burst_wav!("2", "1"), burst_wav!("2", "2")],
    [burst_wav!("3", "1"), burst_wav!("3", "2")],
    [burst_wav!("4", "1"), burst_wav!("4", "2")],
    [burst_wav!("5", "1"), burst_wav!("5", "2")],
];
const ENEMY_EXPLODE_WAV:  &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/enemy_explode.wav"));
const PLAYER_EXPLODE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/player_explode.wav"));
const PLAYER_HIT_WAV:     &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/player_hit.wav"));
const RICOCHET_WAV: [&[u8]; 2] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/ricochet1.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/ricochet2.wav")),
];
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
const BARRIER_HUM2_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/barrier_hum2.wav"));
const ENGINE_START_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/engine_start.wav"));
const ENEMY_GUN_WAV:      &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/enemy_gun.wav"));
const BACKFIRE_WAV:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/backfire.wav"));
const ENGINE_WAV: [&[u8]; 5] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/engine0.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/engine1.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/engine2.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/engine3.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/engine4.wav")),
];
const MUSIC_WAV:          &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music.wav"));
const MUSIC2_WAV:         &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music2.wav"));
// music: 40.25s (150 BPM rock, E minor, verse/chorus/solo)  music2: 22.07s (176 BPM drop-D thrash)
const MUSIC_DURATIONS: [f32; 2] = [40.25, 22.07];

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

    let mut shoot = Vec::new();
    for [a, b] in SHOOT_WAV {
        shoot.push([blip::audio::load_sound(a).await, blip::audio::load_sound(b).await]);
    }
    let sfx = Sounds {
        shoot,
        enemy_explode:  blip::audio::load_sound(ENEMY_EXPLODE_WAV).await,
        player_explode: blip::audio::load_sound(PLAYER_EXPLODE_WAV).await,
        player_hit:     blip::audio::load_sound(PLAYER_HIT_WAV).await,
        ricochet: [
            blip::audio::load_sound(RICOCHET_WAV[0]).await,
            blip::audio::load_sound(RICOCHET_WAV[1]).await,
        ],
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
        barrier_hum2:   blip::audio::load_sound(BARRIER_HUM2_WAV).await,
        engine_start:   blip::audio::load_sound(ENGINE_START_WAV).await,
        enemy_gun:      blip::audio::load_sound(ENEMY_GUN_WAV).await,
        backfire:       blip::audio::load_sound(BACKFIRE_WAV).await,
        engine: [
            blip::audio::load_sound(ENGINE_WAV[0]).await,
            blip::audio::load_sound(ENGINE_WAV[1]).await,
            blip::audio::load_sound(ENGINE_WAV[2]).await,
            blip::audio::load_sound(ENGINE_WAV[3]).await,
            blip::audio::load_sound(ENGINE_WAV[4]).await,
        ],
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
            State::Launch => update_launch(&mut g, dt, &sfx),
            State::Play   => update_play(&mut g, dt, &sfx),
            State::Dead   => update_dead(&mut g, dt),
            State::Win    => update_win(&mut g, dt),
            State::Won    => update_won(&mut g),
            State::Over   => update_over(&mut g, dt),
        }
        if prev_state != State::Win  && g.state == State::Win  { play_sfx(&sfx.stage_clear); }
        if prev_state != State::Over && g.state == State::Over { play_sfx(&sfx.game_over); }
        if (prev_state != State::Over && g.state == State::Over)
            || (prev_state != State::Won && g.state == State::Won)
        {
            web::report_score(g.sess.score);
        }
        // In flight the engine is the crossfaded bank; it starts when play
        // does and stops when it ends (death, stage clear, game over).
        let flying = |s: State| matches!(s, State::Launch | State::Play);
        if !flying(prev_state) && flying(g.state) {
            for e in sfx.engine.iter() { play_sound(e, PlaySoundParams { looped: true, volume: 0.0 }); }
        }
        if flying(prev_state) && !flying(g.state) {
            for e in sfx.engine.iter() { stop_sound(e); }
        }
        if flying(g.state) {
            for (e, v) in sfx.engine.iter().zip(engine_mix(&g)) { set_sound_volume(e, v); }
        }
        if prev_state != State::Launch && g.state == State::Launch {
            play_sfx(&sfx.engine_start);
        }

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title  => draw_title(&blip, &player_tex, &web::high_score()),
            State::Launch => draw_launch(&blip, &g, &player_tex, &enemy_tex, &carrier_tex, &boat_tex, &cloud_tex, &island_tex),
            State::Win    => draw_win(&blip, g.sess.level),
            State::Won    => draw_won(&blip, g.sess.score, &web::high_score()),
            State::Over   => draw_over(&blip, g.sess.score, &web::high_score(), g.over_timer.active()),
            State::Play | State::Dead => {
                draw_play(&blip, &g, &player_tex, &enemy_tex, &boss_tex, &powerup_tex, &health_tex, &boss_name_ja_tex, &boat_tex, &cloud_tex, &island_tex);
            }
        }

        blip.next_frame(60).await;
    }
}
