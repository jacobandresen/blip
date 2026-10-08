//! Native-only autopilot (BLIP_BOT=1): BFS to the egg through the pit, one
//! turn at a time like a player, and the throttle down only when the way there
//! is long enough to steer out of.

use super::*;
use std::collections::VecDeque;

/// Steps to the egg under which the bot keeps its hand off the throttle.
const STRIKE_AFTER: i32 = 8;

fn key(d: Dir) -> blip::macroquad::input::KeyCode {
    match d {
        Dir::Up => BLIP_KEY_UP,
        Dir::Down => BLIP_KEY_DOWN,
        Dir::Left => BLIP_KEY_LEFT,
        Dir::Right => BLIP_KEY_RIGHT,
    }
}

/// The pit as the bot sees it. The last segment is left open: it moves on
/// before the head arrives.
fn blocked(g: &Game) -> Vec<bool> {
    let mut b = vec![false; CELLS];
    for i in 0..g.len.saturating_sub(1) { b[at(g.seg(i))] = true; }
    for i in 0..g.skin_len { b[at(g.skin[i])] = true; }
    b
}

fn room(b: &[bool], c: Cell) -> usize {
    if !inside(c) || b[at(c)] { return 0; }
    let mut seen = b.to_vec();
    seen[at(c)] = true;
    let mut n = 0;
    let mut stack = vec![c];
    while let Some(c) = stack.pop() {
        n += 1;
        for d in DIRS {
            let (dc, dr) = d.delta();
            let c = Cell { c: c.c + dc, r: c.r + dr };
            if inside(c) && !seen[at(c)] { seen[at(c)] = true; stack.push(c); }
        }
    }
    n
}

/// The first step of the shortest way to the egg, and how long it is.
fn route(b: &[bool], from: Cell, to: Cell, back: Dir) -> Option<(Dir, i32)> {
    let mut seen = b.to_vec();
    let mut q = VecDeque::new();
    for d in DIRS {
        if d == back { continue; }
        let (dc, dr) = d.delta();
        let c = Cell { c: from.c + dc, r: from.r + dr };
        if inside(c) && !seen[at(c)] {
            seen[at(c)] = true;
            q.push_back((c, d, 1));
        }
    }
    while let Some((c, first, steps)) = q.pop_front() {
        if c == to { return Some((first, steps)); }
        for d in DIRS {
            let (dc, dr) = d.delta();
            let next = Cell { c: c.c + dc, r: c.r + dr };
            if inside(next) && !seen[at(next)] {
                seen[at(next)] = true;
                q.push_back((next, first, steps + 1));
            }
        }
    }
    None
}

pub fn drive(g: &Game) {
    match g.state {
        State::Title => { blip::bot::hold(&[BLIP_KEY_SPACE]); return; }
        State::Play => {}
        _ => { blip::bot::hold(&[]); return; }
    }
    let b = blocked(g);
    let head = g.seg(0);
    let back = g.dir.opposite();
    let roomy = |d: Dir| {
        let (dc, dr) = d.delta();
        room(&b, Cell { c: head.c + dc, r: head.r + dr }) > g.len * 2
    };
    // A safe way to the egg, or failing that the roomiest way on.
    let (want, steps) = match route(&b, head, g.egg, back) {
        Some((d, n)) if roomy(d) => (d, n),
        _ => (DIRS.iter().copied()
                .filter(|d| *d != back)
                .max_by_key(|d| {
                    let (dc, dr) = d.delta();
                    room(&b, Cell { c: head.c + dc, r: head.r + dr })
                })
                .unwrap_or(g.dir), 0),
    };
    let mut held = Vec::new();
    if want != g.dir { held.push(key(want)); }
    if steps >= STRIKE_AFTER { held.push(BLIP_KEY_SPACE); }
    blip::bot::hold(&held);
}
