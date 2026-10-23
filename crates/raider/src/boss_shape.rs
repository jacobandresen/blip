// The bosses' shapes, drawn by gfx.rs and flown by levels.rs: both include! this file.

// (span, length), growing with the level; the last, the Fugaku, is twice
// the first.
pub const BOSS_SIZES: [(i32, i32); 7] = [
    (96, 76),
    (104, 84),
    (116, 92),
    (150, 106),
    (164, 122),
    (182, 134),
    (192, 152),
];
// Gun positions per boss: (x as a fraction of the half span from the
// centreline, y as a fraction of the length from the nose). The game fires
// from the same points.
pub const BOSS_TURRETS: [&[(f32, f32)]; 7] = [
    &[(0.0, 0.06), (0.0, 0.45), (0.0, 0.97)],
    &[(0.0, 0.06), (0.0, 0.50), (0.0, 0.97)],
    &[
        (0.0, 0.07),
        (0.0, 0.36),
        (-0.09, 0.62),
        (0.09, 0.62),
        (0.0, 0.98),
    ],
    &[(0.0, 0.06), (0.0, 0.30), (0.0, 0.60), (0.0, 0.97)],
    &[
        (0.0, 0.07),
        (0.0, 0.36),
        (-0.07, 0.60),
        (0.07, 0.60),
        (0.0, 0.97),
    ],
    &[(0.0, 0.06), (0.0, 0.28), (0.0, 0.55), (0.0, 0.97)],
    &[(0.0, 0.05), (0.0, 0.97)],
];
