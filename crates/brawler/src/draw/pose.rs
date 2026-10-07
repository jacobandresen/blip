//! Poses: where the hips, head, hands and feet are for every action and
//! every frame of it, and the skeleton solved from them. Attack poses are
//! built from the move table, so the drawn fist is where the hitbox is.

use super::*;

/// Every joint that is placed rather than solved.
#[derive(Copy, Clone)]
pub(crate) struct Pose {
    pub(crate) hip: P,
    pub(crate) head: P,
    pub(crate) lead_hand: P,
    pub(crate) rear_hand: P,
    /// The near foot. Attacks with a leg are always thrown with this
    /// one, because the near leg is the one drawn in front of the body.
    pub(crate) lead_foot: P,
    pub(crate) rear_foot: P,
    /// Hands open — a grab, or a palm — rather than closed into fists.
    pub(crate) open: bool,
    /// How far each heel is raised, in radians (see `draw_leg`).
    pub(crate) lead_heel: f32,
    pub(crate) rear_heel: f32,
    /// How far the shoulders are turned: 0 is the open stance, chest to the
    /// viewer, the near shoulder back and the far one forward; 1 has the near
    /// shoulder driven through in front, as at the end of a punch.
    pub(crate) twist: f32,
}

impl Pose {
    /// This point moved `k` of the way to `o`.
    pub(crate) fn to(self, o: Pose, k: f32) -> Pose {
        Pose {
            hip: self.hip.to(o.hip, k),
            head: self.head.to(o.head, k),
            lead_hand: self.lead_hand.to(o.lead_hand, k),
            rear_hand: self.rear_hand.to(o.rear_hand, k),
            lead_foot: self.lead_foot.to(o.lead_foot, k),
            rear_foot: self.rear_foot.to(o.rear_foot, k),
            open: if k < 0.5 { self.open } else { o.open },
            lead_heel: self.lead_heel + (o.lead_heel - self.lead_heel) * k,
            rear_heel: self.rear_heel + (o.rear_heel - self.rear_heel) * k,
            twist: self.twist + (o.twist - self.twist) * k,
        }
    }
}

/// The walk: one foot's (forward, lift) through a stride. Planted, it slides
/// back at exactly the body's speed, so on the boards it stays put.
pub(crate) const STRIDE: f32 = std::f32::consts::PI / (2.0 * STEP);  // radians of cycle per pixel travelled
pub(crate) const STEP: f32 = 18.0;
/// A walking foot through one stride at phase `ph`: how far forward of its
/// centre, and how high. Planted for half the cycle, swung for the other.
pub(crate) fn foot_cycle(ph: f32, lift: f32) -> (f32, f32) {
    let u = (ph / std::f32::consts::TAU).rem_euclid(1.0);
    if u < 0.5 {
        (STEP * (1.0 - 4.0 * u), 0.0)
    } else {
        let k = (u - 0.5) * 2.0;
        let e = k * k * (3.0 - 2.0 * k);
        (-STEP + 2.0 * STEP * e, lift * (k * std::f32::consts::PI).sin())
    }
}

/// The heel through the same stride: it peels off the floor as the foot
/// pushes away, hangs toes-down through the swing, and the toes come up just
/// before the foot is set down again.
pub(crate) fn heel_cycle(ph: f32) -> f32 {
    let u = (ph / std::f32::consts::TAU).rem_euclid(1.0);
    if u < 0.5 {
        let k = ((u - 0.28) / 0.22).clamp(0.0, 1.0);
        0.7 * k * k * (3.0 - 2.0 * k)
    } else {
        let k = (u - 0.5) * 2.0;
        0.7 * (1.0 - k) * (1.0 - k) - 0.3 * k * (k * std::f32::consts::PI).sin()
    }
}

/// The neck, derived from the spine: most of the way up and a little behind, square to it.
pub(crate) const SPINE: f32 = 0.73;
pub(crate) const SHOULDERS_BACK: f32 = 2.4;

/// The neck, derived from the spine: most of the way up it and a little
/// behind.
pub(crate) fn neck_of(q: &Pose) -> P {
    let (dx, du) = (q.head.f - q.hip.f, q.head.u - q.hip.u);
    let len = (dx * dx + du * du).sqrt().max(0.001);
    let (ux, uu) = (dx / len, du / len);
    p(q.hip.f + ux * len * SPINE - uu * SHOULDERS_BACK,
      q.hip.u + uu * len * SPINE + ux * SHOULDERS_BACK)
}

/// How one fighter carries themself: offsets on the shared stance, so nine
/// fighters do not stand as one.
pub(crate) struct Style {
    /// The idle bounce: how fast (radians a second) and how big.
    pub(crate) rate: f32,
    pub(crate) amp: f32,
    /// Hips this much lower, feet this much further apart, head this much
    /// further forward.
    pub(crate) sink: f32,
    pub(crate) wide: f32,
    pub(crate) lean: f32,
    /// Where the guard is held, as offsets on the standard hands.
    pub(crate) lead: P,
    pub(crate) rear: P,
    /// Open hands in the guard: a grappler, not a boxer.
    pub(crate) open: bool,
    /// Wins with both arms in the air instead of one.
    pub(crate) cheer: bool,
}

/// A `Style`, positionally: what `STYLES` is written in.
pub(crate) const fn style(rate: f32, amp: f32, sink: f32, wide: f32, lean: f32, lead: P, rear: P,
               open: bool, cheer: bool) -> Style {
    Style { rate, amp, sink, wide, lean, lead, rear, open, cheer }
}

