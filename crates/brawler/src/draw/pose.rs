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

/// How one fighter carries themself: offsets on the shared stance, so the
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
    /// How they celebrate a win.
    pub(crate) win: Win,
}

/// A `Style`, positionally: what `STYLES` is written in.
pub(crate) const fn style(rate: f32, amp: f32, sink: f32, wide: f32, lean: f32, lead: P, rear: P,
               open: bool, win: Win) -> Style {
    Style { rate, amp, sink, wide, lean, lead, rear, open, win }
}

/// A win, each fighter's own.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum Win {
    /// One fist straight up.
    Fist,
    /// Both arms in the air.
    Both,
    /// Both arms bent, showing the muscle, and a roar.
    Flex,
    /// Too pleased to keep still: hopping on the spot, both arms up.
    Hop,
    /// A fist pumped down at the hip, again and again.
    Pump,
}

/// One per fighter, in roster order.
pub(crate) const STYLES: [Style; FIGHTERS.len()] = [
    // RYUKA: the textbook karate stance.
    style(4.4, 1.0, 0.0, 0.0, 0.0, p(0.0, 0.0), p(0.0, 0.0), false, Win::Fist),
    // BRUTUS: a wrestler, wide and low, open hands ready to grab.
    style(3.0, 0.8, 3.0, 4.0, 2.0, p(-4.0, -8.0), p(-4.0, -13.0), true, Win::Flex),
    // KESTREL: a kickboxer on the toes, hands high.
    style(6.5, 1.4, 0.0, -5.0, -2.0, p(-2.0, 8.0), p(5.0, 3.0), false, Win::Hop),
    // GIOTTO: the disciplined one, rear fist by the chin.
    style(4.0, 0.9, 1.0, 0.0, 0.0, p(2.0, -2.0), p(3.0, -4.0), false, Win::Fist),
    // TITIAN: the hothead, hunched with the fists low.
    style(3.6, 1.0, 4.0, 3.0, 7.0, p(-6.0, -6.0), p(-2.0, -10.0), false, Win::Pump),
    // VERMEER: tall, the lead hand held out long and open.
    style(4.2, 0.8, 0.0, -2.0, -3.0, p(5.0, 4.0), p(-4.0, -6.0), true, Win::Fist),
    // BOSCH: never still.
    style(7.5, 1.8, 1.0, 0.0, 2.0, p(-4.0, 6.0), p(0.0, -12.0), false, Win::Hop),
    // WEBBER: low to the floor, fingers spread.
    style(5.5, 1.2, 7.0, 8.0, 8.0, p(-2.0, -18.0), p(8.0, -32.0), true, Win::Hop),
    // GAMMA: hunched, the arms hanging heavy.
    style(2.6, 1.2, 4.0, 6.0, 9.0, p(-8.0, -14.0), p(-4.0, -20.0), false, Win::Flex),
    // ZENITH: upright and unbothered, the fists low.
    style(3.2, 0.7, 0.0, -3.0, -3.0, p(-8.0, -14.0), p(-5.0, -22.0), false, Win::Fist),
    style(6.0, 1.2, 0.0, 1.0, -2.0, p(1.0, 7.0), p(-1.0, 8.0), true, Win::Hop),
    style(2.8, 0.5, 2.0, 5.0, 3.0, p(-5.0, -2.0), p(-4.0, -10.0), false, Win::Both),
    style(6.4, 1.0, 0.0, 0.0, -2.0, p(1.0, 7.0), p(3.0, 3.0), false, Win::Pump),
    style(2.7, 0.7, 4.0, 5.0, 1.0, p(-1.0, -3.0), p(-4.0, -9.0), true, Win::Flex),
    // MACK: loose, grounded boxer's guard; weight rides the beat.
    style(4.8, 1.1, 1.0, 2.0, 1.0, p(-3.0, -1.0), p(1.0, -5.0), false, Win::Pump),
];

