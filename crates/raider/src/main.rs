//! Raider — a 1942-style vertical dogfighting shoot-'em-up.

use blip::color::*;
use blip::input::{coin_start, key_active, play_again, Stick, BLIP_KEY_SPACE};
use blip::macroquad::audio::{play_sound, set_sound_volume, stop_sound, PlaySoundParams};
use blip::macroquad::math::vec2;
use blip::macroquad::rand::rand;
use blip::macroquad::texture::{draw_texture_ex, DrawTextureParams, Texture2D};
use blip::{
    load_png, load_png_smooth,
    clamp, Jukebox, play_sfx, play_sfx_volume, pool_iter, pool_iter_mut, pool_spawn, rand_range_f32, rects_overlap, web,
    window_conf, Blip, BlipColor, Fx, LifeResult, Session, Timer, segment_circle_overlap, segment_rect_overlap,
    BLIP_BLACK, BLIP_CYAN, BLIP_GRAY, BLIP_GREEN, BLIP_ORANGE, BLIP_RED, BLIP_WHITE,
    BLIP_YELLOW,
};

mod tunes;
// The pictures are made by build.rs; the tests check them here.
#[cfg(test)]
mod gfx;
mod bot;
mod land;
mod levels;
mod world;
mod enemies;
mod boss;
mod draw;
mod launch;
mod combat;
mod wrecks;
mod plane;
use launch::*;
use combat::*;
use wrecks::*;
use plane::*;
use world::*;
use enemies::*;
use boss::*;
use draw::*;
use levels::{
    BossPattern, BOSS_LASERS, BOSS_SIZES, BOSS_SPECS, BOSS_TURRETS, BOSS_WING_GUNS,
    BOSS_WINGS, BARRIER_MIN_LEVEL, FLIGHT_BONUS, MAX_LASERS, MAX_LEVEL, WAVE_KILL_BASE,
};

// ---- layout -------------------------------------------------------------
const WIN_W: i32 = 480;
const WIN_H: i32 = 540;
const HUD_H: i32 = 28;

// ---- player ---------------------------------------------------------------
const PLAYER_W: i32 = 66;
const PLAYER_H: i32 = 58;
// The hit box stays the size it was on the smaller 54x48 sprite, so drawing
// the plane bigger does not make it easier to hit.
const PLAYER_HIT: (f32, f32) = (38.0, 34.0);
/// The top left of the plane's hit box (`PLAYER_HIT`) for a plane drawn at (`x`, `y`): the fuselage and inner wings,
/// centred across and at 47 % of the sprite's height.
fn plane_hit_box(x: f32, y: f32) -> (f32, f32) {
    (x + (PLAYER_W as f32 - PLAYER_HIT.0) / 2.0, y + PLAYER_H as f32 * 0.47 - PLAYER_HIT.1 / 2.0)
}
const PLAYER_SPEED: f32 = 252.0; // a touch quicker — the d-pad is digital, so response has to carry it
const PLAYER_ACCEL: f32 = 2600.0;
const PLAYER_BRAKE: f32 = 3400.0;
const PLAYER_MIN_Y: f32 = (HUD_H + 150) as f32; // player stays out of the HUD band
// Kept well clear of the bottom edge so the on-screen control pad (which
// rises over the lowest ~15% of the playfield on a touch screen) can never
// hide the plane behind it.
const PLAYER_MAX_Y: f32 = (WIN_H - 98) as f32;

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
const ENEMY_W: i32 = 62;
const ENEMY_H: i32 = 52;
const MAX_ENEMIES: usize = 30;
const ENEMY_BULLET_SPEED: f32 = 205.0;
const MAX_ENEMY_BULLETS: usize = 260;

// ---- flight model ---------------------------------------------------------
// Coordinated turn: rate = g tan(bank)/v; load = 1/cos(bank).
// Stall limits bank to acos((v_stall/v)^2); drag costs speed, and roll rate limits bank changes.
const FLIGHT_G: f32 = 130.0;       // px/s^2: g at this game's scale and pace
const FLIGHT_STALL: f32 = 55.0;    // px/s, wings level
const FLIGHT_ROLL: f32 = 1.6;      // rad/s
const FLIGHT_MAX_BANK: f32 = 1.15; // ~66 degrees, a hard fighter turn
const FLIGHT_DRAG: f32 = 0.18;     // speed lost per second per unit of (n^2 - 1)
const FLIGHT_THRUST: f32 = 0.5;    // how fast the engine returns it to trim speed
const MAX_FLIGHTS: usize = 10;

// ---- boss ---------------------------------------------------------------
const BOSS_INTRO_TIME: f32 = 1.3;
const MAX_TURRETS: usize = 16;

// Each level opens with an Essex-class carrier launch. Sizes match gfx.rs.
const CARRIER_W: i32 = 186;
const CARRIER_H: i32 = 760;
const CARRIER_DECK_X: f32 = 88.0;  // the deck centreline in the sprite
const CARRIER_BOW: f32 = 42.0;     // where the flight deck starts in the sprite
const CARRIER_DECK_LEN: f32 = 700.0;
const LAUNCH_TIME: f32 = 7.6;
/// How much faster the take-off plays while fire is held: 7.6 s becomes 1.9 s.
const LAUNCH_HURRY: f32 = 4.0;
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
// A 5-point bar per life. A rare pickup (from regular fighters; the ace drops
// a weapon tier instead) restores some of it.
const PLAYER_HEALTH_MAX: i32 = 5;
const HEALTH_RESTORE: i32 = 2;
const HEALTH_DROP_CHANCE: f32 = 0.06;
const HEALTH_W: f32 = 14.0;
const HEALTH_H: f32 = 14.0;
const MAX_HEALTH_PICKUPS: usize = 1;
// A flinch of invulnerability after a non-lethal hit (the respawn-grace timer
// and its blink), so one burst cannot drain the whole bar in a frame.
const HIT_GRACE: f32 = 1.15;

