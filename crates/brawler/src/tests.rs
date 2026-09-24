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
    let jab = move_data(MoveId::Jab);
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
    for id in [MoveId::Jab, MoveId::LowKick, MoveId::HighKick, MoveId::CrouchJab,
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
    let base = move_data(MoveId::HighKick);
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
            || inp.punch || inp.any_kick() || inp.special;
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
    // It beats the pokes for damage — it is not what you throw to
    // win an exchange, it is what you throw to end one. The high kick
    // out-damages it and is meant to: that is a fourteen-frame overhead
    // you have to read, where this comes out in eleven and knocks down.
    assert!(base.damage > move_data(MoveId::LowKick).damage);
    // And it costs more to miss than anything else in the table, which
    // is the only thing keeping it from being the whole game.
    for id in [MoveId::Jab, MoveId::LowKick, MoveId::HighKick, MoveId::CrouchJab,
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
        a.punch = frame % 3 == 0;
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

    // The safe poke is the yardstick, and since the middle kick was
    // dropped that is the low one. Measured out on a high kick — a
    // fourteen-frame overhead with twenty-two frames of recovery — the
    // poker threw its most punishable move as its jab and went from
    // winning a round 100-0 to losing it 0-90.
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
        // Otherwise hold guard — and *change* which guard. Held low
        // for the whole round it ate every high kick: with no middle
        // kick left there is nothing a single guard covers, which is
        // the entire point of dropping it.
        *back = true;
        i.down = frame % 96 < 52;
    }
    i
}

