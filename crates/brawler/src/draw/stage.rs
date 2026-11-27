//! The stages: seven backdrops and the floor under them. Scenery only: the
//! floor line, the walls and the height are the same everywhere.

use super::*;

std::thread_local! {
    static FILM_POSTER: blip::macroquad::texture::Texture2D =
        blip::load_png(blip::png!("film_poster"));
}

/// Backdrop contrast reduction keeps fighters prominent; all stages share floor, height and walls.
pub(crate) const HAZE: f32 = 0.24;

/// The current stage: its backdrop, the knockout burst if there is one, and
/// the floor.
pub(crate) fn draw_stage(blip: &Blip, g: &Game) {
    // A hit shakes the whole picture, which is most of what selling an
    // impact means when the fighters themselves are simple shapes.
    let shake = g.shake_px();
    // The scenery is further off than the floor: it zooms and pans half as much.
    look_through(blip, g.view, 0.5);
    // At the pagoda the school's students bow to the fighters as the round opens, in time with them.
    let bow = if g.state == State::RoundIntro && g.stage == 8 && g.p[0].act == Act::Bow {
        ((g.p[0].t / 1.3).clamp(0.0, 1.0) * std::f32::consts::PI).sin()
    } else { 0.0 };
    BOW.set(bow);
    draw_backdrop(blip, g.stage, shake, g.now, g.roar, (g.p[0].x + g.p[1].x) / 2.0);
    BOW.set(0.0);
    look_through(blip, g.view, 1.0);
    if g.slow > 0.0 { draw_finish(blip, g); }
    draw_floor(blip, g.stage, shake);
}

std::thread_local! {
    /// How far into a bow the pagoda's students are (0 to 1): set around the backdrop by `draw_stage`.
    static BOW: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
}

/// Draw what follows through the fight camera, as a layer `depth` back (see `View::rect`).
pub(crate) fn look_through(blip: &Blip, view: View, depth: f32) {
    let (x, y, w, h) = view.rect(depth);
    blip.set_view_rect(x, y, w, h);
}

/// One stage's scenery and the air in front of it, moved down by `shift`.
/// `focus` is where the fight is (x): the crowd's eyes follow it.
pub(crate) fn draw_backdrop(blip: &Blip, stage: usize, shift: f32, now: f32, roar: Roar, focus: f32) {
    let air = |r: f32, g: f32, b: f32, a: f32| BlipColor { r, g, b, a };
    let air = match stage {
        0 => { draw_dock(blip, shift, now, roar); air(0.40, 0.17, 0.20, HAZE) }
        1 => { draw_temple(blip, shift, now, roar); air(0.07, 0.07, 0.13, HAZE) }
        // The daylight stages are bright already: less air, or they wash out.
        2 => { draw_airbase(blip, shift, now, roar); air(0.70, 0.80, 0.90, HAZE * 0.6) }
        3 => { draw_bathhouse(blip, shift, now, roar); air(0.50, 0.38, 0.26, HAZE * 0.8) }
        5 => { draw_fortress(blip, shift, now); air(0.55, 0.75, 0.90, HAZE * 0.5) }
        6 => { draw_rooftop(blip, shift, now, roar); air(0.42, 0.26, 0.34, HAZE * 0.8) }
        7 => { draw_hood(blip, shift, now); air(0.16, 0.11, 0.19, HAZE * 0.6) }
        8 => { draw_pagoda(blip, shift, now, roar); air(0.16, 0.08, 0.16, HAZE * 0.7) }
        9 => { draw_island(blip, shift, now, roar); air(0.80, 0.86, 0.80, HAZE * 0.5) }
        10 => { draw_colosseum(blip, shift, now, focus, roar); air(0.70, 0.62, 0.52, HAZE * 0.45) }
        _ => { draw_village(blip, shift, now, roar); air(0.50, 0.56, 0.36, HAZE * 0.7) }
    };
    // The air in two veils, the bystanders between them: the scenery gets as much as ever
    // (1 - (1 - 0.55a)(1 - 0.5a) is about a), and the crowd half of it, enough to set it back behind the fighters.
    blip.fill_rect(0.0, 0.0, WIN_W as f32, FLOOR_Y + shift, BlipColor { a: air.a * 0.55, ..air });
    draw_onlookers(blip, stage, shift, now, roar, focus);
    blip.fill_rect(0.0, 0.0, WIN_W as f32, FLOOR_Y + shift, BlipColor { a: air.a * 0.5, ..air });
    draw_life(blip, stage, shift, now);
}

/// Who watches from the back of each stage, as SF2's stages are full of them (the soldiers on the air base's
/// crates, the monks, the dockers): in full colour in front of the air, toward the sides so the middle stays
/// clear, cheering when a blow lands. (shirt, trousers, hat or none)
type Rgb = (f32, f32, f32);
const CROWDS: [&[(Rgb, Rgb, Option<Rgb>)]; STAGES] = [
    // Dockers: work shirts and caps.
    &[((0.85, 0.45, 0.18), (0.20, 0.24, 0.36), Some((0.22, 0.30, 0.52))), ((0.30, 0.42, 0.62), (0.26, 0.22, 0.20), None),
      ((0.78, 0.74, 0.62), (0.22, 0.24, 0.30), Some((0.80, 0.30, 0.20))), ((0.32, 0.50, 0.36), (0.20, 0.18, 0.22), None)],
    // Monks in saffron.
    &[((0.95, 0.58, 0.16), (0.95, 0.58, 0.16), None), ((0.80, 0.36, 0.12), (0.80, 0.36, 0.12), None),
      ((0.95, 0.58, 0.16), (0.95, 0.58, 0.16), None)],
    // Ground crew and soldiers in olive.
    &[((0.40, 0.46, 0.28), (0.36, 0.40, 0.26), Some((0.34, 0.40, 0.24))), ((0.44, 0.50, 0.32), (0.38, 0.42, 0.28), None),
      ((0.90, 0.88, 0.80), (0.32, 0.36, 0.26), None), ((0.40, 0.46, 0.28), (0.36, 0.40, 0.26), Some((0.34, 0.40, 0.24)))],
    // Bathers in cotton yukata: indigo, rose, moss. (Not white: in white they were small copies of RYUKA.)
    &[((0.26, 0.32, 0.56), (0.26, 0.32, 0.56), None), ((0.42, 0.56, 0.78), (0.42, 0.56, 0.78), None),
      ((0.86, 0.48, 0.52), (0.86, 0.48, 0.52), None), ((0.44, 0.54, 0.36), (0.44, 0.54, 0.36), None)],
    // Villagers in straw hats.
    &[((0.58, 0.66, 0.78), (0.30, 0.30, 0.36), Some((0.88, 0.76, 0.44))), ((0.82, 0.30, 0.26), (0.28, 0.24, 0.20), None),
      ((0.88, 0.84, 0.70), (0.34, 0.30, 0.26), Some((0.88, 0.76, 0.44)))],
    // Nobody lives at the fortress.
    &[],
    // City folk.
    &[((0.86, 0.22, 0.30), (0.20, 0.22, 0.30), None), ((0.94, 0.80, 0.30), (0.24, 0.26, 0.38), Some((0.20, 0.20, 0.24))),
      ((0.36, 0.62, 0.70), (0.30, 0.26, 0.24), None), ((0.60, 0.40, 0.70), (0.18, 0.18, 0.22), None)],
    // The block.
    &[((0.92, 0.92, 0.94), (0.24, 0.34, 0.60), Some((0.80, 0.14, 0.18))), ((0.20, 0.20, 0.24), (0.30, 0.28, 0.34), None),
      ((0.96, 0.66, 0.20), (0.22, 0.22, 0.28), None), ((0.30, 0.60, 0.40), (0.24, 0.24, 0.30), Some((0.18, 0.18, 0.22)))],
    // The school's students: black uniforms, a few senior ones in red.
    &[((0.13, 0.13, 0.15), (0.13, 0.13, 0.15), None), ((0.56, 0.12, 0.12), (0.12, 0.12, 0.14), None),
      ((0.16, 0.15, 0.18), (0.16, 0.15, 0.18), None)],
    // The tournament's entrants: white uniforms, and the host's guard in black. (Yellow, the other colour of his
    // guard, made a row of DRAKE lookalikes.)
    &[((0.94, 0.94, 0.92), (0.94, 0.94, 0.92), None), ((0.12, 0.12, 0.14), (0.12, 0.12, 0.14), None),
      ((0.92, 0.92, 0.90), (0.92, 0.92, 0.90), None)],
    // THE COLOSSEUM: nobody, as in Way of the Dragon. Only the kitten watches (`draw_colosseum`).
    &[],
];
const SKINS: [(f32, f32, f32); 6] = [(0.92, 0.74, 0.58), (0.74, 0.52, 0.36), (0.50, 0.33, 0.24), (0.86, 0.66, 0.50),
    (0.64, 0.44, 0.30), (0.96, 0.80, 0.68)];
const HAIRS: [(f32, f32, f32); 6] = [(0.12, 0.10, 0.10), (0.30, 0.20, 0.13), (0.20, 0.14, 0.10), (0.62, 0.60, 0.58),
    (0.78, 0.62, 0.34), (0.56, 0.26, 0.14)];

/// What a bystander is doing with their arms between blows.
#[derive(Copy, Clone)]
enum Stand { Hanging, Folded, Chin, Pointing }

/// The ways to cheer, one picked afresh by each person for each roar (`Roar::kind`).
const CHEERS: usize = 5;

/// Two loose rows at each side: the back row smaller, higher up and in the shade. Nobody stands on a grid:
/// each place is jittered, and each person faces the fight or, now and then, turns to a neighbour.
fn draw_onlookers(blip: &Blip, stage: usize, shift: f32, now: f32, roar: Roar, focus: f32) {
    let cast = CROWDS[stage % STAGES];
    if cast.is_empty() { return; }
    let ground = FLOOR_Y + shift - 1.0;
    draw_passers(blip, stage, ground, now);
    for back in [true, false] {
        let per_side = if back { 2 } else { 3 };
        for n in 0..per_side * 2 {
            let i = n + if back { 11 } else { 0 };
            let pick = |salt: u32| scatter(i, salt + stage as u32 * 7);
            let left = n % 2 == 0;
            // At the pagoda the students keep to the right: the moon gate and the wooden dummy are on the left.
            if stage == 8 && left { continue; }
            let slot = (n / 2) as f32;
            // Inside what the closest camera shows of the scenery (x 42 to 598), loosely spaced.
            let off = if back { 92.0 } else { 60.0 } + slot * 60.0 + (pick(0x51) - 0.5) * 24.0;
            let x = if left { off } else { WIN_W as f32 - off };
            let inward = if left { 1.0 } else { -1.0 };
            // A quarter of them talk to a neighbour now and then, turned away from the fight.
            let chat = pick(0x5c) < 0.25 && (now * (0.21 + 0.1 * pick(0x5d)) + pick(0x5e) * 6.0).sin() > 0.55;
            let (top, legs, hat) = cast[(i + stage) % cast.len()];
            // One in seven a child; everyone else between slight and stout.
            let child = pick(0x61) > 0.86;
            let size = if child { 0.42 } else { 0.56 + 0.10 * pick(0x62) } * if back { 0.88 } else { 1.0 };
            let who = Person {
                skin: SKINS[(pick(0x63) * 6.0) as usize % 6],
                hair: HAIRS[(pick(0x64) * 6.0) as usize % 6],
                // A shirt a shade off the next one's, so two in the same colours are still two people.
                shirt: tint(top, 0.85 + 0.3 * pick(0x65)),
                trousers: tint(legs, 0.85 + 0.3 * pick(0x66)),
                hat: if child { None } else { hat },
                straw: stage == 4,
                hairdo: if stage == 1 { 1 } else { (pick(0x67) * 5.0) as u8 },
                bulk: if child { 0.9 } else { 0.92 + 0.36 * pick(0x68) },
                size,
                stand: [Stand::Hanging, Stand::Folded, Stand::Chin, Stand::Pointing][(pick(0x69) * 4.0) as usize % 4],
                tone: if back { 0.78 } else { 1.0 },
                // Everyone keeps their own time: clapping, pumping, swaying.
                pace: 0.75 + 0.55 * pick(0x6a),
                turned: false,
                stride: None,
            };
            let salt = 0x700 + stage as u32;
            let cheer = (roar.of(i, salt, now), roar.kind(i, salt, CHEERS));
            // Further back, a little higher up: a few pixels of depth each.
            let lift = if back { 7.0 } else { 0.0 } + 4.0 * pick(0x6b);
            let face = if chat { -inward } else { inward };
            let t = now * who.pace + i as f32 * 0.77;
            // Every few seconds each one may turn: side-on to the fight, square to it, or away to the back row.
            let epoch = ((now + pick(0x6d) * 9.0) / (6.0 + 5.0 * pick(0x6e))).floor() as u32;
            let turn = scatter(i, 0x6c ^ epoch.wrapping_mul(0x9E37) ^ stage as u32);
            let mut who = who;
            if turn < 0.38 { person_front(blip, &who, V(x, ground - lift), t, cheer, focus) }
            else if turn < 0.48 && cheer.0 == 0.0 { who.turned = true; person_front(blip, &who, V(x, ground - lift), t, cheer, focus) }
            else { person(blip, &who, V(x, ground - lift), face, t, cheer) }
        }
    }
}

/// A couple of people at the very back walking from one side of the stage to the other and back, in no hurry:
/// a crowd that only ever stands still is a painting.
fn draw_passers(blip: &Blip, stage: usize, ground: f32, now: f32) {
    let cast = CROWDS[stage % STAGES];
    for j in 0..2usize {
        let i = 40 + j;
        let pick = |salt: u32| scatter(i, salt + stage as u32 * 13);
        let speed = 14.0 + 10.0 * pick(0x81);
        // To and fro between the ends, starting anywhere along the way.
        let (from, to) = (36.0, WIN_W as f32 - 36.0);
        let span = to - from;
        let run = (now * speed + pick(0x82) * 2.0 * span).rem_euclid(2.0 * span);
        let (x, face) = if run < span { (from + run, 1.0) } else { (to - (run - span), -1.0) };
        let size = 0.50 + 0.08 * pick(0x83);
        let (top, legs, hat) = cast[(i + stage) % cast.len()];
        let who = Person {
            skin: SKINS[(pick(0x84) * 6.0) as usize % 6],
            hair: HAIRS[(pick(0x85) * 6.0) as usize % 6],
            shirt: tint(top, 0.8 + 0.3 * pick(0x86)),
            trousers: tint(legs, 0.8 + 0.3 * pick(0x87)),
            hat, straw: stage == 4,
            hairdo: if stage == 1 { 1 } else { (pick(0x88) * 5.0) as u8 },
            bulk: 0.92 + 0.3 * pick(0x89),
            size,
            stand: Stand::Hanging,
            tone: 0.72,
            pace: 1.0,
            turned: false,
            // The stride follows the ground covered, so the planted foot stays put.
            stride: Some(run * STRIDE / size),
        };
        person(blip, &who, V(x, ground - 13.0 - 3.0 * j as f32), face, now + j as f32, (0.0, 0));
    }
}

fn tint((r, g, b): (f32, f32, f32), k: f32) -> (f32, f32, f32) { ((r * k).min(1.0), (g * k).min(1.0), (b * k).min(1.0)) }

/// One bystander, as the fighters are built (`Build::Civvy` on the same rig): who they are.
struct Person {
    skin: (f32, f32, f32),
    hair: (f32, f32, f32),
    shirt: (f32, f32, f32),
    trousers: (f32, f32, f32),
    hat: Option<(f32, f32, f32)>,
    /// A wide straw hat rather than a cap.
    straw: bool,
    hairdo: u8,
    bulk: f32,
    size: f32,
    stand: Stand,
    /// Under 1 in the back row, in the shade.
    tone: f32,
    /// How fast this one moves, around 1.
    pace: f32,
    /// Seen from behind (`person_front` only): turned away to the back row, no face.
    turned: bool,
    /// Walking (`person` only): where the feet are in the stride, as `walk_phase`.
    stride: Option<f32>,
}

