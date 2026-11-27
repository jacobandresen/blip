//! The plush monsters: walkers, hoppers and ghosts, how they move, hop and throw.

use super::*;

pub(super) fn nearest_player(g: &Game, x: f32, y: f32) -> Option<(f32, f32)> {
    g.p.iter()
        .filter(|p| p.joined && p.alive && p.dead_t <= 0.0)
        .map(|p| (p.cx(), p.cy()))
        .min_by(|a, b| ((a.0 - x).hypot(a.1 - y)).total_cmp(&(b.0 - x).hypot(b.1 - y)))
}

/// Returns whether a toy crouched to hop this frame (its squeak is the jump's tell).
pub(super) fn move_toys(g: &mut Game, dt: f32) -> bool {
    let mut hopped = false;
    for i in 0..g.enemies.len() {
        let mut e = g.enemies[i];
        if !e.active { continue; }
        e.t += dt;
        e.pop_in = (e.pop_in + dt * 3.0).min(1.0);
        let fast = if e.angry { 1.35 } else { 1.0 } * monster_pace(g.round);
        let target = nearest_player(g, e.x + E_W / 2.0, e.y + E_H / 2.0);
        // A jump is told before it happens: the monster crouches, still,
        // for JUMP_TELL, then springs.
        if e.crouch > 0.0 {
            e.crouch -= dt;
            e.vx = 0.0;
            if e.crouch <= 0.0 {
                (e.vx, e.vy) = e.leap;
                e.on_ground = false;
            }
        }
        hopped |= match e.kind {
            Kind::Walker => walk(g, &mut e, dt, fast, target),
            Kind::Hopper => hop(g, &mut e, dt, fast, target),
            Kind::Ghost => { float(&mut e, dt, fast); false }
        };
        throw(g, &mut e, dt, fast, target);
        g.enemies[i] = e;
    }
    hopped
}

/// Moves a toy's body through the level; whether it walked into a wall.
pub(super) fn step_body(g: &Game, e: &mut Enemy, dt: f32) -> bool {
    let (ground, wall) = g.move_body(&mut e.x, &mut e.y, &mut e.vx, &mut e.vy, E_W, E_H, dt);
    e.on_ground = ground;
    wall
}

/// A walker strolls its platform, now and then turning back at an edge, and hops up toward a player above it;
/// whether it crouched to hop.
pub(super) fn walk(g: &Game, e: &mut Enemy, dt: f32, fast: f32, target: Option<(f32, f32)>) -> bool {
    let mut hopped = false;
    if e.on_ground && e.crouch <= 0.0 {
        e.vx = if e.windup > 0.0 { 0.0 } else { e.dir * 58.0 * fast };
        e.edge_cd -= dt;
        let ahead = if e.dir > 0.0 { e.x + E_W + 2.0 } else { e.x - 2.0 };
        if e.edge_cd <= 0.0 && !g.floor_at(ahead, ahead + 1.0, e.y + E_H) {
            e.edge_cd = 0.6;
            if blip::rand_range_f32(0.0, 1.0) < 0.55 {
                e.dir = -e.dir;
                e.vx = -e.vx;
            }
        }
        e.jump_cd -= dt;
        if e.jump_cd <= 0.0 {
            e.jump_cd = blip::rand_range_f32(1.0, 2.4) / fast;
            if let Some((px, py)) = target {
                if py < e.y - 30.0 && (px - e.x).abs() < 150.0 && blip::rand_range_f32(0.0, 1.0) < 0.7 {
                    e.crouch = JUMP_TELL;
                    hopped = true;
                    e.leap = (e.dir * 58.0 * fast, -JUMP_V);
                } else if blip::rand_range_f32(0.0, 1.0) < 0.15 {
                    e.dir = if px > e.x { 1.0 } else { -1.0 };
                }
            }
        }
    }
    if step_body(g, e, dt) { e.dir = -e.dir; }
    hopped
}

