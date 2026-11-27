//! The player's plane: flying it, its guns, smoke from damage, hits and losses, stalls, the engine's splutter and its
//! sound.

use super::*;

/// The plane: throttle and airspeed, a weighty sideways slide, the bank, and a
/// stall when it flies too slow for too long.
pub(super) fn fly_plane(g: &mut Game, dt: f32, sfx: &Sounds) {
    if g.stall_fall > 0.0 {
        update_stall(g, dt, sfx);
    } else {
        let left  = Stick::Left.active();
        let right = Stick::Right.active();
        let up    = Stick::Up.active();
        let down  = Stick::Down.active();
        let throttle = up as i32 as f32 - down as i32 as f32;
        if g.engine_out() {
            // No thrust: the plane sheds speed whatever the throttle, but
            // a splutter never stalls it on its own.
            g.airspeed = (g.airspeed - SPLUTTER_DRAG * dt).max(g.airspeed.min(SPLUTTER_FLOOR));
        } else if throttle != 0.0 {
            g.airspeed += throttle * AIRSPEED_RATE * dt;
        } else {
            g.airspeed += (1.0 - g.airspeed).clamp(-AIRSPEED_RELAX * dt, AIRSPEED_RELAX * dt);
        }
        g.airspeed = g.airspeed.clamp(0.0, AIRSPEED_MAX);
        update_splutter(g, dt, sfx);
        // Sideways: spool up and brake over ~0.1s so the plane has weight.
        let dx = right as i32 as f32 - left as i32 as f32;
        g.player_vx = blip::spool(g.player_vx, dx * PLAYER_SPEED, PLAYER_ACCEL, PLAYER_BRAKE, dt);
        // Up and down the screen is only gaining on or falling behind the
        // scrolling world.
        let vy_target = (1.0 - g.airspeed) * AIRSPEED_DRIFT;
        g.player_vy += (vy_target - g.player_vy) * (6.0 * dt).min(1.0);
        g.player_x += g.player_vx * dt;
        g.player_y += g.player_vy * dt;
        let (max_x, max_y) = ((WIN_W - PLAYER_W) as f32, PLAYER_MAX_Y);
        if g.player_x < 0.0 || g.player_x > max_x { g.player_vx = 0.0; }
        g.player_x = clamp(g.player_x, 0.0, max_x);
        g.player_y = clamp(g.player_y, PLAYER_MIN_Y, max_y);

        // Bank follows lateral velocity.
        let target_bank = g.player_vx / PLAYER_SPEED * 0.30;
        g.player_bank += (target_bank - g.player_bank) * (dt * 9.0).min(1.0);

        if g.airspeed <= STALL_SPEED { g.stall_t += dt; } else { g.stall_t = 0.0; }
        if g.stall_t >= STALL_HOLD {
            g.stall_fall = 1e-4;
            g.stall_spin = if rand_range_f32(0.0, 1.0) < 0.5 { -1.0 } else { 1.0 };
        }
    }
    g.scroll_k = if g.airspeed < 1.0 { 0.25 + 0.75 * g.airspeed } else { 1.0 + 0.5 * (g.airspeed - 1.0) };
    g.prop_angle = (g.prop_angle + blip::lerp(40.0, 7.0, glide(g)) * dt) % std::f32::consts::TAU;
}

/// The guns: a volley while fire is held and they are ready, and the brass it throws.
pub(super) fn fire_volley(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.fire_cd.tick(dt);
    if g.stall_fall == 0.0 && key_active(BLIP_KEY_SPACE) && !g.fire_cd.active() && !g.respawn_grace.active() {
        g.fire_cd.start(weapon_cooldown(g.weapon_level));
        let (n, fan, r) = weapon_guns(g.weapon_level);
        let cx = g.player_x + PLAYER_W as f32 / 2.0;
        for i in 0..n {
            let dx = if n > 1 { (i as f32 / (n - 1) as f32 - 0.5) * fan } else { 0.0 };
            // Guns are not lasers: each round leaves a little off true.
            pool_spawn(&mut g.bullets, Round {
                x: cx + dx + (rand_range_f32(0.0, 1.0) - 0.5) * 3.0,
                y: g.player_y + rand_range_f32(0.0, 1.0) * 6.0,
                vx: dx * 2.2 + (rand_range_f32(0.0, 1.0) - 0.5) * 28.0 + g.player_vx * 0.15,
                vy: BULLET_SPEED,
                r,
                bounced: false,
                active: true,
            });
        }
        g.muzzle_t = 0.05;
        g.eject_left = !g.eject_left;
        let side = if g.eject_left { -1.0 } else { 1.0 };
        pool_spawn(&mut g.casings, Casing {
            x: cx + side * 6.0, y: g.player_y + PLAYER_H as f32 * 0.45,
            vx: side * (70.0 + rand_range_f32(0.0, 1.0) * 50.0) + g.player_vx * 0.5,
            vy: 20.0 + rand_range_f32(0.0, 1.0) * 40.0,
            rot: rand_range_f32(0.0, 1.0) * 3.0, ttl: 0.45, active: true,
        });
        let takes = &sfx.shoot[(g.weapon_level.clamp(1, MAX_WEAPON_LEVEL) - 1) as usize];
        play_sfx_volume(&takes[(rand() % takes.len() as u32) as usize], 0.8 + 0.04 * g.weapon_level as f32);
    }
    g.muzzle_t = (g.muzzle_t - dt).max(0.0);
    for c in pool_iter_mut(&mut g.casings) {
        c.x += c.vx * dt;
        c.y += c.vy * dt;
        c.vy += SEA_SCROLL_SPEED * 3.0 * dt;
        c.vx *= 1.0 - 2.5 * dt;
        c.rot += 14.0 * dt;
        c.ttl -= dt;
        if c.ttl <= 0.0 { c.active = false; }
    }
}

