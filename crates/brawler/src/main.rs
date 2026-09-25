//! Brawler — a one-on-one fighting game, in tribute to Street Fighter II.
//!
//! What a fighting game actually is, under the sprites, is a
//! rock-paper-scissors played at arm's length: every attack beats
//! something and loses to something else, and the whole game is the
//! argument about distance while you decide which one to throw. This
//! build is that argument and nothing else — three fighters, two
//! locations, one CPU opponent at a time. No sprite sheets, no
//! super meters, no second player yet.
//!
//! The three things it takes seriously, because they are the game:
//!
//! 1. **Attack height.** Every attack is low, mid or overhead, and a
//!    block only works at the right height. Crouch-blocking eats sweeps
//!    and loses to jump-ins; stand-blocking is the reverse. Without this
//!    a fighting game is two people mashing, because blocking would
//!    have no cost.
//! 2. **Frame data.** Every attack has startup, active and recovery
//!    measured in frames, and is *punishable* in proportion to its reach
//!    and damage. A whiffed sweep should hurt you; a jab should not.
//!    This is what makes spacing a decision rather than a reflex.
//! 3. **What you can see.** The pose is assembled from the same numbers
//!    the hitboxes come from, so an arm that looks extended really is
//!    the thing that will hit you.

mod draw;

use blip::macroquad::input::KeyCode;
use blip::input::{key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_C,
    BLIP_KEY_D, BLIP_KEY_DOWN, BLIP_KEY_F, BLIP_KEY_G, BLIP_KEY_I, BLIP_KEY_J, BLIP_KEY_K,
    BLIP_KEY_LEFT, BLIP_KEY_R, BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_T,
    BLIP_KEY_U, BLIP_KEY_UP, BLIP_KEY_W, BLIP_KEY_X};