/// A bystander standing at `at` facing `face`: weight shifting from foot to foot, arms at rest their own way.
/// `cheer` is how far into answering a blow they are (0 to 1) and which way they do it: both fists up, clapping,
/// one arm waving, hands on the head, or jumping with a fist up.
fn person(blip: &Blip, who: &Person, at: V, face: f32, t: f32, (cheer, kind): (f32, usize)) {
    let (x, ground) = (at.0, at.1);
    let tone = who.tone;
    let shift = (t * 0.9).sin();
    let jump = if kind == 4 { cheer * (t * 9.0).sin().abs() * 7.0 } else { cheer * (t * 7.0).sin().abs() * 1.5 };
    let rig = Rig::upright(x, ground - jump * who.size, face, face, who.size);
    // Upright and relaxed: the knees a little bent, the weight on one leg and then the other.
    let mut q = Pose {
        hip: p(shift * 1.5, 57.0), head: p(2.0 + shift * 0.8, 107.0 + 0.5 * (t * 2.1).sin()),
        lead_hand: p(4.0, 57.0), rear_hand: p(-2.0, 58.0),
        lead_foot: p(9.0, 0.0), rear_foot: p(-8.0, 0.0),
        open: true, lead_heel: 0.0, rear_heel: 0.0, twist: 0.0,
    };
    match who.stand {
        Stand::Hanging => {}
        Stand::Folded => { q.lead_hand = p(15.0, 80.0); q.rear_hand = p(17.0, 84.0); q.open = false; }
        // A hand to the chin, the other arm across under it.
        Stand::Chin => { q.lead_hand = p(13.0, 99.0); q.rear_hand = p(14.0, 78.0); q.open = false; }
        // Pointing at the fight now and then.
        Stand::Pointing => {
            let k = ((t * 0.6).sin() * 2.0 - 0.8).clamp(0.0, 1.0);
            q.lead_hand = q.lead_hand.to(p(40.0, 98.0), k);
        }
    }
    if let Some(ph) = who.stride {
        // A plain walk: the feet passing under the body, the arms swinging against them.
        let (lf, ll) = foot_cycle(ph, 7.0);
        let (rf, rl) = foot_cycle(ph + std::f32::consts::PI, 6.3);
        q.lead_foot = p(WALK_MID + lf, ll);
        q.rear_foot = p(WALK_MID + rf, rl);
        q.lead_heel = heel_cycle(ph);
        q.rear_heel = heel_cycle(ph + std::f32::consts::PI);
        let s = ph.sin();
        q.hip = p(1.0, 58.0 + 1.2 * (2.0 * ph).cos());
        q.head = p(4.0, 109.0 + 1.0 * (2.0 * ph).cos());
        q.lead_hand = p(5.0 - 10.0 * s, 53.0 + 2.0 * s.abs());
        q.rear_hand = p(5.0 + 10.0 * s, 53.0 + 2.0 * s.abs());
    }
    // A bow (rei): hips back, the back straight and tipped forward, hands to the thighs.
    let bow = BOW.get();
    if bow > 0.0 && who.stride.is_none() {
        q.hip = p(q.hip.f - 4.0 * bow, q.hip.u - 1.0 * bow);
        q.head = p(q.head.f + 18.0 * bow, q.head.u - 14.0 * bow);
        q.lead_hand = q.lead_hand.to(p(10.0, 46.0), bow);
        q.rear_hand = q.rear_hand.to(p(4.0, 48.0), bow);
        q.open = true;
    }
    let mood = if cheer > 0.3 { if kind == 3 { Mood::Hurt } else { Mood::Shout } } else { Mood::Calm };
    if cheer > 0.0 {
        let (lead, rear, open) = match kind {
            // Both fists up and pumping.
            0 => { let pump = 4.0 * (t * 12.0).sin(); (p(10.0, 136.0 + pump), p(4.0, 132.0 - pump), false) }
            // Clapping in front of the chest.
            1 => { let gap = 4.0 + 5.0 * (t * 16.0).sin().abs(); (p(22.0, 96.0), p(22.0 - gap, 98.0), true) }
            // One arm up, waving side to side.
            2 => (p(14.0 + 8.0 * (t * 8.0).sin(), 134.0), q.rear_hand, true),
            // Hands to the head: that must have hurt.
            3 => (p(8.0, 116.0), p(-2.0, 114.0), true),
            // Up off the boards, one fist punching the air.
            _ => (p(12.0, 138.0), p(16.0, 82.0), false),
        };
        q.lead_hand = q.lead_hand.to(lead, cheer);
        q.rear_hand = q.rear_hand.to(rear, cheer);
        if cheer > 0.5 { q.open = open; }
        if kind == 3 { q.head.f -= 3.0 * cheer; }
    }
    let base = Look {
        cloth: shade(rgb(who.trousers), tone), trim: shade(rgb(who.shirt), tone),
        skin: shade(rgb(who.skin), tone), hair: shade(rgb(who.hair), tone),
        build: Build::Civvy, bulk: who.bulk * who.size, size: who.size, far: false, blink: (t * 0.37).fract() < 0.04,
        tone: 1.0, wind: (0.0, 0.0), floor: f32::INFINITY, prone: false, flash: 0.0, hairdo: who.hairdo, claw: false, torn: false, scars: false,
    };
    let far = Look { far: true, tone: FAR, ..base };
    // A shadow on the boards.
    let w = 15.0 * who.bulk * who.size;
    for row in 0..4 {
        let y = (row as f32 + 0.5) / 2.0 - 1.0;
        let half = w * (1.0 - y * y).sqrt();
        blip.fill_rect(x - half, ground + row as f32 - 2.0, half * 2.0, 1.0, BlipColor { r: 0.0, g: 0.0, b: 0.0, a: 0.26 });
    }
    let k = skeleton(rig, &q);
    light_from(Some(face));
    draw_leg(blip, &far, k.hip_rear, k.knee_rear, k.ankle_rear, face, rig.ground, q.rear_heel);
    draw_arm(blip, &far, k.sh_rear, k.elbow_rear, k.hand_rear, q.open, face);
    draw_torso(blip, &base, k.hip, k.neck, face);
    draw_head(blip, &base, face, k.head, k.neck, mood);
    draw_leg(blip, &base, k.hip_lead, k.knee_lead, k.ankle_lead, face, rig.ground, q.lead_heel);
    draw_belt(blip, &base, k.hip, k.neck, face);
    draw_arm(blip, &base, k.sh_lead, k.elbow_lead, k.hand_lead, q.open, face);
    light_from(None);
    // The hat, over the hair.
    if let Some(c) = who.hat {
        let r = head_r(base.bulk / base.size) * base.size;
        let crown = V(k.head.0 - face * 0.15 * r, k.head.1 - 0.80 * r);
        let c = shade(rgb(c), tone);
        if who.straw {
            // A wide cone of straw.
            shape(blip, &[(V(crown.0 - 1.6 * r, crown.1 + 0.25 * r), V(crown.0 + 1.6 * r, crown.1 + 0.25 * r), 0.16 * r, 0.16 * r),
                          (V(crown.0, crown.1 - 0.55 * r), V(crown.0, crown.1 + 0.05 * r), 0.15 * r, 0.75 * r)], c, INK);
        } else {
            // A cap, its peak out over the face.
            shape(blip, &[(V(crown.0 - face * 0.45 * r, crown.1), V(crown.0 + face * 0.40 * r, crown.1), 0.50 * r, 0.50 * r),
                          (V(crown.0 + face * 0.5 * r, crown.1 + 0.30 * r), V(crown.0 + face * 1.15 * r, crown.1 + 0.34 * r),
                           0.13 * r, 0.11 * r)], c, INK);
        }
    }
}

/// A bystander square to the viewer, facing the fight head-on: both legs, a chest across, both arms, a whole face
/// with eyes turned to `focus` (the fight's x). The same cheers as `person`, seen from the front.
fn person_front(blip: &Blip, who: &Person, at: V, t: f32, (cheer, kind): (f32, usize), focus: f32) {
    let (s, b) = (who.size, who.bulk * who.size);
    let jump = if kind == 4 { cheer * (t * 9.0).sin().abs() * 7.0 } else { cheer * (t * 7.0).sin().abs() * 1.5 };
    let sway = (t * 0.9).sin() * 1.2;
    // Pose space as the fighters' (x across, u up), scaled to this person.
    let pt = |x: f32, u: f32| V(at.0 + (x + sway * u / 110.0) * s, at.1 - (u + jump) * s);
    let tone = who.tone;
    let (skin, shirt, trousers, hair) = (shade(rgb(who.skin), tone), shade(rgb(who.shirt), tone),
        shade(rgb(who.trousers), tone), shade(rgb(who.hair), tone));
    let shoe = shade(BOOT, tone);
    // Shadow on the boards.
    let w = 16.0 * b;
    for row in 0..4 {
        let y = (row as f32 + 0.5) / 2.0 - 1.0;
        let half = w * (1.0 - y * y).sqrt();
        blip.fill_rect(at.0 - half, at.1 + row as f32 - 2.0, half * 2.0, 1.0, BlipColor { r: 0.0, g: 0.0, b: 0.0, a: 0.26 });
    }
    light_from(None);
    // Legs a little apart, the weight shifting from one to the other.
    let lean = sway * 0.6;
    for side in [-1.0f32, 1.0] {
        let hip = pt(side * 5.5, 58.0);
        let knee = pt(side * 7.0 + lean, 31.0);
        let ankle = pt(side * 8.0, 7.0);
        shape(blip, &[(hip, knee, 7.0 * b, 5.8 * b), (knee, ankle, 5.8 * b, 4.2 * b)], trousers, INK);
        // A shoe seen toe-on: short and wide.
        shape(blip, &[(pt(side * 8.0 - 2.0, 3.0), pt(side * 8.0 + 2.0, 3.0), 3.8 * b, 3.8 * b)], shoe, INK);
    }
    // Arms by the cheer, or at rest the person's own way.
    let rest = match if who.turned { Stand::Hanging } else { who.stand } {
        Stand::Hanging | Stand::Pointing => [(V(17.0, 74.0), V(17.0, 54.0)), (V(-17.0, 74.0), V(-17.0, 54.0))],
        Stand::Folded => [(V(18.0, 76.0), V(-6.0, 80.0)), (V(-18.0, 76.0), V(6.0, 83.0))],
        Stand::Chin => [(V(18.0, 78.0), V(3.0, 99.0)), (V(-18.0, 76.0), V(6.0, 80.0))],
    };
    let up = |k: f32| {
        let pump = 4.0 * (t * 12.0 + k).sin();
        match kind {
            0 => (V(20.0 * k, 112.0), V(17.0 * k, 134.0 + pump * k)),
            1 => { let gap = 2.0 + 5.0 * (t * 16.0).sin().abs(); (V(19.0 * k, 78.0), V(gap * k, 92.0)) }
            2 if k > 0.0 => (V(22.0, 112.0), V(18.0 + 9.0 * (t * 8.0).sin(), 134.0)),
            3 => (V(23.0 * k, 110.0), V(10.0 * k, 120.0)),
            4 if k > 0.0 => (V(20.0, 114.0), V(16.0, 138.0)),
            _ => (V(18.0 * k, 76.0), V(17.0 * k, 55.0)),
        }
    };
    let arms: [(V, V); 2] = [0, 1].map(|j| {
        let k = if j == 0 { 1.0 } else { -1.0 };
        let (e0, h0) = rest[j];
        let (e1, h1) = up(k);
        (along(e0, e1, cheer), along(h0, h1, cheer))
    });
    // Neck, then the hair that hangs behind the shoulders.
    let r = head_r(who.bulk) * s;
    // Bowing, seen from the front: the head and shoulders dip toward the viewer.
    let head = pt(0.0, 108.0 - 9.0 * BOW.get());
    if who.hairdo == 2 {
        shape(blip, &[(V(head.0, head.1), V(head.0, head.1 + 1.3 * r), 1.0 * r, 0.9 * r)], hair, INK);
    }
    shape(blip, &[(pt(0.0, 92.0), pt(0.0, 101.0), 4.4 * b, 4.0 * b)], skin, INK);
    // The trunk square on: shoulders across the top, narrowing to the waist; trousers to the belt.
    shape(blip, &[(pt(-11.0, 91.0), pt(11.0, 91.0), 5.6 * b, 5.6 * b), (pt(0.0, 86.0), pt(0.0, 66.0), 13.0 * b, 10.0 * b)],
        shirt, INK);
    // Light from the front and to one side: the other side of the shirt in shade.
    stroke(blip, pt(-8.0, 86.0), pt(-6.5, 66.0), 4.0 * b, 3.2 * b, shade(shirt, 0.82));
    shape(blip, &[(pt(-5.0, 59.0), pt(5.0, 59.0), 7.6 * b, 7.6 * b)], trousers, INK);
    stroke(blip, pt(-9.5, 64.0), pt(9.5, 64.0), 2.2 * b, 2.2 * b, shoe);
    if !who.turned {
        stroke(blip, pt(0.0, 95.0), pt(0.0, 89.0), 3.4 * b, 0.8 * b, skin);
        stroke(blip, pt(0.0, 87.0), pt(0.0, 68.0), 0.6 * b, 0.6 * b, shade(shirt, 0.70));
    }
    for (j, &(elbow, hand)) in arms.iter().enumerate() {
        let side = if j == 0 { 1.0 } else { -1.0 };
        let (sh, el, ha) = (pt(side * 13.0, 91.0), pt(elbow.0, elbow.1), pt(hand.0, hand.1));
        shape(blip, &[(sh, el, 5.0 * b, 4.2 * b), (el, ha, 4.2 * b, 3.2 * b)], skin, INK);
        shape(blip, &[(sh, along(sh, el, 0.55), 5.8 * b, 5.0 * b)], shirt, INK);
        shape(blip, &[(ha, ha, 3.4 * b, 3.4 * b)], skin, INK);
    }
    // The head: ears, face, hair over the top.
    for side in [-1.0f32, 1.0] { shape(blip, &[(V(head.0 + side * 0.95 * r, head.1 + 0.05 * r), V(head.0 + side * 0.95 * r, head.1 + 0.05 * r), 0.22 * r, 0.22 * r)], skin, INK); }
    shape(blip, &[(V(head.0, head.1 - 0.1 * r), V(head.0, head.1 + 0.25 * r), 0.92 * r, 0.80 * r)], skin, INK);
    // Hair over the crown and down the sides to the ears, clear of the brows.
    let top = |grow: f32| [(V(head.0 - 0.40 * r, head.1 - 0.62 * r), V(head.0 + 0.40 * r, head.1 - 0.62 * r), (0.42 + grow) * r, (0.42 + grow) * r),
        (V(head.0 - 0.80 * r, head.1 - 0.45 * r), V(head.0 - 0.86 * r, head.1 - 0.05 * r), (0.22 + grow) * r, 0.16 * r),
        (V(head.0 + 0.80 * r, head.1 - 0.45 * r), V(head.0 + 0.86 * r, head.1 - 0.05 * r), (0.22 + grow) * r, 0.16 * r)];
    match who.hairdo {
        1 => {}
        3 => { let [a, b2, c] = top(0.0); shape(blip, &[a, b2, c, (V(head.0, head.1 - 1.12 * r), V(head.0, head.1 - 1.12 * r), 0.34 * r, 0.34 * r)], hair, INK); }
        4 => { shape(blip, &top(0.16), hair, INK); }
        _ => { shape(blip, &top(0.0), hair, INK); }
    }
    // From behind: the hair over the whole head (or the bare scalp), and nothing more.
    if who.turned {
        if who.hairdo != 1 { shape(blip, &[(V(head.0, head.1 - 0.15 * r), V(head.0, head.1 + 0.15 * r), 0.92 * r, 0.80 * r)], hair, INK); }
        if who.hairdo == 3 { shape(blip, &[(V(head.0, head.1 - 0.95 * r), V(head.0, head.1 - 0.95 * r), 0.32 * r, 0.32 * r)], hair, INK); }
        if let Some(c) = who.hat {
            let c = shade(rgb(c), tone);
            let crown = if who.straw { (1.7, 0.16) } else { (0.62, 0.52) };
            shape(blip, &[(V(head.0 - crown.0 * r, head.1 - 0.62 * r), V(head.0 + crown.0 * r, head.1 - 0.62 * r), crown.1 * r, crown.1 * r)], c, INK);
        }
        return;
    }
    // Eyes turned to the fight, a brow over each, a nose, a mouth.
    let look = ((focus - at.0) / 200.0).clamp(-1.0, 1.0) * 0.08 * r;
    let shout = cheer > 0.3;
    for side in [-1.0f32, 1.0] {
        let eye = V(head.0 + side * 0.36 * r, head.1 + 0.02 * r);
        if kind == 3 && shout {
            stroke(blip, V(eye.0 - 0.14 * r, eye.1), V(eye.0 + 0.14 * r, eye.1), 0.06 * r, 0.06 * r, INK);
        } else if (t * 0.37).fract() < 0.04 {
            stroke(blip, V(eye.0 - 0.14 * r, eye.1), V(eye.0 + 0.14 * r, eye.1), 0.05 * r, 0.05 * r, INK);
        } else {
            blip.fill_circle(eye.0, eye.1, 0.19 * r, BLIP_WHITE);
            blip.fill_circle(eye.0 + look, eye.1, 0.11 * r, INK);
        }
        let lift = if shout { 0.08 * r } else { 0.0 };
        stroke(blip, V(eye.0 - 0.16 * r, eye.1 - 0.30 * r - lift), V(eye.0 + 0.16 * r, eye.1 - 0.30 * r - lift + side * 0.02 * r),
            0.07 * r, 0.07 * r, shade(hair, 0.8));
    }
    stroke(blip, V(head.0, head.1 + 0.12 * r), V(head.0 + 0.04 * r, head.1 + 0.30 * r), 0.07 * r, 0.06 * r, shade(skin, 0.75));
    if shout {
        blip.fill_circle(head.0, head.1 + 0.52 * r, 0.17 * r, BlipColor { r: 0.36, g: 0.08, b: 0.10, a: 1.0 });
    } else {
        stroke(blip, V(head.0 - 0.15 * r, head.1 + 0.52 * r), V(head.0 + 0.15 * r, head.1 + 0.52 * r), 0.05 * r, 0.05 * r, INK);
    }
    // The hat, square on.
    if let Some(c) = who.hat {
        let c = shade(rgb(c), tone);
        if who.straw {
            shape(blip, &[(V(head.0 - 1.7 * r, head.1 - 0.55 * r), V(head.0 + 1.7 * r, head.1 - 0.55 * r), 0.16 * r, 0.16 * r),
                          (V(head.0, head.1 - 1.25 * r), V(head.0, head.1 - 0.68 * r), 0.15 * r, 0.80 * r)], c, INK);
        } else {
            // The crown above the brows, and the peak as a band just over them.
            shape(blip, &[(V(head.0 - 0.40 * r, head.1 - 0.80 * r), V(head.0 + 0.40 * r, head.1 - 0.80 * r), 0.48 * r, 0.48 * r)], c, INK);
            shape(blip, &[(V(head.0 - 0.72 * r, head.1 - 0.50 * r), V(head.0 + 0.72 * r, head.1 - 0.50 * r), 0.11 * r, 0.11 * r)],
                shade(c, 0.8), INK);
        }
    }
}

