//! The drawn body: bone lengths, joints, which limb is where, the walk, and what a pose may never do.

use super::*;

// ---- anatomy -------------------------------------------------------------
// The fighters are drawn from a skeleton, and a skeleton can be checked:
// every pose of every fighter across its animation, asserting that bones keep
// their length, hinges neither over-fold nor bend backwards, and nothing
// sinks through the floor. A failure names the joint and the frame.

/// Every pose the game can put a fighter in, as (label, fighter state).
pub(crate) fn every_pose() -> Vec<(String, Fighter)> {
    let mut out = Vec::new();
    let moves = [MoveId::LowPunch, MoveId::LowKick, MoveId::HighKick,
                 MoveId::HighPunch, MoveId::Sweep, MoveId::Uppercut, MoveId::JumpPunch, MoveId::JumpKick,
                 MoveId::FlyingKick, MoveId::Special, MoveId::Throw];
    for who in 0..FIGHTERS.len() {
        for act in [Act::Idle, Act::Walk, Act::Crouch, Act::Block, Act::Hitstun,
                    Act::Knockdown, Act::Victory, Act::Defeat] {
            for step in 0..12 {
                let mut f = Fighter::new(who, 300.0, 1.0);
                f.act = act;
                f.t = match act {
                    Act::Knockdown => step as f32 / 11.0 * 1.15,
                    Act::Hitstun => step as f32 / 11.0 * 0.35,
                    _ => step as f32 / 11.0,
                };
                if act == Act::Walk { f.x = 300.0 + step as f32 * 7.0; }
                out.push((format!("{} {:?} t={:.2}", FIGHTERS[who].name, act, f.t), f));
                if act == Act::Block {
                    let mut c = f;
                    c.crouch_block = true;
                    out.push((format!("{} crouch-block t={:.2}", FIGHTERS[who].name, c.t), c));
                }
                // The knees giving under a landing is drawn on top of
                // whatever the fighter does next, so it has to satisfy
                // the anatomy rules in every one of those actions too.
                if matches!(act, Act::Idle | Act::Walk | Act::Crouch | Act::Block) {
                    let mut l = f;
                    l.land = LAND_ABSORB * (1.0 - step as f32 / 11.0);
                    l.land_force = 1.0;
                    out.push((format!("{} {:?} landing land={:.3}",
                        FIGHTERS[who].name, act, l.land), l));
                }
            }
        }
        // The whole jump, take-off to touchdown. The airborne pose is
        // read off vertical speed rather than off a clock, so the way
        // to cover it is to fly the arc: rising hard, apex, falling.
        for step in 0..12 {
            let k = step as f32 / 11.0;
            let mut f = Fighter::new(who, 300.0, 1.0);
            f.act = Act::Air;
            f.vy = JUMP_VY * (1.0 - 2.0 * k);
            f.y = FLOOR_Y - 120.0 * (1.0 - (1.0 - 2.0 * k).powi(2)).max(0.02);
            f.t = k * 0.7;
            out.push((format!("{} Air vy={:.0}", FIGHTERS[who].name, f.vy), f));
        }
        for mv in moves {
            let air = matches!(mv, MoveId::JumpPunch | MoveId::JumpKick | MoveId::FlyingKick);
            for step in 0..16 {
                let mut f = Fighter::new(who, 300.0, 1.0);
                f.act = Act::Attack;
                f.mv = mv;
                if air { f.y = FLOOR_Y - 50.0; }
                let m = f.scaled(move_data(mv));
                let total = (m.startup + m.active + m.recovery) * F;
                f.t = step as f32 / 15.0 * total;
                out.push((format!("{} {:?} t={:.3}", FIGHTERS[who].name, mv, f.t), f));
            }
        }
    }
    out
}

/// The skeleton as it is drawn, at the fighter's own size. (`bones_of` is the
/// same pose at full size, which is what the anatomy rules are written in.)
pub(crate) fn drawn(f: &Fighter) -> draw::Skeleton {
    let rig = draw::Rig::upright(f.x, f.y, f.facing, f.facing, f.size());
    draw::skeleton(rig, &draw::pose_of(0.37, f, 0))
}

pub(crate) fn bones_of(f: &Fighter) -> draw::Skeleton {
    let rig = draw::Rig::upright(f.x, f.y, f.facing, f.facing, 1.0);
    draw::skeleton(rig, &draw::pose_of(0.37, f, 0))
}

#[test]
pub(crate) fn bones_never_change_length() {
    let mut bad = vec![];
    for (label, f) in every_pose() {
        let k = bones_of(&f);
        for (name, a, b, want) in k.bones() {
            let got = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            // A tenth of a pixel of float slop; anything more is the
            // solver having been asked for something impossible and
            // having said yes.
            if (got - want).abs() > 0.1 {
                bad.push(format!("{label}: {name} is {got:.1} long, should be {want:.1}"));
            }
        }
    }
    assert!(bad.is_empty(), "bones changed length:\n  {}", bad.join("\n  "));
}

#[test]
pub(crate) fn hinges_stay_inside_their_range() {
    let mut bad = vec![];
    for (label, f) in every_pose() {
        let k = bones_of(&f);
        for (name, j, a, b, shut) in k.hinges() {
            let (ax, ay) = (a.0 - j.0, a.1 - j.1);
            let (bx, by) = (b.0 - j.0, b.1 - j.1);
            let la = (ax * ax + ay * ay).sqrt().max(0.001);
            let lb = (bx * bx + by * by).sqrt().max(0.001);
            let ang = ((ax * bx + ay * by) / (la * lb)).clamp(-1.0, 1.0).acos();
            // Straight is PI. Anything under `shut` is a joint folded
            // past the point flesh gets in the way.
            if ang < shut - 0.02 {
                bad.push(format!("{label}: {name} folded to {:.0}deg (limit {:.0})",
                    ang.to_degrees(), shut.to_degrees()));
            }
        }
    }
    assert!(bad.is_empty(), "joints folded past their limit:\n  {}", bad.join("\n  "));
}

#[test]
pub(crate) fn nothing_is_drawn_underneath_the_floor() {
    let mut bad = vec![];
    for (label, f) in every_pose() {
        let k = bones_of(&f);
        let named = [("knee-lead", k.knee_lead), ("knee-rear", k.knee_rear),
                     ("elbow-lead", k.elbow_lead), ("elbow-rear", k.elbow_rear),
                     ("ankle-lead", k.ankle_lead), ("ankle-rear", k.ankle_rear),
                     ("hand-lead", k.hand_lead), ("hand-rear", k.hand_rear),
                     ("hip", k.hip), ("neck", k.neck), ("head", k.head)];
        for (name, j) in named {
            // Screen y grows downward, so "below the floor" is greater.
            if j.1 > FLOOR_Y + 0.5 {
                bad.push(format!("{label}: {name} is {:.1}px under the floor", j.1 - FLOOR_Y));
            }
        }
    }
    assert!(bad.is_empty(), "joints below the floor:\n  {}", bad.join("\n  "));
}

