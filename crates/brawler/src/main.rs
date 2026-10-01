//! Brawler: a one-on-one fighting game in tribute to Street Fighter II. Ten
//! fighters, five stages, a CPU ladder or two players.
//! What it takes seriously, because they are the game:
//! 1. **Attack height.** Every attack is low, mid or overhead, and a block
//! works only at the right height: crouch-blocking eats sweeps and loses to
//! jump-ins, standing is the reverse.
//! 2. **Frame data.** Startup, active and recovery in frames; punishable in
//! proportion to reach and damage. A whiffed sweep hurts, a jab does not.
//! 3. **What you can see.** Poses are built from the same numbers as the
//! hitboxes, so an arm that looks extended is what hits you.

mod draw;
#[cfg(not(target_arch = "wasm32"))]
mod bot;

use blip::macroquad::input::KeyCode;
use blip::input::{key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_D,
    BLIP_KEY_DOWN, BLIP_KEY_F, BLIP_KEY_G, BLIP_KEY_J, BLIP_KEY_K, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W};
use blip::audio::play_sfx_volume;
use blip::{clamp, play_sfx, Jukebox, rand_int, rects_overlap, web,
    window_conf, Blip,
    BlipColor, Session, Timer, BLIP_BLACK, BLIP_WHITE, BLIP_YELLOW};

// ---- stage ---------------------------------------------------------------
const WIN_W: i32 = 640;
const WIN_H: i32 = 400;
const FLOOR_Y: f32 = 330.0;
/// How close a fighter's centre may get to the edge. The wall matters:
/// cornering someone is half of what a round is about, so the box has to
/// be tight enough that a corner is a real place to be.
const WALL_MARGIN: f32 = 46.0;

// ---- timing --------------------------------------------------------------
/// One frame at 60fps. Frame data is written in frames because that is
/// the unit fighting games are argued about in, and converted here once.
const F: f32 = 1.0 / 60.0;
/// Forty-five seconds, not the arcade's sixty: at sixty, rounds against a
/// masher ran the full count and the clock decided them. Good play finishes
/// in twenty to thirty.
const ROUND_SECS: f32 = 45.0;
const ROUNDS_TO_WIN: i32 = 2;

// ---- bodies --------------------------------------------------------------
// The width of a fighter, for being hit and for being drawn: the same number,
// or attacks land on air beside what the player sees. The silhouette is built
// to fill it. These are a full-size fighter's; `Archetype::size` scales them.
const BODY_W: f32 = 30.0;
const STAND_H: f32 = 120.0;
const CROUCH_H: f32 = 74.0;
/// Height of a fighter on their back (the raised knee is the highest point),
/// so a flying kick sails over a prone body.
const PRONE_H: f32 = 34.0;

// ---- physics -------------------------------------------------------------
const GRAVITY: f32 = 1500.0;
const JUMP_VY: f32 = -600.0;
/// The flying kick's arc: lower than a jump and much faster forward.
const FLY_VY: f32 = -372.0;
const FLY_SPEED: f32 = 300.0;
/// The flying kick's landing cost, paid on the ground after the blow lands in
/// the air. One flat cost made it 19 frames minus on hit; connecting buys
/// most of the landing back, being blocked does not.
const FLY_LAND_LAG: f32 = 16.0 * F;
const FLY_HIT_LAG: f32 = 4.0 * F;
/// How long into a jump the flying kick is available: up and kick are never
/// the same frame for a human, and up jumps on the frame it is seen. A kick
/// inside this window levels the jump off into the flying kick.
const FLY_WINDOW: f32 = 6.0 * F;
/// A half-size fighter's lunge, in pixels a second at the first frame: about
/// 30px of ground by the time a kick lands, the reach the size took away.
const LUNGE: f32 = 520.0;
/// A punch within this long of the last one is thrown with the other hand.
const CHAIN_PUNCH: f32 = 0.5;
/// What a guarded laser still costs.
const LASER_CHIP: i32 = 9;
/// A web holds for this long, or until this many blows have landed on it;
/// then no web holds for a while.
const WEB_SECS: f32 = 5.0;
const WEB_HITS: u8 = 2;
const WEB_REST: f32 = 3.0;
/// The turtles called in: how fast they run, how far apart they arrive, and
/// what each does when he gets there. All low, so one guard answers the lot.
const HELPER_RUN: f32 = 340.0;
const HELPER_GAP: f32 = 0.38;
const HELPER_MOVES: [MoveId; 3] = [MoveId::LowKick, MoveId::LowPunch, MoveId::Sweep];
/// How long after a throw the one thrown cannot be thrown again: the time on
/// the floor and a second on their feet.
const THROW_REST: f32 = 2.6;
/// What a small fighter's blows are worth against another as small. At 1.0
/// two turtles settle a round in five to fifteen seconds.
const SMALL_ON_SMALL: f32 = 0.7;
const LASER_SPEED: f32 = 600.0;
/// How fast a flier moves through the air, and how high the feet may go: the
/// head stays under the health bars.
const SOAR: f32 = 170.0;
const CEILING: f32 = 150.0;
/// No air control: the direction held at take-off is the commitment; a
/// steerable jump-in is a guess the defender cannot answer.
const AIR_DRIFT: f32 = 180.0;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Level { Low, Mid, Overhead }

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum MoveId { LowPunch, HighPunch, LowKick, HighKick, Sweep, JumpPunch, JumpKick,
    FlyingKick, Special, Throw }

/// One attack, in frames: `startup` before it can hit, `active` while it can,
/// `recovery` helpless afterwards. The gap between reach and recovery is its
/// personality.
#[derive(Copy, Clone)]
struct MoveData {
    startup: f32,
    active: f32,
    recovery: f32,
    damage: i32,
    /// Frames the defender is locked up for on hit, and on block. Hit
    /// always exceeds block, which is what makes landing one better than
    /// being blocked even when the damage is small.
    hitstun: f32,
    blockstun: f32,
    /// Reach from the fighter's centre, and how high off the floor the
    /// blow lands. Both are read by the drawing code too, so the pose
    /// cannot disagree with the hitbox.
    reach: f32,
    height: f32,
    thickness: f32,
    level: Level,
    knockdown: bool,
}

const fn mv(startup: f32, active: f32, recovery: f32, damage: i32, hitstun: f32, blockstun: f32,
            reach: f32, height: f32, thickness: f32, level: Level, knockdown: bool) -> MoveData {
    MoveData { startup, active, recovery, damage, hitstun, blockstun, reach, height, thickness,
               level, knockdown }
}

/// The move table, read as trades: the jab (4f, 6 damage) wins scrambles and
/// nothing else; the sweep (10f startup, 20 recovery, knockdown) beats a
/// crouch that will not block low and loses to a whiff. Jump attacks are long
/// overheads, answered by hitting the jumper out of the air.
fn move_data(id: MoveId) -> MoveData {
    match id {
        // Four normals in a square: punch or kick, low or high. At each
        // height a fast one to open with and a slow one to commit to, and no
        // single guard covers both heights. The high punch is the quickest
        // overhead (11f) and does the least, or down-back stops being a
        // choice.
        MoveId::LowPunch  => mv(5.0,  3.0,  9.0,  5,  12.0, 7.0,  48.0, 30.0, 14.0, Level::Low,      false),
        MoveId::HighPunch => mv(11.0, 3.0, 16.0,  9,  16.0, 10.0, 50.0, 92.0, 14.0, Level::Overhead, false),
        // Two kick heights and nothing between: low must be crouch-blocked,
        // high blocked standing (a middle kick answered everything). The low
        // kick is the safe poke, the high kick the punishable overhead you
        // commit to.
        MoveId::LowKick   => mv(6.0,  3.0, 10.0,  7,  14.0, 9.0,  58.0, 26.0, 14.0, Level::Low,      false),
        // No knockdown on the high kick: a 17-damage overhead that also
        // floors you is a win condition, not a mix-up (a crouch-blocker
        // survived one fight in six). Its forward reach is short because the
        // leg's length goes up.
        MoveId::HighKick  => mv(14.0, 5.0, 22.0, 17,  22.0, 13.0, 62.0, 98.0, 18.0, Level::Overhead, false),
        MoveId::Sweep     => mv(10.0, 4.0, 22.0, 13,  0.0,  12.0, 72.0, 18.0, 16.0, Level::Low,      true),
        MoveId::JumpPunch => mv(4.0,  8.0,  2.0,  9,  15.0, 9.0,  52.0, 34.0, 14.0, Level::Overhead, false),
        MoveId::JumpKick  => mv(6.0, 10.0,  2.0, 13,  17.0, 10.0, 66.0, 14.0, 16.0, Level::Overhead, false),
        // The flying kick: the only attack that crosses a screen of ground,
        // and the only one whose cost comes after it: it is not cancelled by
        // landing (see `advance`), so a blocked one is a free heavy punish.
        // No knockdown, like the high kick. The blow lands level with the
        // hips, a straight line from hip to heel.
        MoveId::FlyingKick => mv(7.0, 16.0, 20.0, 15, 19.0, 11.0, 76.0, 42.0, 18.0,
                                 Level::Overhead, false),
        // Specials differ per fighter; this is the shape they share.
        MoveId::Special   => mv(11.0, 6.0, 26.0, 16,  20.0, 12.0, 74.0, 70.0, 18.0, Level::Mid,      true),
        // The throw. Short, unblockable, and brutal if it misses: 20
        // frames of recovery inside your opponent's range is the price
        // of the one move a guard cannot answer.
        MoveId::Throw     => mv(3.0,  2.0, 20.0, 18,  0.0,  0.0,  40.0, 58.0, 26.0, Level::Mid,      true),
    }
}

// ---- fighters ------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Special { ChiBolt, BullRush, TalonKick, LaserVision }

/// What a fighter wears and how they are built, kept beside the numbers so
/// looks and moves stay one roster.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Build {
    /// Karate gi: jacket to the elbows, trousers to mid-shin, bare feet.
    Gi,
    /// Stripped to the waist, heavy boots.
    Bare,
    /// A one-piece flight suit, wrapped forearms and shins.
    Suit,
    /// Shell on the back, plastron in front, a mask and pads in `trim`.
    Turtle,
    /// Masked head to toe: `cloth` limbs, `trim` body, head, gloves and boots.
    Spider,
    /// Stripped to the waist and barefoot, trousers torn off at the knee.
    Giant,
    /// A `cloth` suit with a cape and boots in `trim`.
    Caped,
}

/// A fighter's identity, in the only terms that change how they play.
#[derive(Copy, Clone)]
struct Archetype {
    name: &'static str,
    health: i32,
    walk: f32,
    back_walk: f32,
    jump_scale: f32,
    /// Scales every attack's damage and reach: the heavies hit hardest from
    /// closest, the light fighters poke from outside.
    power: f32,
    reach: f32,
    special: Special,
    special_name: &'static str,
    color: (f32, f32, f32),
    trim: (f32, f32, f32),
    skin: (f32, f32, f32),
    hair: (f32, f32, f32),
    build: Build,
    /// Limb and torso thickness, around 1.0: where a heavyweight looks like
    /// one without being any bigger.
    bulk: f32,
    /// The whole fighter's scale: body, hurtbox and reach together, so a
    /// small fighter is hard to hit and has to get close.
    size: f32,
    /// Blows land on them and move them, and cost them nothing, unless
    /// thrown by someone bigger (see `Fighter::hurt`).
    invincible: bool,
}

const FIGHTERS: [Archetype; 10] = [
    Archetype {
        name: "RYUKA", health: 100, walk: 132.0, back_walk: 108.0, jump_scale: 1.0,
        power: 1.0, reach: 1.0, special: Special::ChiBolt, special_name: "CHI BOLT",
        color: (0.92, 0.92, 0.96), trim: (0.85, 0.25, 0.25),
        skin: (0.85, 0.68, 0.52), hair: (0.24, 0.16, 0.12), build: Build::Gi, bulk: 1.0, size: 1.0, invincible: false,
    },
    Archetype {
        name: "BRUTUS", health: 120, walk: 96.0, back_walk: 78.0, jump_scale: 0.88,
        power: 1.35, reach: 0.88, special: Special::BullRush, special_name: "BULL RUSH",
        color: (0.30, 0.20, 0.26), trim: (0.95, 0.75, 0.2),
        skin: (0.74, 0.50, 0.34), hair: (0.20, 0.14, 0.12), build: Build::Bare, bulk: 1.28, size: 1.0, invincible: false,
    },
    Archetype {
        name: "KESTREL", health: 88, walk: 164.0, back_walk: 140.0, jump_scale: 1.12,
        power: 0.82, reach: 1.12, special: Special::TalonKick, special_name: "TALON KICK",
        color: (0.25, 0.65, 0.85), trim: (0.95, 0.95, 0.35),
        skin: (0.90, 0.74, 0.60), hair: (0.55, 0.42, 0.18), build: Build::Suit, bulk: 0.86, size: 1.0, invincible: false,
    },
    // Four turtles, told apart by the mask: `color` is the shell.
    Archetype {
        name: "GIOTTO", health: 88, walk: 220.0, back_walk: 180.0, jump_scale: 1.30,
        power: 1.00, reach: 1.12, special: Special::ChiBolt, special_name: "SHURIKEN",
        color: TURTLE_SHELL, trim: (0.25, 0.50, 0.95),
        skin: TURTLE_SKIN, hair: TURTLE_SHELL, build: Build::Turtle, bulk: 1.14, size: 0.5, invincible: false,
    },
    Archetype {
        name: "TITIAN", health: 96, walk: 200.0, back_walk: 164.0, jump_scale: 1.25,
        power: 1.10, reach: 1.06, special: Special::BullRush, special_name: "SHELL RAM",
        color: TURTLE_SHELL, trim: (0.90, 0.22, 0.20),
        skin: TURTLE_SKIN, hair: TURTLE_SHELL, build: Build::Turtle, bulk: 1.22, size: 0.5, invincible: false,
    },
    Archetype {
        name: "VERMEER", health: 82, walk: 235.0, back_walk: 192.0, jump_scale: 1.32,
        power: 0.92, reach: 1.15, special: Special::TalonKick, special_name: "RISING KICK",
        color: TURTLE_SHELL, trim: (0.62, 0.36, 0.85),
        skin: TURTLE_SKIN, hair: TURTLE_SHELL, build: Build::Turtle, bulk: 1.08, size: 0.5, invincible: false,
    },
    Archetype {
        name: "BOSCH", health: 78, walk: 250.0, back_walk: 205.0, jump_scale: 1.38,
        power: 0.85, reach: 1.12, special: Special::ChiBolt, special_name: "PIZZA TOSS",
        color: TURTLE_SHELL, trim: (0.98, 0.58, 0.14),
        skin: TURTLE_SKIN, hair: TURTLE_SHELL, build: Build::Turtle, bulk: 1.12, size: 0.5, invincible: false,
    },
    Archetype {
        name: "WEBBER", health: 90, walk: 300.0, back_walk: 250.0, jump_scale: 1.20,
        power: 0.85, reach: 1.08, special: Special::ChiBolt, special_name: "WEB SHOT",
        color: (0.20, 0.32, 0.78), trim: (0.86, 0.14, 0.16),
        skin: (0.86, 0.14, 0.16), hair: (0.10, 0.10, 0.12), build: Build::Spider, bulk: 0.88, size: 1.0, invincible: false,
    },
    Archetype {
        name: "GAMMA", health: 126, walk: 84.0, back_walk: 70.0, jump_scale: 0.90,
        power: 1.80, reach: 0.82, special: Special::BullRush, special_name: "RAMPAGE",
        color: (0.46, 0.28, 0.62), trim: (0.72, 0.52, 0.92),
        skin: (0.38, 0.64, 0.26), hair: (0.10, 0.10, 0.12), build: Build::Giant, bulk: 1.15, size: 1.5, invincible: false,
    },
    // The man of steel: only the giant can hurt him, he flies instead of
    // jumping, his punch hits three times as hard and his kick is the laser,
    // which kills. He is the last fight of every ladder (see `round_result`).
    Archetype {
        name: "ZENITH", health: 200, walk: 330.0, back_walk: 275.0, jump_scale: 1.0,
        power: 3.0, reach: 1.0, special: Special::LaserVision, special_name: "LASER VISION",
        color: (0.16, 0.36, 0.84), trim: (0.86, 0.14, 0.16),
        skin: (0.90, 0.72, 0.58), hair: (0.08, 0.08, 0.10), build: Build::Caped, bulk: 1.12,
        size: 1.0, invincible: true,
    },
];

