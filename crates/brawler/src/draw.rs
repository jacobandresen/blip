//! Drawing. The fighters are posed from the same numbers the rules use.
//!
//! Every limb that matters is placed from `move_data()` — the reach and
//! height a punch is judged by are the reach and height it is *drawn*
//! at. A fighting game where the picture and the hitbox disagree teaches
//! the player to stop believing what they see, and there is nothing left
//! to play after that.

use super::*;

fn rgb(c: (f32, f32, f32)) -> BlipColor { BlipColor { r: c.0, g: c.1, b: c.2, a: 1.0 } }

/// Width of a string in pixels at size `sz`.
///
/// Not to be confused with blip's `text_cx()`, which returns the x that
/// centres a string on the *whole canvas* — useful for a banner, useless
/// for right-aligning a name over a health bar, and silently wrong if
/// mistaken for a width. The font is a fixed 6px cell per character.
fn text_w(text: &str, sz: f32) -> f32 { text.len() as f32 * 6.0 * sz }
fn rgba(c: (f32, f32, f32), a: f32) -> BlipColor { BlipColor { r: c.0, g: c.1, b: c.2, a } }

pub fn draw(blip: &Blip, g: &Game) {
    #[cfg(feature = "gallery")]
    { draw_gallery(blip, g.now); return; }
    #[allow(unreachable_code)]
    match g.state {
        State::Title => draw_title(blip),
        State::Select => draw_select(blip, g),
        _ => {
            draw_stage(blip, g);
            draw_fight(blip, g);
            draw_hud(blip, g);
            draw_banner(blip, g);
        }
    }
}

// ---- locations -----------------------------------------------------------

/// Two places to fight, and they are only allowed to be scenery: the
/// floor is the same flat line in both, at the same height, with the
/// same walls. A stage that changed the fight would make the ladder's
/// difficulty a matter of where you were standing.
fn draw_stage(blip: &Blip, g: &Game) {
    // A hit shakes the whole picture, which is most of what selling an
    // impact means when the fighters themselves are simple shapes.
    let shake = if g.shake > 0.0 { (g.shake * 60.0).sin() * g.shake * 30.0 } else { 0.0 };
    if g.stage == 0 { draw_dock(blip, shake, g.now); } else { draw_temple(blip, shake, g.now); }

    draw_floor(blip, g.stage, shake);
}

/// The ground they are standing on, seen flat-on from in front.
///
/// It used to be a near-black band, which was fine while the fighters
/// had nothing to cast onto it. A shadow needs somewhere to land: a
/// floor darker than the shadow is a floor with no shadows on it, and
/// the fighters read as stickers on a backdrop. So the boards are lit
/// now, and fall off toward the bottom of the screen the way a plane
/// receding from a low sun does.
fn draw_floor(blip: &Blip, stage: usize, shake: f32) {
    let near = if stage == 0 {
        BlipColor { r: 0.38, g: 0.28, b: 0.21, a: 1.0 }
    } else {
        BlipColor { r: 0.26, g: 0.25, b: 0.31, a: 1.0 }
    };
    let h = WIN_H as f32 - FLOOR_Y;
    let bands = 10;
    for i in 0..bands {
        let k = i as f32 / (bands - 1) as f32;
        let f = 1.0 - 0.55 * k;
        blip.fill_rect(0.0, FLOOR_Y + shake + k * h, WIN_W as f32, h / bands as f32 + 1.0,
            shade(near, f));
    }
    // Board seams, converging a little so the plane reads as going away
    // from the viewer rather than standing up behind them.
    let seam = shade(near, 0.62);
    for i in 0..17 {
        let x = i as f32 * 40.0;
        blip.draw_line(x, FLOOR_Y + shake, x + (x - WIN_W as f32 / 2.0) * 0.30,
            WIN_H as f32 + shake, seam);
    }
    // The lip where the floor meets the scenery, catching the light.
    blip.fill_rect(0.0, FLOOR_Y + shake - 1.0, WIN_W as f32, 2.0,
        blend(near, BLIP_WHITE, 0.35));
}

/// THE DOCKS — sunset, water, cranes, crates.
fn draw_dock(blip: &Blip, shake: f32, t: f32) {
    let bands = 12;
    let h = FLOOR_Y / bands as f32;
    for i in 0..bands {
        let k = i as f32 / (bands - 1) as f32;
        let c = BlipColor {
            r: 0.95 - 0.55 * k, g: 0.45 - 0.22 * k, b: 0.30 + 0.18 * k, a: 1.0,
        };
        blip.fill_rect(0.0, i as f32 * h + shake, WIN_W as f32, h + 1.0, c);
    }
    // Sun low over the water.
    blip.fill_circle(500.0, 190.0 + shake, 46.0, BlipColor { r: 1.0, g: 0.85, b: 0.45, a: 0.9 });
    // Cranes: verticals with a jib, far enough back to read as skyline.
    for (x, hgt) in [(90.0f32, 150.0f32), (160.0, 110.0), (560.0, 130.0)] {
        let base = FLOOR_Y - 64.0 + shake;
        let c = BlipColor { r: 0.18, g: 0.12, b: 0.14, a: 1.0 };
        blip.fill_rect(x, base - hgt, 7.0, hgt, c);
        blip.fill_rect(x - 26.0, base - hgt, 70.0, 6.0, c);
        blip.draw_line(x + 3.0, base - hgt + 6.0, x + 40.0, base - hgt + 34.0, c);
    }
    // Water, with a moving glitter line — and then the quay in front of
    // it. The fighters stand on the planks; drawing the sea right up to
    // the floor line put them ankle-deep in it.
    blip.fill_rect(0.0, FLOOR_Y - 64.0 + shake, WIN_W as f32, 36.0,
        BlipColor { r: 0.20, g: 0.26, b: 0.40, a: 1.0 });
    for i in 0..16 {
        let x = ((i as f32 * 53.0) + (t * 18.0) % 53.0) % WIN_W as f32;
        let y = FLOOR_Y - 58.0 + ((i * 7) % 24) as f32 + shake;
        blip.fill_rect(x, y, 18.0, 1.5, BlipColor { r: 1.0, g: 0.8, b: 0.6, a: 0.28 });
    }
    blip.fill_rect(0.0, FLOOR_Y - 28.0 + shake, WIN_W as f32, 28.0,
        BlipColor { r: 0.33, g: 0.24, b: 0.17, a: 1.0 });
    for i in 0..22 {
        let x = i as f32 * 30.0;
        blip.draw_line(x, FLOOR_Y - 28.0 + shake, x, FLOOR_Y + shake,
            BlipColor { r: 0.22, g: 0.15, b: 0.10, a: 1.0 });
    }
    // Crates stacked at the edges — the only thing telling you where the
    // corner is before you are in it.
    for (x, n) in [(2.0f32, 2), (604.0, 3)] {
        for i in 0..n {
            let y = FLOOR_Y - 24.0 * (i + 1) as f32 + shake;
            blip.fill_rect(x, y, 34.0, 24.0, BlipColor { r: 0.42, g: 0.29, b: 0.16, a: 1.0 });
            blip.draw_rect(x, y, 34.0, 24.0, BlipColor { r: 0.22, g: 0.15, b: 0.08, a: 1.0 });
        }
    }
}

/// THE TEMPLE — night, moon, pillars, hanging banners.
fn draw_temple(blip: &Blip, shake: f32, t: f32) {
    blip.fill_rect(0.0, shake, WIN_W as f32, FLOOR_Y,
        BlipColor { r: 0.06, g: 0.07, b: 0.14, a: 1.0 });
    blip.fill_circle(120.0, 96.0 + shake, 34.0, BlipColor { r: 0.90, g: 0.92, b: 0.82, a: 0.95 });
    blip.fill_circle(108.0, 88.0 + shake, 30.0, BlipColor { r: 0.06, g: 0.07, b: 0.14, a: 1.0 });
    for i in 0..40 {
        let x = ((i * 79) % WIN_W as usize) as f32;
        let y = ((i * 43) % 220) as f32 + 10.0 + shake;
        let tw = 0.5 + 0.5 * ((t * 1.7 + i as f32).sin());
        blip.fill_rect(x, y, 1.6, 1.6, BlipColor { r: 0.9, g: 0.95, b: 1.0, a: 0.25 + 0.35 * tw });
    }
    // Pillars, and the roof line they hold up.
    let stone = BlipColor { r: 0.20, g: 0.19, b: 0.24, a: 1.0 };
    let stone_lit = BlipColor { r: 0.30, g: 0.29, b: 0.35, a: 1.0 };
    blip.fill_rect(0.0, 40.0 + shake, WIN_W as f32, 22.0, stone);
    for i in 0..5 {
        let x = 40.0 + i as f32 * 140.0;
        blip.fill_rect(x, 62.0 + shake, 26.0, FLOOR_Y - 62.0, stone);
        blip.fill_rect(x, 62.0 + shake, 6.0, FLOOR_Y - 62.0, stone_lit);
    }
    // Banners that move a little, so the place is not a photograph.
    for i in 0..3 {
        let x = 110.0 + i as f32 * 200.0;
        let sway = (t * 1.1 + i as f32).sin() * 3.0;
        blip.fill_rect(x + sway, 62.0 + shake, 22.0, 120.0,
            BlipColor { r: 0.62, g: 0.12, b: 0.16, a: 1.0 });
        blip.fill_rect(x + 8.0 + sway, 84.0 + shake, 6.0, 60.0,
            BlipColor { r: 0.95, g: 0.85, b: 0.5, a: 0.9 });
    }
}

// ---- anatomy -------------------------------------------------------------
//
// A fighter is drawn as a body, not a symbol. The difference is not
// decoration: a player reads an attack off the *shape* of the person
// throwing it — where the weight is, which leg is loaded, whether the
// shoulder has turned over yet — and none of that survives being drawn
// as a line from the hips to wherever the hitbox ends. A kick has to
// come off a knee that chambered first, or it is not a kick, it is a
// pointer.
//
// Everything below is built from three ideas:
//
// 1. **Pose space.** Joints are written as (forward, up) from the floor
//    under the fighter's feet, so one pose serves both sides of the
//    screen and every number reads as a height off the ground.
// 2. **Two bones per limb.** Hands and feet are placed; knees and
//    elbows are solved. A limb with a joint in it bends the way a body
//    bends, and it is that bend — not the endpoint — that makes a kick
//    look like a leg.
// 3. **Tapered strokes.** Every part is a run of circles from a thick
//    end to a thin one: hip to knee to ankle, deltoid to wrist. Uniform
//    width is what makes a stick figure a stick figure.

/// A point on a fighter: `f` forward — the way they face — and `u` up
/// from the floor under them.
#[derive(Copy, Clone)]
struct P { f: f32, u: f32 }
const fn p(f: f32, u: f32) -> P { P { f, u } }

impl P {
    /// Part of the way from here to there. Animation is almost entirely
    /// this, which is why it is the only operator a pose needs.
    fn to(self, o: P, k: f32) -> P { p(self.f + (o.f - self.f) * k, self.u + (o.u - self.u) * k) }
}

/// A point on screen.
#[derive(Copy, Clone, Debug)]
pub(crate) struct V(pub f32, pub f32);

fn dist(a: V, b: V) -> f32 { ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt() }
fn along(a: V, b: V, k: f32) -> V { V(a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k) }

/// Where a fighter's pose space lands on screen.
#[derive(Copy, Clone)]
pub(crate) struct Rig {
    cx: f32,
    ground: f32,
    /// Which way "forward" points in pose space.
    fwd: f32,
    /// Which way the face points. The same as `fwd` on a fighter who is
    /// upright, and the opposite on one who has been put on their back.
    face: f32,
    /// How much of a joint's height survives the mapping, and which way
    /// that height then runs on screen.
    ///
    /// One for the fighter: height goes *up*. Negative for their
    /// shadow, because the floor is the plane between them and the
    /// viewer — the light in both stages is behind the fighters, so
    /// their shadows run toward the camera, down the screen, not up the
    /// scenery behind them.
    squash: f32,
    /// How far a joint slides sideways per unit of height — the angle
    /// of the light. Zero for the fighter.
    skew: f32,
    /// Height of the fighter's feet above the floor, added to every
    /// joint before projecting. Zero for the fighter, since their pose
    /// is already measured from their own feet; for the shadow it is
    /// what makes a jump throw its shadow further away and smaller.
    rise: f32,
}

impl Rig {
    fn at(self, q: P) -> V {
        let u = q.u + self.rise;
        V(self.cx + self.fwd * q.f + u * self.skew, self.ground - u * self.squash)
    }
    pub(crate) fn upright(cx: f32, ground: f32, fwd: f32, face: f32) -> Rig {
        Rig { cx, ground, fwd, face, squash: 1.0, skew: 0.0, rise: 0.0 }
    }
    /// How long a bone is once this rig has had it. One upright; less
    /// for a shadow, which is the same body seen at a glancing angle.
    fn bone(self) -> f32 { self.squash.abs().max(0.45) }
}

