//! Authored rooms and tuning for WARLOCK: the mazes, spawn groups, rune
//! stones, exits, gates and the Warlock's chamber. Constants live here so the
//! tests can share them; layout (cells to pixels) lives in main.rs.

use crate::Walls;

// ---- board ----------------------------------------------------------------
pub const COLS: usize = 15;
pub const ROWS: usize = 13;

// ---- movement and timing (seconds) ----------------------------------------
pub const MOVE_SECS: f32 = 0.09; // a wizard takes 90ms to step one tile
pub const BOLT_INTERVAL: f32 = 0.05; // a bolt crosses a tile in 50ms
pub const BOLT_MAX: usize = 3; // live bolts, so firing can't fill a corridor
pub const BOLT_RATE: f32 = 0.16; // hold A: a bolt every 160ms
pub const REVEAL_CD: f32 = 2.5; // reveal cooldown, shown as a ring
pub const REVEAL_SECS: f32 = 3.5; // how long a revealed wraith stays visible
pub const RUNE_LIGHT: f32 = 2.4; // a cleansed stone forces wraiths visible within this
pub const SIGHTING_FADE: f32 = 2.5; // a reveal's "last seen" outline lingers this long
pub const GATE_CD: f32 = 3.0; // how long a gate pair stays closed after use
pub const RESPAWN_PROTECT: f32 = 2.0; // protected respawn, seconds

// ---- enemies (step interval / behaviour tuned per kind in main.rs) --------
pub const IMP_STEP: f32 = 0.50;
pub const WRAITH_STEP: f32 = 0.42;
pub const WRAITH_FLASH_ON: f32 = 0.18; // brief visible flash, every cycle
pub const WRAITH_FLASH_OFF: f32 = 1.0; // invisible stretch between flashes
pub const WRAITH_LUNGE_RANGE: f32 = 2.3; // becomes fully visible this close, then pounces
pub const WRAITH_LUNGE_STEP: f32 = 0.20;
pub const HOUND_WANDER_STEP: f32 = 0.55;
pub const HOUND_HUNT_STEP: f32 = 0.26;
pub const GARGOYLE_TELL: f32 = 0.9; // wings rise this long before the shot
pub const GARGOYLE_CD: f32 = 3.2; // between shots
pub const GARGOYLE_SHOT_STEP: f32 = 0.11; // fireball speed (per tile)
pub const GARGOYLE_RANGE: i32 = 8; // how far a fireball flies

// ---- scoring --------------------------------------------------------------
pub const SCORE_IMP: i32 = 100;
pub const SCORE_WRAITH: i32 = 200;
pub const SCORE_HOUND: i32 = 250;
pub const SCORE_GARGOYLE: i32 = 300;
pub const SCORE_RUNE: i32 = 150;
pub const SCORE_CLEAR_NODEATH: i32 = 1000;
pub const SCORE_WARD: i32 = 500;

// ---- starting conditions ---------------------------------------------------
pub const LIVES_START: i32 = 3;

/// One authored room: a maze (`#` wall, `.` floor), the player's start, the
/// cursed rune stones, the exit, the monsters, and where relevant a gate pair
/// or the Warlock's casting circles. Coordinates are `(col, row)`.
pub struct Room {
    pub maze: &'static [&'static str],
    pub player: (i32, i32),
    pub runes: &'static [(i32, i32)],
    pub exit: (i32, i32),
    pub imps: &'static [(i32, i32)],
    pub wraiths: &'static [(i32, i32)],
    pub hounds: &'static [(i32, i32)],
    pub gargoyles: &'static [(i32, i32)],
    /// Matching floor sigils: entering one emerges at its partner.
    pub gates: &'static [((i32, i32), (i32, i32))],
    pub warlock: bool,
    pub warlock_circles: &'static [(i32, i32)],
    pub warlock_wards: i32,
}

