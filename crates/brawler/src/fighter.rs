//! One fighter in a round: where they are, what they are doing, and the
//! questions the rules ask of them (how tall, how far, can it be hit).

use super::*;

/// What a fighter is doing. One at a time, and each with its own clock
/// (`Fighter::t`).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Act { Idle, Walk, Crouch, Air, Attack, Block, Hitstun, Knockdown, Victory, Defeat, Bow }

/// A fighter in a round. `Copy`, so the rules can take two out of the game,
/// work on them and put them back.
#[derive(Copy, Clone)]
pub(crate) struct Fighter {
    pub(crate) who: usize,
    pub(crate) x: f32,
    pub(crate) y: f32, // feet
    pub(crate) vy: f32,
    pub(crate) vx: f32, // air momentum only; grounded walking is direct
    pub(crate) facing: f32, // +1 right, -1 left
    pub(crate) health: i32,
    pub(crate) act: Act,
    /// Seconds elapsed inside the current action. Every state machine
    /// here is "this act, this long", which keeps hitstun, recovery and
    /// wakeup the same kind of thing.
    pub(crate) t: f32,
    pub(crate) mv: MoveId,
    /// One attack, one hit. Without this an active window of 4 frames
    /// would land 4 times.
    pub(crate) hit_done: bool,
    /// Whether this attack actually dealt damage, as opposed to being
    /// blocked. Only the flying kick reads it, to decide how much of
    /// its landing it has to pay for.
    pub(crate) hit_clean: bool,
    pub(crate) crouch_block: bool,
    pub(crate) stun: f32,
    pub(crate) rounds: i32,
    /// How long is left to cancel this attack's recovery into another
    /// one — set only when a light attack *lands*. See CANCEL_WINDOW.
    pub(crate) cancel_t: f32,
    /// Whether this attack was itself started from a cancel. A chain
    /// that could chain again is an infinite: jab into jab into jab,
    /// faster than the hitstun it causes, forever.
    pub(crate) chained: bool,
    /// Hits taken without recovering in between — the combo counter,
    /// kept on the receiving end because that is what it scales.
    pub(crate) combo: i32,
    /// An attack pressed while busy, remembered briefly, so a press one frame
    /// before recovery ends still comes out.
    pub(crate) buffered: Option<MoveId>,
    pub(crate) buffer_t: f32,

