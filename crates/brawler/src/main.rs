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

use blip::input::{btn1_pressed, btn2_pressed, key_held, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_DOWN,
    BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_UP, BLIP_KEY_W};
use blip::{clamp, play_music, play_sfx, rand_int, rects_overlap, web, window_conf, Blip,
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
const ROUND_SECS: f32 = 60.0;
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

// ---- physics -------------------------------------------------------------
const GRAVITY: f32 = 1500.0;
const JUMP_VY: f32 = -600.0;
/// Air control is deliberately absent: the direction held at take-off is
/// the whole commitment. A jump you can steer mid-air turns every jump-in
/// into a guess the defender cannot answer, which is exactly the
/// rock-paper-scissors this game is built on.
const AIR_DRIFT: f32 = 180.0;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Level { Low, Mid, Overhead }

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum MoveId { Jab, Kick, CrouchJab, Sweep, JumpPunch, JumpKick, Special }

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
        MoveId::Jab       => mv(4.0,  3.0,  7.0,  6,  13.0, 8.0,  54.0, 80.0, 14.0, Level::Mid,      false),
        MoveId::Kick      => mv(9.0,  4.0, 15.0, 12,  18.0, 11.0, 74.0, 68.0, 16.0, Level::Mid,      false),
        MoveId::CrouchJab => mv(5.0,  3.0,  8.0,  5,  12.0, 7.0,  52.0, 44.0, 14.0, Level::Mid,      false),
        MoveId::Sweep     => mv(10.0, 4.0, 22.0, 13,  0.0,  12.0, 72.0, 18.0, 16.0, Level::Low,      true),
        MoveId::JumpPunch => mv(4.0,  8.0,  2.0,  9,  15.0, 9.0,  52.0, 34.0, 14.0, Level::Overhead, false),
        MoveId::JumpKick  => mv(6.0, 10.0,  2.0, 13,  17.0, 10.0, 66.0, 14.0, 16.0, Level::Overhead, false),
        // Specials differ per fighter; this is the shape they share.
        MoveId::Special   => mv(11.0, 6.0, 26.0, 16,  20.0, 12.0, 74.0, 70.0, 18.0, Level::Mid,      true),
    }
}

// ---- fighters ------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Special { ChiBolt, BullRush, TalonKick }

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
}

const FIGHTERS: [Archetype; 3] = [
    Archetype {
        name: "RYUKA", health: 100, walk: 132.0, back_walk: 108.0, jump_scale: 1.0,
        power: 1.0, reach: 1.0, special: Special::ChiBolt, special_name: "CHI BOLT",
        color: (0.92, 0.92, 0.96), trim: (0.85, 0.25, 0.25),
    },
    Archetype {
        name: "BRUTUS", health: 120, walk: 96.0, back_walk: 78.0, jump_scale: 0.88,
        power: 1.35, reach: 0.88, special: Special::BullRush, special_name: "BULL RUSH",
        color: (0.55, 0.35, 0.22), trim: (0.95, 0.75, 0.2),
    },
    Archetype {
        name: "KESTREL", health: 88, walk: 164.0, back_walk: 140.0, jump_scale: 1.12,
        power: 0.82, reach: 1.12, special: Special::TalonKick, special_name: "TALON KICK",
        color: (0.25, 0.65, 0.85), trim: (0.95, 0.95, 0.35),
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
    crouch_block: bool,
    stun: f32,
    rounds: i32,
}

impl Fighter {
    fn new(who: usize, x: f32, facing: f32) -> Self {
        Fighter {
            who, x, y: FLOOR_Y, vy: 0.0, vx: 0.0, facing,
            health: FIGHTERS[who].health,
            act: Act::Idle, t: 0.0, mv: MoveId::Jab, hit_done: false,
            crouch_block: false, stun: 0.0, rounds: 0,
        }
    }

    fn arch(&self) -> Archetype { FIGHTERS[self.who] }
    fn airborne(&self) -> bool { self.y < FLOOR_Y - 0.01 }
    fn crouching(&self) -> bool {
        self.act == Act::Crouch || (self.act == Act::Block && self.crouch_block)
            || (self.act == Act::Attack && matches!(self.mv, MoveId::CrouchJab | MoveId::Sweep))
    }
    fn height(&self) -> f32 { if self.crouching() { CROUCH_H } else { STAND_H } }

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
        self.act = Act::Attack;
        self.mv = id;
        self.t = 0.0;
        self.hit_done = false;
    }
}

