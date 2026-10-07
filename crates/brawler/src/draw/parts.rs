//! What a fighter is drawn from: bone lengths, the look of each build, and
//! the limbs, torso, cape, belt and head, each one flat colour inside one
//! outline.

use super::*;

// Poses use forward/up coordinates; hands and feet are placed, joints solved.
// Each limb is outlined and filled as one shape.

/// A point on a fighter: `f` forward — the way they face — and `u` up
/// from the floor under them.
#[derive(Copy, Clone)]
pub(crate) struct P { pub(crate) f: f32, pub(crate) u: f32 }
/// A pose point: `f` forward of the feet, `u` up from the floor.
pub(crate) const fn p(f: f32, u: f32) -> P { P { f, u } }

impl P {
    /// Part of the way from here to there. Animation is almost entirely
    /// this, which is why it is the only operator a pose needs.
    pub(crate) fn to(self, o: P, k: f32) -> P { p(self.f + (o.f - self.f) * k, self.u + (o.u - self.u) * k) }
}

/// A point on screen.
#[derive(Copy, Clone, Debug)]
pub(crate) struct V(pub f32, pub f32);

/// Distance between two screen points.
pub(crate) fn dist(a: V, b: V) -> f32 { ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt() }
/// The point `k` of the way from `a` to `b`.
pub(crate) fn along(a: V, b: V, k: f32) -> V { V(a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k) }

/// Where a fighter's pose space lands on screen.
#[derive(Copy, Clone)]
pub(crate) struct Rig {
    pub(crate) cx: f32,
    pub(crate) ground: f32,
    /// Which way "forward" points in pose space.
    pub(crate) fwd: f32,
    /// Which way the face points. The same as `fwd` on a fighter who is
    /// upright, and the opposite on one who has been put on their back.
    pub(crate) face: f32,
    /// The fighter's size: pose space is a full-size fighter's.
    pub(crate) scale: f32,
}

impl Rig {
    /// A pose point as a screen point.
    pub(crate) fn at(self, q: P) -> V {
        V(self.cx + self.fwd * q.f * self.scale, self.ground - q.u * self.scale)
    }
    /// A rig standing at (`cx`, `ground`), body turned `fwd` and face turned
    /// `face`.
    pub(crate) fn upright(cx: f32, ground: f32, fwd: f32, face: f32, scale: f32) -> Rig {
        Rig { cx, ground, fwd, face, scale }
    }
}

/// Bone lengths in pixels. The legs add up to more than the hip height, so a
/// fighter stands with bent knees.
pub(crate) const THIGH: f32 = 30.0;
pub(crate) const SHIN: f32 = 29.0;
pub(crate) const UARM: f32 = 24.0;
pub(crate) const FARM: f32 = 21.0;

/// Joint heights standing and crouching: hips and the centre of the head (the
/// spine between is one bone, see `neck_of`). Hips low: at 58 the solver
/// clamped both legs straight (see `a_waiting_fighter_has_their_knees_bent`).
pub(crate) const HIP_U: f32 = 53.0;
pub(crate) const HEAD_U: f32 = 105.0;
pub(crate) const C_HIP: f32 = 31.0;
pub(crate) const C_HEAD: f32 = 61.0;

/// Five heads tall, not life's seven and a half: a cartoon's proportions,
/// and a skull big enough to carry a face.
pub(crate) fn head_r(bulk: f32) -> f32 { 12.0 + 1.5 * (bulk - 1.0) }

/// The line around everything, and its width. A fighter without one dissolves
/// into whichever stage happens to be a similar colour.
pub(crate) const INK: BlipColor = BlipColor { r: 0.07, g: 0.06, b: 0.09, a: 1.0 };
/// Lettering that is not the headline: a white with the glare taken off.
pub(crate) const PALE: BlipColor = BlipColor { r: 0.92, g: 0.92, b: 0.96, a: 1.0 };
pub(crate) const LINE: f32 = 2.0;

/// How much darker the far arm and leg are: the one shadow tone.
pub(crate) const FAR: f32 = 0.80;

/// What the face is doing. An expression says who just got hit faster than
/// a health bar does.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum Mood { Calm, Shout, Hurt, Happy }

/// How a fighter is coloured and clothed.
#[derive(Copy, Clone)]
pub(crate) struct Look {
    pub(crate) cloth: BlipColor,
    pub(crate) trim: BlipColor,
    pub(crate) skin: BlipColor,
    pub(crate) hair: BlipColor,
    pub(crate) build: Build,
    /// Every radius on the body is multiplied by this: the fighter's bulk
    /// times their size.
    pub(crate) bulk: f32,
    pub(crate) size: f32,
    pub(crate) far: bool,
    /// Eyes shut for a moment: a blink.
    pub(crate) blink: bool,
    /// How dark this part is drawn: 1.0 in the light, less for what is
    /// further from it.
    pub(crate) tone: f32,
    /// Which way loose ends trail, in head radii: against the way the fighter
    /// is moving.
    pub(crate) wind: (f32, f32),
    /// Blown toward white for the frames a hit is frozen on.
    pub(crate) flash: f32,
}