/// Bone lengths, in the same pixels as everything else. The legs add up
/// to more than the height of the hips on purpose: a fighter stands
/// with their knees bent, and a fighter who does not is standing in a
/// queue.
// Checked against figure-drawing proportions rather than guessed: the
// femur is a little longer than the tibia, not a little shorter, and
// the two together put the hip joint at half the standing height.
const THIGH: f32 = 30.0;
const SHIN: f32 = 29.0;
const UARM: f32 = 24.0;
const FARM: f32 = 21.0;

/// Joint heights for a fighter standing, and for one crouching. The
/// crouch numbers put the top of the head at `CROUCH_H`, because the
/// crouch that ducks a jab is the crouch you can see.
// The canon: a figure is seven to seven and a half heads tall, the hip
// joint sits at half of that, and trunk plus head make up four heads.
// At a hundred and twenty pixels that is a sixteen-pixel head, hips at
// fifty-eight, and shoulders at ninety-five.
const HIP_U: f32 = 58.0;
const NECK_U: f32 = 95.0;
const HEAD_U: f32 = 109.0;
const C_HIP: f32 = 31.0;
const C_NECK: f32 = 53.0;
const C_HEAD: f32 = 64.0;

fn shade(c: BlipColor, k: f32) -> BlipColor {
    BlipColor { r: c.r * k, g: c.g * k, b: c.b * k, a: c.a }
}

fn blend(a: BlipColor, b: BlipColor, k: f32) -> BlipColor {
    BlipColor {
        r: a.r + (b.r - a.r) * k,
        g: a.g + (b.g - a.g) * k,
        b: a.b + (b.b - a.b) * k,
        a: a.a + (b.a - a.a) * k,
    }
}

/// The line around everything. A fighter without one dissolves into
/// whichever stage they are standing in front of the moment the two
/// happen to be a similar colour.
const INK: BlipColor = BlipColor { r: 0.07, g: 0.06, b: 0.09, a: 1.0 };

/// What the lit floor looks like with a fighter standing in the way of
/// the light. Opaque, and a little cooler than the boards it falls on —
/// a shadow takes its colour from the sky, not from the thing casting it.
const SHADOW: BlipColor = BlipColor { r: 0.17, g: 0.14, b: 0.16, a: 1.0 };

/// How a fighter is coloured and clothed, already shaded for the limb
/// being drawn.
#[derive(Copy, Clone)]
struct Hide {
    cloth: BlipColor,
    trim: BlipColor,
    skin: BlipColor,
    hair: BlipColor,
    build: Build,
    bulk: f32,
    /// 1.0 for a near limb, less for a far one. Depth, on a flat
    /// picture, is mostly just this.
    tone: f32,
    /// Blown toward white for the frames a hit is frozen on.
    flash: f32,
    /// Whether this limb is in front of the body and needs an edge
    /// cutting round it. A white sleeve crossing a white jacket has no
    /// silhouette of its own: the ordinary hairline outline vanishes
    /// into the fill behind it and the arm stops existing. Near limbs
    /// get a heavier line, which is also true of how we see — the
    /// nearer edge is the sharper one.
    front: bool,
}

impl Hide {
    fn c(&self, base: BlipColor) -> BlipColor {
        blend(shade(base, self.tone), BLIP_WHITE, self.flash)
    }
    /// The outline barely darkens with distance. An edge that fades
    /// with the limb it belongs to stops separating it from the limb in
    /// front, which is the entire job of having one.
    fn ink(&self) -> BlipColor { blend(shade(INK, 0.8 + 0.2 * self.tone), BLIP_WHITE, self.flash * 0.6) }
    /// The far side of the body. Darker, and pulled toward the colour
    /// of the shadows rather than simply dimmed — distance takes the
    /// warmth out of a colour as well as the light, and a far limb that
    /// is only a darker version of the near one still reads as the same
    /// limb drawn twice.
    fn far(self) -> Hide {
        let cool = |c: BlipColor| blend(shade(c, 0.58), SHADOW, 0.22);
        Hide {
            cloth: cool(self.cloth),
            trim: cool(self.trim),
            skin: cool(self.skin),
            hair: cool(self.hair),
            front: false,
            ..self
        }
    }
    fn infront(self) -> Hide { Hide { front: true, ..self } }

    /// Cut this limb out of whatever is behind it.
    ///
    /// Each node carries its own radius, because a limb tapers: one
    /// radius for the whole arm draws a shoulder-width line around the
    /// wrist, and the fist comes out wearing a black mitten.
    fn cut(&self, blip: &Blip, nodes: &[(V, f32)]) {
        if !self.front { return; }
        let ink = blend(INK, BLIP_WHITE, self.flash * 0.5);
        for w in nodes.windows(2) {
            stroke(blip, w[0].0, w[1].0, w[0].1, w[1].1, ink);
        }
    }
    /// The belt. Black over a gi, the fighter's own colour otherwise —
    /// a red belt on a white jacket is a cummerbund.
    fn belt(&self) -> BlipColor {
        match self.build {
            Build::Gi => self.c(BlipColor { r: 0.13, g: 0.12, b: 0.15, a: 1.0 }),
            _ => self.c(self.trim),
        }
    }
}

/// A tapered capsule — the only primitive a body is made of.
fn stroke(blip: &Blip, a: V, b: V, r1: f32, r2: f32, c: BlipColor) {
    let d = dist(a, b);
    let n = (d / 1.7).ceil().max(1.0);
    let steps = n as i32;
    for i in 0..=steps {
        let t = i as f32 / n;
        blip.fill_circle(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, r1 + (r2 - r1) * t, c);
    }
}

/// The same shape, drawn once fatter in ink and once in colour, so the
/// part has an edge. Overlapping parts then read as one in front of the
/// other rather than as a single blob of the same colour.
fn part(blip: &Blip, a: V, b: V, r1: f32, r2: f32, c: BlipColor, ink: BlipColor) {
    stroke(blip, a, b, r1 + 1.7, r2 + 1.7, ink);
    stroke(blip, a, b, r1, r2, c);
    // Light along one side of the limb and shadow along the other,
    // both *across* it rather than down its length. This is the whole
    // difference between a cylinder and a flat sausage — and on a
    // fighter dressed in white it is also the only thing keeping an arm
    // from dissolving into the chest behind it.
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let d = (dx * dx + dy * dy).sqrt().max(0.001);
    // Perpendicular, pointed up-screen: the light is always overhead.
    let (mut nx, mut ny) = (-dy / d, dx / d);
    if ny > 0.0 { nx = -nx; ny = -ny; }
    let lit = blend(c, BLIP_WHITE, 0.20);
    let dark = shade(c, 0.74);
    stroke(blip, V(a.0 - nx * r1 * 0.46, a.1 - ny * r1 * 0.46),
                 V(b.0 - nx * r2 * 0.46, b.1 - ny * r2 * 0.46), r1 * 0.44, r2 * 0.44, dark);
    stroke(blip, V(a.0 + nx * r1 * 0.42, a.1 + ny * r1 * 0.42),
                 V(b.0 + nx * r2 * 0.42, b.1 + ny * r2 * 0.42), r1 * 0.40, r2 * 0.40, lit);
}

fn blob(blip: &Blip, at: V, r: f32, c: BlipColor, ink: BlipColor) {
    blip.fill_circle(at.0, at.1, r + 1.7, ink);
    blip.fill_circle(at.0, at.1, r, c);
    blip.fill_circle(at.0 - r * 0.28, at.1 - r * 0.32, r * 0.5, blend(c, BLIP_WHITE, 0.18));
}

/// How far a two-bone limb can fold.
///
/// A knee closes to about thirty degrees between thigh and calf before
/// the calf is against the hamstring; an elbow to about thirty-five.
/// Past that a real limb stops, and a drawn one that does not stop puts
/// the joint somewhere a joint cannot be.
const KNEE_SHUT: f32 = 0.52;  // ~30 degrees, in radians
const ELBOW_SHUT: f32 = 0.61; // ~35 degrees

/// The two-bone solve: where the joint goes, and where the end of the
/// limb actually ends up.
///
/// Bones do not change length — not to reach a hitbox, not for
/// anything. An earlier version of this stretched them to meet whatever
/// the pose asked for, which is how the fighter ended up standing on a
/// support leg a fifth longer than the leg it was kicking with. So the
/// target is a *request*: it is clamped into the ring the limb can
/// actually reach, between fully folded and fully straight, and the
/// caller is handed back the point it settled on.
///
/// `toward` picks which way the joint breaks — knees forward, elbows
/// back and down — and getting it wrong is the difference between a
/// fighter and something with its legs on backwards.
fn solve(root: V, want: V, l1: f32, l2: f32, shut: f32, toward: V, floor: f32) -> (V, V) {
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
    // There are exactly two solutions and they are mirror images about
    // the line between the ends, so the only real decision is which way
    // the joint breaks — and `toward` makes it, always.
    //
    // An earlier version overrode this to keep joints out of the
    // floorboards, and since the two candidates are always on opposite
    // sides of that line, "not underground" sometimes meant "knee
    // bending backwards". A leg on backwards is worse than a knee an
    // inch into the boards, and the real fix is not to ask for either:
    // a pose that puts a knee underground is a pose to correct, and
    // there is a test that says which ones do.
    let _ = floor;
    let score = |c: V| (c.0 - root.0) * toward.0 + (c.1 - root.1) * toward.1;
    (if score(one) >= score(two) { one } else { two }, end)
}

// ---- cloth ---------------------------------------------------------------

/// Where a hanging piece of cloth ends up.
///
/// Not a simulation with state, but not a decoration either: a closed
/// form of the two forces that actually shape a hanging panel. Gravity
/// pulls it along `hang`, and its own inertia leaves it behind whatever
/// it is pinned to — each node further from the pin lagging more than
/// the last, because it has more cloth between it and the anchor to
/// take up the slack.
///
/// That second term is the whole point. A gi that keeps its shape
/// through a roundhouse is painted on; one that snaps out flat behind
/// the leg and takes a moment to fall back is a garment with a body
/// inside it. The lag is quadratic in the distance along the panel,
/// which is what gives the trailing edge its whip.
///
/// Every segment is renormalised to its own length afterwards, so the
/// cloth swings but never stretches — the same rule the bones live by.
fn drape(anchor: V, hang: (f32, f32), len: f32, vel: V, t: f32, phase: f32) -> [V; 4] {
    let mut out = [anchor; 4];
    let seg = len / 3.0;
    let (px, py) = (-hang.1, hang.0); // across the panel
    for i in 1..4 {
        let k = i as f32 / 3.0;
        // Inertia: the far edge is still where the body used to be.
        let lag = k * k * 0.85;
        // And a little life of its own, so a standing fighter's gi is
        // not a plank.
        let flutter = (t * 6.5 + phase + k * 2.6).sin() * 1.5 * k;
        let prev = out[i - 1];
        let want = V(prev.0 + hang.0 * seg - vel.0 * lag + px * flutter,
                     prev.1 + hang.1 * seg - vel.1 * lag + py * flutter);
        let d = dist(prev, want).max(0.001);
        out[i] = V(prev.0 + (want.0 - prev.0) / d * seg, prev.1 + (want.1 - prev.1) / d * seg);
    }
    out
}

/// Draw a draped panel as a tapered run through its nodes.
fn panel(blip: &Blip, nodes: &[V; 4], r1: f32, r2: f32, c: BlipColor, ink: BlipColor) {
    for i in 0..3 {
        let a = i as f32 / 3.0;
        let b = (i + 1) as f32 / 3.0;
        part(blip, nodes[i], nodes[i + 1], r1 + (r2 - r1) * a, r1 + (r2 - r1) * b, c, ink);
    }
}

// ---- parts ---------------------------------------------------------------

