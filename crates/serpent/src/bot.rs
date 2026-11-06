//! How to play Serpent: steer the snake to the food along a shortest path,
//! fetch a bonus or the shears when they can be reached in time, and never
//! turn into a pocket smaller than the snake. One tap per turn, like a player.

use super::*;
use blip::bot::grid::{self, Step};
use blip::bot::{Goal::*, Pad, Playthrough};

/// What the bot must show it can do in Serpent.
pub const PLAYTHROUGHS: &[Playthrough] = &[
    Playthrough {
        name: "the snake eats food and grows into level 2",
        goals: &[AtLeast("food", 5.0), AtLeast("level", 2.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it steers around the obstacle field to clear level 2",
        level: 2,
        goals: &[AtLeast("level", 3.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it catches a bonus before it fades",
        seconds: 240.0,
        goals: &[AtLeast("bonus_eaten", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "the shears cut a long snake short",
        level: 4,
        seconds: 300.0,
        goals: &[AtLeast("shears", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it survives the frenzy from level 5",
        level: FRENZY_LEVEL as u32,
        goals: &[AtLeast("level", FRENZY_LEVEL as f64 + 1.0)],
        ..Playthrough::DEFAULT
    },
];

pub fn drive(g: &Game) {
    let mut pad = Pad::new();
    match g.state {
        State::Title => { pad.press_start(); }
        // A turn is waiting to be taken: let it happen first.
        State::Play if g.turns_len == 0 => {
            if let Some(turn) = choose_way(g) {
                if turn != step(g.cur_dir) { pad.hold(turn.key()); }
            }
        }
        _ => {}
    }
    pad.send();
}

/// The bonus or shears if they can be reached before they fade, else the
/// food; with no safe way to any, the roomiest way on.
fn choose_way(g: &Game) -> Option<Step> {
    let head = cell(g.snake_at(0));
    let back = step(g.cur_dir).back();
    let open = |c: i32, r: i32| is_open(g, c, r);
    // Never into a pocket the snake will not fit in.
    let roomy = |s: Step| s != back && grid::room(s.from(head), open, g.snake_len + 1) > g.snake_len;
    let secs_per_step = g.move_interval() / 1000.0;
    let in_time = |to: Cell, ttl: f32| {
        grid::path(head, cell(to), open, roomy).filter(|(_, n)| *n as f32 * secs_per_step < ttl).map(|(s, _)| s)
    };
    let bonus = g.bonus_active().then(|| in_time(g.bonus, g.bonus_ttl)).flatten();
    let shears = g.shears_active().then(|| in_time(g.shears, g.shears_ttl)).flatten();
    let food = || grid::path(head, cell(g.food), open, roomy).map(|(s, _)| s);
    let roomiest = || Step::ALL.into_iter().filter(|s| *s != back)
        .max_by_key(|s| grid::room(s.from(head), open, COLS as usize * ROWS as usize));
    bonus.or(shears).or_else(food).or_else(roomiest)
}

/// Inside the field and free of body and obstacles. The tail cell frees up
/// as the head moves, so it counts as open.
fn is_open(g: &Game, c: i32, r: i32) -> bool {
    let at = Cell { c, r };
    (0..COLS).contains(&c) && (0..ROWS).contains(&r)
        && !(0..g.snake_len.saturating_sub(1)).any(|i| g.snake_at(i) == at)
        && !g.obstacles[..g.obstacle_count].contains(&at)
}

fn cell(c: Cell) -> grid::Cell { (c.c, c.r) }

fn step(d: Dir) -> Step {
    match d { Dir::Up => Step::Up, Dir::Right => Step::Right, Dir::Down => Step::Down, Dir::Left => Step::Left }
}
