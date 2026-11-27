//! The rules of a blow: what an input asks for, how a fighter moves through
//! an action, and what happens when a hitbox meets a body: guard, guard
//! impact, counter, knockdown, dizziness.

use super::*;

/// Does a block at this stance stop an attack at this height? Standing covers
/// overheads and mids, crouching covers lows and mids; neither covers
/// everything.
pub(crate) fn blocks(level: Level, crouch_block: bool) -> bool {
    match level {
        Level::Mid => true,
        Level::Low => crouch_block,
        Level::Overhead => !crouch_block,
    }
}

/// Throws cannot be blocked: block beats attack, throw beats block, attack
/// beats throw. Without it, two players holding back ran the clock out.
pub(crate) fn unblockable(id: MoveId) -> bool { id == MoveId::Throw }

/// How close the two have to be for a punch to become a throw, as a share of
/// the throw's own range: well inside it, so a throw that starts connects.
pub(crate) const THROW_CLOSE: f32 = 0.74;

/// Tell each fighter how big the other is, and whether they are close enough
/// to throw. Called before input, every frame.
pub(crate) fn face_off(p: &mut [Fighter; 2]) -> [bool; 2] {
    let dist = (p[1].x - p[0].x).abs();
    p[0].foe_size = p[1].size();
    p[1].foe_size = p[0].size();
    p[0].foe = p[1].who;
    p[1].foe = p[0].who;
    [0, 1].map(|i| dist <= throw_range(&p[i]))
}

/// How close the centres must be for a punch to become a throw.
pub(crate) fn throw_range(f: &Fighter) -> f32 { attack_range(f, MoveId::Throw) * THROW_CLOSE }

/// The distance between centres at which `id` would just touch the
/// other fighter. Reach plus both half-widths — the number a player is
/// judging by eye every time they decide whether to step in.
pub(crate) fn attack_range(f: &Fighter, id: MoveId) -> f32 {
    f.scaled(move_data(id)).reach + (f.width() + BODY_W * f.foe_size) / 2.0
}

/// Is this fighter holding away from the other one?
pub(crate) fn holding_back(hold: Input, facing: f32) -> bool {
    (facing > 0.0 && hold.left) || (facing < 0.0 && hold.right)
}

/// One frame of a player's stick and buttons. The CPU and the bot produce the
/// same thing.
#[derive(Copy, Clone, Default)]
pub(crate) struct Input {
    pub(crate) left: bool,
    pub(crate) right: bool,
    pub(crate) up: bool,
    pub(crate) down: bool,
    pub(crate) punch_low: bool,
    pub(crate) punch_high: bool,
    /// The two kick heights, one button each.
    pub(crate) kick_low: bool,
    pub(crate) kick_high: bool,
    pub(crate) special: bool,
}

impl Input {
    /// Any kick button at all. Used where the height does not matter:
    /// the special, and reading a kick while airborne.
    pub(crate) fn any_kick(self) -> bool { self.kick_low || self.kick_high }
    /// Either punch.
    pub(crate) fn any_punch(self) -> bool { self.punch_low || self.punch_high }
}