fn draw_leg(blip: &Blip, h: &Hide, hip: V, knee: V, ankle: V, fwd: f32, ground: f32,
            vel: V, t: f32) {
    let b = h.bulk;
    let ink = h.ink();
    let skin = h.c(h.skin);
    let cloth = h.c(h.cloth);

    h.cut(blip, &[(hip, 10.6 * h.bulk), (knee, 8.8 * h.bulk),
                  (ankle, if h.build == Build::Bare { 6.4 } else { 8.0 } * h.bulk)]);

    // The leg itself: thigh, calf, ankle. The calf is drawn as its own
    // short run so it can bulge — a leg that thins evenly from hip to
    // toe is a cone.
    // A leg, in the proportions a leg has: the thigh is the thickest
    // part of a person's limb and it narrows by a third before the
    // knee, the knee itself is bone and narrow, the calf swells again
    // just below it, and the ankle is barely wider than the bone.
    // Drawn at anything like an even width the whole thing is a pipe.
    part(blip, hip, knee, 8.4 * b, 4.8 * b, skin, ink);
    let calf = along(knee, ankle, 0.26);
    part(blip, knee, calf, 4.8 * b, 5.4 * b, skin, ink);
    part(blip, calf, ankle, 5.4 * b, 2.5 * b, skin, ink);

    // What they are wearing over it.
    //
    // A gi covers the leg. The trousers used to stop halfway down the
    // shin, leaving the lower half of every leg a bare tube — the
    // fighter looked undressed from the knee down. They run to the
    // ankle now.
    //
    // But the cloth on the leg *follows the leg*. There is a bone under
    // it and a bone is stiff: a thigh is one straight thing, a shin is
    // one straight thing, and they meet at a knee that bends. Drawing
    // the trouser as a hanging chain over the shin — which is what a
    // simulated panel does — put a curve in the middle of the shinbone
    // and made the leg look boneless. So the cloth on the bone is drawn
    // on the bone, and only what hangs *past* it is free to swing.
    let (hem, boot) = match h.build {
        Build::Gi => (0.86, 0.0),
        Build::Bare => (0.30, 0.60),
        Build::Suit => (0.94, 0.0),
    };
    let hem_at = along(knee, ankle, hem);
    // The trouser follows the leg under it and narrows with it. Cut
    // straight, it made both legs into columns and undid the taper the
    // leg had just been given.
    part(blip, hip, knee, 9.6 * b, 6.4 * b, cloth, ink);
    part(blip, knee, hem_at, 6.4 * b, if h.build == Build::Gi { 6.0 } else { 5.0 } * b,
        cloth, ink);
    if h.build == Build::Gi {
        // The loose hem, and only the hem: a short flare hanging off
        // the end of the trouser leg that trails when the leg swings
        // out and falls back over the ankle when it stops.
        let (dx, dy) = unit(knee, ankle);
        let nodes = drape(hem_at, (dx, dy), 9.0, vel, t, ankle.0 * 0.08);
        panel(blip, &nodes, 6.2 * b, 4.6 * b, cloth, ink);
    }
    // A fold line down the front of the leg, so a white trouser against
    // a white jacket is still two garments.
    if h.build != Build::Bare {
        stroke(blip, along(hip, knee, 0.35), along(knee, hem_at, 0.7), 1.2, 1.0,
            shade(cloth, 0.78));
    }
    if boot > 0.0 {
        part(blip, along(knee, ankle, boot), ankle, 4.4 * b, 3.2 * b, h.c(h.trim), ink);
    }
    if h.build == Build::Suit {
        // Wrapped shins: a band at the hem, which is where the eye
        // looks for a joint between cloth and skin.
        stroke(blip, along(knee, ankle, hem - 0.12), hem_at, 6.4 * b, 6.2 * b, h.c(h.trim));
    }

    // The foot.
    //
    // It was one tapered stroke, and one stroke is a shoe-shaped blob:
    // no ankle, no heel, no toes. A foot is four things and they are
    // all visible from the side even at this size — the ankle is
    // narrow, the heel sticks out *behind* the leg, the instep arches
    // forward and down from the ankle, and the toes are a separate,
    // thinner piece past the ball. Leave any of them out and the leg
    // ends in a wedge.
    //
    // Planted, the foot lies along the floor; off the ground it points
    // along the shin, which is what makes a kick land with a foot
    // rather than with the end of a line.
    let flat = ankle.1 > ground - 7.0;
    let (foot_c, toe_c) = match h.build {
        // Boot leather, not boot polish. Painted in the fighter's full
        // accent the two feet were one bright lump with no edge between
        // them — the accent belongs on the belt and the headband, where
        // there is only one of it.
        Build::Bare => (shade(h.c(h.trim), 0.72), shade(h.c(h.trim), 0.62)),
        _ => (skin, skin),
    };
    let boot = h.build == Build::Bare;
    // Direction the foot points, and the one square to it (its "up").
    let (fx, fy, ux, uy) = if flat {
        (fwd, 0.0, 0.0, -1.0)
    } else {
        let d = dist(knee, ankle).max(0.001);
        let (dx, dy) = ((ankle.0 - knee.0) / d, (ankle.1 - knee.1) / d);
        (dx, dy, dy, -dx)
    };
    // Ankle: the narrowest part of the whole leg, and the reason the
    // calf above it reads as a calf.
    let ank = if flat { V(ankle.0, ground - 6.5 * b) } else { ankle };
    let at = |along: f32, up: f32| V(ank.0 + fx * along + ux * up, ank.1 + fy * along + uy * up);
    // A foot is about a seventh of a person long, and it is longer in
    // front of the ankle than behind it.
    let fb = if boot { b * 0.92 } else { b };
    let heel = at(-4.2 * fb, -3.0 * fb);
    let ball = at(6.2 * fb, -4.8 * fb);
    let tip = at(10.6 * fb, -4.4 * fb);
    // The sole, heel to ball, and the instep arching down onto it from
    // the ankle. Two strokes, drawn as one piece — an earlier version
    // put a separate blob on the heel and two nicks in the toes, and at
    // this size that is not detail, it is a handful of pebbles where
    // the foot should be.
    part(blip, heel, ball, 3.2 * fb, 2.5 * fb, foot_c, ink);
    part(blip, at(-1.2 * fb, -0.5 * fb), ball, 3.4 * fb, 2.5 * fb, foot_c, ink);
    // Toes: past the ball, thinner, and tapering to nothing.
    part(blip, ball, tip, if boot { 2.5 } else { 2.3 } * b, 1.3 * b, toe_c, ink);
    if !boot {
        // One crease where the toes leave the ball. One is detail; two
        // is a rendering of a foot with the toes counted.
        let p = along(ball, tip, 0.3);
        stroke(blip, V(p.0 + ux * 1.7 * b, p.1 + uy * 1.7 * b), p, 0.7, 0.7, shade(toe_c, 0.62));
    }
}

fn draw_arm(blip: &Blip, h: &Hide, shoulder: V, elbow: V, hand: V, open: bool, vel: V, t: f32) {
    let b = h.bulk;
    let ink = h.ink();
    let skin = h.c(h.skin);
    let cloth = h.c(h.cloth);

    // Cut the whole arm out of the body behind it first, in one piece,
    // so the separation follows the arm rather than each segment of it.
    let (ax0, ay0) = unit(shoulder, elbow);
    let b0 = h.bulk;
    let sleeve = if h.build == Build::Bare { 6.6 } else { 7.4 };
    h.cut(blip, &[
        (V(shoulder.0 - ax0 * 3.0, shoulder.1 - ay0 * 3.0), (sleeve + 1.6) * b0),
        (elbow, 6.4 * b0),
        (hand, 6.0 * b0),
    ]);

    // The deltoid: a cap of muscle sitting over the top of the joint,
    // drawn before the arm and overlapping the torso.
    //
    // Without it the upper arm is a tube that begins in mid-air beside
    // the chest, and the whole limb reads as stuck on rather than
    // grown from. The shoulder is the one joint where the limb and the
    // body are the same piece of flesh, and the drawing has to say so.
    let (ax, ay) = unit(shoulder, elbow);
    blob(blip, V(shoulder.0 - ax * 1.5, shoulder.1 - ay * 1.5), 6.6 * b, skin, ink);

    // Same again for the arm: the biceps is the thick part, the elbow
    // is bone, the forearm swells just below it and runs down to a
    // wrist barely wider than the bone in it.
    part(blip, shoulder, elbow, 6.4 * b, 4.2 * b, skin, ink);
    let brawn = along(elbow, hand, 0.28);
    part(blip, elbow, brawn, 4.2 * b, 4.6 * b, skin, ink);
    part(blip, brawn, hand, 4.6 * b, 2.9 * b, skin, ink);
    // The sleeve. A gi sleeve is cut wide and reaches past the elbow —
    // these fighters are dressed, and an arm that is a bare tube from
    // the shoulder down looks it. The cuff hangs off the end rather
    // than gripping the arm, so it swings when the arm does.
    match h.build {
        Build::Gi => {
            part(blip, V(shoulder.0 - ax * 2.0, shoulder.1 - ay * 2.0),
                 along(shoulder, elbow, 0.86), 7.0 * b, 5.0 * b, cloth, ink);
            // The cuff, and only the cuff, hangs free — the sleeve
            // above it is stretched over a stiff upper arm.
            let cuff = along(shoulder, elbow, 0.86);
            let (dx, dy) = unit(shoulder, elbow);
            let nodes = drape(cuff, (dx * 0.5, dy * 0.5 + 0.7), 8.0, vel, t, shoulder.0 * 0.1);
            panel(blip, &nodes, 6.0 * b, 4.2 * b, cloth, ink);
            // A seam down the sleeve, which is what stops a white
            // sleeve on a white jacket being one shape.
            stroke(blip, along(shoulder, elbow, 0.2), along(shoulder, elbow, 0.8),
                1.1, 0.9, shade(cloth, 0.78));
        }
        Build::Suit => {
            // A one-piece suit covers the whole arm to the wrist.
            part(blip, V(shoulder.0 - ax * 2.0, shoulder.1 - ay * 2.0), elbow,
                 7.4 * b, 5.2 * b, cloth, ink);
            part(blip, elbow, along(elbow, hand, 0.72), 5.2 * b, 4.0 * b, cloth, ink);
        }
        // Bare arms are the point of being bare — but the shoulder
        // still gets a strap so the torso and the arm are not one
        // uninterrupted field of skin.
        Build::Bare => {
            stroke(blip, V(shoulder.0 - ax * 3.0, shoulder.1 - ay * 3.0),
                along(shoulder, elbow, 0.22), 6.4 * b, 5.6 * b, shade(skin, 0.82));
        }
    }

    // Wrist wrap, then the fist on the end of it.
    stroke(blip, along(elbow, hand, 0.74), along(elbow, hand, 0.92), 3.4 * b, 3.7 * b, h.c(h.trim));
    let (fx, fy) = unit(elbow, hand);
    if open {
        // A palm: flatter and longer than a fist, because a grab and a
        // punch are not the same thing and the picture should say which.
        part(blip, hand, V(hand.0 + fx * 6.0, hand.1 + fy * 6.0), 4.0 * b, 3.2 * b, skin, ink);
    } else {
        blob(blip, hand, 5.0 * b, skin, ink);
        // One knuckle line, which is all it takes at this size.
        stroke(blip, V(hand.0 + fx * 3.4 - fy * 2.6, hand.1 + fy * 3.4 + fx * 2.6),
               V(hand.0 + fx * 3.4 + fy * 2.6, hand.1 + fy * 3.4 - fx * 2.6),
               1.1, 1.1, shade(skin, 0.7));
    }
}

/// The pelvis: a short bar between the two hip sockets. Without it the
/// thighs appear to sprout from the same point, and a fighter whose
/// legs share one origin walks like a pair of compasses.
fn draw_pelvis(blip: &Blip, h: &Hide, a: V, b: V) {
    let body = if h.build == Build::Bare { h.c(h.skin) } else { h.c(h.cloth) };
    part(blip, a, b, 9.0 * h.bulk, 9.0 * h.bulk, body, h.ink());
}

fn draw_torso(blip: &Blip, h: &Hide, hip: V, neck: V, vel: V, t: f32) {
    let b = h.bulk;
    let ink = h.ink();
    let bare = h.build == Build::Bare;
    let body = if bare { h.c(h.skin) } else { h.c(h.cloth) };
    let waist = along(hip, neck, 0.30);
    let chest = along(hip, neck, 0.76);

    // The trunk is a ribcage, not a pair of shoulders.
    //
    // Widening this capsule to a shoulder's width made the whole torso
    // that wide — a barrel from armpit to hip that swallowed both arms
    // and the neck with them. Shoulder width is ribcage *plus two
    // deltoids*, and the deltoids are part of the arms; they are drawn
    // there, outside this, which is where they are on a person.
    //
    // The top stops short of the neck joint so there is a neck to see.
    part(blip, hip, waist, 9.6 * b, 8.2 * b, body, ink);
    part(blip, waist, chest, 8.2 * b, 11.4 * b, body, ink);
    part(blip, chest, along(chest, neck, 0.72), 11.4 * b, 6.0 * b, body, ink);

    // What is worn over the chest.
    match h.build {
        Build::Gi => {
            // Open jacket: bare sternum down the middle, with the two
            // lapels crossing over it to the belt. This is the one
            // marking that says karate rather than pyjamas.
            let v = along(chest, neck, 0.25);
            let low = along(waist, chest, 0.5);
            stroke(blip, v, low, 4.2 * b, 1.8 * b, h.c(h.skin));
            part(blip, along(chest, neck, 0.55), low, 2.4, 1.8, shade(body, 0.70), shade(body, 0.52));
        }
        Build::Bare => {
            // Collarbone, pectoral crease and a stomach line. Three
            // marks, and a bare chest stops being a slab of colour.
            let dark = shade(body, 0.70);
            stroke(blip, along(chest, neck, 0.15), along(chest, neck, 0.55), 1.8, 1.4, dark);
            stroke(blip, along(waist, chest, 0.68), along(chest, neck, 0.2), 1.9, 1.5, dark);
            stroke(blip, along(waist, chest, 0.25), along(waist, chest, 0.62), 1.5, 1.2, shade(body, 0.80));
        }
        Build::Suit => {
            stroke(blip, along(waist, chest, 0.2), along(chest, neck, 0.4), 2.2, 1.8, h.c(h.trim));
        }
    }

    let belt_a = along(hip, neck, 0.16);
    let belt_b = along(hip, neck, 0.28);

    // The jacket below the belt: two panels hanging off the waist,
    // front and back. They are most of a gi's silhouette and all of its
    // movement — the thing that flares when the hips turn.
    if h.build != Build::Bare {
        let waist = along(belt_a, belt_b, 0.4);
        for (side, len, phase) in [(1.0f32, 21.0f32, 0.0f32), (-1.0, 18.0, 2.1)] {
            let root = V(waist.0 + side * 6.5 * b, waist.1 + 1.0);
            let nodes = drape(root, (side * 0.18, 1.0), len, vel, t, phase);
            panel(blip, &nodes, 5.2 * b, 3.4 * b, body, ink);
        }
    }

    part(blip, belt_a, belt_b, 9.0 * b, 8.6 * b, h.belt(), ink);
    // The knot, and the two ends hanging off it. They hang, swing and
    // settle on their own: the one part of a fighter that is still
    // moving after the fighter has stopped, which is the cheapest thing
    // on screen that says there is gravity in this picture.
    let knot = V(along(belt_a, belt_b, 0.5).0 + (neck.0 - hip.0).signum() * 2.0,
                 along(belt_a, belt_b, 0.5).1);
    for (dx, len, phase) in [(-2.4f32, 15.0f32, 0.7f32), (2.4, 12.0, 3.3)] {
        let nodes = drape(V(knot.0 + dx, knot.1), (0.0, 1.0), len, vel, t, phase);
        panel(blip, &nodes, 2.3 * b, 1.5 * b, h.belt(), ink);
    }
}