/// One per fighter, in roster order.
pub(crate) const STYLES: [Style; FIGHTERS.len()] = [
    // RYUKA: the textbook karate stance.
    style(4.4, 1.0, 0.0, 0.0, 0.0, p(0.0, 0.0), p(0.0, 0.0), false, false),
    // BRUTUS: a wrestler, wide and low, open hands ready to grab.
    style(3.0, 0.8, 3.0, 4.0, 2.0, p(-4.0, -8.0), p(-4.0, -13.0), true, true),
    // KESTREL: a kickboxer on the toes, hands high.
    style(6.5, 1.4, 0.0, -5.0, -2.0, p(-2.0, 8.0), p(5.0, 3.0), false, false),
    // GIOTTO: the disciplined one, rear fist by the chin.
    style(4.0, 0.9, 1.0, 0.0, 0.0, p(2.0, -2.0), p(3.0, -4.0), false, false),
    // TITIAN: the hothead, hunched with the fists low.
    style(3.6, 1.0, 4.0, 3.0, 7.0, p(-6.0, -6.0), p(-2.0, -10.0), false, false),
    // VERMEER: tall, the lead hand held out long and open.
    style(4.2, 0.8, 0.0, -2.0, -3.0, p(5.0, 4.0), p(-4.0, -6.0), true, false),
    // BOSCH: never still.
    style(7.5, 1.8, 1.0, 0.0, 2.0, p(-4.0, 6.0), p(0.0, -12.0), false, true),
    // WEBBER: low to the floor, fingers spread.
    style(5.5, 1.2, 7.0, 8.0, 8.0, p(-2.0, -18.0), p(8.0, -32.0), true, false),
    // GAMMA: hunched, the arms hanging heavy.
    style(2.6, 1.2, 4.0, 6.0, 9.0, p(-8.0, -14.0), p(-4.0, -20.0), false, true),
    // ZENITH: upright and unbothered, the fists low.
    style(3.2, 0.7, 0.0, -3.0, -3.0, p(-8.0, -14.0), p(-5.0, -22.0), false, false),
];

/// The stance: weight back, knees bent, both hands up, side on. Everything
/// departs from and returns to this. `bob` is the idle bounce, -1..1: the
/// whole body rides it, the hands a little further.
pub(crate) fn stance(bob: f32, who: usize) -> Pose {
    let s = &STYLES[who];
    let bob = bob * s.amp;
    Pose {
        hip: p(0.0, HIP_U - 1.0 - s.sink + bob * 1.2),
        head: p(3.0 + s.lean, HEAD_U - 1.0 - s.sink - s.lean * 0.35 + bob * 1.4),
        // The near fist is held back at the chest; the far one leads, out
        // past it and higher, so both show.
        lead_hand: p(22.0 + s.rear.f, 70.0 + s.rear.u - s.sink + bob * 1.8),
        rear_hand: p(35.0 + s.lead.f, 79.0 + s.lead.u - s.sink + bob * 2.2),
        lead_foot: p(24.0 + s.wide, 0.0),
        rear_foot: p(-22.0 - s.wide, 0.0),
        open: s.open,
        lead_heel: 0.0,
        rear_heel: 0.30 + 0.10 * bob,
        twist: 0.0,
    }
}

/// The crouch every fighter shares.
pub(crate) fn crouched(bob: f32) -> Pose {
    Pose {
        hip: p(-2.0, C_HIP + bob * 0.5),
        head: p(2.0, C_HEAD + bob * 0.6),
        lead_hand: p(12.0, 34.0 + bob * 0.6),
        rear_hand: p(28.0, 41.0 + bob * 0.8),
        lead_foot: p(18.0, 0.0),
        rear_foot: p(-19.0, 0.0),
        open: false,
        lead_heel: 0.10,
        rear_heel: 0.55,
        twist: 0.0,
    }
}

/// Off the ground, posed from vertical speed: legs trailing at take-off,
/// tucked at the apex, reaching down on the way in, right at every jump
/// height.
pub(crate) fn airborne_pose(vy: f32) -> Pose {
    let r = (-vy / -JUMP_VY).clamp(-1.0, 1.0);
    let rise = r.max(0.0);
    let fall = (-r).max(0.0);
    let tuck = 1.0 - r.abs();
    let tuck = tuck * tuck * (3.0 - 2.0 * tuck);
    Pose {
        hip: p(-2.0 + 2.0 * fall, 48.0 + 4.0 * tuck),
        head: p(-3.0 - 4.0 * rise + 3.0 * fall, 99.0 + 3.0 * tuck),
        lead_hand: p(15.0 + 2.0 * rise + 2.0 * fall, 60.0 + 14.0 * rise - 4.0 * fall),
        rear_hand: p(27.0 + 2.0 * rise + 6.0 * fall, 69.0 + 14.0 * rise - 2.0 * fall),
        lead_foot: p(14.0 + 4.0 * tuck + 6.0 * fall, 8.0 + 24.0 * tuck),
        rear_foot: p(-16.0 + 6.0 * tuck + 2.0 * fall, 4.0 + 24.0 * tuck),
        open: false,
        lead_heel: 0.45,
        rear_heel: 0.55,
        twist: 0.0,
    }
}

/// A flier in the air: hanging upright with the fists on the hips, or laid
/// out level behind one fist, `level` of the way between.
pub(crate) fn flying_pose(level: f32) -> Pose {
    let hover = Pose {
        hip: p(0.0, 54.0),
        head: p(2.0, 106.0),
        lead_hand: p(11.0, 50.0),
        rear_hand: p(22.0, 57.0),
        lead_foot: p(12.0, 12.0),
        rear_foot: p(-8.0, 0.0),
        open: false,
        lead_heel: 0.5,
        rear_heel: 0.6,
        twist: 0.0,
    };
    let fly = Pose {
        hip: p(-10.0, 56.0),
        head: p(38.0, 72.0),
        lead_hand: p(78.0, 76.0),
        rear_hand: p(8.0, 50.0),
        lead_foot: p(-66.0, 46.0),
        rear_foot: p(-62.0, 56.0),
        open: false,
        lead_heel: 0.7,
        rear_heel: 0.7,
        twist: 1.0,
    };
    hover.to(fly, level * level * (3.0 - 2.0 * level))
}

/// Flat on the floor, head away from whoever put them there.
pub(crate) fn floored() -> Pose {
    Pose {
        hip: p(8.0, 13.0),
        head: p(-43.0, 9.5),
        lead_hand: p(30.0, 8.0),
        rear_hand: p(8.0, 26.0),
        lead_foot: p(26.0, 6.0),
        rear_foot: p(62.0, 8.0),
        open: false,
        lead_heel: 0.3,
        rear_heel: 0.3,
        twist: 0.0,
    }
}