/// The stance: weight back, knees bent, both hands up, side on. Everything
/// departs from and returns to this. `bob` is the idle bounce, -1..1: the
/// whole body rides it, the hands a little further.
pub(crate) fn stance(bob: f32, who: usize) -> Pose {
    let s = &STYLES[who];
    let bob = bob * s.amp;
    Pose {
        // (The bounce is a fighter's: at 1.2 px of hip it read as standing still.)
        hip: p(0.0, HIP_U - 1.0 - s.sink + bob * 1.8),
        head: p(3.0 + s.lean, HEAD_U - 1.0 - s.sink - s.lean * 0.35 + bob * 2.1),
        // The near fist is held back at the chest; the far one leads, out
        // past it and higher, so both show.
        lead_hand: p(22.0 + s.rear.f, 70.0 + s.rear.u - s.sink + bob * 2.6),
        rear_hand: p(35.0 + s.lead.f, 79.0 + s.lead.u - s.sink + bob * 3.0),
        lead_foot: p(24.0 + s.wide, 0.0),
        rear_foot: p(-22.0 - s.wide, 0.0),
        open: s.open,
        // The quick ones (a bounce rate over 5.5) are up on their toes, a heel at a time.
        lead_heel: if s.rate > 5.5 { 0.35 * bob.max(0.0) } else { 0.0 },
        rear_heel: 0.30 + 0.10 * bob + if s.rate > 5.5 { 0.25 * (-bob).max(0.0) } else { 0.0 },
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
            // The slower the blow, the further it is drawn back first: a jab barely, a high kick a lot.
            let draw = (0.08 + 0.01 * m.startup).min(0.24);
            return -draw * (std::f32::consts::PI * k / COIL).sin();
        }
        let k = (k - COIL) / (1.0 - COIL);
        k * k * (3.0 - 2.0 * k)
    } else if f.t <= end {
        // A snap at full stretch: a touch past, and settled back by the end of the hit.
        let k = ((f.t - start) / (m.active * F).max(0.0001)).clamp(0.0, 1.0);
        1.0 + 0.04 * (std::f32::consts::PI * k).sin() * (1.0 - k)
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
        MoveId::LowPunch | MoveId::HighPunch | MoveId::Hook => Shape::Punch(false),
        MoveId::JumpPunch => Shape::Punch(true),
        MoveId::Uppercut => Shape::Upper,
        MoveId::LowKick => Shape::Kick(false, 0.7),
        MoveId::HighKick => Shape::Kick(false, 1.25),
        MoveId::Roundhouse => Shape::Kick(false, 0.95),
        MoveId::JumpKick => Shape::Kick(true, 1.0),
        MoveId::FlyingKick => Shape::Fly,
        MoveId::Sweep => Shape::Sweep,
        MoveId::Throw => Shape::Throw,
        MoveId::Special if f.calling => Shape::Call,
        MoveId::Special => match f.arch().special {
            Special::ChiBolt => Shape::Bolt,
            Special::LaserVision => Shape::Laser,
            Special::BullRush => Shape::Rush,
            Special::TalonKick if f.arch().build == Build::Commando => Shape::Fly,
            Special::DriveBy => Shape::Bolt,
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
            if f.mv == MoveId::Hook {
                // A hook comes round, not straight: the fist swings out and
                // up on its way in, and the body turns hard behind it.
                let swing = (std::f32::consts::PI * e).sin();
                q.lead_hand.u += 10.0 * swing;
                q.lead_hand.f -= 6.0 * swing;
                q.hip.f += 3.0 * e;
                q.twist = 1.0f32.min(q.twist + 0.4 * e);
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
            // A throw that caught someone hoists them and hurls them on: the knees dip, then both arms drive
            // up and out over the head, the body rising behind them. A missed one only reaches.
            let held = (m.startup + m.active) * F;
            if f.hit_clean && f.t > held {
                let k = ((f.t - held) / (m.recovery * F * 0.7)).clamp(0.0, 1.0);
                let dip = (k * std::f32::consts::PI).sin() * (1.0 - k);
                let hurl = k * k * (3.0 - 2.0 * k);
                // Up and out, as far as the arms reach (higher, both fists clamped to one point).
                let top = p(tip.f + 6.0, 112.0);
                q.rear_hand = p(tip.f, tip.u + 6.0).to(top, hurl);
                q.lead_hand = p(tip.f - 6.0, tip.u - 8.0).to(p(top.f - 16.0, top.u - 16.0), hurl);
                q.hip = p(4.0 + 3.0 * hurl, q.hip.u - 8.0 * dip + 3.0 * hurl);
                q.head = p(q.head.f + 8.0 - 6.0 * hurl, q.head.u - 6.0 * dip + 4.0 * hurl);
                q.lead_foot.f = q.lead_foot.f.max(14.0);
                q.rear_heel = 0.3 + 0.5 * hurl;
                q.twist = 0.5 + 0.5 * hurl;
            }
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
            // The bolt leaves with a kick back through the arms: hands, head and hips jolt back for a
            // few frames after it goes, then settle into the recovery.
            let gone = f.t - m.startup * F;
            if gone > 0.0 {
                let r = (1.0 - gone / (8.0 * F)).max(0.0);
                let r = r * r;
                q.lead_hand.f -= 9.0 * r;
                q.rear_hand.f -= 9.0 * r;
                q.head.f -= 6.0 * r;
                q.hip.f -= 2.0 * r;
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
    let q = blended_pose(now, f, idx);
    if f.turn <= 0.0 { return q; }
    // Turning round: from the pose as it was, seen from the new side (every point mirrored), to the pose
    // now. Halfway through, the body is side on.
    let mut was = q;
    for j in [&mut was.hip, &mut was.head, &mut was.lead_hand, &mut was.rear_hand, &mut was.lead_foot, &mut was.rear_foot] {
        j.f = -j.f;
    }
    let k = 1.0 - f.turn;
    was.to(q, k * k * (3.0 - 2.0 * k))
}

/// The pose with the tail of the action before it.
pub(crate) fn blended_pose(now: f32, f: &Fighter, idx: usize) -> Pose {
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
            // Swept: the feet go out forward and up, and the body drops straight down onto its back.
            if f.downed_by == MoveId::Sweep {
                let mut cut = stance(0.0, f.who);
                cut.hip = p(4.0, 40.0);
                cut.head = p(-16.0, 84.0);
                cut.lead_foot = p(40.0, 30.0);
                cut.rear_foot = p(30.0, 18.0);
                cut.lead_hand = p(-4.0, 76.0);
                cut.rear_hand = p(-22.0, 70.0);
                return cut.to(down, k * k);
            }
            // Thrown, the body goes over: it starts half a turn further round and comes down flat.
            if f.downed_by == MoveId::Throw { return spun(air.to(down, k * k), -std::f32::consts::PI * (1.0 - k) * (1.0 - k)); }
            // Uppercut, the body is lifted off the boards first and comes down in an arc.
            let mut q = air.to(down, k * k);
            if f.downed_by == MoveId::Uppercut {
                let lift = 30.0 * (std::f32::consts::PI * k).sin();
                for j in [&mut q.hip, &mut q.head, &mut q.lead_hand, &mut q.rear_hand, &mut q.lead_foot, &mut q.rear_foot] {
                    j.u += lift;
                }
            }
            return q;
        }
        if t > 0.85 && nimble(f) {
            // The acrobats kip up: knees pulled over the chest, hands by the head, then the whole body
            // springs off the boards and lands in the crouch.
            let k = ((t - 0.85) / 0.30).clamp(0.0, 1.0);
            let tuck = Pose {
                hip: p(2.0, 14.0),
                head: p(-36.0, 12.0),
                lead_hand: p(-30.0, 26.0),
                rear_hand: p(-22.0, 30.0),
                // Up but still forward of the hips: over them, the knees flipped from one bend to the other.
                lead_foot: p(24.0, 40.0),
                rear_foot: p(16.0, 34.0),
                open: true,
                lead_heel: 0.6,
                rear_heel: 0.6,
                twist: 0.0,
            };
            let spring = Pose {
                hip: p(0.0, 52.0),
                head: p(-4.0, 100.0),
                lead_hand: p(14.0, 74.0),
                rear_hand: p(24.0, 82.0),
                // (Feet well forward: the hips rising straight past an ankle flipped the knee in one frame.)
                lead_foot: p(24.0, 16.0),
                rear_foot: p(16.0, 12.0),
                open: false,
                lead_heel: 0.5,
                rear_heel: 0.5,
                twist: 0.0,
            };
            let ease = |a: f32, b: f32| { let x = ((k - a) / (b - a)).clamp(0.0, 1.0); x * x * (3.0 - 2.0 * x) };
            return down.to(tuck, ease(0.0, 0.30)).to(spring, ease(0.30, 0.58)).to(crouched(0.0), ease(0.58, 0.78))
                .to(stance(0.0, f.who), ease(0.78, 1.0));
        }
        if t > 0.85 {
            let k = ((t - 0.85) / 0.30).clamp(0.0, 1.0);
            let mut up = crouched(0.0);
            up.rear_hand = p(22.0, 16.0);
            up.lead_hand = p(28.0, 34.0);
            up.rear_foot = p(-10.0, 0.0);
            return down.to(up, (k * 1.8).min(1.0)).to(stance(0.0, f.who), (k - 0.55).max(0.0) / 0.45);
        }
        // The boards give the body back: one bounce, and a smaller one.
        let mut down = down;
        let b = if (0.22..0.40).contains(&t) { 9.0 * (std::f32::consts::PI * (t - 0.22) / 0.18).sin() }
            else if (0.40..0.50).contains(&t) { 2.5 * (std::f32::consts::PI * (t - 0.40) / 0.10).sin() } else { 0.0 };
        for j in [&mut down.hip, &mut down.head, &mut down.lead_hand, &mut down.rear_hand, &mut down.lead_foot, &mut down.rear_foot] {
            j.u += b;
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
            let toes = 1.0 + 0.4 * (1.0 - (f.arch().bulk - 0.85).clamp(0.0, 0.4) / 0.4);
            q.lead_heel = heel_cycle(ph) * toes;
            q.rear_heel = heel_cycle(ph + std::f32::consts::PI) * toes;
            // Up out of the stance to walk: the legs are under the body now,
            // and nearly straight as each foot passes.
            // (A fighter who sits low in the stance comes up most of that too.)
            // How much the body rides each step is the fighter's weight: the heavies sink into every one,
            // the light ones are springy on their toes.
            let tread = (f.arch().bulk - 0.85).clamp(0.0, 0.4) / 0.4;
            let ride = 0.9 + 1.6 * tread;
            let sink = ride + ride * (2.0 * ph).cos() - WALK_RISE - 0.8 * STYLES[f.who].sink;
            q.hip.u -= sink;
            q.head.u -= sink * 0.7;
            q.hip.f += 1.4 * s;
            q.head.f -= 1.0 * s;
            q.lead_hand.f -= 2.6 * s;
            q.rear_hand.f += 2.2 * s;
            q.lead_hand.u -= sink * 0.7;
            q.rear_hand.u -= sink * 0.7;
            // Into the advance, chin first. (Walking back is the guard, below.)
            q.head.f += 4.0;
            q.hip.f += 1.5;
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
            // A blocked blow (its blockstun is running) drives the forearms back into the body, and the
            // head with them.
            let jolt = (f.stun / (10.0 * F)).clamp(0.0, 1.0);
            let jolt = jolt * jolt;
            q.lead_hand.f -= 6.0 * jolt;
            q.rear_hand.f -= 7.0 * jolt;
            q.head.f -= 4.0 * jolt;
            q.hip.f -= 2.0 * jolt;
        }
        // Snapped back off the blow: head first, back foot skidding out.
        Act::Hitstun => {
            // The head goes first and furthest, and comes back last.
            let k = (1.0 - f.t * 3.4).clamp(0.40, 1.0);
            let drop = if low { 0.5 } else { 1.0 };
            let force = f.struck_force;
            let fling = ((k - 0.4) / 0.6).max(0.0);
            if f.struck_legs && !low {
                // Kicked in the shins: the front knee gives and the foot is swept back under, the body
                // dropping over it and the arms out for balance.
                // (The feet stay put: swept back, the weight went past them.)
                let give = k * force;
                q.hip = p(q.hip.f + 1.0 * give, q.hip.u - 6.0 * give);
                q.head = p(q.head.f + 3.0 * give, q.head.u - 7.0 * give);
                q.lead_foot = p(q.lead_foot.f - 3.0 * give, 0.0);
                q.lead_heel = 0.5 * give;
                q.lead_hand = p(q.lead_hand.f - 8.0 * fling, q.lead_hand.u - 4.0 * give);
                q.rear_hand = p(q.rear_hand.f + 6.0 * fling, q.rear_hand.u - 8.0 * give);
            } else if f.struck_body && !low {
                // Winded: doubled over the blow, the hips pushed back, both hands to the gut, the knees giving.
                let fold = k * force;
                q.hip = p(q.hip.f - 9.0 * fold, q.hip.u - 7.0 * fold);
                q.head = p(q.head.f + 10.0 * fold, q.head.u - 22.0 * fold);
                // (Held further out than the belly itself: a heavy's forward lean put the far hand
                // through his back.)
                let belly = q.hip.f.max(q.head.f - 10.0);
                q.lead_hand = q.lead_hand.to(p(belly + 18.0, q.hip.u + 4.0), 0.85 * k);
                q.rear_hand = q.rear_hand.to(p(belly + 26.0, q.hip.u + 11.0), 0.85 * k);
            } else {
                q.hip = p(q.hip.f - 6.0 * k * force, q.hip.u - 3.0 * k);
                q.head = p(q.head.f - 24.0 * k * drop * force, q.head.u - 1.6 + 3.0 * fling * (force - 1.0).max(0.0));
                // The arms are thrown up and out by it, and drop back in.
                q.lead_hand = p(q.lead_hand.f - 9.0 - 4.0 * fling * force, q.lead_hand.u - 5.0 + 9.0 * fling * force);
                q.rear_hand = p(q.rear_hand.f - 7.0 + 6.0 * fling, q.rear_hand.u - 9.0 + 12.0 * fling * force);
            }
            q.lead_foot = p(q.lead_foot.f - 10.0, 0.0);
            q.rear_foot = p(q.rear_foot.f - 6.0 * force, 0.0);
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
            let win = STYLES[f.who].win;
            if win == Win::Flex {
                // The crunch: hunched forward over both fists, knuckles together in front of the belly,
                // head pushed out roaring, and squeezed again and again. (Arms up beside the head read,
                // side on, as a punch.)
                let lift = ease(0.0, 0.4);
                let squeeze = ((t - 0.6).max(0.0) * 6.0).sin().abs() * lift;
                q.hip = p(q.hip.f - 3.0 * lift, q.hip.u - 7.0 * lift - 2.0 * squeeze);
                q.head = p(q.head.f + 12.0 * lift, q.head.u - 12.0 * lift - 3.0 * squeeze);
                q.lead_hand = q.lead_hand.to(p(22.0 - 3.0 * squeeze, 48.0), lift);
                q.rear_hand = q.rear_hand.to(p(36.0 - 2.0 * squeeze, 64.0), ease(0.06, 0.4));
                q.lead_foot.f += 6.0 * lift;
                q.rear_foot.f -= 6.0 * lift;
                q.twist = 0.0;
                q.open = false;
            } else if win == Win::Pump {
                // The fist comes down to the hip, hard, three times a second, the knees dipping with it.
                let on = ease(0.0, 0.3);
                let pump = ((t - 0.3).max(0.0) * 9.0).sin().max(0.0) * on;
                q.lead_hand = q.lead_hand.to(p(30.0, 92.0 - 30.0 * pump), on);
                q.rear_hand = q.rear_hand.to(p(10.0, 50.0), on);
                q.hip.u -= 3.0 * pump;
                q.head.u -= 3.0 * pump;
                q.head.f += 3.0 * pump;
                q.twist = 0.5 * pump;
            } else if matches!(win, Win::Both | Win::Hop) {
                // Both arms up, the far one first so the near one comes up
                // outside it.
                q.rear_hand = q.rear_hand.to(p(20.0, 126.0 - bob), ease(0.0, 0.35));
                let lift = ease(0.12, 0.45);
                q.lead_hand = q.lead_hand.to(p(38.0, 118.0 + bob), lift);
                q.twist = lift;
                if win == Win::Hop {
                    // Bouncing on the toes, the knees giving at the bottom of each. (Off the boards, the
                    // head left the hurtbox by 8 px.)
                    let ph = ((t - 0.4).max(0.0) * 7.0).sin();
                    let up = ph.max(0.0);
                    let give = (-ph).max(0.0) * 5.0;
                    q.hip.u -= give;
                    q.head.u -= give * 0.8;
                    q.lead_hand.u -= give * 0.6;
                    q.rear_hand.u -= give * 0.6;
                    q.lead_heel = 0.7 * up;
                    q.rear_heel = 0.7 * up;
                }
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
        Act::Bow if intro_of(f.who) != Intro::Bow && !f.bows => {
            let ease = |from: f32, len: f32| {
                let k = ((f.t - from) / len).clamp(0.0, 1.0);
                k * k * (3.0 - 2.0 * k)
            };
            let on = ease(0.15, 0.3) * (1.0 - ease(1.25, 0.3));
            // Standing tall to start from: feet under the hips, hands down.
            let mut tall = q;
            tall.hip = p(0.0, HIP_U + 4.0);
            tall.head = p(1.0, HEAD_U + 3.0);
            tall.lead_foot = p(10.0, 0.0);
            tall.rear_foot = p(-10.0, 0.0);
            tall.lead_hand = p(6.0, 48.0);
            tall.rear_hand = p(14.0, 46.0);
            tall.lead_heel = 0.0;
            tall.rear_heel = 0.0;
            let mut show = tall;
            match intro_of(f.who) {
                Intro::Pound => {
                    // Two fists beaten on the chest, one after the other, the head back roaring.
                    let beat = (f.t * 9.0).sin();
                    show.lead_hand = p(16.0 + 9.0 * beat.max(0.0), 82.0);
                    show.rear_hand = p(36.0 + 9.0 * (-beat).max(0.0), 92.0);
                    show.head = p(-3.0, HEAD_U + 2.0);
                    show.lead_foot = p(18.0, 0.0);
                    show.rear_foot = p(-18.0, 0.0);
                    show.hip.u -= 3.0;
                }
                Intro::Hips => {
                    // Fists on the hips, chest out, and one slow nod. (Folded arms read, side on, as a guard.)
                    let nod = (1.0 - ((f.t - 0.85) / 0.22).abs()).max(0.0);
                    show.lead_hand = p(11.0, 50.0);
                    show.rear_hand = p(-4.0, 54.0);
                    show.head = p(-3.0 + 5.0 * nod, HEAD_U + 3.0 - 4.0 * nod);
                    show.hip.f += 1.0;
                }
                Intro::Limber | Intro::Bow => {
                    // Shaking it out: up on the toes and down, the hands loose and flicking.
                    let hop = (f.t * 11.0).sin();
                    let flick = (f.t * 23.0).sin();
                    show.hip.u -= 4.0 * hop.max(0.0);
                    show.head.u -= 3.5 * hop.max(0.0);
                    show.head.f += 2.0 * (f.t * 5.0).sin();
                    show.lead_hand = p(12.0 + 6.0 * flick, 54.0 + 4.0 * flick);
                    show.rear_hand = p(22.0 - 6.0 * flick, 52.0 - 4.0 * flick);
                    show.lead_heel = 0.5 * (-hop).max(0.0);
                    show.rear_heel = 0.5 * (-hop).max(0.0);
                }
            }
            q = tall.to(show, on);
            q.open = false;
        }
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

    // Enraged (nearly out, and hitting harder for it): head down and forward, glaring over the fists,
    // which come up and in, and the shoulders heaving with each breath.
    if f.enraged() && f.health > 0 && matches!(f.act, Act::Idle | Act::Walk | Act::Block) && !f.airborne() && !low {
        let breath = (now * 3.6 + idx as f32).sin();
        // (Less of a lean on the move: walking, the crouching spider's weight went past his toes.)
        let glare = if f.act == Act::Walk { 1.0 } else { 5.0 };
        q.head = p(q.head.f + glare, q.head.u - 4.0 + 1.6 * breath);
        q.lead_hand = p(q.lead_hand.f + 3.0, q.lead_hand.u + 3.0 + 1.2 * breath);
        q.rear_hand = p(q.rear_hand.f + 2.0, q.rear_hand.u + 2.0 + 1.2 * breath);
        q.hip.u -= 2.0;
    }

    if !f.airborne() && f.act != Act::Knockdown {
        q.lead_foot.u = q.lead_foot.u.max(0.0);
        q.rear_foot.u = q.rear_foot.u.max(0.0);
    }
    if f.stumble > 0.0 && f.act != Act::Attack {
        let k = (1.0 - f.stumble / 0.34).clamp(0.0, 1.0);
        let sway = (std::f32::consts::PI * k).sin();
        q.hip.f -= 7.0 * sway;
        q.hip.u -= 3.5 * sway;
        q.head.f += 5.0 * sway;
        q.head.u += 2.5 * sway;
        q.lead_hand.f -= 5.0 * sway;
        q.lead_hand.u += 6.0 * sway;
        q.rear_hand.f += 5.0 * sway;
        q.lead_foot.f -= 9.0 * sway;
        q.rear_foot.f += 6.0 * sway;
    }
    // Staggering: rocked off balance. The body sways slowly back and forth
    // over the feet and bobs out of time with it, the guard sags, and the
    // feet shuffle to catch the weight; it all settles as the stagger ends.
    if f.stagger > 0.0 && !f.airborne()
        && matches!(f.act, Act::Idle | Act::Walk | Act::Crouch | Act::Block | Act::Attack) {
        let left = (f.stagger / 0.5).min(1.0);
        let k = if f.act == Act::Attack { 0.4 * left } else { left };
        let (sway, bob) = ((now * 5.2).sin(), (now * 8.3 + 1.0).sin());
        q.hip.f += 6.0 * sway * k;
        q.hip.u -= (3.0 + 2.0 * bob) * k;
        q.head.f += 13.0 * sway * k;
        q.head.u -= (4.0 + 3.0 * bob) * k;
        q.lead_hand.u -= 14.0 * k;
        q.lead_hand.f -= 4.0 * k;
        q.rear_hand.u -= 10.0 * k;
        q.rear_hand.f += 5.0 * sway * k;
        q.lead_foot.f += 5.0 * sway * k;
        q.rear_foot.f -= 4.0 * sway * k;
        q.lead_heel = q.lead_heel.max(0.4 * (sway.max(0.0)) * k);
    }
    // The other foot forward: a high kick can switch lead legs, and the
    // roundhouse comes off the back one. (A walk is already on both; the bow
    // and the end of the round stand square.)
    let switched = match f.mv {
        MoveId::HighKick => f.alt_kick,
        MoveId::Roundhouse => true,
        _ => false,
    };
    let far_leads = f.far_leads ^ (f.act == Act::Attack && switched && !f.airborne());
    if far_leads && !f.airborne()
        && matches!(f.act, Act::Idle | Act::Crouch | Act::Block | Act::Hitstun | Act::Attack) {
        std::mem::swap(&mut q.lead_foot, &mut q.rear_foot);
        std::mem::swap(&mut q.lead_heel, &mut q.rear_heel);
    }
    if f.act == Act::Air { q = spun(q, flip_of(f)); }
    q
}

/// How a fighter opens the round, before the guard comes up.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum Intro {
    /// The martial artists bow (rei).
    Bow,
    /// The heavyweights beat their chests.
    Pound,
    /// The officer and the boss stand with their fists on their hips and nod.
    Hips,
    /// Everyone else shakes it out on their toes.
    Limber,
}

pub(crate) fn intro_of(who: usize) -> Intro {
    match FIGHTERS[who].build {
        Build::Gi | Build::Qipao => Intro::Bow,
        Build::Bare | Build::Giant | Build::Gladiator => Intro::Pound,
        Build::Uniform | Build::Caped => Intro::Hips,
        _ => Intro::Limber,
    }
}

/// The acrobats: everyone but the chargers, the giant and the flier.
pub(crate) fn nimble(f: &Fighter) -> bool {
    let a = f.arch();
    !f.flies() && a.special != Special::BullRush && a.build != Build::Giant
}

/// How far round a jump has turned the fighter, in radians (back, below zero): the light ones somersault through it,
/// tucked, from just after take-off to just past the top; the heavies and a jump straight up do not.
/// Read off vertical speed, like the airborne pose, so a blend from it agrees.
pub(crate) fn flip_of(f: &Fighter) -> f32 {
    if !nimble(f) || f.vx == 0.0 || !f.airborne() { return 0.0; }
    // Forward, head over heels; away, a backflip.
    let way = (f.vx * f.facing).signum();
    let vy0 = JUMP_VY * f.arch().jump_scale;
    let k = ((f.vy - vy0) / (-2.0 * vy0)).clamp(0.0, 1.0);
    // Round and done by just past the top, where a jump kick is usually thrown: one thrown mid-turn
    // blends out of it.
    let k = ((k - 0.08) / 0.50).clamp(0.0, 1.0);
    way * std::f32::consts::TAU * k * k * (3.0 - 2.0 * k)
}

/// The whole pose turned `a` radians forward (head over heels) about the middle of the body.
pub(crate) fn spun(mut q: Pose, a: f32) -> Pose {
    if a == 0.0 { return q; }
    let c = q.hip.to(q.head, 0.35);
    let (s, co) = a.sin_cos();
    for j in [&mut q.hip, &mut q.head, &mut q.lead_hand, &mut q.rear_hand, &mut q.lead_foot, &mut q.rear_foot] {
        let (df, du) = (j.f - c.f, j.u - c.u);
        *j = p(c.f + df * co + du * s, c.u - df * s + du * co);
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
