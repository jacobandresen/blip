//! A whole fighter on the stage, and what flies and lands around them:
//! bolts, the web, the bonus fruit, hit splashes and the words that pop up.

use super::*;

std::thread_local! {
    /// Drawing the second of two of the same fighter: the alternate colours, as an arcade gives player two.
    pub(crate) static SECOND: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A fighter's colours as the second copy of them on screen: DRAKE in black with yellow stripes, everyone else
/// with their main colour taken well down, so a mirror match is two fighters and not one twice.
pub(crate) fn second_look(l: Look) -> Look {
    // DRAKE's other colours are Way of the Dragon's: bare-chested in black kung fu trousers.
    if l.build == Build::Track {
        return Look { cloth: rgb((0.09, 0.09, 0.11)), trim: rgb((0.09, 0.09, 0.11)), torn: true, ..l };
    }
    // LANCER's are navy karate trousers: taken down, the white went a dull grey.
    if l.build == Build::Duelist {
        return Look { cloth: rgb((0.16, 0.20, 0.36)), ..l };
    }
    Look { cloth: shade(l.cloth, 0.55), ..l }
}

/// One of the two fighters, with the hit shake and (in versus) the player's
/// mark under them.
pub(crate) fn draw_fighter(blip: &Blip, g: &Game, i: usize) {
    let shake = g.shake_px();
    // The alternate colours: picked with the kick, or the second of a mirror match.
    let human = g.mode == Mode::Versus || i == 0;
    SECOND.set((i == 1 && g.p[0].who == g.p[1].who) || (human && !g.demo && g.costume[i]));

    // In an exchange the two fighters overlap four frames in five, so in
    // versus a mark in each player's colour on the boards under them shows
    // which is yours.
    if g.mode == Mode::Versus {
        let c = if i == 0 { BlipColor { r: 1.0, g: 0.35, b: 0.35, a: 0.55 } }
                else { BlipColor { r: 0.40, g: 0.72, b: 1.0, a: 0.55 } };
        let f = &g.p[i];
        let w = 21.0 * f.arch().bulk * f.size();
        stroke(blip, V(f.x - w, FLOOR_Y + shake + 2.0), V(f.x + w, FLOOR_Y + shake + 2.0),
            2.2, 2.2, c);
    }
    draw_body(blip, &g.p[i], g.now, shake, g.hitstop, i, true);
    SECOND.set(false);
}

/// The face a fighter is wearing for what they are doing.
pub(crate) fn mood_of(f: &Fighter) -> Mood {
    match f.act {
        Act::Attack => Mood::Shout,
        Act::Hitstun | Act::Knockdown | Act::Defeat => Mood::Hurt,
        // The crunch is roared, every other win smiled.
        Act::Victory if STYLES[f.who].win == Win::Flex && f.t > 0.3 => Mood::Shout,
        // DRAKE's stamp is cried out, the face screwed up.
        Act::Victory if formless(f.who) && f.stumble_roll % 4 == 3 && !plucking(f) && f.t > 0.25 => Mood::Shout,
        Act::Victory => Mood::Happy,
        // Rocked, and it shows on the face too; and gritted teeth behind a guard taking a blow.
        _ if f.stagger > 0.0 => Mood::Hurt,
        Act::Block if f.stun > 0.0 => Mood::Hurt,
        _ => Mood::Calm,
    }
}

/// Draw one fighter in depth order: far limbs, body and head, then near limbs.
/// `shift` affects drawing only; `shadow` disables the floor shadow.
pub(crate) fn draw_body(blip: &Blip, f: &Fighter, now: f32, shift: f32, hitstop: f32, i: usize, shadow: bool) {
    let a = f.arch();
    let prone = down_time(f).is_some_and(|t| t < 0.9);
    // Held in the hitstop, the one struck shudders on the spot.
    let shudder = if hitstop > 0.0 && f.act == Act::Hitstun { 2.0 * if ((now * 60.0) as i32) % 2 == 0 { 1.0 } else { -1.0 } } else { 0.0 };
    // Turning round, the face goes with the body: still the old way until halfway.
    // (And through a spinning back kick, looking back over the shoulder.)
    let face = if f.turn > 0.5 || spin_turn(f) > 0.5 { -f.facing } else { f.facing };
    let rig = Rig::upright(f.x + shudder, f.y + shift, f.facing, face, a.size);

    let flash = if hitstop > 0.0 && f.act == Act::Hitstun { 0.5 }
                else if invulnerable(f) && ((now * 30.0) as i32) % 2 == 0 { 0.3 }
                else { 0.0 };
    // Loose ends trail: back when moving forward, down on the way up, up on
    // the way down, with a small flutter at rest.
    let drift = match f.act {
        Act::Walk => a.walk,
        // The head snaps back and the loose ends are left behind it, flung forward.
        Act::Hitstun if !f.struck_body && !f.struck_legs => -300.0 * (1.0 - f.t * 3.4).clamp(0.0, 1.0) * f.struck_force,
        _ => f.vx * f.facing,
    };
    let rise = if f.airborne() { f.vy } else { 0.0 };
    let wind = (-(drift / 260.0).clamp(-1.0, 1.0) * 0.5 + (now * 5.0 + i as f32).sin() * 0.05,
                -(rise / 600.0).clamp(-1.0, 1.0) * 0.7);
    // A blink every three seconds or so, the two fighters out of step.
    let blink = (now * 0.31 + i as f32 * 0.37).fract() < 0.035;
    let claw = ARTS[f.who] == Art::HungGar;
    // DRAKE fights on with the suit top torn away once he has taken real punishment (under 45%).
    let torn = formless(f.who) && (f.health as f32) < a.health as f32 * 0.45;
    let near = Look { wind, blink, prone, floor: FLOOR_Y + shift, claw, torn, scars: torn, ..look_of(&a, flash) };
    let near = if SECOND.get() { second_look(near) } else { near };
    let far = Look { far: true, tone: FAR, ..near };

    if shadow && a.build == Build::Uniform && f.act == Act::Attack && f.mv == MoveId::Special {
        let pulse = 38.0 + 4.0 * (now * 14.0).sin();
        blip.fill_glow_circle(f.x, f.y + shift - 60.0, pulse,
            BlipColor { r: 0.62, g: 0.22, b: 0.94, a: 0.53 });
    }

    // A special gathers before it goes: a glow in the fighter's own colour swells through the startup.
    // It is the tell (the laser has the eyes, and Regent his own violet).
    if shadow && f.act == Act::Attack && f.mv == MoveId::Special && a.build != Build::Uniform
        && a.special != Special::LaserVision {
        let start = f.scaled(move_data(MoveId::Special)).startup * F;
        if f.t < start + 3.0 * F {
            let k = (f.t / start).min(1.0);
            let fade = if f.t > start { 1.0 - (f.t - start) / (3.0 * F) } else { 1.0 };
            // Half white, so a red trim does not read as the rage behind a fighter.
            let (r, g, b) = a.trim;
            let (r, g, b) = (0.5 + 0.5 * r, 0.5 + 0.5 * g, 0.5 + 0.5 * b);
            blip.fill_glow_circle(f.x, f.y + shift - f.height() * 0.5, f.height() * (0.16 + 0.14 * k),
                BlipColor { r, g, b, a: (0.12 + 0.24 * k) * fade });
        }
    }

    // Dust off the boards where a body lands. Knockdowns are the one
    // moment the floor is part of the fight.
    // (On landing, at the end of the fall.)
    if let Some(t) = down_time(f).map(|t| t - (FALL - 0.04)).filter(|t| (0.0..0.26).contains(t)) {
        let k = (1.0 - t / 0.26).max(0.0);
        for j in 0..5 {
            let dx = (j as f32 - 2.0) * 13.0 - f.facing * 14.0;
            let rise = (1.0 - k) * 16.0;
            blip.fill_circle(f.x + dx, FLOOR_Y + shift - 3.0 - rise, 3.0 + 7.0 * (1.0 - k),
                BlipColor { r: 0.62, g: 0.55, b: 0.46, a: 0.34 * k });
        }
    }

    // And a puff either side of the feet coming down from a jump, bigger
    // the harder the landing.
    if f.land > 0.0 && f.act != Act::Knockdown {
        let age = 1.0 - f.land / LAND_ABSORB;
        let size = f.size().max(0.6) * (0.5 + 0.5 * f.land_force);
        for side in [-1.0f32, 1.0] {
            let x = f.x + side * (12.0 + 18.0 * age) * size;
            blip.fill_circle(x, FLOOR_Y + shift - 2.0 - 5.0 * age, (3.0 + 5.0 * age) * size,
                BlipColor { r: 0.66, g: 0.60, b: 0.52, a: 0.42 * (1.0 - age) });
        }
    }

    // DRAKE's pluck: three hairs blown off the fingertips, drifting up and away and fading.
    if plucking(f) && (0.75..2.6).contains(&f.t) {
        let k = ((f.t - 0.8) / 1.8).max(0.0);
        let z = f.size();
        let tip = V(f.x + f.facing * 30.0 * z, f.y + shift - 96.0 * z);
        // The breath: three short strokes out from the lips across the fingertips.
        if f.t < 1.05 {
            for j in 0..3 {
                let y = tip.1 - 3.0 + 3.0 * j as f32;
                stroke(blip, V(tip.0 - f.facing * 8.0, y), V(tip.0 + f.facing * 4.0, y), 0.6, 0.6,
                    BlipColor { r: 1.0, g: 1.0, b: 1.0, a: 0.7 });
            }
        }
        for j in 0..3 {
            let drift = V(tip.0 + f.facing * (8.0 + 46.0 * k + 7.0 * j as f32),
                tip.1 - 4.0 - 16.0 * k + 5.0 * (k * 9.0 + j as f32 * 2.0).sin());
            let c = BlipColor { r: 0.30, g: 0.20, b: 0.10, a: (1.0 - k).min(1.0) };
            let bend = 2.0 * (k * 7.0 + j as f32).cos();
            stroke(blip, drift, V(drift.0 + 6.0 * f.facing, drift.1 + bend), 0.9, 0.6, c);
        }
    }
    // DRAKE's stamp throws up dust round the foot.
    if formless(f.who) && f.act == Act::Victory && f.stumble_roll % 4 == 3 && !plucking(f) && (0.3..0.6).contains(&f.t) {
        let k = (f.t - 0.3) / 0.3;
        for side in [-1.0f32, 1.0] {
            blip.fill_circle(f.x + f.facing * 18.0 + side * (6.0 + 16.0 * k), FLOOR_Y + shift - 3.0 - 4.0 * k, 3.0 + 5.0 * k,
                BlipColor { r: 0.66, g: 0.60, b: 0.52, a: 0.45 * (1.0 - k) });
        }
    }
    // Sent sliding by the one-inch punch: dust kicked up along the boards behind the body.
    if f.act == Act::Knockdown && f.vx.abs() > 80.0 {
        let k = (f.vx.abs() / 560.0).min(1.0);
        for j in 0..3 {
            let back = f.x - f.vx.signum() * (10.0 + 14.0 * j as f32);
            blip.fill_circle(back, FLOOR_Y + shift - 3.0 - 3.0 * j as f32, (3.0 + 2.5 * j as f32) * k,
                BlipColor { r: 0.66, g: 0.60, b: 0.52, a: 0.40 * k });
        }
    }

    // Pushing off for a jump kicks up a puff where the feet left the boards (it stays there).
    if f.act == Act::Air && f.vy < 0.0 && f.t < 0.16 && !f.soaring() {
        let age = f.t / 0.16;
        let size = f.size().max(0.6);
        let at = f.x - f.vx * f.t;
        for side in [-1.0f32, 1.0] {
            blip.fill_circle(at + side * (8.0 + 14.0 * age) * size, FLOOR_Y + shift - 2.0 - 3.0 * age, (2.5 + 4.0 * age) * size,
                BlipColor { r: 0.66, g: 0.60, b: 0.52, a: 0.36 * (1.0 - age) });
        }
    }
    // The heavy ones raise a little dust with every step: each foot as it comes down.
    if f.act == Act::Walk && a.bulk >= 1.12 {
        let ph = walk_phase(f);
        for off in [0.0f32, std::f32::consts::PI] {
            let u = ((ph + off) / std::f32::consts::TAU).rem_euclid(1.0);
            // The first fifth of the planted half: the foot just down.
            if u < 0.10 {
                let age = u / 0.10;
                let (fwd, _) = gait_foot(GAITS[f.who], ph + off, 0.0);
                let x = f.x + f.facing * (WALK_MID + fwd) * f.size();
                blip.fill_circle(x, FLOOR_Y + shift - 2.0, (2.0 + 4.0 * age) * f.size(),
                    BlipColor { r: 0.66, g: 0.60, b: 0.52, a: 0.30 * (1.0 - age) });
            }
        }
    }

    // A guard driven back by a blow scuffs the boards behind the back foot.
    if f.act == Act::Block && f.stun > 0.0 && !f.airborne() {
        let k = (f.stun / (10.0 * F)).clamp(0.0, 1.0);
        let x = f.x - f.facing * (24.0 + 10.0 * (1.0 - k)) * f.size();
        blip.fill_circle(x, FLOOR_Y + shift - 2.0 - 3.0 * (1.0 - k), (2.0 + 4.0 * (1.0 - k)) * f.size(),
            BlipColor { r: 0.66, g: 0.60, b: 0.52, a: 0.38 * k });
    }

    // A flat oval on the boards, smaller under a jump: what glues the body
    // down, and how a player judges where a jump will land.
    if shadow {
        let lift = ((FLOOR_Y - f.y) / 120.0).clamp(0.0, 1.0);
        let (w, h) = (30.0 * a.bulk * a.size * (1.0 - 0.45 * lift), 5.0);
        let c = BlipColor { r: 0.0, g: 0.0, b: 0.0, a: 0.34 * (1.0 - 0.4 * lift) };
        for row in 0..h as i32 * 2 {
            let y = (row as f32 + 0.5) / h - 1.0;
            let half = w * (1.0 - y * y).sqrt();
            blip.fill_rect(f.x - half, FLOOR_Y + shift + 1.0 + row as f32 - h, half * 2.0, 1.0, c);
        }
    }

    // Rage: a red heat behind the fighter, beating.
    if f.enraged() && !matches!(f.act, Act::Defeat | Act::Victory | Act::Bow) {
        let beat = 0.5 + 0.5 * (now * 9.0).sin();
        // DRAKE's is gold, not red: a dragon's heat, not a temper.
        let heat = if formless(f.who) { BlipColor { r: 1.0, g: 0.72, b: 0.12, a: 0.18 + 0.12 * beat } }
                   else { BlipColor { r: 1.0, g: 0.18, b: 0.10, a: 0.20 + 0.12 * beat } };
        blip.fill_glow_circle(f.x, f.y + shift - f.height() * 0.5, f.height() * (0.42 + 0.06 * beat), heat);
    }

    // A turtle guarding low is drawn as what it is: a shell on the boards
    // with a pair of eyes under the rim. The shell ram is the same shell off
    // the boards and spinning, the mask coming round and going away again.
    let ramming = a.build == Build::Turtle && a.special == Special::BullRush
        && f.act == Act::Attack && f.mv == MoveId::Special && !f.calling
        && f.t >= f.scaled(move_data(MoveId::Special)).startup * F;
    if f.shelled() || ramming {
        // As tall as the crouch it stands for, or blows land on thin air.
        let b = a.bulk * a.size * 1.7;
        let (gy, fw) = (f.y + shift - if ramming { 9.0 * b } else { 0.0 }, f.facing);
        let turn = if ramming { (now * 26.0).cos() } else { 1.0 };
        if !ramming {
            shape(blip, &[(V(f.x - 13.0 * b, gy - 4.0 * b - LINE), V(f.x + 15.0 * b, gy - 4.0 * b - LINE), 3.2 * b, 3.2 * b)],
                near.c(near.skin), near.ink());
        }
        shape(blip, &[(V(f.x - 8.0 * b, gy - 15.0 * b), V(f.x + 8.0 * b, gy - 15.0 * b), 12.0 * b, 12.0 * b)],
            near.c(near.cloth), near.ink());
        stroke(blip, V(f.x - 17.0 * b, gy - 8.0 * b), V(f.x + 17.0 * b, gy - 8.0 * b), 2.6 * b, 2.6 * b,
            near.c(PLASTRON));
        // The seams between the plates, going round with it.
        for k in [-1.0f32, 0.0, 1.0] {
            let x = f.x + (k * 7.0 + turn * 3.0) * b;
            stroke(blip, V(x, gy - 24.0 * b), V(x, gy - 13.0 * b), 0.6 * b, 0.6 * b, shade(near.c(near.cloth), 0.66));
        }
        // The mask is on the side that is toward us for half of each turn.
        if turn > 0.0 {
            let at = f.x + fw * 13.0 * b * turn;
            stroke(blip, V(at - 4.0 * b, gy - 13.0 * b), V(at + 4.0 * b, gy - 13.0 * b), 2.6 * b, 2.6 * b,
                near.c(near.trim));
            blip.fill_circle(at + fw * 2.0 * b, gy - 13.0 * b, 1.7 * b, BLIP_WHITE);
        }
        return;
    }

    let q = pose_of(now, f, i);
    let k = skeleton(rig, &q);
    light_from(Some(rig.fwd));
    if matches!(a.build, Build::Caped | Build::Uniform) {
        draw_cape(blip, &far, k.hip, k.neck, rig.fwd, if prone { 1.0 } else { f.soar }, now, FLOOR_Y + shift);
    }
    draw_leg(blip, &far, k.hip_rear, k.knee_rear, k.ankle_rear, rig.fwd, rig.ground, q.rear_heel);
    // DRAKE's nunchaku, tucked in the back of his waistband when they are not in his hand.
    let in_hand = (f.act == Act::Attack && f.mv == MoveId::Special && f.special() == Special::Nunchaku)
        || (f.act == Act::Victory && f.t < 1.6 && f.stumble_roll % 4 == 0 && !plucking(f)) || (f.act == Act::Bow && intro_now(f) != Intro::WarmUp && f.t > 0.15 && f.t < 0.6)
        || (f.act == Act::Idle && f.taunt > 0.15 && f.taunt < 1.25 && f.stumble_roll % 2 == 1);
    if formless(f.who) && !in_hand && !prone {
        let z = a.size;
        let (sx, sy) = unit(k.hip, k.neck);
        let belt = along(k.hip, k.neck, 0.30);
        // (Far enough back to show behind him: 8 px hid them behind the torso.)
        let back = V(belt.0 - rig.fwd * 14.0 * z, belt.1);
        let wood = near.c(BlipColor { r: 0.34, g: 0.20, b: 0.11, a: 1.0 });
        for lean in [-0.25f32, 0.25] {
            let top = V(back.0 + sx * 10.0 * z + lean * 4.0 * z, back.1 + sy * 10.0 * z);
            let bottom = V(back.0 - sx * 6.0 * z - lean * 2.0 * z, back.1 - sy * 6.0 * z);
            shape(blip, &[(top, bottom, 1.8 * z, 1.8 * z)], wood, INK);
        }
    }
    // The far arm is beyond the chest: what shows of it is what clears it.
    draw_arm(blip, &far, k.sh_rear, k.elbow_rear, k.hand_rear, q.open, rig.fwd);
    draw_torso(blip, &near, k.hip, k.neck, rig.fwd);
    draw_head(blip, &near, rig.face, k.head, k.neck, mood_of(f));
    draw_leg(blip, &near, k.hip_lead, k.knee_lead, k.ankle_lead, rig.fwd, rig.ground, q.lead_heel);
    // The near thigh grows out of the pelvis: without this its outline
    // closes across the hip and draws a buttock on the side of the body.
    {
        let b = near.bulk;
        let (sx, sy) = unit(k.hip, k.neck);
        let from = V(k.hip.0 - sx * b, k.hip.1 - sy * b);
        let hip_c = near.c(if near.build == Build::Turtle { near.skin } else { near.cloth });
        stroke(blip, from, along(k.hip_lead, k.knee_lead, 0.24), 6.8 * b, 6.4 * b, hip_c);
    }
    draw_belt(blip, &near, k.hip, k.neck, rig.fwd);
    draw_arm(blip, &near, k.sh_lead, k.elbow_lead, k.hand_lead, q.open, rig.fwd);
    light_from(None);
    if (f.act == Act::Attack && f.mv == MoveId::Special && f.special() == Special::Nunchaku)
        || (f.act == Act::Victory && formless(f.who) && f.t < 1.6 && f.stumble_roll % 4 == 0 && !plucking(f))
        || (f.act == Act::Bow && formless(f.who) && !f.bows && intro_now(f) != Intro::WarmUp && f.t > 0.15 && f.t < 0.6)
        || (f.act == Act::Idle && formless(f.who) && f.taunt > 0.15 && f.taunt < 1.25 && f.stumble_roll % 2 == 1) {
        draw_sticks(blip, f, k.hand_lead, rig.fwd);
    }

    // Stars going round the head of a dizzy fighter, in front and behind; and over one knocked out cold,
    // once they have stopped moving.
    let out_cold = f.act == Act::Defeat && f.health <= 0 && f.t > 0.7;
    if f.dizzy > 0.0 || out_cold {
        let r = head_r(a.bulk) * f.size().max(0.6);
        for j in 0..3 {
            let ang = now * 6.0 + j as f32 * std::f32::consts::TAU / 3.0;
            let at = V(k.head.0 + ang.cos() * r * 1.5, k.head.1 - r * 1.25 + ang.sin() * r * 0.4);
            for (arm, col) in [(4.6, INK), (3.2, BLIP_YELLOW)] {
                stroke(blip, V(at.0 - arm, at.1), V(at.0 + arm, at.1), arm * 0.34, arm * 0.34, col);
                stroke(blip, V(at.0, at.1 - arm), V(at.0, at.1 + arm), arm * 0.34, arm * 0.34, col);
            }
        }
    }

    // The laser's tell: the eyes light up through the whole of its startup.
    if f.act == Act::Attack && f.mv == MoveId::Special && a.special == Special::LaserVision {
        // The stare is the fighter's own (scaled), not the table's: it lasts the whole LASER_STARE.
        let stare = f.scaled(move_data(f.mv)).startup;
        let charge = (f.t / (stare * F)).min(1.0);
        if f.t < (stare + 6.0) * F {
            let r = head_r(a.bulk) * a.size;
            let eye = V(k.head.0 + rig.face * 0.5 * r, k.head.1 - 0.08 * r);
            blip.fill_glow_circle(eye.0, eye.1, 6.0 + 12.0 * charge,
                BlipColor { r: 1.0, g: 0.16, b: 0.12, a: 0.5 });
            blip.fill_circle(eye.0, eye.1, 2.0 + 2.0 * charge, BlipColor { r: 1.0, g: 0.3, b: 0.2, a: 1.0 });
        }
    }

    // The epic punch splits the air: three lines fanning out ahead of the
    // fist while it travels and lands.
    if f.act == Act::Attack && f.epic_punch() {
        let m = move_data(f.mv);
        if f.t > m.startup * F * 0.6 && f.t < (m.startup + m.active + 3.0) * F {
            for up in [-0.7f32, 0.0, 0.7] {
                let (dx, dy) = (f.facing * up.cos(), up.sin());
                let at = |d: f32| V(k.hand_lead.0 + dx * d, k.hand_lead.1 + dy * d);
                stroke(blip, at(13.0), at(30.0), 2.0, 0.6, BLIP_WHITE);
            }
        }
    }
}

/// The nunchaku in the near hand: two sticks on a short chain. Through the startup they whirl round in a
/// figure of eight, the free stick swinging wide behind the held one; then they crack out level, in line past
/// the fist to the end of the special's reach (`NUNCHAKU`); in the recovery the free stick drops and swings.
fn draw_sticks(blip: &Blip, f: &Fighter, hand: V, fwd: f32) {
    let m = f.scaled(move_data(MoveId::Special));
    let z = f.size();
    let (stick, chain) = (13.0 * z, 4.0 * z);
    let start = m.startup * F;
    let end = start + m.active * F;
    // The held stick's angle (screen radians), and the free one's, trailing it.
    let tucked = f.act == Act::Victory && f.t >= 0.9;
    let trick = if f.act == Act::Attack { stick_trick(f) } else { None };
    let whirling = matches!(f.act, Act::Victory | Act::Bow | Act::Idle) || f.t < start;
    let (held, free) = if tucked {
        // Caught under the arm: the held stick laid back along the chest toward the armpit, the other hanging.
        let back = if fwd > 0.0 { std::f32::consts::PI } else { 0.0 };
        (back - fwd * 0.35, std::f32::consts::FRAC_PI_2)
    } else if let Some((_, _, up)) = trick {
        // The routine's trick: the held stick at its angle, the free one trailing it round in a whip.
        let held = (-up.sin()).atan2(fwd * up.cos());
        (held, held - fwd * 0.5)
    } else if whirling {
        let a = f.t * 26.0;
        (a, a - 1.4 - 0.8 * (2.0 * a).sin())
    } else if f.t <= end {
        let level = if fwd > 0.0 { 0.0 } else { std::f32::consts::PI };
        (level, level)
    } else {
        let k = ((f.t - end) / (m.recovery * F)).clamp(0.0, 1.0);
        let down = std::f32::consts::FRAC_PI_2;
        let level = if fwd > 0.0 { 0.0 } else { std::f32::consts::PI };
        (level + (down - level) * k, down + 0.6 * (f.t * 14.0).sin() * (1.0 - k))
    };
    let tip = |from: V, a: f32, len: f32| V(from.0 + a.cos() * len, from.1 + a.sin() * len);
    // Through the routine a pale swoosh follows the sticks' ends over the last few frames.
    if trick.is_some() {
        let screen = |up: f32| (-up.sin()).atan2(fwd * up.cos());
        let mut last = tip(hand, held, 2.0 * stick + chain);
        for j in 1..=6 {
            let mut was = *f;
            was.t -= j as f32 * F * 0.5;
            let Some((_, _, up)) = stick_trick(&was) else { break };
            let at = tip(hand, screen(up), 2.0 * stick + chain);
            stroke(blip, last, at, 2.4 * z, 2.4 * z, BlipColor { r: 1.0, g: 0.96, b: 0.84, a: 0.55 * (1.0 - j as f32 / 7.0) });
            last = at;
        }
    }
    let a_end = tip(hand, held, stick);
    let b_start = tip(a_end, (held + free) / 2.0, chain);
    let b_end = tip(b_start, free, stick);
    let wood = BlipColor { r: 0.34, g: 0.20, b: 0.11, a: 1.0 };
    let steel = BlipColor { r: 0.70, g: 0.72, b: 0.76, a: 1.0 };
    stroke(blip, a_end, b_start, 0.7 * z, 0.7 * z, steel);
    shape(blip, &[(hand, a_end, 2.0 * z, 2.0 * z), (b_start, b_end, 2.0 * z, 2.0 * z)], wood, INK);
    // A ring of brass at each chain end.
    for at in [a_end, b_start] { blip.fill_circle(at.0, at.1, 1.2 * z, BlipColor { r: 0.90, g: 0.72, b: 0.30, a: 1.0 }); }
}

/// What each bolt-thrower throws: every one the same hitbox, each its own
/// picture.
pub(crate) fn draw_bolt(blip: &Blip, who: usize, at: V, dir: f32, now: f32, from: V) {
    let a = &FIGHTERS[who];
    let spin = now * 14.0 * dir;
    let arm = |ang: f32, r: f32| V(at.0 + ang.cos() * r, at.1 + ang.sin() * r);
    if a.special == Special::DriveBy {
        let hot = BlipColor { r: 1.0, g: 0.66, b: 0.22, a: 1.0 };
        blip.fill_glow_circle(at.0, at.1, 8.0, BlipColor { a: 0.52, ..hot });
        stroke(blip, V(at.0 - dir * 27.0, at.1), at, 5.0, 3.0, hot);
        blip.fill_rect(at.0 - 3.5, at.1 - 1.5, 7.0, 3.0, BLIP_WHITE);
        return;
    }
    match a.special_name {
        // A slice-less pizza, spinning flat.
        "PIZZA TOSS" => {
            blip.fill_circle(at.0, at.1, 9.0, INK);
            blip.fill_circle(at.0, at.1, 7.5, BlipColor { r: 0.86, g: 0.62, b: 0.30, a: 1.0 });
            blip.fill_circle(at.0, at.1, 5.8, BlipColor { r: 0.98, g: 0.84, b: 0.36, a: 1.0 });
            for j in 0..3 {
                let v = arm(spin + j as f32 * 2.094, 3.2);
                blip.fill_circle(v.0, v.1, 1.5, BlipColor { r: 0.82, g: 0.18, b: 0.14, a: 1.0 });
            }
        }
        // A four-pointed star.
        "SHURIKEN" => {
            for (r, c) in [(10.0, INK), (8.0, BlipColor { r: 0.80, g: 0.84, b: 0.90, a: 1.0 })] {
                for j in 0..2 {
                    let ang = spin + j as f32 * std::f32::consts::FRAC_PI_2;
                    stroke(blip, arm(ang, r), arm(ang + std::f32::consts::PI, r), r * 0.14, r * 0.14, c);
                }
                blip.fill_circle(at.0, at.1, r * 0.4, c);
            }
            blip.fill_circle(at.0, at.1, 1.4, INK);
        }
        "LASER VISION" => {
            let red = BlipColor { r: 1.0, g: 0.16, b: 0.12, a: 1.0 };
            stroke(blip, from, at, 2.6, 3.4, red);
            stroke(blip, from, at, 1.0, 1.4, BLIP_WHITE);
            blip.fill_circle(at.0, at.1, 5.0, red);
            blip.fill_circle(at.0, at.1, 2.2, BLIP_WHITE);
        }
        // A ball of web on the end of its line.
        "WEB SHOT" => {
            stroke(blip, V(at.0 - dir * 30.0, at.1), at, 0.8, 1.6, BLIP_WHITE);
            blip.fill_circle(at.0, at.1, 8.0, INK);
            blip.fill_circle(at.0, at.1, 6.2, BLIP_WHITE);
        }
        // A ball of fire with its tail licking back behind it: flat colour
        // inside one outline, hottest at the front.
        _ => {
            let hot = rgb(a.trim);
            let tail = |k: f32| {
                let lick = (now * 34.0 + k * 2.1).sin() * 2.2;
                (V(at.0 - dir * (7.0 + 7.0 * k), at.1 + lick * k * 0.5), 6.6 - 1.8 * k)
            };
            for pass in 0..2 {
                let (grow, col) = if pass == 0 { (LINE, INK) } else { (0.0, hot) };
                for k in [3.0f32, 2.0, 1.0] {
                    let (v, r) = tail(k);
                    blip.fill_circle(v.0, v.1, r + grow, col);
                }
                blip.fill_circle(at.0, at.1, 8.5 + grow, col);
            }
            blip.fill_circle(at.0 + dir * 1.0, at.1, 5.6, blend(hot, BLIP_YELLOW, 0.7));
            blip.fill_circle(at.0 + dir * 2.0, at.1, 2.8, BLIP_WHITE);
        }
    }
}

/// One of the eight bonus fruits, sitting on the boards at `x`.
pub(crate) fn draw_fruit(blip: &Blip, kind: usize, x: f32, ground: f32) {
    let y = ground - 9.0;
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let (red, green, gold) = (c(0.88, 0.14, 0.16), c(0.30, 0.70, 0.28), c(0.98, 0.84, 0.22));
    let ball = |dx: f32, dy: f32, r: f32, col: BlipColor| {
        blip.fill_circle(x + dx, y + dy, r + LINE, INK);
        blip.fill_circle(x + dx, y + dy, r, col);
    };
    let stalk = |x0: f32, y0: f32, x1: f32, y1: f32, col: BlipColor| {
        stroke(blip, V(x + x0, y + y0), V(x + x1, y + y1), 1.2, 1.0, col);
    };
    match kind {
        // Cherries: two on one stalk.
        0 => {
            stalk(-5.0, 0.0, 2.0, -12.0, green);
            stalk(5.0, 2.0, 2.0, -12.0, green);
            ball(-5.0, 2.0, 4.5, red);
            ball(5.0, 3.5, 4.5, red);
        }
        // Strawberry: seeded, with a leaf cap.
        1 => {
            ball(0.0, 0.0, 7.0, red);
            for (dx, dy) in [(-3.0f32, -1.0f32), (2.5, 1.0), (-0.5, 3.5), (3.0, -3.0)] {
                blip.fill_circle(x + dx, y + dy, 0.9, BLIP_WHITE);
            }
            stalk(-4.0, -7.0, 4.0, -7.0, green);
        }
        // Orange.
        2 => {
            ball(0.0, 0.0, 7.0, c(0.98, 0.58, 0.14));
            stalk(0.0, -7.0, 4.0, -10.0, green);
        }
        // Apple.
        3 => {
            ball(0.0, 0.0, 7.0, red);
            stalk(0.0, -6.0, 1.0, -11.0, c(0.45, 0.28, 0.14));
            blip.fill_circle(x - 2.5, y - 2.5, 1.6, BLIP_WHITE);
        }
        // Melon: netted green.
        4 => {
            ball(0.0, 0.0, 7.5, green);
            for dx in [-3.5f32, 0.0, 3.5] { stalk(dx, -6.0, dx, 6.0, c(0.60, 0.88, 0.50)); }
            stalk(0.0, -7.0, 0.0, -11.0, c(0.45, 0.28, 0.14));
        }
        // The flagship of another fleet: a yellow dart with red wings.
        5 => {
            ball(0.0, 0.0, 4.0, gold);
            stroke(blip, V(x - 8.0, y + 4.0), V(x - 2.0, y - 2.0), 2.4, 1.6, red);
            stroke(blip, V(x + 8.0, y + 4.0), V(x + 2.0, y - 2.0), 2.4, 1.6, red);
            stalk(0.0, -4.0, 0.0, -10.0, c(0.30, 0.45, 0.95));
        }
        // Bell.
        6 => {
            ball(0.0, -1.0, 6.5, gold);
            blip.fill_rect(x - 8.0, y + 3.0, 16.0, 3.0, gold);
            blip.fill_circle(x, y + 7.0, 1.8, c(0.75, 0.80, 0.90));
        }
        // Key.
        _ => {
            ball(0.0, -4.0, 4.5, c(0.45, 0.75, 0.95));
            blip.fill_rect(x - 1.5, y, 3.0, 9.0, c(0.80, 0.84, 0.90));
            blip.fill_rect(x + 1.5, y + 4.0, 3.0, 2.0, c(0.80, 0.84, 0.90));
        }
    }
}

/// Everything on the stage during a round: dust, fruit, fighters, helpers,
/// bolts, splashes and words.
pub(crate) fn draw_fight(blip: &Blip, g: &Game) {
    // DRAKE gathering a special: for its startup the stage dims around him and a warm glow comes up behind,
    // the moment a whole arcade looks at one fighter.
    for f in g.p.iter().filter(|f| formless(f.who) && f.act == Act::Attack && f.mv == MoveId::Special) {
        let start = f.scaled(move_data(MoveId::Special)).startup * F;
        if f.t >= start + 4.0 * F { continue; }
        let k = (f.t / (start * 0.4)).min(1.0) * (1.0 - ((f.t - start) / (4.0 * F)).clamp(0.0, 1.0));
        // Over the scenery the camera shows (a camera-sized margin either side).
        blip.fill_rect(-200.0, -200.0, WIN_W as f32 + 400.0, FLOOR_Y + 200.0, BlipColor { r: 0.02, g: 0.01, b: 0.04, a: 0.62 * k });
        let c = V(f.x, f.y - f.height() * 0.55);
        blip.fill_glow_circle(c.0, c.1, f.height() * 0.55, BlipColor { r: 1.0, g: 0.62, b: 0.12, a: 0.42 * k });
    }
    // The one-inch punch landing: a white ring bursting from the fist, gone in a quarter of a second.
    for f in g.p.iter().filter(|f| formless(f.who) && f.act == Act::Attack && f.mv == MoveId::Special
        && f.special() == Special::OneInch && f.hit_clean) {
        let m = f.scaled(move_data(MoveId::Special));
        let age = (f.t - m.startup * F) / 0.25;
        if !(0.0..1.0).contains(&age) { continue; }
        let at = V(f.x + f.facing * (BODY_W / 2.0 + m.reach) * 0.9, f.y - m.height);
        let r = 8.0 + 34.0 * age;
        for k in 0..20 {
            let a = k as f32 / 20.0 * std::f32::consts::TAU;
            blip.fill_circle(at.0 + a.cos() * r, at.1 + a.sin() * r * 0.8, 2.6 * (1.0 - age) + 0.6,
                BlipColor { r: 1.0, g: 1.0, b: 0.92, a: 0.9 * (1.0 - age) });
        }
    }
    // The floor after a rage jump: dust thrown up along the whole of it,
    // rolling out from where he landed.
    if g.quake_t > 0.0 {
        let k = 1.0 - g.quake_t / QUAKE_SECS;
        for j in 0..16 {
            let x = (j as f32 + 0.5) * WIN_W as f32 / 16.0;
            // Each puff starts as the wave reaches it.
            let age = k - (x - g.quake_x).abs() / WIN_W as f32 * 0.5;
            if age <= 0.0 { continue; }
            blip.fill_circle(x, FLOOR_Y - 2.0 - age * 22.0, 4.0 + age * 12.0,
                BlipColor { r: 0.70, g: 0.62, b: 0.52, a: 0.5 * (1.0 - age).max(0.0) });
        }
    }
    // The fruit lies on the boards behind the fighters, and blinks before
    // it goes.
    if g.fruit.ttl > 0.0 && (g.fruit.ttl > 2.0 || (g.now * 8.0) as i32 % 2 == 0) {
        let shake = g.shake_px();
        draw_fruit(blip, g.fruit.kind, g.fruit.x, FLOOR_Y + shake);
    }
    for driveby in g.drivebys.iter().filter(|driveby| driveby.active) {
        draw_lowrider(blip, driveby, g.shake_px());
    }
    for i in 0..2 { draw_fighter(blip, g, i); }
    let shake = g.shake_px();
    for (k, h) in g.helpers.iter().enumerate() {
        if let Some(h) = h.filter(|h| h.wait <= 0.0) { draw_body(blip, &h.f, g.now, shake, 0.0, 2 + k, true); }
    }
    for f in g.p.iter().filter(|f| f.webbed > 0.0) { draw_web(blip, f, shake, g.now); }
    // On the ice their breath shows: a puff from each every couple of
    // seconds, rising and thinning.
    if g.stage == 5 {
        for (i, f) in g.p.iter().enumerate() {
            let age = (g.now * 0.42 + i as f32 * 0.5).fract() * 2.4;
            if age > 1.0 || f.act == Act::Knockdown { continue; }
            let at = V(f.x + f.facing * (16.0 + 10.0 * age) * f.size(), f.y + shake - f.height() * 0.84 - 8.0 * age);
            // Rimmed in a cooler tone, or white on snow is nothing at all.
            for (grow, c) in [(1.2, BlipColor { r: 0.62, g: 0.74, b: 0.86, a: 1.0 }), (0.0, BLIP_WHITE)] {
                for (dx, r) in [(0.0f32, 3.4f32), (4.0, 2.6), (-3.0, 2.2)] {
                    blip.fill_circle(at.0 + dx * (0.5 + age), at.1 - dx.abs() * 0.4, r * (1.0 - 0.6 * age) + grow, c);
                }
            }
        }
    }

    for b in g.bolts.iter() {
        if !b.active { continue; }
        // While he is still staring, the laser runs unbroken from his eyes;
        // after that it is a pulse trailing the way it came.
        let f = &g.p[b.owner];
        let speed = b.vx.hypot(b.vy).max(1.0);
        let tail = V(b.x - b.vx / speed * 54.0, b.y - b.vy / speed * 54.0);
        let from = if f.arch().special == Special::DriveBy {
            tail
        } else if f.act == Act::Attack && f.mv == MoveId::Special {
            V(f.x + f.facing * 14.0 * f.size(), f.y - 104.0 * f.size())
        } else {
            tail
        };
        draw_bolt(blip, f.who, V(b.x, b.y), b.vx.signum(), g.now, from);
    }

    for s in g.hitspark.iter().filter(|s| s.ttl > 0.0) { draw_splash(blip, s); }
}

/// The words over the fight, drawn on the canvas itself (pixel text at whole sizes) where the camera shows
/// what they are about.
pub(crate) fn draw_words(blip: &Blip, g: &Game) {
    // Words for what just happened, popping up over it and rising.
    // (Not over the banner that calls the round.)
    for w in g.pops.iter().filter(|w| w.ttl > 0.0 && g.state == State::Fight) {
        let age = POP_SECS - w.ttl;
        let sz = if age < 0.08 { 3.0 } else { 2.0 };
        let (wx, wy) = g.view.screen(w.x, w.y);
        let (x, y) = (wx - text_w(w.text, sz) / 2.0, wy - age * 22.0);
        let x = x.clamp(4.0, WIN_W as f32 - text_w(w.text, sz) - 4.0);
        let a = (w.ttl * 4.0).min(1.0);
        for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
            blip.draw_text(w.text, x + dx * sz, y + dy * sz, sz, BlipColor { a, ..INK });
        }
        blip.draw_text(w.text, x, y, sz, BlipColor { a, ..BLIP_WHITE });
    }

    // The combo count, over the shoulder of whoever earned it.
    if g.combo_t > 0.0 && g.combo_shown > 1 {
        let f = g.p[g.combo_side];
        let text = format!("{} HITS", g.combo_shown);
        let rise = (1.1 - g.combo_t) * 26.0;
        if g.combo_t > 0.25 || (g.now * 20.0) as i32 % 2 == 0 {
            let (x, y) = g.view.screen(f.x, f.y - STAND_H * f.size());
            stamp_at(blip, &text, x, y - 26.0 - rise, 2.0,
                BlipColor { r: 1.0, g: 0.85, b: 0.25, a: 1.0 });
        }
    }
}