fn draw_head(blip: &Blip, h: &Hide, rig: Rig, head: V, neck: V, t: f32) {
    let fw = rig.face;
    let ink = h.ink();
    let skin = h.c(h.skin);
    let hair = h.c(h.hair);
    // A heavyweight gets a bigger head as well as bigger arms. Scaling
    // only the limbs with `bulk` builds a bodybuilder with a pin for a
    // skull, which reads as a toy rather than as a large man.
    let r = 7.6 + 2.2 * (h.bulk - 1.0);

    // Neck first, so the head sits on it rather than beside it.
    // The neck, drawn long enough to be a neck. A head sitting
    // straight on the shoulders is a head on a snowman.
    part(blip, neck, V(head.0 - fw * 1.3, head.1 + 5.2), 4.4, 3.8, skin, ink);

    // Skull, then the jaw hung off the front of it.
    blob(blip, head, r, skin, ink);
    part(blip, V(head.0 + fw * 0.4, head.1 + 1.0), V(head.0 + fw * 3.8, head.1 + 4.6), 5.0, 3.3, skin, ink);
    blip.fill_circle(head.0 - fw * 2.5, head.1 + 1.0, 1.7, shade(skin, 0.78));

    // Hair, as a handful of overlapping lumps around the back and top.
    for (dx, dy, rr) in [(-3.4f32, -5.4f32, 4.5f32), (-5.9, -2.0, 4.0), (0.6, -6.6, 4.2),
                         (4.0, -5.3, 3.2), (-6.8, 1.6, 2.9)] {
        blip.fill_circle(head.0 + fw * dx, head.1 + dy, rr, hair);
    }

    // Headband, and two ties trailing behind it. The ties are the
    // cheapest motion on the whole fighter and do more for "this is
    // alive" than anything else here.
    let band_a = V(head.0 - fw * 6.3, head.1 - 1.8);
    let band_b = V(head.0 + fw * 5.8, head.1 - 2.9);
    stroke(blip, band_a, band_b, 2.1, 1.9, h.c(h.trim));
    let sway = (t * 5.5).sin() * 3.0;
    stroke(blip, band_a, V(head.0 - fw * 18.0, head.1 + 1.0 + sway), 2.0, 1.0, h.c(h.trim));
    stroke(blip, band_a, V(head.0 - fw * 14.0, head.1 + 6.4 - sway * 0.7), 1.8, 0.9, h.c(h.trim));

    // A face: brow, eye, nose, mouth. Four marks, and the head stops
    // being a ball.
    stroke(blip, V(head.0 + fw * 1.6, head.1 - 1.7), V(head.0 + fw * 5.2, head.1 - 1.3), 1.1, 0.9, hair);
    blip.fill_circle(head.0 + fw * 3.6, head.1 + 0.4, 1.4, BLIP_WHITE);
    blip.fill_circle(head.0 + fw * 4.2, head.1 + 0.5, 0.9, INK);
    blip.fill_circle(head.0 + fw * 5.9, head.1 + 1.3, 1.6, skin);
    stroke(blip, V(head.0 + fw * 3.9, head.1 + 3.8), V(head.0 + fw * 5.6, head.1 + 3.5), 0.9, 0.8,
        shade(skin, 0.5));
}

// ---- poses ---------------------------------------------------------------

/// Every joint that is placed rather than solved.
#[derive(Copy, Clone)]
pub(crate) struct Pose {
    hip: P,
    neck: P,
    head: P,
    lead_hand: P,
    rear_hand: P,
    /// The near foot. Attacks with a leg are always thrown with this
    /// one, because the near leg is the one drawn in front of the body.
    lead_foot: P,
    rear_foot: P,
    /// Hands open — a grab, or a palm — rather than closed into fists.
    open: bool,
}

impl Pose {
    fn to(self, o: Pose, k: f32) -> Pose {
        Pose {
            hip: self.hip.to(o.hip, k),
            neck: self.neck.to(o.neck, k),
            head: self.head.to(o.head, k),
            lead_hand: self.lead_hand.to(o.lead_hand, k),
            rear_hand: self.rear_hand.to(o.rear_hand, k),
            lead_foot: self.lead_foot.to(o.lead_foot, k),
            rear_foot: self.rear_foot.to(o.rear_foot, k),
            open: if k < 0.5 { self.open } else { o.open },
        }
    }
}

/// The stance: weight back, knees bent, both hands up, side on.
///
/// This is the frame the whole game departs from and returns to, so it
/// is worth being fussy about. A fighter stands side-on with the lead
/// foot forward and the rear foot turned out; the hands are at chin
/// height, the lead one further out. Everything else below is this,
/// moved.
/// The stance, with `breath` running from -1 to 1 over one slow cycle.
///
/// A fighter waiting is never still. The weight rocks between the feet,
/// the knees give and take it, the shoulders roll against the hips and
/// the guard drifts — all of it small, none of it in phase. Driving
/// every part off one sine wave makes a figure bob like a float on
/// water; offsetting them is what turns it into someone breathing.
fn stance(breath: f32) -> Pose {
    // `breath` is one slow cycle. Everything here is driven off it at a
    // different phase and a different scale, because a figure whose
    // parts all rise and fall together is a float bobbing on water.
    //
    // The bounce runs at twice the breath: a fighter's knees give and
    // take the weight faster than they breathe. The shoulders roll
    // against the hips rather than with them, which is what a torso
    // does when the weight shifts under it. And the guard drifts on its
    // own, slowest of all, because hands held up get heavy.
    let bob = (breath * 2.0).clamp(-1.0, 1.0);
    let roll = -breath;
    Pose {
        hip: p(breath * 1.2, HIP_U - 1.0 + bob * 0.9),
        neck: p(-3.5 + roll * 1.1, NECK_U - 0.6 + bob * 0.7),
        head: p(-1.5 + roll * 1.6, HEAD_U - 0.4 + bob * 0.6),
        // A guard, measured off a photograph of one rather than
        // guessed at: both fists up by the jaw, the upper arms hanging
        // almost straight down and the elbows tucked in at the ribs, so
        // the forearms finish near vertical. That shape is what makes
        // it read as a guard — hands held out in front at chest height,
        // which is what this was, is a man offering to shake hands.
        //
        // The near fist sits a little lower and further forward than
        // the far one, so that two fists can be told from one.
        // Fists just below the jaw and forward of it, not level with
        // it. A real guard puts them beside the face, but a face seen
        // from the side is fifteen pixels wide and a forearm is
        // thirteen — level with the jaw, the guard simply deletes the
        // head. Dropping them a little keeps the shape of a guard and
        // leaves a face to read it on.
        lead_hand: p(22.0 + breath * 1.4, 90.0 + bob * 1.2),
        rear_hand: p(11.0 + breath * 0.8, 94.0 + bob * 0.9),
        // The feet stay planted. Weight moving between them is the
        // point; feet sliding about is a fighter who has lost it.
        lead_foot: p(19.0, 0.0),
        rear_foot: p(-21.0, 0.0),
        open: false,
    }
}

fn crouched(breath: f32) -> Pose {
    Pose {
        hip: p(-2.0, C_HIP + breath * 0.4),
        neck: p(-1.0, C_NECK + breath * 0.4),
        head: p(1.5, C_HEAD + breath * 0.4),
        lead_hand: p(17.0, 43.0 + breath * 0.6),
        rear_hand: p(5.0, 36.0),
        lead_foot: p(18.0, 0.0),
        rear_foot: p(-19.0, 0.0),
        open: false,
    }
}

/// Off the ground: knees come up, arms come in. A jump drawn with the
/// legs left hanging is a fighter who has been lifted rather than one
/// who has jumped.
fn airborne_pose() -> Pose {
    Pose {
        hip: p(-2.0, 50.0),
        neck: p(-5.0, 90.0),
        head: p(-3.0, 101.0),
        lead_hand: p(11.0, 80.0),
        rear_hand: p(-7.0, 86.0),
        lead_foot: p(16.0, 21.0),
        rear_foot: p(-12.0, 27.0),
        open: false,
    }
}

/// Flat on the floor, head away from whoever put them there.
fn floored() -> Pose {
    // Flat out, head away from whoever put them there.
    //
    // Everything is low: a head resting on boards has its centre one
    // head-radius off them, not twenty pixels up. And the limbs are
    // deliberately spread *in height* — near arm on the floor, far arm
    // across the chest, far leg out along the ground, near knee up.
    // Laid at one height they stacked into a heap of white capsules
    // with no readable head, arms or legs in it, which is what a body
    // on the floor must never be: the one frame where the player needs
    // to see at a glance that somebody is down.
    //
    // The raised knee is doing most of the work. It is the only part
    // above the body line, so it is what makes the silhouette read as a
    // person lying down rather than as a dropped bundle.
    Pose {
        hip: p(8.0, 13.0),
        neck: p(-26.0, 12.0),
        head: p(-43.0, 9.5),
        // Both arms lie down the body toward the feet, and neither goes
        // out past the head.
        //
        // The arms are drawn after the head — they are in front of it —
        // so an arm flung out over the skull simply erases the face,
        // and the one part of a downed fighter a player must be able to
        // find is the head. One arm along the side, one bent across the
        // chest: nothing crosses.
        lead_hand: p(30.0, 8.0),
        rear_hand: p(14.0, 24.0),
        // The near knee up. It is the only thing above the body line
        // and it is doing most of the work: without it the silhouette
        // is a horizontal bar, and a horizontal bar is not a person.
        lead_foot: p(14.0, 10.0),
        rear_foot: p(62.0, 8.0),
        open: false,
    }
}

/// How far through its own animation an attack is: out through startup,
/// held while it can hit, snapped back through recovery.
fn extension(f: &Fighter, m: &MoveData) -> f32 {
    let start = m.startup * F;
    let end = start + m.active * F;
    if f.t < start {
        let k = (f.t / start.max(0.0001)).clamp(0.0, 1.0);
        // Anticipation.
        //
        // Nothing a body does starts from rest and goes straight where
        // it is going. A punch draws back before it goes out, a kick
        // settles onto the support foot before the other leg leaves the
        // floor — the coil is what the blow is thrown *from*, and it is
        // also what a defender reads. Without it every attack began at
        // dead stop and accelerated forward, which is how a machine
        // moves, not a person.
        //
        // So the first third of the startup runs slightly *negative*:
        // the pose extrapolates back past the guard, the body loads,
        // and only then does it uncoil. It returns to zero exactly
        // where the drive begins, so nothing jumps.
        const COIL: f32 = 0.28;
        if k < COIL {
            return -0.16 * (std::f32::consts::PI * k / COIL).sin();
        }
        let k = (k - COIL) / (1.0 - COIL);
        // Ease in: the hand accelerates rather than sliding out at a
        // constant rate, which is the whole difference between a punch
        // and an extending pole.
        //
        // Accelerate out of the coil and ease off as the joint locks,
        // which is what a limb thrown to full extension does — it
        // cannot arrive at speed, the knee stops it.
        //
        // It has to reach exactly 1.0 as the startup ends. It used to
        // stop at 0.82 and jump to full on the first active frame: the
        // leg covered the last fifth of a roundhouse in one frame,
        // fifty pixels of foot with no picture in between, on the frame
        // the blow lands. That single discontinuity was most of what
        // made the kick look wrong — the eye never saw the leg arrive,
        // only that it had.
        //
        // Squaring it instead put all the speed at the end and the foot
        // moved half again as fast as a real one can, so it is eased at
        // both ends now.
        k * k * (3.0 - 2.0 * k)
    } else if f.t <= end {
        1.0
    } else {
        let k = ((f.t - end) / (m.recovery * F).max(0.0001)).clamp(0.0, 1.0);
        (1.0 - k * k * 0.9 - k * 0.1).max(0.0)
    }
}

