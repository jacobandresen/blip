//! Combat scene: side-by-side cannon duel with vertical dodge and boarding option.

use blip::input::{btn1_pressed, btn2_pressed, key_held, BLIP_KEY_DOWN, BLIP_KEY_S, BLIP_KEY_UP, BLIP_KEY_W};
use blip::macroquad::prelude::Color;
use blip::{
    clamp, play_music, play_sfx, rand_int, rects_overlap, web, Blip,
    BLIP_CYAN, BLIP_DARKGRAY, BLIP_GRAY, BLIP_GREEN, BLIP_ORANGE, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
};

use crate::screens::draw_hud_canaris;
use crate::sea::{draw_sea_bg, draw_ship_fire};
use crate::state::*;

pub fn update_combat(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.time += dt;
    g.retreat_t -= dt;

    let pidx = g.combat_enemy_idx;

    // Player dodge (UP/DOWN)
    if key_held(BLIP_KEY_UP) || key_held(BLIP_KEY_W) {
        g.player.y = clamp(g.player.y - DODGE_SPEED * dt, COMBAT_Y_MIN, COMBAT_Y_MAX);
    }
    if key_held(BLIP_KEY_DOWN) || key_held(BLIP_KEY_S) {
        g.player.y = clamp(g.player.y + DODGE_SPEED * dt, COMBAT_Y_MIN, COMBAT_Y_MAX);
    }

    // Player fire (Button 1)
    g.player.reload_t -= dt;
    if btn1_pressed() && g.player.reload_t <= 0.0 && g.player.cannons > 0 {
        g.fire_ball(true, COMBAT_PLAYER_X + PLAYER_W, g.player.y + PLAYER_H / 2.0);
        g.player.reload_t = PLAYER_RELOAD;
        g.player.cannons -= 1;
        play_sfx(&sfx.cannon_fire);
    }

    // Boarding (Button 2) — only when ships are close in Y
    let y_gap = (g.player.y - g.enemies[pidx].combat_y).abs();
    if btn2_pressed() && y_gap < BOARD_Y_DIST {
        g.enter_boarding();
        return;
    }

    // Enemy AI — oscillating dodge pattern, not player-tracking
    let osc = (g.time * 0.7 + pidx as f32 * 1.3).sin();
    let target_y = COMBAT_BASE_Y + osc * (COMBAT_Y_MAX - COMBAT_Y_MIN) * 0.38;
    let ey = &mut g.enemies[pidx].combat_y;
    let ev = &mut g.enemies[pidx].combat_vy;
    *ev += (target_y - *ey) * 2.0 * dt;
    *ev *= 1.0 - 4.0 * dt;
    *ey = clamp(*ey + *ev * dt, COMBAT_Y_MIN, COMBAT_Y_MAX);

    g.enemies[pidx].reload_t -= dt;
    if g.enemies[pidx].reload_t <= 0.0 {
        let reload = (ENEMY_RELOAD_BASE - g.level as f32 * 0.15).max(0.6);
        g.enemies[pidx].reload_t = reload;
        // Aim slightly toward player Y, base arc upward
        let aim_vy = (g.player.y - g.enemies[pidx].combat_y).clamp(-60.0, 60.0);
        let ex = COMBAT_ENEMY_X;
        let ey = g.enemies[pidx].combat_y + ENEMY_H / 2.0;
        for i in 2..MAX_CANNONBALLS {
            if !g.cannonballs[i].active {
                g.cannonballs[i] = Cannonball {
                    active: true, x: ex, y: ey,
                    vx: -CANNON_SPEED, vy: -CANNON_ARC_VY + aim_vy * 0.5, player: false,
                };
                break;
            }
        }
        play_sfx(&sfx.cannon_fire);
    }

    // Sprite animation
    g.player.anim_t += dt;
    if g.player.anim_t >= ANIM_FRAME_DUR {
        g.player.anim_t = 0.0;
        g.player.anim_frame ^= 1;
    }
    g.enemies[pidx].anim_t += dt;
    if g.enemies[pidx].anim_t >= ANIM_FRAME_DUR {
        g.enemies[pidx].anim_t = 0.0;
        g.enemies[pidx].anim_frame ^= 1;
    }

    // Hit flash timers
    if g.player.hit_flash_t > 0.0        { g.player.hit_flash_t -= dt; }
    if g.enemies[pidx].hit_flash_t > 0.0 { g.enemies[pidx].hit_flash_t -= dt; }

    // Move cannonballs (index loop avoids borrow conflict with spawn_explosion)
    for i in 0..MAX_CANNONBALLS {
        if !g.cannonballs[i].active { continue; }
        g.cannonballs[i].vy += CANNON_GRAVITY * dt;
        g.cannonballs[i].x  += g.cannonballs[i].vx * dt;
        g.cannonballs[i].y  += g.cannonballs[i].vy * dt;
        let bx = g.cannonballs[i].x;
        let by = g.cannonballs[i].y;
        let is_player = g.cannonballs[i].player;

        // Above HUD (mid-arc): keep flying, gravity will bring it back
        if by < HUD_H as f32 { continue; }

        // Exits screen — spawn splash at the edge where it left
        let off_left  = bx < -BALL_W;
        let off_right = bx > WIN_W as f32 + BALL_W;
        let off_bot   = by > WIN_H as f32;
        if off_left || off_right || off_bot {
            let sx = if off_left  { 4.0 }
                     else if off_right { WIN_W as f32 - 4.0 }
                     else { bx.clamp(4.0, WIN_W as f32 - 4.0) };
            let sy = by.clamp(HUD_H as f32 + 10.0, WIN_H as f32 - 8.0);
            g.spawn_splash(sx, sy);
            play_sfx(&sfx.splash);
            g.cannonballs[i].active = false;
            continue;
        }

        // Hit player ship
        if !is_player && rects_overlap(bx, by, BALL_W, BALL_H,
            COMBAT_PLAYER_X, g.player.y, PLAYER_W, PLAYER_H)
        {
            g.cannonballs[i].active = false;
            g.player.hull -= 2;
            g.player.hit_flash_t = 0.18;
            let ex = COMBAT_PLAYER_X + PLAYER_W / 2.0;
            let ey_p = g.player.y + PLAYER_H / 2.0;
            g.spawn_explosion(ex, ey_p);
            play_sfx(&sfx.hull_hit);
        }

        // Hit enemy ship
        if is_player && rects_overlap(bx, by, BALL_W, BALL_H,
            COMBAT_ENEMY_X, g.enemies[pidx].combat_y, ENEMY_W, ENEMY_H)
        {
            g.cannonballs[i].active = false;
            g.enemies[pidx].hull -= 2;
            g.enemies[pidx].hit_flash_t = 0.18;
            let ex = COMBAT_ENEMY_X + ENEMY_W / 2.0;
            let ey_e = g.enemies[pidx].combat_y + ENEMY_H / 2.0;
            g.spawn_explosion(ex, ey_e);
            play_sfx(&sfx.hull_hit);
        }
    }

    // Explosions + splashes
    for e in g.explosions.iter_mut() {
        if e.active { e.ttl -= dt; if e.ttl <= 0.0 { e.active = false; } }
    }
    for s in g.splashes.iter_mut() {
        if s.active { s.ttl -= dt; if s.ttl <= 0.0 { s.active = false; } }
    }

    // Win: enemy sunk
    if g.enemies[pidx].hull <= 0 {
        g.enemies[pidx].active = false;
        let loot = g.enemies[pidx].gold_loot;
        g.player.gold += loot;
        g.score += 200 * g.level;
        if g.score > g.hi_score { g.hi_score = g.score; web::save_hi_score(web::GAME_CANARIS, g.hi_score); }
        play_sfx(&sfx.coin_jingle);
        g.spawn_explosion(COMBAT_ENEMY_X + ENEMY_W / 2.0, g.enemies[pidx].combat_y + ENEMY_H / 2.0);
        for b in g.cannonballs.iter_mut() { b.active = false; }
        g.state = State::Sea;
        play_music(&sfx.sea_music);
        return;
    }

    // Player death
    if g.player.hull <= 0 {
        play_sfx(&sfx.life_lost);
        g.lives -= 1;
        g.dead_t = DEAD_TTL;
        for b in g.cannonballs.iter_mut() { b.active = false; }
        g.state = State::Dead;
        return;
    }

    // Retreat: push enemy far away so it can't immediately re-engage
    if g.retreat_t <= 0.0 {
        g.enemies[pidx].world_x = g.player.world_x + WORLD_W * 0.2
            + rand_int(200, 600) as f32;
        for b in g.cannonballs.iter_mut() { b.active = false; }
        g.state = State::Sea;
        play_music(&sfx.sea_music);
    }
}

