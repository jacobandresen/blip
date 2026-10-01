//! The rules, pinned: blocking is a choice with a wrong answer, slow moves
//! are punishable, a knockdown is not a free second hit.

use super::*;

fn at(who: usize, x: f32, facing: f32) -> Fighter {
    Fighter::new(who, x, facing)
}

/// Run a fighter forward to the first frame of an attack's active window.
fn wind_to_active(f: &mut Fighter, id: MoveId) {
    f.start_attack(id);
    let m = f.scaled(move_data(id));
    f.t = m.startup * F + F * 0.5;
}

fn back_input(facing: f32, crouch: bool) -> Input {
    let mut i = Input::default();
    if facing > 0.0 { i.left = true; } else { i.right = true; }
    i.down = crouch;
    i
}

// ---- blocking is a choice ------------------------------------------------

#[test]
fn a_standing_block_stops_a_mid_and_a_jump_in_but_not_a_sweep() {
    // The whole reason attacks have heights. If standing block covered
    // everything, there would be no reason to ever do anything else, and
    // no reason for the attacker to pick one move over another.
    assert!(blocks(Level::Mid, false), "a mid should be blockable standing");
    assert!(blocks(Level::Overhead, false), "a jump-in should be blockable standing");
    assert!(!blocks(Level::Low, false), "a sweep must beat a standing block");
}

#[test]
fn a_crouching_block_stops_a_sweep_but_not_a_jump_in() {
    assert!(blocks(Level::Low, true));
    assert!(blocks(Level::Mid, true));
    assert!(!blocks(Level::Overhead, true), "a jump-in must beat a crouching block");
}

#[test]
fn blocking_requires_holding_away_from_the_opponent() {
    // Holding toward them is walking into it. This is what makes a
    // player commit: you cannot advance and be safe at the same time.
    assert!(holding_back(back_input(1.0, false), 1.0));
    let mut forward = Input::default();
    forward.right = true;
    assert!(!holding_back(forward, 1.0), "holding forward is not a block");
}

#[test]
fn a_sweep_goes_through_a_standing_block_and_knocks_down() {
    let mut atk = at(0, 200.0, 1.0);
    let mut def = at(1, 240.0, -1.0);
    wind_to_active(&mut atk, MoveId::Sweep);
    let (dmg, blocked, knocked) = resolve_hit(&mut atk, &mut def, back_input(-1.0, false));
    assert!(!blocked, "a standing block stopped a low attack");
    assert!(dmg > 0);
    assert!(knocked && def.act == Act::Knockdown, "a sweep should put them on the floor");
}

#[test]
fn the_same_sweep_is_blocked_by_crouching() {
    let mut atk = at(0, 200.0, 1.0);
    let mut def = at(1, 240.0, -1.0);
    wind_to_active(&mut atk, MoveId::Sweep);
    let (dmg, blocked, _) = resolve_hit(&mut atk, &mut def, back_input(-1.0, true));
    assert!(blocked && dmg == 0, "crouch-blocking should stop a sweep");
    assert_eq!(def.health, FIGHTERS[def.who].health, "a blocked sweep should do no damage");
}

#[test]
fn a_jump_attack_beats_a_crouching_block() {
    let mut atk = at(0, 200.0, 1.0);
    atk.y = FLOOR_Y - 40.0;
    let mut def = at(1, 236.0, -1.0);
    wind_to_active(&mut atk, MoveId::JumpKick);
    let (dmg, blocked, _) = resolve_hit(&mut atk, &mut def, back_input(-1.0, true));
    assert!(!blocked && dmg > 0, "a crouch block should not cover an overhead");
}

// ---- frame data ----------------------------------------------------------

#[test]
fn an_attack_can_only_hit_during_its_active_window() {
    // Startup you can walk into; recovery you can punish. If a move
    // could hit across its whole animation, neither would exist.
    let mut f = at(0, 200.0, 1.0);
    let m = f.scaled(move_data(MoveId::HighKick));
    f.start_attack(MoveId::HighKick);

    f.t = m.startup * F * 0.5;
    assert!(f.hit_box().is_none(), "a kick hit during its startup");

    f.t = (m.startup + m.active * 0.5) * F;
    assert!(f.hit_box().is_some(), "a kick did not hit during its active frames");

    f.t = (m.startup + m.active + m.recovery * 0.5) * F;
    assert!(f.hit_box().is_none(), "a kick hit during its recovery");
}

#[test]
fn a_slow_frame_cannot_step_over_the_active_window() {
    // One long frame (a slow phone) that starts before the active window
    // and would end after it must still stop in it.
    for id in [MoveId::LowPunch, MoveId::HighKick] {
        let mut f = at(0, 200.0, 1.0);
        let m = f.scaled(move_data(id));
        f.start_attack(id);
        advance(&mut f, (m.startup + m.active) * F + 0.01);
        assert!(f.hit_box().is_some(), "{id:?}: a long frame stepped over its active window");
    }
}

#[test]
fn one_attack_lands_one_hit() {
    // Four active frames is four chances to overlap. Without the latch
    // a kick would do four times its listed damage.
    let mut atk = at(0, 200.0, 1.0);
    let mut def = at(1, 240.0, -1.0);
    wind_to_active(&mut atk, MoveId::HighKick);
    let first = resolve_hit(&mut atk, &mut def, Input::default());
    let second = resolve_hit(&mut atk, &mut def, Input::default());
    assert!(first.0 > 0);
    assert_eq!(second.0, 0, "the same attack hit twice");
}

#[test]
fn the_slow_moves_are_the_punishable_ones() {
    // The core trade of the whole game: reach and damage are paid for in
    // recovery. If a move were both long and safe there would be nothing
    // to think about.
    let jab = move_data(MoveId::LowPunch);
    // The low kick is the poke this is about: longer than a jab and
    // slower to recover from, with the sweep costing more than either.
    let kick = move_data(MoveId::LowKick);
    let sweep = move_data(MoveId::Sweep);
    assert!(kick.reach > jab.reach && kick.recovery > jab.recovery,
        "the longer poke must also be the slower one to recover from");
    assert!(sweep.knockdown && sweep.recovery > kick.recovery,
        "a knockdown must cost more on whiff than a normal poke");
    assert!(jab.startup < kick.startup && jab.damage < kick.damage,
        "the fast move must be the weak one");
}

#[test]
fn hitstun_always_exceeds_blockstun() {
    // Otherwise blocking would be worse than being hit, and the game
    // would reward standing still and eating everything.
    for id in [MoveId::LowPunch, MoveId::LowKick, MoveId::HighKick, MoveId::HighPunch,
               MoveId::JumpPunch, MoveId::JumpKick, MoveId::FlyingKick, MoveId::Special] {
        let m = move_data(id);
        assert!(m.hitstun > m.blockstun, "{id:?} punishes blocking more than getting hit");
    }
}

// ---- states --------------------------------------------------------------

#[test]
fn you_cannot_act_while_you_are_busy() {
    // Being able to cancel recovery by pressing another button would
    // delete the entire punish game.
    let mut f = at(0, 200.0, 1.0);
    f.start_attack(MoveId::Sweep);
    let before = f.mv;
    let mut jab = Input::default();
    jab.punch_low = true;
    apply_input(&mut f, jab, false, F);
    assert_eq!(f.mv, before, "an attack was cancelled into another attack");
    assert_eq!(f.act, Act::Attack);

    f.act = Act::Hitstun;
    f.stun = 10.0 * F;
    apply_input(&mut f, jab, false, F);
    assert_eq!(f.act, Act::Hitstun, "a fighter attacked out of hitstun");
}

#[test]
fn a_knockdown_ends_with_a_moment_of_invulnerability() {
    // A knockdown that could be hit again on the way up is not a
    // knockdown, it is a loop — one sweep and the round is over.
    let mut f = at(0, 200.0, 1.0);
    f.act = Act::Knockdown;
    f.t = 0.2;
    assert!(!invulnerable(&f), "a fighter lying down should still be a target early");
    f.t = 1.0;
    assert!(invulnerable(&f), "there is no safe wakeup");

    // And it ends: the state cannot be held forever.
    f.t = 1.2;
    advance(&mut f, F);
    assert_eq!(f.act, Act::Idle, "a knockdown never got up");
}

#[test]
fn an_invulnerable_wakeup_cannot_be_hit() {
    let mut atk = at(0, 200.0, 1.0);
    let mut def = at(1, 240.0, -1.0);
    def.act = Act::Knockdown;
    def.t = 1.0;
    wind_to_active(&mut atk, MoveId::HighKick);
    let (dmg, _, _) = resolve_hit(&mut atk, &mut def, Input::default());
    assert_eq!(dmg, 0, "a waking fighter was hit through their invulnerability");
}

#[test]
fn health_stops_at_zero() {
    let mut atk = at(1, 200.0, 1.0); // Brutus, the hardest hitter
    let mut def = at(2, 236.0, -1.0);
    def.health = 3;
    wind_to_active(&mut atk, MoveId::HighKick);
    resolve_hit(&mut atk, &mut def, Input::default());
    assert_eq!(def.health, 0, "health went negative and the bar would draw backwards");
}

// ---- archetypes ----------------------------------------------------------

#[test]
fn the_fighters_are_actually_different() {
    // Palette swaps would not be fighters. Somebody has to be the best at
    // each thing, and pay for it.
    // (The invincible one is outside every trade, on purpose.)
    let fair = || FIGHTERS.iter().enumerate().filter(|(_, a)| !a.invincible);
    let fastest = fair().max_by(|a, b| a.1.walk.total_cmp(&b.1.walk)).unwrap().0;
    let strongest = fair().max_by(|a, b| a.1.power.total_cmp(&b.1.power)).unwrap().0;
    let toughest = fair().max_by_key(|(_, a)| a.health).unwrap().0;
    assert_ne!(fastest, strongest, "the fastest fighter is also the strongest");
    // The heavies pay in speed for what they hit with and what they take.
    for heavy in [strongest, toughest] {
        assert!(FIGHTERS[heavy].walk < 100.0, "{} is heavy and quick", FIGHTERS[heavy].name);
    }

    // And the fast one pays for it.
    let fast = FIGHTERS[fastest];
    assert!(fast.power < 1.0 && fast.health < 100, "the fast fighter pays no price");
    assert!(fast.reach > 1.0, "the fast fighter has nothing to poke with");

    // No two fighters share a name or a full set of numbers.
    for (i, a) in FIGHTERS.iter().enumerate() {
        for b in &FIGHTERS[i + 1..] {
            assert_ne!(a.name, b.name);
            assert!(a.walk != b.walk || a.power != b.power || a.health != b.health,
                "{} and {} play the same", a.name, b.name);
        }
    }
    // Every special is somebody's.
    for sp in [Special::ChiBolt, Special::BullRush, Special::TalonKick, Special::LaserVision] {
        assert!(FIGHTERS.iter().any(|a| a.special == sp), "nobody has {sp:?}");
    }
}

#[test]
fn power_and_reach_scale_the_shared_move_table() {
    // The archetype has to reach into the numbers, or "power 1.35" is
    // decoration.
    let brutus = at(1, 0.0, 1.0);
    let kestrel = at(2, 0.0, 1.0);
    let base = move_data(MoveId::HighKick);
    assert!(brutus.scaled(base).damage > base.damage);
    assert!(kestrel.scaled(base).damage < base.damage);
    assert!(kestrel.scaled(base).reach > brutus.scaled(base).reach,
        "the lighter fighter should out-range the heavy one");
    // Size scales the whole fighter: a turtle half the height has half the
    // body to hit and little more than half the reach, the giant half as
    // much again of both.
    let (small, big) = (at(3, 0.0, 1.0), at(8, 0.0, 1.0));
    assert!(small.height() < kestrel.height() * 0.55 && big.height() > kestrel.height() * 1.45);
    assert!(small.scaled(base).reach < brutus.scaled(base).reach);
    assert!(big.scaled(base).reach > kestrel.scaled(base).reach);
}

// ---- the stage -----------------------------------------------------------

#[test]
fn nobody_leaves_the_stage() {
    let mut p = [at(0, WALL_MARGIN, 1.0), at(1, WIN_W as f32 - WALL_MARGIN, -1.0)];
    push_apart(&mut p, 40.0);
    for f in p.iter() {
        assert!(f.x >= WALL_MARGIN - 0.01 && f.x <= WIN_W as f32 - WALL_MARGIN + 0.01,
            "a fighter was pushed off the stage to {}", f.x);
    }
}

#[test]
fn a_cornered_fighter_pushes_the_attacker_back_instead() {
    // Corner pressure only exists because the pushback has to go
    // somewhere. Without this the corner is not a place, and half the
    // reason to advance disappears.
    let corner = WIN_W as f32 - WALL_MARGIN;
    let mut p = [at(0, corner - 40.0, 1.0), at(1, corner, -1.0)];
    let attacker_before = p[0].x;
    push_apart(&mut p, 12.0);
    assert!((p[1].x - corner).abs() < 0.01, "the cornered fighter slid out of the corner");
    assert!(p[0].x < attacker_before, "the attacker was not pushed back by the corner");
}

#[test]
fn fighters_cannot_stand_inside_each_other() {
    let mut p = [at(0, 300.0, 1.0), at(1, 302.0, -1.0)];
    separate(&mut p);
    assert!((p[1].x - p[0].x).abs() >= BODY_W * 0.8 - 0.01,
        "two fighters occupied the same space");
}

// ---- the ladder ----------------------------------------------------------

#[test]
fn the_ladder_is_everybody_else_once() {
    let boss = FIGHTERS.iter().position(|a| a.invincible).unwrap();
    for pick in 0..FIGHTERS.len() {
        let ladder = Game { pick, ..Game::new() }.ladder();
        assert_eq!(ladder.len(), FIGHTERS.len() - 1);
        let mut met = ladder.to_vec();
        met.sort();
        met.dedup();
        assert_eq!(met.len(), ladder.len(), "{} meets somebody twice", FIGHTERS[pick].name);
        assert!(!ladder.contains(&pick), "{} fights themself", FIGHTERS[pick].name);
        // The one nothing hurts is the last fight, never an earlier one, and
        // the giant is the one before: nobody opens against either.
        let giant = FIGHTERS.iter().position(|a| a.build == Build::Giant).unwrap();
        let ends: Vec<usize> = [giant, boss].into_iter().filter(|&w| w != pick).collect();
        assert_eq!(&ladder[RUNGS - ends.len()..], &ends[..],
            "{}'s ladder does not end with the giant and the boss", FIGHTERS[pick].name);
    }
}

