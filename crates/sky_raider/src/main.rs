//! Raider — a 1942-style vertical dogfighting shoot-'em-up.

use blip::input::{
    btn1_pressed, key_active, key_held, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_DOWN, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::audio::{play_sound, set_sound_volume, stop_sound, PlaySoundParams};
use blip::macroquad::math::vec2;
use blip::macroquad::shapes::draw_triangle;
use blip::macroquad::rand::rand;
use blip::macroquad::texture::{draw_texture_ex, DrawTextureParams, Texture2D};
use blip::{
    load_png, load_png_smooth,
    clamp, Jukebox, play_sfx, play_sfx_volume, pool_iter, pool_iter_mut, pool_spawn, rects_overlap, web,
    window_conf, Blip, BlipColor, Fx, LifeResult, Pooled, Session, Timer,
    BLIP_BLACK, BLIP_BLUE, BLIP_CYAN, BLIP_GRAY, BLIP_GREEN, BLIP_ORANGE, BLIP_RED, BLIP_WHITE,
    BLIP_YELLOW,
};

#[cfg(not(target_arch = "wasm32"))]
mod bot;
mod land;

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
const WAVE_KILL_BASE: i32 = 60;
/// Per plane, times the level, for shooting down every plane of a flight.
const FLIGHT_BONUS: i32 = 50;

// ---- flight model ---------------------------------------------------------
// Each enemy flight flies one path the way an aeroplane does: it banks to
// turn. In a level coordinated turn
//   turn rate  w = g tan(bank) / v        radius  r = v^2 / (g tan(bank))
//   load       n = 1 / cos(bank)
// and the stall speed rises with load, v_stall(n) = v_stall sqrt(n), so at
// speed v the bank is capped at acos((v_stall / v)^2). Turning costs speed
// (induced drag grows with n^2 - 1) and the engine wins it back on the
// straights; bank changes at the roll rate, so paths curve, never corner.
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
    (96, 76), (104, 84), (116, 92), (150, 106), (164, 122), (182, 134), (192, 152),
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
    &[(0.0, 0.05), (0.0, 0.97)],
];
const BOSS_INTRO_TIME: f32 = 1.3;

// Wing gun positions per boss, as fractions of the half span (both sides),
// on the wing's centre chord; the fuselage guns are BOSS_TURRETS. More
// turrets each level, 3, 5, 7, 8, 9, 12, until the last: only 4 guns, but
// two laser turrets (BOSS_LASERS).
const BOSS_WING_GUNS: [&[f32]; 7] = [
    &[], &[0.62], &[0.62], &[0.36, 0.72], &[0.31, 0.62], &[0.32, 0.6, 0.8], &[0.62],
];
// Laser turrets: one on the second-last boss's spine, one on each of the
// last boss's wings (x, y as in BOSS_TURRETS).
const BOSS_LASERS: [&[(f32, f32)]; 7] = [
    &[], &[], &[], &[], &[], &[(0.0, 0.42)], &[(-0.36, 0.38), (0.36, 0.38)],
];
const MAX_LASERS: usize = 2;
// Wing chords per boss (root LE, root TE, tip LE, tip TE as fractions of
// the length from the nose). Must match blip_assets' boss_plane().
const BOSS_WINGS: [(f32, f32, f32, f32); 7] = [
    (0.30, 0.52, 0.37, 0.46), (0.30, 0.50, 0.37, 0.45), (0.32, 0.54, 0.40, 0.48), (0.30, 0.52, 0.38, 0.46),
    (0.30, 0.46, 0.34, 0.41), (0.30, 0.48, 0.36, 0.43), (0.30, 0.46, 0.36, 0.41),
];
const MAX_TURRETS: usize = 16;

// ---- carrier launch -------------------------------------------------------
// Every level opens on an Essex-class deck: engine start, the launch
// officer's signal, the deck run, and the climb away. Sizes must match
// blip_assets' CARRIER_W / CARRIER_H.
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

// ---- background: sea, sky, boats
// -------------------------------------------
// The sea and its boats share one scroll speed; clouds float between the sea
// and the planes.
const SEA_SCROLL_SPEED: f32 = 70.0;

// ---- airspeed ---------------------------------------------------------
// 1.0 is cruise. Forward opens the throttle, back closes it; the plane never
// flies backwards. Below cruise the scroll carries it down the screen, and
// slow flight slows the scroll.
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
use blip_assets::sky_raider::{SPLUTTER_CATCHES, SPLUTTER_DIES, SPLUTTER_SECS};
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

// ---- islands & their flak ---------------------------------------------------
// A rare set-piece: an island drifting down with the sea, its flak cannons
// firing shells that burst where the player was when they fired. Size, HP,
// score and guns scale together; at most one on screen, every 24-42s.
// Sizes must match blip_assets' ISLAND_SIZES.
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
const FLAK_BURST_R: f32 = 24.0;    // how far a burst reaches
const FLAK_LETHAL: f32 = 0.25;     // seconds a burst can hurt; then it is smoke
const FLAK_SMOKE: f32 = 1.4;
const MAX_FLAK: usize = 12;

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
// How long GAME OVER stays before a key can dismiss it, so fire still mashed
// from the fight does not skip the score.
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

/// A player round: centred on x, moving at (vx, -vy), calibre r (drawn
/// radius; hit box 2r + 2 wide). `bounced` once it has ricocheted: it can
/// still hit, not glance off again.
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
    /// How many planes it launched with and how many were shot down: all of
    /// them is a formation bonus (see FLIGHT_BONUS).
    members: u8, downed: u8,
}
impl Pooled for Enemy {
    fn is_active(&self) -> bool { self.active }
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
const POPUP_GOLD: BlipColor = BlipColor { r: 1.0, g: 0.9, b: 0.45, a: 1.0 };

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

/// An enemy boat on the sea: scrolls with the water plus its own slow cruise;
/// shootable for a small bonus. `bob_phase` keeps a flotilla out of step.
#[derive(Copy, Clone)]
struct Boat { x: f32, y: f32, active: bool, vx: f32, bob_phase: f32 }
impl Pooled for Boat {
    fn is_active(&self) -> bool { self.active }
}

/// An island drifting down with the current, its flak cannons firing on
/// their own timers; shootable for a bonus. `size` indexes ISLAND_SIZES /
/// HP / SCORE / FLAK_GUNS.
#[derive(Copy, Clone)]
struct Island { x: f32, y: f32, size: u8, active: bool, hp: i32, guns: [FlakGun; 3] }
impl Pooled for Island {
    fn is_active(&self) -> bool { self.active }
}

/// A flak cannon: its barrel follows the player; `reload` counts to the next shot.
#[derive(Copy, Clone)]
struct FlakGun { angle: f32, reload: f32, flash: f32 }

/// A flak shell in flight, fused to burst after `left` more pixels: where
/// the player was when it was fired (a real shell is not steered).
#[derive(Copy, Clone)]
struct FlakShell { x: f32, y: f32, vx: f32, vy: f32, left: f32, active: bool }
impl Pooled for FlakShell {
    fn is_active(&self) -> bool { self.active }
}

/// A flak burst: lethal for FLAK_LETHAL, then a drifting smoke ball.
#[derive(Copy, Clone)]
struct FlakBurst { x: f32, y: f32, t: f32, active: bool }
impl Pooled for FlakBurst {
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
/// turret's slew rate and fires along its barrels only once they are on
/// it, in bursts at a steady rhythm, from a drum that runs dry and is
/// changed by hand (the drum-fed Type 92 and Type 99 guns of these
/// bombers), so where and when it fires can be read off the turret.
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

fn rand01() -> f32 {
    (rand() as f32) / (u32::MAX as f32)
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
        }
    }

