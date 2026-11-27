//! The words over the cellar: the spell circle's hint, the Warlock's taunt, the lives, and game over.

use super::*;

/// The spell circle says what it wants: watch, then stand on each stone in turn and reveal.
pub(super) fn draw_bonus_hint(blip: &Blip, g: &Game) {
    let Some(b) = &g.bonus else { return };
    let y = (HUD_H + ROWS as i32 * CELL - 46) as f32;
    let cx = WIN_W as f32 / 2.0;
    if b.showing {
        blip.draw_centered_outlined("WATCH THE STONES", cx, y, 2.0, RUNE_CLEAN, BLIP_BLACK);
        return;
    }
    blip.draw_centered_outlined("STEP ON THE STONES IN ORDER", cx, y, 2.0, RUNE_CLEAN, BLIP_BLACK);
    let press = web::controls().pick("PRESS Z ON EACH", "PRESS REVEAL ON EACH", "PRESS REVEAL ON EACH");
    let count = format!("{press}   {}/{}   {}", b.expect, b.seq.len(), b.left.ceil().max(0.0) as i32);
    blip.draw_centered_outlined(&count, cx, y + 18.0, 2.0, BLIP_WHITE, BLIP_BLACK);
}

pub(super) fn draw_taunt(blip: &Blip, g: &Game) {
    draw_bonus_hint(blip, g);
    let banner_up = g.banner.as_ref().is_some_and(|(_, t)| *t > 0.0);
    if let Some((text, t)) = &g.taunt {
        if *t > 0.0 && !banner_up {
            blip.draw_message(text, WIN_W as f32 / 2.0, HUD_H as f32 + 18.0, 2.0, WARLOCK_C, 1.0);
        }
    }
    if let Some((text, t)) = &g.banner {
        if *t > 0.0 {
            let k = (*t * 3.0).min(1.0);
            let sz = if text.chars().count() * 18 <= WIN_W as usize - 16 { 3.0 } else { 2.0 };
            blip.draw_message(text, WIN_W as f32 / 2.0, (HUD_H + ROWS as i32 * CELL / 2) as f32, sz, RUNE_CLEAN, k);
        }
    }
}

pub(super) fn hud_lives(g: &Game) -> i32 {
    if g.two_player {
        g.lives[0] + g.lives[1]
    } else {
        g.lives[0]
    }
}

pub(super) fn draw_over(blip: &Blip, g: &Game) {
    let hi = web::high_score();
    blip.draw_game_over(g.sess.score, hi, WARLOCK_C, BLIP_YELLOW, BLIP_WHITE, false);
    blip.draw_game_over_note(&format!("WINGS CLEARED {}", g.cleared_no_death), BLIP_GRAY);
    blip.draw_continue(&g.cont, BLIP_YELLOW);
}

