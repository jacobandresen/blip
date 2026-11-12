//! `BLIP_SHOT_SCENE=n` selects a screenshot scene; `BLIP_SCREENSHOT_OUT` captures it and exits.
//!
//! | n | scene |
//! |---|---|
//! | 0 | a high kick landing, RYUKA against BRUTUS on the docks |
//! | 1 | the laser, aimed from the air |
//! | 2 | a rage jump coming down |
//! | 3 | the title screen |
//! | 4, 5, 6 | the air base, the bath house, the river village |
//! | 7 | the billing before the fourth fight |
//! | 8 | a knockout |
//! | 9 | a turtle calling the others |
//! | 10 | a fighter in a web |
//! | 11, 12 | the crystal fortress, the rooftop |
//! | 13 | a kick off the far leg |
//! | 14 | the second punch of a chain |
//! | 15 | the card's poster |
//! | 16 | the ending |
//! | 17, 18 | a bolt and a pizza in flight |
//! | 19 | the bonus round |
//! | 20 | a dizzy fighter |
//! | 21 | the second bonus round |
//! | 22 | a turtle's shell ram |

use super::*;

/// Set this frame's scene up. Called every frame of a screenshot run with
/// the count of frames so far.
pub(crate) fn pose(g: &mut Game, shot_frame: &mut u32) {
    let scene = std::env::var("BLIP_SHOT_SCENE").ok().and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    if scene == 3 {
        // The title is where the game already is.
    } else if scene == 15 {
        g.poster = true;
    } else if scene == 19 || scene == 21 {
        // The bonus round, a barrel already gone and one cracked.
        if g.state != State::Bonus {
            g.pick = 0;
            g.start_bonus();
            g.p[0].x = 250.0;
            g.opponent_index = if scene == 21 { BONUS_AFTER[1] } else { BONUS_AFTER[0] };
            g.start_bonus();
            g.p[0].x = 250.0;
            g.barrels[0] = Barrel { hp: 0, broke_t: 0.25, wait: 0.0, ..g.barrels[0] };
            if scene == 21 { g.barrels[1].wait = 0.0; g.barrels[2] = Barrel { wait: 0.0, y: FLOOR_Y, vx: 90.0, ..g.barrels[2] }; }
            g.barrels[1].hp = 1;
            g.clock = 13.2;
        }
    } else if scene == 16 {
        g.pick = 0;
        g.sess.score = 48200;
        g.state = State::Won;
    } else if scene == 7 {
        if g.state != State::Vs {
            g.pick = 0;
            g.start_match(3);
            g.announce();
        }
    } else if scene > 0 && !matches!(scene, 8 | 13 | 14) {
        *shot_frame += 1;
        let (a, b, stage) = match scene {
            1 => (9, 8, 1),
            2 => (8, 4, 0),
            4 => (0, 2, 2),
            5 => (1, 3, 3),
            9 => (3, 0, 3),
            11 => (0, 9, 5),
            17 => (0, 1, 0),
            20 => (0, 1, 0),
            18 => (6, 0, 3),
            22 => (4, 0, 3),
            12 => (0, 7, 6),
            10 => (7, 0, 2),
            _ => (0, 8, 4),
        };
        if *shot_frame == 1 {
            g.pick = a;
            g.start_match(0);
            g.stage = stage;
            g.state = State::Fight;
            g.p = [Fighter::new(a, 210.0, 1.0), Fighter::new(b, 440.0, -1.0)];
            g.ghost = [g.p[0].health as f32, g.p[1].health as f32];
            g.clock = ROUND_SECS * 0.6;
            g.fruit_due = false;
            if scene == 1 {
                g.p[0].y = FLOOR_Y - 70.0;
                g.p[0].act = Act::Air;
            }
        }
        g.cpu_plan = CpuPlan::Wait;
        g.cpu_delay = 99.0;
        if scene == 20 {
            g.p[1].act = Act::Hitstun;
            g.p[1].stun = 1.0;
            g.p[1].dizzy = 1.0;
            g.p[1].x = 330.0;
        }
        if *shot_frame == 4 && matches!(scene, 1 | 9 | 10 | 17 | 18 | 22) {
            g.p[0].called = matches!(scene, 18 | 22);
            g.p[0].start_attack(MoveId::Special);
        }
        if *shot_frame == 4 && scene == 2 {
            g.p[0].y = FLOOR_Y - 130.0;
            g.p[0].act = Act::Air;
            g.p[0].start_attack(MoveId::JumpKick);
        }
    } else {
        *shot_frame += 1;
        if *shot_frame == 1 {
            g.pick = 0;
            g.start_match(0);
            g.state = State::Fight;
            // The two the game began with, on the docks.
            g.p[1] = Fighter::new(1, 370.0, -1.0);
            g.stage = home_of(1);
            g.p[0].x = 286.0;
            g.p[1].health = (FIGHTERS[g.p[1].who].health as f32 * 0.55) as i32;
            // Scene 8: the same kick is the last of the round.
            if scene == 8 { g.p[1].health = 1; }
            g.p[0].health = (FIGHTERS[g.p[0].who].health as f32 * 0.8) as i32;
            g.clock = ROUND_SECS * 0.72;
            g.fruit_due = false;
        }
        // Hold the opponent still: left alone the CPU ducks and the high
        // kick sails over it.
        g.cpu_plan = CpuPlan::Wait;
        g.cpu_delay = 99.0;
        // Captured at BLIP_SCREENSHOT_FRAME=30: the kick has landed, the
        // hitstop flash (near-white in a still) has passed, and the spark
        // is still up.
        if *shot_frame == 4 {
            // Scene 13 kicks off the far leg; 14 is the second punch of a chain.
            g.p[0].far_leads = scene == 13;
            g.p[0].since_punch = 0.1;
            g.p[0].start_attack(if scene == 14 { MoveId::HighPunch } else { MoveId::HighKick });
        }
        if scene == 14 && *shot_frame > 14 {
            let m = move_data(MoveId::HighPunch);
            g.p[0].act = Act::Attack;
            g.p[0].t = (m.startup + m.active * 0.5) * F;
        }
        // Frames run on the wall clock: hold the splash open so the still
        // has it whichever frame it lands on.
        for s in g.hitspark.iter_mut().filter(|s| s.ttl > 0.0) { s.ttl = s.ttl.max(s.life * 0.55); }
        // And, short of a knockout, the kick at full stretch.
        if matches!(scene, 0 | 13) && g.p[0].hit_done && g.p[0].mv == MoveId::HighKick {
            let m = move_data(MoveId::HighKick);
            g.p[0].act = Act::Attack;
            g.p[0].t = (m.startup + m.active * 0.5) * F;
            g.hitstop = 0.0;
        }
    }
}
