//! Native playtest autopilot (BLIP_BOT=1). It walks the corridors, cleanses
//! runes, reveals wraiths, and fires down corridors to clear the introductory
//! rooms. Its paths never step on top of a monster, so deaths record real
//! monster contact, not pathing mistakes.

use super::*;
use blip::macroquad::input::KeyCode;

fn dir_key(d: Dir) -> KeyCode {
    match d {
        Dir::Up => BLIP_KEY_UP,
        Dir::Down => BLIP_KEY_DOWN,
        Dir::Left => BLIP_KEY_LEFT,
        Dir::Right => BLIP_KEY_RIGHT,
    }
}

fn blocked_cells(g: &Game) -> Vec<Cell> {
    g.enemies.iter().filter(|e| !e.gone).map(|e| e.tile).collect()
}

/// Is there a hittable enemy in a straight line from the wizard in `dir`?
fn los(g: &Game, dir: Dir) -> bool {
    let (dc, dr) = dir.delta();
    let mut c = g.wizards[0].tile;
    loop {
        c = Cell { c: c.c + dc, r: c.r + dr };
        if g.walls.get(c.c, c.r) {
            return false;
        }
        if let Some(e) = g.enemies.iter().find(|e| !e.gone && e.tile == c) {
            if e.kind == EnemyKind::Wraith && !e.visible {
                return false; // an invisible wraith can't be hit yet
            }
            return true;
        }
        if let Some(wl) = &g.warlock {
            if wl.wards > 0 && wl.state == WState::Pausing && wl.circles[wl.circle] == c {
                return true;
            }
        }
        if g.whelp.as_ref().is_some_and(|w| w.tile == c) {
            return true;
        }
    }
}

pub fn drive(g: &Game) {
    match g.state {
        State::Title | State::Over => {
            // WARLOCK_BOT_TITLE holds the title for that many seconds first (the attract loop shows it)
            let wait: f32 = std::env::var("WARLOCK_BOT_TITLE").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            if g.state == State::Title && blip::bot::clock() < wait {
                blip::bot::hold(&[]);
                return;
            }
            blip::bot::hold(&[BLIP_KEY_SPACE]);
            return;
        }
        State::Dead => {
            blip::bot::hold(&[]);
            return;
        }
        State::Bonus => {
            bonus_drive(g);
            return;
        }
        State::Play => {}
    }

    // cleanse an adjacent rune
    // (a held key only casts once: let go while the spell recharges, so
    // the next press is a fresh one)
    if g.runes.iter().any(|r| !r.cleansed && distance_lt(g.wizards[0].tile, r.tile, 1.5)) {
        blip::bot::hold(if g.wizards[0].reveal_cd <= 0.0 { &[BLIP_KEY_G] } else { &[] });
        return;
    }
    // cast reveal to flush out a hidden wraith, so it can be seen and shot
    let hidden_wraith = g
        .enemies
        .iter()
        .any(|e| !e.gone && e.kind == EnemyKind::Wraith && !e.visible);
    if hidden_wraith && g.wizards[0].reveal_cd <= 0.0 {
        blip::bot::hold(&[BLIP_KEY_G]);
        return;
    }
    // fire down a facing that already lines up
    if los(g, g.wizards[0].facing) {
        blip::bot::hold(&[BLIP_KEY_F]);
        return;
    }
    // turn to face a lined-up enemy
    for d in DIRS {
        if d != g.wizards[0].facing && los(g, d) {
            blip::bot::hold(&[dir_key(d)]);
            return;
        }
    }
    // otherwise path to the nearest goal
    if let Some((_, d)) = bot_path(g) {
        blip::bot::hold(&[dir_key(d)]);
    } else {
        blip::bot::hold(&[]);
    }
}

fn bot_path(g: &Game) -> Option<(Cell, Dir)> {
    let blocked = blocked_cells(g);
    // a potion close by is worth the detour
    let near_potion = g.potions.iter().map(|p| p.tile).filter(|&t| manhattan(g.wizards[0].tile, t) <= 5).min_by_key(|&t| manhattan(g.wizards[0].tile, t));
    let goal = near_potion.or_else(|| g
        .runes
        .iter()
        .filter(|r| !r.cleansed)
        .min_by_key(|r| manhattan(g.wizards[0].tile, r.tile))
        .map(|r| r.tile)
        .or_else(|| {
            g.enemies
                .iter()
                .filter(|e| !e.gone)
                .min_by_key(|e| manhattan(g.wizards[0].tile, e.tile))
                .map(|e| e.tile)
        })
        .or_else(|| g.whelp.as_ref().map(|w| w.tile)))
        .unwrap_or(g.exit);
    // the goal itself may be a monster's tile; the bot fires before it would step in
    let blocked: Vec<Cell> = blocked.into_iter().filter(|&c| c != goal).collect();
    bfs_next(&g.walls, &blocked, g.wizards[0].tile, goal)
}

fn manhattan(a: Cell, b: Cell) -> i32 {
    (a.c - b.c).abs() + (a.r - b.r).abs()
}

fn bonus_drive(g: &Game) {
    let Some(b) = &g.bonus else {
        blip::bot::hold(&[]);
        return;
    };
    if b.showing {
        blip::bot::hold(&[]);
        return;
    }
    let stone = b.stones[b.seq[b.expect]];
    if distance_lt(g.wizards[0].tile, stone, 1.5) {
        blip::bot::hold(&[BLIP_KEY_G]);
    } else if let Some((_, d)) = bfs_next(&g.walls, &[], g.wizards[0].tile, stone) {
        blip::bot::hold(&[dir_key(d)]);
    } else {
        blip::bot::hold(&[]);
    }
}