/// THE PAGODA: DRAKE's home, a nod to the five-storey tower of Game of Death: a pagoda of red and gold at dusk,
/// the moon over the mountains, a courtyard wall with a moon gate, lanterns on a line, an incense burner smoking
/// and a wooden dummy by the gate.
pub(crate) fn draw_pagoda(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 258.0;
    let c = |r, g, b| BlipColor { r, g, b, a: 1.0 };
    // Dusk: indigo overhead, the last of the light red along the mountains.
    let sky_at = |k: f32| BlipColor { r: 0.06 + 0.40 * k * k, g: 0.05 + 0.15 * k * k, b: 0.15 + 0.12 * k, a: 1.0 };
    blip.fill_bands(0.0, shake, WIN_W as f32, HORIZON, 16, sky_at);
    let low = sky_at(1.0);
    blip.fill_rect(0.0, HORIZON + shake, WIN_W as f32, FLOOR_Y - HORIZON, low);
    for i in 0..30 {
        let x = scatter(i, 0x8a) * WIN_W as f32;
        let y = scatter(i, 0x8b) * 120.0 + 6.0 + shake;
        let tw = 0.5 + 0.5 * (t * 1.3 + i as f32).sin();
        blip.fill_rect(x, y, 1.0, 1.0, BlipColor { r: 1.0, g: 0.95, b: 0.85, a: 0.25 + 0.5 * tw });
    }
    // A big low moon, hazed gold.
    let (mx, my) = (508.0, 118.0 + shake);
    for r in [62.0f32, 48.0] { blip.fill_circle(mx, my, r, BlipColor { r: 1.0, g: 0.86, b: 0.62, a: 0.10 }); }
    blip.fill_circle(mx, my, 34.0, c(0.99, 0.90, 0.68));
    blip.fill_circle(mx + 9.0, my - 6.0, 6.0, c(0.93, 0.82, 0.60));
    blip.fill_circle(mx - 12.0, my + 9.0, 4.0, c(0.93, 0.82, 0.60));
    // Two ranges of mountains and the mist lying between them.
    for (base, amp, k, step, salt) in [(HORIZON - 40.0, 78.0f32, 0.62f32, 140.0f32, 0x8c), (HORIZON - 12.0, 52.0, 0.40, 104.0, 0x8d)] {
        let col = hazed(c(0.12, 0.08, 0.18), low, k);
        let n = (WIN_W as f32 / step) as usize + 2;
        for i in 0..n {
            let x = i as f32 * step - 50.0;
            let pk = amp * (0.55 + 0.45 * scatter(i, salt));
            let rows = 10;
            for r in 0..rows {
                let f = r as f32 / rows as f32;
                let w = step * 0.95 * (1.0 - f);
                blip.fill_rect(x + step * 0.75 - w * 0.5, base + shake - pk * f - pk / rows as f32, w, pk / rows as f32 + 1.0, col);
            }
        }
        blip.fill_rect(0.0, base + shake - 6.0, WIN_W as f32, 5.0, BlipColor { a: 0.35, ..low });
    }
    // Pines either side, layered tiers of needles.
    for (x, hgt) in [(40.0f32, 120.0f32), (92.0, 96.0), (566.0, 112.0), (612.0, 90.0)] {
        let pine = c(0.07, 0.08, 0.10);
        blip.fill_rect(x - 2.0, HORIZON - hgt * 0.3 + shake, 4.0, hgt * 0.3, pine);
        for tier in 0..5 {
            let k = tier as f32 / 5.0;
            let w = 46.0 * (1.0 - k * 0.75);
            blip.fill_rect(x - w / 2.0, HORIZON - hgt * (0.25 + 0.15 * tier as f32) + shake, w, 12.0, pine);
        }
    }

    // The pagoda: five storeys, each narrower, red walls lit from inside, black eaves turned up at the ends
    // and edged in gold, a spire of rings on top.
    let (cx, base) = (320.0, HORIZON + 8.0 + shake);
    let wall = c(0.50, 0.10, 0.10);
    let eave = c(0.10, 0.08, 0.10);
    let gold = c(0.92, 0.70, 0.28);
    let mut y = base;
    for storey in 0..5 {
        let w = 132.0 - storey as f32 * 20.0;
        let tall = 30.0 - storey as f32 * 2.0;
        blip.fill_rect(cx - w / 2.0, y - tall, w, tall, wall);
        blip.fill_rect(cx - w / 2.0, y - tall, 5.0, tall, shade(wall, 0.7));
        blip.fill_rect(cx + w / 2.0 - 5.0, y - tall, 5.0, tall, shade(wall, 0.7));
        // Windows lit amber, one flickering now and then.
        let n = (w / 20.0) as usize;
        for k in 0..n {
            let flick = if (t * 0.7 + (storey * 7 + k) as f32).sin() > 0.95 { 0.6 } else { 1.0 };
            let (wx, wy, wh) = (cx - w / 2.0 + 10.0 + k as f32 * 20.0, y - tall + 8.0, tall - 14.0);
            blip.fill_rect(wx, wy, 8.0, wh, BlipColor { r: 1.0 * flick, g: 0.72 * flick, b: 0.34 * flick, a: 1.0 });
            // On each floor one window shows its master at practice: a shadow on the paper, stepping and striking.
            if k == (storey * 3 + 1) % n.max(1) {
                let step = (t * (1.6 + 0.3 * storey as f32) + storey as f32).sin();
                let sx = wx + 4.0 + 1.5 * step;
                let shadow = BlipColor { r: 0.30, g: 0.12, b: 0.08, a: 0.85 };
                blip.fill_circle(sx, wy + 2.5, 1.6, shadow);
                blip.fill_rect(sx - 1.5, wy + 4.0, 3.0, wh * 0.45, shadow);
                blip.fill_rect(sx - 1.5, wy + 4.0 + wh * 0.45, 1.2, wh * 0.45 - 2.0, shadow);
                blip.fill_rect(sx + 0.3, wy + 4.0 + wh * 0.45, 1.2, wh * 0.45 - 2.0, shadow);
                // An arm thrown out now and then.
                if step > 0.5 { blip.fill_rect(sx + 1.0, wy + 5.0, 3.0, 1.2, shadow); }
            }
        }
        // The eave: a wide roof slab, its ends swept up.
        let ew = w + 34.0;
        let ey = y - tall - 6.0;
        blip.fill_rect(cx - ew / 2.0, ey, ew, 7.0, eave);
        for side in [-1.0f32, 1.0] {
            let tip = V(cx + side * (ew / 2.0 + 8.0), ey - 7.0);
            stroke(blip, V(cx + side * (ew / 2.0 - 10.0), ey + 3.0), tip, 3.5, 1.5, eave);
            blip.fill_circle(tip.0, tip.1, 1.6, gold);
        }
        blip.fill_rect(cx - ew / 2.0, ey + 6.0, ew, 1.0, gold);
        y = ey;
    }
    // The spire.
    blip.fill_rect(cx - 1.5, y - 30.0, 3.0, 30.0, eave);
    for r in 0..5 { blip.fill_rect(cx - 5.0 + r as f32 * 0.6, y - 8.0 - r as f32 * 5.0, 10.0 - r as f32 * 1.2, 2.0, gold); }
    blip.fill_circle(cx, y - 33.0, 3.0, gold);
    // The students watching from the courtyard steps.
    draw_crowd(blip, HORIZON + 10.0 + shake, 26.0, hazed(c(0.12, 0.10, 0.14), low, 0.12), t, 0xd1e, 26, roar);

    // The courtyard wall: plaster under a tiled cap, the moon gate cut through it on the left.
    let plaster = c(0.62, 0.52, 0.44);
    let top = HORIZON + 16.0 + shake;
    blip.fill_rect(0.0, top, WIN_W as f32, FLOOR_Y + shake - top, plaster);
    blip.fill_rect(0.0, top - 6.0, WIN_W as f32, 7.0, c(0.16, 0.12, 0.14));
    for i in 0..33 { blip.fill_rect(i as f32 * 20.0, top - 6.0, 16.0, 3.0, c(0.26, 0.20, 0.22)); }
    blip.fill_rect(0.0, FLOOR_Y + shake - 8.0, WIN_W as f32, 8.0, shade(plaster, 0.7));
    let gate = V(170.0, top + 14.0);
    blip.fill_circle(gate.0, gate.1, 18.0, c(0.22, 0.14, 0.16));
    blip.fill_circle(gate.0, gate.1, 15.0, sky_at(0.9));
    blip.fill_rect(gate.0 - 15.0, gate.1, 30.0, FLOOR_Y + shake - gate.1, sky_at(0.9));
    // Lanterns on a line across the top of the stage, swaying out of step.
    // (Below the health bars, which cover the top 60 px.)
    let line = 104.0 + shake;
    blip.draw_line(0.0, line, WIN_W as f32, line + 6.0, c(0.15, 0.10, 0.10));
    for i in 0..7 {
        let x = 46.0 + i as f32 * 92.0;
        // A big blow sets the crowd off and the line swings with it, dying away over a second.
        let since = t - roar.at;
        let jolt = if since >= 0.0 { roar.loud * 20.0 * (-since * 2.5).exp() } else { 0.0 };
        let sway = (t * (1.2 + jolt * 0.3) + i as f32 * 1.7).sin() * (2.5 + jolt);
        let y = line + 3.0 + (x / WIN_W as f32) * 6.0;
        let flick = 0.86 + 0.14 * (t * 8.0 + i as f32 * 2.3).sin();
        blip.fill_glow_circle(x + sway, y + 12.0, 20.0, BlipColor { r: 1.0, g: 0.62, b: 0.28, a: 0.18 * flick });
        blip.fill_rect(x + sway - 6.0, y + 4.0, 12.0, 16.0, c(0.82, 0.14, 0.12));
        blip.fill_rect(x + sway - 4.0, y + 6.0, 8.0, 12.0, BlipColor { r: 1.0, g: 0.62 * flick, b: 0.30, a: 1.0 });
        blip.fill_rect(x + sway - 7.0, y + 3.0, 14.0, 2.0, gold);
        blip.fill_rect(x + sway - 7.0, y + 19.0, 14.0, 2.0, gold);
    }
    // The bronze incense burner on its legs, smoke curling up and away.
    let (bx, by) = (500.0, FLOOR_Y + shake - 10.0);
    let bronze = c(0.42, 0.30, 0.16);
    blip.fill_rect(bx - 16.0, by - 22.0, 32.0, 18.0, bronze);
    blip.fill_rect(bx - 19.0, by - 25.0, 38.0, 4.0, shade(bronze, 1.3));
    for lx in [-13.0f32, 11.0] { blip.fill_rect(bx + lx, by - 4.0, 3.0, 8.0, shade(bronze, 0.7)); }
    for k in 0..6 {
        let age = (t * 0.25 + k as f32 / 6.0).fract();
        let sx = bx + (age * 9.0).sin() * 6.0 + age * 14.0;
        blip.fill_circle(sx, by - 28.0 - age * 70.0, 2.0 + age * 6.0, BlipColor { r: 0.80, g: 0.76, b: 0.82, a: 0.35 * (1.0 - age) });
    }
    // The wooden dummy by the gate: a post, three arms and a leg.
    let (dx, dy) = (118.0, FLOOR_Y + shake - 6.0);
    let wood = c(0.40, 0.24, 0.12);
    blip.fill_rect(dx - 5.0, dy - 52.0, 10.0, 52.0, wood);
    blip.fill_rect(dx - 5.0, dy - 52.0, 3.0, 52.0, shade(wood, 1.25));
    for (ay, ang) in [(dy - 44.0, -0.25f32), (dy - 44.0, 0.25), (dy - 34.0, 0.0)] {
        let (ex, ey) = (dx + 14.0 * ang.cos().signum() * 1.0, ay + 14.0 * ang.sin());
        stroke(blip, V(dx, ay), V(ex + 4.0, ey), 1.6, 1.4, shade(wood, 0.85));
    }
    stroke(blip, V(dx, dy - 18.0), V(dx + 12.0, dy - 10.0), 2.0, 1.6, shade(wood, 0.85));
}

/// THE ISLAND: a karate tournament's grounds at noon, after Enter the Dragon's: hills and palms, a fortress wall
/// hung with yellow banners, the host on a canopied dais, and ranks of entrants in white along the wall.
pub(crate) fn draw_island(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 250.0;
    let c = |r, g, b| BlipColor { r, g, b, a: 1.0 };
    let sky_at = |k: f32| BlipColor { r: 0.46 + 0.36 * k, g: 0.66 + 0.24 * k, b: 0.92 - 0.02 * k, a: 1.0 };
    blip.fill_bands(0.0, shake, WIN_W as f32, HORIZON, 14, sky_at);
    let low = sky_at(1.0);
    blip.fill_rect(0.0, HORIZON + shake, WIN_W as f32, FLOOR_Y - HORIZON, low);
    // Clouds drifting.
    for i in 0..4 {
        let sx = scatter(i, 0x9a1);
        let x = (t * (3.0 + sx * 3.0) + sx * 700.0) % (WIN_W as f32 + 200.0) - 100.0;
        let y = 70.0 + sx * 60.0 + shake;
        for (dx, r) in [(0.0f32, 14.0f32), (16.0, 18.0), (34.0, 13.0)] {
            blip.fill_circle(x + dx, y, r, BlipColor { r: 1.0, g: 1.0, b: 1.0, a: 0.85 });
        }
    }
    // Green hills behind the wall.
    for (base, amp, k, step, salt) in [(HORIZON - 30.0, 70.0f32, 0.45f32, 150.0f32, 0x9a2), (HORIZON - 10.0, 46.0, 0.20, 110.0, 0x9a3)] {
        let col = hazed(c(0.20, 0.42, 0.22), low, k);
        let n = (WIN_W as f32 / step) as usize + 2;
        for i in 0..n {
            let x = i as f32 * step - 40.0;
            let pk = amp * (0.6 + 0.4 * scatter(i, salt));
            for r in 0..10 {
                let f = r as f32 / 10.0;
                let w = step * 1.1 * (1.0 - f * f);
                blip.fill_rect(x + step * 0.6 - w * 0.5, base + shake - pk * f - pk / 10.0, w, pk / 10.0 + 1.0, col);
            }
        }
    }
    // Palms either side, swaying a little.
    for (x, hgt, lean) in [(44.0f32, 120.0f32, 0.18f32), (96.0, 96.0, -0.10), (560.0, 112.0, -0.16), (606.0, 92.0, 0.12)] {
        let trunk = c(0.42, 0.32, 0.20);
        let sway = (t * 0.9 + x).sin() * 2.0;
        let top = V(x + lean * hgt + sway, HORIZON - hgt + shake);
        stroke(blip, V(x, HORIZON + 6.0 + shake), top, 3.0, 2.0, trunk);
        let leaf = c(0.18, 0.46, 0.20);
        for k in 0..6 {
            let a = k as f32 / 6.0 * std::f32::consts::TAU + 0.3;
            let tip = V(top.0 + a.cos() * 26.0, top.1 + a.sin() * 10.0 + 8.0);
            stroke(blip, top, tip, 2.6, 1.0, leaf);
        }
    }
    // The fortress wall, grey stone in courses, crenellated along the top.
    let stone = c(0.58, 0.56, 0.52);
    let top = HORIZON - 64.0 + shake;
    blip.fill_rect(120.0, top, 400.0, HORIZON + 10.0 + shake - top, stone);
    for i in 0..20 { if i % 2 == 0 { blip.fill_rect(120.0 + i as f32 * 20.0, top - 10.0, 20.0, 10.0, stone); } }
    for course in 0..6 {
        let y = top + 4.0 + course as f32 * 12.0;
        blip.fill_rect(120.0, y, 400.0, 1.0, shade(stone, 0.75));
        let off = if course % 2 == 0 { 0.0 } else { 15.0 };
        for i in 0..14 { blip.fill_rect(124.0 + off + i as f32 * 30.0, y, 1.0, 12.0, shade(stone, 0.80)); }
    }
    // The host's dais in the middle, under a red canopy: a dark figure on a carved chair.
    let (dx, dy) = (320.0, HORIZON - 4.0 + shake);
    blip.fill_rect(dx - 46.0, dy - 8.0, 92.0, 12.0, c(0.46, 0.20, 0.14));
    blip.fill_rect(dx - 40.0, dy - 70.0, 80.0, 8.0, c(0.74, 0.12, 0.10));
    for side in [-1.0f32, 1.0] { blip.fill_rect(dx + side * 36.0 - 2.0, dy - 64.0, 4.0, 58.0, c(0.30, 0.16, 0.10)); }
    blip.fill_rect(dx - 12.0, dy - 44.0, 24.0, 36.0, c(0.66, 0.50, 0.18));
    blip.fill_circle(dx, dy - 34.0, 5.0, c(0.80, 0.64, 0.48));
    blip.fill_rect(dx - 7.0, dy - 28.0, 14.0, 20.0, c(0.10, 0.10, 0.12));
    // Yellow banners along the wall, a black device on each, rippling.
    for i in 0..8 {
        let x = 132.0 + i as f32 * 52.0;
        if (x - dx).abs() < 54.0 { continue; }
        let pole = c(0.30, 0.22, 0.14);
        blip.fill_rect(x, top - 40.0, 2.0, 60.0, pole);
        let ripple = (t * 3.0 + i as f32).sin() * 2.0;
        blip.fill_rect(x + 2.0, top - 38.0 + ripple * 0.3, 18.0, 24.0, c(0.98, 0.82, 0.14));
        blip.fill_circle(x + 11.0, top - 26.0 + ripple * 0.3, 4.0, c(0.10, 0.10, 0.10));
    }
    // The entrants in their ranks along the foot of the wall.
    draw_crowd(blip, HORIZON + 10.0 + shake, 24.0, hazed(c(0.86, 0.86, 0.84), low, 0.10), t, 0xe17, 30, roar);
    // The courtyard: sandy ground from the wall down to the fighting floor, darker where it is trodden.
    let sand = c(0.80, 0.70, 0.50);
    blip.fill_rect(0.0, HORIZON + 10.0 + shake, WIN_W as f32, FLOOR_Y - HORIZON - 10.0, sand);
    for i in 0..30usize {
        let x = scatter(i, 0x9a7) * WIN_W as f32;
        let y = HORIZON + 14.0 + scatter(i, 0x9a8) * (FLOOR_Y - HORIZON - 18.0) + shake;
        blip.fill_rect(x, y, 8.0 + 14.0 * scatter(i, 0x9a9), 1.0, shade(sand, 0.86));
    }
}