/// The stages. Scenery only: the same floor line, height and walls in all.
const STAGES: usize = 7;
const STAGE_NAMES: [&str; STAGES] =
    ["THE DOCKS", "MOONLIT TEMPLE", "THE AIR BASE", "THE BATH HOUSE", "RIVER VILLAGE",
     "CRYSTAL FORTRESS", "QUEENS ROOFTOP"];

/// Where a fighter is at home. As in Street Fighter II, a fight is held at
/// the opponent's; a ladder never meets two of a kind in a row, so it never
/// visits one stage twice.
fn home_of(who: usize) -> usize {
    match FIGHTERS[who].build {
        Build::Bare => 0,
        Build::Gi => 1,
        Build::Suit => 2,
        Build::Turtle => 3,
        Build::Giant => 4,
        Build::Caped => 5,
        Build::Spider => 6,
    }
}

const TURTLE_SKIN: (f32, f32, f32) = (0.44, 0.68, 0.30);
const TURTLE_SHELL: (f32, f32, f32) = (0.44, 0.31, 0.17);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Act { Idle, Walk, Crouch, Air, Attack, Block, Hitstun, Knockdown, Victory, Defeat, Bow }

#[derive(Copy, Clone)]
struct Fighter {
    who: usize,
    x: f32,
    y: f32, // feet
    vy: f32,
    vx: f32, // air momentum only; grounded walking is direct
    facing: f32, // +1 right, -1 left
    health: i32,
    act: Act,
    /// Seconds elapsed inside the current action. Every state machine
    /// here is "this act, this long", which keeps hitstun, recovery and
    /// wakeup the same kind of thing.
    t: f32,
    mv: MoveId,
    /// One attack, one hit. Without this an active window of 4 frames
    /// would land 4 times.
    hit_done: bool,
    /// Whether this attack actually dealt damage, as opposed to being
    /// blocked. Only the flying kick reads it, to decide how much of
    /// its landing it has to pay for.
    hit_clean: bool,
    crouch_block: bool,
    stun: f32,
    rounds: i32,
    /// How long is left to cancel this attack's recovery into another
    /// one — set only when a light attack *lands*. See CANCEL_WINDOW.
    cancel_t: f32,
    /// Whether this attack was itself started from a cancel. A chain
    /// that could chain again is an infinite: jab into jab into jab,
    /// faster than the hitstun it causes, forever.
    chained: bool,
    /// Hits taken without recovering in between — the combo counter,
    /// kept on the receiving end because that is what it scales.
    combo: i32,
    /// An attack pressed while busy, remembered briefly, so a press one frame
    /// before recovery ends still comes out.
    buffered: Option<MoveId>,
    buffer_t: f32,

    // ---- what is being drawn, as opposed to played --
    // Poses are a function of (action, timer), so a changed action changed
    // the picture on the same frame. The drawing keeps a short memory
    // instead: the previous action, its timer frozen at the handover, and how
    // much of it still shows (see draw::pose_of()).
    shown: Act,
    shown_t: f32,
    shown_mv: MoveId,
    /// Whether the last-seen action was off the ground, and how fast.
    /// The airborne pose is read off vertical speed, so remembering the
    /// action without the speed remembers nothing.
    shown_air: bool,
    shown_vy: f32,
    prev_act: Act,
    prev_mv: MoveId,
    prev_t: f32,
    prev_air: bool,
    prev_vy: f32,
    /// 1.0 the frame the action changed, falling to 0 over `blend_len`.
    blend: f32,
    /// How long this handover gets: `POSE_BLEND`, or longer for
    /// standing up and landing.
    blend_len: f32,
    /// Seconds left of the give in the knees after a landing, and how
    /// hard the landing was. Drawing state only — nothing in the rules
    /// sees it, so a landing is still actionable on the touchdown frame.
    land: f32,
    land_force: f32,
    /// How far a flier has tipped over into level flight, 0 to 1, eased so
    /// the picture does not snap with the stick. Drawing state only.
    soar: f32,
    /// Seconds left of the guard impact window, and whether toward was held
    /// last frame (the window opens on the press, not the hold).
    parry_t: f32,
    /// Seconds for which this fighter cannot be thrown: no throw loops.
    throw_rest: f32,
    /// Seconds left wrapped in a web, the blows taken in it, and the time
    /// after it during which another web will not hold.
    webbed: f32,
    web_hits: u8,
    web_rest: f32,
    /// Punches thrown in quick succession change hands: this one is with the
    /// far hand. `since_punch` is the time since the last one started.
    alt_punch: bool,
    since_punch: f32,
    /// The far leg is the one in front: whichever foot a walk stopped on.
    /// Kicks come off the front leg.
    far_leads: bool,
    /// This round's call for the other turtles has been made; and whether
    /// the special now under way is that call.
    called: bool,
    calling: bool,
    was_toward: bool,
    /// A rage jump is on its way down: the landing shakes the floor.
    stomp: bool,
    /// What just happened to this fighter's blow, for `update_fight` to
    /// announce: it landed as a counter, or it was turned by a guard impact.
    countered: bool,
    turned: bool,
    /// The opponent's size, kept up to date by `face_off`: blows are aimed
    /// at the body in front of them.
    foe_size: f32,
}

/// How long a pose takes to hand over to the next one. Four frames:
/// long enough that nothing teleports, short enough that a jab still
/// lands on the frame the rules say it does.
pub(crate) const POSE_BLEND: f32 = 4.0 * F;

/// How long the knees take to absorb a landing and push back out of it.
pub(crate) const LAND_ABSORB: f32 = 12.0 * F;

/// How long standing up from a crouch takes to show (crouching down is
/// quick). Hitboxes change on the frame the action does; this is only the
/// picture.
pub(crate) const RISE_BLEND: f32 = 9.0 * F;
/// The opening bow (rei), seconds: attention, bow, hold, rise, attention.
pub(crate) const BOW_TIME: f32 = 1.6;
/// Rising out of the bow into the guard: unhurried.
const BOW_TO_GUARD: f32 = 0.35;
/// The round intro: the bow, then the guard coming up as FIGHT shows.
const ROUND_INTRO: f32 = BOW_TIME + 0.7;
/// How long the two fighters are shown off before a match.
const VS_SECS: f32 = 2.6;

impl Fighter {
    fn new(who: usize, x: f32, facing: f32) -> Self {
        Fighter {
            who, x, y: FLOOR_Y, vy: 0.0, vx: 0.0, facing,
            health: FIGHTERS[who].health,
            act: Act::Idle, t: 0.0, mv: MoveId::LowPunch, hit_done: false, hit_clean: false,
            crouch_block: false, stun: 0.0, rounds: 0,
            cancel_t: 0.0, chained: false, combo: 0,
            buffered: None, buffer_t: 0.0,
            shown: Act::Idle, shown_t: 0.0, shown_mv: MoveId::LowPunch,
            shown_air: false, shown_vy: 0.0, prev_air: false, prev_vy: 0.0,
            prev_act: Act::Idle, prev_mv: MoveId::LowPunch, prev_t: 0.0, blend: 0.0,
            blend_len: POSE_BLEND, land: 0.0, land_force: 0.0,
            soar: 0.0, stomp: false, parry_t: 0.0, throw_rest: 0.0,
            webbed: 0.0, web_hits: 0, web_rest: 0.0, called: false, calling: false,
            alt_punch: false, since_punch: 9.0, far_leads: false, was_toward: false, countered: false, turned: false,
            foe_size: FIGHTERS[who].size,
        }
    }

    fn arch(&self) -> Archetype { FIGHTERS[self.who] }
    fn airborne(&self) -> bool { self.y < FLOOR_Y - 0.01 }
    /// Seconds until the feet touch, from where they are and how fast
    /// they are moving. Solves the same fall `advance` integrates.
    fn air_time(&self) -> f32 {
        let d = (FLOOR_Y - self.y).max(0.0);
        ((-self.vy + (self.vy * self.vy + 2.0 * GRAVITY * d).sqrt()) / GRAVITY).max(0.0)
    }
    fn crouching(&self) -> bool { Self::is_low(self.act, self.mv, self.crouch_block) }
    /// Whether an action is played from down on the haunches. Off
    /// `self` so the drawing can ask about a past action too.
    fn is_low(act: Act, mv: MoveId, crouch_block: bool) -> bool {
        act == Act::Crouch || (act == Act::Block && crouch_block)
            || (act == Act::Attack && matches!(mv, MoveId::Sweep))
    }
    fn size(&self) -> f32 { self.arch().size }
    /// How fast a ground attack throws the body forward. Nothing for a
    /// full-size fighter; a small one has half the reach and has to leap in
    /// behind every blow, which leaves them in the other's face if it misses.
    fn lunge(&self) -> f32 { (1.0 - self.size()).max(0.0) * LUNGE }
    /// Down to the last quarter of the bar (see `RAGE`).
    fn enraged(&self) -> bool {
        let a = self.arch();
        !a.invincible && self.health > 0 && self.health * 4 <= a.health
    }
    /// A turtle guarding low is inside its shell, where nothing chips it.
    fn shelled(&self) -> bool {
        self.arch().build == Build::Turtle && self.act == Act::Block && self.crouch_block
    }
    /// Flies instead of jumping: up rises, down lands, no gravity between.
    fn flies(&self) -> bool { self.arch().build == Build::Caped }
    /// In the air under their own power, as opposed to falling.
    fn soaring(&self) -> bool {
        self.flies() && self.airborne() && matches!(self.act, Act::Air | Act::Attack)
    }
    /// The punch that sends them flying, whatever it hits.
    fn epic_punch(&self) -> bool {
        self.flies() && matches!(self.mv, MoveId::HighPunch | MoveId::LowPunch | MoveId::JumpPunch)
    }
    /// Lose health to a blow from a fighter of size `from`. The invincible
    /// feel only someone bigger than themselves, and then a quarter of it.
    fn hurt(&mut self, damage: i32, from: f32) {
        // A web takes two blows and no more, whatever they cost.
        if self.webbed > 0.0 && damage > 0 {
            self.web_hits += 1;
            if self.web_hits >= WEB_HITS { self.unweb(); }
        }
        let damage = if !self.arch().invincible { damage }
            else if from > self.size() { (damage / 4).max(1) }
            else { 0 };
        self.health = (self.health - damage).max(0);
    }
    /// Wrap this fighter up, unless a web has only just come off.
    fn web(&mut self) -> bool {
        if self.webbed > 0.0 || self.web_rest > 0.0 { return false; }
        self.webbed = WEB_SECS;
        self.web_hits = 0;
        self.vx = 0.0;
        true
    }
    fn unweb(&mut self) {
        self.webbed = 0.0;
        self.web_rest = WEB_REST;
    }
    fn width(&self) -> f32 { BODY_W * self.size() }
    fn height(&self) -> f32 {
        self.size() * if self.act == Act::Knockdown { PRONE_H }
            else if self.crouching() { CROUCH_H }
            else { STAND_H }
    }

    /// The box that can be hit. Deliberately the same box the fighter is
    /// drawn in: a hurtbox that does not match the picture is how a
    /// player learns to stop trusting their eyes.
    fn hurt_box(&self) -> (f32, f32, f32, f32) {
        let h = self.height();
        let w = self.width();
        (self.x - w / 2.0, self.y - h, w, h)
    }

    /// Can this fighter act at all right now?
    fn free(&self) -> bool {
        matches!(self.act, Act::Idle | Act::Walk | Act::Crouch | Act::Block)
    }

    fn scaled(&self, m: MoveData) -> MoveData {
        let a = self.arch();
        // Reach is the fighter's own; height is where the blow is aimed, and
        // nobody strikes over the head of a smaller opponent.
        let aim = a.size.min(self.foe_size);
        let rage = if self.enraged() { RAGE } else { 1.0 };
        // A small fighter hits above its weight to make up for its reach.
        // Against its own size there is nothing to make up.
        let even = if a.size < 1.0 && self.foe_size <= a.size { SMALL_ON_SMALL } else { 1.0 };
        MoveData {
            damage: ((m.damage as f32) * a.power * rage * even).round() as i32,
            reach: m.reach * a.reach * a.size,
            height: m.height * aim,
            thickness: m.thickness * aim,
            ..m
        }
    }

    /// Where the current attack can hit, during its active window only.
    fn hit_box(&self) -> Option<(f32, f32, f32, f32)> {
        if self.act != Act::Attack || self.hit_done || self.calling { return None; }
        let m = self.scaled(move_data(self.mv));
        let start = m.startup * F;
        let end = start + m.active * F;
        if self.t < start || self.t > end { return None; }
        let len = m.reach;
        let half = self.width() / 2.0;
        let x = if self.facing > 0.0 { self.x + half } else { self.x - half - len };
        Some((x, self.y - m.height - m.thickness / 2.0, len, m.thickness))
    }

    fn start_attack(&mut self, id: MoveId) {
        // A turtle's first special of the round calls the others in.
        self.calling = id == MoveId::Special && self.arch().build == Build::Turtle && !self.called;
        if self.calling { self.called = true; }
        self.chained = self.cancel_t > 0.0 && self.act == Act::Attack;
        if matches!(id, MoveId::LowPunch | MoveId::HighPunch) {
            self.alt_punch = self.since_punch < CHAIN_PUNCH && !self.alt_punch;
            self.since_punch = 0.0;
        }
        self.act = Act::Attack;
        self.mv = id;
        self.t = 0.0;
        self.hit_done = false;
        self.hit_clean = false;
        self.countered = false;
        self.stomp = id == MoveId::JumpKick && self.arch().build == Build::Giant;
        self.cancel_t = 0.0;
        if !self.airborne() && matches!(id, MoveId::LowPunch | MoveId::HighPunch
            | MoveId::LowKick | MoveId::HighKick | MoveId::Sweep) {
            self.vx = self.facing * self.lunge();
        }
        // The flying kick sets its own arc, flatter and faster than a jump;
        // thrown early in a jump it levels that jump off.
        if id == MoveId::FlyingKick {
            let launch = FLY_VY * self.arch().jump_scale;
            if self.airborne() {
                // Thrown mid-jump it must peak where it would off the floor,
                // so the rise already spent counts against the arc.
                let peak = launch * launch / (2.0 * GRAVITY);
                let risen = (FLOOR_Y - self.y).max(0.0);
                self.vy = -(2.0 * GRAVITY * (peak - risen).max(0.0)).sqrt();
            } else {
                self.vy = launch;
                self.y -= 0.5;
            }
            self.vx = self.facing * FLY_SPEED;
        }
    }

    /// Is this fighter inside the window where a landed light attack can
    /// be cancelled into something else?
    fn can_cancel(&self) -> bool {
        self.act == Act::Attack && self.cancel_t > 0.0 && !self.chained
    }
}

// ---- projectiles ---------------------------------------------------------