/// Every number a kick is made of, in one place.
///
/// These are the knobs. Pulled out of the pose code so the shape of a
/// kick can be argued about by changing eight numbers and looking at
/// it, rather than by reading an animation routine and guessing which
/// of its constants is the one making the leg look wrong.
struct KickShape {
    /// How far the hips drive forward over the support foot.
    hip_drive: f32,
    /// How much they rise doing it. Up, not down: you push off.
    hip_rise: f32,
    /// How far the shoulders fall back to pay for the hips.
    lean: f32,
    /// The same lean in the air, as a fraction — there is nothing to
    /// counterbalance against up there, and a jump kick thrown lying
    /// back is a fighter falling.
    air_lean: f32,
    /// Where the support foot ends up relative to the hips.
    plant: f32,
    /// How far in front of the hip the chambered knee points.
    knee_lead: f32,
    /// Height of the chambered foot, hanging below that knee.
    chamber: f32,
    chamber_air: f32,
}

const KICK: KickShape = KickShape {
    hip_drive: 11.0,
    hip_rise: 2.0,
    lean: 17.0,
    air_lean: 0.3,
    plant: 5.0,
    // Far enough forward that the solved knee comes up *level with the
    // hip* rather than hanging below it. The knee is not placed, it is
    // solved from the foot, so this number is the only way to raise it.
    knee_lead: 16.0,
    chamber: 31.0,
    chamber_air: 25.0,
};

/// Where a kick is in its two halves, as 0..1 for the chamber and then
/// 1..2 for the extension.
///
/// A kick is not one motion at one speed. The knee comes up fast and
/// then waits — that wait is the frame the defender is reading — and
/// the shin snaps out of it. Driving both halves off the same even ramp
/// gives a leg that unfolds at a constant rate, which is a mechanism
/// opening, not a person kicking.
fn kick_curve(ext: f32) -> f32 {
    const SPLIT: f32 = KICK_SNAP;
    if ext < SPLIT {
        // Up fast, easing out into the held chamber.
        let k = ext / SPLIT;
        1.0 - (1.0 - k) * (1.0 - k)
    } else {
        // Out hard, accelerating all the way to contact.
        let k = (ext - SPLIT) / (1.0 - SPLIT);
        1.0 + k * k
    }
}

/// The pose an attack is in, `ext` of the way through it.
/// How far the foot reaches past the ankle. Poses that aim a kick have
/// to aim the *ankle* short by this much, or the toes end up out past
/// the hitbox and the attack is drawn reaching further than it reaches.
const FOOT: f32 = 10.0;

fn attack_pose(q: &mut Pose, f: &Fighter, m: &MoveData, ext: f32) {
    let tip = p(BODY_W / 2.0 + m.reach, m.height);
    // Where the ankle goes for a kick: the toes carry it the rest.
    let toe_tip = p(tip.f - FOOT, tip.u);

    // What the body does, rather than which button produced it: a
    // fighter's jumping punch and their standing punch are the same
    // shoulder doing the same thing, and Kestrel's special is a kick
    // whatever the move table calls it.
    enum Shape { Punch(bool), CrouchPunch, Kick(bool, f32), Sweep, Throw, Bolt, Rush }
    let shape = match f.mv {
        MoveId::Jab => Shape::Punch(false),
        MoveId::JumpPunch => Shape::Punch(true),
        MoveId::CrouchJab => Shape::CrouchPunch,
        // All three heights are the same kick; the move's own `height`
        // is what aims it, so nothing here has to know about the
        // difference. The high one leans a shade further back, because
        // a kick at head height needs more counterweight.
        MoveId::LowKick => Shape::Kick(false, 0.7),
        MoveId::Kick => Shape::Kick(false, 1.0),
        MoveId::HighKick => Shape::Kick(false, 1.25),
        MoveId::JumpKick => Shape::Kick(true, 1.0),
        MoveId::Sweep => Shape::Sweep,
        MoveId::Throw => Shape::Throw,
        MoveId::Special => match f.arch().special {
            Special::ChiBolt => Shape::Bolt,
            Special::BullRush => Shape::Rush,
            // A rising kick: the same roundhouse, thrown higher and
            // with more of the body going up behind it.
            Special::TalonKick => Shape::Kick(false, 1.25),
        },
    };

    match shape {
        // A straight lead punch. The shoulder turns over and the whole
        // body steps into it — which is also the only way the fist
        // reaches as far as the hitbox says it does.
        Shape::Punch(air) => {
            q.hip.f += 5.0 * ext;
            q.neck.f += if air { 10.0 } else { 18.0 } * ext;
            q.head.f += if air { 8.0 } else { 15.0 } * ext;
            if !air {
                q.lead_foot.f += 5.0 * ext;
                q.rear_foot.f -= 3.0 * ext;
            }
            q.rear_hand = q.rear_hand.to(p(6.0, q.rear_hand.u + 6.0), ext);
            q.lead_hand = q.lead_hand.to(tip, ext);
        }
        Shape::CrouchPunch => {
            q.neck.f += 15.0 * ext;
            q.head.f += 11.0 * ext;
            q.lead_hand = q.lead_hand.to(tip, ext);
            q.rear_hand.f += 3.0 * ext;
        }
        // The roundhouse. Knee up first, then the shin unfolds into the
        // target while the torso falls back as a counterweight and the
        // arms swing across. Without the chamber the leg sweeps out
        // from the hip in one piece, and a leg that does that is not a
        // leg.
        // A roundhouse. Three sprite sheets were laid side by side for
        // this — Ryu and Ken from the arcade original and Chun-Li from
        // Super — and where they agree is what is drawn here.
        //
        // All three: the support leg goes dead straight and near
        // vertical with the foot under the hips, the hips rise slightly
        // rather than dropping, the chamber puts the knee at hip height
        // with the shin hanging *down* off it, the extended leg keeps a
        // little bend at the knee, and both arms stay in tight — one
        // across the chest, one at the chin. Where they disagree is how
        // far the torso falls back: Ryu commits about forty degrees,
        // Ken twenty-five, Chun-Li barely twenty on the mid kick. The
        // numbers below take the middle of that, which is what stops
        // the pose looking either like a falling man or like a kick
        // thrown from a bus queue.
        Shape::Kick(air, lift) => {
            let k = KICK;
            let back = if air { k.air_lean } else { 1.0 } * lift;
            q.hip = p(q.hip.f + k.hip_drive * ext, q.hip.u + k.hip_rise * ext);
            q.neck.f -= k.lean * ext * back;
            q.neck.u -= 1.0 * ext;
            q.head.f -= k.lean * 1.3 * ext * back;
            // The support foot travels in under the raised hips. It has
            // to: the leg standing on it is only as long as it is, and
            // leaving it behind was what stretched the *standing* leg
            // by a fifth and made the whole move read as toppling over
            // on a stilt.
            if !air {
                q.rear_foot = p(-18.0 + (q.hip.f - k.plant - -18.0) * ext, 0.0);
            } else {
                q.rear_foot = p(q.rear_foot.f, 30.0);
            }
            // Knee up to hip height, shin hanging down off it, foot
            // under the knee. Folding the foot up level with the hip
            // instead lays the shin flat along the thigh, and two limb
            // segments on top of each other are one shape — the leg
            // stops being legible as a leg at exactly the moment a
            // defender has to read which one is coming.
            let chamber = p(q.hip.f + k.knee_lead, if air { k.chamber_air } else { k.chamber });
            let kc = kick_curve(ext);
            q.lead_foot = if kc < 1.0 {
                q.lead_foot.to(chamber, kc)
            } else {
                chamber.to(toe_tip, kc - 1.0)
            };
            // Arms in, not flung back. All three sheets keep the guard
            // up through the whole kick; only the lead arm crosses.
            // The arms drop and trail. In every photograph of a
            // roundhouse the guard is *down* — both arms hanging
            // across and behind the hips as counterweight — not held
            // at the chest. A kick thrown with the hands still up is a
            // kick nobody put their body into.
            q.lead_hand = q.lead_hand.to(p(2.0, 60.0), ext);
            q.rear_hand = q.rear_hand.to(p(-18.0, 68.0), ext);
        }
        // Down on the back leg, one hand on the floor, the front leg
        // laid out flat along it.
        Shape::Sweep => {
            let hip_f = (tip.f - 52.0).clamp(2.0, 28.0);
            // The hip does not sink as far as it used to. A sweep is
            // nearly a kneel and the back knee comes down to the
            // boards — but with the hip that low the folded rear leg
            // had nowhere to put that knee except through the floor.
            q.hip = p(hip_f * ext, C_HIP + 3.0 * ext);
            q.neck = p(q.neck.f - 5.0 * ext, C_NECK - 5.0 * ext);
            q.head = p(q.head.f + 1.0 * ext, C_HEAD - 6.0 * ext);
            q.rear_foot = p(-3.0, 2.0);
            q.lead_foot = q.lead_foot.to(toe_tip, ext);
            q.rear_hand = q.rear_hand.to(p(-23.0, 4.0), ext);
            q.lead_hand = q.lead_hand.to(p(6.0, 40.0), ext);
        }
        // Both hands out, low and open. A throw drawn as a punch is the
        // game lying about the one move a guard cannot stop.
        Shape::Throw => {
            q.open = true;
            q.hip.f += 4.0 * ext;
            q.neck.f += 7.0 * ext;
            q.head.f += 6.0 * ext;
            q.lead_foot.f += 7.0 * ext;
            q.lead_hand = q.lead_hand.to(p(tip.f, tip.u + 6.0), ext);
            q.rear_hand = q.rear_hand.to(p(tip.f - 6.0, tip.u - 8.0), ext);
        }
        Shape::Bolt => {
            // Drawn from the hip and pushed out on both palms. The
            // hands do not have to reach the hitbox here because the
            // bolt carries it — see the spawn in update().
            q.open = true;
            q.hip = p(2.0, 51.0);
            q.lead_foot = p(21.0, 0.0);
            q.rear_foot = p(-22.0, 0.0);
            let charge = p(-17.0, 57.0);
            let out = p(44.0, 70.0);
            if ext < 0.55 {
                let k = ext / 0.55;
                q.lead_hand = q.lead_hand.to(charge, k);
                q.rear_hand = q.rear_hand.to(p(charge.f - 5.0, charge.u - 7.0), k);
                q.neck.f -= 5.0 * k;
                q.head.f -= 4.0 * k;
            } else {
                let k = (ext - 0.55) / 0.45;
                q.lead_hand = charge.to(out, k);
                q.rear_hand = p(charge.f - 5.0, charge.u - 7.0).to(p(out.f - 9.0, out.u - 8.0), k);
                q.neck.f += -5.0 + 11.0 * k;
                q.head.f += -4.0 + 9.0 * k;
            }
        }
        // A charge behind a straight right: the whole body goes with it,
        // which is what the forward velocity in update() is doing.
        Shape::Rush => {
            q.hip = p(8.0 * ext, HIP_U - 5.0 * ext);
            q.neck.f += 16.0 * ext;
            q.head.f += 14.0 * ext;
            q.lead_foot = p(15.0 + 17.0 * ext, 5.0 * ext);
            q.rear_foot.f -= 11.0 * ext;
            q.lead_hand = q.lead_hand.to(tip, ext);
            q.rear_hand = q.rear_hand.to(p(-17.0, 68.0), ext);
        }
    }
}

/// The pose a fighter is in this frame.
/// The pose a fighter is drawn in, including the tail of the one they
/// were in a moment ago.
///
/// Actions change on a single frame — a guard becomes a chambered kick
/// between one picture and the next — and a drawing that follows them
/// exactly teleports. Everything below hands over across four frames
/// instead, eased at both ends, which costs nothing in the rules (the
/// hitbox still appears on the frame the move says) and is most of the
/// difference between a figure that moves and a figure that cuts.
pub(crate) fn pose_of(now: f32, f: &Fighter, idx: usize) -> Pose {
    let q = pose_now(now, f, idx);
    if f.blend <= 0.0 { return q; }
    let mut was = *f;
    was.act = f.prev_act;
    was.mv = f.prev_mv;
    was.t = f.prev_t;
    was.blend = 0.0;
    let k = (1.0 - f.blend).clamp(0.0, 1.0);
    pose_now(now, &was, idx).to(q, k * k * (3.0 - 2.0 * k))
}

