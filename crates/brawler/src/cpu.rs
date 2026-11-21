//! The CPU opponent. It decides on a plan a few times a second from what it
//! can see, and turns the plan into the same `Input` a player produces, so it
//! plays by the same rules.

use super::*;

/// What the CPU wants to do, decided on a delay that shortens with difficulty
/// rather than every frame, so it reacts like a player and can be baited.
pub(crate) fn cpu_think(g: &mut Game) -> CpuPlan {
    let me = g.p[1];
    let foe = g.p[0];
    let dist = (foe.x - me.x).abs();
    let roll = || (rand_int(0, 99) as f32) / 100.0;

    // Below zero on the dial it stands and thinks for some of its turns.
    if roll() < -g.difficulty { return CpuPlan::Wait; }

    // Ranges come from the move table. "Kicking range" is the high kick's.
    let kick_range = attack_range(&me, MoveId::HighKick);
    let jab_range = attack_range(&me, MoveId::LowPunch);
    // The high kick is the shortest kick; thrown from further out it hits
    // nothing.
    let high_range = attack_range(&me, MoveId::HighKick);
    let foe_range = attack_range(&foe, MoveId::HighKick);


    // A rage jump is coming down: be off the floor when it lands. The better
    // the opponent, the more often it sees it.
    if foe.stomp && foe.airborne() && !me.airborne() {
        return if roll() < 0.35 + 0.5 * g.difficulty { CpuPlan::Jump } else { CpuPlan::Wait };
    }

    // Somebody who cannot move, wrapped up or seeing stars, is walked up to
    // and hit as hard as there is. (A flier keeps its kick, which is the laser.)
    if (foe.webbed > 0.0 || foe.dizzy > 0.0) && !foe.airborne() && !me.airborne() {
        return if dist > high_range * 0.9 { CpuPlan::Approach }
            else if me.flies() { CpuPlan::Attack(MoveId::HighPunch) }
            else { CpuPlan::Attack(MoveId::HighKick) };
    }

    // The one with the laser uses it from range: level from the ground, which
    // a crouch goes under, and from the air at whoever is sitting under it.
    if me.arch().special == Special::LaserVision && !me.airborne() && me.free()
        && dist > kick_range * 1.4 && roll() < 0.30 + 0.35 * g.difficulty {
        return if foe.crouching() { CpuPlan::Jump } else { CpuPlan::Attack(MoveId::Special) };
    }

    // React to what they are doing, before deciding what to do.
    if foe.airborne() && dist < kick_range * 1.4 {
        return if roll() < 0.4 + 0.5 * g.difficulty {
            // Meet them on the way down: the uppercut if they are close
            // enough to be under, the high kick if not.
            if dist < attack_range(&me, MoveId::Uppercut) && !me.flies() { CpuPlan::Attack(MoveId::Uppercut) }
            else { CpuPlan::Attack(MoveId::HighKick) }
        } else {
            CpuPlan::Block
        };
    }
    // The intercepting fist: DRAKE meets a blow still on its way out with a jab into it, as often as not,
    // where the others block. (Only while it is winding up: after that it is too late to beat it.)
    if formless(me.who) && foe.act == Act::Attack && dist < jab_range * 1.1 && foe.t < foe.scaled(move_data(foe.mv)).startup * F * 0.6
        && roll() < 0.35 + 0.3 * g.difficulty {
        return CpuPlan::Attack(MoveId::LowPunch);
    }
    if foe.act == Act::Attack && dist < foe_range * 1.2 {
        // Block 55–95% against heavy or knockdown moves, 22–42% against jabs.
        let scary = move_data(foe.mv).damage >= 10 || move_data(foe.mv).knockdown;
        let want = if scary { 0.55 + 0.4 * g.difficulty } else { 0.22 + 0.2 * g.difficulty };
        if roll() < want { return CpuPlan::Block; }
        // Not blocking means taking the turn back with the fastest thing
        // available, or a throw up close. (Retreating hands a rushing
        // opponent free ground.)
        if dist <= throw_range(&me) && foe.throw_rest <= 0.0 { return CpuPlan::Attack(MoveId::Throw); }
        if dist < jab_range { return CpuPlan::Attack(MoveId::LowPunch); }
    }
    // They committed to something slow and it missed: take the turn.
    if foe.act == Act::Attack && foe.hit_done && dist < kick_range && roll() < 0.5 + 0.4 * g.difficulty {
        return CpuPlan::Attack(MoveId::LowPunch);
    }

    // Read the guard: without it a crouch-blocker took zero damage over a
    // full round.
    if foe.act == Act::Block && dist < kick_range * 1.1 {
        let read = roll() < 0.4 + 0.5 * g.difficulty;
        if read {
            return if dist <= throw_range(&me) && foe.throw_rest <= 0.0 {
                CpuPlan::Attack(MoveId::Throw)  // nothing guards against this
            } else if foe.crouch_block {
                // A low guard loses to an overhead: the high kick (slow,
                // readable) is preferred, the jump-in kept as the surprise.
                if dist < high_range && roll() < 0.45 { CpuPlan::Attack(MoveId::HighKick) }
                else { CpuPlan::Jump }
            } else {
                // A high guard loses to a low, and there are two of
                // those as well: the sweep if it can afford the
                // recovery, the quick one if it cannot.
                if roll() < 0.6 { CpuPlan::Attack(MoveId::Sweep) }
                else { CpuPlan::Attack(MoveId::LowKick) }
            };
        }
    }

    // The round's plan, played as sharply as the dial allows: hardly at all on the first fight, seven
    // turns in ten on the last. (At one in four there, the bot's rounds on fight 1 fell from 17-5 to 13-5; at this, 17-4.)
    let sharp = (0.3 + 0.5 * g.difficulty).max(0.05);
    match g.cpu_style {
        // Spacing answers a foe who comes in: poke them as they step into it, or step back out of
        // reach. Against one who waits too it fights as the rest do (waiting for each other was a
        // stalemate in 75 of 336 test rounds).
        Plan::Spacing if dist > jab_range && room_behind(&me) && foe.act == Act::Walk => {
            let (poke, reach) = longest_poke(&me);
            if dist < reach && roll() < sharp { return CpuPlan::Attack(poke); }
            if dist < spacing_edge(&foe) - 20.0 { return CpuPlan::Retreat; }
        }
        Plan::Zoning if dist > 160.0 => {
            return if roll() < sharp { CpuPlan::Attack(MoveId::Special) }
                else if dist > 380.0 { CpuPlan::Approach } else { CpuPlan::Wait };
        }
        Plan::JumpIn if (0.8..1.15).contains(&(dist / jump_travel(&me))) && roll() < sharp * 0.6 => {
            return CpuPlan::Jump;
        }
        _ => {}
    }

    if dist > 230.0 {
        // Too far to do anything but close the gap — or throw something
        // that crosses it.
        if matches!(me.arch().special, Special::ChiBolt | Special::LaserVision) && roll() < 0.2 + 0.4 * g.difficulty {
            return CpuPlan::Attack(MoveId::Special);
        }
        return CpuPlan::Approach;
    }
    if dist > kick_range * 1.25 {
        // Jumping in is only worth it from outside their reach: into a
        // poke it is a free knockdown for them.
        if dist > foe_range * 1.3 && roll() < 0.10 + 0.14 * g.difficulty {
            // Two ways in on one budget: the flying kick and the jump-in.
            // Both on top of each other, a crouch-blocker never survived a
            // round.
            return if roll() < 0.35 {
                CpuPlan::Attack(MoveId::FlyingKick)
            } else {
                CpuPlan::Jump
            };
        }
        // Out-ranged, inside their reach and outside your own, the gap must
        // be crossed, not walked. Without this Brutus landed nothing against
        // a long-limbed poker; the charge and the jump-in cover ground.
        if dist < foe_range * 1.15 && foe_range > kick_range * 1.1 {
            return match roll() {
                r if r < 0.34 => CpuPlan::Attack(MoveId::Special),
                r if r < 0.48 => CpuPlan::Jump,
                r if r < 0.60 => CpuPlan::Attack(MoveId::FlyingKick),
                r if r < 0.88 => CpuPlan::Approach,
                _ => CpuPlan::Block,
            };
        }
        return CpuPlan::Approach;
    }

    // In range, the mix is the personality: wide enough that no single answer
    // covers it, weighted to the fighter. `r` shifts toward attacking when
    // the opponent is nearly out, so the CPU can close a round.
    let nearly_out = (foe.health as f32) < FIGHTERS[foe.who].health as f32 * 0.35;
    // Half the clock gone with both bars nearly full is a round the clock
    // will decide; force the issue, the same nudge as smelling a finish.
    let stalling = g.clock < ROUND_SECS * 0.5
        && (foe.health as f32) > FIGHTERS[foe.who].health as f32 * 0.72
        && (me.health as f32) > FIGHTERS[me.who].health as f32 * 0.72;
    let r = if nearly_out || stalling { roll() * 0.7 } else { roll() };

    if dist >= jab_range {
        // At the edge of its reach: long pokes only, and the sweep is the
        // one that really reaches out here.
        return match r {
            _ if r < 0.44 => CpuPlan::Attack(MoveId::Sweep),
            _ if r < 0.58 => CpuPlan::Attack(MoveId::LowKick),
            _ if r < 0.72 => CpuPlan::Attack(MoveId::Special),
            _ if r < 0.94 => CpuPlan::Approach,
            _ => CpuPlan::Block,
        };
    }

    // Jeet kune do has no pattern to read: up close DRAKE spreads his choices almost evenly over everything he
    // has, feints out and back in, and jumps when nobody expects it.
    if formless(me.who) {
        let close = dist < jab_range * 1.1;
        return match r {
            _ if r < 0.11 => CpuPlan::Attack(MoveId::LowPunch),
            _ if r < 0.21 => CpuPlan::Attack(MoveId::HighPunch),
            _ if r < 0.31 => CpuPlan::Attack(MoveId::LowKick),
            _ if r < 0.42 => if dist < high_range { CpuPlan::Attack(MoveId::HighKick) } else { CpuPlan::Attack(MoveId::LowKick) },
            _ if r < 0.50 => CpuPlan::Attack(MoveId::Sweep),
            _ if r < 0.57 && close => CpuPlan::Attack(MoveId::Uppercut),
            _ if r < 0.66 => CpuPlan::Attack(MoveId::Special),
            _ if r < 0.73 && foe.throw_rest <= 0.0 && dist <= throw_range(&me) => CpuPlan::Attack(MoveId::Throw),
            _ if r < 0.76 => CpuPlan::Jump,
            // The feint: a step out, to come straight back in on the next thought.
            _ if r < 0.83 => CpuPlan::Retreat,
            _ if r < 0.94 => CpuPlan::Block,
            _ => CpuPlan::Approach,
        };
    }

    // LANCER fights as the champion did in the Colosseum: kicks first, high ones most, the spinning heel kick often,
    // the hands only to set them up.
    if me.arch().build == Build::Duelist {
        return match r {
            _ if r < 0.12 => CpuPlan::Attack(MoveId::LowPunch),
            _ if r < 0.20 => CpuPlan::Attack(MoveId::HighPunch),
            _ if r < 0.34 => CpuPlan::Attack(MoveId::LowKick),
            _ if r < 0.56 => if dist < high_range { CpuPlan::Attack(MoveId::HighKick) } else { CpuPlan::Attack(MoveId::LowKick) },
            _ if r < 0.70 => CpuPlan::Attack(MoveId::Special),
            _ if r < 0.75 => CpuPlan::Attack(MoveId::Sweep),
            _ if r < 0.80 && foe.throw_rest <= 0.0 && dist <= throw_range(&me) => CpuPlan::Attack(MoveId::Throw),
            _ if r < 0.94 => CpuPlan::Block,
            _ => CpuPlan::Approach,
        };
    }

    // Up close every fighter keeps a fast option, even the heavy (without one
    // he was jabbed for a whole round); the heavy leans to the slow end of
    // the same list.
    let heavy = me.arch().special == Special::BullRush;
    let fast_share = if heavy { 0.20 } else { 0.32 };
    // Special and throw shares vary by archetype; all bands stay below 0.94.
    let (special, throw) = match me.arch().special {
        Special::ChiBolt => (0.16, 0.05),
        Special::BullRush => (0.08, 0.11),
        Special::TalonKick => (0.12, 0.06),
        Special::LaserVision => (0.10, 0.09),
        Special::DriveBy => (0.11, 0.05),
        Special::Nunchaku | Special::OneInch => (0.14, 0.05),
    };
    // The bands must leave room for the last two arms, or `Block` becomes
    // unreachable and the CPU stops guarding up close.
    match r {
        _ if r < fast_share * 0.6 => CpuPlan::Attack(MoveId::LowPunch),
        _ if r < fast_share => CpuPlan::Attack(MoveId::LowKick),
        _ if r < fast_share + 0.20 => CpuPlan::Attack(MoveId::Sweep),
        // Low or high: the guard can only be in one place.
        _ if r < fast_share + 0.34 => {
            if dist < high_range { CpuPlan::Attack(MoveId::HighKick) }
            else { CpuPlan::Attack(MoveId::LowKick) }
        }
        _ if r < fast_share + 0.34 + special => CpuPlan::Attack(MoveId::Special),
        _ if r < fast_share + 0.34 + special + throw && foe.throw_rest <= 0.0 => CpuPlan::Attack(MoveId::Throw),
        _ if r < 0.94 => CpuPlan::Block,
        _ => CpuPlan::Retreat,
    }
}