    // ---- what is being drawn, as opposed to played --
    // Poses are a function of (action, timer), so a changed action changed
    // the picture on the same frame. The drawing keeps a short memory
    // instead: the previous action, its timer frozen at the handover, and how
    // much of it still shows (see draw::pose_of()).
    pub(crate) shown: Act,
    pub(crate) shown_t: f32,
    pub(crate) shown_mv: MoveId,
    /// Whether the last-seen action was off the ground, and how fast.
    /// The airborne pose is read off vertical speed, so remembering the
    /// action without the speed remembers nothing.
    pub(crate) shown_air: bool,
    pub(crate) shown_vy: f32,
    pub(crate) prev_act: Act,
    pub(crate) prev_mv: MoveId,
    pub(crate) prev_t: f32,
    pub(crate) prev_air: bool,
    pub(crate) prev_vy: f32,
    /// 1.0 the frame the action changed, falling to 0 over `blend_len`.
    pub(crate) blend: f32,
    /// How long this handover gets: `POSE_BLEND`, or longer for
    /// standing up and landing.
    pub(crate) blend_len: f32,
    /// Seconds left of the give in the knees after a landing, and how
    /// hard the landing was. Drawing state only — nothing in the rules
    /// sees it, so a landing is still actionable on the touchdown frame.
    pub(crate) land: f32,
    pub(crate) land_force: f32,
    /// How far a flier has tipped over into level flight, 0 to 1, eased so
    /// the picture does not snap with the stick. Drawing state only.
    pub(crate) soar: f32,
    /// Seconds left of the guard impact window, and whether toward was held
    /// last frame (the window opens on the press, not the hold).
    pub(crate) parry_t: f32,
    /// Seconds for which this fighter cannot be thrown: no throw loops.
    pub(crate) throw_rest: f32,
    /// Seconds left wrapped in a web, the blows taken in it, and the time
    /// after it during which another web will not hold.
    pub(crate) webbed: f32,
    pub(crate) web_hits: u8,
    pub(crate) web_rest: f32,
    /// Punches thrown in quick succession change hands: this one is with the
    /// far hand. `since_punch` is the time since the last one started.
    pub(crate) alt_punch: bool,
    pub(crate) since_punch: f32,
    /// The far leg is the one in front: whichever foot a walk stopped on.
    /// Kicks come off the front leg.
    pub(crate) far_leads: bool,
    /// Blows taken lately (it drains), the seconds of seeing stars left, and
    /// whether this round's one dizzy spell has been had.
    pub(crate) daze: f32,
    pub(crate) dizzy: f32,
    pub(crate) dazed: bool,
    /// This round's call for the other turtles has been made; and whether
    /// the special now under way is that call.
    pub(crate) called: bool,
    pub(crate) calling: bool,
    /// Where the laser will go: where the opponent stood when the stare began.
    pub(crate) aim: Option<(f32, f32)>,
    pub(crate) was_toward: bool,
    /// A rage jump is on its way down: the landing shakes the floor.
    pub(crate) stomp: bool,
    /// What just happened to this fighter's blow, for `update_fight` to
    /// announce: it landed as a counter, or it was turned by a guard impact.
    pub(crate) countered: bool,
    pub(crate) turned: bool,
    /// The opponent's size, kept up to date by `face_off`: blows are aimed
    /// at the body in front of them.
    pub(crate) foe_size: f32,
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
pub(crate) const BOW_TO_GUARD: f32 = 0.35;
/// The round intro: the bow, then the guard coming up as FIGHT shows.
pub(crate) const ROUND_INTRO: f32 = BOW_TIME + 0.7;
/// What a continue takes off the difficulty dial, up to three times over.
pub(crate) const MERCY: f32 = 0.12;
/// How long a beaten player has to decide to try that fight again.
pub(crate) const CONTINUE_SECS: f32 = 9.0;
/// How long a new screen takes to come up out of black.
pub(crate) const FADE: f32 = 0.3;
/// How long the two fighters are shown off before a match.
pub(crate) const VS_SECS: f32 = 2.6;

impl Fighter {
    /// Fighter `who` standing at `x`, facing `facing`, at full health.
    pub(crate) fn new(who: usize, x: f32, facing: f32) -> Self {
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
            webbed: 0.0, web_hits: 0, web_rest: 0.0, called: false, calling: false, aim: None,
            alt_punch: false, since_punch: 9.0, far_leads: false,
            daze: 0.0, dizzy: 0.0, dazed: false, was_toward: false, countered: false, turned: false,
            foe_size: FIGHTERS[who].size,
        }
    }