impl Look {
    /// `base` in this part's tone, flashed toward white while a hit is
    /// freezing the frame.
    pub(crate) fn c(&self, base: BlipColor) -> BlipColor {
        blend(shade(base, self.tone), BLIP_WHITE, self.flash)
    }
    /// The outline colour, flashed with the rest.
    pub(crate) fn ink(&self) -> BlipColor { blend(INK, BLIP_WHITE, self.flash * 0.6) }
}

/// A tapered capsule — the only primitive a body is made of.
pub(crate) type Cap = (V, V, f32, f32);

/// A tapered capsule between two points.
pub(crate) fn stroke(blip: &Blip, a: V, b: V, r1: f32, r2: f32, c: BlipColor) {
    blip.fill_capsule(a.0, a.1, b.0, b.1, r1, r2, c);
}

/// One shape out of several capsules: all the ink, then all the colour, so
/// the outline runs round the whole and not round each piece.
pub(crate) fn shape(blip: &Blip, caps: &[Cap], fill: BlipColor, ink: BlipColor) {
    for &(a, b, r1, r2) in caps { stroke(blip, a, b, r1 + LINE, r2 + LINE, ink); }
    for &(a, b, r1, r2) in caps { stroke(blip, a, b, r1, r2, fill); }
}

/// How far a two-bone limb can fold: a knee to about thirty degrees, an elbow
/// to about thirty-five.
pub(crate) const KNEE_SHUT: f32 = 0.52;
pub(crate) const ELBOW_SHUT: f32 = 0.61;
/// The nearest a hand is drawn to its own shoulder.
pub(crate) const ARM_MIN: f32 = 18.0;

/// Solve a two-bone limb without stretching; clamp unreachable targets and choose the bend with `toward`.
pub(crate) fn solve(root: V, want: V, l1: f32, l2: f32, shut: f32, toward: V) -> (V, V) {
    let (dx, dy) = (want.0 - root.0, want.1 - root.1);
    let d_raw = (dx * dx + dy * dy).sqrt();
    // How close the two ends can get with the joint fully shut.
    let d_min = (l1 * l1 + l2 * l2 - 2.0 * l1 * l2 * shut.cos()).sqrt();
    let d = d_raw.clamp(d_min, l1 + l2);
    // A target sitting exactly on the root has no direction to go in.
    let (ux, uy) = if d_raw > 0.001 { (dx / d_raw, dy / d_raw) } else { (toward.0, toward.1) };
    let end = V(root.0 + ux * d, root.1 + uy * d);

    let cs = ((l1 * l1 + d * d - l2 * l2) / (2.0 * l1 * d)).clamp(-1.0, 1.0);
    let base = uy.atan2(ux);
    let off = cs.acos();
    let one = V(root.0 + l1 * (base + off).cos(), root.1 + l1 * (base + off).sin());
    let two = V(root.0 + l1 * (base - off).cos(), root.1 + l1 * (base - off).sin());
    // The two solutions mirror about the line between the ends; `toward`
    // always picks.
    let score = |c: V| (c.0 - root.0) * toward.0 + (c.1 - root.1) * toward.1;
    (if score(one) >= score(two) { one } else { two }, end)
}

// Big fists, big feet, big head: the ends of a fighter are what hit and what
// get hit, so they are the clearest shapes on it.

/// Height of the ankle over the sole, and how far the toes reach past it.
/// Kicks aim the ankle short by `FOOT`, or the toes overshoot the hitbox.
pub(crate) const ANKLE: f32 = 7.8;
pub(crate) const FOOT: f32 = 16.0;

/// The same for a punch: how far the front of the fist is past the wrist.
pub(crate) const FIST: f32 = 7.0;

/// Which way the toes point, and how much of a kick this is (0 on or near the
/// floor, 1 well off it): a standing foot lies flat, a kicking one points
/// along the shin.
pub(crate) fn foot_dir(knee: V, ankle: V, fwd: f32, ground: f32, size: f32) -> (f32, f32, f32) {
    let (sx, sy) = unit(knee, ankle);
    let k = ((ground - ankle.1 - 22.0 * size) / (12.0 * size)).clamp(0.0, 1.0);
    let k = k * k * (3.0 - 2.0 * k);
    let (x, y) = (fwd + (sx - fwd) * k, sy * k);
    let m = x.hypot(y).max(1e-3);
    (x / m, y / m, k)
}