    /// Score `pts` for something destroyed at (`x`, `y`), shown where it was.
    fn score_at(&mut self, x: f32, y: f32, pts: i32) {
        self.sess.add_score(pts);
        self.fx.popup(x, y - 12.0, &pts.to_string(), POPUP_GOLD);
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
        self.weapon_level = 1;
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
    /// Several takes of each explosion, picked at random (play_take).
    enemy_explode: [blip::BlipSound; 4],
    player_explode: [blip::BlipSound; 2],
    player_hit: blip::BlipSound,
    ricochet: [blip::BlipSound; 2],
    boss_explode: [blip::BlipSound; 2],
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
    flak_burst: blip::BlipSound,
    laser_charge: blip::BlipSound,
    laser_fire: blip::BlipSound,
    // Looped and volume-ridden live by update_barrier() — not a one-shot.
    barrier_hum: blip::BlipSound,
    barrier_hum2: blip::BlipSound,
    // Carrier launch: a one-shot engine crank/catch, plus a seamless
    // propeller loop update_launch() fades in and out around it.
    engine_start: blip::BlipSound,
    engine_splutter: blip::BlipSound,
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
    spawn_island_sized(g, (rand() % ISLAND_SIZES.len() as u32) as u8);
}

fn spawn_island_sized(g: &mut Game, size: u8) {
    let (w, h) = ISLAND_SIZES[size as usize];
    let x = 10.0 + rand01() * (WIN_W as f32 - w as f32 - 20.0);
    // a moment to scroll fully into view before they open up, guns staggered
    let guns = [0, 1, 2].map(|k| FlakGun { angle: 0.0, reload: 1.6 + k as f32 * 0.7 + rand01() * 0.5, flash: 0.0 });
    pool_spawn(&mut g.islands, Island { x, y: -(h as f32), size, active: true, hp: ISLAND_HP[size as usize], guns });
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

/// The world under the fight: sea scroll, cloud drift, boat and island
/// spawns, shared by the launch and play so it keeps moving on the carrier.
/// Turrets fire only in play (update_islands()).
fn update_background(g: &mut Game, dt: f32) {
    let sc = SEA_SCROLL_SPEED * g.scroll_k;
    g.sea_scroll += sc * dt;
    if land::is_land_level(g) { g.land_dist += sc * dt; }

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
    // Nothing more puts to sea once the coast is in sight.
    let at_sea = land::coast_y(g).is_none_or(|cy| cy < -200.0);
    if g.boat_timer.tick(dt) && at_sea {
        spawn_boat(g);
        g.boat_timer.start(4.0 + rand01() * 4.0);
    }

    for isl in pool_iter_mut(&mut g.islands) {
        isl.y += sc * dt;
        if isl.y - ISLAND_SIZES[isl.size as usize].1 as f32 > WIN_H as f32 {
            isl.active = false;
        }
    }
    if g.island_timer.tick(dt) && at_sea {
        spawn_island(g);
        g.island_timer.start(ISLAND_MIN_INTERVAL + rand01() * (ISLAND_MAX_INTERVAL - ISLAND_MIN_INTERVAL));
    }
}

/// Turret AI + shell flight — play-only (see update_background() for why).
/// Where flak gun `k` of an island stands on screen.
fn flak_gun_pos(isl: &Island, k: usize) -> (f32, f32) {
    let (w, h) = ISLAND_SIZES[isl.size as usize];
    let (fx, fy) = FLAK_GUNS[isl.size as usize][k];
    (isl.x + w as f32 * fx, isl.y + h as f32 * fy)
}

/// A flak shell from (`x`, `y`), fused to burst where the player is now.
fn fire_flak(g: &mut Game, x: f32, y: f32, sfx: &Sounds) {
    let (px, py) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0);
    let (dx, dy) = (px - x, py - y);
    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
    // The pool is the cap on shells in the air: a full sky holds its fire.
    if pool_spawn(&mut g.flak, FlakShell { x, y, vx: dx / dist * FLAK_SPEED, vy: dy / dist * FLAK_SPEED, left: dist, active: true }) {
        play_sfx_volume(&sfx.turret_fire, 0.8);
    }
}

/// Each island flak gun follows the player and, on its own timer once the
/// island is in view, fires (see fire_flak).
fn update_islands(g: &mut Game, dt: f32, sfx: &Sounds) {
    let (px, py) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0);
    for ii in 0..MAX_ISLANDS {
        if !g.islands[ii].active || g.islands[ii].y < 0.0 { continue; } // still scrolling in
        for k in 0..FLAK_GUNS[g.islands[ii].size as usize].len() {
            let (x, y) = flak_gun_pos(&g.islands[ii], k);
            let gun = &mut g.islands[ii].guns[k];
            if !aim_flak_gun(gun, x, y, px, py, dt, FLAK_RELOAD) { continue; }
            fire_flak(g, x, y, sfx);
        }
    }
}

/// Turn a flak gun on the player and count down its reload; true when it fires.
fn aim_flak_gun(gun: &mut FlakGun, x: f32, y: f32, px: f32, py: f32, dt: f32, reload: (f32, f32)) -> bool {
    gun.angle = (px - x).atan2(py - y);
    gun.flash = (gun.flash - dt).max(0.0);
    gun.reload -= dt;
    if gun.reload > 0.0 { return false; }
    gun.reload = reload.0 + rand01() * (reload.1 - reload.0);
    gun.flash = 0.08;
    true
}