/// How far through its own animation an attack is: out through startup,
/// held while it can hit, snapped back through recovery.
pub(crate) fn extension(f: &Fighter, m: &MoveData) -> f32 {
    let start = m.startup * F;
    let end = start + m.active * F;
    if f.t < start {
        let k = (f.t / start.max(0.0001)).clamp(0.0, 1.0);
        const COIL: f32 = 0.28;
        if k < COIL {
            return -0.16 * (std::f32::consts::PI * k / COIL).sin();
        }
        let k = (k - COIL) / (1.0 - COIL);
        k * k * (3.0 - 2.0 * k)
    } else if f.t <= end {
        1.0
    } else {
        let k = ((f.t - end) / (m.recovery * F).max(0.0001)).clamp(0.0, 1.0);
        const SNAP: f32 = 0.6;
        let back = (k / SNAP).min(1.0);
        1.0 - back * back * (3.0 - 2.0 * back)
    }
}

/// Every number a kick is made of, in one place.
pub(crate) struct KickShape {
    /// How far the hips drive forward over the support foot.
    pub(crate) hip_drive: f32,
    /// How much they rise doing it. Up, not down: you push off.
    pub(crate) hip_rise: f32,
    /// How far the shoulders fall back to pay for the hips.
    pub(crate) lean: f32,
    /// The same lean in the air, as a fraction — there is nothing to
    /// counterbalance against up there, and a jump kick thrown lying
    /// back is a fighter falling.
    pub(crate) air_lean: f32,
    /// Where the support foot ends up relative to the hips.
    pub(crate) plant: f32,
    /// How far in front of the hip the chambered knee points.
    pub(crate) knee_lead: f32,
    /// Height of the chambered foot, hanging below that knee.
    pub(crate) chamber: f32,
    pub(crate) chamber_air: f32,
}

pub(crate) const KICK: KickShape = KickShape {
    hip_drive: 11.0,
    hip_rise: 2.0,
    lean: 17.0,
    air_lean: 0.3,
    plant: 5.0,
    knee_lead: 16.0,
    chamber: 31.0,
    chamber_air: 25.0,
};

/// Kick progress: 0..1 for the chamber, 1..2 for the extension. The knee comes up fast and waits (the frame a defender reads), then the shin snaps out.
pub(crate) fn kick_curve(ext: f32) -> f32 {
    const SPLIT: f32 = KICK_SNAP;
    if ext < SPLIT {
        let k = ext / SPLIT;
        1.0 - (1.0 - k) * (1.0 - k)
    } else {
        let k = (ext - SPLIT) / (1.0 - SPLIT);
        1.0 + k * k
    }
}