fn draw_lowrider(blip: &Blip, driveby: &DriveBy, shake: f32) {
    let face = driveby.facing;
    let x = driveby_car_x_at(driveby.start_x, face, driveby.t);
    let y = FLOOR_Y + shake - 35.0 + driveby_bounce(driveby.t);
    let roll = if driveby.t < DRIVEBY_ARRIVE { driveby.t * 16.0 }
        else if driveby.t < DRIVEBY_DEPART { DRIVEBY_ARRIVE * 16.0 }
        else { DRIVEBY_ARRIVE * 16.0 + (driveby.t - DRIVEBY_DEPART) * 16.0 };
    let body = BlipColor { r: 0.30, g: 0.07, b: 0.12, a: 1.0 };
    let dark = BlipColor { r: 0.07, g: 0.08, b: 0.12, a: 1.0 };
    let glass = BlipColor { r: 0.42, g: 0.68, b: 0.70, a: 1.0 };
    let chrome = BlipColor { r: 0.78, g: 0.77, b: 0.68, a: 1.0 };
    blip.fill_rect(x - 64.0, y - 17.0, 128.0, 24.0, body);
    blip.fill_rect(x - 35.0, y - 34.0, 70.0, 19.0, body);
    blip.fill_rect(x - 31.0, y - 31.0, 28.0, 13.0, glass);
    blip.fill_rect(x + 3.0, y - 31.0, 31.0, 13.0, glass);
    // Three riders have distinct faces, bandanas and shoulders behind glass.
    for (j, sx) in [-22.0f32, 11.0, 27.0].into_iter().enumerate() {
        let skin = if j == 1 { BlipColor { r: 0.56, g: 0.36, b: 0.25, a: 1.0 } }
            else { BlipColor { r: 0.39, g: 0.24, b: 0.18, a: 1.0 } };
        let shirt = [
            BlipColor { r: 0.45, g: 0.14, b: 0.18, a: 1.0 },
            BlipColor { r: 0.56, g: 0.40, b: 0.21, a: 1.0 },
            BlipColor { r: 0.14, g: 0.25, b: 0.36, a: 1.0 },
        ][j];
        blip.fill_rect(x + sx - 5.0, y - 23.0, 10.0, 8.0, dark);
        blip.fill_rect(x + sx - 4.0, y - 22.0, 8.0, 6.0, shirt);
        blip.draw_line_ex(x + sx - 3.0, y - 20.0, x + sx + 3.0, y - 18.0, 1.0, chrome);
        blip.fill_circle(x + sx, y - 26.0, 3.6, skin);
        blip.fill_rect(x + sx - 4.0, y - 30.0, 8.0, 2.0, dark);
        blip.fill_rect(x + sx - 3.5, y - 26.0, 7.0, 1.6,
            if j == 1 { chrome } else { BlipColor { r: 0.82, g: 0.12, b: 0.18, a: 1.0 } });
        blip.fill_rect(x + sx + face * 1.0, y - 26.8, 1.0, 1.0, BLIP_WHITE);
        if j > 0 {
            stroke(blip, V(x + sx, y - 22.0), V(x + sx + face * 9.0, y - 21.0), 2.6, 1.8, dark);
            blip.draw_line_ex(x + sx + face * 7.0, y - 22.0,
                x + sx + face * 14.0, y - 22.0, 1.0, chrome);
        }
    }
    // Glass rails, door seams and a gold side pinstripe keep the car legible in motion.
    for (x1, x2) in [(-33.0f32, -1.0f32), (1.0, 36.0)] {
        blip.draw_line_ex(x + x1, y - 32.0, x + x2, y - 32.0, 1.0, chrome);
        blip.draw_line_ex(x + x1, y - 17.0, x + x2, y - 17.0, 1.0, chrome);
    }
    blip.draw_line_ex(x - 5.0, y - 15.0, x - 5.0, y + 1.0, 1.0, dark);
    blip.draw_line_ex(x + 34.0, y - 15.0, x + 34.0, y + 1.0, 1.0, dark);
    blip.draw_line_ex(x - 55.0, y - 4.0, x + 51.0, y - 4.0, 2.0,
        BlipColor { r: 0.95, g: 0.70, b: 0.28, a: 1.0 });
    for wx in [-38.0f32, 39.0] {
        let wheel_x = x + wx;
        let wheel_y = y + 5.0;
        blip.fill_circle(wheel_x, wheel_y, 12.0, dark);
        blip.fill_circle(wheel_x, wheel_y, 6.0, chrome);
        blip.fill_circle(wheel_x, wheel_y, 2.0, BlipColor { r: 0.75, g: 0.16, b: 0.18, a: 1.0 });
        for spoke in 0..4 {
            let a = roll + spoke as f32 * std::f32::consts::FRAC_PI_2;
            stroke(blip, V(wheel_x, wheel_y), V(wheel_x + a.cos() * 4.4, wheel_y + a.sin() * 4.4),
                1.0, 1.0, dark);
        }
    }
    blip.fill_rect(x - face * 59.0, y - 12.0, 5.0, 5.0,
        BlipColor { r: 1.0, g: 0.24, b: 0.12, a: 1.0 });
    blip.fill_glow_circle(x + face * 59.0, y - 10.0, 9.0,
        BlipColor { r: 1.0, g: 0.78, b: 0.35, a: 0.50 });
    blip.fill_rect(x + face * 57.0, y - 12.0, 6.0, 4.0,
        BlipColor { r: 1.0, g: 0.78, b: 0.35, a: 1.0 });
    blip.draw_line_ex(x + face * 54.0, y - 10.0, x + face * 54.0, y - 3.0, 3.0, dark);
    for grille_y in [-9.0f32, -7.0, -5.0] {
        blip.draw_line_ex(x + face * 54.0, y + grille_y, x + face * 60.0, y + grille_y, 1.0, chrome);
    }
    blip.draw_line_ex(x + face * 53.0, y - 3.0, x + face * 66.0, y - 3.0, 3.0, chrome);
    for at in DRIVEBY_SHOT_TIMES {
        if driveby.t >= at && driveby.t < at + 0.12 {
            let flash = V(x + face * 39.0, y - 23.0);
            blip.fill_glow_circle(flash.0, flash.1, 14.0, BlipColor { r: 1.0, g: 0.68, b: 0.25, a: 0.9 });
            blip.fill_rect(flash.0, flash.1 - 2.0, face * 18.0, 4.0, BLIP_WHITE);
        }
    }
}