/// A hopper bounds about, mostly toward the nearest player, high when they are above it; whether it crouched to hop.
pub(super) fn hop(g: &Game, e: &mut Enemy, dt: f32, fast: f32, target: Option<(f32, f32)>) -> bool {
    let mut hopped = false;
    if e.on_ground && e.crouch <= 0.0 {
        e.vx *= 0.8;
        e.jump_cd -= dt;
        if e.jump_cd <= 0.0 {
            e.jump_cd = blip::rand_range_f32(0.6, 1.3) / fast;
            let toward = target.map(|(px, _)| if px > e.x { 1.0 } else { -1.0 }).unwrap_or(e.dir);
            e.dir = if blip::rand_range_f32(0.0, 1.0) < 0.75 { toward } else { -toward };
            let high = target.map(|(_, py)| py < e.y - 20.0).unwrap_or(false) || blip::rand_range_f32(0.0, 1.0) < 0.3;
            e.crouch = JUMP_TELL;
            hopped = true;
            e.leap = (e.dir * 85.0 * fast, if high { -JUMP_V * 0.98 } else { -300.0 });
        }
    }
    if step_body(g, e, dt) {
        e.dir = -e.dir;
        e.vx = -e.vx;
    }
    hopped
}

/// A ghost floats on the slant, bobbing, and bounces off the walls, floor and ceiling.
pub(super) fn float(e: &mut Enemy, dt: f32, fast: f32) {
    let sp = 62.0 * fast;
    e.vx = e.vx.signum() * sp;
    e.vy = e.vy.signum() * sp * 0.8;
    e.x += e.vx * dt;
    e.y += e.vy * dt + (e.t * 3.0).sin() * 12.0 * dt;
    let (x_max, y_max) = (WIN_W as f32 - TILE - E_W, WIN_H as f32 - TILE - E_H);
    if e.x < TILE { e.x = TILE; e.vx = e.vx.abs(); }
    if e.x > x_max { e.x = x_max; e.vx = -e.vx.abs(); }
    if e.y < HUD + TILE { e.y = HUD + TILE; e.vy = e.vy.abs(); }
    if e.y > y_max { e.y = y_max; e.vy = -e.vy.abs(); }
}

/// Walkers wind up before rolling rocks at aligned players; angry ghosts aim slow sparks at the nearest player.
pub(super) fn throw(g: &mut Game, e: &mut Enemy, dt: f32, fast: f32, target: Option<(f32, f32)>) {
    let (ex, ey) = (e.x + E_W / 2.0, e.y + E_H / 2.0);
    let shot = |x, y, vx, vy, spark| Shot { x, y, vx, vy, t: 0.0, spark, active: true };
    e.shot_cd -= dt;
    if e.windup > 0.0 {
        e.windup -= dt;
        if e.windup <= 0.0 { g.shots.push(shot(ex + e.dir * 12.0, ey + 2.0, e.dir * ROCK_V, 0.0, false)); }
        return;
    }
    let Some((px, py)) = target else { return };
    if e.shot_cd > 0.0 || e.pop_in < 1.0 { return; }
    if e.kind == Kind::Walker && g.round >= ROCKS_FROM && e.on_ground && (py - ey).abs() < 14.0 && (px - ex).abs() < 300.0 {
        e.dir = if px > ex { 1.0 } else { -1.0 };
        e.windup = ROCK_WINDUP;
        e.shot_cd = blip::rand_range_f32(2.2, 4.0) / fast;
    } else if e.kind == Kind::Ghost && g.round >= SPARKS_FROM && e.angry {
        let (dx, dy) = (px - ex, py - ey);
        let d = dx.hypot(dy).max(1.0);
        g.shots.push(shot(ex, ey, dx / d * SPARK_V, dy / d * SPARK_V, true));
        e.shot_cd = blip::rand_range_f32(3.0, 5.0) / fast;
    } else {
        e.shot_cd = 0.4;
    }
}