/// Damaged: smoke from the engine at 3 health, fire with it at 2, more of
/// both at 1; near a stall, backfires.
pub(super) fn damage_smoke(g: &mut Game, dt: f32, sfx: &Sounds) {
    if g.health <= 3 && g.stall_fall == 0.0 {
        g.smoke_t -= dt;
        if g.smoke_t <= 0.0 {
            let hurt = (PLAYER_HEALTH_MAX - g.health) as f32; // 2..4
            g.smoke_t = 0.13 - 0.025 * hurt;
            let (cx, ny) = (g.player_x + PLAYER_W as f32 / 2.0 + (rand_range_f32(0.0, 1.0) - 0.5) * 6.0, g.player_y + PLAYER_H as f32 * 0.3);
            let dark = if g.health <= 2 { 1.0 } else { 0.6 };
            pool_spawn(&mut g.puffs, Puff { x: cx, y: ny, r: 2.5 * dark + 1.0, grow: 12.0 + 8.0 * dark,
                ttl: 0.9, max_ttl: 0.9, fire: false, top: true, active: true });
            if g.health <= 2 {
                // flames licking back from both sides of the cowling
                for side in [-1.0f32, 1.0] {
                    let fx = g.player_x + PLAYER_W as f32 / 2.0 + side * (3.0 + rand_range_f32(0.0, 1.0) * 3.0);
                    pool_spawn(&mut g.puffs, Puff { x: fx, y: g.player_y + PLAYER_H as f32 * 0.2, r: 2.5 + hurt * 0.9 + rand_range_f32(0.0, 1.0) * 2.0,
                        grow: -9.0, ttl: 0.28, max_ttl: 0.28, fire: true, top: true, active: true });
                }
            }
        }
    }
    // Near a stall the engine starts to backfire.
    if g.airspeed < STALL_WARN && g.stall_fall == 0.0 {
        g.backfire_t -= dt;
        if g.backfire_t <= 0.0 {
            g.backfire_t = 0.18 + rand_range_f32(0.0, 1.0) * 0.35;
            play_sfx_volume(&sfx.backfire, 0.75);
            let (cx, ny) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 * 0.2);
            pool_spawn(&mut g.puffs, Puff { x: cx, y: ny, r: 3.0, grow: -8.0, ttl: 0.12, max_ttl: 0.12, fire: true, top: true, active: true });
            pool_spawn(&mut g.puffs, Puff { x: cx, y: ny, r: 2.5, grow: 14.0, ttl: 0.7, max_ttl: 0.7, fire: false, top: true, active: true });
        }
    }
}

/// The plane is hit at `px`,`py`: a health point and a gun tier, or the plane.
pub(super) fn hurt_plane(g: &mut Game, px: f32, py: f32, sfx: &Sounds) {
    g.health -= 1;
    g.streak.miss();
    // A non-fatal hit drops the gun one tier, never below 2; a fatal
    // one leaves it.
    if g.health > 0 && g.weapon_level > 2 {
        g.weapon_level -= 1;
    }
    if g.health > 0 {
        // Still flying: a flinch of invulnerability (with the usual
        // respawn blink) instead of going down outright.
        play_sfx(&sfx.player_hit);
        g.respawn_grace.start(HIT_GRACE);
    } else {
        g.spawn_player_death(px + PLAYER_W as f32 / 2.0, py + PLAYER_H as f32 / 2.0);
        play_take(&sfx.player_explode, 1.0);
        lose_plane(g);
    }
}

/// The plane is down: the pause before the next, or the continue offer.
pub(super) fn lose_plane(g: &mut Game) {
    match g.sess.lose_life() {
        LifeResult::StillAlive => { g.dead_timer.start(DEAD_PAUSE); g.state = State::Dead; }
        LifeResult::GameOver   => { g.cont.start(CONTINUE_SECS); g.state = State::Over; }
    }
}

/// Whether a pickup `w` by `h`, moving `from` -> `to` this frame, met the
/// plane on its way from `before`: both move, so the test is swept.
pub(super) fn caught(g: &Game, before: (f32, f32), from: (f32, f32), to: (f32, f32), w: f32, h: f32) -> bool {
    let (pw, ph) = (PLAYER_W as f32, PLAYER_H as f32);
    let centre = |(x, y): (f32, f32), w: f32, h: f32| (x + w / 2.0, y + h / 2.0);
    blip::sweep_hits(centre(from, w, h), centre(to, w, h), centre(before, pw, ph), centre((g.player_x, g.player_y), pw, ph),
        (-(pw + w) / 2.0, -(ph + h) / 2.0, pw + w, ph + h))
}

