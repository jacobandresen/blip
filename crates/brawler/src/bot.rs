//! How to play Brawler: hit back at once when the foe's attack misses; now
//! and then change stance out of reach (down twice); walk in to kick range,
//! then punch, kick, sweep or throw the special at random, often stringing
//! punches into a run that ends in the hook, or a low kick into the roundhouse;
//! block about half the attacks it sees start, low against a sweep; uppercut
//! anyone coming down on top of it.
//! `BLIP_BOT_PICK` (fighter), `BLIP_BOT_FOE` and `BLIP_BOT_LEVEL` (the ladder
//! fight, from 1) choose the matchup; `BLIP_BOT_CONTINUES=n` takes a lost
//! fight again up to n times; `BLIP_BOT_STYLE` and `BLIP_BOT_BLOCK` change how it fights.
//! It fights through the game's own `Input`, so tests can stage fights without a window.

use super::*;
use blip::bot::{Goal::*, Pad, Playthrough};
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
}

static BOT: std::sync::Mutex<Bot> = std::sync::Mutex::new(Bot { react: 0.0, next_attack: 0.0, blocking: 0.0, switching: 0, run: 0, run_kicks: false });

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
    // BLIP_BOT_CONTINUES=n takes a lost fight again, up to n times.
    static USED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    static LAST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let allowed = blip::bot::num("CONTINUES").unwrap_or(0.0) as u32;
    let at_continue = g.state == State::Continue;
    if at_continue && !LAST.swap(true, std::sync::atomic::Ordering::Relaxed) {
        USED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    if !at_continue { LAST.store(false, std::sync::atomic::Ordering::Relaxed); }
    let again = at_continue && USED.load(std::sync::atomic::Ordering::Relaxed) <= allowed;
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
    blip::bot::hold(&[]);
    let mut guard = BOT.lock().unwrap();
    let b = &mut *guard;
    let (me, foe) = (g.p[0], g.p[1]);
    b.react -= dt;
    b.next_attack -= dt;
    b.blocking -= dt;
    punish(b, &me, &foe)
        .or_else(|| block(b, &me, &foe))
        .or_else(|| style(b, &me, &foe))
        .or_else(|| uppercut(b, &me, &foe))
        .or_else(|| switch_stance(b, &me, &foe))
        .unwrap_or_else(|| close_in_and_attack(b, &me, &foe))
}

/// Step toward the foe (`forward`) or back from it, whichever way it faces.
fn walk(inp: &mut Input, me: &Fighter, forward: bool) {
    if (me.facing > 0.0) == forward { inp.right = true; } else { inp.left = true; }
}

/// The foe's attack has missed or been blocked and is still recovering: hit
/// back with the quickest blow that reaches before it can guard again.
fn punish(b: &mut Bot, me: &Fighter, foe: &Fighter) -> Option<Input> {
    if !me.free() || b.next_attack > 0.0 || foe.act != Act::Attack || blip::bot::is("STYLE", "basic") { return None; }
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

/// Seeing an attack start: about a fifth of a second to react, and not
/// always; then hold back (and down against a sweep) for a moment.
fn block(b: &mut Bot, me: &Fighter, foe: &Fighter) -> Option<Input> {
    let dist = (foe.x - me.x).abs();
    if foe.act == Act::Attack && dist < 150.0 && b.react <= 0.0 && b.blocking <= 0.0 {
        b.react = 0.2;
        if gen_range(0.0, 1.0) < block_odds() { b.blocking = 0.35; }
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
    if gen_range(0.0, 1.0) >= 0.7 { return None; }
    Some(Input { down: true, punch_low: true, ..Input::default() })
}

/// Out of reach, now and then: change the leg in front with a double tap
/// of down (three frames down, three up, three down).
fn switch_stance(b: &mut Bot, me: &Fighter, foe: &Fighter) -> Option<Input> {
    if b.switching == 0 {
        let far = (foe.x - me.x).abs() > attack_range(me, MoveId::HighKick) + 30.0;
        if !(far && me.free() && !blip::bot::is("STYLE", "basic") && gen_range(0.0, 1.0) < 0.02) { return None; }
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
        if dist > 220.0 && b.next_attack <= 0.0 && gen_range(0.0, 1.0) < 0.02 {
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
        // BLIP_BOT_STYLE=basic: quick punches and the odd low kick only.
        let basic = blip::bot::is("STYLE", "basic");
        match if basic { [0, 1, 0, 1, 2][gen_range(0, 5)] } else { gen_range(0, 7) } {
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
        if !basic && (inp.punch_low || inp.punch_high) && gen_range(0.0, 1.0) < 0.4 { (b.run, b.run_kicks) = (2, false); }
        if !basic && inp.kick_low && !inp.down && gen_range(0.0, 1.0) < 0.4 { (b.run, b.run_kicks) = (1, true); }
        if b.run > 0 { b.next_attack = 0.2; }
    }
    inp
}
