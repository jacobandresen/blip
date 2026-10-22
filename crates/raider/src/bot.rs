//! Native-only autopilot (BLIP_BOT=1): each frame tries nine stick
//! positions, projects the plane and every round 0.5 s ahead, and takes
//! the safest one that also lines the guns up.

use super::*;

pub fn drive(g: &Game, t: f32) {
    let tap = (t * 10.0) as i32 % 2 == 0;
    let keys: Vec<_> = match g.state {
        State::Title | State::Won => if tap { vec![BLIP_KEY_SPACE] } else { vec![] },
        State::Play => {
            let (pw, ph) = (PLAYER_W as f32, PLAYER_H as f32);
            let (cx, cy) = (g.player_x + pw / 2.0, g.player_y + ph / 2.0);
            let mut threats: Vec<(f32, f32, f32, f32)> = Vec::new();
            for b in g.enemy_bullets.iter().filter(|b| b.active) { threats.push((b.x, b.y, b.vx, b.vy)); }
            // flak: where each shell will burst, and bursts still lethal
            for f in g.flak.iter().filter(|f| f.active) {
                let k = f.left / f.vx.hypot(f.vy).max(1.0);
                for d in [-12.0f32, 0.0, 12.0] {
                    threats.push((f.x + f.vx * k + d, f.y + f.vy * k, 0.0, 0.0));
                }
            }
            for b in g.flak_bursts.iter().filter(|b| b.active && b.t < FLAK_LETHAL) { threats.push((b.x, b.y, 0.0, 0.0)); }
            // A charging or firing laser: its whole line is danger.
            for i in 0..g.boss.n_lasers {
                let l = &g.boss.lasers[i];
                if !g.boss.active || l.hp <= 0 || l.phase == LaserPhase::Cooling { continue; }
                let (x, y) = mount_pos(g, l.fx, l.fy);
                let (s, c) = l.angle.sin_cos();
                for d in (20..700).step_by(14) {
                    threats.push((x + s * d as f32, y + c * d as f32, 0.0, 0.0));
                }
            }
            for e in g.enemies.iter().filter(|e| e.active) {
                threats.push((e.x + ENEMY_W as f32 / 2.0, e.y + ENEMY_H as f32 / 2.0, e.heading.sin() * e.speed, e.heading.cos() * e.speed));
            }
            // Where the guns should be: under the boss, the barrier motor, or the nearest plane.
            // In a boss fight, fly the lane its gunners leave open.
            let aim_x = if g.boss.active { lane_x(g.boss.t) }
                else if g.barrier.active { g.barrier.motor_x }
                else {
                    g.enemies.iter().filter(|e| e.active && e.y < cy - 40.0)
                        .min_by(|a, b| (a.x - cx).abs().partial_cmp(&(b.x - cx).abs()).unwrap())
                        .map_or(WIN_W as f32 / 2.0, |e| e.x + ENEMY_W as f32 / 2.0)
                };
            // Pickups pull harder than the aim.
            let pick = g.powerups.iter().filter(|p| p.active).map(|p| (p.x, p.y))
                .chain(g.health_pickups.iter().filter(|p| p.active && g.health < PLAYER_HEALTH_MAX).map(|p| (p.x, p.y)))
                .next();
            let mut best = (f32::MAX, 0, 0);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    // Throttle is airspeed: stay clear of the stall.
                    if dy == 1 && g.airspeed < 0.55 { continue; }
                    let vx = dx as f32 * PLAYER_SPEED;
                    let vy = dy as f32 * 110.0;
                    let mut cost = 0.0;
                    for k in 1..=10 {
                        let s = k as f32 * 0.05;
                        let px = (cx + vx * s).clamp(pw / 2.0, WIN_W as f32 - pw / 2.0);
                        let py = (cy + vy * s).clamp(PLAYER_MIN_Y + ph / 2.0, PLAYER_MAX_Y + ph / 2.0);
                        for &(x, y, bvx, bvy) in &threats {
                            let (ex, ey) = (x + bvx * s - px, y + bvy * s - py);
                            let d2 = ex * ex / (22.0 * 22.0) + ey * ey / (20.0 * 20.0);
                            cost += (-d2).exp() * (1.2 - s);
                        }
                        if g.barrier.active {
                            let d = (g.barrier.y - py) / 30.0;
                            cost += (-d * d).exp() * 0.8;
                        }
                    }
                    let px = cx + vx * 0.3;
                    let py = cy + vy * 0.3;
                    let (tx, ty) = pick.unwrap_or((aim_x, WIN_H as f32 - 150.0));
                    cost += ((px - tx).abs() / WIN_W as f32) * 0.8 + ((py - ty).abs() / WIN_H as f32) * 0.3;
                    if cost < best.0 { best = (cost, dx, dy); }
                }
            }
            let mut k = vec![BLIP_KEY_SPACE];
            if best.1 < 0 { k.push(BLIP_KEY_LEFT); } else if best.1 > 0 { k.push(BLIP_KEY_RIGHT); }
            // BLIP_BOT_GLIDE=1: ease off for a stretch every eight seconds, to watch slow flight.
            let glide = std::env::var_os("BLIP_BOT_GLIDE").is_some() && t % 8.0 > 5.0 && g.airspeed > 0.42;
            if glide { k.push(BLIP_KEY_DOWN); }
            else if best.2 < 0 { k.push(BLIP_KEY_UP); } else if best.2 > 0 { k.push(BLIP_KEY_DOWN); }
            k
        }
        _ => vec![],
    };
    blip::bot::hold(&keys);
}