/// Stalled: the plane falls off into a spin, trailing smoke, and goes into
/// the sea. It costs a life like being shot down.
pub(super) fn update_stall(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.stall_fall += dt;
    g.player_bank += g.stall_spin * (2.0 + g.stall_fall * 5.0) * dt;
    g.player_y += 30.0 * dt;
    let k = g.stall_fall / STALL_FALL;
    let (cx, cy) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0);
    g.stall_t -= dt;
    if g.stall_t <= 0.0 {
        g.stall_t = 0.05;
        pool_spawn(&mut g.puffs, Puff { x: cx, y: cy, r: 3.0 * (1.0 - 0.5 * k), grow: 14.0,
            ttl: 0.9, max_ttl: 0.9, fire: false, top: false, active: true });
    }
    if g.stall_fall >= STALL_FALL {
        g.stall_fall = 0.0;
        g.spawn_explosion(cx, cy, 1.4, BlipColor::new(0.85, 0.93, 1.0, 1.0));
        play_take(&sfx.player_explode, 1.0);
        g.health = 0;
        blip::bot::add("death_stall", 1.0);
        lose_plane(g);
    }
}

/// Crossfade engine loops by airspeed, raise the cough near stall, and fade the engine during a fall.
/// Fuel starvation can briefly cut the engine; the loops duck under the splutter sound.
pub(super) fn update_splutter(g: &mut Game, dt: f32, sfx: &Sounds) {
    if g.splutter_t < 0.0 {
        g.splutter_next -= dt;
        // Not near a stall or in a boss's entrance, where it would be unfair.
        if g.splutter_next <= 0.0 && g.airspeed > 0.7 && !g.boss_intro.active() {
            g.splutter_t = 0.0;
            g.splutter_next = SPLUTTER_EVERY.0 + rand_range_f32(0.0, 1.0) * (SPLUTTER_EVERY.1 - SPLUTTER_EVERY.0);
            play_sfx_volume(&sfx.engine_splutter, PROP_MAX_VOLUME * 1.1);
            blip::bot::add("splutters", 1.0);
        }
        return;
    }
    g.splutter_t += dt;
    if g.engine_out() {
        // dark puffs from the exhaust stacks either side of the nose
        g.smoke_t -= dt;
        if g.smoke_t <= 0.0 {
            g.smoke_t = 0.09;
            for side in [-1.0f32, 1.0] {
                let x = g.player_x + PLAYER_W as f32 / 2.0 + side * 4.0;
                pool_spawn(&mut g.puffs, Puff { x, y: g.player_y + PLAYER_H as f32 * 0.22, r: 2.0, grow: 16.0,
                    ttl: 0.7, max_ttl: 0.7, fire: false, top: true, active: true });
            }
        }
    }
    if g.splutter_t >= SPLUTTER_SECS { g.splutter_t = -1.0; }
}

/// How much of the engine loops plays through a splutter: fading out as it
/// dies, silent while it is out, back as it catches.
pub(super) fn splutter_duck(g: &Game) -> f32 {
    let t = g.splutter_t;
    if t < 0.0 { 1.0 }
    else if t < SPLUTTER_DIES { 1.0 - t / SPLUTTER_DIES }
    else if t < SPLUTTER_CATCHES { 0.0 }
    else { ((t - SPLUTTER_CATCHES) / 0.25).min(1.0) }
}

/// One of several takes of a sound, at a slightly varied volume, so a run
/// of the same event never repeats exactly.
pub(super) fn play_take(takes: &[blip::BlipSound], vol: f32) {
    let i = ((rand_range_f32(0.0, 1.0) * takes.len() as f32) as usize).min(takes.len() - 1);
    play_sfx_volume(&takes[i], vol * (0.85 + 0.15 * rand_range_f32(0.0, 1.0)));
}

pub(super) fn engine_mix(g: &Game) -> [f32; 5] {
    let a = g.airspeed.clamp(ENGINE_AT[0], AIRSPEED_MAX);
    let mut w = [0.0f32; 5];
    for i in 0..4 {
        if a >= ENGINE_AT[i] && a <= ENGINE_AT[i + 1] {
            let t = (a - ENGINE_AT[i]) / (ENGINE_AT[i + 1] - ENGINE_AT[i]);
            w[i] = 1.0 - t;
            w[i + 1] = t;
        }
    }
    if g.airspeed < STALL_WARN {
        let s = 1.0 - g.airspeed / STALL_WARN;
        w[0] = w[0].max(s);
    }
    let loud = PROP_MAX_VOLUME * (0.55 + 0.45 * g.airspeed / AIRSPEED_MAX);
    let dying = if g.stall_fall > 0.0 { (1.0 - g.stall_fall / STALL_FALL).max(0.0).powi(2) } else { 1.0 };
    let duck = splutter_duck(g);
    w.map(|v| v * loud * dying * duck)
}