/// Move `q` into the attack `f` is throwing, `ext` of the way out: -1..0
/// winding up, 0..1 extending.
pub(crate) fn attack_pose(q: &mut Pose, f: &Fighter, m: &MoveData, ext: f32) {
    q.open = false;
    // The move is in screen pixels; the pose is a full-size fighter's.
    let z = f.size();
    let end = p(BODY_W / 2.0 + m.reach / z, m.height / z);
    let bulk = f.arch().bulk;
    let tip = p(end.f - FIST * bulk, end.u);
    let toe_tip = p(end.f - FOOT * bulk, end.u);

    enum Shape { Punch(bool), Kick(bool, f32), Upper, Fly, Sweep, Throw, Bolt, Laser, Rush, Call }
    let shape = match f.mv {
        MoveId::LowPunch | MoveId::HighPunch => Shape::Punch(false),
        MoveId::JumpPunch => Shape::Punch(true),
        MoveId::Uppercut => Shape::Upper,
        MoveId::LowKick => Shape::Kick(false, 0.7),
        MoveId::HighKick => Shape::Kick(false, 1.25),
        MoveId::JumpKick => Shape::Kick(true, 1.0),
        MoveId::FlyingKick => Shape::Fly,
        MoveId::Sweep => Shape::Sweep,
        MoveId::Throw => Shape::Throw,
        MoveId::Special if f.calling => Shape::Call,
        MoveId::Special => match f.arch().special {
            Special::ChiBolt => Shape::Bolt,
            Special::LaserVision => Shape::Laser,
            Special::BullRush => Shape::Rush,
            Special::TalonKick => Shape::Kick(false, 1.25),
        },
    };

    match shape {
        // The epic punch: the fist drawn right back, then the whole body
        // thrown after it, back foot off the floor.
        Shape::Punch(air) if f.flies() => {
            let reach = if ext < 0.0 { ext * 5.0 } else { ext };
            let drive = ext.max(0.0);
            q.hip.f += 14.0 * drive;
            q.head.f += 30.0 * drive - 6.0 * (reach.min(0.0)).abs();
            if !air {
                q.lead_foot.f += 8.0 * drive;
                q.rear_foot = p(q.rear_foot.f - 12.0 * drive, 9.0 * drive);
            }
            q.rear_hand = q.rear_hand.to(p(q.head.f + 6.0, q.head.u - 30.0), drive);
            q.lead_hand = q.lead_hand.to(tip, reach);
            q.rear_heel = 0.9 * drive;
            q.twist = drive;
        }
        Shape::Punch(true) => {
            q.hip.f += 12.0 * ext;
            q.head.f += 24.0 * ext;
            q.lead_foot = p(q.hip.f + 14.0, q.lead_foot.u);
            q.rear_foot = p(q.hip.f - 10.0, q.rear_foot.u.min(24.0));
            q.rear_hand = q.rear_hand.to(p(q.head.f + 6.0, q.head.u - 30.0), ext.max(0.0));
            q.lead_hand = q.lead_hand.to(tip, ext);
            q.twist = ext.max(0.0);
        }
        Shape::Punch(false) => {
            let e = ext.max(0.0);
            // Step in and sit into it; a low one is reached by sinking at the
            // knees, not by folding over.
            let low = ((60.0 - tip.u) / 30.0).clamp(0.0, 1.0) * e;
            q.hip = p(q.hip.f + 4.0 * ext, q.hip.u - 2.0 * e - 9.0 * low);
            q.head.u -= 1.5 * e + 10.0 * low;
            q.lead_foot.f += 8.0 * ext;
            q.rear_foot.f -= 3.0 * ext;
            // The shoulder goes where a straight arm puts it: the body turns
            // and leans just far enough for the fist to land at full reach,
            // and no further.
            const ARM: f32 = UARM + FARM - 3.0;
            let drop = q.hip.u + SPINE * (q.head.u - q.hip.u) - 8.5 - tip.u;
            let shoulder = tip.f - (ARM * ARM - drop * drop).max(0.0).sqrt();
            let lean = (q.hip.f + (shoulder - 7.0 - q.hip.f) / SPINE - q.head.f).clamp(2.0, 24.0);
            q.head.f += lean * ext;
            if f.alt_punch {
                // A chain: the far fist goes straight down the middle from
                // the shoulder that is already forward, and the near one
                // comes back to the chest under it.
                q.rear_hand = q.rear_hand.to(tip, ext);
                q.lead_hand = q.lead_hand.to(p(q.head.f + 9.0, tip.u - 16.0), e.sqrt());
                q.head.f -= lean * 0.45 * ext;
            } else {
                // The far fist snaps back to guard the chin, out of the way
                // before the punch comes through where it was.
                q.rear_hand = q.rear_hand.to(p(q.head.f + 6.0, q.head.u - 22.0), e.sqrt());
                q.lead_hand = q.lead_hand.to(tip, ext);
                // The shoulder comes through with the hip.
                q.twist = e;
            }
            // The back foot turns over onto its ball as the hip comes through.
            q.rear_heel += 0.5 * e;
        }
        Shape::Kick(air, lift) => {
            let k = KICK;
            let e = ext.max(0.0);
            // 0 for the low kick, 1 for the high one.
            let high = ((lift - 0.7) / 0.55).clamp(0.0, 1.0);
            // The higher the kick the further the shoulders go back to pay
            // for it, and the more the standing leg straightens under it.
            let back = if air { k.air_lean * lift } else { 0.35 + 0.90 * high };
            let drive = if air { k.hip_drive + 9.0 } else { k.hip_drive };
            let rise = k.hip_rise + if air { 0.0 } else { 4.0 * high };
            q.hip = p(q.hip.f + drive * ext, q.hip.u + rise * ext);
            q.head.f -= k.lean * 1.25 * ext * back;
            q.head.u -= 1.4 * ext;
            if !air {
                q.rear_foot = p(-18.0 + (q.hip.f - k.plant - -18.0) * ext, 0.0);
            } else {
                q.rear_foot = p(q.hip.f - 7.0, 26.0);
            }
            // Up on the ball of the standing foot. A high kick lands with
            // the foot pointed; a low one with it pulled back, toes up.
            if !air { q.rear_heel = 0.25 + (0.35 + 0.25 * high) * e; }
            q.lead_heel = if !air && high < 0.5 { -0.55 * e } else { 0.45 * e };
            let chamber = p(q.hip.f + k.knee_lead,
                if air { k.chamber_air } else { k.chamber * (0.72 + 0.28 * high) });
            let kc = kick_curve(ext);
            q.lead_foot = if kc < 1.0 {
                q.lead_foot.to(chamber, kc)
            } else {
                chamber.to(toe_tip, kc - 1.0)
            };
            if air && f.stomp {
                // The rage jump: both fists over the head on the way down.
                q.lead_hand = q.lead_hand.to(p(20.0, 128.0), ext);
                q.rear_hand = q.rear_hand.to(p(-12.0, 124.0), ext);
            } else {
                // The far fist stays out as the guard; the near one tucks in
                // at the ribs, in front of the body.
                q.rear_hand = q.rear_hand.to(p(34.0, 70.0), ext);
                let tuck = if air { p(q.hip.f + 10.0, q.hip.u - 6.0) } else { p(q.hip.f + 12.0, q.hip.u + 12.0) };
                q.lead_hand = q.lead_hand.to(tuck, ext);
            }
        }
        Shape::Fly => {
            q.hip = p(q.hip.f + 21.0 * ext, q.hip.u + 7.0 * ext);
            q.head = p(q.head.f - 22.0 * ext, q.head.u - 13.0 * ext);
            q.rear_foot = p(q.hip.f - 24.0 - 10.0 * ext, 14.0 + 4.0 * ext);
            q.lead_foot = q.lead_foot.to(toe_tip, ext);
            q.rear_hand = q.rear_hand.to(p(32.0, 70.0), ext);
            // The near fist is held to the chest, on the front of a body
            // that is laid right back.
            let chest = q.hip.to(q.head, 0.5);
            q.lead_hand = q.lead_hand.to(p(chest.f + 7.0, chest.u + 7.0), ext);
        }
        Shape::Sweep => {
            let hip_f = (tip.f - 52.0).clamp(2.0, 28.0);
            q.hip = p(hip_f * ext, C_HIP + 3.0 * ext);
            q.head = p(q.head.f - 4.0 * ext, C_HEAD - 7.0 * ext);
            q.rear_foot = p(-3.0, 2.0);
            q.lead_foot = q.lead_foot.to(toe_tip, ext);
            q.lead_hand = q.lead_hand.to(p(q.hip.f + 12.0, 24.0), ext);
            q.rear_hand = q.rear_hand.to(p(34.0, 44.0), ext);
            q.rear_heel = 0.7;
            q.lead_heel = 0.0;
        }
        Shape::Throw => {
            q.open = true;
            q.hip.f += 4.0 * ext;
            q.head.f += 8.0 * ext;
            q.lead_foot.f += 7.0 * ext;
            q.rear_hand = q.rear_hand.to(p(tip.f, tip.u + 6.0), ext);
            q.lead_hand = q.lead_hand.to(p(tip.f - 6.0, tip.u - 8.0), ext);
            q.twist = 0.5 * ext.max(0.0);
        }
        Shape::Bolt => {
            q.open = true;
            q.hip = p(2.0, 51.0);
            q.lead_foot = p(21.0, 0.0);
            q.rear_foot = p(-22.0, 0.0);
            // Drawn back to the hip, as far as the fighter's lean leaves room for.
            let charge = p(2.0 + STYLES[f.who].lean, 56.0);
            // The far hand cups the charge from above, and rides out over
            // the near one: wrists together, one palm over the other.
            let cup = p(charge.f + 16.0, charge.u + 6.0);
            let out = p(62.0, 74.0);
            if ext < 0.55 {
                let k = ext / 0.55;
                q.lead_hand = q.lead_hand.to(charge, k);
                q.rear_hand = q.rear_hand.to(cup, k);
                q.head.f -= 6.0 * k;
            } else {
                // The shoulders come round behind the palms as they go out.
                let k = (ext - 0.55) / 0.45;
                // The whole body goes after them: weight onto the front foot,
                // the back leg straightening behind.
                q.hip = p(2.0 + 9.0 * k, 51.0 - 2.0 * k);
                q.lead_foot.f += 7.0 * k;
                q.rear_foot.f -= 5.0 * k;
                q.rear_heel = 0.6 * k;
                q.lead_hand = charge.to(out, k);
                q.rear_hand = cup.to(p(out.f - 5.0, out.u + 11.0), k);
                q.head.f += -6.0 + 24.0 * k;
                q.twist = k;
            }
        }
        // Fists on the hips, chin down, eyes on the target.
        Shape::Laser => {
            q.hip = q.hip.to(p(0.0, HIP_U + 1.0), ext);
            q.head = q.head.to(p(8.0, HEAD_U - 2.0), ext);
            q.lead_hand = q.lead_hand.to(p(11.0, 49.0), ext);
            q.rear_hand = q.rear_hand.to(p(23.0, 56.0), ext);
        }
        // The uppercut: down into the knees with the fist at the hip, then
        // the legs straighten under it and the fist goes up past the chin
        // with the whole body behind it, heels off the floor.
        Shape::Upper => {
            let wind = (-ext).max(0.0);
            let e = ext.max(0.0);
            let rise = e * e;
            q.hip = p(q.hip.f + 6.0 * e, q.hip.u - 12.0 * wind + 9.0 * rise);
            q.head = p(q.head.f + 5.0 * e - 3.0 * rise, q.head.u - 12.0 * wind + 11.0 * rise);
            q.lead_foot.f += 6.0 * e;
            let low = p(q.hip.f + 12.0, q.hip.u - 6.0);
            q.lead_hand = if ext < 0.0 { q.lead_hand.to(low, wind) } else { low.to(tip, e) };
            q.rear_hand = q.rear_hand.to(p(q.head.f + 4.0, q.head.u - 30.0), e.sqrt());
            q.twist = e;
            q.lead_heel = 0.5 * rise;
            q.rear_heel = 0.3 + 0.6 * rise;
        }
        // A fist in the air and a shout over the shoulder: the others are
        // coming.
        Shape::Call => {
            let e = ext.max(0.0);
            q.head = p(q.head.f - 4.0 * e, q.head.u + 2.0 * e);
            q.lead_hand = q.lead_hand.to(p(30.0, 122.0), e);
            q.rear_hand = q.rear_hand.to(p(10.0, 46.0), e);
            q.twist = e;
        }
        Shape::Rush => {
            q.hip = p(8.0 * ext, HIP_U - 5.0 * ext);
            q.head.f += 20.0 * ext;
            q.lead_foot = p(15.0 + 17.0 * ext, 5.0 * ext);
            q.rear_foot.f -= 11.0 * ext;
            q.lead_hand = q.lead_hand.to(tip, ext);
            q.rear_hand = q.rear_hand.to(p(q.head.f + 6.0, q.head.u - 30.0), ext);
            q.rear_heel = 0.8 * ext.max(0.0);
            q.twist = ext.max(0.0);
        }
    }
}