/// A fighter wrapped in a web: rings and spokes round the whole body, tied
/// down to the boards. It thins with the first blow and flickers before it
/// lets go.
pub(crate) fn draw_web(blip: &Blip, f: &Fighter, shift: f32, now: f32) {
    if f.webbed < 1.0 && (now * 10.0) as i32 % 2 == 0 { return; }
    let h = f.height();
    let c = V(f.x, f.y + shift - h * 0.5);
    let (rx, ry) = (f.width() * 0.5 + 16.0, h * 0.5 + 9.0);
    let silk = BlipColor { r: 0.94, g: 0.96, b: 1.0, a: 1.0 };
    let line = |a: V, b: V| {
        stroke(blip, a, b, 1.9, 1.9, BlipColor { a: 0.5, ..INK });
        stroke(blip, a, b, 1.0, 1.0, silk);
    };
    let at = |ang: f32, k: f32| V(c.0 + ang.cos() * rx * k, c.1 + ang.sin() * ry * k);
    let torn = f.web_hits > 0;
    let spokes = 10;
    let step = std::f32::consts::TAU / spokes as f32;
    for j in 0..spokes {
        if torn && j % 3 == 1 { continue; }
        let ang = 0.3 + j as f32 * step;
        line(at(ang, 0.18), at(ang, 1.0));
        // Each ring sags between its spokes, as silk does.
        for (ring, k) in [0.45f32, 0.74, 1.0].into_iter().enumerate() {
            if torn && ring == 1 { continue; }
            let mid = at(ang + step / 2.0, k * 0.90);
            line(at(ang, k), mid);
            line(mid, at(ang + step, k));
        }
    }
    // Tied to the floor either side.
    let floor = FLOOR_Y + shift;
    for side in [-1.0f32, 1.0] {
        line(V(c.0 + side * rx * 0.8, c.1 + ry * 0.55), V(c.0 + side * (rx + 14.0), floor));
    }
}

