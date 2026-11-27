//! Enemy aircraft: formations sent in flights, flown along their legs, and the wingmen who hold station.

use super::*;

pub(super) fn trim_speed(kind: EnemyKind) -> f32 {
    match kind {
        EnemyKind::Grunt  => 115.0, // A6M Zero
        EnemyKind::Weaver => 125.0, // Ki-43
        EnemyKind::Ace    => 110.0, // Ki-84, flown by someone who knows how
    }
}

#[derive(Copy, Clone)]
pub(super) enum Formation { Vic, Pair, Echelon, Abreast, FingerFour, Solo }

/// Slots (right, behind) in the leader's frame, px: wingtips just clear
/// for ENEMY_W = 62.
pub(super) fn slots(f: Formation) -> &'static [(f32, f32)] {
    match f {
        Formation::Vic     => &[(0.0, 0.0), (-62.0, 50.0), (62.0, 50.0)],
        Formation::Pair    => &[(0.0, 0.0), (67.0, 45.0)],
        Formation::Echelon => &[(0.0, 0.0), (62.0, 48.0), (124.0, 95.0)],
        Formation::Abreast => &[(-74.0, 0.0), (0.0, 0.0), (74.0, 0.0)],
        Formation::FingerFour => &[(0.0, 0.0), (-62.0, 48.0), (67.0, 41.0), (129.0, 86.0)],
        Formation::Solo    => &[(0.0, 0.0)],
    }
}

/// Where a slot sits in the world for a flight point and heading.
pub(super) fn slot_pos(fx: f32, fy: f32, heading: f32, slot: (f32, f32)) -> (f32, f32) {
    let (fwd_x, fwd_y) = (heading.sin(), heading.cos());
    let (right_x, right_y) = (-heading.cos(), heading.sin());
    (fx + right_x * slot.0 - fwd_x * slot.1, fy + right_y * slot.0 - fwd_y * slot.1)
}

pub(super) fn active_enemies(g: &Game) -> usize { pool_iter(&g.enemies).count() }

/// Put a formation in the air at (x, y) flying `heading` along `legs`.
pub(super) fn spawn_flight(g: &mut Game, kind: EnemyKind, form: Formation, x: f32, y: f32, heading: f32, legs: &[Leg]) {
    let members = slots(form);
    if active_enemies(g) + members.len() > MAX_ENEMIES { return; }
    let Some(fi) = (0..MAX_FLIGHTS).find(|&i| !g.flights[i].active && !g.enemies.iter().any(|e| e.active && e.flight == i)) else { return };
    let mut plan = [Leg::Straight(6.0); 4];
    for (i, l) in legs.iter().take(4).enumerate() { plan[i] = *l; }
    let trim = trim_speed(kind);
    // In a level turn the whole formation turns at one rate, so a wingman
    // `lat` inside flies at v(1 - lat/r). Keeping that above stall needs
    // r >= lat / (1 - v_min/v), i.e. bank <= atan(v^2 / (g r)).
    let lat = members.iter().map(|s| s.0.abs()).fold(0.0, f32::max);
    let max_bank = if lat == 0.0 { FLIGHT_MAX_BANK } else {
        let r = lat / (1.0 - FLIGHT_STALL * 1.27 / trim);
        (trim * trim / (FLIGHT_G * r)).atan().min(FLIGHT_MAX_BANK)
    };
    g.flights[fi] = Flight { active: true, x, y, heading, bank: 0.0, speed: trim, trim, max_bank,
        legs: plan, n: legs.len().min(4), leg: 0, leg_t: 0.0, turned: 0.0, t: 0.0,
        members: members.len() as u8, downed: 0 };
    let can_hide = kind != EnemyKind::Ace && rand_range_f32(0.0, 1.0) < 0.4;
    for &slot in members {
        let (sx, sy) = slot_pos(x, y, heading, slot);
        let mut fire_timer = Timer::default();
        fire_timer.start(0.8 + rand_range_f32(0.0, 1.0) * 1.4);
        pool_spawn(&mut g.enemies, Enemy {
            x: sx - ENEMY_W as f32 / 2.0, y: sy - ENEMY_H as f32 / 2.0, heading, bank: 0.0, speed: trim, active: true,
            kind, t: 0.0, flight: fi, slot, fire_timer, burst: 0, burst_t: 0.0, can_hide,
        });
    }
}

