//! Native-only staged scenes for screenshots: `WARLOCK_SCENE=<name>` with
//! `BLIP_SCREENSHOT_OUT` and `BLIP_SCREENSHOT_FRAME` sets up a chamber and
//! fires one spell a few frames before the capture, so each effect can be
//! reviewed on its own. `WARLOCK_LEVEL` picks the chamber (default 1).
//! Scenes: title, intro, play, bolt, reveal, cleanse, kill, ward, death, open,
//! gargoyle, warlock, bonus, over, potions, lance.

use super::*;

fn scene() -> Option<String> {
    std::env::var("WARLOCK_SCENE").ok()
}

/// After `start_game`, stage the chamber for the scene.
pub fn setup(g: &mut Game) {
    let Some(s) = scene() else { return };
    if let Some(level) = std::env::var("WARLOCK_LEVEL").ok().and_then(|v| v.parse().ok()) {
        g.sess.level = level;
        g.load_room();
    }
    if s != "intro" {
        g.intro = 0.0;
    }
    // hold the monsters still so the capture shows the staged moment
    for e in &mut g.enemies {
        e.step_t = 1000.0;
    }
    match s.as_str() {
        "title" => g.state = State::Title,
        "over" => {
            g.state = State::Over;
            g.dead_timer.start(0.1);
        }
        "bonus" => {
            g.state = State::Bonus;
            g.start_bonus();
        }
        _ => {}
    }
}

/// Fire the scene's spell on the frame it is due.
pub fn tick(g: &mut Game, frame: u32) {
    let Some(s) = scene() else { return };
    let at: u32 = std::env::var("BLIP_SCREENSHOT_FRAME").ok().and_then(|v| v.parse().ok()).unwrap_or(25);
    let lead: u32 = std::env::var("WARLOCK_LEAD").ok().and_then(|v| v.parse().ok()).unwrap_or(8);
    if frame + lead != at {
        return;
    }
    let w = g.wizards[0].tile;
    let (x, y) = g.wizard_px(0);
    match s.as_str() {
        "bolt" => {
            g.wizards[0].facing = Dir::Right;
            g.cast_bolt(0);
        }
        "reveal" => {
            g.wizards[0].reveal_cd = 0.0;
            g.reveal(0);
        }
        "cleanse" => {
            if let Some(r) = g.runes.first() {
                let t = r.tile;
                g.place_wizard(0, Cell { c: t.c - 1, r: t.r });
                g.wizards[0].facing = Dir::Right;
                g.wizards[0].reveal_cd = 0.0;
                g.reveal(0);
            }
        }
        "kill" => {
            if !g.enemies.is_empty() {
                g.enemies[0].tile = Cell { c: w.c + 3, r: w.r };
                g.enemies[0].from = g.enemies[0].tile;
                g.kill_enemy(0);
            }
        }
        "ward" => g.hit_warlock(),
        "death" => g.hit_wizard(0),
        "open" => {
            for r in &mut g.runes {
                r.cleansed = true;
            }
            for e in &mut g.enemies {
                e.gone = true;
            }
            g.warlock = None;
            g.check_exit();
        }
        "lance" => {
            g.wizards[0].facing = Dir::Right;
            g.wizards[0].reveal_cd = 0.0;
            g.reveal(0);
            g.cast_bolt(0);
        }
        "potions" => {
            g.drop_potion(Cell { c: w.c + 2, r: w.r }, PotionKind::Haste);
            g.drop_potion(Cell { c: w.c + 3, r: w.r }, PotionKind::Gold);
            g.drop_potion(Cell { c: w.c + 4, r: w.r }, PotionKind::Life);
            g.wizards[0].haste = HASTE_SECS;
        }
        "gargoyle" => {
            for e in &mut g.enemies {
                if e.kind == EnemyKind::Gargoyle {
                    e.windup = GARGOYLE_TELL * 0.4;
                }
            }
        }
        _ => {
            let _ = (x, y);
        }
    }
}
