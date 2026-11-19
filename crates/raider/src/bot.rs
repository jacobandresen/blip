//! How to play Raider: keep the guns firing; each frame try the nine stick
//! positions, fly the plane and every enemy round, flak burst, laser and
//! plane (and the rest of each burst) 0.8 s ahead, and take the safest move that also lines the
//! guns up (under the boss's open lane, the barrier motor or the nearest
//! plane) or fetches a pickup. Never throttle back into a stall.
//! `BLIP_BOT_GOD=1` counts hits without damage, to reach the late game.

use super::*;
use blip::bot::rollout::Mover;
use blip::bot::{best, Goal::*, Pad, Playthrough};

/// What the bot must show it can do in Raider.
pub const PLAYTHROUGHS: &[Playthrough] = &[
    Playthrough {
        name: "the plane takes off from the carrier and shoots down a flight",
        goals: &[AtLeast("planes", 10.0), AtLeast("flights_down", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it catches power-ups to arm more guns",
        seconds: 240.0,
        goals: &[AtLeast("powerups", 2.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it sinks a patrol boat",
        seconds: 240.0,
        goals: &[AtLeast("boats", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it brings down the first boss",
        seconds: 300.0,
        goals: &[AtLeast("bosses", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "after game over it continues on the same level",
        level: 3,
        settings: &[("CONTINUES", "1")],
        seconds: 480.0,
        // Still on level 3 or beyond after the continue: a continue does not send the plane back to level 1.
        goals: &[AtLeast("continues", 1.0), AtLeast("deaths", 4.0), AtLeast("level", 3.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it shoots out a laser barrier's motor (invulnerable)",
        level: BARRIER_MIN_LEVEL as u32,
        settings: &[("GOD", "1")],
        seconds: 300.0,
        goals: &[AtLeast("barriers", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it fights inland and beats the last boss (invulnerable)",
        level: MAX_LEVEL as u32,
        settings: &[("GOD", "1")],
        seconds: 480.0,
        goals: &[Ends("won")],
        ..Playthrough::DEFAULT
    },
];

pub fn drive(g: &Game) {
    let mut pad = Pad::new();
    match g.state {
        State::Title | State::Won => { pad.press_start(); }
        State::Over if blip::bot::may_continue() => { pad.tap_fire(); }
        // Like a player who has seen the take-off before: fire hurries it.
        State::Launch => { pad.fire(); }
        State::Play => {
            hit_boxes(g);
            pad.fire();
            let (dx, dy) = best_move(g);
            pad.steer(0.0, dx as f32, 0.5);
            // BLIP_BOT_GLIDE=1: ease off for a stretch every eight seconds, to watch slow flight.
            let glide = blip::bot::flag("GLIDE") && blip::bot::clock() % 8.0 > 5.0 && g.airspeed > 0.42;
            if glide { pad.down(); } else { pad.climb(0.0, dy as f32, 0.5); }
        }
        _ => {}
    }
    pad.send();
}

/// The hit boxes the game tests, for the fun meter: the plane's, every enemy round (2 px each way), flak shells in
/// flight (the tell of a burst) and bursts still lethal (the box round their circle), planes (their middle 60 %)
/// and the boss's body.
fn hit_boxes(g: &Game) {
    // Nothing can hit the plane while it blinks after a hit or a respawn.
    if g.respawn_grace.active() { return; }
    let (hx, hy) = plane_hit_box(g.player_x, g.player_y);
    let me = Mover::still(hx + PLAYER_HIT.0 / 2.0, hy + PLAYER_HIT.1 / 2.0).sized(PLAYER_HIT.0 / 2.0, PLAYER_HIT.1 / 2.0);
    let mut hazards: Vec<Mover> = g.enemy_bullets.iter().filter(|b| b.active).map(|b| Mover::still(b.x, b.y).sized(2.0, 2.0)).collect();
    hazards.extend(g.flak.iter().filter(|f| f.active).map(|f| Mover::still(f.x, f.y).sized(3.0, 3.0)));
    hazards.extend(g.flak_bursts.iter().filter(|b| b.active && b.t < FLAK_LETHAL).map(|b| Mover::still(b.x, b.y).sized(FLAK_BURST_R, FLAK_BURST_R)));
    let (ew, eh) = (ENEMY_W as f32, ENEMY_H as f32);
    hazards.extend(g.enemies.iter().filter(|e| e.active).map(|e| Mover::still(e.x + ew / 2.0, e.y + eh / 2.0).sized(ew * 0.3, eh * 0.3)));
    if g.boss.active {
        let (bw, bh) = boss_size(g.boss.tier);
        hazards.push(Mover::still(g.boss.x + bw / 2.0, g.boss.y + bh / 2.0).sized(bw * 0.3, bh * 0.35));
    }
    blip::bot::near(&[me], &hazards);
}

/// The plane's centre.
fn plane(g: &Game) -> (f32, f32) {
    (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0)
}

/// Enemy rounds and the rest of every burst being fired, where each flak
/// shell will burst and bursts still lethal, every point along a live laser,
/// and the planes themselves.
fn threats(g: &Game) -> Vec<Mover> {
    let mut threats: Vec<Mover> = g.enemy_bullets.iter().filter(|b| b.active).map(|b| Mover::moving(b.x, b.y, b.vx, b.vy)).collect();
    for e in g.enemies.iter().filter(|e| e.active) {
        let (cx, cy) = (e.x + ENEMY_W as f32 / 2.0, e.y + ENEMY_H as f32 / 2.0);
        let (fx, fy) = (e.heading.sin(), e.heading.cos());
        // A burst fires a pair of rounds along the nose every 0.08 s.
        for k in 0..e.burst {
            let from = e.burst_t.max(0.0) + k as f32 * 0.08;
            let (gx, gy) = (cx + fx * (16.0 + e.speed * from), cy + fy * (16.0 + e.speed * from));
            for side in [-15.0f32, 15.0] {
                threats.push(Mover::moving(gx - fy * side, gy + fx * side, fx * ENEMY_BULLET_SPEED, fy * ENEMY_BULLET_SPEED).from(from));
            }
        }
        threats.push(Mover::moving(cx, cy, fx * e.speed, fy * e.speed));
    }
    for f in g.flak.iter().filter(|f| f.active) {
        let k = f.left / f.vx.hypot(f.vy).max(1.0);
        for d in [-12.0f32, 0.0, 12.0] { threats.push(Mover::still(f.x + f.vx * k + d, f.y + f.vy * k)); }
    }
    threats.extend(g.flak_bursts.iter().filter(|b| b.active && b.t < FLAK_LETHAL).map(|b| Mover::still(b.x, b.y)));
    for l in g.boss.lasers[..g.boss.n_lasers].iter() {
        if !g.boss.active || l.hp <= 0 || l.phase == LaserPhase::Cooling { continue; }
        let (x, y) = mount_pos(g, l.fx, l.fy);
        let (s, c) = l.angle.sin_cos();
        threats.extend((20..700).step_by(14).map(|d| Mover::still(x + s * d as f32, y + c * d as f32)));
    }
    threats
}

/// Where the plane wants to be: on a pickup, else with its guns under the
/// boss's open lane, the barrier motor or the nearest plane ahead.
fn goal(g: &Game) -> (f32, f32) {
    let pickup = g.powerups.iter().filter(|p| p.active).map(|p| (p.x, p.y))
        .chain(g.health_pickups.iter().filter(|p| p.active && g.health < PLAYER_HEALTH_MAX).map(|p| (p.x, p.y)))
        .next();
    if let Some(at) = pickup { return at; }
    let (cx, cy) = plane(g);
    let aim_x = if g.boss.active { lane_x(g.boss.t) }
        else if g.barrier.active { g.barrier.motor_x }
        else {
            best(g.enemies.iter().filter(|e| e.active && e.y < cy - 40.0), |e| (e.x - cx).abs())
                .map_or(WIN_W as f32 / 2.0, |e| e.x + ENEMY_W as f32 / 2.0)
        };
    (aim_x, WIN_H as f32 - 150.0)
}

/// A stick position: -1, 0 or 1 sideways, and throttle (up is -1 on the stick, +1 throttle).
type Stick = (i32, i32);

/// Steps of the look-ahead flown on the first stick position before the second takes over.
const SWITCH: usize = 6;

/// Where the plane's centre will be every 0.05 s for 0.8 s with the stick held at `first`, then from
/// `SWITCH` steps on at `then`, by the game's own flight model: sideways speed spools up, and climbing is
/// airspeed gaining on the scroll.
fn fly_ahead(g: &Game, first: Stick, then: Stick) -> [(f32, f32); 16] {
    let (pw, ph) = (PLAYER_W as f32, PLAYER_H as f32);
    let (mut x, mut y) = plane(g);
    let (mut vx, mut vy, mut airspeed) = (g.player_vx, g.player_vy, g.airspeed);
    let dt = 0.05;
    std::array::from_fn(|k| {
        let (dx, throttle) = if k < SWITCH { first } else { then };
        vx = blip::spool(vx, dx as f32 * PLAYER_SPEED, PLAYER_ACCEL, PLAYER_BRAKE, dt);
        airspeed = (airspeed + throttle as f32 * AIRSPEED_RATE * dt).clamp(0.0, AIRSPEED_MAX);
        if throttle == 0 { airspeed += (1.0 - airspeed).clamp(-AIRSPEED_RELAX * dt, AIRSPEED_RELAX * dt); }
        vy += ((1.0 - airspeed) * AIRSPEED_DRIFT - vy) * (6.0 * dt).min(1.0);
        x = (x + vx * dt).clamp(pw / 2.0, WIN_W as f32 - pw / 2.0);
        y = (y + vy * dt).clamp(PLAYER_MIN_Y + ph / 2.0, PLAYER_MAX_Y + ph / 2.0);
        (x, y)
    })
}

/// The stick position (-1, 0 or 1 each way; up is -1) that keeps clearest of the threats over the next
/// 0.8 s and heads for the goal, allowing one change of stick partway (a dodge, then back on course).
fn best_move(g: &Game) -> (i32, i32) {
    let threats = threats(g);
    let (tx, ty) = goal(g);
    let risk = |(dx, dy): Stick, (dx2, dy2): Stick| {
        let path = fly_ahead(g, (dx, -dy), (dx2, -dy2));
        let mut cost = 0.0;
        for (k, &(px, py)) in path.iter().enumerate() {
            let s = (k + 1) as f32 * 0.05;
            for t in &threats {
                let Some((x, y)) = t.at(s) else { continue };
                let (ex, ey) = (x - px, y - py);
                cost += (-(ex * ex / (22.0 * 22.0) + ey * ey / (20.0 * 20.0))).exp() * (1.2 - s);
                // Inside the hit box, with a margin: a hit, not a risk.
                if ex.abs() < PLAYER_HIT.0 / 2.0 + 8.0 && ey.abs() < PLAYER_HIT.1 / 2.0 + 8.0 { cost += 10.0 * (1.2 - s); }
            }
            if g.barrier.active {
                let d = (g.barrier.y - py) / 30.0;
                cost += (-d * d).exp() * 0.8;
            }
        }
        let (px, py) = path[5];
        cost + ((px - tx).abs() / WIN_W as f32) * 0.8 + ((py - ty).abs() / WIN_H as f32) * 0.3
    };
    // Throttle is airspeed: no slowing down near a stall.
    let moves = (-1..=1).flat_map(|dx| (-1..=1).map(move |dy| (dx, dy))).filter(|&(_, dy)| !(dy == 1 && g.airspeed < 0.55));
    let moves: Vec<Stick> = moves.collect();
    let plan = |m: Stick| moves.iter().map(|&n| risk(m, n)).fold(f32::INFINITY, f32::min);
    best(moves.iter().copied(), plan).unwrap_or((0, 0))
}