#[test]
fn every_fight_is_at_the_opponents_home_and_every_stage_is_somebodys() {
    // No ladder has two fights running in one place, whoever climbs it.
    for pick in 0..FIGHTERS.len() {
        let mut g = Game { pick, ..Game::new() };
        let ladder = g.ladder();
        let mut seen = vec![];
        for (rung, foe) in ladder.into_iter().enumerate() {
            g.start_match(rung);
            assert_eq!(g.stage, home_of(foe), "{} is not fought at home", FIGHTERS[foe].name);
            assert!(g.stage < STAGES);
            seen.push(g.stage);
        }
        assert!(seen.windows(2).all(|w| w[0] != w[1]),
            "{} fights twice running in one place: {seen:?}", FIGHTERS[pick].name);
    }
    for stage in 0..STAGES {
        assert!((0..FIGHTERS.len()).any(|who| home_of(who) == stage),
            "nobody lives at {}", STAGE_NAMES[stage]);
    }
    // Two players meet at player two's.
    let mut g = Game { pick: 0, pick2: 8, ..Game::new() };
    g.start_versus();
    assert_eq!(g.stage, home_of(8));
}

#[test]
fn each_opponent_is_fought_somewhere_else() {
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    let first = g.stage;
    g.start_match(1);
    assert_ne!(first, g.stage, "both opponents were fought on the same stage");
}

#[test]
fn the_second_opponent_is_harder_than_the_first() {
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    let first = g.difficulty;
    g.start_match(1);
    assert!(g.difficulty > first, "the ladder does not climb");
}

#[test]
fn a_round_starts_both_fighters_whole_and_apart() {
    let mut g = Game::new();
    g.pick = 2;
    g.start_match(0);
    assert_eq!(g.p[0].health, FIGHTERS[g.p[0].who].health);
    assert_eq!(g.p[1].health, FIGHTERS[g.p[1].who].health);
    assert!((g.p[1].x - g.p[0].x).abs() > 150.0, "the round starts inside each other");
    assert!(g.p[0].facing > 0.0 && g.p[1].facing < 0.0, "the fighters do not face each other");
}

#[test]
fn round_wins_survive_the_round_that_earned_them() {
    // start_round() rebuilds both fighters; the score of the match is
    // the one thing that must not be rebuilt with them.
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    g.p[0].rounds = 1;
    g.start_round();
    assert_eq!(g.p[0].rounds, 1, "a round win was lost between rounds");
    assert_eq!(g.p[0].health, FIGHTERS[g.p[0].who].health, "health did not reset");
}

// ---- the CPU -------------------------------------------------------------

