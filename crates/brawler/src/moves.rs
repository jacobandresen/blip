//! Physics constants and the move table: how fast things fall and fly, and
//! for every attack its startup, active and recovery frames, damage, reach
//! and the height it is guarded at.

use super::*;

/// How long the announcement of a second player stays up.
pub(crate) const CHALLENGER_SECS: f32 = 2.4;
/// How long the result of a match stays up: long enough to read what the
/// winner has to say.
pub(crate) const MATCH_END: f32 = 3.6;
/// How long the title is left alone before the attract mode starts.
pub(crate) const ATTRACT_AFTER: f32 = 12.0;
pub(crate) const GRAVITY: f32 = 1500.0;
pub(crate) const JUMP_VY: f32 = -600.0;
/// The flying kick's arc: lower than a jump and much faster forward.
pub(crate) const FLY_VY: f32 = -372.0;
pub(crate) const FLY_SPEED: f32 = 300.0;
/// The flying kick's landing cost, paid on the ground after the blow lands in
/// the air. One flat cost made it 19 frames minus on hit; connecting buys
/// most of the landing back, being blocked does not.
pub(crate) const FLY_LAND_LAG: f32 = 16.0 * F;
pub(crate) const FLY_HIT_LAG: f32 = 4.0 * F;
/// How long into a jump the flying kick is available: up and kick are never
/// the same frame for a human, and up jumps on the frame it is seen. A kick
/// inside this window levels the jump off into the flying kick.
pub(crate) const FLY_WINDOW: f32 = 6.0 * F;
/// A half-size fighter's lunge, in pixels a second at the first frame: about
/// 30px of ground by the time a kick lands, the reach the size took away.
pub(crate) const LUNGE: f32 = 520.0;
/// Points for landing the first blow of a round.
pub(crate) const FIRST_HIT: i32 = 500;
/// Seeing stars: this much damage taken faster than it drains (per second)
/// leaves a fighter helpless for a moment, once a round. Three good blows in
/// a row do it; a jab now and then never does.
pub(crate) const DAZE_AT: f32 = 46.0;
pub(crate) const DAZE_DRAIN: f32 = 9.0;
pub(crate) const DIZZY_SECS: f32 = 1.5;
/// A punch within this long of the last one is thrown with the other hand.
pub(crate) const CHAIN_PUNCH: f32 = 0.5;
/// What a guarded laser still costs.
pub(crate) const LASER_CHIP: i32 = 9;
/// A web holds for this long, or until this many blows have landed on it;
/// then no web holds for a while.
pub(crate) const WEB_SECS: f32 = 5.0;
pub(crate) const WEB_HITS: u8 = 2;
pub(crate) const WEB_REST: f32 = 3.0;
/// The turtles called in: how fast they run, how far apart they arrive, and
/// what each does when he gets there. All low, so one guard answers the lot.
pub(crate) const HELPER_RUN: f32 = 340.0;
pub(crate) const HELPER_GAP: f32 = 0.38;
pub(crate) const HELPER_MOVES: [MoveId; 3] = [MoveId::LowKick, MoveId::LowPunch, MoveId::Sweep];
/// How long after a throw the one thrown cannot be thrown again: the time on
/// the floor and a second on their feet.
pub(crate) const THROW_REST: f32 = 2.6;
/// What a small fighter's blows are worth against another as small. At 1.0
/// two turtles settle a round in five to fifteen seconds.
pub(crate) const SMALL_ON_SMALL: f32 = 0.7;
pub(crate) const LASER_SPEED: f32 = 600.0;
/// How high the level laser flies, per unit of size, and how tall its beam
/// is: over a crouch (74), and low and thin enough to jump (a jump peaks at
/// 120, and clears 91 a fifth of a second after leaving the ground).
pub(crate) const LASER_HEIGHT: f32 = 88.0;
pub(crate) const LASER_THICK: f32 = 6.0;
/// How fast a flier moves through the air, and how high the feet may go: the
/// head stays under the health bars.
pub(crate) const SOAR: f32 = 170.0;
pub(crate) const CEILING: f32 = 150.0;
/// No air control: the direction held at take-off is the commitment; a
/// steerable jump-in is a guess the defender cannot answer.
pub(crate) const AIR_DRIFT: f32 = 180.0;

/// The height a blow comes in at, which decides the guard that stops it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Level { Low, Mid, Overhead }

/// Every attack there is.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum MoveId { LowPunch, HighPunch, LowKick, HighKick, Sweep, Uppercut, JumpPunch, JumpKick,
    FlyingKick, Special, Throw }

/// One attack, in frames: `startup` before it can hit, `active` while it can,
/// `recovery` helpless afterwards. The gap between reach and recovery is its
/// personality.
#[derive(Copy, Clone)]
pub(crate) struct MoveData {
    pub(crate) startup: f32,
    pub(crate) active: f32,
    pub(crate) recovery: f32,
    pub(crate) damage: i32,
    /// Frames the defender is locked up for on hit, and on block. Hit
    /// always exceeds block, which is what makes landing one better than
    /// being blocked even when the damage is small.
    pub(crate) hitstun: f32,
    pub(crate) blockstun: f32,
    /// Reach from the fighter's centre, and how high off the floor the
    /// blow lands. Both are read by the drawing code too, so the pose
    /// cannot disagree with the hitbox.
    pub(crate) reach: f32,
    pub(crate) height: f32,
    pub(crate) thickness: f32,
    pub(crate) level: Level,
    pub(crate) knockdown: bool,
}

/// A `MoveData`, positionally: what the move table is written in.
pub(crate) const fn mv(startup: f32, active: f32, recovery: f32, damage: i32, hitstun: f32, blockstun: f32,
            reach: f32, height: f32, thickness: f32, level: Level, knockdown: bool) -> MoveData {
    MoveData { startup, active, recovery, damage, hitstun, blockstun, reach, height, thickness,
               level, knockdown }
}

/// The move table, read as trades: the jab (4f, 6 damage) wins scrambles and
/// nothing else; the sweep (10f startup, 20 recovery, knockdown) beats a
/// crouch that will not block low and loses to a whiff. Jump attacks are long
/// overheads, answered by hitting the jumper out of the air.
pub(crate) fn move_data(id: MoveId) -> MoveData {
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
        // The uppercut: down and punch. It reaches up where a jump comes
        // in and puts whoever it meets on the floor; it misses a croucher
        // altogether, and 24 frames of recovery is the price of guessing.
        MoveId::Uppercut  => mv(6.0,  6.0, 24.0, 15,  0.0,  12.0, 30.0, 110.0, 40.0, Level::Mid,     true),
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
