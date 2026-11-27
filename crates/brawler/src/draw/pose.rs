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
#[allow(clippy::too_many_arguments)]
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
    /// The kung fu salute: the right fist pressed into the left palm before the chest, a small bow over it.
    Salute,
    /// The Thai wai: palms pressed together under the chin, the head bowed to them.
    Wai,
    /// A thumb flicked across the nose, twice, the head tipped back.
    Thumb,
    /// The far hand out palm up, curling: come back for more.
    Beckon,
    /// The karate bow, heels together and hands at the thighs, then the ready stance, fists before the belt.
    Rei,
    /// The pinched fingers held up before the lips, and a blow: the chest hair plucked and blown away (DRAKE over
    /// LANCER, Way of the Dragon).
    Pluck,
    /// The knee raised high and the foot stamped down, the face screwed up in that anguished cry (Enter the Dragon).
    Stamp,
}

/// The martial art a fighter moves in. Studied from open references and footage (see the notes beside
/// `ARTS`): it decides the stance's extra motion, how a punch is thrown and which kicks come out.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Art {
    /// Shotokan: a long front stance under every punch, the other fist pulled back to the hip (hikite).
    Karate,
    /// A wrestler's crouch: open hands low and forward.
    Wrestling,
    /// Up on the toes, hands low, kicks high with the body laid well back.
    Taekwondo,
    /// Swinging hooks, the shoulders rolling into them.
    Brawling,
    /// Narrow and upright, both hands on the centre line, vertical-fist chain punches.
    WingChun,
    /// The ginga: rocking a foot back and forward, the arms crossing in front of the face.
    Capoeira,
    /// Southern tiger-crane: a low horse stance, clawed hands, palm strikes.
    HungGar,
    /// Tall and square, a high guard, the lead foot light; the shin kick with the arm swung down.
    MuayThai,
    /// Chin down behind both fists, elbows in, slipping the head side to side.
    Boxing,
    /// Jeet kune do: strong side forward, bouncing on the balls of the feet, the lead hand low and loose;
    /// the straight lead, the stop kick at the knee, the lead side kick.
    JeetKuneDo,
    /// Everyone else keeps the style they had.
    Own,
}

/// One per fighter, in roster order.
pub(crate) const ARTS: [Art; FIGHTERS.len()] = {
    use Art::*;
    // RYUKA BRUTUS KESTREL GIOTTO TITIAN VERMEER BOSCH WEBBER GAMMA ZENITH JADE REGENT KITE AURELIA MACK DRAKE LANCER
    [Karate, Wrestling, Taekwondo, Karate, Brawling, WingChun, Capoeira, Own, Own, Own, HungGar, Own, MuayThai, Own,
     Boxing, JeetKuneDo, Karate]
};

/// One per fighter, in roster order.
pub(crate) const STYLES: [Style; FIGHTERS.len()] = [
    // RYUKA: the textbook karate stance.
    style(4.4, 1.0, 0.0, 0.0, 0.0, p(0.0, 0.0), p(0.0, 0.0), false, Win::Fist),
    // BRUTUS: a wrestler, wide and low, open hands ready to grab.
    style(3.0, 0.8, 3.0, 4.0, 2.0, p(-4.0, -8.0), p(-4.0, -13.0), true, Win::Flex),
    // KESTREL: taekwondo, light on the toes and side-on, the hands carried low.
    style(6.5, 1.6, 0.0, -3.0, -2.0, p(4.0, -10.0), p(-4.0, -13.0), false, Win::Hop),
    // GIOTTO: the disciplined one, rear fist by the chin.
    style(4.0, 0.9, 1.0, 0.0, 0.0, p(2.0, -2.0), p(3.0, -4.0), false, Win::Fist),
    // TITIAN: the hothead, hunched with the fists low.
    style(3.6, 1.0, 4.0, 3.0, 7.0, p(-6.0, -6.0), p(-2.0, -10.0), false, Win::Pump),
    // VERMEER: wing chun, narrow and upright; the far hand out on the centre line, the near one behind its elbow.
    style(3.4, 0.6, 2.0, -9.0, -4.0, p(10.0, 2.0), p(10.0, 4.0), true, Win::Salute),
    // BOSCH: capoeira, never still: the ginga's rocking step (see `stance`).
    style(3.0, 1.0, 7.0, 0.0, 3.0, p(-2.0, 6.0), p(2.0, 4.0), true, Win::Hop),
    // WEBBER: low to the floor, fingers spread.
    style(5.5, 1.2, 7.0, 8.0, 8.0, p(-2.0, -18.0), p(8.0, -32.0), true, Win::Hop),
    // GAMMA: hunched, the arms hanging heavy.
    style(2.6, 1.2, 4.0, 6.0, 9.0, p(-8.0, -14.0), p(-4.0, -20.0), false, Win::Flex),
    // ZENITH: upright and unbothered, the fists low.
    style(3.2, 0.7, 0.0, -3.0, -3.0, p(-8.0, -14.0), p(-5.0, -22.0), false, Win::Fist),
    // JADE: Hung Gar, sat low in a wide horse stance, the clawed hands out in front.
    style(2.6, 0.6, 6.0, 5.0, 0.0, p(6.0, 0.0), p(2.0, -4.0), true, Win::Salute),
    style(2.8, 0.5, 2.0, 5.0, 3.0, p(-5.0, -2.0), p(-4.0, -10.0), false, Win::Both),
    // KITE: Muay Thai, tall and square, the guard high by the brow, the lead foot light.
    style(3.6, 0.8, -1.0, -6.0, -3.0, p(6.0, 12.0), p(-2.0, 14.0), false, Win::Wai),
    style(2.7, 0.7, 4.0, 5.0, 1.0, p(-1.0, -3.0), p(-4.0, -9.0), true, Win::Flex),
    // MACK: a boxer, chin down behind both fists, elbows in; the head slips side to side (see `stance`).
    style(4.8, 1.1, 2.0, 0.0, 3.0, p(2.0, 6.0), p(-6.0, 8.0), false, Win::Pump),
    // DRAKE: the on-guard of jeet kune do, light and bouncing, the lead hand low, the rear by the chin.
    // (The lead hand at chest height, from Way of the Dragon's on-guard; it was carried low.)
    style(7.0, 1.4, 2.0, -2.0, 2.0, p(8.0, 4.0), p(-6.0, 12.0), false, Win::Thumb),
    // LANCER: the American karate champion's on-guard from Way of the Dragon: upright and side-on, light on his feet,
    // both fists carried high in front.
    // (Hips level with RYUKA's and feet 3 nearer: higher, the knees locked straight.)
    // (The lead arm reaching well out at shoulder height, the rear fist at the chest, as in the film's stills.)
    style(5.0, 1.2, 0.0, -3.0, -2.0, p(14.0, 8.0), p(0.0, 0.0), false, Win::Rei),
];

/// The stance: weight back, knees bent, both hands up, side on. Everything
/// departs from and returns to this. `bob` is the idle bounce, -1..1: the
/// whole body rides it, the hands a little further.
pub(crate) fn stance(bob: f32, who: usize) -> Pose {
    let s = &STYLES[who];
    let bob = bob * s.amp;
    let mut q = Pose {
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
    };
    match ARTS[who] {
        // The ginga: the back foot rocks back and forward under a body that sways with it, the arms crossing
        // in front of the face in turn.
        Art::Capoeira => {
            q.rear_foot.f -= 7.0 * bob;
            q.hip.f -= 4.0 * bob;
            q.head.f -= 5.0 * bob;
            q.rear_hand = p(q.rear_hand.f - 6.0 * bob, q.rear_hand.u + 8.0 * bob);
            q.lead_hand.u -= 6.0 * bob;
        }
        // In and out on the bounce: jeet kune do's footwork, the whole body rocking toward range and away.
        Art::JeetKuneDo => {
            // The lead hand never quite still: it paws, small and loose, on the bounce.
            q.rear_hand.f += 3.0 * bob * bob;
            q.rear_hand.u -= 2.0 * bob;
            q.hip.f += 2.5 * bob;
            q.head.f += 3.0 * bob;
            q.lead_foot.f += 2.0 * bob;
            q.rear_foot.f += 1.5 * bob;
            q.lead_hand.f += 2.5 * bob;
            q.rear_hand.f += 3.0 * bob;
        }
        // Slipping: the head dips and rolls under an imagined jab.
        Art::Boxing => {
            q.head.f += 3.0 * bob;
            q.head.u -= 2.0 * bob.abs();
        }
        // Weight on the back foot, the front one light and tapping.
        Art::MuayThai => q.lead_heel = 0.45 * bob.max(0.0),
        // Hand-fighting: the open hands paw forward in turn, feeling for a grip.
        Art::Wrestling => {
            q.lead_hand.f += 6.0 * bob.max(0.0);
            q.rear_hand.f += 6.0 * (-bob).max(0.0);
        }
        _ => {}
    }
    q
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
        // DRAKE's blows snap back home in a third of the recovery, as Bruce Lee's do on film; the rest take six tenths.
        let snap = if formless(f.who) { 0.35 } else { 0.6 };
        let back = (k / snap).min(1.0);
        1.0 - back * back * (3.0 - 2.0 * back)
    }
}