/// THE COLOSSEUM, after Way of the Dragon's last fight: inside the old walls, brick in the shade and arches open on
/// the sunlit tiers beyond, the stone floor scattered with rubble, nobody watching but a kitten on a block, its head
/// following the fight (`focus`).
pub(crate) fn draw_colosseum(blip: &Blip, shake: f32, t: f32, focus: f32, roar: Roar) {
    let c = |r, g, b| BlipColor { r, g, b, a: 1.0 };
    let w = WIN_W as f32;
    let top = FLOOR_Y + shake;
    // The sky over the far side, seen through the arches.
    blip.fill_bands(0.0, shake, w, 220.0, 10, |k| c(0.38 + 0.30 * k, 0.60 + 0.22 * k, 0.86 + 0.06 * k));
    // The far side of the bowl: tier on tier of sunlit arcades, paler with distance.
    let stone = c(0.86, 0.74, 0.56);
    for tier in 0..4 {
        let y = 92.0 + tier as f32 * 34.0 + shake;
        let tone = hazed(stone, c(0.70, 0.80, 0.90), 0.35 - tier as f32 * 0.08);
        blip.fill_rect(0.0, y, w, 34.0, tone);
        for i in 0..32 {
            let x = i as f32 * 21.0 + if tier % 2 == 0 { 0.0 } else { 10.0 };
            blip.fill_rect(x + 5.0, y + 9.0, 9.0, 20.0, shade(tone, 0.62));
            blip.fill_circle(x + 9.5, y + 9.0, 4.5, shade(tone, 0.62));
        }
        blip.fill_rect(0.0, y, w, 3.0, shade(tone, 0.85));
    }
    // The arena floor beyond, in the sun, and the shadow along the foot of the far tiers.
    blip.fill_rect(0.0, 228.0 + shake, w, FLOOR_Y - 228.0, c(0.84, 0.74, 0.58));
    blip.fill_rect(0.0, 228.0 + shake, w, 8.0, c(0.62, 0.52, 0.40));
    // The broken top edge of the far wall against the sky.
    for i in 0..16 {
        let x = i as f32 * 40.0;
        let h = 6.0 + 18.0 * scatter(i, 0xc01);
        blip.fill_rect(x, 92.0 - h + shake, 40.0, h, hazed(stone, c(0.70, 0.80, 0.90), 0.4));
    }
    // The near wall: old brick in shadow, three great arches cut through it, the top crumbling.
    let brick = c(0.46, 0.26, 0.20);
    let mortar = shade(brick, 0.78);
    let arches = [(110.0f32, 62.0f32), (320.0, 74.0), (530.0, 62.0)];
    let wall_top = 40.0 + shake;
    // Wall in vertical strips, leaving the arch openings.
    let mut x = 0.0;
    while x < w {
        let mut open: Option<(f32, f32)> = None;
        for &(ax, ar) in &arches { if (x + 2.0 - ax).abs() < ar { open = Some((ax, ar)); } }
        let crumble = 10.0 * scatter((x / 4.0) as usize, 0xc02) + if (x / 80.0) as usize % 3 == 1 { 22.0 } else { 0.0 };
        let y0 = wall_top + crumble;
        match open {
            Some((ax, ar)) => {
                // Above the round head of the arch only.
                let dx = (x + 2.0 - ax) / ar;
                let spring = top - 150.0;
                let head = spring - ar * (1.0 - dx * dx).max(0.0).sqrt();
                if head > y0 { blip.fill_rect(x, y0, 4.0, head - y0, brick); }
            }
            None => blip.fill_rect(x, y0, 4.0, top - y0, brick),
        }
        x += 4.0;
    }
    // Courses of mortar and the brick joints, only on the brick.
    for row in 0..40 {
        let y = wall_top + 14.0 + row as f32 * 7.0;
        if y > top { break; }
        let off = if row % 2 == 0 { 0.0 } else { 7.0 };
        let mut x = 0.0;
        while x < w {
            let inside = arches.iter().any(|&(ax, ar)| {
                let dx = (x - ax) / ar;
                dx.abs() < 1.0 && y > top - 150.0 - ar * (1.0 - dx * dx).max(0.0).sqrt()
            });
            if !inside {
                blip.fill_rect(x, y, 14.0, 1.0, mortar);
                blip.fill_rect(x + off, y - 6.0, 1.0, 6.0, mortar);
            }
            x += 14.0;
        }
    }
    // Weathering: dark gaps where bricks have fallen out, a few cracks, weeds at the foot of the wall.
    for i in 0..14usize {
        let x = scatter(i, 0xc21) * w;
        let y = wall_top + 30.0 + scatter(i, 0xc22) * (top - wall_top - 60.0);
        let inside = arches.iter().any(|&(ax, ar)| (x - ax).abs() < ar + 8.0 && y > top - 150.0 - ar);
        if inside { continue; }
        if i % 3 == 0 {
            let (cx, cy) = (x, y);
            stroke(blip, V(cx, cy), V(cx + 5.0, cy + 9.0), 0.6, 0.6, shade(brick, 0.55));
            stroke(blip, V(cx + 5.0, cy + 9.0), V(cx + 2.0, cy + 17.0), 0.6, 0.6, shade(brick, 0.55));
        } else {
            blip.fill_rect(x, y, 13.0, 6.0, shade(brick, 0.58));
        }
    }
    for i in 0..10usize {
        let x = scatter(i, 0xc23) * w;
        let green = BlipColor { r: 0.34, g: 0.46, b: 0.22, a: 1.0 };
        for k in 0..3 {
            let lean = (k as f32 - 1.0) * 2.5 + (t * 0.8 + i as f32).sin() * 0.6;
            stroke(blip, V(x + k as f32 * 2.0, top), V(x + k as f32 * 2.0 + lean, top - 5.0 - 3.0 * scatter(i * 3 + k, 0xc24)), 0.7, 0.4, green);
        }
    }
    // Each arch's rim of dressed stone, and its shadowed reveal.
    for &(ax, ar) in &arches {
        let spring = top - 150.0;
        for k in 0..24 {
            let a0 = std::f32::consts::PI * k as f32 / 24.0;
            let a1 = std::f32::consts::PI * (k + 1) as f32 / 24.0;
            let p = |a: f32, r: f32| V(ax - a.cos() * r, spring - a.sin() * r);
            stroke(blip, p(a0, ar + 4.0), p(a1, ar + 4.0), 4.0, 4.0, c(0.62, 0.50, 0.38));
            stroke(blip, p(a0, ar - 2.0), p(a1, ar - 2.0), 2.0, 2.0, shade(brick, 0.60));
        }
        for side in [-1.0f32, 1.0] {
            blip.fill_rect(ax + side * (ar + 2.0) - 4.0, spring, 8.0, 150.0, c(0.62, 0.50, 0.38));
            blip.fill_rect(ax + side * (ar - 3.0) - 2.0, spring, 4.0, 150.0, shade(brick, 0.60));
        }
    }
    // Sun through each arch, slanting down onto the floor, soft.
    for &(ax, ar) in &arches {
        let spring = top - 150.0;
        let sun = BlipColor { r: 1.0, g: 0.90, b: 0.66, a: 0.10 };
        for k in 0..6 {
            let x0 = ax - ar * 0.8 + k as f32 * ar * 0.3;
            stroke(blip, V(x0, spring - ar * 0.5), V(x0 + 70.0, top), 8.0, 14.0, sun);
        }
    }
    // Blocks of fallen stone along the foot of the wall, and the kitten on the left one.
    let block = c(0.58, 0.54, 0.48);
    // (Inside x 42 to 598, what the closest camera keeps.)
    for (bx, bw, bh) in [(52.0f32, 50.0f32, 30.0f32), (250.0, 30.0, 14.0), (556.0, 40.0, 24.0)] {
        blip.fill_rect(bx - 1.0, top - bh - 1.0, bw + 2.0, bh + 2.0, INK);
        blip.fill_rect(bx, top - bh, bw, bh, block);
        blip.fill_rect(bx, top - bh, bw, 3.0, shade(block, 1.12));
        blip.fill_rect(bx + bw * 0.6, top - bh + 6.0, 1.0, bh - 8.0, shade(block, 0.8));
    }
    // A big blow startles it: up off its haunches for half a second, then settling (the crowd's roar, with no crowd).
    let since = t - roar.at;
    let startle = if (0.0..0.6).contains(&since) { roar.loud.min(1.0) * (1.0 - since / 0.6) } else { 0.0 };
    draw_kitten(blip, V(78.0, top - 30.0 - 3.0 * startle), t, focus, startle);
}

/// The kitten that watches the fight at the Colosseum: white with grey patches, sitting up, its tail curling, its
/// head turning to follow the fight and its eyes blinking.
fn draw_kitten(blip: &Blip, at: V, t: f32, focus: f32, startle: f32) {
    let white = BlipColor { r: 0.94, g: 0.92, b: 0.88, a: 1.0 };
    let grey = BlipColor { r: 0.46, g: 0.44, b: 0.44, a: 1.0 };
    let look = ((focus - at.0) / 240.0).clamp(-1.0, 1.0);
    // (1.6 times the first drawing: at 14 px tall it was lost against the block.)
    let z = 1.6;
    let at_ = at;
    let p = |dx: f32, dy: f32| V(at_.0 + dx * z, at_.1 + dy * z);
    // Body, sitting.
    shape(blip, &[(p(0.0, -6.0), p(1.0, -2.0), 5.0 * z, 6.0 * z)], white, INK);
    blip.fill_circle(p(2.0, -4.0).0, p(2.0, -4.0).1, 2.4 * z, grey);
    // Tail curled round, the tip lifting now and then.
    let flick = (t * 1.3).sin().max(0.0) * 3.0 + 6.0 * startle;
    stroke(blip, p(-5.0, -1.0), p(-9.0, -4.0 - flick), 1.6 * z, 1.2 * z, INK);
    stroke(blip, p(-5.0, -1.0), p(-9.0, -4.0 - flick), 1.0 * z, 0.7 * z, grey);
    // Head, turned toward the fight.
    let h = p(1.0 + 2.0 * look, -14.0);
    shape(blip, &[(h, h, 4.2 * z, 4.2 * z)], white, INK);
    blip.fill_circle(h.0 + (-1.5 + look) * z, h.1 - 2.0 * z, 2.0 * z, grey);
    for side in [-1.0f32, 1.0] {
        let ear = V(h.0 + (side * 2.6 + look) * z, h.1 - 3.6 * z);
        stroke(blip, V(ear.0, ear.1 + 1.5 * z), V(ear.0 + side * 0.4 * z, ear.1 - 1.6 * z), 1.6 * z, 0.4 * z, INK);
        stroke(blip, V(ear.0, ear.1 + 1.5 * z), V(ear.0 + side * 0.4 * z, ear.1 - 1.2 * z), 1.0 * z, 0.3 * z, if side < 0.0 { grey } else { white });
    }
    // Eyes, shut for a blink every few seconds.
    let blink = (t * 0.37).fract() < 0.04;
    for side in [-1.0f32, 1.0] {
        let e = V(h.0 + (side * 1.6 + 1.2 * look) * z, h.1 - 0.4 * z);
        if blink { blip.fill_rect(e.0 - 1.2, e.1, 2.4, 1.0, INK); }
        else { blip.fill_rect(e.0 - 1.0, e.1 - 1.2, 2.0, 2.4, BlipColor { r: 0.30, g: 0.50, b: 0.30, a: 1.0 }); }
    }
}

/// THE HOOD: a warm dusk block, fire escapes, murals and lowrider lights.
fn draw_hood(blip: &Blip, shift: f32, now: f32) {
    let c = |r, g, b| BlipColor { r, g, b, a: 1.0 };
    blip.fill_rect(0.0, 0.0, WIN_W as f32, FLOOR_Y + shift, c(0.20, 0.12, 0.18));
    blip.fill_circle(500.0, 94.0 + shift, 45.0, c(0.91, 0.42, 0.28));
    for i in 0..9 {
        let x = i as f32 * 78.0 - 10.0;
        let h = 88.0 + scatter(i, 0x2f) * 105.0;
        let w = 66.0 + scatter(i, 0x31) * 22.0;
        let y = FLOOR_Y - h + shift;
        blip.fill_rect(x, y, w, h, c(0.13, 0.12, 0.19));
        blip.fill_rect(x + 4.0, y + 5.0, w - 8.0, 4.0, c(0.38, 0.21, 0.22));
        for row in 0..4 {
            for col in 0..3 {
                let glow = (now * 0.8 + (i * 13 + row * 7 + col) as f32).sin() > -0.45;
                if glow {
                    blip.fill_rect(x + 10.0 + col as f32 * 16.0, y + 20.0 + row as f32 * 22.0,
                        6.0, 9.0, c(0.88, 0.55, 0.28));
                }
            }
        }
    }
    // Painted block letters and a metal fire escape give the skyline its scale.
    let wall = c(0.18, 0.15, 0.21);
    blip.fill_rect(0.0, 188.0 + shift, WIN_W as f32, 56.0, wall);
    for y in [198.0f32, 220.0] {
        stroke(blip, V(25.0, y + shift), V(256.0, y + shift), 1.5, 1.5, c(0.55, 0.30, 0.34));
    }
    for x in [45.0f32, 118.0, 191.0, 264.0] {
        stroke(blip, V(x, 186.0 + shift), V(x, 235.0 + shift), 1.2, 1.2, c(0.62, 0.44, 0.40));
    }
    for (i, col) in [(0, c(0.95, 0.28, 0.30)), (1, c(0.98, 0.69, 0.25)), (2, c(0.25, 0.78, 0.75))] {
        let x = 324.0 + i as f32 * 28.0;
        stroke(blip, V(x, 214.0 + shift), V(x + 10.0, 198.0 + shift), 3.0, 3.0, col);
        stroke(blip, V(x + 10.0, 198.0 + shift), V(x + 20.0, 214.0 + shift), 3.0, 3.0, col);
    }
    // A parked lowrider silhouette, with softly pulsing tail lamps.
    let bob = (now * 1.4).sin() * 1.0;
    blip.fill_rect(432.0, 226.0 + shift + bob, 132.0, 18.0, c(0.10, 0.10, 0.15));
    blip.fill_rect(454.0, 214.0 + shift + bob, 65.0, 14.0, c(0.12, 0.12, 0.18));
    blip.fill_rect(463.0, 216.0 + shift + bob, 22.0, 9.0, c(0.45, 0.61, 0.66));
    blip.fill_rect(489.0, 216.0 + shift + bob, 24.0, 9.0, c(0.45, 0.61, 0.66));
    for x in [456.0, 534.0] {
        blip.fill_circle(x, 243.0 + shift, 9.0, c(0.06, 0.06, 0.08));
        blip.fill_circle(x, 243.0 + shift, 4.0, c(0.59, 0.56, 0.51));
    }
    blip.fill_rect(554.0, 231.0 + shift, 5.0, 4.0, c(0.96, 0.17, 0.14));
}

/// Something small and alive in front of the air of three stages: gulls
/// over the docks, petals in the temple yard, fireflies by the river.
pub(crate) fn draw_life(blip: &Blip, stage: usize, shift: f32, now: f32) {
    let w = WIN_W as f32;
    match stage {
        // The Colosseum: dust turning slowly in the sun that slants in through the arches.
        10 => for i in 0..26 {
            let x = (scatter(i, 0xc11) * w + (now * (3.0 + 4.0 * scatter(i, 0xc12)) + i as f32).sin() * 14.0).rem_euclid(w);
            let y = 90.0 + scatter(i, 0xc13) * 190.0 + (now * 0.4 + i as f32).sin() * 8.0 + shift;
            let glint = 0.25 + 0.35 * ((now * (0.7 + scatter(i, 0xc14)) + i as f32).sin() * 0.5 + 0.5);
            blip.fill_rect(x, y, 1.5, 1.5, BlipColor { r: 1.0, g: 0.94, b: 0.80, a: glint });
        },
        0 => for i in 0..3 {
            // Each gull glides across on its own line and beats its wings.
            let x = (now * (14.0 + 5.0 * i as f32) + scatter(i, 0xe1) * 900.0).rem_euclid(w + 80.0) - 40.0;
            let y = 70.0 + scatter(i, 0xe2) * 60.0 + (now * 0.7 + i as f32).sin() * 6.0 + shift;
            let flap = (now * 5.0 + i as f32 * 2.0).sin() * 3.5;
            let bird = BlipColor { r: 0.16, g: 0.10, b: 0.14, a: 1.0 };
            stroke(blip, V(x - 7.0, y - flap), V(x, y), 0.9, 1.1, bird);
            stroke(blip, V(x + 7.0, y - flap), V(x, y), 0.9, 1.1, bird);
        },
        1 => for i in 0..16 {
            // Petals slide down and across, turning as they fall.
            let fall = 16.0 + scatter(i, 0xe4) * 14.0;
            let y = (scatter(i, 0xe5) * FLOOR_Y + now * fall).rem_euclid(FLOOR_Y);
            let x = (scatter(i, 0xe6) * w + now * 9.0 + (now * 1.3 + i as f32).sin() * 12.0).rem_euclid(w);
            let turn = (now * 3.0 + i as f32 * 1.7).sin();
            blip.fill_rect(x, y + shift, 2.0 + turn.abs() * 1.5, 2.0,
                BlipColor { r: 0.98, g: 0.72, b: 0.80, a: 1.0 });
        },
        4 => for i in 0..12 {
            // Fireflies wander and wink.
            let x = scatter(i, 0xe7) * w + (now * 0.6 + i as f32 * 2.1).sin() * 18.0;
            let y = 150.0 + scatter(i, 0xe8) * 150.0 + (now * 0.9 + i as f32).cos() * 10.0 + shift;
            if (now * 1.4 + scatter(i, 0xe9) * 6.0).sin() > 0.1 {
                blip.fill_rect(x, y, 2.0, 2.0, BlipColor { r: 0.92, g: 1.0, b: 0.50, a: 1.0 });
            }
        },
        // Maple leaves drifting down across the pagoda's courtyard, turning as they go.
        8 => for i in 0..10 {
            let fall = 12.0 + scatter(i, 0xf1) * 10.0;
            let y = (scatter(i, 0xf2) * FLOOR_Y + now * fall).rem_euclid(FLOOR_Y);
            let x = (scatter(i, 0xf3) * w - now * 14.0 + (now * 1.1 + i as f32).sin() * 16.0).rem_euclid(w);
            let turn = (now * 2.4 + i as f32 * 1.3).sin();
            let c = if i % 3 == 0 { BlipColor { r: 0.95, g: 0.70, b: 0.20, a: 1.0 } } else { BlipColor { r: 0.82, g: 0.22, b: 0.12, a: 1.0 } };
            blip.fill_rect(x, y + shift, 1.0 + turn.abs() * 2.5, 2.5, c);
        },
        7 => for i in 0..5 {
            let x = (now * (22.0 + i as f32 * 7.0) + i as f32 * 173.0).rem_euclid(w + 50.0) - 25.0;
            let y = 236.0 + (now * 8.0 + i as f32).sin() * 2.0 + shift;
            blip.fill_rect(x, y, 3.0, 2.0, BlipColor { r: 0.95, g: 0.67, b: 0.35, a: 0.85 });
        },
        _ => {}
    }
}