// ---- background: sea, sky and boats --------------------------------------
// Sea and boats share a scroll speed; clouds move between the sea and planes.
const SEA_SCROLL_SPEED: f32 = 70.0;

// ---- airspeed ---------------------------------------------------------
// 1.0 is cruise; throttle adjusts airspeed, and below cruise the plane drifts down-screen.
const AIRSPEED_MAX: f32 = 1.4;
const AIRSPEED_RATE: f32 = 0.9;    // per second of throttle held
const AIRSPEED_RELAX: f32 = 0.3;   // back toward cruise with no input
const AIRSPEED_DRIFT: f32 = 150.0; // screen px/s per unit of airspeed off cruise
const STALL_WARN: f32 = 0.35;      // the engine labours and STALL flashes below this
const STALL_SPEED: f32 = 0.06;
const STALL_HOLD: f32 = 0.5;       // seconds at stall speed before it drops
const STALL_FALL: f32 = 1.5;       // spinning down to the sea
// Bad fuel: a splutter every SPLUTTER_EVERY seconds (the first after
// SPLUTTER_FIRST), losing SPLUTTER_DRAG airspeed a second while the engine
// is out, never below SPLUTTER_FLOOR (well clear of STALL_WARN).
use crate::tunes::{SPLUTTER_CATCHES, SPLUTTER_DIES, SPLUTTER_SECS};
const SPLUTTER_FIRST: f32 = 25.0;
const SPLUTTER_EVERY: (f32, f32) = (22.0, 45.0);
const SPLUTTER_DRAG: f32 = 0.45;
const SPLUTTER_FLOOR: f32 = 0.5;
// The engine loops, slowest (coughing, the stall warning) to full throttle,
// and the airspeed each one belongs to.
const ENGINE_AT: [f32; 5] = [0.1, 0.45, 1.0, 1.2, 1.4];
const MAX_CLOUDS: usize = 8;
const BOAT_W: i32 = 34;
const BOAT_H: i32 = 16;
const MAX_BOATS: usize = 3;
// Altitude cue: planes drop a dark, offset copy of their sprite on the sea,
// lit from the upper left.
const PLANE_SHADOW_DX: f32 = 6.0;
const PLANE_SHADOW_DY: f32 = 10.0;

// At most one island appears every 24–42 seconds; size, health, score and guns scale together.
// Sizes match `gfx::ISLAND_SIZES`; flak targets the player's position when fired.
const ISLAND_SIZES: [(i32, i32); 3] = [(64, 44), (98, 68), (140, 96)];
const ISLAND_HP: [i32; 3] = [26, 46, 74];
const ISLAND_SCORE: [i32; 3] = [150, 260, 420];
const MAX_ISLANDS: usize = 1;
const ISLAND_MIN_INTERVAL: f32 = 24.0;
const ISLAND_MAX_INTERVAL: f32 = 42.0;
/// Flak cannon emplacements per island size (fractions of its width and
/// height); the first sits on the sprite's baked gun pit.
const FLAK_GUNS: [&[(f32, f32)]; 3] = [
    &[(0.5, 0.42)],
    &[(0.5, 0.42), (0.3, 0.62)],
    &[(0.5, 0.42), (0.28, 0.6), (0.72, 0.66)],
];
const FLAK_RELOAD: (f32, f32) = (2.2, 3.2); // seconds between one gun's shots
const FLAK_SPEED: f32 = 240.0;
/// Flying clean pays: every STREAK_PER things destroyed without taking a hit add one to the score multiple, up to
/// STREAK_MAX; any hit starts the count again. Dodging well is worth more than spraying.
const STREAK_PER: i32 = 5;
// At 4 the bot scored three and a half times what random play did (skill 0.27 in the fun report): a clean flier
// holds the top multiple for long stretches, so a higher one pays the dodging that keeps it.
const STREAK_MAX: i32 = 6;
/// Guns hold fire at a plane closer than this: a shell's flight (0.46 s) must leave time to see it and turn away.
const FLAK_MIN_RANGE: f32 = 110.0;
const FLAK_BURST_R: f32 = 24.0;    // how far a burst reaches
const FLAK_LETHAL: f32 = 0.25;     // seconds a burst can hurt; then it is smoke
const FLAK_SMOKE: f32 = 1.4;
const MAX_FLAK: usize = 12;

// From level 3, a full-width barrier approaches; destroy its 3–10 HP motor before it reaches you.
// Barriers never overlap a boss fight.
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
/// How long a continue is offered at game over.
const CONTINUE_SECS: f32 = 9.0;
const DEAD_PAUSE: f32 = 1.6;
const RESPAWN_GRACE: f32 = 1.5;
const WIN_PAUSE: f32 = 2.2;
// How long GAME OVER stays before a key can dismiss it, so fire still mashed
// from the fight does not skip the score.
const OVER_MIN_WAIT: f32 = 3.0;

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Launch, Play, Dead, Win, Won, Over }

#[derive(Copy, Clone, PartialEq, Eq)]
enum EnemyKind { Grunt, Weaver, Ace } // also the index of its sprite

#[derive(Copy, Clone)]
struct Bullet { x: f32, y: f32, vx: f32, vy: f32, active: bool }
blip::pooled!(Bullet, Round, Casing, Enemy, Wreck, Puff, Explosion, Powerup, HealthPickup, Boat, Island, FlakShell, FlakBurst);

/// A player round: centred on x, moving at (vx, -vy), calibre r (drawn
/// radius; hit box 2r + 2 wide). `bounced` once it has ricocheted: it can
/// still hit, not glance off again.
#[derive(Copy, Clone)]
struct Round { x: f32, y: f32, vx: f32, vy: f32, r: f32, bounced: bool, active: bool }

/// A spent brass casing kicked out of the guns, tumbling away and fading.
#[derive(Copy, Clone)]
struct Casing { x: f32, y: f32, vx: f32, vy: f32, rot: f32, ttl: f32, active: bool }