// ---- projectiles ---------------------------------------------------------

#[derive(Copy, Clone)]
struct Bolt { x: f32, y: f32, vx: f32, owner: usize, active: bool, damage: i32 }

// ---- the match -----------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum State { Title, Select, RoundIntro, Fight, RoundEnd, MatchEnd, Over, Won }

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum RoundResult { P1, P2, Draw }

struct Game {
    state: State,
    sess: Session,
    p: [Fighter; 2],
    bolts: [Bolt; 4],
    pick: usize,
    /// Which of the other two fighters this match is against, 0 then 1.
    opponent_index: usize,
    stage: usize,
    round: i32,
    clock: f32,
    phase: Timer,
    banner: &'static str,
    result: RoundResult,
    hitspark: [(f32, f32, f32); 4],
    shake: f32,
    /// Rising as the ladder goes on: reaction time shortens and the
    /// reads get better. See cpu_think().
    difficulty: f32,
    cpu_delay: f32,
    cpu_plan: CpuPlan,
    /// Held so a punch and a kick pressed together read as one input
    /// rather than two — see read_special().
    punch_at: f32,
    kick_at: f32,
    now: f32,
    /// Select-screen edge detection. Separate from the button
    /// timestamps above because a menu step and a punch are not the same
    /// event, and sharing the slots made each one eat the other's.
    sel_held: [bool; 2],
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
            pick: 0,
            opponent_index: 0,
            stage: 0,
            round: 1,
            clock: ROUND_SECS,
            phase: Timer::default(),
            banner: "",
            result: RoundResult::Draw,
            hitspark: [(0.0, 0.0, 0.0); 4],
            shake: 0.0,
            difficulty: 0.0,
            cpu_delay: 0.0,
            cpu_plan: CpuPlan::Wait,
            punch_at: -1.0,
            kick_at: -1.0,
            now: 0.0,
            sel_held: [false; 2],
        }
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

    fn spark(&mut self, x: f32, y: f32, big: bool) {
        for s in self.hitspark.iter_mut() {
            if s.2 <= 0.0 { *s = (x, y, if big { 0.22 } else { 0.14 }); return; }
        }
        self.hitspark[0] = (x, y, if big { 0.22 } else { 0.14 });
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
    punch: bool,
    kick: bool,
    special: bool,
}

