//! Drops: falling from broken bricks, caught by the paddle, and what each does (wide, narrow, slow, a life, more balls).

use super::*;

pub(super) fn move_drops(g: &mut Game, dt: f32, sfx: &Sounds) {
    let pad_start_x = g.pad_x - g.pad_vx * dt;
    let (pad_x, pad_w) = (g.pad_x, g.pad_w);
    let mut caught = Vec::new();
    for d in pool_iter_mut(&mut g.drops) {
        let from = (d.x, d.y);
        d.y += DROP_SPEED * dt;
        let expired = d.y > WIN_H as f32;
        let pad = (-DROP_W, -DROP_H, pad_w + DROP_W, PAD_H as f32 + DROP_H);
        if blip::sweep_hits(from, (d.x, d.y), (pad_start_x, PAD_Y as f32), (pad_x, PAD_Y as f32), pad) {
            d.active = false;
            caught.push((d.x, d.kind));
        }
        if expired { d.active = false; }
    }
    let mut split = false;
    for (x, kind) in caught { split |= catch_drop(g, x, kind, sfx); }
    if split { g.split_ball(); }
}

/// The paddle caught a drop at `x`: its label and sound, and its effect. True for Multi, which splits the ball.
fn catch_drop(g: &mut Game, x: f32, kind: DropKind, sfx: &Sounds) -> bool {
    blip::bot::add("drops", 1.0);
    blip::bot::add(["got_wide", "got_narrow", "got_slow", "got_life", "got_multi"][kind as usize], 1.0);
    let (label, c) = match kind {
        DropKind::Wide => ("WIDE", BLIP_GREEN),
        DropKind::Narrow => ("NARROW", BLIP_RED),
        DropKind::Slow => ("SLOW", BLIP_CYAN),
        DropKind::Life => ("+1 LIFE", BLIP_YELLOW),
        DropKind::Multi => ("MULTI", BlipColor { r: 0.8, g: 0.55, b: 1.0, a: 1.0 }),
    };
    g.fx.popup(x + DROP_W / 2.0, PAD_Y as f32 - 14.0, label, c);
    play_sfx(match kind {
        DropKind::Wide | DropKind::Slow | DropKind::Multi => &sfx.pickup_good,
        DropKind::Narrow => &sfx.pickup_bad,
        DropKind::Life => &sfx.pickup_life,
    });
    match kind {
        DropKind::Wide | DropKind::Narrow => {
            g.pad_w = if kind == DropKind::Wide { PAD_W_WIDE } else { PAD_W_NARROW };
            g.pad_effect_timer.start(EFFECT_DURATION);
        }
        DropKind::Slow => g.slow_timer.start(EFFECT_DURATION),
        DropKind::Life => g.sess.lives += 1,
        DropKind::Multi => return true,
    }
    false
}