/// A leader follows its flight path; wingmen steer toward their leader-relative `slot`.
/// Both use the same flight physics and move along their nose direction.
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
    /// How many planes it launched with and how many were shot down: all of
    /// them is a formation bonus (see FLIGHT_BONUS).
    members: u8, downed: u8,
}

#[derive(Copy, Clone)]
struct Explosion { x: f32, y: f32, ttl: f32, max_ttl: f32, scale: f32, color: BlipColor, active: bool }

/// A shot-down plane falling to the sea: it carries on its way (bent a
/// little, never back), rolls, shrinks, trails fire and smoke, and splashes.
/// `heading` turns the sprite; `course` is where it goes.
#[derive(Copy, Clone)]
struct Wreck {
    x: f32, y: f32, heading: f32, spin: f32, speed: f32, t: f32, kind: EnemyKind, puff_t: f32, active: bool,
    /// `course` is the direction it is moving (its momentum, carried over
    /// from the plane), which `bend` turns; `heading` is where the airframe
    /// points, which `spin` and `wobble` turn. `dur` is the fall's length.
    course: f32, bend: f32, wobble: f32, dur: f32,
    fall: Fall,
    /// A tumble flips the way it spins when this runs out.
    flip_t: f32,
}

/// How a shot-down plane goes in.
#[derive(Copy, Clone, PartialEq)]
enum Fall {
    /// Rolling down its course, the roll tightening.
    Roll,
    /// A corkscrew: circling down round a point, the circles closing.
    Spiral,
    /// Spinning about itself on the spot, dropping fast.
    FlatSpin,
    /// Nose down and steady, burning hard, gathering speed.
    Dive,
    /// A wing gone: lurching one way then the other.
    Tumble,
}

const DEAD_WRECK: Wreck = Wreck { x: 0.0, y: 0.0, heading: 0.0, spin: 0.0, speed: 0.0, t: 0.0, kind: EnemyKind::Grunt,
    puff_t: 0.0, active: false, course: 0.0, bend: 0.0, wobble: 0.0, dur: 1.0, fall: Fall::Roll, flip_t: 0.0 };

/// Fire (short, bright, shrinking) or smoke (dark, growing, drifting with the sea).
#[derive(Copy, Clone)]
struct Puff { x: f32, y: f32, r: f32, grow: f32, ttl: f32, max_ttl: f32, fire: bool, top: bool, active: bool }  // top: drawn over the player (its own damage)

/// Standard fireball orange, the default for combat explosions.
const EXPLOSION_ORANGE: BlipColor = BlipColor { r: 1.0, g: 0.6, b: 0.15, a: 1.0 };
const POPUP_GOLD: BlipColor = BlipColor { r: 1.0, g: 0.9, b: 0.45, a: 1.0 };

#[derive(Copy, Clone)]
struct Powerup { x: f32, y: f32, active: bool }

#[derive(Copy, Clone)]
struct HealthPickup { x: f32, y: f32, active: bool }

#[derive(Copy, Clone)]
struct Cloud { x: f32, y: f32, r: f32, speed: f32, variant: u8 }

/// An enemy boat on the sea: scrolls with the water plus its own slow cruise;
/// shootable for a small bonus. `bob_phase` keeps a flotilla out of step.
#[derive(Copy, Clone)]
struct Boat { x: f32, y: f32, active: bool, vx: f32, bob_phase: f32 }

/// An island drifting down with the current, its flak cannons firing on
/// their own timers; shootable for a bonus. `size` indexes ISLAND_SIZES /
/// HP / SCORE / FLAK_GUNS.
#[derive(Copy, Clone)]
struct Island { x: f32, y: f32, size: u8, active: bool, hp: i32, guns: [FlakGun; 3] }

/// A flak cannon: its barrel follows the player; `reload` counts to the next shot.
#[derive(Copy, Clone)]
struct FlakGun { angle: f32, reload: f32, flash: f32 }

/// A flak shell in flight, fused to burst after `left` more pixels: where
/// the player was when it was fired (a real shell is not steered).
#[derive(Copy, Clone)]
struct FlakShell { x: f32, y: f32, vx: f32, vy: f32, left: f32, active: bool }

/// A flak burst: lethal for FLAK_LETHAL, then a drifting smoke ball.
#[derive(Copy, Clone)]
struct FlakBurst { x: f32, y: f32, t: f32, active: bool }

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

/// A boss turret tracks at a limited slew rate and fires aimed bursts from a finite drum.
#[derive(Copy, Clone)]
struct Turret {
    fx: f32, fy: f32, // x: fraction of the half span, y: fraction of the length from the nose
    angle: f32,       // barrels' direction, 0 = straight down the screen
    hp: i32,
    burst: u8, burst_t: f32,
    smoke_t: f32,
    /// Rounds left in the drum; while `reload_t` runs, a new drum is going on.
    ammo: u8,
    reload_t: f32,
    /// Seconds until the next burst may start.
    next_t: f32,
    /// Muzzle flash, counting down.
    flash: f32,
}

/// A laser turret: it turns slowly, locks on and charges where it is
/// pointing (the aim line shows the beam's path), then fires a beam that
/// kills outright, and cools down. Easy to step out of, deadly to ignore.
#[derive(Copy, Clone)]
struct Laser {
    fx: f32, fy: f32,
    angle: f32,
    hp: i32,
    phase: LaserPhase,
    /// Seconds left in the phase.
    t: f32,
}

#[derive(Copy, Clone, PartialEq)]
enum LaserPhase { Cooling, Charging, Firing }

const DEAD_LASER: Laser = Laser { fx: 0.0, fy: 0.0, angle: 0.0, hp: 0, phase: LaserPhase::Cooling, t: 0.0 };
const LASER_TURN: f32 = 0.35;     // radians a second: a slow traverse
const LASER_COOL: f32 = 2.6;
const LASER_CHARGE: f32 = 1.3;    // the warning, aim locked
const LASER_FIRE: f32 = 0.7;
const LASER_HALF_W: f32 = 5.0;    // the beam's lethal half width
const LASER_HP: i32 = 30;

