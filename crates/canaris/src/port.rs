//! Port scene (shop / set sail) and the World Map zone selector reached from it.

use blip::input::{
    btn1_pressed, btn2_pressed, key_pressed,
    BLIP_KEY_DOWN, BLIP_KEY_S, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::input::KeyCode;
use blip::macroquad::prelude::Color;
use blip::{
    play_music, play_sfx, Blip,
    BLIP_BLACK, BLIP_CYAN, BLIP_DARKGRAY, BLIP_GRAY, BLIP_GREEN, BLIP_ORANGE, BLIP_RED,
    BLIP_WHITE, BLIP_YELLOW,
};

use crate::screens::draw_hud_canaris;
use crate::state::*;

// ── port (shop / set sail) ────────────────────────────────────────────────────

pub fn update_port(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.port_msg_t -= dt;

    if key_pressed(BLIP_KEY_UP) || key_pressed(BLIP_KEY_W) {
        g.port_cursor = g.port_cursor.prev();
    }
    if key_pressed(BLIP_KEY_DOWN) || key_pressed(BLIP_KEY_S) {
        g.port_cursor = g.port_cursor.next();
    }

    let confirm = btn1_pressed() || key_pressed(KEY_ENTER);
    if confirm {
        match g.port_cursor {
            PortItem::Sail => {
                g.spawn_enemies();
                g.state = State::Sea;
                play_music(&sfx.sea_music);
            }
            PortItem::Map => {
                g.state = State::Map;
            }
            item => {
                let cost = item.cost();
                if g.player.gold >= cost {
                    g.player.gold -= cost;
                    match item {
                        PortItem::Repair  => { g.player.hull = PLAYER_HULL_MAX; g.port_msg = "HULL REPAIRED"; }
                        PortItem::Crew    => { g.player.crew += 2; g.port_msg = "CREW HIRED"; }
                        PortItem::Cannons => { g.player.cannons += 4; g.port_msg = "CANNONS LOADED"; }
                        PortItem::Food    => { g.player.food = FOOD_MAX as f32; g.port_msg = "PROVISIONS STOCKED"; }
                        PortItem::Sail    => {}
                        PortItem::Map     => {}
                    }
                    g.port_msg_t  = 1.5;
                    g.port_msg_ok = true;
                    play_sfx(&sfx.coin_jingle);
                } else {
                    g.port_msg    = "NOT ENOUGH GOLD";
                    g.port_msg_t  = 1.5;
                    g.port_msg_ok = false;
                }
            }
        }
    }
}

pub fn draw_port(blip: &Blip, g: &Game, tex: &Textures) {
    blip.draw_texture(&tex.port_bg, 0.0, HUD_H as f32, WIN_W as f32, (WIN_H - HUD_H) as f32);

    let panel_x = WIN_W as f32 * 0.12;
    let panel_w = WIN_W as f32 * 0.76;

    // ── Player stats strip ────────────────────────────────────────────────────
    let stats_y = 218.0_f32;
    let stats_h = 52.0_f32;
    blip.fill_rect(panel_x - 2.0, stats_y - 2.0, panel_w + 4.0, stats_h + 4.0, BLIP_DARKGRAY);
    blip.fill_rect(panel_x, stats_y, panel_w, stats_h, Color::new(0.04, 0.07, 0.12, 1.0));

    let hull_frac = (g.player.hull as f32 / PLAYER_HULL_MAX as f32).clamp(0.0, 1.0);
    let hull_col  = if hull_frac > 0.5 { BLIP_GREEN } else if hull_frac > 0.25 { BLIP_YELLOW } else { BLIP_RED };
    blip.draw_text("HULL", panel_x + 4.0, stats_y + 4.0, 1.0, BLIP_GRAY);
    blip.draw_text(&format!("{}/{}", g.player.hull, PLAYER_HULL_MAX),
                   panel_x + panel_w - 52.0, stats_y + 4.0, 1.0, hull_col);
    blip.fill_rect(panel_x + 4.0, stats_y + 14.0, panel_w - 8.0, 8.0, BLIP_DARKGRAY);
    blip.fill_rect(panel_x + 4.0, stats_y + 14.0, (panel_w - 8.0) * hull_frac, 8.0, hull_col);

    let col_w = (panel_w - 8.0) / 3.0;
    blip.draw_text(&format!("CREW {}", g.player.crew),
                   panel_x + 4.0, stats_y + 30.0, 2.0, BLIP_CYAN);
    blip.draw_text(&format!("FOOD {}", g.player.food as i32),
                   panel_x + 4.0 + col_w, stats_y + 30.0, 2.0, BLIP_GREEN);
    blip.draw_text(&format!("GUNS {}", g.player.cannons),
                   panel_x + 4.0 + col_w * 2.0, stats_y + 30.0, 2.0, BLIP_ORANGE);

    // ── Status message (between strips) ──────────────────────────────────────
    if g.port_msg_t > 0.0 {
        let msg_col = if g.port_msg_ok { BLIP_GREEN } else { BLIP_RED };
        blip.draw_centered(g.port_msg, stats_y + stats_h + 8.0, 2.0, msg_col);
    }

    // ── Menu panel ────────────────────────────────────────────────────────────
    let panel_y = stats_y + stats_h + 28.0;
    let panel_h = 182.0_f32;
    blip.fill_rect(panel_x - 2.0, panel_y - 2.0, panel_w + 4.0, panel_h + 4.0, BLIP_CYAN);
    blip.fill_rect(panel_x, panel_y, panel_w, panel_h, BLIP_BLACK);

    blip.draw_text(&format!("GOLD: {}", g.player.gold),
                   panel_x + 8.0, panel_y + 6.0, 2.0, BLIP_YELLOW);

    let items = [PortItem::Sail, PortItem::Map, PortItem::Repair, PortItem::Crew, PortItem::Cannons, PortItem::Food];
    for (i, &item) in items.iter().enumerate() {
        let iy         = panel_y + 30.0 + i as f32 * 24.0;
        let no_cost    = item == PortItem::Sail || item == PortItem::Map;
        let can_afford = no_cost || g.player.gold >= item.cost();
        let selected   = item == g.port_cursor;
        let col = if selected { BLIP_YELLOW } else if can_afford { BLIP_WHITE } else { BLIP_DARKGRAY };
        let prefix = if selected { ">" } else { " " };
        blip.draw_text(&format!("{} {}", prefix, item.label()), panel_x + 8.0, iy, 2.0, col);
        if !no_cost {
            let cost_col = if can_afford { BLIP_YELLOW } else { BLIP_DARKGRAY };
            blip.draw_text(&format!("{}G", item.cost()),
                           panel_x + panel_w - 44.0, iy, 2.0, cost_col);
        }
    }

    draw_hud_canaris(blip, g);
}

// ── world map (zone select) ───────────────────────────────────────────────────

pub fn update_map(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.time += dt;

    if key_pressed(BLIP_KEY_UP) || key_pressed(BLIP_KEY_W) {
        if g.map_cursor + 1 < ZONES.len() { g.map_cursor += 1; }
    }
    if key_pressed(BLIP_KEY_DOWN) || key_pressed(BLIP_KEY_S) {
        if g.map_cursor > 0 { g.map_cursor -= 1; }
    }

    if btn1_pressed() {
        let z = &ZONES[g.map_cursor];
        g.level   = z.level_eq;
        g.level_t = 90.0;
        g.spawn_enemies_n(z.ships);
        g.state = State::Sea;
        play_music(&sfx.sea_music);
    }

    if key_pressed(KeyCode::Escape) || btn2_pressed() {
        g.state = State::Port;
    }
}

pub fn draw_map(blip: &Blip, g: &Game, tex: &Textures) {
    let play_y = HUD_H as f32;

    blip.draw_texture(&tex.map_bg, 0.0, play_y, WIN_W as f32, (WIN_H - HUD_H) as f32);

    blip.draw_centered("KATTEGAT", play_y + 20.0, 2.0, BLIP_CYAN);

    // Route line connecting adjacent zone dots
    for i in 0..ZONES.len() - 1 {
        let a = &ZONES[i];
        let b = &ZONES[i + 1];
        let lx = (a.map_x + b.map_x) * 0.5;
        let ly = play_y + b.map_y;
        let lh = a.map_y - b.map_y;
        blip.fill_rect(lx, ly, 2.0, lh, Color::new(0.3, 0.5, 0.7, 0.6));
    }

    // Zone crosshair dots
    for (i, z) in ZONES.iter().enumerate() {
        let selected = i == g.map_cursor;
        let blink_on = !selected || (g.time * 3.0) as u32 % 2 == 0;
        let col = if selected { BLIP_YELLOW } else { Color::new(0.3, 0.6, 0.9, 1.0) };
        if blink_on {
            blip.fill_rect(z.map_x - 5.0, play_y + z.map_y - 5.0, 10.0, 10.0, col);
            blip.fill_rect(z.map_x - 8.0, play_y + z.map_y - 1.0, 16.0, 2.0, col);
            blip.fill_rect(z.map_x - 1.0, play_y + z.map_y - 8.0, 2.0, 16.0, col);
        }
    }

    // Info panel
    let panel_y = play_y + (WIN_H - HUD_H) as f32 * 0.60;
    blip.fill_rect(8.0, panel_y, 300.0, 110.0, Color::new(0.0, 0.05, 0.15, 0.82));
    let z = &ZONES[g.map_cursor];
    blip.draw_text(z.name, 16.0, panel_y + 10.0, 2.0, BLIP_YELLOW);
    blip.draw_text(z.desc, 16.0, panel_y + 34.0, 1.0, BLIP_WHITE);
    let stars_str = match z.stars {
        1 => "DANGER  *",
        2 => "DANGER  **",
        3 => "DANGER  ***",
        _ => "DANGER  ****",
    };
    blip.draw_text(stars_str, 16.0, panel_y + 52.0, 1.0, BLIP_RED);
    let ships_str = format!("SHIPS   {}", z.ships);
    blip.draw_text(&ships_str, 16.0, panel_y + 68.0, 1.0, BLIP_CYAN);
    blip.draw_text("W/S SELECT    [1] SAIL    [2] BACK",
                   16.0, panel_y + 90.0, 1.0, BLIP_GRAY);

    draw_hud_canaris(blip, g);
}
