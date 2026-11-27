//! The world under and around the fight: boats, islands and their flak, the barrier, the scrolling sea.

use super::*;

pub(super) fn spawn_boat(g: &mut Game) {
    let x = 20.0 + rand_range_f32(0.0, 1.0) * (WIN_W as f32 - BOAT_W as f32 - 40.0);
    let vx = (rand_range_f32(0.0, 1.0) - 0.5) * 30.0; // slow lateral cruise, +/- 15 px/s
    let bob_phase = rand_range_f32(0.0, 1.0) * std::f32::consts::TAU;
    pool_spawn(&mut g.boats, Boat { x, y: -(BOAT_H as f32), active: true, vx, bob_phase });
}

pub(super) fn spawn_island(g: &mut Game) {
    spawn_island_sized(g, (rand() % ISLAND_SIZES.len() as u32) as u8);
}

pub(super) fn spawn_island_sized(g: &mut Game, size: u8) {
    let (w, h) = ISLAND_SIZES[size as usize];
    let x = 10.0 + rand_range_f32(0.0, 1.0) * (WIN_W as f32 - w as f32 - 20.0);
    // a moment to scroll fully into view before they open up, guns staggered
    let guns = [0, 1, 2].map(|k| FlakGun { angle: 0.0, reload: 1.6 + k as f32 * 0.7 + rand_range_f32(0.0, 1.0) * 0.5, flash: 0.0 });
    pool_spawn(&mut g.islands, Island { x, y: -(h as f32), size, active: true, hp: ISLAND_HP[size as usize], guns });
}

/// Arm a fresh laser barrier: a random motor position and a random 3-10 HP,
/// then re-arm `barrier_timer` for the next (seldom) one.
pub(super) fn spawn_barrier(g: &mut Game, sfx: &Sounds) {
    let span = (BARRIER_HP_MAX - BARRIER_HP_MIN + 1) as f32;
    let hp = (BARRIER_HP_MIN as f32 + rand_range_f32(0.0, 1.0) * span) as i32;
    let hp = hp.clamp(BARRIER_HP_MIN, BARRIER_HP_MAX);
    let motor_x = MOTOR_W / 2.0 + 20.0 + rand_range_f32(0.0, 1.0) * (WIN_W as f32 - MOTOR_W - 40.0);
    let mut warmup = Timer::default();
    warmup.start(BARRIER_WARMUP);
    g.barrier = Barrier { active: true, motor_x, hp, max_hp: hp, warmup, t: 0.0, y: BARRIER_START_Y };
    g.barrier_timer.start(BARRIER_MIN_INTERVAL + rand_range_f32(0.0, 1.0) * (BARRIER_MAX_INTERVAL - BARRIER_MIN_INTERVAL));
    // Silent to start; update_barrier() rides both loops by distance.
    play_sound(&sfx.barrier_hum, PlaySoundParams { looped: true, volume: 0.0 });
    play_sound(&sfx.barrier_hum2, PlaySoundParams { looped: true, volume: 0.0 });
}

/// Move the warmed-up barrier and crossfade its warning hum as it approaches and passes.
pub(super) fn update_barrier(g: &mut Game, dt: f32, sfx: &Sounds) {
    if !g.barrier.active {
        stop_sound(&sfx.barrier_hum);
        stop_sound(&sfx.barrier_hum2);
        return;
    }
    g.barrier.t += dt;
    g.barrier.warmup.tick(dt);
    if !g.barrier.warmup.active() {
        g.barrier.y += (BARRIER_SPEED + 2.0 * (g.sess.level - BARRIER_MIN_LEVEL).max(0) as f32) * dt;
    }
    if g.barrier.y > WIN_H as f32 + 20.0 {
        g.barrier.active = false;
        stop_sound(&sfx.barrier_hum);
        stop_sound(&sfx.barrier_hum2);
        return;
    }
    let player_cy = g.player_y + PLAYER_H as f32 / 2.0;
    let ahead = player_cy - g.barrier.y; // > 0 while it is still coming
    let k = if ahead >= 0.0 {
        1.0 - (ahead / (player_cy - BARRIER_START_Y).max(1.0)).clamp(0.0, 1.0)
    } else {
        (1.0 + ahead / 120.0).max(0.0)
    };
    set_sound_volume(&sfx.barrier_hum, (0.12 + 0.88 * k) * BARRIER_HUM_MAX_VOLUME);
    set_sound_volume(&sfx.barrier_hum2, ((k - 0.5) * 2.0).max(0.0).powf(1.5) * BARRIER_HUM_MAX_VOLUME);
}

