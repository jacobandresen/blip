//! The stages: a backdrop each (one module per stage below) and the floor under them, with the crowds and passers-by
//! they share. Scenery only: the floor line, the walls and the height are the same everywhere.

use super::*;

mod airbase;
mod bathhouse;
mod colosseum;
mod dock;
mod fortress;
mod hood;
mod island;
mod pagoda;
mod rooftop;
mod temple;
mod village;
pub(crate) use airbase::*;
pub(crate) use bathhouse::*;
pub(crate) use colosseum::*;
pub(crate) use dock::*;
pub(crate) use fortress::*;
use hood::*;
pub(crate) use island::*;
pub(crate) use pagoda::*;
pub(crate) use rooftop::*;
pub(crate) use temple::*;
pub(crate) use village::*;

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

/// A sky in flat bands from `top` down to `low` at the horizon, and `low`
/// carried on down to the floor so no layer leaves a hole.
pub(crate) fn draw_sky(blip: &Blip, shake: f32, horizon: f32, top: BlipColor, low: BlipColor) {
    blip.fill_bands(0.0, shake, WIN_W as f32, horizon, 14, |k| blend(top, low, k));
    blip.fill_rect(0.0, horizon + shake, WIN_W as f32, FLOOR_Y - horizon, low);
}