/// Where a blow landed: a star of flat colour that snaps open and thins away.
/// Hot for damage, a cold four-point glint for a guard: whether the exchange
/// cost you reads from the shape as well as the colour.
pub(crate) fn draw_splash(blip: &Blip, s: &Spark) {
    let age = 1.0 - (s.ttl / s.life).clamp(0.0, 1.0);
    let big = s.life > 0.2;
    // Open in the first third, then hold the size and lose the weight.
    let open = (age / 0.3).min(1.0);
    let thin = 1.0 - ((age - 0.45) / 0.55).clamp(0.0, 1.0);
    let reach = if big { 34.0 } else { 22.0 } * (0.45 + 0.55 * open);
    let (rim, fill) = if s.blocked {
        (BlipColor { r: 0.20, g: 0.50, b: 1.0, a: 1.0 }, BlipColor { r: 0.78, g: 0.93, b: 1.0, a: 1.0 })
    } else {
        (BlipColor { r: 1.0, g: 0.48, b: 0.10, a: 1.0 }, BlipColor { r: 1.0, g: 0.93, b: 0.35, a: 1.0 })
    };
    let points = if s.blocked { 4 } else { 8 };
    let turn = s.x * 0.05;
    let c = V(s.x, s.y);
    let spike = |j: usize, len: f32, w: f32, col: BlipColor| {
        let ang = turn + j as f32 * std::f32::consts::TAU / points as f32;
        // Every other point is short, so it is a star and not a wheel.
        let len = len * if j % 2 == 1 && !s.blocked { 0.62 } else { 1.0 };
        stroke(blip, c, V(c.0 + ang.cos() * len, c.1 + ang.sin() * len), w, 0.6, col);
    };
    let w = reach * 0.30 * thin + 0.8;
    for j in 0..points { spike(j, reach + LINE, w + LINE, INK); }
    for j in 0..points { spike(j, reach, w, rim); }
    for j in 0..points { spike(j, reach * 0.66, w * 0.62, fill); }
    blip.fill_circle(c.0, c.1, w * 0.55, BLIP_WHITE);
    // Chips thrown clear of it, between the points.
    for j in 0..points {
        let ang = turn + (j as f32 + 0.5) * std::f32::consts::TAU / points as f32;
        let (dx, dy) = (ang.cos(), ang.sin());
        let (near, far) = (reach * (0.9 + 0.5 * age), reach * (1.05 + 0.75 * age));
        stroke(blip, V(c.0 + dx * near, c.1 + dy * near), V(c.0 + dx * far, c.1 + dy * far),
            1.6 * thin + 0.3, 0.3, fill);
    }
}

/// Lettering over the fight: inked all round and dropped onto its own
/// shadow, because yellow on a sunset is not text, it is a smudge.
pub(crate) fn stamp(blip: &Blip, text: &str, y: f32, sz: f32, c: BlipColor) {
    stamp_at(blip, text, WIN_W as f32 / 2.0, y, sz, c);
}

/// The same, centred on `cx`.
pub(crate) fn stamp_at(blip: &Blip, text: &str, cx: f32, y: f32, sz: f32, c: BlipColor) {
    let d = (sz * 0.5).max(1.0);
    blip.draw_text(text, cx - text_w(text, sz) / 2.0 + d, y + d * 2.5, sz, BlipColor { a: 0.55 * c.a, ..INK });
    blip.draw_centered_outlined(text, cx, y, sz, c, INK);
}