/// Apply one fighter's intent. Movement, jumping, crouching, blocking and
/// attack starts all live here so that the one rule that matters —
/// *you cannot act while you are busy* — is stated once.
fn apply_input(f: &mut Fighter, inp: Input, dt: f32) {
    if !f.free() { return; }

    // Airborne fighters have already committed; they only get to attack.
    if f.airborne() {
        if inp.punch { f.start_attack(MoveId::JumpPunch); }
        else if inp.kick { f.start_attack(MoveId::JumpKick); }
        return;
    }

    if inp.special { f.start_attack(MoveId::Special); return; }

    let crouch = inp.down;
    if inp.punch {
        f.start_attack(if crouch { MoveId::CrouchJab } else { MoveId::Jab });
        return;
    }
    if inp.kick {
        f.start_attack(if crouch { MoveId::Sweep } else { MoveId::Kick });
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
    f.t += dt;

    if f.airborne() || f.vy < 0.0 {
        f.vy += GRAVITY * dt;
        f.y += f.vy * dt;
        f.x += f.vx * dt;
        if f.y >= FLOOR_Y {
            f.y = FLOOR_Y;
            f.vy = 0.0;
            f.vx = 0.0;
            // Landing cancels an air attack: the attack was the jump's
            // one commitment and it ends with the jump.
            if f.act == Act::Air || f.act == Act::Attack { f.act = Act::Idle; f.t = 0.0; }
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
            if f.stun <= 0.0 && f.act == Act::Hitstun { f.act = Act::Idle; f.t = 0.0; }
        }
        Act::Knockdown => {
            // Down, then up with a moment of invulnerability — otherwise
            // a knockdown is a free second hit and the game becomes one
            // sweep repeated.
            if f.t >= 1.15 { f.act = Act::Idle; f.t = 0.0; }
        }
        _ => {}
    }
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
    attacker.hit_done = true;

    // A defender can only block on the ground, holding away, and not
    // while already committed to something of their own.
    let guarding = !defender.airborne()
        && holding_back(hold, defender.facing)
        && matches!(defender.act, Act::Idle | Act::Walk | Act::Crouch | Act::Block);
    if guarding && blocks(m.level, defender.crouch_block || hold.down) {
        defender.act = Act::Block;
        defender.crouch_block = hold.down;
        defender.stun = m.blockstun * F;
        return (0, true, false);
    }

    defender.health = (defender.health - m.damage).max(0);
    if m.knockdown || defender.airborne() {
        defender.act = Act::Knockdown;
        defender.t = 0.0;
        defender.vy = 0.0;
        defender.y = FLOOR_Y;
    } else {
        defender.act = Act::Hitstun;
        defender.stun = m.hitstun * F;
        defender.t = 0.0;
    }
    (m.damage, false, m.knockdown)
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
    let arch = me.arch();
    let roll = || (rand_int(0, 99) as f32) / 100.0;

    // Answer what the opponent is doing first — that is what reading
    // looks like from the outside.
    if foe.airborne() && dist < 130.0 && roll() < g.difficulty {
        return CpuPlan::Attack(MoveId::Kick); // anti-air
    }
    if foe.act == Act::Attack && dist < 100.0 {
        // Block, in proportion to difficulty. A CPU that always blocks
        // is a wall; one that never does is a punching bag.
        if roll() < 0.35 + 0.5 * g.difficulty { return CpuPlan::Block; }
    }
    // Punish: they threw something slow and missed.
    if foe.act == Act::Attack && foe.hit_done && dist < 90.0 && roll() < g.difficulty {
        return CpuPlan::Attack(MoveId::Kick);
    }

    let close = 58.0 * arch.reach;
    let poke = 86.0 * arch.reach;
    if dist > 190.0 {
        if matches!(arch.special, Special::ChiBolt) && roll() < 0.35 * g.difficulty + 0.1 {
            return CpuPlan::Attack(MoveId::Special);
        }
        return CpuPlan::Approach;
    }
    if dist > poke {
        if roll() < 0.12 + 0.12 * g.difficulty { return CpuPlan::Jump; }
        return CpuPlan::Approach;
    }
    if dist < close && roll() < 0.25 {
        return CpuPlan::Retreat;
    }
    // In range: pick something. The mix is what stops a player from
    // finding one answer and holding it.
    let r = roll();
    if r < 0.34 { CpuPlan::Attack(MoveId::Jab) }
    else if r < 0.58 { CpuPlan::Attack(MoveId::Kick) }
    else if r < 0.76 { CpuPlan::Attack(MoveId::Sweep) }
    else if r < 0.88 { CpuPlan::Attack(MoveId::Special) }
    else { CpuPlan::Block }
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
            inp.up = true;
            if me.facing > 0.0 { inp.right = true; } else { inp.left = true; }
        }
        CpuPlan::Attack(id) => match id {
            MoveId::Jab => inp.punch = true,
            MoveId::Kick => inp.kick = true,
            MoveId::Sweep => { inp.down = true; inp.kick = true; }
            MoveId::CrouchJab => { inp.down = true; inp.punch = true; }
            MoveId::Special => inp.special = true,
            _ => inp.punch = true,
        },
    }
    inp
}

