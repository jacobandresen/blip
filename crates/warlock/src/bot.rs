//! How to play Warlock: cleanse a rune within reach, cast reveal to show a
//! hidden wraith, bolt anything lined up down a corridor (turning to face
//! it first), else walk to a nearby potion, the nearest rune, the nearest
//! monster, the Warlock's circle or the open exit, through a sigil when that
//! is the short way or a monster is at its heels (and once a run anyway). Paths never step onto a monster, so deaths
//! are real monster contact, not pathing mistakes. In the bonus room it
//! walks to each stone in the order shown and touches it.

use super::*;
use blip::bot::{grid, Goal::*, Pad, Playthrough};
use blip::macroquad::input::KeyCode;

/// What the bot must show it can do in Warlock: every chamber of the
/// first wing, by name, and the spells that open them.
pub const PLAYTHROUGHS: &[Playthrough] = &[
    Playthrough {
        name: "the wizard cleanses the runes of the Undercroft and walks out",
        goals: &[AtLeast("cleanse", 1.0), AtLeast("level", 2.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it reveals and banishes the wraiths of the Bone Crypt",
        level: 2,
        goals: &[AtLeast("reveal", 1.0), AtLeast("kill", 1.0), AtLeast("level", 3.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it gets past the hounds of the Kennels",
        level: 3,
        goals: &[AtLeast("level", 4.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it clears the Hall of Gates and steps through a sigil",
        level: 4,
        seconds: 240.0,
        goals: &[AtLeast("gate", 1.0), AtLeast("level", 5.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it gets past the Gargoyle Gallery",
        level: 5,
        goals: &[AtLeast("level", 6.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it breaks the Warlock's wards in his Sanctum and plays the bonus stones",
        level: 6,
        seconds: 240.0,
        goals: &[AtLeast("ward", 1.0), AtLeast("level", 7.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "a bolt right after a reveal combines into a lance",
        level: 2,
        seconds: 240.0,
        goals: &[AtLeast("lance", 1.0)],
        ..Playthrough::DEFAULT
    },
];

/// The bolt and spell keys (player one).
const BOLT: KeyCode = BLIP_KEY_F;
const SPELL: KeyCode = BLIP_KEY_G;

pub fn drive(g: &Game) {
    let mut pad = Pad::new();
    match g.state {
        State::Title | State::Over => { pad.press_start(); }
        State::Play => play(g, &mut pad),
        State::Bonus => bonus(g, &mut pad),
        _ => {}
    }
    pad.send();
}

/// One frame in a chamber: the first thing worth doing.
fn play(g: &Game, pad: &mut Pad) {
    let me = &g.wizards[0];
    let spell_ready = me.reveal_cd <= 0.0;
    let rune_in_reach = g.runes.iter().any(|r| !r.cleansed && distance_lt(me.tile, r.tile, 1.5));
    let hidden_wraith = g.enemies.iter().any(|e| !e.gone && e.kind == EnemyKind::Wraith && !e.visible);
    if rune_in_reach {
        // The spell key cleanses it; wait for the spell to recharge.
        if spell_ready { pad.tap(SPELL); }
    } else if hidden_wraith && spell_ready {
        pad.tap(SPELL); // reveal
    } else if los(g, me.facing) {
        pad.hold(BOLT);
    } else if let Some(d) = DIRS.into_iter().find(|&d| los(g, d)) {
        pad.hold(dir_key(d)); // turn to face it
    } else if let Some((_, d)) = bot_path(g) {
        pad.hold(dir_key(d));
    }
}

/// The bonus room: walk to the next stone in the order shown, and touch it.
fn bonus(g: &Game, pad: &mut Pad) {
    let Some(b) = &g.bonus else { return };
    if b.showing { return; }
    let stone = b.stones[b.seq[b.expect]];
    if distance_lt(g.wizards[0].tile, stone, 1.5) {
        pad.tap(SPELL);
    } else if let Some((_, d)) = bfs_next(&g.walls, &[], g.wizards[0].tile, stone) {
        pad.hold(dir_key(d));
    }
}

fn dir_key(d: Dir) -> KeyCode {
    match d { Dir::Up => BLIP_KEY_UP, Dir::Down => BLIP_KEY_DOWN, Dir::Left => BLIP_KEY_LEFT, Dir::Right => BLIP_KEY_RIGHT }
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

/// The first step toward where the wizard should go next.
fn bot_path(g: &Game) -> Option<(Cell, Dir)> {
    let me = g.wizards[0].tile;
    let nearest = |tiles: &mut dyn Iterator<Item = Cell>| tiles.min_by_key(|&t| manhattan(me, t));
    // A potion close by is worth the detour; then runes, monsters, the
    // whelp, the Warlock at his circle while his wards hold, and the exit.
    let goal = nearest(&mut g.potions.iter().map(|p| p.tile).filter(|&t| manhattan(me, t) <= 5))
        .or_else(|| nearest(&mut g.runes.iter().filter(|r| !r.cleansed).map(|r| r.tile)))
        .or_else(|| nearest(&mut g.enemies.iter().filter(|e| !e.gone).map(|e| e.tile)))
        .or_else(|| g.whelp.as_ref().map(|w| w.tile))
        .or_else(|| g.warlock.as_ref().filter(|w| w.wards > 0).map(|w| w.circles[w.circle]))
        .or_else(|| try_a_sigil(g, me))
        .unwrap_or(g.exit);
    let goal = escape_gate(g, me).or_else(|| gate_toward(g, me, goal)).unwrap_or(goal);
    // The goal itself may be a monster's tile; the bot fires before it would step in.
    let blocked: Vec<Cell> = blocked_cells(g).into_iter().filter(|&c| c != goal).collect();
    bfs_next(&g.walls, &blocked, me, goal)
}

/// A ready sigil to step into, when walking to it and on from its partner
/// is shorter than walking to `goal`.
fn gate_toward(g: &Game, me: Cell, goal: Cell) -> Option<Cell> {
    let walk = steps(g, me, goal).unwrap_or(usize::MAX);
    g.gates.iter().filter(|gate| gate.cooldown <= 0.0)
        .flat_map(|gate| [(gate.a, gate.b), (gate.b, gate.a)])
        .filter_map(|(enter, out)| Some((enter, steps(g, me, enter)? + steps(g, out, goal)?)))
        .filter(|&(_, via)| via < walk)
        .min_by_key(|&(_, via)| via)
        .map(|(enter, _)| enter)
}

/// A playtest walks through a sigil once a run, so the gates get used: with
/// the room clear and none taken yet, the nearest ready one before the exit.
fn try_a_sigil(g: &Game, me: Cell) -> Option<Cell> {
    if blip::bot::stat("gate") > 0.0 { return None; }
    g.gates.iter().filter(|gate| gate.cooldown <= 0.0).flat_map(|gate| [gate.a, gate.b])
        .min_by_key(|&enter| steps(g, me, enter).unwrap_or(usize::MAX))
}

/// With a monster at its heels: a ready sigil close by whose partner has no
/// monster near, to slip away through.
fn escape_gate(g: &Game, me: Cell) -> Option<Cell> {
    let near = |at: Cell, d: i32| g.enemies.iter().any(|e| !e.gone && manhattan(e.tile, at) <= d);
    if !near(me, 2) { return None; }
    g.gates.iter().filter(|gate| gate.cooldown <= 0.0)
        .flat_map(|gate| [(gate.a, gate.b), (gate.b, gate.a)])
        .filter(|&(enter, out)| steps(g, me, enter).is_some_and(|n| n <= 3) && !near(out, 3))
        .map(|(enter, _)| enter)
        .min_by_key(|&enter| manhattan(me, enter))
}

/// Steps from `a` to `b` through the corridors, monsters ignored.
fn steps(g: &Game, a: Cell, b: Cell) -> Option<usize> {
    if a == b { return Some(0); }
    let open = |c: i32, r: i32| Walls::in_bounds(c, r) && !g.walls.get(c, r);
    grid::path((a.c, a.r), (b.c, b.r), open, |_| true).map(|(_, n)| n)
}

fn manhattan(a: Cell, b: Cell) -> i32 {
    (a.c - b.c).abs() + (a.r - b.r).abs()
}

