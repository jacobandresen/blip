//! Boarding scene: cutlass duel modeled as crew slots whittling each other down.

use blip::input::btn1_pressed;
use blip::macroquad::prelude::Color;
use blip::{
    play_music, play_sfx, web, Blip,
    BLIP_CYAN, BLIP_DARKGRAY, BLIP_GRAY, BLIP_ORANGE, BLIP_RED, BLIP_WHITE,
};

use crate::screens::draw_hud_canaris;
use crate::state::*;

pub fn update_boarding(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.boarding_t        -= dt;
    g.boarding_total_t  -= dt;
    g.time              += dt;
    if g.boarding_hit_t > 0.0 { g.boarding_hit_t -= dt; }

    // ── Player action (Button 1): attack the frontmost enemy slot ──
    if btn1_pressed() {
        // leftmost remaining enemy slot = the one closest to player territory
        for i in 0..BOARDING_SLOTS {
            if g.slots[i].owner == SlotOwner::Enemy {
                g.slots[i].hp -= 1;
                g.boarding_hit_slot = i;
                g.boarding_hit_t    = 0.28;
                play_sfx(&sfx.boarding_clash);
                if g.slots[i].hp <= 0 {
                    g.slots[i].owner = SlotOwner::Empty;
                    g.slots[i].hp    = 0;
                }
                break;
            }
        }
    }

    // ── Enemy auto-tick: attack rightmost remaining player slot ──
    if g.boarding_t <= 0.0 {
        g.boarding_t = BOARDING_TICK;
        // rightmost player slot = the one closest to the enemy side
        let mut hit: Option<usize> = None;
        for i in (0..BOARDING_SLOTS).rev() {
            if g.slots[i].owner == SlotOwner::Player {
                hit = Some(i);
                break;
            }
        }
        if let Some(i) = hit {
            g.slots[i].hp -= 1;
            g.boarding_hit_slot = i;
            g.boarding_hit_t    = 0.28;
            play_sfx(&sfx.boarding_clash);
            if g.slots[i].hp <= 0 {
                // Enemy crew captures this slot
                g.slots[i].owner = SlotOwner::Enemy;
                g.slots[i].hp    = 2;
            }
        }
    }

    // ── Outcome checks ──
    let enemy_alive  = g.slots.iter().any(|s| s.owner == SlotOwner::Enemy);
    let player_alive = g.slots.iter().any(|s| s.owner == SlotOwner::Player);

    if !enemy_alive {
        let idx  = g.combat_enemy_idx;
        let loot = g.enemies[idx].gold_loot * 2;
        g.player.gold += loot;
        g.score       += 150 * g.level + loot;
        if g.score > g.hi_score { g.hi_score = g.score; web::save_hi_score(web::GAME_CANARIS, g.hi_score); }
        g.enemies[idx].active = false;
        play_sfx(&sfx.coin_jingle);
        g.state = State::Sea;
        play_music(&sfx.sea_music);
        return;
    }

    if !player_alive {
        g.player.hull = 0;
        g.lives      -= 1;
        g.dead_t      = DEAD_TTL;
        play_sfx(&sfx.life_lost);
        g.state = State::Dead;
        play_music(&sfx.sea_music);
        return;
    }

    if g.boarding_total_t <= 0.0 {
        // Timeout — both sides withdraw, no loot
        g.state = State::Sea;
        play_music(&sfx.sea_music);
    }
}

