//! The player's rounds against everything they can hit (planes, the boss, boats, batteries, islands, the barrier's
//! motor), and the pickups caught.

use super::*;

pub(super) fn round_hits_rect(from: (f32, f32), round: &Round, x: f32, y: f32, w: f32, h: f32) -> bool {
    let width = round.r * 2.0 + 2.0;
    segment_rect_overlap(
        from.0, from.1 + BULLET_H * 0.5,
        round.x, round.y + BULLET_H * 0.5,
        x - width * 0.5, y - BULLET_H * 0.5,
        w + width, h + BULLET_H,
    )
}

/// The player's rounds against the enemies, the boss and the barrier: each one
/// shoots a plane down, glances off one (once), or chips at the boss.
pub(super) fn hit_with_bullets(g: &mut Game, sfx: &Sounds, bullet_starts: &[(f32, f32); MAX_PLAYER_BULLETS],
                    bullet_left_screen: &mut [bool; MAX_PLAYER_BULLETS]) {
    for bi in 0..MAX_PLAYER_BULLETS {
        if !g.bullets[bi].active { continue; }
        let start = bullet_starts[bi];
        let bw = g.bullets[bi].r * 2.0 + 2.0;
        let (bx, by) = (g.bullets[bi].x - bw / 2.0, g.bullets[bi].y);
        match hit_planes(g, sfx, bi, start) {
            Some(Strike::Down) => { g.bullets[bi].active = false; continue; }
            // glanced off: it flies on, away from the plane, even if it was leaving the screen
            Some(Strike::Glanced) => { bullet_left_screen[bi] = false; continue; }
            None => {}
        }
        let (bossw, bossh) = boss_size(g.boss.tier);
        if g.boss.active && round_hits_rect(start, &g.bullets[bi], g.boss.x, g.boss.y, bossw, bossh) {
            g.bullets[bi].active = false;
            hit_boss(g, bx, by, sfx);
        }
        if g.bullets[bi].active { hit_boats(g, sfx, bi, start); }
        // Flak batteries on land.
        if g.bullets[bi].active && land::hit_battery(g, bx + bw / 2.0, by, sfx) { g.bullets[bi].active = false; }
        if g.bullets[bi].active { hit_islands(g, sfx, bi, start, (bx, by)); }
        if g.bullets[bi].active && g.barrier.active && !g.barrier.warmup.active() { hit_motor(g, sfx, bi, start, (bx, by)); }
    }
}

/// What a round did to the planes it met.
pub(super) enum Strike { Down, Glanced }

/// The first plane round `bi` meets: shot down (some go down in flames), or, now and then, a glancing blow: a
/// spark, and the round away at 35-80 degrees to one side, a little slower, the plane unharmed (once a round).
pub(super) fn hit_planes(g: &mut Game, sfx: &Sounds, bi: usize, start: (f32, f32)) -> Option<Strike> {
    for ei in 0..MAX_ENEMIES {
        if !g.enemies[ei].active { continue; }
        let (ex, ey) = (g.enemies[ei].x, g.enemies[ei].y);
        if !round_hits_rect(start, &g.bullets[bi], ex, ey, ENEMY_W as f32, ENEMY_H as f32) { continue; }
        if !g.bullets[bi].bounced && rand_range_f32(0.0, 1.0) < RICOCHET_CHANCE {
            let b = &mut g.bullets[bi];
            let side = if rand_range_f32(0.0, 1.0) < 0.5 { -1.0 } else { 1.0 };
            let a = 0.6 + rand_range_f32(0.0, 1.0) * 0.8;
            let v = BULLET_SPEED * 0.8;
            b.vx = side * v * a.sin();
            b.vy = v * a.cos();
            b.bounced = true;
            let (sx, sy) = (b.x, b.y);
            g.spawn_explosion(sx, sy, 0.18, BlipColor::new(1.0, 0.95, 0.7, 1.0));
            let take = &sfx.ricochet[(rand() % sfx.ricochet.len() as u32) as usize];
            play_sfx_volume(take, 0.7);
            return Some(Strike::Glanced);
        }
        shoot_down(g, ei, sfx, rand_range_f32(0.0, 1.0) < FLAMES_CHANCE);
        return Some(Strike::Down);
    }
    None
}

/// Boats don't fight back — a quick bonus target for a stray shot.
pub(super) fn hit_boats(g: &mut Game, sfx: &Sounds, bi: usize, start: (f32, f32)) {
    for boi in 0..MAX_BOATS {
        if !g.boats[boi].active { continue; }
        let (boat_x, boat_y) = (g.boats[boi].x, g.boats[boi].y);
        if !round_hits_rect(start, &g.bullets[bi], boat_x, boat_y, BOAT_W as f32, BOAT_H as f32) { continue; }
        g.bullets[bi].active = false;
        g.boats[boi].active = false;
        blip::bot::add("boats", 1.0);
        g.spawn_explosion(boat_x + BOAT_W as f32 / 2.0, boat_y + BOAT_H as f32 / 2.0, 1.3, EXPLOSION_ORANGE);
        g.score_at(boat_x + BOAT_W as f32 / 2.0, boat_y, 40 * g.sess.level);
        play_take(&sfx.enemy_explode, 1.0);
        return;
    }
}

