//! Finding the way on a tile map: the first step of a shortest path, and how
//! much room there is. A cell is `(column, row)`; `open(c, r)` says whether a
//! bot may stand there, and must be false off the map.

use crate::input::{BLIP_KEY_DOWN, BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_UP};
use macroquad::input::KeyCode;
use std::collections::{HashSet, VecDeque};

pub type Cell = (i32, i32);

/// One step on the grid.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Step { Up, Right, Down, Left }

impl Step {
    pub const ALL: [Step; 4] = [Step::Up, Step::Right, Step::Down, Step::Left];

    /// Column and row change.
    pub fn delta(self) -> Cell {
        match self { Step::Up => (0, -1), Step::Right => (1, 0), Step::Down => (0, 1), Step::Left => (-1, 0) }
    }

    /// The arrow key that takes it.
    pub fn key(self) -> KeyCode {
        match self { Step::Up => BLIP_KEY_UP, Step::Right => BLIP_KEY_RIGHT, Step::Down => BLIP_KEY_DOWN, Step::Left => BLIP_KEY_LEFT }
    }

    pub fn back(self) -> Step {
        match self { Step::Up => Step::Down, Step::Right => Step::Left, Step::Down => Step::Up, Step::Left => Step::Right }
    }

    /// The cell one step from `at`.
    pub fn from(self, at: Cell) -> Cell {
        let (dc, dr) = self.delta();
        (at.0 + dc, at.1 + dr)
    }
}

/// The first step of a shortest path from `from` to `to`, and its length in
/// steps. `first` may rule out some first steps (a snake cannot reverse).
/// `to` counts as reachable even when `open` says no (food under a monster).
pub fn path(from: Cell, to: Cell, open: impl Fn(i32, i32) -> bool, first: impl Fn(Step) -> bool) -> Option<(Step, usize)> {
    let mut seen = HashSet::from([from]);
    let mut queue = VecDeque::new();
    for step in Step::ALL.into_iter().filter(|s| first(*s)) {
        let cell = step.from(from);
        if (cell == to || open(cell.0, cell.1)) && seen.insert(cell) {
            queue.push_back((cell, step, 1));
        }
    }
    while let Some((cell, step, n)) = queue.pop_front() {
        if cell == to { return Some((step, n)); }
        for next in Step::ALL.map(|s| s.from(cell)) {
            if (next == to || open(next.0, next.1)) && seen.insert(next) {
                queue.push_back((next, step, n + 1));
            }
        }
    }
    None
}

/// How many open cells can be reached from `at` (0 if `at` is closed),
/// counting no further than `enough`.
pub fn room(at: Cell, open: impl Fn(i32, i32) -> bool, enough: usize) -> usize {
    if !open(at.0, at.1) { return 0; }
    let mut seen = HashSet::from([at]);
    let mut stack = vec![at];
    while let Some(cell) = stack.pop() {
        if seen.len() >= enough { break; }
        for next in Step::ALL.map(|s| s.from(cell)) {
            if open(next.0, next.1) && seen.insert(next) { stack.push(next); }
        }
    }
    seen.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_goes_around_a_wall_and_counts_its_steps() {
        // A 5x3 room with a wall down the middle column, open at the bottom.
        let open = |c: i32, r: i32| (0..5).contains(&c) && (0..3).contains(&r) && !(c == 2 && r < 2);
        assert_eq!(path((0, 0), (4, 0), open, |_| true), Some((Step::Right, 8)));
        assert_eq!(path((0, 0), (4, 0), open, |s| s != Step::Right), Some((Step::Down, 8)));
        // The goal may itself be closed (food under a monster).
        assert_eq!(path((0, 0), (2, 0), open, |_| true), Some((Step::Right, 2)));
        assert_eq!(room((0, 0), open, 100), 13);
        assert_eq!(room((2, 0), open, 100), 0);
    }
}