pub fn draw_boarding(blip: &Blip, g: &Game, tex: &Textures) {
    // Night sea backdrop
    blip.fill_rect(0.0, HUD_H as f32, WIN_W as f32, (WIN_H - HUD_H) as f32,
                   Color::new(0.04, 0.07, 0.16, 1.0));

    // Title
    blip.draw_centered("BOARDING ACTION", HUD_H as f32 + 10.0, 3.0, BLIP_RED);

    // Side labels, centered in each half (each char ~7px at size 2 = 14px)
    blip.draw_text("YOUR CREW", 57.0, HUD_H as f32 + 40.0, 2.0, BLIP_CYAN);
    blip.draw_text("ENEMY CREW", 290.0, HUD_H as f32 + 40.0, 2.0, BLIP_RED);

    // Ship deck planks
    let deck_y = 100.0_f32;
    let deck_h = 200.0_f32;
    blip.fill_rect(0.0, deck_y, WIN_W as f32, deck_h,
                   Color::new(0.29, 0.18, 0.10, 1.0));
    for row in 1..5 {
        let py = deck_y + row as f32 * (deck_h / 5.0);
        blip.fill_rect(0.0, py, WIN_W as f32, 2.0, Color::new(0.17, 0.10, 0.05, 1.0));
    }
    // Deck rail
    blip.fill_rect(0.0, deck_y - 8.0, WIN_W as f32, 9.0, Color::new(0.45, 0.28, 0.14, 1.0));
    blip.fill_rect(0.0, deck_y - 8.0, WIN_W as f32, 2.0, Color::new(0.65, 0.42, 0.22, 1.0));

    // Centre battle-line divider
    blip.fill_rect(237.0, deck_y - 8.0, 6.0, deck_h + 8.0, BLIP_RED);
    blip.fill_rect(239.5, deck_y - 8.0, 2.0, deck_h + 8.0, Color::new(1.0, 0.55, 0.55, 1.0));

    // Crew slots — 3 player (left) + 3 enemy (right), 80px columns
    let fig_w   = 24.0_f32;
    let fig_h   = 40.0_f32;
    let fig_y   = deck_y + (deck_h - fig_h) / 2.0 - 8.0;
    let hp_y    = fig_y + fig_h + 5.0;
    let pip_w   = 18.0_f32;
    let pip_gap = 3.0_f32;
    let pips_w  = 3.0 * pip_w + 2.0 * pip_gap;

    for i in 0..BOARDING_SLOTS {
        let slot  = &g.slots[i];
        let col_x = i as f32 * 80.0;
        let fig_x = col_x + (80.0 - fig_w) / 2.0;

        // Hit flash
        if g.boarding_hit_slot == i && g.boarding_hit_t > 0.0 {
            let a = (g.boarding_hit_t / 0.28).min(1.0) * 0.55;
            blip.fill_rect(col_x + 1.0, deck_y + 2.0, 78.0, deck_h - 4.0,
                           Color::new(1.0, 1.0, 0.3, a));
        }

        match slot.owner {
            SlotOwner::Empty => {
                blip.fill_rect(fig_x + 4.0, fig_y + 8.0, fig_w - 8.0, fig_h - 8.0,
                               Color::new(0.15, 0.09, 0.05, 1.0));
                blip.draw_text("X", col_x + 30.0, fig_y + 12.0, 2.0, BLIP_DARKGRAY);
            }
            SlotOwner::Player => {
                blip.draw_texture_tinted(&tex.crew, fig_x, fig_y, fig_w, fig_h, BLIP_CYAN);
            }
            SlotOwner::Enemy => {
                blip.draw_texture_tinted(&tex.crew, fig_x, fig_y, fig_w, fig_h, BLIP_RED);
            }
        }

        // HP pips (3 max)
        if slot.owner != SlotOwner::Empty {
            let px0 = col_x + (80.0 - pips_w) / 2.0;
            for pip in 0..3_i32 {
                let px  = px0 + pip as f32 * (pip_w + pip_gap);
                let col = if pip < slot.hp {
                    match slot.owner {
                        SlotOwner::Player => BLIP_CYAN,
                        SlotOwner::Enemy  => BLIP_RED,
                        _                 => BLIP_GRAY,
                    }
                } else {
                    BLIP_DARKGRAY
                };
                blip.fill_rect(px, hp_y, pip_w, 6.0, col);
            }
        }
    }

    // Timeout bar
    let t_frac = (g.boarding_total_t / BOARDING_TIMEOUT).clamp(0.0, 1.0);
    let tb_y   = deck_y + deck_h + 16.0;
    let tb_x   = 38.0_f32;
    let tb_w   = WIN_W as f32 - 76.0;
    blip.draw_text("TIME", 4.0, tb_y + 1.0, 1.0, BLIP_GRAY);
    blip.fill_rect(tb_x - 1.0, tb_y - 1.0, tb_w + 2.0, 10.0, BLIP_DARKGRAY);
    blip.fill_rect(tb_x, tb_y, tb_w * t_frac, 8.0, BLIP_ORANGE);
    blip.draw_text(&format!("{:.0}s", g.boarding_total_t.max(0.0)),
                   tb_x + tb_w + 4.0, tb_y + 1.0, 1.0, BLIP_GRAY);

    // Crew-count totals
    let p_alive = g.slots.iter().filter(|s| s.owner == SlotOwner::Player).count();
    let e_alive = g.slots.iter().filter(|s| s.owner == SlotOwner::Enemy).count();
    blip.draw_text(&format!("{}/3", p_alive), 8.0, tb_y + 18.0, 2.0, BLIP_CYAN);
    blip.draw_text(&format!("{}/3", e_alive), WIN_W as f32 - 44.0, tb_y + 18.0, 2.0, BLIP_RED);

    // Controls hint
    blip.draw_centered("[1] ATTACK", tb_y + 38.0, 2.0, BLIP_WHITE);

    draw_hud_canaris(blip, g);
}