/// The intro this fighter is giving: their own, or for DRAKE the warm-up one round in two, and always against LANCER,
/// as both men limber up before the duel in Way of the Dragon.
pub(crate) fn intro_now(f: &Fighter) -> Intro {
    if formless(f.who) && (f.stumble_roll % 2 == 1 || FIGHTERS[f.foe].build == Build::Duelist) { Intro::WarmUp }
    else { intro_of(f.who) }
}

/// DRAKE winning over LANCER: the pluck, in place of the win his roll would pick.
pub(crate) fn plucking(f: &Fighter) -> bool {
    formless(f.who) && f.act == Act::Victory && FIGHTERS[f.foe].build == Build::Duelist
}

/// How far the nunchaku's sticks reach past the fist when they crack out in line: two sticks and the chain.
pub(crate) const NUNCHAKU: f32 = 30.0;

/// One trick of DRAKE's nunchaku: where the near fist is at the strike (forward and up of where it cracks out
/// level), and the sticks' sweep through the strike, in radians from level forward, up positive.
#[derive(Clone, Copy)]
struct Trick { df: f32, du: f32, from: f32, to: f32 }

const fn tr(df: f32, du: f32, from: f32, to: f32) -> Trick { Trick { df, du, from, to } }

/// Three routines, one picked per throw: Enter the Dragon's figure of eight, forehand and backhand at the head;
/// Fist of Fury's chops, high and low; Game of Death's pokes between whips over the shoulder. Each ends on a big
/// overhead smash, the strike that knocks down.
const ROUTINES: [[Trick; 6]; 3] = [
    [tr(0.0, 4.0, 1.0, -0.6), tr(-4.0, -6.0, -1.0, 0.6), tr(0.0, 4.0, 1.0, -0.6), tr(-4.0, -6.0, -1.0, 0.6),
     tr(0.0, 2.0, 1.1, -0.7), tr(-2.0, 10.0, 1.7, -0.4)],
    [tr(-2.0, 8.0, 1.3, -0.5), tr(-4.0, -10.0, -1.3, 0.3), tr(-2.0, 8.0, 1.3, -0.5), tr(0.0, -14.0, 0.4, -1.1),
     tr(-4.0, -8.0, -1.2, 0.5), tr(-2.0, 10.0, 2.0, -0.6)],
    [tr(4.0, 0.0, 0.08, -0.08), tr(-6.0, 8.0, 1.4, -0.4), tr(4.0, -4.0, 0.08, -0.08), tr(-6.0, 8.0, 1.4, -0.4),
     tr(4.0, 2.0, 0.08, -0.08), tr(-2.0, 10.0, 1.8, -0.5)],
];

/// Where DRAKE's nunchaku routine has the near fist (forward and up of the level strike) and the sticks (radians
/// up from level forward), in the middle of the strikes; `None` in the whirl and the recovery. Through a strike's
/// window the sticks sweep through it; between strikes they twirl on round, half a turn or more, to the next.
pub(crate) fn stick_trick(f: &Fighter) -> Option<(f32, f32, f32)> {
    let (k, _) = f.stick_strike()?;
    let r = &ROUTINES[(f.stumble_roll / 3 % 3) as usize];
    let at = f.t / F - STICK_WHIRL - k as f32 * STICK_EVERY;
    let a = r[k as usize];
    let sweep = STICK_HIT + 1.0;
    if at < sweep {
        let s = at / sweep;
        return Some((a.df, a.du, a.from + (a.to - a.from) * s * s * (3.0 - 2.0 * s)));
    }
    let Some(b) = r.get(k as usize + 1) else { return Some((a.df, a.du, a.to)) };
    let s = ((at - sweep) / (STICK_EVERY - sweep)).clamp(0.0, 1.0);
    let sense = if a.to < a.from { -1.0 } else { 1.0 };
    let mut d = b.from - a.to;
    while d * sense < std::f32::consts::PI { d += sense * std::f32::consts::TAU; }
    let e = s * s * (3.0 - 2.0 * s);
    Some((a.df + (b.df - a.df) * e, a.du + (b.du - a.du) * e, a.to + d * s))
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
    air_lean: 0.12,
    plant: 5.0,
    knee_lead: 16.0,
    chamber: 31.0,
    chamber_air: 25.0,
};

/// How a fighter kicks high (the roundhouse button is a roundhouse for all but the stompers; the low kick a snap).
/// Studied from SF2 footage: a roundhouse turns the hip over and swings the instep round in an arc, the body
/// tipping away; a side kick is cocked at the chest and driven out heel first; a crescent swings a straight leg
/// up and over; a heavy's push kick leans into it.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum KickStyle {
    Snap, Round, Side, Crescent, Push,
    /// Taekwondo's axe kick: a straight leg raised past the head and brought down heel first.
    Axe,
    /// Muay Thai's round kick: no chamber, the leg swung up from the floor like a bat and landing with the shin,
    /// the hip turned right over and the kicking-side arm thrown down to drive it.
    Thai,
    /// The spinning back kick: through the chamber the body turns its back on the opponent (the pose mirrored,
    /// `spin_turn`) and comes round to drive the heel out like a side kick.
    Spin,
}

/// Each fighter's own high kick, in roster order.
pub(crate) const KICKS: [KickStyle; FIGHTERS.len()] = {
    use KickStyle::*;
    // RYUKA BRUTUS KESTREL GIOTTO TITIAN VERMEER BOSCH WEBBER GAMMA ZENITH JADE REGENT KITE AURELIA MACK
    // (BOSCH's is capoeira's martelo, a round kick.)
    // (LANCER's is the round kick; his high kick is as often the spinning heel kick, see `kick_style`.)
    [Round, Push, Axe, Side, Push, Side, Round, Side, Push, Side, Crescent, Side, Thai, Push, Snap, Side, Round]
};

/// Jeet kune do has no fixed technique: which one this attack is comes from the fighter's own per-attack
/// roll (`stumble_roll`, a 16-step cycle that every attack advances), so DRAKE never shows the same answer twice
/// in a row and a run still plays the same every time.
pub(crate) fn jkd_pick(f: &Fighter, n: usize) -> usize {
    (f.stumble_roll as usize + f.mv as usize * 7) % n
}

/// The kick `f` is throwing.
pub(crate) fn kick_style(f: &Fighter) -> KickStyle {
    let own = KICKS[f.who];
    // LANCER's special is the spinning heel kick, and every other high kick of his is too.
    if f.arch().build == Build::Duelist {
        match f.mv {
            MoveId::Special => return KickStyle::Spin,
            MoveId::HighKick if f.stumble_roll % 2 == 1 => return KickStyle::Spin,
            _ => {}
        }
    }
    if ARTS[f.who] == Art::JeetKuneDo {
        use KickStyle::*;
        return match f.mv {
            // The stop kick at the knee, a snap to the shin, a push to the belly.
            MoveId::LowKick => [Side, Snap, Push][jkd_pick(f, 3)],
            // The lead side kick, the hook kick, a front snap to the chin, an axe. (A crescent swung through
            // his standing foot from this stance.)
            // (And the spinning back kick.)
            MoveId::HighKick => [Side, Round, Snap, Axe, Spin][jkd_pick(f, 5)],
            MoveId::Roundhouse => [Round, Crescent, Side][jkd_pick(f, 3)],
            _ => own,
        };
    }
    match f.mv {
        // The teep: Muay Thai's low kick is a push with the ball of the foot.
        MoveId::LowKick if own == KickStyle::Push || own == KickStyle::Thai => KickStyle::Push,
        // The low kick by art: taekwondo whips a low round kick, wing chun stamps a side kick at the knee,
        // Hung Gar and capoeira push with the heel.
        MoveId::LowKick => match ARTS[f.who] {
            Art::Taekwondo => KickStyle::Round,
            Art::WingChun | Art::JeetKuneDo => KickStyle::Side,
            Art::HungGar | Art::Capoeira => KickStyle::Push,
            _ => KickStyle::Snap,
        },
        MoveId::Roundhouse if own == KickStyle::Push || own == KickStyle::Thai => own,
        MoveId::Roundhouse => KickStyle::Round,
        _ => own,
    }
}