/// A point `at` turned about `pivot` by `ang`, clockwise on screen for a
/// fighter facing right (`fwd` mirrors it).
pub(crate) fn turn(at: V, pivot: V, ang: f32, fwd: f32) -> V {
    let (s, c) = ((ang * fwd).sin(), (ang * fwd).cos());
    let (x, y) = (at.0 - pivot.0, at.1 - pivot.1);
    V(pivot.0 + x * c - y * s, pivot.1 + x * s + y * c)
}

/// A leg and its foot. `heel` is how far the heel is raised, in radians: on
/// the floor the foot bends at the ball and the toes stay down, the way a
/// fighter stands on the back foot; in the air the whole foot points.
pub(crate) fn draw_leg(blip: &Blip, l: &Look, hip: V, knee: V, ankle: V, fwd: f32, ground: f32, heel: f32) {
    let b = l.bulk;
    let ink = l.ink();
    let (skin, cloth, trim) = (l.c(l.skin), l.c(l.cloth), l.c(l.trim));
    let booted = matches!(l.build, Build::Bare | Build::Suit | Build::Spider | Build::Caped);

    // The pose places the sole; a kick (well off the floor) places the ankle
    // itself, with the foot pointed along the shin.
    let (dx, dy, k) = foot_dir(knee, ankle, fwd, ground, l.size);
    let rest = ground - ANKLE * b - LINE;
    let planted = ankle.1 - (ANKLE * b + LINE) * (1.0 - k) >= rest - 0.5;
    let flat = V(ankle.0, (ankle.1 - (ANKLE * b + LINE) * (1.0 - k)).min(rest));

    // The foot in its own frame: `d` toward the toes, `n` toward the sole.
    let (nx, ny) = (-dy * fwd, dx * fwd);
    let at = |d: f32, n: f32| V(flat.0 + (dx * d + nx * n) * b, flat.1 + (dy * d + ny * n) * b);
    let toe_r = if booted { 3.4 } else { 2.8 };
    let (heel_at, ball, toe) = (at(-3.0, ANKLE - 5.0), at(8.0, ANKLE - 3.6),
                                at(FOOT - toe_r - LINE / b, ANKLE - toe_r));
    // Heel up: about the ball on the floor, about the ankle off it.
    let heel = if planted { heel.max(0.0) } else { heel };
    let (ankle, heel_at, ball, toe) = if planted {
        let pivot = at(8.0, ANKLE);
        (turn(flat, pivot, heel, fwd), turn(heel_at, pivot, heel, fwd), ball, toe)
    } else {
        (flat, turn(heel_at, flat, heel, fwd), turn(ball, flat, heel, fwd), turn(toe, flat, heel, fwd))
    };
    let pad = if booted { 0.5 } else { 0.0 };
    let foot = [(heel_at, ball, (5.0 + pad) * b, (3.6 + pad) * b), (ball, toe, (3.6 + pad) * b, toe_r * b)];

    // Thigh, a calf that swells below the knee, and a thin ankle.
    let (top, mid, calf, thin) = match l.build {
        Build::Gi => (10.0, 8.0, 7.6, 3.8),
        Build::Suit | Build::Spider | Build::Caped => (8.8, 6.4, 6.2, 4.2),
        Build::Bare => (10.0, 7.6, 7.2, 5.0),
        Build::Turtle | Build::Giant => (10.0, 7.4, 7.2, 4.0),
    };
    let swell = along(knee, ankle, 0.30);
    let thigh = (hip, knee, top * b, mid * b);
    let shin = [(knee, swell, mid * b, calf * b), (swell, ankle, calf * b, thin * b)];
    match l.build {
        // Bare legs and feet, one skin: knee pads, or trousers torn at the knee.
        Build::Turtle | Build::Giant => {
            shape(blip, &[thigh, shin[0], shin[1], foot[0], foot[1]], skin, ink);
            if l.build == Build::Turtle { blip.fill_circle(knee.0, knee.1, mid * b, trim); }
            else { stroke(blip, hip, along(hip, knee, 0.85), top * b, (mid + 0.4) * b, cloth); }
        }
        // Loose trousers to the shin, bare below.
        Build::Gi => {
            let hem = along(swell, ankle, 0.72);
            shape(blip, &[(hem, ankle, 4.6 * b, thin * b), foot[0], foot[1]], skin, ink);
            shape(blip, &[thigh, shin[0], (swell, hem, calf * b, 6.6 * b)], cloth, ink);
        }
        // Boots, with a shaft up the shin: dark leather on the wrestler.
        _ => {
            shape(blip, &[thigh, shin[0], shin[1]], cloth, ink);
            let boot = if l.build == Build::Bare { l.c(BOOT) } else { trim };
            let cuff = along(swell, ankle, 0.45);
            shape(blip, &[(cuff, ankle, (thin + 1.6) * b, (thin + 0.6) * b), foot[0], foot[1]], boot, ink);
        }
    }

    // The back of the near leg in shadow, like the back of the body.
    if !l.far {
        let behind = |a: V, c: V| {
            let (x, y) = unit(a, c);
            let (p, q) = ((y, -x), (-y, x));
            // Under the leg when it is raised, behind it when it stands.
            if -0.4 * p.0 * fwd + p.1 >= -0.4 * q.0 * fwd + q.1 { p } else { q }
        };
        let thigh_c = if l.build == Build::Turtle { skin } else { cloth };
        let (px, py) = behind(hip, knee);
        let off = |v: V, d: f32| V(v.0 + px * d * b, v.1 + py * d * b);
        stroke(blip, off(along(hip, knee, 0.25), top * 0.85 - 3.0), off(knee, mid - 2.8), 2.8 * b, 2.6 * b,
            shade(thigh_c, 0.84));
    }

    // What makes it a foot and not a shoe-shape, on the near one only: the
    // big toe set apart and the ankle bone, or a sole under the boot.
    if l.far { return; }
    let dark = shade(skin, 0.62);
    if booted {
        let sole = shade(if l.build == Build::Bare { l.c(BOOT) } else { trim }, 0.55);
        let under = |p: V, r: f32| V(p.0 + nx * r, p.1 + ny * r);
        stroke(blip, under(heel_at, 4.3 * b), under(ball, 3.0 * b), 1.1 * b, 1.0 * b, sole);
        stroke(blip, under(ball, 3.0 * b), under(toe, (toe_r - 1.0) * b), 1.0 * b, 0.9 * b, sole);
    } else {
        let (tx, ty) = unit(ball, toe);
        let gap = V(toe.0 - tx * 2.2 * b, toe.1 - ty * 2.2 * b);
        stroke(blip, V(gap.0 - nx * 2.2 * b, gap.1 - ny * 2.2 * b), gap, 0.6, 0.6, dark);
        if l.build == Build::Turtle {
            // Two broad toes, not five.
            let mid = along(ball, toe, 0.2);
            stroke(blip, V(mid.0 - nx * 2.8 * b, mid.1 - ny * 2.8 * b), mid, 0.6, 0.6, dark);
        } else {
            blip.fill_circle(ankle.0 - dx * 0.8 * b, ankle.1 + 0.6 * b, 1.0 * b.min(1.3), dark);
        }
    }
}

