//! Bubbles: blown, trapping a monster on the way, rising to the ceiling and gathering, ridden or popped (with all
//! they touch), and bursting with an angry monster when their time is up.

use super::*;

/// Bubble `start` and every bubble touching it, and every one touching those (within CHAIN_REACH of their radii).
fn touching(bubbles: &[Bubble], start: usize) -> Vec<usize> {
    let mut chain = vec![start];
    let mut k = 0;
    while k < chain.len() {
        let a = bubbles[chain[k]];
        for (j, b) in bubbles.iter().enumerate() {
            if chain.contains(&j) || !b.active { continue; }
            if blip::distance(a.x, a.y, b.x, b.y) < (a.radius() + b.radius()) * CHAIN_REACH { chain.push(j); }
        }
        k += 1;
    }
    chain
}

/// Player `by` pops bubble `start` and everything touching it, and everything touching those: trapped monsters in the
/// chain are finished, each worth double the last, and each drops a sweet.
pub(super) fn pop_chain(g: &mut Game, start: usize, by: usize, sfx: &Sounds) {
    let chain = touching(&g.bubbles, start);
    let mut kills = 0;
    // One popup for the whole chain: catches sit 24px apart and a popup is
    // wider than that, so one each piled up into an unreadable smear.
    let (mut chain_total, mut chain_at) = (0, (0.0, 0.0));
    for &j in &chain {
        let b = g.bubbles[j];
        g.bubbles[j].active = false;
        let ring = if b.owner == 0 { rgba(0.6, 1.0, 0.6, 0.9) } else { rgba(0.6, 0.85, 1.0, 0.9) };
        g.burst(b.x, b.y, 8, ring, 90.0, false);
        g.ring(b.x, b.y, ring, b.radius());
        if b.trapped.is_some() {
            let value = 1000 << kills.min(4);
            g.p[by].score += value;
            chain_total += value;
            if kills == 0 {
                chain_at = (b.x, b.y - 6.0);
            }
            g.burst(b.x, b.y, 10, rgba(1.0, 0.85, 0.3, 1.0), 150.0, true);
            g.fruits.push(Fruit {
                x: b.x - 8.0,
                y: b.y - 8.0,
                vx: blip::rand_range_f32(-110.0, 110.0),
                vy: -300.0,
                kind: kills.min(3),
                t: 0.0,
                on_ground: false,
                active: true,
            });
            kills += 1;
        } else {
            g.p[by].score += 10;
        }
    }
    if chain_total > 0 {
        g.pops.push(Popup { x: chain_at.0, y: chain_at.1, t: 0.0, value: chain_total, c: rgba(1.0, 0.95, 0.4, 1.0) });
    }
    play_sfx(&sfx.pop);
    g.stats.pops += 1;
    g.stats.kills += kills as u32;
    if kills > 0 {
        g.stats.chains[kills.min(5)] += 1;
        play_sfx(&sfx.kill);
    }
    if kills >= 2 {
        g.chain = (kills as i32, 1.2);
        g.shake = 0.25 + 0.08 * kills as f32;
        play_sfx(&sfx.sparkle);
    }
}

pub(super) fn move_bubbles(
    g: &mut Game,
    dt: f32,
    inp: [Input; 2],
    sfx: &Sounds,
    enemy_starts: &[(f32, f32)],
    player_starts: [(f32, f32); 2],
) {
    let bubble_starts: Vec<(f32, f32)> = g.bubbles.iter().map(|b| (b.x, b.y)).collect();
    for i in 0..g.bubbles.len() {
        let mut b = g.bubbles[i];
        if !b.active { continue; }
        b.age += dt;
        b.wob += dt;
        b.squish = (b.squish - dt * 4.0).max(0.0);
        let was_free = b.trapped.is_none();
        match b.phase {
            Phase::Shoot => blow(g, &mut b, dt, sfx, enemy_starts),
            Phase::Float => rise(&mut b, dt),
            Phase::Top => gather(&mut b, dt),
        }
        // inside the walls, at whatever size it now is
        let r = b.radius();
        b.x = b.x.clamp(TILE + r, WIN_W as f32 - TILE - r);
        if was_free && b.trapped.is_some() { g.stats.traps += 1; }
        let life = if b.trapped.is_some() { trap_life(g.round) } else { FREE_LIFE };
        if b.age > life && b.phase != Phase::Shoot { burst(g, &mut b, sfx); }
        g.bubbles[i] = b;
    }
    jostle(&mut g.bubbles);
    ride_or_pop(g, inp, sfx, &bubble_starts, player_starts);
    g.bubbles.retain(|b| b.active);
}