// Room 1: open corridors, one rune stone, visible Imps. Learn: move, face, cast, cleanse.
// Room 2: Wraiths. Learn: reveal and read a last-seen outline.
// Room 3: Spellhounds. Learn: reveal draws danger to where you stood.
// Room 4: paired rune gates. Learn: a marked passage moves you across the room.
// Room 5: Gargoyles. Learn: read a firing tell and clear the corridor.
// Room 6: the Warlock's chamber. Fight a rival wizard with every skill so far.
pub static ROOMS: &[Room] = &[
    Room {
        maze: &[
            "###############",
            "#.............#",
            "#..##....##...#",
            "#.............#",
            "#....##.......#",
            "#.........##..#",
            "#.##.......##.#",
            "#.........#...#",
            "#...##........#",
            "#.............#",
            "#......##..##.#",
            "#.............#",
            "###############",
        ],
        player: (1, 1),
        runes: &[(7, 6)],
        exit: (13, 11),
        imps: &[(5, 5), (9, 7)],
        wraiths: &[],
        hounds: &[],
        gargoyles: &[],
        gates: &[],
        warlock: false,
        warlock_circles: &[],
        warlock_wards: 0,
    },
    Room {
        maze: &[
            "###############",
            "#.............#",
            "#.####.####...#",
            "#....#....#...#",
            "#....#....#...#",
            "#.............#",
            "#.####.####...#",
            "#.............#",
            "#...###.###...#",
            "#.............#",
            "#.............#",
            "#.............#",
            "###############",
        ],
        player: (1, 1),
        runes: &[(7, 1), (7, 7)],
        exit: (13, 11),
        imps: &[],
        wraiths: &[(4, 7), (9, 3)],
        hounds: &[],
        gargoyles: &[],
        gates: &[],
        warlock: false,
        warlock_circles: &[],
        warlock_wards: 0,
    },
    Room {
        maze: &[
            "###############",
            "#...###...###.#",
            "#...#.....#...#",
            "#...#..#..#...#",
            "#.............#",
            "#.##..##..###.#",
            "#.............#",
            "#.###....###..#",
            "#.............#",
            "#...##..##....#",
            "#.............#",
            "#.............#",
            "###############",
        ],
        player: (1, 1),
        runes: &[(7, 4), (7, 8)],
        exit: (13, 11),
        imps: &[(11, 6)],
        wraiths: &[],
        hounds: &[(7, 1), (9, 6)],
        gargoyles: &[],
        gates: &[],
        warlock: false,
        warlock_circles: &[],
        warlock_wards: 0,
    },
    Room {
        maze: &[
            "###############",
            "#.............#",
            "#...###...###.#",
            "#...#.....#...#",
            "#...#.....#...#",
            "#.............#",
            "#...###...###.#",
            "#.............#",
            "#...#.....#...#",
            "#...#.....#...#",
            "#...###...###.#",
            "#.............#",
            "###############",
        ],
        player: (1, 1),
        runes: &[(7, 5), (7, 7)],
        exit: (13, 11),
        imps: &[(5, 1), (9, 1)],
        wraiths: &[(11, 5)],
        hounds: &[],
        gargoyles: &[],
        gates: &[((1, 11), (13, 1))],
        warlock: false,
        warlock_circles: &[],
        warlock_wards: 0,
    },
    Room {
        maze: &[
            "###############",
            "#....#...#....#",
            "#.##.#.#.#.##.#",
            "#.#......#.#..#",
            "#.#.###.#.##..#",
            "#...#...#..#..#",
            "#....###......#",
            "#..#.#.#.#....#",
            "#.##......#...#",
            "#.#..#.#..#...#",
            "#....#.#.#....#",
            "#.............#",
            "###############",
        ],
        player: (1, 1),
        runes: &[(7, 11), (7, 1)],
        exit: (13, 11),
        imps: &[(3, 5)],
        wraiths: &[],
        hounds: &[],
        gargoyles: &[(7, 5), (7, 8)],
        gates: &[],
        warlock: false,
        warlock_circles: &[],
        warlock_wards: 0,
    },
    Room {
        maze: &[
            "###############",
            "#.............#",
            "#.####..####..#",
            "#.#........#..#",
            "#.#..####..#..#",
            "#.#..#..#..#..#",
            "#.#..#..#..#..#",
            "#..##....##...#",
            "#...#..##.....#",
            "#....#..#.....#",
            "#.............#",
            "#.............#",
            "###############",
        ],
        player: (1, 1),
        runes: &[(13, 1), (13, 11)],
        exit: (7, 11),
        imps: &[],
        wraiths: &[(3, 9), (11, 9)],
        hounds: &[],
        gargoyles: &[],
        gates: &[],
        warlock: true,
        warlock_circles: &[(7, 1), (7, 11), (4, 6), (10, 6)],
        warlock_wards: 4,
    },
];