#[test]
fn every_round_is_decided_by_something_that_happened() {
    // A round may run the clock out — a defensive fight is a legitimate
    // fight, and the count is meant to be pressure. What it may never
    // do is *often* end with both fighters largely untouched, because
    // that is the signature of a stalemate the rules cannot break: two
    // players holding back at each other until the arithmetic picks one.
    //
    // Sampled across several seeds and asserted as a rate. One seed is
    // a coin flip: every change to how a fighter moves reshuffles a
    // chaotic simulation, and a single round drifting over the line
    // said nothing about whether the game had got worse. Measured over
    // 180 rounds the rate held at 2-3% either side of a change that
    // moved one seeded round from a draw to a loss.
    let styles: [(&str, fn(usize, &[Fighter; 2]) -> Input); 3] =
        [("rusher", rusher), ("poker", poker), ("turtle", turtle)];
    let mut stalemates = vec![];
    let mut decided_fast = 0;
    let mut rounds = 0;

    for s in 0..4u64 {
        let _sim = simulating(0xB4A17E + s * 0x9E37);
        for pick in 0..3 {
            for foe in 0..2 {
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
    kick.kick_high = true;
    apply_input(&mut atk, kick, false, F);
    assert_eq!(atk.mv, MoveId::HighKick, "the follow-up never came out");
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
    for id in [MoveId::Jab, MoveId::LowKick, MoveId::CrouchJab] {
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
    let moves = [MoveId::Jab, MoveId::LowKick, MoveId::HighKick,
                 MoveId::CrouchJab, MoveId::Sweep, MoveId::JumpPunch, MoveId::JumpKick,
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
        for mv in [MoveId::Jab, MoveId::LowKick, MoveId::HighKick,
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
                let leg = matches!(mv, MoveId::LowKick | MoveId::HighKick
                    | MoveId::Sweep);
                // The tip is the end of the foot or the front of the
                // fist, not the joint behind it.
                // Both scale with the fighter, because the drawn foot
                // and the drawn fist do.
                let bulk = FIGHTERS[who].bulk;
                let tip = if leg {
                    let (dx, dy) = (k.ankle_lead.0 - k.knee_lead.0, k.ankle_lead.1 - k.knee_lead.1);
                    let d = (dx * dx + dy * dy).sqrt().max(0.001);
                    k.ankle_lead.0 + dx / d * 12.6 * bulk
                } else {
                    k.hand_lead.0 + 6.4 * bulk
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
    let script: [(Act, MoveId, f32); 11] = [
        (Act::Idle, MoveId::Jab, 0.20),
        (Act::Attack, MoveId::HighKick, 0.55),
        (Act::Idle, MoveId::Jab, 0.10),
        (Act::Attack, MoveId::HighKick, 0.70),
        (Act::Block, MoveId::Jab, 0.20),
        (Act::Attack, MoveId::Sweep, 0.60),
        // A jump comes out of a neutral stance, because that is the
        // only thing it can come out of: you cannot jump out of a
        // sweep's recovery, and a script that pretends you can is
        // measuring a handover the game never performs.
        (Act::Idle, MoveId::Jab, 0.12),
        (Act::Air, MoveId::Jab, 0.80),
        (Act::Hitstun, MoveId::Jab, 0.30),
        (Act::Knockdown, MoveId::Jab, 1.20),
        (Act::Victory, MoveId::Jab, 0.80),
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
                // advance() ends an action when its own clock runs out
                // and resets the timer with it. Hold both, or the run
                // restarts the action mid-script and the "teleport"
                // being measured is the test's own doing.
                // A jump is the one leg of the script that is allowed
                // to end on its own: it finishes by landing, and the
                // landing — the touchdown, the handover to standing and
                // the give in the knees after it — is exactly the part
                // worth watching.
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



#[test]
fn a_waiting_fighter_has_their_knees_bent() {
    // Straight knees read as standing in a queue. It went wrong in a
    // way nothing else here could see: the hip sat a leg's length from
    // the ankles, the solver clamped both legs straight because that
    // was the closest it could legally get, and every length, hinge
    // and balance rule still passed.
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
    // A two-bone limb stops being posable before it stops being legal.
    // As the target nears the fold limit the joint stops answering to
    // it and swings out along the bias instead, and a pixel at the hand
    // throws it a long way. The rear guard hand, parked at 1.07x the
    // minimum, put the far elbow through the fighter's own chest.
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
    // reads as a plank bolted to the shoulder.
    let mut bad = vec![];
    for (label, f) in every_pose() {
        if f.act == Act::Knockdown { continue; }
        let k = bones_of(&f);
        for (n, sh, el, hd) in [("lead", k.sh_lead, k.elbow_lead, k.hand_lead),
                                ("rear", k.sh_rear, k.elbow_rear, k.hand_rear)] {
            let back = (sh.0 - el.0) * f.facing;
            let hand = ((sh.0 - hd.0) * f.facing).max(0.0);
            if back - hand > 19.0 {
                bad.push(format!("{label}: {n} elbow {:.0}px behind its shoulder, \
                    hand only {hand:.0}px", back));
            }
        }
    }
    assert!(bad.is_empty(), "elbows winging out behind the back:\n  {}", bad.join("\n  "));
}

#[test]
fn a_fighter_has_two_arms_two_legs_and_one_torso() {
    // Near and far limbs have to be far enough apart to be counted, and
    // close enough not to read as someone else's. Both failures have
    // happened: a guard that put the two fists in one place, and a fist
    // parked beside the skull that read as a second head.
    let mut bad = vec![];
    for (label, f) in every_pose() {
        // A body rolling up off the floor passes its own limbs across
        // each other, which is what getting up is.
        if f.act == Act::Knockdown { continue; }
        let k = bones_of(&f);
        let gap = |a: draw::V, b: draw::V| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        if gap(k.hand_lead, k.hand_rear) < 6.0 {
            bad.push(format!("{label}: the two fists are in the same place"));
        }
        if gap(k.ankle_lead, k.ankle_rear) < 6.0 {
            bad.push(format!("{label}: the two feet are in the same place"));
        }
        // Nothing but the head belongs on the head. A hand over the
        // face deletes the one part a player has to find.
        let head_r = 9.8 + 2.2 * (FIGHTERS[f.who].bulk - 1.0);
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
        let k = bones_of(&f);
        let top = f.y - f.height();
        let head_r = 9.8 + 2.2 * (FIGHTERS[f.who].bulk - 1.0);
        if k.head.1 - head_r < top - 6.0 {
            bad.push(format!("{label}: the head is {:.0}px above the hurtbox",
                top - (k.head.1 - head_r)));
        }
    }
    assert!(bad.is_empty(), "bodies outside their own box:\n  {}", bad.join("\n  "));
}

#[test]
fn a_fighter_who_covers_ground_takes_steps() {
    // Both directions, because they are drawn by different code.
    // Retreating is a block — holding away is the block and the
    // back-step at once — and the block pose had both feet pinned, so
    // a fighter giving ground slid backwards without moving a foot.
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
    for mv in [MoveId::Jab, MoveId::LowKick, MoveId::HighKick, MoveId::Sweep, MoveId::JumpKick,
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
    for pick in 0..3 {
        for foe in 0..2 {
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
    // Pressing attack and getting nothing is the worst thing a
    // fighting game can do, and a jump used to do it twice: the flying
    // kick needed up and kick on the same frame, and a kick pressed in
    // the last five frames of a jump had a startup longer than the
    // fall left, so the feet touched first and landing threw it away.
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
    // A move that is minus on hit is a move nobody should ever throw.
    // This one was nineteen frames minus: it connects in the air and
    // pays its recovery on the ground, and the flight in between was
    // time the defender spent recovering from the hit.
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
    // The answer to it is to hit them out of it, so that has to work.
    // The no-slide check is a guard rather than a bug it caught: the
    // momentum is cleared on knockdown, and grounded fighters skip the
    // physics step anyway, so it takes both going wrong to slide.
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
    // The grounded version of this rule has been enforced since the
    // drawing was written; the air moves were never covered by it, so
    // the flying kick's foot had never once been checked against the
    // box that does its damage. Flown for real rather than posed at a
    // guessed height, because the pose is read off vertical speed and
    // a still frame is not the move.
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
                let rig = draw::Rig::upright(f.x, f.y, f.facing, f.facing);
                let k = draw::skeleton(rig, &draw::pose_of(0.37, &f, 0));
                let bulk = FIGHTERS[who].bulk;
                let (joint, tip) = if leg {
                    let (dx, dy) = (k.ankle_lead.0 - k.knee_lead.0,
                                    k.ankle_lead.1 - k.knee_lead.1);
                    let d = (dx * dx + dy * dy).sqrt().max(0.001);
                    (k.ankle_lead, k.ankle_lead.0 + dx / d * 12.6 * bulk)
                } else {
                    (k.hand_lead, k.hand_lead.0 + 6.4 * bulk)
                };
                let far = hx + hw;
                if far - tip > 15.0 {
                    bad.push(format!("{} {mv:?}: reaches {:.0}px short of its hitbox",
                        FIGHTERS[who].name, far - tip));
                    break;
                }
                if tip > far + 8.0 {
                    bad.push(format!("{} {mv:?}: drawn {:.0}px past its hitbox",
                        FIGHTERS[who].name, tip - far));
                    break;
                }
                // And the right height: a box the limb is nowhere near
                // is a box that hits things the picture never touched.
                if joint.1 < hy - 26.0 || joint.1 > hy + hh + 26.0 {
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
    // The round ends by assigning Victory and Defeat outright, so a
    // fighter who landed the killing blow mid-flight is airborne when
    // it happens. Nothing advanced them afterwards, so they hung there
    // for the whole two seconds — and the victory pose, which is read
    // off the action timer, stayed frozen on its first frame.
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
    // The hurtbox exists to be the box the fighter is drawn in. A
    // downed one is flat on the boards, and was carrying a full
    // standing box: a flying kick passing over them connected with the
    // air above a prone body.
    let mut d = at(0, 300.0, -1.0);
    d.act = Act::Knockdown;
    d.t = 0.4;
    let (_, by, _, bh) = d.hurt_box();
    assert!(bh <= 40.0, "a fighter on their back is {bh:.0}px tall");
    // Nothing drawn on them pokes far out of it.
    let rig = draw::Rig::upright(d.x, d.y, d.facing, -d.facing);
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
fn no_guard_height_covers_every_kick() {
    // The reason the middle kick was dropped. It was a mid — stopped
    // by either guard — so against the kicks a player could pick a
    // stance and hold it all round. What is left has to force the
    // choice: for each guard there is a kick it does not stop, and
    // there is no kick that both guards stop.
    let kicks = [MoveId::LowKick, MoveId::HighKick, MoveId::Sweep,
                 MoveId::JumpKick, MoveId::FlyingKick];
    for crouch in [false, true] {
        assert!(kicks.iter().any(|&k| !blocks(move_data(k).level, crouch)),
            "a fighter holding {} blocks every kick in the game",
            if crouch { "down-back" } else { "back" });
    }
    for k in kicks {
        let m = move_data(k);
        assert!(!(blocks(m.level, false) && blocks(m.level, true)),
            "{k:?} is stopped by either guard — that is the move that was dropped");
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
    // A four-bar loop is seven seconds against a forty-five second
    // round: you hear it round six times, and by the third you are
    // listening to the seam instead of the fight. A theme longer than
    // the round has no seam to hear.
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
    // The whole reason the clusters were chosen: two people at one
    // keyboard, and a key that moves both fighters makes the mode
    // unplayable rather than merely awkward.
    //
    // Read off the pads themselves, not a list copied beside them — a
    // test that restates the thing it is checking only ever checks the
    // typing.
    let keys = |p: &Pad| {
        let mut v: Vec<String> = Vec::new();
        for set in [p.up, p.down, p.left, p.right, p.punch, p.kick_low, p.kick_high] {
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
        assert_eq!(before, 7, "{name} should have seven controls, has {before}");
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

// ---- two-player play tests -----------------------------------------------
//
// These drive the real flow rather than a mirror of it: `update_menus`
// is the same function main() calls, and the fight below is the same
// `apply_input`/`advance`/`resolve_hit` the game runs. Only the
// keyboard and the speaker are replaced.

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
        match g.state {
            State::RoundIntro => { if g.phase.tick(F) { g.state = State::Fight; } }
            State::RoundEnd => update_round_end(g, F),
            State::MatchEnd => { update_match_end(g, F); }
            State::Over | State::Won => break,
            State::Fight => {
                if g.hitstop > 0.0 { g.hitstop -= F; continue; }
                g.clock -= F;
                let ins = [style(0, frames, &g.p), style(1, frames, &g.p)];
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
                for i in 0..2 {
                    let (a, d) = if i == 0 { (0, 1) } else { (1, 0) };
                    let (dmg, _, _) = {
                        let mut atk = g.p[a];
                        let mut def = g.p[d];
                        let r = resolve_hit(&mut atk, &mut def, ins[d]);
                        g.p[a] = atk;
                        g.p[d] = def;
                        r
                    };
                    let _ = dmg;
                }
                // Round over?
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
            _ => break,
        }
    }
    (frames, [g.p[0].rounds, g.p[1].rounds])
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
        i.punch = true;
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
    assert_eq!(g.state, State::RoundIntro, "locking both did not start the match");
    assert_ne!(g.p[0].who, g.p[1].who, "the same fighter reached the ring twice");

    let (frames, rounds) = versus_bout(&mut g, brawlers);
    assert!(frames < 60 * 240, "the match never finished");
    assert!(rounds[0] >= ROUNDS_TO_WIN || rounds[1] >= ROUNDS_TO_WIN,
        "nobody took the match: {rounds:?}");
    assert_eq!(g.state, State::Over, "a versus match did not end at the result screen");
}

#[test]
fn playtest_2_player_two_is_moved_off_a_fighter_that_gets_taken() {
    // Both cursors start on the same fighter and player one takes it.
    // Player two was legal a moment ago and is not any more, so they
    // are moved to a free one rather than left pressing a button that
    // does nothing.
    let mut g = at_versus_select();
    g.pick = 0;
    g.pick2 = 0;
    lock(&mut g, 0);
    assert!(g.locked[0]);
    assert_ne!(g.pick2, g.pick, "player two was left stranded on a taken fighter");
    lock(&mut g, 1);
    assert!(g.locked[1], "player two could not lock the fighter they were moved to");
    assert_ne!(g.p[0].who, g.p[1].who, "the same fighter reached the ring twice");
    assert_eq!(g.state, State::RoundIntro);
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
        assert_eq!(g.state, State::RoundIntro);
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
    assert_eq!(g.state, State::RoundIntro, "the solo game would not start");
    assert_eq!(g.p[1].who, g.ladder()[0], "the solo ladder picked the wrong opponent");
}

#[test]
fn a_versus_result_reports_a_winner_not_a_score() {
    // A versus match ends on the same state a lost solo run does, and
    // that screen said "GAME OVER" over a score — which is true of a
    // run against the ladder and says nothing about a match two people
    // just played.
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
