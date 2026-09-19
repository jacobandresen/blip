//! The rules, pinned.
//!
//! A fighting game is mostly invisible: what a player feels as "that
//! should have hit" is a number in a table. These tests are where the
//! numbers are held to what the game claims about itself — that blocking
//! is a choice with a wrong answer, that slow moves are punishable, that
//! a knockdown is not a free second hit.

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
    let m = f.scaled(move_data(MoveId::Kick));
    f.start_attack(MoveId::Kick);

    f.t = m.startup * F * 0.5;
    assert!(f.hit_box().is_none(), "a kick hit during its startup");

    f.t = (m.startup + m.active * 0.5) * F;
    assert!(f.hit_box().is_some(), "a kick did not hit during its active frames");

    f.t = (m.startup + m.active + m.recovery * 0.5) * F;
    assert!(f.hit_box().is_none(), "a kick hit during its recovery");
}

#[test]
fn one_attack_lands_one_hit() {
    // Four active frames is four chances to overlap. Without the latch
    // a kick would do four times its listed damage.
    let mut atk = at(0, 200.0, 1.0);
    let mut def = at(1, 240.0, -1.0);
    wind_to_active(&mut atk, MoveId::Kick);
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
    let jab = move_data(MoveId::Jab);
    let kick = move_data(MoveId::Kick);
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
    for id in [MoveId::Jab, MoveId::Kick, MoveId::CrouchJab, MoveId::JumpPunch,
               MoveId::JumpKick, MoveId::Special] {
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
    jab.punch = true;
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
    wind_to_active(&mut atk, MoveId::Kick);
    let (dmg, _, _) = resolve_hit(&mut atk, &mut def, Input::default());
    assert_eq!(dmg, 0, "a waking fighter was hit through their invulnerability");
}

#[test]
fn health_stops_at_zero() {
    let mut atk = at(1, 200.0, 1.0); // Brutus, the hardest hitter
    let mut def = at(2, 236.0, -1.0);
    def.health = 3;
    wind_to_active(&mut atk, MoveId::Kick);
    resolve_hit(&mut atk, &mut def, Input::default());
    assert_eq!(def.health, 0, "health went negative and the bar would draw backwards");
}

// ---- archetypes ----------------------------------------------------------

#[test]
fn the_three_fighters_are_actually_different() {
    // Three palette swaps would not be three fighters. Each has to be
    // the best at something and the worst at something.
    let names: Vec<&str> = FIGHTERS.iter().map(|a| a.name).collect();
    assert_eq!(names.len(), 3);

    let fastest = FIGHTERS.iter().enumerate().max_by(|a, b| a.1.walk.total_cmp(&b.1.walk)).unwrap().0;
    let strongest = FIGHTERS.iter().enumerate().max_by(|a, b| a.1.power.total_cmp(&b.1.power)).unwrap().0;
    let toughest = FIGHTERS.iter().enumerate().max_by_key(|(_, a)| a.health).unwrap().0;
    assert_ne!(fastest, strongest, "the fastest fighter is also the strongest");
    assert_eq!(strongest, toughest, "the heavy should be the one who takes hits, too");

    // And the fast one pays for it.
    let fast = FIGHTERS[fastest];
    assert!(fast.power < 1.0 && fast.health < 100, "the fast fighter pays no price");
    assert!(fast.reach > 1.0, "the fast fighter has nothing to poke with");

    // Every fighter has their own special.
    let mut specials: Vec<Special> = FIGHTERS.iter().map(|a| a.special).collect();
    specials.dedup();
    assert_eq!(specials.len(), 3, "two fighters share a special move");
}

#[test]
fn power_and_reach_scale_the_shared_move_table() {
    // The archetype has to reach into the numbers, or "power 1.35" is
    // decoration.
    let brutus = at(1, 0.0, 1.0);
    let kestrel = at(2, 0.0, 1.0);
    let base = move_data(MoveId::Kick);
    assert!(brutus.scaled(base).damage > base.damage);
    assert!(kestrel.scaled(base).damage < base.damage);
    assert!(kestrel.scaled(base).reach > brutus.scaled(base).reach,
        "the lighter fighter should out-range the heavy one");
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
fn the_ladder_is_the_two_fighters_you_did_not_pick() {
    for pick in 0..FIGHTERS.len() {
        let mut g = Game::new();
        g.pick = pick;
        let ladder = g.ladder();
        assert_ne!(ladder[0], pick);
        assert_ne!(ladder[1], pick);
        assert_ne!(ladder[0], ladder[1], "the same opponent twice");
    }
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
            || inp.punch || inp.kick || inp.special;
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
    // If a special were just another poke, the fighters would differ
    // only in colour. Each one has to break a rule the normals obey:
    // the bolt leaves the fighter behind, the rush crosses the stage,
    // the talon kick goes airborne.
    let base = move_data(MoveId::Special);
    assert!(base.knockdown, "a special that does not knock down is just a slow kick");
    assert!(base.damage > move_data(MoveId::Kick).damage);
    assert!(base.recovery > move_data(MoveId::Kick).recovery,
        "a special has to be punishable or it is the only move anyone would throw");
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
        let close = (p[1].x - p[0].x).abs() <= THROW_RANGE;
        for i in 0..2 {
            apply_input(&mut p[i], ins[i], close, F);
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
        if frame % 24 < 2 { a.punch = true; }
        if frame % 30 < 2 { b.kick = true; }
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
        if frame % 40 < 2 { a.kick = true; } // sweep on a loop
        [a, Input::default()]
    };
    // Inside sweep range from the first frame: this test is about the
    // block, not about walking in.
    let open = spar(0, 1, 74.0, 60 * 10, &mut sweeping);
    let damage_taken_standing = FIGHTERS[open[1].who].health - open[1].health;

    let mut sweeping_vs_block = |frame: usize, _p: &[Fighter; 2]| {
        let mut a = Input::default();
        a.down = true;
        if frame % 40 < 2 { a.kick = true; }
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
        a.punch = frame % 3 == 0;
        a.kick = frame % 5 == 0;
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

    assert_eq!(g.state, State::RoundIntro, "the next match did not start");
    assert_ne!(g.p[1].who, first_foe, "the same opponent came back for a second match");
    assert_ne!(g.stage, first_stage, "the second match is in the same place as the first");
    assert_eq!(g.p[0].rounds, 0, "round wins carried over into the next opponent");
    assert_eq!(g.p[1].rounds, 0);
    assert_eq!(g.p[0].health, FIGHTERS[g.p[0].who].health, "the next match started hurt");
}

#[test]
fn beating_both_opponents_wins_the_game() {
    let mut g = Game::new();
    g.pick = 1;
    g.start_match(1); // the last rung
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
    punch.punch = true;
    // Press with a few frames of recovery still to run.
    f.t = total - 4.0 * F;
    apply_input(&mut f, punch, false, F);
    assert_eq!(f.mv, MoveId::Sweep, "the press interrupted the sweep");
    assert_eq!(f.buffered, Some(MoveId::Jab), "the press was thrown away");

    // Let the sweep finish, then hold nothing at all.
    for _ in 0..5 { advance(&mut f, F); }
    apply_input(&mut f, Input::default(), false, F);
    assert_eq!(f.act, Act::Attack, "the buffered punch never came out");
    assert_eq!(f.mv, MoveId::Jab);
}

#[test]
fn a_buffered_attack_is_forgotten_if_it_waits_too_long() {
    // A buffer that never expires fires attacks the player asked for
    // seconds ago, which is its own kind of not listening.
    let mut f = at(0, 200.0, 1.0);
    f.act = Act::Hitstun;
    f.stun = 1.0;
    let mut punch = Input::default();
    punch.punch = true;
    apply_input(&mut f, punch, false, F);
    assert!(f.buffered.is_some());

    for _ in 0..20 { apply_input(&mut f, Input::default(), false, F); }
    assert_eq!(f.buffered, None, "the buffer held an input far past its welcome");
}

#[test]
fn a_buffered_move_remembers_the_stance_it_was_asked_for_in() {
    // Pressing down+kick during recovery is a sweep, and must still be
    // a sweep when it comes out — resolving it later against whatever
    // the stick happens to be doing would hand the player a move they
    // did not ask for.
    let mut f = at(0, 200.0, 1.0);
    f.start_attack(MoveId::Kick);
    let mut low = Input::default();
    low.down = true;
    low.kick = true;
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

/// A whole round, fought headlessly: the CPU against a scripted player,
/// reported as numbers rather than as a feeling.
///
/// Tuning a fighting game by playing it is how you tune it for the one
/// person who has played it a hundred times. This runs the real CPU
/// against three deliberately crude opponents — a rusher, a turtle, a
/// poker — because how a game treats players who are *not* good at it is
/// most of whether it is fun.
struct RoundStats {
    seconds: f32,
    player_health: i32,
    cpu_health: i32,
    player_hits: i32,
    cpu_hits: i32,
    timed_out: bool,
}

/// Every test that runs the CPU has to take this first.
///
/// The CPU's choices come out of a random number generator that is
/// global to the process, and `cargo test` runs tests in parallel — so
/// two simulations running at once draw from the same stream, in an
/// order that depends on thread scheduling. The balance tests were
/// passing and failing by coincidence, which is worse than not having
/// them: a red run told you nothing and a green one told you less.
///
/// Holding this lock serialises them, and seeding inside it makes each
/// one start from the same place every time.
static SIMULATION: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn simulating(seed: u64) -> std::sync::MutexGuard<'static, ()> {
    // A poisoned lock just means some other simulation test already
    // failed its assertion; this one can still run.
    let guard = SIMULATION.lock().unwrap_or_else(|e| e.into_inner());
    blip::rand_seed(seed);
    guard
}

fn fight_round(pick: usize, foe_index: usize, style: fn(usize, &[Fighter; 2]) -> Input) -> RoundStats {
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
            let close = (g.p[1].x - g.p[0].x).abs() <= THROW_RANGE;
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
    i.punch = frame % 8 < 2;
    i
}

/// Holds back and blocks, and does nothing else.
fn turtle(_frame: usize, p: &[Fighter; 2]) -> Input {
    let mut i = Input::default();
    if p[1].x > p[0].x { i.left = true; } else { i.right = true; }
    i.down = true;
    i
}

/// Keeps its distance, blocks low, punishes what it sees miss, and pokes
/// from the edge of its range — roughly what the game is asking for.
///
/// It crouch-blocks rather than standing, because standing into sweeps
/// is a blind spot the game is designed to punish, and a "good player"
/// model with that hole in it measures the hole rather than the game.
fn poker(frame: usize, p: &[Fighter; 2]) -> Input {
    let mut i = Input::default();
    let dist = (p[1].x - p[0].x).abs();
    let toward = p[1].x > p[0].x;
    let (fwd, back) = if toward { (&mut i.right as *mut bool, &mut i.left as *mut bool) }
                      else { (&mut i.left as *mut bool, &mut i.right as *mut bool) };
    // SAFETY-free alternative would need two branches everywhere; these
    // are two distinct fields of a local struct.
    let (fwd, back) = unsafe { (&mut *fwd, &mut *back) };

    let my_kick = attack_range(&p[0], MoveId::Kick);
    let whiffed = p[1].act == Act::Attack && p[1].hit_done;

    if whiffed && dist < my_kick {
        i.kick = true;                      // punish
    } else if dist > my_kick * 1.15 {
        *fwd = true;                        // close the gap
    } else if frame % 30 < 3 {
        i.kick = true;                      // poke from the edge
    } else if frame % 30 < 6 {
        i.down = true;
        i.kick = true;                      // and mix in a low
    } else {
        *back = true;                       // otherwise hold guard, low
        i.down = true;
    }
    i
}

#[test]
fn every_round_is_decided_by_something_that_happened() {
    // A round may run the clock out — a defensive fight is a legitimate
    // fight, and the count is meant to be pressure. What it may never
    // do is end with both fighters largely untouched, because that is
    // the signature of a stalemate the rules cannot break: two players
    // holding back at each other until the arithmetic picks one.
    let _sim = simulating(0xB4A17E);
    let styles: [(&str, fn(usize, &[Fighter; 2]) -> Input); 3] =
        [("rusher", rusher), ("poker", poker), ("turtle", turtle)];
    let mut stalemates = vec![];
    let mut decided_fast = 0;
    let mut rounds = 0;

    for pick in 0..3 {
        for foe in 0..2 {
            for (name, style) in styles {
                let r = fight_round(pick, foe, style);
                rounds += 1;
                if !r.timed_out { decided_fast += 1; }
                let cpu_who = Game { pick, ..Game::new() }.ladder()[foe];
                let both_healthy = r.player_health as f32 > FIGHTERS[pick].health as f32 * 0.55
                    && r.cpu_health as f32 > FIGHTERS[cpu_who].health as f32 * 0.55;
                if r.timed_out && both_healthy {
                    stalemates.push(format!("{} vs opponent {foe} [{name}]: {} / {}",
                        FIGHTERS[pick].name, r.player_health, r.cpu_health));
                }
            }
        }
    }

    assert!(stalemates.is_empty(),
        "rounds ended with nothing having happened: {stalemates:?}");
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
    for pick in 0..3 {
        for foe in 0..2 {
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
    let mut survived = 0;
    let mut won = 0;
    for pick in 0..3 {
        for foe in 0..2 {
            let r = fight_round(pick, foe, turtle);
            if r.player_health > 0 { survived += 1; }
            if r.cpu_health <= 0 { won += 1; }
        }
    }
    assert_eq!(won, 0, "a fighter who never attacked won {won} rounds");
    assert!(survived >= 2, "crouch-blocking survived only {survived} of 6 rounds — blocking is not working");
}

#[test]
#[ignore]
fn diagnose_matchups() {
    for pick in 0..3 {
        for foe in 0..2 {
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
                let close = (g.p[1].x - g.p[0].x).abs() <= THROW_RANGE;
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
    wind_to_active(&mut atk, MoveId::Jab);
    let (dmg, _, _) = resolve_hit(&mut atk, &mut def, Input::default());
    assert!(dmg > 0);
    assert!(atk.can_cancel(), "a landed jab opened no follow-up window");

    let mut kick = Input::default();
    kick.kick = true;
    apply_input(&mut atk, kick, false, F);
    assert_eq!(atk.mv, MoveId::Kick, "the follow-up never came out");
    assert!(atk.chained, "the follow-up was not marked as a chain");
}

#[test]
fn a_blocked_jab_buys_nothing() {
    // Pressing buttons into a guard must not be a combo. If it were,
    // blocking would be the losing option at every range.
    let mut atk = at(0, 280.0, 1.0);
    let mut def = at(1, 320.0, -1.0);
    wind_to_active(&mut atk, MoveId::Jab);
    let mut back = Input::default();
    back.right = true; // away from the attacker on the left
    let (_, blocked, _) = resolve_hit(&mut atk, &mut def, back);
    assert!(blocked);
    assert!(!atk.can_cancel(), "a blocked jab opened a follow-up window");
}

#[test]
fn a_chain_cannot_chain_again() {
    // One cancel per attack. Jab into jab into jab comes out faster than
    // the hitstun it causes, which is an infinite, which is not a game.
    let mut atk = at(0, 280.0, 1.0);
    let mut def = at(1, 320.0, -1.0);
    wind_to_active(&mut atk, MoveId::Jab);
    resolve_hit(&mut atk, &mut def, Input::default());
    let mut jab = Input::default();
    jab.punch = true;
    apply_input(&mut atk, jab, false, F);          // the one cancel
    assert!(atk.chained);

    def.hit_done = false;
    wind_to_active(&mut atk, MoveId::Jab);
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
    wind_to_active(&mut atk, MoveId::Kick);
    let (first, _, _) = resolve_hit(&mut atk, &mut def, Input::default());
    let mut atk2 = at(0, 280.0, 1.0);
    wind_to_active(&mut atk2, MoveId::Kick);
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
    punch.punch = true;
    assert_eq!(pressed_move(&f, punch, true), Some(MoveId::Throw));
    assert_eq!(pressed_move(&f, punch, false), Some(MoveId::Jab));
    // And crouching punch is never a throw — you cannot throw from down.
    punch.down = true;
    assert_eq!(pressed_move(&f, punch, true), Some(MoveId::CrouchJab));
}

#[test]
fn a_whiffed_throw_is_the_worst_thing_you_can_do() {
    // It has to be, or throwing would simply be correct at close range.
    let throw = move_data(MoveId::Throw);
    for id in [MoveId::Jab, MoveId::Kick, MoveId::CrouchJab] {
        assert!(throw.recovery > move_data(id).recovery,
            "a missed throw recovers faster than a {id:?}");
    }
}

// ---- anatomy -------------------------------------------------------------
//
// The fighters are drawn from a skeleton, and a skeleton can be checked.
// These walk every pose of every fighter across the whole of its
// animation and assert the things a body cannot do: bones do not change
// length, hinges do not fold past shut or bend backwards, and nothing
// ends up underneath the floor it is standing on.
//
// This is the difference between "the kick looked wrong in that
// screenshot" and "the kick is wrong, here is which joint, at which
// frame". A drawing bug you can only see is a drawing bug you will
// reintroduce.

/// Every pose the game can put a fighter in, as (label, fighter state).
fn every_pose() -> Vec<(String, Fighter)> {
    let mut out = Vec::new();
    let moves = [MoveId::Jab, MoveId::LowKick, MoveId::Kick, MoveId::HighKick,
                 MoveId::CrouchJab, MoveId::Sweep, MoveId::JumpPunch, MoveId::JumpKick,
                 MoveId::Special, MoveId::Throw];
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
            }
        }
        for mv in moves {
            let air = matches!(mv, MoveId::JumpPunch | MoveId::JumpKick);
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

fn bones_of(f: &Fighter) -> draw::Skeleton {
    let rig = draw::Rig::upright(f.x, f.y, f.facing, f.facing);
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
    // The rule the whole drawing is built on: the limb a player can see
    // coming is the thing that will hit them. So on every active frame
    // of every attack, the striking hand or foot has to be *inside* its
    // own hitbox, and not far from the end of it.
    //
    // Two ways to break this, and both have happened here. Letting the
    // bones stretch to reach the box put the fighter on a support leg a
    // fifth longer than the one it kicked with. Refusing to stretch
    // them left the foot short of the box, so an attack landed from
    // further away than it looked. What is left is a body that reaches
    // as far as a body reaches, and move ranges that sit inside that.
    let mut bad = vec![];
    for who in 0..FIGHTERS.len() {
        for mv in [MoveId::Jab, MoveId::LowKick, MoveId::Kick, MoveId::HighKick,
                   MoveId::CrouchJab, MoveId::Sweep, MoveId::Throw] {
            let mut f = Fighter::new(who, 300.0, 1.0);
            f.act = Act::Attack;
            f.mv = mv;
            let m = f.scaled(move_data(mv));
            // Sample the active window, where the move can actually hit.
            for step in 0..5 {
                f.t = (m.startup + m.active * step as f32 / 4.0) * F;
                let Some((hx, _, hw, _)) = f.hit_box() else { continue };
                let k = bones_of(&f);
                let leg = matches!(mv, MoveId::LowKick | MoveId::Kick | MoveId::HighKick
                    | MoveId::Sweep);
                // The tip is the end of the foot or the front of the
                // fist, not the joint behind it.
                let tip = if leg {
                    let (dx, dy) = (k.ankle_lead.0 - k.knee_lead.0, k.ankle_lead.1 - k.knee_lead.1);
                    let d = (dx * dx + dy * dy).sqrt().max(0.001);
                    k.ankle_lead.0 + dx / d * 11.0
                } else {
                    k.hand_lead.0 + 5.0
                };
                let far = hx + hw;
                let short = far - tip;
                if short > 14.0 {
                    bad.push(format!("{} {:?}: reaches {:.0}px short of its own hitbox",
                        FIGHTERS[who].name, mv, short));
                    break;
                }
                if tip > far + 6.0 {
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
    for pick in 0..3 {
        for foe in 0..2 {
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
    // Weight over the feet. A body whose mass is outside the ground it
    // is standing on is a body falling over, and a pose that reads as
    // falling over reads as wrong even to someone who could not say
    // why.
    //
    // Attacks get a wider allowance, because leaning past your own feet
    // is exactly what throwing a punch is — you are falling forward and
    // the recovery frames are you catching yourself. What is not
    // allowed is standing, walking or guarding off balance.
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
    // A hinge has a direction as well as a limit. The length and range
    // tests above are both satisfied by a leg with the knee on the
    // wrong side of it — which is a leg on backwards, and the most
    // obviously inhuman thing a drawing of a person can do.
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
    }
    assert!(bad.is_empty(), "joints bending the wrong way:\n  {}", bad.join("\n  "));
}

#[test]
fn nothing_teleports_between_one_frame_and_the_next() {
    // The complaint that is hardest to pin down is "the animation looks
    // strange", and most of the time what is behind it is not a bad
    // pose but a missing one: an action changes, the drawing follows on
    // the same frame, and a limb crosses half the screen between two
    // pictures. The eye reads that as a different fighter, not as the
    // same fighter moving.
    //
    // So: drive one through a realistic run of actions at the real
    // frame rate and watch every joint. Nothing may move further in one
    // frame than a body part can.
    let _sim = simulating(0x5EED05);
    let script: [(Act, MoveId, f32); 9] = [
        (Act::Idle, MoveId::Jab, 0.20),
        (Act::Attack, MoveId::Kick, 0.55),
        (Act::Idle, MoveId::Jab, 0.10),
        (Act::Attack, MoveId::HighKick, 0.70),
        (Act::Block, MoveId::Jab, 0.20),
        (Act::Attack, MoveId::Sweep, 0.60),
        (Act::Hitstun, MoveId::Jab, 0.30),
        (Act::Knockdown, MoveId::Jab, 1.20),
        (Act::Victory, MoveId::Jab, 0.80),
    ];
    let mut worst = (0.0f32, String::new());
    let mut over: Vec<String> = vec![];
    for who in 0..FIGHTERS.len() {
        let mut f = Fighter::new(who, 300.0, 1.0);
        let mut last: Option<[(&'static str, draw::V); 6]> = None;
        for (act, mv, secs) in script {
            f.act = act;
            f.mv = mv;
            f.t = 0.0;
            f.stun = secs;
            let frames = (secs / F) as i32;
            let mut held = 0.0f32;
            for _ in 0..frames {
                advance(&mut f, F);
                held += F;
                // advance() ends an action when its own clock runs out
                // and resets the timer with it. Hold both, or the run
                // restarts the action mid-script and the "teleport"
                // being measured is the test's own doing.
                if f.act != act {
                    f.act = act;
                    f.t = held;
                    f.shown = act;
                    f.blend = 0.0;
                }
                let k = bones_of(&f);
                let now = [("hand-lead", k.hand_lead), ("hand-rear", k.hand_rear),
                           ("ankle-lead", k.ankle_lead), ("ankle-rear", k.ankle_rear),
                           ("head", k.head), ("hip", k.hip)];
                if let Some(prev) = last {
                    // Being hit is allowed to snap: a guard knocked
                    // aside is the fastest thing that happens to a
                    // body, and softening it would cost the hit its
                    // impact. Everything else has to travel.
                    let limit = match act {
                        Act::Hitstun => 70.0,
                        // The peak of a kick's whip, where the shin is
                        // travelling fastest and the motion smear is
                        // drawn behind it.
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
    // A foot at the fast end of a kick covers about fifteen pixels a
    // frame; thirty is not a limb moving, it is a cut. This is the test
    // that caught the roundhouse arriving fifty pixels in a single
    // frame — the leg covering the last fifth of the kick with no
    // picture in between, on the frame it lands.
    assert!(over.is_empty(), "limbs teleporting:\n  {}\n(worst overall: {})",
        over.join("\n  "), worst.1);
}