/// The finishing blow: for as long as the slow motion lasts the stage is
/// gone, and there is only the winner's colour bursting out from where the
/// loser was hit.
pub(crate) fn draw_finish(blip: &Blip, g: &Game) {
    let Some(lost) = g.p.iter().position(|f| f.health == 0) else { return };
    let (winner, loser) = (g.p[1 - lost].arch(), &g.p[lost]);
    let base = blend(rgb(winner.accent()), rgb(winner.color), 0.25);
    blip.fill_rect(0.0, 0.0, WIN_W as f32, FLOOR_Y, shade(base, 0.22));
    let c = V(loser.x, loser.y - loser.height() * 0.6);
    // The rays jump round a notch at a time: a flicker, not a spin.
    let turn = (g.now * 14.0).floor() * 0.21;
    blip.fill_sunburst(c.0, c.1, 14, turn, 760.0, [shade(base, 0.62), shade(base, 0.40)]);
}

/// The ground, seen flat-on: lit boards falling off toward the bottom of the
/// screen, bright enough for the fighters' shadows to land on.
pub(crate) fn draw_floor(blip: &Blip, stage: usize, shake: f32) {
    // Boards on the docks, stone in the temple, tarmac, wet boards, a jetty,
    // ice, a tarred roof.
    const FLOORS: [(f32, f32, f32); STAGES] = [(0.38, 0.28, 0.21), (0.26, 0.25, 0.31),
        (0.46, 0.48, 0.50), (0.56, 0.42, 0.26), (0.42, 0.31, 0.19),
        (0.62, 0.76, 0.88), (0.30, 0.27, 0.30), (0.22, 0.18, 0.19), (0.36, 0.28, 0.24), (0.56, 0.46, 0.32), (0.62, 0.62, 0.56)];
    let near = rgb(FLOORS[stage % STAGES]);
    let h = WIN_H as f32 - FLOOR_Y;
    let bands = 10;
    for i in 0..bands {
        let k = i as f32 / (bands - 1) as f32;
        let f = 1.0 - 0.55 * k;
        blip.fill_rect(0.0, FLOOR_Y + shake + k * h, WIN_W as f32, h / bands as f32 + 1.0,
            shade(near, f));
    }
    draw_floor_grain(blip, stage, near, shake);
    // The lip where the floor meets the scenery, catching the light.
    blip.fill_rect(0.0, FLOOR_Y + shake - 1.0, WIN_W as f32, 2.0,
        blend(near, BLIP_WHITE, 0.35));
}

/// What the floor is made of, drawn in perspective: rows that deepen toward the viewer and joints that converge
/// on the middle, as SF2's floors are painted boards, flags and slabs rather than a flat colour.
fn draw_floor_grain(blip: &Blip, stage: usize, near: BlipColor, shake: f32) {
    let (w, h) = (WIN_W as f32, WIN_H as f32 - FLOOR_Y);
    let top = FLOOR_Y + shake;
    // Row k's top edge, crowded at the back and opening toward the viewer.
    let row = |k: f32, n: f32| top + h * (k / n).powf(1.5);
    // A joint at `x` on the back edge, carried down toward the viewer.
    let across = |x: f32, y: f32| x + (x - w / 2.0) * 0.30 * (y - top) / h;
    let seam = shade(near, 0.60);
    let lit = blend(near, BLIP_WHITE, 0.16);
    match stage % STAGES {
        // Boards: long planks, their ends staggered row to row, the grain running along them.
        0 | 3 | 4 => {
            let n = 7.0;
            for k in 0..7 {
                let (y0, y1) = (row(k as f32, n), row(k as f32 + 1.0, n));
                blip.fill_rect(0.0, y0, w, 1.0, seam);
                blip.fill_rect(0.0, y0 + 1.0, w, 1.0, lit);
                let mut x = -40.0 + scatter(k, 0x5ee + stage as u32) * 90.0;
                while x < w + 40.0 {
                    blip.draw_line(across(x, y0), y0, across(x, y1), y1, seam);
                    x += 110.0 + scatter(k * 7 + (x as usize % 97), 0x1d) * 60.0;
                }
                for g in 0..3 {
                    let y = y0 + (y1 - y0) * (0.3 + 0.22 * g as f32);
                    let x = scatter(k * 3 + g, 0x9a + stage as u32) * w;
                    blip.fill_rect(x, y, 30.0 + 50.0 * scatter(g, k as u32), 1.0, shade(near, 0.84));
                }
            }
        }
        // Flagstones and slabs: courses of blocks, every other course offset by half, each lit along its top.
        1 | 2 | 6 | 7 | 8 | 10 => {
            let (n, wide) = if stage == 2 { (4.0, 120.0) } else if stage == 1 { (5.0, 64.0) } else { (5.0, 80.0) };
            for k in 0..n as usize {
                let (y0, y1) = (row(k as f32, n), row(k as f32 + 1.0, n));
                blip.fill_rect(0.0, y0, w, 1.0, seam);
                blip.fill_rect(0.0, y0 + 1.0, w, 1.0, lit);
                let mut x = if k % 2 == 0 { 0.0 } else { wide / 2.0 };
                while x < w + wide {
                    blip.draw_line(across(x, y0), y0, across(x, y1), y1, seam);
                    x += wide;
                }
            }
            // Tarmac: the painted line down the apron, and oil stains.
            if stage == 2 {
                let y = row(1.6, 4.0);
                blip.fill_rect(0.0, y, w, 3.0, BlipColor { r: 0.92, g: 0.80, b: 0.30, a: 0.75 });
                for s in 0..4 {
                    let x = scatter(s, 0x011) * w;
                    blip.fill_circle(x, row(2.5 + scatter(s, 0x012), 4.0), 9.0, BlipColor { a: 0.10, ..INK });
                }
            }
        }
        // Ice: long glints raking across it.
        // Packed earth: pebbles and tufts of grass, more of them toward the viewer.
        9 => {
            for k in 0..40usize {
                let y = row(scatter(k, 0x1d1) * 9.0, 9.0);
                let x = across(scatter(k, 0x1d2) * w, y);
                if k % 3 == 0 {
                    blip.fill_rect(x, y - 2.0, 1.0, 3.0, BlipColor { r: 0.36, g: 0.50, b: 0.22, a: 0.9 });
                    blip.fill_rect(x + 2.0, y - 1.0, 1.0, 2.0, BlipColor { r: 0.36, g: 0.50, b: 0.22, a: 0.9 });
                } else {
                    blip.fill_rect(x, y, 2.0, 1.0, shade(near, 0.70));
                }
            }
        }
        _ => {
            for k in 0..9 {
                let y = row(k as f32 + 0.5, 9.0);
                let x = scatter(k, 0x1ce) * w;
                blip.fill_rect(x, y, 60.0 + 80.0 * scatter(k, 0x1cf), 2.0, BlipColor { r: 1.0, g: 1.0, b: 1.0, a: 0.35 });
            }
        }
    }
}

/// Distance as colour: far things lose contrast toward the sky rather than
/// just darkening.
pub(crate) fn hazed(c: BlipColor, sky: BlipColor, k: f32) -> BlipColor { blend(c, sky, k) }

/// A row of onlookers: small, dense, overlapping silhouettes below the
/// fighters' heads, so they read as a crowd and not more fighters. They bob
/// out of step and stand up for a big hit.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_crowd(blip: &Blip, ground: f32, h: f32, c: BlipColor, t: f32, salt: u32, n: usize,
              roar: Roar) {
    let mass = ground - h * 0.42;
    blip.fill_rect(0.0, mass, WIN_W as f32, ground - mass, shade(c, 0.70));
    // In colour, but hazed toward the stage's own crowd colour so they stay behind the fight.
    const SHIRTS: [Rgb; 8] = [(0.80, 0.26, 0.22), (0.24, 0.40, 0.70), (0.90, 0.78, 0.36), (0.30, 0.56, 0.38),
        (0.86, 0.84, 0.78), (0.56, 0.36, 0.62), (0.92, 0.52, 0.22), (0.22, 0.22, 0.26)];
    let line = shade(c, 0.55);
    for i in 0..n {
        let sx = scatter(i, salt);
        let x = -10.0 + (i as f32 + 0.5 + (sx - 0.5) * 1.1) * (WIN_W as f32 + 20.0) / n as f32;
        // Each answers a blow in their own time (`Roar::of`): up on their toes, arms up.
        let k = roar.of(i, salt, t);
        let tall = h * (0.76 + 0.48 * scatter(i, salt ^ 0x5bd1)) * (1.0 + 0.15 * k);
        let bob = ((t * 2.6 + sx * 9.1).sin()).max(0.0) * 1.6;
        let top = ground - tall + bob;
        let pick = |k: u32, n: usize| (scatter(i, salt ^ k) * n as f32) as usize % n;
        let haze = 0.50;
        let shirt = blend(rgb(SHIRTS[pick(0x31, 8)]), c, haze);
        let skin = blend(rgb(SKINS[pick(0x32, 6)]), c, haze * 0.8);
        let hair = blend(rgb(HAIRS[pick(0x33, 6)]), c, haze * 0.8);
        // Shoulders twice the width of a head, which is the proportion that reads as a person at any size.
        let sh = tall * 0.21;
        let r = tall * 0.108;
        let head = V(x, top + tall * 0.13);
        let shoulders = top + tall * 0.30;
        // A few of them with their arms up, and more of them when something has just landed.
        let cheer = scatter(i, salt ^ 0x1f) < 0.20 || k > 0.3;
        let up = tall * (0.30 + 0.10 * ((t * (4.0 + 3.0 * sx) + sx * 7.0).sin()));
        // Some put one arm up, not both.
        let one = roar.kind(i, salt, 3) == 0;
        let arms: [(V, V); 2] = [(V(x - sh * 0.8, shoulders), V(x - sh * 1.3, shoulders - up)),
                                 (V(x + sh * 0.8, shoulders), V(x + sh * 1.2, shoulders - up * 0.86))];
        // Outlined in a darker tone of the crowd, not ink: they are far off.
        let w = 0.24 * tall;
        let arms = &arms[if one { 1 } else { 0 }..];
        if cheer { for &(a, b) in arms { stroke(blip, a, b, w * 0.42 + 1.0, w * 0.36 + 1.0, line); } }
        // Square shoulders on a body down into the crowd: a rounded top made bowling pins of them.
        let yoke = shoulders + sh * 0.40;
        let torso = |grow: f32, col: BlipColor| {
            stroke(blip, V(x - sh * 0.55, yoke), V(x + sh * 0.55, yoke), sh * 0.45 + grow, sh * 0.45 + grow, col);
            blip.fill_rect(x - sh - grow, yoke, 2.0 * (sh + grow), mass + 2.0 - yoke, col);
        };
        torso(1.0, line);
        blip.fill_circle(head.0, head.1, r + 1.0, line);
        if cheer {
            for &(a, b) in arms {
                stroke(blip, a, b, w * 0.42, w * 0.36, shirt);
                blip.fill_circle(b.0, b.1, w * 0.36, skin);
            }
        }
        torso(0.0, shirt);
        // Lit from the front: one side of the shirt in shade.
        blip.fill_rect(x - sh, yoke + sh * 0.2, sh * 0.7, mass + 2.0 - yoke - sh * 0.2, shade(shirt, 0.80));
        stroke(blip, V(x, shoulders - r * 0.3), V(x, head.1), r * 0.45, r * 0.45, skin);
        blip.fill_circle(head.0, head.1, r, skin);
        // Hair over the crown, or a bald one now and then.
        if pick(0x34, 7) != 0 { stroke(blip, V(x - r * 0.45, head.1 - r * 0.45), V(x + r * 0.45, head.1 - r * 0.45), r * 0.62, r * 0.62, hair); }
    }
}

/// How often a cameo crosses the back of a stage, and how long the crossing
/// takes: rare enough to be a surprise, never in front of the fight.
pub(crate) const CAMEO_EVERY: f32 = 36.0;
pub(crate) const CAMEO_SECS: f32 = 9.0;

/// Where a cameo is on its way across, if one is crossing now.
pub(crate) fn cameo_x(t: f32, offset: f32) -> Option<f32> {
    let u = (t + offset).rem_euclid(CAMEO_EVERY) / CAMEO_SECS;
    (u < 1.0).then(|| -30.0 + u * (WIN_W as f32 + 80.0))
}

/// Along the far wharf: a yellow mouth on the run from a red ghost, the
/// oldest chase in the arcade. `gap` is the colour behind it, for the mouth.
pub(crate) fn draw_chase(blip: &Blip, y: f32, t: f32, gap: BlipColor) {
    let Some(x) = cameo_x(t, 12.0) else { return };
    blip.fill_circle(x, y, 4.5, BlipColor { r: 0.98, g: 0.86, b: 0.20, a: 1.0 });
    if (t * 9.0).sin() > 0.0 {
        for up in [-2.0f32, 0.0, 2.0] { blip.draw_line(x, y, x + 5.5, y + up, gap); }
    }
    let gx = x - 26.0;
    let red = BlipColor { r: 0.90, g: 0.20, b: 0.18, a: 1.0 };
    blip.fill_circle(gx, y - 0.5, 4.5, red);
    blip.fill_rect(gx - 4.5, y - 0.5, 9.0, 5.0, red);
    for dx in [-0.8f32, 2.6] {
        blip.fill_rect(gx + dx, y - 2.5, 2.0, 2.4, BLIP_WHITE);
    }
}

/// Across the night sky behind the pillars: a white fighter and the bee it
/// is chasing, out of a shooting gallery of 1981.
pub(crate) fn draw_flyby(blip: &Blip, y: f32, t: f32) {
    let Some(x) = cameo_x(t, 0.0) else { return };
    let white = BlipColor { r: 0.92, g: 0.94, b: 1.0, a: 1.0 };
    let red = BlipColor { r: 0.90, g: 0.20, b: 0.20, a: 1.0 };
    let blue = BlipColor { r: 0.30, g: 0.45, b: 0.95, a: 1.0 };
    // The fighter: nose, wings, red tips.
    blip.fill_rect(x - 1.0, y - 7.0, 2.0, 9.0, white);
    blip.fill_rect(x - 6.0, y - 1.0, 12.0, 3.0, white);
    blip.fill_rect(x - 6.0, y - 4.0, 2.0, 3.0, red);
    blip.fill_rect(x + 4.0, y - 4.0, 2.0, 3.0, red);
    blip.fill_rect(x - 1.0, y - 3.0, 2.0, 2.0, blue);
    // Its shot, and the bee weaving ahead of it.
    let shot = (t * 2.0).fract();
    blip.fill_rect(x + 18.0 + shot * 22.0, y - 3.0, 4.0, 1.5, white);
    let (bx, by) = (x + 58.0, y - 6.0 + (t * 3.0).sin() * 9.0);
    blip.fill_rect(bx - 2.0, by - 3.0, 4.0, 6.0, BlipColor { r: 0.98, g: 0.84, b: 0.20, a: 1.0 });
    blip.fill_rect(bx - 5.0, by - 2.0, 3.0, 4.0, blue);
    blip.fill_rect(bx + 2.0, by - 2.0, 3.0, 4.0, blue);
    blip.fill_rect(bx - 2.0, by - 4.0, 4.0, 1.5, red);
}