/// A diving pass: in from the top toward where the player is, then a hard
/// banked break-away to one side and out.
pub(super) fn spawn_dive_pass(g: &mut Game, kind: EnemyKind, form: Formation) {
    let x = 90.0 + rand_range_f32(0.0, 1.0) * (WIN_W as f32 - 180.0);
    let px = g.player_x + PLAYER_W as f32 / 2.0;
    let heading = (px - x).atan2(420.0).clamp(-0.35, 0.35);
    let side = if x < WIN_W as f32 / 2.0 { 1.0 } else { -1.0 };
    spawn_flight(g, kind, form, x, -70.0, heading, &[
        Leg::Straight(1.4 + rand_range_f32(0.0, 1.0) * 0.6),
        Leg::Turn { bank: side * 0.95, by: 1.8 + rand_range_f32(0.0, 1.0) * 0.6 },
        Leg::Straight(6.0),
    ]);
}

/// A crossing pass: in from a top corner on a long diagonal, easing into a
/// gentle turn downward and out the bottom (`high`: straight across the
/// top of the screen instead).
pub(super) fn spawn_crossing(g: &mut Game, kind: EnemyKind, form: Formation, high: bool) {
    let from_left = rand_range_f32(0.0, 1.0) < 0.5;
    let s = if from_left { 1.0 } else { -1.0 };
    let x = if from_left { -70.0 } else { WIN_W as f32 + 70.0 };
    let y = 40.0 + rand_range_f32(0.0, 1.0) * 110.0;
    if high {
        spawn_flight(g, kind, form, x, y, s * 1.35, &[Leg::Straight(9.0)]);
        return;
    }
    spawn_flight(g, kind, form, x, y, s * (0.95 + rand_range_f32(0.0, 1.0) * 0.25), &[
        Leg::Straight(1.8 + rand_range_f32(0.0, 1.0) * 0.6),
        Leg::Turn { bank: -s * 0.5, by: 0.7 },
        Leg::Straight(6.0),
    ]);
}

/// Down into the fight, a 180-degree turn, and away up the screen.
pub(super) fn spawn_turn_back(g: &mut Game, kind: EnemyKind, form: Formation) {
    let x = 110.0 + rand_range_f32(0.0, 1.0) * (WIN_W as f32 - 220.0);
    let side = if x < WIN_W as f32 / 2.0 { 1.0 } else { -1.0 };
    spawn_flight(g, kind, form, x, -70.0, 0.0, &[
        Leg::Straight(1.2 + rand_range_f32(0.0, 1.0) * 0.5),
        Leg::Turn { bank: side * 1.0, by: std::f32::consts::PI },
        Leg::Straight(6.0),
    ]);
}

/// A lone ace: chases the player's lead point, then breaks away.
pub(super) fn spawn_ace(g: &mut Game) {
    let x = 80.0 + rand_range_f32(0.0, 1.0) * (WIN_W as f32 - 160.0);
    let side = if x < WIN_W as f32 / 2.0 { 1.0 } else { -1.0 };
    spawn_flight(g, EnemyKind::Ace, Formation::Solo, x, -70.0, 0.0, &[
        Leg::Pursue(3.4),
        Leg::Turn { bank: side * 1.1, by: 2.2 },
        Leg::Straight(6.0),
    ]);
}

/// The opening wave, in the air as the player climbs away from the carrier.
pub(super) fn spawn_opening_wave(g: &mut Game) {
    spawn_dive_pass(g, EnemyKind::Grunt, Formation::Vic);
    spawn_crossing(g, EnemyKind::Weaver, Formation::Pair, true);
    if g.sess.level >= 2 { spawn_dive_pass(g, EnemyKind::Grunt, Formation::Pair); }
}