/// Run the CPU turn for `side`, swapping fighters around it for attract mode's player one.
pub(crate) fn cpu_turn(g: &mut Game, side: usize, dt: f32) -> Input {
    if side == 0 {
        g.p.swap(0, 1);
        std::mem::swap(&mut g.cpu_plan, &mut g.demo_cpu.0);
        std::mem::swap(&mut g.cpu_delay, &mut g.demo_cpu.1);
        std::mem::swap(&mut g.cpu_style, &mut g.demo_style);
        let inp = cpu_turn(g, 1, dt);
        g.p.swap(0, 1);
        std::mem::swap(&mut g.cpu_plan, &mut g.demo_cpu.0);
        std::mem::swap(&mut g.cpu_delay, &mut g.demo_cpu.1);
        std::mem::swap(&mut g.cpu_style, &mut g.demo_style);
        return inp;
    }
    // A plan runs for its delay unless the world changes: coming out of a
    // hit, or being attacked up close, forces a rethink.
    let jolted = g.p[1].act == Act::Hitstun || g.p[0].stomp
        || (g.p[0].act == Act::Attack && (g.p[0].x - g.p[1].x).abs() < attack_range(&g.p[0], MoveId::HighKick));
    g.cpu_delay -= dt;
    if g.cpu_delay <= 0.0 || (jolted && g.cpu_delay < 0.12) {
        // Well behind, now and then it tries another way (about every five seconds of decisions).
        if well_behind(&g.p[1], &g.p[0]) && rand_int(0, 99) < 3 { g.cpu_style = Plan::pick(&g.p[1]); }
        let was = g.cpu_plan;
        g.cpu_plan = cpu_think(g);
        let jkd = formless(g.p[1].who);
        // DRAKE never throws the same thing twice running: a repeat is thought again, once.
        if jkd && matches!(was, CpuPlan::Attack(_)) && g.cpu_plan == was { g.cpu_plan = cpu_think(g); }
        // And changes his whole game plan often (one thought in eight), where the others only do when losing.
        if jkd && rand_int(0, 7) == 0 { g.cpu_style = Plan::pick(&g.p[1]); }
        // The early turtles fight alone: the call is kept for opponents who
        // have had three fights to learn to guard low.
        if g.cpu_plan == CpuPlan::Attack(MoveId::Special) && g.difficulty < 0.0
            && g.p[1].arch().build == Build::Turtle && !g.p[1].called {
            g.cpu_plan = CpuPlan::Attack(MoveId::LowKick);
        }
        // Faster decisions as the ladder climbs — this is the difficulty
        // dial that actually matters, far more than damage numbers.
        g.cpu_delay = 0.38 - 0.18 * g.difficulty + (rand_int(0, 12) as f32) * 0.01;
        // Broken rhythm: DRAKE's next thought comes anywhere from half to one and a half times as soon (the
        // same on average, so no harder for it), and his timing cannot be counted.
        if jkd { g.cpu_delay *= 0.5 + (rand_int(0, 100) as f32) * 0.01; }
    }
    cpu_input(g)
}