/// The pose a fighter is in this frame.
/// The pose a fighter is drawn in, including the tail of the one before it: actions change on
/// one frame, so the hand-over is eased across four frames (the hitbox still follows the move).
pub(crate) fn pose_of(now: f32, f: &Fighter, idx: usize) -> Pose {
    let q = pose_now(now, f, idx);
    if f.blend <= 0.0 { return q; }
    let mut was = *f;
    was.act = f.prev_act;
    was.mv = f.prev_mv;
    was.t = f.prev_t;
    was.blend = 0.0;
    was.y = if f.prev_air { FLOOR_Y - 1.0 } else { FLOOR_Y };
    was.vy = f.prev_vy;
    let k = (1.0 - f.blend).clamp(0.0, 1.0);
    pose_now(now, &was, idx).to(q, k * k * (3.0 - 2.0 * k))
}

/// Mid-stride, is the far foot the one in front? Where a walk stops, that
/// is the stance it leaves.
pub(crate) fn far_foot_leads(f: &Fighter) -> bool {
    let ph = f.x * f.facing * STRIDE / f.size();
    let (near, _) = foot_cycle(ph, 9.0);
    let (far, _) = foot_cycle(ph + std::f32::consts::PI, 8.0);
    far > near
}

/// Where both feet swing about in a walk, forward of the fighter's centre.
pub(crate) const WALK_MID: f32 = 2.0;
/// How far the hips come up out of the stance to walk.
pub(crate) const WALK_RISE: f32 = 3.5;

/// The idle bounce, -1 to 1, at the fighter's own rate. The two sides are out
/// of phase, or a mirror match reads as one animation played twice.
pub(crate) fn bounce_of(now: f32, who: usize, idx: usize) -> f32 {
    (now * STYLES[who].rate + idx as f32 * 2.3).sin()
}

/// Time into a fall if this fighter is down: knocked down, or beaten by KO
/// (who stays flat instead of kneeling; a time-up loser kneels).
pub(crate) fn down_time(f: &Fighter) -> Option<f32> {
    match f.act {
        Act::Knockdown => Some(f.t),
        Act::Defeat if f.health <= 0 => Some(f.t.min(0.6)),
        _ => None,
    }
}