/// Flak shells flying to their fuse point and bursting; bursts turning to
/// smoke and drifting back.
fn update_flak(g: &mut Game, dt: f32, sfx: &Sounds) {
    let sc = SEA_SCROLL_SPEED * g.scroll_k;
    for i in 0..MAX_FLAK {
        let s = &mut g.flak[i];
        if !s.active { continue; }
        let step = (s.vx.hypot(s.vy) * dt).min(s.left);
        s.x += s.vx * dt;
        s.y += s.vy * dt;
        s.left -= step;
        if s.left <= 0.0 {
            s.active = false;
            let (x, y) = (s.x, s.y);
            pool_spawn(&mut g.flak_bursts, FlakBurst { x, y, t: 0.0, active: true });
            play_sfx_volume(&sfx.flak_burst, 0.75);
        }
    }
    for b in pool_iter_mut(&mut g.flak_bursts) {
        b.t += dt;
        b.y += sc * 0.6 * dt; // the smoke drifts back with the wind
        if b.t > FLAK_LETHAL + FLAK_SMOKE { b.active = false; }
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

/// Slots (right, behind) in the leader's frame, px: wingtips just clear
/// for ENEMY_W = 62.
fn slots(f: Formation) -> &'static [(f32, f32)] {
    match f {
        Formation::Vic     => &[(0.0, 0.0), (-62.0, 50.0), (62.0, 50.0)],
        Formation::Pair    => &[(0.0, 0.0), (67.0, 45.0)],
        Formation::Echelon => &[(0.0, 0.0), (62.0, 48.0), (124.0, 95.0)],
        Formation::Abreast => &[(-74.0, 0.0), (0.0, 0.0), (74.0, 0.0)],
        Formation::FingerFour => &[(0.0, 0.0), (-62.0, 48.0), (67.0, 41.0), (129.0, 86.0)],
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
        legs: plan, n: legs.len().min(4), leg: 0, leg_t: 0.0, turned: 0.0, t: 0.0,
        members: members.len() as u8, downed: 0 };
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
    let mut escort_timer = Timer::default();
    if spec.escorts { escort_timer.start(2.5); }
    let mut turrets = [DEAD_TURRET; MAX_TURRETS];
    let drum = boss_burst_len(tier) * DRUM_BURSTS;
    let mut n = 0;
    let (rle, rte, tle, tte) = BOSS_WINGS[tier];
    let mounts = BOSS_TURRETS[tier].iter().copied().chain(BOSS_WING_GUNS[tier].iter().flat_map(|&t| {
        let mid = (rle + (tle - rle) * t + rte + (tte - rte) * t) / 2.0;
        [(-t, mid), (t, mid)]
    }));
    for (fx, fy) in mounts.take(MAX_TURRETS) {
        // Drums start part-used and the first bursts staggered, so the guns
        // never all fall silent to reload at once.
        let ammo = drum - (n as u8 * 3) % drum;
        turrets[n] = Turret { fx, fy, hp: 6 + tier as i32 * 2, ammo, next_t: 0.6 + n as f32 * 0.13, ..DEAD_TURRET };
        n += 1;
    }
    let mut lasers = [DEAD_LASER; MAX_LASERS];
    for (i, &(fx, fy)) in BOSS_LASERS[tier].iter().enumerate().take(MAX_LASERS) {
        // the two of a pair out of step, so they never fire together
        lasers[i] = Laser { fx, fy, hp: LASER_HP, t: 1.5 + i as f32 * 1.6, ..DEAD_LASER };
    }
    g.boss = Boss {
        turrets,
        n_turrets: n,
        lasers,
        n_lasers: BOSS_LASERS[tier].len().min(MAX_LASERS),
        x: (WIN_W as f32 - bw) / 2.0, // centred: its patrol swings about the middle
        y: -bh,
        active: true,
        entered: false,
        hp: spec.hp, max_hp: spec.hp,
        dir: 1.0,
        tier,
        t: 0.0,
        escort_timer,
    };
    g.boss_intro.start(BOSS_INTRO_TIME);
    // The last fight is over the city: it begins at the top edge now.
    if let Some(cy) = land::coast_y(g) { g.city_from = Some(cy.max(0.0)); }
    play_sfx(&sfx.boss_warning);
}

/// The boss's bank in the draw (radians): it leans into its patrol.
fn boss_bank(g: &Game) -> f32 { -g.boss.dir * 0.05 }

/// A turret's centre on screen: the sprite flies nose-down and is drawn
/// rotated by boss_bank() about its centre.
fn turret_pos(g: &Game, k: usize) -> (f32, f32) {
    let t = &g.boss.turrets[k];
    mount_pos(g, t.fx, t.fy)
}

/// Where a mount at (`fx`, `fy`) on the boss is on screen (see BOSS_TURRETS).
fn mount_pos(g: &Game, fx: f32, fy: f32) -> (f32, f32) {
    let (bw, bh) = boss_size(g.boss.tier);
    let (cx, cy) = (g.boss.x + bw / 2.0, g.boss.y + bh / 2.0);
    let (dx, dy) = (fx * bw / 2.0, (0.5 - fy) * bh);
    let (s, c) = boss_bank(g).sin_cos();
    (cx + dx * c - dy * s, cy + dx * s + dy * c)
}

fn laser_radius(tier: usize) -> f32 { turret_radius(tier) * 1.35 }

/// Turn, lock, charge, fire and cool every laser turret.
fn update_lasers(g: &mut Game, dt: f32, sfx: &Sounds) {
    let (px, py) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0);
    for i in 0..g.boss.n_lasers {
        let (x, y) = mount_pos(g, g.boss.lasers[i].fx, g.boss.lasers[i].fy);
        let l = &mut g.boss.lasers[i];
        if l.hp <= 0 { continue; }
        l.t -= dt;
        match l.phase {
            LaserPhase::Cooling => {
                let want = (px - x).atan2(py - y);
                let err = (want - l.angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                l.angle += err.clamp(-LASER_TURN * dt, LASER_TURN * dt);
                if l.t <= 0.0 {
                    l.phase = LaserPhase::Charging;
                    l.t = LASER_CHARGE;
                    play_sfx_volume(&sfx.laser_charge, 0.8);
                }
            }
            LaserPhase::Charging => if l.t <= 0.0 {
                l.phase = LaserPhase::Firing;
                l.t = LASER_FIRE;
                play_sfx_volume(&sfx.laser_fire, 0.9);
            },
            LaserPhase::Firing => if l.t <= 0.0 {
                l.phase = LaserPhase::Cooling;
                l.t = LASER_COOL;
            },
        }
    }
}

/// Is the player's hit box in a firing laser's beam?
fn laser_hits(g: &Game, hx: f32, hy: f32, hw: f32, hh: f32) -> bool {
    let (cx, cy) = (hx + hw / 2.0, hy + hh / 2.0);
    (0..g.boss.n_lasers).any(|i| {
        let l = &g.boss.lasers[i];
        if l.hp <= 0 || l.phase != LaserPhase::Firing { return false; }
        let (x, y) = mount_pos(g, l.fx, l.fy);
        let (s, c) = l.angle.sin_cos();
        let (dx, dy) = (cx - x, cy - y);
        let along = dx * s + dy * c;
        let across = (dx * c - dy * s).abs();
        along > 0.0 && across < LASER_HALF_W + hw.min(hh) * 0.35
    })
}

fn turret_radius(tier: usize) -> f32 { 5.6 + tier as f32 * 0.35 }

/// One round from (x, y) at `angle` off straight down.
fn boss_round(g: &mut Game, x: f32, y: f32, angle: f32) {
    let (s, c) = angle.sin_cos();
    pool_spawn(&mut g.enemy_bullets, Bullet {
        x, y, vx: s * ENEMY_BULLET_SPEED * 0.95, vy: c * ENEMY_BULLET_SPEED * 0.95, active: true,
    });
}

/// Rounds per turret burst: more every other level.
fn boss_burst_len(tier: usize) -> u8 { 2 + tier as u8 / 2 }

/// Half the width of the lane a boss's spray leaves open (see lane_x).
const LANE_HALF: f32 = 46.0;

/// Where the open lane through a boss's spray is, at the player's height:
/// drifting slowly from side to side so the way through moves, but never
/// faster than the plane can follow.
fn lane_x(t: f32) -> f32 {
    WIN_W as f32 / 2.0 + (t * 0.35).sin() * (WIN_W as f32 * 0.32)
}

/// Seconds between one turret's bursts: steady for a boss, so its rhythm
/// can be learnt. The guns share the old volley rate between them.
fn boss_burst_gap(g: &Game) -> f32 {
    let spec = &BOSS_SPECS[g.boss.tier];
    let hp_frac = g.boss.hp as f32 / g.boss.max_hp as f32;
    // The largest boss enrages below half health: its gunners fire twice as often.
    let enraged = g.boss.tier == BOSS_SPECS.len() - 1 && hp_frac <= 0.5;
    let gap = (spec.fire_min + spec.fire_max) / 2.0 * (1.0 + g.boss.n_turrets as f32 * 0.09);
    if enraged { gap * 0.5 } else { gap }
}

/// Traverse, fire and reload every turret. Traverse is rate-limited (about
/// 100 degrees a second), so a fast-moving player can outrun the gunners.
fn update_turrets(g: &mut Game, dt: f32) {
    let spec = &BOSS_SPECS[g.boss.tier];
    let pattern = spec.patterns[(g.boss.t / PATTERN_SECS) as usize % spec.patterns.len()];
    let (px, py) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0);
    let r = turret_radius(g.boss.tier);
    let gap = boss_burst_gap(g);
    let len = boss_burst_len(g.boss.tier);
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
        let mut want = match pattern {
            BossPattern::Aimed => aim,
            BossPattern::Fan => aim * 0.5 + t.fx * 0.6,
            BossPattern::Sweep => (bt * 0.8 + k as f32 * 0.7).sin() * 0.9,
            BossPattern::Curtain => t.fx * 1.1 + (bt * 1.3).sin() * 0.3,
        };
        if pattern != BossPattern::Aimed {
            // The spray keeps a lane open at the player's height: a gun whose
            // rounds would cross inside it swings to the lane's nearer edge.
            let lane = lane_x(bt);
            let cross = x + want.tan() * (py - y).max(1.0);
            if (cross - lane).abs() < LANE_HALF {
                let edge = lane + LANE_HALF * if cross < lane { -1.0 } else { 1.0 };
                want = (edge - x).atan2((py - y).max(1.0));
            }
        }
        let err = (want - t.angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        t.angle += err.clamp(-1.75 * dt, 1.75 * dt);
        t.flash = (t.flash - dt).max(0.0);
        if t.reload_t > 0.0 {
            t.reload_t -= dt;
            if t.reload_t <= 0.0 { t.ammo = len * DRUM_BURSTS; }
            continue;
        }
        t.next_t -= dt;
        // A burst starts on the beat, and only with the barrels on target.
        if t.burst == 0 && t.next_t <= 0.0 && err.abs() < ON_TARGET {
            t.burst = len.min(t.ammo);
            t.burst_t = 0.0;
            t.next_t = gap;
        }
        if t.burst > 0 {
            t.burst_t -= dt;
            if t.burst_t <= 0.0 {
                t.burst -= 1;
                t.ammo -= 1;
                t.burst_t = ROUND_GAP;
                t.flash = 0.05;
                let (s, c) = t.angle.sin_cos();
                // twin barrels, firing alternately, straight along them
                let side = if t.burst % 2 == 0 { 1.0 } else { -1.0 };
                let (ox, oy) = (c * 1.8 * side, -s * 1.8 * side);
                let a = t.angle;
                if t.ammo == 0 { t.reload_t = RELOAD_SECS; t.burst = 0; }
                boss_round(g, x + s * r * 2.6 + ox, y + c * r * 2.6 + oy, a);
            }
        }
    }
}