fn pose_now(now: f32, f: &Fighter, idx: usize) -> Pose {
    // One slow cycle per fighter, offset so two of them on screen are
    // never breathing in step.
    let breath = (now * 2.6 + idx as f32 * 2.3).sin();

    if f.act == Act::Knockdown {
        // Thrown down, still, and then up again. Landing and rising are
        // both blends rather than cuts, because a body that teleports
        // between two poses is the one thing on screen a player cannot
        // unsee.
        let down = floored();
        if f.t < 0.22 {
            // Off their feet. The hips go first and the legs come up
            // after them, so it reads as being taken off the ground
            // rather than as lying down on purpose — and the head is
            // already falling while the feet are still in the air.
            let k = (f.t / 0.22).clamp(0.0, 1.0);
            let mut air = stance(0.0);
            air.hip = p(-6.0, 48.0);
            air.neck = p(-26.0, 62.0);
            air.head = p(-40.0, 62.0);
            air.lead_foot = p(30.0, 44.0);
            air.rear_foot = p(10.0, 30.0);
            air.lead_hand = p(-18.0, 70.0);
            air.rear_hand = p(-34.0, 54.0);
            // Falling: quick at first as the legs are swept, then the
            // landing itself, which is the fastest part of it.
            return air.to(down, k * k);
        }
        if f.t > 0.85 {
            // Up onto one hand first, then onto the feet. The hand on
            // the floor is what makes it a fighter getting up instead
            // of a fighter being winched.
            let k = ((f.t - 0.85) / 0.30).clamp(0.0, 1.0);
            let mut up = crouched(0.0);
            up.rear_hand = p(-18.0, 5.0);
            up.lead_hand = p(10.0, 38.0);
            up.rear_foot = p(-10.0, 0.0);
            return down.to(up, (k * 1.8).min(1.0)).to(stance(0.0), (k - 0.55).max(0.0) / 0.45);
        }
        return down;
    }

    // Hitstun has no crouch of its own, so a fighter swept out of a
    // crouch used to stand bolt upright on the frame the blow landed
    // and then fold over — the body rose two feet and fell again inside
    // a tenth of a second. Being hit while low keeps you low, which is
    // both what happens and what the picture needs.
    let low = f.crouching()
        || (f.act == Act::Hitstun && matches!(f.prev_act, Act::Crouch))
        || (f.act == Act::Hitstun && f.prev_act == Act::Attack
            && matches!(f.prev_mv, MoveId::CrouchJab | MoveId::Sweep))
        || (f.act == Act::Hitstun && f.prev_act == Act::Block && f.crouch_block);
    let mut q = if f.airborne() {
        airborne_pose()
    } else if low {
        crouched(breath)
    } else {
        stance(breath)
    };

    match f.act {
        // Feet cycle with the ground rather than with the clock, so a
        // walking fighter's feet do not skate: the phase is where they
        // are, not how long they have been walking.
        Act::Walk => {
            let ph = f.x * 0.085;
            let (s, c) = (ph.sin(), ph.cos());
            q.lead_foot = p(15.0 + 11.0 * s, (9.0 * c).max(0.0));
            q.rear_foot = p(-17.0 - 11.0 * s, (-9.0 * c).max(0.0));
            q.hip.u -= 1.4 + 1.4 * (2.0 * ph).cos();
            q.neck.u -= 1.0 + 1.0 * (2.0 * ph).cos();
            q.head.u -= 1.0 + 1.0 * (2.0 * ph).cos();
            q.lead_hand.f += 3.0 * c;
            q.rear_hand.f -= 2.5 * c;
        }
        // Turtled up: weight off the front foot, shoulder raised, both
        // forearms stacked in front of the head or the belly.
        Act::Block => {
            if f.crouch_block {
                q.lead_hand = p(16.0, 46.0);
                q.rear_hand = p(11.0, 34.0);
                q.neck.f -= 4.0;
                q.head.f -= 4.0;
            } else {
                q.hip = p(-4.0, HIP_U - 2.0);
                q.neck = p(-8.0, NECK_U - 2.0);
                q.head = p(-7.0, HEAD_U - 2.0);
                q.lead_foot = p(11.0, 0.0);
                q.rear_foot = p(-20.0, 0.0);
                q.lead_hand = p(15.0, 93.0);
                q.rear_hand = p(11.0, 80.0);
            }
        }
        // Snapped back off the blow: head first, then the shoulders,
        // with the back foot skidding out to catch it.
        Act::Hitstun => {
            let k = (1.0 - f.t * 6.0).clamp(0.3, 1.0);
            let drop = if low { 0.5 } else { 1.0 };
            q.hip = p(q.hip.f - 6.0 * k, q.hip.u - 3.0 * k);
            q.neck = p(q.neck.f - 14.0 * k * drop, q.neck.u - 2.0);
            q.head = p(q.head.f - 21.0 * k * drop, q.head.u - 1.0);
            q.lead_foot = p(q.lead_foot.f - 10.0, 0.0);
            q.rear_foot = p(q.rear_foot.f - 6.0, 0.0);
            // The guard is knocked aside rather than teleported: the
            // hands are pushed off wherever they already were.
            q.lead_hand = p(q.lead_hand.f - 9.0, q.lead_hand.u - 5.0);
            q.rear_hand = p(q.rear_hand.f - 13.0, q.rear_hand.u - 9.0);
        }
        Act::Attack => {
            let m = f.scaled(move_data(f.mv));
            let ext = extension(f, &m);
            attack_pose(&mut q, f, &m, ext);
        }
        // Won.
        //
        // Three beats rather than a held pose: the guard comes down and
        // the fighter straightens, then the arm sweeps up, then it is
        // held and breathing. A victory that is simply *on* the frame
        // the round ends is a fighter who was already celebrating.
        //
        // The arm goes up and *forward*, not straight up. Raised
        // vertically it was drawn through the head — the fist came out
        // above the skull and the face was behind the sleeve, so the
        // whole figure read as headless with a pole through it. A real
        // arm raised in celebration swings out away from the head, and
        // the only "out" a side view has is forward.
        Act::Victory => {
            let t = f.t;
            let settle = (t / 0.30).clamp(0.0, 1.0);
            let raise = ((t - 0.26) / 0.34).clamp(0.0, 1.0);
            // Ease the sweep, and let the hand trail the elbow by
            // arriving later than the shoulder does.
            let sweep = raise * raise * (3.0 - 2.0 * raise);
            let bob = ((t - 0.6).max(0.0) * 2.2).sin() * 1.8;

            q.neck = p(-3.5 + 2.5 * settle, NECK_U + 1.0 * settle);
            // And they look up at it, which is what a person does.
            q.head = p(-1.5 + 1.0 * settle - 2.0 * sweep,
                       HEAD_U + 1.0 * settle + 1.5 * sweep + bob * 0.3);
            q.hip = p(0.0, HIP_U + 1.0 * settle);
            q.lead_foot = p(16.0 - 3.0 * settle, 0.0);
            q.rear_foot = p(-21.0 + 4.0 * settle, 0.0);
            // Rear hand comes to rest on the hip.
            q.rear_hand = q.rear_hand.to(p(-9.0, 64.0), settle);
            // Lead arm: guard, then down and back to load the swing,
            // then up and forward. The dip is what makes it a swing
            // rather than a hand appearing in the air.
            let load = p(6.0, 64.0);
            // Far enough forward that the whole arm clears the head.
            // Raised closer in, the sleeve crossed the face and the
            // celebration was performed by a man with no head.
            let up = p(33.0, 124.0 + bob);
            q.lead_hand = if raise <= 0.0 {
                q.lead_hand.to(load, settle)
            } else {
                load.to(up, sweep)
            };
        }
        // Lost: down on the back knee, head hanging.
        Act::Defeat => {
            q.hip = p(-4.0, 31.0);
            q.neck = p(3.0, 59.0);
            q.head = p(9.0, 67.0);
            q.lead_foot = p(15.0, 0.0);
            q.rear_foot = p(-15.0, 2.0);
            q.lead_hand = p(17.0, 27.0);
            q.rear_hand = p(-2.0, 15.0);
        }
        _ => {}
    }
    // No foot goes through the floor. The coil at the start of an
    // attack extrapolates back past the pose it starts from, which for
    // a foot already standing on the boards means below them.
    if !f.airborne() && f.act != Act::Knockdown {
        q.lead_foot.u = q.lead_foot.u.max(0.0);
        q.rear_foot.u = q.rear_foot.u.max(0.0);
    }
    q
}

/// Where the striking limb was a few frames ago, smeared behind it.
///
/// A blow that crosses sixty pixels in four frames is, at sixty frames
/// a second, three pictures of a limb in three places — and the eye
/// reads three pictures of a limb in three places as three limbs. The
/// smear is what turns them back into one limb moving fast, and it is
/// the single cheapest thing on this screen that makes an attack feel
/// like it was thrown rather than extruded.
fn draw_trail(blip: &Blip, f: &Fighter, now: f32, rig: Rig, i: usize, trim: BlipColor) {
    if f.act != Act::Attack { return; }
    let m = f.scaled(move_data(f.mv));
    let end = (m.startup + m.active) * F;
    if f.t > end + 2.0 * F { return; }
    let leg = is_kick(f);
    for k in 1..=3 {
        let back = k as f32 * 2.2 * F;
        if f.t < back { continue; }
        let mut old = *f;
        old.t = f.t - back;
        let q = pose_of(now, &old, i);
        let bones = skeleton(rig, &q);
        let (root, tip) = if leg { (bones.knee_lead, bones.ankle_lead) }
                          else { (bones.elbow_lead, bones.hand_lead) };
        let a = 0.22 - 0.06 * k as f32;
        stroke(blip, root, tip, 5.0, 3.4,
            BlipColor { r: trim.r, g: trim.g, b: trim.b, a });
    }
}

/// The fighter, flattened onto the floor.
///
/// Not a soft ellipse under the feet: the same skeleton, run through
/// the same joint solver, projected by a rig with the height squashed
/// and sheared toward the light. So the shadow kicks when the fighter
/// kicks, and slides away and shrinks when they jump, because it *is*
/// the fighter — there is one pose, and both drawings are made from it.
///
/// Drawn opaque rather than translucent. A shadow assembled from a
/// dozen overlapping translucent strokes darkens wherever two limbs
/// cross, which is the one thing real shadows never do; an opaque
/// silhouette in a fixed colour has no seams in it at all.
fn draw_shadow(blip: &Blip, rig: Rig, q: &Pose, bulk: f32, c: BlipColor) {
    let k = skeleton(rig, q);
    let b = bulk;
    for (hip, knee, ankle) in [(k.hip_rear, k.knee_rear, k.ankle_rear),
                               (k.hip_lead, k.knee_lead, k.ankle_lead)] {
        stroke(blip, hip, knee, 7.0 * b, 5.0 * b, c);
        stroke(blip, knee, ankle, 5.0 * b, 3.0 * b, c);
    }
    for (sh, elbow, hand) in [(k.sh_rear, k.elbow_rear, k.hand_rear),
                              (k.sh_lead, k.elbow_lead, k.hand_lead)] {
        stroke(blip, sh, elbow, 5.5 * b, 4.0 * b, c);
        stroke(blip, elbow, hand, 4.0 * b, 3.5 * b, c);
    }
    stroke(blip, k.hip_rear, k.hip_lead, 8.0 * b, 8.0 * b, c);
    stroke(blip, k.hip, k.neck, 9.5 * b, 12.0 * b, c);
    blip.fill_circle(k.head.0, k.head.1, 7.5 * b, c);
}

/// Every joint of one fighter, in screen coordinates, after the solver
/// has had its say.
///
/// This is the single answer to "where is this body". The fighter, the
/// shadow and the motion smear are all drawn from it, and the anatomy
/// tests are run against it — so if a knee is in the wrong place it is
/// in the wrong place in exactly one function, and a test can say so.
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

