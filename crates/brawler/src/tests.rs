//! The rules, pinned: blocking is a choice with a wrong answer, slow moves
//! are punishable, a knockdown is not a free second hit.

use super::*;

mod rules;
mod roster;
mod ladder;
mod cpu;
mod specials;
mod fight;
mod feel;
mod balance;
mod anatomy;
mod versus;
mod sound;

pub(crate) use ladder::*;
pub(crate) use anatomy::*;
/// Every test that runs the CPU takes this first.
pub(crate) use blip::testing::seeded as simulating;

/// The CPU's input this frame on a plain think timer (the game's `cpu_turn` also rethinks when jolted).
pub(crate) fn timed_cpu(g: &mut Game) -> Input {
    g.cpu_delay -= F;
    if g.cpu_delay <= 0.0 {
        g.cpu_plan = cpu_think(g);
        g.cpu_delay = 0.38 - 0.18 * g.difficulty;
    }
    cpu_input(g)
}

/// One frame of both fighters moving on `ins`: each faces the other, steps, and stays inside the walls and apart.
pub(crate) fn move_both(g: &mut Game, ins: [Input; 2]) {
    for i in 0..2 {
        let other = g.p[1 - i].x;
        if g.p[i].free() && !g.p[i].airborne() {
            g.p[i].facing = if other >= g.p[i].x { 1.0 } else { -1.0 };
        }
        let close = face_off(&mut g.p)[i];
        apply_input(&mut g.p[i], ins[i], close, F);
        advance(&mut g.p[i], F);
        g.p[i].x = clamp(g.p[i].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
    }
    separate(&mut g.p);
}

pub(crate) fn at(who: usize, x: f32, facing: f32) -> Fighter {
    Fighter::new(who, x, facing)
}

/// Run a fighter forward to the first frame of an attack's active window.
pub(crate) fn wind_to_active(f: &mut Fighter, id: MoveId) {
    f.start_attack(id);
    let m = f.scaled(move_data(id));
    f.t = m.startup * F + F * 0.5;
}

pub(crate) fn back_input(facing: f32, crouch: bool) -> Input {
    let mut i = Input::default();
    if facing > 0.0 { i.left = true; } else { i.right = true; }
    i.down = crouch;
    i
}
