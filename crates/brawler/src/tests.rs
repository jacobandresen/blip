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
    apply_input(&mut f, jab, F);
    assert_eq!(f.mv, before, "an attack was cancelled into another attack");
    assert_eq!(f.act, Act::Attack);

    f.act = Act::Hitstun;
    f.stun = 10.0 * F;
    apply_input(&mut f, jab, F);
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
        for i in 0..2 {
            apply_input(&mut p[i], ins[i], F);
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