const DEAD_TURRET: Turret = Turret { fx: 0.0, fy: 0.0, angle: 0.0, hp: 0, burst: 0, burst_t: 0.0, smoke_t: 0.0,
    ammo: 0, reload_t: 0.0, next_t: 0.0, flash: 0.0 };
/// Bursts in a drum, and the seconds a gunner takes to change it.
const DRUM_BURSTS: u8 = 5;
const RELOAD_SECS: f32 = 2.6;
/// Seconds between rounds in a burst.
const ROUND_GAP: f32 = 0.075;
/// How far the barrels may be off target and still fire (radians).
const ON_TARGET: f32 = 0.12;
/// Gunners switch how they aim (spec.patterns) this often.
const PATTERN_SECS: f32 = 4.0;

#[derive(Copy, Clone)]
struct Boss {
    turrets: [Turret; MAX_TURRETS],
    n_turrets: usize,
    lasers: [Laser; MAX_LASERS],
    n_lasers: usize,
    x: f32, y: f32,
    active: bool,
    entered: bool, // finished its entrance descent, patrolling now
    hp: i32, max_hp: i32,
    dir: f32,
    tier: usize, // 0..=6, indexes BOSS_SPECS / BOSS_SIZES (level - 1)
    t: f32,      // seconds since spawn, drives the dip wiggle and the sweep pattern
    escort_timer: Timer,
}

/// How a boss's gunners aim a volley; tougher bosses mix in more of them.
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
    prop_angle: f32,  // the propeller, drawn turning once it slows enough to see
    /// Seconds into an engine splutter (bad fuel), negative when running
    /// clean; and seconds until the next one.
    splutter_t: f32,
    splutter_next: f32,
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
    flak: [FlakShell; MAX_FLAK],
    flak_bursts: [FlakBurst; MAX_FLAK],
    sea_scroll: f32,
    /// The last level's flight over land (see land.rs): pixels scrolled since
    /// the launch, where the city begins (inland), and the flak batteries.
    land_dist: f32,
    city_from: Option<f32>,
    batteries: Vec<land::Battery>,
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
    /// Points rising from whatever was just shot down.
    fx: Fx,
    /// Things destroyed in a row without being hit (see STREAK_PER).
    streak: blip::Streak,
    /// The continue offered at game over, and the best score of the run before one was taken (a continue starts
    /// the score afresh; the run's best is what is reported).
    cont: blip::Continue,
}

fn wave_target_for(level: i32) -> i32 {
    let n = (WAVE_KILL_BASE + (level - 1) * 15).min(160);
    // the last level is the long flight in over the coast to the city
    if level == MAX_LEVEL { n * 7 / 5 } else { n }
}

/// Seconds between flights arriving.
fn spawn_interval_range(level: i32) -> (f32, f32) {
    let l = (level - 1).min(6) as f32;
    (1.5 - l * 0.08, 2.6 - l * 0.14)
}

// ---- weapon tiers ---------------------------------------------------------
// Every property indexed by weapon_level (1..=MAX_WEAPON_LEVEL), so the
// ladder from single gun to seven-way barrage tunes in one place.

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
            trim: 100.0, max_bank: FLIGHT_MAX_BANK, legs: [Leg::Straight(0.0); 4], n: 0, leg: 0, leg_t: 0.0, turned: 0.0, t: 0.0, members: 0, downed: 0 };
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
            prop_angle: 0.0,
            splutter_t: -1.0,
            splutter_next: SPLUTTER_FIRST,
            launch_t: 0.0,
            carrier_scale: 1.0,
            explosions: [dead_explosion; MAX_EXPLOSIONS],
            wrecks: [DEAD_WRECK; MAX_WRECKS],
            puffs: [Puff { x: 0.0, y: 0.0, r: 0.0, grow: 0.0, ttl: 0.0, max_ttl: 1.0, fire: false, top: false, active: false }; MAX_PUFFS],
            powerups: [dead_powerup; MAX_POWERUPS],
            health_pickups: [dead_health_pickup; MAX_HEALTH_PICKUPS],
            clouds,
            boats: [Boat { x: 0.0, y: 0.0, active: false, vx: 0.0, bob_phase: 0.0 }; MAX_BOATS],
            boat_timer: { let mut t = Timer::default(); t.start(3.0); t },
            islands: [Island { x: 0.0, y: 0.0, size: 0, active: false, hp: 0, guns: [FlakGun { angle: 0.0, reload: 0.0, flash: 0.0 }; 3] }; MAX_ISLANDS],
            island_timer: { let mut t = Timer::default(); t.start(14.0); t }, // first one shows up a bit into the level, not instantly
            flak: [FlakShell { x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, left: 0.0, active: false }; MAX_FLAK],
            flak_bursts: [FlakBurst { x: 0.0, y: 0.0, t: 0.0, active: false }; MAX_FLAK],
            sea_scroll: 0.0,
            land_dist: 0.0,
            city_from: None,
            batteries: Vec::new(),
            barrier: Barrier {
                active: false, motor_x: 0.0, hp: 0, max_hp: 0,
                warmup: Timer::default(), t: 0.0, y: BARRIER_START_Y,
            },
            barrier_timer: { let mut t = Timer::default(); t.start(BARRIER_MIN_INTERVAL); t },
            boss: Boss {
                turrets: [DEAD_TURRET; MAX_TURRETS],
                n_turrets: 0,
                lasers: [DEAD_LASER; MAX_LASERS],
                n_lasers: 0,
                x: 0.0, y: 0.0, active: false, entered: false,
                hp: 0, max_hp: 0, dir: 1.0, tier: 0, t: 0.0,
                escort_timer: Timer::default(),
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
            fx: Fx::new(),
            streak: blip::Streak::new(STREAK_PER, STREAK_MAX),
            cont: blip::Continue::default(),
        }
    }

    /// Score `pts` for something destroyed at (`x`, `y`), shown where it was.
    fn score_at(&mut self, x: f32, y: f32, pts: i32) {
        let times = self.streak.hit();
        let pts = pts * times;
        self.sess.add_score(pts);
        let text = if times > 1 { format!("{pts} X{times}") } else { pts.to_string() };
        self.fx.popup(x, y - 12.0, &text, POPUP_GOLD);
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

    fn new_plane(&mut self) {
        self.player_x = ((WIN_W - PLAYER_W) / 2) as f32;
        self.player_y = PLAYER_MAX_Y;
        self.splutter_t = -1.0;
        self.splutter_next = SPLUTTER_FIRST;
    }

    /// The engine is dead for this part of a splutter: no thrust.
    fn engine_out(&self) -> bool {
        self.splutter_t >= SPLUTTER_DIES && self.splutter_t < SPLUTTER_CATCHES
    }

    /// Fresh wave: clears all entities and (re)arms the spawner. Used both for
    /// a brand-new game and for the stage transition after a boss kill — the
    /// caller decides whether `sess` gets reset first.
    fn start_round(&mut self) {
        // A new stage keeps the guns earned, one tier down, so the next
        // stage still has power-ups worth chasing.
        self.weapon_level = (self.weapon_level - 1).max(1);
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
        for b in self.flak.iter_mut() { b.active = false; }
        for b in self.flak_bursts.iter_mut() { b.active = false; }
        self.boss.active = false;
        self.barrier.active = false;
        self.wave_kills = 0;
        self.wave_target = wave_target_for(self.sess.level);
        self.land_dist = 0.0;
        self.city_from = None;
        self.batteries = if self.sess.level == MAX_LEVEL { land::lay_batteries() } else { Vec::new() };
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
        self.streak.miss();
        self.sess.level = self.sess.level.min(MAX_LEVEL);
        blip::bot::levels(MAX_LEVEL as usize);
        blip::bot::reached_level(self.sess.level);
        // A bot run that starts past level 1 is armed like a player who got there.
        self.weapon_level = self.sess.level.min(MAX_WEAPON_LEVEL);
        self.start_round();
    }

    /// Just after a death: put the player back without touching the wave in
    /// progress (enemies and the boss, if any, keep going).
    fn respawn(&mut self) {
        self.new_plane();
        self.health = PLAYER_HEALTH_MAX;
        self.respawn_grace.start(RESPAWN_GRACE);
        self.state = State::Play;
    }
}