#[test]
fn the_cpu_plays_through_the_same_input_struct_as_the_player() {
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
fn the_cpu_holds_away_when_it_blocks_and_toward_when_it_approaches() {
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
fn the_cpu_blocks_low_against_a_sweep() {
    // The one read it must get right, because a CPU that never crouches
    // teaches the player that sweeping is always correct.
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    g.p[0].start_attack(MoveId::Sweep);
    g.cpu_plan = CpuPlan::Block;
    assert!(cpu_input(&g).down, "the CPU stood up into a sweep it was blocking");
}

// ---- specials ------------------------------------------------------------

#[test]
fn a_fireball_leaves_the_hand_pointing_the_way_the_fighter_is_facing() {
    // The one attack that keeps travelling after the animation ends, so
    // it is also the one that can be aimed backwards by a sign error and
    // look almost right while doing nothing.
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    g.p[0].facing = 1.0;
    g.spawn_bolt(0, 16);
    let bolt = g.bolts.iter().find(|b| b.active).expect("no bolt was spawned");
    assert!(bolt.vx > 0.0, "the fireball travelled backwards");
    assert!(bolt.x > g.p[0].x, "the fireball spawned behind the fighter who threw it");
    assert!(bolt.y < g.p[0].y, "the fireball spawned underground");

    // And the other way round.
    let mut g2 = Game::new();
    g2.pick = 0;
    g2.start_match(0);
    g2.p[0].facing = -1.0;
    g2.spawn_bolt(0, 16);
    let bolt = g2.bolts.iter().find(|b| b.active).unwrap();
    assert!(bolt.vx < 0.0 && bolt.x < g2.p[0].x);
}

#[test]
fn the_projectile_pool_cannot_be_overrun() {
    // A fighter holding the special down must not be able to allocate
    // forever; the pool is fixed and the overflow has to be dropped.
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    for _ in 0..50 { g.spawn_bolt(0, 16); }
    assert!(g.bolts.iter().filter(|b| b.active).count() <= g.bolts.len());
}

#[test]
fn every_special_is_something_the_shared_table_cannot_do() {
    // Each special breaks a rule the normals obey (the bolt leaves the
    // fighter, the rush crosses the stage, the talon kick goes airborne), or
    // fighters differ only in colour.
    let base = move_data(MoveId::Special);
    assert!(base.knockdown, "a special that does not knock down is just a slow kick");
    // It out-damages the pokes (it ends an exchange); the high kick
    // out-damages it and is meant to, a 14-frame overhead you must read.
    assert!(base.damage > move_data(MoveId::LowKick).damage);
    // And it costs more to miss than anything else in the table, which
    // is the only thing keeping it from being the whole game.
    for id in [MoveId::LowPunch, MoveId::LowKick, MoveId::HighKick, MoveId::HighPunch,
               MoveId::Sweep, MoveId::Throw, MoveId::FlyingKick] {
        assert!(base.recovery > move_data(id).recovery,
            "a special recovers faster than a {id:?} — it has to be punishable or it \
             is the only move anyone would throw");
    }
}

// ---- a whole fight -------------------------------------------------------

/// Run the simulation the way update_fight() does, minus the sound and
/// the scoring. Two fighters, two input streams, for as many frames as
/// asked — enough to prove the loop terminates rather than jamming.
fn spar(a_who: usize, b_who: usize, gap: f32, frames: usize,
        mut inputs: impl FnMut(usize, &[Fighter; 2]) -> [Input; 2]) -> [Fighter; 2] {
    let mid = WIN_W as f32 / 2.0;
    let mut p = [Fighter::new(a_who, mid - gap / 2.0, 1.0),
                 Fighter::new(b_who, mid + gap / 2.0, -1.0)];
    for frame in 0..frames {
        let ins = inputs(frame, &p);
        for i in 0..2 {
            let other = p[1 - i].x;
            if p[i].free() && !p[i].airborne() {
                p[i].facing = if other >= p[i].x { 1.0 } else { -1.0 };
            }
        }
        let close = face_off(&mut p);
        for i in 0..2 {
            apply_input(&mut p[i], ins[i], close[i], F);
            advance(&mut p[i], F);
            p[i].x = clamp(p[i].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
        }
        separate(&mut p);
        for atk in 0..2 {
            let def = 1 - atk;
            let (mut a, mut d) = (p[atk], p[def]);
            let (dmg, blocked, _) = resolve_hit(&mut a, &mut d, ins[def]);
            p[atk] = a;
            p[def] = d;
            if dmg > 0 || blocked { push_apart(&mut p, if blocked { 6.0 } else { 9.0 }); }
        }
    }
    p
}

#[test]
fn two_fighters_left_alone_actually_hurt_each_other() {
    let _sim = simulating(0x5EED01);
    // The loop has to make progress. A fight where nothing connects is
    // the failure mode that every individual rule above can pass while
    // the game is still unplayable.
    let mut forward_and_punch = |frame: usize, _p: &[Fighter; 2]| {
        let mut a = Input::default();
        let mut b = Input::default();
        a.right = true;
        b.left = true;
        if frame % 24 < 2 { a.punch_low = true; }
        if frame % 30 < 2 { b.kick_low = true; }
        [a, b]
    };
    let p = spar(0, 1, 240.0, 60 * 12, &mut forward_and_punch);
    assert!(p[0].health < FIGHTERS[p[0].who].health, "nobody ever landed a hit on P1");
    assert!(p[1].health < FIGHTERS[p[1].who].health, "nobody ever landed a hit on P2");
}

#[test]
fn a_fighter_who_blocks_correctly_takes_far_less_than_one_who_does_not() {
    // The single claim the whole design rests on: defence is worth
    // something. Same attacker, same attack, one defender holding back
    // and crouching, the other standing still.
    let mut sweeping = |frame: usize, _p: &[Fighter; 2]| {
        let mut a = Input::default();
        a.down = true;
        if frame % 40 < 2 { a.kick_low = true; } // sweep on a loop
        [a, Input::default()]
    };
    // Inside sweep range from the first frame: this test is about the
    // block, not about walking in.
    let open = spar(0, 1, 74.0, 60 * 10, &mut sweeping);
    let damage_taken_standing = FIGHTERS[open[1].who].health - open[1].health;

    let mut sweeping_vs_block = |frame: usize, _p: &[Fighter; 2]| {
        let mut a = Input::default();
        a.down = true;
        if frame % 40 < 2 { a.kick_low = true; }
        let mut b = Input::default();
        b.right = true; // away from P1, who is on the left
        b.down = true;  // and low, because a sweep is low
        [a, b]
    };
    let guarded = spar(0, 1, 74.0, 60 * 10, &mut sweeping_vs_block);
    let damage_taken_blocking = FIGHTERS[guarded[1].who].health - guarded[1].health;

    assert!(damage_taken_standing > 0, "the attacker never landed anything at all");
    assert!(damage_taken_blocking * 3 < damage_taken_standing,
        "blocking low against a sweep saved almost nothing: {damage_taken_blocking} vs \
         {damage_taken_standing} damage");
}

#[test]
fn nobody_gets_stuck_in_a_state_they_cannot_leave() {
    // Twelve seconds of someone mashing every button at once. Whatever
    // that produces, both fighters must still be able to act at the end
    // — a state machine with a dead end is a game that freezes.
    let mut mash = |frame: usize, _p: &[Fighter; 2]| {
        let mut a = Input::default();
        a.punch_low = frame % 3 == 0;
        a.kick_high = frame % 5 == 0;
        a.down = frame % 7 < 3;
        a.up = frame % 31 == 0;
        a.right = frame % 11 < 5;
        a.left = frame % 13 < 4;
        a.special = frame % 97 == 0;
        [a, a]
    };
    let mut p = spar(0, 2, 200.0, 60 * 12, &mut mash);
    for f in p.iter_mut() {
        // Give each one a second of no input; they should come to rest.
        for _ in 0..60 { advance(f, F); }
        assert!(f.free() || f.act == Act::Attack,
            "a fighter ended up stuck in {:?} with no way out", f.act);
        assert!(f.y <= FLOOR_Y + 0.01, "a fighter ended up below the floor");
    }
}

// ---- winning and losing --------------------------------------------------

/// Push a phase timer past its end the way the frame loop would.
fn run_phase(g: &mut Game, step: impl Fn(&mut Game, f32)) {
    for _ in 0..600 {
        let before = g.state;
        step(g, F);
        if g.state != before { return; }
    }
    panic!("a phase never ended: still in a state after 10 seconds");
}

#[test]
fn beating_the_first_opponent_moves_you_to_the_second_somewhere_else() {
    // The ladder is the whole single-player structure, and it is the one
    // part a player only reaches by being good — so it is the part least
    // likely to be exercised by hand before shipping.
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    let first_foe = g.p[1].who;
    let first_stage = g.stage;

    g.p[0].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    g.phase.start(0.1);
    run_phase(&mut g, update_match_end);

    assert_eq!(g.state, State::Vs, "the next match was not billed");
    assert_ne!(g.p[1].who, first_foe, "the same opponent came back for a second match");
    assert_ne!(g.stage, first_stage, "the second match is in the same place as the first");
    assert_eq!(g.p[0].rounds, 0, "round wins carried over into the next opponent");
    assert_eq!(g.p[1].rounds, 0);
    assert_eq!(g.p[0].health, FIGHTERS[g.p[0].who].health, "the next match started hurt");
}

#[test]
fn beating_every_opponent_wins_the_game() {
    let mut g = Game::new();
    g.pick = 1;
    g.start_match(RUNGS - 1); // the last rung
    g.p[0].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    g.phase.start(0.1);
    run_phase(&mut g, update_match_end);
    assert_eq!(g.state, State::Won, "clearing the ladder did not end the game");
}

#[test]
fn losing_a_match_ends_the_run() {
    let mut g = Game::new();
    g.pick = 2;
    g.start_match(0);
    g.p[1].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    g.phase.start(0.1);
    run_phase(&mut g, update_match_end);
    assert_eq!(g.state, State::Over, "losing did not end the run");
}

#[test]
fn a_match_cannot_run_forever() {
    // Every round ends with somebody getting a point, including a double
    // KO — which gives both fighters one, so even a match of nothing but
    // draws terminates at two rounds rather than looping.
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    for round in 0..8 {
        g.p[0].rounds += 1;
        g.p[1].rounds += 1; // the worst case: every round a draw
        g.state = State::RoundEnd;
        g.phase.start(0.05);
        run_phase(&mut g, update_round_end);
        if g.state == State::MatchEnd { return; }
        assert!(round < 4, "a drawn match kept starting new rounds");
    }
    panic!("the match never reached its end");
}

#[test]
fn a_round_win_is_worth_more_when_you_are_barely_scratched() {
    // Scoring has to reward playing well rather than merely surviving,
    // or the leaderboard measures patience.
    let mut clean = Game::new();
    clean.pick = 0;
    clean.start_match(0);
    clean.sess.add_score(0);
    let before = clean.sess.score;
    clean.p[0].health = FIGHTERS[clean.p[0].who].health;
    clean.sess.add_score(1000 + clean.p[0].health * 20);
    let clean_score = clean.sess.score - before;

    let mut scraped = Game::new();
    scraped.pick = 0;
    scraped.start_match(0);
    scraped.p[0].health = 1;
    let before2 = scraped.sess.score;
    scraped.sess.add_score(1000 + scraped.p[0].health * 20);
    let scraped_score = scraped.sess.score - before2;

    assert!(clean_score > scraped_score * 2,
        "winning untouched scores about the same as scraping through");
}

// ---- feel ----------------------------------------------------------------

#[test]
fn an_attack_pressed_during_recovery_comes_out_when_recovery_ends() {
    // The difference between "I was a frame early" and "this game
    // ignored me". A player pressing punch at the end of a sweep is
    // asking for a punch, and should get one.
    let mut f = at(0, 200.0, 1.0);
    f.start_attack(MoveId::Sweep);
    let sweep = f.scaled(move_data(MoveId::Sweep));
    let total = (sweep.startup + sweep.active + sweep.recovery) * F;

    let mut punch = Input::default();
    punch.punch_low = true;
    // Press with a few frames of recovery still to run.
    f.t = total - 4.0 * F;
    apply_input(&mut f, punch, false, F);
    assert_eq!(f.mv, MoveId::Sweep, "the press interrupted the sweep");
    assert_eq!(f.buffered, Some(MoveId::LowPunch), "the press was thrown away");

    // Let the sweep finish, then hold nothing at all.
    for _ in 0..5 { advance(&mut f, F); }
    apply_input(&mut f, Input::default(), false, F);
    assert_eq!(f.act, Act::Attack, "the buffered punch never came out");
    assert_eq!(f.mv, MoveId::LowPunch);
}

#[test]
fn a_buffered_attack_is_forgotten_if_it_waits_too_long() {
    // A buffer that never expires fires attacks the player asked for
    // seconds ago, which is its own kind of not listening.
    let mut f = at(0, 200.0, 1.0);
    f.act = Act::Hitstun;
    f.stun = 1.0;
    let mut punch = Input::default();
    punch.punch_low = true;
    apply_input(&mut f, punch, false, F);
    assert!(f.buffered.is_some());

    for _ in 0..20 { apply_input(&mut f, Input::default(), false, F); }
    assert_eq!(f.buffered, None, "the buffer held an input far past its welcome");
}

#[test]
fn a_buffered_move_remembers_the_stance_it_was_asked_for_in() {
    // Down+kick pressed in recovery is a sweep when it comes out, whatever
    // the stick does meanwhile.
    let mut f = at(0, 200.0, 1.0);
    f.start_attack(MoveId::HighKick);
    let mut low = Input::default();
    low.down = true;
    low.kick_low = true;
    apply_input(&mut f, low, false, F);
    assert_eq!(f.buffered, Some(MoveId::Sweep));
}

#[test]
fn the_heavier_the_hit_the_longer_the_world_stops() {
    // Hitstop is how weight is communicated. If every hit froze for the
    // same time, a jab and a knockdown would feel identical.
    let block = hitstop_for(0, false, true);
    let light = hitstop_for(6, false, false);
    let heavy = hitstop_for(13, false, false);
    let knock = hitstop_for(13, true, false);
    assert!(block < light, "a blocked hit should feel lighter than a landed one");
    assert!(light < heavy, "a jab should not land like a kick");
    assert!(heavy < knock, "a knockdown should be the heaviest thing in the game");
    assert!(knock < 0.2, "the freeze is long enough to be a pause rather than a hit");
}

#[test]
fn a_knockdown_throws_the_two_fighters_apart() {
    // Landing one has to be a reward. Standing over the wakeup is not a
    // reward — it is a guess against invulnerability, and on the
    // receiving end it reads as being stomped.
    let mut p = [at(0, 280.0, 1.0), at(1, 330.0, -1.0)];
    let before = (p[1].x - p[0].x).abs();
    push_apart(&mut p, 34.0);
    let after = (p[1].x - p[0].x).abs();
    assert!(after > before + 50.0,
        "a knockdown left them {after:.0}px apart, barely more than the {before:.0} they started at");
}

// ---- balance, measured ---------------------------------------------------

/// A whole round fought headlessly, the real CPU against a crude rusher,
/// turtle or poker, reported as numbers: how the game treats players who are
/// not good at it is most of whether it is fun.
struct RoundStats {
    seconds: f32,
    player_health: i32,
    cpu_health: i32,
    player_hits: i32,
    cpu_hits: i32,
    timed_out: bool,
}

/// Every test that runs the CPU takes this first: the RNG is process-global
/// and `cargo test` runs in parallel, so without the lock (and a seed inside
/// it) the balance tests pass or fail by scheduling.
static SIMULATION: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn simulating(seed: u64) -> std::sync::MutexGuard<'static, ()> {
    // A poisoned lock just means some other simulation test already
    // failed its assertion; this one can still run.
    let guard = SIMULATION.lock().unwrap_or_else(|e| e.into_inner());
    blip::rand_seed(seed);
    guard
}

fn fight_round(pick: usize, foe_index: usize, style: fn(usize, &[Fighter; 2]) -> Input) -> RoundStats {
    fight_round_seen(pick, foe_index, style, &mut |_, _, _| {})
}

/// The same, calling `seen(attacker, their fighter, damage)` for every blow
/// that lands.
fn fight_round_seen(pick: usize, foe_index: usize, style: fn(usize, &[Fighter; 2]) -> Input,
                    seen: &mut dyn FnMut(usize, &Fighter, i32)) -> RoundStats {
    let mut g = Game::new();
    g.pick = pick;
    g.start_match(foe_index);
    g.state = State::Fight;

    let mut stats = RoundStats {
        seconds: 0.0, player_health: 0, cpu_health: 0,
        player_hits: 0, cpu_hits: 0, timed_out: false,
    };

    let mut frame = 0usize;
    while g.p[0].health > 0 && g.p[1].health > 0 && stats.seconds < ROUND_SECS {
        if g.hitstop > 0.0 { g.hitstop -= F; frame += 1; continue; }
        stats.seconds += F;

        let p_in = style(frame, &g.p);
        g.cpu_delay -= F;
        if g.cpu_delay <= 0.0 {
            g.cpu_plan = cpu_think(&mut g);
            g.cpu_delay = 0.38 - 0.18 * g.difficulty;
        }
        let c_in = cpu_input(&g);
        let ins = [p_in, c_in];

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

        for a in 0..2 {
            let d = 1 - a;
            let (mut atk, mut def) = (g.p[a], g.p[d]);
            let (dmg, blocked, knock) = resolve_hit(&mut atk, &mut def, ins[d]);
            g.p[a] = atk;
            g.p[d] = def;
            if dmg > 0 {
                if a == 0 { stats.player_hits += 1; } else { stats.cpu_hits += 1; }
                seen(a, &g.p[a], dmg);
                g.hitstop = hitstop_for(dmg, knock, false);
                push_apart(&mut g.p, if knock { 34.0 } else { 9.0 });
            } else if blocked {
                g.hitstop = hitstop_for(0, false, true);
                push_apart(&mut g.p, 6.0);
            }
        }
        frame += 1;
    }
    stats.timed_out = g.p[0].health > 0 && g.p[1].health > 0;
    stats.player_health = g.p[0].health;
    stats.cpu_health = g.p[1].health;
    stats
}

/// Walks forward and mashes punch — the way everybody plays a fighting
/// game for the first sixty seconds of their life.
fn rusher(frame: usize, p: &[Fighter; 2]) -> Input {
    let mut i = Input::default();
    if p[1].x > p[0].x { i.right = true; } else { i.left = true; }
    i.punch_low = frame % 8 < 2;
    i
}

/// Holds back and blocks, and does nothing else.
fn turtle(_frame: usize, p: &[Fighter; 2]) -> Input {
    let mut i = Input::default();
    if p[1].x > p[0].x { i.left = true; } else { i.right = true; }
    i.down = true;
    i
}

/// Keeps its distance, crouch-blocks (standing into sweeps is a hole the game
/// punishes), punishes whiffs, and pokes from range.
fn poker(frame: usize, p: &[Fighter; 2]) -> Input {
    let mut i = Input::default();
    let dist = (p[1].x - p[0].x).abs();
    let toward = p[1].x > p[0].x;
    let (fwd, back) = if toward { (&mut i.right as *mut bool, &mut i.left as *mut bool) }
                      else { (&mut i.left as *mut bool, &mut i.right as *mut bool) };
    // SAFETY-free alternative would need two branches everywhere; these
    // are two distinct fields of a local struct.
    let (fwd, back) = unsafe { (&mut *fwd, &mut *back) };

    // The safe poke is the yardstick: the low kick. (Poking with the high
    // kick made the poker throw its most punishable move as its jab.)
    let my_kick = attack_range(&p[0], MoveId::LowKick);
    let whiffed = p[1].act == Act::Attack && p[1].hit_done;

    if whiffed && dist < my_kick {
        i.kick_high = true;                 // punish
    } else if dist > my_kick * 1.15 {
        *fwd = true;                        // close the gap
    } else if frame % 30 < 3 {
        i.kick_low = true;                  // poke from the edge
    } else if frame % 30 < 6 {
        i.down = true;
        i.kick_low = true;                  // and mix in a low
    } else {
        // Otherwise hold guard, and change which: one guard held all round
        // ate every high kick.
        *back = true;
        i.down = frame % 96 < 52;
    }
    i
}

#[test]
fn every_round_is_decided_by_something_that_happened() {
    // A round may run the clock out, but must not often end with both
    // fighters largely untouched: that is a stalemate the rules cannot break.
    // Asserted as a rate over several seeds, since one seed is a coin flip in
    // a chaotic simulation (the rate held at 2-3% over 180 rounds).
    let styles: [(&str, fn(usize, &[Fighter; 2]) -> Input); 3] =
        [("rusher", rusher), ("poker", poker), ("turtle", turtle)];
    let mut stalemates = vec![];
    let mut decided_fast = 0;
    let mut rounds = 0;

    for s in 0..4u64 {
        let _sim = simulating(0xB4A17E + s * 0x9E37);
        for pick in (0..FIGHTERS.len()).filter(|&i| !FIGHTERS[i].invincible) {
            for foe in [0, RUNGS - 3] {
                for (name, style) in styles {
                    let r = fight_round(pick, foe, style);
                    rounds += 1;
                    if !r.timed_out { decided_fast += 1; }
                    let cpu_who = Game { pick, ..Game::new() }.ladder()[foe];
                    let both_healthy = r.player_health as f32
                            > FIGHTERS[pick].health as f32 * 0.55
                        && r.cpu_health as f32 > FIGHTERS[cpu_who].health as f32 * 0.55;
                    if r.timed_out && both_healthy {
                        stalemates.push(format!("{} vs opponent {foe} [{name}] seed {s}: {} / {}",
                            FIGHTERS[pick].name, r.player_health, r.cpu_health));
                    }
                }
            }
        }
    }

    assert!(stalemates.len() * 12 <= rounds,
        "{} of {rounds} rounds ended with nothing having happened: {stalemates:?}",
        stalemates.len());
    assert!(decided_fast * 2 >= rounds,
        "only {decided_fast} of {rounds} rounds ended in a knockout — the clock is refereeing");
}

#[test]
fn playing_well_beats_playing_badly() {
    let _sim = simulating(0x5EED02);
    // The claim the whole design rests on. Poking and blocking at range
    // has to do measurably better against the same CPU than walking
    // forward and mashing, or none of the rules above are load-bearing.
    let mut rush_margin = 0;
    let mut poke_margin = 0;
    // Between fighters who have a range to keep: a half-size one is built
    // to leap in behind every blow, on either side of the fight.
    let spaces = |who: usize| !FIGHTERS[who].invincible && FIGHTERS[who].size >= 1.0;
    for pick in (0..FIGHTERS.len()).filter(|&i| spaces(i)) {
        for foe in [0, RUNGS - 3] {
            if !spaces(Game { pick, ..Game::new() }.ladder()[foe]) { continue; }
            let r = fight_round(pick, foe, rusher);
            rush_margin += r.player_health - r.cpu_health;
            let p = fight_round(pick, foe, poker);
            poke_margin += p.player_health - p.cpu_health;
        }
    }
    assert!(poke_margin > rush_margin,
        "spacing and blocking ({poke_margin}) did no better than mashing ({rush_margin})");
}

#[test]
fn turtling_does_not_win_on_its_own() {
    let _sim = simulating(0x5EED03);
    // Blocking has to be worth doing and not worth doing *only*. A
    // player who crouch-blocks forever should survive longer than a
    // rusher and still lose, because they never take the round.
    // Every rung but the last: two rungs are mostly turtles, who throw.
    let (mut survived, mut won, mut rounds) = (0, 0, 0);
    for pick in (0..FIGHTERS.len()).filter(|&i| !FIGHTERS[i].invincible) {
        for foe in 0..RUNGS - 1 {
            let r = fight_round(pick, foe, turtle);
            rounds += 1;
            if r.player_health > 0 { survived += 1; }
            if r.cpu_health <= 0 { won += 1; }
        }
    }
    assert_eq!(won, 0, "a fighter who never attacked won {won} rounds");
    assert!(survived * 3 >= rounds,
        "crouch-blocking survived only {survived} of {rounds} rounds — blocking is not working");
}

#[test]
#[ignore]
fn diagnose_matchups() {
    for pick in (0..FIGHTERS.len()).filter(|&i| !FIGHTERS[i].invincible) {
        for foe in [0, RUNGS - 3] {
            for (name, style) in [("rusher", rusher as fn(usize, &[Fighter; 2]) -> Input),
                                  ("poker", poker), ("turtle", turtle)] {
                let r = fight_round(pick, foe, style);
                println!("{:8} vs {:8} [{:6}] {:5.1}s  player {:4}  cpu {:4} {}",
                    FIGHTERS[pick].name,
                    FIGHTERS[Game { pick, ..Game::new() }.ladder()[foe]].name,
                    name, r.seconds, r.player_health, r.cpu_health,
                    if r.timed_out { "TIME" } else { "" });
            }
        }
    }
}

#[test]
#[ignore]
fn diagnose_cpu() {
    for style_name in ["rusher", "poker"] {
        let style: fn(usize, &[Fighter; 2]) -> Input = if style_name == "rusher" { rusher } else { poker };
        let mut g = Game::new();
        g.pick = 0;
        g.start_match(0);
        g.state = State::Fight;
        let mut plans: std::collections::BTreeMap<String, i32> = Default::default();
        let mut cpu_state: std::collections::BTreeMap<String, i32> = Default::default();
        let mut dist_sum = 0.0;
        let mut frames = 0;
        let mut secs = 0.0;
        while g.p[0].health > 0 && g.p[1].health > 0 && secs < ROUND_SECS {
            if g.hitstop > 0.0 { g.hitstop -= F; continue; }
            secs += F;
            let p_in = style(frames, &g.p);
            g.cpu_delay -= F;
            if g.cpu_delay <= 0.0 {
                g.cpu_plan = cpu_think(&mut g);
                g.cpu_delay = 0.38 - 0.18 * g.difficulty;
                *plans.entry(format!("{:?}", g.cpu_plan)).or_default() += 1;
            }
            *cpu_state.entry(format!("{:?}", g.p[1].act)).or_default() += 1;
            dist_sum += (g.p[1].x - g.p[0].x).abs();
            let c_in = cpu_input(&g);
            let ins = [p_in, c_in];
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
            for a in 0..2 {
                let d = 1 - a;
                let (mut atk, mut def) = (g.p[a], g.p[d]);
                let (dmg, blocked, knock) = resolve_hit(&mut atk, &mut def, ins[d]);
                g.p[a] = atk; g.p[d] = def;
                if dmg > 0 { g.hitstop = hitstop_for(dmg, knock, false); push_apart(&mut g.p, if knock {34.0} else {9.0}); }
                else if blocked { g.hitstop = hitstop_for(0,false,true); push_apart(&mut g.p, 6.0); }
            }
            frames += 1;
        }
        println!("--- {style_name}: {secs:.1}s  player {} cpu {}  avg dist {:.0}",
            g.p[0].health, g.p[1].health, dist_sum / frames as f32);
        println!("    plans: {plans:?}");
        println!("    cpu was: {cpu_state:?}");
    }
}

// ---- combos --------------------------------------------------------------

#[test]
fn a_landed_jab_can_be_cancelled_into_something_heavier() {
    // The reward for seeing your own hit land. Without it a jab is six
    // damage and a shrug, and there is no reason to ever throw the fast
    // move rather than the big one.
    let mut atk = at(0, 280.0, 1.0);
    let mut def = at(1, 320.0, -1.0);
    wind_to_active(&mut atk, MoveId::LowPunch);
    let (dmg, _, _) = resolve_hit(&mut atk, &mut def, Input::default());
    assert!(dmg > 0);
    assert!(atk.can_cancel(), "a landed jab opened no follow-up window");

    let mut kick = Input::default();
    kick.kick_high = true;
    apply_input(&mut atk, kick, false, F);
    assert_eq!(atk.mv, MoveId::HighKick, "the follow-up never came out");
    assert!(atk.chained, "the follow-up was not marked as a chain");
}

#[test]
fn a_blocked_poke_buys_nothing() {
    // Pressing buttons into a guard must not be a combo. If it were,
    // blocking would be the losing option at every range.
    let mut atk = at(0, 280.0, 1.0);
    let mut def = at(1, 320.0, -1.0);
    wind_to_active(&mut atk, MoveId::LowPunch);
    let mut back = Input::default();
    back.right = true; // away from the attacker on the left
    back.down = true;  // and low, because the low punch is low
    let (_, blocked, _) = resolve_hit(&mut atk, &mut def, back);
    assert!(blocked);
    assert!(!atk.can_cancel(), "a blocked poke opened a follow-up window");
}

#[test]
fn a_chain_cannot_chain_again() {
    // One cancel per attack. Jab into jab into jab comes out faster than
    // the hitstun it causes, which is an infinite, which is not a game.
    let mut atk = at(0, 280.0, 1.0);
    let mut def = at(1, 320.0, -1.0);
    wind_to_active(&mut atk, MoveId::LowPunch);
    resolve_hit(&mut atk, &mut def, Input::default());
    let mut jab = Input::default();
    jab.punch_low = true;
    apply_input(&mut atk, jab, false, F);          // the one cancel
    assert!(atk.chained);

    def.hit_done = false;
    wind_to_active(&mut atk, MoveId::LowPunch);
    atk.chained = true; // as start_attack would have left it
    let mut def2 = at(1, 320.0, -1.0);
    resolve_hit(&mut atk, &mut def2, Input::default());
    assert!(!atk.can_cancel(), "a chained attack could chain again");
}

#[test]
fn the_second_hit_of_a_combo_is_worth_less_than_the_first() {
    // Otherwise the reward for one read is the whole round.
    assert!(combo_scale(1) < combo_scale(0));
    assert!(combo_scale(2) < combo_scale(1));
    assert!(combo_scale(9) > 0.0, "a long combo should still do something");

    let mut atk = at(0, 280.0, 1.0);
    let mut def = at(1, 320.0, -1.0);
    wind_to_active(&mut atk, MoveId::HighKick);
    let (first, _, _) = resolve_hit(&mut atk, &mut def, Input::default());
    let mut atk2 = at(0, 280.0, 1.0);
    wind_to_active(&mut atk2, MoveId::HighKick);
    let (second, _, _) = resolve_hit(&mut atk2, &mut def, Input::default());
    assert!(second < first, "the second hit landed for full value ({second} vs {first})");
}

#[test]
fn getting_out_of_a_combo_resets_it() {
    let mut f = at(0, 280.0, 1.0);
    f.combo = 3;
    f.act = Act::Hitstun;
    f.stun = F;
    advance(&mut f, F * 2.0);
    assert_eq!(f.act, Act::Idle);
    assert_eq!(f.combo, 0, "the combo counter survived the recovery that ended it");
}

// ---- throws --------------------------------------------------------------

#[test]
fn a_throw_goes_through_any_guard() {
    // The third corner. Block beats attack, throw beats block, attack
    // beats throw — without this the game has a stalemate in it, and it
    // had a measurable one.
    for crouch in [false, true] {
        let mut atk = at(0, 300.0, 1.0);
        let mut def = at(1, 330.0, -1.0);
        wind_to_active(&mut atk, MoveId::Throw);
        let (dmg, blocked, knock) = resolve_hit(&mut atk, &mut def, back_input(-1.0, crouch));
        assert!(!blocked, "a throw was blocked (crouching: {crouch})");
        assert!(dmg > 0 && knock, "a throw should land hard and put them down");
    }
}

#[test]
fn a_throw_cannot_catch_someone_in_the_air() {
    // Jumping is an answer to a throw, as attacking is. Without that the
    // triangle collapses into "walk in and throw".
    let mut atk = at(0, 300.0, 1.0);
    let mut def = at(1, 330.0, -1.0);
    def.y = FLOOR_Y - 50.0;
    wind_to_active(&mut atk, MoveId::Throw);
    let (dmg, _, _) = resolve_hit(&mut atk, &mut def, Input::default());
    assert_eq!(dmg, 0, "a throw plucked someone out of the air");
}

#[test]
fn a_throw_only_comes_out_from_throw_range() {
    // Punch is punch at arm's length and a throw up close; at range the
    // player must still get the move they expect.
    let f = at(0, 300.0, 1.0);
    let mut punch = Input::default();
    punch.punch_low = true;
    assert_eq!(pressed_move(&f, punch, true), Some(MoveId::Throw));
    assert_eq!(pressed_move(&f, punch, false), Some(MoveId::LowPunch));
    // And crouching punch is never a throw — you cannot throw from
    // down. Holding down asks for the low one whichever punch button
    // found it, the same way it asks for the sweep.
    punch.down = true;
    assert_eq!(pressed_move(&f, punch, true), Some(MoveId::LowPunch));
    let mut high = Input::default();
    high.punch_high = true;
    high.down = true;
    assert_eq!(pressed_move(&f, high, false), Some(MoveId::LowPunch));
}

#[test]
fn a_whiffed_throw_is_the_worst_thing_you_can_do() {
    // It has to be, or throwing would simply be correct at close range.
    let throw = move_data(MoveId::Throw);
    for id in [MoveId::LowPunch, MoveId::LowKick, MoveId::HighPunch] {
        assert!(throw.recovery > move_data(id).recovery,
            "a missed throw recovers faster than a {id:?}");
    }
}

// ---- anatomy -------------------------------------------------------------
// The fighters are drawn from a skeleton, and a skeleton can be checked:
// every pose of every fighter across its animation, asserting that bones keep
// their length, hinges neither over-fold nor bend backwards, and nothing
// sinks through the floor. A failure names the joint and the frame.

/// Every pose the game can put a fighter in, as (label, fighter state).
fn every_pose() -> Vec<(String, Fighter)> {
    let mut out = Vec::new();
    let moves = [MoveId::LowPunch, MoveId::LowKick, MoveId::HighKick,
                 MoveId::HighPunch, MoveId::Sweep, MoveId::JumpPunch, MoveId::JumpKick,
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
fn drawn(f: &Fighter) -> draw::Skeleton {
    let rig = draw::Rig::upright(f.x, f.y, f.facing, f.facing, f.size());
    draw::skeleton(rig, &draw::pose_of(0.37, f, 0))
}

fn bones_of(f: &Fighter) -> draw::Skeleton {
    let rig = draw::Rig::upright(f.x, f.y, f.facing, f.facing, 1.0);
    draw::skeleton(rig, &draw::pose_of(0.37, f, 0))
}

#[test]
fn bones_never_change_length() {
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
fn hinges_stay_inside_their_range() {
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
fn nothing_is_drawn_underneath_the_floor() {
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
fn what_you_see_is_what_can_hit_you() {
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
fn why_is_that_matchup_quiet() {
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
fn a_standing_fighter_is_standing_on_something() {
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
fn knees_bend_forwards_and_elbows_bend_backwards() {
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
fn nothing_teleports_between_one_frame_and_the_next() {
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
fn a_waiting_fighter_has_their_knees_bent() {
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
fn no_limb_is_folded_up_to_nothing() {
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
fn dump_folds() {
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
fn dump_wings() {
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
fn no_elbow_sticks_out_behind_the_back() {
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
fn the_far_hand_stays_in_front_of_the_body() {
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
fn a_fighter_has_two_arms_two_legs_and_one_torso() {
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
fn a_fighter_fits_inside_their_own_hurtbox() {
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
fn a_fighter_who_covers_ground_takes_steps() {
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
fn a_fighter_blocking_on_the_spot_keeps_still() {
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
fn a_flying_kick_crosses_ground_no_other_move_can() {
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
fn a_flying_kick_must_be_blocked_standing() {
    let m = move_data(MoveId::FlyingKick);
    assert!(blocks(m.level, false), "a flying kick should be stopped by a standing guard");
    assert!(!blocks(m.level, true), "crouching should not stop a flying kick");
}

#[test]
fn a_blocked_flying_kick_is_a_free_punish() {
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
fn dump_flying_kick() {
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
fn the_cpu_throws_the_flying_kick_too() {
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
fn a_kick_pressed_in_the_air_always_comes_out() {
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
fn the_flying_kick_can_be_asked_for_by_a_human() {
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
fn a_flying_kick_looks_the_same_however_it_was_asked_for() {
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
fn fly_advantage(gap: f32, guard: bool) -> i32 {
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
fn landing_a_flying_kick_keeps_your_turn() {
    // A flying kick that hits must not leave the attacker minus: it pays
    // recovery on the ground after the defender's hitstun.
    for gap in [60.0f32, 100.0, 140.0, 175.0] {
        let adv = fly_advantage(gap, false);
        assert!(adv >= 0, "a flying kick that hit at gap {gap} left the attacker \
            {} frames behind", -adv);
    }
}

#[test]
fn a_blocked_flying_kick_gives_the_turn_away() {
    // The other half of the trade, and the whole risk of the move:
    // blocking it has to buy enough time to punish with something real.
    for gap in [60.0f32, 100.0, 140.0] {
        let adv = fly_advantage(gap, true);
        assert!(adv <= -9, "a blocked flying kick at gap {gap} left the attacker only \
            {} frames behind — not a punish", -adv);
    }
}

#[test]
fn crouching_under_a_flying_kick_does_not_work() {
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
fn a_flying_kick_can_be_met_in_the_air() {
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
fn what_you_see_is_what_can_hit_you_in_the_air_too() {
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
fn winning_a_round_in_mid_air_puts_you_back_on_the_floor() {
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
fn a_fighter_on_the_floor_is_only_as_tall_as_they_are_drawn() {
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
fn no_guard_height_covers_any_normal() {
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
fn wav_seconds(wav: &[u8]) -> f32 {
    let u32_at = |i: usize| u32::from_le_bytes([wav[i], wav[i + 1], wav[i + 2], wav[i + 3]]);
    let rate = u32_at(24) as f32;
    let bytes = u32_at(40) as f32;
    bytes / 2.0 / rate
}

#[test]
fn a_theme_outlasts_the_round_it_plays_under() {
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
fn the_three_themes_are_actually_different() {
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
fn building_the_themes_is_quick_enough_to_do_at_startup() {
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
fn dump_themes() {
    for which in 0..3 {
        let wav = blip_assets::brawler::theme_wav(which);
        std::fs::write(format!("/tmp/theme{which}.wav"), &wav).unwrap();
        println!("wrote /tmp/theme{which}.wav ({:.1}s)", wav_seconds(&wav));
    }
}

// ---- two players ---------------------------------------------------------

#[test]
fn the_two_players_share_no_keys() {
    // No key moves both fighters, read off the pads themselves rather than a
    // copied list.
    let keys = |p: &Pad| {
        let mut v: Vec<String> = Vec::new();
        for set in [p.up, p.down, p.left, p.right, p.punch, p.kick] {
            v.extend(set.iter().map(|k| format!("{k:?}")));
        }
        v
    };
    let one = keys(pad(Mode::Versus, 0));
    let two = keys(pad(Mode::Versus, 1));
    for a in &one {
        assert!(!two.contains(a), "{a} is bound to both players");
    }
    // And nobody has the same key on two of their own controls.
    for (name, set) in [("player one", &one), ("player two", &two)] {
        let mut sorted = set.clone();
        sorted.sort();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), before, "{name} has a key bound twice");
        assert_eq!(before, 6, "{name} should have six controls, has {before}");
    }
    // Sharing takes the arrows and the deck buttons off player one;
    // alone, they get everything back.
    let alone = keys(pad(Mode::Solo, 0));
    assert!(alone.len() > one.len(), "the solo player lost their cabinet keys");
    assert!(alone.iter().any(|k| k == "Up"), "the solo player cannot use the stick");
    // Player two is player two in every mode. Deciding by mode first
    // handed them player one's keys on every screen before the mode
    // had been chosen.
    assert_eq!(keys(pad(Mode::Solo, 1)), two,
        "player two was given player one's keys in one-player mode");
}

#[test]
fn two_players_cannot_bring_the_same_fighter() {
    // The one rule the select screen exists to enforce.
    let mut g = Game::new();
    g.mode = Mode::Versus;
    for taken in 0..FIGHTERS.len() {
        g.pick = taken;
        g.locked = [true, false];
        for at in 0..FIGHTERS.len() {
            assert_eq!(g.taken_by_other(1, at), at == taken,
                "player two's view of fighter {at} with {taken} taken is wrong");
        }
        // And the other way round.
        g.pick2 = taken;
        g.locked = [false, true];
        for at in 0..FIGHTERS.len() {
            assert_eq!(g.taken_by_other(0, at), at == taken);
        }
    }
}

#[test]
fn a_locked_fighter_is_free_again_in_one_player() {
    // Solo has nobody to collide with, and the lock-out must not leak
    // into it — a one-player game that refuses a fighter is a bug.
    let mut g = Game::new();
    g.mode = Mode::Solo;
    g.pick2 = 1;
    g.locked = [false, true];
    for at in 0..FIGHTERS.len() {
        assert!(!g.taken_by_other(0, at), "solo refused fighter {at}");
    }
}

#[test]
fn a_versus_match_is_one_match_with_a_winner() {
    // No ladder, no second opponent, no score: two people played one
    // match and one of them won it.
    let mut g = Game::new();
    g.mode = Mode::Versus;
    g.pick = 0;
    g.pick2 = 2;
    g.start_versus();
    assert_eq!(g.p[0].who, 0);
    assert_eq!(g.p[1].who, 2);
    assert_eq!(g.state, State::RoundIntro);

    // Give player one the match and run the end-of-match step.
    g.p[0].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    // Armed with a real duration: tick() reports nothing for a timer
    // that was never running.
    g.phase.start(0.05);
    for _ in 0..8 { update_match_end(&mut g, F); }
    assert_eq!(g.state, State::Over, "a versus match sent someone up a ladder");
    assert_eq!(g.opponent_index, 0, "a versus match advanced the ladder index");
}

#[test]
fn a_solo_run_still_climbs_the_ladder() {
    // The other half of the same branch: adding versus must not have
    // taken the ladder away from the one-player game.
    let mut g = Game::new();
    g.mode = Mode::Solo;
    g.pick = 0;
    g.start_match(0);
    g.p[0].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    // Armed with a real duration: tick() reports nothing for a timer
    // that was never running.
    g.phase.start(0.05);
    for _ in 0..8 { update_match_end(&mut g, F); }
    assert_eq!(g.opponent_index, 1, "beating the first opponent did not advance the ladder");
    assert_ne!(g.state, State::Over);
}

#[test]
fn each_player_has_their_own_special_window() {
    // Punch and kick inside a short window is the special. Held in one
    // pair of slots, player one's punch armed player two's kick and
    // both of them threw specials neither asked for.
    let mut g = Game::new();
    g.mode = Mode::Versus;
    g.now = 1.0;
    g.punch_at = [-1.0; 2];
    g.kick_at = [-1.0; 2];
    // Player one punches, player two kicks, on the same frame.
    g.punch_at[0] = g.now;
    g.kick_at[1] = g.now;
    for who in 0..2 {
        let (pa, ka) = (g.punch_at[who], g.kick_at[who]);
        let together = (pa - ka).abs() <= 0.08 && g.now - pa.max(ka) <= 0.08
            && pa > 0.0 && ka > 0.0;
        assert!(!together, "player {who} threw a special off the other player's button");
    }
}

#[test]
fn a_special_pressed_a_few_frames_apart_still_comes_out() {
    // Two buttons never land on the same frame. The first starts its own
    // attack; the second, a moment later, has to turn that into the special
    // (it used to be buffered behind the punch and forgotten).
    for (first, late) in [(Input { punch_low: true, ..Default::default() }, 3),
                          (Input { kick_low: true, ..Default::default() }, 4)] {
        let mut f = at(0, 200.0, 1.0);
        apply_input(&mut f, first, false, F);
        advance(&mut f, F);
        assert_eq!(f.act, Act::Attack);
        for _ in 1..late {
            apply_input(&mut f, Input::default(), false, F);
            advance(&mut f, F);
        }
        apply_input(&mut f, Input { special: true, ..Default::default() }, false, F);
        assert_eq!(f.mv, MoveId::Special, "the second button {late} frames late was lost");
        assert!(f.t < F, "the special did not start from its first frame");
    }
    // Once the first attack is out, it is too late: no special from a jab
    // that has already landed or missed.
    let mut f = at(0, 200.0, 1.0);
    f.start_attack(MoveId::LowPunch);
    for _ in 0..8 { advance(&mut f, F); }
    apply_input(&mut f, Input { special: true, ..Default::default() }, false, F);
    assert_eq!(f.mv, MoveId::LowPunch);
}

// ---- sizes ---------------------------------------------------------------

#[test]
fn nobody_strikes_over_the_head_of_a_smaller_opponent() {
    // Blows are aimed at the body in front of them: every grounded attack
    // of every fighter, thrown from just inside its range, touches every
    // other standing fighter, however small.
    let mut bad = vec![];
    for a in 0..FIGHTERS.len() {
        for d in 0..FIGHTERS.len() {
            for mv in [MoveId::LowPunch, MoveId::HighPunch, MoveId::LowKick, MoveId::HighKick,
                       MoveId::Sweep, MoveId::Throw] {
                let mut p = [at(a, 200.0, 1.0), at(d, 300.0, -1.0)];
                face_off(&mut p);
                p[1].x = p[0].x + attack_range(&p[0], mv) - 3.0;
                wind_to_active(&mut p[0], mv);
                let [atk, def] = &mut p;
                let (dmg, _, _) = resolve_hit(atk, def, Input::default());
                if dmg == 0 {
                    bad.push(format!("{}'s {mv:?} misses {}", FIGHTERS[a].name, FIGHTERS[d].name));
                }
            }
        }
    }
    assert!(bad.is_empty(), "attacks that cannot land:\n  {}", bad.join("\n  "));
}

#[test]
fn a_throw_is_offered_only_where_it_reaches() {
    // "Close enough to throw" is each fighter's own distance: a turtle has
    // to be nearer than a giant does.
    for a in 0..FIGHTERS.len() {
        for d in 0..FIGHTERS.len() {
            let mut p = [at(a, 200.0, 1.0), at(d, 300.0, -1.0)];
            face_off(&mut p);
            p[1].x = p[0].x + throw_range(&p[0]) - 0.5;
            assert!(face_off(&mut p)[0]);
            assert!(throw_range(&p[0]) < attack_range(&p[0], MoveId::Throw),
                "{} is offered a throw that cannot reach {}", FIGHTERS[a].name, FIGHTERS[d].name);
            // And it is a distance the two can actually stand at.
            assert!(throw_range(&p[0]) > (p[0].width() + p[1].width()) / 2.0 * 0.82,
                "{} can never get close enough to throw {}", FIGHTERS[a].name, FIGHTERS[d].name);
        }
    }
    let (small, big) = (at(3, 0.0, 1.0), at(8, 0.0, 1.0));
    assert!(throw_range(&small) < throw_range(&big));
}

// ---- the one nothing hurts -----------------------------------------------

#[test]
fn only_the_giant_hurts_the_invincible_fighter_but_blows_still_land() {
    let who = FIGHTERS.iter().position(|a| a.invincible).expect("somebody is invincible");
    let full = FIGHTERS[who].health;
    for mv in [MoveId::LowPunch, MoveId::HighKick, MoveId::Sweep, MoveId::Throw, MoveId::Special] {
        let mut p = [at(1, 200.0, 1.0), at(who, 300.0, -1.0)];
        face_off(&mut p);
        p[1].x = p[0].x + attack_range(&p[0], mv) - 3.0;
        wind_to_active(&mut p[0], mv);
        let [atk, def] = &mut p;
        let (dmg, blocked, _) = resolve_hit(atk, def, Input::default());
        assert!(dmg > 0 && !blocked, "{mv:?} did not land");
        assert_eq!(def.health, full, "{mv:?} hurt the fighter nothing hurts");
        assert!(matches!(def.act, Act::Hitstun | Act::Knockdown), "{mv:?} did not move them");
    }
    // Chip through a guard costs nothing either.
    let mut p = [at(1, 200.0, 1.0), at(who, 300.0, -1.0)];
    face_off(&mut p);
    p[1].x = p[0].x + attack_range(&p[0], MoveId::Special) - 3.0;
    wind_to_active(&mut p[0], MoveId::Special);
    let [atk, def] = &mut p;
    resolve_hit(atk, def, back_input(-1.0, false));
    assert_eq!(def.health, full);

    // Only someone bigger gets through, and only a little.
    let giant = FIGHTERS.iter().position(|a| a.size > FIGHTERS[who].size).unwrap();
    let mut p = [at(giant, 200.0, 1.0), at(who, 300.0, -1.0)];
    face_off(&mut p);
    p[1].x = p[0].x + attack_range(&p[0], MoveId::HighKick) - 3.0;
    wind_to_active(&mut p[0], MoveId::HighKick);
    let [atk, def] = &mut p;
    let (dmg, _, _) = resolve_hit(atk, def, Input::default());
    let lost = full - def.health;
    assert!(lost > 0 && lost * 3 < dmg, "the giant's {dmg} cost them {lost}");
}

#[test]
fn the_laser_is_aimed_at_the_head_and_goes_over_a_crouch() {
    let who = FIGHTERS.iter().position(|a| a.special == Special::LaserVision).unwrap();
    for foe in [0, 3] {
        let mut g = Game::new();
        g.p = [at(who, 200.0, 1.0), at(foe, 400.0, -1.0)];
        face_off(&mut g.p);
        g.spawn_bolt(0, 16);
        let b = g.bolts.iter().find(|b| b.active).unwrap();
        let (bx, by) = (0.0, b.y - 8.0);
        let stand = g.p[1].hurt_box();
        assert!(rects_overlap(bx, by, 20.0, 16.0, 0.0, stand.1, stand.2, stand.3),
            "the laser misses a standing {}", FIGHTERS[foe].name);
        g.p[1].act = Act::Crouch;
        let duck = g.p[1].hurt_box();
        assert!(!rects_overlap(bx, by, 20.0, 16.0, 0.0, duck.1, duck.2, duck.3),
            "{} cannot duck the laser", FIGHTERS[foe].name);
    }
}

#[test]
fn the_laser_fired_from_the_air_is_aimed_at_the_fighter() {
    // Level, it would pass over everyone's head. From anywhere in the air,
    // either side, it has to reach whoever it was fired at, big or small.
    let who = FIGHTERS.iter().position(|a| a.special == Special::LaserVision).unwrap();
    for foe in [0, 3, 8] {
        for (x, up) in [(120.0, 140.0), (300.0, 60.0), (520.0, 150.0), (330.0, 140.0)] {
            let mut g = Game::new();
            g.p = [at(who, x, 1.0), at(foe, 320.0, -1.0)];
            g.p[0].y = FLOOR_Y - up;
            g.p[0].act = Act::Air;
            face_off(&mut g.p);
            g.spawn_bolt(0, 16);
            let mut b = *g.bolts.iter().find(|b| b.active).unwrap();
            let (hx, hy, hw, hh) = g.p[1].hurt_box();
            let mut hit = false;
            for _ in 0..120 {
                b.x += b.vx * F;
                b.y += b.vy * F;
                if rects_overlap(b.x - 10.0, b.y - 8.0, 20.0, 16.0, hx, hy, hw, hh) { hit = true; break; }
                if b.y > FLOOR_Y { break; }
            }
            assert!(hit, "fired from ({x}, {up} up) the laser missed {}", FIGHTERS[foe].name);
        }
    }
}

#[test]
fn the_flier_hangs_in_the_air_until_told_to_land() {
    let who = FIGHTERS.iter().position(|a| a.build == Build::Caped).unwrap();
    let mut f = at(who, 300.0, 1.0);
    let hold = |up: bool, down: bool, right: bool| Input { up, down, right, ..Default::default() };
    let mut step = |f: &mut Fighter, inp: Input, frames: usize| {
        for _ in 0..frames { apply_input(f, inp, false, F); advance(f, F); }
    };
    step(&mut f, hold(true, false, false), 30);
    let high = FLOOR_Y - f.y;
    assert!(high > 60.0, "half a second of up lifted him {high:.0}px");
    // Hands off: he stays exactly where he is.
    step(&mut f, Input::default(), 90);
    assert!((FLOOR_Y - f.y - high).abs() < 0.5 && f.act == Act::Air, "he did not hover");
    // The stick moves him without gravity, and never above the ceiling.
    let x = f.x;
    step(&mut f, hold(true, false, true), 120);
    assert!(f.x > x + 100.0, "he did not fly forward");
    assert!(FLOOR_Y - f.y <= CEILING + 0.5, "he flew off the top of the screen");
    // Down brings him back to the boards, free to act.
    step(&mut f, hold(false, true, false), 120);
    assert!(!f.airborne() && f.free(), "he never landed: {:?} at {:.0}", f.act, FLOOR_Y - f.y);
    // Nobody else flies.
    let mut g = at(0, 300.0, 1.0);
    step(&mut g, hold(true, false, false), 5);
    step(&mut g, Input::default(), 120);
    assert!(!g.airborne());
}

#[test]
fn the_fliers_kick_is_the_laser_and_his_punch_hits_three_times_as_hard() {
    let who = FIGHTERS.iter().position(|a| a.build == Build::Caped).unwrap();
    let f = at(who, 300.0, 1.0);
    for kick in [Input { kick_low: true, ..Default::default() },
                 Input { kick_high: true, ..Default::default() },
                 Input { kick_low: true, down: true, ..Default::default() }] {
        assert_eq!(pressed_move(&f, kick, true), Some(MoveId::Special));
    }
    let punch = Input { punch_low: true, ..Default::default() };
    assert_eq!(pressed_move(&f, punch, true), Some(MoveId::HighPunch), "he throws, or jabs");
    let base = move_data(MoveId::HighPunch).damage;
    assert_eq!(f.scaled(move_data(MoveId::HighPunch)).damage, base * 3);
    // And it sends them flying.
    let mut p = [f, at(1, 300.0, -1.0)];
    face_off(&mut p);
    p[1].x = p[0].x + attack_range(&p[0], MoveId::HighPunch) - 3.0;
    wind_to_active(&mut p[0], MoveId::HighPunch);
    let [atk, def] = &mut p;
    let (dmg, _, floored) = resolve_hit(atk, def, Input::default());
    assert!(dmg == base * 3 && floored && def.act == Act::Knockdown);
}

#[test]
fn the_last_fight_is_won_by_being_on_your_feet_at_the_bell() {
    // Nobody but the giant can knock out the fighter nothing hurts, so a
    // round against him goes to the player who lasts it out.
    let boss = FIGHTERS.iter().position(|a| a.invincible).unwrap();
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(RUNGS - 1);
    assert_eq!(g.p[1].who, boss);
    g.p[0].health = 7;
    assert!(survived(&g));
    assert_eq!(round_result(&g, false), RoundResult::P1, "time ran out and he still took the round");
    // Knocked out is knocked out.
    g.p[0].health = 0;
    assert_eq!(round_result(&g, true), RoundResult::P2);
    // Against anyone else the bell goes to whoever has more left.
    g.start_match(0);
    g.p[0].health = 7;
    assert!(!survived(&g));
    assert_eq!(round_result(&g, false), RoundResult::P2);
    // And two players settle it on health, whoever they picked.
    g.mode = Mode::Versus;
    g.p[1] = at(boss, 440.0, -1.0);
    assert!(!survived(&g));
}

#[test]
fn the_ladder_gets_harder_all_the_way_up() {
    let mut g = Game { pick: 0, ..Game::new() };
    let mut last = f32::MIN;
    for rung in 0..RUNGS {
        g.start_match(rung);
        assert!(g.difficulty > last, "fight {} is no harder than the one before", rung + 1);
        last = g.difficulty;
    }
    assert!((last - LAST_RUNG).abs() < 1e-4);
}

// ---- ten additions --------------------------------------------------------

/// `a` at full extension of `mv`, just inside its range of `d`.
fn in_range(a: usize, d: usize, mv: MoveId) -> [Fighter; 2] {
    let mut p = [at(a, 200.0, 1.0), at(d, 300.0, -1.0)];
    face_off(&mut p);
    p[1].x = p[0].x + attack_range(&p[0], mv) - 3.0;
    wind_to_active(&mut p[0], mv);
    p
}

#[test]
fn hitting_someone_out_of_their_attack_is_a_counter() {
    let plain = {
        let [mut a, mut d] = in_range(0, 2, MoveId::LowKick);
        resolve_hit(&mut a, &mut d, Input::default()).0
    };
    // Caught winding up: more damage, more stun, and it is announced.
    let [mut a, mut d] = in_range(0, 2, MoveId::LowKick);
    d.start_attack(MoveId::HighKick);
    let (dmg, _, _) = resolve_hit(&mut a, &mut d, Input::default());
    assert!(a.countered && dmg > plain, "a counter did {dmg} against a plain {plain}");
    assert!(d.stun > move_data(MoveId::LowKick).hitstun * F);
    // Caught recovering is a punish, not a counter.
    let [mut a, mut d] = in_range(0, 2, MoveId::LowKick);
    d.start_attack(MoveId::HighKick);
    d.t = 0.5;
    let (dmg, _, _) = resolve_hit(&mut a, &mut d, Input::default());
    assert!(!a.countered && dmg == plain);
}

#[test]
fn the_last_quarter_of_the_bar_hits_harder() {
    let base = move_data(MoveId::HighKick);
    let mut f = at(0, 200.0, 1.0);
    let fresh = f.scaled(base).damage;
    f.health = FIGHTERS[0].health / 4;
    assert!(f.enraged() && f.scaled(base).damage > fresh);
    f.health = FIGHTERS[0].health / 4 + 1;
    assert!(!f.enraged() && f.scaled(base).damage == fresh);
    // Out is out, and the fighter nothing hurts has nothing to be angry about.
    f.health = 0;
    assert!(!f.enraged());
    let mut z = at(FIGHTERS.iter().position(|a| a.invincible).unwrap(), 200.0, 1.0);
    z.health = 1;
    assert!(!z.enraged());
}

#[test]
fn a_tap_toward_at_the_last_moment_turns_the_blow() {
    let toward = Input { left: true, ..Default::default() }; // the defender faces left
    // Tapped just before it lands: no damage, and the attacker is left open.
    let [mut a, mut d] = in_range(0, 1, MoveId::HighKick);
    apply_input(&mut d, toward, false, F);
    let full = d.health;
    let (dmg, blocked, _) = resolve_hit(&mut a, &mut d, toward);
    assert!(dmg == 0 && blocked && d.health == full && a.turned);
    assert!(a.act == Act::Hitstun && a.stun >= IMPACT_STUN - 0.001);
    assert!(d.free(), "the defender is not free to answer");
    // Held, not tapped: walking into a kick is just being kicked.
    let [mut a, mut d] = in_range(0, 1, MoveId::HighKick);
    for _ in 0..12 { apply_input(&mut d, toward, false, F); }
    assert!(resolve_hit(&mut a, &mut d, toward).0 > 0);
    // A throw is not a blow, and cannot be turned.
    let [mut a, mut d] = in_range(0, 1, MoveId::Throw);
    apply_input(&mut d, toward, false, F);
    assert!(resolve_hit(&mut a, &mut d, toward).0 > 0);
}

#[test]
fn the_fruit_comes_once_a_round_and_feeds_whoever_reaches_it() {
    let mut g = Game::new();
    g.start_match(0);
    g.state = State::Fight;
    assert!(fruit_step(&mut g, F).is_none() && g.fruit.ttl <= 0.0, "it is there from the bell");
    g.clock = FRUIT_AT - 0.01;
    fruit_step(&mut g, F);
    assert!(g.fruit.ttl > 0.0, "it never appeared");
    assert!((g.fruit.x - g.p[0].x).abs() > 60.0 && (g.fruit.x - g.p[1].x).abs() > 60.0);
    // Jumping over it is not eating it.
    g.p[0].health = 40;
    g.p[0].x = g.fruit.x;
    g.p[0].y = FLOOR_Y - 60.0;
    assert!(fruit_step(&mut g, F).is_none());
    // Walking onto it is, and it gives some health back and is gone.
    g.p[0].y = FLOOR_Y;
    assert_eq!(fruit_step(&mut g, F), Some(0));
    assert!(g.p[0].health > 40 && g.p[0].health < FIGHTERS[g.p[0].who].health);
    assert!(g.fruit.ttl <= 0.0 && g.sess.score > 0);
    assert!(fruit_step(&mut g, F).is_none() && g.fruit.ttl <= 0.0, "a second helping");
    // It never fills past full, and the next round serves the next one.
    let first = g.fruit.kind;
    g.round += 1;
    g.start_round();
    g.clock = FRUIT_AT - 0.01;
    g.p[1].x = WIN_W as f32 / 2.0;
    assert_eq!(fruit_step(&mut g, F), Some(1));
    assert_eq!(g.p[1].health, FIGHTERS[g.p[1].who].health);
    assert_ne!(g.fruit.kind, first);
    // Left alone it goes away.
    g.round += 1;
    g.start_round();
    g.clock = FRUIT_AT - 0.01;
    for _ in 0..((FRUIT_STAYS / F) as usize + 5) { fruit_step(&mut g, F); }
    assert!(g.fruit.ttl <= 0.0);
}

#[test]
fn two_bolts_cancel_and_the_laser_burns_through() {
    let bolt = |x: f32, vx: f32, owner: usize| Bolt { x, y: 276.0, vx, vy: 0.0, owner, active: true, damage: 16 };
    let mut g = Game::new();
    g.p = [at(0, 100.0, 1.0), at(3, 540.0, -1.0)];
    g.bolts[0] = bolt(300.0, 300.0, 0);
    g.bolts[1] = bolt(312.0, -300.0, 1);
    assert!(clash_bolts(&mut g).is_some());
    assert!(g.bolts.iter().all(|b| !b.active), "a bolt survived the clash");
    // Two from the same hand pass each other by.
    g.bolts[0] = bolt(300.0, 300.0, 0);
    g.bolts[1] = bolt(312.0, 300.0, 0);
    assert!(clash_bolts(&mut g).is_none());
    // The laser eats the bolt and carries on.
    let laser = FIGHTERS.iter().position(|a| a.special == Special::LaserVision).unwrap();
    g.p[1] = at(laser, 540.0, -1.0);
    g.bolts[1] = bolt(312.0, -600.0, 1);
    assert!(clash_bolts(&mut g).is_some());
    assert!(!g.bolts[0].active && g.bolts[1].active);
}

#[test]
fn a_turtle_in_its_shell_cannot_be_chipped() {
    let chip = |who: usize, crouch: bool| {
        let [mut a, mut d] = in_range(1, who, MoveId::Special);
        let full = d.health;
        let (_, blocked, _) = resolve_hit(&mut a, &mut d, back_input(-1.0, crouch));
        assert!(blocked);
        (full - d.health, d.shelled())
    };
    assert_eq!(chip(3, true), (0, true), "the shell was chipped");
    assert!(chip(3, false).0 > 0 && !chip(3, false).1, "a standing turtle is not in its shell");
    assert!(chip(0, true).0 > 0, "only turtles have a shell");
}

#[test]
fn the_announcer_calls_a_perfect_and_a_great() {
    let mut g = Game::new();
    g.p = [at(0, 200.0, 1.0), at(1, 440.0, -1.0)];
    g.result = RoundResult::P1;
    assert_eq!(round_call(&g), "PERFECT");
    g.p[0].health = FIGHTERS[0].health - 1;
    assert_eq!(round_call(&g), "");
    g.p[0].health = FIGHTERS[0].health / 10;
    assert_eq!(round_call(&g), "GREAT");
    g.result = RoundResult::Draw;
    assert_eq!(round_call(&g), "");
    // Untouched is nothing to shout about when nothing can touch you.
    let z = FIGHTERS.iter().position(|a| a.invincible).unwrap();
    g.p[1] = at(z, 440.0, -1.0);
    g.result = RoundResult::P2;
    assert_eq!(round_call(&g), "");
}

#[test]
fn the_finishing_blow_plays_in_slow_motion_and_the_bar_drains_after_it() {
    let mut g = Game::new();
    g.start_match(0);
    g.state = State::RoundEnd;
    g.phase.start(2.2);
    g.p[0].act = Act::Victory;
    g.p[0].t = 0.0;
    g.slow = SLOW_MO_SECS;
    for _ in 0..12 { update_round_end(&mut g, F); }
    assert!(g.p[0].t < 12.0 * F * 0.5, "the fighters ran at full speed: {:.3}", g.p[0].t);
    for _ in 0..60 { update_round_end(&mut g, F); }
    assert!(g.slow <= 0.0, "the slow motion never ended");
    // A bar's ghost falls to the health under it and never past.
    g.p[1].health = 20;
    g.ghost[1] = 100.0;
    drain_ghosts(&mut g, 0.1);
    assert!(g.ghost[1] < 100.0 && g.ghost[1] > 20.0);
    for _ in 0..100 { drain_ghosts(&mut g, 0.1); }
    assert_eq!(g.ghost[1], 20.0);
}

#[test]
fn the_giants_rage_jump_floors_anyone_standing_anywhere() {
    let giant = FIGHTERS.iter().position(|a| a.build == Build::Giant).unwrap();
    // A whole jump: up, the kick in the air, and down again.
    let jump = |g: &mut Game, kick: bool| {
        apply_input(&mut g.p[0], Input { up: true, ..Default::default() }, false, F);
        let mut shook = false;
        for frame in 0..120 {
            let inp = Input { kick_low: kick && frame == 12, ..Default::default() };
            apply_input(&mut g.p[0], inp, false, F);
            let was_air = g.p[0].airborne();
            advance(&mut g.p[0], F);
            if was_air && !g.p[0].airborne() { shook = land_quake(g, 0); break; }
        }
        shook
    };
    let fresh = |foe_x: f32| {
        let mut g = Game::new();
        g.p = [at(giant, 80.0, 1.0), at(0, foe_x, -1.0)];
        g
    };
    // From the far end of the stage, a fighter on the floor goes down.
    let mut g = fresh(WIN_W as f32 - WALL_MARGIN);
    assert!(jump(&mut g, true), "the floor did not shake");
    assert_eq!(g.p[1].act, Act::Knockdown);
    assert!(g.p[1].health < FIGHTERS[0].health && g.quake_t > 0.0 && g.shake > 0.2);
    // Being in the air when he lands is the answer.
    let mut g = fresh(500.0);
    g.p[1].y = FLOOR_Y - 40.0;
    assert!(jump(&mut g, true));
    assert_ne!(g.p[1].act, Act::Knockdown);
    // A plain jump shakes nothing, and neither does anybody else's kick.
    let mut g = fresh(500.0);
    assert!(!jump(&mut g, false) && g.p[1].act != Act::Knockdown);
    let mut g = fresh(500.0);
    g.p[0] = at(1, 80.0, 1.0);
    assert!(!jump(&mut g, true));
    // The CPU knows the answer, and at the top of the ladder usually finds it.
    let _sim = simulating(0x57031);
    let mut g = fresh(500.0);
    g.difficulty = 0.75;
    g.p[0].y = FLOOR_Y - 80.0;
    g.p[0].stomp = true;
    let jumps = (0..200).filter(|_| cpu_think(&mut g) == CpuPlan::Jump).count();
    assert!(jumps > 120, "the CPU jumped a rage jump {jumps} times in 200");
    // Knocked out of the air, he does not get his landing.
    let mut g = fresh(500.0);
    g.p[0].stomp = true;
    g.p[0].act = Act::Knockdown;
    assert!(!land_quake(&mut g, 0) && !g.p[0].stomp);
}

// ---- two-player play tests -----------------------------------------------
// The real flow: `update_menus` as main() calls it, and the game's own
// `apply_input` / `advance` / `resolve_hit`. Only keyboard and speaker are
// replaced.

fn press(back: bool, fwd: bool, fire: bool) -> MenuIn { MenuIn { back, fwd, fire } }
const NOTHING: MenuIn = MenuIn { back: false, fwd: false, fire: false };

/// Walk the title menu into two-player mode and return the game sitting
/// on the select screen.
fn at_versus_select() -> Game {
    let mut g = Game::new();
    assert_eq!(g.state, State::Title);
    update_menus(&mut g, [press(false, true, false), NOTHING]); // move to 2 PLAYERS
    assert_eq!(g.menu, 1, "the title menu did not move");
    update_menus(&mut g, [press(false, false, true), NOTHING]); // confirm
    assert_eq!(g.state, State::Select);
    assert_eq!(g.mode, Mode::Versus);
    g
}

/// Step a player's cursor `n` times forward.
fn cursor(g: &mut Game, who: usize, n: usize) {
    for _ in 0..n {
        let mut m = [NOTHING; 2];
        m[who] = press(false, true, false);
        update_menus(g, m);
    }
}

fn lock(g: &mut Game, who: usize) {
    let mut m = [NOTHING; 2];
    m[who] = press(false, false, true);
    update_menus(g, m);
}

/// Fight it out with two scripted humans until somebody wins the match
/// or the clock beats them both. Returns (frames, rounds won by each).
fn versus_bout(g: &mut Game, style: fn(usize, usize, &[Fighter; 2]) -> Input) -> (usize, [i32; 2]) {
    let mut frames = 0usize;
    while frames < 60 * 240 {
        frames += 1;
        if !step_versus(g, frames, style) { break; }
    }
    (frames, [g.p[0].rounds, g.p[1].rounds])
}

/// One frame of a versus match. Returns false once the match is over.
fn step_versus(g: &mut Game, frames: usize, style: fn(usize, usize, &[Fighter; 2]) -> Input) -> bool {
    match g.state {
        State::Vs => update_vs(g, F, false),
        State::RoundIntro => { if g.phase.tick(F) { g.state = State::Fight; } }
        State::RoundEnd => update_round_end(g, F),
        State::MatchEnd => { update_match_end(g, F); }
        State::Over | State::Won => return false,
        State::Fight => {
            if g.hitstop > 0.0 { g.hitstop -= F; return true; }
            g.clock -= F;
            let ins = [style(0, frames, &g.p), style(1, frames, &g.p)];
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
            for i in 0..2 {
                let (a, d) = if i == 0 { (0, 1) } else { (1, 0) };
                let mut atk = g.p[a];
                let mut def = g.p[d];
                let _ = resolve_hit(&mut atk, &mut def, ins[d]);
                g.p[a] = atk;
                g.p[d] = def;
            }
            if g.p[0].health <= 0 || g.p[1].health <= 0 || g.clock <= 0.0 {
                g.result = if g.p[0].health == g.p[1].health { RoundResult::Draw }
                    else if g.p[0].health > g.p[1].health { RoundResult::P1 }
                    else { RoundResult::P2 };
                match g.result {
                    RoundResult::P1 => { g.p[0].rounds += 1; }
                    RoundResult::P2 => { g.p[1].rounds += 1; }
                    RoundResult::Draw => { g.p[0].rounds += 1; g.p[1].rounds += 1; }
                }
                g.state = State::RoundEnd;
                g.phase.start(0.2);
            }
        }
        _ => return false,
    }
    true
}

/// Two humans who both actually fight.
fn brawlers(who: usize, frame: usize, p: &[Fighter; 2]) -> Input {
    let mut i = Input::default();
    let me = p[who];
    let foe = p[1 - who];
    let toward_right = foe.x > me.x;
    let phase = frame + who * 17;
    if (foe.x - me.x).abs() > 80.0 {
        if toward_right { i.right = true; } else { i.left = true; }
    } else if phase % 37 < 4 {
        i.punch_low = true;
    } else if phase % 37 < 8 {
        i.kick_low = true;
    } else if phase % 37 < 11 {
        i.kick_high = true;
    } else if phase % 37 < 20 {
        if toward_right { i.left = true; } else { i.right = true; } // guard
    } else if toward_right { i.right = true; } else { i.left = true; }
    i
}

#[test]
fn playtest_1_a_whole_two_player_session() {
    // Title to winner, through the real flow, pressing the real keys.
    let _sim = simulating(0x2F1A);
    let mut g = at_versus_select();
    cursor(&mut g, 1, 1);            // P2 looks around
    lock(&mut g, 0);                 // P1 takes RYUKA
    lock(&mut g, 1);                 // P2 takes whoever they are on
    assert_eq!(g.state, State::Vs, "locking both did not start the match");
    assert_ne!(g.p[0].who, g.p[1].who, "the same fighter reached the ring twice");

    let (frames, rounds) = versus_bout(&mut g, brawlers);
    assert!(frames < 60 * 240, "the match never finished");
    assert!(rounds[0] >= ROUNDS_TO_WIN || rounds[1] >= ROUNDS_TO_WIN,
        "nobody took the match: {rounds:?}");
    assert_eq!(g.state, State::Over, "a versus match did not end at the result screen");
}

#[test]
fn playtest_2_player_two_is_moved_off_a_fighter_that_gets_taken() {
    // Both cursors on one fighter, player one takes it: player two moves to a
    // free one.
    let mut g = at_versus_select();
    g.pick = 0;
    g.pick2 = 0;
    lock(&mut g, 0);
    assert!(g.locked[0]);
    assert_ne!(g.pick2, g.pick, "player two was left stranded on a taken fighter");
    lock(&mut g, 1);
    assert!(g.locked[1], "player two could not lock the fighter they were moved to");
    assert_ne!(g.p[0].who, g.p[1].who, "the same fighter reached the ring twice");
    assert_eq!(g.state, State::Vs);
}

#[test]
fn playtest_3_either_player_may_lock_in_first() {
    // The lock-out has to work both ways round, and the cursor that is
    // still moving must never be able to land on the taken fighter.
    for first in 0..2 {
        let second = 1 - first;
        let mut g = at_versus_select();
        g.pick = 1;
        g.pick2 = 1;
        lock(&mut g, first);
        let taken = g.picked(first);
        assert_ne!(g.picked(second), taken,
            "player {second} was left on the fighter player {first} took");
        // Walk the whole ring twice; it must never land on the taken one.
        for _ in 0..FIGHTERS.len() * 2 {
            cursor(&mut g, second, 1);
            assert_ne!(g.picked(second), taken,
                "player {second}'s cursor landed on the taken fighter");
        }
        lock(&mut g, second);
        assert!(g.locked[second], "player {second} could not lock a free fighter");
        assert_eq!(g.state, State::Vs);
        assert_ne!(g.p[0].who, g.p[1].who);
    }
}

#[test]
fn playtest_4_each_player_drives_only_their_own_fighter() {
    // The failure that makes the mode unplayable: one person's stick
    // moving both fighters.
    let mut g = at_versus_select();
    lock(&mut g, 0);
    lock(&mut g, 1);
    g.state = State::Fight;
    let (x0, x1) = (g.p[0].x, g.p[1].x);
    // Only player one presses anything, for a while.
    for _ in 0..40 {
        let mut a = Input::default();
        a.right = true;
        for i in 0..2 {
            let inp = if i == 0 { a } else { Input::default() };
            apply_input(&mut g.p[i], inp, false, F);
            advance(&mut g.p[i], F);
        }
    }
    assert!(g.p[0].x > x0 + 20.0, "player one did not move on their own input");
    assert!((g.p[1].x - x1).abs() < 0.5,
        "player two moved {:.0}px without touching anything", (g.p[1].x - x1).abs());
}

#[test]
fn playtest_5_one_player_mode_is_untouched() {
    // The whole feature is worthless if it broke the game that was
    // already there. Walk the solo path through the same real flow.
    let _sim = simulating(0x1F5A);
    let mut g = Game::new();
    update_menus(&mut g, [press(false, false, true), NOTHING]);  // 1 PLAYER
    assert_eq!(g.mode, Mode::Solo);
    assert_eq!(g.state, State::Select);
    // One cursor, and no lock-out: every fighter is selectable.
    for _ in 0..FIGHTERS.len() {
        cursor(&mut g, 0, 1);
        assert!(!g.taken_by_other(0, g.pick));
    }
    // Player two's keys do nothing on the solo select screen.
    let before = g.pick;
    cursor(&mut g, 1, 3);
    assert_eq!(g.pick, before, "player two's keys moved the solo cursor");
    update_menus(&mut g, [press(false, false, true), NOTHING]);
    assert_eq!(g.state, State::Vs, "the solo game would not start");
    assert_eq!(g.p[1].who, g.ladder()[0], "the solo ladder picked the wrong opponent");
}

#[test]
fn a_versus_result_reports_a_winner_not_a_score() {
    // A versus match ends with a winner, not a solo run's GAME OVER over a
    // score.
    let mut g = Game::new();
    g.mode = Mode::Versus;
    g.pick = 0;
    g.pick2 = 2;
    g.start_versus();
    g.p[1].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    g.phase.start(0.05);
    for _ in 0..8 { update_match_end(&mut g, F); }
    assert_eq!(g.state, State::Over);
    // Nothing was scored, and nothing should claim to have been.
    assert_eq!(g.sess.score, 0, "a versus match reported a score");
    assert!(g.p[1].rounds > g.p[0].rounds, "the result lost track of who won");
}

// ---- playtest diagnostics (run with --ignored) ---------------------------

#[test]
#[ignore = "diagnostic, not an assertion"]
fn diag_round_1_how_long_before_a_press_becomes_a_hit() {
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
fn diag_round_2_does_the_planted_foot_skate() {
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
fn diag_round_3_how_much_of_the_round_is_spent_on_top_of_each_other() {
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
fn diag_round_4_is_a_waiting_fighter_visibly_alive() {
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
fn diag_round_5_how_fast_does_a_fighter_answer_the_stick() {
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

// ---- the walk and the idle bounce ---------------------------------

#[test]
fn a_planted_foot_stays_where_it_was_put() {
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
fn a_step_is_a_step_and_not_a_hop() {
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
fn two_fighters_never_bounce_in_step() {
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

// ---- ten rounds of looking at it ----------------------------------------
// Diagnostics of the drawing (`--ignored`): what a player can tell apart at
// sixty pixels. The ones that found something have a real assertion further
// down.

/// Worst offenders first, with the label, so a number leads somewhere.
fn report(title: &str, mut rows: Vec<(f32, String)>) {
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    println!("--- {title} ({} samples)", rows.len());
    for (v, what) in rows.iter().take(8) { println!("    {v:8.2}  {what}"); }
}

#[test]
#[ignore = "diagnostic, not an assertion"]
fn diag_round_6_is_any_leg_curled_up_behind_the_back() {
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
fn diag_round_7_can_the_two_of_anything_be_told_apart() {
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
fn diag_round_8_is_the_head_ever_swallowed() {
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
fn diag_round_9_how_tall_is_a_fighter_really() {
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
fn diag_round_10_does_the_background_compete_with_the_fight() {
    // Not geometry: how much ink the stage spends. Counted as the number
    // of separate things drawn behind the fighters, because every one of
    // them is something the eye has to rule out.
    println!("counted by hand from draw_dock / draw_temple — see the test below");
}

fn dist2(a: draw::V, b: draw::V) -> f32 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}

#[test]
fn no_leg_is_curled_up_behind_the_back() {
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

// ---- what a fight sounds like -------------------------------------------

/// Every sample of a mono 16-bit WAV, as floats.
fn wav_samples(wav: &[u8]) -> Vec<f32> {
    wav[44..].chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
        .collect()
}

/// How high a sound sits, as zero crossings per second. Crude against a
/// real spectrum and exactly right for the question asked of it: is
/// this one brighter than that one.
fn brightness(wav: &[u8]) -> f32 {
    let s = wav_samples(wav);
    let crossings = s.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count();
    crossings as f32 / (s.len() as f32 / 44_100.0).max(0.001)
}

/// How much energy a sound carries at one frequency — one bin of a
/// discrete Fourier transform, done the direct way. Forty of these is a
/// coarse spectrum, which is all any question here needs.
fn bin(s: &[f32], hz: f32) -> f32 {
    let w = 2.0 * std::f32::consts::PI * hz / 44_100.0;
    let (mut re, mut im) = (0.0f32, 0.0f32);
    for (i, x) in s.iter().enumerate() {
        re += x * (w * i as f32).cos();
        im += x * (w * i as f32).sin();
    }
    (re * re + im * im).sqrt() / s.len() as f32
}

/// The frequency a sound sits on and how much it stands out. Not zero
/// crossings: a hit is two thirds hiss, and noise swamps the thump's pitch.
fn dominant(wav: &[u8], lo: f32, hi: f32) -> (f32, f32) {
    let s = wav_samples(wav);
    let n = 40;
    let bins: Vec<(f32, f32)> = (0..n)
        .map(|i| {
            let hz = lo * (hi / lo).powf(i as f32 / (n - 1) as f32);
            (hz, bin(&s, hz))
        })
        .collect();
    let mean = bins.iter().map(|b| b.1).sum::<f32>() / n as f32;
    let peak = bins.iter().cloned().fold((0.0, 0.0), |a, b| if b.1 > a.1 { b } else { a });
    (peak.0, peak.1 / mean.max(1e-9))
}

fn asset(name: &str) -> Vec<u8> {
    blip_assets::brawler::generate().into_iter()
        .find(|(n, _)| n.ends_with(name))
        .unwrap_or_else(|| panic!("no asset {name}")).1
}

#[test]
fn a_combo_climbs_as_it_lands() {
    // Three light hits, each brighter, picked by the combo count, so a
    // four-hit string sounds like one.
    let b: Vec<f32> = ["hit_light.wav", "hit_light2.wav", "hit_light3.wav"]
        .iter().map(|n| dominant(&asset(n), 150.0, 1600.0).0).collect();
    assert!(b[1] > b[0] * 1.15 && b[2] > b[1] * 1.15,
        "the three light hits do not climb: {b:?}");
}

#[test]
fn a_body_hitting_the_floor_does_not_sound_like_a_jab() {
    // A knockdown is the biggest thing in a round: its own sound, not a
    // heavy hit's.
    let jab = asset("hit_light.wav");
    let heavy = asset("hit_heavy.wav");
    let crunch = asset("crunch.wav");
    assert_ne!(crunch, heavy, "a knockdown still plays the heavy hit");
    // Lower than both, and longer than both: that is what makes it read
    // as weight rather than as speed.
    let (c_hz, _) = dominant(&crunch, 80.0, 1600.0);
    let (h_hz, _) = dominant(&heavy, 80.0, 1600.0);
    assert!(c_hz < h_hz,
        "the crunch ({c_hz:.0}Hz) is not lower than a heavy hit ({h_hz:.0}Hz)");
    assert!(brightness(&crunch) < brightness(&jab) * 0.9,
        "the crunch is nearly as bright as a jab");
    assert!(wav_seconds(&crunch) > wav_seconds(&heavy) * 1.2,
        "the crunch is no longer than a heavy hit");
}

#[test]
fn the_crowd_reacts_for_longer_than_it_takes_to_notice() {
    // Thirty-odd onlookers have been drawn watching every round in
    // silence. A cheer that is over before the eye reaches them is the
    // same as no cheer.
    let cheer = blip_assets::brawler::crowd_wav(false);
    let roar = blip_assets::brawler::crowd_wav(true);
    assert!(wav_seconds(&cheer) > 0.6, "the knockdown cheer is {:.2}s", wav_seconds(&cheer));
    assert!(wav_seconds(&roar) > wav_seconds(&cheer) * 1.5,
        "a KO gets no more of a reaction than a knockdown");
    // It has to be a room, not a chord: noise is flat, so no one
    // frequency in it should stand far above its neighbours the way a
    // hit's thump does.
    let (_, crowd_peak) = dominant(&roar, 200.0, 4000.0);
    let (_, hit_peak) = dominant(&asset("hit_heavy.wav"), 200.0, 4000.0);
    assert!(crowd_peak < hit_peak,
        "the crowd has a pitch in it ({crowd_peak:.1}x over flat, against a hit's \
         {hit_peak:.1}x) — that is a chord, not a room");
    // And it must not clip: it plays under a hit and a bell.
    let peak = wav_samples(&roar).iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(peak < 0.99, "the crowd clips at {peak:.3}");
}

#[test]
fn building_the_crowd_is_quick_enough_to_do_at_startup() {
    // Synthesised on the device rather than shipped — a quarter of a
    // megabyte that never crosses the wire — which is only a good trade
    // if the player does not wait for it.
    let t = std::time::Instant::now();
    std::hint::black_box(blip_assets::brawler::crowd_wav(false));
    std::hint::black_box(blip_assets::brawler::crowd_wav(true));
    let ms = t.elapsed().as_millis();
    assert!(ms < 400, "the crowd took {ms}ms to build");
}

// ---- player two joins by reaching for their own stick -------------------

#[test]
fn player_two_touching_anything_chooses_two_players() {
    // A cabinet has never asked player one to select two-player mode on
    // behalf of somebody standing next to them. The second player
    // reaches for their own stick and the machine works out the rest.
    for (what, m2) in [("their stick forward", press(false, true, false)),
                       ("their stick back", press(true, false, false)),
                       ("a button", press(false, false, true))] {
        let mut g = Game::new();
        assert_eq!(g.menu, 0, "the title does not start on 1 PLAYER");
        let cue = update_menus(&mut g, [NOTHING, m2]);
        assert_eq!(g.menu, 1, "player two used {what} and the cabinet ignored them");
        assert_eq!(cue, Some(Cue::Step), "claiming the second slot made no sound");
        assert_eq!(g.state, State::Title, "player two's first touch started the match");
    }
}

#[test]
fn player_two_cannot_give_the_second_slot_back() {
    // The place being given up in that direction is theirs, and nobody
    // else is asking for it — so their stick only ever selects two
    // players. Player one keeps the toggle.
    let mut g = Game::new();
    update_menus(&mut g, [NOTHING, press(false, true, false)]);
    assert_eq!(g.menu, 1);
    for _ in 0..4 {
        update_menus(&mut g, [NOTHING, press(false, true, false)]);
        assert_eq!(g.menu, 1, "player two waggled their stick out of the game");
    }
    // Player one still can.
    update_menus(&mut g, [press(false, true, false), NOTHING]);
    assert_eq!(g.menu, 0, "player one lost the toggle");
}

#[test]
fn either_player_starts_the_match_once_the_second_slot_is_claimed() {
    // Player two has as much right to the start button as the player
    // who put the coin in — but only once they are actually in.
    let mut g = Game::new();
    // Before they have joined, their fire is the join, not a start.
    update_menus(&mut g, [NOTHING, press(false, false, true)]);
    assert_eq!(g.state, State::Title, "player two's join also started the match");
    assert_eq!(g.menu, 1);
    // And now it starts it.
    let cue = update_menus(&mut g, [NOTHING, press(false, false, true)]);
    assert_eq!(g.state, State::Select, "player two could not start the match they joined");
    assert_eq!(g.mode, Mode::Versus);
    assert_eq!(cue, Some(Cue::Confirm));
}

#[test]
fn a_solo_player_is_never_dragged_into_two_player_mode() {
    // Player one working the title on their own has to be able to reach
    // 1 PLAYER and start there, with the second station sitting
    // untouched beside them.
    let mut g = Game::new();
    update_menus(&mut g, [press(false, true, false), NOTHING]);
    assert_eq!(g.menu, 1);
    update_menus(&mut g, [press(false, true, false), NOTHING]);
    assert_eq!(g.menu, 0);
    update_menus(&mut g, [press(false, false, true), NOTHING]);
    assert_eq!(g.state, State::Select);
    assert_eq!(g.mode, Mode::Solo, "a solo player was put into a versus match");
}

#[test]
#[ignore = "diagnostic"]
fn dump_high_kick_arms() {
    let mut f = at(0, 300.0, 1.0);
    f.act = Act::Attack;
    f.mv = MoveId::HighKick;
    let m = f.scaled(move_data(f.mv));
    for step in 0..16 {
        f.t = step as f32 / 15.0 * (m.startup + m.active + m.recovery) * F;
        let k = bones_of(&f);
        let rel = |v: draw::V| (v.0 - f.x, f.y - v.1);
        println!("t={:.3} hip={:?} neck={:?} lead={:?} rear={:?} elbow_rear={:?}", f.t,
            rel(k.hip), rel(k.neck), rel(k.hand_lead), rel(k.hand_rear), rel(k.elbow_rear));
    }
}

#[test]
#[ignore = "diagnostic"]
fn dump_ladders() {
    for pick in 0..FIGHTERS.len() {
        let names: Vec<&str> = Game { pick, ..Game::new() }.ladder().iter().map(|&w| FIGHTERS[w].name).collect();
        println!("{:8} -> {}", FIGHTERS[pick].name, names.join(", "));
    }
}

#[test]
fn the_billing_gives_way_to_the_bow_and_a_press_cuts_it_short() {
    let mut g = Game { pick: 0, ..Game::new() };
    g.start_match(0);
    g.announce();
    // A press on the frame it appears is the one that chose the fighter.
    update_vs(&mut g, F, true);
    assert_eq!(g.state, State::Vs, "the billing was skipped before it was seen");
    for _ in 0..60 { update_vs(&mut g, F, false); }
    update_vs(&mut g, F, true);
    assert_eq!(g.state, State::RoundIntro, "a press did not move it on");

    g.announce();
    run_phase(&mut g, |g, dt| update_vs(g, dt, false));
    assert_eq!(g.state, State::RoundIntro, "left alone, it never started the round");
    assert!(g.phase.active(), "the bow got no time");
}

#[test]
fn a_turtle_hits_its_own_size_softer_than_it_hits_a_grown_fighter() {
    let turtle = FIGHTERS.iter().position(|a| a.build == Build::Turtle).unwrap();
    let blow = |foe: usize| {
        let mut p = [at(turtle, 200.0, 1.0), at(foe, 240.0, -1.0)];
        face_off(&mut p);
        p[0].scaled(move_data(MoveId::HighKick)).damage
    };
    let (small, big) = (blow(turtle + 1), blow(0));
    assert!(small < big, "{small} against a turtle, {big} against a grown fighter");
}

#[test]
#[ignore = "diagnostic"]
fn dump_what_beats_a_blocker() {
    let _sim = simulating(1);
    for (pick, rung) in [(0, 0), (1, 0), (2, 0), (0, 6)] {
        let mut hits: std::collections::BTreeMap<String, (i32, i32)> = Default::default();
        let r = fight_round_seen(pick, rung, turtle, &mut |a, f, dmg| {
            if a == 1 { let e = hits.entry(format!("{:?}", f.mv)).or_default(); e.0 += 1; e.1 += dmg; }
        });
        println!("{} vs {} {:.1}s left {}: {:?}", FIGHTERS[pick].name,
            FIGHTERS[Game { pick, ..Game::new() }.ladder()[rung]].name, r.seconds, r.player_health, hits);
    }
}

#[test]
fn nobody_is_thrown_twice_running() {
    let mut p = [at(0, 300.0, 1.0), at(1, 330.0, -1.0)];
    face_off(&mut p);
    wind_to_active(&mut p[0], MoveId::Throw);
    {
        let [atk, def] = &mut p;
        let (dmg, _, _) = resolve_hit(atk, def, Input::default());
        assert!(dmg > 0, "the first throw missed");
        assert!(def.throw_rest > 0.0);
    }
    // Back on their feet, and grabbed again at once: it does not take.
    let hp = p[1].health;
    p[1].act = Act::Idle;
    p[1].x = 330.0;
    wind_to_active(&mut p[0], MoveId::Throw);
    let [atk, def] = &mut p;
    assert_eq!(resolve_hit(atk, def, Input::default()).0, 0, "thrown again straight away");
    assert_eq!(def.health, hp);
    // It wears off.
    for _ in 0..(THROW_REST / F) as usize + 2 { advance(def, F); }
    assert_eq!(def.throw_rest, 0.0);
}

// ---- the turtle call and the web -------------------------------------------

/// A turtle against RYUKA, the call already made; runs the helpers until
/// they have all gone, the opponent holding `hold`.
fn turtle_call(hold: fn(f32) -> Input) -> (Game, Vec<(i32, bool, bool)>) {
    let turtle = FIGHTERS.iter().position(|a| a.build == Build::Turtle).unwrap();
    let mut g = Game::new();
    g.p = [at(turtle, 200.0, 1.0), at(0, 420.0, -1.0)];
    face_off(&mut g.p);
    g.call_turtles(0);
    let mut landed = vec![];
    for _ in 0..600 {
        let holds = [Input::default(), hold(g.p[1].facing)];
        landed.extend(update_helpers(&mut g, F, holds));
        advance(&mut g.p[1], F);
        if g.helpers.iter().all(|h| h.is_none()) { break; }
    }
    (g, landed)
}

#[test]
fn a_turtles_first_special_of_the_round_calls_the_other_three() {
    let turtle = FIGHTERS.iter().position(|a| a.build == Build::Turtle).unwrap();
    let mut f = at(turtle, 200.0, 1.0);
    f.start_attack(MoveId::Special);
    assert!(f.calling && f.called);
    assert!(f.hit_box().is_none(), "the caller struck a blow of his own");
    // The second one is his own special again.
    f.act = Act::Idle;
    f.start_attack(MoveId::Special);
    assert!(!f.calling, "he called twice in a round");
    // Nobody else calls anybody.
    let mut other = at(0, 200.0, 1.0);
    other.start_attack(MoveId::Special);
    assert!(!other.calling);

    let (g, landed) = turtle_call(|_| Input::default());
    assert!(g.helpers.iter().all(|h| h.is_none()), "a helper never left the stage");
    assert_eq!(landed.len(), 3, "the three did not each land a blow: {landed:?}");
    assert!(landed.iter().all(|&(dmg, blocked, _)| dmg > 0 && !blocked));
    assert!(landed[2].2, "the last of them did not put the opponent down");
    assert!(g.p[1].health < FIGHTERS[0].health);
}

#[test]
fn the_helpers_are_three_different_turtles_and_never_the_caller() {
    for turtle in (0..FIGHTERS.len()).filter(|&w| FIGHTERS[w].build == Build::Turtle) {
        let mut g = Game::new();
        g.p[0] = at(turtle, 200.0, 1.0);
        g.call_turtles(0);
        let mut who: Vec<usize> = g.helpers.iter().map(|h| h.unwrap().f.who).collect();
        who.sort();
        who.dedup();
        assert_eq!(who.len(), 3);
        assert!(!who.contains(&turtle));
        assert!(who.iter().all(|&w| FIGHTERS[w].build == Build::Turtle));
    }
}

#[test]
fn one_low_guard_answers_the_whole_call() {
    let (g, landed) = turtle_call(|facing| back_input(facing, true));
    assert_eq!(landed.len(), 3);
    assert!(landed.iter().all(|&(dmg, blocked, _)| dmg == 0 && blocked), "{landed:?}");
    assert_eq!(g.p[1].health, FIGHTERS[0].health);
}

#[test]
fn a_web_holds_for_five_seconds_or_two_blows() {
    let mut f = at(0, 300.0, 1.0);
    assert!(f.web());
    assert_eq!(f.webbed, WEB_SECS);
    assert_eq!(WEB_SECS, 5.0);
    // One blow leaves it on; the second takes it off.
    f.hurt(5, 1.0);
    assert!(f.webbed > 0.0);
    f.hurt(5, 1.0);
    assert_eq!(f.webbed, 0.0, "two blows did not break the web");
    // And another does not hold straight away.
    assert!(!f.web(), "webbed again the moment it came off");
    for _ in 0..(WEB_REST / F) as usize + 2 { advance(&mut f, F); }
    assert!(f.web());
    // Left alone it lets go by itself.
    for _ in 0..(WEB_SECS / F) as usize + 2 { advance(&mut f, F); }
    assert_eq!(f.webbed, 0.0);
    assert!(f.web_rest > 0.0);
}

// ---- chain punches and the front leg ---------------------------------------

#[test]
fn punches_thrown_quickly_change_hands() {
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
fn a_walk_leaves_whichever_foot_it_stopped_on_in_front_and_the_kick_comes_off_it() {
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