// ---- update --------------------------------------------------------------

fn player_input(g: &mut Game) -> Input {
    let mut inp = Input::default();
    inp.left = key_held(BLIP_KEY_LEFT) || key_held(BLIP_KEY_A);
    inp.right = key_held(BLIP_KEY_RIGHT) || key_held(BLIP_KEY_D);
    inp.up = key_held(BLIP_KEY_UP) || key_held(BLIP_KEY_W);
    inp.down = key_held(BLIP_KEY_DOWN) || key_held(BLIP_KEY_S);
    let punch = btn1_pressed();
    let kick = btn2_pressed();
    if punch { g.punch_at = g.now; }
    if kick { g.kick_at = g.now; }
    // Both buttons inside a short window is the special. Two buttons is
    // all this cabinet has, so the special cannot be a quarter-circle;
    // it is the one input a player can reliably hit on a stick with two
    // buttons, and it stays out of the way of every other move.
    let together = (g.punch_at - g.kick_at).abs() <= 0.08
        && g.now - g.punch_at.max(g.kick_at) <= 0.08
        && g.punch_at > 0.0 && g.kick_at > 0.0;
    if together {
        inp.special = true;
        g.punch_at = -1.0;
        g.kick_at = -1.0;
    } else {
        inp.punch = punch;
        inp.kick = kick;
    }
    inp
}

fn update_fight(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.clock -= dt;

    let p_in = player_input(g);

    g.cpu_delay -= dt;
    if g.cpu_delay <= 0.0 {
        g.cpu_plan = cpu_think(g);
        // Faster decisions as the ladder climbs — this is the difficulty
        // dial that actually matters, far more than damage numbers.
        g.cpu_delay = 0.38 - 0.18 * g.difficulty + (rand_int(0, 12) as f32) * 0.01;
    }
    let c_in = cpu_input(g);

    // Face each other whenever both are free to turn.
    for i in 0..2 {
        let other = g.p[1 - i].x;
        if g.p[i].free() && !g.p[i].airborne() {
            g.p[i].facing = if other >= g.p[i].x { 1.0 } else { -1.0 };
        }
    }

    apply_input(&mut g.p[0], p_in, dt);
    apply_input(&mut g.p[1], c_in, dt);
    advance(&mut g.p[0], dt);
    advance(&mut g.p[1], dt);

    for i in 0..2 {
        g.p[i].x = clamp(g.p[i].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
    }
    separate(&mut g.p);

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
            g.spark(x, y, knock);
            if blocked {
                play_sfx(&sfx.block);
                push_apart(&mut g.p, 6.0);
            } else {
                play_sfx(if knock || dmg >= 12 { &sfx.hit_heavy } else { &sfx.hit_light });
                push_apart(&mut g.p, if knock { 14.0 } else { 9.0 });
                g.shake = if knock { 0.16 } else { 0.08 };
                if a == 0 { g.sess.add_score(dmg * 10); }
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
            play_sfx(&sfx.block);
        } else {
            let dmg = g.bolts[i].damage;
            g.p[d].health = (g.p[d].health - dmg).max(0);
            g.p[d].act = Act::Hitstun;
            g.p[d].stun = 18.0 * F;
            g.p[d].t = 0.0;
            play_sfx(&sfx.hit_light);
            g.shake = 0.08;
            if d == 1 { g.sess.add_score(dmg * 10); }
        }
        g.spark(g.bolts[i].x, g.bolts[i].y, false);
    }

    for s in g.hitspark.iter_mut() { if s.2 > 0.0 { s.2 -= dt; } }
    if g.shake > 0.0 { g.shake -= dt; }

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
        if ko { play_sfx(&sfx.ko); }
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
    if !g.phase.tick(dt) { return; }
    let (a, b) = (g.p[0].rounds, g.p[1].rounds);
    if a >= ROUNDS_TO_WIN || b >= ROUNDS_TO_WIN {
        g.state = State::MatchEnd;
        g.phase.start(2.4);
        g.banner = if a > b { "WINNER" } else if b > a { "YOU LOSE" } else { "DRAW GAME" };
    } else {
        g.round += 1;
        g.start_round();
    }
}

