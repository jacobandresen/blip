//! The CPU opponent: what it does about what it sees.

use super::*;

// ---- the CPU -------------------------------------------------------------

#[test]
pub(crate) fn the_cpu_plays_through_the_same_input_struct_as_the_player() {
    // The CPU is not allowed a private API into the simulation: if it
    // could set its own state directly, "the CPU cheats" would stop
    // being a thing anyone could check.
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    for plan in [CpuPlan::Approach, CpuPlan::Retreat, CpuPlan::Block, CpuPlan::Jump,
                 CpuPlan::Attack(MoveId::Sweep), CpuPlan::Attack(MoveId::Special)] {
        g.cpu_plan = plan;
        let inp = cpu_input(&g);
        let touched = inp.left || inp.right || inp.up || inp.down
            || inp.any_punch() || inp.any_kick() || inp.special;
        assert!(touched, "{plan:?} produced no input at all");
    }
}

#[test]
pub(crate) fn the_cpu_holds_away_when_it_blocks_and_toward_when_it_approaches() {
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    // CPU is player 2, on the right, facing left.
    assert!(g.p[1].facing < 0.0);

    g.cpu_plan = CpuPlan::Block;
    assert!(cpu_input(&g).right, "blocking CPU held toward the player");
    g.cpu_plan = CpuPlan::Approach;
    assert!(cpu_input(&g).left, "approaching CPU walked away");
}

#[test]
pub(crate) fn the_cpu_blocks_low_against_a_sweep() {
    // The one read it must get right, because a CPU that never crouches
    // teaches the player that sweeping is always correct.
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    g.p[0].start_attack(MoveId::Sweep);
    g.cpu_plan = CpuPlan::Block;
    assert!(cpu_input(&g).down, "the CPU stood up into a sweep it was blocking");
}
