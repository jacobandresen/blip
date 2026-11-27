//! The invaders: the march, their bombs, a diver breaking ranks, and being shot to chips.

use super::*;

/// Pick, shake and fly the diver. Returns true when it has hit the ship.
pub(super) fn update_diver(g: &mut Game, dt: f32, sfx: &Sounds) -> bool {
    if let Some(d) = g.diver { return dive(g, d, dt, sfx); }
    if let Some((idx, left)) = g.dive_pick {
        if !g.aliens[idx].alive {
            g.dive_pick = None;
        } else if left <= dt {
            let a = g.aliens[idx];
            g.aliens[idx].alive = false;
            g.dive_pick = None;
            g.diver = Some(Diver { x: a.x, y: a.y, kind: a.kind, t: 0.0 });
            play_sfx(&sfx.dive);
        } else {
            g.dive_pick = Some((idx, left - dt));
        }
        return false;
    }
    if g.sess.level < DIVE_LEVEL || g.boss_active { return false; }
    g.dive_cd -= dt;
    if g.dive_cd > 0.0 || g.aliens_alive() < DIVE_MIN_ALIENS { return false; }
    g.dive_cd = rand_int(DIVE_EVERY.0, DIVE_EVERY.1) as f32;
    // The lowest alien of a random column.
    let col = rand_int(0, g.active_cols - 1);
    let pick = (0..g.active_rows).rev()
        .map(|r| (r * g.active_cols + col) as usize)
        .find(|&i| g.aliens[i].alive);
    if let Some(idx) = pick { g.dive_pick = Some((idx, DIVE_TELL)); }
    false
}

/// The diver's fall, steering for the ship and through any shield in its way: true when it rams the ship.
pub(super) fn dive(g: &mut Game, mut d: Diver, dt: f32, sfx: &Sounds) -> bool {
    d.t += dt;
    d.y += DIVE_SPEED * dt;
    let aim = g.player_x - d.x;
    d.x += aim.clamp(-DIVE_STEER * dt, DIVE_STEER * dt);
    // Through a shield, not round it.
    for (s, r, c) in blocks() {
        let (bx, by) = shield_block_xy(&g.shields[s], r, c);
        if g.shields[s].alive[r][c] && rects_overlap(d.x + 6.0, d.y + 6.0, ALIEN_W as f32 - 12.0, ALIEN_H as f32 - 12.0,
            bx, by, SHIELD_BLOCK as f32, SHIELD_BLOCK as f32) {
            g.shields[s].alive[r][c] = false;
            g.fx.burst(bx + 6.0, by + 6.0, 4, 60.0, BLIP_GREEN);
        }
    }
    let ship_top = (GROUND_Y - 24) as f32;
    if rects_overlap(d.x + 6.0, d.y + 6.0, ALIEN_W as f32 - 12.0, ALIEN_H as f32 - 12.0,
        g.player_x + 5.0, ship_top, ALIEN_W as f32 - 10.0, 24.0) {
        g.diver = None;
        return true;
    }
    if d.y + ALIEN_H as f32 >= GROUND_Y as f32 {
        // Missed: it ploughs into the ground and is gone.
        g.fx.burst(d.x + ALIEN_W as f32 / 2.0, GROUND_Y as f32 - 4.0, 12, 110.0, alien_color(d.kind));
        play_sfx(&sfx.explosion);
        g.diver = None;
    } else {
        g.diver = Some(d);
    }
    false
}

/// The formation steps on the beat, and drops a row at the edge.
pub(super) fn march(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.march_timer += dt * 1000.0;
    if g.march_timer >= g.march_interval() {
        g.march_timer = 0.0;
        if g.aliens_alive() > 0 {
            play_sfx(&sfx.march[g.march_step]);
            g.march_step = (g.march_step + 1) % 4;
        }
        if g.march_drop_next {
            let drop = lerp(MARCH_DROP_FIRST, MARCH_DROP, ((g.sess.level - 1) as f32 / 3.0).min(1.0));
            for a in g.aliens.iter_mut() {
                if a.alive { a.y += drop; }
            }
            g.march_dir = -g.march_dir;
            g.march_drop_next = false;
        } else {
            let step = (ALIEN_W / 3) as f32;
            let mut hit_edge = false;
            for a in g.aliens.iter_mut() {
                if !a.alive { continue; }
                a.x += step * g.march_dir as f32;
                a.anim ^= 1;
                if a.x < 2.0 || a.x + ALIEN_W as f32 > (WIN_W - 2) as f32 {
                    hit_edge = true;
                }
            }
            if hit_edge { g.march_drop_next = true; }
        }
    }
}

/// Every so often the lowest alien of a random column lets a bomb go.
pub(super) fn drop_bombs(g: &mut Game, dt: f32) {
    if g.bomb_timer.tick(dt) {
        let r01 = (rand() as f32) / (u32::MAX as f32);
        let (lo, hi) = g.bomb_interval_range;
        g.bomb_timer.start(lerp(lo, hi, r01));
        let mut candidates = [0usize; ALIEN_COLS as usize];
        let mut nc = 0usize;
        for c in 0..g.active_cols {
            for r in (0..g.active_rows).rev() {
                let idx = (r * g.active_cols + c) as usize;
                if g.aliens[idx].alive {
                    candidates[nc] = idx;
                    nc += 1;
                    break;
                }
            }
        }
        if nc > 0 {
            if let Some(bi) = g.free_bullet(false) {
                let idx = candidates[rand_int(0, nc as i32 - 1) as usize];
                let a = g.aliens[idx];
                g.bullets[bi] = Bullet {
                    x: a.x + (ALIEN_W / 2 - 4) as f32,
                    y: a.y + ALIEN_H as f32,
                    active: true,
                    player: false,
                };
            }
        }
    }
}