pub fn draw_combat(blip: &Blip, g: &Game, tex: &Textures) {
    let pidx = g.combat_enemy_idx;

    // Scrolling sea background
    draw_sea_bg(blip, &tex.sea_wave, &tex.sea_wave_b, g.time * 15.0, g.time);

    // Player ship — with muzzle flash: player just fired if reload_t is nearly full
    let pt = if g.player.anim_frame == 0 { &tex.player_a } else { &tex.player_b };
    if g.player.hit_flash_t > 0.0 {
        blip.draw_texture_tinted(pt, COMBAT_PLAYER_X, g.player.y, PLAYER_W, PLAYER_H, BLIP_WHITE);
    } else {
        blip.draw_texture(pt, COMBAT_PLAYER_X, g.player.y, PLAYER_W, PLAYER_H);
    }
    // Muzzle flash: bright spark at cannon mouth for first 0.08s of reload
    if g.player.reload_t > PLAYER_RELOAD - 0.08 && g.player.reload_t > 0.0 {
        let fx = COMBAT_PLAYER_X + PLAYER_W + 2.0;
        let fy = g.player.y + PLAYER_H * 0.55 - 4.0;
        blip.fill_rect(fx, fy, 10.0, 8.0, BLIP_YELLOW);
        blip.fill_rect(fx + 2.0, fy + 1.0, 6.0, 6.0, BLIP_WHITE);
    }
    draw_ship_fire(blip, COMBAT_PLAYER_X, g.player.y, g.player.hull, PLAYER_HULL_MAX, g.time);

    // Enemy ship
    let et = if g.enemies[pidx].anim_frame == 0 { &tex.enemy_a } else { &tex.enemy_b };
    let enemy_cy = g.enemies[pidx].combat_y;
    if g.enemies[pidx].hit_flash_t > 0.0 {
        blip.draw_texture_tinted(et, COMBAT_ENEMY_X, enemy_cy, ENEMY_W, ENEMY_H, BLIP_WHITE);
    } else {
        blip.draw_texture(et, COMBAT_ENEMY_X, enemy_cy, ENEMY_W, ENEMY_H);
    }

    // Enemy muzzle flash (left side, firing toward player)
    if g.enemies[pidx].reload_t > (ENEMY_RELOAD_BASE - g.level as f32 * 0.15).max(0.6) - 0.08 {
        let fx = COMBAT_ENEMY_X - 12.0;
        let fy = enemy_cy + ENEMY_H * 0.55 - 4.0;
        blip.fill_rect(fx, fy, 10.0, 8.0, BLIP_ORANGE);
        blip.fill_rect(fx + 2.0, fy + 1.0, 6.0, 6.0, BLIP_YELLOW);
    }
    draw_ship_fire(blip, COMBAT_ENEMY_X, enemy_cy, g.enemies[pidx].hull, g.enemies[pidx].hull_max, g.time);

    // Cannonballs
    for b in g.cannonballs.iter() {
        if !b.active { continue; }
        blip.draw_texture(&tex.ball, b.x, b.y, BALL_W, BALL_H);
    }

    // Explosions — grow from small to large then shrink
    for e in g.explosions.iter() {
        if !e.active { continue; }
        let t    = e.ttl / EXPLOSION_TTL;
        let size = 20.0 + (1.0 - t) * 52.0; // 72 → 20 px
        blip.draw_texture(&tex.explosion, e.x - size / 2.0, e.y - size / 2.0, size, size);
    }

    // Water splashes — expanding oval rings with rising droplets
    for s in g.splashes.iter() {
        if !s.active { continue; }
        let age  = 1.0 - s.ttl / SPLASH_TTL; // 0 fresh → 1 old
        let fade = s.ttl / SPLASH_TTL;
        let col  = Color::new(0.65, 0.88, 1.0, fade * 0.9);
        // Two expanding rings
        for ring in 0..2u32 {
            let r  = (age * 22.0 + ring as f32 * 9.0).max(0.0);
            let ry = (r * 0.42).max(1.0); // oval (wider than tall)
            blip.fill_rect(s.x - r,      s.y - ry,        r * 2.0, 2.0, col);
            blip.fill_rect(s.x - r,      s.y + ry - 2.0,  r * 2.0, 2.0, col);
            blip.fill_rect(s.x - r,      s.y - ry,        2.0, ry * 2.0, col);
            blip.fill_rect(s.x + r - 2.0,s.y - ry,        2.0, ry * 2.0, col);
        }
        // Upward droplets in the first half of the animation
        if age < 0.45 {
            let up = age * 32.0;
            let dc = Color::new(0.8, 0.95, 1.0, fade);
            blip.fill_rect(s.x - 1.5, s.y - up,        3.0, 5.0, dc);
            blip.fill_rect(s.x + 6.0, s.y - up * 0.72, 2.0, 4.0, dc);
            blip.fill_rect(s.x - 8.0, s.y - up * 0.65, 2.0, 4.0, dc);
        }
    }

    // ── Bottom UI strip ──────────────────────────────────────────────────────
    // Dark panel behind UI
    blip.fill_rect(0.0, COMBAT_UI_Y - 4.0, WIN_W as f32, WIN_H as f32 - COMBAT_UI_Y + 4.0,
                   Color::new(0.0, 0.0, 0.0, 0.55));

    // Row 1 (COMBAT_UI_Y): hull bars
    let bar_w = 90.0_f32;
    let bar_h = 8.0_f32;
    let bar_y = COMBAT_UI_Y;

    // Player hull bar (left side, centered on player ship)
    let player_bar_x = COMBAT_PLAYER_X + PLAYER_W / 2.0 - bar_w / 2.0;
    let phull_frac = (g.player.hull as f32 / PLAYER_HULL_MAX as f32).clamp(0.0, 1.0);
    let p_col = if phull_frac < 0.3 { BLIP_RED } else if phull_frac < 0.6 { BLIP_YELLOW } else { BLIP_GREEN };
    blip.fill_rect(player_bar_x, bar_y, bar_w, bar_h, BLIP_DARKGRAY);
    blip.fill_rect(player_bar_x, bar_y, bar_w * phull_frac, bar_h, p_col);
    blip.draw_text("YOU",  player_bar_x, bar_y - 8.0, 1.0, BLIP_WHITE);

    // Enemy hull bar (right side, centered on enemy ship)
    let enemy_bar_x = COMBAT_ENEMY_X + ENEMY_W / 2.0 - bar_w / 2.0;
    let ehull_frac = (g.enemies[pidx].hull as f32 / g.enemies[pidx].hull_max as f32).clamp(0.0, 1.0);
    let e_col = if ehull_frac < 0.3 { BLIP_GREEN } else if ehull_frac < 0.6 { BLIP_YELLOW } else { BLIP_RED };
    blip.fill_rect(enemy_bar_x, bar_y, bar_w, bar_h, BLIP_DARKGRAY);
    blip.fill_rect(enemy_bar_x, bar_y, bar_w * ehull_frac, bar_h, e_col);
    blip.draw_text("ENEMY", enemy_bar_x, bar_y - 8.0, 1.0, BLIP_WHITE);

    // Row 2: retreat timer bar (centered)
    let rt_y = COMBAT_UI_Y + bar_h + 5.0;
    let rt = (g.retreat_t / RETREAT_TIMER).clamp(0.0, 1.0);
    let rt_w = WIN_W as f32 * 0.5;
    blip.fill_rect(WIN_W as f32 / 2.0 - rt_w / 2.0, rt_y, rt_w, 4.0, BLIP_DARKGRAY);
    blip.fill_rect(WIN_W as f32 / 2.0 - rt_w / 2.0, rt_y, rt_w * rt, 4.0, BLIP_ORANGE);
    blip.draw_text("TIME", WIN_W as f32 / 2.0 - 10.0, rt_y - 7.0, 1.0, BLIP_ORANGE);

    // Row 3: boarding hint / no-ammo warning
    let hint_y = rt_y + 8.0;
    let y_gap = (g.player.y - g.enemies[pidx].combat_y).abs();
    if g.player.cannons == 0 {
        if (g.time * 3.0) as u32 % 2 == 0 {
            blip.draw_centered("NO AMMO - [2] BOARD OR RETREAT", hint_y, 1.0, BLIP_RED);
        }
    } else if y_gap < BOARD_Y_DIST {
        blip.draw_centered("[1] FIRE  [2] BOARD", hint_y, 1.0, BLIP_CYAN);
    } else {
        blip.draw_centered("[1] FIRE  [UP/DN] DODGE", hint_y, 1.0, BLIP_GRAY);
    }

    draw_hud_canaris(blip, g);
}