/// A bubble just blown flies out, slowing, until it reaches a wall or SHOOT_T runs out, trapping the first monster it
/// meets on the way (swept from where both were).
pub(super) fn blow(g: &mut Game, b: &mut Bubble, dt: f32, sfx: &Sounds, enemy_starts: &[(f32, f32)]) {
    let from = (b.x, b.y);
    b.x += b.vx * dt;
    b.vx *= 1.0 - 2.5 * dt;
    if b.x < TILE + BUB_R {
        b.x = TILE + BUB_R;
        b.phase = Phase::Float;
    }
    if b.x > WIN_W as f32 - TILE - BUB_R {
        b.x = WIN_W as f32 - TILE - BUB_R;
        b.phase = Phase::Float;
    }
    if b.age > SHOOT_T { b.phase = Phase::Float; }
    if b.trapped.is_some() { return; }
    for (ei, e) in g.enemies.iter_mut().enumerate() {
        if !e.active || e.pop_in < 1.0 { continue; }
        let (ex, ey) = (e.x + E_W / 2.0, e.y + E_H / 2.0);
        let enemy_from = enemy_starts.get(ei).copied().unwrap_or((ex, ey));
        let (rx, ry) = (BUB_R + E_W / 2.0 - 2.0, BUB_R + E_H / 2.0 - 2.0);
        let (x0, y0, x1, y1) = (from.0 - enemy_from.0, from.1 - enemy_from.1, b.x - ex, b.y - ey);
        if segment_rect_overlap(x0, y0, x1, y1, -rx, -ry, rx * 2.0, ry * 2.0) {
            e.active = false;
            b.trapped = Some((e.kind, e.angry));
            b.age = 0.0;
            b.phase = Phase::Float;
            b.x = ex;
            b.y = ey;
            play_sfx(&sfx.trap);
            return;
        }
    }
}

/// It floats up, swaying (slower with a monster inside), to the ceiling.
pub(super) fn rise(b: &mut Bubble, dt: f32) {
    b.y -= if b.trapped.is_some() { RISE_TRAPPED } else { RISE } * dt;
    b.x += (b.wob * 2.2).sin() * 10.0 * dt;
    if b.y <= b.top_y() {
        b.y = b.top_y();
        b.phase = Phase::Top;
    }
}

/// Along the ceiling it drifts to the middle, bobbing.
pub(super) fn gather(b: &mut Bubble, dt: f32) {
    let d = WIN_W as f32 / 2.0 - b.x;
    b.x += d.signum() * d.abs().min(38.0) * dt + (b.wob * 1.7).sin() * 6.0 * dt;
    b.y = b.top_y() + (b.wob * 2.0).sin() * 1.5;
}

/// A bubble that has lasted its time bursts; a monster inside comes out, and it is not happy about it.
pub(super) fn burst(g: &mut Game, b: &mut Bubble, sfx: &Sounds) {
    b.active = false;
    if let Some((kind, _)) = b.trapped {
        g.stats.escapes += 1;
        play_sfx_volume(&sfx.huff, 0.7);
        let ghost = if kind == Kind::Ghost { 62.0 } else { 0.0 };
        g.enemies.push(Enemy {
            kind,
            x: b.x - E_W / 2.0,
            y: b.y - E_H / 2.0,
            vx: ghost,
            vy: ghost,
            dir: if blip::rand_range_f32(0.0, 1.0) < 0.5 { -1.0 } else { 1.0 },
            on_ground: false,
            angry: true,
            t: 0.0,
            jump_cd: 0.5,
            edge_cd: 0.0,
            active: true,
            pop_in: 1.0,
            seed: b.x * 0.37 + b.y * 0.11,
            shot_cd: blip::rand_range_f32(1.5, 3.0),
            windup: 0.0,
            crouch: 0.0,
            leap: (0.0, 0.0),
        });
    }
    g.burst(b.x, b.y, 6, rgba(1.0, 1.0, 1.0, 0.6), 60.0, false);
}

/// Bubbles along the ceiling jostle into a cluster instead of stacking up.
pub(super) fn jostle(bubbles: &mut [Bubble]) {
    for i in 0..bubbles.len() {
        for j in (i + 1)..bubbles.len() {
            let (a, b) = (bubbles[i], bubbles[j]);
            if !a.active || !b.active || a.phase == Phase::Shoot || b.phase == Phase::Shoot { continue; }
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let d = dx.hypot(dy).max(0.1);
            let min = a.radius() + b.radius() - 1.0;
            if d >= min { continue; }
            let push = (min - d) * 0.5;
            bubbles[i].x -= dx / d * push;
            bubbles[j].x += dx / d * push;
            if a.phase != Phase::Top || b.phase != Phase::Top {
                bubbles[i].y -= dy / d * push * 0.5;
                bubbles[j].y += dy / d * push * 0.5;
            }
        }
    }
}

/// Players against bubbles, swept from where both were: falling onto one with jump held rides it up, otherwise it
/// pops (taking its neighbours with it), except a player's own just blown.
pub(super) fn ride_or_pop(g: &mut Game, inp: [Input; 2], sfx: &Sounds, bubble_starts: &[(f32, f32)], player_starts: [(f32, f32); 2]) {
    for pi in 0..2 {
        let p = &g.p[pi];
        if !p.joined || !p.alive || p.dead_t > 0.0 { continue; }
        let (px, py, vy) = (p.cx(), p.cy(), p.vy);
        let (sx, sy) = player_starts[pi];
        for i in 0..g.bubbles.len() {
            let b = g.bubbles[i];
            if !b.active { continue; }
            let (bx, by) = bubble_starts.get(i).copied().unwrap_or((b.x, b.y));
            if !segment_circle_overlap(sx - bx, sy - by, px - b.x, py - b.y, 0.0, 0.0, b.radius() + 11.0) { continue; }
            if vy > 40.0 && sy < by - 6.0 && inp[pi].jump_held {
                g.p[pi].vy = -JUMP_V * 0.82;
                g.p[pi].squash = 1.3;
                g.bubbles[i].squish = 1.0;
                g.stats.bounces += 1;
                play_sfx_volume(&sfx.bounce, 0.7);
                break;
            }
            // your own, just blown
            if b.owner == pi && b.age < 0.25 && b.trapped.is_none() { continue; }
            pop_chain(g, i, pi, sfx);
            break;
        }
    }
}