fn update_match_end(g: &mut Game, dt: f32) {
    if !g.phase.tick(dt) { return; }
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

const HIT_LIGHT_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/hit_light.wav"));
const HIT_HEAVY_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/hit_heavy.wav"));
const WHOOSH_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/whoosh.wav"));
const BLOCK_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/block.wav"));
const BELL_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/bell.wav"));
const KO_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/ko.wav"));
const PROJECTILE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/projectile.wav"));
const MUSIC_DOCK_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music_dock.wav"));
const MUSIC_TEMPLE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/music_temple.wav"));

struct Sounds {
    hit_light: blip::BlipSound,
    hit_heavy: blip::BlipSound,
    whoosh: blip::BlipSound,
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
        hit_light: blip::audio::load_sound(HIT_LIGHT_WAV).await,
        hit_heavy: blip::audio::load_sound(HIT_HEAVY_WAV).await,
        whoosh: blip::audio::load_sound(WHOOSH_WAV).await,
        block: blip::audio::load_sound(BLOCK_WAV).await,
        bell: blip::audio::load_sound(BELL_WAV).await,
        ko: blip::audio::load_sound(KO_WAV).await,
        projectile: blip::audio::load_sound(PROJECTILE_WAV).await,
    };
    let music = [
        blip::audio::load_sound(MUSIC_DOCK_WAV).await,
        blip::audio::load_sound(MUSIC_TEMPLE_WAV).await,
    ];
    let mut playing_stage = usize::MAX;
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
            }
            if shot_frame == 16 { g.p[0].start_attack(MoveId::Kick); }
        }

        match g.state {
            State::Title => {
                if btn1_pressed() || btn2_pressed() {
                    g.state = State::Select;
                    g.pick = 0;
                }
            }
            State::Select => {
                if key_pressed_once(&mut g, true) { g.pick = (g.pick + FIGHTERS.len() - 1) % FIGHTERS.len(); }
                if key_pressed_once(&mut g, false) { g.pick = (g.pick + 1) % FIGHTERS.len(); }
                if btn1_pressed() || btn2_pressed() {
                    web::spend_coin();
                    g.sess.reset(1);
                    g.p[0].rounds = 0;
                    g.p[1].rounds = 0;
                    g.start_match(0);
                    play_sfx(&sfx.bell);
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
                if !g.phase.active() && (btn1_pressed() || btn2_pressed()) {
                    g = Game::new();
                }
            }
        }

        if matches!(g.state, State::RoundIntro | State::Fight | State::RoundEnd) {
            if playing_stage != g.stage {
                play_music(&music[g.stage]);
                playing_stage = g.stage;
            }
        }

        blip.clear(BLIP_BLACK);
        draw::draw(&blip, &g);
        blip.next_frame(60).await;
    }
}

/// Select-screen stepping, debounced by hand: on this screen a held
/// direction should move one step and wait, not slide through the roster.
fn key_pressed_once(g: &mut Game, up: bool) -> bool {
    let held = if up {
        key_held(BLIP_KEY_UP) || key_held(BLIP_KEY_W) || key_held(BLIP_KEY_LEFT) || key_held(BLIP_KEY_A)
    } else {
        key_held(BLIP_KEY_DOWN) || key_held(BLIP_KEY_S) || key_held(BLIP_KEY_RIGHT) || key_held(BLIP_KEY_D)
    };
    let slot = &mut g.sel_held[usize::from(!up)];
    let stepped = held && !*slot;
    *slot = held;
    stepped
}

#[cfg(test)]
mod tests;