/// An arm and its hand. `hand` is the middle of the fist; the wrist is short
/// of it. `fwd` keeps the palm side down whichever way the fighter faces.
pub(crate) fn draw_arm(blip: &Blip, l: &Look, shoulder: V, elbow: V, hand: V, open: bool, fwd: f32) {
    let b = l.bulk;
    let ink = l.ink();
    let (skin, trim) = (l.c(l.skin), l.c(l.trim));
    let sleeved = matches!(l.build, Build::Suit | Build::Spider | Build::Caped);
    let (dx, dy) = unit(elbow, hand);
    let (px, py) = (-dy * fwd, dx * fwd);
    let at = |f: f32, a: f32| V(hand.0 + (dx * f + px * a) * b, hand.1 + (dy * f + py * a) * b);

    // Shoulder cap, an upper arm that swells at the biceps, a forearm that
    // swells below the elbow and narrows to the wrist.
    let wrist = at(-4.0, 0.0);
    let biceps = along(shoulder, elbow, 0.45);
    let swell = along(elbow, wrist, 0.30);
    shape(blip, &[
        (shoulder, biceps, 6.8 * b, 6.2 * b),
        (biceps, elbow, 6.2 * b, 4.6 * b),
        (elbow, swell, 4.6 * b, 5.2 * b),
        (swell, wrist, 5.2 * b, 3.6 * b),
    ], if sleeved { l.c(l.cloth) } else { skin }, ink);
    // What is worn on the forearm, inside the arm's own outline.
    let band = |from: f32, to: f32| stroke(blip, along(swell, wrist, from), along(swell, wrist, to),
        (5.2 - 1.6 * from) * b, (5.2 - 1.6 * to) * b, trim);
    match l.build {
        Build::Giant | Build::Caped => {}
        Build::Spider => band(0.2, 1.0),
        Build::Turtle => {
            blip.fill_circle(elbow.0, elbow.1, 4.6 * b, trim);
            band(0.45, 1.0);
        }
        _ => band(0.45, 1.0),
    }

    let lines = !l.far;
    let dark = shade(skin, 0.60);
    let crease = |a: V, c: V| stroke(blip, a, c, 0.5 * b.max(1.0), 0.5 * b.max(1.0), dark);
    if open {
        // A hand: palm, fingers a little apart, the thumb out on top. A
        // turtle has three fingers.
        let fingers: &[f32] = if l.build == Build::Turtle { &[-2.0, 0.0, 2.0] } else { &[-2.4, -0.8, 0.8, 2.4] };
        let r = if l.build == Build::Turtle { 1.5 } else { 1.15 };
        let mut caps = vec![(at(-3.0, 0.0), at(1.0, 0.0), 3.3 * b, 3.3 * b),
                            (at(-1.0, -2.6), at(2.4, -4.8), 1.4 * b, 1.1 * b)];
        for a in fingers { caps.push((at(2.0, *a), at(6.2, a * 1.25 + 0.6), r * b, (r - 0.2) * b)); }
        shape(blip, &caps, skin, ink);
        if lines {
            for w in fingers.windows(2) {
                let a = (w[0] + w[1]) / 2.0;
                crease(at(2.8, a), at(6.0, a * 1.25 + 0.6));
            }
        }
    } else {
        // A fist in profile: the flat back of the hand, the fingers curled
        // down the front under two knuckles, the thumb locked across them.
        shape(blip, &[
            (at(-3.2, -0.8), at(1.6, -1.2), 3.3 * b, 3.3 * b),
            (at(2.0, -1.0), at(1.2, 2.0), 2.9 * b, 2.9 * b),
            (at(2.6, -3.5), at(0.6, -3.9), 1.3 * b, 1.2 * b),
        ], skin, ink);
        if lines {
            // The thumb's edge and tip, and the fold of the fingers.
            crease(at(-1.6, 0.9), at(2.3, 1.9));
            crease(at(2.3, 1.9), at(2.5, 3.5));
            crease(at(4.0, -1.6), at(4.4, 0.4));
        }
    }
}

