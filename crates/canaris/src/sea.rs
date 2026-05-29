//! Sea scene: free-roam sailing, enemy engagement check, port docking.
//! Also exports background/foreground/ship-fire helpers reused by combat and title.

use blip::input::{btn1_pressed, key_held, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_LEFT, BLIP_KEY_RIGHT};
use blip::macroquad::prelude::Color;
use blip::macroquad::texture::Texture2D;
use blip::{clamp, play_music, play_sfx, rand_int, web, Blip, BLIP_WHITE, BLIP_YELLOW};

use crate::screens::draw_hud_canaris;
use crate::state::*;

// ── update ────────────────────────────────────────────────────────────────────

pub fn update_sea(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.time += dt;
    g.hi_score = g.hi_score.max(web::load_hi_score(web::GAME_CANARIS));

    // Player movement
    if key_held(BLIP_KEY_RIGHT) || key_held(BLIP_KEY_D) {
        g.player.vx = clamp(g.player.vx + 300.0 * dt, -PLAYER_SPEED, PLAYER_SPEED);
    } else if key_held(BLIP_KEY_LEFT) || key_held(BLIP_KEY_A) {
        g.player.vx = clamp(g.player.vx - 300.0 * dt, -PLAYER_SPEED, PLAYER_SPEED);
    } else {
        g.player.vx *= 1.0 - 5.0 * dt;
    }
    g.player.world_x = (g.player.world_x + g.player.vx * dt).rem_euclid(WORLD_W);

    // Vertical bob
    g.player.y = SEA_LANE_Y + (g.time * BOB_FREQ).sin() * BOB_AMP;

    // Camera
    let target_cam = g.player.world_x - WIN_W as f32 * 0.35;
    g.cam_x = clamp(target_cam, 0.0, WORLD_W - WIN_W as f32);

    // Sprite animation
    g.player.anim_t += dt;
    if g.player.anim_t >= ANIM_FRAME_DUR {
        g.player.anim_t = 0.0;
        g.player.anim_frame ^= 1;
    }

    // Food decay
    g.player.food -= FOOD_DECAY_RATE * dt;
    if g.player.food < 0.0 {
        g.player.food = 0.0;
        g.player.starve_t -= dt;
        if g.player.starve_t <= 0.0 {
            g.player.starve_t = FOOD_HULL_DMG_RATE;
            g.player.hull -= 1;
        }
    } else {
        g.player.starve_t = FOOD_HULL_DMG_RATE;
    }

    // Hull flash
    if g.player.hit_flash_t > 0.0 { g.player.hit_flash_t -= dt; }

    // Level timer
    g.level_t -= dt;
    if g.level_t <= 0.0 {
        g.level += 1;
        g.level_t = 60.0 + g.level as f32 * 10.0;
        g.score += 500 * g.level;
        if g.score > g.hi_score { g.hi_score = g.score; web::save_hi_score(web::GAME_CANARIS, g.hi_score); }
        g.spawn_enemies();
    }

    // Enemy movement + engagement check
    let enemy_speed = 30.0 + g.level as f32 * 5.0;
    for i in 0..MAX_ENEMIES {
        if !g.enemies[i].active { continue; }
        g.enemies[i].world_x -= enemy_speed * dt;
        // Bob enemies gently
        g.enemies[i].y = SEA_LANE_Y - 8.0 + (g.time * BOB_FREQ * 0.9 + i as f32).sin() * BOB_AMP;
        if g.enemies[i].world_x < -ENEMY_W {
            g.enemies[i].world_x = WORLD_W + rand_int(200, 800) as f32;
        }

        // Check engagement — suppress near port so the player can always dock
        let near_port = (g.player.world_x - PORT_ANCHOR_X).abs() < PORT_SAFE_RADIUS;
        let dist = (g.enemies[i].world_x - g.player.world_x).abs();
        if !near_port && dist < ENGAGEMENT_DIST {
            g.enter_combat(i);
            play_music(&sfx.combat_music);
            return;
        }
    }

    // Port docking — player must sail close and press Button 1
    if (g.player.world_x - PORT_ANCHOR_X).abs() < PORT_DOCK_RADIUS
        && btn1_pressed()
    {
        g.enter_port();
        play_music(&sfx.port_music);
        return;
    }

    // Death check
    if g.player.hull <= 0 {
        play_sfx(&sfx.life_lost);
        g.lives -= 1;
        g.dead_t = DEAD_TTL;
        g.state  = State::Dead;
    }
}