async fn load_takes(wavs: &[&[u8]]) -> Vec<blip::BlipSound> {
    let mut v = Vec::with_capacity(wavs.len());
    for w in wavs { v.push(blip::audio::load_sound(w).await); }
    v
}

blip::sounds!(Sounds {
    /// Per weapon tier, two takes of that many guns firing together.
    shoot: Vec<[blip::BlipSound; 2]> = {
        let mut shoot = Vec::new();
        for [a, b] in SHOOT_WAV {
            shoot.push([blip::audio::load_sound(a).await, blip::audio::load_sound(b).await]);
        }
        shoot
    },
    /// Several takes of each explosion, picked at random (play_take).
    enemy_explode: [blip::BlipSound; 4] = load_takes(&ENEMY_EXPLODE_WAV).await.try_into().unwrap(),
    player_explode: [blip::BlipSound; 2] = load_takes(&PLAYER_EXPLODE_WAV).await.try_into().unwrap(),
    player_hit,
    ricochet: [blip::BlipSound; 2] = load_takes(&RICOCHET_WAV).await.try_into().unwrap(),
    boss_explode: [blip::BlipSound; 2] = load_takes(&BOSS_EXPLODE_WAV).await.try_into().unwrap(),
    boss_warning,
    // Escalating weapon-tier pickup chimes: index 0 = reaching tier 2, ...,
    // index 3 = reaching tier 5 (the big fanfare). Index 0 is also reused for
    // the "already maxed, bonus points" catch.
    powerup_up: [blip::BlipSound; 4] = [blip::sound!("powerup2"), blip::sound!("powerup3"), blip::sound!("powerup4"), blip::sound!("max_power")],
    health_pickup,
    tick,
    streak_up: [blip::BlipSound; 3] = [blip::sound!("streak2"), blip::sound!("streak3"), blip::sound!("streak4")],
    stage_clear,
    victory,
    game_over,
    turret_fire,
    flak_burst,
    laser_charge,
    laser_fire,
    // Looped and volume-ridden live by update_barrier() — not a one-shot.
    barrier_hum,
    barrier_hum2,
    // Carrier launch: a one-shot engine crank/catch, plus a seamless
    // propeller loop update_launch() fades in and out around it.
    engine_start,
    engine_splutter,
    enemy_gun,
    backfire,
    /// In-flight engine loops, slowest first (see ENGINE_AT, engine_mix()).
    engine: [blip::BlipSound; 5] = load_takes(&ENGINE_WAV).await.try_into().unwrap(),
});

