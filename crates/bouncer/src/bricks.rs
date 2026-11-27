//! The wall: where each brick is, a brick breaking (points, shards, drops, a bomb's blast), and the ball against the
//! bricks.

use super::*;

/// Crack a brick at `(bx, by)` from the impact point `hit` into 3 to 5 shards: cuts run out from the impact at
/// uneven angles, and each shard is the piece of the rectangle between two cuts. Shards fly out from the impact,
/// pushed along the ball's travel.
pub(super) fn shatter(bx: f32, by: f32, hit: (f32, f32), push: (f32, f32), base: BlipColor, rng: &mut ShardRng) -> Vec<Shard> {
    let (w, h) = (BRICK_W as f32, BRICK_H as f32);
    // 3 to 5 pieces, drifting rather than flung: 3 to 7 flung at up to 140 px/s, spinning, read as too busy (user, 2026-11-27).
    let n = 3 + (rng.next_f32() * 3.0) as usize;
    // Cuts jitter within the middle of their share of the circle: wider jitter left slivers that read as lines.
    let mut cuts: Vec<f32> = (0..n).map(|k| (k as f32 + 0.35 + rng.next_f32() * 0.3) / n as f32 * std::f32::consts::TAU).collect();
    cuts.sort_by(f32::total_cmp);
    // Where a ray from the impact leaves the rectangle.
    let exit = |a: f32| {
        let (dx, dy) = (a.cos(), a.sin());
        let tx = if dx > 0.0 { (bx + w - hit.0) / dx } else if dx < 0.0 { (bx - hit.0) / dx } else { f32::MAX };
        let ty = if dy > 0.0 { (by + h - hit.1) / dy } else if dy < 0.0 { (by - hit.1) / dy } else { f32::MAX };
        let t = tx.min(ty);
        (hit.0 + dx * t, hit.1 + dy * t)
    };
    let corners = [(bx + w, by + h), (bx, by + h), (bx, by), (bx + w, by)];
    let corner_angle = |c: (f32, f32)| (c.1 - hit.1).atan2(c.0 - hit.0).rem_euclid(std::f32::consts::TAU);
    let mut out = Vec::new();
    for k in 0..n {
        let (a0, a1) = (cuts[k], if k + 1 < n { cuts[k + 1] } else { cuts[0] + std::f32::consts::TAU });
        let mut poly = vec![hit, exit(a0)];
        let mut inside: Vec<(f32, f32)> = corners.iter().copied().filter(|&c| {
            let a = corner_angle(c);
            (a > a0 && a < a1) || (a + std::f32::consts::TAU > a0 && a + std::f32::consts::TAU < a1)
        }).collect();
        inside.sort_by(|p, q| {
            let ap = (corner_angle(*p) - a0).rem_euclid(std::f32::consts::TAU);
            let aq = (corner_angle(*q) - a0).rem_euclid(std::f32::consts::TAU);
            ap.total_cmp(&aq)
        });
        poly.extend(inside);
        poly.push(exit(a1));
        let (cx, cy) = poly.iter().fold((0.0, 0.0), |s, p| (s.0 + p.0, s.1 + p.1));
        let (cx, cy) = (cx / poly.len() as f32, cy / poly.len() as f32);
        let mid = (a0 + a1) / 2.0;
        let fling = 25.0 + rng.next_f32() * 35.0;
        let light = 0.75 + rng.next_f32() * 0.4;
        out.push(Shard {
            x: cx, y: cy,
            vx: mid.cos() * fling + push.0 * 0.06, vy: mid.sin() * fling + push.1 * 0.06 - 20.0,
            turn: 0.0, spin: (rng.next_f32() - 0.5) * 6.0,
            pts: poly.iter().map(|p| (p.0 - cx, p.1 - cy)).collect(),
            color: BlipColor { r: (base.r * light).min(1.0), g: (base.g * light).min(1.0), b: (base.b * light).min(1.0), a: 1.0 },
        });
    }
    out
}