/// The world under the fight: sea scroll, cloud drift, boat and island
/// spawns, shared by the launch and play so it keeps moving on the carrier.
/// Turrets fire only in play (update_islands()).
pub(super) fn update_background(g: &mut Game, dt: f32) {
    let sc = SEA_SCROLL_SPEED * g.scroll_k;
    g.sea_scroll += sc * dt;
    if land::is_land_level(g) { g.land_dist += sc * dt; }

    for c in g.clouds.iter_mut() {
        c.y += c.speed * g.scroll_k * dt;
        if c.y - c.r > WIN_H as f32 {
            c.y = -c.r;
            c.x = rand_range_f32(0.0, 1.0) * WIN_W as f32;
        }
    }

    for b in pool_iter_mut(&mut g.boats) {
        b.y += sc * dt;
        b.x += b.vx * dt;
        if b.y - BOAT_H as f32 > WIN_H as f32
            || b.x < -(BOAT_W as f32) - 20.0
            || b.x > WIN_W as f32 + 20.0
        {
            b.active = false;
        }
    }
    // Nothing more puts to sea once the coast is in sight.
    let at_sea = land::coast_y(g).is_none_or(|cy| cy < -200.0);
    if g.boat_timer.tick(dt) && at_sea {
        spawn_boat(g);
        g.boat_timer.start(4.0 + rand_range_f32(0.0, 1.0) * 4.0);
    }

    for isl in pool_iter_mut(&mut g.islands) {
        isl.y += sc * dt;
        if isl.y - ISLAND_SIZES[isl.size as usize].1 as f32 > WIN_H as f32 {
            isl.active = false;
        }
    }
    if g.island_timer.tick(dt) && at_sea {
        spawn_island(g);
        g.island_timer.start(ISLAND_MIN_INTERVAL + rand_range_f32(0.0, 1.0) * (ISLAND_MAX_INTERVAL - ISLAND_MIN_INTERVAL));
    }
}

/// Turret AI + shell flight — play-only (see update_background() for why).
/// Where flak gun `k` of an island stands on screen.
pub(super) fn flak_gun_pos(isl: &Island, k: usize) -> (f32, f32) {
    let (w, h) = ISLAND_SIZES[isl.size as usize];
    let (fx, fy) = FLAK_GUNS[isl.size as usize][k];
    (isl.x + w as f32 * fx, isl.y + h as f32 * fy)
}

/// A flak shell from (`x`, `y`), fused to burst where the player is now.
pub(super) fn fire_flak(g: &mut Game, x: f32, y: f32, sfx: &Sounds) {
    let (px, py) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0);
    let (dx, dy) = (px - x, py - y);
    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
    if dist < FLAK_MIN_RANGE { return; }
    // The pool is the cap on shells in the air: a full sky holds its fire.
    if pool_spawn(&mut g.flak, FlakShell { x, y, vx: dx / dist * FLAK_SPEED, vy: dy / dist * FLAK_SPEED, left: dist, active: true }) {
        play_sfx_volume(&sfx.turret_fire, 0.8);
    }
}

/// Each island flak gun follows the player and, on its own timer once the
/// island is in view, fires (see fire_flak).
pub(super) fn update_islands(g: &mut Game, dt: f32, sfx: &Sounds) {
    let (px, py) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0);
    for ii in 0..MAX_ISLANDS {
        if !g.islands[ii].active || g.islands[ii].y < 0.0 { continue; } // still scrolling in
        for k in 0..FLAK_GUNS[g.islands[ii].size as usize].len() {
            let (x, y) = flak_gun_pos(&g.islands[ii], k);
            let gun = &mut g.islands[ii].guns[k];
            if !aim_flak_gun(gun, x, y, px, py, dt, FLAK_RELOAD) { continue; }
            fire_flak(g, x, y, sfx);
        }
    }
}

/// Turn a flak gun on the player and count down its reload; true when it fires.
pub(super) fn aim_flak_gun(gun: &mut FlakGun, x: f32, y: f32, px: f32, py: f32, dt: f32, reload: (f32, f32)) -> bool {
    gun.angle = (px - x).atan2(py - y);
    gun.flash = (gun.flash - dt).max(0.0);
    gun.reload -= dt;
    if gun.reload > 0.0 { return false; }
    gun.reload = reload.0 + rand_range_f32(0.0, 1.0) * (reload.1 - reload.0);
    gun.flash = 0.08;
    true
}

/// Flak shells flying to their fuse point and bursting; bursts turning to
/// smoke and drifting back.
pub(super) fn update_flak(g: &mut Game, dt: f32, sfx: &Sounds) {
    let sc = SEA_SCROLL_SPEED * g.scroll_k;
    for i in 0..MAX_FLAK {
        let s = &mut g.flak[i];
        if !s.active { continue; }
        let step = (s.vx.hypot(s.vy) * dt).min(s.left);
        s.x += s.vx * dt;
        s.y += s.vy * dt;
        s.left -= step;
        if s.left <= 0.0 {
            s.active = false;
            let (x, y) = (s.x, s.y);
            pool_spawn(&mut g.flak_bursts, FlakBurst { x, y, t: 0.0, active: true });
            play_sfx_volume(&sfx.flak_burst, 0.75);
        }
    }
    for b in pool_iter_mut(&mut g.flak_bursts) {
        b.t += dt;
        b.y += sc * 0.6 * dt; // the smoke drifts back with the wind
        if b.t > FLAK_LETHAL + FLAK_SMOKE { b.active = false; }
    }
}