/// Docks layers run from sky to quay with distance haze; keep the fighter band low-detail.
pub(crate) fn draw_dock(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 250.0;
    let sky_at = |k: f32| BlipColor {
        r: 0.99 - 0.76 * k, g: 0.52 - 0.40 * k, b: 0.34 + 0.06 * k, a: 1.0,
    };
    blip.fill_bands(0.0, shake, WIN_W as f32, HORIZON, 16, |k| sky_at(1.0 - k));
    let low = sky_at(0.0);
    // Everything below the gradient, before the near layers cover it.
    // Without this the strip between the horizon and the quay is
    // whatever the frame was cleared to, which is black.
    blip.fill_rect(0.0, HORIZON + shake, WIN_W as f32, FLOOR_Y - HORIZON, low);

    // Sun, with the haze around it that a sun near the horizon has.
    let (sun_x, sun_y) = (486.0, 196.0 + shake);
    for r in [74.0f32, 58.0, 46.0] {
        blip.fill_circle(sun_x, sun_y, r,
            BlipColor { r: 1.0, g: 0.78, b: 0.42, a: 0.16 });
    }
    blip.fill_circle(sun_x, sun_y, 38.0, BlipColor { r: 1.0, g: 0.88, b: 0.55, a: 1.0 });

    // Cloud, in flat lit bars. Anything softer at this palette turns to
    // mud against the gradient behind it.
    for i in 0..7 {
        let sx = scatter(i, 0x10c);
        let y = 74.0 + sx * 120.0 + shake;
        let w = 90.0 + scatter(i, 0x20c) * 150.0;
        let x = (t * (3.0 + sx * 4.0) + sx * 900.0) % (WIN_W as f32 + 260.0) - 130.0;
        let lit = 1.0 - ((y - shake) / 200.0).clamp(0.0, 1.0);
        let c = blend(BlipColor { r: 0.62, g: 0.34, b: 0.42, a: 0.85 },
                      BlipColor { r: 1.0, g: 0.80, b: 0.58, a: 0.85 }, lit);
        blip.fill_rect(x, y, w, 7.0, c);
        blip.fill_rect(x + 22.0, y + 7.0, w * 0.6, 5.0, shade(c, 0.9));
    }

    // Far skyline: warehouses and gantries, well hazed.
    let far = hazed(BlipColor { r: 0.26, g: 0.16, b: 0.22, a: 1.0 }, low, 0.55);
    for i in 0..9 {
        let sx = scatter(i, 0x31);
        let w = 34.0 + sx * 56.0;
        let x = i as f32 * 74.0 - 20.0;
        let hh = 20.0 + scatter(i, 0x32) * 34.0;
        blip.fill_rect(x, HORIZON - hh + shake, w, hh, far);
        // Sawtooth warehouse roofs.
        for j in 0..(w as usize / 14) {
            let rx = x + j as f32 * 14.0;
            blip.fill_rect(rx, HORIZON - hh - 5.0 + shake, 8.0, 5.0, far);
        }
        // The sun catches the edge that faces it, and a few windows are lit already.
        let toward_sun = if x + w * 0.5 < sun_x { x + w - 2.0 } else { x };
        blip.fill_rect(toward_sun, HORIZON - hh + shake, 2.0, hh, blend(far, BlipColor { r: 1.0, g: 0.74, b: 0.45, a: 1.0 }, 0.35));
        for row in 0..((hh - 8.0) / 9.0) as usize {
            for col in 0..((w - 8.0) / 9.0) as usize {
                if scatter(i * 97 + row * 13 + col, 0x33) > 0.72 {
                    blip.fill_rect(x + 5.0 + col as f32 * 9.0, HORIZON - hh + 6.0 + row as f32 * 9.0 + shake, 3.0, 3.0,
                        hazed(BlipColor { r: 1.0, g: 0.80, b: 0.42, a: 1.0 }, low, 0.35));
                }
            }
        }
    }
    // Gantry cranes: a tower, a jib out over the water, and the cable
    // under it. Three legs and a bar read as a pylon; the jib is what
    // makes it a crane.
    let crane = hazed(BlipColor { r: 0.17, g: 0.10, b: 0.15, a: 1.0 }, low, 0.30);
    for (n, (x, hgt, dir)) in [(96.0f32, 118.0f32, 1.0f32), (238.0, 92.0, 1.0), (566.0, 106.0, -1.0)].into_iter().enumerate() {
        let base = HORIZON + 4.0 + shake;
        let top = base - hgt;
        // A lattice tower, not a pole: two chords and the bracing zigzagging between them.
        blip.fill_rect(x - 4.0, top, 1.5, hgt, crane);
        blip.fill_rect(x + 2.5, top, 1.5, hgt, crane);
        let mut y = top;
        let mut flip = false;
        while y < base - 8.0 {
            let (a, b) = if flip { (x - 3.5, x + 3.5) } else { (x + 3.5, x - 3.5) };
            blip.draw_line(a, y, b, y + 7.0, crane);
            y += 7.0;
            flip = !flip;
        }
        // A warning lamp on top, blinking.
        if (t * 1.3 + n as f32 * 0.7).fract() < 0.45 {
            blip.fill_circle(x, top - 2.0, 1.8, BlipColor { r: 1.0, g: 0.20, b: 0.12, a: 1.0 });
            blip.fill_glow_circle(x, top - 2.0, 6.0, BlipColor { r: 1.0, g: 0.25, b: 0.15, a: 0.35 });
        }
        blip.fill_rect(x - 16.0, base - 8.0, 32.0, 8.0, crane);
        blip.draw_line(x, base, x - dir * 16.0, top + 14.0, crane);
        // Jib and the hook hanging off it.
        // The jib, a truss too.
        let jx = x.min(x + dir * 62.0);
        blip.fill_rect(jx, top, 62.0, 1.5, crane);
        blip.fill_rect(jx, top + 4.0, 62.0, 1.5, crane);
        for k in 0..9 {
            let a = jx + k as f32 * 7.0;
            blip.draw_line(a, top + 1.0, a + 7.0, top + 4.5, crane);
        }
        blip.draw_line(x + dir * 54.0, top + 5.0, x + dir * 54.0, top + 30.0, crane);
        blip.fill_rect(x + dir * 50.0, top + 30.0, 9.0, 6.0, crane);
        blip.draw_line(x, top, x + dir * 24.0, top - 16.0, crane);
        blip.draw_line(x + dir * 24.0, top - 16.0, x + dir * 60.0, top, crane);
    }

    // A moored ship, because a dock with nothing tied up at it is a
    // sea wall.
    let hull = hazed(BlipColor { r: 0.22, g: 0.24, b: 0.34, a: 1.0 }, low, 0.22);
    blip.fill_rect(300.0, HORIZON - 26.0 + shake, 186.0, 26.0, hull);
    blip.fill_rect(300.0, HORIZON - 30.0 + shake, 186.0, 5.0,
        blend(hull, BLIP_WHITE, 0.25));
    blip.fill_rect(352.0, HORIZON - 54.0 + shake, 58.0, 24.0, shade(hull, 1.15));
    blip.fill_rect(392.0, HORIZON - 72.0 + shake, 7.0, 20.0, shade(hull, 0.8));
    // Containers stacked two high, ribbed.
    for i in 0..6 {
        for tier in 0..2 {
            let c = [BlipColor { r: 0.55, g: 0.30, b: 0.22, a: 1.0 }, BlipColor { r: 0.26, g: 0.42, b: 0.40, a: 1.0 },
                     BlipColor { r: 0.62, g: 0.52, b: 0.24, a: 1.0 }][(i + tier * 2) % 3];
            if tier == 1 && i % 3 == 2 { continue; }
            let (cx, cy) = (414.0 + i as f32 * 12.0, HORIZON - 42.0 - tier as f32 * 12.0 + shake);
            let c = hazed(c, low, 0.3);
            blip.fill_rect(cx, cy, 11.0, 12.0, c);
            for r in 0..3 { blip.fill_rect(cx + 2.0 + r as f32 * 3.0, cy + 1.0, 1.0, 10.0, shade(c, 0.8)); }
        }
    }
    // Portholes along the hull, a red waterline, a rail along the deck.
    for i in 0..12 {
        blip.fill_circle(312.0 + i as f32 * 14.0, HORIZON - 17.0 + shake, 1.6, hazed(BlipColor { r: 1.0, g: 0.82, b: 0.5, a: 1.0 }, low, 0.4));
    }
    blip.fill_rect(300.0, HORIZON - 4.0 + shake, 186.0, 4.0, hazed(BlipColor { r: 0.58, g: 0.16, b: 0.14, a: 1.0 }, low, 0.3));
    for i in 0..24 { blip.fill_rect(302.0 + i as f32 * 8.0, HORIZON - 35.0 + shake, 1.0, 5.0, shade(hull, 0.8)); }
    blip.fill_rect(300.0, HORIZON - 35.0 + shake, 186.0, 1.0, shade(hull, 0.8));
    // Smoke off the funnel, drifting away from the sun and thinning.
    for k in 0..6 {
        let age = (t * 0.35 + k as f32 / 6.0).fract();
        let (px, py) = (395.0 - age * 70.0, HORIZON - 76.0 - age * 34.0 + shake);
        blip.fill_circle(px, py, 3.0 + age * 9.0, BlipColor { r: 0.55, g: 0.42, b: 0.46, a: 0.45 * (1.0 - age) });
    }

    // The far wharf, and the dockers watching from it.
    let wharf = hazed(BlipColor { r: 0.30, g: 0.20, b: 0.20, a: 1.0 }, low, 0.18);
    draw_crowd(blip, HORIZON + 8.0 + shake, 29.0,
        hazed(BlipColor { r: 0.15, g: 0.09, b: 0.13, a: 1.0 }, low, 0.16),
        t, 0x9e3, 34, roar);
    blip.fill_rect(0.0, HORIZON + 4.0 + shake, WIN_W as f32, 10.0, wharf);
    draw_chase(blip, HORIZON + 9.0 + shake, t, wharf);

    // Water: flat, dark, and the quietest thing on screen, because it
    // is what sits directly behind the fighters.
    let sea = BlipColor { r: 0.17, g: 0.22, b: 0.36, a: 1.0 };
    blip.fill_rect(0.0, HORIZON + 14.0 + shake, WIN_W as f32, 72.0, sea);
    // The sun's road on the water, and ripples crossing it.
    for i in 0..13 {
        let k = i as f32 / 12.0;
        let y = HORIZON + 18.0 + k * 62.0 + shake;
        let w = 16.0 + k * 54.0 + (t * 2.0 + i as f32).sin() * 5.0;
        blip.fill_rect(sun_x - w * 0.5, y, w, 2.5,
            BlipColor { r: 1.0, g: 0.80, b: 0.52, a: 0.30 - 0.16 * k });
    }
    // The ship and the cranes upside down in it, broken up by the swell.
    for row in 0..14 {
        let y = HORIZON + 14.0 + row as f32 * 2.0 + shake;
        let wob = (t * 2.2 + row as f32 * 0.9).sin() * (1.0 + row as f32 * 0.25);
        let fade = 0.30 * (1.0 - row as f32 / 14.0);
        blip.fill_rect(300.0 + wob, y, 186.0, 1.0, BlipColor { r: 0.08, g: 0.08, b: 0.14, a: fade });
        for x in [96.0f32, 238.0, 566.0] {
            blip.fill_rect(x - 4.0 + wob, y, 8.0, 1.0, BlipColor { r: 0.08, g: 0.06, b: 0.12, a: fade });
        }
    }
    // Glitter on the sun's road: points that wink on and off.
    for i in 0..26 {
        let k = scatter(i, 0x5eb);
        let y = HORIZON + 18.0 + k * 62.0 + shake;
        let x = sun_x + (scatter(i, 0x5ec) - 0.5) * (16.0 + k * 54.0);
        if (t * 3.0 + scatter(i, 0x5ed) * 9.0).sin() > 0.6 {
            blip.fill_rect(x, y, 2.0, 1.0, BlipColor { r: 1.0, g: 0.95, b: 0.75, a: 0.9 });
        }
    }
    for i in 0..22 {
        let sx = scatter(i, 0x5ea);
        let y = HORIZON + 20.0 + sx * 58.0 + shake;
        let x = ((i as f32 * 61.0) + t * (7.0 + sx * 9.0)) % (WIN_W as f32 + 60.0) - 30.0;
        blip.fill_rect(x, y, 14.0 + sx * 20.0, 1.5,
            BlipColor { r: 0.62, g: 0.74, b: 0.92, a: 0.16 });
    }

    // The quay wall the fighters stand on top of, with its bollards and
    // the tyres hung over the edge.
    let quay = BlipColor { r: 0.31, g: 0.22, b: 0.16, a: 1.0 };
    blip.fill_rect(0.0, FLOOR_Y - 30.0 + shake, WIN_W as f32, 30.0, quay);
    blip.fill_rect(0.0, FLOOR_Y - 32.0 + shake, WIN_W as f32, 3.0, blend(quay, BLIP_WHITE, 0.22));
    // Dressed stone in courses, each block a shade of its own, the joints staggered.
    for course in 0..3 {
        let y = FLOOR_Y - 29.0 + course as f32 * 10.0 + shake;
        let stagger = if course % 2 == 0 { 0.0 } else { 15.0 };
        for i in 0..23 {
            let x = i as f32 * 30.0 - stagger;
            blip.fill_rect(x + 1.0, y + 1.0, 28.0, 8.0, shade(quay, 0.90 + 0.18 * scatter(i + course * 31, 0x9a)));
        }
        blip.fill_rect(0.0, y + 9.0, WIN_W as f32, 1.0, shade(quay, 0.65));
    }
    // Wet and weedy along the bottom, where the tide reaches.
    blip.fill_rect(0.0, FLOOR_Y - 6.0 + shake, WIN_W as f32, 6.0, BlipColor { r: 0.14, g: 0.20, b: 0.12, a: 0.55 });
    for i in 0..5 {
        let x = 54.0 + i as f32 * 136.0;
        blip.fill_circle(x, FLOOR_Y - 14.0 + shake, 7.0, shade(quay, 0.55));
        blip.fill_circle(x, FLOOR_Y - 14.0 + shake, 3.5, sea);
    }
    for (x, n) in [(2.0f32, 2), (598.0, 3)] {
        for i in 0..n {
            let y = FLOOR_Y - 26.0 * (i + 1) as f32 + shake;
            let c = BlipColor { r: 0.44, g: 0.30, b: 0.17, a: 1.0 };
            blip.fill_rect(x, y, 38.0, 26.0, c);
            blip.fill_rect(x, y, 38.0, 4.0, blend(c, BLIP_WHITE, 0.2));
            blip.draw_rect(x, y, 38.0, 26.0, shade(c, 0.5));
        }
    }
}

/// THE TEMPLE: night under a roof, the mountain behind. Built like the docks;
/// the light comes from the lanterns.
pub(crate) fn draw_temple(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 262.0;
    let sky_at = |k: f32| BlipColor {
        r: 0.05 + 0.13 * k, g: 0.06 + 0.13 * k, b: 0.13 + 0.17 * k, a: 1.0,
    };
    blip.fill_bands(0.0, shake, WIN_W as f32, HORIZON, 14, sky_at);
    let low = sky_at(1.0);
    blip.fill_rect(0.0, HORIZON + shake, WIN_W as f32, FLOOR_Y - HORIZON, low);

    for i in 0..52 {
        let sx = scatter(i, 0x2a);
        let x = sx * WIN_W as f32;
        let y = scatter(i, 0x2b) * 210.0 + 8.0 + shake;
        let tw = 0.5 + 0.5 * ((t * 1.7 + sx * 30.0).sin());
        let r = 0.8 + scatter(i, 0x2c) * 1.2;
        blip.fill_circle(x, y, r,
            BlipColor { r: 0.9, g: 0.95, b: 1.0, a: 0.20 + 0.40 * tw });
    }

    // Moon, with its glow and the cloud that drifts over it.
    let (mx, my) = (143.0, 92.0 + shake);
    for r in [46.0f32, 34.0] {
        blip.fill_circle(mx, my, r, BlipColor { r: 0.75, g: 0.82, b: 0.95, a: 0.10 });
    }
    blip.fill_circle(mx, my, 26.0, BlipColor { r: 0.92, g: 0.94, b: 0.86, a: 1.0 });
    blip.fill_circle(mx - 10.0, my - 7.0, 23.0, sky_at(0.28));
    for i in 0..5 {
        let sx = scatter(i, 0x40);
        let y = 60.0 + sx * 96.0 + shake;
        let w = 110.0 + scatter(i, 0x41) * 170.0;
        let x = (t * (2.0 + sx * 3.0) + sx * 800.0) % (WIN_W as f32 + 300.0) - 150.0;
        blip.fill_rect(x, y, w, 6.0, BlipColor { r: 0.20, g: 0.21, b: 0.31, a: 0.7 });
    }

    // Mountains, in two ranges so there is a distance between them.
    for (base, amp, k, step) in [(HORIZON - 52.0, 66.0f32, 0.66f32, 128.0f32),
                                 (HORIZON - 26.0, 44.0, 0.44, 96.0)] {
        let c = hazed(BlipColor { r: 0.10, g: 0.11, b: 0.20, a: 1.0 }, low, k);
        let n = (WIN_W as f32 / step) as usize + 2;
        for i in 0..n {
            let x = i as f32 * step - 40.0;
            let pk = amp * (0.55 + 0.45 * scatter(i, if k > 0.5 { 0x51 } else { 0x52 }));
            // A peak drawn as a stack of shrinking bars: at this size a
            // triangle and a staircase are the same picture.
            let rows = 9;
            for r in 0..rows {
                let f = r as f32 / rows as f32;
                let w = step * 0.95 * (1.0 - f);
                blip.fill_rect(x + step * 0.75 - w * 0.5, base + shake - pk * f - pk / rows as f32,
                    w, pk / rows as f32 + 1.0, c);
            }
        }
    }

    draw_flyby(blip, 112.0 + shake, t);

    // The roof over the whole stage, and the rafters holding it up.
    let stone = BlipColor { r: 0.19, g: 0.18, b: 0.24, a: 1.0 };
    let lit = BlipColor { r: 0.31, g: 0.29, b: 0.37, a: 1.0 };
    let tile = BlipColor { r: 0.14, g: 0.13, b: 0.19, a: 1.0 };
    blip.fill_rect(0.0, 30.0 + shake, WIN_W as f32, 20.0, tile);
    for i in 0..27 {
        blip.fill_rect(i as f32 * 24.0, 30.0 + shake, 20.0, 20.0, shade(tile, 1.3));
        blip.fill_rect(i as f32 * 24.0, 46.0 + shake, 20.0, 4.0, shade(tile, 0.7));
    }
    blip.fill_rect(0.0, 50.0 + shake, WIN_W as f32, 7.0, lit);
    for i in 0..14 {
        blip.fill_rect(10.0 + i as f32 * 46.0, 57.0 + shake, 12.0, 16.0, stone);
    }

    // Colonnade, and the onlookers standing between the pillars.
    draw_crowd(blip, HORIZON + 6.0 + shake, 32.0,
        BlipColor { r: 0.20, g: 0.20, b: 0.29, a: 1.0 }, t, 0x77a, 30, roar);
    for i in 0..5 {
        let x = 22.0 + i as f32 * 146.0;
        blip.fill_rect(x, 57.0 + shake, 30.0, HORIZON - 45.0, stone);
        blip.fill_rect(x, 57.0 + shake, 7.0, HORIZON - 45.0, lit);
        // Capital and base, which is what stops a pillar being a bar.
        blip.fill_rect(x - 5.0, 57.0 + shake, 40.0, 9.0, lit);
        blip.fill_rect(x - 5.0, HORIZON + 4.0 + shake, 40.0, 10.0, lit);
        blip.fill_rect(x - 3.0, HORIZON + 14.0 + shake, 36.0, 6.0, shade(stone, 0.8));
    }

    // Banners between the pillars, and the lanterns that light them.
    for i in 0..4 {
        let x = 96.0 + i as f32 * 146.0;
        let sway = (t * 1.1 + i as f32).sin() * 3.0;
        blip.fill_rect(x + sway, 73.0 + shake, 24.0, 118.0,
            BlipColor { r: 0.60, g: 0.11, b: 0.16, a: 1.0 });
        blip.fill_rect(x + 8.0 + sway, 95.0 + shake, 8.0, 62.0,
            BlipColor { r: 0.95, g: 0.85, b: 0.5, a: 0.9 });
        blip.fill_rect(x - 2.0 + sway, 73.0 + shake, 28.0, 5.0,
            BlipColor { r: 0.30, g: 0.26, b: 0.20, a: 1.0 });
    }
    for i in 0..5 {
        let x = 22.0 + i as f32 * 146.0 + 15.0;
        let sway = (t * 1.4 + i as f32 * 1.9).sin() * 2.2;
        let y = 82.0 + shake;
        blip.draw_line(x, 66.0 + shake, x + sway, y, BlipColor { r: 0.25, g: 0.22, b: 0.18, a: 1.0 });
        let flick = 0.86 + 0.14 * ((t * 9.0 + i as f32 * 2.1).sin());
        blip.fill_glow_circle(x + sway, y + 9.0, 22.0,
            BlipColor { r: 1.0, g: 0.72, b: 0.32, a: 0.16 * flick });
        blip.fill_circle(x + sway, y + 9.0, 8.0,
            BlipColor { r: 0.95, g: 0.40, b: 0.25, a: 1.0 });
        blip.fill_circle(x + sway, y + 8.0, 5.0,
            BlipColor { r: 1.0, g: 0.85 * flick, b: 0.5, a: 1.0 });
    }

    // The terrace wall under the colonnade, down to the floor.
    let wall = BlipColor { r: 0.15, g: 0.15, b: 0.20, a: 1.0 };
    blip.fill_rect(0.0, HORIZON + 20.0 + shake, WIN_W as f32, FLOOR_Y - HORIZON - 20.0, wall);
    for i in 0..13 {
        let x = i as f32 * 50.0;
        blip.fill_rect(x + 2.0, HORIZON + 24.0 + shake, 46.0, 20.0, shade(wall, 1.25));
        blip.fill_rect(x + 27.0, HORIZON + 46.0 + shake, 46.0, 20.0, shade(wall, 1.12));
    }
    blip.fill_rect(0.0, HORIZON + 20.0 + shake, WIN_W as f32, 3.0, lit);
}

