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
mod additions;
mod diagnostics;
mod sound;

#[allow(unused_imports)]
pub(crate) use rules::*;
#[allow(unused_imports)]
pub(crate) use roster::*;
#[allow(unused_imports)]
pub(crate) use ladder::*;
#[allow(unused_imports)]
pub(crate) use cpu::*;
#[allow(unused_imports)]
pub(crate) use specials::*;
#[allow(unused_imports)]
pub(crate) use fight::*;
#[allow(unused_imports)]
pub(crate) use feel::*;
#[allow(unused_imports)]
pub(crate) use balance::*;
#[allow(unused_imports)]
pub(crate) use anatomy::*;
#[allow(unused_imports)]
pub(crate) use versus::*;
#[allow(unused_imports)]
pub(crate) use additions::*;
#[allow(unused_imports)]
pub(crate) use diagnostics::*;
#[allow(unused_imports)]
pub(crate) use sound::*;

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