#[test]
pub(crate) fn what_you_see_is_what_can_hit_you() {
    // The limb a player sees coming is what hits them: on every active frame
    // the striking hand or foot is inside its hitbox and near its end.
    // (Stretching bones to reach broke the support leg; refusing left the
    // foot short.)
    let mut bad = vec![];
    for who in 0..FIGHTERS.len() {
        for mv in [MoveId::LowPunch, MoveId::LowKick, MoveId::HighKick,
                   MoveId::HighPunch, MoveId::Sweep, MoveId::Throw] {
            let mut f = Fighter::new(who, 300.0, 1.0);
            f.act = Act::Attack;
            f.mv = mv;
            let m = f.scaled(move_data(mv));
            // Sample the active window, where the move can actually hit.
            for step in 0..5 {
                f.t = (m.startup + m.active * step as f32 / 4.0) * F;
                let Some((hx, _, hw, _)) = f.hit_box() else { continue };
                let k = drawn(&f);
                let leg = matches!(mv, MoveId::LowKick | MoveId::HighKick
                    | MoveId::Sweep);
                // The tip is the end of the foot or the front of the fist,
                // scaled with the fighter like the drawing.
                let size = f.size();
                let bulk = FIGHTERS[who].bulk * size;
                let tip = if leg {
                    let (dx, _, _) = draw::foot_dir(k.knee_lead, k.ankle_lead, f.facing, f.y, size);
                    k.ankle_lead.0 + dx * draw::FOOT * bulk
                } else {
                    k.hand_lead.0 + draw::FIST * bulk
                };
                let far = hx + hw;
                let short = far - tip;
                if short > 14.0 * size {
                    bad.push(format!("{} {:?}: reaches {:.0}px short of its own hitbox",
                        FIGHTERS[who].name, mv, short));
                    break;
                }
                if tip > far + 6.0 * size {
                    bad.push(format!("{} {:?}: drawn {:.0}px past its own hitbox",
                        FIGHTERS[who].name, mv, tip - far));
                    break;
                }
            }
        }
    }
    assert!(bad.is_empty(), "picture and hitbox disagree:\n  {}", bad.join("\n  "));
}

#[test]
#[ignore = "diagnostic, not an assertion"]
pub(crate) fn why_is_that_matchup_quiet() {
    // Same order and the same seed as the balance test, so the rounds
    // land on the same randomness and the quiet one can be looked at.
    let _sim = simulating(0xB4A17E);
    let styles: [(&str, fn(usize, &[Fighter; 2]) -> Input); 3] =
        [("rusher", rusher), ("poker", poker), ("turtle", turtle)];
    for pick in (0..FIGHTERS.len()).filter(|&i| !FIGHTERS[i].invincible) {
        for foe in [0, RUNGS - 3] {
            for (name, style) in styles {
                let r = fight_round(pick, foe, style);
                println!("{} vs foe{foe} [{name}]: {}s hp {}/{} hits {}/{}", FIGHTERS[pick].name,
                    r.seconds as i32, r.player_health, r.cpu_health, r.player_hits, r.cpu_hits);
            }
        }
    }
}

#[test]
pub(crate) fn a_standing_fighter_is_standing_on_something() {
    // Weight over the feet, or the pose reads as falling. Attacks get a wider
    // allowance (a punch is falling forward); standing, walking and guarding
    // do not.
    let mut bad = vec![];
    for (label, f) in every_pose() {
        if f.airborne() || matches!(f.act, Act::Knockdown | Act::Defeat) { continue; }
        let k = bones_of(&f);
        // Segment masses, roughly anthropometric: the trunk and head
        // are half of a person, the legs a third, the arms the rest.
        let parts = [
            (k.hip, 0.14f32), (k.neck, 0.30), (k.head, 0.08),
            (k.knee_lead, 0.09), (k.ankle_lead, 0.05),
            (k.knee_rear, 0.09), (k.ankle_rear, 0.05),
            (k.elbow_lead, 0.05), (k.hand_lead, 0.03),
            (k.elbow_rear, 0.05), (k.hand_rear, 0.03),
        ];
        let total: f32 = parts.iter().map(|p| p.1).sum();
        let com: f32 = parts.iter().map(|p| p.0 .0 * p.1).sum::<f32>() / total;
        let (lo, hi) = (k.ankle_lead.0.min(k.ankle_rear.0), k.ankle_lead.0.max(k.ankle_rear.0));
        // The foot is longer than the ankle point, and a body can lean
        // onto the ball of its foot; the slack covers both.
        let slack = if f.act == Act::Attack { 26.0 } else { 12.0 };
        if com < lo - slack || com > hi + slack {
            bad.push(format!("{label}: weight at {com:.0}, feet between {lo:.0} and {hi:.0}"));
        }
    }
    assert!(bad.is_empty(), "fighters standing off balance:\n  {}", bad.join("\n  "));
}

#[test]
pub(crate) fn knees_bend_forwards_and_elbows_bend_backwards() {
    // A hinge has a direction as well as a limit: a knee on the wrong side
    // passes the length and range tests.
    let mut bad = vec![];
    for (label, f) in every_pose() {
        // A fighter on their back has no meaningful forward, and their
        // limbs are folded under them; the rule is about standing.
        if f.act == Act::Knockdown { continue; }
        let k = bones_of(&f);
        let side = |joint: draw::V, a: draw::V, b: draw::V| {
            // Which side of the line a->b the joint falls on.
            (b.0 - a.0) * (joint.1 - a.1) - (b.1 - a.1) * (joint.0 - a.0)
        };
        // Facing right, a knee ahead of the hip-to-ankle line is a
        // negative cross product in screen coordinates.
        let want = -f.facing;
        for (name, j, a, b) in [("knee-lead", k.knee_lead, k.hip_lead, k.ankle_lead),
                                ("knee-rear", k.knee_rear, k.hip_rear, k.ankle_rear)] {
            let s = side(j, a, b) * want;
            // Dead straight is zero and fine; only a real bend the
            // wrong way counts.
            if s < -1.5 {
                bad.push(format!("{label}: {name} bends backwards"));
            }
        }
        // The other half of the name. An elbow goes to the back of the
        // arm while the hand is below the shoulder it hangs from; above
        // that the arm turns over, so a raised one is skipped.
        for (name, j, a, b) in [("elbow-lead", k.elbow_lead, k.sh_lead, k.hand_lead),
                                ("elbow-rear", k.elbow_rear, k.sh_rear, k.hand_rear)] {
            if b.1 < a.1 + 6.0 { continue; }
            if side(j, a, b) * -want < -1.5 {
                bad.push(format!("{label}: {name} bends forwards"));
            }
        }
    }
    assert!(bad.is_empty(), "joints bending the wrong way:\n  {}", bad.join("\n  "));
}

