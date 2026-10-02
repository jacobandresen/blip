//! Diagnostics, not assertions: tables printed for a person to read (`--ignored --nocapture`).

use super::*;

// ---- playtest diagnostics (run with --ignored) ---------------------------

#[test]
#[ignore = "diagnostic, not an assertion"]
pub(crate) fn diag_round_1_how_long_before_a_press_becomes_a_hit() {
    // Ease of use is mostly this number: press the button, how many
    // frames until the fighter is actually threatening.
    for mv in [MoveId::LowPunch, MoveId::HighPunch, MoveId::LowKick,
               MoveId::HighKick, MoveId::Sweep, MoveId::FlyingKick] {
        let d = move_data(mv);
        println!("{mv:?}: startup {} active {} recovery {}", d.startup, d.active, d.recovery);
    }
    // And how often a press during recovery survives to become a move.
    let mut kept = 0;
    let mut tried = 0;
    for press_on in 0..40 {
        let mut f = at(0, 200.0, 1.0);
        f.act = Act::Attack;
        f.mv = MoveId::HighKick;
        f.t = 0.0;
        let mut saw_punch = false;
        for k in 0..70 {
            let mut inp = Input::default();
            if k == press_on { inp.punch_low = true; }
            apply_input(&mut f, inp, false, F);
            advance(&mut f, F);
            if f.act == Act::Attack && f.mv == MoveId::LowPunch { saw_punch = true; }
        }
        tried += 1;
        if saw_punch { kept += 1; }
    }
    println!("presses during a high kick that still came out: {kept}/{tried}");
}

#[test]
#[ignore = "diagnostic, not an assertion"]
pub(crate) fn diag_round_2_does_the_planted_foot_skate() {
    // A walking fighter's planted foot should stay where it was put.
    // Measure it in world pixels: the foot that is down, frame to frame.
    let mut f = at(0, 120.0, 1.0);
    let mut inp = Input::default();
    inp.right = true;
    // Each foot, tracked on its own: while it is on the floor, how far
    // does it travel in world space? A planted foot should not travel.
    let mut worst = [0.0f32; 2];
    let mut slid = [0.0f32; 2];
    let mut prev: [Option<f32>; 2] = [None, None];
    let mut body = 0.0f32;
    let mut down_n = [0usize; 2];
    let x0 = f.x;
    for k in 0..240 {
        apply_input(&mut f, inp, false, F);
        advance(&mut f, F);
        let rig = draw::Rig::upright(f.x, f.y, f.facing, f.facing, 1.0);
        let s = draw::skeleton(rig, &draw::pose_of(k as f32 * F, &f, 0));
        let feet = [(s.ankle_lead.0, s.ankle_lead.1), (s.ankle_rear.0, s.ankle_rear.1)];
        for (i, (fx, fy)) in feet.iter().enumerate() {
            // On the floor: within a pixel of the ground line.
            if (*fy - f.y).abs() < 2.5 {
                down_n[i] += 1;
                if let Some(px) = prev[i] {
                    let d = (fx - px).abs();
                    worst[i] = worst[i].max(d);
                    slid[i] += d;
                }
                prev[i] = Some(*fx);
            } else {
                prev[i] = None;
            }
        }
        body = f.x - x0;
    }
    println!("body travelled {body:.0}px; planted feet slid {:.0}px / {:.0}px \
        (worst single frame {:.2} / {:.2}); frames down {:?}", slid[0], slid[1], worst[0], worst[1], down_n);
}

#[test]
#[ignore = "diagnostic, not an assertion"]
pub(crate) fn diag_round_3_how_much_of_the_round_is_spent_on_top_of_each_other() {
    let _sim = simulating(0x51C3);
    let mut g = at_versus_select();
    lock(&mut g, 0);
    lock(&mut g, 1);
    let mut close = 0usize;
    let mut total = 0usize;
    let mut idle = 0usize;
    for frame in 0..(60 * 60) {
        if g.state != State::Fight && g.state != State::RoundIntro { }
        if g.state == State::Fight {
            total += 1;
            let gap = (g.p[1].x - g.p[0].x).abs();
            if gap < 46.0 { close += 1; }
            if g.p[0].act == Act::Idle && g.p[1].act == Act::Idle { idle += 1; }
        }
        step_versus(&mut g, frame, brawlers);
        if matches!(g.state, State::Over | State::Won) { break; }
    }
    println!("frames nose to nose: {close}/{total}, both idle: {idle}/{total}");
}