/// A wedge: narrow hips, wide chest.
pub(crate) const WAIST_R: f32 = 8.6;
pub(crate) const CHEST_R: f32 = 12.8;
pub(crate) const CHEST_AT: f32 = 0.60;
pub(crate) const BELT_AT: f32 = 0.30;
/// The body's radius at the belt.
pub(crate) const BELT_R: f32 = WAIST_R + (CHEST_R - WAIST_R) * BELT_AT / CHEST_AT;

pub(crate) const GOLD: BlipColor = BlipColor { r: 1.0, g: 0.84, b: 0.22, a: 1.0 };
pub(crate) const BOOT: BlipColor = BlipColor { r: 0.24, g: 0.16, b: 0.13, a: 1.0 };
pub(crate) const PLASTRON: BlipColor = BlipColor { r: 0.92, g: 0.80, b: 0.44, a: 1.0 };

/// The trunk and pelvis in one outline, the chest detail of the build, and
/// the shadow down the back.
pub(crate) fn draw_torso(blip: &Blip, l: &Look, hip: V, neck: V, fwd: f32) {
    let b = l.bulk;
    let ink = l.ink();
    let chest = along(hip, neck, CHEST_AT);
    let front = |k: f32, out: f32| { let v = along(hip, neck, k); V(v.0 + fwd * out * b, v.1) };
    let body = match l.build {
        Build::Gi | Build::Suit | Build::Caped => l.c(l.cloth),
        Build::Bare | Build::Giant => l.c(l.skin),
        Build::Turtle => l.c(PLASTRON),
        Build::Spider => l.c(l.trim),
    };
    // The shell, behind the body.
    if l.build == Build::Turtle {
        shape(blip, &[(front(0.10, -7.0), front(0.72, -8.0), 10.0 * b, 11.5 * b)], l.c(l.cloth), ink);
    }
    // The ribcage, and above it the shoulders narrowing to the neck, both
    // centred on the spine: the back is as flat as the chest is deep, and
    // slopes in to the neck instead of standing up behind it.
    let trunk = [(hip, chest, WAIST_R * b, CHEST_R * b),
                 (chest, along(hip, neck, 0.90), CHEST_R * b, 6.5 * b)];
    // Under it the pelvis, the seat both thighs come out of, in the near
    // side's colour and inside the same outline: it covers the root of the
    // far leg, so that leg is not stuck on behind.
    let (sx, sy) = unit(hip, neck);
    let seat_at = |across: f32| V(hip.0 + (-sy * fwd * across - sx) * b, hip.1 + (sx * fwd * across - sy) * b);
    let seat = (seat_at(-1.0), seat_at(3.0), 8.2 * b, 8.6 * b);
    let seat_c = if l.build == Build::Turtle { l.c(l.skin) } else { l.c(l.cloth) };
    for &(a, c, r1, r2) in trunk.iter().chain([&seat]) { stroke(blip, a, c, r1 + LINE, r2 + LINE, ink); }
    stroke(blip, seat.0, seat.1, seat.2, seat.3, seat_c);
    for &(a, c, r1, r2) in &trunk { stroke(blip, a, c, r1, r2, body); }
    let crease = |a: V, b: V| stroke(blip, a, b, 1.0, 1.0, shade(body, 0.62));
    match l.build {
        // The open collar of a gi: skin in a V down to the belt.
        Build::Gi => stroke(blip, front(0.92, 1.0), front(0.42, 2.0), 4.6 * b, 1.2 * b, l.c(l.skin)),
        // Trousers to the waist, and the line under the chest.
        Build::Bare | Build::Giant => {
            stroke(blip, hip, along(hip, neck, BELT_AT), WAIST_R * b, BELT_R * b, l.c(l.cloth));
            // That stroke ends in a dome above the belt; the skin is put
            // back over it in slices across the body, so the trousers stop
            // at the waist.
            let (sx, sy) = unit(hip, neck);
            let len = dist(hip, neck);
            let mut t = BELT_AT;
            while t < BELT_AT + (BELT_R * b + 2.0) / len {
                let at = along(hip, neck, t);
                let w = (WAIST_R + (CHEST_R - WAIST_R) * (t / CHEST_AT).min(1.0)) * b - 1.2;
                stroke(blip, V(at.0 + sy * w, at.1 - sx * w), V(at.0 - sy * w, at.1 + sx * w), 1.2, 1.2, body);
                t += 1.0 / len;
            }
            // The chest faces the viewer: a line under each side of it, the
            // breastbone between, and the top of the stomach.
            crease(front(0.66, -8.0), front(0.61, -1.0));
            crease(front(0.61, 1.5), front(0.66, 9.0));
            crease(front(0.84, 0.5), front(0.63, 0.5));
            crease(front(0.50, 0.5), front(0.40, 0.5));
        }
        // A stripe down the front of the suit.
        Build::Suit => stroke(blip, front(0.36, 1.0), front(0.82, 1.0), 2.0 * b, 2.0 * b, l.c(l.trim)),
        // The plates of the plastron.
        Build::Turtle => {
            crease(front(0.52, -8.0), front(0.50, 10.0));
            crease(front(0.74, -7.0), front(0.72, 10.0));
            crease(front(0.86, 1.0), front(0.34, 1.0));
        }
        // The shield on the chest. No trunks: a round red seat under a belt
        // is a nappy.
        Build::Caped => {
            let at = front(0.70, 1.0);
            blip.fill_circle(at.0, at.1, 5.6 * b, l.c(GOLD));
            blip.fill_circle(at.0, at.1, 3.4 * b, l.c(l.trim));
        }
        // The spider on the chest.
        Build::Spider => {
            let at = front(0.66, 1.0);
            blip.fill_circle(at.0, at.1, 2.4 * b, ink);
            for (dx, dy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                stroke(blip, at, V(at.0 + dx * 5.0 * b, at.1 + dy * 4.0 * b), 0.8, 0.8, ink);
            }
        }
    }

    // One shadow tone down the back and under the seat: the light is in
    // front and above, and a body with no dark side is a paper cut-out.
    if l.build == Build::Turtle { return; }
    let radius = |t: f32| if t <= CHEST_AT { WAIST_R + (CHEST_R - WAIST_R) * t / CHEST_AT }
        else { CHEST_R + (6.5 - CHEST_R) * (t - CHEST_AT) / (0.90 - CHEST_AT) };
    let back = |t: f32, r: f32| {
        let v = along(hip, neck, t);
        let out = (radius(t) - r) * b;
        V(v.0 + sy * fwd * out, v.1 - sx * fwd * out)
    };
    let dim = |c: BlipColor| shade(c, 0.84);
    let lower = if matches!(l.build, Build::Bare | Build::Giant) { l.c(l.cloth) } else { body };
    stroke(blip, back(0.38, 3.4), back(0.62, 3.6), 3.4 * b, 3.6 * b, dim(body));
    stroke(blip, back(0.62, 3.6), back(0.84, 2.6), 3.6 * b, 2.6 * b, dim(body));
    stroke(blip, back(0.04, 3.0), back(0.24, 3.2), 3.0 * b, 3.2 * b, dim(lower));
    let under = |across: f32, down: f32| V(seat.0 .0 + (-sy * fwd * across - sx * down) * b,
                                           seat.0 .1 + (sx * fwd * across - sy * down) * b);
    stroke(blip, under(-5.0, 1.0), under(-1.5, 5.6), 3.2 * b, 3.0 * b, dim(seat_c));
}