/// The plan as the same `Input` a player produces, so the CPU is playing the
/// same game by the same rules and not a private version of it.
pub(crate) fn cpu_input(g: &Game) -> Input {
    let me = g.p[1];
    let foe = g.p[0];
    let mut inp = Input::default();
    // A flier comes back down unless the plan is to be up there.
    if me.soaring() && g.cpu_plan != CpuPlan::Jump { inp.down = true; }
    let back = if me.facing > 0.0 { &mut inp.left } else { &mut inp.right };
    match g.cpu_plan {
        CpuPlan::Wait => {}
        CpuPlan::Block => { *back = true; if foe.act == Act::Attack && foe.mv == MoveId::Sweep { inp.down = true; } }
        CpuPlan::Retreat => { *back = true; }
        CpuPlan::Approach => {
            if me.facing > 0.0 { inp.right = true; } else { inp.left = true; }
        }
        CpuPlan::Jump => {
            if me.airborne() {
                // Commit to an overhead; a flier beyond arm's reach fires downward.
                let far = (foe.x - me.x).abs() > attack_range(&me, MoveId::LowPunch) * 1.5;
                if !me.flies() { inp.kick_high = true; }
                else if far { inp.kick_low = true; }
                else { inp.punch_low = true; }
            } else {
                inp.up = true;
                if me.facing > 0.0 { inp.right = true; } else { inp.left = true; }
            }
        }
        // A flier's kick is the laser, which ends the round: the CPU keeps it
        // for when it plans a special, and punches otherwise.
        CpuPlan::Attack(id) if me.flies() && id != MoveId::Special => {
            inp.punch_low = true;
        }
        CpuPlan::Attack(id) => match id {
            MoveId::LowPunch => inp.punch_low = true,
            MoveId::HighPunch => inp.punch_high = true,
            MoveId::LowKick => inp.kick_low = true,
            MoveId::HighKick => inp.kick_high = true,
            MoveId::Sweep => { inp.down = true; inp.kick_low = true; }
            MoveId::Uppercut => { inp.down = true; inp.punch_low = true; }
            MoveId::FlyingKick => { inp.up = true; inp.kick_high = true; }
            MoveId::Special => inp.special = true,
            // A throw is a punch thrown from close enough; cpu_think()
            // only asks for one when it is already that close.
            MoveId::Throw => inp.punch_low = true,
            _ => inp.punch_low = true,
        },
    }
    inp
}
