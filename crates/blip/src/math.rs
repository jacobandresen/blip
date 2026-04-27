//! Math / collision helpers.

use macroquad::rand;

#[inline]
pub fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo { lo } else if v > hi { hi } else { v }
}

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Inclusive on both ends, mirroring the C `blip_rand_int`.
pub fn rand_int(lo: i32, hi: i32) -> i32 {
    if hi <= lo {
        return lo;
    }
    rand::gen_range(lo, hi + 1)
}

#[inline]
pub fn rects_overlap(
    x1: f32, y1: f32, w1: f32, h1: f32,
    x2: f32, y2: f32, w2: f32, h2: f32,
) -> bool {
    x1 < x2 + w2 && x1 + w1 > x2 && y1 < y2 + h2 && y1 + h1 > y2
}