/// The cape: pinned across the shoulders and lying down the back before it
/// hangs, so it trails behind a body that leans instead of dropping between
/// the legs; in flight (`level`) it streams straight back along the body.
pub(crate) fn draw_cape(blip: &Blip, l: &Look, hip: V, neck: V, fwd: f32, level: f32, now: f32) {
    let b = l.bulk;
    let (sx, sy) = unit(hip, neck);
    let top = V(neck.0 - sx * 6.0 * b - fwd * 5.0 * b, neck.1 - sy * 6.0 * b);
    // Half down the spine, half straight down with a little drift behind.
    let follow = 0.5 + 0.5 * level;
    let (hx, hy) = (-fwd * 0.30, 1.0);
    let (dx, dy) = (hx + (-sx - hx) * follow, hy + (-sy - hy) * follow);
    let m = dx.hypot(dy).max(1e-3);
    let (dx, dy) = (dx / m, dy / m);
    let wave = (now * (3.0 + 6.0 * level)).sin() * (1.5 + 3.0 * level) * b;
    let len = 64.0 * b;
    let at = |k: f32, side: f32| V(top.0 + dx * len * k - dy * side, top.1 + dy * len * k + dx * side);
    shape(blip, &[(top, at(0.45, 0.0), 8.0 * b, 11.0 * b), (at(0.45, 0.0), at(1.0, wave), 11.0 * b, 7.0 * b)],
        l.c(l.trim), l.ink());
}