/// The pose for exactly what `f` is doing now, before any blend from the
/// action before.
pub(crate) fn pose_now(now: f32, f: &Fighter, idx: usize) -> Pose {
    let bob = bounce_of(now, f.who, idx);

    if let Some(t) = down_time(f) {
        let down = floored();
        if t < 0.22 {
            let k = (t / 0.22).clamp(0.0, 1.0);
            let mut air = stance(0.0, f.who);
            air.hip = p(-6.0, 48.0);
            air.head = p(-40.0, 62.0);
            air.lead_foot = p(30.0, 44.0);
            air.rear_foot = p(22.0, 24.0);
            air.lead_hand = p(-8.0, 82.0);
            air.rear_hand = p(-30.0, 66.0);
            return air.to(down, k * k);
        }
        if t > 0.85 {
            let k = ((t - 0.85) / 0.30).clamp(0.0, 1.0);
            let mut up = crouched(0.0);
            up.rear_hand = p(22.0, 16.0);
            up.lead_hand = p(28.0, 34.0);
            up.rear_foot = p(-10.0, 0.0);
            return down.to(up, (k * 1.8).min(1.0)).to(stance(0.0, f.who), (k - 0.55).max(0.0) / 0.45);
        }
        return down;
    }

    let low = f.crouching()
        || (f.act == Act::Hitstun && matches!(f.prev_act, Act::Crouch))
        || (f.act == Act::Hitstun && f.prev_act == Act::Attack
            && matches!(f.prev_mv, MoveId::Sweep))
        || (f.act == Act::Hitstun && f.prev_act == Act::Block && f.crouch_block);
    let mut q = if f.soaring() {
        flying_pose(f.soar)
    } else if f.airborne() {
        airborne_pose(f.vy)
    } else if low {
        crouched(bob)
    } else {
        stance(bob, f.who)
    };

    match f.act {
        Act::Walk => {
            let ph = f.x * f.facing * STRIDE / f.size();
            let s = ph.sin();
            let (lf, ll) = foot_cycle(ph, 9.0);
            let (rf, rl) = foot_cycle(ph + std::f32::consts::PI, 8.0);
            // A real walk: the feet pass each other under the body, so the
            // front leg changes with every step.
            q.lead_foot = p(WALK_MID + lf, ll);
            q.rear_foot = p(WALK_MID + rf, rl);
            q.lead_heel = heel_cycle(ph);
            q.rear_heel = heel_cycle(ph + std::f32::consts::PI);
            // Up out of the stance to walk: the legs are under the body now,
            // and nearly straight as each foot passes.
            // (A fighter who sits low in the stance comes up most of that too.)
            let sink = 1.4 + 1.4 * (2.0 * ph).cos() - WALK_RISE - 0.8 * STYLES[f.who].sink;
            q.hip.u -= sink;
            q.head.u -= sink * 0.7;
            q.hip.f += 1.4 * s;
            q.head.f -= 1.0 * s;
            q.lead_hand.f -= 2.6 * s;
            q.rear_hand.f += 2.2 * s;
            q.lead_hand.u -= sink * 0.7;
            q.rear_hand.u -= sink * 0.7;
        }
        // Turtled up: weight off the front foot, forearms stacked before head or belly.
        Act::Block => {
            let b = bob;
            if f.crouch_block {
                q.lead_hand = p(12.0, 31.0 + b * 0.5);
                q.rear_hand = p(27.0, 42.0 + b * 0.7);
                q.head.f -= 4.0;
            } else {
                q.hip = p(-4.0, HIP_U - 2.0 + b * 0.6);
                q.head = p(-8.0 - b * 0.7, HEAD_U - 2.0 + b * 0.4);
                q.lead_hand = p(13.0 + b * 0.4, 73.0 + b * 0.6);
                q.rear_hand = p(26.0 + b * 0.6, 88.0 + b * 0.8);
                let ph = f.x * f.facing * STRIDE / f.size();
                let sink = 1.0 + (2.0 * ph).cos();
                let (lf, ll) = foot_cycle(ph, 7.0);
                let (rf, rl) = foot_cycle(ph + std::f32::consts::PI, 6.0);
                q.lead_foot = p(14.0 + lf, ll);
                q.rear_foot = p(-17.0 + rf, rl);
                q.lead_heel = heel_cycle(ph) * 0.6;
                q.rear_heel = heel_cycle(ph + std::f32::consts::PI) * 0.6;
                q.hip.u -= sink;
                q.head.u -= sink * 0.7;
                q.lead_hand.u -= sink * 0.7;
                q.rear_hand.u -= sink * 0.7;
            }
        }
        // Snapped back off the blow: head first, back foot skidding out.
        Act::Hitstun => {
            // The head goes first and furthest, and comes back last.
            let k = (1.0 - f.t * 3.4).clamp(0.40, 1.0);
            let drop = if low { 0.5 } else { 1.0 };
            q.hip = p(q.hip.f - 6.0 * k, q.hip.u - 3.0 * k);
            q.head = p(q.head.f - 24.0 * k * drop, q.head.u - 1.6);
            q.lead_foot = p(q.lead_foot.f - 10.0, 0.0);
            q.rear_foot = p(q.rear_foot.f - 6.0, 0.0);
            // The arms are thrown up and out by it, and drop back in.
            let fling = ((k - 0.4) / 0.6).max(0.0);
            q.lead_hand = p(q.lead_hand.f - 9.0 - 4.0 * fling, q.lead_hand.u - 5.0 + 9.0 * fling);
            q.rear_hand = p(q.rear_hand.f - 7.0 + 6.0 * fling, q.rear_hand.u - 9.0 + 12.0 * fling);
            // Seeing stars: swaying on the spot, arms hanging.
            if f.dizzy > 0.0 {
                let sway = (now * 5.0).sin();
                q.head = p(q.head.f + 6.0 * sway, q.head.u - 2.0);
                q.hip.f += 2.0 * sway;
                q.lead_hand = p(10.0 + 3.0 * sway, 50.0);
                q.rear_hand = p(20.0 + 3.0 * sway, 52.0);
            }
            // Rocked back onto flat feet, the hands knocked open.
            q.open = true;
            q.lead_heel = 0.0;
            q.rear_heel = 0.0;
        }
        Act::Attack => {
            let m = f.scaled(move_data(f.mv));
            let ext = extension(f, &m);
            attack_pose(&mut q, f, &m, ext);
        }
        // Won: guard drops, the arm sweeps up and forward (raised straight it hides the face), then held.
        Act::Victory => {
            let t = f.t;
            let settle = (t / 0.30).clamp(0.0, 1.0);
            let raise = ((t - 0.26) / 0.34).clamp(0.0, 1.0);
            let sweep = raise * raise * (3.0 - 2.0 * raise);
            let bob = ((t - 0.6).max(0.0) * 2.2).sin() * 1.8;

            q.head = p(-1.5 + 2.0 * settle - 2.0 * sweep,
                       HEAD_U + 1.2 * settle + 1.5 * sweep + bob * 0.3);
            q.hip = p(0.0, HIP_U + 1.0 * settle);
            q.lead_foot = p(16.0 - 3.0 * settle, 0.0);
            q.rear_foot = p(-21.0 + 4.0 * settle, 0.0);
            q.rear_heel *= 1.0 - settle;
            let ease = |from: f32, len: f32| {
                let k = ((t - from) / len).clamp(0.0, 1.0);
                k * k * (3.0 - 2.0 * k)
            };
            if STYLES[f.who].cheer {
                // Both arms up, the far one first so the near one comes up
                // outside it.
                q.rear_hand = q.rear_hand.to(p(20.0, 126.0 - bob), ease(0.0, 0.35));
                let lift = ease(0.12, 0.45);
                q.lead_hand = q.lead_hand.to(p(38.0, 118.0 + bob), lift);
                q.twist = lift;
            } else {
                // One fist straight up, the shoulder coming round under it;
                // the other waits for it to pass, then comes down to the hip.
                let lift = ease(0.0, 0.5);
                q.lead_hand = q.lead_hand.to(p(36.0, 120.0 + bob), lift);
                q.rear_hand = q.rear_hand.to(p(8.0, 42.0), ease(0.14, 0.30));
                q.twist = lift;
            }
        }
        // Rei: heels together, open hands at thighs, bow about 30°, then blend back to Idle.
        Act::Bow => {
            let ease = |from: f32, len: f32| {
                let k = ((f.t - from) / len).clamp(0.0, 1.0);
                k * k * (3.0 - 2.0 * k)
            };
            let bow = ease(0.3, 0.42) * (1.0 - ease(1.0, 0.42));
            // standing tall: legs all but straight, the hips going back as
            // the torso tips forward
            let hip = p(-6.0 * bow, HIP_U + 5.5 - 1.5 * bow);
            let spine = HEAD_U - HIP_U + 1.0;
            let lean = 0.62 * bow; // about 35 degrees at the bottom
            let head = p(hip.f + spine * lean.sin() + 3.0 * bow, hip.u + spine * lean.cos() - 3.0 * bow);
            q = Pose {
                hip,
                head,
                // open hands at the sides of the thighs, sliding toward the knees
                lead_hand: p(3.0 + 12.0 * bow, 46.0 - 12.0 * bow),
                rear_hand: p(11.0 + 12.0 * bow, 44.0 - 12.0 * bow),
                lead_foot: p(6.0, 0.0),
                rear_foot: p(-6.0, 0.0),
                open: true,
                lead_heel: 0.0,
                rear_heel: 0.0,
                twist: 0.0,
            };
        }
        Act::Defeat => {
            let fall = |from: f32, len: f32| {
                let k = ((f.t - from) / len).clamp(0.0, 1.0);
                k * k * (3.0 - 2.0 * k)
            };
            let (hips, slump) = (fall(0.0, 0.55), fall(0.2, 0.8));
            let heave = (f.t * 2.4).sin() * slump;
            q.hip = q.hip.to(p(-4.0, 30.0), hips);
            q.head = q.head.to(p(25.0, 72.0 + heave * 0.8), slump);
            q.lead_foot = q.lead_foot.to(p(22.0, 0.0), hips);
            q.rear_foot = q.rear_foot.to(p(-34.0, 3.0), hips);
            q.rear_heel = 0.9 * hips;
            q.lead_heel = 0.0;
            q.lead_hand = q.lead_hand.to(p(-2.0, 18.0 + heave * 0.5), slump);
            q.rear_hand = q.rear_hand.to(p(28.0, 28.0), slump);
        }
        _ => {}
    }
    if f.land > 0.0 && !f.airborne()
        && matches!(f.act, Act::Idle | Act::Walk | Act::Crouch | Act::Block) {
        let k = 1.0 - (f.land / LAND_ABSORB).clamp(0.0, 1.0);
        let room = if low { 0.3 } else { 1.0 };
        let dip = (std::f32::consts::PI * k).sin() * (1.0 - k) * f.land_force * room;
        q.hip.u -= 9.0 * dip;
        q.head.u -= 6.5 * dip;
        q.lead_hand.u -= 6.0 * dip;
        q.rear_hand.u -= 6.0 * dip;
        q.lead_foot.f += 2.0 * dip;
        q.rear_foot.f -= 2.0 * dip;
        q.lead_heel *= 1.0 - dip.min(1.0);
        q.rear_heel *= 1.0 - dip.min(1.0);
    }

    if !f.airborne() && f.act != Act::Knockdown {
        q.lead_foot.u = q.lead_foot.u.max(0.0);
        q.rear_foot.u = q.rear_foot.u.max(0.0);
    }
    // The other foot forward: every pose that stands on the stance changes
    // legs, and a kick comes off whichever leg is in front. (A walk is
    // already on both; the bow and the end of the round stand square.)
    if f.far_leads && !f.airborne()
        && matches!(f.act, Act::Idle | Act::Crouch | Act::Block | Act::Hitstun | Act::Attack) {
        std::mem::swap(&mut q.lead_foot, &mut q.rear_foot);
        std::mem::swap(&mut q.lead_heel, &mut q.rear_heel);
    }
    q
}

