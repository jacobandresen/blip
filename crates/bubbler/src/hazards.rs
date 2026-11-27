//! The monsters' shots (rocks that roll, sparks that drift through walls) and the skull that hunts the players when
//! a round runs long.

use super::*;

/// Rocks roll until they hit a block or a player's bubble; sparks drift
/// through walls and fade after a few seconds.
pub(super) fn move_shots(g: &mut Game, dt: f32, player_starts: [(f32, f32); 2]) -> [bool; 2] {
    let mut player_hits = [false; 2];
    for i in 0..g.shots.len() {
        let mut s = g.shots[i];
        let from = (s.x, s.y);
        s.t += dt;
        s.x += s.vx * dt;
        s.y += s.vy * dt;
        let expired =
            (s.spark && s.t > 4.0) || s.x < -10.0 || s.x > WIN_W as f32 + 10.0 || s.y < HUD || s.y > WIN_H as f32;
        if !s.spark {
            let (c, r) = (((s.x + s.vx.signum() * 5.0) / TILE).floor() as i32, ((s.y - HUD) / TILE).floor() as i32);
            if g.platform(c, r) || c <= 0 || c >= COLS as i32 - 1 {
                s.active = false;
            }
        }
        // a bubble you blow knocks it out of the air
        if s.active
            && g.bubbles.iter().any(|b| {
                b.active
                    && b.phase == Phase::Shoot
                    && b.trapped.is_none()
                    && (b.x - s.x).hypot(b.y - s.y) < b.radius() + 5.0
            })
        {
            s.active = false;
        }
        if s.active {
            for pi in 0..2 {
                let p = &g.p[pi];
                if !p.joined || !p.alive || p.dead_t > 0.0 || p.safe > 0.0 {
                    continue;
                }
                if blip::sweep_hits(from, (s.x, s.y), player_starts[pi], (p.cx(), p.cy()),
                    (-P_W / 2.0 - SHOT_REACH.0, -P_H / 2.0 - SHOT_REACH.1, P_W + 2.0 * SHOT_REACH.0, P_H + 2.0 * SHOT_REACH.1)) {
                    player_hits[pi] = true;
                    s.active = false;
                    break;
                }
            }
        }
        if expired {
            s.active = false;
        }
        if !s.active {
            let (x, y) = (s.x, s.y);
            g.burst(x, y, 5, rgba(1.0, 0.85, 0.6, 0.8), 70.0, s.spark);
        }
        g.shots[i] = s;
    }
    g.shots.retain(|s| s.active);
    player_hits
}

pub(super) fn update_skull(g: &mut Game, dt: f32) -> Option<((f32, f32), (f32, f32))> {
    if !g.skull.active {
        if g.round_t >= skull_at(g.round) {
            g.skull = Skull { active: true, x: WIN_W as f32 / 2.0, y: HUD + 10.0, speed: 45.0 };
            g.stats.skulls += 1;
        }
        return None;
    }
    let from = (g.skull.x, g.skull.y);
    g.skull.speed += (4.0 + 1.5 * g.round as f32) * dt;
    if let Some((px, py)) = nearest_player(g, g.skull.x, g.skull.y) {
        let (dx, dy) = (px - g.skull.x, py - g.skull.y);
        let d = dx.hypot(dy).max(1.0);
        g.skull.x += dx / d * g.skull.speed * dt;
        g.skull.y += dy / d * g.skull.speed * dt;
        return Some((from, (g.skull.x, g.skull.y)));
    }
    None
}

pub(super) fn shot_hits_player(s: &Shot, x: f32, y: f32) -> bool {
    (s.x - x).abs() < P_W / 2.0 + SHOT_REACH.0 && (s.y - y).abs() < P_H / 2.0 + SHOT_REACH.1
}
