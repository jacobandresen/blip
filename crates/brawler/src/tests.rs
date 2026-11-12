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
pub(crate) use balance::*;
pub(crate) use anatomy::*;

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
