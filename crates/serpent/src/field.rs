//! The field's layout, shared by the game (main.rs) and the picture its build script paints (gfx.rs).
#![allow(dead_code)] // each side uses its own part

/// The board's columns and rows, and a cell's side in pixels.
pub const COLS: i32 = 20;
pub const ROWS: i32 = 20;
pub const CELL: i32 = 24;
/// The wood round the lawn, this deep on every side.
pub const FOREST: i32 = 40;
/// The field picture's side: the board and the wood round it.
pub const FIELD: i32 = COLS * CELL + 2 * FOREST;
/// Where the treasure chest sits in the field picture (its top left): in a small glade below the lawn, left of middle.
pub const CHEST: (i32, i32) = (FIELD / 4, FIELD - 28);