#[test]
#[ignore = "diagnostic, not an assertion"]
pub(crate) fn diag_round_4_is_a_waiting_fighter_visibly_alive() {
    // Peak-to-peak travel of every joint over one breath, in pixels.
    let f = at(0, 200.0, 1.0);
    let mut lo = [f32::MAX; 6];
    let mut hi = [f32::MIN; 6];
    for k in 0..200 {
        let rig = draw::Rig::upright(f.x, f.y, f.facing, f.facing, 1.0);
        let s = draw::skeleton(rig, &draw::pose_of(k as f32 * 0.01, &f, 0));
        let vals = [s.hip.1, s.head.1, s.hand_lead.1, s.hand_rear.1, s.head.0, s.hip.0];
        for i in 0..6 { lo[i] = lo[i].min(vals[i]); hi[i] = hi[i].max(vals[i]); }
    }
    let names = ["hip y", "head y", "lead hand y", "rear hand y", "head x", "hip x"];
    for i in 0..6 { println!("{}: {:.2}px", names[i], hi[i] - lo[i]); }
}

#[test]
#[ignore = "diagnostic, not an assertion"]
pub(crate) fn diag_round_5_how_fast_does_a_fighter_answer_the_stick() {
    // Forward to backward, and the frames it takes.
    let mut f = at(0, 200.0, 1.0);
    let mut fwd = Input::default(); fwd.right = true;
    let mut back = Input::default(); back.left = true;
    for _ in 0..30 { apply_input(&mut f, fwd, false, F); advance(&mut f, F); }
    let x0 = f.x;
    let mut turned = None;
    for k in 0..30 {
        apply_input(&mut f, back, false, F);
        advance(&mut f, F);
        if f.x < x0 - 0.01 && turned.is_none() { turned = Some(k); }
    }
    println!("frames from forward to actually moving back: {turned:?}");
    println!("walk {} back_walk {}", f.arch().walk, f.arch().back_walk);
}

// ---- ten rounds of looking at it ----------------------------------------
// Diagnostics of the drawing (`--ignored`): what a player can tell apart at
// sixty pixels. The ones that found something have a real assertion further
// down.

/// Worst offenders first, with the label, so a number leads somewhere.
pub(crate) fn report(title: &str, mut rows: Vec<(f32, String)>) {
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    println!("--- {title} ({} samples)", rows.len());
    for (v, what) in rows.iter().take(8) { println!("    {v:8.2}  {what}"); }
}

#[test]
#[ignore = "diagnostic, not an assertion"]
pub(crate) fn diag_round_6_is_any_leg_curled_up_behind_the_back() {
    // A foot that is both behind the hip and high off the floor is a leg
    // folded up behind the fighter — the single thing that most makes a
    // figure read as a bundle rather than a person.
    let mut rows = Vec::new();
    for (name, f) in every_pose() {
        let k = bones_of(&f);
        for (side, ankle, knee) in [("lead", k.ankle_lead, k.knee_lead),
                                    ("rear", k.ankle_rear, k.knee_rear)] {
            let behind = (k.hip.0 - ankle.0) * f.facing;
            let up = f.y - ankle.1;
            if behind > 0.0 && up > 0.0 {
                rows.push((behind.min(up), format!("{name} {side} foot {behind:.0}px behind, {up:.0}px up")));
            }
            let kbehind = (k.hip.0 - knee.0) * f.facing;
            if kbehind > 0.0 {
                rows.push((kbehind * 0.5, format!("{name} {side} knee {kbehind:.0}px behind the hip")));
            }
        }
    }
    report("legs behind the back", rows.clone());
    // One worst case per pose name, so eight lines cover eight poses
    // instead of eight frames of one.
    let mut best: std::collections::BTreeMap<String, (f32, String)> = Default::default();
    for (v, what) in rows {
        let key = what.split(" t=").next().unwrap_or(&what).to_string();
        let e = best.entry(key).or_insert((0.0, String::new()));
        if v > e.0 { *e = (v, what); }
    }
    report("worst frame of each pose", best.into_values().collect());
}