/// The belt, drawn after the near leg so it lies over the top of the thigh.
pub(crate) fn draw_belt(blip: &Blip, l: &Look, hip: V, neck: V, fwd: f32) {
    let b = l.bulk;
    let c = match l.build {
        Build::Gi => l.c(BlipColor { r: 0.13, g: 0.12, b: 0.15, a: 1.0 }),
        Build::Turtle => l.c(BlipColor { r: 0.38, g: 0.24, b: 0.13, a: 1.0 }),
        Build::Bare | Build::Suit => l.c(l.trim),
        Build::Caped => l.c(GOLD),
        Build::Spider | Build::Giant => return,
    };
    let at = along(hip, neck, BELT_AT);
    let (sx, sy) = unit(hip, neck);
    let (px, py) = (-sy, sx);
    let r = 2.8 * b;
    let w = BELT_R * b - r;
    stroke(blip, V(at.0 - px * w, at.1 - py * w), V(at.0 + px * w, at.1 + py * w), r, r, c);
    let knot = V(at.0 + fwd * 1.0 * b, at.1);
    match l.build {
        // The knot and one end hanging off it.
        Build::Gi => stroke(blip, knot, V(knot.0 + fwd * 1.5, knot.1 + 10.0 * b), 2.2 * b, 1.2 * b, c),
        // A buckle in the mask's colour.
        Build::Turtle => blip.fill_circle(knot.0, knot.1, 3.4 * b, l.c(l.trim)),
        _ => {}
    }
}