/// Player bullets against the formation: the lowest alien in a bullet's path falls, for points that grow with a streak.
pub(super) fn shoot_aliens(g: &mut Game, bullet_starts: &[(f32, f32); N_BULLETS], sfx: &Sounds) {
    // `bi` pairs the bullet slot with its swept start (the player slots head the array).
    for (bi, start) in bullet_starts.iter().enumerate().take(MAX_PLAYER_BULLETS) {
        if !g.bullets[bi].active { continue; }
        'rows: for row in (0..g.active_rows).rev() {
            for col in 0..g.active_cols {
                let ai = (row * g.active_cols + col) as usize;
                if !g.aliens[ai].alive { continue; }
                if !bullet_hits_rect(*start, (g.bullets[bi].x, g.bullets[bi].y),
                                     g.aliens[ai].x, g.aliens[ai].y,
                                     ALIEN_W as f32, ALIEN_H as f32) {
                    continue;
                }
                play_sfx(&sfx.explosion);
                let (ax, ay, kind) = (g.aliens[ai].x, g.aliens[ai].y, g.aliens[ai].kind);
                g.spawn_explosion(ax, ay);
                g.aliens[ai].alive = false;
                g.bullets[bi].active = false;
                blip::bot::add("invaders", 1.0);
                let times = g.streak.hit();
                let pts = match kind { 0 => 30, 1 => 20, _ => 10 } * g.sess.level * times;
                g.sess.add_score(pts);
                let (cx, cy) = (ax + ALIEN_W as f32 / 2.0, ay + ALIEN_H as f32 / 2.0);
                g.fx.burst(cx, cy, 10, 110.0, alien_color(kind));
                break_alien(g, ax, ay, kind, g.bullets[bi].x);
                if times > 1 {
                    g.fx.popup(cx, cy - 6.0, &format!("{pts} X{times}"), BLIP_YELLOW);
                } else {
                    g.fx.popup(cx, cy - 6.0, &format!("{pts}"), alien_color(kind));
                }
                break 'rows;
            }
        }
    }
}

/// A shot alien bursts into a grid of chips in its colour, thrown out from where the shot struck. Their scatter comes
/// from a hash of where it happened, so the game's random numbers are untouched.
pub(super) fn break_alien(g: &mut Game, ax: f32, ay: f32, kind: usize, shot_x: f32) {
    let c = alien_color(kind);
    let (cols, rows) = (4, 3);
    let (cw, ch) = (ALIEN_W as f32 / cols as f32, ALIEN_H as f32 / rows as f32);
    let seed = (ax as u32).wrapping_mul(73_856_093) ^ (ay as u32).wrapping_mul(19_349_663) ^ shot_x.to_bits();
    for r in 0..rows {
        for k in 0..cols {
            let h = hash(seed ^ (r * cols + k) as u32);
            let (x, y) = (ax + (k as f32 + 0.5) * cw, ay + (r as f32 + 0.5) * ch);
            let (dx, dy) = (x - shot_x, y - (ay + ALIEN_H as f32));
            let len = dx.hypot(dy).max(1.0);
            let v = 60.0 + (h % 90) as f32;
            g.chips.push(Chip {
                x, y, vx: dx / len * v, vy: dy / len * v - 30.0,
                turn: 0.0, spin: ((h >> 8) % 13) as f32 - 6.0, ttl: CHIP_SECS, c,
            });
        }
    }
}

pub(super) fn update_chips(g: &mut Game, dt: f32) {
    for c in g.chips.iter_mut() {
        c.vy += 380.0 * dt;
        c.x += c.vx * dt;
        c.y += c.vy * dt;
        c.turn += c.spin * dt;
        c.ttl -= dt;
    }
    g.chips.retain(|c| c.ttl > 0.0);
}

/// Player bullets against the diver: double points.
pub(super) fn shoot_diver(g: &mut Game, bullet_starts: &[(f32, f32); N_BULLETS], sfx: &Sounds) {
    if let Some(d) = g.diver {
        // `bi` pairs the bullet slot with its swept start (the player slots head the array).
        for (bi, start) in bullet_starts.iter().enumerate().take(MAX_PLAYER_BULLETS) {
            if !g.bullets[bi].active { continue; }
            if bullet_hits_rect(*start, (g.bullets[bi].x, g.bullets[bi].y),
                                d.x, d.y, ALIEN_W as f32, ALIEN_H as f32) {
                g.bullets[bi].active = false;
                g.diver = None;
                blip::bot::add("divers", 1.0);
                play_sfx(&sfx.explosion);
                g.spawn_explosion(d.x, d.y);
                g.streak.hit();
                let pts = match d.kind { 0 => 30, 1 => 20, _ => 10 } * g.sess.level * 2;
                g.sess.add_score(pts);
                let (cx, cy) = (d.x + ALIEN_W as f32 / 2.0, d.y + ALIEN_H as f32 / 2.0);
                g.fx.burst(cx, cy, 14, 130.0, alien_color(d.kind));
                break_alien(g, d.x, d.y, d.kind, cx);
                g.fx.popup(cx, cy - 6.0, &format!("DIVER {pts}"), BLIP_YELLOW);
                break;
            }
        }
    }
}