#[test]
#[ignore = "diagnostic, not an assertion"]
pub(crate) fn diag_round_7_can_the_two_of_anything_be_told_apart() {
    // Two arms and two legs only read as two if they are drawn apart.
    let mut rows = Vec::new();
    for (name, f) in every_pose() {
        let k = bones_of(&f);
        rows.push((-dist2(k.ankle_lead, k.ankle_rear), format!("{name} feet")));
        rows.push((-dist2(k.hand_lead, k.hand_rear), format!("{name} hands")));
    }
    rows.iter_mut().for_each(|r| r.0 = -r.0);
    rows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    println!("--- limbs drawn on top of each other (closest first)");
    for (v, what) in rows.iter().take(8) { println!("    {v:8.2}  {what}"); }
}

#[test]
#[ignore = "diagnostic, not an assertion"]
pub(crate) fn diag_round_8_is_the_head_ever_swallowed() {
    // A face is four marks on a ten-pixel ball. Anything drawn over it
    // and the fighter has no face at all.
    let mut rows = Vec::new();
    for (name, f) in every_pose() {
        let k = bones_of(&f);
        let r = 11.0 * FIGHTERS[f.who].bulk;
        for (what, at) in [("chest", k.neck), ("lead hand", k.hand_lead),
                           ("rear hand", k.hand_rear), ("lead knee", k.knee_lead),
                           ("rear knee", k.knee_rear), ("hip", k.hip)] {
            let d = dist2(k.head, at);
            if d < r { rows.push((r - d, format!("{name}: {what} is {d:.0}px into the head"))); }
        }
    }
    report("things drawn over the face", rows);
}

#[test]
#[ignore = "diagnostic, not an assertion"]
pub(crate) fn diag_round_9_how_tall_is_a_fighter_really() {
    // A crouch that folds to nothing and a jump that curls into a ball
    // are the same bug: the silhouette stops being a person.
    let mut rows = Vec::new();
    for (name, f) in every_pose() {
        let k = bones_of(&f);
        let xs = [k.head.0, k.hip.0, k.ankle_lead.0, k.ankle_rear.0, k.hand_lead.0, k.hand_rear.0];
        let ys = [k.head.1, k.hip.1, k.ankle_lead.1, k.ankle_rear.1, k.hand_lead.1, k.hand_rear.1];
        let w = xs.iter().cloned().fold(f32::MIN, f32::max) - xs.iter().cloned().fold(f32::MAX, f32::min);
        let h = ys.iter().cloned().fold(f32::MIN, f32::max) - ys.iter().cloned().fold(f32::MAX, f32::min);
        rows.push((w / h.max(1.0), format!("{name}: {w:.0} wide by {h:.0} tall")));
    }
    report("widest against their own height", rows);
}

#[test]
#[ignore = "diagnostic, not an assertion"]
pub(crate) fn diag_round_10_does_the_background_compete_with_the_fight() {
    // Not geometry: how much ink the stage spends. Counted as the number
    // of separate things drawn behind the fighters, because every one of
    // them is something the eye has to rule out.
    println!("counted by hand from draw_dock / draw_temple — see the test below");
}

pub(crate) fn dist2(a: draw::V, b: draw::V) -> f32 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}

#[test]
pub(crate) fn no_leg_is_curled_up_behind_the_back() {
    // A foot far behind the hip and far above the floor is a heel pulled to
    // the backside, which balls up the figure. Each alone is real (a trailing
    // leg, a chambered knee), so the product is bounded.
    let mut bad = Vec::new();
    for (name, f) in every_pose() {
        let k = bones_of(&f);
        for (side, ankle) in [("lead", k.ankle_lead), ("rear", k.ankle_rear)] {
            let behind = (k.hip.0 - ankle.0) * f.facing;
            let up = f.y - ankle.1;
            if behind.min(up) > 24.0 {
                bad.push(format!("{name}: {side} foot is {behind:.0}px behind the hip \
                    and {up:.0}px off the floor"));
            }
        }
    }
    bad.dedup();
    assert!(bad.is_empty(), "legs folded up behind the back:\n  {}",
        bad.iter().take(6).cloned().collect::<Vec<_>>().join("\n  "));
}