    /// This fighter's entry in the roster.
    pub(crate) fn arch(&self) -> Archetype { FIGHTERS[self.who] }
    /// Off the floor.
    pub(crate) fn airborne(&self) -> bool { self.y < FLOOR_Y - 0.01 }
    /// Seconds until the feet touch, from where they are and how fast
    /// they are moving. Solves the same fall `advance` integrates.
    pub(crate) fn air_time(&self) -> f32 {
        let d = (FLOOR_Y - self.y).max(0.0);
        ((-self.vy + (self.vy * self.vy + 2.0 * GRAVITY * d).sqrt()) / GRAVITY).max(0.0)
    }
    /// Low: crouched, or guarding low.
    pub(crate) fn crouching(&self) -> bool { Self::is_low(self.act, self.mv, self.crouch_block) }
    /// Whether an action is played from down on the haunches. Off
    /// `self` so the drawing can ask about a past action too.
    pub(crate) fn is_low(act: Act, mv: MoveId, crouch_block: bool) -> bool {
        act == Act::Crouch || (act == Act::Block && crouch_block)
            || (act == Act::Attack && matches!(mv, MoveId::Sweep))
    }
    /// How big, as a multiple of a grown fighter.
    pub(crate) fn size(&self) -> f32 { self.arch().size }
    /// How fast a ground attack throws the body forward. Nothing for a
    /// full-size fighter; a small one has half the reach and has to leap in
    /// behind every blow, which leaves them in the other's face if it misses.
    pub(crate) fn lunge(&self) -> f32 { (1.0 - self.size()).max(0.0) * LUNGE }
    /// Down to the last quarter of the bar (see `RAGE`).
    pub(crate) fn enraged(&self) -> bool {
        let a = self.arch();
        !a.invincible && self.health > 0 && self.health * 4 <= a.health
    }
    /// A turtle guarding low is inside its shell, where nothing chips it.
    pub(crate) fn shelled(&self) -> bool {
        self.arch().build == Build::Turtle && self.act == Act::Block && self.crouch_block
    }
    /// Flies instead of jumping: up rises, down lands, no gravity between.
    pub(crate) fn flies(&self) -> bool { self.arch().build == Build::Caped }
    /// In the air under their own power, as opposed to falling.
    pub(crate) fn soaring(&self) -> bool {
        self.flies() && self.airborne() && matches!(self.act, Act::Air | Act::Attack)
    }
    /// The punch that sends them flying, whatever it hits.
    pub(crate) fn epic_punch(&self) -> bool {
        self.flies() && matches!(self.mv, MoveId::HighPunch | MoveId::LowPunch | MoveId::JumpPunch | MoveId::Uppercut)
    }
    /// Lose health to a blow from a fighter of size `from`. The invincible
    /// feel only someone bigger than themselves, and then a quarter of it.
    pub(crate) fn hurt(&mut self, damage: i32, from: f32) {
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
    pub(crate) fn web(&mut self) -> bool {
        if self.webbed > 0.0 || self.web_rest > 0.0 { return false; }
        self.webbed = WEB_SECS;
        self.web_hits = 0;
        self.vx = 0.0;
        true
    }
    /// Out of the web, and safe from another for a while.
    pub(crate) fn unweb(&mut self) {
        self.webbed = 0.0;
        self.web_rest = WEB_REST;
    }
    /// Body width in pixels.
    pub(crate) fn width(&self) -> f32 { BODY_W * self.size() }
    /// Body height in pixels for what they are doing: standing, crouched or
    /// flat.
    pub(crate) fn height(&self) -> f32 {
        self.size() * if self.act == Act::Knockdown { PRONE_H }
            else if self.crouching() { CROUCH_H }
            else { STAND_H }
    }

    /// The box that can be hit. Deliberately the same box the fighter is
    /// drawn in: a hurtbox that does not match the picture is how a
    /// player learns to stop trusting their eyes.
    pub(crate) fn hurt_box(&self) -> (f32, f32, f32, f32) {
        let h = self.height();
        let w = self.width();
        (self.x - w / 2.0, self.y - h, w, h)
    }

    /// Can this fighter act at all right now?
    pub(crate) fn free(&self) -> bool {
        matches!(self.act, Act::Idle | Act::Walk | Act::Crouch | Act::Block)
    }

    /// A move as this fighter throws it: their power and reach, aimed no
    /// higher than the opponent stands.
    pub(crate) fn scaled(&self, m: MoveData) -> MoveData {
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
    pub(crate) fn hit_box(&self) -> Option<(f32, f32, f32, f32)> {
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

    /// Begin move `id`: reset the clocks, and set what depends on how it
    /// started (the chain hand, the call, the lunge, the flying arc).
    pub(crate) fn start_attack(&mut self, id: MoveId) {
        // A turtle's first special of the round calls the others in.
        self.calling = id == MoveId::Special && self.arch().build == Build::Turtle && !self.called;
        if self.calling { self.called = true; }
        self.chained = self.cancel_t > 0.0 && self.act == Act::Attack;
        if matches!(id, MoveId::LowPunch | MoveId::HighPunch) {
            self.alt_punch = self.since_punch < CHAIN_PUNCH && !self.alt_punch;
            self.since_punch = 0.0;
        }
        self.aim = None;
        self.act = Act::Attack;
        self.mv = id;
        self.t = 0.0;
        self.hit_done = false;
        self.hit_clean = false;
        self.countered = false;
        self.stomp = id == MoveId::JumpKick && self.arch().build == Build::Giant;
        self.cancel_t = 0.0;
        if !self.airborne() && matches!(id, MoveId::LowPunch | MoveId::HighPunch
            | MoveId::LowKick | MoveId::HighKick | MoveId::Sweep | MoveId::Uppercut) {
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
    pub(crate) fn can_cancel(&self) -> bool {
        self.act == Act::Attack && self.cancel_t > 0.0 && !self.chained
    }
}
