//! Native-only autopilot (BLIP_BOT=1) for player one: walk in, mix the
//! moves, block what it sees coming at a human reaction time.
//! BLIP_BOT_PICK=0..2 chooses the fighter.

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
    let fire = matches!(g.state, State::Title | State::Select | State::Over | State::Won);
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