fn update_title(g: &mut Game) {
    if coin_start() { g.start_game(); }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    let player_before = (g.player_x, g.player_y);
    let powerup_starts = blip::frame_starts(&g.powerups, |p| p.active.then_some((p.x, p.y)));
    let health_starts = blip::frame_starts(&g.health_pickups, |p| p.active.then_some((p.x, p.y)));
    let enemy_starts = blip::frame_starts(&g.enemies, |e| e.active.then_some((e.x, e.y)));
    g.fx.update(dt);
    if let Some(m) = g.streak.risen() { play_sfx(&sfx.streak_up[(m as usize - 2).min(2)]); }
    g.respawn_grace.tick(dt);
    g.boss_intro.tick(dt);
    g.max_power_banner.tick(dt);

    fly_plane(g, dt, sfx);
    fire_volley(g, dt, sfx);

    let bullet_starts: [(f32, f32); MAX_PLAYER_BULLETS] = std::array::from_fn(|i| (g.bullets[i].x, g.bullets[i].y));
    // Rounds fired later this frame land in free slots and sweep from where they are.
    let enemy_bullet_starts = blip::frame_starts(&g.enemy_bullets, |b| b.active.then_some((b.x, b.y)));
    let (mut bullet_left_screen, mut enemy_bullet_left_screen) = move_shots(g, dt);
    damage_smoke(g, dt, sfx);
    let (powerup_expired, health_expired) = drift_pickups(g, dt);
    for e in pool_iter_mut(&mut g.explosions) {
        e.ttl -= dt;
        if e.ttl <= 0.0 { e.active = false; }
    }
    update_wrecks(g, dt, sfx);
    update_background(g, dt);

    move_enemies(g, dt, true, sfx);
    update_boss(g, dt, sfx);
    update_islands(g, dt, sfx);
    land::update_batteries(g, dt, sfx);
    update_flak(g, dt, sfx);
    update_barrier(g, dt, sfx);
    if !g.boss.active { send_waves(g, dt, sfx); }

    hit_with_bullets(g, sfx, &bullet_starts, &mut bullet_left_screen);
    retire(&mut g.bullets, &bullet_left_screen, |b| b.active = false);
    catch_powerups(g, player_before, &powerup_starts, sfx);
    catch_health(g, player_before, &health_starts, sfx);
    retire(&mut g.powerups, &powerup_expired, |p| p.active = false);
    retire(&mut g.health_pickups, &health_expired, |h| h.active = false);

    if g.state == State::Play && !g.respawn_grace.active() && g.stall_fall == 0.0 {
        let mut hit = struck(g, player_before, &enemy_bullet_starts, &mut enemy_bullet_left_screen, &enemy_starts);
        // BLIP_BOT_GOD=1: hits are counted, not felt, to reach the late game.
        if hit && blip::bot::flag("GOD") { g.health = PLAYER_HEALTH_MAX; hit = false; }
        if hit { hurt_plane(g, g.player_x, g.player_y, sfx); }
    }
    retire(&mut g.enemy_bullets, &enemy_bullet_left_screen, |b| b.active = false);
}

/// Switches off each slot flagged `gone`.
fn retire<T>(pool: &mut [T], gone: &[bool], off: impl Fn(&mut T)) {
    for (slot, _) in pool.iter_mut().zip(gone).filter(|(_, &gone)| gone) { off(slot); }
}

/// Both sides' rounds fly on; which of them left the screen (they stay live until the hit tests have had them).
fn move_shots(g: &mut Game, dt: f32) -> ([bool; MAX_PLAYER_BULLETS], [bool; MAX_ENEMY_BULLETS]) {
    let mut ours = [false; MAX_PLAYER_BULLETS];
    let mut theirs = [false; MAX_ENEMY_BULLETS];
    for (i, b) in g.bullets.iter_mut().enumerate() {
        if !b.active { continue; }
        b.y -= b.vy * dt;
        b.x += b.vx * dt;
        ours[i] = b.y < -BULLET_H || b.y > WIN_H as f32 || b.x < -10.0 || b.x > WIN_W as f32 + 10.0;
    }
    for (i, b) in g.enemy_bullets.iter_mut().enumerate() {
        if !b.active { continue; }
        b.y += b.vy * dt;
        b.x += b.vx * dt;
        theirs[i] = b.y > WIN_H as f32 + 10.0 || b.y < -20.0 || b.x < -10.0 || b.x > WIN_W as f32 + 10.0;
    }
    (ours, theirs)
}

/// Pickups drift down the screen; which of them fell off the bottom (still catchable this frame).
fn drift_pickups(g: &mut Game, dt: f32) -> ([bool; MAX_POWERUPS], [bool; MAX_HEALTH_PICKUPS]) {
    let mut powerups = [false; MAX_POWERUPS];
    let mut health = [false; MAX_HEALTH_PICKUPS];
    for (i, p) in g.powerups.iter_mut().enumerate() {
        if !p.active { continue; }
        p.y += POW_SPEED * dt;
        powerups[i] = p.y > WIN_H as f32;
    }
    for (i, h) in g.health_pickups.iter_mut().enumerate() {
        if !h.active { continue; }
        h.y += POW_SPEED * dt;
        health[i] = h.y > WIN_H as f32;
    }
    (powerups, health)
}

/// Between bosses: flights at the level's pace, the boss once enough are down, and now and then the barrier.
fn send_waves(g: &mut Game, dt: f32, sfx: &Sounds) {
    if g.spawn_timer.tick(dt) {
        send_flights(g);
        let (mn, mx) = spawn_interval_range(g.sess.level);
        g.spawn_timer.start(mn + rand_range_f32(0.0, 1.0) * (mx - mn));
    }
    if g.wave_kills >= g.wave_target {
        start_boss(g, sfx);
    }
    // A seldom, higher-level set-piece: checked only when nothing else
    // is already claiming the screen (no boss, no barrier already up).
    if !g.barrier.active && g.sess.level >= BARRIER_MIN_LEVEL && g.barrier_timer.tick(dt) {
        spawn_barrier(g, sfx);
    }
}