/// The room for a given 1-based level. Six authored rooms, then the wings
/// cycle back through the set from level 7 with the same layouts (the mazes
/// are reusable; difficulty is scaled in main.rs by speed and extra imps).
pub fn room_for(level: i32) -> &'static Room {
    &ROOMS[((level - 1).rem_euclid(ROOMS.len() as i32)) as usize]
}

/// Parse a room's maze into a wall bitmap (`true` = wall).
pub fn walls_for(room: &Room) -> Walls {
    let mut w = Walls::default();
    for (r, row) in room.maze.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            w.set(c as i32, r as i32, ch == '#');
        }
    }
    w
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::Cell;
    use std::collections::VecDeque;

    fn floor(w: &Walls, c: i32, r: i32) -> bool {
        (0..COLS as i32).contains(&c) && (0..ROWS as i32).contains(&r) && !w.get(c, r)
    }

    /// Every authored room is rectangular, bordered by walls, and its player
    /// start, runes, exit, gates, enemy spawns and warlock circles all sit on
    /// open floor reachable from the entrance. A maze that fails this is a
    /// softlock in the shipped game.
    #[test]
    fn every_room_is_rectangular_bordered_and_connected() {
        for (i, room) in ROOMS.iter().enumerate() {
            let tag = format!("room {}", i + 1);
            assert_eq!(room.maze.len(), ROWS, "{tag}: wrong row count");
            for (r, row) in room.maze.iter().enumerate() {
                assert_eq!(row.len(), COLS, "{tag}: row {r} not {COLS} wide");
            }
            for c in 0..COLS as i32 {
                assert!(room.maze[0].chars().nth(c as usize) == Some('#'), "{tag}: top open");
                assert!(room.maze[ROWS - 1].chars().nth(c as usize) == Some('#'), "{tag}: bottom open");
            }
            for row in room.maze {
                assert!(row.starts_with('#') && row.ends_with('#'), "{tag}: open side");
            }

            let w = walls_for(room);
            let mut reachable = std::collections::HashSet::new();
            let mut q = VecDeque::new();
            let start = Cell { c: room.player.0, r: room.player.1 };
            reachable.insert(start);
            q.push_back(start);
            while let Some(cell) = q.pop_front() {
                for d in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let n = Cell { c: cell.c + d.0, r: cell.r + d.1 };
                    if floor(&w, n.c, n.r) && reachable.insert(n) {
                        q.push_back(n);
                    }
                }
            }
            let on_floor = |p: (i32, i32)| floor(&w, p.0, p.1);
            let must = |p: (i32, i32), what: &str| {
                assert!(on_floor(p), "{tag}: {what} at {p:?} not on floor");
                assert!(reachable.contains(&Cell { c: p.0, r: p.1 }), "{tag}: {what} at {p:?} unreachable");
            };
            must(room.player, "player start");
            for p in room.runes { must(*p, "rune"); }
            must(room.exit, "exit");
            for p in room.imps { must(*p, "imp"); }
            for p in room.wraiths { must(*p, "wraith"); }
            for p in room.hounds { must(*p, "spellhound"); }
            for p in room.gargoyles { must(*p, "gargoyle"); }
            for (a, b) in room.gates { must(*a, "gate a"); must(*b, "gate b"); }
            for p in room.warlock_circles { must(*p, "warlock circle"); }
        }
    }
}