/// Every joint of one fighter in screen coordinates, after the solver. The
/// fighter is drawn from it and the anatomy tests run against it.
#[derive(Copy, Clone, Debug)]
pub(crate) struct Skeleton {
    pub hip: V,
    pub neck: V,
    pub head: V,
    pub hip_lead: V,
    pub hip_rear: V,
    pub sh_lead: V,
    pub sh_rear: V,
    pub knee_lead: V,
    pub knee_rear: V,
    pub ankle_lead: V,
    pub ankle_rear: V,
    pub elbow_lead: V,
    pub elbow_rear: V,
    pub hand_lead: V,
    pub hand_rear: V,
}

#[cfg_attr(not(test), allow(dead_code))]
impl Skeleton {
    /// The solved bones, with the length each one is supposed to be.
    /// A bone whose ends are not that far apart is a bone that got
    /// stretched, and a stretched bone is a joint in the wrong place.
    pub fn bones(&self) -> [(&'static str, V, V, f32); 8] {
        [
            ("thigh-lead", self.hip_lead, self.knee_lead, THIGH),
            ("shin-lead", self.knee_lead, self.ankle_lead, SHIN),
            ("thigh-rear", self.hip_rear, self.knee_rear, THIGH),
            ("shin-rear", self.knee_rear, self.ankle_rear, SHIN),
            ("upperarm-lead", self.sh_lead, self.elbow_lead, UARM),
            ("forearm-lead", self.elbow_lead, self.hand_lead, FARM),
            ("upperarm-rear", self.sh_rear, self.elbow_rear, UARM),
            ("forearm-rear", self.elbow_rear, self.hand_rear, FARM),
        ]
    }

    /// Each hinge, as (name, the joint, its two neighbours, how far it
    /// is allowed to shut).
    pub fn hinges(&self) -> [(&'static str, V, V, V, f32); 4] {
        [
            ("knee-lead", self.knee_lead, self.hip_lead, self.ankle_lead, KNEE_SHUT),
            ("knee-rear", self.knee_rear, self.hip_rear, self.ankle_rear, KNEE_SHUT),
            ("elbow-lead", self.elbow_lead, self.sh_lead, self.hand_lead, ELBOW_SHUT),
            ("elbow-rear", self.elbow_rear, self.sh_rear, self.hand_rear, ELBOW_SHUT),
        ]
    }
}

/// Solve one fighter's pose into joints. `rig` maps pose space to the screen.
pub(crate) fn skeleton(rig: Rig, q: &Pose) -> Skeleton {
    let hip = rig.at(q.hip);
    let neck = rig.at(neck_of(q));
    let head = rig.at(q.head);

    // Sockets hang off the spine and turn with it; fixed screen offsets put
    // the arm out of the wrong side of a leaning chest.
    let (sx, sy) = unit(hip, neck);
    let (px, py) = (-sy, sx); // across the body, square to the spine
    let socket = |at: V, half: f32, up: f32| {
        (V(at.0 + px * half * rig.fwd + sx * up, at.1 + py * half * rig.fwd + sy * up),
         V(at.0 - px * half * rig.fwd + sx * up, at.1 - py * half * rig.fwd + sy * up))
    };
    // Sockets well out from the spine: that span plus the deltoids is the
    // shoulders, the wedge a fighting sprite is drawn as.
    let z = rig.scale;
    // An open stance, as every 2D fighter stands: the chest is turned to the
    // viewer, the near shoulder at the back edge of it and the far one at
    // the front. A punch turns the near shoulder through to the front.
    let t = q.twist.clamp(0.0, 1.0);
    let (sh_lead, _) = socket(neck, (-7.0 + 15.6 * t) * z, -8.5 * z);
    let (sh_rear, _) = socket(neck, (8.0 - 10.0 * t) * z, -8.5 * z);
    // Both legs hang from one pelvis: the far hip is just behind the near
    // one, not out at the back of the body.
    let (hip_lead, _) = socket(hip, 4.5 * z, 0.0);
    let (_, hip_rear) = socket(hip, 1.5 * z, 0.0);

    // Apply parallax to far-limb targets to preserve bone lengths; far feet rise toward the horizon.
    let hand_back = |v: V| V(v.0 - rig.fwd * 3.0 * z, v.1 + z);
    let foot_back = |v: V| V(v.0 - rig.fwd * 3.5 * z, v.1 - 1.5 * z);

    let leg_at = |root: V, want: V| {
        // Derive the knee bend side from the thigh direction; a fixed forward vector inverted high kicks.
        let (dx, dy) = unit(root, want);
        let anterior = V(dy * rig.fwd, -dx * rig.fwd);
        solve(root, want, THIGH * z, SHIN * z, KNEE_SHUT, anterior)
    };
    let arm_at = |root: V, want: V| {
        // Bend elbows behind the arm; a fixed direction caused mirrored-solution flips.
        // Push hands away from shoulders when a near-closed elbow has no stable solution.
        let (dx, dy) = unit(root, want);
        let reach = dist(root, want).max(ARM_MIN * z);
        let want = V(root.0 + dx * reach, root.1 + dy * reach);
        let posterior = V(-dy * rig.fwd, dx * rig.fwd);
        solve(root, want, UARM * z, FARM * z, ELBOW_SHUT, posterior)
    };

    let (knee_lead, ankle_lead) = leg_at(hip_lead, rig.at(q.lead_foot));
    let (knee_rear, ankle_rear) = leg_at(hip_rear, foot_back(rig.at(q.rear_foot)));
    let (elbow_lead, hand_lead) = arm_at(sh_lead, rig.at(q.lead_hand));
    let (elbow_rear, hand_rear) = arm_at(sh_rear, hand_back(rig.at(q.rear_hand)));

    Skeleton {
        hip, neck, head, hip_lead, hip_rear, sh_lead, sh_rear,
        knee_lead, knee_rear, ankle_lead, ankle_rear,
        elbow_lead, elbow_rear, hand_lead, hand_rear,
    }
}

/// The unit vector from `a` to `b`.
pub(crate) fn unit(a: V, b: V) -> (f32, f32) {
    let d = dist(a, b).max(0.001);
    ((b.0 - a.0) / d, (b.1 - a.1) / d)
}
