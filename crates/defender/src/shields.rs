//! The shields: blocks chipped by every shot and bomb, and ploughed through by invaders low enough to reach them.

use super::*;

/// The top left of shield block `r`, `c`.
pub(super) fn shield_block_xy(shield: &Shield, r: usize, c: usize) -> (f32, f32) {
    (shield.x + (c as i32 * SHIELD_BLOCK) as f32, shield.y + (r as i32 * SHIELD_BLOCK) as f32)
}

/// Every block of every shield, as (shield, row, column), shield by shield and row by row.
pub(super) fn blocks() -> impl Iterator<Item = (usize, usize, usize)> {
    (0..SHIELDS).flat_map(|s| (0..SHIELD_ROWS).flat_map(move |r| (0..SHIELD_COLS).map(move |c| (s, r, c))))
}

/// Any bullet against the shields: it takes out one block.
pub(super) fn hit_shields(g: &mut Game, bullet_starts: &[(f32, f32); N_BULLETS], bullet_was_active: &[bool; N_BULLETS]) {
    for bi in 0..N_BULLETS {
        if !g.bullets[bi].active { continue; }
        for (s, r, c) in blocks() {
            if !g.shields[s].alive[r][c] { continue; }
            let (bx, by) = shield_block_xy(&g.shields[s], r, c);
            let end = (g.bullets[bi].x, g.bullets[bi].y);
            let hit = if bullet_was_active[bi] {
                bullet_hits_rect(bullet_starts[bi], end, bx, by, SHIELD_BLOCK as f32, SHIELD_BLOCK as f32)
            } else {
                rects_overlap(end.0, end.1, 8.0, 16.0, bx, by, SHIELD_BLOCK as f32, SHIELD_BLOCK as f32)
            };
            if hit {
                g.shields[s].alive[r][c] = false;
                g.bullets[bi].active = false;
                g.fx.burst(bx + SHIELD_BLOCK as f32 / 2.0, by + SHIELD_BLOCK as f32 / 2.0, 5, 50.0, BLIP_GREEN);
                break;
            }
        }
    }
}

/// The formation plows through any shield blocks in its path as it marches down, as in the classic game.
pub(super) fn plow_shields(g: &mut Game) {
    for a in g.aliens.iter() {
        if !a.alive { continue; }
        for (s, r, c) in blocks() {
            if !g.shields[s].alive[r][c] { continue; }
            let (bx, by) = shield_block_xy(&g.shields[s], r, c);
            if rects_overlap(a.x, a.y, ALIEN_W as f32, ALIEN_H as f32, bx, by, SHIELD_BLOCK as f32, SHIELD_BLOCK as f32) {
                g.shields[s].alive[r][c] = false;
            }
        }
    }
}