/// Whether anything struck the plane this frame: the aircraft, not its box (the fuselage and inner wings), swept
/// from where it was against rounds, flak, rammers, the boss, the barrier's beam and the boss's laser.
fn struck(g: &mut Game, before: (f32, f32), round_starts: &[Option<(f32, f32)>], round_left: &mut [bool; MAX_ENEMY_BULLETS],
    enemy_starts: &[Option<(f32, f32)>]) -> bool {
    let (hw, hh) = PLAYER_HIT;
    let (hx, hy) = plane_hit_box(g.player_x, g.player_y);
    let (hx0, hy0) = plane_hit_box(before.0, before.1);
    let mut hit = false;
    for i in 0..MAX_ENEMY_BULLETS {
        if !g.enemy_bullets[i].active && !round_left[i] { continue; }
        let now = (g.enemy_bullets[i].x, g.enemy_bullets[i].y);
        let start = blip::swept_from(round_starts[i], now);
        if blip::sweep_hits(start, now, (hx0, hy0), (hx, hy), (-2.0, -2.0, hw + 4.0, hh + 4.0)) {
            g.enemy_bullets[i].active = false;
            round_left[i] = false;
            blip::bot::add(if g.boss.active { "hit_boss_round" } else { "hit_round" }, 1.0);
            hit = true;
        }
    }
    for b in pool_iter_mut(&mut g.flak_bursts) {
        // a burst hurts once, in its first moments, if the plane is in it
        let (x0, y0, x1, y1) = (hx0 + hw / 2.0 - b.x, hy0 + hh / 2.0 - b.y, hx + hw / 2.0 - b.x, hy + hh / 2.0 - b.y);
        if b.t < FLAK_LETHAL && segment_circle_overlap(x0, y0, x1, y1, 0.0, 0.0, FLAK_BURST_R) {
            b.t = FLAK_LETHAL;
            blip::bot::add("hit_flak", 1.0);
            hit = true;
        }
    }
    // `i` pairs the enemy slot with the swept start recorded for it this frame.
    for (i, start0) in enemy_starts.iter().enumerate().take(MAX_ENEMIES) {
        if !g.enemies[i].active { continue; }
        let (ew, eh) = (ENEMY_W as f32 * 0.6, ENEMY_H as f32 * 0.6);
        let (ex, ey) = (g.enemies[i].x, g.enemies[i].y);
        let (ex0, ey0) = blip::swept_from(*start0, (ex, ey));
        let (ox, oy) = (ENEMY_W as f32 * 0.2, ENEMY_H as f32 * 0.2);
        if blip::sweep_hits((hx0, hy0), (hx, hy), (ex0 + ox, ey0 + oy), (ex + ox, ey + oy), (-hw, -hh, ew + hw, eh + hh)) {
            g.spawn_explosion(g.enemies[i].x, g.enemies[i].y, 1.0, EXPLOSION_ORANGE);
            g.enemies[i].active = false;
            blip::bot::add("hit_ram", 1.0);
            hit = true;
        }
    }
    let (bossw, bossh) = boss_size(g.boss.tier);
    if g.boss.active && rects_overlap(hx, hy, hw, hh, g.boss.x + bossw * 0.2, g.boss.y + bossh * 0.15, bossw * 0.6, bossh * 0.7) {
        hit = true;
    }
    // The beam: full width, so only shooting the motor stops it.
    let beam = (0.0, g.barrier.y - BARRIER_BEAM_H / 2.0, WIN_W as f32, BARRIER_BEAM_H);
    if g.barrier.active && !g.barrier.warmup.active() && rects_overlap(hx, hy, hw, hh, beam.0, beam.1, beam.2, beam.3) {
        blip::bot::add("hit_barrier", 1.0);
        hit = true;
    }
    // A laser beam is no glancing blow.
    if g.boss.active && laser_hits(g, hx, hy, hw, hh) {
        blip::bot::add("hit_laser", 1.0);
        hit = true;
        g.health = 1;
    }
    hit
}

fn update_dead(g: &mut Game, dt: f32) {
    g.fx.update(dt);
    if g.dead_timer.tick(dt) { g.respawn(); }
}

fn update_win(g: &mut Game, dt: f32) {
    g.fx.update(dt);
    if g.win_timer.tick(dt) { g.start_round(); }
}

fn update_won(g: &mut Game, dt: f32) {
    if play_again(&mut g.over_timer, dt) { g.start_game(); }
}

/// Game over offers a continue: fire within CONTINUE_SECS flies the same level again with fresh lives (and the
/// score afresh), so a player can see the later stages; otherwise the run's best is reported and the title returns.
fn update_over(g: &mut Game, dt: f32) {
    match g.cont.update(dt, blip::input::btn1_pressed()) {
        Some(true) => {
            g.sess.play_on(LIVES_START);
            g.start_round();
        }
        Some(false) => {
            g.sess.report();
            g.state = State::Title;
        }
        None => {}
    }
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("RAIDER", WIN_W, WIN_H)
}

const CLOUD_PNGS: [&[u8]; 3] = [blip::png!("cloud_1"), blip::png!("cloud_2"), blip::png!("cloud_3")];
// One sprite per island size (small/medium/large) — see ISLAND_SIZES above.
const ISLAND_PNGS: [&[u8]; 3] = [blip::png!("island_small"), blip::png!("island_medium"), blip::png!("island_large")];

// One sprite per boss tier (levels 1-7) — see BOSS_SPECS / BOSS_SIZES above.
const BOSS_PNGS: [&[u8]; 7] = [
    blip::png!("boss_1"), blip::png!("boss_2"), blip::png!("boss_3"), blip::png!("boss_4"), blip::png!("boss_5"),
    blip::png!("boss_6"), blip::png!("boss_7"),
];

// The Japanese banner for each boss name — see draw_play()'s WARNING banner.
const BOSS_NAME_JA_PNGS: [&[u8]; 7] = [
    blip::png!("boss_name_ja_1"), blip::png!("boss_name_ja_2"), blip::png!("boss_name_ja_3"),
    blip::png!("boss_name_ja_4"), blip::png!("boss_name_ja_5"), blip::png!("boss_name_ja_6"),
    blip::png!("boss_name_ja_7"),
];

