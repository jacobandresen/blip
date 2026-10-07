//! The pose gallery, a development build (`--features gallery`): every pose
//! of a fighter on one sheet.

use super::*;

// `cargo build -p brawler --features gallery` replaces the game with a looping contact sheet of every pose.
/// The sheet: one pose, five moments of it across the screen.
#[cfg(feature = "gallery")]
pub(crate) fn draw_gallery(blip: &Blip, now: f32) {
    blip.clear(BlipColor { r: 0.15, g: 0.16, b: 0.21, a: 1.0 });
    let acts: [(&str, Act, MoveId); 23] = [
        ("LOW PUNCH", Act::Attack, MoveId::LowPunch),
        ("HIGH PUNCH", Act::Attack, MoveId::HighPunch),
        ("LOW KICK", Act::Attack, MoveId::LowKick),
        ("HIGH KICK", Act::Attack, MoveId::HighKick),
        ("SWEEP", Act::Attack, MoveId::Sweep),
        ("THROW", Act::Attack, MoveId::Throw),
        ("SPECIAL", Act::Attack, MoveId::Special),
        ("JUMP KICK", Act::Attack, MoveId::JumpKick),
        ("FLYING KICK", Act::Attack, MoveId::FlyingKick),
        ("JUMP PUNCH", Act::Attack, MoveId::JumpPunch),
        ("IDLE", Act::Idle, MoveId::LowPunch),
        ("WALK", Act::Walk, MoveId::LowPunch),
        ("CROUCH", Act::Crouch, MoveId::LowPunch),
        ("BLOCK", Act::Block, MoveId::LowPunch),
        ("HITSTUN", Act::Hitstun, MoveId::LowPunch),
        ("KNOCKDOWN", Act::Knockdown, MoveId::LowPunch),
        ("VICTORY", Act::Victory, MoveId::LowPunch),
        ("DEFEAT", Act::Defeat, MoveId::LowPunch),
        ("KO", Act::Defeat, MoveId::LowPunch),
        ("BOW", Act::Bow, MoveId::LowPunch),
        ("FLYING", Act::Air, MoveId::LowPunch),
        ("LOW GUARD", Act::Block, MoveId::LowPunch),
        ("UPPERCUT", Act::Attack, MoveId::Uppercut),
    ];
    // One move at a time, five frames of it across the screen, stepped with
    // left / right so a capture lands on a known frame.
    use std::sync::atomic::{AtomicUsize, Ordering};
    static AT: AtomicUsize = AtomicUsize::new(usize::MAX);
    static WHO: AtomicUsize = AtomicUsize::new(0);
    // BLIP_POSE / BLIP_WHO aim a capture at one cell from the command
    // line; with BLIP_SCREENSHOT_OUT the sheet can be photographed.
    if AT.load(Ordering::Relaxed) == usize::MAX {
        let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<usize>().ok());
        AT.store(env("BLIP_POSE").unwrap_or(0), Ordering::Relaxed);
        WHO.store(env("BLIP_WHO").unwrap_or(0), Ordering::Relaxed);
    }
    if blip::input::key_pressed(BLIP_KEY_RIGHT) { AT.fetch_add(1, Ordering::Relaxed); }
    if blip::input::key_pressed(BLIP_KEY_LEFT) { AT.fetch_add(acts.len() - 1, Ordering::Relaxed); }
    if blip::input::key_pressed(BLIP_KEY_UP) { WHO.fetch_add(1, Ordering::Relaxed); }
    let n = AT.load(Ordering::Relaxed) % acts.len();
    let who = WHO.load(Ordering::Relaxed) % FIGHTERS.len();
    let (label, act, mv) = acts[n];
    blip.fill_rect(0.0, FLOOR_Y, WIN_W as f32, WIN_H as f32 - FLOOR_Y,
        BlipColor { r: 0.11, g: 0.10, b: 0.13, a: 1.0 });

    let probe = Fighter::new(who, 0.0, 1.0);
    let m = probe.scaled(move_data(mv));
    let total = (m.startup + m.active + m.recovery) * F;
    for k in 0..5 {
        let x = 74.0 + k as f32 * 124.0;
        let mut f = Fighter::new(who, x, 1.0);
        f.act = act;
        f.mv = mv;
        f.y = FLOOR_Y;
        if label == "KO" { f.health = 0; }
        f.t = match act {
            // Sampled inside the startup, because the startup is where
            // the shape of a move is decided and the part a defender
            // has to read.
            Act::Attack => [m.startup * F * 0.35, m.startup * F * 0.7, m.startup * F,
                            (m.startup + m.active) * F, total * 0.8][k],
            Act::Knockdown => [0.05, 0.22, 0.6, 0.95, 1.12][k],
            Act::Hitstun => [0.02, 0.06, 0.12, 0.2, 0.3][k],
            // Everything else gets a time sweep too, so the sheet shows
            // whether a pose moves.
            Act::Victory | Act::Defeat => [0.0, 0.18, 0.45, 0.9, 1.6][k],
            Act::Bow => [0.1, 0.5, 0.85, 1.2, 1.55][k],
            _ => now + k as f32 * 0.09,
        };
        if matches!(mv, MoveId::JumpKick | MoveId::JumpPunch) && act == Act::Attack {
            f.y = FLOOR_Y - 46.0;
        }
        // The flying kick is drawn along its own arc, since the pose is
        // read off vertical speed and a still one says nothing.
        if mv == MoveId::FlyingKick && act == Act::Attack {
            let k = k as f32 / 4.0;
            f.y = FLOOR_Y - 44.0 * (1.0 - (2.0 * k - 1.0).powi(2));
            f.vy = FLY_VY * (1.0 - 2.0 * k);
            f.vx = f.facing * FLY_SPEED;
        }
        if act == Act::Walk { f.x = x + (now * 60.0) % 24.0; }
        if label == "LOW GUARD" { f.crouch_block = true; }
        if act == Act::Air {
            f.y = FLOOR_Y - 70.0;
            f.soar = k as f32 / 4.0;
        }
        blip.draw_line(x - 60.0, FLOOR_Y, x + 74.0, FLOOR_Y,
            BlipColor { r: 0.34, g: 0.34, b: 0.44, a: 0.8 });
        // The hitbox this frame, so the picture can be checked against
        // the thing it claims to be a picture of.
        if let Some((hx, hy, hw, hh)) = f.hit_box() {
            blip.fill_rect(hx, hy, hw, hh, BlipColor { r: 1.0, g: 0.3, b: 0.3, a: 0.30 });
        }
        draw_body(blip, &f, now, 0.0, 0.0, k, false);
    }
    blip.draw_centered(label, 22.0, 3.0, BLIP_YELLOW);
    blip.draw_centered(FIGHTERS[who].name, 54.0, 2.0, BLIP_WHITE);
    // The small things, so they can be looked at too: the fruits and the
    // two cameos.
    for kind in 0..FRUIT_POINTS.len() { draw_fruit(blip, kind, 30.0 + kind as f32 * 26.0, 110.0); }
    draw_flyby(blip, 100.0, 5.0 + (now * 0.2).fract());
    draw_chase(blip, 104.0, 0.5 + (now * 0.2).fract(), BlipColor { r: 0.15, g: 0.16, b: 0.21, a: 1.0 });
}
