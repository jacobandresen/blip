//! Native-only autopilot (BLIP_BOT=1) for player one: walk in, mix the
//! moves, block what it sees coming at a human reaction time.
//! BLIP_BOT_PICK=0..9 chooses the fighter, BLIP_BOT_FOE=0..9 every opponent,
//! BLIP_BOT_RUNG=0..8 the fight to start at.

use super::*;
use blip::macroquad::rand::gen_range;

#[derive(Default)]
struct Bot { react: f32, next_attack: f32, blocking: f32 }

static BOT: std::sync::Mutex<Bot> = std::sync::Mutex::new(Bot { react: 0.0, next_attack: 0.0, blocking: 0.0 });

/// Menus: fire through the title, pick the fighter, fire again.
pub fn menus(g: &mut Game, t: f32) {
    let tap = (t * 6.0) as i32 % 2 == 0;
    if g.state == State::Select {
        g.pick = std::env::var("BLIP_BOT_PICK").ok().and_then(|v| v.parse().ok()).unwrap_or(0) % FIGHTERS.len();
    }
    // BLIP_BOT_RUNG=n starts the run at fight n+1 of the ladder.
    if g.state == State::Vs {
        let rung = std::env::var("BLIP_BOT_RUNG").ok().and_then(|v| v.parse::<usize>().ok());
        if let Some(rung) = rung.filter(|&r| r < RUNGS && g.opponent_index < r) {
            g.start_match(rung);
            g.announce();
        }
    }
    if g.state == State::RoundIntro {
        let foe = std::env::var("BLIP_BOT_FOE").ok().and_then(|v| v.parse::<usize>().ok());
        if let Some(foe) = foe.filter(|&f| f < FIGHTERS.len() && f != g.p[1].who) {
            g.p[1] = Fighter { rounds: g.p[1].rounds, ..Fighter::new(foe, 440.0, -1.0) };
        }
    }
    // BLIP_BOT_CONTINUES=n takes a lost fight again, up to n times.
    static USED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    static LAST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let allowed = std::env::var("BLIP_BOT_CONTINUES").ok().and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let at_continue = g.state == State::Continue;
    if at_continue && !LAST.swap(true, std::sync::atomic::Ordering::Relaxed) {
        USED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    if !at_continue { LAST.store(false, std::sync::atomic::Ordering::Relaxed); }
    let again = at_continue && USED.load(std::sync::atomic::Ordering::Relaxed) <= allowed;
    // (It taps through the result screen too, as a player does.)
    let fire = again || matches!(g.state, State::Title | State::Select | State::MatchEnd | State::Over | State::Won);
    blip::bot::hold(if fire && tap { &[BLIP_KEY_SPACE] } else { &[] });
}

/// How often it blocks an attack it sees start: BLIP_BOT_BLOCK, default 0.55.
fn block_odds() -> f32 {
    std::env::var("BLIP_BOT_BLOCK").ok().and_then(|v| v.parse().ok()).unwrap_or(0.55)
}

pub fn fight(g: &Game, dt: f32) -> Input {
    blip::bot::hold(&[]);
    let mut guard = BOT.lock().unwrap();
    let b = &mut *guard;
    let (me, foe) = (g.p[0], g.p[1]);
    let mut inp = Input::default();
    let dist = (foe.x - me.x).abs();
    let fwd = me.facing > 0.0;
    b.react -= dt;
    b.next_attack -= dt;
    b.blocking -= dt;
    // Seeing an attack start: about a fifth of a second to react, and not always.
    if foe.act == Act::Attack && dist < 150.0 && b.react <= 0.0 && b.blocking <= 0.0 {
        b.react = 0.2;
        if gen_range(0.0, 1.0) < block_odds() { b.blocking = 0.35; }
    }
    if b.blocking > 0.0 {
        if fwd { inp.left = true; } else { inp.right = true; }
        if foe.mv == MoveId::Sweep { inp.down = true; }
        return inp;
    }
    // BLIP_BOT_STYLE=survive: last the round out. Sit under the ground laser,
    // stand up to guard what comes from above or from close, and jump clear
    // of a grab.
    if std::env::var("BLIP_BOT_STYLE").is_ok_and(|s| s == "survive") {
        let away = |inp: &mut Input| if fwd { inp.left = true } else { inp.right = true };
        let overhead = foe.airborne() || (foe.act == Act::Attack
            && matches!(foe.mv, MoveId::LowPunch | MoveId::HighPunch | MoveId::JumpPunch));
        let grab = !foe.airborne() && foe.act != Act::Attack && dist < throw_range(&foe) + 30.0;
        if grab && me.free() { inp.up = true; away(&mut inp); return inp; }
        away(&mut inp);
        inp.down = !overhead;
        return inp;
    }
    // BLIP_BOT_STYLE=fly: a flier climbs and fires down from up there.
    if me.flies() && std::env::var("BLIP_BOT_STYLE").is_ok_and(|s| s == "fly") {
        if FLOOR_Y - me.y < 110.0 { inp.up = true; }
        else if b.next_attack <= 0.0 { inp.kick_low = true; b.next_attack = 1.2; }
        return inp;
    }
    // BLIP_BOT_STYLE=stomp: jump on the spot and kick on the way down.
    if std::env::var("BLIP_BOT_STYLE").is_ok_and(|s| s == "stomp") {
        if me.airborne() { inp.kick_low = me.act == Act::Air && me.vy > 0.0; }
        else if b.next_attack <= 0.0 && me.free() { inp.up = true; b.next_attack = 2.0; }
        return inp;
    }
    // Somebody coming down on top of it: the uppercut, most of the time.
    if foe.airborne() && dist < attack_range(&me, MoveId::Uppercut) && me.free() && b.next_attack <= 0.0 {
        b.next_attack = 0.5;
        if gen_range(0.0, 1.0) < 0.7 {
            inp.down = true;
            inp.punch_low = true;
            return inp;
        }
    }
    let reach = attack_range(&me, MoveId::HighKick);
    if dist > reach + 10.0 {
        if fwd { inp.right = true; } else { inp.left = true; }
        if dist > 220.0 && b.next_attack <= 0.0 && gen_range(0.0, 1.0) < 0.02 {
            inp.up = true;
            inp.kick_high = true;
            b.next_attack = 0.8;
        }
    } else if b.next_attack <= 0.0 && me.free() {
        // BLIP_BOT_STYLE=basic: quick punches and the odd low kick only.
        let basic = std::env::var("BLIP_BOT_STYLE").is_ok_and(|s| s == "basic");
        match if basic { [0, 1, 0, 1, 2][gen_range(0, 5)] } else { gen_range(0, 7) } {
            0 => inp.punch_low = true,
            1 => inp.punch_high = true,
            2 => inp.kick_low = true,
            3 => inp.kick_high = true,
            4 => { inp.down = true; inp.kick_low = true; }
            5 => inp.special = true,
            _ => { if fwd { inp.left = true; } else { inp.right = true; } }
        }
        b.next_attack = gen_range(0.25, 0.7);
    }
    inp
}