/// One spawner tick: a flight of Zeros diving on the player (a finger-four
/// from level 2), Ki-43s crossing in echelon or turning back in a vic, or a
/// lone Ki-84; from level 2 more and more often a second flight with it.
pub(super) fn send_flights(g: &mut Game) {
    let level = g.sess.level;
    let flights = if rand_range_f32(0.0, 1.0) < (0.15 * (level - 1) as f32).min(0.6) { 2 } else { 1 };
    for _ in 0..flights {
        let r = rand_range_f32(0.0, 1.0);
        if r < 0.32 {
            let form = if level >= 2 && rand_range_f32(0.0, 1.0) < 0.5 { Formation::FingerFour } else { Formation::Vic };
            spawn_dive_pass(g, EnemyKind::Grunt, form);
        }
        else if r < 0.52 { spawn_dive_pass(g, EnemyKind::Grunt, Formation::Pair); }
        else if r < 0.68 { spawn_crossing(g, EnemyKind::Weaver, Formation::Echelon, false); }
        else if r < 0.78 { spawn_crossing(g, EnemyKind::Weaver, Formation::Abreast, false); }
        else if r < 0.90 { spawn_turn_back(g, EnemyKind::Weaver, Formation::Vic); }
        else { spawn_ace(g); }
    }
}

/// Fly every flight one step (see the flight model constants).
pub(super) fn update_flight(f: &mut Flight, dt: f32, player: (f32, f32, f32)) {
    f.t += dt;
    f.leg_t += dt;
    let leg = if f.leg < f.n { f.legs[f.leg] } else { Leg::Straight(99.0) };
    let w_now = FLIGHT_G * f.bank.tan() / f.speed;
    let mut cmd = match leg {
        Leg::Straight(_) => 0.0,
        Leg::Turn { bank, .. } => bank,
        Leg::Pursue(_) => {
            // pure pursuit of where the player will be, damped by the turn
            // already under way so it converges instead of hunting
            let (px, py, pvx) = player;
            let (lx, ly) = (px + pvx * 0.5, py);
            let want = (lx - f.x).atan2((ly - f.y).max(40.0));
            let mut err = want - f.heading;
            err = blip::angle_diff(0.0, err);
            err * 1.4 - w_now * 0.5
        }
    };
    let lim = ((FLIGHT_STALL / f.speed).powi(2)).min(1.0).acos().min(f.max_bank);
    cmd = cmd.clamp(-lim, lim);
    f.bank += (cmd - f.bank).clamp(-FLIGHT_ROLL * dt, FLIGHT_ROLL * dt);
    let w = FLIGHT_G * f.bank.tan() / f.speed;
    f.heading += w * dt;
    f.turned += (w * dt).abs();
    let n = 1.0 / f.bank.cos();
    f.speed += (FLIGHT_THRUST * (f.trim - f.speed) - FLIGHT_DRAG * (n * n - 1.0) * f.speed) * dt;
    f.speed = f.speed.max(FLIGHT_STALL * 1.15);
    f.x += f.heading.sin() * f.speed * dt;
    f.y += f.heading.cos() * f.speed * dt;
    let done = match leg {
        Leg::Straight(secs) | Leg::Pursue(secs) => f.leg_t >= secs,
        Leg::Turn { by, .. } => f.turned >= by,
    };
    if done && f.leg < f.n { f.leg += 1; f.leg_t = 0.0; f.turned = 0.0; }
}

/// Match the leader's turn rate, steer toward a point ahead of the slot, and hold position with throttle.
pub(super) fn fly_wing(e: &mut Enemy, f: &Flight, dt: f32) -> (f32, f32) {
    let (cx, cy) = (e.x + ENEMY_W as f32 / 2.0, e.y + ENEMY_H as f32 / 2.0);
    let (sx, sy) = slot_pos(f.x, f.y, f.heading, e.slot);
    let (ax, ay) = (sx + f.heading.sin() * 110.0, sy + f.heading.cos() * 110.0);
    let want = (ax - cx).atan2(ay - cy);
    let err = blip::angle_diff(e.heading, want);
    let lim = ((FLIGHT_STALL / e.speed).powi(2)).min(1.0).acos().min(FLIGHT_MAX_BANK);
    let w_lead = FLIGHT_G * f.bank.tan() / f.speed;
    let cmd = ((w_lead * e.speed / FLIGHT_G).atan() + err * 1.6).clamp(-lim, lim);
    e.bank += (cmd - e.bank).clamp(-FLIGHT_ROLL * 1.3 * dt, FLIGHT_ROLL * 1.3 * dt);
    e.heading += FLIGHT_G * e.bank.tan() / e.speed * dt;
    let along = (sx - cx) * f.heading.sin() + (sy - cy) * f.heading.cos();
    let target = (f.speed + along * 1.2).min(f.speed * 1.35).max(FLIGHT_STALL * 1.15);
    e.speed += (target - e.speed) * (2.0 * dt).min(1.0);
    (cx + e.heading.sin() * e.speed * dt, cy + e.heading.cos() * e.speed * dt)
}

