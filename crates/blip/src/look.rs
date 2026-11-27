//! The blip look, shared by every game: a warm toy-box palette after the Game Boy-era Zelda adventures, dark ink
//! outlines, soft round shadows, hearts for lives and one message box. Plain RGB here so build scripts (gfx.rs) can
//! paint with it; `color` has the same colours for runtime drawing and `frame` the shared HUD pieces.

/// Outlines, the HUD and message boxes, shadows: a deep plum, never pure black.
pub const INK: [u8; 3] = [34, 26, 46];
/// Text and box borders: warm paper white.
pub const CREAM: [u8; 3] = [250, 240, 212];
/// Lives, danger.
pub const HEART: [u8; 3] = [232, 64, 76];
/// Score, coins, highlights.
pub const GOLD: [u8; 3] = [250, 196, 60];
/// Grass, go, health.
pub const LEAF: [u8; 3] = [104, 190, 76];
/// Water, cold, the second player.
pub const SEA: [u8; 3] = [72, 168, 214];
/// Magic, specials.
pub const PLUM: [u8; 3] = [168, 92, 200];
/// Night and space backgrounds: a deep blue, so ink outlines still read on it.
pub const NIGHT: [u8; 3] = [26, 30, 58];
/// A softer night for patterned floors and the far sky.
pub const DUSK: [u8; 3] = [44, 46, 86];

/// `[r, g, b, a]` of a palette colour, for `Image` painting.
pub const fn rgba(c: [u8; 3], a: u8) -> [u8; 4] { [c[0], c[1], c[2], a] }