/// Islands take several hits before the turret goes down — unlike the boats, this one shoots back, so knocking it
/// out is worth a lot more.
pub(super) fn hit_islands(g: &mut Game, sfx: &Sounds, bi: usize, start: (f32, f32), (bx, by): (f32, f32)) {
    for isi in 0..MAX_ISLANDS {
        if !g.islands[isi].active { continue; }
        let (iw, ih) = ISLAND_SIZES[g.islands[isi].size as usize];
        let (ix, iy) = (g.islands[isi].x, g.islands[isi].y);
        if !round_hits_rect(start, &g.bullets[bi], ix, iy, iw as f32, ih as f32) { continue; }
        g.bullets[bi].active = false;
        g.islands[isi].hp -= 1;
        g.spawn_explosion(bx, by, 0.5, EXPLOSION_ORANGE);
        if g.islands[isi].hp <= 0 {
            g.islands[isi].active = false;
            blip::bot::add("islands", 1.0);
            let (cx, cy) = (ix + iw as f32 / 2.0, iy + ih as f32 / 2.0);
            g.spawn_explosion(cx, cy, 1.9, EXPLOSION_ORANGE);
            g.score_at(cx, cy, ISLAND_SCORE[g.islands[isi].size as usize]);
            play_take(&sfx.boss_explode, 1.0);
        } else {
            play_take(&sfx.enemy_explode, 1.0);
        }
        return;
    }
}

/// The laser barrier's motor — the only part of it that's shootable, and not shootable at all while it's still
/// warming up (matches the beam itself not being able to hurt the player yet either).
pub(super) fn hit_motor(g: &mut Game, sfx: &Sounds, bi: usize, start: (f32, f32), (bx, by): (f32, f32)) {
    let (mx, my) = (g.barrier.motor_x - MOTOR_W / 2.0, g.barrier.y - MOTOR_H / 2.0);
    if !round_hits_rect(start, &g.bullets[bi], mx, my, MOTOR_W, MOTOR_H) { return; }
    g.bullets[bi].active = false;
    g.barrier.hp -= 1;
    g.spawn_explosion(bx, by, 0.6, EXPLOSION_ORANGE);
    if g.barrier.hp > 0 {
        play_take(&sfx.enemy_explode, 1.0);
        return;
    }
    g.barrier.active = false;
    blip::bot::add("barriers", 1.0);
    stop_sound(&sfx.barrier_hum);
    stop_sound(&sfx.barrier_hum2);
    let (mx, my) = (g.barrier.motor_x, g.barrier.y);
    g.spawn_explosion(mx, my, 2.0, EXPLOSION_ORANGE);
    g.score_at(mx, my, 300 * g.sess.level);
    play_take(&sfx.boss_explode, 1.0);
}

/// A power-up the plane flew through this frame: one more weapon level, or a bonus at the top.
pub(super) fn catch_powerups(g: &mut Game, player_before: (f32, f32), starts: &[Option<(f32, f32)>], sfx: &Sounds) {
    // `i` pairs the power-up slot with the swept start recorded for it this frame.
    for (i, start) in starts.iter().enumerate().take(MAX_POWERUPS) {
        if !g.powerups[i].active { continue; }
        let (from_x, from_y) = blip::swept_from(*start, (g.powerups[i].x, g.powerups[i].y));
        if caught(g, player_before, (from_x, from_y), (g.powerups[i].x, g.powerups[i].y), POW_W, POW_H) {
            g.powerups[i].active = false;
            blip::bot::add("powerups", 1.0);
            let (cx, cy) = (g.powerups[i].x + POW_W / 2.0, g.powerups[i].y + POW_H / 2.0);
            if g.weapon_level < MAX_WEAPON_LEVEL {
                g.weapon_level += 1;
                g.spawn_powerup_burst(cx, cy, g.weapon_level, weapon_tier_color(g.weapon_level));
                play_sfx(&sfx.powerup_up[(g.weapon_level - 2) as usize]);
                g.sess.add_score(30 * g.weapon_level);
                if g.weapon_level == MAX_WEAPON_LEVEL {
                    g.max_power_banner.start(MAX_POWER_BANNER_TIME);
                }
            } else {
                // Already at max: a small sparkle and a score bonus instead.
                g.spawn_explosion(cx, cy, 1.1, weapon_tier_color(MAX_WEAPON_LEVEL));
                play_sfx(&sfx.powerup_up[0]);
                g.sess.add_score(150);
            }
        }
    }
}