const SHOOT_WAV: [[&[u8]; 2]; 5] = [
    [blip::wav!("burst1_1"), blip::wav!("burst1_2")], [blip::wav!("burst2_1"), blip::wav!("burst2_2")],
    [blip::wav!("burst3_1"), blip::wav!("burst3_2")], [blip::wav!("burst4_1"), blip::wav!("burst4_2")],
    [blip::wav!("burst5_1"), blip::wav!("burst5_2")],
];
const ENEMY_EXPLODE_WAV: [&[u8]; 4] = [
    blip::wav!("enemy_explode0"), blip::wav!("enemy_explode1"), blip::wav!("enemy_explode2"),
    blip::wav!("enemy_explode3"),
];
const PLAYER_EXPLODE_WAV: [&[u8]; 2] = [blip::wav!("player_explode0"), blip::wav!("player_explode1")];
const RICOCHET_WAV: [&[u8]; 2] = [blip::wav!("ricochet1"), blip::wav!("ricochet2")];
const BOSS_EXPLODE_WAV: [&[u8]; 2] = [blip::wav!("boss_explode0"), blip::wav!("boss_explode1")];
const ENGINE_WAV: [&[u8]; 5] = [
    blip::wav!("engine0"), blip::wav!("engine1"), blip::wav!("engine2"), blip::wav!("engine3"), blip::wav!("engine4"),
];

#[blip::macroquad::main(conf)]
async fn main() {
    blip::bot::playthroughs(bot::PLAYTHROUGHS);
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();
    let art = Art::load();
    let sfx = Sounds::load().await;
    // Two loops in rotation, so a long level doesn't hear the same ~40s on repeat.
    let mut music = Jukebox::new("raider", &["LOOP1", "LOOP2"]);
    if blip.screenshot_mode { g.start_game(); }

    loop {
        let dt = blip.delta_time;
        music.rotate(dt);
        if blip::bot::active() { bot::drive(&g); }
        let prev_state = g.state;
        match g.state {
            State::Title  => update_title(&mut g),
            // Fire held hurries the take-off: it plays before every level, and a player who has seen it can skip ahead.
            State::Launch => update_launch(&mut g, if key_active(BLIP_KEY_SPACE) { dt * LAUNCH_HURRY } else { dt }, &sfx),
            State::Play   => update_play(&mut g, dt, &sfx),
            State::Dead   => update_dead(&mut g, dt),
            State::Win    => update_win(&mut g, dt),
            State::Won    => update_won(&mut g, dt),
            State::Over   => update_over(&mut g, dt),
        }
        state_sounds(&mut g, prev_state, &sfx);

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title  => draw_title(&blip, &art.title_backdrop, &art.player),
            State::Launch => draw_launch(&blip, &g, &art.player, &art.enemies, &art.carrier, &art.boat, &art.clouds),
            State::Win    => draw_win(&blip, g.sess.level),
            State::Won    => draw_won(&blip, g.sess.score, web::high_score(), g.over_timer.active()),
            State::Over   => {
                blip.draw_game_over(g.sess.score, web::high_score(), BLIP_RED, BLIP_GREEN, BLIP_YELLOW, false);
                blip.draw_continue(&g.cont, BLIP_YELLOW);
            }
            State::Play | State::Dead => draw_play(&blip, &g, &art),
        }

        blip.next_frame().await;
    }
}

/// The game's pictures.
struct Art {
    player: Texture2D, title_backdrop: Texture2D, enemies: [Texture2D; 3], bosses: [Texture2D; BOSS_PNGS.len()],
    boss_names_ja: [Texture2D; BOSS_NAME_JA_PNGS.len()], powerup: Texture2D, health: Texture2D, carrier: Texture2D,
    boat: Texture2D, clouds: [Texture2D; CLOUD_PNGS.len()], islands: [Texture2D; ISLAND_PNGS.len()],
}

impl Art {
    fn load() -> Art {
        Art {
            player: load_png(blip::png!("player_plane")),
            title_backdrop: load_png(blip::png!("title_backdrop")),
            enemies: [load_png(blip::png!("enemy_grunt")), load_png(blip::png!("enemy_weaver")), load_png(blip::png!("enemy_ace"))],
            bosses: BOSS_PNGS.map(load_png),
            boss_names_ja: BOSS_NAME_JA_PNGS.map(load_png),
            powerup: load_png(blip::png!("powerup")),
            health: load_png(blip::png!("health_pack")),
            carrier: load_png(blip::png!("carrier")),
            boat: load_png(blip::png!("boat")),
            clouds: CLOUD_PNGS.map(load_png_smooth),
            islands: ISLAND_PNGS.map(load_png),
        }
    }
}

/// The sounds that follow a change of state: the continue's tick, the stage clear, game over (a new best ends on the
/// stage-clear fanfare), the engine's start, and in flight the engine itself, a crossfaded bank that runs while the
/// plane flies and stops at a death, a stage clear or game over.
fn state_sounds(g: &mut Game, prev: State, sfx: &Sounds) {
    if g.state == State::Over && g.cont.ticked() { play_sfx(&sfx.tick); }
    if prev != State::Win && g.state == State::Win { play_sfx(&sfx.stage_clear); }
    if prev != State::Over && g.state == State::Over {
        play_sfx(if g.sess.beats_best() { &sfx.stage_clear } else { &sfx.game_over });
    }
    if prev != State::Won && g.state == State::Won { g.sess.report(); }
    let flying = |s: State| matches!(s, State::Launch | State::Play);
    if !flying(prev) && flying(g.state) {
        for e in sfx.engine.iter() { play_sound(e, PlaySoundParams { looped: true, volume: 0.0 }); }
    }
    if flying(prev) && !flying(g.state) {
        for e in sfx.engine.iter() { stop_sound(e); }
    }
    if flying(g.state) {
        for (e, v) in sfx.engine.iter().zip(engine_mix(g)) { set_sound_volume(e, v); }
    }
    if prev != State::Launch && g.state == State::Launch { play_sfx(&sfx.engine_start); }
}

#[cfg(test)]
mod tests;
