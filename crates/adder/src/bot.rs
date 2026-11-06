//! How to play Adder: each viper the bot steers asks the CPU's own brain
//! (`cpu::steer_cpu`) where to go — away from trails, toward an unspent gap
//! — and turns with the keys a person would hold.

use super::*;
use blip::bot::{Goal::*, Pad, Playthrough};

/// What the bot must show it can do in Adder.
pub const PLAYTHROUGHS: &[Playthrough] = &[
    Playthrough {
        name: "the viper outlasts the CPU worms and wins a round",
        seconds: 180.0,
        goals: &[AtLeast("rounds_won", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it slips through a gap in a trail",
        seconds: 180.0,
        goals: &[AtLeast("through", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it lives through the fast rounds",
        level: FAST_ROUND as u32,
        seconds: 180.0,
        goals: &[AtLeast("level", FAST_ROUND as f64 + 2.0)],
        ..Playthrough::DEFAULT
    },
];

/// Fire through the title.
pub fn title() {
    Pad::new().press_start().send();
}

/// Turn each person's viper the way the CPU brain would.
pub fn drive(w: &World) {
    let mut pad = Pad::new();
    for (i, s) in w.snakes.iter().enumerate().filter(|(_, s)| s.human && s.alive) {
        let [left, right, _] = seat_keys(s.seat);
        pad.steer_with(0.0, cpu::steer_cpu(w, i).turn, 0.15, left, right);
    }
    pad.send();
}