/// A flash where something connected. `blocked` picks the colour, which
/// is how a player tells "that cost me nothing" from "that cost me".
#[derive(Copy, Clone)]
struct Spark { x: f32, y: f32, ttl: f32, life: f32, blocked: bool }

/// One of the other turtles, called in: a fighter run by a script instead of
/// a stick. He waits his turn, runs in, strikes once and runs off the far side.
#[derive(Copy, Clone)]
struct Helper { f: Fighter, side: usize, wait: f32, mv: MoveId, struck: bool }

/// A word that pops up where something happened: COUNTER!, a fruit's points.
#[derive(Copy, Clone)]
struct Pop { text: &'static str, x: f32, y: f32, ttl: f32 }
const POP_SECS: f32 = 0.9;

/// The bonus fruit on the boards: where, how long it has left, and which of
/// the eight it is.
#[derive(Copy, Clone)]
struct Fruit { x: f32, ttl: f32, kind: usize }

#[derive(Copy, Clone)]
struct Bolt { x: f32, y: f32, vx: f32, vy: f32, owner: usize, active: bool, damage: i32 }

// ---- the match -----------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum State { Title, Select, Vs, RoundIntro, Fight, RoundEnd, MatchEnd, Over, Won }

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum RoundResult { P1, P2, Draw }

/// One player against the ladder (score, difficulty curve) or two in a single
/// match with a winner; they share only the round.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Mode { Solo, Versus }

struct Game {
    state: State,
    sess: Session,
    p: [Fighter; 2],
    bolts: [Bolt; 4],
    helpers: [Option<Helper>; 3],
    /// Screenshot scene 15: draw the kiosk card's poster instead of the game.
    poster: bool,
    mode: Mode,
    pick: usize,
    /// Player two's fighter, and whether each player has committed to
    /// their choice. Nobody fights until both have.
    pick2: usize,
    locked: [bool; 2],
    /// Which line of the title menu is highlighted.
    menu: usize,
    /// Which of the other two fighters this match is against, 0 then 1.
    opponent_index: usize,
    stage: usize,
    round: i32,
    clock: f32,
    phase: Timer,
    banner: &'static str,
    result: RoundResult,
    hitspark: [Spark; 4],
    shake: f32,
    /// Frames where everything stops on contact: it gives a hit weight, and
    /// it is when both players need to see who got hit with what.
    hitstop: f32,
    /// This round's opening bow has begun.
    bowed: bool,
    /// The last combo worth shouting about, and how long to shout it —
    /// a reward the player cannot see is not a reward.
    combo_shown: i32,
    combo_t: f32,
    /// Who landed it, so the number appears over the right shoulder.
    combo_side: usize,
    /// Rising as the ladder goes on: reaction time shortens and the
    /// reads get better. See cpu_think().
    difficulty: f32,
    cpu_delay: f32,
    cpu_plan: CpuPlan,
    /// Held so a punch and a kick pressed together read as one input
    /// rather than two — see read_special(). One per player: shared,
    /// each player's punch armed the other player's special.
    punch_at: [f32; 2],
    kick_at: [f32; 2],
    now: f32,
    /// Menu edge detection, per player and per direction. A menu step
    /// and a punch are not the same event, and sharing the slots made
    /// each one eat the other's.
    sel_held: [[bool; 2]; 2],
    sel_fire: [bool; 2],
    pops: [Pop; 4],
    fruit: Fruit,
    /// This round's fruit has not appeared yet.
    fruit_due: bool,
    /// Each health bar as it was a moment ago: the bar drains to the new
    /// value, so a player sees what a blow cost.
    ghost: [f32; 2],
    /// Where the floor was last shaken by a rage jump, and for how much longer
    /// the dust of it hangs.
    quake_x: f32,
    quake_t: f32,
    /// Real seconds of slow motion left on the finishing blow.
    slow: f32,
    /// The announcer's verdict on how the round was won: PERFECT, GREAT.
    call: &'static str,
}

/// How many fights a solo run is: everybody else, once.
const RUNGS: usize = FIGHTERS.len() - 1;
/// The difficulty dial for the first and the last opponent on the ladder.
const FIRST_RUNG: f32 = -0.45;
const LAST_RUNG: f32 = 0.8;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum CpuPlan { Wait, Approach, Retreat, Attack(MoveId), Jump, Block }

impl Game {
    fn new() -> Self {
        Game {
            state: State::Title,
            sess: Session::new(1),
            p: [Fighter::new(0, 200.0, 1.0), Fighter::new(1, 440.0, -1.0)],
            bolts: [Bolt { x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, owner: 0, active: false, damage: 0 }; 4],
            helpers: [None; 3],
            poster: false,
            mode: Mode::Solo,
            pick: 0,
            pick2: 1,
            locked: [false; 2],
            menu: 0,
            opponent_index: 0,
            stage: 0,
            round: 1,
            clock: ROUND_SECS,
            phase: Timer::default(),
            banner: "",
            result: RoundResult::Draw,
            hitspark: [Spark { x: 0.0, y: 0.0, ttl: 0.0, life: 1.0, blocked: false }; 4],
            shake: 0.0,
            hitstop: 0.0,
            bowed: false,
            combo_shown: 0,
            combo_t: 0.0,
            combo_side: 0,
            difficulty: 0.0,
            cpu_delay: 0.0,
            cpu_plan: CpuPlan::Wait,
            punch_at: [-1.0; 2],
            kick_at: [-1.0; 2],
            now: 0.0,
            sel_held: [[false; 2]; 2],
            sel_fire: [false; 2],
            pops: [Pop { text: "", x: 0.0, y: 0.0, ttl: 0.0 }; 4],
            fruit: Fruit { x: 0.0, ttl: 0.0, kind: 0 },
            fruit_due: false,
            ghost: [0.0; 2],
            quake_x: 0.0,
            quake_t: 0.0,
            slow: 0.0,
            call: "",
        }
    }

    fn picked(&self, who: usize) -> usize {
        if who == 0 { self.pick } else { self.pick2 }
    }
    fn set_pick(&mut self, who: usize, v: usize) {
        if who == 0 { self.pick = v; } else { self.pick2 = v; }
    }
    /// Has the *other* player already locked this fighter? Two players
    /// cannot bring the same fighter to the same fight.
    fn taken_by_other(&self, who: usize, at: usize) -> bool {
        self.mode == Mode::Versus && self.locked[1 - who] && self.picked(1 - who) == at
    }

    /// The ladder: every other fighter, once. Round the roster from the pick,
    /// reordered so no two fights running are at the same home stage, with
    /// the giant and then the fighter nothing hurts kept for the end.
    fn ladder(&self) -> [usize; RUNGS] {
        // The fights that close the ladder, in order, and everyone before.
        fn place(ring: &[usize], ends: &[usize], out: &mut Vec<usize>) -> bool {
            if out.len() == ring.len() { return true; }
            let early = ring.len() - ends.len();
            for &who in ring {
                let fits = if out.len() < early { !ends.contains(&who) } else { ends[out.len() - early] == who };
                if !fits || out.contains(&who) { continue; }
                if out.last().is_some_and(|&prev| home_of(prev) == home_of(who)) { continue; }
                out.push(who);
                if place(ring, ends, out) { return true; }
                out.pop();
            }
            false
        }
        let n = FIGHTERS.len();
        let ring: Vec<usize> = (1..n).map(|k| (self.pick + k) % n).collect();
        let mut ends: Vec<usize> = ring.iter().copied()
            .filter(|&w| FIGHTERS[w].invincible || FIGHTERS[w].build == Build::Giant).collect();
        ends.sort_by_key(|&w| FIGHTERS[w].invincible);
        let mut out = Vec::with_capacity(RUNGS);
        if !place(&ring, &ends, &mut out) { out = ring; }
        std::array::from_fn(|i| out[i])
    }

    /// Two players, one match, no ladder and no difficulty dial.
    fn start_versus(&mut self) {
        self.mode = Mode::Versus;
        self.opponent_index = 0;
        // Player two is the challenger here, so the fight is at their home.
        self.stage = home_of(self.pick2);
        self.difficulty = 0.0;
        self.p[0] = Fighter::new(self.pick, 200.0, 1.0);
        self.p[1] = Fighter::new(self.pick2, 440.0, -1.0);
        self.round = 1;
        self.start_round();
    }

    /// Put the match just set up behind its billing: who, where, and how
    /// far up the ladder.
    fn announce(&mut self) {
        self.state = State::Vs;
        self.phase.start(VS_SECS);
    }

    fn start_match(&mut self, opponent_index: usize) {
        self.opponent_index = opponent_index;
        let foe = self.ladder()[opponent_index];
        self.stage = home_of(foe);
        // The first opponent is a warm-up a newcomer can beat: below zero the
        // CPU also hesitates (see cpu_think). At 0.15 a playtest bot lost the
        // first match with six fighters in nine. The dial then climbs evenly
        // to the last.
        self.difficulty = FIRST_RUNG
            + (LAST_RUNG - FIRST_RUNG) * opponent_index as f32 / (RUNGS - 1) as f32;
        self.p[0] = Fighter::new(self.pick, 200.0, 1.0);
        self.p[1] = Fighter::new(foe, 440.0, -1.0);
        self.round = 1;
        self.start_round();
    }

    fn start_round(&mut self) {
        let (a, b) = (self.p[0].who, self.p[1].who);
        let (r0, r1) = (self.p[0].rounds, self.p[1].rounds);
        self.p[0] = Fighter::new(a, 200.0, 1.0);
        self.p[1] = Fighter::new(b, 440.0, -1.0);
        self.p[0].rounds = r0;
        self.p[1].rounds = r1;
        for b in self.bolts.iter_mut() { b.active = false; }
        self.helpers = [None; 3];
        for s in self.hitspark.iter_mut() { s.ttl = 0.0; }
        for w in self.pops.iter_mut() { w.ttl = 0.0; }
        self.fruit.ttl = 0.0;
        self.fruit_due = true;
        self.ghost = [self.p[0].health as f32, self.p[1].health as f32];
        self.slow = 0.0;
        self.quake_t = 0.0;
        self.call = "";
        self.hitstop = 0.0;
        self.clock = ROUND_SECS;
        self.banner = "ROUND";
        self.state = State::RoundIntro;
        self.phase.start(ROUND_INTRO);
        self.bowed = false;
        self.cpu_plan = CpuPlan::Wait;
        self.cpu_delay = 0.4;
    }

    /// The other three turtles come in from behind whoever called, one after
    /// another.
    fn call_turtles(&mut self, side: usize) {
        let caller = self.p[side];
        let from = if caller.facing > 0.0 { -30.0 } else { WIN_W as f32 + 30.0 };
        let others = (0..FIGHTERS.len())
            .filter(|&w| FIGHTERS[w].build == Build::Turtle && w != caller.who);
        for (k, who) in others.take(3).enumerate() {
            self.helpers[k] = Some(Helper {
                f: Fighter::new(who, from, caller.facing),
                side, wait: k as f32 * HELPER_GAP, mv: HELPER_MOVES[k], struck: false,
            });
        }
    }

    fn spawn_bolt(&mut self, owner: usize, damage: i32) {
        let foe = self.p[1 - owner];
        let laser = self.p[owner].arch().special == Special::LaserVision;
        // From the air the laser is aimed, so he turns to look first.
        if laser && self.p[owner].airborne() {
            self.p[owner].facing = if foe.x >= self.p[owner].x { 1.0 } else { -1.0 };
        }
        let f = self.p[owner];
        let x = f.x + f.facing * (f.width() / 2.0 + 10.0);
        // A bolt leaves the hands at chest height. The laser leaves the eyes,
        // twice as fast: level at the head in front of it from the ground (a
        // crouch goes under either), straight at the body from the air.
        let (y, vx, vy) = if !laser {
            (f.y - 54.0 * f.size(), f.facing * 300.0, 0.0)
        } else if !f.airborne() {
            (f.y - 98.0 * f.size().min(f.foe_size), f.facing * LASER_SPEED, 0.0)
        } else {
            let y = f.y - 100.0 * f.size();
            let (dx, dy) = (foe.x - x, foe.y - foe.height() * 0.5 - y);
            let d = dx.hypot(dy).max(1.0);
            (y, dx / d * LASER_SPEED, dy / d * LASER_SPEED)
        };
        if let Some(b) = self.bolts.iter_mut().find(|b| !b.active) {
            *b = Bolt { x, y, vx, vy, owner, active: true, damage };
        }
    }

    fn pop(&mut self, text: &'static str, x: f32, y: f32) {
        let fresh = Pop { text, x, y, ttl: POP_SECS };
        let slot = self.pops.iter().position(|w| w.ttl <= 0.0).unwrap_or(0);
        self.pops[slot] = fresh;
    }