// ── draw helpers (shared across scenes) ───────────────────────────────────────

pub fn draw_sea_bg(blip: &Blip, tex_a: &Texture2D, tex_b: &Texture2D, cam_x: f32, time: f32) {
    let play_y    = HUD_H as f32;
    let play_h    = (WIN_H - HUD_H) as f32;
    let horizon_y = play_y + play_h * 0.32;

    // Sky
    blip.fill_rect(0.0, play_y, WIN_W as f32, horizon_y - play_y,
                   Color::new(0.05, 0.10, 0.22, 1.0));
    // Horizon glow strip
    blip.fill_rect(0.0, horizon_y - 2.0, WIN_W as f32, 4.0,
                   Color::new(0.10, 0.25, 0.40, 1.0));
    // Deep water
    blip.fill_rect(0.0, horizon_y, WIN_W as f32, (WIN_H as f32) - horizon_y,
                   Color::new(0.04, 0.18, 0.30, 1.0));

    let tile_w = 120.0_f32;
    // Pick A or B frame at 2.5 Hz; phase offset desynchronises the two layers
    let wave_tex = |phase: f32| -> &Texture2D {
        if ((time + phase) * 2.5) as u32 % 2 == 0 { tex_a } else { tex_b }
    };

    // Layer 1 — horizon waves, full colour, fastest scroll
    {
        let tex    = wave_tex(0.0);
        let offset = (cam_x + time * 18.0).rem_euclid(tile_w);
        let mut sx = -offset;
        while sx < WIN_W as f32 {
            blip.draw_texture(tex, sx, horizon_y - 10.0, tile_w, 40.0);
            sx += tile_w;
        }
    }

    // Layer 2 — mid-sea waves, slightly desaturated, medium scroll
    {
        let tex    = wave_tex(0.4);
        let offset = (cam_x * 1.05 + time * 11.0).rem_euclid(tile_w);
        let tint   = Color::new(0.8, 0.9, 1.0, 1.0);
        let mut sx = -offset;
        while sx < WIN_W as f32 {
            blip.draw_texture_tinted(tex, sx, horizon_y + 90.0, tile_w, 40.0, tint);
            sx += tile_w;
        }
    }
}

pub fn draw_sea_foreground(blip: &Blip, tex_a: &Texture2D, tex_b: &Texture2D, cam_x: f32, time: f32) {
    let tile_w = 120.0_f32;
    let tex    = if ((time * 1.8) as u32) % 2 == 0 { tex_a } else { tex_b };
    let offset = (cam_x * 1.15 + time * 5.0).rem_euclid(tile_w);
    let tint   = Color::new(0.7, 0.85, 1.0, 0.85);
    let fg_y   = SEA_LANE_Y + 18.0;
    let mut sx = -offset;
    while sx < WIN_W as f32 {
        blip.draw_texture_tinted(tex, sx, fg_y, tile_w, 48.0, tint);
        sx += tile_w;
    }
}

pub fn draw_ship_fire(blip: &Blip, sx: f32, sy: f32, hull: i32, hull_max: i32, time: f32) {
    if hull * 2 >= hull_max { return; }

    let intensity = 1.0 - (hull as f32 / (hull_max as f32 * 0.5)).clamp(0.0, 1.0);
    let n_flames: usize = if hull * 4 < hull_max { 4 } else { 2 };

    const X_OFF: [f32; 4] = [8.0, 18.0, 30.0, 14.0];
    const FREQS: [f32; 4] = [7.3,  9.1, 11.7,  6.8];

    for i in 0..n_flames {
        let flicker = (time * FREQS[i]).sin() * 0.35 + 0.65;
        let h = (6.0 + intensity * 10.0) * flicker;
        let w = 4.0_f32;
        let fx = sx + X_OFF[i] + (time * 3.1 + i as f32 * 1.7).sin() * 1.5;
        // Base on deck (sy+16); flames grow upward — never touches hull/waterline at sy+22+
        let fy = sy + 16.0 - h;
        let seg = h / 3.0;
        blip.fill_rect(fx,       fy + seg * 2.0, w,       seg, Color::new(0.75, 0.15, 0.0, 0.90));
        blip.fill_rect(fx,       fy + seg,        w,       seg, Color::new(1.0,  0.45, 0.0, 0.90));
        blip.fill_rect(fx + 0.5, fy,              w - 1.0, seg, Color::new(1.0,  0.90, 0.1, 0.85));
    }
}