use blip::audio::play_sfx_volume;
use blip::{clamp, play_music, play_sfx, rand_int, rects_overlap, web,
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
/// Forty-five, not the arcade's sixty.
///
/// The clock should be a pressure, not the referee. At sixty every
/// round against a point-blank masher ran the full count — the CPU won
/// all of them on health, but the clock was doing the deciding, and a
/// round decided by arithmetic is a round nobody watched. Good play
/// finishes in twenty to thirty seconds here, so forty-five leaves room
/// for a careful fight and none for a stalemate.
const ROUND_SECS: f32 = 45.0;
const ROUNDS_TO_WIN: i32 = 2;

// ---- bodies --------------------------------------------------------------
/// The width of a fighter, for being hit and for being drawn.
///
/// These have to be the same number. It was 42 while the drawn torso was
/// 18 across, which meant attacks landing on empty air either side of a
/// fighter — a player judging distance by what they can see being wrong
/// by twelve pixels on each side, with nothing on screen to explain it.
/// The silhouette below (torso, shoulders, head) is built to fill this.
const BODY_W: f32 = 30.0;
const STAND_H: f32 = 120.0;
const CROUCH_H: f32 = 74.0;
/// How tall a fighter on their back is. They are drawn flat — the
/// highest thing on them is the knee they have pulled up — while the
/// box stayed at full standing height, so a flying kick sailing over a
/// prone body still connected with the air above it.
const PRONE_H: f32 = 34.0;

// ---- physics -------------------------------------------------------------
const GRAVITY: f32 = 1500.0;
const JUMP_VY: f32 = -600.0;
/// The flying kick's arc: lower than a jump and much faster forward.
const FLY_VY: f32 = -372.0;
const FLY_SPEED: f32 = 300.0;
/// What is left of the move once the feet touch. A jump attack is
/// cancelled by landing; this is the one that is not.
///
/// Two of them, because the blow connects in the air and the recovery
/// is paid on the ground, and the flight in between is time the
/// defender spends in hitstun and the attacker spends committed. At
/// one flat cost the move was nineteen frames *minus on hit*: landing
/// it handed the opponent a free turn, which is a move nobody should
/// ever throw. Connecting buys most of the landing back; being blocked
/// does not, and that is where the whole risk of it lives.
const FLY_LAND_LAG: f32 = 16.0 * F;
const FLY_HIT_LAG: f32 = 4.0 * F;
/// How long into a jump the flying kick is still available.
///
/// Pressing up and kick "together" is never the same frame for a
/// human, and up jumps on the frame it is seen — so asking for both at
/// once asked for a one-frame window, and the move was unreachable in
/// practice. A kick inside this window levels the jump off into the
/// flying kick; after it, a kick is the ordinary jump kick.
const FLY_WINDOW: f32 = 6.0 * F;
/// Air control is deliberately absent: the direction held at take-off is
/// the whole commitment. A jump you can steer mid-air turns every jump-in
/// into a guess the defender cannot answer, which is exactly the
/// rock-paper-scissors this game is built on.
const AIR_DRIFT: f32 = 180.0;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Level { Low, Mid, Overhead }

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum MoveId { LowPunch, HighPunch, LowKick, HighKick, Sweep, JumpPunch, JumpKick,
    FlyingKick, Special, Throw }

/// One attack, in frames.
///
/// `startup` is how long before it can hit, `active` how long it can,
/// `recovery` how long you are helpless afterwards. The gap between a
/// move's reach and its recovery is its whole personality: the sweep
/// reaches further than anything grounded and leaves you on the floor
/// for 20 frames if it misses.
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

/// The move table — the closest thing this game has to a rulebook.
///
/// Read it as a set of trades. The jab is 4 frames of startup and 6
/// damage: it wins scrambles and wins nothing else. The sweep is 10
/// frames of startup, 20 of recovery, and knocks down: it beats a
/// crouching opponent who will not block low and loses the round if you
/// throw it at someone standing just out of range. Jump attacks are
/// overheads with long reach, which is why the answer to them is to hit
/// the jumper out of the air rather than to block correctly.
fn move_data(id: MoveId) -> MoveData {
    match id {
        // Four normals, in a square: punch or kick, low or high. A
        // punch is the fast, short one at its height and a kick is the
        // slow, long one, so at every height there is something to open
        // with and something to commit to — and at neither height is
        // there anything a single guard covers.
        //
        // The high punch is the quickest overhead in the game and does
        // the least for it. That is the trade: eleven frames is barely
        // readable, so it has to be worth almost nothing when it lands,
        // or holding down-back stops being a decision and starts being
        // a mistake nobody can avoid making.
        MoveId::LowPunch  => mv(5.0,  3.0,  9.0,  5,  12.0, 7.0,  48.0, 30.0, 14.0, Level::Low,      false),
        MoveId::HighPunch => mv(11.0, 3.0, 16.0,  9,  16.0, 10.0, 50.0, 92.0, 14.0, Level::Overhead, false),
        // Two kick heights, and they are the argument: low has to be
        // crouch-blocked, high has to be blocked standing, and there is
        // nothing in between to cover both. A middle kick used to sit
        // here — a plain mid the guard stopped either way — and it was
        // the answer to every question the other two asked. Taking it
        // out is what makes the height of a kick a decision rather than
        // a preference.
        //
        // The low kick is fast and safe on block, so it is the poke you
        // open with. The high kick is slow, is an overhead, and is
        // punishable, so it is the one you commit to.
        MoveId::LowKick   => mv(6.0,  3.0, 10.0,  7,  14.0, 9.0,  58.0, 26.0, 14.0, Level::Low,      false),
        // No knockdown on the high kick. It was given one, and a
        // seventeen-damage overhead that also puts you on the floor is
        // not a mix-up, it is a win condition: a crouch-blocker went
        // from losing the exchange to losing the round, surviving one
        // fight in six. An overhead's job is to make holding down cost
        // something, not to end the argument.
        // A high kick does not reach as far *forward* as a mid one, and
        // it should not: the leg spends its length going up. All three
        // reference sheets show the same thing — the head-height kick
        // lands close, the mid kick is the long poke — and the drawing
        // cannot do otherwise without a longer leg. So the range here
        // is what a leg that goes to head height can actually cover,
        // and the move earns its keep by being an overhead instead.
        MoveId::HighKick  => mv(14.0, 5.0, 22.0, 17,  22.0, 13.0, 62.0, 98.0, 18.0, Level::Overhead, false),
        MoveId::Sweep     => mv(10.0, 4.0, 22.0, 13,  0.0,  12.0, 72.0, 18.0, 16.0, Level::Low,      true),
        MoveId::JumpPunch => mv(4.0,  8.0,  2.0,  9,  15.0, 9.0,  52.0, 34.0, 14.0, Level::Overhead, false),
        MoveId::JumpKick  => mv(6.0, 10.0,  2.0, 13,  17.0, 10.0, 66.0, 14.0, 16.0, Level::Overhead, false),
        // The flying kick: the only attack that closes a screen's worth
        // of ground on its own, and the only one whose cost is paid
        // after it is over.
        //
        // Every other jump attack is cancelled by touching the floor,
        // which is what makes a jump-in safe when it is blocked. This
        // one is not — see the landing in `advance` — so it is thrown
        // from outside the opponent's reach and lands inside it, and a
        // guard it does not beat is a free heavy punish. That is the
        // trade: the move that answers a turtle is also the move that
        // loses the round to one who saw it coming.
        //
        // No knockdown, for the same reason the high kick has none: an
        // overhead that also puts you on the floor stops being a way in
        // and starts being a win condition.
        // The blow lands level with the hips, not below them. At 24 the
        // leg ran downhill out of a reclining body, which is a stomp;
        // a flying kick is a straight line from the hip to the heel.
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
enum Special { ChiBolt, BullRush, TalonKick }

/// What a fighter is wearing and how they are put together.
///
/// Kept beside the numbers rather than in the drawing code, because
/// "who is that" and "what can they do" are the same question to a
/// player, and splitting them across two files is how a roster ends up
/// with three fighters who move differently and look identical.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Build {
    /// Karate gi: jacket to the elbows, trousers to mid-shin, bare feet.
    Gi,
    /// Stripped to the waist, heavy boots.
    Bare,
    /// A one-piece flight suit, wrapped forearms and shins.
    Suit,
}

/// A fighter's identity, in the only terms that change how they play.
#[derive(Copy, Clone)]
struct Archetype {
    name: &'static str,
    health: i32,
    walk: f32,
    back_walk: f32,
    jump_scale: f32,
    /// Scales every attack's damage and its reach, so an archetype is a
    /// real trade rather than a palette swap: Brutus hits hardest and
    /// stands closest, Kestrel pokes from outside and takes three hits
    /// to Brutus's two.
    power: f32,
    reach: f32,
    special: Special,
    special_name: &'static str,
    color: (f32, f32, f32),
    trim: (f32, f32, f32),
    skin: (f32, f32, f32),
    hair: (f32, f32, f32),
    build: Build,
    /// How thick the limbs and torso are drawn, around 1.0. The hurtbox
    /// is the same width for everyone — it has to be, or the fighters
    /// would not be trading the same trades — so bulk is the one place
    /// a heavyweight is allowed to look like one.
    bulk: f32,
}

const FIGHTERS: [Archetype; 3] = [
    Archetype {
        name: "RYUKA", health: 100, walk: 132.0, back_walk: 108.0, jump_scale: 1.0,
        power: 1.0, reach: 1.0, special: Special::ChiBolt, special_name: "CHI BOLT",
        color: (0.92, 0.92, 0.96), trim: (0.85, 0.25, 0.25),
        skin: (0.85, 0.68, 0.52), hair: (0.24, 0.16, 0.12), build: Build::Gi, bulk: 1.0,
    },
    Archetype {
        name: "BRUTUS", health: 120, walk: 96.0, back_walk: 78.0, jump_scale: 0.88,
        power: 1.35, reach: 0.88, special: Special::BullRush, special_name: "BULL RUSH",
        color: (0.30, 0.20, 0.26), trim: (0.95, 0.75, 0.2),
        skin: (0.74, 0.50, 0.34), hair: (0.20, 0.14, 0.12), build: Build::Bare, bulk: 1.28,
    },
    Archetype {
        name: "KESTREL", health: 88, walk: 164.0, back_walk: 140.0, jump_scale: 1.12,
        power: 0.82, reach: 1.12, special: Special::TalonKick, special_name: "TALON KICK",
        color: (0.25, 0.65, 0.85), trim: (0.95, 0.95, 0.35),
        skin: (0.90, 0.74, 0.60), hair: (0.55, 0.42, 0.18), build: Build::Suit, bulk: 0.86,
    },
];

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Act { Idle, Walk, Crouch, Air, Attack, Block, Hitstun, Knockdown, Victory, Defeat }

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
    /// An attack pressed while busy, and how long it stays remembered.
    ///
    /// Without this, a player pressing punch one frame before their
    /// recovery ends gets nothing, and the game feels like it is
    /// ignoring them — which, frame-accurately, it is. Every fighting
    /// game buffers; this is the difference between "I was too early"
    /// and "this game dropped my input".
    buffered: Option<MoveId>,
    buffer_t: f32,

    // ---- what is being *drawn*, as opposed to what is being played --
    //
    // Every pose in this game is a pure function of (action, timer), so
    // when the action changed the drawing changed on the same frame:
    // a fighter went from a guard to a fully chambered kick between one
    // picture and the next, with nothing in between. That is the single
    // biggest reason the animation read as wrong — not any one pose,
    // but the absence of anything joining them.
    //
    // So the drawing keeps its own short memory: the action it was in a
    // few frames ago, that action's timer frozen at the handover, and
    // how much of it is still showing. See draw::pose_of().
    /// The action last seen, for spotting the change, and the move and
    /// timer it was carrying when it was last seen.
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
}

/// How long a pose takes to hand over to the next one. Four frames:
/// long enough that nothing teleports, short enough that a jab still
/// lands on the frame the rules say it does.
pub(crate) const POSE_BLEND: f32 = 4.0 * F;

/// How long the knees take to absorb a landing and push back out of it.
pub(crate) const LAND_ABSORB: f32 = 12.0 * F;

/// How long getting up off the haunches takes. Dropping into a crouch
/// is gravity and is quick; standing up is not. Hitboxes still change
/// on the frame the action does — this is only how long the picture
/// takes to agree.
pub(crate) const RISE_BLEND: f32 = 9.0 * F;

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
    fn height(&self) -> f32 {
        if self.act == Act::Knockdown { PRONE_H }
        else if self.crouching() { CROUCH_H }
        else { STAND_H }
    }

    /// The box that can be hit. Deliberately the same box the fighter is
    /// drawn in: a hurtbox that does not match the picture is how a
    /// player learns to stop trusting their eyes.
    fn hurt_box(&self) -> (f32, f32, f32, f32) {
        let h = self.height();
        (self.x - BODY_W / 2.0, self.y - h, BODY_W, h)
    }

    /// Can this fighter act at all right now?
    fn free(&self) -> bool {
        matches!(self.act, Act::Idle | Act::Walk | Act::Crouch | Act::Block)
    }

    fn scaled(&self, m: MoveData) -> MoveData {
        let a = self.arch();
        MoveData {
            damage: ((m.damage as f32) * a.power).round() as i32,
            reach: m.reach * a.reach,
            ..m
        }
    }

    /// Where the current attack can hit, during its active window only.
    fn hit_box(&self) -> Option<(f32, f32, f32, f32)> {
        if self.act != Act::Attack || self.hit_done { return None; }
        let m = self.scaled(move_data(self.mv));
        let start = m.startup * F;
        let end = start + m.active * F;
        if self.t < start || self.t > end { return None; }
        let len = m.reach;
        let x = if self.facing > 0.0 { self.x + BODY_W / 2.0 } else { self.x - BODY_W / 2.0 - len };
        Some((x, self.y - m.height - m.thickness / 2.0, len, m.thickness))
    }

    fn start_attack(&mut self, id: MoveId) {
        self.chained = self.cancel_t > 0.0 && self.act == Act::Attack;
        self.act = Act::Attack;
        self.mv = id;
        self.t = 0.0;
        self.hit_done = false;
        self.hit_clean = false;
        self.cancel_t = 0.0;
        // The flying kick sets its own arc: flatter and faster than a
        // jump, so it crosses ground rather than gaining height. Thrown
        // a few frames into a jump it levels that jump off into the
        // same arc, which is what the move looks like anyway.
        if id == MoveId::FlyingKick {
            let launch = FLY_VY * self.arch().jump_scale;
            if self.airborne() {
                // Thrown a few frames into a jump, it has to reach the
                // same height it would off the floor, or the same move
                // is a flat dart one time and a lob the next. So the
                // rise already spent counts against the arc: whatever
                // is left of it is all they get.
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
struct Spark { x: f32, y: f32, ttl: f32, blocked: bool }

#[derive(Copy, Clone)]
struct Bolt { x: f32, y: f32, vx: f32, owner: usize, active: bool, damage: i32 }

// ---- the match -----------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum State { Title, Select, RoundIntro, Fight, RoundEnd, MatchEnd, Over, Won }

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum RoundResult { P1, P2, Draw }

/// One player against the ladder, or two against each other.
///
/// The difference runs deeper than who moves the second fighter: a
/// solo run is a ladder with a score and a difficulty curve, and a
/// versus match is one match that ends with a winner. They share the
/// round, and nothing else.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Mode { Solo, Versus }

struct Game {
    state: State,
    sess: Session,
    p: [Fighter; 2],
    bolts: [Bolt; 4],
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
    /// Frames where everything stops dead on contact.
    ///
    /// The cheapest and largest improvement available to a fighting
    /// game: without it a punch is a number leaving one health bar, and
    /// with it the punch has weight. It also does real work for
    /// readability — the freeze is exactly when both players need to
    /// see who got hit and what with.
    hitstop: f32,
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
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum CpuPlan { Wait, Approach, Retreat, Attack(MoveId), Jump, Block }

impl Game {
    fn new() -> Self {
        Game {
            state: State::Title,
            sess: Session::new(1),
            p: [Fighter::new(0, 200.0, 1.0), Fighter::new(1, 440.0, -1.0)],
            bolts: [Bolt { x: 0.0, y: 0.0, vx: 0.0, owner: 0, active: false, damage: 0 }; 4],
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
            hitspark: [Spark { x: 0.0, y: 0.0, ttl: 0.0, blocked: false }; 4],
            shake: 0.0,
            hitstop: 0.0,
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

    /// The two fighters the player did not pick, in order — the ladder.
    fn ladder(&self) -> [usize; 2] {
        let mut out = [0usize; 2];
        let mut n = 0;
        for i in 0..FIGHTERS.len() {
            if i != self.pick { out[n] = i; n += 1; }
        }
        out
    }

    /// Two players, one match, no ladder and no difficulty dial.
    fn start_versus(&mut self) {
        self.mode = Mode::Versus;
        self.opponent_index = 0;
        // Each fighter's own stage would be arbitrary with nobody
        // climbing anything, so the pick decides it: choose the same
        // fighter twice in a row and you get the same fight twice.
        self.stage = (self.pick + self.pick2) % 2;
        self.difficulty = 0.0;
        self.p[0] = Fighter::new(self.pick, 200.0, 1.0);
        self.p[1] = Fighter::new(self.pick2, 440.0, -1.0);
        self.round = 1;
        self.start_round();
    }

    fn start_match(&mut self, opponent_index: usize) {
        self.opponent_index = opponent_index;
        let foe = self.ladder()[opponent_index];
        self.stage = opponent_index % 2;
        self.difficulty = 0.35 + 0.4 * opponent_index as f32;
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
        self.clock = ROUND_SECS;
        self.banner = "ROUND";
        self.state = State::RoundIntro;
        self.phase.start(1.6);
        self.cpu_plan = CpuPlan::Wait;
        self.cpu_delay = 0.4;
    }

    fn spawn_bolt(&mut self, owner: usize, damage: i32) {
        let f = self.p[owner];
        for b in self.bolts.iter_mut() {
            if b.active { continue; }
            *b = Bolt {
                x: f.x + f.facing * (BODY_W / 2.0 + 10.0),
                y: f.y - 54.0,
                vx: f.facing * 300.0,
                owner,
                active: true,
                damage,
            };
            return;
        }
    }

    fn spark(&mut self, x: f32, y: f32, big: bool, blocked: bool) {
        let fresh = Spark { x, y, ttl: if big { 0.22 } else { 0.14 }, blocked };
        for s in self.hitspark.iter_mut() {
            if s.ttl <= 0.0 { *s = fresh; return; }
        }
        self.hitspark[0] = fresh;
    }
}

// ---- rules ---------------------------------------------------------------

/// Does a block at this stance stop an attack at this height?
///
/// The whole rock-paper-scissors in one function. Standing block covers
/// overheads and mids; crouch block covers lows and mids; neither covers
/// everything, so "block" is never simply the right answer.
fn blocks(level: Level, crouch_block: bool) -> bool {
    match level {
        Level::Mid => true,
        Level::Low => crouch_block,
        Level::Overhead => !crouch_block,
    }
}

/// Throws cannot be blocked, which is the whole point of them.
///
/// Block beats attack, throw beats block, attack beats throw — a
/// fighting game without the third corner has a stalemate in it, and
/// this one had a measurable one: two players who both knew how to hold
/// back ran the clock out, and a point-blank jab loop could not be
/// interrupted by anything at all.
fn unblockable(id: MoveId) -> bool { id == MoveId::Throw }

/// How close the two have to be for a punch to become a throw.
const THROW_RANGE: f32 = 52.0;

/// The distance between centres at which `id` would just touch the
/// other fighter. Reach plus both half-widths — the number a player is
/// judging by eye every time they decide whether to step in.
fn attack_range(f: &Fighter, id: MoveId) -> f32 {
    f.scaled(move_data(id)).reach + BODY_W
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

/// Apply one fighter's intent. Movement, jumping, crouching, blocking and
/// attack starts all live here so that the one rule that matters —
/// *you cannot act while you are busy* — is stated once.
/// Which attack these buttons are asking for, given where the fighter
/// is standing. One place decides, so a buffered press and a live one
/// can never disagree about what was asked for.
fn pressed_move(f: &Fighter, inp: Input, close: bool) -> Option<MoveId> {
    if f.airborne() { return air_move(f, inp); }
    grounded_move(inp, close)
}

fn air_move(f: &Fighter, inp: Input) -> Option<MoveId> {
    {
        if inp.any_punch() { return Some(MoveId::JumpPunch); }
        // Any kick button in the air is the jump kick. Height is a
        // ground decision — in the air the arc already decided it.
        // The exception is a kick still held with up, early enough in
        // the jump to turn it into a flying kick; see FLY_WINDOW.
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
    // Standing punch, right up against them, is a throw. Two buttons is
    // all this cabinet has, so the throw cannot have its own; proximity
    // is how the arcade originals did it too, and it makes the move
    // discoverable by walking in and pressing what you already know.
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
    // Up plus a kick is the flying kick, and it is checked before the
    // jump below so the two cannot both happen. Holding a direction is
    // already how the sweep is aimed, so this needs no motion and no
    // button the cabinet does not have.
    if inp.up && inp.any_kick() { return Some(MoveId::FlyingKick); }
    if inp.kick_low { return Some(MoveId::LowKick); }
    if inp.kick_high { return Some(MoveId::HighKick); }
    None
}

/// How long a pressed attack waits for its turn. Ten frames is long
/// enough to cover a human pressing early, short enough that it never
/// fires an attack the player has forgotten asking for.
const BUFFER: f32 = 10.0 * F;

/// A blocked special still costs the blocker a sixth of its damage.
///
/// Normals chip for nothing; only specials do. Without it two players
/// who both know how to block have no reason to ever stop, and the
/// round becomes a staring contest that the clock decides — measured at
/// seven rounds in twelve timing out. Chip is the pressure that makes
/// holding back a cost rather than a resting state, and it is why
/// throwing a special at a turtle is worth the recovery.
const CHIP_DIVISOR: i32 = 6;

/// Frames to cancel a landed light attack into a heavier one.
///
/// This is the whole reward for precision. A jab that lands is worth six
/// damage and nothing else; a jab that lands and is followed up is worth
/// the jab plus most of a kick, and the difference is a player who saw
/// the hit and reacted to it. Only light attacks that *connect* open the
/// window — whiffing one leaves you in recovery like everything else,
/// and a blocked one gets nothing, so pressing buttons at a guard is not
/// a combo, it is a turn given away.
const CANCEL_WINDOW: f32 = 14.0 * F;

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
    if f.buffer_t > 0.0 {
        f.buffer_t -= dt;
        if f.buffer_t <= 0.0 { f.buffered = None; }
    }

    // In the air, on the way up or down, and not yet committed to
    // anything: the jump-in is the whole reason to jump, so the attack
    // has to come out *now*.
    //
    // Being airborne is not being free — you cannot walk, crouch,
    // block or jump again — so the ordinary busy path below used to
    // catch a jumping kick, put it in the buffer, and hand it back as a
    // grounded kick on landing. The jump attacks were in the move table
    // and reachable from `pressed_move()`, and there was no way to
    // press one. Landing still ends whatever came out (see advance()),
    // which is what keeps this to one attack per jump.
    if f.airborne() && f.act == Act::Air {
        if let Some(id) = air_move(f, inp) {
            // An attack whose startup outlasts the fall never becomes
            // live: the feet touch first and landing ends it, so the
            // press is simply eaten. Measured at the last five frames
            // of every jump. Hold it for the ground instead, which is
            // what the buffer is for — and resolve it as the grounded
            // move, because a jump kick performed standing up is not
            // what anybody asked for.
            // Plus a couple of frames, or it comes out on exactly the
            // frame the feet touch and is cancelled having hit nothing.
            if f.air_time() >= (move_data(id).startup + 2.0) * F {
                f.start_attack(id);
            } else if let Some(g) = grounded_move(inp, close) {
                f.buffered = Some(g);
                f.buffer_t = BUFFER;
            }
        }
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

    f.t += dt;
    if f.cancel_t > 0.0 { f.cancel_t -= dt; }

    if f.airborne() || f.vy < 0.0 {
        f.vy += GRAVITY * dt;
        f.y += f.vy * dt;
        f.x += f.vx * dt;
        if f.y >= FLOOR_Y {
            f.y = FLOOR_Y;
            // How hard they arrived, for the drawing.
            f.land = LAND_ABSORB;
            f.land_force = (f.vy / -JUMP_VY).clamp(0.25, 1.0);
            f.vy = 0.0;
            f.vx = 0.0;
            // Landing cancels an air attack: the attack was the jump's
            // one commitment and it ends with the jump. The flying kick
            // is the exception — its recovery plays out on the ground,
            // which is the entire price of the distance it covered.
            if f.act == Act::Attack && f.mv == MoveId::FlyingKick {
                let m = f.scaled(move_data(f.mv));
                let total = (m.startup + m.active + m.recovery) * F;
                // A floor when it was blocked or whiffed — at least
                // this much left to pay. A ceiling when it connected —
                // at most this much, because otherwise the move's own
                // recovery is the binding cost and shortening the
                // landing changes nothing.
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

    match f.act {
        Act::Attack => {
            let m = f.scaled(move_data(f.mv));
            let total = (m.startup + m.active + m.recovery) * F;
            if !f.airborne() && f.t >= total { f.act = Act::Idle; f.t = 0.0; }
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

/// Spot one action becoming another and freeze what the drawing needs
/// to keep showing the old one.
///
/// Runs at the *end* of `advance`, and that is the point: a jump ends
/// when the feet touch, an attack when recovery runs out, hitstun when
/// the timer does — all inside `advance`. Looking at the top of the
/// next frame drew the frame it happened on with no blend at all.
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
        f.blend_len = if (was_low && !f.crouching()) || landed { RISE_BLEND } else { POSE_BLEND };
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
    if attacker.mv == MoveId::Throw && defender.airborne() { return (0, false, false); }
    attacker.hit_done = true;

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
        defender.health = (defender.health - chip).max(0);
        return (0, true, false);
    }

    let damage = ((m.damage as f32) * combo_scale(defender.combo)).round().max(1.0) as i32;
    defender.combo += 1;
    // A light attack that lands buys the right to follow it up.
    if matches!(attacker.mv, MoveId::LowPunch) && !attacker.chained {
        attacker.cancel_t = CANCEL_WINDOW;
    }
    defender.health = (defender.health - damage).max(0);
    attacker.hit_clean = true;
    // A flying kick that lands in a body stops flying, which is what
    // hitting something solid does — and it is what closes the gap
    // between the blow landing and the attacker being able to act.
    // Connecting early in the flight used to mean the defender's
    // hitstun ran out while the attacker was still in the air, so the
    // move was nineteen frames minus on hit. A *blocked* one carries
    // on through: being shoved past a guard you failed to beat is how
    // you end up standing in front of it to be punished.
    if attacker.mv == MoveId::FlyingKick {
        attacker.vx = 0.0;
        attacker.vy = attacker.vy.max(0.0);
    }
    if m.knockdown || defender.airborne() {
        defender.act = Act::Knockdown;
        defender.t = 0.0;
        defender.vy = 0.0;
        defender.vx = 0.0;
        defender.y = FLOOR_Y;
    } else {
        defender.act = Act::Hitstun;
        defender.stun = m.hitstun * F;
        defender.t = 0.0;
    }
    (damage, false, m.knockdown)
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
    let min = BODY_W * 0.82;
    let d = p[1].x - p[0].x;
    if d.abs() >= min { return; }
    let fix = (min - d.abs()) / 2.0 * if d >= 0.0 { 1.0 } else { -1.0 };
    let (lo, hi) = (WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
    p[0].x = clamp(p[0].x - fix, lo, hi);
    p[1].x = clamp(p[1].x + fix, lo, hi);
}

// ---- the CPU -------------------------------------------------------------

/// What the CPU wants to do, decided on a timer rather than every frame.
///
/// Deciding every frame produces a player that reacts in zero time and
/// is unbeatable for the wrong reason. Deciding on a delay that shortens
/// with difficulty produces one that can be baited — which is the thing
/// a fighting game CPU has to be, or the rock-paper-scissors is only
/// being played by one side.
fn cpu_think(g: &mut Game) -> CpuPlan {
    let me = g.p[1];
    let foe = g.p[0];
    let dist = (foe.x - me.x).abs();
    let roll = || (rand_int(0, 99) as f32) / 100.0;

    // Ranges come from the move table, not from guesses. They were
    // guesses — and they were shorter than the CPU's own legs, so it
    // spent entire rounds walking toward an opponent it was already
    // close enough to hit, taking the whole fight in the face. It threw
    // five attacks in twenty seconds.
    // The yardstick for "in kicking range". It was the middle kick,
    // which reached further than either of the two that are left, so
    // it is the high kick now and the CPU closes a little more before
    // it commits.
    let kick_range = attack_range(&me, MoveId::HighKick);
    let jab_range = attack_range(&me, MoveId::LowPunch);
    // The high kick is the shortest of the three, because a leg going
    // to head height spends its length going up. Throwing it from mid
    // kick range is throwing it at nothing, and a CPU that spends its
    // turns on attacks that cannot connect runs the clock out while
    // looking busy.
    let high_range = attack_range(&me, MoveId::HighKick);
    let foe_range = attack_range(&foe, MoveId::HighKick);


    // React to what they are doing, before deciding what to do.
    if foe.airborne() && dist < kick_range * 1.4 {
        return if roll() < 0.4 + 0.5 * g.difficulty {
            CpuPlan::Attack(MoveId::HighKick) // meet them on the way down
        } else {
            CpuPlan::Block
        };
    }
    if foe.act == Act::Attack && dist < foe_range * 1.2 {
        // The block that stops a mash. A CPU that only reconsiders on a
        // timer is a punching bag at close range, so being under attack
        // is one of the things that forces a fresh decision — see
        // update_fight().
        //
        // How badly it wants to block depends on what is coming. A sweep
        // or a special is worth respecting; a jab is worth trading with,
        // because six damage is cheaper than never taking a turn. Always
        // blocking is how the CPU ended up in a sixty-second stalemate
        // with someone holding down the punch button.
        let scary = move_data(foe.mv).damage >= 10 || move_data(foe.mv).knockdown;
        let want = if scary { 0.55 + 0.4 * g.difficulty } else { 0.22 + 0.2 * g.difficulty };
        if roll() < want { return CpuPlan::Block; }
        // Not blocking means taking the turn back, with the fastest
        // thing available — or, right up close, the move their guard
        // cannot help them with. (Retreating instead was tried and
        // measured: it hands a rushing opponent free ground and the CPU
        // stops winning those rounds at all.)
        if dist <= THROW_RANGE { return CpuPlan::Attack(MoveId::Throw); }
        if dist < jab_range { return CpuPlan::Attack(MoveId::LowPunch); }
    }
    // They committed to something slow and it missed: take the turn.
    if foe.act == Act::Attack && foe.hit_done && dist < kick_range && roll() < 0.5 + 0.4 * g.difficulty {
        return CpuPlan::Attack(MoveId::LowPunch);
    }

    // Read the guard. This is the game's own rock-paper-scissors played
    // from the other side, and without it the CPU has no answer at all
    // to someone who simply holds back: a crouch-blocker took *zero*
    // damage across a full sixty-second round, because crouch block
    // covers every attack the CPU was willing to throw.
    if foe.act == Act::Block && dist < kick_range * 1.1 {
        let read = roll() < 0.4 + 0.5 * g.difficulty;
        if read {
            return if dist <= THROW_RANGE {
                CpuPlan::Attack(MoveId::Throw)  // nothing guards against this
            } else if foe.crouch_block {
                // A low guard loses to an overhead, and there are two.
                // The high kick is the honest one — slow, telegraphed,
                // and punishable if it is read — so the CPU prefers it
                // and keeps the jump-in as the surprise.
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
        if me.arch().special == Special::ChiBolt && roll() < 0.2 + 0.4 * g.difficulty {
            return CpuPlan::Attack(MoveId::Special);
        }
        return CpuPlan::Approach;
    }
    if dist > kick_range * 1.25 {
        // Jumping in is only worth it from outside their reach; jumping
        // into a poke is a free knockdown for them, and the CPU used to
        // spend a third of every round on the floor learning that.
        if dist > foe_range * 1.3 && roll() < 0.10 + 0.14 * g.difficulty {
            // Two ways in, sharing one budget. The flying kick has to
            // be in the CPU's hands or the answer to being out-ranged
            // is a tool only the player has — but it is an overhead
            // added to a kit that already has two, and handed out on
            // top of the jump-in rather than instead of it, it stopped
            // a crouch-blocker surviving a round at all.
            return if roll() < 0.35 {
                CpuPlan::Attack(MoveId::FlyingKick)
            } else {
                CpuPlan::Jump
            };
        }
        // Out-ranged: standing outside your own reach and inside theirs
        // is the worst place on the stage, and walking out of it in a
        // straight line just means eating the poke on the way. A
        // short-armed fighter has to *cross* that gap.
        //
        // Without this, Brutus against a long-limbed opponent who keeps
        // pressing the button landed nothing at all: zero hits in a
        // forty-five second round, walking into a jab, being knocked
        // back out of range, and walking in again. The charge and the
        // jump-in are the two moves that cover ground, so out here they
        // are most of the answer rather than an occasional flourish.
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

    // In range. The mix is the personality: a spread wide enough that no
    // single answer covers it, weighted toward what this fighter is for.
    //
    // `r` is nudged down when the opponent is nearly out, which shifts
    // the whole mix toward attacking: a CPU that keeps politely blocking
    // a beaten opponent cannot close a round, and a round it cannot
    // close is one the clock decides. Measured at 60 seconds and 102-12
    // before this — dominant, and still unable to finish.
    let nearly_out = (foe.health as f32) < FIGHTERS[foe.who].health as f32 * 0.35;
    // A round that has run half its clock with both bars still full is
    // a round the clock is about to decide, and a round decided by
    // arithmetic is one nobody watched. Half-time with nothing on the
    // board is itself a reason to start forcing the issue — the same
    // nudge as smelling a finish, for the opposite reason.
    let stalling = g.clock < ROUND_SECS * 0.5
        && (foe.health as f32) > FIGHTERS[foe.who].health as f32 * 0.72
        && (me.health as f32) > FIGHTERS[me.who].health as f32 * 0.72;
    let r = if nearly_out || stalling { roll() * 0.7 } else { roll() };

    if dist >= jab_range {
        // At the edge of its reach: long pokes only. With the middle
        // kick gone the sweep is the only thing that genuinely reaches
        // out here, so it takes the share that used to be split.
        return match r {
            _ if r < 0.44 => CpuPlan::Attack(MoveId::Sweep),
            _ if r < 0.58 => CpuPlan::Attack(MoveId::LowKick),
            _ if r < 0.72 => CpuPlan::Attack(MoveId::Special),
            _ if r < 0.94 => CpuPlan::Approach,
            _ => CpuPlan::Block,
        };
    }

    // Right up close. Every fighter keeps a fast option here, including
    // the heavy: weighting Brutus entirely toward his big swings left
    // him with nothing that came out in time, and a fast opponent
    // simply jabbed him for forty-five seconds while neither health bar
    // moved. The heavy leans on the slow, heavy end of the same list
    // rather than being handed a different list.
    let heavy = me.arch().special == Special::BullRush;
    let fast_share = if heavy { 0.20 } else { 0.32 };
    // The bands have to leave room for the last two. Adding the new
    // kicks pushed the attacking share up to 0.96 and squeezed `Block`
    // down to nothing — the arm was unreachable, and the CPU silently
    // stopped guarding at close range entirely. A match arm that can
    // never be taken is a rule that has been deleted by arithmetic.
    match r {
        _ if r < fast_share * 0.6 => CpuPlan::Attack(MoveId::LowPunch),
        _ if r < fast_share => CpuPlan::Attack(MoveId::LowKick),
        _ if r < fast_share + 0.20 => CpuPlan::Attack(MoveId::Sweep),
        // The share the middle kick held goes to the choice it used to
        // let the CPU avoid: low or high, and the guard can only be in
        // one place.
        _ if r < fast_share + 0.34 => {
            if dist < high_range { CpuPlan::Attack(MoveId::HighKick) }
            else { CpuPlan::Attack(MoveId::LowKick) }
        }
        _ if r < fast_share + 0.44 => CpuPlan::Attack(MoveId::Special),
        _ if r < fast_share + 0.53 => CpuPlan::Attack(MoveId::Throw),
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
                inp.kick_high = true;
            } else {
                inp.up = true;
                if me.facing > 0.0 { inp.right = true; } else { inp.left = true; }
            }
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

/// The keys one player answers to.
///
/// Four attacks want a square, not a row: punches in the left column,
/// kicks in the right, high on the top row and low on the bottom, so
/// the buttons are laid out the way the move list is.
///
/// Two people at one keyboard need two such squares that never
/// overlap, so the split is the one a keyboard already has. The left
/// player drives with W A S D and hits with R T over F G; the right
/// player drives with the arrows and hits with U I over J K. Each has
/// a hand either side of the line a touch typist's hands already sit
/// on, and neither reaches across the other.
///
/// Playing alone, player one also answers to everything the cabinet
/// has — the arrows, and the two deck buttons — because a stick and
/// two buttons is all a cabinet is, and a one-player game has nobody
/// to take the arrows away for.
///
/// Stated once. The fight, the title menu and the select screen all
/// read it, and each used to spell the alias rule out again and could
/// get it wrong on its own.
struct Pad {
    up: &'static [KeyCode],
    down: &'static [KeyCode],
    left: &'static [KeyCode],
    right: &'static [KeyCode],
    punch_low: &'static [KeyCode],
    punch_high: &'static [KeyCode],
    kick_low: &'static [KeyCode],
    kick_high: &'static [KeyCode],
}

static P1_ALONE: Pad = Pad {
    up: &[BLIP_KEY_W, BLIP_KEY_UP],
    down: &[BLIP_KEY_S, BLIP_KEY_DOWN],
    left: &[BLIP_KEY_A, BLIP_KEY_LEFT],
    right: &[BLIP_KEY_D, BLIP_KEY_RIGHT],
    // The cabinet has one punch button and one kick button, and they
    // are the low ones, because those are the pokes you open with.
    punch_low: &[BLIP_KEY_F, BLIP_KEY_SPACE],
    punch_high: &[BLIP_KEY_R, BLIP_KEY_C],
    kick_low: &[BLIP_KEY_G, BLIP_KEY_BUTTON2],
    kick_high: &[BLIP_KEY_T, BLIP_KEY_X],
};

static P1_SHARING: Pad = Pad {
    up: &[BLIP_KEY_W],
    down: &[BLIP_KEY_S],
    left: &[BLIP_KEY_A],
    right: &[BLIP_KEY_D],
    punch_low: &[BLIP_KEY_F],
    punch_high: &[BLIP_KEY_R],
    kick_low: &[BLIP_KEY_G],
    kick_high: &[BLIP_KEY_T],
};

static P2: Pad = Pad {
    up: &[BLIP_KEY_UP],
    down: &[BLIP_KEY_DOWN],
    left: &[BLIP_KEY_LEFT],
    right: &[BLIP_KEY_RIGHT],
    punch_low: &[BLIP_KEY_J],
    punch_high: &[BLIP_KEY_U],
    kick_low: &[BLIP_KEY_K],
    kick_high: &[BLIP_KEY_I],
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
    // Which button was pressed is the height — the whole control scheme
    // for attack height, so a player never has to remember a motion to
    // aim one.
    let (plo, phi) = (any_pressed(k.punch_low), any_pressed(k.punch_high));
    let (low, high) = (any_pressed(k.kick_low), any_pressed(k.kick_high));
    let punch = plo || phi;
    if punch { g.punch_at[who] = g.now; }
    if low || high { g.kick_at[who] = g.now; }
    // Both buttons inside a short window is the special. Two buttons is
    // all this cabinet has, so the special cannot be a quarter-circle;
    // it is the one input a player can reliably hit on a stick with two
    // buttons, and it stays out of the way of every other move.
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

/// How long the world stops on contact — longer for the hits that are
/// supposed to feel heavy.
/// Is this attack thrown with a leg? The drawing code asks the same
/// question to decide which limb to throw, and the sound asks it to
/// decide whether there is a gi to crack.
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

fn update_fight(g: &mut Game, dt: f32, sfx: &Sounds) {
    // The freeze holds the fighters and the clock: a round should not
    // lose time to the impacts that make it worth watching.
    if g.hitstop > 0.0 {
        g.hitstop -= dt;
        for s in g.hitspark.iter_mut() { if s.ttl > 0.0 { s.ttl -= dt; } }
        return;
    }

    g.clock -= dt;

    let p_in = human_input(g, 0);

    // A plan runs for its delay, *except* when the world changes under
    // it: coming out of a hit, or being attacked at close range, is
    // exactly when a human would think again, and a CPU that waits out
    // its timer through a flurry of jabs is a heavy bag.
    let jolted = g.p[1].act == Act::Hitstun
        || (g.p[0].act == Act::Attack && (g.p[0].x - g.p[1].x).abs() < attack_range(&g.p[0], MoveId::HighKick));
    g.cpu_delay -= dt;
    if g.cpu_delay <= 0.0 || (jolted && g.cpu_delay < 0.12) {
        g.cpu_plan = cpu_think(g);
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
        if g.p[i].free() && !g.p[i].airborne() {
            g.p[i].facing = if other >= g.p[i].x { 1.0 } else { -1.0 };
        }
    }

    let close = (g.p[1].x - g.p[0].x).abs() <= THROW_RANGE;
    apply_input(&mut g.p[0], p_in, close, dt);
    apply_input(&mut g.p[1], c_in, close, dt);
    let was_air = [g.p[0].airborne(), g.p[1].airborne()];
    advance(&mut g.p[0], dt);
    advance(&mut g.p[1], dt);
    // Boots on boards. A jump is the one thing in the game that used to
    // happen in silence from take-off to landing, and the touchdown is
    // where a player finds out they are on the ground again.
    for i in 0..2 {
        if was_air[i] && !g.p[i].airborne() {
            play_sfx_volume(&sfx.land, 0.35 + 0.5 * g.p[i].land_force);
        }
    }

    for i in 0..2 {
        g.p[i].x = clamp(g.p[i].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
    }
    separate(&mut g.p);

    // Kicks crack their gi on the frame the shin starts to unfold.
    //
    // Not on the first frame of the move — the knee is still coming up
    // then and nothing is moving fast. `KICK_SNAP` is the same split
    // the drawing uses to divide the chamber from the extension, so
    // what you hear and what you see are the same event.
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
    }

    // Specials fire their effect at the end of startup.
    for i in 0..2 {
        let f = g.p[i];
        if f.act == Act::Attack && f.mv == MoveId::Special && !f.hit_done {
            let m = f.scaled(move_data(MoveId::Special));
            let at = m.startup * F;
            if f.t >= at && f.t - dt < at {
                match f.arch().special {
                    Special::ChiBolt => {
                        g.p[i].hit_done = true; // the bolt carries the hit, not the hand
                        g.spawn_bolt(i, m.damage);
                        play_sfx(&sfx.projectile);
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
    // Bull Rush slides along the floor; bleed it off so it ends.
    for i in 0..2 {
        if !g.p[i].airborne() && g.p[i].act == Act::Attack && g.p[i].mv == MoveId::Special {
            let slide = g.p[i].vx * dt;
            g.p[i].x = clamp(g.p[i].x + slide, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
            g.p[i].vx *= 1.0 - (6.0 * dt).min(1.0);
        }
    }

    // Attacks, both ways, in the same frame — trades are part of the game.
    let holds = [p_in, c_in];
    for a in 0..2 {
        let d = 1 - a;
        let (mut atk, mut def) = (g.p[a], g.p[d]);
        let (dmg, blocked, knock) = resolve_hit(&mut atk, &mut def, holds[d]);
        g.p[a] = atk;
        g.p[d] = def;
        if dmg > 0 || blocked {
            let x = (g.p[a].x + g.p[d].x) / 2.0;
            let y = g.p[d].y - g.p[d].height() * 0.55;
            g.spark(x, y, knock, blocked);
            g.hitstop = g.hitstop.max(hitstop_for(dmg, knock, blocked));
            if blocked {
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
                // A knockdown throws them clear — landing one used to
                // leave the attacker standing over the wakeup, which is
                // not a reward but a coin flip against invulnerability.
                // A throw is the exception: it already ends with the
                // thrower on top of the situation, and flinging them
                // full distance only means walking back in, which is how
                // a fight against someone who never moves turned into a
                // sixty-second commute.
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

    // Projectiles.
    for i in 0..g.bolts.len() {
        if !g.bolts[i].active { continue; }
        g.bolts[i].x += g.bolts[i].vx * dt;
        if g.bolts[i].x < -20.0 || g.bolts[i].x > WIN_W as f32 + 20.0 { g.bolts[i].active = false; continue; }
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
            g.p[d].stun = 12.0 * F;
            let chip = (g.bolts[i].damage / CHIP_DIVISOR).max(1);
            g.p[d].health = (g.p[d].health - chip).max(0);
            play_sfx(&sfx.block);
        } else {
            let dmg = g.bolts[i].damage;
            g.p[d].health = (g.p[d].health - dmg).max(0);
            g.p[d].act = Act::Hitstun;
            g.p[d].stun = 18.0 * F;
            g.p[d].t = 0.0;
            play_sfx(&sfx.hit_light[0]);
            g.shake = 0.08;
            if d == 1 { g.sess.add_score(dmg * 10); }
        }
        g.spark(g.bolts[i].x, g.bolts[i].y, false, guarding);
    }

    for s in g.hitspark.iter_mut() { if s.ttl > 0.0 { s.ttl -= dt; } }
    if g.shake > 0.0 { g.shake -= dt; }
    if g.combo_t > 0.0 { g.combo_t -= dt; }

    // Round over?
    let ko = g.p[0].health <= 0 || g.p[1].health <= 0;
    let time = g.clock <= 0.0;
    if ko || time {
        g.result = if g.p[0].health == g.p[1].health { RoundResult::Draw }
            else if g.p[0].health > g.p[1].health { RoundResult::P1 }
            else { RoundResult::P2 };
        match g.result {
            RoundResult::P1 => { g.p[0].rounds += 1; g.p[1].act = Act::Defeat; g.p[0].act = Act::Victory; }
            RoundResult::P2 => { g.p[1].rounds += 1; g.p[0].act = Act::Defeat; g.p[1].act = Act::Victory; }
            // A double KO gives the round to both, the way the cabinets
            // did. It cannot loop forever: the match ends as soon as
            // either fighter reaches two, and a draw takes both there.
            RoundResult::Draw => { g.p[0].rounds += 1; g.p[1].rounds += 1; }
        }
        if ko {
            play_sfx(&sfx.ko);
            play_sfx_volume(&sfx.roar, 0.75);
        }
        g.banner = if ko { "K.O." } else { "TIME UP" };
        g.state = State::RoundEnd;
        g.phase.start(2.2);
        // Surviving a round is worth something, and so is surviving it
        // untouched — a player who wins 100-0 has done more than one who
        // wins 100-99.
        if g.result == RoundResult::P1 { g.sess.add_score(1000 + g.p[0].health * 20); }
    }
}

fn update_round_end(g: &mut Game, dt: f32) {
    // Keep the fighters running. The round ends by assigning Victory
    // and Defeat outright and then only ticking a timer, so nothing
    // moved for the whole two seconds: a fighter who landed the
    // killing blow in mid-air hung there in the sky striking a pose,
    // and the victory animation — which is a function of the action
    // timer — never played at all, because the timer never advanced.
    for i in 0..2 {
        advance(&mut g.p[i], dt);
        g.p[i].x = clamp(g.p[i].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
    }
    if g.shake > 0.0 { g.shake -= dt; }
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

/// One player's menu intent this frame, already debounced.
///
/// The menus take this rather than reading the keyboard, so the whole
/// front of the game — mode, both cursors, the lock-in and the rule
/// that two players cannot bring the same fighter — can be driven by a
/// test. A flow that can only be exercised by a person with two hands
/// on a keyboard is a flow that never gets exercised.
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
            // The title is where the mode is chosen, on a stick and one
            // button, because that is all a cabinet has.
            if m[0].back || m[0].fwd {
                g.menu = 1 - g.menu;
                // The second station appears on the deck the moment the
                // cursor lands on 2 PLAYERS, not when it is confirmed.
                // A player choosing between one and two is asking what
                // two looks like, and the answer is a second stick
                // arriving in front of them — after the fact it is a
                // surprise, and on the select screen it is too late to
                // be an answer at all.
                web::set_mode(g.menu == 1);
                return Some(Cue::Step);
            }
            if m[0].fire {
                g.mode = if g.menu == 0 { Mode::Solo } else { Mode::Versus };
                web::set_mode(g.mode == Mode::Versus);
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
                    // Move the other player off it if they were sitting
                    // there. Their cursor was legal a moment ago and is
                    // not any more, and a cursor parked on something it
                    // can no longer confirm is a player pressing a
                    // button that does nothing and being told nothing.
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
                cue = Some(Cue::Confirm);
            }
            cue
        }
        _ => None,
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
    if g.opponent_index + 1 < 2 {
        g.p[0].rounds = 0;
        g.p[1].rounds = 0;
        g.start_match(g.opponent_index + 1);
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
// The music is not here. Every other asset is baked into the binary,
// but a theme long enough not to repeat inside a forty-five second
// round is about a megabyte of PCM and there are three of them — more
// than the whole rest of the game. The code that synthesises one is a
// couple of hundred lines, so the game builds its own at startup.

struct Sounds {
    /// Three light hits at rising pitch. A combo picks the next one up,
    /// so a chain sounds like it is climbing rather than like the same
    /// hit played four times.
    hit_light: [blip::BlipSound; 3],
    hit_heavy: blip::BlipSound,
    /// A body hitting boards. The biggest thing that happens in a
    /// round, and it used to share a sound with an ordinary heavy.
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
}

fn main_conf() -> blip::macroquad::window::Conf { window_conf("BRAWLER", WIN_W, WIN_H) }

#[blip::macroquad::main(main_conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

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
    };
    // Built here rather than shipped: see the note where the other
    // assets are declared.
    let mut music = Vec::with_capacity(3);
    for i in 0..3 {
        let wav = blip_assets::brawler::theme_wav(i);
        music.push(blip::audio::load_sound(&wav).await);
    }
    /// Which loop is playing. The title and select screens have one of
    /// their own — they were silent, and silence in front of a noisy
    /// game reads as something not having loaded.
    const SELECT_TRACK: usize = 2;
    let mut playing_track = usize::MAX;
    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;
        g.now += dt;

        // Screenshot mode (BLIP_SCREENSHOT_OUT) drops straight into a
        // fight: the cabinet's card should show the game being played,
        // not its title screen. Timed so the kick below is extended on
        // the frame that gets captured.
        if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 {
                g.pick = 0;
                g.start_match(0);
                g.state = State::Fight;
                g.p[0].x = 286.0;
                g.p[1].x = 370.0;
                g.p[1].health = (FIGHTERS[g.p[1].who].health as f32 * 0.55) as i32;
                g.p[0].health = (FIGHTERS[g.p[0].who].health as f32 * 0.8) as i32;
                g.clock = ROUND_SECS * 0.72;
            }
            // Hold the opponent standing still. Left to itself the CPU
            // ducks, and a high kick sails over a crouching fighter
            // exactly as it should — correct, and a card with nobody
            // hitting anybody on it. Walking it in instead put the two
            // of them inside each other.
            g.cpu_plan = CpuPlan::Wait;
            g.cpu_delay = 99.0;
            // The card is taken at BLIP_SCREENSHOT_FRAME=30: by then the
            // kick has landed, the hitstop flash has passed — it washes
            // the fighter it is on nearly white, which is right in
            // motion and wrong in a still — and the spark is still up.
            if shot_frame == 4 { g.p[0].start_attack(MoveId::HighKick); }
        }

        match g.state {
            State::Title | State::Select => {
                let m = read_menu(&mut g);
                if let Some(cue) = update_menus(&mut g, m) {
                    play_sfx(if cue == Cue::Step { &sfx.block } else { &sfx.bell });
                }
            }
            State::RoundIntro => {
                if g.phase.tick(dt) {
                    g.state = State::Fight;
                    play_sfx(&sfx.bell);
                }
            }
            State::Fight => update_fight(&mut g, dt, &sfx),
            State::RoundEnd => update_round_end(&mut g, dt),
            State::MatchEnd => update_match_end(&mut g, dt),
            State::Over | State::Won => {
                g.phase.tick(dt);
                // Either player can take it back to the title.
                let m = read_menu(&mut g);
                if !g.phase.active() && (m[0].fire || m[1].fire) {
                    g = Game::new();
                    web::set_mode(false);
                }
            }
        }

        let want = match g.state {
            State::Title | State::Select => Some(SELECT_TRACK),
            State::RoundIntro | State::Fight | State::RoundEnd | State::MatchEnd => {
                Some(g.stage)
            }
            _ => None,
        };
        if let Some(track) = want {
            if playing_track != track {
                play_music(&music[track]);
                playing_track = track;
            }
        }

        blip.clear(BLIP_BLACK);
        draw::draw(&blip, &g);
        blip.next_frame(60).await;
    }
}

/// Select-screen stepping, debounced by hand: on this screen a held
/// direction should move one step and wait, not slide through the roster.
/// A menu step for one player, debounced by hand: on these screens a
/// held direction must move the cursor once, not sixty times a second.
/// `who` picks whose keys are read, so two cursors can live on one
/// screen without either seeing the other's.
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
    let held = any_held(k.punch_low) || any_held(k.punch_high)
        || any_held(k.kick_low) || any_held(k.kick_high);
    let slot = &mut g.sel_fire[who];
    let fired = held && !*slot;
    *slot = held;
    fired
}

#[cfg(test)]
mod tests;
