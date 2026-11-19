//! The ways to fight a round, shared by the CPU and the playtest bot: which a fighter can use, and the
//! distances each is played at. One is picked at the bell; a fighter well behind may try another.

use super::*;

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub(crate) enum Plan {
    /// Walk in and attack.
    #[default]
    Pressure,
    /// Wait just outside the foe's reach and poke whoever steps into it.
    Spacing,
    /// Throw the special from across the stage.
    Zoning,
    /// Jump in from just out of reach and kick on the way down.
    JumpIn,
}

impl Plan {
    pub(crate) fn name(self) -> &'static str {
        match self { Plan::Pressure => "pressure", Plan::Spacing => "spacing", Plan::Zoning => "zoning", Plan::JumpIn => "jump_in" }
    }
    /// Zoning needs a special that is thrown (the laser has rules of its own).
    pub(crate) fn open_to(f: &Fighter) -> &'static [Plan] {
        if f.arch().special == Special::ChiBolt && !f.flies() {
            &[Plan::Pressure, Plan::Spacing, Plan::Zoning, Plan::JumpIn]
        } else {
            &[Plan::Pressure, Plan::Spacing, Plan::JumpIn]
        }
    }
    pub(crate) fn pick(f: &Fighter) -> Plan {
        let open = Plan::open_to(f);
        open[rand_int(0, open.len() as i32 - 1) as usize]
    }
    /// The same, from `salt` instead of the shared random numbers: a round starting must not draw on them, or
    /// every test that starts one shifts the numbers under a simulation running beside it.
    pub(crate) fn pick_by(f: &Fighter, salt: u32) -> Plan {
        let open = Plan::open_to(f);
        let h = salt.wrapping_mul(0x9E37_79B9).rotate_left(13).wrapping_mul(0x85EB_CA6B);
        open[(h >> 16) as usize % open.len()]
    }
}

/// Behind by a third of a health bar or more (as shares of each bar).
pub(crate) fn well_behind(me: &Fighter, foe: &Fighter) -> bool {
    me.health as f32 / me.arch().health as f32 + 0.3 < foe.health as f32 / foe.arch().health as f32
}

/// Where spacing waits: just outside the foe's high kick.
pub(crate) fn spacing_edge(foe: &Fighter) -> f32 { attack_range(foe, MoveId::HighKick) + 12.0 }

/// The low blow that reaches furthest, the sweep or the low kick, and how far.
pub(crate) fn longest_poke(me: &Fighter) -> (MoveId, f32) {
    let (sweep, kick) = (attack_range(me, MoveId::Sweep), attack_range(me, MoveId::LowKick));
    if sweep > kick { (MoveId::Sweep, sweep) } else { (MoveId::LowKick, kick) }
}

/// How far a jump carries: AIR_DRIFT for the whole time in the air. A jump-in starts about this far out;
/// from the high kick's reach it sailed over and kicked air (the bot lost fight 1 on seed 2 by it).
pub(crate) fn jump_travel(f: &Fighter) -> f32 {
    AIR_DRIFT * 2.0 * -JUMP_VY * f.arch().jump_scale / GRAVITY
}

/// Spacing needs room behind: from this close to a side it fights its way out instead.
pub(crate) const CORNER: f32 = WALL_MARGIN + 30.0;
pub(crate) fn room_behind(me: &Fighter) -> bool { (CORNER..WIN_W as f32 - CORNER).contains(&me.x) }