/// How a fighter punches: a karate straight, a boxer's jab that slips the head off the line, an open palm, an
/// overhand hammer from above the head, a backfist flicked out from the chest.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum PunchStyle { Straight, Boxer, Palm, Hammer, Backfist }

/// Each fighter's own punch, in roster order.
pub(crate) const PUNCHES: [PunchStyle; FIGHTERS.len()] = {
    use PunchStyle::*;
    // RYUKA BRUTUS KESTREL GIOTTO TITIAN VERMEER BOSCH WEBBER GAMMA ZENITH JADE REGENT KITE AURELIA MACK
    [Straight, Hammer, Boxer, Straight, Hammer, Palm, Boxer, Backfist, Hammer, Straight, Palm, Straight, Backfist,
     // (LANCER's is the backfist, his own hand technique; the straight was RYUKA's.)
     Hammer, Boxer, Straight, Backfist]
};

/// The punch `f` is throwing: the fighter's own, or for DRAKE whichever of the straight lead, the backfist and
/// the finger jab (an open hand) this one is. (His fourth is the hook, see `jkd_hook`.)
pub(crate) fn punch_style(f: &Fighter) -> PunchStyle {
    if ARTS[f.who] == Art::JeetKuneDo {
        if f.mv == MoveId::Special { return PunchStyle::Straight; }
        return [PunchStyle::Straight, PunchStyle::Backfist, PunchStyle::Palm, PunchStyle::Straight][jkd_pick(f, 4)];
    }
    PUNCHES[f.who]
}

/// DRAKE's fourth punch: a lead hook.
pub(crate) fn jkd_hook(f: &Fighter) -> bool {
    ARTS[f.who] == Art::JeetKuneDo && f.mv != MoveId::Special && jkd_pick(f, 4) == 3
}

