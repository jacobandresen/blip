//! Title, death, and game-over scenes, plus the canaris-specific HUD overlay
//! used by every in-game scene.

use blip::input::any_key_pressed;
use blip::macroquad::prelude::Color;
use blip::{
    play_music, web, Blip,
    BLIP_BLACK, BLIP_CYAN, BLIP_GRAY, BLIP_GREEN, BLIP_ORANGE, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
};

use crate::sea::{draw_sea, draw_sea_bg};
use crate::state::*;

// ── HUD shared by sea/combat/boarding/port/map ────────────────────────────────

pub fn draw_hud_canaris(blip: &Blip, g: &Game) {
    blip.draw_hud(g.score, g.hi_score, g.lives);
    // Secondary stat row at y=20 (within the 28px HUD black band, below the score row)
    let y = 20.0_f32;
    let hull_col = if g.player.hull < 6 { BLIP_RED } else { BLIP_GREEN };
    let food_col = if g.player.food < 5.0 { BLIP_RED } else { BLIP_WHITE };
    blip.draw_text(&format!("H:{}", g.player.hull),    4.0,  y, 1.0, hull_col);
    blip.draw_text(&format!("G:{}", g.player.gold),  100.0,  y, 1.0, BLIP_YELLOW);
    blip.draw_text(&format!("F:{}", g.player.food as i32), 210.0, y, 1.0, food_col);
    blip.draw_text(&format!("C:{}", g.player.cannons), 320.0, y, 1.0, BLIP_ORANGE);
    blip.draw_text(&format!("LV{}", g.level),         410.0,  y, 1.0, BLIP_CYAN);
}

// ── title ─────────────────────────────────────────────────────────────────────

pub fn update_title(g: &mut Game, dt: f32) {
    g.time += dt;
    g.hi_score = g.hi_score.max(web::load_hi_score(web::GAME_CANARIS));
    if any_key_pressed() {
        g.start_game();
    }
}

pub fn draw_title(blip: &Blip, g: &Game, tex: &Textures) {
    draw_sea_bg(blip, &tex.sea_wave, &tex.sea_wave_b, g.time * 20.0, g.time);

    // Ship bobbing on the horizon
    let bob = (g.time * BOB_FREQ).sin() * BOB_AMP;
    let ship_w = PLAYER_W * 2.0;
    let ship_h = PLAYER_H * 2.0;
    let ship_x = WIN_W as f32 / 2.0 - ship_w / 2.0;
    let ship_y = WIN_H as f32 * 0.40 + bob;
    let ship_t = if (g.time * (1.0 / ANIM_FRAME_DUR)) as u32 % 2 == 0 { &tex.player_a } else { &tex.player_b };
    blip.draw_texture(ship_t, ship_x, ship_y, ship_w, ship_h);

    blip.draw_centered("CANARIS",               WIN_H as f32 * 0.10, 6.0, BLIP_CYAN);
    blip.draw_centered("PRIVATEER OF KATTEGAT", WIN_H as f32 * 0.22, 2.0, BLIP_YELLOW);

    if g.hi_score > 0 {
        let hi_str = format!("BEST {}", g.hi_score);
        blip.draw_centered(&hi_str, WIN_H as f32 * 0.63, 2.0, BLIP_YELLOW);
    }
    // Blink "PRESS ANY KEY" at 1Hz
    if (g.time * 2.0) as u32 % 2 == 0 {
        blip.draw_centered("PRESS ANY KEY",     WIN_H as f32 * 0.72, 3.0, BLIP_WHITE);
    }
    blip.draw_centered("SAIL. RAID. PLUNDER.",  WIN_H as f32 * 0.84, 2.0, BLIP_GRAY);
}

// ── dead ──────────────────────────────────────────────────────────────────────

pub fn update_dead(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.dead_t -= dt;
    if g.dead_t <= 0.0 {
        if g.lives > 0 {
            g.respawn_at_sea();
            play_music(&sfx.sea_music);
        } else {
            g.state = State::GameOver;
        }
    }
}

pub fn draw_dead(blip: &Blip, g: &Game, tex: &Textures) {
    let t = g.dead_t / DEAD_TTL;
    if t > 0.7 {
        // Flash red
        blip.clear(BLIP_RED);
    } else {
        draw_sea(blip, g, tex);
        // Fade-out overlay
        let alpha = (1.0 - t / 0.7).min(1.0);
        let dim = Color::new(0.0, 0.0, 0.0, alpha * 0.6);
        blip.fill_rect(0.0, 0.0, WIN_W as f32, WIN_H as f32, dim);
    }
    blip.draw_centered("SHIP SUNK!", (WIN_H / 2) as f32, 4.0, BLIP_RED);
    let lives_str = format!("LIVES {}", g.lives);
    blip.draw_centered(&lives_str, (WIN_H / 2 + 40) as f32, 3.0, BLIP_WHITE);
}

// ── game over ─────────────────────────────────────────────────────────────────

pub fn update_gameover(g: &mut Game) {
    g.hi_score = g.hi_score.max(web::load_hi_score(web::GAME_CANARIS));
    web::game_over(web::GAME_CANARIS, g.score);
    if any_key_pressed() {
        web::spend_coin();
        g.state = State::Title;
    }
}

pub fn draw_gameover(blip: &Blip, g: &Game) {
    blip.clear(BLIP_BLACK);
    blip.draw_centered("GAME OVER",     (WIN_H / 4) as f32,     5.0, BLIP_RED);
    let score_str = format!("SCORE {}", g.score);
    blip.draw_centered(&score_str,      (WIN_H / 2) as f32,     3.0, BLIP_WHITE);
    let hi_str = format!("BEST  {}", g.hi_score);
    blip.draw_centered(&hi_str,         (WIN_H / 2 + 30) as f32, 2.0, BLIP_YELLOW);
    blip.draw_centered("PRESS ANY KEY", (WIN_H * 2 / 3) as f32, 3.0, BLIP_CYAN);
}