/// A sky in flat bands from `top` down to `low` at the horizon, and `low`
/// carried on down to the floor so no layer leaves a hole.
pub(crate) fn draw_sky(blip: &Blip, shake: f32, horizon: f32, top: BlipColor, low: BlipColor) {
    blip.fill_bands(0.0, shake, WIN_W as f32, horizon, 14, |k| blend(top, low, k));
    blip.fill_rect(0.0, horizon + shake, WIN_W as f32, FLOOR_Y - horizon, low);
}

/// THE AIR BASE: midday on the apron, a jet parked behind the ground crew.
/// The brightest stage; the tarmac behind the fighters is kept bare.
pub(crate) fn draw_airbase(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    // A high horizon: the jet stands above the fighters' heads and what is
    // behind them is bare tarmac.
    const HORIZON: f32 = 222.0;
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let low = c(0.76, 0.87, 0.95);
    draw_sky(blip, shake, HORIZON, c(0.26, 0.50, 0.86), low);

    for i in 0..6 {
        let sx = scatter(i, 0xa1);
        let y = 40.0 + sx * 110.0 + shake;
        let w = 80.0 + scatter(i, 0xa2) * 120.0;
        let x = (t * (4.0 + sx * 4.0) + sx * 900.0) % (WIN_W as f32 + 240.0) - 120.0;
        blip.fill_rect(x, y, w, 8.0, BlipColor { a: 0.9, ..BLIP_WHITE });
        blip.fill_rect(x + 18.0, y + 8.0, w * 0.6, 5.0, c(0.88, 0.93, 0.98));
    }

    // Hills, hangars and the tower, far off.
    blip.fill_rect(0.0, HORIZON - 12.0 + shake, WIN_W as f32, 14.0, hazed(c(0.30, 0.42, 0.30), low, 0.5));
    let shed = hazed(c(0.50, 0.54, 0.58), low, 0.35);
    for x in [36.0f32, 120.0, 500.0] {
        blip.fill_circle(x + 30.0, HORIZON - 14.0 + shake, 30.0, shed);
        blip.fill_rect(x, HORIZON - 14.0 + shake, 60.0, 16.0, shed);
        blip.fill_rect(x + 18.0, HORIZON - 16.0 + shake, 24.0, 18.0, shade(shed, 0.7));
    }
    let tower = hazed(c(0.62, 0.64, 0.66), low, 0.25);
    blip.fill_rect(596.0, HORIZON - 84.0 + shake, 10.0, 86.0, tower);
    blip.fill_rect(584.0, HORIZON - 100.0 + shake, 34.0, 18.0, tower);
    blip.fill_rect(587.0, HORIZON - 96.0 + shake, 28.0, 7.0, c(0.30, 0.48, 0.66));

    // The jet: nose to the left, canopy, wing, a tall fin, on its wheels.
    let y = HORIZON + 4.0 + shake;
    let (skin, dark) = (c(0.66, 0.70, 0.74), c(0.44, 0.48, 0.54));
    for wx in [222.0f32, 372.0, 392.0] {
        blip.draw_line_ex(wx, y - 22.0, wx, y - 4.0, 3.0, dark);
        blip.fill_circle(wx, y - 4.0, 5.0, c(0.14, 0.14, 0.16));
    }
    stroke(blip, V(404.0, y - 36.0), V(446.0, y - 84.0), 9.0, 4.0, dark);
    stroke(blip, V(420.0, y - 30.0), V(462.0, y - 34.0), 5.0, 3.0, dark);
    stroke(blip, V(196.0, y - 26.0), V(436.0, y - 30.0), 13.0, 9.0, skin);
    stroke(blip, V(196.0, y - 26.0), V(150.0, y - 23.0), 13.0, 2.5, dark);
    stroke(blip, V(232.0, y - 38.0), V(270.0, y - 40.0), 7.0, 5.0, c(0.30, 0.52, 0.74));
    stroke(blip, V(300.0, y - 22.0), V(392.0, y - 12.0), 6.0, 3.0, dark);
    blip.fill_rect(200.0, y - 28.0, 228.0, 2.0, c(0.80, 0.22, 0.20));
    blip.fill_circle(338.0, y - 30.0, 6.0, c(0.92, 0.94, 0.96));
    blip.fill_circle(338.0, y - 30.0, 3.5, c(0.20, 0.36, 0.70));

    // The ground crew along the barrier, and bare tarmac down to the kerb.
    draw_crowd(blip, HORIZON + 14.0 + shake, 28.0, hazed(c(0.24, 0.30, 0.20), low, 0.12),
        t, 0xa7c, 30, roar);
    blip.fill_rect(0.0, HORIZON + 10.0 + shake, WIN_W as f32, 9.0, c(0.80, 0.80, 0.78));
    for i in 0..17 { blip.fill_rect(i as f32 * 40.0, HORIZON + 10.0 + shake, 20.0, 9.0, c(0.86, 0.30, 0.24)); }
    let tarmac = c(0.50, 0.52, 0.55);
    blip.fill_rect(0.0, HORIZON + 19.0 + shake, WIN_W as f32, FLOOR_Y - HORIZON - 19.0, tarmac);
    blip.fill_rect(0.0, HORIZON + 40.0 + shake, WIN_W as f32, 2.5, c(0.92, 0.80, 0.28));
    blip.fill_rect(0.0, FLOOR_Y - 8.0 + shake, WIN_W as f32, 8.0, shade(tarmac, 0.8));

    // Ammunition crates stacked at either edge.
    for (x, n) in [(2.0f32, 3), (598.0, 2)] {
        for i in 0..n {
            let top = FLOOR_Y - 26.0 * (i + 1) as f32 + shake;
            let crate_c = c(0.36, 0.42, 0.26);
            blip.fill_rect(x, top, 40.0, 26.0, crate_c);
            blip.fill_rect(x, top, 40.0, 4.0, blend(crate_c, BLIP_WHITE, 0.2));
            blip.fill_rect(x + 8.0, top + 11.0, 24.0, 3.0, c(0.90, 0.86, 0.60));
            blip.draw_rect(x, top, 40.0, 26.0, shade(crate_c, 0.5));
        }
    }
}

/// THE CRYSTAL FORTRESS: polar night, an aurora, and a hall of leaning ice
/// crystals with nobody in it. The snowfield behind the fighters is the
/// quiet band.
pub(crate) fn draw_fortress(blip: &Blip, shake: f32, t: f32) {
    const HORIZON: f32 = 230.0;
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let low = c(0.36, 0.60, 0.74);
    draw_sky(blip, shake, HORIZON, c(0.04, 0.07, 0.20), low);

    // Stars, and the aurora: curtains of green hung across the dark, swaying.
    for i in 0..40 {
        let (x, y) = (scatter(i, 0xf1) * WIN_W as f32, scatter(i, 0xf2) * 120.0);
        let lit = 0.5 + 0.5 * (t * (1.0 + scatter(i, 0xf3) * 2.0) + i as f32).sin();
        blip.fill_rect(x, y + shake, 1.5, 1.5, blend(c(0.10, 0.16, 0.32), BLIP_WHITE, 0.35 + 0.5 * lit));
    }
    for i in 0..64 {
        let x = i as f32 * 10.0;
        let wave = (x * 0.012 + t * 0.25).sin() + 0.5 * (x * 0.031 - t * 0.4).sin();
        let top = 46.0 + wave * 18.0 + shake;
        let len = 46.0 + 22.0 * (x * 0.021 + t * 0.6).sin();
        // Flat colour, mixed with the sky it hangs in: brightest along the top.
        let sky = blend(c(0.04, 0.07, 0.20), low, top / HORIZON);
        let green = c(0.30, 0.95, 0.62);
        blip.fill_rect(x, top, 10.0, len, blend(sky, green, 0.22));
        blip.fill_rect(x, top, 10.0, len * 0.4, blend(sky, green, 0.42));
    }

    // Far ice ridges on the horizon.
    let ridge = hazed(c(0.70, 0.84, 0.94), low, 0.45);
    for i in 0..9 {
        let x = i as f32 * 80.0 - 20.0 + scatter(i, 0xf5) * 30.0;
        let h = 18.0 + scatter(i, 0xf6) * 30.0;
        stroke(blip, V(x, HORIZON + shake), V(x + 8.0, HORIZON - h + shake), 22.0, 1.0, ridge);
    }

    // The crystals: great shafts driven in at every angle, a lit face and a
    // shadowed one, taller toward the middle of the hall.
    let (lit, dark, edge) = (c(0.72, 0.88, 0.98), c(0.40, 0.60, 0.80), c(0.94, 0.98, 1.0));
    let shafts: [(f32, f32, f32, f32); 11] = [
        (40.0, -0.50, 150.0, 15.0), (96.0, 0.30, 120.0, 12.0), (150.0, -0.20, 190.0, 18.0),
        (214.0, 0.42, 150.0, 13.0), (268.0, -0.10, 216.0, 20.0), (330.0, 0.16, 226.0, 21.0),
        (392.0, -0.38, 160.0, 14.0), (446.0, 0.24, 196.0, 18.0), (508.0, -0.28, 130.0, 13.0),
        (560.0, 0.48, 156.0, 15.0), (612.0, -0.12, 110.0, 12.0)];
    for (x, lean, len, w) in shafts {
        let base = V(x, HORIZON + 8.0 + shake);
        let tip = V(x + lean * len, HORIZON + 8.0 - len * (1.0 - lean * lean * 0.5) + shake);
        stroke(blip, base, tip, w, 1.5, dark);
        // The lit face: the same shaft, narrower, moved toward the light.
        stroke(blip, V(base.0 - w * 0.34, base.1), V(tip.0 - 0.6, tip.1), w * 0.60, 1.0, lit);
        stroke(blip, V(base.0 - w * 0.62, base.1), V(tip.0 - 1.0, tip.1), 1.2, 0.6, edge);
    }
    // Grown, not built: beams thrown right across the hall overhead, crossing.
    for (a, b, w) in [(V(-10.0, 150.0), V(420.0, 34.0), 9.0f32), (V(650.0, 136.0), V(190.0, 20.0), 8.0),
                      (V(-10.0, 96.0), V(250.0, 8.0), 6.0)] {
        let (a, b) = (V(a.0, a.1 + shake), V(b.0, b.1 + shake));
        stroke(blip, a, b, w, 1.5, dark);
        stroke(blip, V(a.0, a.1 - w * 0.4), V(b.0, b.1 - 0.5), w * 0.5, 0.8, lit);
    }
    // The console: a bank of crystal rods stood in a block of ice, each one
    // lit from inside and none in step with its neighbour.
    let desk = HORIZON + 2.0 + shake;
    stroke(blip, V(268.0, desk), V(348.0, desk), 9.0, 9.0, dark);
    stroke(blip, V(268.0, desk - 3.0), V(348.0, desk - 3.0), 6.0, 6.0, lit);
    for i in 0..9 {
        let x = 274.0 + i as f32 * 8.5;
        let h = 16.0 + scatter(i, 0xfd) * 22.0;
        let glow = 0.5 + 0.5 * (t * (1.0 + scatter(i, 0xfe)) + i as f32 * 1.7).sin();
        let rod = blend(c(0.62, 0.90, 1.0), BLIP_WHITE, glow);
        blip.fill_rect(x - 2.0, desk - 6.0 - h, 4.0, h, dark);
        blip.fill_rect(x - 1.0, desk - 6.0 - h, 2.0, h, rod);
    }

    // The snowfield, down to the ice the fight is on, and broken crystal at
    // either edge.
    let snow = c(0.84, 0.91, 0.97);
    blip.fill_rect(0.0, HORIZON + 6.0 + shake, WIN_W as f32, FLOOR_Y - HORIZON - 6.0, snow);
    blip.fill_rect(0.0, HORIZON + 6.0 + shake, WIN_W as f32, 4.0, edge);
    blip.fill_rect(0.0, FLOOR_Y - 10.0 + shake, WIN_W as f32, 10.0, shade(snow, 0.88));
    for (x, lean) in [(14.0f32, 0.5f32), (44.0, -0.2), (600.0, -0.5), (628.0, 0.2)] {
        let base = V(x, FLOOR_Y + shake);
        stroke(blip, base, V(x + lean * 40.0, FLOOR_Y - 62.0 + shake), 11.0, 1.2, dark);
        stroke(blip, V(x - 3.5, base.1), V(x + lean * 40.0 - 0.6, FLOOR_Y - 62.0 + shake), 6.5, 1.0, lit);
    }
    // Snow coming down, slowly.
    for i in 0..36 {
        let x = (scatter(i, 0xfa) * WIN_W as f32 + (t * 6.0 + i as f32).sin() * 8.0) % WIN_W as f32;
        let y = (scatter(i, 0xfb) * FLOOR_Y + t * (14.0 + scatter(i, 0xfc) * 16.0)) % FLOOR_Y;
        blip.fill_rect(x, y + shake, 2.0, 2.0, BLIP_WHITE);
    }
}