#[inline]
pub fn world_to_screen(wx: f32, cam_x: f32) -> f32 { wx - cam_x }

// ── draw ──────────────────────────────────────────────────────────────────────

pub fn draw_sea(blip: &Blip, g: &Game, tex: &Textures) {
    draw_sea_bg(blip, &tex.sea_wave, &tex.sea_wave_b, g.cam_x, g.time);

    // Enemies
    for e in g.enemies.iter() {
        if !e.active { continue; }
        let sx = world_to_screen(e.world_x, g.cam_x);
        if sx < -ENEMY_W || sx > WIN_W as f32 { continue; }
        let et = if e.anim_frame == 0 { &tex.enemy_a } else { &tex.enemy_b };
        if e.hit_flash_t > 0.0 {
            blip.draw_texture_tinted(et, sx, e.y, ENEMY_W, ENEMY_H, BLIP_WHITE);
        } else {
            blip.draw_texture(et, sx, e.y, ENEMY_W, ENEMY_H);
        }
        draw_ship_fire(blip, sx, e.y, e.hull, e.hull_max, g.time);
    }

    // Port marker
    let port_sx = world_to_screen(PORT_ANCHOR_X, g.cam_x);
    if port_sx > -20.0 && port_sx < WIN_W as f32 + 20.0 {
        blip.fill_rect(port_sx, WIN_H as f32 - 40.0, 8.0, 30.0, BLIP_YELLOW);
        blip.draw_text("PORT", port_sx - 10.0, WIN_H as f32 - 52.0, 1.0, BLIP_YELLOW);
        let near = (g.player.world_x - PORT_ANCHOR_X).abs() < PORT_DOCK_RADIUS;
        if near && (g.time * 2.0) as u32 % 2 == 0 {
            blip.draw_centered("[1] DOCK", WIN_H as f32 - 65.0, 1.0, BLIP_YELLOW);
        }
    }

    // Player
    let psx = world_to_screen(g.player.world_x, g.cam_x);
    let pt  = if g.player.anim_frame == 0 { &tex.player_a } else { &tex.player_b };
    if g.player.hit_flash_t > 0.0 {
        blip.draw_texture_tinted(pt, psx, g.player.y, PLAYER_W, PLAYER_H, BLIP_WHITE);
    } else {
        blip.draw_texture(pt, psx, g.player.y, PLAYER_W, PLAYER_H);
    }
    draw_ship_fire(blip, psx, g.player.y, g.player.hull, PLAYER_HULL_MAX, g.time);

    // Wake trail when moving fast enough
    if g.player.vx.abs() > 5.0 {
        let wake_col = Color::new(0.5, 0.85, 1.0, 0.5);
        let wake_y   = g.player.y + PLAYER_H - 4.0;
        let offsets: [f32; 4]        = [6.0, 14.0, 24.0, 36.0];
        let sizes:   [(f32, f32); 4] = [(6.0, 4.0), (5.0, 3.0), (4.0, 2.0), (3.0, 2.0)];
        for (i, &off) in offsets.iter().enumerate() {
            let wx = if g.player.vx > 0.0 {
                psx - off
            } else {
                psx + PLAYER_W + off - sizes[i].0
            };
            blip.fill_rect(wx, wake_y, sizes[i].0, sizes[i].1, wake_col);
        }
    }

    // Foreground wave layer drawn over hull bottoms — ships appear at the waterline
    draw_sea_foreground(blip, &tex.sea_wave, &tex.sea_wave_b, g.cam_x, g.time);

    draw_hud_canaris(blip, g);
}
