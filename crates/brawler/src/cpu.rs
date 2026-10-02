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
    if foe.act == Act::Attack && dist < foe_range * 1.2 {
        // Being attacked at close range forces a fresh decision (see
        // update_fight()). How much it blocks depends on what is coming: a
        // sweep or special is respected, a jab traded with; always blocking
        // stalemated against a mashed punch.
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

    // Up close every fighter keeps a fast option, even the heavy (without one
    // he was jabbed for a whole round); the heavy leans to the slow end of
    // the same list.
    let heavy = me.arch().special == Special::BullRush;
    let fast_share = if heavy { 0.20 } else { 0.32 };
    // What each kind of fighter likes: those with something to throw throw
    // it, the heavy ones would rather take hold, the kickers keep their feet.
    // (How much of the roll goes to the special and to the throw; with the
    // rest of the bands it must stay under 0.94.)
    let (special, throw) = match me.arch().special {
        Special::ChiBolt => (0.16, 0.05),
        Special::BullRush => (0.08, 0.11),
        Special::TalonKick => (0.12, 0.06),
        Special::LaserVision => (0.10, 0.09),
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

/// The CPU's turn for the fighter on `side`: think again if the plan has run
/// out, and press what the plan says. It is written for the second fighter;
/// for the first (the attract mode's) the two are changed over for the
/// length of the call.
pub(crate) fn cpu_turn(g: &mut Game, side: usize, dt: f32) -> Input {
    if side == 0 {
        g.p.swap(0, 1);
        std::mem::swap(&mut g.cpu_plan, &mut g.demo_cpu.0);
        std::mem::swap(&mut g.cpu_delay, &mut g.demo_cpu.1);
        let inp = cpu_turn(g, 1, dt);
        g.p.swap(0, 1);
        std::mem::swap(&mut g.cpu_plan, &mut g.demo_cpu.0);
        std::mem::swap(&mut g.cpu_delay, &mut g.demo_cpu.1);
        return inp;
    }
    // A plan runs for its delay unless the world changes: coming out of a
    // hit, or being attacked up close, forces a rethink.
    let jolted = g.p[1].act == Act::Hitstun || g.p[0].stomp
        || (g.p[0].act == Act::Attack && (g.p[0].x - g.p[1].x).abs() < attack_range(&g.p[0], MoveId::HighKick));
    g.cpu_delay -= dt;
    if g.cpu_delay <= 0.0 || (jolted && g.cpu_delay < 0.12) {
        g.cpu_plan = cpu_think(g);
        // The early turtles fight alone: the call is kept for opponents who
        // have had three fights to learn to guard low.
        if g.cpu_plan == CpuPlan::Attack(MoveId::Special) && g.difficulty < 0.0
            && g.p[1].arch().build == Build::Turtle && !g.p[1].called {
            g.cpu_plan = CpuPlan::Attack(MoveId::LowKick);
        }
        // Faster decisions as the ladder climbs — this is the difficulty
        // dial that actually matters, far more than damage numbers.
        g.cpu_delay = 0.38 - 0.18 * g.difficulty + (rand_int(0, 12) as f32) * 0.01;
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
                // Already committed — throw the overhead. A jump-in that
                // never attacks is just a fighter volunteering to be hit
                // out of the air.
                // A flier out of arm's reach looks down and fires instead.
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
            let _ = id;
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