/// Head, neck, hair or mask, and a face in the given mood.
pub(crate) fn draw_head(blip: &Blip, l: &Look, fw: f32, head: V, neck: V, mood: Mood) {
    let r = head_r(l.bulk / l.size) * l.size;
    let ink = l.ink();
    let hair = l.c(l.hair);
    let masked = l.build == Build::Spider;
    let skin = if masked { l.c(l.trim) } else { l.c(l.skin) };
    // Every offset is in head radii, so the head scales whole: `f` toward
    // the face, `d` down.
    let at = |f: f32, d: f32| V(head.0 + fw * f * r, head.1 + d * r);
    let dot = |f: f32, d: f32, k: f32| (at(f, d), at(f, d), k * r, k * r);

    // Hair first, since it sticks out behind the skull and needs the line.
    let locks: &[Cap] = match l.build {
        // Short and spiked.
        Build::Gi => &[dot(-0.22, -0.42, 0.80), dot(-0.66, 0.02, 0.52),
                       (at(0.25, -0.80), at(0.62, -1.02), 0.30 * r, 0.10 * r),
                       (at(-0.25, -0.90), at(-0.30, -1.22), 0.34 * r, 0.10 * r)],
        // Swept back into a tail.
        Build::Suit => &[dot(-0.26, -0.40, 0.78),
                         (at(-0.90, -0.10), at(-1.45 + l.wind.0, 0.85 + l.wind.1), 0.36 * r, 0.14 * r)],
        // A mop, falling over the brow.
        Build::Giant => &[dot(-0.26, -0.50, 0.78), dot(-0.70, -0.02, 0.50),
                          dot(0.22, -0.86, 0.36)],
        // Neat, with one curl on the forehead.
        Build::Caped => &[dot(-0.22, -0.44, 0.80), dot(-0.66, 0.00, 0.50),
                          dot(0.52, -0.66, 0.20)],
        Build::Bare | Build::Turtle | Build::Spider => &[],
    };
    for &(a, b, r1, r2) in locks { stroke(blip, a, b, r1 + LINE, r2 + LINE, ink); }

    // Neck, skull and jaw as one silhouette. The jaw hung off the front of
    // the skull is what makes it a profile; a turtle has a muzzle instead.
    let neck = (neck, at(-0.12, 0.55), 5.2 * l.bulk, 4.8 * l.bulk);
    match l.build {
        Build::Turtle => shape(blip, &[neck, dot(0.0, 0.0, 1.0),
            (at(0.16, 0.36), at(0.56, 0.42), 0.62 * r, 0.56 * r)], skin, ink),
        Build::Spider => shape(blip, &[neck, dot(0.0, 0.0, 1.0),
            (at(0.08, 0.30), at(0.40, 0.62), 0.60 * r, 0.42 * r)], skin, ink),
        _ => shape(blip, &[neck, dot(0.0, 0.0, 1.0),
            (at(0.08, 0.30), at(0.44, 0.64), 0.60 * r, 0.42 * r),
            dot(0.98, 0.14, 0.15)], skin, ink),
    }
    for &(a, b, r1, r2) in locks { stroke(blip, a, b, r1, r2, hair); }

    // A band round the head with two ends trailing behind: a headband at the
    // hairline, or a turtle's mask across the eyes.
    let band = |d: f32, w: f32| {
        let c = l.c(l.trim);
        let knot = at(-0.94, d + 0.14);
        let (wf, wd) = l.wind;
        stroke(blip, knot, at(0.92, d), w * r, w * r, c);
        stroke(blip, knot, at(-1.80 + wf, d + 0.42 + wd), 0.15 * r, 0.07 * r, c);
        stroke(blip, knot, at(-1.62 + wf * 0.8, d + 0.78 + wd * 1.2), 0.15 * r, 0.07 * r, c);
    };
    match l.build {
        Build::Gi => band(-0.40, 0.19),
        Build::Turtle => band(-0.14, 0.30),
        // A beard along the jaw.
        Build::Bare => stroke(blip, at(-0.10, 0.52), at(0.50, 0.70), 0.44 * r, 0.40 * r, hair),
        _ => {}
    }
    let dark = blend(INK, BLIP_WHITE, l.flash * 0.6);
    let line = |a: V, b: V, k: f32| stroke(blip, a, b, k * r, k * r, dark);

    // The mask has no face, only two big lenses that narrow when hurt.
    if masked {
        let k = if mood == Mood::Hurt { 0.10 } else { 0.20 };
        stroke(blip, at(0.30, -0.26), at(0.72, 0.02), (k + 0.08) * r, (k + 0.06) * r, dark);
        stroke(blip, at(0.30, -0.26), at(0.72, 0.02), k * r, (k - 0.02) * r, BLIP_WHITE);
        return;
    }
    if l.build != Build::Turtle {
        let ear = at(-0.14, 0.08);
        blip.fill_circle(ear.0, ear.1, 0.17 * r, shade(skin, 0.80));
    }

    // The face: brow, eye, mouth, and each one changes with the mood.
    let brow = if l.build == Build::Giant { 0.14 } else { 0.09 };
    if mood == Mood::Hurt {
        // Screwed shut.
        line(at(0.34, -0.24), at(0.66, -0.08), 0.07);
        line(at(0.66, -0.08), at(0.34, 0.08), 0.07);
    } else {
        let eye = at(0.50, -0.08);
        if l.blink && mood == Mood::Calm {
            line(at(0.30, -0.06), at(0.70, -0.06), 0.06);
        } else {
            blip.fill_circle(eye.0, eye.1, 0.22 * r, BLIP_WHITE);
            blip.fill_circle(eye.0 + fw * 0.08 * r, eye.1, 0.12 * r, dark);
        }
        if mood == Mood::Happy { line(at(0.24, -0.44), at(0.80, -0.44), brow); }
        else { line(at(0.20, -0.40), at(0.84, -0.24), brow); }
    }
    match mood {
        Mood::Calm => line(at(0.50, 0.50), at(0.80, 0.48), 0.06),
        Mood::Happy => {
            line(at(0.42, 0.44), at(0.62, 0.56), 0.06);
            line(at(0.62, 0.56), at(0.84, 0.42), 0.06);
        }
        Mood::Shout | Mood::Hurt => {
            let m = at(0.62, 0.50);
            blip.fill_circle(m.0, m.1, 0.17 * r, BlipColor { r: 0.36, g: 0.08, b: 0.10, a: 1.0 });
        }
    }
}

/// A fighter's colours and build as a `Look`, in full light.
pub(crate) fn look_of(a: &Archetype, flash: f32) -> Look {
    Look {
        cloth: rgb(a.color),
        trim: rgb(a.trim),
        skin: rgb(a.skin),
        hair: rgb(a.hair),
        build: a.build,
        bulk: a.bulk * a.size,
        size: a.size,
        far: false,
        tone: 1.0,
        blink: false,
        wind: (0.0, 0.0),
        flash,
    }
}