/// The top-left corner of brick `i`.
pub(super) fn brick_rect(i: usize) -> (f32, f32) {
    let (row, col) = (i as i32 / BRICK_COLS, i as i32 % BRICK_COLS);
    ((BRICK_OX + col * (BRICK_W + BRICK_GAP)) as f32, (BRICK_OY + row * (BRICK_H + BRICK_GAP)) as f32)
}

/// Brick `i` breaks: the points (see CHAIN_MAX), the sparks, the ball a
/// little faster, perhaps a drop. A bomb takes its eight neighbours with it
/// (`blasted`: this one went that way).
pub(super) fn break_brick(g: &mut Game, i: usize, speed: f32, sfx: &Sounds, blasted: bool) {
    let (row, col) = (i as i32 / BRICK_COLS, i as i32 % BRICK_COLS);
    let (bx, by) = brick_rect(i);
    let (mx, my) = (bx + BRICK_W as f32 / 2.0, by + BRICK_H as f32 / 2.0);
    g.bricks[i].alive = false;
    g.since_break = 0.0;
    blip::bot::add(if blasted { "bricks_blasted" } else { "bricks" }, 1.0);
    g.chain += 1;
    let times = g.chain.min(CHAIN_MAX);
    let points = (BRICK_ROWS - row) * 10 * g.sess.level * times;
    g.sess.add_score(points);
    // The brick shatters from where the ball struck it (its centre when blasted), and its neighbours shake.
    let hit = if blasted { (mx, my) } else {
        ((g.ball_x + BALL_W as f32 / 2.0).clamp(bx + 2.0, bx + BRICK_W as f32 - 2.0),
         (g.ball_y + BALL_H as f32 / 2.0).clamp(by + 2.0, by + BRICK_H as f32 - 2.0))
    };
    let mut rng = ShardRng((i as u32 + 1).wrapping_mul(2654435761) ^ (g.sess.score as u32).wrapping_mul(40503) ^ g.ball_x.to_bits() | 1);
    let shards = shatter(bx, by, hit, (g.ball_vx, g.ball_vy), BRICK_COLORS[g.bricks[i].kind], &mut rng);
    g.shards.extend(shards);
    for (dr, dc) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let (r, c) = (row + dr, col + dc);
        if (0..BRICK_ROWS).contains(&r) && (0..BRICK_COLS).contains(&c) { g.bricks[(r * BRICK_COLS + c) as usize].wobble = 1.0; }
    }
    // Only a multiplied score earns a label; plain points show in the score.
    if times > 1 {
        g.fx.popup(mx, my, &format!("{points} X{times}"), BLIP_YELLOW);
    }
    // Same ramp on every level, so the ball plays identically no
    // matter how far the player has gotten.
    g.ball_speed = clamp(g.ball_speed + SPEED_INC, 0.0, BALL_SPEED_MAX);

    maybe_drop(g, bx, by);
    // The blast has one sound of its own: nine bricks at once would clip.
    if g.bricks[i].kind == BRICK_BOMB {
        play_sfx(&sfx.bomb);
        blast(g, i, speed, sfx);
    } else if !blasted {
        play_variant(&sfx.brick_break, speed);
    }
}

/// 30% chance to spawn a loot drop from the brick at (`bx`, `by`); one in ten of those a life
/// (at one in five a steady player ended level 1 with six).
fn maybe_drop(g: &mut Game, bx: f32, by: f32) {
    if rand() % 10 >= 3 { return; }
    let drop_x = bx + BRICK_W as f32 / 2.0 - DROP_W / 2.0;
    let drop_kind = match rand() % 10 {
        0..=1 => DropKind::Wide,
        2..=3 => DropKind::Slow,
        4..=5 => DropKind::Multi,
        6..=8 => DropKind::Narrow,
        _ => DropKind::Life,
    };
    pool_spawn(&mut g.drops, Drop { x: drop_x, y: by, active: true, kind: drop_kind });
}