pub(super) fn move_enemies(g: &mut Game, dt: f32, allow_fire: bool, sfx: &Sounds) {
    let player = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0, g.player_vx);
    for f in g.flights.iter_mut() {
        if f.active { update_flight(f, dt, player); }
    }
    g.gun_cd = (g.gun_cd - dt).max(0.0);
    for i in 0..MAX_ENEMIES {
        if !g.enemies[i].active { continue; }
        let e = &mut g.enemies[i];
        e.t += dt;
        let f = g.flights[e.flight];
        let (cx, cy) = if e.slot == (0.0, 0.0) {
            e.heading = f.heading;
            e.bank = f.bank;
            e.speed = f.speed;
            (f.x, f.y)
        } else {
            fly_wing(e, &f, dt)
        };
        e.x = cx - ENEMY_W as f32 / 2.0;
        e.y = cy - ENEMY_H as f32 / 2.0;
        if e.t > 2.0 && (cx < -80.0 || cx > WIN_W as f32 + 80.0 || cy > WIN_H as f32 + 80.0 || cy < -140.0) {
            e.active = false;
            continue;
        }
        if allow_fire { fire_guns(g, i, (cx, cy), (player.0, player.1), dt, sfx); }
    }
    for fi in 0..MAX_FLIGHTS {
        if g.flights[fi].active && g.flights[fi].t > 3.0 && !g.enemies.iter().any(|e| e.active && e.flight == fi) {
            g.flights[fi].active = false;
        }
    }
}

/// Plane `i`'s guns fire along the nose: only when it points down the screen and the player is near the line of fire,
/// in short bursts from both wings. Every level fires longer bursts, more often.
fn fire_guns(g: &mut Game, i: usize, (cx, cy): (f32, f32), player: (f32, f32), dt: f32, sfx: &Sounds) {
    let lvl = (g.sess.level - 1) as f32;
    let e = &mut g.enemies[i];
    let (fx, fy) = (e.heading.sin(), e.heading.cos());
    if e.burst == 0 && e.fire_timer.tick(dt) {
        let (dx, dy) = (player.0 - cx, player.1 - cy);
        let d = dx.hypot(dy).max(1.0);
        let on_line = (dx * fx + dy * fy) / d;
        if fy > 0.3 && on_line > 0.9 - 0.015 * lvl && d < 420.0 + 20.0 * lvl {
            e.burst = (3.0 + lvl * 0.5).min(7.0) as u8;
            e.burst_t = 0.0;
            if g.gun_cd <= 0.0 {
                play_sfx_volume(&sfx.enemy_gun, 0.4);
                g.gun_cd = 0.18;
            }
        }
        let (mn, mx) = if e.kind == EnemyKind::Ace { (0.8, 1.5) } else { (1.2, 2.4) };
        e.fire_timer.start((mn + rand_range_f32(0.0, 1.0) * (mx - mn)) / (1.0 + 0.12 * lvl));
    }
    if e.burst == 0 { return; }
    e.burst_t -= dt;
    if e.burst_t > 0.0 { return; }
    e.burst -= 1;
    e.burst_t = 0.08;
    let (rx, ry) = (-e.heading.cos(), e.heading.sin());
    for side in [-1.0f32, 1.0] {
        let (gx, gy) = (cx + rx * side * 15.0 + fx * 16.0, cy + ry * side * 15.0 + fy * 16.0);
        pool_spawn(&mut g.enemy_bullets, Bullet { x: gx, y: gy, vx: fx * ENEMY_BULLET_SPEED, vy: fy * ENEMY_BULLET_SPEED, active: true });
    }
}