fn update_boss(g: &mut Game, dt: f32, sfx: &Sounds) {
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

    update_turrets(g, dt);
    update_lasers(g, dt, sfx);
    if spec.escorts && g.boss.escort_timer.tick(dt) {
        spawn_dive_pass(g, EnemyKind::Grunt, Formation::Pair);
        g.boss.escort_timer.start(5.0 + rand01() * 2.5);
    }
}

fn update_title(g: &mut Game) {
    if btn1_pressed() {
        web::spend_coin();
        g.start_game();
    }
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
        // RAIDER_HP=1..5 starts damaged, RAIDER_BARRIER=1 brings a barrier in,
        // RAIDER_ISLAND=0..2 an island of that size, RAIDER_LEVEL=1..7 starts
        // that level's waves.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<i32>().ok());
            if let Some(n) = env("RAIDER_BOSS") {
                g.sess.level = n.clamp(1, BOSS_SPECS.len() as i32);
                g.wave_kills = g.wave_target;
                // the last boss is met inland, as in a full run
                if g.sess.level == MAX_LEVEL {
                    g.batteries = land::lay_batteries();
                    g.land_dist = land::COAST_DIST + 1400.0;
                }
            }
            if let Some(hp) = env("RAIDER_HP") { g.health = hp.clamp(1, PLAYER_HEALTH_MAX); }
            if let Some(n) = env("RAIDER_ISLAND") { spawn_island_sized(g, n.clamp(0, 2) as u8); }
            if let Some(n) = env("RAIDER_LEVEL") {
                g.sess.level = n.clamp(1, MAX_LEVEL);
                g.wave_target = wave_target_for(g.sess.level);
                if g.sess.level == MAX_LEVEL { g.batteries = land::lay_batteries(); }
            }
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
    g.fx.update(dt);
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
        if g.engine_out() {
            // No thrust: the plane sheds speed whatever the throttle, but
            // a splutter never stalls it on its own.
            g.airspeed = (g.airspeed - SPLUTTER_DRAG * dt).max(g.airspeed.min(SPLUTTER_FLOOR));
        } else if throttle != 0.0 {
            g.airspeed += throttle * AIRSPEED_RATE * dt;
        } else {
            g.airspeed += (1.0 - g.airspeed).clamp(-AIRSPEED_RELAX * dt, AIRSPEED_RELAX * dt);
        }
        g.airspeed = g.airspeed.clamp(0.0, AIRSPEED_MAX);
        update_splutter(g, dt, sfx);
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
    g.prop_angle = (g.prop_angle + blip::lerp(40.0, 7.0, glide(g)) * dt) % std::f32::consts::TAU;

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
    update_boss(g, dt, sfx);
    update_islands(g, dt, sfx);
    land::update_batteries(g, dt, sfx);
    update_flak(g, dt, sfx);
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
            // a round on a laser turret damages it; enough and it is out
            let lr = laser_radius(g.boss.tier) + 3.0;
            for i in 0..g.boss.n_lasers {
                if g.boss.lasers[i].hp <= 0 { continue; }
                let (lx, ly) = mount_pos(g, g.boss.lasers[i].fx, g.boss.lasers[i].fy);
                if (lx - bx).hypot(ly - by) < lr {
                    g.boss.lasers[i].hp -= 1;
                    if g.boss.lasers[i].hp <= 0 {
                        g.spawn_explosion(lx, ly, 1.4, EXPLOSION_ORANGE);
                        g.score_at(lx, ly, 150 * g.sess.level);
                        play_take(&sfx.enemy_explode, 0.9);
                    }
                }
            }
            // a round on a turret damages the turret too; enough and it is out
            let r = turret_radius(g.boss.tier) + 3.0;
            for k in 0..g.boss.n_turrets {
                if g.boss.turrets[k].hp <= 0 { continue; }
                let (tx, ty) = turret_pos(g, k);
                if (tx - bx).hypot(ty - by) < r {
                    g.boss.turrets[k].hp -= 1;
                    if g.boss.turrets[k].hp <= 0 {
                        g.spawn_explosion(tx, ty, 1.0, EXPLOSION_ORANGE);
                        g.score_at(tx, ty, 50 * g.sess.level);
                        play_take(&sfx.enemy_explode, 0.7);
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
                g.score_at(cx, cy, 500 * g.sess.level);
                play_take(&sfx.boss_explode, 1.0);
                if g.sess.level >= MAX_LEVEL {
                    // The final boss is down — the game is won.
                    g.sess.add_score(2000);
                    blip::bot::set("score", g.sess.score as f64);
                    blip::bot::finish("won");
                    play_sfx(&sfx.victory);
                    g.state = State::Won;
                } else {
                    blip::bot::set(&format!("t_boss{}", g.sess.level), blip::bot::clock() as f64);
                    g.sess.next_level();
                    blip::bot::set("level", g.sess.level as f64);
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
                    g.score_at(boat_x + BOAT_W as f32 / 2.0, boat_y, 40 * g.sess.level);
                    play_take(&sfx.enemy_explode, 1.0);
                    break;
                }
            }
        }

        // Flak batteries on land.
        if g.bullets[bi].active && land::hit_battery(g, bx + bw / 2.0, by, sfx) {
            g.bullets[bi].active = false;
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
                        g.score_at(cx, cy, ISLAND_SCORE[g.islands[isi].size as usize]);
                        play_take(&sfx.boss_explode, 1.0);
                    } else {
                        play_take(&sfx.enemy_explode, 1.0);
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
                    let (mx, my) = (g.barrier.motor_x, g.barrier.y);
                    g.score_at(mx, my, 300 * g.sess.level);
                    play_take(&sfx.boss_explode, 1.0);
                } else {
                    play_take(&sfx.enemy_explode, 1.0);
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
    // The aircraft, not its box: the fuselage and inner wings.
    if g.state == State::Play && !g.respawn_grace.active() && g.stall_fall == 0.0 {
        let (px, py) = (g.player_x, g.player_y);
        let (hw, hh) = PLAYER_HIT;
        let (hx, hy) = (px + (PLAYER_W as f32 - hw) / 2.0, py + PLAYER_H as f32 * 0.47 - hh / 2.0);
        let mut hit = false;
        for i in 0..MAX_ENEMY_BULLETS {
            if !g.enemy_bullets[i].active { continue; }
            if rects_overlap(hx, hy, hw, hh, g.enemy_bullets[i].x - 2.0, g.enemy_bullets[i].y - 2.0, 4.0, 4.0) {
                g.enemy_bullets[i].active = false;
                blip::bot::add(if g.boss.active { "hit_boss_round" } else { "hit_round" }, 1.0);
                hit = true;
            }
        }
        for b in pool_iter_mut(&mut g.flak_bursts) {
            // a burst hurts once, in its first moments, if the plane is in it
            if b.t < FLAK_LETHAL && (b.x - (hx + hw / 2.0)).hypot(b.y - (hy + hh / 2.0)) < FLAK_BURST_R {
                b.t = FLAK_LETHAL;
                blip::bot::add("hit_flak", 1.0);
                hit = true;
            }
        }
        for i in 0..MAX_ENEMIES {
            if !g.enemies[i].active { continue; }
            let (ew, eh) = (ENEMY_W as f32 * 0.6, ENEMY_H as f32 * 0.6);
            if rects_overlap(hx, hy, hw, hh, g.enemies[i].x + ENEMY_W as f32 * 0.2, g.enemies[i].y + ENEMY_H as f32 * 0.2, ew, eh) {
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
        if g.barrier.active && !g.barrier.warmup.active()
            && rects_overlap(hx, hy, hw, hh,
                0.0, g.barrier.y - BARRIER_BEAM_H / 2.0, WIN_W as f32, BARRIER_BEAM_H)
        {
            blip::bot::add("hit_barrier", 1.0);
            hit = true;
        }
        // A laser beam is no glancing blow.
        if g.boss.active && laser_hits(g, hx, hy, hw, hh) {
            blip::bot::add("hit_laser", 1.0);
            hit = true;
            g.health = 1;
        }
        if hit {
            g.health -= 1;
            // A non-fatal hit drops the gun one tier, never below 2; a fatal
            // one leaves it.
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
                play_take(&sfx.player_explode, 1.0);
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
        pool_spawn(&mut g.wrecks, new_wreck(&e));
        play_take(&sfx.enemy_explode, 0.55);
    } else {
        g.spawn_explosion(ecx, ecy, 1.0, EXPLOSION_ORANGE);
        play_take(&sfx.enemy_explode, 1.0);
    }
    let pts = match e.kind { EnemyKind::Grunt => 20, EnemyKind::Weaver => 30, EnemyKind::Ace => 50 };
    g.score_at(ecx, ecy, pts * g.sess.level);
    g.wave_kills += 1;
    // The whole flight shot down, none got away.
    let f = &mut g.flights[e.flight];
    f.downed += 1;
    if f.members >= 2 && f.downed == f.members {
        let bonus = FLIGHT_BONUS * f.members as i32 * g.sess.level;
        g.sess.add_score(bonus);
        g.fx.popup(ecx, ecy - 30.0, &format!("FLIGHT +{bonus}"), BLIP_YELLOW);
        blip::bot::add("flights_down", 1.0);
    }
    if e.kind == EnemyKind::Ace {
        pool_spawn(&mut g.powerups, Powerup { x: e.x, y: e.y, active: true });
    } else if rand01() < HEALTH_DROP_CHANCE {
        pool_spawn(&mut g.health_pickups, HealthPickup { x: e.x, y: e.y, active: true });
    }
}

/// A burning plane's fall, one of the Fall styles at random. Whatever the
/// style it starts with the plane's own velocity: a wreck keeps flying the
/// way the plane was, and only drag and the turn it falls into change that.
fn new_wreck(e: &Enemy) -> Wreck {
    // A banked plane falls into a turn that way; a level one either way.
    let side = if e.bank.abs() > 0.05 { e.bank.signum() } else if rand01() < 0.5 { -1.0 } else { 1.0 };
    let fall = match rand01() {
        r if r < 0.30 => Fall::Roll,
        r if r < 0.55 => Fall::Spiral,
        r if r < 0.70 => Fall::FlatSpin,
        r if r < 0.85 => Fall::Dive,
        _ => Fall::Tumble,
    };
    let mut w = Wreck { x: e.x, y: e.y, heading: e.heading, kind: e.kind, active: true,
        course: e.heading, speed: e.speed, fall,
        spin: side * (1.0 + rand01() * 3.0), bend: side * rand01() * 0.25,
        wobble: rand01() * 2.0, dur: WRECK_SECS * (0.75 + rand01() * 0.55), ..DEAD_WRECK };
    match fall {
        Fall::Roll => {}
        // a descending turn: `spin` is the turn rate, tightening as it falls
        Fall::Spiral => { w.spin = side * (1.4 + rand01() * 0.8); w.dur *= 1.15; }
        Fall::FlatSpin => { w.spin = side * (7.0 + rand01() * 4.0); w.dur *= 0.8; }
        Fall::Dive => { w.spin = 0.0; w.bend = 0.0; w.wobble = 0.4 + rand01() * 0.5; }
        Fall::Tumble => { w.spin = side * (3.0 + rand01() * 3.0); w.flip_t = 0.2 + rand01() * 0.3; }
    }
    w
}

/// One step of a wreck's fall. The velocity (`course`, `speed`) carries on
/// from the plane; drag takes speed off, the turn bends the course, and the
/// airframe rotates on its own. As it loses height it falls behind with the
/// ground scrolling under the fight.
fn fly_wreck(w: &mut Wreck, scroll: f32, dt: f32) {
    let drag = match w.fall {
        Fall::Dive => -0.15,     // nose down, gravity outruns drag
        Fall::Spiral => 0.1,
        Fall::Roll => 0.35,
        Fall::Tumble => 0.8,
        Fall::FlatSpin => 1.3,   // a flat spin bleeds speed fast
    };
    w.speed *= 1.0 - drag * dt;
    match w.fall {
        Fall::Spiral => {
            // the turn tightens as the spiral winds down; the nose follows the path
            w.spin *= 1.0 + 0.45 * dt;
            w.course += w.spin * dt;
            w.heading = w.course + w.spin.signum() * 0.25;
        }
        Fall::Dive => {
            w.heading = w.course + (w.t * 6.0).sin() * 0.08 * w.wobble;
        }
        Fall::Tumble => {
            w.flip_t -= dt;
            if w.flip_t <= 0.0 {
                w.spin = -w.spin * (0.7 + rand01() * 0.6);
                w.flip_t = 0.15 + rand01() * 0.35;
            }
            w.heading += w.spin * dt;
            w.course += w.bend * dt;
        }
        Fall::Roll | Fall::FlatSpin => {
            w.spin *= 1.0 + 0.8 * dt; // the rotation tightens as it falls
            w.heading += (w.spin + (w.t * 5.0).sin() * w.wobble) * dt;
            w.course += w.bend * dt;
        }
    }
    // height lost, as a share of the fall: the lower, the more it lags with the ground
    let low = (w.t / w.dur).min(1.0);
    w.x += w.course.sin() * w.speed * dt;
    w.y += w.course.cos() * w.speed * dt + SEA_SCROLL_SPEED * scroll * low * dt;
}

fn update_wrecks(g: &mut Game, dt: f32, sfx: &Sounds) {
    for i in 0..MAX_WRECKS {
        if !g.wrecks[i].active { continue; }
        let scroll = g.scroll_k;
        let w = &mut g.wrecks[i];
        w.t += dt;
        fly_wreck(w, scroll, dt);
        w.puff_t -= dt;
        let (cx, cy, k) = (w.x + ENEMY_W as f32 / 2.0, w.y + ENEMY_H as f32 / 2.0, w.t / w.dur);
        let (emit, done) = (w.puff_t <= 0.0, w.t >= w.dur);
        // a diving plane burns hardest, trailing the thickest smoke
        if emit { w.puff_t = if w.fall == Fall::Dive { 0.022 } else { 0.035 }; }
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
            // the splash where it goes in, or the fireball over land
            let land = land::over_land(g, cx, cy);
            let c = if land { EXPLOSION_ORANGE } else { BlipColor::new(0.85, 0.93, 1.0, 1.0) };
            g.spawn_explosion(cx, cy, if land { 1.2 } else { 0.8 }, c);
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
        play_take(&sfx.player_explode, 1.0);
        g.health = 0;
        blip::bot::add("death_stall", 1.0);
        match g.sess.lose_life() {
            LifeResult::StillAlive => { g.dead_timer.start(DEAD_PAUSE); g.state = State::Dead; }
            LifeResult::GameOver   => { g.over_timer.start(OVER_MIN_WAIT); g.state = State::Over; }
        }
    }
}

/// Engine loop volumes for the current airspeed: a crossfade between the
/// two loops either side of it, louder the faster it turns, the coughing
/// low loop pushed up near a stall, and the engine dying through the fall.
/// Now and then low-grade fuel starves the engine: it misses, cuts out for
/// a second with a cough or two, and catches (blip_assets::sky_raider's
/// engine_splutter_sfx carries the sound; the loops are ducked meanwhile).
fn update_splutter(g: &mut Game, dt: f32, sfx: &Sounds) {
    if g.splutter_t < 0.0 {
        g.splutter_next -= dt;
        // Not near a stall or in a boss's entrance, where it would be unfair.
        if g.splutter_next <= 0.0 && g.airspeed > 0.7 && !g.boss_intro.active() {
            g.splutter_t = 0.0;
            g.splutter_next = SPLUTTER_EVERY.0 + rand01() * (SPLUTTER_EVERY.1 - SPLUTTER_EVERY.0);
            play_sfx_volume(&sfx.engine_splutter, PROP_MAX_VOLUME * 1.1);
            blip::bot::add("splutters", 1.0);
        }
        return;
    }
    g.splutter_t += dt;
    if g.engine_out() {
        // dark puffs from the exhaust stacks either side of the nose
        g.smoke_t -= dt;
        if g.smoke_t <= 0.0 {
            g.smoke_t = 0.09;
            for side in [-1.0f32, 1.0] {
                let x = g.player_x + PLAYER_W as f32 / 2.0 + side * 4.0;
                pool_spawn(&mut g.puffs, Puff { x, y: g.player_y + PLAYER_H as f32 * 0.22, r: 2.0, grow: 16.0,
                    ttl: 0.7, max_ttl: 0.7, fire: false, top: true, active: true });
            }
        }
    }
    if g.splutter_t >= SPLUTTER_SECS { g.splutter_t = -1.0; }
}

/// How much of the engine loops plays through a splutter: fading out as it
/// dies, silent while it is out, back as it catches.
fn splutter_duck(g: &Game) -> f32 {
    let t = g.splutter_t;
    if t < 0.0 { 1.0 }
    else if t < SPLUTTER_DIES { 1.0 - t / SPLUTTER_DIES }
    else if t < SPLUTTER_CATCHES { 0.0 }
    else { ((t - SPLUTTER_CATCHES) / 0.25).min(1.0) }
}

/// One of several takes of a sound, at a slightly varied volume, so a run
/// of the same event never repeats exactly.
fn play_take(takes: &[blip::BlipSound], vol: f32) {
    let i = ((rand01() * takes.len() as f32) as usize).min(takes.len() - 1);
    play_sfx_volume(&takes[i], vol * (0.85 + 0.15 * rand01()));
}

async fn load_takes(wavs: &[&[u8]]) -> Vec<blip::BlipSound> {
    let mut v = Vec::with_capacity(wavs.len());
    for w in wavs { v.push(blip::audio::load_sound(w).await); }
    v
}

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
    let duck = splutter_duck(g);
    w.map(|v| v * loud * dying * duck)
}

fn update_dead(g: &mut Game, dt: f32) {
    g.fx.update(dt);
    if g.dead_timer.tick(dt) { g.respawn(); }
}

fn update_win(g: &mut Game, dt: f32) {
    g.fx.update(dt);
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

/// The ocean: a gradient lighter and hazier toward the top (distance), two
/// octaves of wave lines (swell and chop) scrolling with `sea_scroll`, sun
/// glitter in a diagonal band, and each cloud's shadow drifting on the water.
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

/// The sprite again, dark and translucent, offset toward the sea
/// (PLANE_SHADOW_DX / DY). Drawn after sea and clouds, before the entity.
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

/// The sky layer, between sea and planes: noise-generated cumulus
/// (blip_assets cloud_sprite()) in 3 variants.
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

/// Is (x, y), an entity's centre, inside a cloud's visible body? An ellipse
/// shrunk a little, so a plane must be well under it to count.
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
        for k in 0..FLAK_GUNS[isl.size as usize].len() {
            draw_flak_gun(blip, isl, k);
        }
        let max_hp = ISLAND_HP[isl.size as usize];
        if isl.hp < max_hp {
            let frac = (isl.hp as f32 / max_hp as f32).clamp(0.0, 1.0);
            blip.draw_rect(isl.x, isl.y - 6.0, w as f32, 3.0, BLIP_GRAY);
            blip.fill_rect(isl.x, isl.y - 6.0, w as f32 * frac, 3.0, BLIP_RED);
        }
    }
}

/// A flak emplacement: a ring of sandbags round a dark gun, its long barrel
/// on the player.
fn draw_flak_gun(blip: &Blip, isl: &Island, k: usize) {
    let (x, y) = flak_gun_pos(isl, k);
    draw_flak_pit(blip, x, y, &isl.guns[k]);
}

/// A flak gun in its pit at (`x`, `y`), wherever it stands.
fn draw_flak_pit(blip: &Blip, x: f32, y: f32, gun: &FlakGun) {
    blip.fill_circle(x + 1.0, y + 1.5, 8.5, BlipColor::new(0.0, 0.0, 0.0, 0.3));
    for b in 0..10 {
        let a = b as f32 / 10.0 * std::f32::consts::TAU;
        let shade = if b % 2 == 0 { 0.62 } else { 0.54 };
        blip.fill_circle(x + a.cos() * 6.5, y + a.sin() * 6.5, 2.6, BlipColor::new(shade, shade * 0.88, shade * 0.6, 1.0));
    }
    blip.fill_circle(x, y, 4.5, BlipColor::new(0.2, 0.22, 0.18, 1.0));
    let (s, c) = gun.angle.sin_cos();
    blip.draw_line_ex(x, y, x + s * 11.0, y + c * 11.0, 2.4, BlipColor::new(0.08, 0.08, 0.09, 1.0));
    blip.fill_circle(x, y, 2.4, BlipColor::new(0.32, 0.34, 0.3, 1.0));
    // the flash as it fires
    if gun.flash > 0.0 {
        blip.fill_glow_circle(x + s * 13.0, y + c * 13.0, 3.0, BlipColor::new(1.0, 0.85, 0.45, 1.0));
    }
}

/// Flak bursts: a bright flash and a hard black ball while it can hurt,
/// then a soft grey cloud that drifts back and fades.
fn draw_flak_bursts(blip: &Blip, g: &Game) {
    for b in pool_iter(&g.flak_bursts) {
        if b.t < FLAK_LETHAL {
            let k = b.t / FLAK_LETHAL;
            let r = FLAK_BURST_R * (0.45 + 0.55 * k);
            blip.fill_circle(b.x, b.y, r, BlipColor::new(0.08, 0.07, 0.07, 0.9));
            blip.fill_glow_circle(b.x, b.y, r * 0.45 * (1.0 - k), BlipColor::new(1.0, 0.62, 0.25, 1.0 - k));
        } else {
            let k = (b.t - FLAK_LETHAL) / FLAK_SMOKE;
            let r = FLAK_BURST_R * (1.0 + 0.5 * k);
            let a = 0.55 * (1.0 - k);
            for (dx, dy, rr) in [(0.0, 0.0, 1.0), (-0.4, -0.25, 0.7), (0.45, -0.15, 0.65), (0.1, 0.4, 0.6)] {
                blip.fill_circle(b.x + dx * r, b.y + dy * r, r * rr * 0.75, BlipColor::new(0.18, 0.17, 0.17, a));
            }
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

/// An aircraft sprite, nose along `heading`, narrower as it banks (a banked
/// wing is foreshortened from above). Flight moves along (sin h, cos h) and
/// rotating the nose-down sprite by r points it at (-sin r, cos r), hence the
/// minus.
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
    land::draw_land(blip, g);
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
    land::draw_land(blip, g);
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
        // Some planes vanish under a cloud they fly through: draw order only;
        // they keep flying, firing and can be hit.
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
        for k in 0..g.boss.n_turrets {
            draw_turret(blip, g, k);
        }
        for i in 0..g.boss.n_lasers {
            draw_laser(blip, g, i);
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
            blip.draw_centered("LASER BARRIER", by + 22.0, 2.0, color);
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
    for s in pool_iter(&g.flak) {
        draw_slug(blip, s.x, s.y, s.vx, s.vy, 8.0, 3.2, BlipColor::new(0.62, 0.58, 0.5, 1.0));
    }
    draw_flak_bursts(blip, g);
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
            // Slow flight is a glide, not a reverse: the camera overtakes a
            // plane riding the wind, wings rocking, engine idling.
            let gl = glide(g);
            let sway = (tt * 1.7).sin() * 0.07 * gl;
            let by = by + (tt * 1.3).sin() * 1.5 * gl;
            draw_glide_wind(blip, g, gl, tt);
            draw_texture_ex(player_tex, g.player_x + bx, g.player_y + by, BLIP_WHITE, DrawTextureParams {
                dest_size: Some(vec2(PLAYER_W as f32, PLAYER_H as f32)),
                rotation: g.player_bank + sway,
                ..Default::default()
            });
            draw_idle_prop(blip, g, g.player_x + bx, g.player_y + by, g.player_bank + sway, gl);
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

    g.fx.draw(blip);
    blip.draw_hud(g.sess.score, g.sess.lives);
    draw_bottom_hud(blip, g, player_tex);

    if g.max_power_banner.active() {
        let flash = ((g.max_power_banner.remaining() * 14.0) as i32 % 2) == 0;
        let color = if flash { weapon_tier_color(MAX_WEAPON_LEVEL) } else { BLIP_WHITE };
        blip.draw_centered("MAXIMUM POWER", (WIN_H / 2 - 12) as f32, 4.0, color);
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

/// The corner readouts: a mini plane and the lives bottom-left, the health
/// bar bottom-right, where a glance mid-fight lands.
/// 0 at cruise, rising to 1 as the throttle closes to the stall warning.
fn glide(g: &Game) -> f32 {
    ((1.0 - g.airspeed) / (1.0 - STALL_WARN)).clamp(0.0, 1.0)
}

/// Air still streaming past the wingtips while the plane falls behind the
/// camera: it is flying forward through the air, only slower than the scroll.
fn draw_glide_wind(blip: &Blip, g: &Game, gl: f32, tt: f32) {
    if gl <= 0.05 { return; }
    let (cx, top) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y);
    const RUN: f32 = PLAYER_H as f32 + 8.0; // how far a wisp travels, nose to past the tail
    for side in [-1.0f32, 1.0] {
        for i in 0..3 {
            let k = ((tt * 130.0 + i as f32 * 19.0 + if side > 0.0 { 9.0 } else { 0.0 }) % RUN) / RUN;
            let x = cx + side * (PLAYER_W as f32 * 0.47 + 2.0);
            let y = top + 6.0 + k * RUN;
            let a = gl * 0.7 * (1.0 - k) * (k * 6.0).min(1.0);
            blip.draw_line_ex(x, y, x, y + 14.0, 1.6, BlipColor::new(0.88, 0.94, 1.0, a));
        }
    }
}

/// The propeller turning slowly enough to see its blades, over the sprite's
/// blur disc, at the nose of a plane drawn at (`x`, `y`) rotated `rot`.
fn draw_idle_prop(blip: &Blip, g: &Game, x: f32, y: f32, rot: f32, gl: f32) {
    if gl <= 0.1 { return; }
    // In the sprite's 36-unit design the hub is 1 unit below the top edge and
    // a blade 6.3 units long (see blip_assets::sky_raider::player_plane).
    let s = PLAYER_W as f32 / 36.0;
    let (hw, hh) = (PLAYER_W as f32 / 2.0, PLAYER_H as f32 / 2.0);
    let d = hh - 1.0 * s;
    let (hx, hy) = (x + hw + d * rot.sin(), y + hh - d * rot.cos());
    // Seen from above, a blade's length shows as it swings across the nose.
    let reach = 6.3 * s * g.prop_angle.cos();
    let (dx, dy) = (reach * rot.cos(), reach * rot.sin());
    let c = BlipColor::new(0.08, 0.08, 0.09, ((gl - 0.1) / 0.4).min(1.0));
    blip.draw_line_ex(hx - dx, hy - dy, hx + dx, hy + dy, 2.6, c);
}

/// A boss turret, readable at a glance: a steel ring and glazed dome, twin
/// barrels on the aim, a faint line of fire ahead of a loaded gun that
/// brightens as its next burst comes due, the muzzle flash, and an amber
/// ring filling while the gunner changes the drum.
fn draw_turret(blip: &Blip, g: &Game, k: usize) {
    let t = &g.boss.turrets[k];
    let r = turret_radius(g.boss.tier);
    let (x, y) = turret_pos(g, k);
    blip.fill_circle(x + 1.0, y + 1.5, r + 1.2, BlipColor::new(0.0, 0.0, 0.0, 0.35));
    blip.fill_circle(x, y, r + 1.2, BlipColor::new(0.08, 0.08, 0.09, 1.0));
    blip.fill_circle(x, y, r + 0.4, BlipColor::new(0.52, 0.55, 0.58, 1.0));
    if t.hp <= 0 {
        blip.fill_circle(x, y, r * 0.8, BlipColor::new(0.08, 0.07, 0.06, 1.0));
        return;
    }
    let (s, c) = t.angle.sin_cos();
    let (ox, oy) = (c * 1.9, -s * 1.9);
    let len = r * 2.6;
    let reloading = t.reload_t > 0.0;
    if !reloading {
        // The line of fire: where the next rounds will go.
        let due = if t.burst > 0 { 1.0 } else { (1.0 - t.next_t / 0.5).clamp(0.0, 1.0) };
        let a = 0.10 + 0.35 * due;
        let (mx, my) = (x + s * len, y + c * len);
        for d in 0..9 {
            let (d0, d1) = (6.0 + d as f32 * 12.0, 12.0 + d as f32 * 12.0);
            let fade = 1.0 - d as f32 / 9.0;
            blip.draw_line_ex(mx + s * d0, my + c * d0, mx + s * d1, my + c * d1, 1.2,
                BlipColor::new(1.0, 0.55, 0.3, a * fade));
        }
    }
    for side in [-1.0f32, 1.0] {
        let (bx, by) = (x + ox * side, y + oy * side);
        blip.draw_line_ex(bx, by, bx + s * len, by + c * len, 2.0, BlipColor::new(0.06, 0.06, 0.07, 1.0));
    }
    if t.flash > 0.0 {
        blip.fill_glow_circle(x + s * (len + 2.0), y + c * (len + 2.0), 3.0, BlipColor::new(1.0, 0.85, 0.45, 1.0));
    }
    blip.fill_circle(x, y, r * 0.78, BlipColor::new(0.36, 0.42, 0.34, 1.0));
    blip.fill_circle(x - s * 1.5, y - c * 1.5, r * 0.5, BlipColor::new(0.55, 0.68, 0.72, 0.9));
    blip.fill_circle(x - r * 0.25, y - r * 0.3, r * 0.18, BlipColor::new(0.95, 1.0, 1.0, 0.8));
    if reloading {
        // the drum going on: an amber ring filling round the turret
        let done = 1.0 - t.reload_t / RELOAD_SECS;
        let dots = 14;
        for d in 0..dots {
            let lit = (d as f32 / dots as f32) < done;
            let a = d as f32 / dots as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            blip.fill_circle(x + a.cos() * (r + 3.2), y + a.sin() * (r + 3.2), 1.1,
                if lit { BlipColor::new(1.0, 0.75, 0.2, 1.0) } else { BlipColor::new(0.3, 0.25, 0.15, 0.7) });
        }
    }
}

/// A laser turret: a bigger dark dome with a single heavy emitter. While it
/// charges, a thin line shows exactly where the beam will go, pulsing
/// faster as it nears firing; the beam itself is a white core in red glow.
fn draw_laser(blip: &Blip, g: &Game, i: usize) {
    let l = &g.boss.lasers[i];
    let r = laser_radius(g.boss.tier);
    let (x, y) = mount_pos(g, l.fx, l.fy);
    blip.fill_circle(x + 1.0, y + 1.5, r + 1.2, BlipColor::new(0.0, 0.0, 0.0, 0.35));
    blip.fill_circle(x, y, r + 1.2, BlipColor::new(0.08, 0.08, 0.09, 1.0));
    blip.fill_circle(x, y, r + 0.4, BlipColor::new(0.45, 0.2, 0.2, 1.0));
    if l.hp <= 0 {
        blip.fill_circle(x, y, r * 0.8, BlipColor::new(0.08, 0.07, 0.06, 1.0));
        return;
    }
    let (s, c) = l.angle.sin_cos();
    let len = r * 2.4;
    let (mx, my) = (x + s * len, y + c * len);
    let far = 900.0;
    match l.phase {
        LaserPhase::Charging => {
            let k = 1.0 - l.t / LASER_CHARGE;
            let pulse = 0.5 + 0.5 * (g.boss.t * (8.0 + 22.0 * k)).sin();
            blip.draw_line_ex(mx, my, mx + s * far, my + c * far, 1.0 + k,
                BlipColor::new(1.0, 0.2, 0.2, 0.25 + 0.45 * k * pulse));
            blip.fill_glow_circle(mx, my, 1.5 + 3.0 * k, BlipColor::new(1.0, 0.3, 0.3, 0.6 + 0.4 * k));
        }
        LaserPhase::Firing => {
            let flick = 0.85 + 0.15 * (g.boss.t * 60.0).sin();
            blip.draw_line_ex(mx, my, mx + s * far, my + c * far, LASER_HALF_W * 2.8, BlipColor::new(1.0, 0.15, 0.15, 0.35 * flick));
            blip.draw_line_ex(mx, my, mx + s * far, my + c * far, LASER_HALF_W * 1.6, BlipColor::new(1.0, 0.3, 0.25, 0.8 * flick));
            blip.draw_line_ex(mx, my, mx + s * far, my + c * far, LASER_HALF_W * 0.6, BlipColor::new(1.0, 0.95, 0.9, 1.0));
            blip.fill_glow_circle(mx, my, 5.0, BlipColor::new(1.0, 0.6, 0.5, 1.0));
        }
        LaserPhase::Cooling => {}
    }
    blip.draw_line_ex(x, y, mx, my, 4.0, BlipColor::new(0.06, 0.06, 0.07, 1.0));
    blip.fill_circle(x, y, r * 0.75, BlipColor::new(0.22, 0.1, 0.1, 1.0));
    let glow = if l.phase == LaserPhase::Cooling { 0.35 } else { 1.0 };
    blip.fill_circle(x, y, r * 0.35, BlipColor::new(1.0, 0.25, 0.2, glow));
}

fn draw_bottom_hud(blip: &Blip, g: &Game, player_tex: &Texture2D) {
    let y = (WIN_H - 30) as f32;
    // Kept in from the edges, clear of the curved glass's rim.
    let corner_margin = 24.0;

    // Lives, bottom-left.
    let mini_w = PLAYER_W as f32 * 0.6;
    let mini_h = PLAYER_H as f32 * 0.6;
    blip.draw_texture(player_tex, corner_margin, y, mini_w, mini_h);
    // Outlined, and the bar backed: white on a passing cloud was unreadable.
    let ink = BlipColor::new(0.0, 0.05, 0.15, 0.8);
    blip.draw_text_outlined(&format!("x{}", g.sess.lives.max(0)), corner_margin + mini_w + 3.0, y + 3.0, 2.0, BLIP_WHITE, ink);

    // Health, bottom-right: empties with hits, full again each life; pickups
    // restore some.
    let bar_w = 64.0;
    let bar_h = 9.0;
    let bar_x = (WIN_W as f32) - bar_w - corner_margin;
    let bar_y = y + (mini_h - bar_h) / 2.0;
    let hp_frac = (g.health as f32 / PLAYER_HEALTH_MAX as f32).clamp(0.0, 1.0);
    let hp_color = if hp_frac > 0.6 { BLIP_GREEN } else if hp_frac > 0.3 { BLIP_YELLOW } else { BLIP_RED };
    blip.draw_text_outlined("HP", bar_x - 26.0, y + 3.0, 2.0, BLIP_WHITE, ink);
    blip.fill_rect(bar_x - 1.0, bar_y - 1.0, bar_w + 2.0, bar_h + 2.0, ink);
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
    blip.draw_centered("ALL 7 WAVES CLEARED", (WIN_H / 2 - 20) as f32, 3.0, BLIP_GREEN);
    blip.draw_centered(&buf,                  (WIN_H / 2 + 14) as f32, 3.0, BLIP_WHITE);
    blip.draw_best(score, hi, (WIN_H / 2 + 40) as f32, BLIP_GREEN);
    blip.draw_centered("PRESS FIRE",          (WIN_H * 2 / 3) as f32,  3.0, BLIP_CYAN);
}

fn draw_over(blip: &Blip, score: i32, hi: &web::HighScore, waiting: bool) {
    blip.clear(BLIP_BLACK);
    blip.draw_game_over(score, hi, BLIP_RED, BLIP_GREEN, BLIP_YELLOW, !waiting);
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
const ENEMY_EXPLODE_WAV: [&[u8]; 4] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/enemy_explode0.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/enemy_explode1.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/enemy_explode2.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/enemy_explode3.wav")),
];
const PLAYER_EXPLODE_WAV: [&[u8]; 2] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/player_explode0.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/player_explode1.wav")),
];
const PLAYER_HIT_WAV:     &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/player_hit.wav"));
const RICOCHET_WAV: [&[u8]; 2] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/ricochet1.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/ricochet2.wav")),
];
const BOSS_EXPLODE_WAV: [&[u8]; 2] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/boss_explode0.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/boss_explode1.wav")),
];
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
const FLAK_BURST_WAV:     &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/flak_burst.wav"));
const LASER_CHARGE_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/laser_charge.wav"));
const LASER_FIRE_WAV:     &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/laser_fire.wav"));
const BARRIER_HUM_WAV:    &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/barrier_hum.wav"));
const BARRIER_HUM2_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/barrier_hum2.wav"));
const ENGINE_START_WAV:   &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/engine_start.wav"));
const ENGINE_SPLUTTER_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/engine_splutter.wav"));
const ENEMY_GUN_WAV:      &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/enemy_gun.wav"));
const BACKFIRE_WAV:       &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/backfire.wav"));
const ENGINE_WAV: [&[u8]; 5] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/engine0.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/engine1.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/engine2.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/engine3.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/engine4.wav")),
];




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
        enemy_explode:  load_takes(&ENEMY_EXPLODE_WAV).await.try_into().unwrap(),
        player_explode: load_takes(&PLAYER_EXPLODE_WAV).await.try_into().unwrap(),
        player_hit:     blip::audio::load_sound(PLAYER_HIT_WAV).await,
        ricochet: [
            blip::audio::load_sound(RICOCHET_WAV[0]).await,
            blip::audio::load_sound(RICOCHET_WAV[1]).await,
        ],
        boss_explode:   load_takes(&BOSS_EXPLODE_WAV).await.try_into().unwrap(),
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
        flak_burst:     blip::audio::load_sound(FLAK_BURST_WAV).await,
        laser_charge:   blip::audio::load_sound(LASER_CHARGE_WAV).await,
        laser_fire:     blip::audio::load_sound(LASER_FIRE_WAV).await,
        barrier_hum:    blip::audio::load_sound(BARRIER_HUM_WAV).await,
        barrier_hum2:   blip::audio::load_sound(BARRIER_HUM2_WAV).await,
        engine_start:   blip::audio::load_sound(ENGINE_START_WAV).await,
        engine_splutter: blip::audio::load_sound(ENGINE_SPLUTTER_WAV).await,
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
    // Two loops in rotation (synthesised here, see blip_assets::sky_raider),
    // so a long level doesn't hear the same ~40s on repeat.
    let mut music = Jukebox::new(&[blip_assets::sky_raider::music, blip_assets::sky_raider::music2]);
    music.start(0).await;

    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;

        music.rotate(dt);
        if g.state != State::Play { music.warm_up().await; }

        if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 {
                g.start_game();
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        if blip::bot::active() { bot::drive(&g, blip::bot::clock()); }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_fall_style_ends_in_the_sea_near_where_it_began() {
        // A wreck that flies off the screen or goes NaN is a plane that
        // vanished instead of going in.
        let e = Game::new().enemies[0];
        let e = Enemy { x: 200.0, y: 200.0, heading: 0.3, active: true, ..e };
        let mut seen = Vec::new();
        for _ in 0..400 {
            let mut w = new_wreck(&e);
            if !seen.contains(&(w.fall as u8)) { seen.push(w.fall as u8); }
            while w.t < w.dur {
                w.t += F;
                fly_wreck(&mut w, 1.0, F);
                assert!(w.x.is_finite() && w.y.is_finite() && w.heading.is_finite());
            }
            assert!((w.x - e.x).abs() < 260.0 && (w.y - e.y).abs() < 360.0,
                "a {:?} fall ended at ({:.0}, {:.0})", w.fall as u8, w.x, w.y);
        }
        assert_eq!(seen.len(), 5, "not every fall style turned up");
    }

    #[test]
    fn a_wreck_carries_on_the_way_the_plane_was_flying() {
        // Momentum: the first moments of any fall continue the plane's own
        // velocity, not a new direction picked for the fall.
        let e = Game::new().enemies[0];
        for heading in [0.0f32, 0.8, -1.2, 2.5] {
            let e = Enemy { x: 200.0, y: 200.0, heading, speed: 120.0, active: true, ..e };
            for _ in 0..50 {
                let mut w = new_wreck(&e);
                for _ in 0..6 { w.t += F; fly_wreck(&mut w, 0.0, F); }
                let moved = (w.y - e.y).atan2(w.x - e.x);
                let flew = heading.cos().atan2(heading.sin());
                let off = (moved - flew + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                assert!(off.abs() < 0.25, "a {} fall set off {off:.2} rad from the plane's heading", w.fall as u8);
                assert!(((w.x - e.x).hypot(w.y - e.y) - 120.0 * 6.0 * F).abs() < 3.0, "it did not keep the plane's speed");
            }
        }
    }

    const F: f32 = 1.0 / 60.0;
}