/// Which attack these buttons ask for, given where the fighter is. One place
/// decides, so a buffered press and a live one agree.
pub(crate) fn pressed_move(f: &Fighter, inp: Input, close: bool) -> Option<MoveId> {
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

/// The move a press asks for in the air.
pub(crate) fn air_move(f: &Fighter, inp: Input) -> Option<MoveId> {
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
    None
}

/// The move a press asks for on the ground; `close` is within throwing range.
pub(crate) fn grounded_move(inp: Input, close: bool) -> Option<MoveId> {
    if inp.special { return Some(MoveId::Special); }
    // A standing punch right up against them is a throw: the throw has no
    // button of its own, and proximity is how the originals did it.
    if inp.any_punch() && close && !inp.down { return Some(MoveId::Throw); }
    // Crouching turns a punch into the uppercut, the way it turns a kick
    // into the sweep: down plus a button means one thing whichever button
    // found it.
    if inp.down && inp.any_punch() { return Some(MoveId::Uppercut); }
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
pub(crate) const BUFFER: f32 = 10.0 * F;

/// What the shell is told while the title screen is up: a cabinet with
/// its second station lit and nobody at it yet. See `blip::web::set_players`.
pub(crate) const TITLE_OPEN: i32 = 2;

/// A blocked special still chips a sixth of its damage (normals chip
/// nothing). Without it two good blockers time out (seven rounds in twelve);
/// chip makes holding back cost something.
pub(crate) const CHIP_DIVISOR: i32 = 6;

/// Frames to cancel a landed light attack into a heavier one: the reward for
/// seeing the hit. Only a connecting light attack opens it; whiffed or
/// blocked, you get nothing.
pub(crate) const CANCEL_WINDOW: f32 = 14.0 * F;

/// How long after a punch or kick starts the other button still turns it into
/// the special: nobody presses two buttons on the same frame, least of all on
/// a touch screen.
pub(crate) const SPECIAL_WINDOW: f32 = 5.0 * F;

/// Hitting someone out of their own attack is a counter, as in Tekken: a
/// quarter more damage and six frames more stun.
pub(crate) const COUNTER: f32 = 1.25;
pub(crate) const COUNTER_STUN: f32 = 6.0 * F;
/// DRAKE's counter, the intercepting fist: half again, against a quarter for everyone else.
pub(crate) const INTERCEPT: f32 = 1.5;

/// Rage, as in Tekken: with a quarter of the bar left a fighter hits a fifth
/// harder, so a round is not over until it is over.
pub(crate) const RAGE: f32 = 1.2;

/// The guard impact, Soulcalibur's: tap toward the opponent inside this
/// window before a blow lands and it is knocked aside, leaving the attacker
/// open for `IMPACT_STUN`. Throws and projectiles cannot be turned.
pub(crate) const IMPACT_WINDOW: f32 = 6.0 * F;
pub(crate) const IMPACT_STUN: f32 = 24.0 * F;

/// Once per round, fruit appears at `FRUIT_AT`, heals 12%, and awards its listed points.
pub(crate) const FRUIT_AT: f32 = ROUND_SECS - 12.0;
pub(crate) const FRUIT_STAYS: f32 = 9.0;
pub(crate) const FRUIT_HEAL: f32 = 0.12;
pub(crate) const FRUIT_POINTS: [(i32, &str); 8] = [(100, "100"), (300, "300"), (500, "500"), (700, "700"),
    (1000, "1000"), (2000, "2000"), (3000, "3000"), (5000, "5000")];

/// The giant's jump kick is a rage jump: when he comes down the whole floor
/// jumps, and a fighter standing on it anywhere is thrown off their feet for
/// this much. Being in the air when he lands is the only answer.
pub(crate) const QUAKE_DAMAGE: i32 = 6;
pub(crate) const QUAKE_SECS: f32 = 0.5;

/// The finishing blow plays at this speed for this long, as Tekken's does.
pub(crate) const SLOW_MO: f32 = 0.3;
pub(crate) const SLOW_MO_SECS: f32 = 0.6;

/// What each successive hit of a combo is worth. Two hits is a reward;
/// eight would be a cutscene.
pub(crate) fn combo_scale(hits: i32) -> f32 {
    match hits {
        0 => 1.0,
        1 => 0.75,
        _ => 0.5,
    }
}

/// Act on this frame's input: turn, walk, crouch, jump, guard, or start (or
/// buffer) an attack.
pub(crate) fn apply_input(f: &mut Fighter, inp: Input, close: bool, dt: f32) {
    read_taps(f, inp, dt);
    if f.soaring() { return soar(f, inp, close); }
    if f.airborne() && f.act == Act::Air { return jump_attack(f, inp, close); }

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

    // An airborne fighter has already committed; the jump is the
    // decision and only an attack is left.
    if f.airborne() { return; }
    if inp.up { take_off(f, inp); } else { stand(f, inp, dt); }
}

/// The taps the moves read: a fresh push toward opens the parry window, a fresh down counts toward the stance switch,
/// and a buffered move runs out.
fn read_taps(f: &mut Fighter, inp: Input, dt: f32) {
    let toward = if f.facing > 0.0 { inp.right } else { inp.left };
    if toward && !f.was_toward { f.parry_t = IMPACT_WINDOW; }
    else if f.parry_t > 0.0 { f.parry_t -= dt; }
    f.was_toward = toward;
    if inp.down && !f.was_down { f.tap_down(); }
    f.was_down = inp.down;

    if f.buffer_t > 0.0 {
        f.buffer_t -= dt;
        if f.buffer_t <= 0.0 { f.buffered = None; }
    }
}

/// Flying: the stick steers, a button attacks from where he hangs.
fn soar(f: &mut Fighter, inp: Input, close: bool) {
    if f.act == Act::Attack { return; }
    if let Some(id) = pressed_move(f, inp, close) {
        f.vx = 0.0;
        f.vy = 0.0;
        f.start_attack(id);
        return;
    }
    f.vx = if inp.left { -SOAR } else if inp.right { SOAR } else { 0.0 };
    f.vy = if inp.up { -SOAR } else if inp.down { SOAR * 1.4 } else { 0.0 };
}

/// In a jump: a jump-in attack starts here; the busy path buffers grounded moves.
fn jump_attack(f: &mut Fighter, inp: Input, close: bool) {
    let Some(id) = air_move(f, inp) else { return };
    // Landing cancels unfinished attacks, so buffer moves that cannot start two frames early.
    if f.air_time() >= (move_data(id).startup + 2.0) * F {
        f.start_attack(id);
    } else if let Some(g) = grounded_move(inp, close) {
        f.buffered = Some(g);
        f.buffer_t = BUFFER;
    }
}

/// Up leaves the floor: a flyer rises straight up, anyone else jumps, drifting the way the stick leans.
fn take_off(f: &mut Fighter, inp: Input) {
    if f.flies() {
        f.vy = -SOAR;
        f.vx = 0.0;
    } else {
        f.vy = JUMP_VY * f.arch().jump_scale;
        f.vx = if inp.left { -AIR_DRIFT } else if inp.right { AIR_DRIFT } else { 0.0 };
    }
    f.y -= 0.5; // leave the floor this frame so airborne() reads true
    f.act = Act::Air;
    f.t = 0.0;
}

/// On the floor with nothing to start: block, crouch, walk or stand.
fn stand(f: &mut Fighter, inp: Input, dt: f32) {
    let crouch = inp.down;
    // Blocking is not a button: it is holding away, which means every
    // step backwards is already a block and retreating is never free.
    if holding_back(inp, f.facing) {
        f.act = Act::Block;
        f.crouch_block = crouch;
        f.t = 0.0;
        let speed = f.arch().back_walk * f.pace();
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
        f.x += f.facing * f.arch().walk * f.pace() * dt;
    } else {
        f.act = Act::Idle;
    }
}

/// Advance one fighter's physics and action timer.
pub(crate) fn advance(f: &mut Fighter, dt: f32) {
    wear_off(f, dt);
    let from = f.t;
    let was = f.stick_strike();
    f.t += dt;
    // A step forward into each new strike of the nunchaku (170 px/s, bled off by the slide: about 25 px).
    if let Some((k, _)) = f.stick_strike() {
        if was.map(|w| w.0) != Some(k) && !f.hit_done { f.vx = f.facing * 170.0; }
    }
    if f.act == Act::Bow && f.t >= BOW_TIME { f.act = Act::Idle; f.t = 0.0; }
    // A long frame on a slow device can step clean over a two-frame active
    // window; stop in it for this step so the hit is still tested.
    if f.act == Act::Attack && !f.hit_done {
        let m = f.scaled(move_data(f.mv));
        let (start, end) = (m.startup * F, (m.startup + m.active) * F);
        if from < start && f.t > end { f.t = (start + end) / 2.0; }
    }
    if f.cancel_t > 0.0 { f.cancel_t -= dt; }
    if f.taunt > 0.0 { f.taunt -= dt; }
    if f.airborne() || f.vy < 0.0 { fly(f, dt); }
    // An attack that carries the body along the floor (the rush, a small
    // fighter's lunge) slides, and the slide bleeds off so it ends.
    if !f.airborne() && matches!(f.act, Act::Attack | Act::Knockdown) {
        f.x += f.vx * dt;
        f.vx *= 1.0 - (6.0 * dt).min(1.0);
    }
    run_out(f, dt);
    note_handover(f, dt);
}

/// The clocks that run down by themselves: stumbles and staggers, a landing's give, a throw's rest, the daze, the
/// stars, a web, and how long since each button.
fn wear_off(f: &mut Fighter, dt: f32) {
    f.stumble = (f.stumble - dt).max(0.0);
    // A stagger wears off once the fighter is back on their feet.
    if f.act != Act::Hitstun && f.act != Act::Knockdown { f.stagger = (f.stagger - dt).max(0.0); }
    if f.land > 0.0 { f.land = (f.land - dt).max(0.0); }
    if f.throw_rest > 0.0 { f.throw_rest = (f.throw_rest - dt).max(0.0); }
    f.daze = (f.daze - DAZE_DRAIN * dt).max(0.0);
    // The stars last as long as the stun does, whatever ends it.
    f.dizzy = if f.act == Act::Hitstun { (f.dizzy - dt).max(0.0) } else { 0.0 };
    f.since_punch += dt;
    f.since_kick += dt;
    f.down_tap += dt;
    if f.act == Act::Walk && !f.airborne() { f.far_leads = draw::far_foot_leads(f); }
    if f.web_rest > 0.0 { f.web_rest = (f.web_rest - dt).max(0.0); }
    if f.webbed > 0.0 {
        f.webbed -= dt;
        if f.webbed <= 0.0 { f.unweb(); }
    }
    let level = if f.soaring() && f.act == Act::Air { (f.vx * f.facing / SOAR).clamp(0.0, 1.0) } else { 0.0 };
    f.soar += (level - f.soar) * (10.0 * dt).min(1.0);
}

/// Off the floor: a flier hangs where the stick left him (under the ceiling), everyone else falls, and lands.
fn fly(f: &mut Fighter, dt: f32) {
    if f.soaring() { f.vy = f.vy.max(-SOAR); } else { f.vy += GRAVITY * dt; }
    f.y += f.vy * dt;
    if f.soaring() && f.y < FLOOR_Y - CEILING { f.y = FLOOR_Y - CEILING; f.vy = 0.0; }
    f.x += f.vx * dt;
    if f.y < FLOOR_Y { return; }
    f.y = FLOOR_Y;
    // How hard they arrived, for the drawing.
    f.land = LAND_ABSORB;
    f.land_force = (f.vy / -JUMP_VY).clamp(0.25, 1.0);
    f.vy = 0.0;
    f.vx = 0.0;
    // Landing cancels an air attack, except the flying kick: its
    // recovery plays out on the ground, the price of the distance.
    if f.act == Act::Attack && matches!(f.mv, MoveId::FlyingKick | MoveId::Uppercut) {
        let m = f.scaled(move_data(f.mv));
        let total = (m.startup + m.active + m.recovery) * F;
        // Blocked or whiffed: at least this much left to pay.
        // Connected: at most this much.
        f.t = if f.mv == MoveId::Uppercut {
            f.t.min(total - 8.0 * F)
        } else if f.hit_clean {
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

/// An attack, a stun or a knockdown that has run its time hands back to standing.
fn run_out(f: &mut Fighter, dt: f32) {
    match f.act {
        Act::Attack => {
            let m = f.scaled(move_data(f.mv));
            let total = if f.mv == MoveId::Special && f.arch().special == Special::DriveBy {
                DRIVEBY_TOTAL
            } else { (m.startup + m.active + m.recovery) * F };
            if !f.airborne() && f.t >= total {
                if !f.hit_done && f.stumble_roll < 4 { f.stumble = 0.34; }
                // The roundhouse's kicking leg comes down in front.
                if f.mv == MoveId::Roundhouse { f.far_leads = !f.far_leads; }
                f.act = Act::Idle; f.t = 0.0; f.vx = 0.0;
            }
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
}

/// Capture the previous pose when an action changes, so drawing can blend across the handover.
pub(crate) fn note_handover(f: &mut Fighter, dt: f32) {
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
    if f.facing != f.shown_facing {
        f.shown_facing = f.facing;
        f.turn = 1.0;
    } else if f.turn > 0.0 {
        f.turn = (f.turn - dt / TURN_BLEND).max(0.0);
    }
}

/// DRAKE is so plainly skilled that the boss is no problem for him (the user's rule, 2026-11-20): his blows
/// count `DRAKE_EDGE` times over against the boss, and the boss's land on him at the inverse; the laser does
/// not end his round. (CPU against CPU he won 1 round in 20 against ZENITH; with the edge alone 9; with the laser
/// too, 19. `drake_has_no_trouble_with_zenith` holds it at three in four or better.)
pub(crate) const DRAKE_EDGE: f32 = 2.5;

pub(crate) fn master_edge(attacker: &Fighter, defender: &Fighter) -> f32 {
    if formless(attacker.who) && defender.arch().boss { DRAKE_EDGE }
    else if attacker.arch().boss && formless(defender.who) { 1.0 / DRAKE_EDGE }
    else { 1.0 }
}

/// Is this fighter untouchable right now? Only on the way up from a
/// knockdown, and only briefly.
pub(crate) fn invulnerable(f: &Fighter) -> bool {
    f.act == Act::Knockdown && f.t > 0.85
}

/// Resolve `attacker`'s active hitbox against `defender`. Returns the
/// damage dealt (0 for a block or a miss) and whether it was blocked.
pub(crate) fn resolve_hit(attacker: &mut Fighter, defender: &mut Fighter, hold: Input) -> (i32, bool, bool) {
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
    // The nunchaku's strikes each take their turn; the sequence ends at the last (or a block, below).
    if let Some((k, _)) = attacker.stick_strike() {
        attacker.strikes = k + 1;
        if k + 1 >= STICK_STRIKES { attacker.hit_done = true; }
    } else {
        attacker.hit_done = true;
    }
    if attacker.mv == MoveId::Throw { defender.throw_rest = THROW_REST; }
    if parried(attacker, defender) || blocked(attacker, defender, hold, &m) { return (0, true, false); }
    let (damage, floored) = land_blow(attacker, defender, &m);
    (damage, false, floored)
}

/// The guard impact: a defender who pressed toward the blow just as it came (`parry_t`) turns it, and the attacker
/// is left open.
fn parried(attacker: &mut Fighter, defender: &mut Fighter) -> bool {
    if defender.parry_t <= 0.0 || defender.airborne() || !defender.free() || unblockable(attacker.mv) { return false; }
    defender.parry_t = 0.0;
    attacker.turned = true;
    attacker.act = Act::Hitstun;
    attacker.stun = IMPACT_STUN;
    attacker.t = 0.0;
    attacker.vx = 0.0;
    true
}

/// A blow guarded (see `guarding`) at its own height is blocked. A special still chips a little through it.
fn blocked(attacker: &mut Fighter, defender: &mut Fighter, hold: Input, m: &MoveData) -> bool {
    if !guarding(defender, hold) || unblockable(attacker.mv) || !blocks(m.level, defender.crouch_block || hold.down) { return false; }
    // Blocked, the nunchaku's run is over: straight into the recovery.
    if attacker.nunchaku() {
        attacker.hit_done = true;
        attacker.t = attacker.t.max((STICK_WHIRL + STICK_EVERY * STICK_STRIKES as f32) * F);
    }
    defender.act = Act::Block;
    defender.crouch_block = hold.down;
    defender.stun = m.blockstun * F;
    let chip = if attacker.mv == MoveId::Special { (m.damage / CHIP_DIVISOR).max(1) } else { 0 };
    if chip > 0 && !defender.shelled() { defender.hurt(chip); }
    true
}

/// Whether a fighter is guarding against what is coming: on the ground, holding away, and not already committed to
/// something of their own.
pub(crate) fn guarding(f: &Fighter, hold: Input) -> bool {
    !f.airborne() && holding_back(hold, f.facing) && matches!(f.act, Act::Idle | Act::Walk | Act::Crouch | Act::Block)
}

/// A clean hit: the damage (more for a counter, less deep in a combo), and the defender floored or rocked; the
/// damage done and whether they went down.
fn land_blow(attacker: &mut Fighter, defender: &mut Fighter, m: &MoveData) -> (i32, bool) {
    // Caught in the middle of their own attack, before its recovery.
    // (Their own timing: the laser's stare is longer than the table's special.)
    let theirs = defender.scaled(move_data(defender.mv));
    let counter = defender.act == Act::Attack && defender.t <= (theirs.startup + theirs.active) * F;
    attacker.countered = counter;
    // Jeet kune do is "the way of the intercepting fist": DRAKE catching a blow on its way out hits harder still.
    let bonus = if counter && formless(attacker.who) { INTERCEPT } else if counter { COUNTER } else { 1.0 };
    let damage = ((m.damage as f32) * combo_scale(defender.combo) * bonus * master_edge(attacker, defender)).round().max(1.0) as i32;
    defender.combo += 1;
    // A light attack that lands buys the right to follow it up.
    // DRAKE flows: any blow of his that lands (not a special) opens the window, one technique into the next.
    if (matches!(attacker.mv, MoveId::LowPunch) || (formless(attacker.who) && attacker.mv != MoveId::Special))
        && !attacker.chained {
        attacker.cancel_t = CANCEL_WINDOW;
    }
    defender.hurt(damage);
    attacker.hit_clean = true;
    // A flying kick that connects stops flying, so the attacker can act
    // before the defender's hitstun ends. A blocked one carries on through,
    // into punishing range.
    if attacker.mv == MoveId::FlyingKick {
        attacker.vx = 0.0;
        attacker.vy = attacker.vy.max(0.0);
    }
    let floored = m.knockdown || attacker.epic_punch();
    if floored || defender.airborne() { floor(attacker, defender); } else { rock(defender, m, damage, counter); }
    (damage, floored)
}

/// Knocked down (anyone hit in the air is too).
fn floor(attacker: &mut Fighter, defender: &mut Fighter) {
    defender.act = Act::Knockdown;
    // Somebody DRAKE put down gets the thumb across the nose while they lie there (1.4 s, about their time down).
    if formless(attacker.who) { attacker.taunt = 1.4; }
    let one_inch = attacker.mv == MoveId::Special && attacker.special() == Special::OneInch;
    defender.downed_by = if defender.airborne() { MoveId::LowPunch } else { attacker.mv };
    defender.t = 0.0;
    defender.vy = 0.0;
    // The one-inch punch throws them back across the stage: 560 px/s bleeding off at 6 a second is about
    // 90 px of slide, on top of the push every knockdown gets.
    defender.vx = if one_inch { attacker.facing * 560.0 } else { 0.0 };
    defender.y = FLOOR_Y;
}

/// Rocked on their feet: hitstun (longer for a counter), the daze measure filling toward dizzy, and a stagger after
/// a heavy clean blow.
fn rock(defender: &mut Fighter, m: &MoveData, damage: i32, counter: bool) {
    defender.act = Act::Hitstun;
    defender.struck(if m.level == Level::Overhead { defender.height() } else { m.height }, damage);
    defender.stun = m.hitstun * F + if counter { COUNTER_STUN } else { 0.0 };
    defender.t = 0.0;
    // The blow that finds a dizzy fighter wakes them; the one that fills
    // the measure sends them off.
    defender.daze += damage as f32;
    if defender.dizzy > 0.0 {
        defender.dizzy = 0.0;
    } else if defender.daze >= DAZE_AT && !defender.dazed && defender.health > 0 {
        defender.dazed = true;
        defender.daze = 0.0;
        defender.dizzy = DIZZY_SECS;
        defender.stun = DIZZY_SECS;
    }
    if damage >= STAGGER_FROM && defender.dizzy <= 0.0 && defender.health > 0 {
        if defender.stagger <= 0.0 { blip::bot::add("staggers", 1.0); }
        defender.stagger = STAGGER_SECS;
    }
}

/// Judge both blows before either can interrupt the other: active windows trade.
pub(crate) fn resolve_trades(p: &mut [Fighter; 2], holds: [Input; 2]) -> [(i32, bool, bool); 2] {
    let before = *p;
    let (mut attackers, mut defenders) = (before, before);
    let hits = std::array::from_fn(|a| resolve_hit(&mut attackers[a], &mut defenders[1 - a], holds[1 - a]));
    for i in 0..2 {
        let incoming = hits[1 - i].0 > 0 || hits[1 - i].1;
        p[i] = if incoming { defenders[i] } else { attackers[i] };
        p[i].hit_done = attackers[i].hit_done;
        p[i].hit_clean = attackers[i].hit_clean;
        p[i].countered = attackers[i].countered;
        p[i].turned = attackers[i].turned;
        if incoming { p[i].cancel_t = 0.0; }
    }
    hits
}

/// Push the two apart after a hit, and if one of them is against a wall,
/// push the *other* one instead. Corner pressure only exists because of
/// this: without it a cornered fighter slides out for free.
pub(crate) fn push_apart(p: &mut [Fighter; 2], amount: f32) {
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
pub(crate) fn separate(p: &mut [Fighter; 2]) {
    let min = (p[0].width() + p[1].width()) / 2.0 * 0.82;
    let d = p[1].x - p[0].x;
    if d.abs() >= min { return; }
    let fix = (min - d.abs()) / 2.0 * if d >= 0.0 { 1.0 } else { -1.0 };
    let (lo, hi) = (WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
    p[0].x = clamp(p[0].x - fix, lo, hi);
    p[1].x = clamp(p[1].x + fix, lo, hi);
}
