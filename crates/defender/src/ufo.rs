//! The saucer: its passes across the top with a bomb, and now and then the hunt with its death ray; shooting it down.

use super::*;

/// Detonate the laser: kill every alien, shield block, and (if it's standing
/// in the way) the player under the beam's column, with a cascade of
/// explosions down its length. Returns `true` if the player was killed.
pub(super) fn fire_laser(g: &mut Game) -> bool {
    let bx0 = g.ufo_laser_x + UFO_W / 2.0 - LASER_HIT_W / 2.0;
    let bx1 = bx0 + LASER_HIT_W;

    for a in g.aliens.iter_mut() {
        if !a.alive { continue; }
        if a.x < bx1 && a.x + ALIEN_W as f32 > bx0 {
            a.alive = false;
            let pts = match a.kind { 0 => 30, 1 => 20, _ => 10 };
            g.sess.add_score(pts * g.sess.level);
        }
    }
    // A dense cascade of explosions straight down the beam's path, for a
    // "very dramatic" wipe (covers the vaporized aliens too).
    let mut y = UFO_Y;
    while y < GROUND_Y as f32 {
        g.spawn_explosion(bx0 + LASER_HIT_W / 2.0 - ALIEN_W as f32 / 2.0, y);
        y += 26.0;
    }

    for s in g.shields.iter_mut() {
        for r in 0..SHIELD_ROWS {
            for c in 0..SHIELD_COLS {
                if !s.alive[r][c] { continue; }
                let sx = s.x + (c as i32 * SHIELD_BLOCK) as f32;
                if sx < bx1 && sx + SHIELD_BLOCK as f32 > bx0 {
                    s.alive[r][c] = false;
                }
            }
        }
    }

    let player_hit = g.player_x < bx1 && g.player_x + ALIEN_W as f32 > bx0;
    if player_hit {
        blip::bot::add("death_laser", 1.0);
        // Losing the cannon breaks the run of hits too: the multiple is for shooting well and staying alive.
        g.streak.miss();
        let px = g.player_x;
        g.spawn_player_death(px, (GROUND_Y - 28) as f32);
        if lose_cannon(g) { g.bullets.iter_mut().for_each(|b| b.active = false); }
    }
    player_hit
}

/// Returns `true` if the laser just killed the player this frame — the
/// caller should stop processing the round immediately when that happens.
pub(super) fn update_ufo(
    g: &mut Game,
    dt: f32,
    sfx: &Sounds,
    bullet_starts: &[(f32, f32); N_BULLETS],
    ufo_start_x: f32,
    ufo_was_active: bool,
) -> bool {
    g.ufo_score_timer.tick(dt);
    if !g.ufo_active {
        if g.ufo_timer.tick(dt) && g.aliens_alive() > 0 { send_ufo(g, sfx); }
        return false;
    }
    match g.ufo_mode {
        UfoMode::Flying => if fly_ufo(g, dt) { return false; },
        UfoMode::Tracking => track_ufo(g, dt, sfx),
        UfoMode::Charging => if charge_ufo(g, dt, sfx) { return true; },
        UfoMode::Firing => if g.ufo_fire_timer.tick(dt) {
            // The beam burns the UFO itself out.
            g.spawn_explosion(g.ufo_x, UFO_Y);
            play_sfx(&sfx.explosion);
            ufo_gone(g);
        },
    }
    // Player bullet vs UFO — not while it's mid-beam (too late by then).
    if g.ufo_mode != UfoMode::Firing { shoot_ufo(g, sfx, bullet_starts, if ufo_was_active { ufo_start_x } else { g.ufo_x }); }
    false
}

/// The saucer sets off across the top from one side or the other, siren wailing: on one pass a level it hunts the
/// player with its death ray (UFO_LASER_CHANCE), otherwise it drops a bomb on its way past.
pub(super) fn send_ufo(g: &mut Game, sfx: &Sounds) {
    g.ufo_dir = if (rand() & 1) == 0 { 1 } else { -1 };
    g.ufo_x = if g.ufo_dir == 1 { -UFO_W } else { WIN_W as f32 };
    g.ufo_active = true;
    g.ufo_frame = 0;
    g.ufo_spin_timer = 0.0;
    g.bullets[UFO_BOMB_IDX].active = false;
    blip::play_alert(&sfx.ufo_siren);
    let roll = (rand() as f32) / (u32::MAX as f32);
    if !g.laser_used_this_level && roll < UFO_LASER_CHANCE {
        g.laser_used_this_level = true;
        g.ufo_mode = UfoMode::Tracking;
        g.ufo_track_timer.start(UFO_TRACK_MAX_SECS);
    } else {
        g.ufo_mode = UfoMode::Flying;
        g.ufo_bomb_timer.start(3.0);
    }
}