/// A health pickup the plane flew through this frame.
pub(super) fn catch_health(g: &mut Game, player_before: (f32, f32), starts: &[Option<(f32, f32)>], sfx: &Sounds) {
    // `i` pairs the pickup slot with the swept start recorded for it this frame.
    for (i, start) in starts.iter().enumerate().take(MAX_HEALTH_PICKUPS) {
        if !g.health_pickups[i].active { continue; }
        let (from_x, from_y) = blip::swept_from(*start, (g.health_pickups[i].x, g.health_pickups[i].y));
        if caught(g, player_before, (from_x, from_y), (g.health_pickups[i].x, g.health_pickups[i].y), HEALTH_W, HEALTH_H) {
            g.health_pickups[i].active = false;
            let (cx, cy) = (g.health_pickups[i].x + HEALTH_W / 2.0, g.health_pickups[i].y + HEALTH_H / 2.0);
            g.health = (g.health + HEALTH_RESTORE).min(PLAYER_HEALTH_MAX);
            g.spawn_explosion(cx, cy, 1.0, BLIP_GREEN);
            play_sfx(&sfx.health_pickup);
        }
    }
}

/// A round on the boss at `bx`,`by`: the hull, a laser or turret near it, and
/// the boss's end, which wins the game on the last level.
pub(super) fn hit_boss(g: &mut Game, bx: f32, by: f32, sfx: &Sounds) {
    g.boss.hp -= 1;
    g.spawn_explosion(bx, by, 0.6, EXPLOSION_ORANGE);
    hit_lasers(g, bx, by, sfx);
    hit_turret(g, bx, by, sfx);
    if g.boss.hp <= 0 { boss_down(g, sfx); }
}

/// A round on a laser turret damages it; enough and it is out.
fn hit_lasers(g: &mut Game, bx: f32, by: f32, sfx: &Sounds) {
    let lr = laser_radius(g.boss.tier) + 3.0;
    for i in 0..g.boss.n_lasers {
        if g.boss.lasers[i].hp <= 0 { continue; }
        let (lx, ly) = mount_pos(g, g.boss.lasers[i].fx, g.boss.lasers[i].fy);
        if (lx - bx).hypot(ly - by) < lr {
            g.boss.lasers[i].hp -= 1;
            if g.boss.lasers[i].hp <= 0 {
                g.spawn_explosion(lx, ly, 1.4, EXPLOSION_ORANGE);
                g.score_at(lx, ly, 150 * g.sess.level);
                play_take(&sfx.enemy_explode, 0.9);
            }
        }
    }
}

/// A round on a gun turret damages the first one it reaches too; enough and it is out.
fn hit_turret(g: &mut Game, bx: f32, by: f32, sfx: &Sounds) {
    let r = turret_radius(g.boss.tier) + 3.0;
    for k in 0..g.boss.n_turrets {
        if g.boss.turrets[k].hp <= 0 { continue; }
        let (tx, ty) = turret_pos(g, k);
        if (tx - bx).hypot(ty - by) < r {
            g.boss.turrets[k].hp -= 1;
            if g.boss.turrets[k].hp <= 0 {
                g.spawn_explosion(tx, ty, 1.0, EXPLOSION_ORANGE);
                g.score_at(tx, ty, 50 * g.sess.level);
                play_take(&sfx.enemy_explode, 0.7);
            }
            break;
        }
    }
}

/// The boss goes down, the bigger the longer: on to the next level, or after the last one the game is won.
fn boss_down(g: &mut Game, sfx: &Sounds) {
    let (bossw, bossh) = boss_size(g.boss.tier);
    let (cx, cy) = (g.boss.x + bossw / 2.0, g.boss.y + bossh / 2.0);
    g.boss.active = false;
    blip::bot::add("bosses", 1.0);
    let burst = 1.6 + g.boss.tier as f32 * 0.3;
    g.spawn_explosion(cx, cy, burst, EXPLOSION_ORANGE);
    g.spawn_explosion(cx - 14.0, cy - 8.0, burst * 0.6, EXPLOSION_ORANGE);
    g.spawn_explosion(cx + 14.0, cy + 6.0, burst * 0.6, EXPLOSION_ORANGE);
    g.score_at(cx, cy, 500 * g.sess.level);
    play_take(&sfx.boss_explode, 1.0);
    if g.sess.level >= MAX_LEVEL {
        g.sess.add_score(2000);
        blip::bot::finish("won");
        play_sfx(&sfx.victory);
        g.over_timer.start(OVER_MIN_WAIT);
        g.state = State::Won;
    } else {
        g.sess.next_level();
        g.wave_kills = 0;
        g.wave_target = wave_target_for(g.sess.level);
        g.win_timer.start(WIN_PAUSE);
        g.state = State::Win;
    }
}