/// A point on the curve from `a` to `c` pulled toward `b`.
fn bend(a: P, b: P, c: P, t: f32) -> P { a.to(b, t).to(b.to(c, t), t) }

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

    enum Shape { Punch(bool), Kick(bool, f32), Upper, Fly, Sweep, Throw, Bolt, Laser, Rush, Call, Sticks }
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
        MoveId::Special => match f.special() {
            Special::ChiBolt => Shape::Bolt,
            Special::LaserVision => Shape::Laser,
            Special::BullRush => Shape::Rush,
            Special::TalonKick if f.arch().build == Build::Commando => Shape::Fly,
            // DRAKE's is the classic flying side kick: leaned back along the straight leg, the other knee tucked.
            Special::TalonKick if f.arch().build == Build::Track => Shape::Kick(true, 1.0),
            Special::Nunchaku => Shape::Sticks,
            Special::OneInch => Shape::Punch(false),
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
        Shape::Punch(false) => punch_pose(q, f, tip, ext),
        Shape::Kick(air, lift) => kick_pose(q, f, toe_tip, ext, air, lift),
        // The nunchaku: through the startup the near hand whirls the sticks round the shoulder; then the arm
        // whips out level and the sticks crack out in line beyond the fist (drawn in `draw_sticks`).
        Shape::Sticks => {
            let e = ext.max(0.0);
            let sh = p(q.head.f + 2.0, q.head.u - 20.0);
            let whirl = f.t < m.startup * F;
            let trick = stick_trick(f);
            q.lead_hand = if whirl {
                let a = f.t * 26.0;
                // (Forward of the face and low enough to clear it at the top of the circle.)
                p(sh.f + 20.0 + 6.0 * a.cos(), sh.u - 2.0 + 5.0 * (2.0 * a).sin())
            } else if let Some((df, du, _)) = trick {
                p(tip.f - NUNCHAKU + df, tip.u + du)
            } else {
                p(sh.f + 20.0, sh.u - 2.0).to(p(tip.f - NUNCHAKU, tip.u), e)
            };
            q.rear_hand = p(q.head.f + 14.0, q.head.u - 30.0);
            // Stepping in on every strike: the front foot reaches and lands as the sticks crack, the back foot
            // follows it up in the gap.
            if let Some((k, _)) = f.stick_strike() {
                let at = (f.t / F - STICK_WHIRL - k as f32 * STICK_EVERY) / STICK_EVERY;
                q.lead_foot.u += 5.0 * (at * std::f32::consts::PI * 2.0).sin().max(0.0) * (1.0 - at);
                q.rear_foot.f += 6.0 * (at * std::f32::consts::PI).sin();
                // The guard hand swaps from cheek to chest from strike to strike.
                if k % 2 == 1 { q.rear_hand = p(q.head.f + 10.0, q.head.u - 44.0); }
            }
            q.head.f += 5.0 * e;
            q.hip.f += 3.0 * e;
            q.lead_foot.f += 8.0 * e;
            q.twist = e;
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
            // The shoulders ride most of the hips' drive (0.7): left behind, a long reach (DRAKE's drives the hips 28
            // forward) laid the sweeper on his back.
            q.head = p(q.head.f + (0.7 * hip_f - 4.0) * ext, C_HEAD - 7.0 * ext);
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
                // Up out of a low stance first: from a horse stance the arms cannot reach the top of the hurl.
                let (hip_u, head_u) = (q.hip.u.max(HIP_U - 4.0), q.head.u.max(HEAD_U - 4.0));
                q.hip = p(4.0 + 3.0 * hurl, hip_u - 8.0 * dip + 3.0 * hurl);
                q.head = p(q.head.f + 8.0 - 6.0 * hurl, head_u - 6.0 * dip + 4.0 * hurl);
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

/// A punch thrown from the ground: the fist driven out to `tip` as the move extends (`ext`), the body stepping in behind it.
fn punch_pose(q: &mut Pose, f: &Fighter, tip: P, ext: f32) {
    let e = ext.max(0.0);
    let inch = f.mv == MoveId::Special;
    // Step in and sit into it; a low one is reached by sinking at the
    // knees, not by folding over.
    let low = ((60.0 - tip.u) / 30.0).clamp(0.0, 1.0) * e;
    let art = ARTS[f.who];
    // Karate lunges into a long front stance, the back leg driven straight; Hung Gar drops into its horse.
    let (step, brace, sit) = match art {
        Art::Karate => (14.0, 9.0, 4.0),
        Art::HungGar => (6.0, 4.0, 4.0),
        Art::WingChun => (6.0, 1.0, 0.0),
        _ => (8.0, 3.0, 0.0),
    };
    q.hip = p(q.hip.f + 4.0 * ext, q.hip.u - 2.0 * e - 9.0 * low - sit * e);
    q.head.u -= 1.5 * e + 10.0 * low + sit * e;
    q.lead_foot.f += step * ext;
    q.rear_foot.f -= brace * ext;
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
        // The one-inch punch starts with the fist already an inch off the mark.
        let start = if inch { p(tip.f - 9.0, tip.u) } else { q.lead_hand };
        q.lead_hand = match punch_style(f) {
            // Up over the head and brought down onto the mark, the body bowing into it.
            PunchStyle::Hammer if f.mv != MoveId::Hook => {
                q.head.f += 4.0 * e;
                bend(start, p(start.f + 2.0, (tip.u + 30.0).max(start.u + 14.0)), tip, ext.max(0.0))
            }
            // Flicked out from the chest in a little arc, knuckles first.
            PunchStyle::Backfist if f.mv != MoveId::Hook =>
                bend(start, p(tip.f - 16.0, tip.u + 9.0), tip, ext.max(0.0)),
            _ => start.to(tip, ext),
        };
        match punch_style(f) {
            // The head slips down and off the line behind the jab.
            PunchStyle::Boxer => { q.head.u -= 5.0 * e; q.head.f -= 2.0 * e; }
            PunchStyle::Palm => q.open = e > 0.4,
            _ => {}
        }
        // The shoulder comes through with the hip.
        q.twist = e;
    }
    // What the other hand does is most of what tells one art from another.
    let (near, far) = if f.alt_punch { (false, true) } else { (true, false) };
    let guard = |q: &mut Pose, to: P, open: bool| {
        if near { q.rear_hand = q.rear_hand.to(to, e.sqrt()); } else if far { q.lead_hand = q.lead_hand.to(to, e.sqrt()); }
        if open { q.open = e > 0.4; }
    };
    match art {
        // Hikite: the other fist pulled back to the hip, knuckles down, as the punch goes out.
        Art::Karate => guard(q, p(q.hip.f + 7.0, q.hip.u + 15.0), false),
        // The other hand stays out on the centre line behind the punch, open (wu sau); the body stays square.
        Art::WingChun => {
            guard(q, p(tip.f - 24.0, tip.u - 3.0), true);
            if far { q.twist = q.twist.min(0.35); }
        }
        // A palm strike, the other hand clawed back to the chest.
        Art::HungGar => guard(q, p(q.head.f + 10.0, q.head.u - 30.0), true),
        // The guard never drops: the other glove stays at the brow.
        Art::MuayThai => guard(q, p(q.head.f + 9.0, q.head.u - 12.0), false),
        // The straight lead: thrown from where the hand hangs, the rear hand kept at the chin.
        Art::JeetKuneDo => guard(q, p(q.head.f + 8.0, q.head.u - 20.0), false),
        _ => {}
    }
    if f.mv == MoveId::Hook || ((art == Art::Brawling || jkd_hook(f)) && !f.alt_punch) {
        // A hook comes round, not straight: the fist swings out and
        // up on its way in, and the body turns hard behind it.
        let swing = (std::f32::consts::PI * e).sin();
        q.lead_hand.u += 10.0 * swing;
        q.lead_hand.f -= 6.0 * swing;
        q.hip.f += 3.0 * e;
        q.twist = 1.0f32.min(q.twist + 0.4 * e);
    }
    // The one-inch punch: coiled low through the startup, then legs, hips and shoulder fire in one chain
    // (the knees snapping straight first) and only the fist's last inch shows.
    if inch {
        // The other hand stays home at the chest throughout, clear of the one an inch off the mark.
        q.rear_hand = p(q.head.f + 14.0, q.head.u - 30.0);
        let coil = 1.0 - e;
        q.hip.u -= 6.0 * coil;
        q.head.u -= 5.0 * coil;
        q.head.f += 6.0 * e - 3.0 * coil;
        q.hip.f += 4.0 * e;
        q.twist = e;
    }
    // The back foot turns over onto its ball as the hip comes through.
    q.rear_heel += 0.5 * e;
}

/// A kick, from the ground or in the air (`air`), at the height `lift` sets: the foot driven out to `toe_tip` as the move
/// extends (`ext`).
fn kick_pose(q: &mut Pose, f: &Fighter, toe_tip: P, ext: f32, air: bool, lift: f32) {
    let k = KICK;
    let e = ext.max(0.0);
    // 0 for the low kick, 1 for the high one.
    let high = ((lift - 0.7) / 0.55).clamp(0.0, 1.0);
    // The higher the kick the further the shoulders go back to pay
    // for it, and the more the standing leg straightens under it.
    // In the air the side-kickers (and taekwondo) throw the flying side kick; everyone else a snap.
    let style = match (air, KICKS[f.who]) {
        (true, KickStyle::Side | KickStyle::Axe) => KickStyle::Side,
        // LANCER's flying kick is a side kick too, the knee tucked: as a snap it sat on a chair in the air.
        (true, _) if f.arch().build == Build::Duelist => KickStyle::Side,
        (true, _) => KickStyle::Snap,
        _ => kick_style(f),
    };
    let back = if air { k.air_lean * lift } else { 0.35 + 0.90 * high };
    // (Halved from 0.8-1.6: laid back in one line with the leg, the body read as an arabesque. SF2's
    // kickers stay over the standing foot and let the leg do the work.)
    // (DRAKE's side kick lies back the furthest of anyone's, as Bruce Lee's does on film: 1.15 to 0.85.)
    // In the air his flying side kick sits more upright than anyone's (0.6): leaned back as far, it read as
    // sitting on a chair.
    let side_lean = match (formless(f.who), air) { (true, false) => 1.15, (true, true) => 0.6, _ => 0.85 };
    let back = back * match style {
        KickStyle::Snap => 0.35, KickStyle::Round => 0.7, KickStyle::Side => side_lean,
        KickStyle::Crescent => 0.5, KickStyle::Push => 0.25,
        // Taekwondo and Muay Thai both lie back from a high kick, well past the others.
        KickStyle::Axe => 1.0, KickStyle::Thai => 0.95, KickStyle::Spin => 0.7,
    };
    let drive = if air { k.hip_drive + 9.0 } else { k.hip_drive };
    // The standing knee stays bent under a grounded kick: rising onto a straight leg is ballet.
    let rise = if air { k.hip_rise } else { k.hip_rise * 0.7 };
    q.hip = p(q.hip.f + drive * ext, q.hip.u + rise * ext);
    // The shoulders ride forward with most of the hip's drive: left behind, the torso reclined into the
    // kick like a chair (SF2's kickers stay stacked over the hips).
    q.head.f += drive * ext * 0.7 - k.lean * 1.25 * ext * back;
    q.head.u -= 1.4 * ext;
    if !air {
        q.rear_foot = p(-18.0 + (q.hip.f - k.plant - -18.0) * ext, 0.0);
    } else {
        // The other leg tucked up tight under the seat, as SF2's jump kicks are drawn; in a flying side kick
        // the knee comes up to the chest instead, the foot under the hip.
        q.rear_foot = if style == KickStyle::Side {
            // (10 forward and 22 down: nearer, the anatomy test calls the leg folded to guesswork.)
            p(q.hip.f + 10.0, (q.hip.u - 22.0).max(30.0))
        } else {
            p(q.hip.f - 4.0, (q.hip.u - 22.0).max(30.0))
        };
    }
    // The standing foot stays flat on the boards (a roundhouse pivots on its ball). The kicking foot hits
    // with the heel or the ball, toes pulled back, never pointed like a dancer's.
    // (A Thai kick turns right over on the ball of the standing foot.)
    if !air { q.rear_heel = match style { KickStyle::Round => 0.15 + 0.2 * e, KickStyle::Thai => 0.15 + 0.4 * e, _ => 0.05 }; }
    q.lead_heel = match style {
        _ if air => -0.35 * e,
        KickStyle::Round => -0.1 * e,
        KickStyle::Side | KickStyle::Push | KickStyle::Spin => -0.6 * e,
        KickStyle::Snap => -0.5 * e,
        KickStyle::Crescent => -0.25 * e,
        KickStyle::Axe => -0.55 * e,
        KickStyle::Thai => 0.1 * e,
    };
    let chamber = p(q.hip.f + k.knee_lead,
        if air { k.chamber_air } else { k.chamber * (0.72 + 0.28 * high) });
    let kc = kick_curve(ext);
    let start = q.lead_foot;
    q.lead_foot = match style {
        // Knee across and the foot cocked behind it, then swung round in an arc onto the mark.
        KickStyle::Round => {
            let cock = p(q.hip.f + 8.0, chamber.u - 4.0);
            if kc < 1.0 { start.to(cock, kc) }
            else { bend(cock, p(toe_tip.f - 8.0, toe_tip.u + 22.0), toe_tip, kc - 1.0) }
        }
        // Knee to the chest, foot under the seat, then driven straight out.
        KickStyle::Side | KickStyle::Spin => {
            let cock = p(q.hip.f + 18.0, chamber.u);
            if kc < 1.0 { start.to(cock, kc) } else { cock.to(toe_tip, kc - 1.0) }
        }
        // A straight leg swung up past the mark and brought down onto it.
        KickStyle::Crescent => {
            let top = p(toe_tip.f - 3.0, toe_tip.u + 18.0);
            if kc < 1.0 { bend(start, p(start.f + 20.0, 28.0), top, kc) } else { top.to(toe_tip, kc - 1.0) }
        }
        // Up past the mark with the leg straight, then down onto it heel first.
        KickStyle::Axe => {
            let top = p(toe_tip.f - 4.0, toe_tip.u + 16.0);
            if kc < 1.0 { bend(start, p(start.f + 12.0, 40.0), top, kc) } else { top.to(toe_tip, kc - 1.0) }
        }
        // Swung from the floor in one arc, no knee first.
        KickStyle::Thai => {
            let late = p(toe_tip.f - 14.0, toe_tip.u - 8.0);
            if kc < 1.0 { bend(start, p(start.f + 4.0, 22.0), late, kc) } else { late.to(toe_tip, kc - 1.0) }
        }
        _ => if kc < 1.0 { start.to(chamber, kc) } else { chamber.to(toe_tip, kc - 1.0) },
    };
    // At the mark the leg is straight, as it is in every SF2 kick: the body rocks back from the foot until
    // hip to ankle is a whole leg, rather than the knee buckling up under a target too close.
    if !air && kc > 1.0 {
        let leg = THIGH + SHIN - 3.0;
        let ankle_u = q.lead_foot.u + ANKLE;
        let rise = (ankle_u - q.hip.u).abs().min(leg);
        let reach = q.lead_foot.f - FOOT * 0.4 - (leg * leg - rise * rise).sqrt();
        // (A push kick is the heavies' at short reach: they rock back further, 24 px, or it lands bent.)
        let most = if style == KickStyle::Push { 24.0 } else { 14.0 };
        let back = ((q.hip.f - reach).max(0.0) * (kc - 1.0)).min(most);
        q.hip.f -= back;
        q.head.f -= back;
    }
    // A flying side kick lays the body back along the leg, the kick and the spine one line.
    if air && style == KickStyle::Side {
        q.head.f -= 12.0 * e;
        q.head.u -= 5.0 * e;
    }
    if air && f.stomp {
        // The rage jump: both fists over the head on the way down.
        q.lead_hand = q.lead_hand.to(p(20.0, 128.0), ext);
        q.rear_hand = q.rear_hand.to(p(-12.0, 124.0), ext);
    } else {
        // The far fist stays out as the guard; the near one tucks in
        // at the ribs, in front of the body. Each kick carries its arms its own way.
        // The far fist guards the chin in every one: arms out for balance looked like a dancer's.
        let chin = p(q.hip.f + 20.0, q.hip.u + 40.0);
        let (rear, lead) = match style {
            // The near arm driven down and back against the turn of the hip.
            KickStyle::Round => (chin, p(q.hip.f + 9.0, q.hip.u + 16.0)),
            // The near fist down along the thigh, behind the kick.
            KickStyle::Side | KickStyle::Spin => (chin, p(q.hip.f + 16.0, q.hip.u + 6.0)),
            // The near fist tucked at the ribs.
            KickStyle::Crescent => (chin, p(q.hip.f + 10.0, q.hip.u + 26.0)),
            // Both fists forward, pushing in behind the foot.
            KickStyle::Push => (p(q.hip.f + 20.0, q.hip.u + 34.0), p(q.hip.f + 14.0, q.hip.u + 20.0)),
            KickStyle::Axe => (chin, p(q.hip.f + 10.0, q.hip.u + 24.0)),
            // The far glove out at the face as a long guard, the near arm flung down past the hip.
            KickStyle::Thai => (p(q.hip.f + 28.0, q.hip.u + 44.0), p(q.hip.f + 4.0, q.hip.u - 2.0)),
            KickStyle::Snap if air => (p(34.0, 70.0), p(q.hip.f + 10.0, q.hip.u - 6.0)),
            KickStyle::Snap => (p(34.0, 70.0), p(q.hip.f + 12.0, q.hip.u + 12.0)),
        };
        q.rear_hand = q.rear_hand.to(rear, ext);
        q.lead_hand = q.lead_hand.to(lead, ext);
        // A roundhouse brings the shoulders round with the hip; the standing heel pivots right up.
        if matches!(style, KickStyle::Round | KickStyle::Thai) { q.twist = e; }
        if style == KickStyle::Push { q.hip.f += 4.0 * ext; }
    }
    // LANCER's spinning heel kick is jumped: off the floor as the heel goes out (12 px at full extension).
    if f.mv == MoveId::Special && f.arch().build == Build::Duelist {
        let hop = 12.0 * e;
        for j in [&mut q.hip, &mut q.head, &mut q.lead_hand, &mut q.rear_hand, &mut q.lead_foot, &mut q.rear_foot] {
            j.u += hop;
        }
    }
}

/// The pose a fighter is in this frame.
/// The pose a fighter is drawn in, including the tail of the one before it: actions change on
/// one frame, so the hand-over is eased across four frames (the hitbox still follows the move).
pub(crate) fn pose_of(now: f32, f: &Fighter, idx: usize) -> Pose {
    let q = blended_pose(now, f, idx);
    // The spinning back kick: the pose turned to its mirror and back through the chamber.
    let spin = spin_turn(f);
    let q = if spin > 0.0 {
        let mut back = q;
        for j in [&mut back.hip, &mut back.head, &mut back.lead_hand, &mut back.rear_hand, &mut back.lead_foot, &mut back.rear_foot] {
            j.f = -j.f;
        }
        q.to(back, spin)
    } else { q };
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

/// How far round a spinning back kick has turned the fighter: 0 facing the opponent, 1 with the back to them
/// at the middle of the chamber, eased in and out of it.
pub(crate) fn spin_turn(f: &Fighter) -> f32 {
    if f.act != Act::Attack || f.airborne() { return 0.0; }
    // DRAKE's sweep is as often the spinning one, the heel coming round low.
    let spinning_sweep = f.mv == MoveId::Sweep && formless(f.who) && jkd_pick(f, 2) == 1;
    if !spinning_sweep && (!matches!(f.mv, MoveId::HighKick | MoveId::Special) || kick_style(f) != KickStyle::Spin) { return 0.0; }
    let m = f.scaled(move_data(f.mv));
    if spinning_sweep {
        if f.t >= m.startup * F { return 0.0; }
        return (std::f32::consts::PI * extension(f, &m).clamp(0.0, 1.0)).sin();
    }
    // Only on the way out: the leg comes back without turning again.
    if f.t >= m.startup * F { return 0.0; }
    let kc = kick_curve(extension(f, &m).max(0.0));
    if kc >= 1.0 { return 0.0; }
    (std::f32::consts::PI * kc).sin()
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
    if GAITS[f.who] == Gait::Shuffle { return f.far_leads; }
    let ph = walk_phase(f);
    let (near, _) = foot_cycle(ph, 9.0);
    let (far, _) = foot_cycle(ph + std::f32::consts::PI, 8.0);
    far > near
}

/// How a fighter walks.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Gait {
    /// Step through, the feet passing under the body.
    Stride,
    /// The boxer's: short slides in the stance, the front foot always in front.
    Shuffle,
    /// The heavyweight's: long, slow, knees up, the arms swinging wide.
    Stomp,
    /// Low to the floor, head first, hands near the boards.
    Prowl,
    /// Hardly a step at all: the feet skim.
    Glide,
}

impl Gait {
    /// How far one foot travels, planted, in a step: it slides back exactly as far as the body goes, so it stays
    /// put on the boards, and the cadence follows from it.
    pub(crate) fn step(self) -> f32 {
        match self {
            Gait::Shuffle => 9.0,
            Gait::Stomp => 22.0,
            Gait::Prowl => 14.0,
            Gait::Stride | Gait::Glide => STEP,
        }
    }
}

/// One per fighter, in roster order.
pub(crate) const GAITS: [Gait; FIGHTERS.len()] = {
    use Gait::*;
    // RYUKA BRUTUS KESTREL GIOTTO TITIAN VERMEER BOSCH WEBBER GAMMA ZENITH JADE REGENT KITE AURELIA MACK
    // (VERMEER's is wing chun's shuffle, the front foot always leading.)
    [Stride, Stomp, Shuffle, Stride, Stride, Shuffle, Stride, Prowl, Stomp, Glide, Stride, Stride, Shuffle, Stomp,
     Shuffle, Shuffle, Shuffle]
};

/// Where `f` is in its walk cycle, in radians: two feet down per turn.
pub(crate) fn walk_phase(f: &Fighter) -> f32 {
    f.x * f.facing * STRIDE / f.size() * STEP / GAITS[f.who].step()
}

/// One foot's (forward, lift) through the stride of `gait` (`foot_cycle`, its step scaled).
pub(crate) fn gait_foot(gait: Gait, ph: f32, lift: f32) -> (f32, f32) {
    let (fwd, up) = foot_cycle(ph, lift);
    (fwd * gait.step() / STEP, up)
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

/// How long a knocked-down body is in the air before it lands: 0.34 s, the arc SF2's take (0.22 dropped it
/// like a stone).
pub(crate) const FALL: f32 = 0.34;

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

    if let Some(t) = down_time(f) { return down_pose(f, t); }

    let low = f.crouching()
        || (f.act == Act::Hitstun && matches!(f.prev_act, Act::Crouch))
        || (f.act == Act::Hitstun && f.prev_act == Act::Attack
            && matches!(f.prev_mv, MoveId::Sweep))
        || (f.act == Act::Hitstun && f.prev_act == Act::Block && f.crouch_block);
    let mut q = if f.soaring() {
        flying_pose(f.soar)
    } else if f.airborne() {
        airborne_pose(f.vy)
    } else if low && ARTS[f.who] == Art::JeetKuneDo {
        // Low, he keeps his guard: the lead hand out low, the rear by the chin.
        let mut c = crouched(bob);
        c.rear_hand = p(30.0, 46.0 + bob * 0.8);
        c.lead_hand = p(14.0, 48.0 + bob * 0.6);
        c
    } else if low {
        crouched(bob)
    } else {
        stance(bob, f.who)
    };

    q = match f.act {
        Act::Walk => walk_pose(q, f),
        Act::Block => block_pose(q, f, bob),
        Act::Hitstun => hitstun_pose(q, f, low, now),
        Act::Attack => {
            let m = f.scaled(move_data(f.mv));
            let ext = extension(f, &m);
            attack_pose(&mut q, f, &m, ext);
            q
        }
        Act::Victory => victory_pose(q, f),
        Act::Bow if intro_of(f.who) != Intro::Bow && !f.bows => intro_pose(q, f),
        Act::Bow => bow_pose(f),
        Act::Defeat => defeat_pose(q, f),
        _ => q,
    };
    finishing_touches(q, f, now, idx, low)
}

/// Knocked down: the fall (swept, thrown or lifted off the boards and laid out), the bounce off the boards, and back up
/// (the acrobats kip up), `t` seconds in.
fn down_pose(f: &Fighter, t: f32) -> Pose {
    let down = floored();
    if t < FALL {
        let k = (t / FALL).clamp(0.0, 1.0);
        let mut air = stance(0.0, f.who);
        air.hip = p(-6.0, 48.0);
        air.head = p(-40.0, 62.0);
        air.lead_foot = p(30.0, 44.0);
        air.rear_foot = p(22.0, 24.0);
        air.lead_hand = p(-8.0, 82.0);
        air.rear_hand = p(-24.0, 74.0);
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
        // Lifted off the boards and laid out flat in the air before it comes down, as SF2's knockdowns are
        // (an uppercut throws it higher).
        let mut q = air.to(down, k * k);
        {
            // Eased in over the first third, or the elbows flip in the frame the blow lands.
            let lift = if f.downed_by == MoveId::Uppercut { 34.0 } else { 22.0 } * (std::f32::consts::PI * k).sin()
                * (k * 3.0).min(1.0);
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
    let b = if (FALL..FALL + 0.18).contains(&t) { 9.0 * (std::f32::consts::PI * (t - FALL) / 0.18).sin() }
        else if (0.40..0.50).contains(&t) { 2.5 * (std::f32::consts::PI * (t - 0.40) / 0.10).sin() } else { 0.0 };
    for j in [&mut down.hip, &mut down.head, &mut down.lead_hand, &mut down.rear_hand, &mut down.lead_foot, &mut down.rear_foot] {
        j.u += b;
    }
    down
}

/// Walking: the stride in the fighter's own gait.
fn walk_pose(mut q: Pose, f: &Fighter) -> Pose {
    let gait = GAITS[f.who];
    let ph = walk_phase(f);
    let s = ph.sin();
    let (lift, from_lead, from_rear) = match gait {
        // Boxers keep the stance and slide it: front foot first, the back one after.
        Gait::Shuffle => (4.0, q.lead_foot.f, q.rear_foot.f),
        Gait::Stomp => (15.0, WALK_MID + 2.0, WALK_MID - 2.0),
        Gait::Prowl => (9.0, WALK_MID + 3.0, WALK_MID + 3.0),
        Gait::Glide => (8.0, WALK_MID + 1.0, WALK_MID - 1.0),
        Gait::Stride => (9.0, WALK_MID, WALK_MID),
    };
    let (lf, ll) = gait_foot(gait, ph, lift);
    let (rf, rl) = gait_foot(gait, ph + std::f32::consts::PI, lift * 0.9);
    // A stride is a real walk: the feet pass each other under the body, so the front leg changes with
    // every step. A shuffle never crosses them.
    q.lead_foot = p(from_lead + lf, ll);
    q.rear_foot = p(from_rear + rf, rl);
    let toes = 1.0 + 0.4 * (1.0 - (f.arch().bulk - 0.85).clamp(0.0, 0.4) / 0.4);
    q.lead_heel = heel_cycle(ph) * toes;
    q.rear_heel = heel_cycle(ph + std::f32::consts::PI) * toes;
    if gait == Gait::Shuffle { q.rear_heel = q.rear_heel.max(0.45); }
    // Up out of the stance to walk: the legs are under the body now,
    // and nearly straight as each foot passes.
    // (A fighter who sits low in the stance comes up most of that too.)
    // How much the body rides each step is the fighter's weight: the heavies sink into every one,
    // the light ones are springy on their toes.
    let tread = (f.arch().bulk - 0.85).clamp(0.0, 0.4) / 0.4;
    let ride = match gait {
        Gait::Glide => 0.2,
        Gait::Shuffle => 0.8,
        Gait::Stomp => 2.4 + 1.2 * tread,
        _ => 0.9 + 1.6 * tread,
    };
    let rise = match gait { Gait::Shuffle => 0.0, Gait::Prowl => -6.0, _ => WALK_RISE };
    let sink = ride + ride * (2.0 * ph).cos() - rise - 0.8 * STYLES[f.who].sink;
    q.hip.u -= sink;
    q.head.u -= sink * 0.7;
    // The arms swing against the legs; a heavy swings them wide, a boxer hardly at all.
    let swing = match gait { Gait::Stomp => 2.0, Gait::Shuffle => 0.3, Gait::Glide => 0.5, _ => 1.0 };
    q.hip.f += 1.4 * s;
    q.head.f -= 1.0 * s * swing;
    q.lead_hand.f -= 2.6 * s * swing;
    q.rear_hand.f += 2.2 * s * swing;
    q.lead_hand.u -= sink * 0.7;
    q.rear_hand.u -= sink * 0.7;
    // Into the advance, chin first. (Walking back is the guard, below.)
    match gait {
        // Low to the floor, head out and the hands down near the boards.
        Gait::Prowl => {
            q.head.f += 9.0;
            q.head.u -= 4.0;
            q.lead_hand = p(q.lead_hand.f + 2.0, q.lead_hand.u - 8.0);
            q.rear_hand = p(q.rear_hand.f + 10.0, q.rear_hand.u - 18.0);
        }
        // Chest out, upright, as if the floor were optional.
        Gait::Glide => q.head.f -= 1.0,
        _ => q.head.f += 4.0,
    }
    q.hip.f += 1.5;
    q
}

/// Turtled up: weight off the front foot, forearms stacked before head or belly.
fn block_pose(mut q: Pose, f: &Fighter, bob: f32) -> Pose {
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
        // Each art guards its own way.
        let (near, far, open) = match ARTS[f.who] {
            // Soto-uke: the near forearm upright across the face, the other fist back at the hip.
            Art::Karate => (p(20.0, 92.0), p(q.hip.f + 7.0, q.hip.u + 15.0), false),
            // The shell: both gloves to the face, elbows tight to the ribs.
            Art::Boxing => (p(9.0, 86.0), p(25.0, 96.0), false),
            // Forearms up like a wall either side of the head.
            Art::MuayThai => (p(11.0, 90.0), p(29.0, 101.0), false),
            // Bong sau: the far forearm out flat across the chest, elbow up; the near hand behind it.
            Art::WingChun => (p(20.0, 80.0), p(31.0, 85.0), true),
            // The bridge: the far arm thrown out to meet the blow, the near claw at the chest.
            Art::HungGar => (p(16.0, 84.0), p(35.0, 81.0), true),
            // Pak sao: the far hand out slapping the blow aside, the near one cocked at the chin to answer;
            // or (his roll) the blow slipped, leaning back out of it with both hands up.
            Art::JeetKuneDo if f.stumble_roll % 2 == 1 => {
                q.hip.f -= 4.0;
                q.head = p(q.head.f - 10.0, q.head.u - 3.0);
                (p(10.0, 86.0), p(22.0, 92.0), false)
            }
            Art::JeetKuneDo => (p(14.0, 90.0), p(34.0, 86.0), true),
            _ => (q.lead_hand, q.rear_hand, q.open),
        };
        q.lead_hand = near;
        q.rear_hand = far;
        q.open = open;
        q.hip.u -= sink;
        q.head.u -= sink * 0.7;
        q.lead_hand.u -= sink * 0.7;
        q.rear_hand.u -= sink * 0.7;
        // A Thai fighter checks a blow with the shin: the lead knee comes up to meet it.
        if ARTS[f.who] == Art::MuayThai && f.stun > 0.0 {
            let k = (f.stun / (6.0 * F)).clamp(0.0, 1.0);
            q.lead_foot = q.lead_foot.to(p(18.0, 26.0), k);
            q.lead_heel = 0.3 * k;
        }
    }
    // A blocked blow (its blockstun is running) drives the forearms back into the body, and the
    // head with them.
    let jolt = (f.stun / (10.0 * F)).clamp(0.0, 1.0);
    let jolt = jolt * jolt;
    q.lead_hand.f -= 6.0 * jolt;
    q.rear_hand.f -= 7.0 * jolt;
    q.head.f -= 4.0 * jolt;
    q.hip.f -= 2.0 * jolt;
    q
}

/// Snapped back off the blow: head first, back foot skidding out.
fn hitstun_pose(mut q: Pose, f: &Fighter, low: bool, now: f32) -> Pose {
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
    q
}

/// Won: guard drops, the arm sweeps up and forward (raised straight it hides the face), then held.
fn victory_pose(mut q: Pose, f: &Fighter) -> Pose {
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
    // DRAKE wins three ways, at random (his per-attack roll): the nunchaku and the thumb, the salute, or the
    // beckon for more.
    // Over LANCER, DRAKE always plucks the hair and blows it away, as in Way of the Dragon's Colosseum.
    let plucks = plucking(f);
    let win = if plucks { Win::Pluck }
              else if formless(f.who) { [Win::Thumb, Win::Salute, Win::Beckon, Win::Stamp][(f.stumble_roll % 4) as usize] }
              else { STYLES[f.who].win };
    if win == Win::Pluck {
        // Weight back and easy, the far fist low; the near hand comes up to the lips, pinched, at 0.3 s,
        // and the blow is at 0.8 s (the hairs drift off in `draw_body`).
        let up = ease(0.25, 0.25);
        // (Held a little out from the lips: against them it read as eating.)
        q.lead_hand = q.lead_hand.to(p(q.head.f + 18.0, q.head.u - 3.0), up);
        q.rear_hand = q.rear_hand.to(p(30.0, 84.0), ease(0.0, 0.3));
        q.head.f -= 2.0 * up;
        q.open = false;
        q.twist = 0.0;
    } else if win == Win::Rei {
        let bow = ease(0.3, 0.42) * (1.0 - ease(1.2, 0.42));
        let ready = ease(1.5, 0.25);
        let hip = p(-6.0 * bow, HIP_U + 5.5 - 1.5 * bow - 2.0 * ready);
        let spine = HEAD_U - HIP_U + 1.0;
        let lean = 0.62 * bow;
        let head = p(hip.f + spine * lean.sin() + 3.0 * bow, hip.u + spine * lean.cos() - 3.0 * bow);
        let down = |side: f32| p(side + 12.0 * bow, 45.0 - 12.0 * bow);
        q = Pose {
            hip, head,
            // (0 and 14 apart: nearer, from the side the two hands are one.)
            lead_hand: down(0.0).to(p(8.0, 48.0), ready),
            rear_hand: down(14.0).to(p(18.0, 47.0), ready),
            lead_foot: p(6.0 + 4.0 * ready, 0.0),
            rear_foot: p(-6.0 - 4.0 * ready, 0.0),
            open: ready < 0.5,
            lead_heel: 0.0, rear_heel: 0.0, twist: 0.0,
        };
    } else if win == Win::Stamp {
        // Up comes the knee, then the foot drives down at a third of a second and stays planted, the fists
        // clenched low, the body sunk over it.
        let up = ease(0.0, 0.2) * (1.0 - ease(0.3, 0.06));
        let sunk = ease(0.3, 0.08);
        q.lead_foot = p(18.0, 34.0 * up);
        q.lead_heel = 0.2 * up;
        q.hip.u -= 4.0 * sunk;
        q.head = p(q.head.f + 4.0 * sunk, q.head.u - 6.0 * sunk);
        q.lead_hand = q.lead_hand.to(p(18.0, 60.0), ease(0.0, 0.2));
        q.rear_hand = q.rear_hand.to(p(-2.0, 62.0), ease(0.0, 0.2));
        q.open = false;
        q.twist = 0.0;
    } else if win == Win::Beckon {
        // Side on, chin up, the far hand out palm up and curling: come back for more.
        let on = ease(0.0, 0.3);
        let curl = ((t - 0.4).max(0.0) * 9.0).sin().max(0.0);
        q.rear_hand = q.rear_hand.to(p(40.0 - 3.0 * curl, 84.0 + 2.0 * curl), on);
        q.lead_hand = q.lead_hand.to(p(16.0, 80.0), on);
        q.head.u += 1.5 * on;
        q.open = true;
        q.twist = 0.0;
    } else if win == Win::Thumb && (0.9..1.6).contains(&t) {
        // Then caught under the arm, as in Enter the Dragon: the near hand at the chest holding the sticks
        // tucked back under the armpit, the far hand out in guard, held a moment.
        let k = ease(0.9, 0.12);
        q.lead_hand = q.lead_hand.to(p(q.head.f + 8.0, q.head.u - 30.0), k);
        q.rear_hand = q.rear_hand.to(p(36.0, 84.0), k);
        q.open = false;
        q.twist = 0.0;
    } else if win == Win::Thumb && t < 0.9 {
        // First the nunchaku whirled at his side (drawn in `draw_sticks`), weight back, chin up.
        let a = t * 24.0;
        let sh = p(q.head.f + 2.0, q.head.u - 20.0);
        q.lead_hand = q.lead_hand.to(p(sh.f + 20.0 + 6.0 * a.cos(), sh.u - 2.0 + 5.0 * (2.0 * a).sin()), ease(0.0, 0.2));
        q.rear_hand = q.rear_hand.to(p(30.0, 70.0), ease(0.0, 0.3));
        q.open = false;
        q.twist = 0.0;
    } else if win == Win::Thumb {
        // Then the sticks put away, weight back, the far fist low; the near thumb brushes across the nose,
        // twice, head tipped up.
        let on = 1.0;
        let flick = ((t - 1.7).max(0.0) * 10.0).sin().max(0.0) * (1.0 - ease(2.6, 0.3));
        q.head = p(q.head.f - 2.0 * on, q.head.u + 1.5 * on);
        // (Just in front of the face: the thumb is at the nose, the fist clear of the head.)
        q.lead_hand = q.lead_hand.to(p(q.head.f + 16.0 + 4.0 * flick, q.head.u - 8.0), on);
        q.rear_hand = q.rear_hand.to(p(30.0, 70.0), on);
        q.open = false;
        q.twist = 0.0;
    } else if matches!(win, Win::Salute | Win::Wai) {
        // Feet together, hands brought together, then the bow, held.
        let join = ease(0.0, 0.35);
        let bow = ease(0.45, 0.35);
        q.lead_foot = p(8.0, 0.0);
        q.rear_foot = p(-8.0, 0.0);
        q.rear_heel = 0.0;
        q.hip = p(0.0, HIP_U + 3.0);
        q.head = p(1.0 + 7.0 * bow, HEAD_U + 3.0 - 5.0 * bow);
        let at = if win == Win::Salute { p(17.0 + 4.0 * bow, 84.0 - 3.0 * bow) } else { p(19.0 + 5.0 * bow, 90.0 - 4.0 * bow) };
        // Fist and palm, or two palms: side on, one hand just in front of the other.
        q.lead_hand = q.lead_hand.to(p(at.f - 4.0, at.u), join);
        q.rear_hand = q.rear_hand.to(p(at.f + 4.0, at.u + 1.0), join);
        q.open = win == Win::Wai || join < 0.5;
        q.twist = 0.0;
    } else if win == Win::Flex {
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
    q
}

/// Rei: heels together, open hands at thighs, bow about 30°, then blend back to Idle.
fn intro_pose(mut q: Pose, f: &Fighter) -> Pose {
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
    // DRAKE opens either way, at random: the nunchaku and the beckon, or the salute.
    let intro = intro_now(f);
    match intro {
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
        Intro::Salute | Intro::Wai => {
            let nod = (1.0 - ((f.t - 0.8) / 0.25).abs()).max(0.0);
            let at = if intro == Intro::Salute { p(18.0, 84.0) } else { p(19.0, 90.0) };
            show.lead_hand = p(at.f - 4.0, at.u);
            show.rear_hand = p(at.f + 4.0, at.u + 1.0);
            show.head = p(1.0 + 6.0 * nod, HEAD_U + 3.0 - 4.0 * nod);
        }
        Intro::Beckon if f.t < 0.6 => {
            // First a flourish of the nunchaku at his side (drawn in `draw_sticks`).
            let a = f.t * 24.0;
            let sh = p(show.head.f + 2.0, show.head.u - 20.0);
            show.lead_hand = p(sh.f + 20.0 + 6.0 * a.cos(), sh.u - 2.0 + 5.0 * (2.0 * a).sin());
            show.rear_hand = p(16.0, 74.0);
            show.lead_foot = p(16.0, 0.0);
            show.rear_foot = p(-16.0, 0.0);
        }
        Intro::Beckon => {
            // Then side on, the far hand out palm up at the opponent, the fingers curling in twice.
            let curl = ((f.t - 0.7).max(0.0) * 9.0).sin().max(0.0) * (1.0 - ((f.t - 1.2) / 0.25).clamp(0.0, 1.0));
            show.rear_hand = p(40.0 - 3.0 * curl, 84.0 + 2.0 * curl);
            show.lead_hand = p(16.0, 80.0);
            show.head = p(3.0, HEAD_U);
            show.lead_foot = p(16.0, 0.0);
            show.rear_foot = p(-16.0, 0.0);
        }
        Intro::WarmUp => {
            let t = f.t;
            if t < 0.55 {
                // Palms pressed hard together before the chest, the shoulders squeezing.
                show.lead_hand = p(16.0, 86.0);
                show.rear_hand = p(23.0, 87.0);
                show.head = p(2.0, HEAD_U + 1.0);
            } else if t < 0.95 {
                // The neck rolled round, slow.
                let a = (t - 0.55) / 0.4 * std::f32::consts::TAU;
                show.lead_hand = p(10.0, 54.0);
                show.rear_hand = p(16.0, 52.0);
                show.head = p(2.0 + 3.0 * a.cos(), HEAD_U - 1.0 + 2.5 * a.sin());
            } else {
                // The knuckles cracked: one fist pressed into the other palm, and a twist.
                let crack = ((t - 0.95) * 14.0).sin().max(0.0);
                show.lead_hand = p(17.0 - 2.0 * crack, 82.0);
                show.rear_hand = p(24.0, 82.0 + 2.0 * crack);
                show.head = p(4.0, HEAD_U);
            }
        }
        Intro::Gloves => {
            // Two taps, a beat apart, then up into the guard.
            let tap = ((f.t - 0.3) * 8.0).sin().max(0.0) * (1.0 - ((f.t - 0.9) / 0.3).clamp(0.0, 1.0));
            show.lead_hand = p(20.0 - 3.0 * tap, 76.0);
            show.rear_hand = p(28.0 + 3.0 * tap - 10.0 * tap, 78.0);
            show.head = p(4.0, HEAD_U);
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
    q.open = matches!(intro, Intro::Wai | Intro::Beckon) || (intro == Intro::WarmUp && f.t < 0.55);
    q
}

/// The bow before the round.
fn bow_pose(f: &Fighter) -> Pose {
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
    Pose {
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
    }
}

/// Lost.
fn defeat_pose(mut q: Pose, f: &Fighter) -> Pose {
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
    q
}

/// What is laid over any pose: the knees giving on landing, rage, DRAKE shaking off a blow, feinting and taunting, a
/// stumble and a stagger, the lead foot switched, and the spin of a jump.
fn finishing_touches(mut q: Pose, f: &Fighter, now: f32, idx: usize, low: bool) -> Pose {
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

    // Struck, DRAKE shakes it off: a quick shake of the head as he comes out of it (a third of a second).
    if formless(f.who) && f.act == Act::Idle && f.prev_act == Act::Hitstun && f.t < 0.35 {
        let k = 1.0 - f.t / 0.35;
        q.head.f += 2.5 * (f.t * 40.0).sin() * k;
    }
    // DRAKE feints even standing still: every two seconds or so the lead hand flicks out a fake jab and back,
    // the head dipping behind it (a tenth of the cycle, about a fifth of a second).
    if formless(f.who) && f.act == Act::Idle && f.taunt <= 0.0 && !f.airborne() {
        let ph = (now * 0.43 + idx as f32 * 0.37).fract();
        if ph < 0.1 {
            let k = (ph / 0.1 * std::f32::consts::PI).sin();
            q.rear_hand.f += 14.0 * k;
            q.rear_hand.u += 3.0 * k;
            q.head.f += 3.0 * k;
            q.head.u -= 1.5 * k;
        }
    }
    // DRAKE's taunt over somebody he has put down: the thumb flicked across the nose, then the far hand out
    // palm up, curling: come on.
    // Or, as often, the nunchaku twirled at his side (drawn in `draw_sticks`).
    if f.taunt > 0.0 && f.act == Act::Idle && !f.airborne() && f.stumble_roll % 2 == 1 {
        let age = 1.4 - f.taunt;
        let on = (age / 0.2).min(1.0) * (f.taunt / 0.25).min(1.0);
        let a = age * 24.0;
        let sh = p(q.head.f + 2.0, q.head.u - 20.0);
        q.lead_hand = q.lead_hand.to(p(sh.f + 20.0 + 6.0 * a.cos(), sh.u - 2.0 + 5.0 * (2.0 * a).sin()), on);
    } else if f.taunt > 0.0 && f.act == Act::Idle && !f.airborne() {
        let age = 1.4 - f.taunt;
        let on = (age / 0.2).min(1.0) * (f.taunt / 0.25).min(1.0);
        let k = ((age - 0.7) / 0.2).clamp(0.0, 1.0);
        let turn = k * k * (3.0 - 2.0 * k);
        let flick = (age * 12.0).sin().max(0.0);
        let curl = ((age - 0.8).max(0.0) * 10.0).sin().max(0.0);
        // Torn and bleeding, the thumb goes to the lip instead: he tastes it.
        let lip = if (f.health as f32) < f.arch().health as f32 * 0.45 { -12.0 } else { -8.0 };
        q.lead_hand = q.lead_hand.to(p(q.head.f + 16.0 + 4.0 * flick, q.head.u + lip), on * (1.0 - turn));
        q.rear_hand = q.rear_hand.to(p(40.0 - 3.0 * curl, 84.0 + 2.0 * curl), on * turn);
        if on * turn > 0.5 { q.open = true; }
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
        && (matches!(f.act, Act::Idle | Act::Crouch | Act::Block | Act::Hitstun | Act::Attack)
            || f.act == Act::Walk && GAITS[f.who] == Gait::Shuffle) {
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
    /// Kung fu: fist into palm before the chest, and a nod over it.
    Salute,
    /// Muay Thai: palms together under the chin, the head bowed to them (the wai).
    Wai,
    /// A boxer taps the gloves together, twice, and brings them up.
    Gloves,
    /// The lead hand held out palm up, the fingers curling twice: come on.
    Beckon,
    /// DRAKE's warm-up from Way of the Dragon: palms pressed together, the neck rolled, the knuckles cracked.
    WarmUp,
}

pub(crate) fn intro_of(who: usize) -> Intro {
    match ARTS[who] {
        Art::HungGar | Art::WingChun => return Intro::Salute,
        Art::MuayThai => return Intro::Wai,
        Art::Boxing => return Intro::Gloves,
        Art::JeetKuneDo => return Intro::Beckon,
        // Taekwondo bows as karate does.
        Art::Taekwondo => return Intro::Bow,
        _ => {}
    }
    match FIGHTERS[who].build {
        Build::Gi | Build::Tunic => Intro::Bow,
        // (Limbering up before the duel, as both men do in the Colosseum.)
        Build::Duelist => Intro::WarmUp,
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