/// The saucer is gone (off the screen, burnt out or shot down): the next comes after a while.
pub(super) fn ufo_gone(g: &mut Game) {
    g.ufo_active = false;
    g.ufo_mode = UfoMode::Flying;
    g.ufo_timer.start(rand_int(UFO_SPAWN_MIN, UFO_SPAWN_MAX) as f32);
}

/// Flying past: its one bomb on the way, and gone at the far side (true).
pub(super) fn fly_ufo(g: &mut Game, dt: f32) -> bool {
    g.ufo_x += 80.0 * g.ufo_dir as f32 * dt;
    g.spin_lights(dt);
    if g.ufo_x > WIN_W as f32 || g.ufo_x + UFO_W < 0.0 {
        g.bullets[UFO_BOMB_IDX].active = false;
        ufo_gone(g);
        blip::stop_alert();
        return true;
    }
    // Only one bomb per pass — timer stays inactive after first fire.
    if g.ufo_bomb_timer.tick(dt) && !g.bullets[UFO_BOMB_IDX].active {
        g.bullets[UFO_BOMB_IDX] = Bullet { x: g.ufo_x + UFO_W / 2.0 - 2.0, y: UFO_Y + UFO_H * 0.6, active: true, player: false };
    }
    false
}

/// Stalk the player's x before charging, a hunted beat that warns before the charge sound. Faster than the player,
/// it commits only once they hold still (locked on) or its time runs out.
pub(super) fn track_ufo(g: &mut Game, dt: f32, sfx: &Sounds) {
    let target = (g.player_x + ALIEN_W as f32 / 2.0 - UFO_W / 2.0).clamp(0.0, WIN_W as f32 - UFO_W);
    let step = UFO_TRACK_SPEED * dt;
    g.ufo_x += (target - g.ufo_x).clamp(-step, step);
    g.spin_lights(dt);
    let expired = g.ufo_track_timer.tick(dt);
    let elapsed = UFO_TRACK_MAX_SECS - g.ufo_track_timer.remaining();
    let locked_on = (g.ufo_x - target).abs() < UFO_LOCK_DIST;
    let player_still = g.player_still_secs >= UFO_STILL_SECS;
    if expired || (elapsed >= UFO_TRACK_MIN_SECS && locked_on && player_still) {
        g.ufo_mode = UfoMode::Charging;
        g.ufo_charge_timer.start(UFO_CHARGE_SECS);
        blip::stop_alert();
        play_sfx(&sfx.laser_charge);
    }
}

/// Charging, its chase lights flickering frantically, then the beam; whether it caught the player (true).
pub(super) fn charge_ufo(g: &mut Game, dt: f32, sfx: &Sounds) -> bool {
    g.ufo_spin_timer += dt * 1000.0;
    if g.ufo_spin_timer >= UFO_SPIN_STEP_MS * 0.25 {
        g.ufo_spin_timer = 0.0;
        g.ufo_frame = (g.ufo_frame + 1) % UFO_N_LIGHTS;
    }
    if !g.ufo_charge_timer.tick(dt) { return false; }
    g.ufo_mode = UfoMode::Firing;
    g.ufo_fire_timer.start(UFO_FIRE_SECS);
    g.ufo_laser_x = g.ufo_x;
    play_sfx(&sfx.laser_blast);
    fire_laser(g)
}

/// The cannon's shots against the saucer, swept with it from `ufo_from`: a hit is worth 50 to 300.
pub(super) fn shoot_ufo(g: &mut Game, sfx: &Sounds, bullet_starts: &[(f32, f32); N_BULLETS], ufo_from: f32) {
    // `bi` pairs the bullet slot with its swept start; the bomb slot below rules out iterating `g.bullets` directly.
    for (bi, start) in bullet_starts.iter().enumerate().take(MAX_PLAYER_BULLETS) {
        if !g.bullets[bi].active { continue; }
        let now = (g.bullets[bi].x, g.bullets[bi].y);
        if !bullet_hits_moving_rect(*start, now, (ufo_from, UFO_Y), (g.ufo_x, UFO_Y), UFO_W, UFO_H) { continue; }
        play_sfx(&sfx.explosion);
        g.spawn_explosion(g.ufo_x, UFO_Y);
        g.bullets[bi].active = false;
        g.bullets[UFO_BOMB_IDX].active = false;
        blip::bot::add("saucers", 1.0);
        let bonus = rand_int(1, 6) * 50;
        g.sess.add_score(bonus);
        g.ufo_score = bonus;
        g.ufo_score_timer.start(1.5);
        ufo_gone(g);
        blip::stop_alert();
        return;
    }
}

pub(super) fn stop_ufo(g: &mut Game) {
    if g.ufo_active {
        g.ufo_active = false;
        blip::stop_alert();
    }
}
