//! Math and collision helpers — small utilities used across all games.

use macroquad::rand;

/// Clamp `v` so it stays within `[lo, hi]`.
#[inline]
pub fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo { lo } else if v > hi { hi } else { v }
}

/// Linear interpolation between `a` and `b`. `t=0` returns `a`, `t=1` returns `b`.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Random integer in the inclusive range `[lo, hi]`.
pub fn rand_int(lo: i32, hi: i32) -> i32 {
    if hi <= lo {
        return lo;
    }
    rand::gen_range(lo, hi + 1)
}

/// Axis-aligned bounding-box (AABB) overlap test.
/// Returns true if rectangle 1 and rectangle 2 share any area.
/// Both rectangles are specified as (x, y, width, height) with origin at top-left.
#[inline]
pub fn rects_overlap(
    x1: f32, y1: f32, w1: f32, h1: f32,
    x2: f32, y2: f32, w2: f32, h2: f32,
) -> bool {
    x1 < x2 + w2 && x1 + w1 > x2 && y1 < y2 + h2 && y1 + h1 > y2
}