/// Solve one fighter's pose into joints.
///
/// `rig` decides where pose space lands on screen, so the same pose
/// solves into a standing fighter or into the flattened silhouette of
/// their shadow.
pub(crate) fn skeleton(rig: Rig, q: &Pose) -> Skeleton {
    let hip = rig.at(q.hip);
    let neck = rig.at(q.neck);
    let head = rig.at(q.head);

    // Sockets hang off the spine and turn with it. Fixed screen-space
    // offsets put a shoulder in front of a fighter who is leaning back
    // and behind one leaning in, so the arm appears to come out of the
    // wrong side of the chest — wrong in a way that is obvious to look
    // at and hard to name.
    let (sx, sy) = unit(hip, neck);
    let (px, py) = (-sy, sx); // across the body, square to the spine
    let socket = |at: V, half: f32, up: f32| {
        (V(at.0 + px * half * rig.fwd + sx * up, at.1 + py * half * rig.fwd + sy * up),
         V(at.0 - px * half * rig.fwd + sx * up, at.1 - py * half * rig.fwd + sy * up))
    };
    // Sockets sit well out from the spine: it is the distance between
    // them, plus the deltoid on each, that makes a fighter's shoulders.
    let (sh_lead, sh_rear) = socket(neck, 7.0, -5.0);
    let (hip_lead, hip_rear) = socket(hip, 4.5, 0.0);

    // Parallax on the far side.
    //
    // The far arm and far leg are the width of a torso further from the
    // camera, so they sit slightly back and slightly low of where the
    // near ones sit — and that offset is the only reason a viewer can
    // tell there are two of each. Without it a guard puts both fists in
    // the same place, a stance puts both feet in the same place, and
    // the fighter reads as a one-armed, one-legged person however well
    // the near limb is drawn.
    //
    // It is small, constant, and applied to the target rather than the
    // socket, so the far limb keeps its own bone lengths and the
    // separation shows up at the hand and the foot where it is seen.
    // The two go opposite ways vertically, because the camera sits at
    // about chest height: a far *foot* is below the eye and so appears
    // higher up the screen, toward the horizon, while a far hand is
    // around eye level and barely moves. Shifting the foot downward
    // instead — the obvious guess — puts it through the floorboards.
    let hand_back = |v: V| V(v.0 - rig.fwd * 3.0, v.1 + 1.0);
    let foot_back = |v: V| V(v.0 - rig.fwd * 3.5, v.1 - 1.5);

    // Bones scale with the rig, so a squashed shadow bends where a
    // squashed shadow should bend. Upright, this is 1.0.
    let b = rig.bone();
    let leg_at = |root: V, want: V| {
        // Which way a knee breaks is not a fixed direction in the
        // world — it turns with the thigh.
        //
        // Standing, the knee is forward of the hip-to-ankle line. With
        // the leg thrown out horizontally in a kick, that same
        // anatomical "forward" has rotated with the limb and now points
        // *up*, and a rule that says "put the knee as far forward as
        // possible" starts choosing the backwards bend. Which is
        // exactly what it did: on Brutus's kick and both high kicks the
        // knee inverted for the three frames the blow was live.
        //
        // So the bend side is derived from the limb's own direction —
        // the line turned a quarter turn, the way the knee faces —
        // and it is right at every angle rather than at the one angle
        // it was tuned for.
        let (dx, dy) = unit(root, want);
        let anterior = V(dy * rig.fwd, -dx * rig.fwd);
        solve(root, want, THIGH * b, SHIN * b, KNEE_SHUT, anterior, rig.ground)
    };
    let arm_at = |root: V, want: V| {
        // Above the shoulder the elbow goes behind, never below: driving
        // it below a raised shoulder folds the arm back through its own
        // socket, which is the most wrong a body can look and was on
        // screen every time somebody won a round.
        let raised = want.1 < root.1;
        let bias = if raised { V(-rig.fwd * 0.9, 0.15) } else { V(-rig.fwd * 0.45, 0.9) };
        solve(root, want, UARM * b, FARM * b, ELBOW_SHUT, bias, rig.ground)
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

fn unit(a: V, b: V) -> (f32, f32) {
    let d = dist(a, b).max(0.001);
    ((b.0 - a.0) / d, (b.1 - a.1) / d)
}

// ---- fighters ------------------------------------------------------------

/// One fighter, posed for whatever they are doing and then built out of
/// bones.
///
/// Drawing order is depth: far leg, far arm, body, head, near leg, near
/// arm. The near limbs are the ones attacks are thrown with, so an
/// attack always arrives in front of the body that threw it.
fn draw_fighter(blip: &Blip, g: &Game, i: usize) {
    let shake = if g.shake > 0.0 { (g.shake * 60.0).sin() * g.shake * 30.0 } else { 0.0 };
    // The docks put the sun low on the right, the temple puts the moon
    // high on the left. The shadows go the other way from whichever it
    // is: a shadow that disagrees with the only light in the picture is
    // worse than no shadow.
    let light = if g.stage == 0 { -1.0 } else { 1.0 };
    pose_and_draw_lit(blip, &g.p[i], g.now, shake, g.hitstop, i, light);
}

/// `shift` moves the whole fighter down the screen without moving the
/// floor they are standing on: the screen shake during a hit, and the
/// row offset the pose gallery lays its contact sheet out with. It is
/// deliberately not part of `f.y`, because `f.y` is where the fighter
/// *is* — the thing the rules and `airborne()` are judged on.
fn pose_and_draw(blip: &Blip, f: &Fighter, now: f32, shift: f32, hitstop: f32, i: usize) {
    pose_and_draw_lit(blip, f, now, shift, hitstop, i, 0.0);
}

/// `light` is which way shadows fall: negative for a light source off to
/// the right, positive for one off to the left, and zero for no cast
/// shadow at all — the select screen, which has no floor to cast on.
fn pose_and_draw_lit(blip: &Blip, f: &Fighter, now: f32, shift: f32, hitstop: f32, i: usize,
                     light: f32) {
    let f = *f;
    let a = f.arch();
    let prone = f.act == Act::Knockdown && f.t < 0.9;

    let rig = Rig::upright(f.x, f.y + shift, f.facing, if prone { -f.facing } else { f.facing });

    let flash = if hitstop > 0.0 && f.act == Act::Hitstun { 0.5 }
                else if invulnerable(&f) && ((now * 30.0) as i32) % 2 == 0 { 0.3 }
                else { 0.0 };
    let hide = Hide {
        cloth: rgb(a.color),
        trim: rgb(a.trim),
        skin: rgb(a.skin),
        hair: rgb(a.hair),
        build: a.build,
        bulk: a.bulk,
        tone: 1.0,
        flash,
        front: false,
    };
    let far = hide.far();
    let near = hide.infront();

    // The smear takes the colour of what is moving, washed most of the
    // way to white. Drawn in the fighter's accent it reads as a ribbon
    // trailing off them rather than as the limb itself, a frame ago.
    draw_trail(blip, &f, now, rig, i, blend(rgb(a.color), BLIP_WHITE, 0.55));

    // Dust off the boards where a body lands. Knockdowns are the one
    // moment the floor is part of the fight.
    if f.act == Act::Knockdown && f.t < 0.26 {
        let k = (1.0 - f.t / 0.26).max(0.0);
        for j in 0..5 {
            let dx = (j as f32 - 2.0) * 13.0 - f.facing * 14.0;
            let rise = (1.0 - k) * 16.0;
            blip.fill_circle(f.x + dx, FLOOR_Y + shift - 3.0 - rise, 3.0 + 7.0 * (1.0 - k),
                BlipColor { r: 0.62, g: 0.55, b: 0.46, a: 0.34 * k });
        }
    }

    let q = pose_of(now, &f, i);

    // Cast shadow first, then the contact patch under the feet, then the
    // fighter over both. The patch is what actually glues them down —
    // the long shadow says where the light is, but a body with nothing
    // directly beneath it floats however good its shadow.
    if light != 0.0 {
        let shadow = Rig {
            cx: f.x,
            ground: FLOOR_Y + shift,
            fwd: f.facing,
            face: f.facing,
            squash: -0.30,
            skew: light * 0.62,
            rise: FLOOR_Y - f.y,
        };
        draw_shadow(blip, shadow, &q, a.bulk, SHADOW);
    }
    let lift = ((FLOOR_Y - f.y) / 120.0).clamp(0.0, 1.0);
    let sw = 19.0 * a.bulk * (1.0 - 0.5 * lift);
    stroke(blip, V(f.x - sw, FLOOR_Y + shift), V(f.x + sw, FLOOR_Y + shift), 3.0, 3.0,
        BlipColor { r: 0.0, g: 0.0, b: 0.0, a: 0.34 * (1.0 - 0.55 * lift) });

    let k = skeleton(rig, &q);

    // What each part of the body just did, for the cloth to lag behind.
    //
    // No stored state: the pose is a pure function of the action timer,
    // so posing the same fighter a few frames earlier and subtracting
    // gives the real velocity of every joint. The gi then answers to
    // the actual motion of the limb it is hanging on rather than to a
    // sine wave that happens to look busy.
    let mut old = f;
    old.t = (f.t - 4.0 * F).max(0.0);
    let k0 = skeleton(rig, &pose_of(now, &old, i));
    // Grounded walking moves x directly rather than through a velocity,
    // so the body's own travel has to be added back by hand.
    let travel = if f.act == Act::Walk { f.facing * a.walk * 4.0 * F } else { 0.0 };
    let vel = |a: V, b: V| V((a.0 - b.0 + travel) * 0.55, (a.1 - b.1) * 0.55);

    draw_leg(blip, &far, k.hip_rear, k.knee_rear, k.ankle_rear, rig.fwd, rig.ground,
        vel(k.knee_rear, k0.knee_rear), now);
    draw_pelvis(blip, &hide, k.hip_rear, k.hip_lead);
    draw_torso(blip, &hide, k.hip, k.neck, vel(k.hip, k0.hip), now);
    // The far arm goes on after the chest but *before* the head: it is
    // in front of the ribs and behind the face. Drawn after the head it
    // wiped the face out every time the guard came up, which at this
    // size is most of the time.
    draw_arm(blip, &far, k.sh_rear, k.elbow_rear, k.hand_rear, q.open,
        vel(k.elbow_rear, k0.elbow_rear), now);
    draw_head(blip, &hide, rig, k.head, k.neck, now + i as f32);
    draw_leg(blip, &near, k.hip_lead, k.knee_lead, k.ankle_lead, rig.fwd, rig.ground,
        vel(k.knee_lead, k0.knee_lead), now);
    draw_arm(blip, &near, k.sh_lead, k.elbow_lead, k.hand_lead, q.open,
        vel(k.elbow_lead, k0.elbow_lead), now);
}

fn draw_fight(blip: &Blip, g: &Game) {
    for i in 0..2 { draw_fighter(blip, g, i); }

    for b in g.bolts.iter() {
        if !b.active { continue; }
        let c = rgb(FIGHTERS[g.p[b.owner].who].trim);
        blip.fill_glow_circle(b.x, b.y, 11.0, rgba(FIGHTERS[g.p[b.owner].who].trim, 0.35));
        blip.fill_circle(b.x, b.y, 7.0, c);
        blip.fill_circle(b.x - b.vx.signum() * 5.0, b.y, 4.0, BLIP_WHITE);
    }

    for s in g.hitspark.iter() {
        if s.ttl <= 0.0 { continue; }
        let k = s.ttl * 6.0;
        // Gold for damage, cold blue for a guard. The two outcomes have
        // to be tellable apart at a glance, because the whole of
        // defence is knowing whether the last exchange cost you
        // anything.
        let c = if s.blocked {
            BlipColor { r: 0.55, g: 0.85, b: 1.0, a: k.min(0.9) }
        } else {
            BlipColor { r: 1.0, g: 0.95, b: 0.6, a: k.min(0.9) }
        };
        // A burst of spikes rather than a ball of light. A circle is a
        // glow; spokes are a hit, because the eye reads the radiating
        // lines as force leaving a point.
        let spread = 8.0 + 26.0 * (1.0 - k).max(0.0);
        for j in 0..7 {
            let ang = j as f32 * 0.897 + s.x * 0.05;
            let (sx, sy) = (ang.cos(), ang.sin());
            let inner = 2.0 + 5.0 * k;
            let outer = inner + spread * (0.55 + 0.45 * ((j * 3) % 5) as f32 / 4.0);
            stroke(blip, V(s.x + sx * inner, s.y + sy * inner),
                V(s.x + sx * outer, s.y + sy * outer), 2.6 * k.min(1.0) + 0.6, 0.5, c);
        }
        blip.fill_circle(s.x, s.y, 3.0 + 9.0 * k, c);
        blip.fill_circle(s.x, s.y, 1.5 + 5.0 * k, BLIP_WHITE);
    }

    // The combo count, over the shoulder of whoever earned it.
    if g.combo_t > 0.0 && g.combo_shown > 1 {
        let f = g.p[g.combo_side];
        let text = format!("{} HITS", g.combo_shown);
        let rise = (1.1 - g.combo_t) * 26.0;
        blip.draw_text(&text, f.x - text_w(&text, 2.0) / 2.0, f.y - STAND_H - 26.0 - rise, 2.0,
            BlipColor { r: 1.0, g: 0.85, b: 0.25, a: g.combo_t.min(1.0) });
    }
}

// ---- HUD -----------------------------------------------------------------

/// Health, rounds won, and the clock.
///
/// The bar drains toward the centre from each side, the way the cabinets
/// did it, because the thing a player needs at a glance is not "how much
/// have I got" but "who is ahead" — and two bars meeting in the middle
/// answer that without being read.
fn draw_hud(blip: &Blip, g: &Game) {
    let pad = 16.0;
    let bar_w = (WIN_W as f32 - pad * 3.0 - 56.0) / 2.0;
    let bar_h = 16.0;
    let y = 22.0;

    for i in 0..2 {
        let f = g.p[i];
        let frac = (f.health as f32 / f.arch().health as f32).clamp(0.0, 1.0);
        let x = if i == 0 { pad } else { WIN_W as f32 - pad - bar_w };
        blip.fill_rect(x - 2.0, y - 2.0, bar_w + 4.0, bar_h + 4.0,
            BlipColor { r: 0.10, g: 0.10, b: 0.12, a: 1.0 });
        blip.fill_rect(x, y, bar_w, bar_h, BlipColor { r: 0.35, g: 0.06, b: 0.06, a: 1.0 });
        let w = bar_w * frac;
        // Both bars empty away from the centre of the screen.
        let fx = if i == 0 { x + bar_w - w } else { x };
        let c = if frac > 0.45 { BlipColor { r: 0.95, g: 0.85, b: 0.2, a: 1.0 } }
                else if frac > 0.2 { BlipColor { r: 0.95, g: 0.55, b: 0.15, a: 1.0 } }
                else { BlipColor { r: 0.95, g: 0.25, b: 0.2, a: 1.0 } };
        blip.fill_rect(fx, y, w, bar_h, c);
        blip.draw_rect(x, y, bar_w, bar_h, BlipColor { r: 0.8, g: 0.8, b: 0.85, a: 0.7 });

        let name = f.arch().name;
        let nx = if i == 0 { pad } else { WIN_W as f32 - pad - text_w(name, 1.0) };
        blip.draw_text(name, nx, y + bar_h + 6.0, 1.0, BLIP_WHITE);

        // Round pips: a fighter needs two, so two lamps say everything.
        for r in 0..ROUNDS_TO_WIN {
            let px = if i == 0 { pad + r as f32 * 14.0 } else { WIN_W as f32 - pad - 10.0 - r as f32 * 14.0 };
            let lit = f.rounds > r;
            blip.fill_circle(px + 5.0, y + bar_h + 22.0, 5.0,
                if lit { BLIP_YELLOW } else { BlipColor { r: 0.25, g: 0.25, b: 0.28, a: 1.0 } });
        }
    }

    let secs = g.clock.max(0.0).ceil() as i32;
    let cx = WIN_W as f32 / 2.0;
    blip.fill_rect(cx - 26.0, y - 4.0, 52.0, bar_h + 12.0,
        BlipColor { r: 0.10, g: 0.10, b: 0.12, a: 1.0 });
    let tc = if secs <= 10 { BlipColor { r: 1.0, g: 0.35, b: 0.3, a: 1.0 } } else { BLIP_WHITE };
    let text = format!("{secs}");
    blip.draw_text(&text, cx - text_w(&text, 2.0) / 2.0, y + 2.0, 2.0, tc);

    let stage = if g.stage == 0 { "THE DOCKS" } else { "MOONLIT TEMPLE" };
    blip.fill_rect(cx - text_w(stage, 1.0) / 2.0 - 5.0, y + bar_h + 11.0,
        text_w(stage, 1.0) + 10.0, 12.0, BlipColor { r: 0.08, g: 0.08, b: 0.10, a: 0.75 });
    blip.draw_text(stage, cx - text_w(stage, 1.0) / 2.0, y + bar_h + 14.0, 1.0,
        BlipColor { r: 0.92, g: 0.92, b: 0.96, a: 1.0 });
}

fn draw_banner(blip: &Blip, g: &Game) {
    let cy = 150.0;
    match g.state {
        State::RoundIntro => {
            let r = format!("ROUND {}", g.round);
            blip.draw_centered(&r, cy, 4.0, BLIP_YELLOW);
            if g.phase.remaining() < 0.7 {
                blip.draw_centered("FIGHT!", cy + 46.0, 5.0, BLIP_WHITE);
            }
        }
        State::RoundEnd => blip.draw_centered(g.banner, cy, 5.0, BLIP_YELLOW),
        State::MatchEnd => blip.draw_centered(g.banner, cy, 4.0, BLIP_YELLOW),
        State::Over => {
            blip.draw_centered("GAME OVER", cy, 5.0, BlipColor { r: 0.95, g: 0.3, b: 0.3, a: 1.0 });
            let s = format!("SCORE {}", g.sess.score);
            blip.draw_centered(&s, cy + 50.0, 2.0, BLIP_WHITE);
            if !g.phase.active() { blip.draw_centered("PRESS FIRE", cy + 86.0, 2.0, BLIP_YELLOW); }
        }
        State::Won => {
            blip.draw_centered("CHAMPION", cy, 5.0, BLIP_YELLOW);
            let s = format!("SCORE {}", g.sess.score);
            blip.draw_centered(&s, cy + 50.0, 2.0, BLIP_WHITE);
            if !g.phase.active() { blip.draw_centered("PRESS FIRE", cy + 86.0, 2.0, BLIP_YELLOW); }
        }
        _ => {}
    }
}

// ---- front of house ------------------------------------------------------

/// The title screen.
///
/// It used to carry nine lines of rules, on the theory that a player
/// cannot deduce a throw by pressing buttons. True — but nobody reads
/// nine lines standing at a cabinet, and a screen nobody reads teaches
/// nothing however correct it is. So it says the three things needed to
/// start (what moves you, what attacks, and that the three kicks are
/// three heights), and the rest has moved to where it is actually
/// wanted: the two lines under the roster on the select screen, read
/// while choosing, and the round-start reminder of the one move that is
/// genuinely invisible.
fn draw_title(blip: &Blip) {
    let dim = BlipColor { r: 0.74, g: 0.77, b: 0.84, a: 1.0 };
    let hot = BlipColor { r: 0.95, g: 0.85, b: 0.4, a: 1.0 };
    blip.draw_centered("BRAWLER", 78.0, 7.0, BLIP_YELLOW);
    blip.draw_centered("TWO FIGHTERS ENTER", 132.0, 2.0, BLIP_WHITE);

    blip.draw_centered("ARROWS  MOVE AND GUARD", 196.0, 2.0, dim);
    blip.draw_centered("SPACE  PUNCH", 226.0, 2.0, dim);
    blip.draw_centered("Z X C  KICK LOW MID HIGH", 256.0, 2.0, hot);

    blip.draw_centered("PRESS FIRE", 320.0, 3.0, BLIP_WHITE);
}

fn draw_select(blip: &Blip, g: &Game) {
    blip.draw_centered("CHOOSE YOUR FIGHTER", 46.0, 3.0, BLIP_YELLOW);
    for (i, a) in FIGHTERS.iter().enumerate() {
        let x = 70.0 + i as f32 * 180.0;
        let picked = i == g.pick;
        let box_c = if picked { rgb(a.trim) } else { BlipColor { r: 0.3, g: 0.3, b: 0.35, a: 1.0 } };
        blip.draw_rect(x, 96.0, 150.0, 190.0, box_c);
        if picked { blip.draw_rect(x - 2.0, 94.0, 154.0, 194.0, box_c); }

        // The fighter, standing in their box, drawn by the same code
        // that draws them in a match. A select screen that shows
        // something other than what you are about to control is an
        // advertisement, not a choice.
        let cx = x + 75.0;
        let ground = 268.0;
        let mut who = Fighter::new(i, cx, 1.0);
        who.y = FLOOR_Y;
        // Everyone stands in their fighting stance. The winner's pose
        // reads as a fighter with their guard down, which is the one
        // thing a select screen must not say about the fighter you are
        // about to pick.
        who.act = Act::Idle;
        who.facing = 1.0;
        pose_and_draw(blip, &who, g.now, ground - FLOOR_Y, 0.0, i);

        blip.draw_text(a.name, cx - text_w(a.name, 2.0) / 2.0, 102.0, 2.0,
            if picked { BLIP_WHITE } else { BlipColor { r: 0.6, g: 0.6, b: 0.66, a: 1.0 } });

        // The three numbers that actually differ, as bars — a player
        // choosing between archetypes needs the trade, not a biography.
        let stats = [("PWR", a.power / 1.4), ("SPD", a.walk / 150.0), ("HP ", a.health as f32 / 120.0)];
        for (r, (label, v)) in stats.iter().enumerate() {
            let sy = 272.0 + r as f32 * 14.0;
            blip.draw_text(label, x + 8.0, sy, 1.0, BlipColor { r: 0.7, g: 0.7, b: 0.8, a: 1.0 });
            blip.fill_rect(x + 40.0, sy, 100.0 * v.clamp(0.0, 1.0), 7.0, rgb(a.trim));
            blip.draw_rect(x + 40.0, sy, 100.0, 7.0, BlipColor { r: 0.3, g: 0.3, b: 0.36, a: 1.0 });
        }
    }
    let a = FIGHTERS[g.pick];
    let s = format!("SPECIAL: {}    PUNCH + KICK", a.special_name);
    blip.draw_centered(&s, 330.0, 2.0, rgb(a.trim));
    // The two rules a player cannot find by pressing buttons, put where
    // there is nothing else to do but read them.
    blip.draw_centered("GUARD HIGH OR LOW TO MATCH THE ATTACK", 356.0, 1.0,
        BlipColor { r: 0.74, g: 0.77, b: 0.84, a: 1.0 });
    blip.draw_centered("WALK IN CLOSE AND PUNCH TO THROW", 372.0, 1.0,
        BlipColor { r: 0.74, g: 0.77, b: 0.84, a: 1.0 });
    blip.draw_centered("PRESS FIRE TO START", 388.0, 2.0, BLIP_WHITE);
}

// ---- pose gallery (development only) -------------------------------------
//
// `cargo build -p brawler --features gallery` replaces the game with a
// contact sheet of every pose, looping. Posing a fighter is a drawing
// job and drawing jobs need to be *seen*; getting to a sweep by playing
// the game takes a dozen inputs and shows it for four frames.
#[cfg(feature = "gallery")]
pub fn draw_gallery(blip: &Blip, now: f32) {
    blip.clear(BlipColor { r: 0.15, g: 0.16, b: 0.21, a: 1.0 });
    let acts: [(&str, Act, MoveId); 17] = [
        ("JAB", Act::Attack, MoveId::Jab),
        ("LOW KICK", Act::Attack, MoveId::LowKick),
        ("KICK", Act::Attack, MoveId::Kick),
        ("HIGH KICK", Act::Attack, MoveId::HighKick),
        ("SWEEP", Act::Attack, MoveId::Sweep),
        ("LOW JAB", Act::Attack, MoveId::CrouchJab),
        ("THROW", Act::Attack, MoveId::Throw),
        ("SPECIAL", Act::Attack, MoveId::Special),
        ("JUMP KICK", Act::Attack, MoveId::JumpKick),
        ("JUMP PUNCH", Act::Attack, MoveId::JumpPunch),
        ("IDLE", Act::Idle, MoveId::Jab),
        ("WALK", Act::Walk, MoveId::Jab),
        ("CROUCH", Act::Crouch, MoveId::Jab),
        ("BLOCK", Act::Block, MoveId::Jab),
        ("HITSTUN", Act::Hitstun, MoveId::Jab),
        ("KNOCKDOWN", Act::Knockdown, MoveId::Jab),
        ("VICTORY", Act::Victory, MoveId::Jab),
    ];
    // One move at a time, five frames of it across the screen. A pose
    // is only ever half the question — the other half is what it is on
    // its way to and back from.
    //
    // Stepped with left/right rather than on a timer, because a contact
    // sheet that moves on by itself cannot be looked at: every capture
    // lands on whatever it had drifted to, and comparing a change to
    // the frame before it becomes guesswork.
    use std::sync::atomic::{AtomicUsize, Ordering};
    static AT: AtomicUsize = AtomicUsize::new(0);
    static WHO: AtomicUsize = AtomicUsize::new(0);
    if blip::input::key_pressed(BLIP_KEY_RIGHT) { AT.fetch_add(1, Ordering::Relaxed); }
    if blip::input::key_pressed(BLIP_KEY_LEFT) { AT.fetch_add(acts.len() - 1, Ordering::Relaxed); }
    if blip::input::key_pressed(BLIP_KEY_UP) { WHO.fetch_add(1, Ordering::Relaxed); }
    let n = AT.load(Ordering::Relaxed) % acts.len();
    let who = WHO.load(Ordering::Relaxed) % 3;
    let (label, act, mv) = acts[n];
    blip.fill_rect(0.0, FLOOR_Y, WIN_W as f32, WIN_H as f32 - FLOOR_Y,
        BlipColor { r: 0.11, g: 0.10, b: 0.13, a: 1.0 });

    let probe = Fighter::new(who, 0.0, 1.0);
    let m = probe.scaled(move_data(mv));
    let total = (m.startup + m.active + m.recovery) * F;
    for k in 0..5 {
        let x = 74.0 + k as f32 * 124.0;
        let mut f = Fighter::new(who, x, 1.0);
        f.act = act;
        f.mv = mv;
        f.y = FLOOR_Y;
        f.t = match act {
            // Sampled inside the startup, because the startup is where
            // the shape of a move is decided and the part a defender
            // has to read.
            Act::Attack => [m.startup * F * 0.35, m.startup * F * 0.7, m.startup * F,
                            (m.startup + m.active) * F, total * 0.8][k],
            Act::Knockdown => [0.05, 0.22, 0.6, 0.95, 1.12][k],
            Act::Hitstun => [0.02, 0.06, 0.12, 0.2, 0.3][k],
            // Everything else gets a time sweep too. Sampling a
            // standing pose five times at the same instant draws the
            // same fighter five times and says nothing about whether it
            // moves — which is exactly the question.
            Act::Victory | Act::Defeat => [0.0, 0.18, 0.45, 0.9, 1.6][k],
            _ => now + k as f32 * 0.09,
        };
        if matches!(mv, MoveId::JumpKick | MoveId::JumpPunch) && act == Act::Attack {
            f.y = FLOOR_Y - 46.0;
        }
        if act == Act::Walk { f.x = x + (now * 60.0) % 24.0; }
        blip.draw_line(x - 60.0, FLOOR_Y, x + 74.0, FLOOR_Y,
            BlipColor { r: 0.34, g: 0.34, b: 0.44, a: 0.8 });
        // The hitbox this frame, so the picture can be checked against
        // the thing it claims to be a picture of.
        if let Some((hx, hy, hw, hh)) = f.hit_box() {
            blip.fill_rect(hx, hy, hw, hh, BlipColor { r: 1.0, g: 0.3, b: 0.3, a: 0.30 });
        }
        pose_and_draw(blip, &f, now, 0.0, 0.0, k);
    }
    blip.draw_centered(label, 22.0, 3.0, BLIP_YELLOW);
    blip.draw_centered(FIGHTERS[who].name, 54.0, 2.0, BLIP_WHITE);
}
