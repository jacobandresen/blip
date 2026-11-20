//! How to play Brawler: hit back at once when the foe's attack misses; now
//! and then change stance out of reach (down twice); walk in to kick range,
//! then punch, kick, sweep or throw the special at random, often stringing
//! punches into a run that ends in the hook, or a low kick into the roundhouse;
//! block about half the attacks it sees start, low against a sweep; uppercut
//! anyone coming down on top of it. Each round it fights to a plan (plan.rs, which the CPU uses too):
//! pressure, spacing at the edge of the foe's reach, zoning with a thrown special, or jumping in; well behind,
//! it tries another. It jumps a special thrown along the ground.
//! `BLIP_BOT_PICK` (fighter), `BLIP_BOT_FOE` and `BLIP_BOT_LEVEL` (the ladder
//! fight, from 1) choose the matchup; `BLIP_BOT_CONTINUES=n` takes a lost
//! fight again up to n times; `BLIP_BOT_STYLE`, `BLIP_BOT_PLAN` and `BLIP_BOT_BLOCK` change how it fights.
//! It fights through the game's own `Input`, so tests can stage fights without a window.

use super::*;
use blip::bot::{chance, remember, Goal::*, Pad, Playthrough};
use blip::macroquad::rand::gen_range;

/// What the bot must show it can do in Brawler.
pub const PLAYTHROUGHS: &[Playthrough] = &[
    Playthrough {
        name: "the fighter wins its first fight on the ladder",
        seconds: 240.0,
        goals: &[AtLeast("rounds_won", 2.0), AtLeast("fights_won", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "a run of quick punches ends in a hook",
        goals: &[AtLeast("hooks", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "two quick low kicks come round as a roundhouse",
        goals: &[AtLeast("roundhouses", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it changes the leg in front with a double tap of down",
        goals: &[AtLeast("stance_switches", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "a heavy clean blow leaves a fighter staggering",
        goals: &[AtLeast("staggers", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it climbs to the first bonus round",
        level: BONUS_AFTER[0] as u32 + 1,
        seconds: 300.0,
        settings: &[("CONTINUES", "3")],
        goals: &[AtLeast("bonus_rounds", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it climbs to the second bonus round",
        level: BONUS_AFTER[1] as u32 + 1,
        seconds: 600.0,
        settings: &[("CONTINUES", "6")],
        goals: &[AtLeast("bonus_rounds", 1.0)],
        ..Playthrough::DEFAULT
    },
];

#[derive(Default)]
struct Bot {
    react: f32,
    next_attack: f32,
    blocking: f32,
    /// Frames into a change of stance (down, let go, down), 0 when not changing.
    switching: u8,
    /// Blows still to come in a run (punches to the hook, or the second kick).
    run: u8,
    run_kicks: bool,
    plan: Plan,
    /// The round the plan was made for, and how long it has been tried.
    plan_round: i32,
    plan_t: f32,
    /// The distance last frame: smaller now means the foe is stepping in.
    last_dist: f32,
}



/// Menus: fire through the title, pick the fighter, fire again.
pub fn menus(g: &mut Game) {
    if g.state == State::Select {
        g.pick = blip::bot::num("PICK").unwrap_or(0.0) as usize % FIGHTERS.len();
    }
    if g.state == State::Vs {
        let rung = blip::bot::level(RUNGS);
        if g.opponent_index < rung {
            g.start_match(rung);
            g.announce();
        }
    }
    if g.state == State::RoundIntro {
        let foe = blip::bot::num("FOE").map(|f| f as usize);
        if let Some(foe) = foe.filter(|&f| f < FIGHTERS.len() && f != g.p[1].who) {
            g.p[1] = Fighter { rounds: g.p[1].rounds, ..Fighter::new(foe, 440.0, -1.0) };
        }
    }
    let again = g.state == State::Continue && blip::bot::may_continue();
    // (It taps through the result screen too, as a player does.)
    let fire = again || matches!(g.state, State::Title | State::Select | State::MatchEnd | State::Over | State::Won);
    let mut pad = Pad::new();
    if fire { pad.press_start(); }
    pad.send();
}

/// How often it blocks an attack it sees start: BLIP_BOT_BLOCK, default 0.55.
fn block_odds() -> f32 {
    blip::bot::num("BLOCK").map_or(0.55, |b| b as f32)
}

/// One frame of the fight for player one, through the game's own `Input`.
pub fn fight(g: &Game, dt: f32) -> Input {
    let inp = remember(|b: &mut Bot| think(b, g, dt));
    blip::bot::at(g.p[0].x, g.p[0].y - 40.0);
    // The fight reads `inp`, not the keys; holding the keys it stands for lets the fun meter see the play.
    let mut pad = Pad::new();
    for (on, key) in [(inp.left, BLIP_KEY_LEFT), (inp.right, BLIP_KEY_RIGHT), (inp.up, BLIP_KEY_UP), (inp.down, BLIP_KEY_DOWN),
        (inp.punch_low || inp.punch_high, BLIP_KEY_SPACE), (inp.kick_low || inp.kick_high, BLIP_KEY_BUTTON2),
        (inp.special, BLIP_KEY_F)] {
        if on { pad.hold(key); }
    }
    pad.send();
    inp
}

fn think(b: &mut Bot, g: &Game, dt: f32) -> Input {
    let (me, foe) = (g.p[0], g.p[1]);
    b.react -= dt;
    b.next_attack -= dt;
    b.blocking -= dt;
    choose_plan(b, g, &me, &foe, dt);
    let inp = punish(b, &me, &foe)
        .or_else(|| duck_laser(&me, &foe))
        .or_else(|| jump_bolt(g, &me))
        .or_else(|| block(b, &me, &foe))
        .or_else(|| style(b, &me, &foe))
        .or_else(|| uppercut(b, &me, &foe))
        .or_else(|| switch_stance(b, &me, &foe))
        .unwrap_or_else(|| match b.plan {
            Plan::Pressure => close_in_and_attack(b, &me, &foe),
            Plan::Spacing => space(b, &me, &foe),
            Plan::Zoning => zone(b, &me, &foe),
            Plan::JumpIn => jump_in(b, &me, &foe),
        });
    b.last_dist = (foe.x - me.x).abs();
    inp
}

/// A plan at the bell, at random; another after four seconds well behind on health. With plans the bot took
/// 7 of 14 rounds from Jade on fight 8 (1 of 9 before) and 4 of 12 from Regent (0 of 8).
fn choose_plan(b: &mut Bot, g: &Game, me: &Fighter, foe: &Fighter, dt: f32) {
    if blip::bot::is("STYLE", "simple") { return; }
    b.plan_t += dt;
    let behind = well_behind(me, foe);
    let round = g.round * 10 + me.rounds + foe.rounds;
    if round == b.plan_round && !(behind && b.plan_t > 4.0) { return; }
    let was = b.plan;
    b.plan = Plan::pick(me);
    // BLIP_BOT_PLAN=pressure|spacing|zoning|jump_in fights the whole run one way.
    for &p in Plan::open_to(me) {
        if blip::bot::is("PLAN", p.name()) { b.plan = p; }
    }
    if round != b.plan_round || b.plan != was { blip::bot::add(&format!("plan_{}", b.plan.name()), 1.0); }
    (b.plan_round, b.plan_t) = (round, 0.0);
}

/// A ball of chi (or a web, or a pizza) coming at it along the ground: jump it, toward the thrower.
fn jump_bolt(g: &Game, me: &Fighter) -> Option<Input> {
    if !me.free() || me.airborne() || blip::bot::is("STYLE", "simple") { return None; }
    let coming = g.bolts.iter().any(|t| t.active && t.owner == 1 && g.p[1].arch().special != Special::LaserVision
        && (t.vx > 0.0) == (me.x > t.x) && (40.0..150.0).contains(&(me.x - t.x).abs()));
    if !coming { return None; }
    let mut inp = Input { up: true, ..Input::default() };
    walk(&mut inp, me, true);
    Some(inp)
}

/// Spacing: hover at the edge of the foe's longest kick and poke with its own longest low blow whenever the
/// foe steps in (and now and then anyway). In a corner there is no room to wait, so it fights its way out.
fn space(b: &mut Bot, me: &Fighter, foe: &Fighter) -> Input {
    let dist = (foe.x - me.x).abs();
    if !room_behind(me) { return close_in_and_attack(b, me, foe); }
    let mut inp = Input::default();
    let (poke, mine) = longest_poke(me);
    let edge = spacing_edge(foe);
    let stepping_in = dist < b.last_dist - 0.5;
    if b.next_attack <= 0.0 && me.free() && dist < mine && (stepping_in || chance(0.04)) {
        inp.kick_low = true;
        inp.down = poke == MoveId::Sweep && chance(0.6);
        b.next_attack = gen_range(0.4, 0.8);
    } else if dist < edge - 20.0 {
        walk(&mut inp, me, false);
    } else if dist > edge + 25.0 {
        walk(&mut inp, me, true);
    }
    inp
}

/// Zoning: across the stage, throw the special and wait for the next; up close, fight.
fn zone(b: &mut Bot, me: &Fighter, foe: &Fighter) -> Input {
    let dist = (foe.x - me.x).abs();
    if dist < 160.0 { return close_in_and_attack(b, me, foe); }
    let mut inp = Input::default();
    if b.next_attack <= 0.0 && me.free() {
        inp.special = true;
        b.next_attack = gen_range(0.9, 1.6);
    } else if dist > 380.0 {
        walk(&mut inp, me, true);
    }
    inp
}

/// Jump-in: from just outside reach, jump forward, kick on the way down and land in a run of punches.
fn jump_in(b: &mut Bot, me: &Fighter, foe: &Fighter) -> Input {
    let dist = (foe.x - me.x).abs();
    let mut inp = Input::default();
    if me.airborne() {
        inp.kick_low = me.act == Act::Air && me.vy > 0.0 && dist < attack_range(me, MoveId::JumpKick) + 30.0;
        return inp;
    }
    // (From the high kick's reach it lost both rounds of fight 1 on seed 2, where every other plan won;
    // from a jump's length, 3-0 on four seeds.)
    let travel = jump_travel(me);
    if dist > travel * 1.15 {
        walk(&mut inp, me, true);
    } else if dist > travel * 0.8 && b.next_attack <= 0.0 && me.free() {
        inp.up = true;
        walk(&mut inp, me, true);
        b.next_attack = 0.3;
        (b.run, b.run_kicks) = (2, false);
    } else {
        return close_in_and_attack(b, me, foe);
    }
    inp
}

/// Step toward the foe (`forward`) or back from it, whichever way it faces.
fn walk(inp: &mut Input, me: &Fighter, forward: bool) {
    if (me.facing > 0.0) == forward { inp.right = true; } else { inp.left = true; }
}

/// The foe's attack has missed or been blocked and is still recovering: hit
/// back with the quickest blow that reaches before it can guard again.
fn punish(b: &mut Bot, me: &Fighter, foe: &Fighter) -> Option<Input> {
    if !me.free() || b.next_attack > 0.0 || foe.act != Act::Attack || blip::bot::is("STYLE", "simple") { return None; }
    let m = foe.scaled(move_data(foe.mv));
    let recovering = foe.t > (m.startup + m.active) * F && foe.t < (m.startup + m.active + m.recovery - 4.0) * F;
    if !recovering { return None; }
    let dist = (foe.x - me.x).abs();
    let mut inp = Input::default();
    if dist < attack_range(me, MoveId::LowPunch) { inp.punch_low = true; }
    else if dist < attack_range(me, MoveId::LowKick) { inp.kick_low = true; }
    else { return None; }
    b.next_attack = 0.25;
    Some(inp)
}

/// The laser's stare is the tell (it fires where the eyes looked): sit under one from the ground, guard one from the
/// air, as a player who has met it once does. Without this the bot lost every round to it within a second.
fn duck_laser(me: &Fighter, foe: &Fighter) -> Option<Input> {
    let staring = foe.arch().special == Special::LaserVision && foe.act == Act::Attack && foe.mv == MoveId::Special;
    if !staring || !me.free() || blip::bot::is("STYLE", "simple") { return None; }
    let mut inp = Input::default();
    walk(&mut inp, me, false);
    inp.down = !foe.airborne();
    Some(inp)
}

/// Seeing an attack start: about a fifth of a second to react, and not
/// always; then hold back (and down against a sweep) for a moment.
fn block(b: &mut Bot, me: &Fighter, foe: &Fighter) -> Option<Input> {
    let dist = (foe.x - me.x).abs();
    if foe.act == Act::Attack && dist < 150.0 && b.react <= 0.0 && b.blocking <= 0.0 {
        b.react = 0.2;
        if chance(block_odds()) { b.blocking = 0.35; }
    }
    if b.blocking <= 0.0 { return None; }
    let mut inp = Input::default();
    walk(&mut inp, me, false);
    if foe.mv == MoveId::Sweep { inp.down = true; }
    Some(inp)
}

/// The playtest styles, `BLIP_BOT_STYLE`: survive, fly or stomp.
fn style(b: &mut Bot, me: &Fighter, foe: &Fighter) -> Option<Input> {
    let mut inp = Input::default();
    let dist = (foe.x - me.x).abs();
    // survive: last the round out. Sit under the ground laser, stand up to
    // guard what comes from above or from close, and jump clear of a grab.
    if blip::bot::is("STYLE", "survive") {
        let overhead = foe.airborne() || (foe.act == Act::Attack
            && matches!(foe.mv, MoveId::LowPunch | MoveId::HighPunch | MoveId::JumpPunch));
        let grab = !foe.airborne() && foe.act != Act::Attack && dist < throw_range(foe) + 30.0;
        if grab && me.free() { inp.up = true; walk(&mut inp, me, false); return Some(inp); }
        walk(&mut inp, me, false);
        inp.down = !overhead;
        return Some(inp);
    }
    // fly: a flier climbs and fires down from up there.
    if me.flies() && blip::bot::is("STYLE", "fly") {
        if FLOOR_Y - me.y < 110.0 { inp.up = true; }
        else if b.next_attack <= 0.0 { inp.kick_low = true; b.next_attack = 1.2; }
        return Some(inp);
    }
    // stomp: jump on the spot and kick on the way down.
    if blip::bot::is("STYLE", "stomp") {
        if me.airborne() { inp.kick_low = me.act == Act::Air && me.vy > 0.0; }
        else if b.next_attack <= 0.0 && me.free() { inp.up = true; b.next_attack = 2.0; }
        return Some(inp);
    }
    None
}

/// Somebody coming down on top of it: the uppercut, most of the time.
fn uppercut(b: &mut Bot, me: &Fighter, foe: &Fighter) -> Option<Input> {
    let dist = (foe.x - me.x).abs();
    if !(foe.airborne() && dist < attack_range(me, MoveId::Uppercut) && me.free() && b.next_attack <= 0.0) { return None; }
    b.next_attack = 0.5;
    if !chance(0.7) { return None; }
    Some(Input { down: true, punch_low: true, ..Input::default() })
}

/// Out of reach, now and then: change the leg in front with a double tap
/// of down (three frames down, three up, three down).
fn switch_stance(b: &mut Bot, me: &Fighter, foe: &Fighter) -> Option<Input> {
    if b.switching == 0 {
        let far = (foe.x - me.x).abs() > attack_range(me, MoveId::HighKick) + 30.0;
        if !(far && me.free() && !blip::bot::is("STYLE", "simple") && chance(0.02)) { return None; }
    }
    b.switching += 1;
    let down = !(3..6).contains(&b.switching);
    if b.switching >= 9 { b.switching = 0; }
    Some(Input { down, ..Input::default() })
}

/// Walk in to kick range (now and then jumping in with a high kick from
/// afar); in range, punch, kick, sweep, special or step back at random.
fn close_in_and_attack(b: &mut Bot, me: &Fighter, foe: &Fighter) -> Input {
    let mut inp = Input::default();
    let dist = (foe.x - me.x).abs();
    let reach = attack_range(me, MoveId::HighKick);
    if dist > reach + 10.0 {
        walk(&mut inp, me, true);
        if dist > 220.0 && b.next_attack <= 0.0 && chance(0.02) {
            inp.up = true;
            inp.kick_high = true;
            b.next_attack = 0.8;
        }
    } else if b.run > 0 && me.free() && b.next_attack <= 0.25 {
        // The rest of a run, as quick as the last blow allows.
        if b.run_kicks { inp.kick_low = true; } else { inp.punch_low = true; }
        b.run -= 1;
        b.next_attack = 0.2;
    } else if b.next_attack <= 0.0 && me.free() {
        // BLIP_BOT_STYLE=simple: quick punches and the odd low kick only.
        let simple = blip::bot::is("STYLE", "simple");
        match if simple { [0, 1, 0, 1, 2][gen_range(0, 5)] } else { gen_range(0, 7) } {
            0 => inp.punch_low = true,
            1 => inp.punch_high = true,
            2 => inp.kick_low = true,
            3 => inp.kick_high = true,
            4 => { inp.down = true; inp.kick_low = true; }
            5 => inp.special = true,
            _ => walk(&mut inp, me, false),
        }
        b.next_attack = gen_range(0.25, 0.7);
        // Often a punch starts a run to the hook, and a low kick is followed
        // by the roundhouse.
        if !simple && (inp.punch_low || inp.punch_high) && chance(0.4) { (b.run, b.run_kicks) = (2, false); }
        if !simple && inp.kick_low && !inp.down && chance(0.4) { (b.run, b.run_kicks) = (1, true); }
        if b.run > 0 { b.next_attack = 0.2; }
    }
    inp
}