#[test]
pub(crate) fn nothing_teleports_between_one_frame_and_the_next() {
    // "The animation looks strange" is usually a missing in-between: an
    // action changes and a limb crosses half the screen in a frame. Drive a
    // fighter through a realistic run at the real frame rate; no joint may
    // move further in a frame than a body part can.
    let _sim = simulating(0x5EED05);
    let script: [(Act, MoveId, f32); 11] = [
        (Act::Idle, MoveId::LowPunch, 0.20),
        (Act::Attack, MoveId::HighKick, 0.55),
        (Act::Idle, MoveId::LowPunch, 0.10),
        (Act::Attack, MoveId::HighKick, 0.70),
        (Act::Block, MoveId::LowPunch, 0.20),
        (Act::Attack, MoveId::Sweep, 0.60),
        // A jump comes out of a neutral stance, as it must in the game.
        (Act::Idle, MoveId::LowPunch, 0.12),
        (Act::Air, MoveId::LowPunch, 0.80),
        (Act::Hitstun, MoveId::LowPunch, 0.30),
        (Act::Knockdown, MoveId::LowPunch, 1.20),
        (Act::Victory, MoveId::LowPunch, 0.80),
    ];
    let mut worst = (0.0f32, String::new());
    let mut over: Vec<String> = vec![];
    for who in 0..FIGHTERS.len() {
        let mut f = Fighter::new(who, 300.0, 1.0);
        let mut last: Option<[(&'static str, draw::V); 10]> = None;
        for (act, mv, secs) in script {
            f.act = act;
            f.mv = mv;
            f.t = 0.0;
            f.stun = secs;
            // Air is not a pose but an arc: give it the velocity a
            // jump actually starts with and let advance() fly it,
            // through the apex and down onto the landing.
            if act == Act::Air {
                f.vy = JUMP_VY * f.arch().jump_scale;
                f.y -= 0.5;
            }
            let frames = (secs / F) as i32;
            let mut held = 0.0f32;
            for _ in 0..frames {
                advance(&mut f, F);
                held += F;
                // advance() ends an action when its clock runs out, so hold
                // both or the test restarts actions itself. A jump may end by
                // landing: the touchdown is part of what is watched.
                if f.act != act && !(act == Act::Air && !f.airborne()) {
                    f.act = act;
                    f.t = held;
                    f.shown = act;
                    f.blend = 0.0;
                }
                let k = bones_of(&f);
                // Knees and elbows too: a two-bone solve has two mirror
                // answers, and near a tie the joint swaps sides while
                // the hand it belongs to does not move at all.
                let now = [("hand-lead", k.hand_lead), ("hand-rear", k.hand_rear),
                           ("ankle-lead", k.ankle_lead), ("ankle-rear", k.ankle_rear),
                           ("elbow-lead", k.elbow_lead), ("elbow-rear", k.elbow_rear),
                           ("knee-lead", k.knee_lead), ("knee-rear", k.knee_rear),
                           ("head", k.head), ("hip", k.hip)];
                if let Some(prev) = last {
                    // Being hit may snap (a guard knocked aside is the
                    // fastest thing that happens to a body); everything else
                    // must travel.
                    let limit = match act {
                        Act::Hitstun => 70.0,
                        // Thrown off their feet: the arms are flung.
                        Act::Knockdown => 45.0,
                        // The peak of a kick's whip, where the shin is
                        // travelling fastest.
                        Act::Attack => 36.0,
                        _ => 30.0,
                    };
                    for i in 0..now.len() {
                        let d = ((now[i].1 .0 - prev[i].1 .0).powi(2)
                               + (now[i].1 .1 - prev[i].1 .1).powi(2)).sqrt();
                        if d > limit {
                            over.push(format!(
                                "{} {} moved {d:.0}px in one frame during {:?} (limit {limit:.0})",
                                FIGHTERS[who].name, now[i].0, act));
                        }
                        if d > worst.0 {
                            worst = (d, format!(
                                "{} {} moved {d:.0}px in one frame during {:?} at t={:.3} blend={:.2}",
                                FIGHTERS[who].name, now[i].0, act, f.t, f.blend));
                        }
                    }
                }
                last = Some(now);
            }
        }
    }
    // A kick's foot covers about fifteen pixels a frame at speed; thirty is a
    // cut (this caught a roundhouse arriving fifty in one frame).
    assert!(over.is_empty(), "limbs teleporting:\n  {}\n(worst overall: {})",
        over.join("\n  "), worst.1);
}



#[test]
pub(crate) fn a_waiting_fighter_has_their_knees_bent() {
    // Standing, walking and guarding with bent knees: a hip a leg's length
    // from the ankles clamped both legs straight while every other rule
    // passed.
    let mut bad = vec![];
    for who in 0..FIGHTERS.len() {
        for act in [Act::Idle, Act::Walk, Act::Block] {
            let mut f = Fighter::new(who, 300.0, 1.0);
            f.act = act;
            let k = bones_of(&f);
            for (name, hip, knee, ankle) in
                [("lead", k.hip_lead, k.knee_lead, k.ankle_lead),
                 ("rear", k.hip_rear, k.knee_rear, k.ankle_rear)] {
                let (ax, ay) = (hip.0 - knee.0, hip.1 - knee.1);
                let (bx, by) = (ankle.0 - knee.0, ankle.1 - knee.1);
                let la = (ax * ax + ay * ay).sqrt().max(0.001);
                let lb = (bx * bx + by * by).sqrt().max(0.001);
                let ang = ((ax * bx + ay * by) / (la * lb)).clamp(-1.0, 1.0).acos();
                // 180 degrees is a locked leg. A guard bends the knee
                // well past twenty.
                let bend = 180.0 - ang.to_degrees();
                if bend < 20.0 {
                    bad.push(format!("{} {:?}: {name} knee bent only {bend:.0} degrees",
                        FIGHTERS[who].name, act));
                }
            }
        }
    }
    assert!(bad.is_empty(), "fighters standing to attention:\n  {}", bad.join("\n  "));
}

#[test]
pub(crate) fn no_limb_is_folded_up_to_nothing() {
    // A two-bone limb stops being posable before it stops being legal: near
    // the fold limit the joint swings along the bias, and a rear guard hand
    // at 1.07x the minimum put the elbow through the chest.
    let shut = |l1: f32, l2: f32, c: f32| (l1 * l1 + l2 * l2 - 2.0 * l1 * l2 * c.cos()).sqrt();
    let arm_min = shut(24.0, 21.0, 0.61);
    let leg_min = shut(30.0, 29.0, 0.52);
    let mut bad = vec![];
    for (label, f) in every_pose() {
        let k = bones_of(&f);
        for (name, root, end, min) in
            [("arm-lead", k.sh_lead, k.hand_lead, arm_min),
             ("arm-rear", k.sh_rear, k.hand_rear, arm_min),
             ("leg-lead", k.hip_lead, k.ankle_lead, leg_min),
             ("leg-rear", k.hip_rear, k.ankle_rear, leg_min)] {
            let d = ((end.0 - root.0).powi(2) + (end.1 - root.1).powi(2)).sqrt();
            if d < min * 1.25 {
                bad.push(format!("{label}: {name} folded to {d:.0}px, \
                    against a {min:.0}px limit ({:.2}x)", d / min));
            }
        }
    }
    assert!(bad.is_empty(), "limbs folded to the point the joint is guesswork:\n  {}",
        bad.join("\n  "));
}

#[test]
#[ignore = "diagnostic"]
pub(crate) fn dump_folds() {
    let shut = |l1: f32, l2: f32, c: f32| (l1 * l1 + l2 * l2 - 2.0 * l1 * l2 * c.cos()).sqrt();
    let amin = shut(24.0, 21.0, 0.61);
    let mut worst: std::collections::BTreeMap<String, (f32, String)> = Default::default();
    for (label, f) in every_pose() {
        let k = bones_of(&f);
        let g = f.y;
        let pose = |v: draw::V| (v.0 - f.x, g - v.1);
        for (name, root, end) in [("arm-lead", k.sh_lead, k.hand_lead),
                                  ("arm-rear", k.sh_rear, k.hand_rear),
                                  ("leg-lead", k.hip_lead, k.ankle_lead),
                                  ("leg-rear", k.hip_rear, k.ankle_rear)] {
            let d = ((end.0 - root.0).powi(2) + (end.1 - root.1).powi(2)).sqrt();
            let fam = if label.contains("Knockdown") { label.split_whitespace().skip(1).collect::<Vec<_>>().join(" ") } else { label.split_whitespace().nth(1).unwrap_or("?").to_string() };
            let key = format!("{fam} {name}");
            let e = worst.entry(key).or_insert((99.0, String::new()));
            if d / amin < e.0 {
                let (sf, su) = pose(root);
                let (hf, hu) = pose(end);
                *e = (d / amin, format!("shoulder=({sf:5.1},{su:5.1}) hand=({hf:5.1},{hu:5.1}) \
                    d={d:4.1} [{label}]"));
            }
        }
    }
    for (k, (r, s)) in worst {
        if r < 1.30 { println!("{r:.2}x {k:16} {s}"); }
    }
}

#[test]
#[ignore = "diagnostic"]
pub(crate) fn dump_wings() {
    let mut rows: Vec<(f32, String)> = vec![];
    for (label, f) in every_pose() {
        let k = bones_of(&f);
        for (n, sh, el, hd) in [("lead", k.sh_lead, k.elbow_lead, k.hand_lead),
                                ("rear", k.sh_rear, k.elbow_rear, k.hand_rear)] {
            let back = (sh.0 - el.0) * f.facing;
            let hand_back = (sh.0 - hd.0) * f.facing;
            rows.push((back - hand_back.max(0.0), format!("back={back:5.1} hand={hand_back:5.1}  {n} {label}")));
        }
    }
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    for (d, s) in rows.iter().take(18) { println!("{d:5.1}  {s}"); }
}

#[test]
pub(crate) fn no_elbow_sticks_out_behind_the_back() {
    // A side-on rig has nowhere to put the elbow of an arm folded
    // across the body, so it projects it backwards and the upper arm
    // reads as a plank bolted to the shoulder. Each arm is measured from its
    // own shoulder.
    let mut bad = vec![];
    for (label, f) in every_pose() {
        if f.act == Act::Knockdown { continue; }
        let k = bones_of(&f);
        for (n, from, el, hd, most) in [("lead", k.sh_lead, k.elbow_lead, k.hand_lead, 19.0),
                                        ("rear", k.sh_rear, k.elbow_rear, k.hand_rear, 22.0)] {
            let back = (from.0 - el.0) * f.facing;
            let hand = ((from.0 - hd.0) * f.facing).max(0.0);
            if back - hand > most {
                bad.push(format!("{label}: {n} elbow {:.0}px behind, hand only {hand:.0}px", back));
            }
        }
    }
    assert!(bad.is_empty(), "elbows winging out behind the back:\n  {}", bad.join("\n  "));
}

#[test]
pub(crate) fn the_far_hand_stays_in_front_of_the_body() {
    // The stance is open: the far arm is in plain sight at the back edge of
    // the chest, and its elbow may show behind it. Its hand may not: a fist
    // below the shoulders and behind the spine by more than the body is
    // thick reads as a hand growing out of the back. (Thrown off the feet or
    // beaten, the arms go where they go.)
    let mut bad = vec![];
    for (label, f) in every_pose() {
        if matches!(f.act, Act::Knockdown | Act::Defeat) { continue; }
        // The flier has no flying kick; his kick is the laser.
        if f.flies() && f.act == Act::Attack && f.mv == MoveId::FlyingKick { continue; }
        let k = bones_of(&f);
        if k.hand_rear.1 < k.neck.1 { continue; }
        let t = ((k.hip.1 - k.hand_rear.1) / (k.hip.1 - k.neck.1).max(1.0)).clamp(0.0, 1.0);
        let spine = k.hip.0 + (k.neck.0 - k.hip.0) * t;
        let behind = (spine - k.hand_rear.0) * f.facing;
        if behind > 9.0 {
            bad.push(format!("{label}: the far hand is {behind:.0}px behind the spine"));
        }
    }
    bad.dedup();
    assert!(bad.is_empty(), "hands through the back:\n  {}",
        bad.iter().take(40).cloned().collect::<Vec<_>>().join("\n  "));
}

#[test]
pub(crate) fn a_fighter_has_two_arms_two_legs_and_one_torso() {
    // Near and far limbs far enough apart to count, close enough not to read
    // as someone else's (two fists in one place, or a fist by the skull
    // reading as a second head).
    let mut bad = vec![];
    for (label, f) in every_pose() {
        // A body rolling up off the floor passes its own limbs across
        // each other, which is what getting up is.
        if f.act == Act::Knockdown { continue; }
        let k = bones_of(&f);
        let gap = |a: draw::V, b: draw::V| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        // One fist passing the other on its way somewhere is a punch going
        // out; two fists that stay in one place are one fist.
        let still_there = {
            let mut later = f;
            later.t += 2.0 * F;
            let k = bones_of(&later);
            gap(k.hand_lead, k.hand_rear) < 6.0
        };
        if gap(k.hand_lead, k.hand_rear) < 6.0 && still_there {
            bad.push(format!("{label}: the two fists are in the same place"));
        }
        if gap(k.ankle_lead, k.ankle_rear) < 6.0 {
            bad.push(format!("{label}: the two feet are in the same place"));
        }
        // Nothing but the head belongs on the head. A hand over the
        // face deletes the one part a player has to find.
        let head_r = draw::head_r(FIGHTERS[f.who].bulk);
        for (n, h) in [("lead", k.hand_lead), ("rear", k.hand_rear)] {
            if gap(h, k.head) < head_r + 2.0 {
                bad.push(format!("{label}: the {n} fist is drawn over the head"));
            }
        }
    }
    assert!(bad.is_empty(), "limbs that cannot be counted:\n  {}", bad.join("\n  "));
}

#[test]
pub(crate) fn a_fighter_fits_inside_their_own_hurtbox() {
    // The box the rules judge and the body the player aims at have to
    // be the same object, near enough.
    let mut bad = vec![];
    for (label, f) in every_pose() {
        if matches!(f.act, Act::Knockdown | Act::Attack) { continue; }
        let k = drawn(&f);
        let top = f.y - f.height();
        let head_r = draw::head_r(FIGHTERS[f.who].bulk) * f.size();
        if k.head.1 - head_r < top - 6.0 * f.size() {
            bad.push(format!("{label}: the head is {:.0}px above the hurtbox",
                top - (k.head.1 - head_r)));
        }
    }
    assert!(bad.is_empty(), "bodies outside their own box:\n  {}", bad.join("\n  "));
}

#[test]
pub(crate) fn a_fighter_who_covers_ground_takes_steps() {
    // Both directions, drawn by different code: retreating is the block pose,
    // which must not slide with both feet pinned.
    for (name, inp, toward) in [
        ("forward", Input { right: true, ..Default::default() }, 1.0f32),
        ("backward", back_input(1.0, false), -1.0),
    ] {
        let mut f = at(0, 300.0, 1.0);
        let start = f.x;
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        let mut lifted = false;
        for _ in 0..40 {
            apply_input(&mut f, inp, false, F);
            advance(&mut f, F);
            let k = bones_of(&f);
            // Where the near foot is relative to the body it hangs off.
            let step = k.ankle_lead.0 - f.x;
            lo = lo.min(step);
            hi = hi.max(step);
            if k.ankle_lead.1 < FLOOR_Y - 3.0 { lifted = true; }
        }
        let moved = (f.x - start) * toward;
        assert!(moved > 30.0, "{name}: covered only {moved:.0}px of ground");
        assert!(hi - lo > 8.0,
            "{name}: the near foot moved {:.0}px against the body over a \
             {moved:.0}px walk — that is a slide, not a step", hi - lo);
        assert!(lifted, "{name}: the near foot never left the floor");
    }
}

#[test]
pub(crate) fn a_fighter_blocking_on_the_spot_keeps_still() {
    // The other half of the same mechanism: the gait is driven by where
    // the fighter is, so standing and guarding must not shuffle.
    let mut f = at(0, 300.0, 1.0);
    let mut last: Option<draw::V> = None;
    for i in 0..30 {
        // Holding away while crouching blocks without retreating.
        apply_input(&mut f, back_input(1.0, true), false, F);
        advance(&mut f, F);
        let k = bones_of(&f);
        // Skip the handover out of the idle stance, which is a real
        // move and is meant to be seen.
        if i < 8 { last = Some(k.ankle_lead); continue; }
        if let Some(p) = last {
            let d = ((k.ankle_lead.0 - p.0).powi(2) + (k.ankle_lead.1 - p.1).powi(2)).sqrt();
            assert!(d < 1.0, "a fighter guarding in place shuffled {d:.1}px");
        }
        last = Some(k.ankle_lead);
    }
}

#[test]
pub(crate) fn a_flying_kick_crosses_ground_no_other_move_can() {
    let mut f = at(0, 200.0, 1.0);
    f.start_attack(MoveId::FlyingKick);
    let (start, mut peak) = (f.x, 0.0f32);
    let mut frames = 0;
    while f.airborne() && frames < 120 {
        advance(&mut f, F);
        peak = peak.max(FLOOR_Y - f.y);
        frames += 1;
    }
    let travel = f.x - start;
    assert!(travel > 110.0, "a flying kick covered only {travel:.0}px");
    // Flatter than a jump, or it is just a jump-in.
    let mut j = at(0, 200.0, 1.0);
    j.vy = JUMP_VY;
    j.y -= 0.5;
    let mut jump_peak = 0.0f32;
    for _ in 0..120 {
        advance(&mut j, F);
        jump_peak = jump_peak.max(FLOOR_Y - j.y);
    }
    assert!(peak < jump_peak * 0.75,
        "the flying kick rose {peak:.0}px against a jump's {jump_peak:.0} — that is a jump");
}

#[test]
pub(crate) fn a_flying_kick_must_be_blocked_standing() {
    let m = move_data(MoveId::FlyingKick);
    assert!(blocks(m.level, false), "a flying kick should be stopped by a standing guard");
    assert!(!blocks(m.level, true), "crouching should not stop a flying kick");
}

#[test]
pub(crate) fn a_blocked_flying_kick_is_a_free_punish() {
    // The whole cost of the move. Every other air attack is cancelled
    // by landing, which is what makes a blocked jump-in safe; this one
    // keeps its recovery, so blocking it buys a turn.
    let mut a = at(0, 240.0, 1.0);
    let mut d = at(0, 330.0, -1.0);
    a.start_attack(MoveId::FlyingKick);
    let hold = back_input(-1.0, false);
    let mut blocked = false;
    for _ in 0..90 {
        if !blocked {
            let (_, b, _) = resolve_hit(&mut a, &mut d, hold);
            blocked |= b;
        }
        advance(&mut a, F);
        advance(&mut d, F);
        if blocked && !a.airborne() { break; }
    }
    assert!(blocked, "the flying kick never reached a standing guard");
    // On the floor again, still stuck in it.
    assert!(!a.free(), "the attacker was free the moment they landed");
    let mut lag = 0;
    while !a.free() && lag < 60 { advance(&mut a, F); lag += 1; }
    assert!(lag >= 10, "only {lag} frames of landing lag — that is not a punish");
    assert!(d.free() || d.act == Act::Block,
        "the defender should be out of blockstun before the attacker recovers");
}

#[test]
#[ignore = "diagnostic"]
pub(crate) fn dump_flying_kick() {
    let mut f = at(0, 200.0, 1.0);
    f.start_attack(MoveId::FlyingKick);
    let (start, mut peak, mut frames) = (f.x, 0.0f32, 0);
    while f.airborne() && frames < 120 {
        advance(&mut f, F);
        peak = peak.max(FLOOR_Y - f.y);
        frames += 1;
    }
    println!("travel {:.0}px  airtime {frames}f  peak {peak:.0}px", f.x - start);
    let mut lag = 0;
    while !f.free() && lag < 90 { advance(&mut f, F); lag += 1; }
    println!("landing lag {lag}f");
    for mv in [MoveId::LowPunch, MoveId::LowKick, MoveId::HighKick, MoveId::Sweep, MoveId::JumpKick,
               MoveId::FlyingKick] {
        let m = move_data(mv);
        println!("{mv:?}: startup {} active {} recovery {} dmg {} reach {} level {:?}",
            m.startup, m.active, m.recovery, m.damage, m.reach, m.level);
    }
}

#[test]
pub(crate) fn the_cpu_throws_the_flying_kick_too() {
    // End to end: the plan has to survive being turned into an input
    // and read back as a move. A wrong button here is silent — the CPU
    // just never uses it — and the move becomes a player-only tool.
    let _sim = simulating(0x5EED07);
    let mut seen = 0;
    for pick in (0..FIGHTERS.len()).filter(|&i| !FIGHTERS[i].invincible) {
        for foe in [0, RUNGS - 3] {
            let mut g = Game::new();
            g.pick = pick;
            g.start_match(foe);
            g.state = State::Fight;
            g.difficulty = 1.0;
            // Out of everyone's reach, which is the gap this move is for.
            g.p[0].x = 120.0;
            g.p[1].x = 520.0;
            for _ in 0..3000 {
                if g.p[0].health <= 0 || g.p[1].health <= 0 { break; }
                g.cpu_delay -= F;
                if g.cpu_delay <= 0.0 {
                    g.cpu_plan = cpu_think(&mut g);
                    g.cpu_delay = 0.38 - 0.18 * g.difficulty;
                }
                let c_in = cpu_input(&g);
                // The player holds their ground and does nothing, so
                // the CPU keeps having to solve the same problem.
                let ins = [Input::default(), c_in];
                for i in 0..2 {
                    let other = g.p[1 - i].x;
                    if g.p[i].free() && !g.p[i].airborne() {
                        g.p[i].facing = if other >= g.p[i].x { 1.0 } else { -1.0 };
                    }
                    apply_input(&mut g.p[i], ins[i], false, F);
                    advance(&mut g.p[i], F);
                    g.p[i].x = clamp(g.p[i].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
                }
                if g.p[1].act == Act::Attack && g.p[1].mv == MoveId::FlyingKick { seen += 1; }
                // Reset the gap so the CPU faces the approach problem
                // again rather than settling into close range.
                if (g.p[1].x - g.p[0].x).abs() < 120.0 && g.p[1].free() {
                    g.p[0].x = 120.0;
                    g.p[1].x = 520.0;
                }
            }
        }
    }
    assert!(seen > 0, "the CPU never threw a flying kick in six approaches");
}

#[test]
pub(crate) fn a_kick_pressed_in_the_air_always_comes_out() {
    // Pressing attack in a jump must always produce one: the flying kick
    // within its window, and a late kick buffered rather than eaten by
    // landing.
    for k in 0..50 {
        for hold_up in [false, true] {
            let mut f = at(0, 300.0, 1.0);
            let mut live = 0;
            let mut attacked = false;
            for frame in 0..110 {
                let mut inp = Input::default();
                // Jump on frame 0; hold up only if this run says to.
                if frame == 0 || hold_up { inp.up = true; }
                if frame == k { inp.kick_high = true; }
                apply_input(&mut f, inp, false, F);
                advance(&mut f, F);
                if f.act == Act::Attack { attacked = true; }
                if f.hit_box().is_some() { live += 1; }
            }
            assert!(attacked, "kick on frame {k} (up held: {hold_up}) produced no attack");
            assert!(live > 0,
                "kick on frame {k} (up held: {hold_up}) never became live — the press \
                 was eaten");
        }
    }
}

#[test]
pub(crate) fn the_flying_kick_can_be_asked_for_by_a_human() {
    // Up and kick "together" is a range of frames, not one. Anything
    // narrower than a few frames is a move that exists only in the
    // move table.
    let mut window = 0;
    for k in 0..20 {
        let mut f = at(0, 300.0, 1.0);
        let mut got = None;
        for frame in 0..40 {
            let mut inp = Input { up: true, ..Default::default() };
            if frame == k { inp.kick_high = true; }
            apply_input(&mut f, inp, false, F);
            advance(&mut f, F);
            if f.act == Act::Attack && got.is_none() { got = Some(f.mv); }
        }
        if got == Some(MoveId::FlyingKick) { window += 1; }
    }
    assert!(window >= 5, "the flying kick is only reachable on {window} frame(s)");
}

#[test]
pub(crate) fn a_flying_kick_looks_the_same_however_it_was_asked_for() {
    // Thrown off the floor or a few frames into a jump, it has to be
    // the same move: a flat dart one time and a lob the next is two
    // moves sharing a name.
    let mut peaks = vec![];
    for k in 0..6 {
        let mut f = at(0, 200.0, 1.0);
        let mut peak = 0.0f32;
        for frame in 0..90 {
            let mut inp = Input { up: true, ..Default::default() };
            if frame == k { inp.kick_high = true; }
            apply_input(&mut f, inp, false, F);
            advance(&mut f, F);
            peak = peak.max(FLOOR_Y - f.y);
            if !f.airborne() && frame > 2 { break; }
        }
        peaks.push(peak);
    }
    let (lo, hi) = (peaks.iter().cloned().fold(f32::MAX, f32::min),
                    peaks.iter().cloned().fold(0.0f32, f32::max));
    assert!(hi - lo < 10.0, "the arc peaks between {lo:.0} and {hi:.0}px depending on \
        when it was asked for: {peaks:?}");
}

/// Frames between the defender being able to act again and the
/// attacker being able to. Positive means the attacker keeps the turn.
pub(crate) fn fly_advantage(gap: f32, guard: bool) -> i32 {
    let mut a = at(0, 200.0, 1.0);
    let mut d = at(0, 200.0 + gap, -1.0);
    a.start_attack(MoveId::FlyingKick);
    let (mut landed, mut a_free, mut d_free) = (false, -1i32, -1i32);
    for frame in 0..160 {
        let (n, b, _) = resolve_hit(&mut a, &mut d, if guard { back_input(-1.0, false) }
                                                    else { Input::default() });
        if n > 0 || b { landed = true; }
        // Hold the guard only while it is doing something: a defender
        // who never lets go never counts as free.
        if guard && !landed { apply_input(&mut d, back_input(-1.0, false), false, F); }
        advance(&mut a, F);
        advance(&mut d, F);
        if landed && a_free < 0 && a.free() { a_free = frame; }
        if landed && d_free < 0 && d.free() { d_free = frame; }
        if a_free >= 0 && d_free >= 0 { break; }
    }
    assert!(landed, "the flying kick never connected at gap {gap}");
    d_free - a_free
}

#[test]
pub(crate) fn landing_a_flying_kick_keeps_your_turn() {
    // A flying kick that hits must not leave the attacker minus: it pays
    // recovery on the ground after the defender's hitstun.
    for gap in [60.0f32, 100.0, 140.0, 175.0] {
        let adv = fly_advantage(gap, false);
        assert!(adv >= 0, "a flying kick that hit at gap {gap} left the attacker \
            {} frames behind", -adv);
    }
}

#[test]
pub(crate) fn a_blocked_flying_kick_gives_the_turn_away() {
    // The other half of the trade, and the whole risk of the move:
    // blocking it has to buy enough time to punish with something real.
    for gap in [60.0f32, 100.0, 140.0] {
        let adv = fly_advantage(gap, true);
        assert!(adv <= -9, "a blocked flying kick at gap {gap} left the attacker only \
            {} frames behind — not a punish", -adv);
    }
}

#[test]
pub(crate) fn crouching_under_a_flying_kick_does_not_work() {
    // End to end, not just the level table: an overhead has to actually
    // go through a crouching guard, because making holding down cost
    // something is the whole reason the move is an overhead.
    let mut a = at(0, 200.0, 1.0);
    let mut d = at(0, 300.0, -1.0);
    d.act = Act::Crouch;
    d.crouch_block = true;
    a.start_attack(MoveId::FlyingKick);
    let hold = back_input(-1.0, true);
    let (mut dealt, mut blocked) = (0, false);
    for _ in 0..120 {
        let (n, b, _) = resolve_hit(&mut a, &mut d, hold);
        dealt += n;
        blocked |= b;
        apply_input(&mut d, hold, false, F);
        advance(&mut a, F);
        advance(&mut d, F);
        if a.free() { break; }
    }
    assert!(!blocked, "a crouching guard stopped a flying kick");
    assert!(dealt > 0, "a flying kick passed straight over a crouching opponent");
}

#[test]
pub(crate) fn a_flying_kick_can_be_met_in_the_air() {
    // Hitting them out of it has to work, without the attacker sliding on
    // (momentum is cleared on knockdown; both guards would have to fail).
    let mut a = at(0, 200.0, 1.0);
    let mut d = at(0, 300.0, -1.0);
    a.start_attack(MoveId::FlyingKick);
    for _ in 0..6 { advance(&mut a, F); }
    assert!(a.airborne(), "the flying kick never left the ground");
    wind_to_active(&mut d, MoveId::HighKick);
    let (n, _, _) = resolve_hit(&mut d, &mut a, Input::default());
    assert!(n > 0, "an anti-air could not reach a fighter mid-flying-kick");
    assert_eq!(a.act, Act::Knockdown, "they were not taken out of the air");
    let x = a.x;
    for _ in 0..20 { advance(&mut a, F); }
    assert!((a.x - x).abs() < 1.0,
        "a knocked-down fighter slid {:.0}px on the momentum of the kick", (a.x - x).abs());
}

#[test]
pub(crate) fn what_you_see_is_what_can_hit_you_in_the_air_too() {
    // Air moves' feet against the boxes that do their damage, flown for real
    // because the pose is read off vertical speed.
    let mut bad = vec![];
    for who in 0..FIGHTERS.len() {
        for mv in [MoveId::FlyingKick, MoveId::JumpKick, MoveId::JumpPunch] {
            let mut f = at(who, 200.0, 1.0);
            if mv != MoveId::FlyingKick {
                // Get airborne the ordinary way, then throw it.
                f.vy = JUMP_VY;
                f.y -= 0.5;
                f.act = Act::Air;
                for _ in 0..8 { advance(&mut f, F); }
            }
            f.start_attack(mv);
            let leg = is_kick(&f);
            for _ in 0..60 {
                advance(&mut f, F);
                let Some((hx, hy, hw, hh)) = f.hit_box() else { continue };
                let k = drawn(&f);
                let size = f.size();
                let bulk = FIGHTERS[who].bulk * size;
                let (joint, tip) = if leg {
                    // The toes point the way the foot is drawn.
                    let (dx, _, _) = draw::foot_dir(k.knee_lead, k.ankle_lead, f.facing, f.y, size);
                    (k.ankle_lead, k.ankle_lead.0 + dx * draw::FOOT * bulk)
                } else {
                    (k.hand_lead, k.hand_lead.0 + draw::FIST * bulk)
                };
                let far = hx + hw;
                if far - tip > 15.0 * size {
                    bad.push(format!("{} {mv:?}: reaches {:.0}px short of its hitbox",
                        FIGHTERS[who].name, far - tip));
                    break;
                }
                if tip > far + 8.0 * size {
                    bad.push(format!("{} {mv:?}: drawn {:.0}px past its hitbox",
                        FIGHTERS[who].name, tip - far));
                    break;
                }
                // And the right height: a box the limb is nowhere near
                // is a box that hits things the picture never touched.
                if joint.1 < hy - 26.0 * size || joint.1 > hy + hh + 26.0 * size {
                    bad.push(format!("{} {mv:?}: the limb is {:.0}px off the height its \
                        hitbox is at", FIGHTERS[who].name, (joint.1 - (hy + hh / 2.0)).abs()));
                    break;
                }
            }
        }
    }
    assert!(bad.is_empty(), "air attacks disagreeing with their hitboxes:\n  {}",
        bad.join("\n  "));
}

#[test]
pub(crate) fn winning_a_round_in_mid_air_puts_you_back_on_the_floor() {
    // A mid-air finisher must land and play the victory pose, not hang frozen
    // for the round-end pause.
    let mut g = Game::new();
    g.start_match(0);
    g.state = State::RoundEnd;
    g.phase.start(2.2);
    g.p[0].act = Act::Victory;
    g.p[1].act = Act::Defeat;
    // Mid-flight, as they would be off a flying kick.
    g.p[0].y = FLOOR_Y - 40.0;
    g.p[0].vy = -120.0;
    let t0 = g.p[0].t;
    for _ in 0..60 { update_round_end(&mut g, F); }
    assert!(!g.p[0].airborne(),
        "the winner is {:.0}px off the floor a second after the round ended",
        FLOOR_Y - g.p[0].y);
    assert!(g.p[0].t > t0 + 0.5,
        "the victory animation never advanced: timer went {t0:.2} -> {:.2}", g.p[0].t);
}

#[test]
pub(crate) fn a_fighter_on_the_floor_is_only_as_tall_as_they_are_drawn() {
    // A downed fighter's hurtbox is flat, so a flying kick passes over them.
    let mut d = at(0, 300.0, -1.0);
    d.act = Act::Knockdown;
    d.t = 0.4;
    let (_, by, _, bh) = d.hurt_box();
    assert!(bh <= 40.0, "a fighter on their back is {bh:.0}px tall");
    // Nothing drawn on them pokes far out of it.
    let rig = draw::Rig::upright(d.x, d.y, d.facing, -d.facing, 1.0);
    let k = draw::skeleton(rig, &draw::pose_of(0.37, &d, 0));
    for (n, j) in [("head", k.head), ("hip", k.hip), ("knee", k.knee_lead),
                   ("hand", k.hand_lead), ("ankle", k.ankle_lead)] {
        assert!(j.1 > by - 12.0,
            "their {n} is drawn {:.0}px above the box that can be hit", by - j.1);
    }
    // And a flying kick aimed where a standing fighter's chest would be
    // now passes over them.
    let mut a = at(0, 220.0, 1.0);
    a.start_attack(MoveId::FlyingKick);
    let mut hits = 0;
    for _ in 0..60 {
        let (n, _, _) = resolve_hit(&mut a, &mut d, Input::default());
        if n > 0 { hits += 1; }
        advance(&mut a, F);
        d.t = 0.4; // hold them down
    }
    assert_eq!(hits, 0, "a flying kick hit a fighter lying on the floor");
}

#[test]
pub(crate) fn no_guard_height_covers_any_normal() {
    // Every normal is low or overhead, so no single stance blocks them all.
    let normals = [MoveId::LowPunch, MoveId::HighPunch, MoveId::LowKick, MoveId::HighKick,
                   MoveId::Sweep, MoveId::JumpKick, MoveId::JumpPunch, MoveId::FlyingKick];
    for crouch in [false, true] {
        assert!(normals.iter().any(|&k| !blocks(move_data(k).level, crouch)),
            "a fighter holding {} blocks every normal in the game",
            if crouch { "down-back" } else { "back" });
    }
    for k in normals {
        let m = move_data(k);
        assert!(!(blocks(m.level, false) && blocks(m.level, true)),
            "{k:?} is stopped by either guard — that is the move that was dropped");
    }
    // And at each height there is a fast, short one and a slow, long
    // one, so the height is a question and the commitment is another.
    for (fast, slow) in [(MoveId::LowPunch, MoveId::LowKick),
                         (MoveId::HighPunch, MoveId::HighKick)] {
        let (f, s) = (move_data(fast), move_data(slow));
        assert!(f.startup < s.startup, "{fast:?} should come out before {slow:?}");
        assert!(f.reach < s.reach, "{fast:?} should not out-reach {slow:?}");
        assert!(f.damage < s.damage, "{fast:?} should not out-damage {slow:?}");
        assert!(f.recovery < s.recovery, "{fast:?} should cost less to miss than {slow:?}");
    }
}

/// Seconds of audio in a WAV, read off its header.
pub(crate) fn wav_seconds(wav: &[u8]) -> f32 {
    let u32_at = |i: usize| u32::from_le_bytes([wav[i], wav[i + 1], wav[i + 2], wav[i + 3]]);
    let rate = u32_at(24) as f32;
    let bytes = u32_at(40) as f32;
    bytes / 2.0 / rate
}

#[test]
pub(crate) fn a_theme_outlasts_the_round_it_plays_under() {
    // A stage theme longer than the round, so no loop seam is heard (a
    // four-bar loop repeats six times in a round).
    for which in 0..2 {
        let secs = wav_seconds(&blip_assets::brawler::theme_wav(which));
        assert!(secs > ROUND_SECS,
            "stage theme {which} is {secs:.0}s against a {ROUND_SECS:.0}s round");
    }
    // The menu is not a round, but it is somewhere people sit.
    let secs = wav_seconds(&blip_assets::brawler::theme_wav(2));
    assert!(secs > 25.0, "the select theme is only {secs:.0}s");
}

#[test]
pub(crate) fn the_three_themes_are_actually_different() {
    // Three calls to the same synthesiser could easily be three copies.
    let w: Vec<Vec<u8>> = (0..3).map(blip_assets::brawler::theme_wav).collect();
    for i in 0..3 {
        for j in i + 1..3 {
            assert_ne!(w[i], w[j], "themes {i} and {j} are the same audio");
            let (a, b) = (wav_seconds(&w[i]), wav_seconds(&w[j]));
            assert!((a - b).abs() > 1.0,
                "themes {i} and {j} are both {a:.0}s — same tempo and length");
        }
    }
}

#[test]
pub(crate) fn building_the_themes_is_quick_enough_to_do_at_startup() {
    // They are synthesised on the device instead of shipped, which is
    // only a good trade if the player does not wait for it.
    let t = std::time::Instant::now();
    for which in 0..3 { std::hint::black_box(blip_assets::brawler::theme_wav(which)); }
    let ms = t.elapsed().as_millis();
    // Generous, because this runs in a debug build too; release is
    // about a quarter of a second for all three.
    assert!(ms < 4000, "synthesising three themes took {ms}ms");
    println!("three themes in {ms}ms");
}

#[test]
#[ignore = "diagnostic"]
pub(crate) fn dump_themes() {
    for which in 0..3 {
        let wav = blip_assets::brawler::theme_wav(which);
        std::fs::write(format!("/tmp/theme{which}.wav"), &wav).unwrap();
        println!("wrote /tmp/theme{which}.wav ({:.1}s)", wav_seconds(&wav));
    }
}

// ---- the walk and the idle bounce ---------------------------------

#[test]
pub(crate) fn a_planted_foot_stays_where_it_was_put() {
    // While a foot is planted it slides back at exactly the walk speed, so in
    // the world it stays put (out of step, the fighter moonwalked a quarter
    // of every step).
    let mut worst: f32 = 0.0;
    let mut planted = 0;
    let mut x = 0.0f32;
    let mut prev: Option<f32> = None;
    while x < 400.0 {
        x += 132.0 * F; // RYUKA's walk speed
        let ph = x * draw::STRIDE;
        let (off, lift) = draw::foot_cycle(ph, 9.0);
        if lift > 0.0 { prev = None; continue; }
        planted += 1;
        if let Some(p) = prev { worst = worst.max((x + off - p).abs()); }
        prev = Some(x + off);
    }
    assert!(planted > 60, "the foot is hardly ever down: {planted} frames");
    assert!(worst < 0.05, "a planted foot slid {worst:.2}px in one frame");
}

#[test]
pub(crate) fn a_step_is_a_step_and_not_a_hop() {
    // The swinging foot has to leave the floor and come back to it,
    // once, and travel forward twice as far as it slid back — anything
    // else and the two halves of the cycle do not meet.
    let mut up = 0;
    let mut high: f32 = 0.0;
    let mut lo: f32 = f32::MAX;
    let mut hi: f32 = f32::MIN;
    for k in 0..400 {
        let ph = k as f32 * std::f32::consts::TAU / 400.0;
        let (off, lift) = draw::foot_cycle(ph, 9.0);
        if lift > 0.01 { up += 1; }
        high = high.max(lift);
        lo = lo.min(off);
        hi = hi.max(off);
    }
    assert!((150..250).contains(&up), "the foot is off the floor {up}/400 of the time");
    assert!((high - 9.0).abs() < 0.2, "the foot lifts {high:.1}px, not the 9 it was asked for");
    assert!((hi - draw::STEP).abs() < 0.1 && (lo + draw::STEP).abs() < 0.1,
        "the step runs {lo:.1}..{hi:.1}, not -{0}..{0}", draw::STEP);
    // Continuous across the seam, or the foot teleports once a stride.
    let a = draw::foot_cycle(-0.0001, 9.0);
    let b = draw::foot_cycle(0.0001, 9.0);
    assert!((a.0 - b.0).abs() < 0.1 && (a.1 - b.1).abs() < 0.1,
        "the foot jumps at the top of the cycle: {a:?} to {b:?}");
}

#[test]
pub(crate) fn two_fighters_never_bounce_in_step() {
    // Two figures rising and falling together read as one animation
    // played twice, which is the single easiest way to make a fight
    // look cheap.
    let mut apart: f32 = 0.0;
    for k in 0..200 {
        let t = k as f32 * 0.01;
        apart = apart.max((draw::bounce_of(t, 0, 0) - draw::bounce_of(t, 0, 1)).abs());
    }
    assert!(apart > 0.5, "the two fighters bounce together: {apart:.2} apart at most");
}

// ---- chain punches and the front leg ---------------------------------------

#[test]
pub(crate) fn punches_thrown_quickly_change_hands() {
    let mut f = at(0, 300.0, 1.0);
    f.start_attack(MoveId::LowPunch);
    assert!(!f.alt_punch, "the first punch is with the near hand");
    for _ in 0..12 { advance(&mut f, F); }
    f.start_attack(MoveId::LowPunch);
    assert!(f.alt_punch, "the second, straight after, is with the other");
    for _ in 0..12 { advance(&mut f, F); }
    f.start_attack(MoveId::HighPunch);
    assert!(!f.alt_punch, "and the third comes back to the first");
    // After a pause the chain starts again from the near hand.
    for _ in 0..(CHAIN_PUNCH / F) as usize + 2 { advance(&mut f, F); }
    f.act = Act::Idle;
    f.start_attack(MoveId::LowPunch);
    assert!(!f.alt_punch);

    // The far fist goes where the near one would: out to the end of the blow.
    let reach = |alt: bool| {
        let mut f = at(0, 300.0, 1.0);
        wind_to_active(&mut f, MoveId::HighPunch);
        f.alt_punch = alt;
        let k = bones_of(&f);
        if alt { k.hand_rear.0 } else { k.hand_lead.0 }
    };
    assert!((reach(true) - reach(false)).abs() < 6.0,
        "the far fist stops at {} and the near one at {}", reach(true), reach(false));
}

#[test]
pub(crate) fn a_walk_leaves_whichever_foot_it_stopped_on_in_front_and_the_kick_comes_off_it() {
    // Over a stride each foot takes its turn in front.
    let mut seen = [false; 2];
    for step in 0..60 {
        let mut f = at(0, 200.0 + step as f32, 1.0);
        f.act = Act::Walk;
        advance(&mut f, F);
        seen[f.far_leads as usize] = true;
        // Standing, the front foot is the one the flag names.
        f.act = Act::Idle;
        f.blend = 0.0;
        let k = bones_of(&f);
        assert_eq!(k.ankle_rear.0 > k.ankle_lead.0, f.far_leads, "at x {}", f.x);
    }
    assert!(seen[0] && seen[1], "a walk never changed the front foot");

    // The kick is thrown with the front leg, whichever it is.
    for far in [false, true] {
        let mut f = at(0, 300.0, 1.0);
        f.far_leads = far;
        wind_to_active(&mut f, MoveId::HighKick);
        let k = bones_of(&f);
        let (kicking, standing) = if far { (k.ankle_rear, k.ankle_lead) } else { (k.ankle_lead, k.ankle_rear) };
        assert!(kicking.1 < standing.1 - 40.0 && kicking.0 > standing.0 + 30.0,
            "far_leads {far}: the kicking foot is at {kicking:?}, the standing one at {standing:?}");
    }
}

#[test]
pub(crate) fn a_round_won_quickly_and_unhurt_is_worth_the_most() {
    assert_eq!(round_bonus(30.9, 100, 100), [3000, 3000]);
    assert_eq!(round_bonus(0.0, 1, 100), [0, 0]);
    assert_eq!(round_bonus(12.0, 50, 100), [1200, 1500]);
    // A bigger fighter is not paid more for the same share of health.
    assert_eq!(round_bonus(0.0, 65, 130)[1], round_bonus(0.0, 50, 100)[1]);
}