/// QUEENS ROOFTOP: dusk over the river, the city lit up across it, the
/// elevated train going by, and the neighbours watching from the next roof.
/// The brick parapet behind the fighters is the quiet band.
pub(crate) fn draw_rooftop(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 214.0;
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let low = c(0.96, 0.60, 0.36);
    draw_sky(blip, shake, HORIZON, c(0.16, 0.14, 0.36), low);
    blip.fill_circle(120.0, HORIZON - 16.0 + shake, 26.0, c(1.0, 0.82, 0.50));

    // The skyline across the river: towers in two rows, windows coming on.
    for (row, (salt, tone)) in [(0xb1u32, 0.50f32), (0xb5, 0.0)].into_iter().enumerate() {
        let body = hazed(c(0.16, 0.13, 0.26), low, tone);
        let n = 20 - row * 4;
        for i in 0..n {
            let w = WIN_W as f32 / n as f32;
            let x = i as f32 * w;
            let h = 28.0 + scatter(i, salt) * (58.0 + 46.0 * row as f32)
                + if scatter(i, salt ^ 9) > 0.86 { 46.0 } else { 0.0 };
            let top = HORIZON - h + shake;
            blip.fill_rect(x, top, w - 2.0, h + 2.0, body);
            // A spire on the tall ones.
            if h > 110.0 { blip.fill_rect(x + w / 2.0 - 1.5, top - 20.0, 3.0, 20.0, body); }
            if row == 0 { continue; }
            let mut wy = top + 6.0;
            let mut k = 0;
            while wy < HORIZON - 6.0 + shake {
                for col in 0..3 {
                    k += 1;
                    if scatter(i * 97 + k, salt ^ 0x33) > 0.55 {
                        blip.fill_rect(x + 4.0 + col as f32 * (w - 10.0) / 3.0, wy, 3.0, 4.0,
                            c(1.0, 0.86, 0.48));
                    }
                }
                wy += 10.0;
            }
        }
    }

    // The steel globe of the old world's fair, and a red sign on its frame.
    let steel = c(0.86, 0.90, 0.96);
    let (gx, gy) = (286.0, HORIZON - 15.0 + shake);
    blip.fill_circle(gx, gy, 14.0, steel);
    blip.fill_circle(gx, gy, 12.2, c(0.22, 0.20, 0.36));
    for k in [-1.0f32, 0.0, 1.0] {
        stroke(blip, V(gx - 12.0, gy + k * 6.0), V(gx + 12.0, gy + k * 6.0), 0.7, 0.7, steel);
        stroke(blip, V(gx + k * 6.0, gy - 12.0), V(gx + k * 6.0, gy + 12.0), 0.7, 0.7, steel);
    }
    stroke(blip, V(gx - 19.0, gy + 7.0), V(gx + 19.0, gy - 9.0), 0.8, 0.8, steel);
    // Neon: it drops out for a blink every couple of seconds.
    let lit = if (t * 0.5).fract() < 0.92 { c(1.0, 0.36, 0.30) } else { c(0.50, 0.14, 0.14) };
    let sy = HORIZON - 118.0 + shake;
    for sx in [182.0f32, 216.0, 250.0] { blip.fill_rect(sx, sy + 18.0, 1.5, 34.0, c(0.10, 0.08, 0.14)); }
    blip.fill_rect(174.0, sy - 4.0, 84.0, 22.0, c(0.10, 0.07, 0.12));
    blip.draw_text("QUEENS", 180.0, sy, 2.0, lit);

    let poster = V(500.0, 112.0 + shake);
    blip.fill_rect(poster.0 - 82.5, poster.1 - 46.0, 165.0, 92.0, c(0.08, 0.06, 0.14));
    FILM_POSTER.with(|art| blip.draw_texture(art, poster.0 - 82.5, poster.1 - 46.0, 165.0, 92.0));
    blip.draw_rect(poster.0 - 82.5, poster.1 - 46.0, 165.0, 92.0, c(0.78, 0.28, 0.42));

    // The river, and the bridge over it: a cantilever in steel, two towers
    // with their spikes and the trusses hung between and beyond them.
    blip.fill_rect(0.0, HORIZON + shake, WIN_W as f32, 14.0, c(0.34, 0.26, 0.42));
    let iron = c(0.20, 0.14, 0.22);
    // Painted steel with the last of the sun on it, or it is lost against
    // the towers behind.
    let span = c(0.80, 0.62, 0.56);
    let deck = HORIZON - 8.0 + shake;
    blip.fill_rect(330.0, deck, 310.0, 4.0, span);
    blip.fill_rect(330.0, deck - 9.0, 310.0, 1.5, span);
    // The top chord: up to each tower, down between them.
    let chord = [(330.0f32, 12.0f32), (400.0, 46.0), (480.0, 18.0), (560.0, 46.0), (640.0, 12.0)];
    for w in chord.windows(2) {
        let (a, b) = (w[0], w[1]);
        stroke(blip, V(a.0, deck - a.1), V(b.0, deck - b.1), 1.3, 1.3, span);
        // The web under it: posts and braces, a panel every sixteen pixels.
        let n = ((b.0 - a.0) / 16.0) as i32;
        for k in 0..n {
            let (u0, u1) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
            let (x0, x1) = (a.0 + (b.0 - a.0) * u0, a.0 + (b.0 - a.0) * u1);
            let (h0, h1) = (a.1 + (b.1 - a.1) * u0, a.1 + (b.1 - a.1) * u1);
            blip.fill_rect(x0, deck - h0, 1.2, h0, span);
            stroke(blip, V(x0, deck), V(x1, deck - h1), 0.6, 0.6, span);
        }
    }
    for tx in [400.0f32, 560.0] {
        blip.fill_rect(tx - 4.0, deck - 46.0, 8.0, 60.0, span);
        for sp in [-3.0f32, 0.0, 3.0] { blip.fill_rect(tx + sp - 0.75, deck - 54.0, 1.5, 9.0, span); }
    }

    // The elevated train, now and then, along its viaduct.
    let rail = HORIZON + 18.0 + shake;
    blip.fill_rect(0.0, rail, WIN_W as f32, 5.0, iron);
    for i in 0..11 { blip.fill_rect(20.0 + i as f32 * 60.0, rail, 6.0, 22.0, iron); }
    if let Some(x) = cameo_x(t, 5.0) {
        for car in 0..4 {
            let cx = x - car as f32 * 64.0;
            blip.fill_rect(cx, rail - 18.0, 60.0, 18.0, c(0.66, 0.68, 0.72));
            blip.fill_rect(cx, rail - 6.0, 60.0, 2.0, c(0.60, 0.20, 0.50));
            for w in 0..5 { blip.fill_rect(cx + 5.0 + w as f32 * 11.0, rail - 15.0, 8.0, 7.0, c(1.0, 0.90, 0.56)); }
        }
    }

    // The next roof: a water tower, washing on a line, the neighbours.
    let roof = HORIZON + 40.0 + shake;
    let wood = c(0.42, 0.26, 0.18);
    for lx in [66.0f32, 86.0, 106.0] { blip.fill_rect(lx, roof - 34.0, 3.0, 34.0, iron); }
    blip.fill_rect(58.0, roof - 78.0, 60.0, 46.0, wood);
    for b in 0..3 { blip.fill_rect(58.0, roof - 70.0 + b as f32 * 14.0, 60.0, 2.0, shade(wood, 0.6)); }
    stroke(blip, V(60.0, roof - 78.0), V(88.0, roof - 98.0), 1.0, 1.0, shade(wood, 0.7));
    for k in 0..15 {
        let y = roof - 78.0 - k as f32 * 1.4;
        let half = 30.0 - k as f32 * 2.0;
        blip.fill_rect(88.0 - half, y, half * 2.0, 1.6, shade(wood, 0.7));
    }
    blip.fill_rect(470.0, roof - 44.0, 2.0, 44.0, iron);
    blip.fill_rect(600.0, roof - 44.0, 2.0, 44.0, iron);
    blip.fill_rect(470.0, roof - 42.0, 132.0, 1.0, iron);
    for (i, col) in [c(0.90, 0.90, 0.94), c(0.80, 0.26, 0.24), c(0.30, 0.44, 0.78), c(0.94, 0.80, 0.36)]
        .into_iter().enumerate() {
        let sway = (t * 1.8 + i as f32).sin() * 1.5;
        blip.fill_rect(484.0 + i as f32 * 28.0 + sway, roof - 41.0, 16.0, 18.0 + (i % 2) as f32 * 6.0, col);
    }
    draw_crowd(blip, roof + 2.0, 26.0, hazed(c(0.20, 0.14, 0.22), low, 0.10), t, 0xb9d, 26, roar);

    // The parapet: brick, with a stone cap, right across behind the fight.
    let brick = c(0.50, 0.24, 0.20);
    blip.fill_rect(0.0, roof, WIN_W as f32, FLOOR_Y - roof + shake.abs() + 1.0, brick);
    for r in 0..9 {
        let y = roof + 8.0 + r as f32 * 9.0;
        if y > FLOOR_Y + shake { break; }
        blip.fill_rect(0.0, y, WIN_W as f32, 1.0, shade(brick, 0.72));
        for i in 0..18 {
            blip.fill_rect(i as f32 * 38.0 + (r % 2) as f32 * 19.0, y - 8.0, 1.0, 8.0, shade(brick, 0.72));
        }
    }
    blip.fill_rect(0.0, roof - 2.0, WIN_W as f32, 7.0, c(0.74, 0.70, 0.66));
    blip.fill_rect(0.0, roof + 5.0, WIN_W as f32, 2.0, shade(brick, 0.6));

    // A vent and an aerial at either edge of this roof.
    blip.fill_rect(8.0, FLOOR_Y - 40.0 + shake, 34.0, 40.0, c(0.46, 0.48, 0.52));
    blip.fill_rect(4.0, FLOOR_Y - 46.0 + shake, 42.0, 8.0, c(0.58, 0.60, 0.64));
    for v in 0..4 { blip.fill_rect(12.0, FLOOR_Y - 32.0 + v as f32 * 7.0 + shake, 26.0, 2.0, c(0.24, 0.24, 0.28)); }
    blip.fill_rect(612.0, FLOOR_Y - 96.0 + shake, 2.5, 96.0, iron);
    for (y, w) in [(90.0f32, 26.0f32), (78.0, 18.0), (66.0, 12.0)] {
        blip.fill_rect(613.0 - w / 2.0, FLOOR_Y - y + shake, w, 2.0, iron);
    }
}

/// THE BATH HOUSE: indoors, steam off the tub, and on the wall the painted
/// mountain every bath house has. The tub's wooden side is the quiet band.
pub(crate) fn draw_bathhouse(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 268.0;
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let wall = c(0.86, 0.80, 0.66);
    let wood = c(0.42, 0.27, 0.16);
    blip.fill_rect(0.0, shake, WIN_W as f32, FLOOR_Y, wall);

    // The mural: sky, a red sun, the mountain with its snow, a line of surf.
    let (mx, my, mw, mh) = (70.0, 44.0 + shake, 500.0, 164.0);
    blip.fill_rect(mx, my, mw, mh, c(0.56, 0.78, 0.92));
    blip.fill_circle(452.0, my + 52.0, 30.0, c(0.90, 0.26, 0.20));
    let rows = 16;
    for r in 0..rows {
        let f = r as f32 / rows as f32;
        let w = 330.0 * (1.0 - f) + 26.0;
        let peak = if f > 0.72 { c(0.96, 0.97, 1.0) } else { c(0.28, 0.42, 0.70) };
        blip.fill_rect(300.0 - w / 2.0, my + mh - 18.0 - (r + 1) as f32 * 8.0, w, 9.0, peak);
    }
    blip.fill_rect(mx, my + mh - 18.0, mw, 18.0, c(0.22, 0.46, 0.66));
    for i in 0..21 { blip.fill_circle(mx + 12.0 + i as f32 * 24.0, my + mh - 17.0, 7.0, c(0.90, 0.95, 1.0)); }
    for (x, y, w, h) in [(mx - 6.0, my - 6.0, mw + 12.0, 6.0), (mx - 6.0, my + mh, mw + 12.0, 6.0),
                         (mx - 6.0, my, 6.0, mh), (mx + mw, my, 6.0, mh)] {
        blip.fill_rect(x, y, w, h, wood);
    }

    // The beam, and paper lanterns hung from it.
    blip.fill_rect(0.0, shake, WIN_W as f32, 26.0, wood);
    blip.fill_rect(0.0, 26.0 + shake, WIN_W as f32, 4.0, shade(wood, 0.7));
    for i in 0..4 {
        let x = 34.0 + i as f32 * 190.0;
        let glow = 0.88 + 0.12 * (t * 8.0 + i as f32 * 2.0).sin();
        blip.fill_glow_circle(x, 46.0 + shake, 18.0, BlipColor { r: 1.0, g: 0.78, b: 0.40, a: 0.16 * glow });
        blip.fill_circle(x, 46.0 + shake, 9.0, c(0.96, 0.90, 0.72));
        blip.fill_rect(x - 9.0, 44.0 + shake, 18.0, 2.0, c(0.74, 0.20, 0.18));
    }

    // Tiles down to the water, the bathers in it, and steam rising off it.
    let tile = c(0.72, 0.84, 0.88);
    blip.fill_rect(0.0, 214.0 + shake, WIN_W as f32, HORIZON - 214.0, tile);
    for i in 0..33 { blip.draw_line(i as f32 * 20.0, 214.0 + shake, i as f32 * 20.0, HORIZON + shake, shade(tile, 0.86)); }
    for j in 0..3 { blip.draw_line(0.0, 214.0 + j as f32 * 18.0 + shake, WIN_W as f32, 214.0 + j as f32 * 18.0 + shake, shade(tile, 0.86)); }
    draw_crowd(blip, HORIZON + 8.0 + shake, 26.0, c(0.62, 0.44, 0.34), t, 0xb47, 22, roar);
    blip.fill_rect(0.0, HORIZON + shake, WIN_W as f32, 9.0, c(0.36, 0.64, 0.74));
    for i in 0..9 {
        let sx = scatter(i, 0xb5);
        let rise = (t * (0.10 + sx * 0.08) + sx).fract();
        blip.fill_circle(30.0 + i as f32 * 72.0 + (t + sx * 6.0).sin() * 6.0, HORIZON - rise * 70.0 + shake,
            9.0 + rise * 12.0, BlipColor { r: 1.0, g: 1.0, b: 1.0, a: 0.16 * (1.0 - rise) });
    }

    // The tub's side: one flat run of boards under a pale rim.
    let tub = c(0.60, 0.42, 0.24);
    blip.fill_rect(0.0, HORIZON + 9.0 + shake, WIN_W as f32, FLOOR_Y - HORIZON - 9.0, tub);
    blip.fill_rect(0.0, HORIZON + 7.0 + shake, WIN_W as f32, 6.0, blend(tub, BLIP_WHITE, 0.3));
    for i in 0..17 { blip.draw_line(i as f32 * 40.0, HORIZON + 13.0 + shake, i as f32 * 40.0, FLOOR_Y + shake, shade(tub, 0.74)); }
    for y in [HORIZON + 26.0, FLOOR_Y - 12.0] { blip.fill_rect(0.0, y + shake, WIN_W as f32, 3.0, shade(tub, 0.6)); }

    // Wash buckets stacked at either edge.
    for (x, n) in [(6.0f32, 3), (606.0, 2)] {
        for i in 0..n {
            let top = FLOOR_Y - 18.0 * (i + 1) as f32 + shake;
            blip.fill_rect(x, top, 28.0, 18.0, c(0.84, 0.68, 0.36));
            blip.fill_rect(x, top + 5.0, 28.0, 2.0, c(0.50, 0.36, 0.18));
            blip.draw_rect(x, top, 28.0, 18.0, c(0.46, 0.32, 0.16));
        }
    }
}

/// THE RIVER VILLAGE: a jetty on a slow green river, a hut on stilts, the
/// jungle behind and something long and green in the big tree.
pub(crate) fn draw_village(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 250.0;
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let low = c(0.94, 0.93, 0.70);
    draw_sky(blip, shake, HORIZON, c(0.50, 0.78, 0.76), low);

    // Jungle in two ranges of rounded crowns, and palms above them.
    for (base, r0, salt, k, n) in [(HORIZON - 26.0, 22.0f32, 0xc1u32, 0.50f32, 22usize),
                                   (HORIZON - 6.0, 28.0, 0xc2, 0.22, 16)] {
        let leaf = hazed(c(0.12, 0.32, 0.18), low, k);
        for i in 0..n {
            let x = (i as f32 + scatter(i, salt)) * WIN_W as f32 / (n - 1) as f32;
            blip.fill_circle(x, base + shake, r0 + scatter(i, salt ^ 9) * 16.0, leaf);
        }
        blip.fill_rect(0.0, base + shake, WIN_W as f32, HORIZON - base + 8.0, leaf);
    }
    let palm = hazed(c(0.10, 0.26, 0.16), low, 0.30);
    for (x, top) in [(232.0f32, 120.0f32), (402.0, 96.0), (470.0, 138.0)] {
        let sway = (t * 0.8 + x).sin() * 3.0;
        blip.draw_line_ex(x, HORIZON - 20.0 + shake, x + sway, top + shake, 4.0, palm);
        for a in [-2.6f32, -2.0, -1.2, -0.5, 0.1] {
            stroke(blip, V(x + sway, top + shake), V(x + sway + a.cos() * 34.0, top - 6.0 + a.sin().abs() * 20.0 + shake),
                3.0, 1.0, palm);
        }
    }

    // The far bank and the villagers on it; the river, slow and green.
    draw_crowd(blip, HORIZON + 8.0 + shake, 28.0, hazed(c(0.24, 0.18, 0.12), low, 0.12), t, 0xc5d, 28, roar);
    blip.fill_rect(0.0, HORIZON + 4.0 + shake, WIN_W as f32, 10.0, c(0.46, 0.36, 0.22));
    let river = c(0.30, 0.42, 0.32);
    blip.fill_rect(0.0, HORIZON + 14.0 + shake, WIN_W as f32, FLOOR_Y - HORIZON - 14.0, river);
    for i in 0..20 {
        let sx = scatter(i, 0xc7);
        let y = HORIZON + 20.0 + sx * 50.0 + shake;
        let x = ((i as f32 * 67.0) + t * (5.0 + sx * 7.0)) % (WIN_W as f32 + 60.0) - 30.0;
        blip.fill_rect(x, y, 16.0 + sx * 22.0, 1.5, BlipColor { r: 0.80, g: 0.90, b: 0.70, a: 0.20 });
    }

    // A hut on stilts, upstream.
    let (hx, hy) = (36.0, HORIZON - 30.0 + shake);
    for sx in [8.0f32, 44.0, 84.0, 118.0] { blip.draw_line_ex(hx + sx, hy + 44.0, hx + sx, HORIZON + 30.0 + shake, 3.0, c(0.30, 0.20, 0.12)); }
    blip.fill_rect(hx, hy, 126.0, 46.0, c(0.52, 0.36, 0.20));
    blip.fill_rect(hx + 50.0, hy + 14.0, 24.0, 32.0, c(0.16, 0.10, 0.08));
    for r in 0..6 {
        let f = r as f32 / 6.0;
        blip.fill_rect(hx - 14.0 + f * 60.0, hy - 8.0 - r as f32 * 8.0, 154.0 - f * 120.0, 9.0,
            shade(c(0.72, 0.58, 0.30), 1.0 - 0.06 * (r % 2) as f32));
    }

    // The big tree at the water's edge, its vines, and the snake asleep on a
    // branch: only its tail moves.
    let bark = c(0.28, 0.19, 0.12);
    blip.fill_rect(566.0, shake, 50.0, FLOOR_Y - 30.0, bark);
    blip.fill_rect(566.0, shake, 10.0, FLOOR_Y - 30.0, blend(bark, BLIP_WHITE, 0.12));
    stroke(blip, V(580.0, 108.0 + shake), V(470.0, 84.0 + shake), 10.0, 5.0, bark);
    for (x, y, r) in [(560.0f32, 26.0f32, 46.0f32), (610.0, 50.0, 40.0), (500.0, 40.0, 34.0), (640.0, 10.0, 40.0)] {
        blip.fill_circle(x, y + shake, r, c(0.14, 0.36, 0.20));
    }
    for (x, len) in [(520.0f32, 70.0f32), (548.0, 104.0), (626.0, 90.0)] {
        blip.draw_line(x, 60.0 + shake, x + (t * 0.9 + x).sin() * 3.0, 60.0 + len + shake, c(0.20, 0.44, 0.24));
    }
    let scale = c(0.36, 0.62, 0.22);
    let mut at = V(540.0, 92.0 + shake);
    for i in 0..9 {
        let next = V(at.0 - 9.0, 86.0 + shake + if i % 2 == 0 { 9.0 } else { -3.0 });
        stroke(blip, at, next, 4.0, 4.0, scale);
        at = next;
    }
    stroke(blip, at, V(at.0 - 4.0, at.1 + 18.0 + (t * 1.6).sin() * 4.0), 3.0, 1.0, scale);
    blip.fill_circle(544.0, 90.0 + shake, 5.5, scale);
    blip.fill_circle(546.0, 88.0 + shake, 1.3, c(0.96, 0.90, 0.30));

    // The jetty's edge: posts and a rope.
    let post = c(0.40, 0.28, 0.16);
    blip.fill_rect(0.0, FLOOR_Y - 30.0 + shake, WIN_W as f32, 30.0, post);
    blip.fill_rect(0.0, FLOOR_Y - 32.0 + shake, WIN_W as f32, 3.0, blend(post, BLIP_WHITE, 0.22));
    for i in 0..22 { blip.draw_line(i as f32 * 30.0, FLOOR_Y - 29.0 + shake, i as f32 * 30.0, FLOOR_Y + shake, shade(post, 0.72)); }
    blip.fill_rect(0.0, FLOOR_Y - 16.0 + shake, WIN_W as f32, 2.0, c(0.74, 0.66, 0.46));
}
