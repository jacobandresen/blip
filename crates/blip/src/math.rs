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

/// Seed the random number generator. For tests: the generator is global, so
/// an unseeded simulation depends on what the tests beside it drew first.
pub fn rand_seed(seed: u64) {
    rand::srand(seed);
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

/// Deterministic scatter in `[0, 1)`: the same value for the same `k` and
/// `salt` on every frame, so scenery (a crowd, stars, windows) can be placed
/// without storing it. `k` indexes the thing, `salt` separates two uses.
pub fn scatter(k: usize, salt: u32) -> f32 {
    let mut h = (k as u32).wrapping_mul(2654435761).wrapping_add(salt);
    h ^= h >> 15;
    h = h.wrapping_mul(2246822519);
    h ^= h >> 13;
    (h % 1024) as f32 / 1024.0
}

#[cfg(test)]
mod tests {
    use super::scatter;

    #[test]
    fn scatter_is_the_same_every_time_and_stays_in_range() {
        for k in 0..500 {
            let v = scatter(k, 0x51);
            assert!((0.0..1.0).contains(&v));
            assert_eq!(v, scatter(k, 0x51));
        }
        // A different salt is a different scatter, and it does spread.
        let differs = (0..100).filter(|&k| scatter(k, 1) != scatter(k, 2)).count();
        assert!(differs > 90);
        let mean = (0..1000).map(|k| scatter(k, 7)).sum::<f32>() / 1000.0;
        assert!((0.4..0.6).contains(&mean), "mean {mean}");
    }
}