/// Bomb brick `i` goes off: a ring and a spray, and the eight bricks round it break too (and set off any bomb among them).
fn blast(g: &mut Game, i: usize, speed: f32, sfx: &Sounds) {
    let (row, col) = (i as i32 / BRICK_COLS, i as i32 % BRICK_COLS);
    let (bx, by) = brick_rect(i);
    let (mx, my) = (bx + BRICK_W as f32 / 2.0, by + BRICK_H as f32 / 2.0);
    g.fx.ring(mx, my, BRICK_W as f32 * 1.4, 0.35, BRICK_COLORS[BRICK_BOMB]);
    g.fx.burst(mx, my, 18, 200.0, BRICK_COLORS[BRICK_BOMB]);
    for (dr, dc) in [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)] {
        let (r, c) = (row + dr, col + dc);
        if !(0..BRICK_ROWS).contains(&r) || !(0..BRICK_COLS).contains(&c) { continue; }
        let j = (r * BRICK_COLS + c) as usize;
        if g.bricks[j].alive { break_brick(g, j, speed, sfx, true); }
    }
}

/// The ball's centre is buried in the brick at (`bx`, `by`): it is ejected along the shallowest axis.
fn buried_normal(cx: f32, cy: f32, bx: f32, by: f32) -> (f32, f32) {
    let ox = (cx - bx).min(bx + BRICK_W as f32 - cx);
    let oy = (cy - by).min(by + BRICK_H as f32 - cy);
    if ox < oy {
        (if cx < bx + BRICK_W as f32 / 2.0 { -1.0 } else { 1.0 }, 0.0)
    } else {
        (0.0, if cy < by + BRICK_H as f32 / 2.0 { -1.0 } else { 1.0 })
    }
}

/// Reflect the ball about the contact normal, lift it `pen` clear of the brick, and put its speed back exactly on the
/// ramp.
fn bounce_off(g: &mut Game, (nx, ny): (f32, f32), pen: f32, speed: f32) {
    let vn = g.ball_vx * nx + g.ball_vy * ny;
    if vn < 0.0 {
        g.ball_vx -= 2.0 * vn * nx;
        g.ball_vy -= 2.0 * vn * ny;
    }
    g.ball_x += nx * pen;
    g.ball_y += ny * pen;
    let m = g.ball_vx.hypot(g.ball_vy).max(1.0);
    g.ball_vx = g.ball_vx / m * speed;
    g.ball_vy = g.ball_vy / m * speed;
}

/// Resolve one ball/brick contact using the closest-point normal for corner hits.
pub(super) fn hit_bricks(g: &mut Game, speed: f32, sfx: &Sounds) {
    let (cx, cy, rad) = ball_circle(g);

    for i in 0..BRICK_TOTAL {
        if !g.bricks[i].alive {
            continue;
        }
        let (bx, by) = brick_rect(i);

        let px = cx.clamp(bx, bx + BRICK_W as f32);
        let py = cy.clamp(by, by + BRICK_H as f32);
        let (dx, dy) = (cx - px, cy - py);
        let d2 = dx * dx + dy * dy;
        if d2 >= rad * rad {
            continue;
        }

        let d = d2.sqrt();
        let normal = if d > 0.001 { (dx / d, dy / d) } else { buried_normal(cx, cy, bx, by) };
        bounce_off(g, normal, rad - d + 0.5, speed);

        g.bricks[i].hp -= 1;
        let (mx, my) = (bx + BRICK_W as f32 / 2.0, by + BRICK_H as f32 / 2.0);
        if g.bricks[i].hp == 0 {
            break_brick(g, i, speed, sfx, false);
        } else {
            // Steel brick survived — the cracked texture warns the next hit
            // finishes it.
            g.bricks[i].kind = BRICK_STEEL_CRACKED;
            g.bricks[i].wobble = 1.0;
            blip::bot::add("steel_cracked", 1.0);
            g.fx.burst(mx, my, 5, 70.0, BRICK_COLORS[BRICK_STEEL]);
            play_variant(&sfx.brick_hit, speed);
        }
        return;
    }
}