    fn spark(&mut self, x: f32, y: f32, big: bool, blocked: bool) {
        let life = if big { 0.26 } else { 0.17 };
        let fresh = Spark { x, y, ttl: life, life, blocked };
        for s in self.hitspark.iter_mut() {
            if s.ttl <= 0.0 { *s = fresh; return; }
        }
        self.hitspark[0] = fresh;
    }
}

// ---- rules ---------------------------------------------------------------

/// Does a block at this stance stop an attack at this height? Standing covers
/// overheads and mids, crouching covers lows and mids; neither covers
/// everything.
fn blocks(level: Level, crouch_block: bool) -> bool {
    match level {
        Level::Mid => true,
        Level::Low => crouch_block,
        Level::Overhead => !crouch_block,
    }
}

/// Throws cannot be blocked: block beats attack, throw beats block, attack
/// beats throw. Without it, two players holding back ran the clock out.
fn unblockable(id: MoveId) -> bool { id == MoveId::Throw }

/// How close the two have to be for a punch to become a throw, as a share of
/// the throw's own range: well inside it, so a throw that starts connects.
const THROW_CLOSE: f32 = 0.74;

/// Tell each fighter how big the other is, and whether they are close enough
/// to throw. Called before input, every frame.
fn face_off(p: &mut [Fighter; 2]) -> [bool; 2] {
    let dist = (p[1].x - p[0].x).abs();
    p[0].foe_size = p[1].size();
    p[1].foe_size = p[0].size();
    [0, 1].map(|i| dist <= throw_range(&p[i]))
}

fn throw_range(f: &Fighter) -> f32 { attack_range(f, MoveId::Throw) * THROW_CLOSE }

/// The distance between centres at which `id` would just touch the
/// other fighter. Reach plus both half-widths — the number a player is
/// judging by eye every time they decide whether to step in.
fn attack_range(f: &Fighter, id: MoveId) -> f32 {
    f.scaled(move_data(id)).reach + (f.width() + BODY_W * f.foe_size) / 2.0
}

/// Is this fighter holding away from the other one?
fn holding_back(hold: Input, facing: f32) -> bool {
    (facing > 0.0 && hold.left) || (facing < 0.0 && hold.right)
}

#[derive(Copy, Clone, Default)]
struct Input {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    punch_low: bool,
    punch_high: bool,
    /// The two kick heights, one button each.
    kick_low: bool,
    kick_high: bool,
    special: bool,
}

impl Input {
    /// Any kick button at all. Used where the height does not matter:
    /// the special, and reading a kick while airborne.
    fn any_kick(self) -> bool { self.kick_low || self.kick_high }
    fn any_punch(self) -> bool { self.punch_low || self.punch_high }
}

/// Which attack these buttons ask for, given where the fighter is. One place
/// decides, so a buffered press and a live one agree.
fn pressed_move(f: &Fighter, inp: Input, close: bool) -> Option<MoveId> {
    // The flier has two moves wherever he is: any kick is the laser, any
    // punch the one big punch. No throw; he does not need one.
    if f.flies() {
        return if inp.special || inp.any_kick() { Some(MoveId::Special) }
            else if !inp.any_punch() { None }
            else if f.airborne() { Some(MoveId::JumpPunch) }
            else { Some(MoveId::HighPunch) };
    }
    if f.airborne() { return air_move(f, inp); }
    grounded_move(inp, close)
}

fn air_move(f: &Fighter, inp: Input) -> Option<MoveId> {
    {
        if inp.any_punch() { return Some(MoveId::JumpPunch); }
        // Any kick in the air is the jump kick (the arc already decided the
        // height), except a kick with up early enough to be the flying kick
        // (FLY_WINDOW).
        if inp.any_kick() {
            if inp.up && f.act == Act::Air && f.t < FLY_WINDOW {
                return Some(MoveId::FlyingKick);
            }
            return Some(MoveId::JumpKick);
        }
        return None;
    }
}

fn grounded_move(inp: Input, close: bool) -> Option<MoveId> {
    if inp.special { return Some(MoveId::Special); }
    // A standing punch right up against them is a throw: the throw has no
    // button of its own, and proximity is how the originals did it.
    if inp.any_punch() && close && !inp.down { return Some(MoveId::Throw); }
    // Crouching turns a punch into the low one, the way it turns a
    // kick into the sweep: down plus a button means one thing whichever
    // button found it.
    if inp.down && inp.any_punch() { return Some(MoveId::LowPunch); }
    if inp.punch_low { return Some(MoveId::LowPunch); }
    if inp.punch_high { return Some(MoveId::HighPunch); }
    // Crouching turns any kick into the sweep, which keeps "down plus a
    // kick" meaning one thing whichever kick button found it.
    if inp.down && inp.any_kick() { return Some(MoveId::Sweep); }
    // Up plus a kick is the flying kick, checked before the jump below so
    // they cannot both happen.
    if inp.up && inp.any_kick() { return Some(MoveId::FlyingKick); }
    if inp.kick_low { return Some(MoveId::LowKick); }
    if inp.kick_high { return Some(MoveId::HighKick); }
    None
}

/// How long a pressed attack waits for its turn. Ten frames is long
/// enough to cover a human pressing early, short enough that it never
/// fires an attack the player has forgotten asking for.
const BUFFER: f32 = 10.0 * F;

/// What the shell is told while the title screen is up: a cabinet with
/// its second station lit and nobody at it yet. See `blip::web::set_players`.
const TITLE_OPEN: i32 = 2;

/// A blocked special still chips a sixth of its damage (normals chip
/// nothing). Without it two good blockers time out (seven rounds in twelve);
/// chip makes holding back cost something.
const CHIP_DIVISOR: i32 = 6;

/// Frames to cancel a landed light attack into a heavier one: the reward for
/// seeing the hit. Only a connecting light attack opens it; whiffed or
/// blocked, you get nothing.
const CANCEL_WINDOW: f32 = 14.0 * F;

/// How long after a punch or kick starts the other button still turns it into
/// the special: nobody presses two buttons on the same frame, least of all on
/// a touch screen.
const SPECIAL_WINDOW: f32 = 5.0 * F;

/// Hitting someone out of their own attack is a counter, as in Tekken: a
/// quarter more damage and six frames more stun.
const COUNTER: f32 = 1.25;
const COUNTER_STUN: f32 = 6.0 * F;

/// Rage, as in Tekken: with a quarter of the bar left a fighter hits a fifth
/// harder, so a round is not over until it is over.
const RAGE: f32 = 1.2;

/// The guard impact, Soulcalibur's: tap toward the opponent inside this
/// window before a blow lands and it is knocked aside, leaving the attacker
/// open for `IMPACT_STUN`. Throws and projectiles cannot be turned.
const IMPACT_WINDOW: f32 = 6.0 * F;
const IMPACT_STUN: f32 = 24.0 * F;

/// The bonus fruit, Pac-Man's: once a round it appears mid-stage when the
/// clock reaches `FRUIT_AT`, waits `FRUIT_STAYS` seconds, and gives whoever
/// walks over it a share of their health back. The eight follow Pac-Man's
/// order and are worth Pac-Man's points.
const FRUIT_AT: f32 = ROUND_SECS - 12.0;
const FRUIT_STAYS: f32 = 9.0;
const FRUIT_HEAL: f32 = 0.12;
const FRUIT_POINTS: [(i32, &str); 8] = [(100, "100"), (300, "300"), (500, "500"), (700, "700"),
    (1000, "1000"), (2000, "2000"), (3000, "3000"), (5000, "5000")];

/// The giant's jump kick is a rage jump: when he comes down the whole floor
/// jumps, and a fighter standing on it anywhere is thrown off their feet for
/// this much. Being in the air when he lands is the only answer.
const QUAKE_DAMAGE: i32 = 6;
const QUAKE_SECS: f32 = 0.5;

/// The finishing blow plays at this speed for this long, as Tekken's does.
const SLOW_MO: f32 = 0.3;
const SLOW_MO_SECS: f32 = 0.6;

/// What each successive hit of a combo is worth. Two hits is a reward;
/// eight would be a cutscene.
fn combo_scale(hits: i32) -> f32 {
    match hits {
        0 => 1.0,
        1 => 0.75,
        _ => 0.5,
    }
}

fn apply_input(f: &mut Fighter, inp: Input, close: bool, dt: f32) {
    let toward = if f.facing > 0.0 { inp.right } else { inp.left };
    if toward && !f.was_toward { f.parry_t = IMPACT_WINDOW; }
    else if f.parry_t > 0.0 { f.parry_t -= dt; }
    f.was_toward = toward;

    if f.buffer_t > 0.0 {
        f.buffer_t -= dt;
        if f.buffer_t <= 0.0 { f.buffered = None; }
    }

    // Flying: the stick steers, a button attacks from where he hangs.
    if f.soaring() {
        if f.act == Act::Attack { return; }
        if let Some(id) = pressed_move(f, inp, close) {
            f.vx = 0.0;
            f.vy = 0.0;
            f.start_attack(id);
            return;
        }
        f.vx = if inp.left { -SOAR } else if inp.right { SOAR } else { 0.0 };
        f.vy = if inp.up { -SOAR } else if inp.down { SOAR * 1.4 } else { 0.0 };
        return;
    }

    // In the air and not yet committed: the jump-in attack comes out now.
    // (Airborne counts as busy, so the busy path below would buffer it into a
    // grounded kick on landing.) Landing ends it, which keeps one attack per
    // jump.
    if f.airborne() && f.act == Act::Air {
        if let Some(id) = air_move(f, inp) {
            // An attack whose startup outlasts the fall never goes live
            // (landing ends it), so buffer it as the grounded move instead.
            // Two frames of margin, or it comes out on the landing frame and
            // is cancelled.
            if f.air_time() >= (move_data(id).startup + 2.0) * F {
                f.start_attack(id);
            } else if let Some(g) = grounded_move(inp, close) {
                f.buffered = Some(g);
                f.buffer_t = BUFFER;
            }
        }
        return;
    }

    // The second button of a special, a moment late: the attack the first
    // button began becomes the special, as long as it has not come out yet.
    if inp.special && f.act == Act::Attack && f.mv != MoveId::Special && !f.airborne()
        && f.t < SPECIAL_WINDOW.min(move_data(f.mv).startup * F) {
        f.start_attack(MoveId::Special);
        return;
    }

    if !f.free() && !f.can_cancel() {
        // Busy, but listening. The move is resolved now, from the stance
        // the player was in when they pressed, so a buffered sweep is
        // still a sweep when it comes out.
        if let Some(id) = pressed_move(f, inp, close) {
            f.buffered = Some(id);
            f.buffer_t = BUFFER;
        }
        return;
    }

    if let Some(id) = f.buffered.take() {
        f.buffer_t = 0.0;
        f.start_attack(id);
        return;
    }

    if let Some(id) = pressed_move(f, inp, close) { f.start_attack(id); return; }

    let crouch = inp.down;
    // An airborne fighter has already committed; the jump is the
    // decision and only an attack is left.
    if f.airborne() { return; }

    if inp.up && f.flies() {
        f.vy = -SOAR;
        f.vx = 0.0;
        f.y -= 0.5;
        f.act = Act::Air;
        f.t = 0.0;
        return;
    }
    if inp.up {
        f.vy = JUMP_VY * f.arch().jump_scale;
        f.vx = if inp.left { -AIR_DRIFT } else if inp.right { AIR_DRIFT } else { 0.0 };
        f.y -= 0.5; // leave the floor this frame so airborne() reads true
        f.act = Act::Air;
        f.t = 0.0;
        return;
    }

    // Blocking is not a button: it is holding away, which means every
    // step backwards is already a block and retreating is never free.
    let back = holding_back(inp, f.facing);
    if back {
        f.act = Act::Block;
        f.crouch_block = crouch;
        f.t = 0.0;
        let speed = f.arch().back_walk;
        if !crouch { f.x -= f.facing * speed * dt; }
        return;
    }
    if crouch {
        f.act = Act::Crouch;
        f.crouch_block = false;
        return;
    }
    let toward = (inp.right && f.facing > 0.0) || (inp.left && f.facing < 0.0);
    if toward {
        f.act = Act::Walk;
        f.x += f.facing * f.arch().walk * dt;
    } else {
        f.act = Act::Idle;
    }
}

/// Advance one fighter's physics and action timer.
fn advance(f: &mut Fighter, dt: f32) {
    if f.land > 0.0 { f.land = (f.land - dt).max(0.0); }
    if f.throw_rest > 0.0 { f.throw_rest = (f.throw_rest - dt).max(0.0); }
    f.since_punch += dt;
    if f.act == Act::Walk && !f.airborne() { f.far_leads = draw::far_foot_leads(f); }
    if f.web_rest > 0.0 { f.web_rest = (f.web_rest - dt).max(0.0); }
    if f.webbed > 0.0 {
        f.webbed -= dt;
        if f.webbed <= 0.0 { f.unweb(); }
    }
    let level = if f.soaring() && f.act == Act::Air { (f.vx * f.facing / SOAR).clamp(0.0, 1.0) } else { 0.0 };
    f.soar += (level - f.soar) * (10.0 * dt).min(1.0);

    let from = f.t;
    f.t += dt;
    if f.act == Act::Bow && f.t >= BOW_TIME { f.act = Act::Idle; f.t = 0.0; }
    // A long frame on a slow device can step clean over a two-frame active
    // window; stop in it for this step so the hit is still tested.
    if f.act == Act::Attack && !f.hit_done {
        let m = f.scaled(move_data(f.mv));
        let (start, end) = (m.startup * F, (m.startup + m.active) * F);
        if from < start && f.t > end { f.t = (start + end) / 2.0; }
    }
    if f.cancel_t > 0.0 { f.cancel_t -= dt; }

    if f.airborne() || f.vy < 0.0 {
        // A flier hangs where the stick left him; everyone else falls.
        if f.soaring() { f.vy = f.vy.max(-SOAR); } else { f.vy += GRAVITY * dt; }
        f.y += f.vy * dt;
        if f.soaring() && f.y < FLOOR_Y - CEILING { f.y = FLOOR_Y - CEILING; f.vy = 0.0; }
        f.x += f.vx * dt;
        if f.y >= FLOOR_Y {
            f.y = FLOOR_Y;
            // How hard they arrived, for the drawing.
            f.land = LAND_ABSORB;
            f.land_force = (f.vy / -JUMP_VY).clamp(0.25, 1.0);
            f.vy = 0.0;
            f.vx = 0.0;
            // Landing cancels an air attack, except the flying kick: its
            // recovery plays out on the ground, the price of the distance.
            if f.act == Act::Attack && f.mv == MoveId::FlyingKick {
                let m = f.scaled(move_data(f.mv));
                let total = (m.startup + m.active + m.recovery) * F;
                // Blocked or whiffed: at least this much left to pay.
                // Connected: at most this much.
                f.t = if f.hit_clean {
                    f.t.max(total - FLY_HIT_LAG)
                } else {
                    f.t.min(total - FLY_LAND_LAG)
                };
                f.hit_done = true; // it stops being an attack on contact with the floor
            } else if f.act == Act::Air || f.act == Act::Attack {
                f.act = Act::Idle;
                f.t = 0.0;
            }
        }
    }

    // An attack that carries the body along the floor (the rush, a small
    // fighter's lunge) slides, and the slide bleeds off so it ends.
    if !f.airborne() && f.act == Act::Attack {
        f.x += f.vx * dt;
        f.vx *= 1.0 - (6.0 * dt).min(1.0);
    }

    match f.act {
        Act::Attack => {
            let m = f.scaled(move_data(f.mv));
            let total = (m.startup + m.active + m.recovery) * F;
            if !f.airborne() && f.t >= total { f.act = Act::Idle; f.t = 0.0; f.vx = 0.0; }
            // In the air a flier's attack ends on its own clock, not on landing.
            else if f.soaring() && f.t >= total { f.act = Act::Air; f.t = 0.0; }
        }
        Act::Hitstun | Act::Block => {
            f.stun -= dt;
            if f.stun <= 0.0 && f.act == Act::Hitstun {
                f.act = Act::Idle;
                f.t = 0.0;
                f.combo = 0; // they got out; the next hit starts a new one
            }
        }
        Act::Knockdown => {
            f.combo = 0;
            // Down, then up with a moment of invulnerability — otherwise
            // a knockdown is a free second hit and the game becomes one
            // sweep repeated.
            if f.t >= 1.15 { f.act = Act::Idle; f.t = 0.0; }
        }
        _ => {}
    }

    note_handover(f, dt);
}

/// Spot one action becoming another and freeze what the drawing needs to keep
/// showing the old one. Runs at the end of `advance`, where jumps, attacks
/// and hitstun end; at the top of the next frame the change frame had no
/// blend.
fn note_handover(f: &mut Fighter, dt: f32) {
    if f.act != f.shown {
        let was_low = Fighter::is_low(f.shown, f.shown_mv, f.crouch_block);
        f.prev_act = f.shown;
        f.prev_t = f.shown_t;
        // Freeze the move with the action it belonged to. Assigned on
        // every unchanged frame, it was overwritten one frame after
        // each handover, so blends collapsed to the target on frame two.
        f.prev_mv = f.shown_mv;
        f.prev_air = f.shown_air;
        f.prev_vy = f.shown_vy;
        f.blend = 1.0;
        // Getting up is the slow direction, and a landing is the same
        // kind of event: the feet arrive, the body takes a moment.
        let landed = f.land >= LAND_ABSORB;
        f.blend_len = if f.prev_act == Act::Bow { BOW_TO_GUARD }
            else if (was_low && !f.crouching()) || landed { RISE_BLEND } else { POSE_BLEND };
        f.shown = f.act;
    } else {
        // `shown_t` has to keep the *old* action's final timer until
        // the handover has been noted, so it is only refreshed on the
        // frames where nothing changed.
        f.shown_t = f.t;
        f.shown_mv = f.mv;
        f.shown_air = f.airborne();
        f.shown_vy = f.vy;
    }
    if f.blend > 0.0 { f.blend = (f.blend - dt / f.blend_len).max(0.0); }
}

/// Is this fighter untouchable right now? Only on the way up from a
/// knockdown, and only briefly.
fn invulnerable(f: &Fighter) -> bool {
    f.act == Act::Knockdown && f.t > 0.85
}

/// Resolve `attacker`'s active hitbox against `defender`. Returns the
/// damage dealt (0 for a block or a miss) and whether it was blocked.
fn resolve_hit(attacker: &mut Fighter, defender: &mut Fighter, hold: Input) -> (i32, bool, bool) {
    let Some(hb) = attacker.hit_box() else { return (0, false, false) };
    if invulnerable(defender) { return (0, false, false); }
    let (dx, dy, dw, dh) = defender.hurt_box();
    if !rects_overlap(hb.0, hb.1, hb.2, hb.3, dx, dy, dw, dh) { return (0, false, false); }

    let m = attacker.scaled(move_data(attacker.mv));
    // A throw cannot catch someone off the ground — jumping is the
    // answer to a throw, as attacking is, which keeps the triangle from
    // collapsing into "walk in and throw".
    if attacker.mv == MoveId::Throw && (defender.airborne() || defender.throw_rest > 0.0) {
        return (0, false, false);
    }
    attacker.hit_done = true;
    if attacker.mv == MoveId::Throw { defender.throw_rest = THROW_REST; }

    // The guard impact: the blow is turned and the attacker left open.
    if defender.parry_t > 0.0 && !defender.airborne() && defender.free()
        && !unblockable(attacker.mv) {
        defender.parry_t = 0.0;
        attacker.turned = true;
        attacker.act = Act::Hitstun;
        attacker.stun = IMPACT_STUN;
        attacker.t = 0.0;
        attacker.vx = 0.0;
        return (0, true, false);
    }

    // A defender can only block on the ground, holding away, and not
    // while already committed to something of their own.
    let guarding = !defender.airborne()
        && holding_back(hold, defender.facing)
        && matches!(defender.act, Act::Idle | Act::Walk | Act::Crouch | Act::Block);
    if guarding && !unblockable(attacker.mv) && blocks(m.level, defender.crouch_block || hold.down) {
        defender.act = Act::Block;
        defender.crouch_block = hold.down;
        defender.stun = m.blockstun * F;
        let chip = if attacker.mv == MoveId::Special { (m.damage / CHIP_DIVISOR).max(1) } else { 0 };
        if chip > 0 && !defender.shelled() { defender.hurt(chip, attacker.size()); }
        return (0, true, false);
    }

    // Caught in the middle of their own attack, before its recovery.
    let theirs = move_data(defender.mv);
    let counter = defender.act == Act::Attack && defender.t <= (theirs.startup + theirs.active) * F;
    attacker.countered = counter;
    let bonus = if counter { COUNTER } else { 1.0 };
    let damage = ((m.damage as f32) * combo_scale(defender.combo) * bonus).round().max(1.0) as i32;
    defender.combo += 1;
    // A light attack that lands buys the right to follow it up.
    if matches!(attacker.mv, MoveId::LowPunch) && !attacker.chained {
        attacker.cancel_t = CANCEL_WINDOW;
    }
    defender.hurt(damage, attacker.size());
    attacker.hit_clean = true;
    // A flying kick that connects stops flying, so the attacker can act
    // before the defender's hitstun ends. A blocked one carries on through,
    // into punishing range.
    if attacker.mv == MoveId::FlyingKick {
        attacker.vx = 0.0;
        attacker.vy = attacker.vy.max(0.0);
    }
    let floored = m.knockdown || attacker.epic_punch();
    if floored || defender.airborne() {
        defender.act = Act::Knockdown;
        defender.t = 0.0;
        defender.vy = 0.0;
        defender.vx = 0.0;
        defender.y = FLOOR_Y;
    } else {
        defender.act = Act::Hitstun;
        defender.stun = m.hitstun * F + if counter { COUNTER_STUN } else { 0.0 };
        defender.t = 0.0;
    }
    (damage, false, floored)
}

/// Push the two apart after a hit, and if one of them is against a wall,
/// push the *other* one instead. Corner pressure only exists because of
/// this: without it a cornered fighter slides out for free.
fn push_apart(p: &mut [Fighter; 2], amount: f32) {
    let dir = if p[0].x < p[1].x { -1.0 } else { 1.0 };
    let (lo, hi) = (WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
    let a0 = p[0].x + dir * amount;
    let a1 = p[1].x - dir * amount;
    if a0 < lo || a0 > hi {
        p[1].x = clamp(p[1].x - dir * amount * 2.0, lo, hi);
    } else if a1 < lo || a1 > hi {
        p[0].x = clamp(p[0].x + dir * amount * 2.0, lo, hi);
    } else {
        p[0].x = a0;
        p[1].x = a1;
    }
}

/// Fighters may not walk through each other.
fn separate(p: &mut [Fighter; 2]) {
    let min = (p[0].width() + p[1].width()) / 2.0 * 0.82;
    let d = p[1].x - p[0].x;
    if d.abs() >= min { return; }
    let fix = (min - d.abs()) / 2.0 * if d >= 0.0 { 1.0 } else { -1.0 };
    let (lo, hi) = (WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
    p[0].x = clamp(p[0].x - fix, lo, hi);
    p[1].x = clamp(p[1].x + fix, lo, hi);
}

// ---- the CPU -------------------------------------------------------------

/// What the CPU wants to do, decided on a delay that shortens with difficulty
/// rather than every frame, so it reacts like a player and can be baited.
fn cpu_think(g: &mut Game) -> CpuPlan {
    let me = g.p[1];
    let foe = g.p[0];
    let dist = (foe.x - me.x).abs();
    let roll = || (rand_int(0, 99) as f32) / 100.0;

    // Below zero on the dial it stands and thinks for some of its turns.
    if roll() < -g.difficulty { return CpuPlan::Wait; }

    // Ranges come from the move table. "Kicking range" is the high kick's.
    let kick_range = attack_range(&me, MoveId::HighKick);
    let jab_range = attack_range(&me, MoveId::LowPunch);
    // The high kick is the shortest kick; thrown from further out it hits
    // nothing.
    let high_range = attack_range(&me, MoveId::HighKick);
    let foe_range = attack_range(&foe, MoveId::HighKick);


    // A rage jump is coming down: be off the floor when it lands. The better
    // the opponent, the more often it sees it.
    if foe.stomp && foe.airborne() && !me.airborne() {
        return if roll() < 0.35 + 0.5 * g.difficulty { CpuPlan::Jump } else { CpuPlan::Wait };
    }

    // The one with the laser uses it from range: level from the ground, which
    // a crouch goes under, and from the air at whoever is sitting under it.
    if me.arch().special == Special::LaserVision && !me.airborne() && me.free()
        && dist > kick_range * 1.4 && roll() < 0.30 + 0.35 * g.difficulty {
        return if foe.crouching() { CpuPlan::Jump } else { CpuPlan::Attack(MoveId::Special) };
    }

    // React to what they are doing, before deciding what to do.
    if foe.airborne() && dist < kick_range * 1.4 {
        return if roll() < 0.4 + 0.5 * g.difficulty {
            CpuPlan::Attack(MoveId::HighKick) // meet them on the way down
        } else {
            CpuPlan::Block
        };
    }
    if foe.act == Act::Attack && dist < foe_range * 1.2 {
        // Being attacked at close range forces a fresh decision (see
        // update_fight()). How much it blocks depends on what is coming: a
        // sweep or special is respected, a jab traded with; always blocking
        // stalemated against a mashed punch.
        let scary = move_data(foe.mv).damage >= 10 || move_data(foe.mv).knockdown;
        let want = if scary { 0.55 + 0.4 * g.difficulty } else { 0.22 + 0.2 * g.difficulty };
        if roll() < want { return CpuPlan::Block; }
        // Not blocking means taking the turn back with the fastest thing
        // available, or a throw up close. (Retreating hands a rushing
        // opponent free ground.)
        if dist <= throw_range(&me) && foe.throw_rest <= 0.0 { return CpuPlan::Attack(MoveId::Throw); }
        if dist < jab_range { return CpuPlan::Attack(MoveId::LowPunch); }
    }
    // They committed to something slow and it missed: take the turn.
    if foe.act == Act::Attack && foe.hit_done && dist < kick_range && roll() < 0.5 + 0.4 * g.difficulty {
        return CpuPlan::Attack(MoveId::LowPunch);
    }

    // Read the guard: without it a crouch-blocker took zero damage over a
    // full round.
    if foe.act == Act::Block && dist < kick_range * 1.1 {
        let read = roll() < 0.4 + 0.5 * g.difficulty;
        if read {
            return if dist <= throw_range(&me) && foe.throw_rest <= 0.0 {
                CpuPlan::Attack(MoveId::Throw)  // nothing guards against this
            } else if foe.crouch_block {
                // A low guard loses to an overhead: the high kick (slow,
                // readable) is preferred, the jump-in kept as the surprise.
                if dist < high_range && roll() < 0.45 { CpuPlan::Attack(MoveId::HighKick) }
                else { CpuPlan::Jump }
            } else {
                // A high guard loses to a low, and there are two of
                // those as well: the sweep if it can afford the
                // recovery, the quick one if it cannot.
                if roll() < 0.6 { CpuPlan::Attack(MoveId::Sweep) }
                else { CpuPlan::Attack(MoveId::LowKick) }
            };
        }
    }

    if dist > 230.0 {
        // Too far to do anything but close the gap — or throw something
        // that crosses it.
        if matches!(me.arch().special, Special::ChiBolt | Special::LaserVision) && roll() < 0.2 + 0.4 * g.difficulty {
            return CpuPlan::Attack(MoveId::Special);
        }
        return CpuPlan::Approach;
    }
    if dist > kick_range * 1.25 {
        // Jumping in is only worth it from outside their reach: into a
        // poke it is a free knockdown for them.
        if dist > foe_range * 1.3 && roll() < 0.10 + 0.14 * g.difficulty {
            // Two ways in on one budget: the flying kick and the jump-in.
            // Both on top of each other, a crouch-blocker never survived a
            // round.
            return if roll() < 0.35 {
                CpuPlan::Attack(MoveId::FlyingKick)
            } else {
                CpuPlan::Jump
            };
        }
        // Out-ranged, inside their reach and outside your own, the gap must
        // be crossed, not walked. Without this Brutus landed nothing against
        // a long-limbed poker; the charge and the jump-in cover ground.
        if dist < foe_range * 1.15 && foe_range > kick_range * 1.1 {
            return match roll() {
                r if r < 0.34 => CpuPlan::Attack(MoveId::Special),
                r if r < 0.48 => CpuPlan::Jump,
                r if r < 0.60 => CpuPlan::Attack(MoveId::FlyingKick),
                r if r < 0.88 => CpuPlan::Approach,
                _ => CpuPlan::Block,
            };
        }
        return CpuPlan::Approach;
    }

    // In range, the mix is the personality: wide enough that no single answer
    // covers it, weighted to the fighter. `r` shifts toward attacking when
    // the opponent is nearly out, so the CPU can close a round.
    let nearly_out = (foe.health as f32) < FIGHTERS[foe.who].health as f32 * 0.35;
    // Half the clock gone with both bars nearly full is a round the clock
    // will decide; force the issue, the same nudge as smelling a finish.
    let stalling = g.clock < ROUND_SECS * 0.5
        && (foe.health as f32) > FIGHTERS[foe.who].health as f32 * 0.72
        && (me.health as f32) > FIGHTERS[me.who].health as f32 * 0.72;
    let r = if nearly_out || stalling { roll() * 0.7 } else { roll() };

    if dist >= jab_range {
        // At the edge of its reach: long pokes only, and the sweep is the
        // one that really reaches out here.
        return match r {
            _ if r < 0.44 => CpuPlan::Attack(MoveId::Sweep),
            _ if r < 0.58 => CpuPlan::Attack(MoveId::LowKick),
            _ if r < 0.72 => CpuPlan::Attack(MoveId::Special),
            _ if r < 0.94 => CpuPlan::Approach,
            _ => CpuPlan::Block,
        };
    }

    // Up close every fighter keeps a fast option, even the heavy (without one
    // he was jabbed for a whole round); the heavy leans to the slow end of
    // the same list.
    let heavy = me.arch().special == Special::BullRush;
    let fast_share = if heavy { 0.20 } else { 0.32 };
    // The bands must leave room for the last two arms, or `Block` becomes
    // unreachable and the CPU stops guarding up close.
    match r {
        _ if r < fast_share * 0.6 => CpuPlan::Attack(MoveId::LowPunch),
        _ if r < fast_share => CpuPlan::Attack(MoveId::LowKick),
        _ if r < fast_share + 0.20 => CpuPlan::Attack(MoveId::Sweep),
        // Low or high: the guard can only be in one place.
        _ if r < fast_share + 0.34 => {
            if dist < high_range { CpuPlan::Attack(MoveId::HighKick) }
            else { CpuPlan::Attack(MoveId::LowKick) }
        }
        _ if r < fast_share + 0.44 => CpuPlan::Attack(MoveId::Special),
        _ if r < fast_share + 0.53 && foe.throw_rest <= 0.0 => CpuPlan::Attack(MoveId::Throw),
        _ if r < 0.94 => CpuPlan::Block,
        _ => CpuPlan::Retreat,
    }
}

/// Turn the plan into the same Input struct the player produces, so the
/// CPU is playing the same game with the same rules and not a private
/// version of it.
fn cpu_input(g: &Game) -> Input {
    let me = g.p[1];
    let foe = g.p[0];
    let mut inp = Input::default();
    // A flier comes back down unless the plan is to be up there.
    if me.soaring() && g.cpu_plan != CpuPlan::Jump { inp.down = true; }
    let back = if me.facing > 0.0 { &mut inp.left } else { &mut inp.right };
    match g.cpu_plan {
        CpuPlan::Wait => {}
        CpuPlan::Block => { *back = true; if foe.act == Act::Attack && foe.mv == MoveId::Sweep { inp.down = true; } }
        CpuPlan::Retreat => { *back = true; }
        CpuPlan::Approach => {
            if me.facing > 0.0 { inp.right = true; } else { inp.left = true; }
        }
        CpuPlan::Jump => {
            if me.airborne() {
                // Already committed — throw the overhead. A jump-in that
                // never attacks is just a fighter volunteering to be hit
                // out of the air.
                // A flier out of arm's reach looks down and fires instead.
                let far = (foe.x - me.x).abs() > attack_range(&me, MoveId::LowPunch) * 1.5;
                if !me.flies() { inp.kick_high = true; }
                else if far { inp.kick_low = true; }
                else { inp.punch_low = true; }
            } else {
                inp.up = true;
                if me.facing > 0.0 { inp.right = true; } else { inp.left = true; }
            }
        }
        // A flier's kick is the laser, which ends the round: the CPU keeps it
        // for when it plans a special, and punches otherwise.
        CpuPlan::Attack(id) if me.flies() && id != MoveId::Special => {
            let _ = id;
            inp.punch_low = true;
        }
        CpuPlan::Attack(id) => match id {
            MoveId::LowPunch => inp.punch_low = true,
            MoveId::HighPunch => inp.punch_high = true,
            MoveId::LowKick => inp.kick_low = true,
            MoveId::HighKick => inp.kick_high = true,
            MoveId::Sweep => { inp.down = true; inp.kick_low = true; }
            MoveId::FlyingKick => { inp.up = true; inp.kick_high = true; }
            MoveId::Special => inp.special = true,
            // A throw is a punch thrown from close enough; cpu_think()
            // only asks for one when it is already that close.
            MoveId::Throw => inp.punch_low = true,
            _ => inp.punch_low = true,
        },
    }
    inp
}

// ---- update --------------------------------------------------------------

/// The keys one player answers to. Four attacks make a square: punches left,
/// kicks right, high on top, low below.
/// Two at one keyboard get squares that never overlap: the left player W A S
/// D with R T over F G, the right player the arrows with U I over J K.
/// Playing alone, player one also answers to the arrows and the generic fire
/// keys.
/// Stated once; the fight, the title menu and the select screen all read it.
struct Pad {
    up: &'static [KeyCode],
    down: &'static [KeyCode],
    left: &'static [KeyCode],
    right: &'static [KeyCode],
    punch: &'static [KeyCode],
    kick: &'static [KeyCode],
}

static P1_ALONE: Pad = Pad {
    up: &[BLIP_KEY_W, BLIP_KEY_UP],
    down: &[BLIP_KEY_S, BLIP_KEY_DOWN],
    left: &[BLIP_KEY_A, BLIP_KEY_LEFT],
    right: &[BLIP_KEY_D, BLIP_KEY_RIGHT],
    // The generic fire keys (Space, button 2) are the low attacks, the
    // pokes you open with; C and X are the high ones.
    punch: &[BLIP_KEY_F, BLIP_KEY_SPACE],
    kick: &[BLIP_KEY_G, BLIP_KEY_BUTTON2],
};

static P1_SHARING: Pad = Pad {
    up: &[BLIP_KEY_W],
    down: &[BLIP_KEY_S],
    left: &[BLIP_KEY_A],
    right: &[BLIP_KEY_D],
    punch: &[BLIP_KEY_F],
    kick: &[BLIP_KEY_G],
};

static P2: Pad = Pad {
    up: &[BLIP_KEY_UP],
    down: &[BLIP_KEY_DOWN],
    left: &[BLIP_KEY_LEFT],
    right: &[BLIP_KEY_RIGHT],
    punch: &[BLIP_KEY_J],
    kick: &[BLIP_KEY_K],
};

fn pad(mode: Mode, who: usize) -> &'static Pad {
    // Who comes first. Matching on the mode first handed player two
    // player one's keys whenever the game was in one-player mode,
    // which is every screen before the mode has been chosen.
    match (who, mode) {
        (1, _) => &P2,
        (_, Mode::Solo) => &P1_ALONE,
        _ => &P1_SHARING,
    }
}

fn any_held(keys: &[KeyCode]) -> bool { keys.iter().any(|k| key_held(*k)) }
fn any_pressed(keys: &[KeyCode]) -> bool { keys.iter().any(|k| key_pressed(*k)) }

fn human_input(g: &mut Game, who: usize) -> Input {
    let k = pad(g.mode, who);
    let mut inp = Input::default();
    inp.left = any_held(k.left);
    inp.right = any_held(k.right);
    inp.up = any_held(k.up);
    inp.down = any_held(k.down);
    // Two buttons; the stick picks the height. Held toward the opponent a
    // punch or kick is the overhead one, otherwise the low poke (down and
    // up turn them into the sweep and the flying kick in grounded_move()).
    let toward = if g.p[who].facing > 0.0 { inp.right } else { inp.left };
    let (punch, kick) = (any_pressed(k.punch), any_pressed(k.kick));
    let (plo, phi) = (punch && !toward, punch && toward);
    let (low, high) = (kick && !toward, kick && toward);
    if punch { g.punch_at[who] = g.now; }
    if kick { g.kick_at[who] = g.now; }
    // Punch and kick together inside a short window is the special: easy on a
    // stick, and in the way of no other move.
    let (pa, ka) = (g.punch_at[who], g.kick_at[who]);
    let together = (pa - ka).abs() <= 0.08 && g.now - pa.max(ka) <= 0.08
        && pa > 0.0 && ka > 0.0;
    if together {
        inp.special = true;
        g.punch_at[who] = -1.0;
        g.kick_at[who] = -1.0;
    } else {
        inp.punch_low = plo;
        inp.punch_high = phi;
        inp.kick_low = low;
        inp.kick_high = high;
    }
    inp
}

/// Is this attack thrown with a leg? The drawing asks to pick the limb, the
/// sound to decide whether a gi cracks.
fn is_kick(f: &Fighter) -> bool {
    matches!(f.mv, MoveId::LowKick | MoveId::HighKick | MoveId::Sweep
        | MoveId::JumpKick | MoveId::FlyingKick)
        || (f.mv == MoveId::Special && f.arch().special == Special::TalonKick)
}

/// How far into an attack's startup the leg stops chambering and starts
/// extending. Shared by the pose and the sound so they cannot drift.
pub(crate) const KICK_SNAP: f32 = 0.42;

fn hitstop_for(damage: i32, knockdown: bool, blocked: bool) -> f32 {
    if blocked { 3.0 * F }
    else if knockdown { 9.0 * F }
    else if damage >= 12 { 7.0 * F }
    else { 5.0 * F }
}

/// Serve this round's fruit when its time comes, and feed it to whoever
/// walks over it. Returns who ate.
fn fruit_step(g: &mut Game, dt: f32) -> Option<usize> {
    if g.fruit_due && g.clock <= FRUIT_AT {
        g.fruit_due = false;
        let kind = (g.opponent_index as i32 * 3 + g.round - 1).rem_euclid(FRUIT_POINTS.len() as i32);
        g.fruit = Fruit { x: WIN_W as f32 / 2.0, ttl: FRUIT_STAYS, kind: kind as usize };
    }
    if g.fruit.ttl <= 0.0 { return None; }
    g.fruit.ttl -= dt;
    let who = (0..2).find(|&i| {
        let f = &g.p[i];
        !f.airborne() && f.act != Act::Knockdown && (f.x - g.fruit.x).abs() < f.width() / 2.0 + 10.0
    })?;
    g.fruit.ttl = 0.0;
    let max = g.p[who].arch().health;
    g.p[who].health = (g.p[who].health + (max as f32 * FRUIT_HEAL).round() as i32).min(max);
    let (points, label) = FRUIT_POINTS[g.fruit.kind];
    g.pop(label, g.fruit.x, FLOOR_Y - 46.0);
    if who == 0 && g.mode == Mode::Solo { g.sess.add_score(points); }
    Some(who)
}

/// Two bolts thrown at each other meet and cancel. A laser is not a thing
/// that can be knocked out of the air: it burns through and carries on.
fn clash_bolts(g: &mut Game) -> Option<(f32, f32)> {
    for i in 0..g.bolts.len() {
        for j in i + 1..g.bolts.len() {
            let (a, b) = (g.bolts[i], g.bolts[j]);
            if !a.active || !b.active || a.owner == b.owner { continue; }
            if !rects_overlap(a.x - 10.0, a.y - 8.0, 20.0, 16.0, b.x - 10.0, b.y - 8.0, 20.0, 16.0) {
                continue;
            }
            for k in [i, j] {
                if g.p[g.bolts[k].owner].arch().special != Special::LaserVision {
                    g.bolts[k].active = false;
                }
            }
            return Some(((a.x + b.x) / 2.0, (a.y + b.y) / 2.0));
        }
    }
    None
}

/// Fighter `i` has just touched down. If that was a rage jump the floor
/// shakes, and the other fighter, if standing on it, goes down wherever they
/// are. Returns whether the floor shook. A jump ended by being hit is not a
/// landing.
fn land_quake(g: &mut Game, i: usize) -> bool {
    let stomp = std::mem::take(&mut g.p[i].stomp);
    if !stomp || !matches!(g.p[i].act, Act::Idle | Act::Attack) { return false; }
    g.quake_x = g.p[i].x;
    g.quake_t = QUAKE_SECS;
    g.shake = 0.3;
    let from = g.p[i].size();
    let d = &mut g.p[1 - i];
    if !d.airborne() && d.act != Act::Knockdown && !invulnerable(d) {
        d.hurt(QUAKE_DAMAGE, from);
        d.act = Act::Knockdown;
        d.t = 0.0;
        d.vx = 0.0;
        if i == 0 && g.mode == Mode::Solo { g.sess.add_score(QUAKE_DAMAGE * 10); }
    }
    true
}

/// Let each bar's ghost drain down to the health it stands for.
fn drain_ghosts(g: &mut Game, dt: f32) {
    for i in 0..2 {
        let (now, max) = (g.p[i].health as f32, g.p[i].arch().health as f32);
        g.ghost[i] = if g.ghost[i] <= now { now } else { (g.ghost[i] - max * 0.5 * dt).max(now) };
    }
}

fn update_fight(g: &mut Game, dt: f32, sfx: &Sounds) {
    // The freeze holds the fighters and the clock: a round should not
    // lose time to the impacts that make it worth watching.
    if g.hitstop > 0.0 {
        g.hitstop -= dt;
        for s in g.hitspark.iter_mut() { if s.ttl > 0.0 { s.ttl -= dt; } }
        return;
    }

    // The last ten seconds are counted out loud.
    let before = g.clock.ceil();
    g.clock -= dt;
    if g.clock < 10.0 && g.clock > 0.0 && g.clock.ceil() < before { play_sfx(&sfx.tick); }
    if fruit_step(g, dt).is_some() { play_sfx(&sfx.fruit); }

    #[cfg(not(target_arch = "wasm32"))]
    let p_in = if blip::bot::active() { bot::fight(g, dt) } else { human_input(g, 0) };
    #[cfg(target_arch = "wasm32")]
    let p_in = human_input(g, 0);

    // A plan runs for its delay unless the world changes: coming out of a
    // hit, or being attacked up close, forces a rethink.
    let jolted = g.p[1].act == Act::Hitstun || g.p[0].stomp
        || (g.p[0].act == Act::Attack && (g.p[0].x - g.p[1].x).abs() < attack_range(&g.p[0], MoveId::HighKick));
    g.cpu_delay -= dt;
    if g.cpu_delay <= 0.0 || (jolted && g.cpu_delay < 0.12) {
        g.cpu_plan = cpu_think(g);
        // The early turtles fight alone: the call is kept for opponents who
        // have had three fights to learn to guard low.
        if g.cpu_plan == CpuPlan::Attack(MoveId::Special) && g.difficulty < 0.0
            && g.p[1].arch().build == Build::Turtle && !g.p[1].called {
            g.cpu_plan = CpuPlan::Attack(MoveId::LowKick);
        }
        // Faster decisions as the ladder climbs — this is the difficulty
        // dial that actually matters, far more than damage numbers.
        g.cpu_delay = 0.38 - 0.18 * g.difficulty + (rand_int(0, 12) as f32) * 0.01;
    }
    // In versus the second fighter answers to a person, and the CPU's
    // plan is never consulted.
    let c_in = if g.mode == Mode::Versus { human_input(g, 1) } else { cpu_input(g) };

    // Face each other whenever both are free to turn.
    for i in 0..2 {
        let other = g.p[1 - i].x;
        let hovering = g.p[i].soaring() && g.p[i].act == Act::Air;
        if (g.p[i].free() && !g.p[i].airborne()) || hovering {
            g.p[i].facing = if other >= g.p[i].x { 1.0 } else { -1.0 };
        }
    }

    // Wrapped in a web there is no stick: no step, no guard, no blow.
    let p_in = if g.p[0].webbed > 0.0 { Input::default() } else { p_in };
    let c_in = if g.p[1].webbed > 0.0 { Input::default() } else { c_in };

    let close = face_off(&mut g.p);
    apply_input(&mut g.p[0], p_in, close[0], dt);
    apply_input(&mut g.p[1], c_in, close[1], dt);
    let was_air = [g.p[0].airborne(), g.p[1].airborne()];
    advance(&mut g.p[0], dt);
    advance(&mut g.p[1], dt);
    // Boots on boards: the touchdown is how a player hears they are on
    // the ground again.
    for i in 0..2 {
        if was_air[i] && !g.p[i].airborne() {
            play_sfx_volume(&sfx.land, 0.35 + 0.5 * g.p[i].land_force);
            if land_quake(g, i) { play_sfx(&sfx.quake); play_sfx(&sfx.crunch); }
        }
    }

    for i in 0..2 {
        g.p[i].x = clamp(g.p[i].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
    }
    separate(&mut g.p);

    // Kicks crack their gi when the shin starts to unfold, at `KICK_SNAP`,
    // the same split the drawing uses between chamber and extension.
    for i in 0..2 {
        let f = g.p[i];
        if f.act == Act::Attack && is_kick(&f) {
            let at = f.scaled(move_data(f.mv)).startup * F * KICK_SNAP;
            if f.t >= at && f.t - dt < at { play_sfx(&sfx.gi); }
        }
        // And the leap itself. The one move that crosses the stage was
        // the only one that made no sound leaving the ground.
        if f.act == Act::Attack && f.mv == MoveId::FlyingKick && f.t <= dt {
            play_sfx(&sfx.whoosh);
        }
        // The big ones are heard coming: the rage jump, the epic punch, the
        // sweep and the high kick.
        if f.act == Act::Attack && f.t <= dt {
            if f.stomp { play_sfx(&sfx.bellow); }
            else if f.epic_punch() || matches!(f.mv, MoveId::Sweep | MoveId::HighKick) {
                play_sfx_volume(&sfx.swing, 0.7);
            }
        }
    }

    // Specials fire their effect at the end of startup.
    for i in 0..2 {
        let f = g.p[i];
        if f.act == Act::Attack && f.mv == MoveId::Special && !f.hit_done {
            let m = f.scaled(move_data(MoveId::Special));
            let at = m.startup * F;
            if f.calling {
                if f.t >= at && f.t - dt < at {
                    g.p[i].hit_done = true;
                    g.call_turtles(i);
                    g.pop("TURTLES!", f.x, f.y - f.height() - 18.0);
                    play_sfx(&sfx.whistle);
                }
            } else if f.t >= at && f.t - dt < at {
                match f.arch().special {
                    Special::ChiBolt | Special::LaserVision => {
                        g.p[i].hit_done = true; // the bolt carries the hit, not the hand
                        g.spawn_bolt(i, m.damage);
                        play_sfx(match f.arch().build {
                            Build::Caped => &sfx.laser,
                            Build::Spider => &sfx.thwip,
                            _ => &sfx.projectile,
                        });
                    }
                    Special::BullRush => {
                        g.p[i].vx = f.facing * 430.0;
                        play_sfx(&sfx.whoosh);
                    }
                    Special::TalonKick => {
                        g.p[i].vy = JUMP_VY * 0.82;
                        g.p[i].y -= 0.5;
                        g.p[i].vx = f.facing * 150.0;
                        play_sfx(&sfx.whoosh);
                    }
                }
            }
        }
    }
    // Attacks, both ways, in the same frame — trades are part of the game.
    let holds = [p_in, c_in];
    for a in 0..2 {
        let d = 1 - a;
        let (mut atk, mut def) = (g.p[a], g.p[d]);
        let reach = atk.hit_box();
        let (dmg, blocked, knock) = resolve_hit(&mut atk, &mut def, holds[d]);
        g.p[a] = atk;
        g.p[d] = def;
        if dmg > 0 || blocked {
            let x = (g.p[a].x + g.p[d].x) / 2.0;
            let (sx, sy) = contact(reach, &g.p[a], &g.p[d]);
            g.spark(sx, sy, knock, blocked);
            g.hitstop = g.hitstop.max(hitstop_for(dmg, knock, blocked));
            let over = g.p[d].y - g.p[d].height() - 18.0;
            if dmg > 0 && g.p[a].countered {
                g.pop("COUNTER!", x, over);
                play_sfx(&sfx.counter);
            }
            if g.p[a].turned {
                g.p[a].turned = false;
                g.pop("GUARD IMPACT!", x, over);
                g.hitstop = g.hitstop.max(8.0 * F);
                play_sfx(&sfx.parry);
                push_apart(&mut g.p, 10.0);
            } else if blocked {
                play_sfx(&sfx.block);
                push_apart(&mut g.p, 6.0);
            } else {
                // The combo counter on the receiving end picks the
                // light hit, so a chain climbs. A knockdown gets the
                // crunch and the crowd with it.
                if knock {
                    play_sfx(&sfx.crunch);
                    play_sfx_volume(&sfx.cheer, 0.5);
                } else if dmg >= 12 {
                    play_sfx(&sfx.hit_heavy);
                } else {
                    let step = (g.p[d].combo.max(1) as usize - 1).min(2);
                    play_sfx(&sfx.hit_light[step]);
                }
                // A knockdown throws them clear, so the attacker is not
                // standing over an invulnerable wakeup. A throw pushes
                // little: it already leaves the thrower on top.
                let push = match (knock, g.p[a].mv) {
                    (_, MoveId::Throw) => 18.0,
                    (true, _) => 34.0,
                    _ => 9.0,
                };
                push_apart(&mut g.p, push);
                g.shake = if knock { 0.16 } else { 0.08 };
                if g.p[d].combo > 1 {
                    g.combo_shown = g.p[d].combo;
                    g.combo_t = 1.1;
                    g.combo_side = a;
                    // A combo is worth more than the sum of its hits —
                    // it is the part of the round the player earned.
                    if a == 0 { g.sess.add_score(dmg * 10 * g.p[d].combo); }
                } else if a == 0 {
                    g.sess.add_score(dmg * 10);
                }
            }
        }
    }

    // The turtles who were called in.
    for (dmg, blocked, knock) in update_helpers(g, dt, holds) {
        if blocked { play_sfx(&sfx.block); }
        else if knock { play_sfx(&sfx.crunch); play_sfx_volume(&sfx.cheer, 0.5); }
        else if dmg > 0 { play_sfx(&sfx.hit_light[1]); }
    }

    // Projectiles.
    if let Some((x, y)) = clash_bolts(g) {
        g.spark(x, y, false, true);
        play_sfx(&sfx.parry);
    }
    for i in 0..g.bolts.len() {
        if !g.bolts[i].active { continue; }
        g.bolts[i].x += g.bolts[i].vx * dt;
        g.bolts[i].y += g.bolts[i].vy * dt;
        // Off the side of the stage, or into the boards.
        if g.bolts[i].x < -20.0 || g.bolts[i].x > WIN_W as f32 + 20.0 || g.bolts[i].y > FLOOR_Y {
            g.bolts[i].active = false;
            continue;
        }
        let d = 1 - g.bolts[i].owner;
        let (dx, dy, dw, dh) = g.p[d].hurt_box();
        let (bx, by) = (g.bolts[i].x - 10.0, g.bolts[i].y - 8.0);
        if !rects_overlap(bx, by, 20.0, 16.0, dx, dy, dw, dh) { continue; }
        g.bolts[i].active = false;
        if invulnerable(&g.p[d]) { continue; }
        let guarding = !g.p[d].airborne()
            && holding_back(holds[d], g.p[d].facing)
            && matches!(g.p[d].act, Act::Idle | Act::Walk | Act::Crouch | Act::Block);
        if guarding {
            g.p[d].act = Act::Block;
            g.p[d].crouch_block = holds[d].down;
            g.p[d].stun = 12.0 * F;
            // A guard stops the laser killing, not hurting: it is ducked or
            // dodged, or it is paid for.
            let laser = g.p[1 - d].arch().special == Special::LaserVision;
            let chip = if laser { LASER_CHIP } else { (g.bolts[i].damage / CHIP_DIVISOR).max(1) };
            if laser || !g.p[d].shelled() { g.p[d].hurt(chip, g.p[1 - d].size()); }
            play_sfx(&sfx.block);
        } else {
            // The laser is the end of the round for whoever it touches.
            let laser = g.p[1 - d].arch().special == Special::LaserVision;
            // A web costs little in itself: what it costs is the next five
            // seconds.
            let web = g.p[1 - d].arch().build == Build::Spider;
            let dmg = if laser { g.p[d].health.max(1) }
                else if web { (g.bolts[i].damage / 3).max(1) }
                else { g.bolts[i].damage };
            g.p[d].hurt(dmg, g.p[1 - d].size());
            g.p[d].act = Act::Hitstun;
            g.p[d].stun = 18.0 * F;
            g.p[d].t = 0.0;
            if web && g.p[d].web() {
                let over = g.p[d].y - g.p[d].height() - 18.0;
                g.pop("WEBBED!", g.p[d].x, over);
                play_sfx(&sfx.thwip);
            }
            play_sfx(&sfx.hit_light[0]);
            g.shake = 0.08;
            if d == 1 { g.sess.add_score(dmg * 10); }
        }
        g.spark(g.bolts[i].x, g.bolts[i].y, false, guarding);
    }

    for s in g.hitspark.iter_mut() { if s.ttl > 0.0 { s.ttl -= dt; } }
    for w in g.pops.iter_mut() { if w.ttl > 0.0 { w.ttl -= dt; } }
    drain_ghosts(g, dt);
    if g.shake > 0.0 { g.shake -= dt; }
    if g.quake_t > 0.0 { g.quake_t -= dt; }
    if g.combo_t > 0.0 { g.combo_t -= dt; }

    // Round over?
    let ko = g.p[0].health <= 0 || g.p[1].health <= 0;
    let time = g.clock <= 0.0;
    if ko || time {
        g.result = round_result(g, ko);
        match g.result {
            RoundResult::P1 => { g.p[0].rounds += 1; g.p[1].act = Act::Defeat; g.p[0].act = Act::Victory;
                g.p[0].t = 0.0; g.p[1].t = 0.0; }
            RoundResult::P2 => { g.p[1].rounds += 1; g.p[0].act = Act::Defeat; g.p[1].act = Act::Victory;
                g.p[0].t = 0.0; g.p[1].t = 0.0; }
            // A double KO gives the round to both, the way the cabinets
            // did. It cannot loop forever: the match ends as soon as
            // either fighter reaches two, and a draw takes both there.
            RoundResult::Draw => { g.p[0].rounds += 1; g.p[1].rounds += 1; }
        }
        if ko {
            play_sfx(&sfx.ko);
            play_sfx_volume(&sfx.roar, 0.75);
            g.slow = SLOW_MO_SECS;
        }
        g.fruit.ttl = 0.0;
        g.call = round_call(g);
        if g.result == RoundResult::P1 && g.mode == Mode::Solo {
            g.sess.add_score(match g.call { "PERFECT" => 3000, "GREAT" => 1000, _ => 0 });
        }
        g.banner = if ko { "K.O." } else if survived(g) { "SURVIVED" } else { "TIME UP" };
        g.state = State::RoundEnd;
        g.phase.start(2.2);
        // Surviving a round is worth something, and so is surviving it
        // untouched — a player who wins 100-0 has done more than one who
        // wins 100-99.
        if g.result == RoundResult::P1 { g.sess.add_score(1000 + g.p[0].health * 20); }
        let side = match g.result { RoundResult::P1 => "won", RoundResult::P2 => "lost", RoundResult::Draw => "drew" };
        blip::bot::add(&format!("round_{side}_vs{}_{}", g.opponent_index, if ko { "ko" } else { "time" }), 1.0);
    }
}

/// Where a blow met the body: on the near face of the defender, at the
/// height the fist or foot came in.
fn contact(reach: Option<(f32, f32, f32, f32)>, atk: &Fighter, def: &Fighter) -> (f32, f32) {
    let (dx, dy, dw, dh) = def.hurt_box();
    let face = if atk.x < def.x { dx } else { dx + dw };
    let Some((hx, hy, hw, hh)) = reach else { return (face, dy + dh * 0.45) };
    (face.clamp(hx, hx + hw), (hy + hh / 2.0).clamp(dy + 6.0, dy + dh - 6.0))
}

/// Run the turtles who were called in: each waits his turn, runs at the
/// opponent, strikes once under the same rules as anyone, and runs on off
/// the far side. Returns what each blow did: (damage, blocked, knockdown).
fn update_helpers(g: &mut Game, dt: f32, holds: [Input; 2]) -> Vec<(i32, bool, bool)> {
    let mut landed = vec![];
    for k in 0..g.helpers.len() {
        let Some(mut h) = g.helpers[k] else { continue };
        let d = 1 - h.side;
        h.f.foe_size = g.p[d].size();
        if h.wait > 0.0 {
            h.wait -= dt;
        } else if h.f.act == Act::Attack {
            advance(&mut h.f, dt);
            let reach = h.f.hit_box();
            let mut def = g.p[d];
            let (dmg, blocked, knock) = resolve_hit(&mut h.f, &mut def, holds[d]);
            g.p[d] = def;
            if dmg > 0 || blocked {
                let (x, y) = contact(reach, &h.f, &g.p[d]);
                g.spark(x, y, knock, blocked);
                g.hitstop = g.hitstop.max(hitstop_for(dmg, knock, blocked));
                if d == 1 { g.sess.add_score(dmg * 10); }
                landed.push((dmg, blocked, knock));
            }
        } else {
            h.f.act = Act::Walk;
            h.f.t += dt;
            h.f.x += h.f.facing * HELPER_RUN * dt;
            let ahead = (g.p[d].x - h.f.x) * h.f.facing;
            if !h.struck && ahead <= attack_range(&h.f, h.mv) * 0.9 {
                h.struck = true;
                h.f.start_attack(h.mv);
            }
            if h.f.x < -40.0 || h.f.x > WIN_W as f32 + 40.0 {
                g.helpers[k] = None;
                continue;
            }
        }
        note_handover(&mut h.f, dt);
        g.helpers[k] = Some(h);
    }
    landed
}

/// Against the CPU fighter nothing can hurt, the player has only to be
/// standing at the bell.
fn survived(g: &Game) -> bool {
    g.mode == Mode::Solo && g.p[1].arch().invincible && !g.p[0].arch().invincible
        && g.p[0].health > 0 && g.p[1].health > 0
}

/// Who took the round: whoever has more health left, by knockout or at the
/// bell. The one exception is `survived`: a round nobody could have won on
/// health goes to the player for lasting it out.
fn round_result(g: &Game, ko: bool) -> RoundResult {
    if !ko && survived(g) { return RoundResult::P1; }
    if g.p[0].health == g.p[1].health { RoundResult::Draw }
    else if g.p[0].health > g.p[1].health { RoundResult::P1 }
    else { RoundResult::P2 }
}

/// The announcer's verdict, Tekken's: PERFECT for a round won untouched,
/// GREAT for one won on the last sliver of the bar. Nothing for the fighter
/// who cannot be hurt; it is no achievement of his.
fn round_call(g: &Game) -> &'static str {
    let winner = match g.result {
        RoundResult::P1 => g.p[0],
        RoundResult::P2 => g.p[1],
        RoundResult::Draw => return "",
    };
    let max = winner.arch().health;
    if winner.arch().invincible { "" }
    else if winner.health >= max { "PERFECT" }
    else if winner.health * 10 <= max { "GREAT" }
    else { "" }
}

fn update_round_end(g: &mut Game, dt: f32) {
    let dt = if g.slow > 0.0 { g.slow -= dt; dt * SLOW_MO } else { dt };
    // Keep the fighters moving after the round ends, or a mid-air finisher
    // hangs in the sky and the victory pose (driven by the action timer)
    // never plays.
    for i in 0..2 {
        advance(&mut g.p[i], dt);
        g.p[i].x = clamp(g.p[i].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
    }
    // A bolt in flight at the bell sails on off screen, harmless, rather than hang there.
    for b in g.bolts.iter_mut().filter(|b| b.active) {
        b.x += b.vx * dt;
        b.y += b.vy * dt;
        if b.x < -20.0 || b.x > WIN_W as f32 + 20.0 || b.y > FLOOR_Y { b.active = false; }
    }
    // The finishing blow's spark fades through the round end too.
    for s in g.hitspark.iter_mut() { if s.ttl > 0.0 { s.ttl -= dt; } }
    for w in g.pops.iter_mut() { if w.ttl > 0.0 { w.ttl -= dt; } }
    drain_ghosts(g, dt);
    if g.shake > 0.0 { g.shake -= dt; }
    if g.quake_t > 0.0 { g.quake_t -= dt; }
    if !g.phase.tick(dt) { return; }
    let (a, b) = (g.p[0].rounds, g.p[1].rounds);
    if a >= ROUNDS_TO_WIN || b >= ROUNDS_TO_WIN {
        g.state = State::MatchEnd;
        g.phase.start(2.4);
        g.banner = if g.mode == Mode::Versus {
            if a > b { "PLAYER 1 WINS" } else if b > a { "PLAYER 2 WINS" } else { "DRAW GAME" }
        } else if a > b { "WINNER" } else if b > a { "YOU LOSE" } else { "DRAW GAME" };
    } else {
        g.round += 1;
        g.start_round();
    }
}

/// One player's menu intent this frame, debounced. The menus take this rather
/// than the keyboard, so the whole front of the game (mode, cursors, lock-in,
/// no shared fighter) can be driven by a test.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
struct MenuIn { back: bool, fwd: bool, fire: bool }

/// What a menu transition asked to be heard. Returned rather than
/// played, for the same reason.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Cue { Step, Confirm }

fn read_menu(g: &mut Game) -> [MenuIn; 2] {
    let mut out = [MenuIn::default(); 2];
    for who in 0..2 {
        out[who] = MenuIn {
            back: menu_step(g, who, true),
            fwd: menu_step(g, who, false),
            fire: menu_fire(g, who),
        };
    }
    out
}

fn update_menus(g: &mut Game, m: [MenuIn; 2]) -> Option<Cue> {
    match g.state {
        State::Title => {
            // Player two announcing themselves is the choice, like a cabinet:
            // they reach for their own stick. It cannot be toggled back off.
            if g.menu == 0 && (m[1].back || m[1].fwd || m[1].fire) {
                g.menu = 1;
                web::set_mode(true);
                return Some(Cue::Step);
            }
            // The title is where the mode is chosen, on a stick and one
            // button, because that is all a cabinet has.
            if m[0].back || m[0].fwd {
                g.menu = 1 - g.menu;
                // The second station appears as the cursor lands on 2
                // PLAYERS, not on confirm: choosing is asking what two looks
                // like.
                web::set_players(if g.menu == 1 { 1 } else { TITLE_OPEN });
                return Some(Cue::Step);
            }
            // Once the second station is claimed, either player may
            // start the match — player two has as much right to it as
            // the player who put the coin in.
            if m[0].fire || (g.menu == 1 && m[1].fire) {
                g.mode = if g.menu == 0 { Mode::Solo } else { Mode::Versus };
                web::set_mode(g.mode == Mode::Versus);
                // a two-player game takes two coins: this is player two's
                if g.mode == Mode::Versus { web::spend_coin(); }
                g.state = State::Select;
                g.pick = 0;
                g.pick2 = FIGHTERS.len() - 1;
                g.locked = [false; 2];
                return Some(Cue::Confirm);
            }
            None
        }
        State::Select => {
            let versus = g.mode == Mode::Versus;
            let players = if versus { 2 } else { 1 };
            let mut cue = None;
            for who in 0..players {
                if g.locked[who] { continue; }
                let n = FIGHTERS.len();
                // A cursor steps over a fighter the other player has
                // already taken rather than stopping on one it is not
                // allowed to confirm.
                for (step, moved) in [(n - 1, m[who].back), (1, m[who].fwd)] {
                    if !moved { continue; }
                    let mut at = g.picked(who);
                    for _ in 0..n {
                        at = (at + step) % n;
                        if !g.taken_by_other(who, at) { break; }
                    }
                    g.set_pick(who, at);
                    cue = Some(Cue::Step);
                }
                if m[who].fire && !g.taken_by_other(who, g.picked(who)) {
                    g.locked[who] = true;
                    // Move the other player's cursor off a fighter now taken,
                    // so it is never parked on something it cannot confirm.
                    let other = 1 - who;
                    if versus && !g.locked[other] && g.picked(other) == g.picked(who) {
                        let mut at = g.picked(other);
                        for _ in 0..n {
                            at = (at + 1) % n;
                            if !g.taken_by_other(other, at) { break; }
                        }
                        g.set_pick(other, at);
                    }
                    cue = Some(Cue::Confirm);
                }
            }
            if (0..players).all(|w| g.locked[w]) {
                web::spend_coin();
                g.sess.reset(1);
                g.p[0].rounds = 0;
                g.p[1].rounds = 0;
                if versus { g.start_versus(); } else { g.start_match(0); }
                g.announce();
                cue = Some(Cue::Confirm);
            }
            cue
        }
        _ => None,
    }
}

/// The billing runs its time, or a press cuts it short once it has been seen.
fn update_vs(g: &mut Game, dt: f32, fire: bool) {
    let skip = fire && g.phase.remaining() < VS_SECS - 0.8;
    if g.phase.tick(dt) || skip {
        g.state = State::RoundIntro;
        g.phase.start(ROUND_INTRO);
    }
}

fn update_match_end(g: &mut Game, dt: f32) {
    if !g.phase.tick(dt) { return; }
    // Versus is one match. There is no ladder to climb and no score to
    // report — the result is the whole of it — so it goes back to the
    // title for the next pair.
    if g.mode == Mode::Versus {
        g.state = State::Over;
        g.phase.start(1.2);
        return;
    }
    let won = g.p[0].rounds > g.p[1].rounds;
    if !won {
        web::report_score(g.sess.score);
        g.state = State::Over;
        g.phase.start(1.2);
        return;
    }
    if g.opponent_index + 1 < RUNGS {
        g.p[0].rounds = 0;
        g.p[1].rounds = 0;
        g.start_match(g.opponent_index + 1);
        g.announce();
    } else {
        g.sess.add_score(5000);
        web::report_score(g.sess.score);
        g.state = State::Won;
        g.phase.start(1.2);
    }
}

// ---- assets --------------------------------------------------------------

const HIT_LIGHT_WAV: [&[u8]; 3] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/hit_light.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/hit_light2.wav")),
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/hit_light3.wav")),
];
const HIT_HEAVY_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/hit_heavy.wav"));
const CRUNCH_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/crunch.wav"));
const LAND_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/land.wav"));
const WHOOSH_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/whoosh.wav"));
const GI_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/gi.wav"));
const BLOCK_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/block.wav"));
const BELL_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/bell.wav"));
const KO_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/ko.wav"));
const PROJECTILE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/projectile.wav"));
const LASER_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/laser.wav"));
const QUAKE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/quake.wav"));
const PARRY_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/parry.wav"));
const COUNTER_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/counter.wav"));
const FRUIT_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/fruit.wav"));
const TICK_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/tick.wav"));
const GONG_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/gong.wav"));
const BELLOW_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/bellow.wav"));
const SWING_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/swing.wav"));
const WHISTLE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/whistle.wav"));
const THWIP_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/thwip.wav"));
const WIN_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/win.wav"));
const LOSE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/lose.wav"));
// The music is synthesised on the device, not baked in: a theme long enough
// not to repeat in a round is megabytes of PCM, more than the rest of the
// game.

struct Sounds {
    /// Three light hits at rising pitch. A combo picks the next one up,
    /// so a chain sounds like it is climbing rather than like the same
    /// hit played four times.
    hit_light: [blip::BlipSound; 3],
    hit_heavy: blip::BlipSound,
    /// A body hitting boards: the biggest thing in a round, with its own
    /// sound.
    crunch: blip::BlipSound,
    land: blip::BlipSound,
    /// The crowd that has been drawn watching every round in silence.
    cheer: blip::BlipSound,
    roar: blip::BlipSound,
    whoosh: blip::BlipSound,
    /// Cloth snapping taut, played on the frame a leg unfolds.
    gi: blip::BlipSound,
    block: blip::BlipSound,
    bell: blip::BlipSound,
    ko: blip::BlipSound,
    projectile: blip::BlipSound,
    laser: blip::BlipSound,
    /// The giant coming down.
    quake: blip::BlipSound,
    /// A guard impact: nothing else in the fight rings.
    parry: blip::BlipSound,
    counter: blip::BlipSound,
    fruit: blip::BlipSound,
    /// The menus: a wood block for a step, a gong for a choice.
    tick: blip::BlipSound,
    gong: blip::BlipSound,
    whistle: blip::BlipSound,
    thwip: blip::BlipSound,
    /// The giant's rage jump, and any blow with a whole body behind it.
    bellow: blip::BlipSound,
    swing: blip::BlipSound,
    /// The match decided: played alone, the stage music stopped under it.
    win: blip::BlipSound,
    lose: blip::BlipSound,
}

fn main_conf() -> blip::macroquad::window::Conf { window_conf("BRAWLER", WIN_W, WIN_H) }

#[blip::macroquad::main(main_conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    // Whole lit scenes, not lines on black: at full bloom every fighter
    // wears a halo and the names smear.
    blip.set_bloom(0.15);
    let mut g = Game::new();

    // Say so before the slow part (the themes take a second or two), or the
    // loading screen shows a dead second station.
    web::set_players(TITLE_OPEN);

    let sfx = Sounds {
        hit_light: [
            blip::audio::load_sound(HIT_LIGHT_WAV[0]).await,
            blip::audio::load_sound(HIT_LIGHT_WAV[1]).await,
            blip::audio::load_sound(HIT_LIGHT_WAV[2]).await,
        ],
        hit_heavy: blip::audio::load_sound(HIT_HEAVY_WAV).await,
        crunch: blip::audio::load_sound(CRUNCH_WAV).await,
        land: blip::audio::load_sound(LAND_WAV).await,
        // Built here rather than shipped, like the music: see
        // blip_assets::brawler::crowd_wav.
        cheer: blip::audio::load_sound(&blip_assets::brawler::crowd_wav(false)).await,
        roar: blip::audio::load_sound(&blip_assets::brawler::crowd_wav(true)).await,
        whoosh: blip::audio::load_sound(WHOOSH_WAV).await,
        gi: blip::audio::load_sound(GI_WAV).await,
        block: blip::audio::load_sound(BLOCK_WAV).await,
        bell: blip::audio::load_sound(BELL_WAV).await,
        ko: blip::audio::load_sound(KO_WAV).await,
        projectile: blip::audio::load_sound(PROJECTILE_WAV).await,
        laser: blip::audio::load_sound(LASER_WAV).await,
        quake: blip::audio::load_sound(QUAKE_WAV).await,
        parry: blip::audio::load_sound(PARRY_WAV).await,
        counter: blip::audio::load_sound(COUNTER_WAV).await,
        fruit: blip::audio::load_sound(FRUIT_WAV).await,
        tick: blip::audio::load_sound(TICK_WAV).await,
        gong: blip::audio::load_sound(GONG_WAV).await,
        whistle: blip::audio::load_sound(WHISTLE_WAV).await,
        bellow: blip::audio::load_sound(BELLOW_WAV).await,
        swing: blip::audio::load_sound(SWING_WAV).await,
        thwip: blip::audio::load_sound(THWIP_WAV).await,
        win: blip::audio::load_sound(WIN_WAV).await,
        lose: blip::audio::load_sound(LOSE_WAV).await,
    };
    // Built here rather than shipped: see the note where the other
    // assets are declared.
    use blip_assets::brawler::theme_wav;
    let mut music = Jukebox::new(&[|| theme_wav(0), || theme_wav(1), || theme_wav(2),
        || theme_wav(3), || theme_wav(4), || theme_wav(5)]);
    /// The theme of each stage, in stage order.
    // The fortress borrows the temple's theme and the rooftop the air base's.
    const STAGE_TRACK: [usize; STAGES] = [0, 1, 3, 4, 5, 1, 3];
    /// The title and select screens have a loop of their own — they were
    /// silent, and silence in front of a noisy game reads as not loaded.
    const SELECT_TRACK: usize = 2;
    music.start(SELECT_TRACK).await;
    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;
        g.now += dt;

        // Screenshot mode (BLIP_SCREENSHOT_OUT) drops straight into a fight,
        // timed so the kick is extended on the captured frame.
        // BLIP_SHOT_SCENE picks another picture (see brawler_card.sh): 1 the
        // laser from the air, 2 a rage jump landing, 3 the title screen, 4 to
        // 6 the air base, the bath house and the river village, 7 the billing
        // before the fourth fight, 8 a knockout, 9 a turtle calling the others,
        // 10 a fighter in a web, 11 the fortress, 12 the rooftop, 13 a kick off
        // the far leg, 14 a chain punch, 15 the card's poster.
        let scene = std::env::var("BLIP_SHOT_SCENE").ok().and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        if blip.screenshot_mode && scene == 3 {
            // The title is where the game already is.
        } else if blip.screenshot_mode && scene == 15 {
            g.poster = true;
        } else if blip.screenshot_mode && scene == 7 {
            if g.state != State::Vs {
                g.pick = 0;
                g.start_match(3);
                g.announce();
            }
        } else if blip.screenshot_mode && scene > 0 && !matches!(scene, 8 | 13 | 14) {
            shot_frame += 1;
            let (a, b, stage) = match scene {
                1 => (9, 8, 1),
                2 => (8, 4, 0),
                4 => (0, 2, 2),
                5 => (1, 3, 3),
                9 => (3, 0, 3),
                11 => (0, 9, 5),
                12 => (0, 7, 6),
                10 => (7, 0, 2),
                _ => (0, 8, 4),
            };
            if shot_frame == 1 {
                g.pick = a;
                g.start_match(0);
                g.stage = stage;
                g.state = State::Fight;
                g.p = [Fighter::new(a, 210.0, 1.0), Fighter::new(b, 440.0, -1.0)];
                g.ghost = [g.p[0].health as f32, g.p[1].health as f32];
                g.clock = ROUND_SECS * 0.6;
                g.fruit_due = false;
                if scene == 1 {
                    g.p[0].y = FLOOR_Y - 70.0;
                    g.p[0].act = Act::Air;
                }
            }
            g.cpu_plan = CpuPlan::Wait;
            g.cpu_delay = 99.0;
            if shot_frame == 4 && matches!(scene, 1 | 9 | 10) { g.p[0].start_attack(MoveId::Special); }
            if shot_frame == 4 && scene == 2 {
                g.p[0].y = FLOOR_Y - 130.0;
                g.p[0].act = Act::Air;
                g.p[0].start_attack(MoveId::JumpKick);
            }
        } else if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 {
                g.pick = 0;
                g.start_match(0);
                g.state = State::Fight;
                // The two the game began with, on the docks.
                g.p[1] = Fighter::new(1, 370.0, -1.0);
                g.stage = home_of(1);
                g.p[0].x = 286.0;
                g.p[1].health = (FIGHTERS[g.p[1].who].health as f32 * 0.55) as i32;
                // Scene 8: the same kick is the last of the round.
                if scene == 8 { g.p[1].health = 1; }
                g.p[0].health = (FIGHTERS[g.p[0].who].health as f32 * 0.8) as i32;
                g.clock = ROUND_SECS * 0.72;
                g.fruit_due = false;
            }
            // Hold the opponent still: left alone the CPU ducks and the high
            // kick sails over it.
            g.cpu_plan = CpuPlan::Wait;
            g.cpu_delay = 99.0;
            // Captured at BLIP_SCREENSHOT_FRAME=30: the kick has landed, the
            // hitstop flash (near-white in a still) has passed, and the spark
            // is still up.
            if shot_frame == 4 {
                // Scene 13 kicks off the far leg; 14 is the second punch of a chain.
                g.p[0].far_leads = scene == 13;
                g.p[0].since_punch = 0.1;
                g.p[0].start_attack(if scene == 14 { MoveId::HighPunch } else { MoveId::HighKick });
            }
            if scene == 14 && shot_frame > 14 {
                let m = move_data(MoveId::HighPunch);
                g.p[0].act = Act::Attack;
                g.p[0].t = (m.startup + m.active * 0.5) * F;
            }
            // Frames run on the wall clock: hold the splash open so the still
            // has it whichever frame it lands on.
            for s in g.hitspark.iter_mut().filter(|s| s.ttl > 0.0) { s.ttl = s.ttl.max(s.life * 0.55); }
            // And, short of a knockout, the kick at full stretch.
            if matches!(scene, 0 | 13) && g.p[0].hit_done && g.p[0].mv == MoveId::HighKick {
                let m = move_data(MoveId::HighKick);
                g.p[0].act = Act::Attack;
                g.p[0].t = (m.startup + m.active * 0.5) * F;
                g.hitstop = 0.0;
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        if blip::bot::active() { bot::menus(&mut g, blip::bot::clock()); }
        match g.state {
            State::Title | State::Select => {
                let m = read_menu(&mut g);
                if let Some(cue) = update_menus(&mut g, m) {
                    play_sfx(if cue == Cue::Step { &sfx.tick } else { &sfx.gong });
                }
            }
            State::Vs => {
                let m = read_menu(&mut g);
                update_vs(&mut g, dt, m[0].fire || m[1].fire);
            }
            State::RoundIntro => {
                // Each round opens with the fighters bowing to each other.
                if !g.bowed {
                    g.bowed = true;
                    for f in g.p.iter_mut() { f.act = Act::Bow; f.shown = Act::Bow; f.t = 0.0; }
                }
                for f in g.p.iter_mut() {
                    f.t += dt;
                    if f.act == Act::Bow && f.t >= BOW_TIME { f.act = Act::Idle; f.t = 0.0; }
                    note_handover(f, dt);
                }
                if g.phase.tick(dt) {
                    g.state = State::Fight;
                    play_sfx(&sfx.bell);
                }
            }
            State::Fight => update_fight(&mut g, dt, &sfx),
            State::RoundEnd => {
                update_round_end(&mut g, dt);
                if g.state == State::MatchEnd {
                    music.stop();
                    let lost = g.mode == Mode::Solo && g.p[0].rounds < g.p[1].rounds;
                    play_sfx(if lost { &sfx.lose } else { &sfx.win });
                }
            }
            State::MatchEnd => {
                update_match_end(&mut g, dt);
                match g.state {
                    State::Vs => play_sfx(&sfx.gong),
                    State::Won => { play_sfx(&sfx.gong); play_sfx(&sfx.roar); }
                    _ => {}
                }
            }
            State::Over | State::Won => {
                g.phase.tick(dt);
                // Either player can take it back to the title.
                let m = read_menu(&mut g);
                if !g.phase.active() && (m[0].fire || m[1].fire) {
                    g = Game::new();
                    web::set_players(TITLE_OPEN);
                }
            }
        }

        let want = match g.state {
            State::Title | State::Select => Some(SELECT_TRACK),
            State::Vs | State::RoundIntro | State::Fight | State::RoundEnd => {
                Some(STAGE_TRACK[g.stage % STAGES])
            }
            _ => None,
        };
        if let Some(track) = want { music.play(track); }
        if matches!(g.state, State::Title | State::Select) { music.warm_up().await; }

        blip.clear(BLIP_BLACK);
        draw::draw(&blip, &g);
        blip.next_frame(60).await;
    }
}

/// A menu step for one player, debounced by hand: a held direction moves the
/// cursor once. `who` picks whose keys are read, so two cursors share a
/// screen.
fn menu_step(g: &mut Game, who: usize, back: bool) -> bool {
    let k = pad(g.mode, who);
    let held = if back { any_held(k.left) || any_held(k.up) }
               else { any_held(k.right) || any_held(k.down) };
    let slot = &mut g.sel_held[who][usize::from(!back)];
    let stepped = held && !*slot;
    *slot = held;
    stepped
}

/// The same, for a player's confirm — any of their attack buttons.
fn menu_fire(g: &mut Game, who: usize) -> bool {
    let k = pad(g.mode, who);
    let held = any_held(k.punch) || any_held(k.kick);
    let slot = &mut g.sel_fire[who];
    let fired = held && !*slot;
    *slot = held;
    fired
}

#[cfg(test)]
mod tests;
