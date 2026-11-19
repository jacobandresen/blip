//! Finding the way on a tile map, for computer players and playtest bots alike: the first step of a shortest path, what can
//! be reached and how much room there is. A cell is `(column, row)`; `open(c, r)` says whether something may stand
//! there, and must be false off the map.

use crate::input::{BLIP_KEY_DOWN, BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_UP};
use macroquad::input::KeyCode;
use std::collections::{HashMap, HashSet, VecDeque};

pub type Cell = (i32, i32);

/// One step on the grid.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Step { Up, Right, Down, Left }

impl Step {
    /// The order a search tries them in, so ties break the same way every time.
    pub const ALL: [Step; 4] = [Step::Up, Step::Down, Step::Left, Step::Right];

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
/// steps. `first` may rule out some first steps (no turning straight back).
/// `to` counts as reachable even when `open` says no (a goal someone stands on). None when already there.
pub fn path(from: Cell, to: Cell, open: impl Fn(i32, i32) -> bool, first: impl Fn(Step) -> bool) -> Option<(Step, usize)> {
    if from == to { return None; }
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
    reachable(at, open, enough).len()
}

/// The open cells that can be reached from `at` (none if `at` is closed), stopping once there are `enough`.
pub fn reachable(at: Cell, open: impl Fn(i32, i32) -> bool, enough: usize) -> HashSet<Cell> {
    if !open(at.0, at.1) { return HashSet::new(); }
    let mut seen = HashSet::from([at]);
    let mut stack = vec![at];
    while let Some(cell) = stack.pop() {
        if seen.len() >= enough { break; }
        for next in Step::ALL.map(|s| s.from(cell)) {
            if open(next.0, next.1) && seen.insert(next) { stack.push(next); }
        }
    }
    seen
}

/// An influence map: each source spreads its strength over the open cells, falling by `decay` (0 to 1) every
/// step it walks, out to `reach` steps; strengths add where they meet. Hazards as sources give a danger field to
/// keep low; goals as sources, one to climb.
pub fn influence(sources: &[(Cell, f32)], open: impl Fn(i32, i32) -> bool, decay: f32, reach: usize) -> HashMap<Cell, f32> {
    let mut field = HashMap::new();
    for &(at, strength) in sources {
        let mut seen = HashSet::from([at]);
        let mut queue = VecDeque::from([(at, 0usize)]);
        while let Some((cell, n)) = queue.pop_front() {
            *field.entry(cell).or_insert(0.0) += strength * (1.0 - decay).powi(n as i32);
            if n >= reach { continue; }
            for next in Step::ALL.map(|s| s.from(cell)) {
                if open(next.0, next.1) && seen.insert(next) { queue.push_back((next, n + 1)); }
            }
        }
    }
    field
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn influence_fades_with_walking_distance_and_adds_up() {
        let open = |c: i32, r: i32| (0..5).contains(&c) && r == 0;
        let f = influence(&[((0, 0), 1.0), ((4, 0), 1.0)], open, 0.5, 4);
        assert_eq!(f[&(0, 0)], 1.0 + 0.0625);
        assert_eq!(f[&(2, 0)], 0.5);
    }

    #[test]
    fn a_path_goes_around_a_wall_and_counts_its_steps() {
        // A 5x3 room with a wall down the middle column, open at the bottom.
        let open = |c: i32, r: i32| (0..5).contains(&c) && (0..3).contains(&r) && !(c == 2 && r < 2);
        // Down and right tie; a search tries down first.
        assert_eq!(path((0, 0), (4, 0), open, |_| true), Some((Step::Down, 8)));
        assert_eq!(path((0, 0), (4, 0), open, |s| s != Step::Down), Some((Step::Right, 8)));
        assert_eq!(path((1, 1), (1, 1), open, |_| true), None);
        // The goal may itself be closed (someone stands on it).
        assert_eq!(path((0, 0), (2, 0), open, |_| true), Some((Step::Right, 2)));
        assert_eq!(room((0, 0), open, 100), 13);
        assert_eq!(room((2, 0), open, 100), 0);
    }
}
